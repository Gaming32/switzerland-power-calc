use crate::db::{PlayerId, SwitzerlandPlayer, SwitzerlandPlayerMap};
use ansi_term::Color;
use itertools::Itertools;
use std::cmp::Ordering;

pub fn print_seeding_instructions<'a, Team, Iter, Format>(
    players: &SwitzerlandPlayerMap,
    teams_iter: Iter,
    formatter: Format,
) -> Vec<(&'a Team, &SwitzerlandPlayer)>
where
    Team: 'a,
    Iter: IntoIterator<Item = (&'a Team, PlayerId)>,
    Format: Fn(&Team, &SwitzerlandPlayer) -> String,
{
    let sorted_teams = teams_iter
        .into_iter()
        .filter_map(|(team, name)| players.get(&name).map(|x| (team, x)))
        .filter(|(_, p)| p.rating.rating != 1500.0)
        .sorted_by(|(_, p1), (_, p2)| p1.descending_rating_order_cmp(p2))
        .collect_vec();
    if sorted_teams.is_empty() {
        return sorted_teams;
    }
    println!(
        "{}",
        Color::Green.paint("The following seeding instructions are being applied:")
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
    print_ranks(Ordering::Greater, "These players will be moved to the top:");
    print_ranks(Ordering::Less, "These players will be moved to the bottom:");
    sorted_teams
}
