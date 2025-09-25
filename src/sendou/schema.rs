use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_repr::Deserialize_repr;
use serde_with::{BoolFromInt, serde_as};

pub type SendouId = u32;

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
    Swiss {},
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

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TournamentTeam {
    pub id: SendouId,
    pub name: String,
    pub members: Vec<TournamentTeamMember>,
    pub check_ins: Vec<TournamentTeamCheckIn>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TournamentTeamMember {
    pub username: String,
    pub discord_id: serenity::all::UserId,
    pub custom_url: Option<String>,
    pub in_game_name: String,
}

#[derive(Copy, Clone, Debug, Deserialize)]
pub struct TournamentTeamCheckIn {}
