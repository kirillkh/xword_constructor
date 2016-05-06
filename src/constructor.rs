use std::cmp::Ordering;
use std::collections::HashMap;
use std::rc::Rc;
use std::cell::Cell;
use rand::distributions::Range;

use common::{dim, Placement, PlacementId, make_rng, filter_indices, AbstractRng};
use fastmath::fastexp;
use board::{Board, Eff, AdjacencyInfo};


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


struct AdjacencyRec {
	counter: Rc<Cell<usize>>
}

struct AdjacencyResolver {
	mv: RankedMove,
	adjs: Vec<AdjacencyRec>
}

type ResolutionMap = HashMap<PlacementId, AdjacencyResolver>;



pub struct Constructor {
	pub h: dim,
	pub w: dim,
	rng: Box<AbstractRng>
}

impl Constructor {
	pub fn new(h: dim, w: dim) -> Constructor {
		Constructor { h:h, w:w, rng:make_rng() }
	}
	
	pub fn construct(&mut self, placements: &[Placement]) -> Vec<Placement> {
		let moves: Vec<_> = placements.iter().map(|p| RankedMove { place:p.clone(), rank: 0. }).collect();
		let (best_seq, best_valid_seq) = self.nrpa(NRPA_LEVEL, &moves);
		best_valid_seq.seq.into_iter().map(|mv| mv.1.place).collect()
	}
	
	fn debug1(&mut self, level: u8, moves: &[RankedMove], new_seq: &ChosenSequence, new_valid_seq: &ChosenSequence) {
		if level == NRPA_LEVEL {
			let mut ranks : Vec<_> = moves.iter().map(|mv| (mv.place.word.id, mv.rank)).collect();
			ranks.sort_by(|a,b| a.1.partial_cmp(&b.1).unwrap_or(Ordering::Equal));
			let skip = if ranks.len()<=40 { 0 } else { ranks.len()-20 };
			ranks = ranks.into_iter().skip(skip).collect();
			println!("top ranks: {:?}", ranks);
			
			let mut ranks : Vec<_> = new_seq.seq.iter().map(|cmv| (cmv.1.place.word.id, cmv.1.rank)).collect();
			println!("new ranks: {:?}", ranks);
			
			let mut board : Board<&ChosenMove> = Board::new(self.h, self.w, &mut *self.rng);
			board.place_all(new_seq.seq.iter().collect());
			board.print();
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
			
			let mut seq : Vec<_> = best_seq.seq.iter().map(|mv| &mv.1).collect();
			println!("{:?}", seq);
		}
	}
	
	// http://www.chrisrosin.com/rosin-ijcai11.pdf
	fn nrpa(&mut self, level: u8, parent_moves: &[RankedMove]) -> (ChosenSequence, ChosenSequence) {
		if level == 0 {
			self.nrpa_monte_carlo(&parent_moves)
		} else {
			let mut parent_moves = parent_moves.to_vec();
			let mut moves = parent_moves.to_vec();
			let mut best_seq = ChosenSequence::default();
				
			let mut best_valid_seq = ChosenSequence::default();
			let mut best_saved_seq = ChosenSequence::default();
			
			let mut last_progress = 0;
			
			for iter in 0..NRPA_ITERS {
				let (new_seq, new_valid_seq) = self.nrpa(level - 1, &moves);
				self.debug1(level, &moves, &new_seq, &new_valid_seq); // TODO debug

				if *new_valid_seq.eff >= *best_valid_seq.eff {
					best_valid_seq = new_valid_seq;
				}
				
				let max_stall = MAX_STALLED_ITERS + (level as u32);
				
				let must_backtrack = (*new_seq.eff <= *best_seq.eff) && (iter - last_progress >= max_stall);
				
				self.debug2(level, must_backtrack, iter, last_progress); // TODO debug
				
				if must_backtrack {
					moves = self.nrpa_backtrack(&best_seq.seq, moves, &mut parent_moves);
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
					moves = self.nrpa_adapt(moves, &best_seq.seq);
				}
				
			}
			
			self.debug3(level, &best_seq); // TODO debug
			
			(best_saved_seq, best_valid_seq)
		}
	}
	
	fn nrpa_monte_carlo(&mut self, parent_moves: &[RankedMove]) -> (ChosenSequence, ChosenSequence) {
		let rng = self.rng.clone_to_box();
		let mut grid = Board::new(self.h, self.w, &*rng);
		let mut best_seq = ChosenSequence::default();
		{
			let mut moves = parent_moves.to_vec();
			let mut refs : Vec<&mut RankedMove> = moves.iter_mut().collect();
			let mut exp_ranks : Vec<f32> = refs.iter().map(|mv| Self::exp_rank(&mv)).collect();
			let mut resolution_map: ResolutionMap = HashMap::new();
			
			// random rollout according to the policy
			while !refs.is_empty() {
//				if let Some((refs_, exp_ranks_, chosen)) = self.nrpa_choose(refs, exp_ranks, &mut grid, &mut resolution_map) {
				let (refs_, exp_ranks_, chosen) = self.nrpa_choose(refs, exp_ranks, &mut grid, &mut resolution_map);
				{
					refs = refs_;
					exp_ranks = exp_ranks_;
		
					// place the chosen move on the grid
//					let success = self.nrpa_place(chosen.clone(), &refs, &mut grid, &mut resolution_map);
//					if !success {
//						let bs = best_seq.clone();
//						return (best_seq, bs);
//					}
					self.nrpa_place(chosen.clone(), &refs, &mut grid, &mut resolution_map);
					
					// append the move to the seq
					best_seq.seq.push(chosen);
//				} else {
//					let bs = best_seq.clone();
//					return (best_seq, bs);
				}
			}
		}
		
		best_seq.eff = grid.efficiency();
		let best_valid_seq = ChosenSequence::new(grid.fixup_adjacent(), grid.efficiency());
		(best_seq, best_valid_seq)
	}
	
	
	fn nrpa_choose<'a, 'b> (&'b mut self, 
				    		refs: Vec<&'a mut RankedMove>,
				    		exp_ranks: Vec<f32>,
					    	grid: &mut Board<ChosenMove>,
							resolution_map: &mut ResolutionMap) 
//	-> (Option<(Vec<&'a mut RankedMove>, 
//			    Vec<f32>, 
//			    ChosenMove)>) 
	-> (Vec<&'a mut RankedMove>, 
	    Vec<f32>, 
	    ChosenMove) 
	{
		// 1. choose a move based on probability proportional to exp(mv.rank)
		let chosen_idx = if resolution_map.is_empty() {
			self.choose_proportional(&exp_ranks)
		} else {
			self.choose_resolver(&refs, &exp_ranks, &resolution_map)
		};
		
		let mv: RankedMove = refs[chosen_idx].clone();
		
		// 2. filter out incompatible placements (we must do this before the subsequent steps, because they depend on it)
		let partition = Constructor::partition_compat(mv.clone(), &refs);
		let (refs, excl) = filter_indices(refs, &partition.incl);
		let (exp_ranks, _) = filter_indices(exp_ranks, &partition.incl);
		
		// 3. for every incompatible move, decrement its associated adjacency counters
		for exmv in excl.iter() {
			if exmv.place.id == mv.place.id {
				// We must not decrement adjacency counters associated with the chosen move, because we check the counters for zero value below.
				// No need to decrement them at all, because every move that resolved them (including the chosen one) is being removed.
				continue; 
			}
			
			if let Some(mut excl_resolver) = resolution_map.remove(&exmv.place.id) {
				for mut adj in excl_resolver.adjs.iter_mut() {
					adj.counter.set(adj.counter.get() - 1);
					if adj.counter.get() == 0 {
//						// fail: this adjacency can no longer be resolved
//						return None;
					}
				}
			}
		}
		
		// 4. remove the chosen move from the map
		resolution_map.remove(&mv.place.id);
		
		let chosen = ChosenMove(chosen_idx as u32, mv, partition);
		
//		Some((refs, exp_ranks, chosen))
		(refs, exp_ranks, chosen)
	}
	
	fn nrpa_place<'a, 'b>(&'b mut self, chosen: ChosenMove,
			    		  refs: &Vec<&'a mut RankedMove>,
		    	  		  grid: &mut Board<ChosenMove>,
			      		  resolution_map: &mut ResolutionMap) -> bool {
		// 1) place the word on the grid
		grid.place(chosen.clone());
		
		// 2) compute the new adjacencies
		let adjacencies: Vec<AdjacencyInfo> = grid.adjacencies_of(&chosen);
		
		// TODO: this is very inefficient
		// 3) for each new adjacency:
		for adj in adjacencies.iter() {
			// 3.1) find all available resolvers and insert them into the map
			let counter = Rc::new(Cell::new(0));
			let (y, x) = (adj.y, adj.x);
			
			for rf in refs.iter() {
				if rf.place.contains(y, x) {
//					let resolver: &mut AdjacencyResolver = if let Some(resolver) = resolution_map.get_mut(&rf.place.id) {
//						resolver
//					} else {
//						let resolver = AdjacencyResolver { mv: (*rf).clone(), adjs: vec![] };
//						resolution_map.insert(rf.place.id, resolver);
//						resolution_map.get_mut(&rf.place.id).unwrap()
//					};
					
					if_let!(Some(resolver) = resolution_map.get_mut(&rf.place.id) => {
						let adj_rec = AdjacencyRec { counter:counter.clone() };
						resolver.adjs.push(adj_rec);
					} else {
						let mut resolver = AdjacencyResolver { mv: (*rf).clone(), adjs: vec![] };
						let adj_rec = AdjacencyRec { counter:counter.clone() };
						resolver.adjs.push(adj_rec);
						resolution_map.insert(rf.place.id, resolver);
					});
					
					counter.set(counter.get() + 1);
				}
			}
			
			// 3.2) fail if there are no resolvers
			// TODO: not exploring all the options contradicts the intuition that the best result is obtained when we ignore unsatiable adjacencies.
			// But returning early here both speeds the whole thing up a lot AND seems to provide more accurate results.
			// Whats's up with that?
			if counter.get() == 0 {
				return false;
			}
		}
		
		return true
	}
	
	
	fn choose_resolver(&mut self, refs: &Vec<&mut RankedMove>, exp_ranks: &Vec<f32>, resolution_map: &ResolutionMap) -> usize {
		// TODO this is very inefficient
		
		let indices: Vec<_> = refs.iter().enumerate()
							      .filter(|&(_, rf)| resolution_map.get(&rf.place.id).is_some())
								  .map(|(idx, _)| idx)
								  .collect();
		let resolver_ranks = indices.iter()
									.map(|&i| exp_ranks[i])
									.collect::<Vec<_>>();
		let idxidx = self.choose_proportional(&resolver_ranks);
		
		indices[idxidx]
	}
	
	
	fn choose_proportional(&mut self, choices: &Vec<f32>) -> usize {
		let z : f32 = choices.iter().fold(0., ::std::ops::Add::add);
		let between = Range::new(0., z);
		let mut v = self.rng.gen_f32(between);
		
		let mut chosen_idx = 0;
		for (i, &rnk) in choices.iter().enumerate() {
	    	chosen_idx = i;
		    if v <= rnk {
		    	return i; 
		    }
		    v -= rnk;
		}
		chosen_idx
	}
	
	fn exp_rank(mv: &RankedMove) -> f32 {
		fastexp(mv.rank + (mv.place.word.str.len() as f32))
	}
	
	
	fn nrpa_adapt<'a>(&self, moves: Vec<RankedMove>, seq: &[ChosenMove]) -> Vec<RankedMove> {
		let mut moves_ = moves.clone();
		
		{
			let mut refs_ : Vec<&mut RankedMove> = moves_.iter_mut().collect();
			
			for &ChosenMove(idx, ref rnk_mv, ref partition) in seq {
				let idx = idx as usize;
				
//				refs_[idx].rank += NRPA_ALPHA;
//				let exp_ranks : Vec<f32> = moves.iter().map(|mv| fastexp(mv.rank)).collect();
//				let z : f32 = exp_ranks.iter().fold(0., ::std::ops::Add::add);
//				for i in 0..refs_.len() {
//					refs_[i].rank -= NRPA_ALPHA * exp_ranks[i] / z;
//				}
//				
//				let (m, _) = filter_indices(moves, &partition.incl);
//				moves = m;
//				let (r, _) = filter_indices(refs_, &partition.incl);
//				refs_ = r;d

				
				let chosen_id = rnk_mv.place.id;
				let (mut incl, mut excl) = filter_indices(refs_, &partition.incl);
				let exp_ranks : Vec<f32> = excl.iter().map(|mv| Self::exp_rank(&mv)).collect();
				let z : f32 = exp_ranks.iter().fold(0., ::std::ops::Add::add);
				for i in 0..excl.len() {
					excl[i].rank -= NRPA_ALPHA * exp_ranks[i] / z;
					if excl[i].place.id == chosen_id {
						excl[i].rank += NRPA_ALPHA;
					}
				}
				refs_ = incl;
			}
		}
		
		moves_
	}
	
	fn nrpa_backtrack<'a>(&self, seq: &[ChosenMove], mut moves: Vec<RankedMove>, parent_moves: &mut Vec<RankedMove>) -> Vec<RankedMove> {
		let exp_ranks : Vec<f32> = parent_moves.iter().map(|mv| Self::exp_rank(&mv)).collect();
		let z : f32 = exp_ranks.iter().fold(0., ::std::ops::Add::add);
		
		for &ChosenMove(_, ref rnk_mv, _) in seq {
			let chosen_id = rnk_mv.place.id as usize;
			parent_moves[chosen_id].rank -= NRPA_ALPHA * exp_ranks[chosen_id] / z;
			moves[chosen_id].rank = parent_moves[chosen_id].rank;
		}
	
		moves
	}
	

	fn partition_compat<'a> (mv: RankedMove, refs: &Vec<&'a mut RankedMove>) -> Partition {
		let mut partition = Partition { incl: Vec::with_capacity(refs.len()), 
								   		excl: Vec::with_capacity(refs.len()) 
		};
		for (i, other) in refs.iter().enumerate() {
			if mv.place.compatible(&other.place) {
				partition.incl.push(i);
			} else {
				partition.excl.push(i);
			}
		}
		
		partition
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



#[derive(Clone, Debug)]
struct RankedMove {
	place: Placement,
	rank: f32,
}

impl AsRef<Placement> for RankedMove {
	fn as_ref(&self) -> &Placement {
		&self.place
	}
}


// TODO: this is bad
#[derive(Clone)]
struct Partition { 
	incl: Vec<usize>,
	excl: Vec<usize> 
}


#[derive(Clone)]
struct ChosenMove(u32, RankedMove, Partition);

impl AsRef<Placement> for ChosenMove {
	fn as_ref(&self) -> &Placement {
		&self.1.place
	}
}

use std::fmt;
impl fmt::Debug for ChosenMove {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    	write!(f, "mv({}, {:?})", self.0, self.1)
    }
}


