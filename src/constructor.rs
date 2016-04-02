use rand::distributions::{IndependentSample, Range};
use rand::XorShiftRng;

use common::{dim, Placement, make_rng};
use board::{Board, Eff};


//const NRPA_LEVEL: u8 = 1;
//const NRPA_ITERS: u32 = 15500;
//const NRPA_LEVEL: u8 = 2;
//const NRPA_ITERS: u32 = 125;
//const NRPA_LEVEL: u8 = 3;
//const NRPA_ITERS: u32 = 25;
const NRPA_LEVEL: u8 = 4;
const NRPA_ITERS: u32 = 11;

const NRPA_ALPHA: f32 = 5.0;




pub struct Constructor {
	pub h: dim,
	pub w: dim,
	rng: XorShiftRng
}

impl Constructor {
	pub fn new(h: dim, w: dim) -> Constructor {
		Constructor { h:h, w:w, rng:make_rng() }
	}
	
	pub fn construct(&mut self, placements: &[Placement]) -> Vec<Placement> {
		let moves: Vec<_> = placements.iter().map(|p| RankedMove { place:p.clone(), rank: 0. }).collect();
		let (seq, _) = self.nrpa(NRPA_LEVEL, &moves);
		seq.into_iter().map(|mv| mv.1.place).collect()
	}
	
	// http://www.chrisrosin.com/rosin-ijcai11.pdf
	fn nrpa(&mut self, level: u8, moves: &[RankedMove]) -> (Vec<ChosenMove>, Eff) {
		let mut moves = moves.to_vec();
		let mut best_seq = vec![];
		let mut best_eff = Eff(0);
			
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
						let mut v = between.ind_sample(&mut self.rng);
						
						let mut chosen_idx = 0;
						for (i, rnk) in exp_ranks.into_iter().enumerate() {
					    	chosen_idx = i;
						    if v <= rnk {
						    	break; 
						    }
						    v -= rnk;
						}
						chosen_idx
					};
					
					
					let mv = refs[chosen_idx].clone();
					
					// 2. filter compatible moves 
					let partition = Constructor::partition_compat(mv.clone(), &refs);
//					println!("refs = {:?}", refs);
//					println!("partition = {:?}", partition.incl);
					refs = Constructor::filter_indices(refs, &partition.incl);
					
					// 3. append the move to the seq
					best_seq.push(ChosenMove(chosen_idx as u32, mv, partition));
				}
			}
			
			self.fixup_board(best_seq)
		} else {
			let mut moves : Vec<RankedMove> = moves.to_vec();
			for _ in 0..NRPA_ITERS {
				let (new_seq, new_eff) = self.nrpa(level - 1, &moves);
				if new_seq.len() > best_seq.len() || 
				   (new_seq.len() == best_seq.len() && *new_eff >= *best_eff) 
				{
					best_seq = new_seq;
					best_eff = new_eff;
				}
				moves = self.nrpa_adapt(moves, &best_seq); 
			}
			
			(best_seq, best_eff)
		}
	}
	
	
	fn nrpa_adapt<'a>(&self, mut moves: Vec<RankedMove>, seq: &[ChosenMove]) -> Vec<RankedMove> {
		let mut moves_ = moves.clone();
		
		{
			let mut refs_ : Vec<&mut RankedMove> = moves_.iter_mut().collect();
			
			for &ChosenMove(idx, _, ref partition) in seq {
				let idx = idx as usize;
				refs_[idx].rank += NRPA_ALPHA;
				let z = moves.iter().fold(0., |acc, mv| acc + mv.rank.exp());
				for i in 0..refs_.len() {
					refs_[i].rank -= NRPA_ALPHA * moves[i].rank.exp() / z;
				}
				
				moves = Constructor::filter_indices(moves, &partition.incl);
				refs_ = Constructor::filter_indices(refs_, &partition.incl);
			}
		}
		
		moves_
	}
	
	
	fn fixup_board<Move: AsRef<Placement>> (&mut self, seq: Vec<Move>) -> (Vec<Move>, Eff) {
		#[derive(Clone)]
		struct IndexedMove<'a> {
			place: &'a Placement,
			moves_idx: usize,
		}
		
		impl<'a> AsRef<Placement> for IndexedMove<'a> {
			fn as_ref(&self) -> &Placement { &self.place }
		}
//		
		let fixed_indices: Vec<usize>;
		let eff: Eff;
		{
			// TODO: this may be expensive, might need to cache
			let mut board = Board::new(self.h, self.w, &mut self.rng);
			
			for (i, mv) in seq.iter().enumerate() {
				let idxmv = IndexedMove { place: mv.as_ref(), moves_idx: i };
				board.place(idxmv); 
			}
			
			let fixed : Vec<IndexedMove> = board.fixup_adjacent();
			fixed_indices = fixed.into_iter().map(|idxmv| idxmv.moves_idx).collect();
			eff = board.efficiency();
		};
		
		(Constructor::filter_indices(seq, &fixed_indices), eff)
	}
	
	
	
	
	fn partition_compat<'a> (mv: RankedMove, refs: &Vec<&'a mut RankedMove>) -> Partition {
		let init = Partition { incl: Vec::with_capacity(refs.len()), 
//							   excl: Vec::with_capacity(refs.len()) 
		};
		let partition = refs.iter().enumerate().fold(init, |mut acc, (i, other)| {
				if mv.place.compatible(&other.place) {
					acc.incl.push(i);
				} else {
//					acc.excl.push(i);
				}
				
				acc
		});
		
		partition
	}
	

	fn filter_indices<T> (mut items: Vec<T>, indices: &[usize]) -> Vec<T> {
//		let mut keep = vec![ false; items.len() ];
//		for &index in indices {
//		    keep[index] = true;
//		}
//
//		let mut out = Vec::with_capacity(items.len());
//		for (i, item) in items.into_iter().enumerate() {
//			if keep[i] {
//				out.push(item)
//			}
//		}
//		
//		out

		let mut keep = vec![ false; items.len() ];
		for &index in indices {
		    keep[index] = true;
		}

		let mut j = 0;
		for i in 0..items.len() {
			if keep[i] {
				if i!=j {
					items.swap(j, i);
				}
				j += 1;
			}
		}
		items.truncate(j);
		items
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


struct Partition { incl: Vec<usize>,
//	 excl: Vec<usize> 
}


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


