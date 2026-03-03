const MIN_COUNT: usize = 10;
const MAX_LEADERBOARD_COUNT: usize = 500;
const MAX_SHOW_PLACEMENT_COUNT: usize = 50_000;
const LEADERBOARD_DIVISOR: usize = 5;
const SHOW_PLACEMENT_DIVISOR: usize = 2;

pub fn leaderboard_count(player_count: usize) -> usize {
    clean_number_multiple(player_count / LEADERBOARD_DIVISOR, true)
        .clamp(MIN_COUNT, MAX_LEADERBOARD_COUNT)
}

pub fn show_placement_count(player_count: usize) -> usize {
    clean_number_multiple(player_count / SHOW_PLACEMENT_DIVISOR, false)
        .clamp(MIN_COUNT, MAX_SHOW_PLACEMENT_COUNT)
}

fn clean_number_multiple(n: usize, clamp_base: bool) -> usize {
    if n < MIN_COUNT {
        return n;
    }
    let divisor = 10usize.pow(n.ilog10());
    let mut base = n / divisor;
    if clamp_base {
        base = base.clamp(1, 5);
    }
    base * divisor
}
