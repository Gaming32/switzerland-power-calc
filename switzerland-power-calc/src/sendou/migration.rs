use crate::Result;
use crate::db::{Database, PlayerId};
use crate::sendou::schema::SendouUserRoot;
use ansi_term::Color;
use itertools::Itertools;
use reqwest::Client;
use std::io;
use std::path::Path;

#[tokio::main]
pub async fn sendou_migration_cli(
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
        "Found {} players with legacy IDs. Please enter their Sendou slug, or enter a blank line if you don't know it.",
        queried_players.len(),
    )));

    let client = Client::new();
    let mut players_map = db.clone().into_map();
    let mut player_name = String::new();

    for player in queried_players {
        let PlayerId::LegacyName(legacy_name) = &player.id else {
            unreachable!();
        };
        println!("\nLegacy player ID: {}", legacy_name);
        let sendou = loop {
            player_name.clear();
            print!("sendou slug> ");
            io::stdin().read_line(&mut player_name)?;
            let player_slug = player_name.trim();
            if player_slug.is_empty() {
                break None;
            }
            match request_player_info(&client, player_slug).await {
                Ok(user) => break Some(user.user),
                Err(e) => println!(
                    "{}",
                    Color::Red.paint(format!("Couldn't find player {player_slug}: {e}"))
                ),
            }
        };
        let Some(sendou) = sendou else {
            continue;
        };
        let mut real_player = players_map.remove(&player.id).unwrap();
        real_player.id = PlayerId::Sendou(sendou.id);
        real_player.display_name = Some(sendou.username);
        players_map.insert(real_player.id.clone(), real_player);
    }

    Database::new_from_map(players_map).write(out_db)?;
    Ok(())
}

async fn request_player_info(client: &Client, slug: &str) -> Result<SendouUserRoot> {
    Ok(client
        .get(format!("https://sendou.ink/u/{slug}?_data"))
        .send()
        .await?
        .json::<SendouUserRoot>()
        .await?)
}
