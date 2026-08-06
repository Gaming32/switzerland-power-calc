use crate::{query_json, Result};
use crate::db::{Database, PlayerId};
use crate::sendou::schema::{GetUserIdsResponse, GetUserResponse};
use ansi_term::Color;
use itertools::Itertools;
use reqwest::Client;
use std::io;
use std::io::Write;
use std::path::Path;
use crate::sendou::utils::sendou_read_token_headers;

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

    let client = Client::builder()
        .default_headers(sendou_read_token_headers()?)
        .build()?;
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
                                user.name, user.id
                            );
                            break Some(user);
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
                (PlayerId::Sendou(sendou.id), Some(sendou.name))
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

async fn request_player_info(client: &Client, slug: &str) -> Result<GetUserResponse> {
    let ids: GetUserIdsResponse = query_json!(client, "/api/user/{}/ids", slug);
    Ok(query_json!(client, "/api/user/{}", ids.id))
}
