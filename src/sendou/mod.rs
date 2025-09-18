mod schema;

use crate::cli_helpers::{TrieHinter, print_seeding_instructions};
use crate::db::{Database, SwitzerlandPlayer};
use crate::sendou::schema::{TournamentRoot, TournamentStageSettings};
use crate::{Error, Result, format_player_rank_summary, summarize_differences};
use ansi_term::Color;
use chrono::Utc;
use reqwest::Url;
use rustyline::Editor;
use rustyline::history::DefaultHistory;
use serenity::all::{
    Builder, Channel, ChannelId, ChannelType, CreateMessage, HttpBuilder, Mentionable,
};
use std::collections::HashMap;
use std::fmt::Write;
use std::fs;
use std::path::Path;
use std::str::FromStr;
use std::time::Duration;
use tokio::time::sleep;
use trie_rs::Trie;

pub async fn sendou_cli(in_db: &Path, out_db: &Path, tournament_url: &str) -> Result<()> {
    if let Some(parent) = out_db.parent() {
        fs::create_dir_all(parent)?;
    }

    let tournament_url = {
        let mut url = Url::parse(tournament_url)?;
        url.set_query(Some("_data=features/tournament/routes/to.$id"));
        url
    };
    let client = reqwest::ClientBuilder::new()
        .user_agent(concat!(
            env!("CARGO_PKG_NAME"),
            " (https://github.com/Gaming32/switzerland-power-calc, ",
            env!("CARGO_PKG_VERSION"),
            ")"
        ))
        .build()?;

    let http = HttpBuilder::new(env_str("DISCORD_BOT_TOKEN")?)
        .client(client.clone())
        .build();

    let chat_category = match env::<ChannelId>("DISCORD_CHAT_CATEGORY_ID")?
        .to_channel(&http)
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
    let chat_guild = chat_category.guild_id.to_partial_guild(&http).await?;

    let moderator_channel = env::<ChannelId>("DISCORD_MODERATOR_CHANNEL_ID")?;

    let get_tournament = async || -> Result<_> {
        Ok(client
            .get(tournament_url.clone())
            .send()
            .await?
            .json::<TournamentRoot>()
            .await?
            .tournament)
    };
    let base_tournament = get_tournament().await?.context;

    let mut rl = Editor::<TrieHinter, DefaultHistory>::new()?;
    let old_players = Database::read(in_db)?.into_map();
    let mut new_players = old_players.clone();

    let mut teams = HashMap::new();
    println!(
        "{}",
        Color::Green.paint(format!(
            "Found {} players. Please enter their seeding names as requested.",
            base_tournament.teams.len()
        ))
    );
    rl.set_helper(Some(TrieHinter {
        trie: Trie::from_iter(old_players.keys()),
        enabled: true,
    }));
    for team in &base_tournament.teams {
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

    print_seeding_instructions(&old_players, teams.values(), |team, player| {
        format!(
            "{} ({}) [{} @ {:.1} SP]",
            team.name,
            team.members.first().unwrap().username,
            player.name,
            player.rating.rating
        )
    });

    if let Ok(delay) = base_tournament
        .start_time
        .signed_duration_since(Utc::now())
        .to_std()
    {
        sleep(delay).await;
    }

    let round_count = loop {
        let round_count = get_tournament()
            .await?
            .data
            .stages
            .iter()
            .filter_map(|x| {
                if let TournamentStageSettings::Swiss { swiss } = &x.settings {
                    Some(swiss.round_count)
                } else {
                    None
                }
            })
            .next();
        if let Some(round_count) = round_count {
            break round_count;
        }
        sleep(Duration::from_secs(30)).await;
    };

    // TODO: Main part

    let new_db = Database::new_from_map(new_players);
    new_db.write(out_db)?;

    println!("SP comparison (switzerland-power-calc compare):");
    summarize_differences(&old_players, &new_db.players);

    println!("\nSending comparison to Discord...");
    {
        let mut message = "## Switzerland Power changes\n".to_string();
        let player_name_to_discord_id = teams
            .into_values()
            .map(|(team, name)| (name, team.members.first().unwrap().discord_id))
            .collect::<HashMap<_, _>>();
        for new_player in &new_db.players {
            let Some(discord_id) = player_name_to_discord_id.get(&new_player.name) else {
                continue;
            };
            let old_result = old_players.get(&new_player.name);
            if let Some(old_result) = old_result
                && old_result.rating == new_player.rating
            {
                continue;
            }
            let Ok(member) = chat_guild.member(&http, discord_id).await else {
                continue;
            };
            let _ = writeln!(
                message,
                "- {} {}",
                member.mention(),
                format_player_rank_summary(old_result, new_player, true)
            );
        }
        message.truncate(message.len() - 1); // Remove trailing \n
        CreateMessage::new()
            .content(message)
            .execute(&http, (moderator_channel, None))
            .await?;
    }
    println!("Sent!");

    Ok(())
}

fn env_str(var: &str) -> Result<String> {
    dotenvy::var(var).map_err(|_| Error::MissingEnv(var.to_string()))
}

fn env<T: FromStr>(var: &str) -> Result<T>
where
    <T as FromStr>::Err: std::error::Error + 'static,
{
    env_str(var)?
        .parse()
        .map_err(|e| Error::InvalidEnv(var.to_string(), Box::new(e)))
}
