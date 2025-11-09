use crate::db::{Database, PlayerId, SwitzerlandPlayerMap};
use crate::sendou::{env_str, format_link};
use serenity::all::{Mentionable, UserId};
use std::cmp::Ordering;
use std::collections::HashMap;

pub fn generate_leaderboard_messages(
    old_players: &SwitzerlandPlayerMap,
    new_db: &Database,
    player_id_to_discord_id: &HashMap<PlayerId, UserId>,
    max_message_len: usize,
) -> Vec<String> {
    let mut messages = Vec::new();
    let mut message = String::from("# Switzerland Top 50");

    let get_arrow = |var, default: &str| {
        let mut arrow = env_str(var).unwrap_or_else(|_| default.to_string());
        arrow.push(' ');
        arrow
    };
    let up_arrow = get_arrow("DISCORD_UP_ARROW", "⇑");
    let down_arrow = get_arrow("DISCORD_DOWN_ARROW", "⇓");
    let right_arrow = get_arrow("DISCORD_RIGHT_ARROW", "⇒");

    for (index, player) in new_db.players.iter().take(50).enumerate() {
        let old_player = old_players.get(&player.id);
        let line = format!(
            "{}. {}{}{} with {:.1} SP",
            index + 1,
            match old_player
                .filter(|x| player.rating != x.rating) // Don't show an arrow if they didn't play
                .map(|x| player.unwrap_rank().cmp(&x.unwrap_rank()))
                .or_else(|| old_player.is_none().then_some(Ordering::Less)) // Treat new players as if they went up in the ranks
            {
                Some(Ordering::Equal) => &right_arrow,
                Some(Ordering::Less) => &up_arrow,
                Some(Ordering::Greater) => &down_arrow,
                None => "",
            },
            match &player.id {
                PlayerId::Sendou(id) => format_link(
                    player.display_name.as_deref().unwrap(),
                    &format!("<https://sendou.ink/u/{id}>")
                ),
                PlayerId::LegacyName(name) => name.clone(),
            },
            player_id_to_discord_id
                .get(&player.id)
                .map(|x| format!(" ({})", x.mention()))
                .unwrap_or_default(),
            player.rating.rating,
        );
        if message.len() + line.len() >= max_message_len {
            messages.push(message.clone());
            message.clear();
        } else {
            if !message.is_empty() {
                message.push('\n');
            }
            message.push_str(&line);
        }
    }

    messages.push(message);
    messages
}
