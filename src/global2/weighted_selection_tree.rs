use global2::data::ScoredMove;
use std::collections::{HashMap, HashSet};
use std::cmp::min;
use std::mem::size_of;
use std::hash::BuildHasher;
use std::hash::BuildHasherDefault;
use fnv::FnvHasher;


//use std::hash::SipHasher;
//type MHasher = BuildHasherDefault<SipHasher>; // remove_n_bench(): 11.7 ms/iter
type MHasher = BuildHasherDefault<FnvHasher>; // remove_n_bench(): 7.5 ms/iter


pub trait Item {
    #[inline]
    fn key(&self) -> i32;
    #[inline]
    fn weight(&self) -> f32;
}

#[derive(Debug)]
struct Node<W: Item> {
    idx: usize,
    item: W,
    total: f32
}


#[derive(Debug)]
pub struct WeightedSelectionTree<W: Item> {
    data: Vec<Node<W>>,
    // mapping from item keys to node array indices 
    keys: HashMap<i32, usize, MHasher>,
}

impl<W: Item> WeightedSelectionTree<W> {
    pub fn new(items: Vec<W>) -> WeightedSelectionTree<W> {
        let mut keys: HashMap<i32, usize, MHasher> = Self::make_hash_map(items.len());
        for (i, item) in items.iter().enumerate() {
            keys.insert(item.key(), i);
        }
        
        let nodes = items.into_iter().enumerate().map(
            |(i, it)| {
                let weight = it.weight();
                Node { idx:i, item:it, total:weight }
            }
        ).collect();
        
        let mut sm = WeightedSelectionTree { data:nodes, keys:keys };
        let levels = sm.levels_count();
        let len = sm.data.len();
        
         for level in (0..levels).rev() {
             let level_from = Self::level_from(level);
            if level+1==levels {
                continue;
            }
            
            let level_count = min(level_from + 1, len - level_from);
            for i in 0..level_count {
                sm.update_node(level_from + i);
            }
        }
        
        sm
    }
    
    pub fn total(&self) -> f32 {
        self.data[0].total
    }
    
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
    
    fn make_hash_map(n: usize) -> HashMap<i32, usize, MHasher> {
        HashMap::with_capacity_and_hasher(n.next_power_of_two(), Default::default())
    }
    
    fn make_hash_set(n: usize) -> HashSet<usize, MHasher> {
         HashSet::with_capacity_and_hasher(n.next_power_of_two(), Default::default())
    }
    
    pub fn remove_n(&mut self, keys: &[i32]) {
        let mut rm_indices: Vec<usize> = keys.iter()
                                              .map(|k| self.keys.remove(k).unwrap())
                                              .collect();
         
         let len = self.data.len();
         let n = rm_indices.len();
         
        assert!(n <= len && n != 0);
         
        // 1. remove the last n items and update their parents
        let mut displaced_items = self.remove_last_n(n);
        
        // 2. discard any items that were among the last n from rm_indices
        let mut displaced_keep = vec![true; n];
        let new_len = self.data.len();
        let mut i = 0;
        while i < rm_indices.len() {
            let idx = rm_indices[i];
            if idx >= new_len {
                rm_indices.swap_remove(i);
                displaced_keep[idx - new_len] = false;
            } else {
                i += 1;
            }
        }
         
        let mut displaced_filtered = Vec::with_capacity(n);
        for (i, item) in displaced_items.into_iter().enumerate() {
            if displaced_keep[i] {
                displaced_filtered.push(item);
            }
        }
        
        assert!(displaced_filtered.len() == rm_indices.len());
        
        // 3. replace the removed items with the displaced ones and initialize the update set
        let mut upd_set = Self::make_hash_set(displaced_filtered.len());
        for (i, item) in displaced_filtered.into_iter().enumerate() {
            let idx = rm_indices[i];
            upd_set.insert(idx);
            self.data[idx].item = item;
        }
         
        // 4. update order statistics bottom-up, level by level 
        let mut upd_vec: Vec<usize> = Vec::with_capacity(upd_set.len());
        let levels = self.levels_count();

         for level in (0..levels).rev() {
             let level_from = Self::level_from(level);

             // move the indices into an array and clear the set
             for idx in upd_set.drain() {
                 upd_vec.push(idx);
             }
             
             for idx in upd_vec.drain(..) {
                 if idx >= level_from {
                     // the item is at the current level: update the node and insert its parent
                     self.update_node(idx);
                     if level > 0 {
                         upd_set.insert(parenti(idx));
                     }
                 } else {
                     // the item is above the current level: just copy it verbatim for future processing
                     upd_set.insert(idx);
                 }
             }
        }
    }


    fn level_from(level: usize) -> usize {
        (1 << level) - 1
    }
    
    
    pub fn remove_last_n(&mut self, n: usize) -> Vec<W> {
        let len = self.data.len();
        let mut upd_from = len - n;
        let mut upd_to = len;
        
        let mut displaced_items = Vec::with_capacity(n);
        for i in upd_from..upd_to {
            let node = self.data.pop().unwrap();
            displaced_items.push(node.item);
        }
        
        while upd_from != 0 {
            upd_from = parenti(upd_from);
            upd_to = parenti(upd_to-1) + 1;
            // corner case: we may have removed parents of removed items (when wrapping back to the prev row)
            let upd_to_ex = min(upd_to, self.data.len());
            
            for i in (upd_from..upd_to_ex).rev() {
                self.update_node(i);
            }
        }
        
        displaced_items.reverse(); // make sure to preserve the order
        displaced_items
    }
    
    
    
    pub fn select_remove(&mut self, x: f32) -> Option<W> {
        let idx = self.find(x, 0);
        let len = self.data.len();
        
        if idx < len {
            return Some(self.remove(idx));
        } else {
            return None
        }
    }
    
    
    fn find(&self, mut x: f32, root: usize) -> usize {
        let mut curr = root;
        let len = self.data.len();
        while curr < len {
            let lefti = lefti(root);
            let righti = righti(root);
        
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
    
    
    fn remove(&mut self, idx: usize) -> W {
        let last = self.remove_last();
        let removed = ::std::mem::replace(&mut self.data[idx].item, last);
        self.update_ancestors(idx);
        
        removed
    }
    
    fn remove_last(&mut self) -> W {
        let curr = self.data.len() - 1;
        let node = self.data.remove(curr);
        self.update_ancestors(parenti(curr));
        
        node.item
    }
    
    fn update_ancestors(&mut self, idx: usize) {
        let mut curr = idx;
        while curr > 0 {
            self.update_node(curr);
            curr = parenti(curr);
        }
        self.update_node(0);
    }
    
    // We could just do "total += delta", but that would lead to unstable results due to limited fp precision as we add and remove nodes 
    // under a parent without touchging the parent itself. Prefer the safe way for now.
    fn update_node(&mut self, idx: usize) {
        self.data[idx].total = 
            unsafe { self.score_at_unchecked(idx) } +
            self.total_at(lefti(idx)) +
            self.total_at(righti(idx));
    }
    
    
    #[inline]
    unsafe fn score_at_unchecked(&self, idx: usize) -> f32 {
        self.data[idx].item.weight()
    }
    
    #[inline]
    fn score_at(&self, idx: usize) -> f32 {
        if idx < self.data.len() {
            unsafe { self.score_at_unchecked(idx) }
        } else {
            0.
        }
    }
    
    #[inline]
    unsafe fn total_at_unchecked(&self, idx: usize) -> f32 {
        self.data[idx].total
    }
    
    #[inline]
    fn total_at(&self, idx: usize) -> f32 {
        if idx < self.data.len() {
            unsafe { self.total_at_unchecked(idx) }
        } else {
            0.
        }
    }
    
    #[inline]
    fn levels_count(&self) -> usize {
        size_of::<usize>()*8 - (self.data.len().leading_zeros() as usize)
    }
}


    
fn parenti(idx: usize) -> usize {
    (idx-1) >> 1
}

fn lefti(idx: usize) -> usize {
    (idx<<1) + 1
}

fn righti(idx: usize) -> usize {
    (idx<<1) + 2
}


fn left<W: Item>(node: &Node<W>) -> usize {
    lefti(node.idx)
}

fn right<W: Item>(node: &Node<W>) -> usize {
    righti(node.idx)
}



#[cfg(test)]
mod tests {
    use super::WeightedSelectionTree;
    use super::Item;
    use test::Bencher;
    use test::black_box;
    
    #[derive(Debug, PartialEq)]
    struct Item0 {
        key: i32,
        weight: f32
    }
    
    impl Item0 {
        fn new(key: i32) -> Item0 {
            Item0 { key:key, weight: key as f32 }
        }
    }
    
    impl Item for Item0 {
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
    
    fn sm_items(v: Vec<i32>) -> WeightedSelectionTree<Item0> {
        let items = items(v);
        WeightedSelectionTree::new(items)
    }
    
    fn check_tree<W: Item>(sm: &mut WeightedSelectionTree<W>) -> bool {
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
    
    
    
    fn new_n_testcase(nkeys: usize) -> WeightedSelectionTree<Item0> {
        let keys = (0..nkeys as i32).into_iter().collect::<Vec<i32>>();
        sm_items(keys)
    }
    
    #[test]
    fn test_new() {
        let mut sm = new_n_testcase(6);
        assert!(check_tree(&mut sm));
    }
    
    #[test]
    fn test_remove_last_n() {
        let mut sm = sm_items(vec![0, 1, 2, 3]);
        assert!(sm.remove_last_n(1) == items(vec![3]));
        
        let mut sm = sm_items(vec![0, 1, 2, 3]);
        assert!(sm.remove_last_n(2) == items(vec![2, 3]));
        
        let mut sm = sm_items(vec![0, 1, 2, 3]);
        assert!(sm.remove_last_n(3) == items(vec![1, 2, 3]));
        
        let mut sm = sm_items(vec![0, 1, 2, 3]);
        assert!(sm.remove_last_n(4) == items(vec![0, 1, 2, 3]));
        
        let mut sm = sm_items(vec![0, 1, 2, 3, 4, 5, 6, 7, 8]);
        assert!(sm.remove_last_n(4) == items(vec![5, 6, 7, 8]));
        assert!(sm.data.len() == 9-4);
        assert!(check_tree(&mut sm));
        
        println!("{:?}", sm)
    }
    
    fn remove_n_testcase(nkeys: usize, remove: Vec<i32>) {
        let keys = (0..nkeys as i32).into_iter().collect::<Vec<i32>>();
        let mut sm = sm_items(keys.clone());
        
        sm.remove_n(&remove);
        
        let should_be_present = keys.iter().map(|k| !remove.contains(k)).collect::<Vec<_>>();
        let mut present = vec![false; nkeys];
         
        for node in sm.data.iter() {
            present[node.item.key as usize] = true; 
        }
        assert!(present == should_be_present);
        assert!(check_tree(&mut sm));
    }
    
    #[test]
    fn test_remove_n() {
        remove_n_testcase(1, vec![0]);
        remove_n_testcase(2, vec![0]);
        remove_n_testcase(3, vec![0]);
        remove_n_testcase(4, vec![0]);
        remove_n_testcase(5, vec![0]);
        remove_n_testcase(6, vec![0]);
        remove_n_testcase(7, vec![0]);
        remove_n_testcase(8, vec![0]);
        remove_n_testcase(9, vec![0]);
        remove_n_testcase(10, vec![0]);
        remove_n_testcase(11, vec![0]);
        
        remove_n_testcase(9, vec![1, 3, 2, 8]);
        remove_n_testcase(16, vec![15, 14, 13, 12, 11, 10, 9, 8, 7]);
        remove_n_testcase(16, vec![7, 8, 9, 10, 11, 12, 13, 14]);
        remove_n_testcase(16, vec![7, 8, 9, 10, 11, 12, 13, 14].into_iter().rev().collect::<Vec<_>>());
        remove_n_testcase(16, vec![7, 8, 9, 10, 11, 12, 13, 14, 15]);
        remove_n_testcase(16, vec![15, 12, 11, 10, 9, 8, 7, 14, 13]);
        remove_n_testcase(16, vec![0]);
        remove_n_testcase(16, vec![0, 1]);
        remove_n_testcase(16, vec![0, 1, 2]);
        remove_n_testcase(16, vec![0, 1, 2, 3]);
        remove_n_testcase(16, vec![0, 1, 2, 3, 4]);
        remove_n_testcase(16, vec![4, 3, 2, 1, 0]);
        remove_n_testcase(16, vec![15, 7, 3, 1, 0]);
        remove_n_testcase(16, vec![15, 7]);
        remove_n_testcase(16, vec![15, 8]);
        remove_n_testcase(16, vec![7, 15]);
        remove_n_testcase(16, vec![8, 15]);
        remove_n_testcase(16, vec![14, 6]);
        remove_n_testcase(16, vec![14, 5]);
        remove_n_testcase(16, vec![13, 6]);
        remove_n_testcase(16, vec![13, 5]);
        remove_n_testcase(16, vec![13, 4]);
        remove_n_testcase(16, vec![6, 14]);
        remove_n_testcase(16, vec![5, 14]);
        remove_n_testcase(16, vec![6, 13]);
        remove_n_testcase(16, vec![5, 13]);
        remove_n_testcase(16, vec![4, 13]);
        remove_n_testcase(9, vec![1, 3, 2, 8]);
    }
    
    
    
    fn remove_n_benchcase(nkeys: usize, remove: Vec<i32>) {
        let keys = (0..nkeys as i32).into_iter().collect::<Vec<i32>>();
        let mut sm = black_box(sm_items(keys));
        
        sm.remove_n(&remove);
    }
    
    
    #[bench]
    fn remove_n_bench(bencher: &mut Bencher) {
        bencher.iter(|| 
            for i in 0..10000 {
                remove_n_benchcase(16, vec![15, 14, 13, 12, 11, 10, 9, 8, 7])
            }
        )
    }
}
