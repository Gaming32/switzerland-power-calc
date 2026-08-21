use crate::db::{PlayerId, SwitzerlandPlayer, SwitzerlandPlayerMap};
use crate::error;
use crate::error::ErrorKind;
use ansi_term::Color;
use itertools::Itertools;
use reqwest::header;
use reqwest::header::{HeaderMap, HeaderValue};
use std::cmp::Ordering;
use std::str::FromStr;

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

#[macro_export]
macro_rules! query_json {
    ($client:ident, $route:literal, $($param:expr),+ $(,)?) => {
        $client.get(format!(concat!("https://sendou.ink", $route), $($param),+))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?
    };
}

pub fn sendou_read_token_headers() -> error::Result<HeaderMap> {
    let mut bearer = HeaderValue::from_str(&format!("Bearer {}", env_str("SENDOU_READ_TOKEN")?))?;
    bearer.set_sensitive(true);

    let mut headers = HeaderMap::new();
    headers.insert(header::AUTHORIZATION, bearer);

    Ok(headers)
}

pub fn env_str(var: &str) -> error::Result<String> {
    dotenvy::var(var).map_err(|_| ErrorKind::MissingEnv(var.to_string()).into())
}

pub fn env<T: FromStr>(var: &str) -> error::Result<T>
where
    <T as FromStr>::Err: std::error::Error + Send + 'static,
{
    env_str(var)?
        .parse()
        .map_err(|e| ErrorKind::InvalidEnv(var.to_string(), Box::new(e)).into())
}
