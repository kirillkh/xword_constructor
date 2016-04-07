use std::ops::DerefMut;
use std::cmp::Ordering;
use rand::distributions::{IndependentSample, Range};
use rand::Rng;

use common::{dim, Placement, make_rng, filter_indices, AbstractRng};
use fastmath::fastexp;
use board::{Board, Eff};


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
const NRPA_ITERS: u32 = 55;
const NRPA_ALPHA: f32 = 0.125;
//const NRPA_ALPHA: f32 = 0.0625;
//const NRPA_ALPHA: f32 = 0.03125;
//const NRPA_ALPHA: f32 = 1.;

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
		let (seq, _) = self.nrpa(NRPA_LEVEL, &moves);
		seq.into_iter().map(|mv| mv.1.place).collect()
	}
	
	// http://www.chrisrosin.com/rosin-ijcai11.pdf
	fn nrpa(&mut self, level: u8, moves: &[RankedMove]) -> (Vec<ChosenMove>, Eff) {
		let mut moves = moves.to_vec();
		let mut best_seq : Vec<ChosenMove> = vec![];
		let mut best_eff = Eff(0);
			
		if level == 0 {
			{
				let mut refs : Vec<&mut RankedMove> = moves.iter_mut().collect();
				let mut exp_ranks : Vec<f32> = refs.iter().map(|mv| fastexp(mv.rank + (mv.place.word.str.len() as f32))).collect();
				
				// random rollout according to the policy
				while !refs.is_empty() {
					// 1. choose a move based on probability proportional to exp(mv.rank)
					let z : f32 = exp_ranks.iter().fold(0., |acc, erank| acc + erank);
					let chosen_idx = {
						let between = Range::new(0., z);
						let mut v = self.rng.gen_f32(between);
						
						let mut chosen_idx = 0;
						for (i, &rnk) in exp_ranks.iter().enumerate() {
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
					let (r, _) = filter_indices(refs, &partition.incl);
					refs = r;
					let (e, _) = filter_indices(exp_ranks, &partition.incl);
					exp_ranks = e;
					
					// 3. append the move to the seq
					best_seq.push(ChosenMove(chosen_idx as u32, mv, partition));
				}
			}
			
			self.fixup_board(best_seq)
//			(best_seq, Eff(0))
		} else {
			let mut moves : Vec<RankedMove> = moves.to_vec();
			for _ in 0..NRPA_ITERS {
				let (new_seq, new_eff) = self.nrpa(level - 1, &moves);
				
					if level == NRPA_LEVEL {
						let mut ranks : Vec<_> = moves.iter().map(|mv| (mv.place.word.id, mv.rank)).collect();
						ranks.sort_by(|a,b| a.1.partial_cmp(&b.1).unwrap_or(Ordering::Equal));
						let skip = if ranks.len()<=40 { 0 } else { ranks.len()-20 };
						ranks = ranks.into_iter().skip(skip).collect();
						println!("top ranks: {:?}", ranks);
						
						let mut ranks : Vec<_> = new_seq.iter().map(|cmv| (cmv.1.place.word.id, cmv.1.rank)).collect();
						println!("new ranks: {:?}", ranks);
						
						let mut board : Board<&ChosenMove> = Board::new(self.h, self.w, &mut *self.rng);
						board.place(new_seq.iter().collect());
						board.print();
						println!("-------------- eff {} ----------------", new_eff.0);
					}
					

				moves = self.nrpa_adapt(moves, &new_seq);
				
//				if new_seq.len() as u32 + *new_eff >= best_seq.len() as u32 + *best_eff 
//				if *new_eff >= *best_eff {
//					best_seq = new_seq;
//					best_eff = new_eff;
//				} else if best_seq.len()>0 {
//					if *new_eff == *best_eff {
//						best_seq = new_seq;
//						best_eff = new_eff;
//					}
					
//					let max_ranked = best_seq.iter().fold((best_seq[0].1.rank, None), |(max, idx), mv| 
//						if mv.1.rank >= max {
//							(mv.1.rank, Some(&mv.1))
//						} else {
//							(max, idx)
//						}
//					).1.unwrap();
//					
//					for  mv in &mut moves {
//						if mv.place.id == max_ranked.place.id {
//							mv.rank -= NRPA_ALPHA;
//							mv.rank = 0.;
//							break;
//						}
//					}



				moves = self.nrpa_adapt(moves, &new_seq);
				if *new_eff >= *best_eff {
					best_seq = new_seq;
					best_eff = new_eff;
				} else {
					moves = self.nrpa_adapt(moves, &best_seq);
				}
			}
			
			if level == NRPA_LEVEL {
//				let mut ranks : Vec<_> = moves.iter().map(|mv| (mv.place.id, mv.rank)).collect();
//				ranks.sort_by(|a,b| a.1.partial_cmp(&b.1).unwrap_or(Ordering::Equal));
//				ranks = ranks.into_iter().collect();
//				println!("{:?}\n", ranks);
				
				let mut seq : Vec<_> = best_seq.iter().map(|mv| &mv.1).collect();
				println!("{:?}", seq);
			}
			
			
			(best_seq, best_eff)
		}
	}
	
	fn nrpa_adapt<'a>(&self, mut moves: Vec<RankedMove>, seq: &[ChosenMove]) -> Vec<RankedMove> {
		let mut moves_ = moves.clone();
		
		{
			let mut refs_ : Vec<&mut RankedMove> = moves_.iter_mut().collect();
			
			for &ChosenMove(idx, ref rnk_mv, ref partition) in seq {
				let idx = idx as usize;
				
//				refs_[idx].rank += NRPA_ALPHA;
//				let exp_ranks : Vec<f32> = moves.iter().map(|mv| fastexp(mv.rank)).collect();
//				let z : f32 = exp_ranks.iter().fold(0., |acc, erank| acc + erank);
//				for i in 0..refs_.len() {
//					refs_[i].rank -= NRPA_ALPHA * exp_ranks[i] / z;
//				}
//				
//				let (m, _) = filter_indices(moves, &partition.incl);
//				moves = m;
//				let (r, _) = filter_indices(refs_, &partition.incl);
//				refs_ = r;

				
				refs_[idx].rank += NRPA_ALPHA;
				let (mut incl, mut excl) = filter_indices(refs_, &partition.incl);
				let exp_ranks : Vec<f32> = excl.iter().map(|mv| fastexp(mv.rank)).collect();
				let z : f32 = exp_ranks.iter().fold(0., |acc, erank| acc + erank);
				for i in 0..excl.len() {
					excl[i].rank -= NRPA_ALPHA * exp_ranks[i] / z;
				}
				refs_ = incl;
				
				let (m, _) = filter_indices(moves, &partition.incl);
				moves = m;
			}
		}
		
		moves_
	}
	
	
	fn fixup_board<Move: AsRef<Placement>+Clone> (&mut self, seq: Vec<Move>) -> (Vec<Move>, Eff) {
//		let fixed_indices: Vec<usize>;
//		let eff: Eff;
//		{
//			// TODO: this may be expensive, might need to cache
//			let mut board = Board::new(self.h, self.w, &mut self.rng);
//			
//			for (i, mv) in seq.iter().enumerate() {
//				let idxmv = IndexedMove { place: mv.as_ref(), moves_idx: i };
//				board.place(idxmv); 
//			}
//			
//			let fixed : Vec<IndexedMove> = board.fixup_adjacent();
//			fixed_indices = fixed.into_iter().map(|idxmv| idxmv.moves_idx).collect();
//			eff = board.efficiency();
//		};
//		
//		(filter_indices(seq, &fixed_indices), eff)

		let mut board = Board::new(self.h, self.w, &mut *self.rng);
		
//		board.place(seq);
//		let fixed : Vec<Move> = board.fixup_adjacent();
//		let eff = board.efficiency();
//		(fixed, eff)

		board.place(seq.clone());
		let eff = board.efficiency();
		(seq, eff)
	}
	
	
	
	
	fn partition_compat<'a> (mv: RankedMove, refs: &Vec<&'a mut RankedMove>) -> Partition {
		let init = Partition { incl: Vec::with_capacity(refs.len()), 
							   excl: Vec::with_capacity(refs.len()) 
		};
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


