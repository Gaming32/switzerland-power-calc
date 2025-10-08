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

#[derive(Copy, Clone, Debug, Eq, PartialEq, Default)]
pub enum MatchOutcome {
    #[default]
    Unplayed,
    Win,
    Lose,
}

pub trait SetScore {
    fn set_score(&self) -> (usize, usize);
}

impl<const N: usize> SetScore for [MatchOutcome; N] {
    fn set_score(&self) -> (usize, usize) {
        let mut wins = 0;
        let mut losses = 0;
        for x in self {
            match x {
                MatchOutcome::Win => wins += 1,
                MatchOutcome::Lose => losses += 1,
                MatchOutcome::Unplayed => {}
            }
        }
        (wins, losses)
    }
}
