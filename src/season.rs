use crate::db::{Database, SeasonState};
use crate::error::Result;
use skillratings::glicko2::Glicko2Rating;
use std::fs;
use std::path::Path;

pub fn new_season(in_db: &Path, out_db: &Path) -> Result<()> {
    if let Some(parent) = out_db.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut db = Database::read(in_db)?;

    let participating_players = db
        .players
        .iter_mut()
        .filter(|x| matches!(x.season, SeasonState::Participated(_)))
        .collect::<Vec<_>>();
    let player_count = participating_players.len();

    // This is basically implemented the same way as SendouQ, just with different percentages.
    // SendouQ uses 5%, 10%, 15%, 17.5%, 20%, 17.5%, 15%
    // We use 5%, 10%, 20%, 30%, 20%, 15%
    const PERCENTAGE_COUNT: usize = 5;
    const PERCENTAGES: [usize; PERCENTAGE_COUNT] = [5, 10, 20, 30, 20];
    let mut groups = [const { Vec::new() }; PERCENTAGE_COUNT + 1];

    let mut group_index = 0;
    for player in participating_players {
        while group_index < PERCENTAGE_COUNT {
            let group_size = player_count * PERCENTAGES[group_index] / 100;
            if groups[group_index].len() < group_size {
                break;
            } else {
                group_index += 1;
            }
        }
        groups[group_index].push(player);
    }

    let mut group_ratings = groups
        .each_ref()
        .map(|group| group.iter().map(|p| p.rating.rating).sum::<f64>() / group.len() as f64);
    group_ratings[0] = group_ratings[1];
    group_ratings[PERCENTAGE_COUNT] = group_ratings[PERCENTAGE_COUNT - 1];

    for (i, (group, new_rating)) in groups.into_iter().zip(group_ratings).enumerate() {
        println!(
            "Group {} ({} players) = {:.1} SP:",
            i + 1,
            group.len(),
            new_rating
        );
        for player in group {
            println!(
                "  #{} {}: {:.1} SP -> {:.1} SP",
                player.season.unwrap_rank(),
                player.name,
                player.rating.rating,
                new_rating
            );
            player.rating = Glicko2Rating {
                rating: new_rating,
                ..Default::default()
            };
        }
    }

    db.write(out_db)?;

    Ok(())
}
