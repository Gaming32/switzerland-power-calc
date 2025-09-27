#[derive(Copy, Clone, Debug)]
pub enum PowerStatus {
    Calculating {
        progress: u32,
        total: u32,
    },
    Calculated {
        calculation_rounds: u32,
        power: f64,
        rank: u32,
    },
    SetPlayed {
        matches: [MatchOutcome; 5],
        old_power: f64,
        new_power: f64,
        old_rank: u32,
        new_rank: u32,
    },
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum MatchOutcome {
    Unplayed,
    Win,
    Lose,
}
