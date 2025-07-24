use crate::db::{Database, SeasonState};
use crate::error::Result;
use skillratings::glicko2::Glicko2Rating;
use std::fs;
use std::path::Path;

pub fn new_season(in_db: &Path, out_db: &Path) -> Result<()> {
    if let Some(parent) = out_db.parent() {
        fs::create_dir_all(parent)?;
    }

    const GROUP_COUNT: usize = 6;

    let mut db = Database::read(in_db)?;

    let participating_players = db
        .players
        .iter_mut()
        .filter(|x| matches!(x.season, SeasonState::Participated(_)))
        .collect::<Vec<_>>();

    // Roughly https://lemire.me/blog/2025/05/22/dividing-an-array-into-fair-sized-chunks
    let small_chunk_size = participating_players.len() / GROUP_COUNT;
    let large_chunk_size = small_chunk_size + 1;
    let large_chunk_count = participating_players.len() % GROUP_COUNT;
    let small_chunk_count = GROUP_COUNT - large_chunk_count;

    let mut groups = [const { Vec::new() }; GROUP_COUNT];
    let mut group_index = 0;
    for player in participating_players {
        loop {
            let group_size = if group_index < small_chunk_count {
                small_chunk_size
            } else {
                large_chunk_size
            };
            if groups[group_index].len() < group_size {
                break;
            } else {
                group_index += 1;
            }
        }
        groups[group_index].push(player);
    }

    for (i, group) in groups.into_iter().enumerate() {
        let average_rating =
            group.iter().map(|p| p.rating.rating).sum::<f64>() / group.len() as f64;
        println!(
            "Group {} ({} players) = {:.1} SP:",
            i + 1,
            group.len(),
            average_rating
        );
        for player in group {
            println!(
                "  #{} {}: {:.1} SP -> {:.1} SP",
                player.season.unwrap_rank(),
                player.name,
                player.rating.rating,
                average_rating
            );
            player.rating = Glicko2Rating {
                rating: average_rating,
                ..Default::default()
            };
        }
    }

    db.write(out_db)?;

    Ok(())
}
