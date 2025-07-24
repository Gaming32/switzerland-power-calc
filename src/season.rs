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
    db.players.retain_mut(|player| {
        if matches!(player.season, SeasonState::NotParticipated) {
            return true;
        }

        let old_rating = player.rating.rating;
        let new_rating = find_new_rating(player.rating.rating);
        println!(
            "#{} {}: {:.1} SP -> {:.1} SP",
            player.season.unwrap_rank(),
            player.name,
            old_rating,
            new_rating
        );

        player.season = SeasonState::NotParticipated;
        if new_rating != 1500.0 {
            player.rating = Glicko2Rating {
                rating: new_rating,
                ..Default::default()
            };
            true
        } else {
            false
        }
    });

    db.write(out_db)?;

    Ok(())
}

fn find_new_rating(old_rating: f64) -> f64 {
    const GRANULARITY: f64 = 250.0;
    let bucket = ((old_rating - 1500.0) / GRANULARITY) as i32;
    bucket as f64 * GRANULARITY + 1500.0
}
