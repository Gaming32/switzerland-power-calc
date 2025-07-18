use crate::calc::{Database, SwitzerlandPlayer};
use crate::error::Result;
use crate::{print_player_simply, summarize_differences};
use ansi_term::{Color, Style};
use linked_hash_map::LinkedHashMap;
use rustyline::Completer;
use rustyline::hint::{Hint, Hinter};
use rustyline::history::DefaultHistory;
use rustyline::{Context, Editor, Helper, Highlighter, Validator};
use skillratings::glicko2::{glicko2, Glicko2Config, Glicko2Rating};
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::fs;
use std::path::Path;
use skillratings::Outcomes;
use trie_rs::Trie;

pub fn tourney_cli(in_db: &Path, out_db: &Path) -> Result<()> {
    if let Some(parent) = out_db.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut rl = Editor::<TrieHinter, DefaultHistory>::new()?;

    let old_players = Database::read(in_db)?
        .players
        .into_iter()
        .map(|x| (x.name.clone(), x))
        .collect::<LinkedHashMap<_, _>>();
    let mut new_players = old_players.clone();

    let mut teams = HashMap::<String, String>::new();

    println!("{}", Color::Green.paint("Enter team names and player names one after the other."));
    println!("{}", Color::Green.paint("Enter a blank team name when done."));
    rl.set_helper(Some(TrieHinter {
        trie: Trie::from_iter(old_players.keys()),
        enabled: false,
    }));
    loop {
        rl.helper_mut().unwrap().enabled = false;
        let team_name = rl.readline(" team > ")?;
        if team_name.is_empty() {
            break;
        }

        rl.helper_mut().unwrap().enabled = true;
        let player_name = rl.readline("player> ")?;

        match teams.entry(team_name) {
            Entry::Vacant(entry) => {
                entry.insert(player_name.clone());
                new_players
                    .entry(player_name.clone())
                    .or_insert_with(|| SwitzerlandPlayer {
                        name: player_name,
                        ..Default::default()
                    });
            }
            Entry::Occupied(entry) => {
                println!("Team name \"{}\" already taken", entry.key());
            }
        }
    }
    println!();

    println!("{}", Color::Green.paint("Enter the teams from each match, in order."));
    println!("{}", Color::Green.paint("After entering both teams, enter 1 or 2 to indicate the winner"));
    println!("{}", Color::Green.paint("Enter a blank team name when done."));
    rl.set_helper(Some(TrieHinter {
        trie: Trie::from_iter(teams.keys()),
        enabled: false,
    }));
    let mut temp_printing_player = SwitzerlandPlayer::default();
    loop {
        rl.helper_mut().unwrap().enabled = true;
        let mut get_player = |prompt| -> Result<Option<(&String, Glicko2Rating)>> {
            loop {
                let name = rl.readline(prompt)?;
                if name.is_empty() {
                    break Ok(None);
                }
                let Some(player_name) = teams.get(&name) else {
                    println!("{} {}", Color::Red.paint("Could not find a team named"), name);
                    continue;
                };
                break Ok(Some((player_name, new_players.get(player_name).expect("Player not found for team").rating)));
            }
        };
        let Some((player1_name, player1)) = get_player(" team 1> ")? else {
            break;
        };
        let Some((player2_name, player2)) = get_player(" team 2> ")? else {
            break;
        };

        rl.helper_mut().unwrap().enabled = false;
        let outcome = loop {
            let text = rl.readline("outcome> ")?;
            match text.as_str() {
                "1" => break Outcomes::WIN,
                "2" => break Outcomes::LOSS,
                _ => println!("{} {}", Color::Red.paint("Unknown outcome"), text),
            }
        };

        let (new_player1, new_player2) = glicko2(
            &player1,
            &player2,
            &outcome,
            &Glicko2Config::default()
        );

        let mut update_player = |player_name, new_rating| {
            let player = new_players.get_mut(player_name).unwrap();
            temp_printing_player.rating = player.rating;
            player.rating = new_rating;
            print_player_simply(Some(&temp_printing_player), player, false);
        };
        update_player(player1_name, new_player1);
        update_player(player2_name, new_player2);
    }
    println!();

    let mut new_db = Database::new();
    new_db
        .players
        .extend(new_players.into_iter().map(|(_, v)| v));
    new_db.sort();
    new_db.write(out_db)?;

    println!("SP comparison (switzerland-power-calc compare):");
    summarize_differences(&old_players, &new_db.players);

    Ok(())
}

#[derive(Completer, Helper, Validator, Highlighter)]
struct TrieHinter {
    trie: Trie<u8>,
    enabled: bool,
}

impl Hinter for TrieHinter {
    type Hint = FormattedHint;

    fn hint(&self, line: &str, pos: usize, ctx: &Context<'_>) -> Option<Self::Hint> {
        let _ = ctx;

        if !self.enabled || line.is_empty() || pos < line.len() {
            return None;
        }

        self.trie
            .postfix_search(line)
            .next()
            .map(FormattedHint::new)
    }
}

struct FormattedHint {
    text: String,
    formatted: String,
}

impl FormattedHint {
    fn new(text: String) -> Self {
        Self {
            formatted: Style::new().dimmed().paint(&text).to_string(),
            text,
        }
    }
}

impl Hint for FormattedHint {
    fn display(&self) -> &str {
        &self.formatted
    }

    fn completion(&self) -> Option<&str> {
        Some(&self.text)
    }
}
