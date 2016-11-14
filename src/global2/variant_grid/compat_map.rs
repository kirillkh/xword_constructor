use common::{Placement, PlacementId};
use bit_set::BitSet;

use std::ops::Index;
use fnv::FnvHashSet;

pub struct CompatMap {
    sets: Vec<BitSet<usize>>
}


impl CompatMap {
    pub fn new(places: &[Placement]) -> CompatMap {
        let mut total_entries = 0;

        let mut sets = Vec::new();
        for place in places.iter() {
            let pid = place.id;
            let mut set: BitSet<usize> = Default::default();

            for place2 in places.iter() {
                if !CompatMap::check_compat(place, place2) {
                    set.insert(place2.id.0);
                    total_entries += 1;
                }
            }

            sets.push(set)
        }

        println!("total_entries: {}", total_entries);
        CompatMap { sets:sets }
    }

    pub fn compat(&self, id: PlacementId, other: PlacementId) -> bool {
        !self.sets[id].contains(other.0)
    }


    fn check_compat(place: &Placement, place2: &Placement) -> bool {
        place.compatible(place2)
    }
}



impl Index<PlacementId> for Vec<BitSet<usize>> {
    type Output = BitSet<usize>;

    #[inline]
    fn index(&self, index: PlacementId) -> &Self::Output {
        self.index(index.0 as usize)
    }
}
