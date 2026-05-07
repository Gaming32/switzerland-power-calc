use skillratings::glicko2::Glicko2Rating;

pub struct RankVec<Id> {
    ratings: Vec<(Id, Glicko2Rating)>,
}

impl<Id> RankVec<Id> {
    pub fn new(mut ratings: Vec<(Id, Glicko2Rating)>) -> Self {
        ratings.sort_by(|(_, a), (_, b)| b.rating.total_cmp(&a.rating));
        Self { ratings }
    }
}

impl<Id> RankVec<Id>
where
    Id: PartialEq,
{
    #[cfg(test)]
    pub fn get_rank(&self, id: &Id, rating: Glicko2Rating) -> Option<usize> {
        self.get_rank_and_index(id, rating).map(|(rank, _)| rank)
    }

    pub fn get_rank_and_remove(&mut self, id: &Id, rating: Glicko2Rating) -> Option<usize> {
        let (rank, index) = self.get_rank_and_index(id, rating)?;
        self.ratings.remove(index);
        Some(rank)
    }

    pub fn insert_and_get_rank(&mut self, id: Id, rating: Glicko2Rating) -> usize {
        let rank = self.get_raw_rank(rating);
        self.ratings.insert(rank, (id, rating));
        rank
    }

    fn get_rank_and_index(&self, id: &Id, rating: Glicko2Rating) -> Option<(usize, usize)> {
        let rank = self.get_raw_rank(rating);
        for index in rank..self.ratings.len() {
            let (other_id, other_rate) = &self.ratings[index];
            if other_rate != &rating {
                break;
            }
            if other_id == id {
                return Some((rank, index));
            }
        }
        None
    }

    fn get_raw_rank(&self, rating: Glicko2Rating) -> usize {
        self.ratings
            .partition_point(|(_, x)| x.rating.total_cmp(&rating.rating).is_gt())
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

        let mut ranks = RankVec::new(vec![
            (1, rate1),
            (2, rate2),
            (3, rate3),
            (4, rate4),
            (5, rate5),
        ]);

        assert_eq!(ranks.get_rank(&1, rate1), Some(0));
        assert_eq!(ranks.get_rank(&2, rate2), Some(0));
        assert_eq!(ranks.get_rank(&3, rate3), Some(0));
        assert_eq!(ranks.get_rank(&4, rate4), Some(0));
        assert_eq!(ranks.get_rank(&5, rate5), Some(0));

        {
            let (new_rate1, new_rate2) =
                glicko2(&rate1, &rate2, &Outcomes::LOSS, &Glicko2Config::new());

            assert_eq!(ranks.get_rank_and_remove(&1, rate1), Some(0));
            assert_eq!(ranks.insert_and_get_rank(1, new_rate1), 4);
            rate1 = new_rate1;

            assert_eq!(ranks.get_rank_and_remove(&2, rate2), Some(0));
            assert_eq!(ranks.insert_and_get_rank(2, new_rate2), 0);
            rate2 = new_rate2;
        }

        {
            let (new_rate3, new_rate4) =
                glicko2(&rate3, &rate4, &Outcomes::WIN, &Glicko2Config::new());

            assert_eq!(ranks.get_rank_and_remove(&3, rate3), Some(1));
            assert_eq!(ranks.insert_and_get_rank(3, new_rate3), 0);
            rate3 = new_rate3;

            assert_eq!(ranks.get_rank_and_remove(&4, rate4), Some(2));
            assert_eq!(ranks.insert_and_get_rank(4, new_rate4), 3);
            rate4 = new_rate4;
        }

        assert_eq!(ranks.get_rank(&1, rate1), Some(3));
        assert_eq!(ranks.get_rank(&2, rate2), Some(0));
        assert_eq!(ranks.get_rank(&3, rate3), Some(0));
        assert_eq!(ranks.get_rank(&4, rate4), Some(3));
        assert_eq!(ranks.get_rank(&5, rate5), Some(2));

        let rate6 = Glicko2Rating::new();
        assert_eq!(ranks.get_rank(&6, rate6), None);
        assert_eq!(ranks.get_rank_and_remove(&6, rate6), None);
        assert_eq!(ranks.insert_and_get_rank(6, rate6), 2);

        // Regression test: Removing a rate can sometimes remove the wrong one
        assert_eq!(ranks.get_rank_and_remove(&5, rate5), Some(2));
        assert_eq!(ranks.get_rank(&6, rate6), Some(2));
    }
}
