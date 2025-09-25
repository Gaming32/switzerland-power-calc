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
    Normal {
        old_power: f64,
        new_power: f64,
        old_rank: u32,
        new_rank: u32,
    },
}
