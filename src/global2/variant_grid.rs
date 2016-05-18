use ndarray::OwnedArray;
use common::{dim, Placement, PlacementId, MatrixDim};
use std::rc::Rc;
use std::ops::{Index, IndexMut};


struct Entry {
    field_indices: Vec<usize>,
//    place_id: PlacementId
}

impl Entry {
    fn new(place: &Placement) -> Entry {
//        Entry { field_indices: vec![0; place.word.len()], place_id: place.id }
        Entry { field_indices: vec![0; place.word.len()] }
    }
}

//impl Index<PlacementId> for Vec<Entry> {
//    type Output = Entry;
//
//	#[inline]
//    fn index(&self, index: PlacementId) -> &Self::Output {
//		self.index(index.0 as usize)
//    }
//}



//pub struct Iter<'a> {
//    it: ::std::slice::Iter<'a, PlacementId>
//}
//
//impl<'a> Iterator for Iter<'a> {
//    type Item = PlacementId;
//    
//    fn next(&mut self) -> Option<Self::Item> {
//        if let Some(place) = self.it.next() {
//            Some(entry.place_id)
//        } else {
//            None
//        }
//    }
//}


// TODO: rewrite using R-Tree or Interval Tree?
pub struct VariantGrid {
    entries: Vec<Entry>,
	field: OwnedArray<Vec<PlacementId>, MatrixDim>,
//    dic: &'a [Placement]
    dic: Rc<Vec<Placement>>
}

impl VariantGrid {
//    pub fn new(dic: &'a [Placement], h: dim, w: dim) -> VariantGrid {
    pub fn new(dic: Rc<Vec<Placement>>, h: dim, w: dim) -> VariantGrid {
        let mut field = OwnedArray::default(MatrixDim(h, w));
        let mut entries = dic.iter().map(|place| Entry::new(place)).collect::<Vec<Entry>>();
        
        for (i, place) in dic.iter().enumerate() {
            let entry = &mut entries[i];
            
            let(place_y, place_x) = (place.y, place.x);
            place.fold_positions((), |(), y, x| {
                let cell_entries: &mut Vec<PlacementId> = &mut field[MatrixDim(y, x)];
                let char_idx = y-place_y + x-place_x;
                entry.field_indices[char_idx] = cell_entries.len();
                cell_entries.push(PlacementId(i as u32));
            });
        }
        
        VariantGrid { entries: entries, field: field, dic: dic }
    }
    
    pub fn iter_at(&self, y: dim, x: dim) -> ::std::slice::Iter<PlacementId> {
//        let v: &Vec<PlacementId> = &self.field[MatrixDim(y, x)];
//        Iter{ it:v.iter() }
        self.field[MatrixDim(y, x)].iter()
    }
    
    pub fn remove_incompat(&mut self, place_id: PlacementId) -> Vec<PlacementId> {
        let dic = self.dic.clone();
        let place = &dic[place_id];
        let (place_y, place_x) = (place.y, place.x);
        let (dy, dx) = place.orientation.align(0, 1);
        let (perpdy, perpdx) = (dx, dy);
        
        let (v0, u0) = place.align(place.orientation);
        let len0 = place.word.len();
        let dim = self.field.dim();
        let (maxv, maxu) = place.orientation.align(dim.0, dim.1);
        
        let mut removed = vec![];
        
        place.fold_positions((), |(), y, x| {
            let char_idx = (y - place_y) + (x - place_x);
            
            // remove incompatible placements in the current cell
            self.filter_incompat(y, x, &mut removed, |other| {
                    let c_idx = (y - other.y) + (x - other.x);
                    
                    place.orientation == other.orientation ||
                    place.word[char_idx] != other.word[c_idx]
            });
            
            // remove incompatible placements in the cell above the current cell (for horizontal orientation)
            // the only case we need to handle here is when "other" is perpendicular to "place" and touches it, but does not intersect
            if v0 > 0 {
                self.filter_incompat(y-perpdy, x-perpdx, &mut removed, |other| {
                    let (v1, _) = other.align(place.orientation);
                    
                    place.orientation != other.orientation && 
                    v1 + other.word.len() == v0	// touches from above but does not intersect => incompatible
                });
            }
            
            // remove incompatible placements in the cell below the current cell (for horizontal orientation)
            if v0 + 1 < maxv {
                self.filter_incompat(y+perpdy, x+perpdx, &mut removed, |other| {
                    let (v1, _) = other.align(place.orientation);
                    
                    place.orientation != other.orientation && 
                    v1 == v0 + 1 // touches from below but does not intersect => incompatible
                });
            }
        });
        
        
            
        // remove incompatible placements in the cell to the left of the placement (for horizontal orientation)
        if u0 > 0 {
            self.filter_incompat(place_y-dy, place_x-dx, &mut removed, |_| {
                // any placement in this cell is incompatible
                true
            });
        }
        
        // remove incompatible placements in the cell to the right of the placement (for horizontal orientation)
        if u0+len0 < maxu {
            self.filter_incompat(place_y+dy, place_x+dx, &mut removed, |_| {
                // any placement in this cell is incompatible
                true
            });
        }
        
        removed
    }
    
    
    fn filter_incompat<F: Fn(&Placement) -> bool>(&mut self, celly: dim, cellx: dim, removed: &mut Vec<PlacementId>, must_remove: F) {
//        for &entry_id in self.field[MatrixDim(celly, cellx)].iter() {
//            let other = &self.dic[entry_id];
//            if must_remove(other) {
//                self.remove(entry_id);
//                removed.push(entry_id);
//            }
//        }
        
        let to_remove: Vec<PlacementId> = self.field[MatrixDim(celly, cellx)]
                                              .iter()
                                              .filter(|&&entry_id| 
                                                  must_remove(&self.dic[entry_id])
                                              )
                                              .cloned()
                                              .collect();
        for entry_id in to_remove {
            self.remove(entry_id);
            removed.push(entry_id);
        }
    }
    
    
    pub fn remove(&mut self, entry_id: PlacementId) {
        // to satisfy the borrow checker, do 2 passes: first collect indices to remove, then remove them
        let to_remove = {
            let place = &self.dic[entry_id];
            let (place_y, place_x) = (place.y, place.x);
            
            let entry = &self.entries[entry_id];
            let indices = &entry.field_indices;
            
            let init = Vec::with_capacity(place.word.len());
            
            place.fold_positions(init, |mut acc, y, x| {
                let char_idx = (y - place_y) + (x - place_x);
                let idx = indices[char_idx];
                acc.push((y, x, char_idx, idx));
                acc
            })
        };
        
        for (y, x, char_idx, idx) in to_remove {
            let cell = &mut self.field[MatrixDim(y,x)];
            
//            // DEBUG
//            if idx == 999999 {
//                panic!("aasdfasdf!");
//            }
//            self.entries[cell[idx]].field_indices[char_idx] = 999999;

            cell.swap_remove(idx);
            
            if idx != cell.len() {
                let swapped_id = cell[idx];
                let swapped = &self.dic[swapped_id];
                let char_idx2 = y-swapped.y + x-swapped.x;
                self.entries[swapped_id].field_indices[char_idx2] = idx;
            }
        }
    }
}


impl Index<PlacementId> for Vec<Entry> {
    type Output = Entry;

	#[inline]
    fn index(&self, index: PlacementId) -> &Self::Output {
		self.index(index.0 as usize)
    }
}

impl IndexMut<PlacementId> for Vec<Entry> {
	#[inline]
    fn index_mut(&mut self, index: PlacementId) -> &mut Self::Output {
		self.index_mut(index.0 as usize)
    }
}
