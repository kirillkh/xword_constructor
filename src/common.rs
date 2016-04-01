//---- Dim -------------------------------------------------------------------------------
use std::ops::{Index, Deref};
use ndarray::{Dimension, Si, RemoveAxis, Axis, Ix};

#[allow(non_camel_case_types)]
pub type dim = Ix;


#[derive(Debug, Clone, PartialEq)]
pub struct MatrixDim(pub dim, pub dim);

impl MatrixDim {
	fn tuple(t: (dim, dim)) -> MatrixDim { MatrixDim(t.0, t.1) }
}

impl Index<usize> for MatrixDim {
    type Output = dim;

    fn index(&self, index: usize) -> &Self::Output {
		match index {
			0 => &self.0,
			1 => &self.1,
			_ => panic!("wrong dimension")
		}
    }
}

impl Eq for MatrixDim {}

unsafe impl Dimension for MatrixDim {
	type SliceArg = [Si; 2];
	#[inline]
	fn ndim(&self) -> usize { 2 }
	
    fn size(&self) -> usize {
    	self.0 * self.1
    }
}

impl RemoveAxis for MatrixDim {
    type Smaller = LineDim;
    
    fn remove_axis(&self, axis: Axis) -> Self::Smaller {
//    	if let Axis(1) = axis {
//    		
//    	} else {
//    		panic!("slicing not supported");
//    	}
		match axis {
			Axis(0) => LineDim(self.1),
			Axis(1) => LineDim(self.0),
			_ => panic!("wrong axis")
		}
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LineDim(pub dim);

impl Eq for LineDim {}


unsafe impl Dimension for LineDim {
	type SliceArg = [Si; 1];
	#[inline]
	fn ndim(&self) -> usize { 1 }
}


impl Deref for LineDim {
	type Target = dim;
	
    fn deref(&self) -> &Self::Target {
    	&self.0
    }
}

//---- Word ----------------------------------------------------------------------------
pub type WordId = u32;

#[derive(Clone, Debug)]
pub struct Word {
	pub id: WordId, // unique id
	pub str: Box<[u8]>,
}

impl Word {
	pub fn new(id: WordId, str: Box<[u8]>) -> Word {
		Word { id:id, str:str }
	}
	
	pub fn len(&self) -> dim {
		self.str.len() as dim
	}
}

impl ::std::ops::Index<dim> for Word {
	type Output = u8;
	
    fn index(&self, index: dim) -> &Self::Output {
    	&self.str[index as usize]
    }
}


//---- Orientation ---------------------------------------------------------------------
#[derive(PartialEq, Clone, Copy, Hash, Debug)]
pub enum Orientation { 
	HOR = 1,
	VER = 0 
}

impl Orientation {
	pub fn values() -> &'static [Orientation] { 
		static VALUES : &'static [Orientation] = &[Orientation::VER, Orientation::HOR];
		VALUES
	}
	
	pub fn align(self, u: dim, v: dim) -> (dim, dim) {
		let d0 = self as dim;
		let d1 = 1 - d0;
		
		(u.cond(d0) + v.cond(d1), u.cond(d1) + v.cond(d0))
	}
	
//	pub fn by_val(val: usize) -> Orientation {
//		match val {
//			Orientation::HOR => Orientation::HOR
//		}
//	}
}


//---- Placement -----------------------------------------------------------------------
pub type PlacementId = u32;

#[derive(Clone, Debug)]
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
		let (x, y, len) = (self.x, self.y, self.word.len() as dim);
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
		let (len0, len1) = (self.word.len(), other.word.len());
		
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

//impl Cond for usize {
//	fn cond(self, flag: Self) -> Self {
//		let (a, f) = (self as i32, flag as i32);
//		(a & -f) as Self
//	}
//}

impl Cond for dim {
	fn cond(self, flag: Self) -> Self {
		let (a, f) = (self as i64, flag as i64);
		(a & -f) as Self
	}
}

