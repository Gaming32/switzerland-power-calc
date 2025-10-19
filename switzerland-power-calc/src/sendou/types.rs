use crate::Result;
use crate::sendou::schema::{SendouId, Tournament, TournamentTeam};
use serenity::all::ChannelId;
use skillratings::glicko2::Glicko2Rating;
use std::cmp::Ordering;
use std::collections::HashMap;

pub type TeamsMap<'a> = HashMap<SendouId, &'a TournamentTeam>;

pub trait GetTournamentFn: AsyncFn() -> Result<Tournament> {}
impl<F> GetTournamentFn for F where F: AsyncFn() -> Result<Tournament> {}

pub type DiscordChannelsMap = HashMap<SendouId, ChannelId>;

#[derive(Copy, Clone)]
pub struct DescendingRatingGlicko2(pub Glicko2Rating);

impl PartialEq for DescendingRatingGlicko2 {
    fn eq(&self, other: &Self) -> bool {
        self.0.rating == other.0.rating
    }
}

impl Eq for DescendingRatingGlicko2 {}

impl PartialOrd for DescendingRatingGlicko2 {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for DescendingRatingGlicko2 {
    fn cmp(&self, other: &Self) -> Ordering {
        other.0.rating.total_cmp(&self.0.rating)
    }
}
