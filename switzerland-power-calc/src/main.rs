mod cli_helpers;
mod db;
mod error;
mod lang;
mod sendou;
mod tourney;

use crate::db::SwitzerlandPlayer;
use crate::sendou::sendou_cli;
use crate::tourney::tourney_cli;
use clap::Parser;
use error::{Error, Result};
use hashlink::LinkedHashMap;
use itertools::Itertools;
use std::cmp::Ordering;
use std::fs;
use std::path::PathBuf;
use std::process::exit;
use switzerland_power_animated::{
    AnimationGenerator, AnimationLanguage, MatchOutcome, PowerStatus,
};
use tracing_subscriber::EnvFilter;
use tracing_subscriber::filter::LevelFilter;

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
    /// Generate an animation
    Animation {
        /// The WebP quality to use for the animation between 0 and 100. For lossy, 0 gives the
        /// smallest size and 100 the largest. For lossless, this parameter is the amount of effort
        /// put into the compression: 0 is the fastest but gives larger files compared to the
        /// slowest, but best, 100.
        #[arg(short, long, default_value = "50", value_parser = clap::value_parser!(u8).range(..=100))]
        quality: u8,
        /// Use lossless encoding
        #[arg(long)]
        lossless: bool,
        #[arg(short, long, default_value = "USen")]
        lang: AnimationLanguage,
        /// The path to output the animation to
        output_path: PathBuf,
        /// The animation to generate
        #[command(subcommand)]
        animation: ParsedPowerStatus,
    },
}

#[derive(clap::Subcommand, Debug)]
enum ParsedPowerStatus {
    /// Generate a progress calculating animation
    Calculating {
        /// The new progress through the calculations
        #[arg(value_parser = clap::value_parser!(u8).range(1..))]
        progress: u8,
        /// The total length of the calculations
        #[arg(value_parser = clap::value_parser!(u8).range(3..))]
        total: u8,
    },
    /// Generate a calculation complete animation
    Calculated {
        /// The total length of the calculations
        #[arg(value_parser = clap::value_parser!(u8).range(3..))]
        round_count: u8,
        /// The calculated Switzerland Power
        power: f64,
        /// The estimated player rank
        #[arg(value_parser = clap::value_parser!(u32).range(1..))]
        rank: u32,
    },
    /// Generate a set played complete animation
    SetPlayed {
        /// The previous Switzerland Power
        old_power: f64,
        /// The updated Switzerland Power
        new_power: f64,
        /// The previous estimated player rank
        #[arg(value_parser = clap::value_parser!(u32).range(1..))]
        old_rank: u32,
        /// The new estimated player rank
        #[arg(value_parser = clap::value_parser!(u32).range(1..))]
        new_rank: u32,
        /// The matches that are won or lost
        #[arg(value_enum, num_args(2..=5))]
        matches: Vec<ParsedMatchOutcome>,
    },
}

impl From<ParsedPowerStatus> for PowerStatus {
    fn from(val: ParsedPowerStatus) -> Self {
        match val {
            ParsedPowerStatus::Calculating { progress, total } => PowerStatus::Calculating {
                progress: progress as u32,
                total: total as u32,
            },
            ParsedPowerStatus::Calculated {
                round_count,
                power,
                rank,
            } => PowerStatus::Calculated {
                calculation_rounds: round_count as u32,
                power,
                rank,
            },
            ParsedPowerStatus::SetPlayed {
                old_power,
                new_power,
                old_rank,
                new_rank,
                matches,
            } => PowerStatus::SetPlayed {
                matches: matches
                    .into_iter()
                    .map_into::<MatchOutcome>()
                    .pad_using(5, |_| MatchOutcome::Unplayed)
                    .collect_array()
                    .unwrap(),
                old_power,
                new_power,
                old_rank,
                new_rank,
            },
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, clap::ValueEnum)]
enum ParsedMatchOutcome {
    Win,
    Lose,
}

impl From<ParsedMatchOutcome> for MatchOutcome {
    fn from(val: ParsedMatchOutcome) -> Self {
        match val {
            ParsedMatchOutcome::Win => MatchOutcome::Win,
            ParsedMatchOutcome::Lose => MatchOutcome::Lose,
        }
    }
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
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env()?,
        )
        .init();
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
        } => sendou_cli(&in_db, &out_db, &tournament_url)?,
        Animation {
            quality,
            lossless,
            lang,
            output_path,
            animation,
        } => {
            let mut animation_generator = AnimationGenerator::new()?;
            {
                let config = animation_generator.webp_config_mut();
                config.quality = quality as f32;
                config.lossless = lossless as core::ffi::c_int;
            }

            println!("Generating animation...");
            let animation = animation_generator.generate(animation.into(), lang)?;

            println!("Saving animation...");
            if let Some(parent) = output_path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(&output_path, &*animation)?;

            println!("Saved animation to {}", output_path.display());
        }
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
    println!(
        "{}",
        format_player_simply(old_player, new_player, show_rank)
    );
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
