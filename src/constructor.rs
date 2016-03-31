use rand;
use rand::distributions::{IndependentSample, Range};

use common::{dim, Placement};
use board::Board;


const NRPA_LEVEL: u8 = 4;
const NRPA_ITERS: u32 = 100;
const NRPA_ALPHA: f32 = 1.0;


pub struct Constructor {
	pub h: dim,
	pub w: dim,
}

impl Constructor {
	pub fn new(h: dim, w: dim) -> Constructor {
		Constructor { h:h, w:w }
	}
	
	pub fn construct(&mut self, placements: &[Placement]) -> Vec<Placement> {
		let moves: Vec<_> = placements.iter().map(|p| RankedMove { place:p.clone(), rank: 0. }).collect();
		let chosen = self.nrpa(NRPA_LEVEL, &moves);
		chosen.into_iter().map(|mv| mv.1.place).collect()
	}
	
	// http://www.chrisrosin.com/rosin-ijcai11.pdf
	fn nrpa(&mut self, level: u8, moves: &[RankedMove]) -> Vec<ChosenMove> {
		let mut moves = moves.to_vec();
		let mut best_seq = vec![];
			
		if level == 0 {
			{
				let mut refs : Vec<&mut RankedMove> = moves.iter_mut().collect();
				
				// random rollout according to the policy
				while !refs.is_empty() {
					// 1. choose a move based on probability proportional to exp(mv.rank)
					let exp_ranks : Vec<f32> = refs.iter().map(|mv| mv.rank.exp()).collect();
					let z : f32 = exp_ranks.iter().fold(0., |acc, erank| acc + erank);
					let chosen_idx = {
						let between = Range::new(0., z);
						let mut v = between.ind_sample(&mut rand::thread_rng());
						
						let mut chosen_idx = refs.len();
						for (i, rnk) in exp_ranks.into_iter().enumerate() {
						    if v <= rnk {
						    	chosen_idx = i;
						    	break; 
						    }
						    v -= rnk;
						}
						chosen_idx
					};
					
					let mv = refs[chosen_idx].clone();
					
					// 2. filter compatible moves 
					let partition = Constructor::partition_compat(mv.clone(), &refs);
					refs = Constructor::filter_indices(refs, &partition.incl);
					
					// 3. append the move to the seq
					best_seq.push(ChosenMove(chosen_idx as u32, mv, partition));
				}
			}
			
			self.fixup_board(best_seq)
			
		} else {
			let mut moves : Vec<RankedMove> = moves.to_vec();
			for _ in 0..NRPA_ITERS {
				let new_seq = self.nrpa(level - 1, &moves);
				if new_seq.len() >= best_seq.len() {
					best_seq = new_seq;
				}
				moves = self.nrpa_adapt(&moves, &best_seq); 
			}
			
			best_seq
		}
	}
	
	
	fn nrpa_adapt<'a>(&self, moves: &[RankedMove], seq: &[ChosenMove]) -> Vec<RankedMove> {
		let mut moves_ = moves.to_vec();
		let mut moves = moves.to_vec();
		
		{
			let mut refs_ : Vec<&mut RankedMove> = moves_.iter_mut().collect();
			let mut refs : Vec<&mut RankedMove> = moves.iter_mut().collect();
			
			for &ChosenMove(idx, _, ref partition) in seq {
				let idx = idx as usize;
				refs_[idx].rank += NRPA_ALPHA;
				let z = refs.iter().fold(0., |acc, mv| acc + mv.rank.exp());
				for i in 0..refs_.len() {
					refs_[i].rank -= NRPA_ALPHA * refs[i].rank.exp() / z;
				}
				
				refs = Constructor::filter_indices(refs, &partition.incl);
				refs_ = Constructor::filter_indices(refs_, &partition.incl);
			}
		}
		
		moves_
	}
	
	
	fn fixup_board<Move: AsRef<Placement>> (&self, seq: Vec<Move>) -> Vec<Move> {
		#[derive(Clone)]
		struct IndexedMove<'a> {
			place: &'a Placement,
			moves_idx: usize,
		}
		
		impl<'a> AsRef<Placement> for IndexedMove<'a> {
			fn as_ref(&self) -> &Placement { &self.place }
		}
//		
		let fixed_indices : Vec<usize> = {
			// TODO: this is very expensive, need to cache
			let mut board = Board::new(self.h, self.w);
			
			for (i, mv) in seq.iter().enumerate() {
				let idxmv = IndexedMove { place: mv.as_ref(), moves_idx: i };
				board.place(idxmv); 
			}
			
			let fixed : Vec<IndexedMove> = board.fixup_adjacent();
			fixed.into_iter().map(|idxmv| idxmv.moves_idx).collect()
		};
		
		Constructor::filter_indices(seq, &fixed_indices)
	}
	
	
	
	
	fn partition_compat<'a> (mv: RankedMove, refs: &Vec<&'a mut RankedMove>) -> Partition {
		let init = Partition { incl: Vec::with_capacity(refs.len()), 
							   excl: Vec::with_capacity(refs.len()) };
		let partition = refs.iter().enumerate().fold(init, |mut acc, (i, other)| {
				if mv.place.compatible(&other.place) {
					acc.incl.push(i);
				} else {
					acc.excl.push(i);
				}
				
				acc
		});
		
		partition
	}
	

	fn filter_indices<T> (items: Vec<T>, indices: &[usize]) -> Vec<T> {
//	    let mut oitems : Vec<Option<T>> = 
//	        items.into_iter().map(|r| Some(r)).collect();
//	    indices.iter().map(|&idx| oitems[idx].take().unwrap()).collect()
	    
		let mut keep = vec![ false; items.len() ];
		for index in indices {
		    keep[*index] = true;
		}
	    items.into_iter()
	        .zip(keep)
	        .filter_map(|(val, flag)| if flag { Some(val) } else { None })
	        .collect()
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


struct Partition { incl: Vec<usize>, excl: Vec<usize> }


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


