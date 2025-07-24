mod db;
mod error;
mod season;
mod tourney;

use crate::db::{SeasonState, SwitzerlandPlayer};
use crate::season::new_season;
use crate::tourney::tourney_cli;
use clap::Parser;
use error::Result;
use linked_hash_map::LinkedHashMap;
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
        /// Include players that have not participated in the current season
        #[arg(short, long)]
        include_non_participated: bool,
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
    /// Start a new season, creating a new database
    NewSeason {
        /// The path to the input database
        in_db: PathBuf,
        /// The path to the database to create as a result
        out_db: PathBuf,
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
        Query {
            db,
            query,
            verbose,
            include_non_participated,
        } => {
            let results = db::query(&db, query.as_ref(), include_non_participated)?;
            println!("Found {} players:", results.len());
            for player in results {
                if !verbose {
                    print_player_simply(None, &player, true);
                } else {
                    println!(
                        "  #{} {}: {:?}",
                        if let SeasonState::Participated(rank) = player.season {
                            format!("{}", rank.unwrap())
                        } else {
                            "?".to_string()
                        },
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
            let old_results = db::query(&old_db, None, false)?
                .into_iter()
                .map(|x| (x.name.clone(), x))
                .collect::<LinkedHashMap<_, _>>();
            let new_results = db::query(&new_db, query.as_ref(), false)?;
            println!("Found {} players:", new_results.len());
            summarize_differences(&old_results, &new_results);
        }
        Tourney { in_db, out_db } => tourney_cli(&in_db, &out_db)?,
        NewSeason { in_db, out_db } => new_season(&in_db, &out_db)?,
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
        print_player_simply(
            old_result.filter(|x| matches!(x.season, SeasonState::Participated(..))),
            new_player,
            true,
        );
    }
}

fn print_player_simply(
    old_player: Option<&SwitzerlandPlayer>,
    new_player: &SwitzerlandPlayer,
    show_rank: bool,
) {
    if let Some(old_player) = old_player {
        println!(
            "- {}: {:.1} SP -> {:.1} SP ({:+.1}){}",
            new_player.name,
            old_player.rating.rating,
            new_player.rating.rating,
            new_player.rating.rating - old_player.rating.rating,
            if show_rank {
                // If we're doing a comparison, there's always a rank
                let old_rank = old_player.season.unwrap_rank();
                let new_rank = new_player.season.unwrap_rank();
                match new_rank.cmp(&old_rank) {
                    Ordering::Equal => format!("; #{new_rank} =>"),
                    Ordering::Less => format!("; #{old_rank} -> #{new_rank} ⇑"),
                    Ordering::Greater => format!("; #{old_rank} -> #{new_rank} ⇓"),
                }
            } else {
                "".to_string()
            }
        );
    } else {
        println!(
            "- {}: {:.1} SP{}",
            new_player.name,
            new_player.rating.rating,
            if show_rank && let SeasonState::Participated(rank) = new_player.season {
                format!("; #{}", rank.unwrap())
            } else {
                "".to_string()
            }
        );
    }
}
