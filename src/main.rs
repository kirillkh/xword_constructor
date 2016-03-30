extern crate ndarray;

use self::board::Board;
use self::common::{dim, Word, Orientation, Placement};

mod board;
mod common;

fn main() {
    println!("Hello, world! 2 3");
}


use std::cmp::max;

//struct Game {
//	fixedMoves: Vec<RankedMove>,
//	openMoves: BTreeSet<RankedMove>,
//}
//
//impl Game {
////	fn fixMove(&mut self, mv: RankedMove) {
////		let h = self.openMoves.clone();
////		for m in h {
////			if !m.place.compatible(mv.place) {
////				self.openMoves.
////			}
////		}
////		
////		self.openMoves.into_iter().filter(|mv| mv.place.compatible(mv.place)).collect()
////		
////		self.openMoves.drain().filter(|mv| mv.place.compatible(mv.place)).collect()
////	}
//}

// returns only the moves with improved rank for backpropagation

const TOP_N_MOVES: usize = 1000;
const NRPA_ITERS: u32 = 100;
const NRPA_ALPHA: f32 = 1.0;


struct Game {
	h: dim,
	w: dim,
	moves: Vec<RankedMove>,
}

impl Game {
	
	// http://www.chrisrosin.com/rosin-ijcai11.pdf
	fn nrpa(&mut self, level: u8, moves: &[RankedMove]) -> Vec<ChosenMove> {
		let mut moves = moves.to_vec();
		let mut best_seq = vec![];
			
		if level == 0 {
			{
				let mut refs : Vec<&mut RankedMove> = moves.iter_mut().collect();
				
				// random rollout according to the policy
				while !refs.is_empty() {
					// 1. choose a move based on probability proportional to exp(move.rank)
					let chosen_idx = 0; // TODO
					let mv = refs[chosen_idx].clone();
					
					// 2. filter compatible moves 
					Game::filter_compat(mv.clone(), &mut refs);
					
					// 3. append the move to the seq
					best_seq.push(ChosenMove(chosen_idx as u32, mv));
				}
			}
			
			self.fixup_board(&best_seq)
			
		} else {
			let mut moves : Vec<RankedMove> = moves.to_vec();
			for i in 0..NRPA_ITERS {
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
			
			for &ChosenMove(idx, _) in seq {
				let idx = idx as usize;
				refs_[idx].rank += NRPA_ALPHA;
				let z = refs.iter().fold(0., |acc, mv| acc + mv.rank.exp());
				for i in 0..refs_.len() {
					refs_[i].rank -= NRPA_ALPHA * refs[i].rank.exp() / z;
				}
				
				Game::filter_compat(refs[idx].clone(), &mut refs);
				Game::filter_compat(refs_[idx].clone(), &mut refs_);
			}
		}
		
		moves_
	}
	
	
	fn fixup_board<Move: AsRef<Placement>+Clone> (&self, seq: &[Move]) -> Vec<Move> {
		#[derive(Clone)]
		struct IndexedMove<'a> {
			place: &'a Placement,
			moves_idx: usize,
		}
		
		impl<'a> AsRef<Placement> for IndexedMove<'a> {
			fn as_ref(&self) -> &Placement { &self.place }
		}
//		
		// TODO: this is very expensive, need to cache
		let mut board = Board::new(self.h, self.w);
		
		for (i, mv) in seq.iter().enumerate() {
			let idxmv = IndexedMove { place: mv.as_ref(), moves_idx: i };
			board.place(idxmv); 
		}
		
		let fixed = board.fixup_adjacent();
		let mut seq_ = vec![None; seq.len()] as Vec<Option<Move>>;
		for idxmv in fixed {
			seq_[idxmv.moves_idx] = Some(seq[idxmv.moves_idx].clone())
		}
		
		seq_.into_iter().filter_map(|mv| mv).collect()
	}
	
	
	
	
	fn filter_compat<'a> (mv: RankedMove, refs: &mut Vec<&'a mut RankedMove>) {
		refs.retain(|other| mv.place.compatible(&other.place))
//		refs.into_iter().filter(|other| mv.place.compatible(&other.place)).collect()
	}
}


struct PlayoutResult {
	ranks: Vec<u32>,
	max_rank: u32,
}



#[derive(Clone)]
struct RankedMove {
	place: Placement,
	rank: f32,
}

impl AsRef<Placement> for RankedMove {
	fn as_ref(&self) -> &Placement {
		&self.place
	}
}



#[derive(Clone)]
struct ChosenMove(u32, RankedMove);

impl AsRef<Placement> for ChosenMove {
	fn as_ref(&self) -> &Placement {
		&self.1.place
	}
}






//	fn random_playout(&self, words_placed: u32, places: &[Placement]) -> PlayoutResult {
//		
//		if places.is_empty() {
//			return PlayoutResult { ranks: vec![], max_rank: words_placed }
//		}
//		
//		let mut ranks = Vec::with_capacity(places.len());
//		for _ in 0..places.len() {  // TODO: optimize this
//			ranks.push(0);
//		}
//	
//	//	let max_rank = 0;
//		
//		// 1. take the highest-ranked move
//		let next_move = &places[0];
//		
//		// 2. filter out incompatible places
//		let mut compat_indices = Vec::with_capacity(places.len());
//		let mut compat_places: Vec<Placement> = Vec::with_capacity(places.len());
//		
//		for (i, mv) in places.iter().skip(1).enumerate() {
//			if mv.compatible(next_move) { // TODO: we can pre-calculate the set of incompatible places for each move and here just check it 
//				compat_places.push(*mv);
//				compat_indices.push(1+i);
//			}
//		}
//		
//		// 3. play it
//		let rslt = random_playout(words_placed+1, &*compat_places);
//		
//		// 4. update the rankings based on the returned values
//		for (i, rnk) in rslt.ranks.iter().enumerate() {
//			let mut r = &mut ranks[compat_indices[i]];
//			*r = max(*r, *rnk); 
//		}
//		
//		ranks[0] = max(ranks[0], rslt.max_rank);
//	//	max_rank = max(max_rank, rslt.max_rank);
//		
//		PlayoutResult { ranks: ranks, max_rank: max_rank }
//	}
