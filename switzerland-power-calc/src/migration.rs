use crate::Result;
use crate::db::{Database, PlayerId};
use crate::sendou::schema::SendouUserRoot;
use crate::sendou::turbo_stream::TurboStreamed;
use ansi_term::Color;
use itertools::Itertools;
use reqwest::Client;
use std::io;
use std::io::Write;
use std::path::Path;

#[derive(clap::ValueEnum, Copy, Clone, Debug)]
pub enum MigrationStyle {
    ToSendou,
    ChangeName,
}

#[tokio::main]
pub async fn migration_cli(
    migration_style: MigrationStyle,
    in_db: &Path,
    out_db: &Path,
    query: Option<&Vec<String>>,
) -> Result<()> {
    let db = Database::read(in_db)?;
    let queried_players = db
        .clone()
        .query(query, false)
        .into_iter()
        .filter(|x| matches!(x.id, PlayerId::LegacyName(_)))
        .collect_vec();
    if queried_players.is_empty() {
        println!("No players found!");
        return Ok(());
    }
    println!("{}", Color::Green.paint(format!(
        "Found {} players with legacy IDs. Please enter their {}, or enter a blank line if you don't know it.",
        queried_players.len(),
        match migration_style {
            MigrationStyle::ToSendou => "Sendou slug",
            MigrationStyle::ChangeName => "new ID/IGN",
        }
    )));

    let client = Client::new();
    let mut players_map = db.clone().into_map();
    let mut player_name = String::new();

    for player in queried_players {
        let PlayerId::LegacyName(legacy_name) = &player.id else {
            unreachable!();
        };
        println!();
        let (new_id, new_display_name) = match migration_style {
            MigrationStyle::ToSendou => {
                println!("Legacy player ID: {}", legacy_name);
                let sendou = loop {
                    player_name.clear();
                    print!("sendou slug> ");
                    io::stdout().flush()?;
                    io::stdin().read_line(&mut player_name)?;
                    let player_slug = player_name.trim();
                    if player_slug.is_empty() {
                        break None;
                    }
                    match request_player_info(&client, player_slug).await {
                        Ok(user) => {
                            println!(
                                "Found player '{}' with ID {}",
                                user.user.username, user.user.id
                            );
                            break Some(user.user);
                        }
                        Err(e) => println!(
                            "{}",
                            Color::Red.paint(format!("Couldn't find player {player_slug}: {e}"))
                        ),
                    }
                };
                let Some(sendou) = sendou else {
                    continue;
                };
                (PlayerId::Sendou(sendou.id), Some(sendou.username))
            }
            MigrationStyle::ChangeName => {
                println!("Current player ID: {}", legacy_name);
                player_name.clear();
                print!("new name> ");
                io::stdout().flush()?;
                io::stdin().read_line(&mut player_name)?;
                let real_name = player_name.trim_end();
                if real_name.is_empty() {
                    continue;
                }
                (PlayerId::LegacyName(real_name.to_string()), None)
            }
        };
        let mut real_player = players_map.remove(&player.id).unwrap();
        real_player.id = new_id;
        real_player.display_name = new_display_name;
        players_map.insert(real_player.id.clone(), real_player);
    }

    Database::new_from_map(players_map).write(out_db)?;
    Ok(())
}

async fn request_player_info(client: &Client, slug: &str) -> Result<SendouUserRoot> {
    Ok(client
        .get(format!("https://sendou.ink/u/{slug}.data"))
        .send()
        .await?
        .json::<TurboStreamed<SendouUserRoot>>()
        .await?
        .0)
}
