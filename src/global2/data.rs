use common::{Placement, PlacementId};
use super::weighted_selection_tree;
use fixed_grid::PlaceMove;

#[derive(Clone, Debug)]
pub struct ScoredMove {
    pub place: Placement,
	pub score: f32,
	pub exp_score: f32,
	
}

impl weighted_selection_tree::Item<PlacementId> for ScoredMove {
    #[inline]
    fn key(&self) -> PlacementId {
        self.place.id
    }
    
    #[inline]
    fn weight(&self) -> f32 {
        self.exp_score
    }
}

impl PlaceMove for ScoredMove {
    fn place(&self) -> &Placement {
        &self.place
    }
}

impl<'a> PlaceMove for &'a Placement {
    fn place(&self) -> &Placement {
        self
    }
}
