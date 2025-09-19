mod cli_helpers;
mod db;
mod error;
mod sendou;
mod tourney;

use crate::db::SwitzerlandPlayer;
use crate::sendou::sendou_cli;
use crate::tourney::tourney_cli;
use clap::Parser;
use error::{Error, Result};
use hashlink::LinkedHashMap;
use std::cmp::Ordering;
use std::path::PathBuf;
use std::process::exit;

#[derive(clap::Parser, Debug)]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(clap::Subcommand, Debug)]
enum Commands {
    /// Initialize the power database
    Init {
        /// The path to the database file to create
        db: PathBuf,
    },
    /// Query the database
    Query {
        /// The path to the database
        db: PathBuf,
        /// The users to query. If none specified, query all
        query: Option<Vec<String>>,
        /// Output powers to more decimal places, as well as outputting the deviation and volatility
        #[arg(short, long)]
        verbose: bool,
    },
    /// Summarizes the differences between databases
    Compare {
        /// The path to the old database
        old_db: PathBuf,
        /// The path to the new database
        new_db: PathBuf,
        /// The users to query. If none specified, query all
        query: Option<Vec<String>>,
    },
    /// Run a tournament
    Tourney {
        /// The path to the input database
        in_db: PathBuf,
        /// The path to the database to create as a result
        out_db: PathBuf,
    },
    /// Automatically process a tournament being run on sendou.ink
    Sendou {
        /// The path to the input database
        in_db: PathBuf,
        /// The path to the database to create as a result
        out_db: PathBuf,
        /// The URL to the tournament on sendou.ink
        tournament_url: String,
    },
}

fn main() {
    let args = Args::parse();
    if let Err(e) = run(args) {
        eprintln!("{e}");
        exit(1);
    }
}

fn run(args: Args) -> Result<()> {
    use Commands::*;
    match args.command {
        Init { db } => {
            db::init_db(&db)?;
            println!("Initialized DB at {}", db.display());
        }
        Query { db, query, verbose } => {
            let results = db::query(&db, query.as_ref())?;
            println!("Found {} players:", results.len());
            for player in results {
                if !verbose {
                    print_player_simply(None, &player, true);
                } else {
                    println!(
                        "  #{} {}: {:?}",
                        player.unwrap_rank(),
                        player.name,
                        player.rating
                    );
                }
            }
        }
        Compare {
            old_db,
            new_db,
            query,
        } => {
            let old_results = db::query(&old_db, None)?
                .into_iter()
                .map(|x| (x.name.clone(), x))
                .collect::<LinkedHashMap<_, _>>();
            let new_results = db::query(&new_db, query.as_ref())?;
            println!("Found {} players:", new_results.len());
            summarize_differences(&old_results, &new_results);
        }
        Tourney { in_db, out_db } => tourney_cli(&in_db, &out_db)?,
        Sendou {
            in_db,
            out_db,
            tournament_url,
        } => tokio::runtime::Runtime::new()?.block_on(sendou_cli(
            &in_db,
            &out_db,
            &tournament_url,
        ))?,
    }
    Ok(())
}

pub fn summarize_differences(
    old_results: &LinkedHashMap<String, SwitzerlandPlayer>,
    new_results: &Vec<SwitzerlandPlayer>,
) {
    for new_player in new_results {
        let old_result = old_results.get(&new_player.name);
        if let Some(old_result) = old_result
            && old_result.rating == new_player.rating
        {
            continue;
        }
        print_player_simply(old_result, new_player, true);
    }
}

pub fn print_player_simply(
    old_player: Option<&SwitzerlandPlayer>,
    new_player: &SwitzerlandPlayer,
    show_rank: bool,
) {
    println!("{}", format_player_simply(old_player, new_player, show_rank));
}

pub fn format_player_simply(
    old_player: Option<&SwitzerlandPlayer>,
    new_player: &SwitzerlandPlayer,
    show_rank: bool,
) -> String {
    format!(
        "- {}: {}",
        new_player.name,
        format_player_rank_summary(old_player, new_player, show_rank)
    )
}

pub fn format_player_rank_summary(
    old_player: Option<&SwitzerlandPlayer>,
    new_player: &SwitzerlandPlayer,
    show_rank: bool,
) -> String {
    if let Some(old_player) = old_player {
        format!(
            "{:.1} SP → {:.1} SP ({:+.1}){}",
            old_player.rating.rating,
            new_player.rating.rating,
            new_player.rating.rating - old_player.rating.rating,
            if show_rank {
                let old_rank = old_player.unwrap_rank();
                let new_rank = new_player.unwrap_rank();
                format!("; {}", format_rank_difference(old_rank, new_rank))
            } else {
                "".to_string()
            }
        )
    } else {
        format!(
            "{:.1} SP{}",
            new_player.rating.rating,
            if show_rank {
                format!("; #{}", new_player.unwrap_rank())
            } else {
                "".to_string()
            }
        )
    }
}

pub fn format_rank_difference(old_rank: u32, new_rank: u32) -> String {
    match new_rank.cmp(&old_rank) {
        Ordering::Equal => format!("#{new_rank} ⇒"),
        Ordering::Less => format!("#{old_rank} → #{new_rank} ⇑"),
        Ordering::Greater => format!("#{old_rank} → #{new_rank} ⇓"),
    }
}
