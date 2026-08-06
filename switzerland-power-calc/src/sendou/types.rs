use crate::sendou::schema::{GetTournamentTeamsResponse, SendouId};
use serenity::all::ChannelId;
use std::collections::HashMap;

pub type TeamsMap<'a> = HashMap<SendouId, &'a GetTournamentTeamsResponse>;

pub type DiscordChannelsMap = HashMap<SendouId, ChannelId>;
