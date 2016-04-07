//---- Dim -------------------------------------------------------------------------------
use std::ops::{Index, Deref};
use std::rc::Rc;
use ndarray::{Dimension, Si, RemoveAxis, Axis, Ix, OwnedArray};
use rand::{SeedableRng, XorShiftRng, Rng, thread_rng};
use rand::distributions::{IndependentSample, Range};

#[allow(non_camel_case_types)]
pub type dim = Ix;


#[derive(Debug, Clone, Copy, PartialEq)]
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
	pub str: Rc<Box<[u8]>>,
}

impl Word {
	pub fn new(id: WordId, str: Box<[u8]>) -> Word {
		Word { id:id, str: Rc::new(str) }
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
//				u0 + len0	<= u1 || u1 + len1	<= u0 // other is right or left
			||	v0 != v1							 // other is below or above
		} else {
//				v0 + 1 		< v1 || v1 + len1	< v0 // other is below or above + gap
//			||	u0 + len0	< u1 || u1 + 1		< u0 // other is right or left + gap

			!(
					(v1 <= v0 + 1	&& v0		<= v1 + len1 &&	// other contains or touches self vertically
					 u0 <= u1		&& u1 + 1 	<= u0 + len0) 	// other contains self horizontally
				||	(v1 <= v0 && v0 + 1 <= v1 + len1 && 		// other contains self vertically
					 (u1 + 1 == u0 || u0 + len0 == u1))			// other touches self horizontally
			)

				
//				v0 + 1 		<= v1 || v1 + len1	<= v0 // other is below or above
//			||	u0 + len0	<= u1 || u1 + 1		<= u0 // other is right or left
			||	(									 // intersection
				(u0 <= u1 && u1+1 <= u0+len0) && (v1 <= v0 && v0+1 <= v1+len1) &&
				self.word[u1-u0] == other.word[v0-v1]
			)
		}
	}
}

impl AsRef<Placement> for Placement {
	fn as_ref(&self) -> &Placement { &self }
}


//---- Problem -------------------------------------------------------------------------

pub struct Problem {
	pub dic: Vec<Word>,
	pub board: OwnedArray<bool, MatrixDim>,
}

impl Problem {
	pub fn new(dic: Vec<Word>, board: OwnedArray<bool, MatrixDim>) -> Problem {
		Problem { dic:dic, board:board }
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




#[cfg(test)]
mod tests {
    use super::*;
	use super::Orientation::*;

    #[test]
    fn same_line_incompat_placements() {
    	let word1 = Word::new(0, b"abc".to_vec().into_boxed_slice());
    	let word2 = Word::new(1, b"ab".to_vec().into_boxed_slice());
    	let p1 = Placement::new(0, Orientation::HOR, 0, 0, word1);
    	let p2 = Placement::new(1, Orientation::HOR, 0, 0, word2);
    	
    	assert_eq!(p1.compatible(&p2), false);
    	assert_eq!(p2.compatible(&p1), false);
    }

    #[test]
    fn compat_placements() {
    	let word = |wid, str: &[u8]| Word::new(wid, str.to_vec().into_boxed_slice());
    	let place = |pid, or, x, y, word| Placement::new(pid, or, x, y, word);
    	
    	{
	    	let word1 = word(0, b"abc");
	    	let word2 = word(1, b"ab");
	    	let p1 = place(0, HOR, 0, 0, word1);
	    	let p2 = place(1, VER, 0, 0, word2);
	    	
	    	assert_eq!(p1.compatible(&p2), true);
	    	assert_eq!(p2.compatible(&p1), true);
    	}
    	
    	{
	    	let word1 = word(0, b"bc");
	    	let word2 = word(1, b"ab");
	    	let p1 = place(0, HOR, 0, 1, word1);
	    	let p2 = place(1, VER, 0, 0, word2);
	    	
	    	assert_eq!(p1.compatible(&p2), true);
	    	assert_eq!(p2.compatible(&p1), true);
    	}
    	
    }
}



    
pub trait AbstractRng {
    fn gen_f32(&mut self, between: Range<f32>) -> f32;
    fn gen_usize(&mut self, between: Range<usize>) -> usize;
    fn gen_u8(&mut self, between: Range<u8>) -> u8;
}


struct XRng<R: Rng>(R);

impl<R: Rng> AbstractRng for XRng<R> {
    fn gen_f32(&mut self, between: Range<f32>) -> f32 {
        between.ind_sample(&mut self.0)
    }
    
    fn gen_usize(&mut self, between: Range<usize>) -> usize {
        between.ind_sample(&mut self.0)
    }
    
    fn gen_u8(&mut self, between: Range<u8>) -> u8 {
        between.ind_sample(&mut self.0)
    }
}



pub fn make_rng() -> Box<AbstractRng> {
//	let seed: [u32;4] = [1, 2, 3, 555];
//	Box::new(XRng(XorShiftRng::from_seed(seed)))
	Box::new(XRng(thread_rng()))
}


pub fn filter_indices<T> (mut items: Vec<T>, indices: &[usize]) -> (Vec<T>, Vec<T>) {
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
//	items.truncate(j);
//	items
	let excl = items.split_off(j);
	(items, excl)
}
