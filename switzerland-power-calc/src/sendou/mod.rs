mod discord;
pub mod lang;
mod schema;
mod types;

use crate::cli_helpers::{TrieHinter, print_seeding_instructions};
use crate::db::{Database, SwitzerlandPlayer, SwitzerlandPlayerMap};
use crate::sendou::discord::{DiscordEventHandler, DiscordHttp};
use crate::sendou::lang::{CommandIdDisplay, Language};
use crate::sendou::schema::{
    MatchRoot, SendouId, TournamentContext, TournamentData, TournamentMatch,
    TournamentMatchOpponent, TournamentMatchResult, TournamentMatchStatus, TournamentRoot,
    TournamentStageSettings, TournamentTeam,
};
use crate::sendou::types::{
    DescendingRatingGlicko2, DiscordChannelsMap, GetTournamentFn, TeamsMap,
};
use crate::{
    Error, Result, format_player_rank_summary, format_player_simply, summarize_differences,
};
use ansi_term::Color;
use chrono::Utc;
use dashmap::DashMap;
use itertools::Itertools;
use reqwest::{Client as ReqwestClient, Url};
use rustyline::history::DefaultHistory;
use rustyline::{DefaultEditor, Editor, ExternalPrinter};
use serenity::FutureExt;
use serenity::all::{
    ActivityData, CacheHttp, Channel, ChannelId, ChannelType, CommandId, CommandOptionType,
    CreateAttachment, CreateChannel, CreateCommand, CreateCommandOption, CreateMessage,
    GatewayIntents, Guild, GuildId, Mentionable, PermissionOverwrite, PermissionOverwriteType,
    Permissions, UserId,
};
use skillratings::Outcomes;
use skillratings::glicko2::{Glicko2Config, Glicko2Rating, glicko2};
use std::collections::{HashMap, HashSet};
use std::fmt::Write;
use std::fs;
use std::io::Read;
use std::path::Path;
use std::str::FromStr;
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;
use switzerland_power_animated::{AsyncAnimationGenerator, MatchOutcome, PowerStatus};
use tokio::sync::oneshot;
use tokio::time::sleep;
use trie_rs::Trie;
use unic_emoji_char::{is_emoji, is_emoji_component};

#[tokio::main]
pub async fn sendou_cli(in_db: &Path, out_db: &Path, tournament_url: &str) -> Result<()> {
    if let Some(parent) = out_db.parent() {
        fs::create_dir_all(parent)?;
    }

    let tournament_url = {
        let mut url = Url::parse(tournament_url)?;
        url.set_query(Some("_data=features/tournament/routes/to.$id"));
        url
    };
    let http_client = reqwest::ClientBuilder::new()
        .user_agent(concat!(
            env!("CARGO_PKG_NAME"),
            " (https://github.com/Gaming32/switzerland-power-calc, ",
            env!("CARGO_PKG_VERSION"),
            ")"
        ))
        .build()?;

    let discord_user_languages = Arc::new(DashMap::new());

    let (discord_ready_send, discord_ready) = oneshot::channel();
    let language_command_lock = Arc::new(RwLock::new(None));
    let discord_client = serenity::client::ClientBuilder::new(
        env_str("DISCORD_BOT_TOKEN")?,
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

    let chat_category = match env::<ChannelId>("DISCORD_CHAT_CATEGORY_ID")?
        .to_channel(&discord_http)
        .await?
    {
        Channel::Private(channel) => {
            return Err(Error::Custom(format!(
                "Discord channel {} is not part of a guild",
                channel.name()
            )));
        }
        Channel::Guild(channel) if channel.kind != ChannelType::Category => {
            return Err(Error::Custom(format!(
                "Discord channel {} is not a Category channel, but a {:?} channel",
                channel.name, channel.kind
            )));
        }
        Channel::Guild(category) => category,
        _ => return Err(Error::Custom("Your Discord channel is weird".to_string())),
    };
    let get_guild = || -> Result<_> {
        chat_category
            .guild_id
            .to_guild_cached(discord_http.cache())
            .ok_or_else(|| {
                Error::Custom("Chat category Discord is not accessible by bot".to_string())
            })
    };
    let moderator_channel = env::<ChannelId>("DISCORD_MODERATOR_CHANNEL_ID")?;

    let get_tournament = async || -> Result<_> {
        Ok(http_client
            .get(tournament_url.clone())
            .send()
            .await?
            .error_for_status()?
            .json::<TournamentRoot>()
            .await?
            .tournament)
    };
    let tournament_context = get_tournament().await?.context;

    let old_players = Database::read(in_db)?.into_map();
    let mut new_players = old_players.clone();

    let teams = initialize_teams(&tournament_context, &old_players, &mut new_players)?;
    wait_for_tournament_start(&tournament_context, &get_tournament).await?;

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
        &get_tournament,
        &mut new_players,
        &teams,
    )
    .await?;

    run_tournament(
        &http_client,
        &discord_http,
        &old_players,
        &mut new_players,
        &teams,
        &discord_user_languages,
        &discord_channels,
        &get_tournament,
    )
    .await?;

    let new_db = finalize_tournament(out_db, &old_players, new_players)?;
    send_summary_to_discord(
        &discord_http,
        &*get_guild()?,
        moderator_channel,
        &old_players,
        &teams,
        &new_db,
        &get_tournament,
    )
    .await?;

    println!("Press enter when finished to clean up Discord channels");
    let _ = std::io::stdin().read(&mut [0]);
    clean_up_discord_channels(&discord_http, discord_channels.into_values()).await;

    get_guild()?
        .delete_command(discord_http.http(), language_command_id)
        .await?;
    discord_client.shard_manager.shutdown_all().await;

    let new_user_languages = teams
        .values()
        .filter_map(|(team, player)| {
            discord_user_languages
                .get(&team.members.first().unwrap().discord_id)
                .as_deref()
                .copied()
                .map(|lang| (player.clone(), lang))
        })
        .collect::<HashMap<_, _>>();
    if !new_user_languages.is_empty() {
        let mut new_db = new_db;
        for user in &mut new_db.players {
            if let Some(new_language) = new_user_languages.get(&user.name) {
                user.language = Some(*new_language);
            }
        }
        new_db.write(out_db)?;
    }

    Ok(())
}

fn env_str(var: &str) -> Result<String> {
    dotenvy::var(var).map_err(|_| Error::MissingEnv(var.to_string()))
}

fn env<T: FromStr>(var: &str) -> Result<T>
where
    <T as FromStr>::Err: std::error::Error + Send + 'static,
{
    env_str(var)?
        .parse()
        .map_err(|e| Error::InvalidEnv(var.to_string(), Box::new(e)))
}

fn initialize_teams<'a>(
    tournament_context: &'a TournamentContext,
    old_players: &SwitzerlandPlayerMap,
    new_players: &mut SwitzerlandPlayerMap,
) -> Result<TeamsMap<'a>> {
    let mut rl = Editor::<TrieHinter, DefaultHistory>::new()?;
    let mut teams = HashMap::new();
    println!(
        "{}",
        Color::Green.paint(format!(
            "Found {} players. Please enter their seeding names as requested.",
            tournament_context.teams.len()
        ))
    );
    rl.set_helper(Some(TrieHinter {
        trie: Trie::from_iter(old_players.keys()),
        enabled: true,
    }));
    for team in &tournament_context.teams {
        let player = team.members.first().expect("Sendou team has no members");
        println!("Team:       {}", team.name);
        println!("IGN:        {}", player.in_game_name);
        println!("Sendou:     {}", player.username);
        println!(
            "Sendou URL: https://sendou.ink/u/{}",
            player
                .custom_url
                .as_ref()
                .unwrap_or(&player.discord_id.to_string())
        );
        loop {
            let seeding_name = rl.readline("seeding name> ")?;
            if seeding_name.is_empty() {
                println!("{}", Color::Red.paint("Please enter a name"));
                continue;
            }
            teams.insert(team.id, (team, seeding_name.clone()));
            new_players
                .entry(seeding_name.clone())
                .or_insert_with(|| SwitzerlandPlayer {
                    name: seeding_name,
                    ..Default::default()
                });
            break;
        }
        println!();
    }

    print_seeding_instructions(old_players, teams.values(), |team, player| {
        format!(
            "{} ({}) [{} @ {:.1} SP]",
            team.name,
            team.members.first().unwrap().username,
            player.name,
            player.rating.rating
        )
    });

    Ok(teams)
}

async fn wait_for_tournament_start(
    tournament_context: &TournamentContext,
    get_tournament: &impl GetTournamentFn,
) -> Result<()> {
    if let Ok(delay) = tournament_context
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
        if !get_tournament().await?.data.stages.is_empty() {
            break;
        }
        sleep(Duration::from_secs(30)).await;
    }

    Ok(())
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
    get_tournament: &impl GetTournamentFn,
    players: &mut SwitzerlandPlayerMap,
    teams: &TeamsMap<'_>,
) -> Result<DiscordChannelsMap> {
    println!("Creating Discord channels...");

    let mut channels = HashMap::new();

    let me_user = discord_http.cache().current_user();

    for team in get_tournament().await?.context.teams {
        if team.check_ins.is_empty() {
            continue;
        }
        let player = team.members.first().unwrap();

        let switzerland_player = players.get_mut(&teams.get(&team.id).unwrap().1).unwrap();
        let guess_language = switzerland_player.language.is_none();
        let language = switzerland_player.language.get_or_insert_with(|| {
            Language::guess_from_country(&player.country).unwrap_or_default()
        });
        let language_command =
            CommandIdDisplay(language.language_command_name(), language_command_id);

        let user = player.discord_id.to_user(discord_http).await?;
        let channel_name = format!("switzerland-{}", user.name.replace('.', ""));
        let channel = if let Some(channel) = guild_channels_by_name.remove(&channel_name) {
            channel
                .say(discord_http, language.bot_crashed(&language_command))
                .await?;
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
                                allow: Permissions::VIEW_CHANNEL
                                    | Permissions::SEND_MESSAGES
                                    | Permissions::USE_APPLICATION_COMMANDS,
                                deny: Permissions::empty(),
                                kind: PermissionOverwriteType::Member(user.id),
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
                    language.language_command_explanation(&language_command),
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
    original_players: &SwitzerlandPlayerMap,
    players: &mut SwitzerlandPlayerMap,
    teams: &TeamsMap<'_>,
    discord_user_languages: &DashMap<UserId, Language>,
    discord_channels: &DiscordChannelsMap,
    get_tournament: &impl GetTournamentFn,
) -> Result<()> {
    enum Action {
        Poll,
        SkipMatch(SendouId),
        Error(Error),
    }
    let (action_send, mut action_recv) = tokio::sync::mpsc::unbounded_channel();
    let mut rl = DefaultEditor::new()?;
    let mut printer = rl.create_external_printer()?;
    std::thread::spawn(move || {
        let mut run = || -> Result<()> {
            loop {
                let line = rl.readline("command> ")?;
                let action = if line == "help" || line == "?" {
                    println!("help");
                    println!("   Prints this message");
                    println!("?");
                    println!("   Prints this message");
                    println!("skip <match-id>");
                    println!("   Ignores the specified match");
                    println!("poll");
                    println!("   Forces a recheck of sendou.ink");
                    None
                } else if line.starts_with("skip ") {
                    line.strip_prefix("skip ").unwrap().parse().map_or_else(
                        |err| {
                            println!("Invalid match ID: {err}");
                            None
                        },
                        |id| {
                            println!("Ignoring match {id}");
                            Some(Action::SkipMatch(id))
                        },
                    )
                } else if line == "poll" {
                    println!("Polling now");
                    Some(Action::Poll)
                } else {
                    println!("Unknown or invalid command: {line}");
                    println!("Type 'help' or '?' to see a list of commands");
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
        if let Err(err) = run() {
            let _ = action_send.send(Action::Error(err));
        }
    });

    macro_rules! rl_print {
        () => { printer.print("".to_string()) };
        ($($args:tt)*) => { printer.print(format!($($args)*)) };
    }

    let mut completed_matches = HashSet::new();
    let mut ignored_matches = HashSet::new();
    let mut old_ranks = players
        .values()
        .filter_map(|x| Some((x.name.clone(), x.rank?.get())))
        .collect::<HashMap<_, _>>();
    let mut ranked_players = players
        .values()
        .map(|x| x.rating)
        .filter(|x| *x != const { Glicko2Rating::new() })
        .map(DescendingRatingGlicko2)
        .collect::<indexset::BTreeSet<_>>();

    let animation_generator = AsyncAnimationGenerator::new().await?;

    let new_players = loop {
        let tournament = get_tournament().await?;
        let swiss_round_ids = {
            let swiss_stage = tournament
                .data
                .stages
                .iter()
                .find(|x| matches!(x.settings, TournamentStageSettings::Swiss {}))
                .unwrap()
                .id;
            tournament
                .data
                .rounds
                .iter()
                .filter(|x| x.stage_id == swiss_stage)
                .sorted_by_key(|x| x.number)
                .map(|x| x.id)
                .collect_vec()
        };

        let mut new_players = players.clone();

        for tourney_match in tournament.data.matches {
            if ignored_matches.contains(&tourney_match.id) {
                continue;
            }

            let calc_index = swiss_round_ids
                .iter()
                .position(|x| *x == tourney_match.round_id);
            let get_player = |opponent: &Option<TournamentMatchOpponent>| {
                teams
                    .get(&opponent.unwrap().id.expect("Null opponent in ready match"))
                    .and_then(|(team, player_name)| {
                        let player = new_players.get(player_name)?;
                        Some((team, player_name, player.rating, player.language.unwrap()))
                    })
                    .unwrap()
            };
            if tourney_match.status == TournamentMatchStatus::Ready
                && (tourney_match.opponent1.is_none() || tourney_match.opponent2.is_none())
            {
                // BYE
                if !completed_matches.insert(tourney_match.id) {
                    continue;
                }
                let opponent = tourney_match.opponent1.or(tourney_match.opponent2);
                let (team, player, rating, language) = get_player(&opponent);
                let rank = old_ranks.get(player).copied();
                send_progress_message_to_player(
                    http_client,
                    http,
                    original_players,
                    discord_channels,
                    discord_user_languages,
                    &tournament.context,
                    &tourney_match,
                    &animation_generator,
                    team,
                    None,
                    calc_index,
                    swiss_round_ids.len(),
                    player,
                    opponent.unwrap(),
                    rating,
                    rating,
                    rank,
                    rank.unwrap_or_default(),
                    language,
                )?;
                continue;
            }
            if tourney_match.status != TournamentMatchStatus::Completed {
                completed_matches.remove(&tourney_match.id);
                continue;
            }
            let new_match = completed_matches.insert(tourney_match.id);
            let (team1, player1, rating1, language1) = get_player(&tourney_match.opponent1);
            let (team2, player2, rating2, language2) = get_player(&tourney_match.opponent2);
            let (new_rating1, new_rating2) = glicko2(
                &rating1,
                &rating2,
                &match tourney_match.opponent1.unwrap().result.unwrap() {
                    TournamentMatchResult::Win => Outcomes::WIN,
                    TournamentMatchResult::Loss => Outcomes::LOSS,
                },
                &Glicko2Config::default(),
            );
            if new_match {
                rl_print!("In match {}:", tourney_match.id)?;
            }
            let mut update_player = async |opponent: Option<TournamentMatchOpponent>,
                                           team: &TournamentTeam,
                                           other_team: &TournamentTeam,
                                           player,
                                           new_rating,
                                           language|
                   -> Result<()> {
                let player = new_players.get_mut(player).unwrap();
                let old_player = player.clone();
                player.rating = new_rating;

                if !new_match {
                    return Ok(());
                }

                ranked_players.remove(&DescendingRatingGlicko2(old_player.rating));
                let new_rank =
                    ranked_players.rank(&DescendingRatingGlicko2(player.rating)) as u32 + 1;
                ranked_players.insert(DescendingRatingGlicko2(player.rating));
                let old_rank = old_ranks.insert(player.name.clone(), new_rank);

                rl_print!(
                    "  {}",
                    format_player_simply(Some(&old_player), player, false)
                )?;
                send_progress_message_to_player(
                    http_client,
                    http,
                    original_players,
                    discord_channels,
                    discord_user_languages,
                    &tournament.context,
                    &tourney_match,
                    &animation_generator,
                    team,
                    Some(other_team),
                    calc_index,
                    swiss_round_ids.len(),
                    &player.name,
                    opponent.unwrap(),
                    old_player.rating,
                    player.rating,
                    old_rank,
                    new_rank,
                    language,
                )?;
                Ok(())
            };
            update_player(
                tourney_match.opponent1,
                team1,
                team2,
                player1,
                new_rating1,
                language1,
            )
            .await?;
            update_player(
                tourney_match.opponent2,
                team2,
                team1,
                player2,
                new_rating2,
                language2,
            )
            .await?;
        }

        if tournament.context.is_finalized {
            break new_players;
        }

        loop {
            let action = tokio::select! {
                _ = sleep(Duration::from_secs(30)) => Action::Poll,
                action = action_recv.recv() => action.expect("Action input thread exited unexpectedly without Error"),
            };
            match action {
                Action::Poll => break,
                Action::SkipMatch(id) => {
                    ignored_matches.insert(id);
                }
                Action::Error(err) => return Err(err),
            }
        }
    };

    *players = new_players;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn send_progress_message_to_player(
    http_client: &ReqwestClient,
    discord_http: &DiscordHttp,
    original_players: &SwitzerlandPlayerMap,
    discord_channels: &DiscordChannelsMap,
    discord_user_languages: &DashMap<UserId, Language>,
    tournament_context: &TournamentContext,
    tourney_match: &TournamentMatch,
    animation_generator: &AsyncAnimationGenerator,
    team: &TournamentTeam,
    other_team: Option<&TournamentTeam>,
    calc_index: Option<usize>,
    swiss_round_count: usize,
    player_name: &String,
    my_result: TournamentMatchOpponent,
    old_rating: Glicko2Rating,
    new_rating: Glicko2Rating,
    old_rank: Option<u32>,
    new_rank: u32,
    original_language: Language,
) -> Result<()> {
    let Some(discord_channel) = discord_channels.get(&team.id).copied() else {
        return Ok(());
    };

    let mut power_status = if let Some(calc_index) = calc_index
        && !original_players.contains_key(player_name)
    {
        if calc_index + 1 < swiss_round_count {
            PowerStatus::Calculating {
                progress: (calc_index + 1) as u32,
                total: swiss_round_count as u32,
            }
        } else {
            PowerStatus::Calculated {
                calculation_rounds: swiss_round_count as u32,
                power: new_rating.rating,
                rank: new_rank,
            }
        }
    } else if other_team.is_some() {
        // Don't show BYEs (after calcs)
        PowerStatus::SetPlayed {
            matches: Default::default(),
            old_power: old_rating.rating,
            new_power: new_rating.rating,
            old_rank: old_rank.unwrap(),
            new_rank,
        }
    } else {
        return Ok(());
    };

    let language = discord_user_languages
        .get(&team.members.first().unwrap().discord_id)
        .as_deref()
        .copied()
        .unwrap_or(original_language);

    // TODO: Translate these
    let message = other_team.map_or_else(
        || "BYE".to_string(),
        |team| {
            format!(
                "{} {}",
                match my_result.result.unwrap() {
                    TournamentMatchResult::Win => "VICTORY",
                    TournamentMatchResult::Loss => "DEFEAT",
                },
                format_link(
                    &format!("vs {}", team.members.first().unwrap().username),
                    &format!(
                        "<https://sendou.ink/to/{}/matches/{}>",
                        tournament_context.id, tourney_match.id
                    ),
                ),
            )
        },
    );

    let http_client = http_client.clone();
    let discord_http = discord_http.clone();
    let tourney_id = tournament_context.id;
    let set_id = tourney_match.id;
    let animation_generator = animation_generator.clone();
    let my_team_id = team.id;
    tokio::spawn(
        async move {
            if let PowerStatus::SetPlayed { matches, .. } = &mut power_status {
                let match_results = http_client
                    .get(format!(
                        "https://sendou.ink/to/{tourney_id}/matches/{set_id}?_data"
                    ))
                    .send()
                    .await?
                    .error_for_status()?
                    .json::<MatchRoot>()
                    .await?
                    .results;
                for (i, result) in match_results.into_iter().enumerate() {
                    matches[i] = if result.winner_team_id == my_team_id {
                        MatchOutcome::Win
                    } else {
                        MatchOutcome::Lose
                    };
                }
            }
            let animation = animation_generator
                .generate(power_status, language.into())
                .await?;
            discord_channel
                .send_message(
                    discord_http,
                    CreateMessage::new()
                        .content(message)
                        .add_file(CreateAttachment::bytes(
                            animation,
                            format!("set-{set_id}-{my_team_id}.webp"),
                        )),
                )
                .await?;
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
    if !body.chars().any(|x| is_emoji(x) || is_emoji_component(x)) {
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

async fn send_summary_to_discord(
    http: &DiscordHttp,
    guild: &Guild,
    moderator_channel: ChannelId,
    old_players: &SwitzerlandPlayerMap,
    teams: &TeamsMap<'_>,
    new_db: &Database,
    get_tournament: &impl GetTournamentFn,
) -> Result<()> {
    println!("\nSending comparison to Discord...");
    let tournament = get_tournament().await?;
    let player_name_to_discord_id = teams
        .values()
        .filter(|(team, _)| !team.check_ins.is_empty())
        .map(|(team, name)| (name, team.members.first().unwrap().discord_id))
        .collect::<HashMap<_, _>>();

    let mut players_in_discord = HashSet::new();
    for user_id in player_name_to_discord_id.values().copied() {
        if matches!(
            guild.member(http, user_id).await,
            Ok(_)
                | Err(serenity::Error::Http(
                    serenity::http::HttpError::UnsuccessfulRequest(serenity::http::ErrorResponse {
                        status_code: reqwest::StatusCode::NOT_FOUND,
                        ..
                    })
                ))
        ) {
            players_in_discord.insert(user_id);
        }
    }

    let mut message = String::new();
    let _ = writeln!(
        message,
        "And that concludes {}! Thank you all for participating, and I hope you had a good time.",
        tournament.context.name
    );

    let mut print_results = |title, results: &[SendouId; 3]| {
        let _ = writeln!(message, "## {title}");
        for (team_id, emoji) in results.iter().zip(['🥇', '🥈', '🥉']) {
            let player = tournament
                .context
                .teams
                .iter()
                .find(|x| x.id == *team_id)
                .unwrap()
                .members
                .first()
                .unwrap();
            let _ = writeln!(
                message,
                "- {emoji} {}{}",
                player.username,
                if players_in_discord.contains(&player.discord_id) {
                    format!(" ({})", player.discord_id.mention())
                } else {
                    "".to_string()
                },
            );
        }
    };
    match &compute_results(&tournament.data)[..] {
        [] => {}
        [(_, results)] => print_results("Results".to_string(), results),
        all_results => {
            for (bracket, results) in all_results {
                print_results(format!("{bracket} results"), results);
            }
        }
    }

    let _ = writeln!(message, "## Switzerland Power changes");
    for new_player in &new_db.players {
        let Some(discord_id) = player_name_to_discord_id.get(&new_player.name) else {
            continue;
        };
        if !players_in_discord.contains(discord_id) {
            continue;
        }
        let old_result = old_players.get(&new_player.name);
        if let Some(old_result) = old_result
            && old_result.rating == new_player.rating
        {
            continue;
        }
        let _ = writeln!(
            message,
            "- {} {}",
            discord_id.mention(),
            format_player_rank_summary(old_result, new_player, true)
        );
    }

    moderator_channel
        .send_message(http, CreateMessage::new().content(message))
        .await?;

    Ok(())
}

fn compute_results(tournament_data: &TournamentData) -> Vec<(&str, [SendouId; 3])> {
    let compute_results_for_stage = |stage_id| {
        let (main_group, third_place_group) = tournament_data
            .groups
            .iter()
            .filter(|x| x.stage_id == stage_id)
            .sorted_by_key(|x| x.number)
            .map(|x| x.id)
            .next_tuple()?;
        let finals_round = tournament_data
            .rounds
            .iter()
            .filter(|x| x.group_id == main_group)
            .max_by_key(|x| x.number)?
            .id;
        let third_place_round = tournament_data
            .rounds
            .iter()
            .find(|x| x.group_id == third_place_group)?
            .id;
        let finals_match = tournament_data
            .matches
            .iter()
            .find(|x| x.round_id == finals_round)?;
        let third_place_match = tournament_data
            .matches
            .iter()
            .find(|x| x.round_id == third_place_round)?;
        Some([
            itertools::chain(finals_match.opponent1, finals_match.opponent2)
                .find(|x| x.result == Some(TournamentMatchResult::Win))?
                .id
                .unwrap(),
            itertools::chain(finals_match.opponent1, finals_match.opponent2)
                .find(|x| x.result == Some(TournamentMatchResult::Loss))?
                .id
                .unwrap(),
            itertools::chain(third_place_match.opponent1, third_place_match.opponent2)
                .find(|x| x.result == Some(TournamentMatchResult::Win))?
                .id
                .unwrap(),
        ])
    };
    tournament_data
        .stages
        .iter()
        .filter(|x| matches!(x.settings, TournamentStageSettings::SingleElimination {}))
        .sorted_by_key(|x| x.number)
        .filter_map(|stage| Some((stage.name.as_str(), compute_results_for_stage(stage.id)?)))
        .collect()
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
