mod schema;

use crate::cli_helpers::{TrieHinter, print_seeding_instructions};
use crate::db::{Database, SwitzerlandPlayer};
use crate::sendou::schema::TournamentRoot;
use crate::{Error, Result};
use ansi_term::Color;
use reqwest::Url;
use rustyline::Editor;
use rustyline::history::DefaultHistory;
use serenity::all::{Channel, ChannelId, ChannelType, HttpBuilder};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::str::FromStr;
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
    let chat_discord = chat_category.guild_id.to_partial_guild(&http).await?;

    let base_tournament = client
        .get(tournament_url)
        .send()
        .await?
        .json::<TournamentRoot>()
        .await?
        .tournament
        .context;

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

    // CreateMessage::new()
    //     .content("This is a test message")
    //     .execute(&http, (ChannelId::new(738850747071987794), None))
    //     .await
    //     .unwrap();

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
