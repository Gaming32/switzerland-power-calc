use skillratings::glicko2::Glicko2Rating;
use std::cmp::Ordering;

pub struct RankVec {
    ratings: Vec<Glicko2Rating>,
}

impl RankVec {
    pub fn new(mut ratings: Vec<Glicko2Rating>) -> Self {
        ratings.sort_by(|a, b| b.rating.total_cmp(&a.rating));
        Self { ratings }
    }

    pub fn get_rank(&self, rating: Glicko2Rating) -> usize {
        let rating = rating.rating;
        self.ratings
            .partition_point(|x| x.rating.total_cmp(&rating) == Ordering::Greater)
    }

    pub fn get_rank_and_remove(&mut self, rating: Glicko2Rating) -> usize {
        let rank = self.get_rank(rating);
        self.ratings.remove(rank);
        rank
    }

    pub fn insert_and_get_rank(&mut self, rating: Glicko2Rating) -> usize {
        let rank = self.get_rank(rating);
        self.ratings.insert(rank, rating);
        rank
    }
}

#[cfg(test)]
mod test {
    use crate::sendou::rank_set::RankVec;
    use skillratings::Outcomes;
    use skillratings::glicko2::{Glicko2Config, Glicko2Rating, glicko2};

    #[test]
    fn rank_vec_test() {
        let mut rate1 = Glicko2Rating::new();
        let mut rate2 = Glicko2Rating::new();
        let mut rate3 = Glicko2Rating::new();
        let mut rate4 = Glicko2Rating::new();
        let rate5 = Glicko2Rating::new();

        let mut ranks = RankVec::new(vec![rate1, rate2, rate3, rate4, rate5]);

        assert_eq!(ranks.get_rank(rate1), 0);
        assert_eq!(ranks.get_rank(rate2), 0);
        assert_eq!(ranks.get_rank(rate3), 0);
        assert_eq!(ranks.get_rank(rate4), 0);
        assert_eq!(ranks.get_rank(rate5), 0);

        {
            let (new_rate1, new_rate2) =
                glicko2(&rate1, &rate2, &Outcomes::LOSS, &Glicko2Config::new());

            assert_eq!(ranks.get_rank_and_remove(rate1), 0);
            assert_eq!(ranks.insert_and_get_rank(new_rate1), 4);
            rate1 = new_rate1;

            assert_eq!(ranks.get_rank_and_remove(rate2), 0);
            assert_eq!(ranks.insert_and_get_rank(new_rate2), 0);
            rate2 = new_rate2;
        }

        {
            let (new_rate3, new_rate4) =
                glicko2(&rate3, &rate4, &Outcomes::WIN, &Glicko2Config::new());

            assert_eq!(ranks.get_rank_and_remove(rate3), 1);
            assert_eq!(ranks.insert_and_get_rank(new_rate3), 0);
            rate3 = new_rate3;

            assert_eq!(ranks.get_rank_and_remove(rate4), 2);
            assert_eq!(ranks.insert_and_get_rank(new_rate4), 3);
            rate4 = new_rate4;
        }

        assert_eq!(ranks.get_rank(rate1), 3);
        assert_eq!(ranks.get_rank(rate2), 0);
        assert_eq!(ranks.get_rank(rate3), 0);
        assert_eq!(ranks.get_rank(rate4), 3);
        assert_eq!(ranks.get_rank(rate5), 2);
    }
}
