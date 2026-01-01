use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_repr::Deserialize_repr;
use serde_with::DefaultOnNull;
use serde_with::{BoolFromInt, serde_as};

pub type SendouId = u32;

#[derive(Clone, Debug, Deserialize)]
pub struct ToResponse {
    #[serde(rename = "features/tournament/routes/to.$id")]
    pub to: DataWrapper<TournamentRoot>,
}

#[derive(Copy, Clone, Debug, Deserialize)]
pub struct DataWrapper<T> {
    pub data: T,
}

#[derive(Clone, Debug, Deserialize)]
pub struct TournamentRoot {
    pub tournament: Tournament,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Tournament {
    pub data: TournamentData,
    #[serde(rename = "ctx")]
    pub context: TournamentContext,
}

#[derive(Clone, Debug, Deserialize)]
pub struct TournamentData {
    #[serde(rename = "stage")]
    pub stages: Vec<TournamentStage>,
    #[serde(rename = "group")]
    pub groups: Vec<TournamentGroup>,
    #[serde(rename = "round")]
    pub rounds: Vec<TournamentRound>,
    #[serde(rename = "match")]
    pub matches: Vec<TournamentMatch>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct TournamentStage {
    pub id: SendouId,
    pub name: String,
    pub number: u32,
    #[serde(flatten)]
    pub settings: TournamentStageSettings,
}

#[derive(Copy, Clone, Debug, Deserialize)]
#[serde(tag = "type", content = "settings", rename_all = "snake_case")]
pub enum TournamentStageSettings {
    SingleElimination {},
    DoubleElimination {},
    RoundRobin {},
    Swiss { swiss: TournamentStageSwissSettings },
}

#[derive(Copy, Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TournamentStageSwissSettings {
    pub round_count: u32,
}

#[derive(Copy, Clone, Debug, Deserialize)]
pub struct TournamentGroup {
    pub id: SendouId,
    pub number: u32,
    pub stage_id: SendouId,
}

#[derive(Copy, Clone, Debug, Deserialize)]
pub struct TournamentRound {
    pub id: SendouId,
    pub group_id: SendouId,
    pub number: u32,
    pub stage_id: SendouId,
}

#[derive(Copy, Clone, Debug, Deserialize)]
pub struct TournamentMatch {
    pub id: SendouId,
    pub opponent1: Option<TournamentMatchOpponent>,
    pub opponent2: Option<TournamentMatchOpponent>,
    pub round_id: SendouId,
    pub status: TournamentMatchStatus,
}

#[derive(Copy, Clone, Debug, Deserialize)]
pub struct TournamentMatchOpponent {
    pub id: Option<SendouId>,
    pub result: Option<TournamentMatchResult>,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TournamentMatchResult {
    Win,
    Loss,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Deserialize_repr)]
#[repr(u8)]
pub enum TournamentMatchStatus {
    Locked = 0,
    Waiting = 1,
    Ready = 2,
    Running = 3,
    Completed = 4,
}

#[serde_as]
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TournamentContext {
    pub id: SendouId,
    pub name: String,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub start_time: DateTime<Utc>,
    #[serde_as(as = "BoolFromInt")]
    pub is_finalized: bool,
    pub teams: Vec<TournamentTeam>,
}

#[serde_as]
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TournamentTeam {
    pub id: SendouId,
    pub name: String,
    pub members: Vec<TournamentTeamMember>,
    pub check_ins: Vec<TournamentTeamCheckIn>,
    #[serde_as(deserialize_as = "DefaultOnNull")]
    pub avg_seeding_skill_ordinal: f64,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TournamentTeamMember {
    pub user_id: SendouId,
    pub username: String,
    pub discord_id: serenity::all::UserId,
    pub country: Option<String>,
}

#[derive(Copy, Clone, Debug, Deserialize)]
pub struct TournamentTeamCheckIn {}

#[derive(Clone, Debug, Deserialize)]
pub struct ToMatchResponse {
    #[serde(rename = "features/tournament-bracket/routes/to.$id.matches.$mid")]
    pub to_match: DataWrapper<MatchRoot>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct MatchRoot {
    pub results: Vec<MatchResult>,
}

#[derive(Copy, Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MatchResult {
    pub winner_team_id: SendouId,
}

#[derive(Clone, Debug, Deserialize)]
pub struct SendouUserRoot {
    pub user: SendouUser,
}

#[derive(Clone, Debug, Deserialize)]
pub struct SendouUser {
    pub id: SendouId,
    pub username: String,
}
