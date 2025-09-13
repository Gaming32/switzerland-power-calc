use serde::Deserialize;
use serde_repr::Deserialize_repr;

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
    #[serde(rename = "match")]
    pub matches: Vec<TournamentMatch>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct TournamentMatch {
    pub opponent1: Option<TournamentMatchOpponent>,
    pub opponent2: Option<TournamentMatchOpponent>,
    pub status: TournamentMatchStatus,
}

#[derive(Clone, Debug, Deserialize)]
pub struct TournamentMatchOpponent {
    pub id: u32,
    pub result: Option<TournamentMatchResult>,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
enum TournamentMatchResult {
    Win,
    Loss,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Deserialize_repr)]
#[repr(u8)]
enum TournamentMatchStatus {
    Locked = 0,
    Waiting = 1,
    Ready = 2,
    Running = 3,
    Completed = 4,
}

#[derive(Clone, Debug, Deserialize)]
pub struct TournamentContext {
    pub settings: TournamentSettings,
    pub teams: Vec<TournamentTeam>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TournamentSettings {
    pub bracket_progression: Vec<TournamentBracketProgression>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(
    tag = "type",
    content = "settings",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum TournamentBracketProgression {
    SingleElimination {},
    DoubleElimination {},
    RoundRobin {},
    Swiss { round_count: u32 },
}

#[derive(Clone, Debug, Deserialize)]
pub struct TournamentTeam {
    pub id: u32,
    pub name: String,
    pub members: Vec<TournamentTeamMember>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TournamentTeamMember {
    pub user_id: u32,
    pub username: String,
    pub discord_id: serenity::model::id::UserId,
    pub in_game_name: String,
}
