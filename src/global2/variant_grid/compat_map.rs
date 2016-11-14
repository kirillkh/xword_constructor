use common::{Placement, PlacementId};

use std::ops::Index;
use fnv::FnvHashSet;

pub struct CompatMap {
//    map: FnvHashSet<IdPair>
    sets: Vec<FnvHashSet<PlacementId>>
}


impl CompatMap {
    pub fn new(places: &[Placement]) -> CompatMap {
        let mut total_entries = 0;

        let mut sets = Vec::new();
        for place in places.iter() {
            let pid = place.id;
            let mut set = FnvHashSet::with_hasher(Default::default());

            for place2 in places.iter() {
                if !CompatMap::check_compat(place, place2) {
                    set.insert(place2.id);
                    total_entries += 1;
                }
            }

            sets.push(set)
        }

        println!("total_entries: {}", total_entries);
        CompatMap { sets:sets }
    }

    pub fn compat(&self, id: PlacementId, other: PlacementId) -> bool {
        !self.sets[id].contains(&other)
    }


    fn check_compat(place: &Placement, place2: &Placement) -> bool {
        place.compatible(place2)
    }
}



impl Index<PlacementId> for Vec<FnvHashSet<PlacementId>> {
    type Output = FnvHashSet<PlacementId>;

    #[inline]
    fn index(&self, index: PlacementId) -> &Self::Output {
        self.index(index.0 as usize)
    }
}
