use crate::error::Result;
use serde::{Deserialize, Serialize};
use skillratings::glicko2::Glicko2Rating;
use std::fs;
use std::num::NonZeroU32;
use std::path::Path;

#[derive(Serialize, Deserialize, Debug)]
pub struct Database {
    pub players: Vec<SwitzerlandPlayer>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct SwitzerlandPlayer {
    pub name: String,
    #[serde(default = "SeasonState::old_db_migration")]
    pub season: SeasonState,
    #[serde(flatten)]
    pub rating: Glicko2Rating,
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone, Default)]
#[serde(rename_all = "snake_case")]
pub enum SeasonState {
    #[default]
    NotParticipated,
    Participated(#[serde(skip)] Option<NonZeroU32>),
}

impl SeasonState {
    fn old_db_migration() -> Self {
        Self::Participated(None)
    }

    pub fn unwrap_rank(&self) -> u32 {
        match self {
            SeasonState::Participated(rank) => rank
                .expect("unwrap_rank() called on uninitialized rank")
                .get(),
            SeasonState::NotParticipated => panic!("unwrap_rank() called on non-season rank"),
        }
    }
}

impl Database {
    pub fn new() -> Self {
        Self { players: vec![] }
    }

    pub fn sort(&mut self) {
        self.players
            .sort_by(|p1, p2| p2.rating.rating.total_cmp(&p1.rating.rating));
        self.init_rank();
    }

    fn init_rank(&mut self) {
        let mut rank = 1;
        for player in self.players.iter_mut() {
            if let SeasonState::Participated(ref mut player_rank) = player.season {
                *player_rank = NonZeroU32::new(rank);
                rank += 1;
            }
        }
    }

    pub fn read(file: &Path) -> Result<Self> {
        let mut result: Database = serde_cbor::from_reader(fs::File::open(file)?)?;
        result.init_rank();
        Ok(result)
    }

    pub fn write(&self, file: &Path) -> Result<()> {
        serde_cbor::to_writer(fs::File::create(file)?, self)?;
        Ok(())
    }
}

pub fn init_db(file: &Path) -> Result<()> {
    Database::new().write(file)?;
    Ok(())
}

pub fn query(
    file: &Path,
    queries: Option<&Vec<String>>,
    include_non_participated: bool,
) -> Result<Vec<SwitzerlandPlayer>> {
    let mut db = Database::read(file)?;
    if db.players.is_empty() {
        return Ok(db.players);
    }

    if !include_non_participated {
        db.players
            .retain(|player| matches!(player.season, SeasonState::Participated(..)));
    }

    let Some(queries) = queries else {
        return Ok(db.players);
    };

    let mut results = Vec::with_capacity(queries.len());
    for query in queries {
        let query = query.to_lowercase();
        let Some((closest_match, _)) = db.players.iter().enumerate().max_by_key(|(_, x)| {
            totally_ordered::TotallyOrdered(strsim::jaro_winkler(&query, &x.name.to_lowercase()))
        }) else {
            break;
        };
        results.push(db.players.remove(closest_match));
    }
    results.sort_by(|p1, p2| p2.rating.rating.total_cmp(&p1.rating.rating));
    Ok(results)
}
