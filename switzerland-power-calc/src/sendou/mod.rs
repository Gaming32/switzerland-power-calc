mod discord;
pub mod lang;
pub mod leaderboard;
mod rank_set;
pub mod schema;
mod types;
pub mod utils;

use crate::db::{Database, PlayerId, SwitzerlandPlayer, SwitzerlandPlayerMap};
use crate::sendou::discord::{DiscordEventHandler, DiscordHttp};
use crate::sendou::lang::{CommandIdDisplay, Language};
use crate::sendou::schema::{
    GetTournamentBracketResponse, GetTournamentBracketStandingsResponse,
    GetTournamentMatchResponse, GetTournamentResponse, GetTournamentTeamsResponse, MatchData,
    ParticipantResult, Side,
};
use crate::sendou::types::{DiscordChannelsMap, TeamsMap};
use crate::{
    Error, MAXIMUM_CALCED_RD, Result, format_player_rank_summary, format_player_simply, format_sp,
    query_json, summarize_differences,
};
use chrono::Utc;
use dashmap::DashMap;
use itertools::Itertools;
use reqwest::{Client as ReqwestClient, Client, StatusCode};
use rustyline_async::{Readline, ReadlineError, ReadlineEvent, SharedWriter};
use serde_json::json;
use serenity::FutureExt;
use serenity::all::{
    ActivityData, CacheHttp, Channel, ChannelId, ChannelType, CommandId, CommandOptionType,
    CreateAttachment, CreateChannel, CreateCommand, CreateCommandOption, CreateMessage,
    GatewayIntents, Guild, GuildId, Mentionable, MessageFlags, PermissionOverwrite,
    PermissionOverwriteType, Permissions, UserId,
};
use serenity::futures::TryStreamExt;
use serenity::futures::future::try_join_all;
use serenity::model::Timestamp;
use skillratings::Outcomes;
use skillratings::glicko2::{Glicko2Config, Glicko2Rating, decay_deviation, glicko2};
use std::collections::{HashMap, HashSet};
use std::fmt::Write as FmtWrite;
use std::io::{Read, Write as IoWrite};
use std::path::{Path, PathBuf};
use std::process::exit;
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;
use std::{fs, io};
use switzerland_power_animated::{AsyncAnimationGenerator, MatchOutcome, PowerStatus};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::sync::oneshot;
use tokio::time;
use tokio::time::{Interval, MissedTickBehavior, sleep};
use unic_emoji_char::is_emoji_presentation;

use crate::counts::{leaderboard_count, show_placement_count};
pub use crate::migration::migration_cli;
use crate::sendou::leaderboard::generate_leaderboard_messages;
use crate::sendou::rank_set::RankVec;
use crate::sendou::utils::{print_seeding_instructions, sendou_read_token_headers};
pub use schema::SendouId;

const POLL_TIME: Duration = Duration::from_secs(10);
const USER_CHANNEL_PERMS: Permissions = Permissions::VIEW_CHANNEL
    .union(Permissions::SEND_MESSAGES)
    .union(Permissions::USE_APPLICATION_COMMANDS);

#[tokio::main]
pub async fn sendou_cli(in_db: &Path, out_db: &Path, tournament_id: SendouId) -> Result<()> {
    if let Some(parent) = out_db.parent() {
        fs::create_dir_all(parent)?;
    }

    let http_client = reqwest::ClientBuilder::new()
        .user_agent(concat!(
            env!("CARGO_PKG_NAME"),
            " (https://github.com/Gaming32/switzerland-power-calc, ",
            env!("CARGO_PKG_VERSION"),
            ")"
        ))
        .default_headers(sendou_read_token_headers()?)
        .build()?;

    let discord_user_languages = Arc::new(DashMap::new());

    let (discord_ready_send, discord_ready) = oneshot::channel();
    let language_command_lock = Arc::new(RwLock::new(None));
    let discord_client = serenity::client::ClientBuilder::new(
        utils::env_str("DISCORD_BOT_TOKEN")?,
        GatewayIntents::GUILDS | GatewayIntents::GUILD_MEMBERS,
    )
    .event_handler(DiscordEventHandler {
        ready: Mutex::new(Some(discord_ready_send)),
        language_command: language_command_lock.clone(),
        language_output: discord_user_languages.clone(),
    })
    .activity(ActivityData::competing("Switzerland"))
    .await?;
    discord_client.shard_manager.set_shards(0, 1, 1).await;
    discord_client.shard_manager.initialize()?;
    discord_ready.await.unwrap();
    let discord_http = DiscordHttp::new(discord_client.cache.clone(), discord_client.http.clone());

    let chat_category = match utils::env::<ChannelId>("DISCORD_CHAT_CATEGORY_ID")?
        .to_channel(&discord_http)
        .await?
    {
        Channel::Private(channel) => {
            return Err(
                format!("Discord channel {} is not part of a guild", channel.name()).into(),
            );
        }
        Channel::Guild(channel) if channel.kind != ChannelType::Category => {
            return Err(format!(
                "Discord channel {} is not a Category channel, but a {:?} channel",
                channel.name, channel.kind
            )
            .into());
        }
        Channel::Guild(category) => category,
        _ => return Err("Your Discord channel is weird".into()),
    };
    let get_guild = || -> Result<_> {
        chat_category
            .guild_id
            .to_guild_cached(discord_http.cache())
            .ok_or_else(|| "Chat category Discord is not accessible by bot".into())
    };
    let leaderboard_channel = utils::env::<ChannelId>("DISCORD_LEADERBOARD_CHANNEL_ID")?;
    let moderator_channel = utils::env::<ChannelId>("DISCORD_MODERATOR_CHANNEL_ID")?;

    let old_players = Database::read(in_db)?.into_map();
    let mut new_players = old_players.clone();

    let tournament_teams: Vec<GetTournamentTeamsResponse> =
        query_json!(http_client, "/api/tournament/{}/teams", tournament_id);
    let teams = initialize_teams(
        tournament_id,
        &tournament_teams,
        &mut new_players,
        &http_client,
    )
    .await?;
    wait_for_tournament_start(tournament_id, &http_client).await?;

    let language_command = create_language_command();
    let language_command_id = get_guild()?
        .create_command(&discord_http, language_command)
        .await?
        .id;
    *language_command_lock.write().unwrap() = Some(language_command_id);
    drop(language_command_lock);

    let guild_channels = get_guild()?
        .channels
        .values()
        .map(|channel| (channel.name.clone(), channel.id))
        .collect();
    let discord_channels = create_discord_channels(
        &discord_http,
        chat_category.guild_id,
        guild_channels,
        chat_category.id,
        language_command_id,
        tournament_id,
        &http_client,
        &mut new_players,
    )
    .await?;

    run_tournament(
        &http_client,
        &discord_http,
        &mut new_players,
        &teams,
        &discord_user_languages,
        &discord_channels,
        tournament_id,
    )
    .await?;

    let new_db = finalize_tournament(out_db, &old_players, new_players)?;
    send_summaries_to_discord(
        &discord_http,
        &*get_guild()?,
        moderator_channel,
        leaderboard_channel,
        &old_players,
        &teams,
        &new_db,
        tournament_id,
        &http_client,
    )
    .await?;

    println!("Press enter when finished to clean up Discord channels");
    let _ = io::stdin().read(&mut [0]);
    clean_up_discord_channels(&discord_http, discord_channels.into_values()).await;

    get_guild()?
        .delete_command(discord_http.http(), language_command_id)
        .await?;
    discord_client.shard_manager.shutdown_all().await;

    let new_user_languages = teams
        .values()
        .filter_map(|team| {
            let player = team.members.first().unwrap();
            discord_user_languages
                .get(&player.discord_id)
                .as_deref()
                .copied()
                .map(|lang| (player.user_id, lang))
        })
        .collect::<HashMap<_, _>>();
    if !new_user_languages.is_empty() {
        let mut new_db = new_db;
        for user in &mut new_db.players {
            let PlayerId::Sendou(sendou_id) = user.id else {
                continue;
            };
            if let Some(new_language) = new_user_languages.get(&sendou_id) {
                user.language = Some(*new_language);
            }
        }
        new_db.write(out_db)?;
    }

    Ok(())
}

async fn initialize_teams<'a>(
    tournament_id: SendouId,
    tournament_teams: &'a [GetTournamentTeamsResponse],
    players: &mut SwitzerlandPlayerMap,
    http_client: &Client,
) -> Result<TeamsMap<'a>> {
    let mut teams = HashMap::new();
    for player in players.values_mut() {
        player.since_played += 1;
    }
    for team in tournament_teams {
        let player = team.members.first().expect("Sendou team has no members");
        // Using unranked power because that's what we've historically done, and the math is tuned to it
        let starting_rating = team
            .seeding_power
            .unranked
            .map_or(0.0, |power| (power - 1000.0) / 15.0)
            .clamp(-10.0, 40.0);
        teams.insert(team.id, team);
        players
            .entry(PlayerId::Sendou(player.user_id))
            .and_modify(|player| {
                // since_played will be 1 above the desired value due to the increment above
                for _ in 1..player.since_played {
                    player.rating = decay_deviation(&player.rating);
                }
                player.since_played = 0;
            })
            .or_insert_with(|| SwitzerlandPlayer {
                id: PlayerId::Sendou(player.user_id),
                rating: Glicko2Rating {
                    rating: starting_rating * 10.0 + 1500.0,
                    deviation: 350.0 - starting_rating.abs() * 3.75,
                    ..Default::default()
                },
                unrated: true,
                ..Default::default()
            })
            .display_name = Some(player.name.clone());
    }

    let sorted_players = print_seeding_instructions(
        players,
        teams.values().map(|team| {
            (
                team,
                PlayerId::Sendou(team.members.first().unwrap().user_id),
            )
        }),
        |team, player| {
            format!(
                "{} ({}) [{}{}]",
                team.name,
                team.members.first().unwrap().name,
                format_sp(player.rating, true),
                if player.unrated { " (NEW)" } else { "" }
            )
        },
    );

    if !tournament_started(tournament_id, http_client).await? {
        let mut seeded_team_ids = vec![];
        let (above_1500, below_1500) = sorted_players.split_at(
            sorted_players
                .iter()
                .position(|(_, p)| p.rating.rating < 1500.0)
                .unwrap_or(sorted_players.len()),
        );
        seeded_team_ids.extend(above_1500.iter().map(|(t, _)| t.id));
        for team in tournament_teams {
            let player = &players[&PlayerId::Sendou(team.members.first().unwrap().user_id)];
            if player.rating.rating == 1500.0 {
                seeded_team_ids.push(team.id);
            }
        }
        seeded_team_ids.extend(below_1500.iter().map(|(t, _)| t.id));
        http_client
            .post(format!(
                "https://sendou.ink/api/tournament/{tournament_id}/seeds"
            ))
            .bearer_auth(utils::env_str("SENDOU_WRITE_TOKEN")?)
            .json(&json!({
                "tournamentTeamIds": seeded_team_ids,
            }))
            .send()
            .await?
            .error_for_status()?;
    }

    Ok(teams)
}

async fn wait_for_tournament_start(
    tournament_id: SendouId,
    http_client: &ReqwestClient,
) -> Result<()> {
    let tournament: GetTournamentResponse =
        query_json!(http_client, "/api/tournament/{}", tournament_id);
    if let Ok(delay) = tournament
        .start_time
        .signed_duration_since(Utc::now())
        .to_std()
    {
        println!(
            "Waiting {}m {}s for tournament start time...",
            delay.as_secs() / 60,
            delay.as_secs() % 60
        );
        sleep(delay).await;
    }

    println!("Waiting for tournament to be started...");
    loop {
        if tournament_started(tournament_id, http_client).await? {
            break;
        }
        sleep(POLL_TIME).await;
    }

    Ok(())
}

async fn tournament_started(tournament_id: SendouId, http_client: &ReqwestClient) -> Result<bool> {
    let tournament_started = http_client
        .get(format!(
            "https://sendou.ink/api/tournament/{tournament_id}/brackets/0/standings"
        ))
        .send()
        .await?
        .status()
        != StatusCode::NOT_FOUND;
    Ok(tournament_started)
}

fn create_language_command() -> CreateCommand {
    let default_language = Language::default();
    let base_command_name = default_language.language_command_name();
    let base_command_desc = default_language.language_command_desc();
    let base_command_arg_desc = default_language.language_command_arg_desc();

    let mut command =
        CreateCommand::new(base_command_name.clone()).description(base_command_desc.clone());
    let mut option = CreateCommandOption::new(
        CommandOptionType::String,
        base_command_name.clone(),
        base_command_arg_desc.clone(),
    );

    for language in Language::supported_languages() {
        if let Some(discord_lang_id) = language.discord_id()
            && let Some(fallback_language) = language.fallback()
        {
            let localized_name = language.language_command_name();
            if localized_name != fallback_language.language_command_name() {
                command = command.name_localized(discord_lang_id, localized_name.clone());
                option = option.name_localized(discord_lang_id, localized_name);
            }

            let localized_desc = language.language_command_desc();
            if localized_desc != fallback_language.language_command_desc() {
                command = command.description_localized(discord_lang_id, localized_desc);
            }

            let localized_arg_dec = language.language_command_arg_desc();
            if localized_arg_dec != fallback_language.language_command_arg_desc() {
                option = option.description_localized(discord_lang_id, localized_arg_dec);
            }
        }

        option = option.add_string_choice(language.name(), language.id());
    }

    command.add_option(option)
}

#[allow(clippy::too_many_arguments)]
async fn create_discord_channels(
    discord_http: &DiscordHttp,
    guild_id: GuildId,
    mut guild_channels_by_name: HashMap<String, ChannelId>,
    category: ChannelId,
    language_command_id: CommandId,
    tournament_id: SendouId,
    http_client: &ReqwestClient,
    players: &mut SwitzerlandPlayerMap,
) -> Result<DiscordChannelsMap> {
    println!("Creating Discord channels...");

    let mut channels = HashMap::new();

    let me_user = discord_http.cache().current_user();
    let commentators_role = utils::env("DISCORD_COMMENTATORS_ROLE_ID")?;

    let mut tournament_teams: Vec<GetTournamentTeamsResponse> =
        query_json!(http_client, "/api/tournament/{}/teams", tournament_id);
    tournament_teams.sort_by(|team1, team2| {
        let player1 = &players[&PlayerId::Sendou(team1.members.first().unwrap().user_id)];
        let player2 = &players[&PlayerId::Sendou(team2.members.first().unwrap().user_id)];
        player1.descending_rating_order_cmp(player2)
    });
    for team in tournament_teams {
        if !team.checked_in {
            continue;
        }
        let player = team.members.first().unwrap();

        let switzerland_player = &mut players[&PlayerId::Sendou(player.user_id)];
        let guess_language = switzerland_player.language.is_none();
        let language = switzerland_player.language.get_or_insert_with(|| {
            player
                .country
                .as_ref()
                .and_then(|lang| Language::guess_from_country(lang))
                .unwrap_or_default()
        });
        let language_command =
            CommandIdDisplay(language.language_command_name(), language_command_id);

        let user = player.discord_id.to_user(discord_http).await?;
        let channel_name = format!("switzerland-{}", user.name.replace('.', ""));
        let channel = if let Some(channel) = guild_channels_by_name.remove(&channel_name) {
            channel.say(discord_http, language.bot_crashed()).await?;
            channel
        } else {
            let channel = guild_id
                .create_channel(
                    discord_http,
                    CreateChannel::new(channel_name)
                        .category(category)
                        .permissions([
                            PermissionOverwrite {
                                allow: Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES,
                                deny: Permissions::empty(),
                                kind: PermissionOverwriteType::Member(me_user.id),
                            },
                            PermissionOverwrite {
                                allow: USER_CHANNEL_PERMS,
                                deny: Permissions::empty(),
                                kind: PermissionOverwriteType::Member(user.id),
                            },
                            PermissionOverwrite {
                                allow: Permissions::VIEW_CHANNEL,
                                deny: Permissions::SEND_MESSAGES,
                                kind: PermissionOverwriteType::Role(commentators_role),
                            },
                            PermissionOverwrite {
                                allow: Permissions::empty(),
                                deny: Permissions::VIEW_CHANNEL,
                                kind: PermissionOverwriteType::Role(guild_id.everyone_role()),
                            },
                        ]),
                )
                .await?;
            channel
                .say(discord_http, language.channel_explanation(user.mention()))
                .await?;
            channel.id
        };
        if guess_language {
            channel
                .say(
                    discord_http,
                    language.language_command_explanation(&language_command, *language),
                )
                .await?;
        }
        channels.insert(team.id, channel);
    }

    Ok(channels)
}

#[allow(clippy::too_many_arguments)]
async fn run_tournament(
    http_client: &ReqwestClient,
    http: &DiscordHttp,
    players: &mut SwitzerlandPlayerMap,
    teams: &TeamsMap<'_>,
    discord_user_languages: &DashMap<UserId, Language>,
    discord_channels: &DiscordChannelsMap,
    tournament_id: SendouId,
) -> Result<()> {
    let mut command_engine = CommandEngine::new()?;

    let mut completed_matches = HashSet::new();

    let animation_generator = AsyncAnimationGenerator::new().await?;
    let top_player_count = leaderboard_count(players.len());
    let show_placement_count = show_placement_count(players.len());

    let new_players = loop {
        let tournament: GetTournamentResponse =
            query_json!(http_client, "/api/tournament/{}", tournament_id);
        let brackets: Vec<GetTournamentBracketResponse> = try_join_all(
            tournament
                .brackets
                .iter()
                .enumerate()
                .map(async |(idx, _)| {
                    Result::Ok(query_json!(
                        http_client,
                        "/api/tournament/{}/brackets/{}",
                        tournament_id,
                        idx,
                    ))
                }),
        )
        .await?;

        let mut new_players = players.clone();
        let mut ranked_players = RankVec::new(
            players
                .values()
                .filter(|p| p.show_rank())
                .map(|p| (p.id.clone(), p.rating))
                .collect_vec(),
        );

        for tourney_match in brackets.iter().flat_map(|bracket| &bracket.data.r#match) {
            if command_engine.ignored_matches.contains(&tourney_match.id) {
                continue;
            }

            if tourney_match.winner_side.is_some() {
                let score1 = tourney_match.opponent1.unwrap().score;
                let score2 = tourney_match.opponent2.unwrap().score;
                if score1.is_none() || score2.is_none() {
                    continue;
                }
            }

            let get_player = |opponent: &Option<ParticipantResult>| {
                teams
                    .get(&opponent.unwrap().id.expect("Null opponent in ready match"))
                    .and_then(|team| {
                        let player_id = PlayerId::Sendou(team.members.first().unwrap().user_id);
                        let player = new_players.get(&player_id)?;
                        Some((team, player_id, player.rating, player.language.unwrap()))
                    })
                    .unwrap()
            };
            if tourney_match.opponent1.is_none() || tourney_match.opponent2.is_none() {
                continue; // BYE
            }
            if tourney_match.winner_side.is_none() {
                completed_matches.remove(&tourney_match.id);
                continue;
            }
            let new_match = completed_matches.insert(tourney_match.id);
            let (team1, player1, rating1, language1) = get_player(&tourney_match.opponent1);
            let (team2, player2, rating2, language2) = get_player(&tourney_match.opponent2);
            let (new_rating1, new_rating2) = glicko2(
                &rating1,
                &rating2,
                &match tourney_match.winner_side.unwrap() {
                    Side::Opponent1 => Outcomes::WIN,
                    Side::Opponent2 => Outcomes::LOSS,
                },
                &Glicko2Config::default(),
            );
            if new_match {
                writeln!(command_engine.printer, "In match {}:", tourney_match.id)?;
            }
            let mut update_player = async |win,
                                           team: &GetTournamentTeamsResponse,
                                           other_team: &GetTournamentTeamsResponse,
                                           player,
                                           new_rating,
                                           language|
                   -> Result<()> {
                let player = &mut new_players[player];
                let old_player = player.clone();
                player.rating = new_rating;
                player.unrated = false;
                if player.rating.deviation <= MAXIMUM_CALCED_RD {
                    player.calced = true;
                }

                let old_rank = ranked_players
                    .get_rank_and_remove(&old_player.id, old_player.rating)
                    .map_or(u32::MAX as usize, |r| r + 1);
                let new_rank =
                    ranked_players.insert_and_get_rank(old_player.id.clone(), player.rating) + 1;
                let rank_change = (old_rank <= show_placement_count
                    || new_rank <= show_placement_count)
                    .then_some((old_rank, new_rank));

                if !new_match {
                    return Ok(());
                }

                writeln!(
                    command_engine.printer,
                    "  {}",
                    format_player_simply(Some(&old_player), player, false, true)
                )?;
                send_progress_message_to_player(
                    http_client,
                    http,
                    discord_channels,
                    discord_user_languages,
                    tournament_id,
                    tourney_match,
                    &animation_generator,
                    team,
                    other_team,
                    win,
                    &old_player,
                    player,
                    rank_change,
                    top_player_count,
                    language,
                )?;
                Ok(())
            };
            update_player(
                tourney_match.winner_side == Some(Side::Opponent1),
                team1,
                team2,
                &player1,
                new_rating1,
                language1,
            )
            .await?;
            update_player(
                tourney_match.winner_side == Some(Side::Opponent2),
                team2,
                team1,
                &player2,
                new_rating2,
                language2,
            )
            .await?;
        }

        if tournament.is_finalized {
            break new_players;
        }

        command_engine.pump().await?;
    };

    *players = new_players;
    Ok(())
}

enum CommandEngineAction {
    Poll(bool),
    SkipMatch(SendouId),
    Error(io::Error),
    Quit,
}

struct CommandEngine {
    action_recv: UnboundedReceiver<CommandEngineAction>,
    printer: SharedWriter,
    ignored_matches: HashSet<SendouId>,
    interval: Interval,
}

impl CommandEngine {
    fn new() -> Result<Self> {
        let (action_send, action_recv) = tokio::sync::mpsc::unbounded_channel();
        let (rl, printer) = Readline::new("command> ".to_string())?;
        Self::start_task(rl, action_send, printer.clone());

        let mut interval = time::interval(POLL_TIME);
        interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

        Ok(Self {
            action_recv,
            printer,
            ignored_matches: HashSet::new(),
            interval,
        })
    }

    fn start_task(
        mut rl: Readline,
        action_send: UnboundedSender<CommandEngineAction>,
        mut printer: SharedWriter,
    ) {
        tokio::task::spawn(async move {
            let mut run = async || -> io::Result<()> {
                loop {
                    let line = match rl.readline().await {
                        Ok(ReadlineEvent::Line(line)) => line,
                        Ok(ReadlineEvent::Eof) => break,
                        Ok(ReadlineEvent::Interrupted) => {
                            let _ = action_send.send(CommandEngineAction::Quit);
                            break;
                        }
                        Err(ReadlineError::IO(err)) => return Err(err),
                        Err(ReadlineError::Closed) => break,
                    };
                    let action = if line == "help" || line == "?" {
                        writeln!(printer, "help")?;
                        writeln!(printer, "   Prints this message")?;
                        writeln!(printer, "?")?;
                        writeln!(printer, "   Prints this message")?;
                        writeln!(printer, "skip <match-id>")?;
                        writeln!(printer, "   Ignores the specified match")?;
                        writeln!(printer, "poll")?;
                        writeln!(printer, "   Forces a recheck of sendou.ink")?;
                        None
                    } else if line.starts_with("skip ") {
                        match line.strip_prefix("skip ").unwrap().parse() {
                            Ok(id) => Some(CommandEngineAction::SkipMatch(id)),
                            Err(err) => {
                                writeln!(printer, "Invalid match ID: {err}")?;
                                None
                            }
                        }
                    } else if line == "poll" {
                        Some(CommandEngineAction::Poll(true))
                    } else {
                        writeln!(printer, "Unknown or invalid command: {line}")?;
                        writeln!(printer, "Type 'help' or '?' to see a list of commands")?;
                        None
                    };
                    if let Some(action) = action
                        && action_send.send(action).is_err()
                    {
                        break;
                    }
                }
                Ok(())
            };
            if let Err(err) = run().await {
                let _ = action_send.send(CommandEngineAction::Error(err));
            }
        });
    }

    async fn pump(&mut self) -> Result<()> {
        loop {
            let action = tokio::select! {
                _ = self.interval.tick() => CommandEngineAction::Poll(false),
                action = self.action_recv.recv() => action.expect("Action input thread exited unexpectedly without Error"),
            };
            match action {
                CommandEngineAction::Poll(forced) => {
                    if forced {
                        writeln!(self.printer, "Polling now")?;
                    }
                    break;
                }
                CommandEngineAction::SkipMatch(id) => {
                    self.ignored_matches.insert(id);
                    writeln!(self.printer, "Ignoring match {id}")?;
                }
                CommandEngineAction::Error(err) => return Err(err.into()),
                CommandEngineAction::Quit => {
                    writeln!(self.printer, "Force quitting now")?;
                    exit(1);
                }
            }
        }
        Ok(())
    }
}

#[allow(clippy::too_many_arguments)]
fn send_progress_message_to_player(
    http_client: &ReqwestClient,
    discord_http: &DiscordHttp,
    discord_channels: &DiscordChannelsMap,
    discord_user_languages: &DashMap<UserId, Language>,
    tournament_id: SendouId,
    tourney_match: &MatchData,
    animation_generator: &AsyncAnimationGenerator,
    team: &GetTournamentTeamsResponse,
    other_team: &GetTournamentTeamsResponse,
    win: bool,
    old_player: &SwitzerlandPlayer,
    new_player: &SwitzerlandPlayer,
    rank_change: Option<(usize, usize)>,
    top_rank: usize,
    original_language: Language,
) -> Result<()> {
    let Some(discord_channel) = discord_channels.get(&team.id).copied() else {
        return Ok(());
    };

    fn calc_percentage(deviation: f64) -> f64 {
        const DEFAULT_RD: f64 = 350.0;
        1.0 - (deviation - MAXIMUM_CALCED_RD) / (DEFAULT_RD - MAXIMUM_CALCED_RD)
    }
    let old_calc_percent = if old_player.unrated {
        0.0
    } else {
        calc_percentage(old_player.rating.deviation)
    };
    let new_calc_percent = calc_percentage(new_player.rating.deviation);

    let mut power_status = if old_player.calced {
        PowerStatus::SetPlayed {
            matches: Default::default(),
            old_power: old_player.rating.rating,
            new_power: new_player.rating.rating,
            rank_change: rank_change.map(|(old, new)| (old as u32, new as u32)),
            top_rank: top_rank as u32,
        }
    } else if new_player.calced {
        PowerStatus::Calculated {
            prev_calc_percent: old_calc_percent,
            power: new_player.rating.rating,
            rank: rank_change.map(|(_, new)| new as u32),
            top_rank: top_rank as u32,
        }
    } else {
        PowerStatus::Calculating {
            old_calc_percent,
            new_calc_percent,
        }
    };

    let player_discord_id = team.members.first().unwrap().discord_id;
    let language = discord_user_languages
        .get(&player_discord_id)
        .as_deref()
        .copied()
        .unwrap_or(original_language);

    let message = format_link(
        &language.round_played(
            match win {
                true => language.to_animation_language().win(),
                false => language.to_animation_language().lose(),
            },
            &other_team.members.first().unwrap().name,
        ),
        &format!(
            "<https://sendou.ink/to/{}/matches/{}>",
            tournament_id, tourney_match.id,
        ),
    );

    let http_client = http_client.clone();
    let discord_http = discord_http.clone();
    let set_id = tourney_match.id;
    let animation_generator = animation_generator.clone();
    let my_team_id = team.id;
    tokio::spawn(
        async move {
            if let PowerStatus::SetPlayed { matches, .. } = &mut power_status {
                let match_data: GetTournamentMatchResponse =
                    query_json!(http_client, "/api/tournament-match/{}", set_id);
                for (i, result) in match_data.map_list.unwrap().into_iter().enumerate() {
                    matches[i] = if result.winner_team_id.unwrap() == my_team_id {
                        MatchOutcome::Win
                    } else {
                        MatchOutcome::Lose
                    };
                }
            }
            let filename = format!("set-{set_id}-{my_team_id}.webp");
            let animation = animation_generator
                .generate(power_status, language.into())
                .await?;
            // discord_channel
            //     .create_permission(
            //         discord_http.http(),
            //         PermissionOverwrite {
            //             allow: USER_CHANNEL_PERMS,
            //             deny: Permissions::empty(),
            //             kind: PermissionOverwriteType::Member(player_discord_id),
            //         },
            //     )
            //     .await?;
            let send_result = discord_channel
                .send_message(
                    discord_http,
                    CreateMessage::new()
                        .content(message)
                        .add_file(CreateAttachment::bytes(animation.clone(), &filename)),
                )
                .await;
            if let Err(result) = send_result {
                if let Ok(backups_dir) = utils::env::<PathBuf>("GENERATED_ANIM_BACKUPS_DIR")
                    && let Err(err) = fs::write(backups_dir.join(&filename), &animation)
                {
                    println!("Failed to save backup animation file for {set_id}: {err}");
                }
                return Err(result.into());
            }
            Ok::<(), Error>(())
        }
        .then(async move |result| {
            if let Err(err) = result {
                println!("Failed to send results message for set {set_id}: {err}");
            }
        }),
    );
    Ok(())
}

fn format_link(body: &str, link: &str) -> String {
    if !body.chars().any(is_emoji_presentation) {
        format!("[{body}]({link})")
    } else {
        format!("{body} ({link})")
    }
}

fn finalize_tournament(
    out_db: &Path,
    old_players: &SwitzerlandPlayerMap,
    new_players: SwitzerlandPlayerMap,
) -> Result<Database> {
    let new_db = Database::new_from_map(new_players);
    new_db.write(out_db)?;

    println!("\nSP comparison (switzerland-power-calc compare):");
    summarize_differences(old_players, &new_db.players);

    Ok(new_db)
}

#[allow(clippy::too_many_arguments)]
async fn send_summaries_to_discord(
    discord_http: &DiscordHttp,
    guild: &Guild,
    moderator_channel: ChannelId,
    leaderboard_channel: ChannelId,
    old_players: &SwitzerlandPlayerMap,
    teams: &TeamsMap<'_>,
    new_db: &Database,
    tournament_id: SendouId,
    http_client: &ReqwestClient,
) -> Result<()> {
    println!("\nSending comparison to Discord...");
    let player_id_to_discord_id = teams
        .values()
        .filter(|team| team.checked_in)
        .map(|team| team.members.first().unwrap())
        .map(|player| (PlayerId::Sendou(player.user_id), player.discord_id))
        .collect::<HashMap<_, _>>();

    let mut players_in_discord = HashSet::new();
    for user_id in player_id_to_discord_id.values().copied() {
        if guild.member(discord_http, user_id).await.is_ok() {
            players_in_discord.insert(user_id);
        }
    }

    let tournament: GetTournamentResponse =
        query_json!(http_client, "/api/tournament/{}", tournament_id);
    let tournament_teams: Vec<GetTournamentTeamsResponse> =
        query_json!(http_client, "/api/tournament/{}/teams", tournament_id);

    {
        let mut message = String::new();
        let _ = writeln!(
            message,
            "And that concludes {}! Thank you all for participating, and I hope you had a good time.",
            tournament.name,
        );
        let mut print_results = |title, results: &[SendouId; 3]| {
            let _ = writeln!(message, "## {title}");
            for (team_id, emoji) in results.iter().zip(['🥇', '🥈', '🥉']) {
                let player = tournament_teams
                    .iter()
                    .find(|x| x.id == *team_id)
                    .unwrap()
                    .members
                    .first()
                    .unwrap();
                let _ = writeln!(
                    message,
                    "- {emoji} {}{}",
                    player.name,
                    if players_in_discord.contains(&player.discord_id) {
                        format!(" ({})", player.discord_id.mention())
                    } else {
                        "".to_string()
                    },
                );
            }
        };

        let standings = try_join_all(
            tournament
                .brackets
                .iter()
                .enumerate()
                .skip(1)
                .filter(|(_, bracket)| !bracket.name.contains("UG"))
                .map(async |(idx, bracket)| {
                    let standings: GetTournamentBracketStandingsResponse = query_json!(
                        http_client,
                        "/api/tournament/{}/brackets/{}/standings",
                        tournament_id,
                        idx,
                    );
                    Result::Ok((
                        bracket.name.as_str(),
                        standings
                            .standings
                            .into_iter()
                            .map(|standing| standing.tournament_team_id)
                            .next_array::<3>()
                            .unwrap(),
                    ))
                }),
        )
        .await?;
        match &standings[..] {
            [] => {}
            [(_, results)] => print_results("Results".to_string(), results),
            all_results => {
                for (bracket, results) in all_results {
                    print_results(format!("{bracket} results"), results);
                }
            }
        }

        let _ = writeln!(message, "## Switzerland Power changes");
        let show_placement_count = show_placement_count(new_db.players.len());
        let should_show_rank = |player: &SwitzerlandPlayer| {
            player
                .rank
                .is_some_and(|r| r.get() as usize <= show_placement_count)
        };
        for new_player in &new_db.players {
            if new_player.rating.deviation > MAXIMUM_CALCED_RD {
                continue;
            }
            let Some(discord_id) = player_id_to_discord_id.get(&new_player.id) else {
                continue;
            };
            if !players_in_discord.contains(discord_id) {
                continue;
            }
            let old_result = old_players.get(&new_player.id);
            if let Some(old_result) = old_result
                && old_result.rating == new_player.rating
            {
                continue;
            }
            let _ = writeln!(
                message,
                "- {} {}",
                discord_id.mention(),
                format_player_rank_summary(
                    old_result,
                    new_player,
                    old_result.is_some_and(should_show_rank) || should_show_rank(new_player),
                    false,
                )
            );
        }

        moderator_channel
            .send_message(discord_http, CreateMessage::new().content(message))
            .await?;
    }

    {
        let old_leaderboard_messages = leaderboard_channel
            .messages_iter(discord_http.http())
            .try_collect::<Vec<_>>()
            .await?;
        for message in
            generate_leaderboard_messages(old_players, new_db, &player_id_to_discord_id, 2000)
        {
            leaderboard_channel
                .send_message(
                    discord_http,
                    CreateMessage::new()
                        .content(message)
                        .flags(MessageFlags::SUPPRESS_NOTIFICATIONS),
                )
                .await?;
        }
        let allow_bulk_delete_timestamp =
            Timestamp::from_unix_timestamp(Timestamp::now().unix_timestamp() - 60 * 60 * 24 * 13)
                .unwrap();
        for messages in old_leaderboard_messages
            .into_iter()
            .chunks(100)
            .into_iter()
            .map(Itertools::collect_vec)
        {
            if messages
                .iter()
                .all(|x| x.timestamp > allow_bulk_delete_timestamp)
            {
                leaderboard_channel
                    .delete_messages(discord_http.http(), messages)
                    .await?;
            } else {
                for message in messages {
                    message.delete(discord_http).await?;
                }
            }
        }
    }

    Ok(())
}

async fn clean_up_discord_channels(
    http: &DiscordHttp,
    channels: impl IntoIterator<Item = ChannelId>,
) {
    println!("Deleting Discord channels...");
    for channel in channels {
        let _ = channel.delete(http.http()).await;
    }
}
