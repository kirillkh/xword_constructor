//---- Dim -----------------------------------------------------------------------------
use ndarray::{Dimension, Si};

pub type dim = i8;

#[derive(Debug, Clone, PartialEq)]
pub struct Dim(pub dim, pub dim);

impl Dim {
	fn tuple(t: (dim, dim)) -> Dim { Dim(t.0, t.1) }
}

impl Eq for Dim {}

unsafe impl Dimension for Dim {
	type SliceArg = [Si; 2];
	#[inline]
	fn ndim(&self) -> usize { 2 }
}


//---- Word ----------------------------------------------------------------------------
pub type WordId = u32;

#[derive(Clone)]
pub struct Word {
	id: WordId, // unique id
	str: Box<[dim]>,
	len: dim,
}

impl ::std::ops::Index<dim> for Word {
	type Output = dim;
	
    fn index(&self, index: dim) -> &Self::Output {
    	&self.str[index as usize]
    }
}


//---- Orientation ---------------------------------------------------------------------
#[derive(PartialEq, Clone, Copy, Hash)]
pub enum Orientation { 
	HOR = 1,
	VER = 0 
}

impl Orientation {
	pub fn align(self, x: dim, y: dim) -> (dim, dim) {
		let d0 = self as dim;
		let d1 = 1 - d0;
		
		(x.cond(d0) + y.cond(d1), x.cond(d1) + y.cond(d0))
	}
}


//---- Placement -----------------------------------------------------------------------
pub type PlacementId = u32;

#[derive(Clone)]
pub struct Placement {
	pub id: PlacementId,
	pub x: dim,
	pub y: dim,
	pub orientation: Orientation,
	pub word: Word,
}

impl Placement {
	pub fn new(id: PlacementId, orientation: Orientation, x: dim, y: dim, word: Word) -> Placement {
		Placement { id:id, orientation:orientation, x:x, y:y, word:word }
	}
	
	pub fn align(&self, or: Orientation) -> (dim, dim) {
		or.align(self.x, self.y)
	}
	
	pub fn fold_positions<A, F>(&self, init: A, mut f: F) -> A 
			where F: FnMut(A, dim, dim) -> A {
		let (x, y, len) = (self.x, self.y, self.word.len as dim);
		let (xc, yc) = self.orientation.align(1, 0);
		
		let mut acc = init;
		for i in 0..len {
			acc = f(acc, x + i.cond(xc), y + i.cond(yc));
		}
		acc
	}
	
	pub fn compatible(&self, other: &Placement) -> bool {
		if self.word.id == other.word.id {
			return false
		}
		
		let (u0, v0) = self.align(self.orientation);
		let (u1, v1) = other.align(self.orientation);
		let (len0, len1) = (self.word.len, other.word.len);
		
		if other.orientation == self.orientation {
				u0 + len0	< u1 || u1 + len1	< u0 // other is right or left + gap
			||	v0 != v1							 // other is below or above
		} else {
				v0 + 1 		< v1 || v1 + len1	< v0 // other is below or above + gap
			||	u0 + len0	< u1 || u1 + 1		< u0 // other is right or left + gap
			||	(									 // intersection
				(u0 <= u1 && u1+1 <= u0+len0) && (v1 <= v0 && v0+1 <= v1+len1)
				&& self.word[u1-u0] == other.word[v0-v1]
			)
		}
	}
}


//---- Cond ----------------------------------------------------------------------------
//pub fn cond(arg: u8, flag: u8) -> u8 {
//	let (a, f) = (arg as i32, flag as i32);
//	(a & -f) as u8
//}

trait Cond {
	fn cond(self, flag: Self) -> Self;
}

impl Cond for usize {
	fn cond(self, flag: Self) -> Self {
		let (a, f) = (self as i32, flag as i32);
		(a & -f) as Self
	}
}

impl Cond for dim {
	fn cond(self, flag: Self) -> Self {
		let (a, f) = (self as i32, flag as i32);
		(a & -f) as Self
	}
}

