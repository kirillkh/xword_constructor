use std::cmp::Ordering;
use std::rc::Rc;
use std::cell::Cell;
use rand::distributions::Range;

use common::{dim, Placement, PlacementId, Word, WordId, make_rng, AbstractRng};
use fastmath::fastexp;
use fixed_grid::{FixedGrid, Eff, AdjacencyInfo, PlaceMove};
use super::weighted_selection_tree::{WeightedSelectionTree, Item};
use super::weighted_selection_tree;
use super::data::ScoredMove;
use super::variant_grid::{VariantGrid};


//const NRPA_LEVEL: u8 = 1;
//const NRPA_ITERS: u32 = 15500;

//const NRPA_LEVEL: u8 = 2;
//const NRPA_ITERS: u32 = 400;
//const NRPA_ALPHA: f32 = 0.8;

//const NRPA_LEVEL: u8 = 2;
//const NRPA_ITERS: u32 = 400;
//const NRPA_ALPHA: f32 = 0.125;

// GREAT RESULTS!
const NRPA_LEVEL: u8 = 3;
const NRPA_ITERS: u32 = 27;
//const NRPA_ALPHA: f32 = 0.03125;
//const NRPA_ALPHA: f32 = 0.0625;
//const NRPA_ALPHA: f32 = 0.125;
//const NRPA_ALPHA: f32 = 0.25;
//const NRPA_ALPHA: f32 = 0.5;
const NRPA_ALPHA: f32 = 1.;
//const NRPA_ALPHA: f32 = 2.;
//const NRPA_ALPHA: f32 = 4.;
const MAX_STALLED_ITERS: u32 = 1;


//const NRPA_LEVEL: u8 = 2;
//const NRPA_ITERS: u32 = 100;
////const NRPA_ALPHA: f32 = 0.125;
////const NRPA_ALPHA: f32 = 0.0625;
////const NRPA_ALPHA: f32 = 0.03125;
//const NRPA_ALPHA: f32 = 0.25;

//const NRPA_LEVEL: u8 = 3;
//const NRPA_ITERS: u32 = 100;
//const NRPA_ALPHA: f32 = 0.125;


//const NRPA_LEVEL: u8 = 6;
//const NRPA_ITERS: u32 = 7;
//const NRPA_ALPHA: f32 = 2.;


//const NRPA_LEVEL: u8 = 5;
//const NRPA_ITERS: u32 = 11;

// GOOD RESULTS
//const NRPA_LEVEL: u8 = 4;
//const NRPA_ITERS: u32 = 20;
//const NRPA_ALPHA: f32 = 1.;

// OK results
//const NRPA_LEVEL: u8 = 6;
//const NRPA_ITERS: u32 = 7;
//const NRPA_ALPHA: f32 = 0.9;

//const NRPA_ALPHA: f32 = 1.4; // 4-5
//const NRPA_ALPHA: f32 = 1.6; // 10+


#[derive(Clone)]
struct AdjacencyRec {
	counter: Rc<Cell<usize>>
}

#[derive(Clone)]
struct AdjacencyResolver {
	mv: ScoredMove,
	adjs: Vec<AdjacencyRec> // TODO: do we want to box this, so that copying adjacencyresolvers around is faster?
}

impl weighted_selection_tree::Item<PlacementId> for AdjacencyResolver {
    #[inline]
    fn key(&self) -> PlacementId {
        self.mv.key()
    }
    
    #[inline]
    fn weight(&self) -> f32 {
        self.mv.weight()
    }
}



//type ResolutionMap = HashMap<PlacementId, AdjacencyResolver>;

type ResolutionMap = WeightedSelectionTree<PlacementId, AdjacencyResolver>;



pub struct Constructor {
    places: Rc<Vec<Placement>>,
    placements_per_word: Vec<Vec<PlacementId>>,  // TODO: we might want to dynamically remove placements in the algorithm
	pub h: dim,
	pub w: dim,
	rng: Box<AbstractRng>
}

impl Constructor {
	pub fn new(h: dim, w: dim, dic: &[Word], places: &[Placement]) -> Constructor {
	    let places = places.iter().cloned().collect::<Vec<_>>();
	    
	    let mut placements_per_word = vec![vec![]; dic.len()];
	    for place in places.iter() {
	        placements_per_word[place.word.id as usize].push(place.id);
	    }
	    
		Constructor { placements_per_word:placements_per_word, places:Rc::new(places), h:h, w:w, rng:make_rng() }
	}
	
	pub fn construct(&mut self) -> Vec<Placement> {
		let moves: Vec<_> = self.places.iter().map(|p| ScoredMove { place:p.clone(), score: 0., exp_score: 1. }).collect();
		let (_, best_valid_seq) = self.nrpa(NRPA_LEVEL, &moves);
		best_valid_seq.seq.into_iter().map(|mv| mv.0).collect()
	}
	
	// http://www.chrisrosin.com/rosin-ijcai11.pdf
	fn nrpa(&mut self, level: u8, parent_policy: &[ScoredMove]) -> (ChosenSequence, ChosenSequence) {
		if level == 0 {
			self.nrpa_monte_carlo(parent_policy)
		} else {
			let mut parent_policy = parent_policy.to_vec();
			let mut policy = parent_policy.to_vec();
			let mut best_seq = ChosenSequence::default();
				
			let mut best_valid_seq = ChosenSequence::default();
			let mut best_saved_seq = ChosenSequence::default();
			
			let mut last_progress = 0;
			
			for iter in 0..NRPA_ITERS {
				let (new_seq, new_valid_seq) = self.nrpa(level - 1, &policy);
				self.debug1(level, &policy, &new_seq, &new_valid_seq); // TODO debug

				if *new_valid_seq.eff >= *best_valid_seq.eff {
					best_valid_seq = new_valid_seq;
				}
				
				let max_stall = MAX_STALLED_ITERS + (level as u32);
				
				let must_backtrack = (*new_seq.eff <= *best_seq.eff) && (iter - last_progress >= max_stall);
				
				self.debug2(level, must_backtrack, iter, last_progress); // TODO debug
				
				if must_backtrack {
					policy = self.nrpa_backtrack(&best_seq.seq, policy, &mut parent_policy);
					best_seq.seq.truncate(0);
					best_seq.eff = Eff(0);
					last_progress = iter;
				} else {
					if *new_seq.eff >= *best_seq.eff {
						if *new_seq.eff > *best_seq.eff {
							last_progress = iter;
						}
						
						best_seq = new_seq;
						
						if *best_seq.eff >= *best_saved_seq.eff {
							best_saved_seq = best_seq.clone();
						}
					} else {
//						moves = self.nrpa_adapt(moves, &best_valid_seq);
					}
					policy = self.nrpa_adapt(policy, &best_seq.seq);
				}
				
			}
			
			self.debug3(level, &best_seq); // TODO debug
			
			(best_saved_seq, best_valid_seq)
		}
	}
	
	
	fn nrpa_backtrack(&self, seq: &[ChosenMove], mut moves: Policy, parent_moves: &mut Policy) -> Policy {
		let z : f32 = parent_moves.iter().fold(0., |acc, mv| acc+mv.exp_score);
		
		for &ChosenMove(ref place, _) in seq {
			let chosen_id = place.id;
			let parent_move = &mut parent_moves[chosen_id];
			parent_move.score -= NRPA_ALPHA * parent_move.exp_score / z;
			moves[chosen_id].score = parent_move.score;
		}
	
		moves
	}
	
	
	fn remove_incompat(&self, mv: &ScoredMove, 
                	   grid: &mut VariantGrid, 
                	   select_tree: &mut SelectTree, 
                	   resolution_map: &mut ResolutionMap)  -> Vec<ScoredMove> {
	    // 1. remove all placements of this word
	    let word_placements = &self.placements_per_word[mv.place.word.id as usize];
	    let mut rmvd_word_moves = Vec::with_capacity(word_placements.len());
	    for &pid in word_placements {
	        if grid.contains(pid) {
    	        grid.remove(pid);
    	        let mv = if select_tree.contains_key(pid) {
    	            select_tree.remove(pid)
    	        } else {
    	            resolution_map.remove(pid).mv
    	        };
	            rmvd_word_moves.push(mv);
	        }
	    }
	    
	    // 2. remove incompatible placements from the variant grid
	    let incompat_ids = grid.remove_incompat(mv.place.id);
	    
	    let mut resolvers_to_rm: Vec<PlacementId> = Vec::with_capacity(incompat_ids.len());
	    let mut moves_to_rm: Vec<PlacementId> = Vec::with_capacity(incompat_ids.len());
	    for pl_id in incompat_ids {
	        if resolution_map.contains_key(pl_id) {
	            resolvers_to_rm.push(pl_id);
	        } else {
	            moves_to_rm.push(pl_id);
	        }
	    }
	    
	    let rmvd_resolvers = resolution_map.remove_bulk(&*resolvers_to_rm);
	    let mut rmvd_moves: Vec<ScoredMove> = Vec::with_capacity(rmvd_resolvers.len());
	    for rslvr in rmvd_resolvers.into_iter() {
	        for adj in rslvr.adjs.iter() {
				adj.counter.set(adj.counter.get() - 1);
				if adj.counter.get() == 0 {
//						// fail: this adjacency can no longer be resolved
//						return None;
				}
	        }
	        
			rmvd_moves.push(rslvr.mv);
	    }
	    
	    let mut rmvd_moves2: Vec<ScoredMove> = select_tree.remove_bulk(&*moves_to_rm);
	    
	    rmvd_word_moves.reserve_exact(rmvd_moves.len() + rmvd_moves2.len());
	    rmvd_word_moves.append(&mut rmvd_moves);
	    rmvd_word_moves.append(&mut rmvd_moves2);
	    
	    rmvd_word_moves
	}
	
	
	fn nrpa_choose<'a, 'b> (&'b mut self, 
				    		select_tree: &mut SelectTree,
					    	grid: &mut VariantGrid,
							resolution_map: &mut ResolutionMap) -> ChosenMove 
	{
		// 1. choose a move based on probability proportional to exp(mv.rank)
		let mv: ScoredMove = if resolution_map.is_empty() {
			self.select_proportional(select_tree)
		} else {
//			self.choose_resolver(select_tree, &resolution_map)
            self.select_proportional(resolution_map).mv
		};
		grid.remove(mv.key());
		
		// 2. filter out incompatible placements (we must do this before the subsequent steps, because they depend on it)
		let excl: Vec<ScoredMove> = self.remove_incompat(&mv, grid, select_tree, resolution_map);
		
//		// 3. for every incompatible move, decrement its associated adjacency counters
//		for exmv in excl.iter() {
//			if exmv.place.id == mv.place.id {
//				// We must not decrement adjacency counters associated with the chosen move, because we check the counters for zero value below.
//				// No need to decrement them at all, because every move that resolved them (including the chosen one) is being removed.
//				continue; 
//			}
//			
//			if let Some(mut excl_resolver) = resolution_map.remove(&exmv.place.id) {
//				for mut adj in excl_resolver.adjs.iter_mut() {
//					adj.counter.set(adj.counter.get() - 1);
//					if adj.counter.get() == 0 {
////						// fail: this adjacency can no longer be resolved
////						return None;
//					}
//				}
//			}
//		}
		
		let excl_keys = excl.into_iter().map(|mv| mv.place.id).collect::<Vec<_>>();
    	ChosenMove(mv.place.clone(), Rc::new(excl_keys))
	}
	
	fn nrpa_place<'a, 'b>(&'b mut self, chosen: ChosenMove,
		    	  		  fixed_grid: &mut FixedGrid<ChosenMove>,
		    	  		  variant_grid: &VariantGrid,
			    		  select_tree: &mut SelectTree,
			      		  resolution_map: &mut ResolutionMap) -> bool {
		// 1) place the word on the grid
		fixed_grid.place(chosen.clone());
		
		// 2) compute the new adjacencies
		let adjacencies: Vec<AdjacencyInfo> = fixed_grid.adjacencies_of(chosen.place().id);
		
		// accumulate resolver ids for bulk transferral
		let mut new_resolver_ids: Vec<PlacementId> = vec![];
		let mut new_resolver_adjs: Vec<AdjacencyRec> = vec![];
		
		let mut success = true;
		
		// 3) for each new adjacency:
		for adj in adjacencies.iter() {
			// 3.1) find all available resolvers
			let counter = Rc::new(Cell::new(0));
			let (y, x) = (adj.y, adj.x);
			
            for &place_id in variant_grid.iter_at(y, x) {
                counter.set(counter.get() + 1);
				let adj_rec = AdjacencyRec { counter:counter.clone() };
				if let Some(resolver) = resolution_map.get_mut(place_id) {
					resolver.adjs.push(adj_rec);
				} else {
			        new_resolver_ids.push(place_id);
			        new_resolver_adjs.push(adj_rec);
				}
            }
            
            
			// 3.2) fail if there are no resolvers
			// TODO: not exploring all the options contradicts the intuition that the best result is obtained when we ignore unsatiable adjacencies.
			// But returning early here both speeds the whole thing up a lot AND seems to provide more accurate results.
			// Whats's up with that?
			if counter.get() == 0 {
				success = false;
				break;
			}
		}
		
		
		let resolvers: Vec<ScoredMove> = select_tree.remove_bulk(&new_resolver_ids);
		let resolvers_adjs = resolvers.into_iter().zip(new_resolver_adjs.into_iter());
		
		let entries = resolvers_adjs.enumerate().map(|(i, (mv, adj))| 
		    (new_resolver_ids[i], AdjacencyResolver { mv: mv, adjs: vec![adj] })
	    ).collect::<Vec<_>>();
		
		resolution_map.insert_bulk(entries);
		
		return success
	}
	
	
	fn select_proportional<T: Item<PlacementId>>(&mut self, select_tree: &mut WeightedSelectionTree<PlacementId, T>) -> T {
		let z : f32 = select_tree.total();
		let between = Range::new(0., z);
		let v = self.rng.gen_f32(between);
		select_tree.select_remove(v).unwrap()
	}
	
	
	
	fn nrpa_monte_carlo(&mut self, policy: &[ScoredMove]) -> (ChosenSequence, ChosenSequence) {
		let rng = self.rng.clone_to_box();
		let mut fixed_grid = FixedGrid::new(self.h, self.w, &*rng);
        let mut variant_grid = VariantGrid::new(self.places.clone(), self.h, self.w);
		let mut best_seq = ChosenSequence::new(Vec::with_capacity(self.placements_per_word.len()), Eff(0));
		{
            let mut select_tree: SelectTree = SelectTree::new(policy, policy.len());
            
			let mut resolution_map: ResolutionMap = WeightedSelectionTree::new(&[], policy.len());
			
			// random rollout according to the policy
			while !select_tree.is_empty() || !resolution_map.is_empty() {
				let chosen = self.nrpa_choose(&mut select_tree, &mut variant_grid, &mut resolution_map);
				{
					// place the chosen move on the grid
//					let success = self.nrpa_place(chosen.clone(), &refs, &mut grid, &mut resolution_map);
//					if !success {
//						let bs = best_seq.clone();
//						return (best_seq, bs);
//					}
					self.nrpa_place(chosen.clone(), &mut fixed_grid, &variant_grid, &mut select_tree, &mut resolution_map);
					
					// append the move to the seq
					best_seq.seq.push(chosen);
//				} else {
//					let bs = best_seq.clone();
//					return (best_seq, bs);
				}
			}
		}
		
		best_seq.eff = fixed_grid.efficiency();
		
		let (valid, valid_eff) = fixed_grid.fixup_adjacent();
		
		let best_valid_seq = ChosenSequence::new(valid, valid_eff);
		(best_seq, best_valid_seq)
	}
	
	
	
	fn exp_score(mv: &ScoredMove) -> f32 {
		fastexp(mv.score + (mv.place.word.str.len() as f32))
	}
	
	
	fn nrpa_adapt<'a>(&self, mut policy: Policy, seq: &[ChosenMove]) -> Policy {
		{
			for &ChosenMove(ref place, ref excl) in seq {
				let chosen_id = place.id;
    			policy[chosen_id].score += NRPA_ALPHA;
    			
				let z : f32 = excl.iter().fold(0., |acc, &pl_id| acc + policy[pl_id].exp_score);
				for &pl_id in excl.iter() {
				    let scored = &mut policy[pl_id];
					scored.score -= NRPA_ALPHA * scored.exp_score / z;
					scored.exp_score = Self::exp_score(&scored);
				}
			}
		}
		
		policy
	}
	
	
	fn debug1(&mut self, level: u8, moves: &[ScoredMove], new_seq: &ChosenSequence, new_valid_seq: &ChosenSequence) {
		if level == NRPA_LEVEL {
			let mut ranks : Vec<_> = moves.iter().map(|mv| (mv.place.word.id, mv.score)).collect();
			ranks.sort_by(|a,b| a.1.partial_cmp(&b.1).unwrap_or(Ordering::Equal));
			let skip = if ranks.len()<=40 { 0 } else { ranks.len()-20 };
			ranks = ranks.into_iter().skip(skip).collect();
			println!("top ranks: {:?}", ranks);
			
			let ranks : Vec<_> = new_seq.seq.iter().map(|cmv: &ChosenMove| (cmv.0.word.id, cmv.0.id.0, moves[cmv.0.id].score)).collect();
			println!("new ranks: {:?}", ranks);
			
			let mut grid : FixedGrid<ChosenMove> = FixedGrid::new(self.h, self.w, &mut *self.rng);
			grid.place_all(new_seq.seq.iter().cloned().collect());
			grid.print();
			println!("-------------- eff new: {} valid: {}, ----------------", *new_seq.eff, *new_valid_seq.eff);
		}
	}
	
	fn debug2(&mut self, level: u8, must_backtrack: bool, iter: u32, last_progress: u32) {
		if level == NRPA_LEVEL {
			println!("backtrack: {}, iter: {}, progress: {}", must_backtrack, iter, last_progress);
		} 
	}
	
	fn debug3(&mut self, level: u8, best_seq: &ChosenSequence) {
		if level == NRPA_LEVEL {
//				let mut ranks : Vec<_> = moves.iter().map(|mv| (mv.place.id, mv.rank)).collect();
//				ranks.sort_by(|a,b| a.1.partial_cmp(&b.1).unwrap_or(Ordering::Equal));
//				ranks = ranks.into_iter().collect();
//				println!("{:?}\n", ranks);
			
			let seq : Vec<_> = best_seq.seq.iter().map(|mv| &mv.0).collect();
			println!("{:?}", seq);
		}
	}
}


type Policy = Vec<ScoredMove>;



type Excluded = Rc<Vec<PlacementId>>;



type SelectTree = WeightedSelectionTree<PlacementId, ScoredMove>;


#[derive(Clone, Debug)]
struct ChosenMove(Placement, Excluded);

impl PlaceMove for ChosenMove {
    fn place(&self) -> &Placement {
        &self.0
    }
}


#[derive(Clone, Debug)]
struct ChosenSequence {
	seq: Vec<ChosenMove>, 
	eff: Eff
}

impl ChosenSequence {
	fn new(seq: Vec<ChosenMove>, eff: Eff) -> ChosenSequence {
		ChosenSequence { seq:seq, eff:eff }
	}
}

impl Default for ChosenSequence {
	fn default() -> ChosenSequence {
		Self::new(vec![], Eff(0))
	}
}

