use std::cmp::Ordering;
use crate::Result;
use crate::sendou::schema::{SendouId, Tournament, TournamentTeam};
use serenity::all::GuildChannel;
use std::collections::HashMap;
use skillratings::glicko2::Glicko2Rating;

pub type TeamsMap<'a> = HashMap<SendouId, (&'a TournamentTeam, String)>;

pub trait GetTournamentFn: AsyncFn() -> Result<Tournament> {}
impl<F> GetTournamentFn for F where F: AsyncFn() -> Result<Tournament> {}

pub type DiscordChannelsMap = HashMap<SendouId, GuildChannel>;

#[derive(Copy, Clone)]
pub struct DescendingRatingGlicko2(pub Glicko2Rating);

impl PartialEq for DescendingRatingGlicko2 {
    fn eq(&self, other: &Self) -> bool {
        self.0.rating == other.0.rating
    }
}

impl Eq for DescendingRatingGlicko2 {
}

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
