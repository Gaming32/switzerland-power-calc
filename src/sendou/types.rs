use crate::Result;
use crate::sendou::schema::{SendouId, Tournament, TournamentTeam};
use serenity::all::GuildChannel;
use std::collections::HashMap;

pub type TeamsMap<'a> = HashMap<SendouId, (&'a TournamentTeam, String)>;

pub trait GetTournamentFn: AsyncFn() -> Result<Tournament> {}
impl<F> GetTournamentFn for F where F: AsyncFn() -> Result<Tournament> {}

pub type DiscordChannelsMap = HashMap<SendouId, GuildChannel>;
