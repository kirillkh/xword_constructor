#![allow(non_snake_case)]

use std::mem::size_of;
use std::mem;
use std::ptr;
use std::marker::PhantomData;
use std::cmp::min;
use std::fmt::Debug;

pub trait Key: Sized+Clone+Copy {
    #[inline]
    fn usize(self) -> usize;
}

type NdIndex = usize;

type UpdMarker = usize;

pub trait Item<K: Key>: Clone+Debug {
    #[inline]
    fn key(&self) -> K;
    #[inline]
    fn weight(&self) -> f32;
}

#[derive(Debug)]
struct Node<K: Key, It: Item<K>> {
    item: It,
    upd_marker: UpdMarker,
    total: f32,
    ph: PhantomData<K>
}


#[derive(Debug)]
pub struct WeightedSelectionTree<K: Key, It: Item<K>> {
    data: Vec<Node<K, It>>,
    // mapping from item keys to node array indices 
    keys: Vec<Option<NdIndex>>,  // TODO: to optimize this, get rid of the Option, use index=0 for "missing key"
    
    upd_count: UpdMarker,
}


impl<K: Key, It: Item<K>> WeightedSelectionTree<K, It> {
//    #[inline(never)]
    pub fn new(items: &[It], max_keys: usize) -> WeightedSelectionTree<K, It> {
        let keys: Vec<Option<NdIndex>> = vec![None; max_keys];
        
        // create nodes
        let n = items.len();
        let mut nodes = Vec::with_capacity(n);
        let nodes_ptr = nodes.as_mut_ptr();
        for i in 0..n {
            let it = &items[i];
            let weight = it.weight();
            let node = Node { upd_marker:0, item:it.clone(), total:weight, ph: PhantomData };
            unsafe {
                ptr::write(nodes_ptr.offset(i as isize), node);
            }
        }
        
        unsafe {
            nodes.set_len(n);
        }
        
        // build the tree
        let mut sm = WeightedSelectionTree { data:nodes, upd_count:0, keys:keys };
        
        for (i, item) in items.iter().enumerate() {
            sm.insert_key(item.key(), i);
        }
        
        let levels = sm.levels_count();
        
        // calculate order statistics
        if levels != 0 {
            let level_from = Self::level_from(levels-1);
            sm.update_ancestors_bulk(level_from, 2*level_from + 1);
        }
        
        sm
    }
    
    pub fn total(&self) -> f32 {
        self.data[0].total
    }
    
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
    
    pub fn len(&self) -> usize {
        self.data.len()
    }
    
////    #[inline(never)]
//    fn remove_bulk__items_to_keep(&mut self, rm_indices: &mut Vec<NdIndex>) -> Vec<bool> {
//        let mut displaced_keep = vec![true; rm_indices.len()];
//        let new_len = self.data.len();
//        let mut i = 0;
//        while i < rm_indices.len() {
//            let idx = rm_indices[i];
//            if idx >= new_len {
//                rm_indices.swap_remove(i);
//                displaced_keep[idx - new_len] = false;
//            } else {
//                i += 1;
//            }
//        }
//
//        displaced_keep
//    }

//    #[inline(never)]
//    #[no_mangle]
    pub fn remove_bulk(&mut self, keys: &[K]) -> Vec<It> {
        self.upd_count += 1;

        if keys.is_empty() {
            return vec![];
        }
        let rm_indices: Vec<NdIndex> = keys.iter()
                                           .map(|&k| self.remove_key(k).unwrap())
                                           .collect();
        
        let rm_len = rm_indices.len();
        let new_len = self.data.len() - rm_len;
        
        // 1. Build `displaced_assoc` map of length rm_len as follows. The map fulfills the task: 
        //         "given index i into data[new_len..], find the index into rm_indices that corresponds to the item in data that will be replaced with data[new_len+i]".
        //    Using it, we can then perform two things:
        //      1) move the replaced items from data into a new `removed` vec in the order, in which the items' indices appear in rm_indices
        //      2) move the items from data[new_len..] to replace the removed items in data
        
        // 1.1. for j'th item in data[new_len..], which is requested to be removed: displaced_assoc[j] = i, where i is the corresponding index into rm_indices
        let KEEP = ::std::usize::MAX;
        let mut displaced_assoc = vec![KEEP; rm_len];
        let mut replacing_len = 0;
        for (i, &idx) in rm_indices.iter().enumerate() {
            if idx >= new_len {
                displaced_assoc[idx - new_len] = i;
            } else {
                replacing_len += 1;
            }
        }
        
        // 1.2. for j'th item in displaced_assoc, s.t. it is not requested to be removed: displaced_assoc[j] = <the next element in rm_indices which has not been added already>
        let mut iremoved = 0;
        for i in 0..rm_len {
            if displaced_assoc[i] == KEEP {
                while rm_indices[iremoved] >= new_len {
                    iremoved += 1;
                }
                
                displaced_assoc[i] = iremoved;
                iremoved += 1;
            } // else displaced_assoc[i] is already pointing at the correct index in rm_indices
        }
        
        // 2. At this point, j'th item in displaced_assoc contains index i into rm_indices for item it will replace.
        // Move the replacing items to the replaced ones' positions. While we're at it, also collect the removed items into the `removed` vec.
        let mut upd_set: Vec<NdIndex> = Vec::with_capacity(replacing_len);
//        let mut removed: Vec<It> = Vec::with_capacity(rm_len);
        let removed = self.remove_bulk__copy(&mut upd_set, rm_len, displaced_assoc, rm_indices); 
        
        // update ancestors of the `rm_len` items that were removed/displaced from the end of the vec
        self.update_ancestors_bulk(new_len, new_len+rm_len);
        
        // 3. mark the replaced items to be updated
        let upd_count = self.upd_count;
        for &idx in upd_set.iter() {
            self.data[idx].upd_marker = upd_count;
        }
        
        // 4. update order statistics bottom-up, level by level 
        self.remove_bulk__levels(upd_set);
        
        removed
    }

//    #[inline(never)]
    fn remove_bulk__levels(&mut self, mut upd_set: Vec<NdIndex>) {
        if self.data.len() == 0 { return; }
        
        let levels = self.levels_count();
        let upd_count = self.upd_count;
        
        let leaves_from = self.leaves_from();
        
        for level in (0..levels).rev() {
            let level_from = Self::level_from(level);

            unsafe {
                let mut i = 0;
                let upd_ptr = upd_set.as_mut_ptr();
                let mut len = upd_set.len();
                while i < len {
                    let idx_ptr = upd_ptr.offset(i as isize);
                    let idx = *idx_ptr;
                    let mut keep = false;
                    if idx >= level_from {
                        // the item is at the current level: update the node and insert its parent
                        if idx >= leaves_from {
                            self.update_node_leaf(idx);
                        } else if idx + 1 == leaves_from {
                            self.update_node(idx);
                        } else {
                            self.update_node_complete(idx);
                        }
                        
                        if level > 0 {
                            let parenti = Self::parenti(idx);
                            let parent = self.data.get_unchecked_mut(parenti);
                            if parent.upd_marker != upd_count {
                                parent.upd_marker = upd_count;
                                *idx_ptr = parenti;
                                keep = true;
                            }
                        }
                    } else {
                        // the item is above the current level: just copy it verbatim for future processing
                        *idx_ptr = idx;
                        keep = true;
                    };
                    
                    if keep {
                        i += 1;
                    } else {
                        len -= 1;
                        *idx_ptr = *upd_set.get_unchecked(len);
                    }
                }
                
                upd_set.set_len(len);
            }
        }
    }
    
    
//    #[inline(never)]
    pub fn select_remove(&mut self, x: f32) -> Option<It> {
        let idx = self.find_idx(x, 0);
        let len = self.data.len();
        
        if idx < len {
            let removed = self.remove_idx(idx);
            self.remove_key(removed.key());
            return Some(removed);
        } else {
            return None
        }
    }
    
//    #[inline(never)]
    pub fn remove(&mut self, k: K) -> It {
        let idx: NdIndex = self.remove_key(k).unwrap();
        self.remove_idx(idx)
    }
    
    pub fn insert(&mut self, k: K, it: It) -> Option<It> {
        let &entry = self.index(k);
        let(old, idx) = match entry {
            Some(idx) => {
                let old = mem::replace(&mut self.data[idx].item, it);
                self.update_node(idx);
                (Some(old), idx)
            },
            None => {
                let idx = self.data.len();
                let weight = it.weight();
                self.data.push(Node { upd_marker:self.upd_count, item:it, total:weight, ph:PhantomData });
                self.insert_key(k, idx);
                (None, idx)
            } 
        };
        self.update_ancestors(idx);
        old
    }
    
    #[inline]
    pub fn get_mut(&mut self, k: K) -> Option<&mut It> {
        if let &Some(idx) = self.index(k) {
            Some(&mut self.data[idx].item)
        } else {
            None
        }
    }
    
    
    #[inline]
    pub fn contains_key(&self, k: K) -> bool {
        self.keys[k.usize()].is_some()
    }
    
    #[inline]
    fn insert_key(&mut self, key: K, idx: NdIndex) -> Option<NdIndex> {
        let mut val = Some(idx);
        mem::swap(&mut val, self.index_mut(key));
        val
    }
    
    #[inline]
    fn remove_key(&mut self, key: K) -> Option<NdIndex> {
        let mut val = None;
        mem::swap(&mut val, self.index_mut(key));
        val
    }
    
    #[inline]
    fn index_mut(&mut self, key: K) -> &mut Option<NdIndex> {
        unsafe { self.keys.get_unchecked_mut(key.usize()) }
    }
    
    #[inline]
    fn index(&self, key: K) -> &Option<NdIndex> {
        &self.keys[key.usize()]
    }

    // Note: it's caller's responsibility to check that the keys are not already in the tree.
//    #[inline(never)]
    pub fn insert_bulk(&mut self, entries: Vec<(K, It)>) { // TODO: we don't need to pass keys in Vec<(Key, It)>, we can get keys from the items 
        let len = self.data.len();
        let upd_from = len;
        let upd_to = len + entries.len();
        
        for (i, entry) in entries.into_iter().enumerate() {
            let (key, it) = entry;
            let idx = len + i;
            let old = self.insert_key(key, idx);
            assert!(old.is_none());
            self.data.push(Node { upd_marker:self.upd_count, total:it.weight(), item:it, ph:PhantomData });
        }
        
        self.update_range(upd_from, upd_to);
        self.update_ancestors_bulk(upd_from, upd_to);
    }
    
    
    
//    #[inline(never)]
    fn find_idx(&self, mut x: f32, root: NdIndex) -> NdIndex {
        let mut curr = root;
        let len = self.data.len();
        while curr < len {
            let lefti = Self::lefti(curr);
            let righti = Self::righti(curr);
        
            // does the current node's interval contain x?
            let self_score = unsafe { self.score_at_unchecked(curr) };
            if x <= self_score || lefti >= len {
                break;
            }
            x -= self_score;
            
            // does the left subtree contain x?
            let left_total = self.total_at(lefti);
            if x <= left_total || righti >= len {
                curr = lefti;
                continue;
            }
            x -= left_total;
            
            // the right subtree contains x
            curr = righti;
        }
        
        curr
    }
    
    
//    #[inline(never)]
    fn remove_idx(&mut self, idx: NdIndex) -> It {
        let last = self.remove_last();
        let removed = if idx == self.data.len() {
            last
        } else {
            self.insert_key(last.key(), idx).unwrap();
            let removed = ::std::mem::replace(&mut self.data[idx].item, last);
            self.update_node(idx);
            self.update_ancestors(idx);
            removed
        };
        
        removed
    }
    
    #[inline]
    fn remove_last(&mut self) -> It {
        let last = self.data.len() - 1;
        let Node{ item, upd_marker:_, total:_, ph:_ } = self.data.swap_remove(last);
        self.update_ancestors(last);
        
        item
    }
    
    #[inline]
    fn update_ancestors(&mut self, idx: NdIndex) {
        let mut curr = idx;
        while curr != 0 {
            curr = Self::parenti(curr);
            self.update_node(curr);
        }
    }
    
    
//    #[inline]
    #[inline]
    fn update_range(&mut self, from: NdIndex, to: NdIndex) {
        for i in (from..to).rev() {
            self.update_node(i);
        }
    }
    
    #[inline]
    unsafe fn update_range_complete(&mut self, from: NdIndex, to: NdIndex) {
        for i in (from..to).rev() {
            self.update_node_complete(i);
        }
    }
    
    #[inline]
    unsafe fn update_range_leaves(&mut self, from: NdIndex, to:NdIndex) {
        for i in (from..to).rev() {
            self.update_node_leaf(i);
        }
    }
    
//    #[inline(never)]
    fn update_ancestors_bulk(&mut self, mut changed_from: NdIndex, mut changed_to: NdIndex) {
        // this is a simple algorithm, complicated only by the programmer's obsessive desire to save a few cycles on bounds checking
         
        if changed_from == changed_to || changed_to <= 1 { return; }
        
        // distinguish 2 cases: self-overlapping (when the range spans more than one whole row) and non-overlapping
        let parent_to = Self::parenti(changed_to-1) + 1;
        if changed_from < parent_to {
            // self-overlapping: update the lowest row as a special case and then just update every whole row above it
            let row_start = Self::row_start(changed_from);
            self.update_range(row_start, changed_from);
            changed_from = row_start;
            changed_to = 1 + (row_start << 1);
            
            // special-case one more row so that we can use update_range_unchecked() afterwards
            if changed_from != 0 {
                changed_from = Self::parenti(changed_from);
                changed_to = Self::parenti(changed_to-1) + 1;
                self.update_range(changed_from, changed_to);
            }
        }
        
        // special-case 2 last batches so that we can use update_range_unchecked() afterwards
        if changed_from != 0 && changed_from != changed_to {
            changed_from = Self::parenti(changed_from);
            changed_to = Self::parenti(changed_to-1) + 1;
            
            let leaves_from = min(self.leaves_from(), changed_to);
            let leaves_to = changed_to;
            
            let mut special_to = leaves_from;
            
            unsafe { self.update_range_leaves(leaves_from, leaves_to); }
            
            if changed_from < leaves_from {
                // boundary case: might have 1 or 0 children
                special_to = leaves_from - 1;
                self.update_node(special_to);
            }
            
            // all others from this batch have 2 children
            unsafe { self.update_range_complete(changed_from, special_to); }
            
            // ... and one more batch: need to handle a tricky corner case where the next changed_to == special_to+1
            if changed_from != 0 {
                changed_from = Self::parenti(changed_from);
                changed_to = Self::parenti(changed_to-1) + 1;
                let upd_to = min(changed_to, special_to);
                unsafe { self.update_range_complete(changed_from, upd_to); }
            }
        }
        
        while changed_from != 0 {
            changed_from = Self::parenti(changed_from);
            changed_to = Self::parenti(changed_to-1) + 1;
            unsafe { self.update_range_complete(changed_from, changed_to); }
        }
    }
    
    fn leaves_from(&self) -> NdIndex {
        let len = self.data.len();
        Self::parenti(len) + 1 - (len & 1)
    }
    
    fn leaves_count(&mut self, idx: NdIndex) -> usize {
        if Self::lefti(idx) >= self.data.len() { 0 } else { 1 +
                (if Self::righti(idx) >= self.data.len() { 0 } else { 1 })
        }
    }
    
    // We could just do "total += delta", but that would lead to unstable results due to limited fp precision as we add and remove nodes 
    // under a parent without touchging the parent itself. Prefer the safe way for now.
    #[inline]
    fn update_node(&mut self, idx: NdIndex) {
        unsafe {
            self.data.get_unchecked_mut(idx).total = 
                self.score_at_unchecked(idx) +
                self.total_at(Self::lefti(idx)) +
                self.total_at(Self::righti(idx));
        }
    }
    
    #[inline]
    unsafe fn update_node_leaf(&mut self, idx: NdIndex) {
        self.data.get_unchecked_mut(idx).total = self.score_at_unchecked(idx);
    }
    
    #[inline]
    unsafe fn update_node_complete(&mut self, idx: NdIndex) {
        self.data.get_unchecked_mut(idx).total = 
            self.score_at_unchecked(idx) +
            self.total_at_unchecked(Self::lefti(idx)) +
            self.total_at_unchecked(Self::righti(idx));
    }
    
    
    #[inline]
    unsafe fn score_at_unchecked(&self, idx: NdIndex) -> f32 {
        self.data.get_unchecked(idx).item.weight()
    }
    
    #[inline]
    fn score_at(&self, idx: NdIndex) -> f32 {
        if idx < self.data.len() {
            unsafe { self.score_at_unchecked(idx) }
        } else {
            0.
        }
    }
    
    #[inline]
    unsafe fn total_at_unchecked(&self, idx: NdIndex) -> f32 {
        self.data.get_unchecked(idx).total
    }
    
    #[inline]
    fn total_at(&self, idx: NdIndex) -> f32 {
        if idx < self.data.len() {
            unsafe { self.total_at_unchecked(idx) }
        } else {
            0.
        }
    }
    
    #[inline]
    fn levels_count(&self) -> usize {
        if self.data.is_empty() {
            0
        } else {
            Self::level_of(self.data.len()-1) + 1
        }
    }
    
    #[inline]
    fn level_from(level: usize) -> NdIndex {
        (1 << level) - 1
    }
    
    #[inline]
    fn level_of(idx: NdIndex) -> usize {
        size_of::<usize>()*8 - ((idx+1).leading_zeros() as usize) - 1
    }
    
    #[inline]
    fn row_start(idx: NdIndex) -> NdIndex {
        Self::level_from(Self::level_of(idx))
    }
    
//    fn make_hash_map(n: usize) -> HashMap<K, usize, MHasher> {
//        HashMap::with_capacity_and_hasher(2*n.next_power_of_two(), Default::default())
//    }
    

    
    fn parenti(idx: NdIndex) -> NdIndex {
        (idx-1) >> 1
    }
    
    fn lefti(idx: NdIndex) -> NdIndex {
        (idx<<1) + 1
    }
    
    fn righti(idx: NdIndex) -> NdIndex {
        (idx<<1) + 2
    }
}



trait WeightedSelectionTreeCopy<K: Key, It: Item<K>> {
    fn remove_bulk__copy(&mut self, upd_set:&mut Vec<NdIndex>, rm_len:usize, displaced_assoc: Vec<usize>, rm_indices: Vec<usize>) -> Vec<It>;
}


impl<K: Key, It: Item<K>> WeightedSelectionTreeCopy<K, It> for WeightedSelectionTree<K, It> {
//    #[inline(never)]
    default fn remove_bulk__copy(&mut self, upd_set:&mut Vec<NdIndex>, rm_len:usize, displaced_assoc: Vec<usize>, rm_indices: Vec<usize>) -> Vec<It> {
        let mut removed = Vec::with_capacity(rm_len);
        
        let new_len = self.data.len() - rm_len; 
        let data_ptr = self.data.as_mut_ptr(); // we need the pointers to overcome the borrow checker that otherwise complains about two mutable references to data
        let removed_ptr = removed.as_mut_ptr();
        
        for i in 0..rm_len {
            let iremoved = displaced_assoc[i];
            let rm_ptr = unsafe { removed_ptr.offset(iremoved as isize) };
            let src_idx = rm_indices[iremoved];
            
            unsafe {
                let src_ptr = &mut (*data_ptr.offset(src_idx as isize)).item;
                ptr::copy_nonoverlapping(src_ptr, rm_ptr, 1);
                
                if src_idx < new_len {
                    let displaced = &(*data_ptr.offset((new_len+i) as isize)).item;
                    self.keys[displaced.key().usize()] = Some(src_idx);
                    ptr::copy_nonoverlapping(displaced, src_ptr, 1);
                    upd_set.push(src_idx);
                }
            }
        }
        
        unsafe {
            self.data.set_len(new_len);
            removed.set_len(rm_len);
        }
        
        removed
    }
}




use common::{PlacementId, Align64};
use super::data::ScoredMove;

struct AlignedItem {
    it: ScoredMove,
    
    _align: [Align64;0],
}

// TODO: this does not currently provide any performance benefits. See https://github.com/rust-lang/rust/issues/33923
impl WeightedSelectionTreeCopy<PlacementId, ScoredMove> for WeightedSelectionTree<PlacementId, ScoredMove> {
//    #[inline(never)]
    fn remove_bulk__copy(&mut self, upd_set:&mut Vec<NdIndex>, rm_len:usize, displaced_assoc: Vec<usize>, rm_indices: Vec<usize>) -> Vec<ScoredMove> {
//        println!("sz(scmv)={}, sz(al)={}", mem::size_of::<ScoredMove>(), mem::size_of::<AlignedItem>());
        let mut removed: Vec<AlignedItem> = Vec::with_capacity(rm_len);
        
        let new_len = self.data.len() - rm_len; 
        let data_ptr = self.data.as_mut_ptr(); // we need the pointers to overcome the borrow checker that otherwise complains about two mutable references to data
        let removed_ptr = removed.as_mut_ptr();
        
        for i in 0..rm_len {
            let iremoved = displaced_assoc[i];
            let rm_ptr: *mut AlignedItem = unsafe { removed_ptr.offset(iremoved as isize) };
            let src_idx = rm_indices[iremoved];
            unsafe {
                let src_ptr: *mut ScoredMove = &mut (*data_ptr.offset(src_idx as isize)).item;
                let src: ScoredMove = ptr::read(src_ptr);
                let src: AlignedItem = mem::transmute(src);
                ptr::write(rm_ptr, src);
                
                if src_idx < new_len {
                    let displaced = &(*data_ptr.offset((new_len+i) as isize)).item;
                    self.keys[displaced.key().usize()] = Some(src_idx);
                    ptr::copy_nonoverlapping(displaced, src_ptr, 1);
                    upd_set.push(src_idx);
                }
            }
        }
        
        unsafe {
            self.data.set_len(new_len);
            removed.set_len(rm_len);
            
            mem::transmute(removed)
        }
    }
}





#[cfg(test)]
mod tests {
    use super::WeightedSelectionTree;
    use super::Item;
    use super::Key;
    use test::Bencher;
    use test::black_box;
    
    
    impl Key for i32 {
        fn usize(self) -> usize { self as usize }
    }
    
    #[derive(Debug, PartialEq, Clone)]
    struct Item0 {
        key: i32,
        weight: f32
    }
    
    impl Item0 {
        fn new(key: i32) -> Item0 {
            Item0 { key:key, weight: key as f32 }
        }
    }
    
    impl Item<i32> for Item0 {
        #[inline]
        fn key(&self) -> i32 {
            self.key
        }
        
        #[inline]
        fn weight(&self) -> f32 {
            self.weight
        }
    }
    
    fn items(v: Vec<i32>) -> Vec<Item0> {
        v.into_iter().map(|k| Item0::new(k)).collect::<Vec<_>>()
    }
    
    fn sm_items(v: Vec<i32>) -> WeightedSelectionTree<i32, Item0> {
        let items = items(v);
        WeightedSelectionTree::new(&items, items.len())
    }
    
    fn check_tree<It: Item<i32>>(sm: &mut WeightedSelectionTree<i32, It>) -> bool {
        for i in (0..sm.data.len()).rev() {
            let old = sm.data[i].total;
            sm.update_node(i);
            if old != sm.data[i].total {
                return false;
            }
        }
        
        true
    }
    
    
    #[test]
    fn test_levels() {
//        assert!(levels_for_len(0) == 0, "levels={}", levels_for_len(0));
//        assert!(levels_for_len(1) == 1);
//        assert!(levels_for_len(2) == 2);
//        assert!(levels_for_len(3) == 2);
//        assert!(levels_for_len(4) == 3);
//        assert!(levels_for_len(5) == 3);
//        assert!(levels_for_len(6) == 3);
//        assert!(levels_for_len(7) == 3);
//        assert!(levels_for_len(8) == 4);
//        assert!(levels_for_len(8) == 4);
    }
    
    
    
    fn new_n_testcase(nkeys: usize) -> WeightedSelectionTree<i32, Item0> {
        let keys = (0..nkeys as i32).into_iter().collect::<Vec<i32>>();
        sm_items(keys)
    }
    
    #[test]
    fn test_new() {
        let mut sm = new_n_testcase(6);
        assert!(check_tree(&mut sm));
    }
    
//    #[test]
//    fn test_displace_last_n() {
//        let mut sm = sm_items(vec![0, 1, 2, 3]);
//        assert!(sm.displace_last_n(3) == items(vec![1, 2, 3]));
//        
//        let mut sm = sm_items(vec![0, 1, 2, 3]);
//        assert!(sm.displace_last_n(1) == items(vec![3]));
//        
//        let mut sm = sm_items(vec![0, 1, 2, 3]);
//        assert!(sm.displace_last_n(2) == items(vec![2, 3]));
//        
//        let mut sm = sm_items(vec![0, 1, 2, 3]);
//        assert!(sm.displace_last_n(4) == items(vec![0, 1, 2, 3]));
//        
//        let mut sm = sm_items(vec![0, 1, 2, 3, 4, 5, 6, 7, 8]);
//        assert!(sm.displace_last_n(4) == items(vec![5, 6, 7, 8]));
//        assert!(sm.data.len() == 9-4);
//        assert!(check_tree(&mut sm));
//        
//        println!("{:?}", sm)
//    }
    
    fn remove_bulk_testcase(nkeys: usize, remove: Vec<i32>) {
        let keys = (0..nkeys as i32).into_iter().collect::<Vec<i32>>();
        let mut sm = sm_items(keys.clone());
        
        sm.remove_bulk(&remove);
        
        let should_be_present = keys.iter().map(|k| !remove.contains(k)).collect::<Vec<_>>();
        let mut present = vec![false; nkeys];
         
        for node in sm.data.iter() {
            present[node.item.key as usize] = true; 
        }
        assert!(present == should_be_present);
        assert!(check_tree(&mut sm));
    }
    
    #[test]
    fn test_remove_bulk() {
        remove_bulk_testcase(1, vec![0]);
        remove_bulk_testcase(2, vec![0]);
        remove_bulk_testcase(3, vec![0]);
        remove_bulk_testcase(4, vec![0]);
        remove_bulk_testcase(5, vec![0]);
        remove_bulk_testcase(6, vec![0]);
        remove_bulk_testcase(7, vec![0]);
        remove_bulk_testcase(8, vec![0]);
        remove_bulk_testcase(9, vec![0]);
        remove_bulk_testcase(10, vec![0]);
        remove_bulk_testcase(11, vec![0]);
        
        remove_bulk_testcase(9, vec![1, 3, 2, 8]);
        remove_bulk_testcase(16, vec![15, 14, 13, 12, 11, 10, 9, 8, 7]);
        remove_bulk_testcase(16, vec![7, 8, 9, 10, 11, 12, 13, 14]);
        remove_bulk_testcase(16, vec![7, 8, 9, 10, 11, 12, 13, 14].into_iter().rev().collect::<Vec<_>>());
        remove_bulk_testcase(16, vec![7, 8, 9, 10, 11, 12, 13, 14, 15]);
        remove_bulk_testcase(16, vec![15, 12, 11, 10, 9, 8, 7, 14, 13]);
        remove_bulk_testcase(16, vec![0]);
        remove_bulk_testcase(16, vec![0, 1]);
        remove_bulk_testcase(16, vec![0, 1, 2]);
        remove_bulk_testcase(16, vec![0, 1, 2, 3]);
        remove_bulk_testcase(16, vec![0, 1, 2, 3, 4]);
        remove_bulk_testcase(16, vec![4, 3, 2, 1, 0]);
        remove_bulk_testcase(16, vec![15, 7, 3, 1, 0]);
        remove_bulk_testcase(16, vec![15, 7]);
        remove_bulk_testcase(16, vec![15, 8]);
        remove_bulk_testcase(16, vec![7, 15]);
        remove_bulk_testcase(16, vec![8, 15]);
        remove_bulk_testcase(16, vec![14, 6]);
        remove_bulk_testcase(16, vec![14, 5]);
        remove_bulk_testcase(16, vec![13, 6]);
        remove_bulk_testcase(16, vec![13, 5]);
        remove_bulk_testcase(16, vec![13, 4]);
        remove_bulk_testcase(16, vec![6, 14]);
        remove_bulk_testcase(16, vec![5, 14]);
        remove_bulk_testcase(16, vec![6, 13]);
        remove_bulk_testcase(16, vec![5, 13]);
        remove_bulk_testcase(16, vec![4, 13]);
        remove_bulk_testcase(9, vec![1, 3, 2, 8]);
    }
    
    
    
    fn remove_bulk_benchcase(nkeys: usize, remove: Vec<i32>) {
        let keys = (0..nkeys as i32).into_iter().collect::<Vec<i32>>();
        let mut sm = black_box(sm_items(keys));
        
        sm.remove_bulk(&remove);
    }
    
    #[bench]
    fn remove_bulk_bench(bencher: &mut Bencher) {
        bencher.iter(|| 
            for i in 0..10000 {
                black_box(i);
                remove_bulk_benchcase(16, vec![15, 14, 13, 12, 11, 10, 9, 8, 7])
            }
        )
    }
}
