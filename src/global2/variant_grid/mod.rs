mod shrink_vec;

use self::shrink_vec::ShrinkVec;

use ndarray::{OwnedArray, Ix};
use common::{dim, Placement, PlacementId, Cond};
use std::rc::Rc;
use std::ops::{Index, IndexMut};
use std::mem;
use std::cmp::{max, min};
use super::sliced_arena::SlicedArena;

const REMOVED: usize = !0;




// TODO: rewrite using R-Tree or Interval Tree?
pub struct VariantGrid {
    // backing storage for cell shrink_vecs
    cell_slices: SlicedArena<PlacementId>,
    
    // cells are arrays of PlacementId's in any order, as long as the invariant is satisfied: 
    // for all cells and all i: entries[cell[i]][corresponding char index] == i
	field: OwnedArray<ShrinkVec<'static, PlacementId>, (Ix, Ix)>,
	
    // map from PlacementId to (map from char_idx to (index into the containing cell))
    entries: SlicedArena<usize>,
    
    // reference map, it never changes
    places: Rc<Vec<Placement>>,
    
    
    tmp_removed: Vec<PlacementId>,
}

impl Clone for VariantGrid {
    fn clone(&self) -> VariantGrid {
        let mut slices = self.cell_slices.clone();
        let entries = self.entries.clone();
        let places = self.places.clone();
        
        // make sure the pointers inside ShrinkVecs are to the cloned cell_slices arena
        let (h, w) = self.field.dim();
        let field = Self::build_grid(h, w, &mut slices);
        
        VariantGrid { cell_slices:slices, entries:entries, field:field, places:places, tmp_removed:vec![] }
    }
}

impl VariantGrid {
    fn build_grid(h: dim, w: dim, cell_slices: &mut SlicedArena<PlacementId>) -> OwnedArray<ShrinkVec<'static, PlacementId>, (Ix, Ix)> {
        let empty: &mut [PlacementId] = &mut [];
        let default_sv = ShrinkVec::new(empty);
        let mut field: OwnedArray<ShrinkVec<PlacementId>, (Ix, Ix)> = OwnedArray::from_elem((h, w), default_sv);
        
        for y in 0..h {
            for x in 0..w {
                let slice: &'static mut [PlacementId] = unsafe {
                    let slice: &mut [PlacementId] = &mut cell_slices[y*w + x];
                    mem::transmute(slice)
                };
//                assert!(slice.len() == cell_counts[(y,x)]); // DEBUG
                field[(y, x)] = ShrinkVec::new(slice);
            }
        }
        
        field
    }
    
    #[inline(never)]
    pub fn new(places: Rc<Vec<Placement>>, h: dim, w: dim) -> VariantGrid {
        // 1. build the entries arena
        let place_word_lens = places.iter().map(|place| place.word.len()).collect::<Vec<_>>();
        let mut entries: SlicedArena<usize> = SlicedArena::new(&place_word_lens);
        
        // 2. calculate the number of placements at each grid cell
        let cell_counts: OwnedArray<usize, (Ix, Ix)> = Self::places_per_cell(&*places, h, w);
        
        // 3. build the sliced array for backing storage of cell vecs
        let mut cell_slices = SlicedArena::new(cell_counts.as_slice().unwrap());
        
        // 4. build the grid
        let mut field: OwnedArray<ShrinkVec<PlacementId>, (Ix, Ix)> = Self::build_grid(h, w, &mut cell_slices);
        
        // 5. initialize the cell vecs and, in the same pass, entires for each placement 
        for (i, place) in places.iter().enumerate() {
            let entry = &mut entries[i];
            
            place.fold_positions_index((), |(), y, x, char_idx| {
                let cell_entries: &mut ShrinkVec<PlacementId> = &mut field[(y, x)];
                
                let counter: usize = unsafe {
                    let last_idx = cell_entries.len() - 1;
                    let cnt = cell_entries.get_unchecked_mut(last_idx); // storing the size in the last element is a hack, but it's temptingly easy
                    let counter = cnt.0;
                    cnt.0 += 1;
                    counter
                };
                entry[char_idx] = counter;
                debug_assert!(place.id.0 == i);
                cell_entries[counter] = PlacementId(i);
            });
        }
        
        VariantGrid { field: field, cell_slices:cell_slices, entries: entries, places: places, tmp_removed: vec![] }
    }
    
    
    fn places_per_cell(places: &Vec<Placement>, h: dim, w: dim) -> OwnedArray<usize, (Ix, Ix)> {
        let mut field = OwnedArray::from_elem((h, w), 0);
        for place in places {
            place.fold_positions((), |(), y, x| {
                field[(y, x)] += 1;
            });
        }
        
        field
    }
    
    
    
    #[inline]
    pub fn iter_at(&self, y: dim, x: dim) -> ::std::slice::Iter<PlacementId> {
        self.field[(y, x)].iter()
    }
    
//    #[inline(never)]
    pub fn remove_incompat(&mut self, place_id: PlacementId) -> Vec<PlacementId> {
        let dic = self.places.clone();
        let place = &dic[place_id];
        let (place_y, place_x) = (place.y, place.x);
        let (yc, xc) = place.orientation.align(0, 1);
        let (perpyc, perpxc) = (xc, yc);
        
        let (v0, u0) = place.align(place.orientation);
        let len0 = place.word.len() as dim;
        let dim = self.field.dim();
        let (maxv, maxu) = place.orientation.align(dim.0, dim.1);
        
//        let mut removed = vec![]; // TODO cache
        let mut removed = Vec::with_capacity(self.places.len());
        
        place.fold_positions_index((), |(), y, x, char_idx| {
            // remove incompatible placements in the current cell
            self.filter_incompat(y, x, &mut removed, |other| {
                    let c_idx = (y - other.y) + (x - other.x);
                    
                    place.orientation == other.orientation ||
                    place.word[char_idx] != other.word[c_idx]
            });
            
            // remove incompatible placements in the cell above the current cell (for horizontal orientation)
            // the only case we need to handle here is when "other" is perpendicular to "place" and touches it, but does not intersect
            if v0 > 0 {
                self.filter_incompat(y-perpyc, x-perpxc, &mut removed, |other| {
                    let (v1, u1) = other.align(place.orientation);
                    let len1 = other.word.len();
                    
                    (place.orientation != other.orientation && 
                     v1 + len1 == v0)	// touches from above but does not intersect => incompatible
//                    // TEST
//                    || place.orientation == other.orientation && {
//                        let overlap_from = max(u0, u1);
//                        let overlap_to = min(u0+len0, u1+len1);
//                        overlap_from < overlap_to-1
//                    }
                });
            }
            
            // remove incompatible placements in the cell below the current cell (for horizontal orientation)
            if v0 + 1 < maxv {
                self.filter_incompat(y+perpyc, x+perpxc, &mut removed, |other| {
                    let (v1, u1) = other.align(place.orientation);
                    let len1 = other.word.len();
                    
                    (place.orientation != other.orientation && 
                     v1 == v0 + 1) // touches from below but does not intersect => incompatible
//                    // TEST
//                    || place.orientation == other.orientation && {
//                        let overlap_from = max(u0, u1);
//                        let overlap_to = min(u0+len0, u1+len1);
//                        overlap_from < overlap_to-1
//                    }
                });
            }
        });
        
        
            
        // remove incompatible placements in the cell to the left of the placement (for horizontal orientation)
        if u0 > 0 {
            self.filter_incompat(place_y-yc, place_x-xc, &mut removed, |_| {
                // any placement in this cell is incompatible
                true
            });
        }
        
        // remove incompatible placements in the cell to the right of the placement (for horizontal orientation)
        if u0+len0 < maxu {
            self.filter_incompat(place_y+len0.cond(yc), place_x+len0.cond(xc), &mut removed, |_| {
                // any placement in this cell is incompatible
                true
            });
        }
        
        removed
    }
    
    
    // this is quite dirty
    pub fn contains(&self, place_id: PlacementId) -> bool {
        self.entries[place_id][0] != REMOVED
    }
    
//    #[inline(never)]
    fn filter_incompat<F: Fn(&Placement) -> bool>(&mut self, celly: dim, cellx: dim, removed: &mut Vec<PlacementId>, must_remove: F) {
//        let to_remove: Vec<PlacementId> = self.iter_at(celly, cellx)
//                                              .filter(|&&entry_id|
//                                                  must_remove(&self.places[entry_id])
//                                              )
//                                              .cloned()
//                                              .collect();


        let tmp_removed: &mut Vec<PlacementId> = unsafe {
            let ptr: *mut Vec<PlacementId> = &mut self.tmp_removed;
            mem::transmute(ptr)
        };
        debug_assert!(tmp_removed.is_empty());
        tmp_removed.reserve_exact(self.field[(celly, cellx)].len());
        
        for &entry_id in self.field[(celly, cellx)].iter() {
            if must_remove(&self.places[entry_id]) {
                tmp_removed.push(entry_id);
                removed.push(entry_id);
            }
        }
        
        for &entry_id in tmp_removed.iter() {
            self.remove(entry_id);
        }
        
        unsafe {
            tmp_removed.set_len(0);
        }
    }
    
    
    pub fn remove(&mut self, entry_id: PlacementId) {
        // this function is the hottest part of the whole program; it is heavily optimized
        
        // must transmute place and entry with static lifetimes to get around the borrow checker
        let place: &'static Placement = unsafe {
            let place: &Placement = self.places.get_unchecked(entry_id.0);
            mem::transmute(place)
        };
        
        let entry: &'static mut [usize] = unsafe {
            let entry: &mut [usize] = self.entries.slice_unchecked_mut(entry_id.0);
            mem::transmute(entry)
        };
        
        place.fold_positions_index((), |(), y, x, char_idx| unsafe {
            let incell_idx = *entry.get_unchecked(char_idx);
            
            let mut cell: &mut ShrinkVec<PlacementId> = self.field.uget_mut((y,x));
            
            // DEBUG
            debug_assert!(incell_idx != REMOVED);
            debug_assert!(cell[incell_idx] == entry_id);
            *entry.get_unchecked_mut(char_idx) = REMOVED;
             
            cell.swap_remove_unchecked(incell_idx);

            if incell_idx != cell.len() {
                let swapped_id = *cell.get_unchecked(incell_idx);
                let swapped = self.places.get_unchecked(swapped_id.0);
                let char_idx2 = y-swapped.y + x-swapped.x;
                
                let mut entry2: &mut [usize] = self.entries.slice_unchecked_mut(swapped_id.0); 
                *entry2.get_unchecked_mut(char_idx2) = incell_idx;
            }
        });
    }
}


impl<T: Sized+Clone> Index<PlacementId> for SlicedArena<T> {
    type Output = [T];

	#[inline]
    fn index(&self, index: PlacementId) -> &Self::Output {
		self.index(index.0 as usize)
    }
}

impl<T: Sized+Clone> IndexMut<PlacementId> for SlicedArena<T> {
	#[inline]
    fn index_mut(&mut self, index: PlacementId) -> &mut Self::Output {
		self.index_mut(index.0 as usize)
    }
}
