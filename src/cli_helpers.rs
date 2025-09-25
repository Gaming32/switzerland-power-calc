use crate::db::{SwitzerlandPlayer, SwitzerlandPlayerMap};
use ansi_term::{Color, Style};
use itertools::Itertools;
use rustyline::hint::{Hint, Hinter};
use rustyline::{Completer, Context, Helper, Highlighter, Validator};
use std::cmp::Ordering;
use trie_rs::Trie;

#[derive(Completer, Helper, Validator, Highlighter)]
pub struct TrieHinter {
    pub trie: Trie<u8>,
    pub enabled: bool,
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

pub struct FormattedHint {
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

pub trait IntoABRef<'a, A, B> {
    fn into_ab_ref(self) -> (&'a A, &'a B);
}

impl<'a, A, B> IntoABRef<'a, A, B> for &'a (A, B) {
    fn into_ab_ref(self) -> (&'a A, &'a B) {
        (&self.0, &self.1)
    }
}

impl<'a, A, B> IntoABRef<'a, A, B> for (&'a A, &'a B) {
    fn into_ab_ref(self) -> (&'a A, &'a B) {
        (self.0, self.1)
    }
}

pub fn print_seeding_instructions<'a, Team, Ref, Iter, Format>(
    old_players: &SwitzerlandPlayerMap,
    teams_iter: Iter,
    formatter: Format,
) where
    Team: 'a,
    Ref: IntoABRef<'a, Team, String>,
    Iter: IntoIterator<Item = Ref>,
    Format: Fn(&Team, &SwitzerlandPlayer) -> String,
{
    let sorted_teams = teams_iter
        .into_iter()
        .map(IntoABRef::into_ab_ref)
        .filter_map(|(team, name)| old_players.get(name).map(|x| (team, x)))
        .sorted_by(|(_, p1), (_, p2)| p1.descending_rating_order_cmp(p2))
        .collect_vec();
    if sorted_teams.is_empty() {
        return;
    }
    println!(
        "{}",
        Color::Green.paint("Follow the following seeding instructions:")
    );
    let print_ranks = |comparison: Ordering, message| {
        let mut ranks = sorted_teams
            .iter()
            .skip_while(|(_, player)| comparison.is_lt() && player.rating.rating >= 1500.0)
            .take_while(|(_, player)| comparison.is_lt() || player.rating.rating > 1500.0)
            .peekable();
        if ranks.peek().is_some() {
            println!("{}", Color::Cyan.paint(message));
            for (team, player) in ranks {
                println!("{}", formatter(team, player));
            }
            println!();
        }
    };
    print_ranks(Ordering::Greater, "Move these players to the top:");
    print_ranks(Ordering::Less, "Move these players to the bottom:");
}
