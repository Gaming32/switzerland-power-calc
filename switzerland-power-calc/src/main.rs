mod db;
mod error;
mod migration;
mod sendou;

use crate::db::{Database, SwitzerlandPlayer, SwitzerlandPlayerMap};
use crate::migration::MigrationStyle;
use crate::sendou::leaderboard::generate_leaderboard_messages;
use crate::sendou::{SendouId, migration_cli, sendou_cli};
use clap::Parser;
use error::{Error, Result};
use hashlink::LinkedHashMap;
use itertools::Itertools;
use std::backtrace::BacktraceStatus;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::fs;
use std::num::{NonZeroU32, NonZeroUsize};
use std::path::{Path, PathBuf};
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
    /// Automatically process a tournament being run on sendou.ink
    Sendou {
        /// The path to the input database
        in_db: PathBuf,
        /// The path to the database to create as a result
        out_db: PathBuf,
        /// The URL to the tournament on sendou.ink
        tournament_id: SendouId,
    },
    /// Migrate old string IDs to new Sendou-based IDs or to other names
    MigrateNames {
        /// The style of migration to perform
        style: MigrationStyle,
        /// The path to the input database
        in_db: PathBuf,
        /// The path to the database to create as a result
        out_db: PathBuf,
        /// The users to migrate. If none specified, query all
        query: Option<Vec<String>>,
    },
    /// Generate an animation
    Animate {
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
    /// Generate a leaderboard message for display in Discord
    Leaderboard {
        /// The path to an old database to compare with
        #[arg(short, long)]
        comparison: Option<PathBuf>,
        /// Splits the leaderboard into parts so it doesn't go over this length.
        #[arg(short, long)]
        max_message_length: Option<NonZeroUsize>,
        /// The path to the database
        db: PathBuf,
    },
    /// Hide the rank of a player in all public places (including animations).
    HideRank {
        /// The path to the input database
        in_db: PathBuf,
        /// The path to the database to create as a result
        out_db: PathBuf,
        /// The player whose rank to hide
        player: String,
    },
    /// Unhide the rank of a player in all public places (including animations).
    UnhideRank {
        /// The path to the input database
        in_db: PathBuf,
        /// The path to the database to create as a result
        out_db: PathBuf,
        /// The player whose rank to unhide
        player: String,
    },
}

#[derive(clap::Subcommand, Debug)]
#[clap(flatten_help = true)]
enum ParsedPowerStatus {
    /// Generate a progress calculating animation
    #[clap(disable_help_flag = true)]
    Calculating {
        /// The new progress through the calculations
        #[arg(value_parser = clap::value_parser!(u8).range(1..))]
        progress: u8,
        /// The total length of the calculations
        #[arg(value_parser = clap::value_parser!(u8).range(3..))]
        total: u8,
    },
    /// Generate a calculation complete animation
    #[clap(disable_help_flag = true)]
    Calculated {
        /// The total length of the calculations
        #[arg(value_parser = clap::value_parser!(u8).range(3..))]
        round_count: u8,
        /// The calculated Switzerland Power
        power: f64,
        /// The estimated player rank
        #[arg(value_parser = clap::value_parser!(u32).range(1..))]
        rank: Option<u32>,
    },
    /// Generate a set played complete animation
    #[clap(disable_help_flag = true)]
    SetPlayed {
        /// The previous Switzerland Power
        old_power: f64,
        /// The updated Switzerland Power
        new_power: f64,
        /// The previous estimated player rank
        #[arg(short, long, num_args = 2, value_parser = clap::value_parser!(u32).range(1..))]
        rank_change: Option<Vec<u32>>,
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
                rank_change,
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
                rank_change: rank_change.map(|x| (x[0], x[1])),
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
        if e.backtrace.status() == BacktraceStatus::Captured {
            eprintln!("{}", e.backtrace);
        }
        exit(1);
    }
}

fn run(args: Args) -> Result<()> {
    use Commands::*;
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::WARN.into())
                .from_env()?,
        )
        .init();
    match args.command {
        Init { db } => {
            db::init_db(&db)?;
            println!("Initialized DB at {}", db.display());
        }
        Query { db, query, verbose } => {
            let results = db::query(&db, query.as_ref(), true)?;
            println!("Found {} players:", results.len());
            for player in results {
                if !verbose {
                    print_player_simply(None, &player, true);
                } else {
                    println!(
                        "  #{} {}: {:?}",
                        player.rank.map_or(0, NonZeroU32::get),
                        player.display_name(),
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
            let old_results = db::query(&old_db, None, true)?
                .into_iter()
                .map(|x| (x.id.clone(), x))
                .collect::<LinkedHashMap<_, _>>();
            let new_results = db::query(&new_db, query.as_ref(), true)?;
            println!("Found {} players:", new_results.len());
            summarize_differences(&old_results, &new_results);
        }
        Sendou {
            in_db,
            out_db,
            tournament_id,
        } => sendou_cli(&in_db, &out_db, tournament_id)?,
        MigrateNames {
            style,
            in_db,
            out_db,
            query,
        } => migration_cli(style, &in_db, &out_db, query.as_ref())?,
        Animate {
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
        Leaderboard {
            comparison,
            max_message_length,
            db,
        } => {
            let db = Database::read(&db)?;
            let old_players = comparison
                .map(|p| Database::read(&p))
                .transpose()?
                .map(Database::into_map)
                .unwrap_or_else(|| db.clone().into_map());
            let max_message_length = max_message_length.map(NonZeroUsize::get);
            let messages = generate_leaderboard_messages(
                &old_players,
                &db,
                &HashMap::new(),
                max_message_length.unwrap_or(usize::MAX),
            );
            if max_message_length.is_some() {
                for (index, message) in messages.into_iter().enumerate() {
                    if index > 0 {
                        println!();
                    }
                    println!("Message #{}:", index + 1);
                    println!("{message}");
                }
            } else {
                for message in messages {
                    println!("{message}");
                }
            }
        }
        HideRank {
            in_db,
            out_db,
            player,
        } => {
            hide_unhide_rank(&in_db, &out_db, player, true, "Hid", "hidden")?;
        }
        UnhideRank {
            in_db,
            out_db,
            player,
        } => {
            hide_unhide_rank(&in_db, &out_db, player, false, "Unhid", "shown")?;
        }
    }
    Ok(())
}

pub fn summarize_differences(
    old_results: &SwitzerlandPlayerMap,
    new_results: &Vec<SwitzerlandPlayer>,
) {
    for new_player in new_results {
        let old_result = old_results.get(&new_player.id);
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
        new_player.display_name(),
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
                match (old_player.rank, new_player.rank) {
                    (Some(old_rank), Some(new_rank)) => {
                        format!(
                            "; {}",
                            match new_rank.cmp(&old_rank) {
                                Ordering::Equal => format!("#{new_rank} ⇒"),
                                Ordering::Less => format!("#{old_rank} → #{new_rank} ⇑"),
                                Ordering::Greater => format!("#{old_rank} → #{new_rank} ⇓"),
                            }
                        )
                    }
                    (None, Some(new_rank)) => format!("; {}", new_rank.get()),
                    (_, None) => "".to_string(),
                }
            } else {
                "".to_string()
            }
        )
    } else {
        format!(
            "{:.1} SP{}",
            new_player.rating.rating,
            if show_rank && let Some(rank) = new_player.rank {
                format!("; #{}", rank)
            } else {
                "".to_string()
            }
        )
    }
}

fn hide_unhide_rank(
    in_db: &Path,
    out_db: &Path,
    player: String,
    hide: bool,
    action: &str,
    already_action: &str,
) -> Result<()> {
    let mut db = Database::read(in_db)?;
    db.for_each_matching_mut(&vec![player], true, |db, idx| {
        let player = &mut db.players[idx];
        if player.hide_rank != hide {
            player.hide_rank = hide;
            println!("{action} rank for {}", player.display_name());
        } else {
            println!(
                "Rank for {} already {already_action}",
                player.display_name()
            );
        }
    });
    db.write(out_db)?;
    Ok(())
}
