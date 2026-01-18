use crate::error::Result;
use crate::sendou::SendouId;
use crate::sendou::lang::Language;
use hashlink::LinkedHashMap;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use skillratings::glicko2::Glicko2Rating;
use std::borrow::Cow;
use std::cmp::Ordering;
use std::fs;
use std::num::NonZeroU32;
use std::path::Path;

pub type SwitzerlandPlayerMap = LinkedHashMap<PlayerId, SwitzerlandPlayer>;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Database {
    pub players: Vec<SwitzerlandPlayer>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct SwitzerlandPlayer {
    #[serde(alias = "name")]
    pub id: PlayerId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<Language>,
    #[serde(skip)]
    pub rank: Option<NonZeroU32>,
    #[serde(default)]
    pub hide_rank: bool,
    #[serde(flatten)]
    pub rating: Glicko2Rating,
    #[serde(skip)]
    pub unrated: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, Hash, Eq, PartialEq)]
#[serde(untagged)]
pub enum PlayerId {
    Sendou(SendouId),
    LegacyName(String),
}

impl SwitzerlandPlayer {
    pub fn sendou_id(&self) -> Option<SendouId> {
        if let PlayerId::Sendou(id) = self.id {
            Some(id)
        } else {
            None
        }
    }

    pub fn display_name(&self) -> Cow<'_, str> {
        self.display_name.as_deref().map_or_else(
            || match &self.id {
                PlayerId::Sendou(id) => Cow::Owned(id.to_string()),
                PlayerId::LegacyName(name) => Cow::Borrowed(name.as_str()),
            },
            Cow::Borrowed,
        )
    }

    pub fn show_rank(&self) -> bool {
        !self.hide_rank
    }

    pub fn descending_rating_order_cmp(&self, other: &Self) -> Ordering {
        other.rating.rating.total_cmp(&self.rating.rating)
    }
}

impl PlayerId {
    pub fn unwrap_sendou(&self) -> SendouId {
        if let PlayerId::Sendou(id) = self
            && *id != 0
        {
            *id
        } else {
            panic!("Not a valid Sendou ID: {self:?}")
        }
    }
}

impl Default for PlayerId {
    fn default() -> Self {
        PlayerId::Sendou(0)
    }
}

impl Database {
    pub fn new() -> Self {
        Self { players: vec![] }
    }

    pub fn new_from_map(map: SwitzerlandPlayerMap) -> Self {
        let mut result = Self {
            players: map
                .into_iter()
                .map(|(_, v)| v)
                .filter(|x| !x.unrated)
                .collect(),
        };
        result.sort();
        result
    }

    pub fn sort(&mut self) {
        self.players
            .sort_by(SwitzerlandPlayer::descending_rating_order_cmp);
        self.init_rank();
    }

    fn init_rank(&mut self) {
        let mut rank = 1;
        for player in self.players.iter_mut() {
            if player.show_rank() {
                player.rank =
                    Some(NonZeroU32::new(rank).expect("Why are there 4 billion players?"));
                rank += 1;
            } else {
                player.rank = None;
            }
        }
    }

    pub fn read(file: &Path) -> Result<Self> {
        let mut result: Database = serde_cbor::from_reader(fs::File::open(file)?)?;
        result.sort();
        Ok(result)
    }

    pub fn into_map(self) -> SwitzerlandPlayerMap {
        self.players
            .into_iter()
            .map(|x| (x.id.clone(), x))
            .collect()
    }

    pub fn write(&self, file: &Path) -> Result<()> {
        serde_cbor::to_writer(fs::File::create(file)?, self)?;
        Ok(())
    }

    pub fn for_each_matching_mut(
        &mut self,
        queries: &Vec<String>,
        allow_sendou_id: bool,
        mut action: impl FnMut(&mut Self, usize),
    ) {
        for query in queries {
            let closest_match = allow_sendou_id
                .then(|| query.parse::<SendouId>().ok())
                .flatten()
                .and_then(|query| {
                    self.players
                        .iter()
                        .position(|x| x.sendou_id() == Some(query))
                })
                .or_else(|| {
                    let query = query.to_lowercase();
                    self.players.iter().position_max_by_key(|x| {
                        totally_ordered::TotallyOrdered(strsim::jaro_winkler(
                            &query,
                            &x.display_name().to_lowercase(),
                        ))
                    })
                });
            if let Some(index) = closest_match {
                action(self, index);
            }
        }
    }

    pub fn query(
        mut self,
        queries: Option<&Vec<String>>,
        allow_sendou_id: bool,
    ) -> Vec<SwitzerlandPlayer> {
        if self.players.is_empty() {
            return self.players;
        }
        let Some(queries) = queries else {
            return self.players;
        };

        let mut results = Vec::with_capacity(queries.len());
        self.for_each_matching_mut(queries, allow_sendou_id, |db, idx| {
            results.push(db.players.remove(idx))
        });
        results
    }
}

pub fn init_db(file: &Path) -> Result<()> {
    Database::new().write(file)?;
    Ok(())
}

pub fn query(
    file: &Path,
    queries: Option<&Vec<String>>,
    allow_sendou_id: bool,
) -> Result<Vec<SwitzerlandPlayer>> {
    Ok(Database::read(file)?.query(queries, allow_sendou_id))
}
