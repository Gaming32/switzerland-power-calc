use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_repr::Deserialize_repr;
use serde_with::DefaultOnNull;
use serde_with::{BoolFromInt, json::JsonString, serde_as};

pub type SendouId = u32;

// https://github.com/sendou-ink/sendou.ink/blob/main/app/features/api-public/schema.ts

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetUserResponse {
    pub id: SendouId,
    pub name: String,
}

#[derive(Copy, Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetUserIdsResponse {
    pub id: SendouId,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetTournamentMatchResponse {
    pub map_list: Option<Vec<MapListMap>>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetTournamentResponse {
    pub name: String,
    pub start_time: DateTime<Utc>,
    pub brackets: Vec<TournamentBracket>,
    pub is_finalized: bool,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetTournamentTeamsResponse {
    pub id: SendouId,
    pub name: String,
    pub checked_in: bool,
    pub seeding_power: GetTournamentTeamsResponseSeedingPower,
    pub members: Vec<GetTournamentTeamsResponseMember>,
}

#[derive(Copy, Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetTournamentTeamsResponseSeedingPower {
    pub unranked: Option<f64>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetTournamentTeamsResponseMember {
    pub user_id: SendouId,
    pub name: String,
    pub discord_id: serenity::all::UserId,
    pub country: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetTournamentBracketResponse {
    pub data: TournamentBracketData,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetTournamentBracketStandingsResponse {
    pub standings: Vec<GetTournamentBracketStandingsResponseStandings>,
}

#[derive(Copy, Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetTournamentBracketStandingsResponseStandings {
    pub tournament_team_id: SendouId,
}

#[derive(Copy, Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MapListMap {
    pub winner_team_id: Option<SendouId>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TournamentBracket {
    pub name: String,
}

pub type TournamentBracketData = BracketData;

// https://github.com/sendou-ink/sendou.ink/blob/main/app/features/tournament-bracket/core/engine/types.ts

#[derive(Copy, Clone, Debug, Eq, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Side {
    Opponent1,
    Opponent2,
}

#[derive(Copy, Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParticipantResult {
    pub id: Option<SendouId>,
    pub score: Option<u32>,
}

#[derive(Copy, Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RoundData {
    pub id: SendouId,
    pub maps: Option<TournamentRoundMaps>,
}

#[derive(Copy, Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MatchData {
    pub opponent1: Option<ParticipantResult>,
    pub opponent2: Option<ParticipantResult>,
    pub winner_side: Option<Side>,
    pub id: SendouId,
    pub round_id: SendouId,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BracketData {
    pub round: Vec<RoundData>,
    pub r#match: Vec<MatchData>,
}

// https://github.com/sendou-ink/sendou.ink/blob/main/app/db/tables-json.ts#L187

#[derive(Copy, Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TournamentRoundMaps {
    pub count: u32,
    pub r#type: TournamentRoundMapsType,
}

#[derive(Copy, Clone, Debug, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TournamentRoundMapsType {
    BestOf,
    PlayAll,
}
