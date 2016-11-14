//---- Dim -------------------------------------------------------------------------------
use std::ops::{Index, IndexMut, Deref};
use std::cmp::{max, min};
use ndarray::{Dimension, Si, RemoveAxis, Axis, Ix, Array};
use rand::{SeedableRng, XorShiftRng, thread_rng};
use rand::distributions::{IndependentSample, Range};
use global2::data::ScoredMove;
use global2::weighted_selection_tree::Key;
use global2::sliced_arena::SlicedArena;
use std::mem;

#[derive(Clone, Debug, Copy)]
#[repr(simd)]
pub struct Align64 { al1: u64, al2: u64 }



#[allow(non_camel_case_types)]
pub type dim = Ix;


#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct MatrixDim(pub dim, pub dim);

impl MatrixDim {
	#[inline]
	fn tuple(t: (dim, dim)) -> MatrixDim { MatrixDim(t.0, t.1) }
}

impl Index<usize> for MatrixDim {
    type Output = dim;

	#[inline]
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
	
	#[inline]
    fn size(&self) -> usize {
    	self.0 * self.1
    }
}

impl RemoveAxis for MatrixDim {
    type Smaller = LineDim;
    
	#[inline]
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

#[derive(Debug, Clone, PartialEq, Default)]
pub struct LineDim(pub dim);

impl Eq for LineDim {}


unsafe impl Dimension for LineDim {
	type SliceArg = [Si; 1];
	#[inline]
	fn ndim(&self) -> usize { 1 }
}


impl Deref for LineDim {
	type Target = dim;
	
	#[inline]
    fn deref(&self) -> &Self::Target {
    	&self.0
    }
}

//---- Word ----------------------------------------------------------------------------
pub type WordId = usize;

#[derive(Clone, Debug)]
pub struct Word {
	pub id: WordId, // unique id
	pub str: &'static [u8],
}

impl Word {
	pub fn new(id: WordId, str: &'static [u8]) -> Word {
		Word { id:id, str:str }
	}
	
	#[inline]
	pub fn len(&self) -> dim {
		self.str.len() as dim
	}
}

impl ::std::ops::Index<dim> for Word {
	type Output = u8;
	
	#[inline]
    fn index(&self, index: dim) -> &Self::Output {
    	&self.str[index as usize]
    }
}

//---- Orientation ---------------------------------------------------------------------
#[derive(PartialEq, Clone, Copy, Hash, Debug)]
pub enum Orientation { 
	VER = 0, 
	HOR = 1
}

impl Orientation {
	#[inline]
	pub fn values() -> &'static [Orientation] { 
		static VALUES : &'static [Orientation] = &[Orientation::VER, Orientation::HOR];
		VALUES
	}
	
	#[inline]
	pub fn perp_orientation(self) -> Orientation {
		Self::values()[1 - (self as dim)] 
	}
	
	#[inline]
	pub fn align(self, v: dim, u: dim) -> (dim, dim) {
		let d0 = self as dim;
		let d1 = 1 - d0;
		
		(v.cond(d0) | u.cond(d1), v.cond(d1) | u.cond(d0))
	}
	
//	pub fn by_val(val: usize) -> Orientation {
//		match val {
//			Orientation::HOR => Orientation::HOR
//		}
//	}
}


//---- Placement -----------------------------------------------------------------------
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct PlacementId(pub usize);

//type PlacementId = u32;

//impl PlacementId {
//    #[inline]
//    pub fn u(&self) -> usize {
//        self.0 as usize
//    }
//}

impl Key for PlacementId {
    #[inline]
    fn usize(self) -> usize {
        self.0 as usize
    }
}

impl Index<PlacementId> for [Placement] {
    type Output = Placement;

	#[inline]
    fn index(&self, index: PlacementId) -> &Self::Output {
		self.index(index.0 as usize)
    }
}

impl Index<PlacementId> for Vec<Placement> {
    type Output = Placement;

	#[inline]
    fn index(&self, index: PlacementId) -> &Self::Output {
		self.index(index.0 as usize)
    }
}

impl Index<PlacementId> for [ScoredMove] {
    type Output = ScoredMove;

	#[inline]
    fn index(&self, index: PlacementId) -> &Self::Output {
		self.index(index.0 as usize)
    }
}

impl Index<PlacementId> for Vec<ScoredMove> {
    type Output = ScoredMove;

	#[inline]
    fn index(&self, index: PlacementId) -> &Self::Output {
		self.index(index.0 as usize)
    }
}

impl IndexMut<PlacementId> for Vec<ScoredMove> {
//    type Output = ScoredMove;

	#[inline]
    fn index_mut(&mut self, index: PlacementId) -> &mut Self::Output {
		self.index_mut(index.0 as usize)
    }
}



//impl ::std::ops::Index<PlacementId> for Vec<ScoredMove> {
//    type Output = ScoredMove;
//
//    fn index(&self, index: PlacementId) -> &ScoredMove {
//        &self[index.0 as usize]
//    }
//}

//impl Index<PlacementId> for [Placement] {
//impl<T: Index<usize, Output=Placement>> Index<PlacementId> for T {
//    type Output = Placement;
//
//	#[inline]
//    fn index(&self, index: PlacementId) -> &Self::Output {
//		self.index(index.0 as usize)
////        self.index(5)
//    }
//}




#[derive(Clone, Debug)]
pub struct Placement {
	pub id: PlacementId,
	pub y: dim,
	pub x: dim,
	pub word: Word,
	pub orientation: Orientation,
}

impl Placement {
	pub fn new(id: usize, orientation: Orientation, y: dim, x: dim, word: Word) -> Placement {
		Placement { id:PlacementId(id), orientation:orientation, y:y, x:x, word:word }
	}
	
	#[inline]
	pub fn align(&self, or: Orientation) -> (dim, dim) {
		or.align(self.y, self.x)
	}
	
	#[inline]
	pub fn fold_positions<A, F>(&self, init: A, mut f: F) -> A 
			where F: FnMut(A, dim, dim) -> A {
		self.fold_positions_index(init, |acc, y, x, _| f(acc, y, x))
	}
	
	#[inline]
	pub fn fold_positions_index<A, F>(&self, init: A, mut f: F) -> A 
			where F: FnMut(A, dim, dim, dim) -> A {
		let (y, x, len) = (self.y, self.x, self.word.len() as dim);
		let (yc, xc) = self.orientation.align(0, 1);
		
		let mut acc = init;
		for i in 0..len {
		    let (dy, dx) = (i.cond(yc), i.cond(xc));
			acc = f(acc, y + dy, x + dx, dy | dx);
		}
		acc
	}
	
	#[inline]
	pub fn contains(&self, y: dim, x: dim) -> bool {
		let (yc, xc) = self.orientation.align(0, 1);
		let len = self.word.len();
		
		self.x <= x && x < self.x + len.cond(xc) + 1.cond(yc) &&
		self.y <= y && y < self.y + len.cond(yc) + 1.cond(xc)
	}
	
	pub fn compatible(&self, other: &Placement) -> bool {
		if self.word.id == other.word.id {
			return self.id == other.id
		}
		
		let (v0, u0) = self.align(self.orientation);
		let (v1, u1) = other.align(self.orientation);
		let (len0, len1) = (self.word.len(), other.word.len());
		
		if other.orientation == self.orientation {
				u0 + len0	< u1 || u1 + len1	< u0 // other is right or left + gap
			||	v0 != v1							 // other is below or above
			// TEST
			&& (((v0 as isize) - (v1 as isize)).abs()>1 || {
					let overlap_from = max(u0, u1);
					let overlap_to = min(u0+len0, u1+len1);
					overlap_from >= overlap_to
				})
		} else {
			!(
					(v1 <= v0 + 1	&& v0		<= v1 + len1 &&	// other contains or touches self vertically
					 u0 <= u1		&& u1 + 1 	<= u0 + len0) 	// other contains self horizontally
				||	(v1 <= v0 && v0 + 1 <= v1 + len1 && 		// other contains self vertically
					 (u1 + 1 == u0 || u0 + len0 == u1))			// other touches self horizontally
			)
			||	(									 // intersection
				(u0 <= u1 && u1+1 <= u0+len0) && (v1 <= v0 && v0+1 <= v1+len1) &&
				self.word[u1-u0] == other.word[v0-v1]
			)
		}
	}
}

impl AsRef<Placement> for Placement {
	#[inline]
	fn as_ref(&self) -> &Placement { &self }
}


//---- Problem -------------------------------------------------------------------------

pub struct Problem {
    dic_arena: SlicedArena<u8>,
	pub dic: Vec<Word>,
	pub board: Array<bool, MatrixDim>,
}

impl Problem {
	pub fn new(dic: Vec<Vec<u8>>, board: Array<bool, MatrixDim>) -> Problem {
    	let (dic, dic_arena) = Self::dic_arena(dic);
		Problem { dic:dic, dic_arena:dic_arena, board:board }
	}
	
    fn dic_arena(dic: Vec<Vec<u8>>) -> (Vec<Word>, SlicedArena<u8>) {
    	let word_lens: Vec<usize> = dic.iter().map(|wordv| wordv.len()).collect();
    	let mut dic_arena: SlicedArena<u8> = SlicedArena::new(&word_lens);
    	let mut dicw: Vec<Word> = Vec::with_capacity(dic.len());
    	for (i, wordv) in dic.into_iter().enumerate() {
    	    {
        	    let slice: &mut [u8] = dic_arena.slice_mut(i);
        	    slice.clone_from_slice(&wordv);
    	    }
    	    let slice: &'static [u8] = unsafe { mem::transmute(dic_arena.slice(i)) };
    	    dicw.push(Word::new(i, slice));
    	}
    	
    	(dicw, dic_arena)
    }
}



//---- Cond ----------------------------------------------------------------------------
//pub fn cond(arg: u8, flag: u8) -> u8 {
//	let (a, f) = (arg as i32, flag as i32);
//	(a & -f) as u8
//}

pub trait Cond {
	#[inline]
	fn cond(self, flag: Self) -> Self;
}

//impl Cond for usize {
//	fn cond(self, flag: Self) -> Self {
//		let (a, f) = (self as i32, flag as i32);
//		(a & -f) as Self
//	}
//}

impl Cond for usize {
	#[inline]
	fn cond(self, flag: Self) -> Self {
		let (a, f) = (self as isize, flag as isize);
		(a & -f) as Self
	}
}



    
pub trait AbstractRng {
	fn clone_to_box(&self) -> Box<AbstractRng>;
    fn gen_f32(&self, between: Range<f32>) -> f32;
    fn gen_usize(&self, between: Range<usize>) -> usize;
    fn gen_u8(&self, between: Range<u8>) -> u8;
}


#[derive(Clone)]
struct TLRng;

impl AbstractRng for TLRng {
	#[inline]
	fn clone_to_box(&self) -> Box<AbstractRng> {
		Box::new(self.clone())
	}
	
	#[inline]
    fn gen_f32(&self, between: Range<f32>) -> f32 {
        between.ind_sample(&mut thread_rng())
    }
    
	#[inline]
    fn gen_usize(&self, between: Range<usize>) -> usize {
        between.ind_sample(&mut thread_rng())
    }
    
	#[inline]
    fn gen_u8(&self, between: Range<u8>) -> u8 {
        between.ind_sample(&mut thread_rng())
    }
}


#[derive(Clone)]
struct XORRng(XorShiftRng);

impl XORRng {
    #[inline]
    #[allow(mutable_transmutes)]
    fn rng(&self) -> &mut XorShiftRng {
//        let rng: &mut XorShiftRng = unsafe { mem::transmute(&self.0) };
        unsafe { mem::transmute(&self.0) }
    }
    
}

impl AbstractRng for XORRng {
	#[inline]
	fn clone_to_box(&self) -> Box<AbstractRng> {
		Box::new(self.clone())
	}
	
	#[inline]
    fn gen_f32(&self, between: Range<f32>) -> f32 {
        between.ind_sample(self.rng())
    }
    
	#[inline]
    fn gen_usize(&self, between: Range<usize>) -> usize {
        between.ind_sample(self.rng())
    }
    
	#[inline]
    fn gen_u8(&self, between: Range<u8>) -> u8 {
        between.ind_sample(self.rng())
    }
}




pub fn xor_rng() -> Box<AbstractRng> {
	let seed: [u32;4] = [27, 81, 3, 555];
    Box::new(XORRng(XorShiftRng::from_seed(seed)))
}

pub fn tl_rng() -> Box<AbstractRng> {
	Box::new(TLRng)
}

pub fn make_rng() -> Box<AbstractRng> {
//	Box::new(XRng(XorShiftRng::from_seed(seed)))
//	Box::new(TLRng)
	let seed: [u32;4] = [27, 81, 3, 555];
    Box::new(XORRng(XorShiftRng::from_seed(seed)))
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



macro_rules! if_let {
    ($x:pat = $e:expr => $a:block else $b:block) => {{
        let opt_value = if let $x = $e {
            Some($a)
        } else {
            None
        };
        if let Some(x) = opt_value {
            x
        } else {
            $b
        }
    }}
}




#[cfg(test)]
mod placement_tests {
    use super::*;
	use super::Orientation::*;

	fn word(wid: WordId, str: &'static [u8]) -> Word {
		Word::new(wid, str)
	}
	
	fn place(pid: usize, or: Orientation, y: dim, x: dim, word: Word) -> Placement {
		Placement::new(pid, or, y, x, word)
	}
	
    #[test]
    fn incompat_overlap() {
    	let word1 = Word::new(0, b"abc");
    	let word2 = Word::new(1, b"ab");
    	let p1 = Placement::new(0, Orientation::HOR, 0, 0, word1);
    	let p2 = Placement::new(1, Orientation::HOR, 0, 0, word2);
    	
    	assert_eq!(p1.compatible(&p2), false);
    	assert_eq!(p2.compatible(&p1), false);
    }
	
    #[test]
    fn incompat_adjacent() {
    	let word1 = Word::new(0, b"abc");
    	let word2 = Word::new(1, b"ab");
    	let p1 = Placement::new(0, Orientation::HOR, 0, 0, word1);
    	let p2 = Placement::new(1, Orientation::VER, 1, 2, word2);
    	
    	assert_eq!(p1.compatible(&p2), false);
    	assert_eq!(p2.compatible(&p1), false);
    }
	
    #[test]
    fn incompat_intersection() {
    	let word1 = Word::new(0, b"abc");
    	let word2 = Word::new(1, b"bb");
    	let p1 = Placement::new(0, Orientation::HOR, 0, 0, word1);
    	let p2 = Placement::new(1, Orientation::VER, 0, 0, word2);
    	
    	assert_eq!(p1.compatible(&p2), false);
    	assert_eq!(p2.compatible(&p1), false);
    }

    #[test]
    fn compat_intersection() {
//    	let word = |wid, str: &[u8]| Word::new(wid, str.to_vec().into_boxed_slice());
//    	let place = |pid, or, y, x, word| Placement::new(pid, or, y, x, word);
    	
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
	    	let p1 = place(0, HOR, 1, 0, word1);
	    	let p2 = place(1, VER, 0, 0, word2);
	    	
	    	assert_eq!(p1.compatible(&p2), true);
	    	assert_eq!(p2.compatible(&p1), true);
    	}
    	
    	{
	    	let word1 = word(0, b"bc");
	    	let word2 = word(1, b"ab");
	    	let p1 = place(0, HOR, 1, 0, word1);
	    	let p2 = place(1, VER, 0, 0, word2);
	    	
	    	assert_eq!(p1.compatible(&p2), true);
	    	assert_eq!(p2.compatible(&p1), true);
    	}
    }
    
    #[test]
    fn compat_corners() {
    	{  // upper left corner VH
	    	let word1 = word(0, b"bc");
	    	let word2 = word(1, b"de");
	    	let p1 = place(0, HOR, 2, 2, word1);
	    	let p2 = place(1, VER, 0, 1, word2);
	    	
	    	assert_eq!(p1.compatible(&p2), true);
	    	assert_eq!(p2.compatible(&p1), true);
    	}
    	
    	{  // upper left corner HH
	    	let word1 = word(0, b"bc");
	    	let word2 = word(1, b"de");
	    	let p1 = place(0, HOR, 2, 2, word1);
	    	let p2 = place(1, HOR, 1, 0, word2);
	    	
	    	assert_eq!(p1.compatible(&p2), true);
	    	assert_eq!(p2.compatible(&p1), true);
    	}
    	
    	{  // lower left corner VH
	    	let word1 = word(0, b"bc");
	    	let word2 = word(1, b"de");
	    	let p1 = place(0, HOR, 2, 2, word1);
	    	let p2 = place(1, VER, 3, 1, word2);
	    	
	    	assert_eq!(p1.compatible(&p2), true);
	    	assert_eq!(p2.compatible(&p1), true);
    	}
    	
    	{  // lower left corner HH
	    	let word1 = word(0, b"bc");
	    	let word2 = word(1, b"de");
	    	let p1 = place(0, HOR, 2, 2, word1);
	    	let p2 = place(1, HOR, 3, 0, word2);
	    	
	    	assert_eq!(p1.compatible(&p2), true);
	    	assert_eq!(p2.compatible(&p1), true);
    	}
    	
    	{  // upper right corner VH
	    	let word1 = word(0, b"bc");
	    	let word2 = word(1, b"de");
	    	let p1 = place(0, HOR, 2, 2, word1);
	    	let p2 = place(1, VER, 0, 4, word2);
	    	
	    	assert_eq!(p1.compatible(&p2), true);
	    	assert_eq!(p2.compatible(&p1), true);
    	}
    	
    	{  // upper right corner HH
	    	let word1 = word(0, b"bc");
	    	let word2 = word(1, b"de");
	    	let p1 = place(0, HOR, 2, 2, word1);
	    	let p2 = place(1, HOR, 1, 4, word2);
	    	
	    	assert_eq!(p1.compatible(&p2), true);
	    	assert_eq!(p2.compatible(&p1), true);
    	}
    	
    	{  // lower right corner VH
	    	let word1 = word(0, b"bc");
	    	let word2 = word(1, b"de");
	    	let p1 = place(0, HOR, 2, 2, word1);
	    	let p2 = place(1, VER, 3, 4, word2);
	    	
	    	assert_eq!(p1.compatible(&p2), true);
	    	assert_eq!(p2.compatible(&p1), true);
    	}
    	
    	{  // lower right corner HH
	    	let word1 = word(0, b"bc");
	    	let word2 = word(1, b"de");
	    	let p1 = place(0, HOR, 2, 2, word1);
	    	let p2 = place(1, HOR, 3, 4, word2);
	    	
	    	assert_eq!(p1.compatible(&p2), true);
	    	assert_eq!(p2.compatible(&p1), true);
    	}
    	
    	{  // lower right corner VV
	    	let word1 = word(0, b"bc");
	    	let word2 = word(1, b"de");
	    	let p1 = place(0, VER, 2, 2, word1);
	    	let p2 = place(1, VER, 4, 3, word2);
	    	
	    	assert_eq!(p1.compatible(&p2), true);
	    	assert_eq!(p2.compatible(&p1), true);
    	}
    	
    }
}


#[cfg(test)]
mod orientation_tests {
    use super::*;
	use super::Orientation::*;

	#[test]
	fn alignment_tests() {
		assert_eq!(Orientation::HOR.align(1, 0), (1, 0));
		assert_eq!(Orientation::HOR.align(0, 1), (0, 1));
		assert_eq!(Orientation::VER.align(1, 0), (0, 1));
		assert_eq!(Orientation::VER.align(0, 1), (1, 0));
	}
}

