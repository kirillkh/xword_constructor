use std::ops::Deref;
use std::collections::{HashMap, HashSet};
use std::fmt::Display;
use ndarray::OwnedArray;
use common::{dim, Placement, PlacementId, MatrixDim, filter_indices, Orientation, AbstractRng};

use rand::distributions::{IndependentSample, Range};
use rand::Rng;


//---- Eff -----------------------------------------------------------------------------
#[allow(non_camel_case_types)]
pub type eff_t = u32;

#[derive(Debug, Clone, Copy)]
pub struct Eff(pub eff_t);

impl Deref for Eff {
	type Target = eff_t;
	
    fn deref(&self) -> &Self::Target {
    	&self.0
    }
}



//---- BoardMove -----------------------------------------------------------------------

#[derive(Clone)]
pub struct BoardMove<Move: AsRef<Placement>+Clone> {
	pub mv: Move,
	moves_idx: usize,
	dependants: Vec<PlacementId>
}


//---- Board ---------------------------------------------------------------------------

pub struct Board<'a, Move: AsRef<Placement>+Clone> {
	pub field: OwnedArray<Vec<PlacementId>, MatrixDim>, // TODO: make the vecs constant size 2
	pub moves: HashMap<PlacementId, BoardMove<Move>>,
	rng: &'a mut AbstractRng
}

impl<'a, Move: AsRef<Placement>+Clone> Board<'a, Move> {
	pub fn new(h: dim, w: dim, rng: &'a mut AbstractRng) -> Board<'a, Move> {
		Board { field: OwnedArray::default(MatrixDim(h, w)), moves: HashMap::new(), rng:rng }
	}
	
	pub fn place(&mut self, seq: Vec<Move>) {
		for (i, mv) in seq.into_iter().enumerate() {
			let place_id = {
				let place = mv.as_ref();
				place.fold_positions((), |_, y, x| {
					self.field[MatrixDim(y, x)].push(place.id);
				});
				place.id
			};
			let bmv = BoardMove { mv:mv, moves_idx: i, dependants: vec![] };
			
			self.moves.insert(place_id, bmv);
		}
	}
	
	
	pub fn delete(&mut self, id: PlacementId) -> Option<BoardMove<Move>> {
		let bmv = self.moves.remove(&id);
		if let Some(ref bmv) = bmv {
			let place = bmv.mv.as_ref();
			place.fold_positions((), |_, y, x| {
					let items = &mut self.field[MatrixDim(y, x)];
					if items.len() == 0 {
					} else if items.len() == 1 || items[0] == id {
						items.swap_remove(0);
					} else {
						items.remove(1);
					}
			});
		}
		
		// TODO: clean up the list of dependants of the move bmv depends upon
		
		bmv
	}
	
	/// Returns 1 for every letter intersected on the board.
	pub fn efficiency(&self) -> Eff {
		let intersections = self.moves.values().fold(0, |acc, bmv| { 
				let place = bmv.mv.as_ref();
				place.fold_positions(acc, |acc, y, x| {
					acc + self.field[MatrixDim(y, x)].len() - 1
				})
		}) as u32;
		
		Eff(intersections / 2)
	}
	
	
	pub fn fixup_adjacent(&mut self) -> Vec<Move> {
		// 1. build the dependency graph, so that when we start removing Moves, we know exactly which other Moves we're breaking
		struct AdjacencyTracker {
			last_isection: Option<PlacementId>,
			added_last: bool
		}
		
		let mut deps : Vec<(PlacementId, PlacementId)> = Vec::new();
		
		for (&id, bmv) in self.moves.iter() {
			let init = AdjacencyTracker { last_isection: None, added_last: false };
			let place = bmv.mv.as_ref();
			place.fold_positions(init, |adjacency, y, x| {
				let placements = &self.field[MatrixDim(y, x)];
				if placements.len() == 2 {
					let other_id = 
						if placements[0] == id { placements[1] } 
						else { placements[0] };
					if let Some(prev_isection) = adjacency.last_isection {
						if !adjacency.added_last {
							deps.push((id, prev_isection)); 
						}
						deps.push((id, other_id)); 
						
						AdjacencyTracker { last_isection: Some(other_id), added_last: true }
					} else {
						AdjacencyTracker { last_isection: Some(other_id), added_last: false }
					}
				} else {
					AdjacencyTracker { last_isection: None, added_last: false }
				}
			});
		}
		
		for (dependency, dependant) in deps {
			self.moves.get_mut(&dependency).map(|d| d.dependants.push(dependant));
		}
		
		
		// 2. collect adjancent words that need to be fixed (no word intersects them both at some position)
		let mut adjacencies : HashSet<PlacementId> = self.find_adjacencies(self.moves.values());
		
		// TODO: we should delete moves with probability inverse proportional to their rank 
		let between = Range::new(0, 2);
//		let mut rng = rand::thread_rng();

		while !adjacencies.is_empty() {
			let mut adj_vec : Vec<_> = adjacencies.into_iter().collect();
			if cfg!(feature = "debug_rng") {
//				adj_vec.sort();
			}
			adjacencies = adj_vec.into_iter().flat_map(|adj| {
					let v = self.rng.gen_usize(between);
					
					let out : Vec<_> = 
						if v==0 {
							let bmv : Option<BoardMove<Move>> = self.delete(adj);
							if let Some(bmv) = bmv {
								bmv.dependants
							} else {
								vec![]
							}
						} else {
							vec![adj]
						};
					out.into_iter()
			}).collect();
			
			let suspects = adjacencies.into_iter()
								  	  .filter(|dep| self.moves.contains_key(&dep))
								      .map(|adj| &self.moves[&adj]);
			adjacencies = self.find_adjacencies(suspects);
		};
		
		let mut fixed : Vec<_> = self.moves.values().cloned().collect();
		fixed.sort_by(|bmv1, bmv2| bmv1.moves_idx.cmp(&bmv2.moves_idx));
		fixed.into_iter().map(|bmv| bmv.mv).collect()
//		
//		let (fixed_moves, fixed_indices) : (Vec<_>, Vec<_>) = 
//			fixed.into_iter()
//			.map(|bmv| (bmv.mv, bmv.moves_idx))
//			.unzip();
//		
//		filter_indices(fixed_moves, &fixed_indices)
	}
	
	
	fn find_adjacencies<'b, Iter: Iterator<Item=&'b BoardMove<Move>>>(&self, suspect_moves: Iter) -> HashSet<PlacementId>
	where Move: 'b
	{
		let adjacencies : HashSet<PlacementId> = suspect_moves.filter_map(|bmv| {
			let place = bmv.mv.as_ref();
			let perp = place.orientation.align(1, 0);
			let (yd, xd) = (perp.0 as isize, perp.1 as isize);
			let adj_found = place.fold_positions(false, |acc, y, x|
				acc || {
					// if there is an intersection at (x,y), it is impossible to have adjacency problems with the neighbours
					if self.field[MatrixDim(y, x)].len() == 2 {
						return false;
					}
					
					// otherwise do the neighbour check
					let (y, x) = (y as isize, x as isize);
					let (y1, x1) = (y + yd, x + xd);
					let (y2, x2) = (y - yd, x - xd);
					let neighbours = [
						self.field.get(MatrixDim(y1 as dim, x1 as dim)) as Option<&Vec<PlacementId>>,
						self.field.get(MatrixDim(y2 as dim, x2 as dim))
					];
					
					neighbours.iter().any(|optvec| optvec.map_or(false, |vec| vec.len()==1))
				} 
			);
			
			if adj_found { Some(place.id) }
			else { None }
		}).collect();
		
		adjacencies
	}
	

	pub fn print(&self) {
		let field = &self.field;
		for j in 0..field.dim()[0] {
			for i in 0..field.dim()[1] {
				let opt_pid = field[MatrixDim(j, i)].first();
				if let Some(pid) = opt_pid {
						let plc = &self.moves[pid].mv.as_ref();
						
						match plc.orientation {
							Orientation::VER =>
								print!("{}", plc.word.str[j - plc.y] as char),
							Orientation::HOR =>
								print!("{}", plc.word.str[i - plc.x] as char),
						}
				} else {
								print!("_")
				}
			}
			
			print!("\n");
		}
		print!("\n");
	}
}



//#[derive(Clone)]
//struct IndexedMove<'a> {
//	place: &'a Placement,
//	moves_idx: usize,
//}
//
//impl<'a> AsRef<Placement> for IndexedMove<'a> {
//	fn as_ref(&self) -> &Placement { &self.place }
//}







//impl<Move: AsRef<Placement>> Hash for Move {
//    fn hash<H: Hasher>(&self, state: &mut H) {
//    	let p = self.as_ref();
////    	p.x.hash(state);
////    	p.y.hash(state);
////    	p.orientation.hash(state);
////    	p.word.id.hash(state);
//    	p.id.hash(state);
//    }
//}
//
//impl<Move: AsRef<Placement>> PartialEq for Move {
//    fn eq(&self, other: &Self) -> bool {
//    	let p = self.as_ref();
//    	let o = other.as_ref();
////    	p.x == o.x && p.y == o.y && p.orientation == o.orientation && p.word.id == o.word.id
//    	p.id == o.id
//    }
//}
//
//impl<Move: AsRef<Placement>> Eq for Move {}
