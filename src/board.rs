use std::collections::{HashMap, HashSet};
use ndarray::OwnedArray;
use common::{dim, Placement, PlacementId, MatrixDim};


//---- BoardMove -----------------------------------------------------------------------

#[derive(Clone)]
pub struct BoardMove<Move: AsRef<Placement>+Clone> {
	mv: Move,
	dependants: Vec<PlacementId>
}


//---- Board ---------------------------------------------------------------------------

pub struct Board<Move: AsRef<Placement>+Clone> {
	field: OwnedArray<Vec<PlacementId>, MatrixDim>,
	moves: HashMap<PlacementId, BoardMove<Move>>
}

impl<Move: AsRef<Placement>+Clone> Board<Move> {
	pub fn new(h: dim, w: dim) -> Board<Move> {
		Board { field: OwnedArray::default(MatrixDim(w, h)), moves: HashMap::new() }
	}
	
	pub fn place(&mut self, mv: Move) {
		let bmv = BoardMove{ mv: mv, dependants: vec![] };
		let place_id = {
			let place = bmv.mv.as_ref();
			place.fold_positions((), |_, x, y| {
				self.field[MatrixDim(x, y)].push(place.id);
			});
			place.id
		};
		self.moves.insert(place_id, bmv);
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
			place.fold_positions(init, |adjacency, x, y| {
				if self.field[MatrixDim(x, y)].len() == 2 {
					if let Some(prev_isection) = adjacency.last_isection {
						if !adjacency.added_last {
							deps.push((id, prev_isection)); 
						}
						deps.push((id, place.id)); 
						
						AdjacencyTracker { last_isection: Some(place.id), added_last: true }
					} else {
						AdjacencyTracker { last_isection: Some(place.id), added_last: false }
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
		let mut adjacencies : HashSet<PlacementId> = self.moves.iter().filter_map(|(&id, bmv)| {
			let place = bmv.mv.as_ref();
			let perp = place.orientation.align(0, 1);
			let adj_found = place.fold_positions(false, |acc, x, y|
				acc || {
					// if there is an intersection at (x,y), it is impossible to have adjacency problems with the neighbours
					if self.field.get(MatrixDim(x, y)).unwrap().len() == 2 {
						return false;
					}
					
					// otherwise do the neighbour check
					let (x, y) = (x as isize, y as isize);
					let (xd, yd) = (perp.0 as isize, perp.1 as isize);
					let (x1, y1) = (x + xd, y + yd);
					let (x2, y2) = (x - xd, y - yd);
					let neighbours = [
						self.field.get(MatrixDim(x1 as dim, y1 as dim)) as Option<&Vec<PlacementId>>,
						self.field.get(MatrixDim(x2 as dim, y2 as dim))
					];
					
					neighbours.iter().any(|optvec| optvec.map_or(false, |vec| vec.len()==1))
				} 
			);
			
			if adj_found { Some(id) }
			else { None }
		}).collect();
		
		// 3. remove them all! (this can be improved: remove random words, then recheck; rinse, repeat)
		let mut bmvs = self.moves.clone();
		while !adjacencies.is_empty() {
			adjacencies = adjacencies.into_iter().flat_map(|adj| {
				let bmv : Option<BoardMove<Move>> = bmvs.remove(&adj);
				let empty = || vec![].into_iter();
				bmv.map_or_else(empty, |bmv| bmv.dependants.into_iter())
			}).collect();
		};
		
		bmvs.into_iter().map(|(_, bmv)| bmv.mv).collect()
	}
}








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
