use std::ops::{Index, IndexMut};

pub type SliceIndex = usize;

#[derive(Clone)]
struct Slice {
    from: usize,
    to: usize
}

#[derive(Clone)]
pub struct SlicedArena<T: Sized+Clone> {
    mem: Vec<T>,
    slices: Vec<Slice>
}

impl<T: Sized+Clone> SlicedArena<T> {
    pub fn new(sizes: &Vec<usize>) -> SlicedArena<T> where T: Default {
        let mut slices = Vec::with_capacity(sizes.len());
        let sum = sizes.iter().fold(0, |from, s| {
                let to = from + s;
                slices.push(Slice { from:from, to: to});
                to
        });
        
        let sum = sizes.iter().fold(0, |acc, s| acc+s);
        let mem = vec![Default::default(); sum];
        
        SlicedArena { mem:mem, slices:slices }
    }
    
    pub fn slice<'a>(&'a self, idx: SliceIndex) -> &'a [T] {
        let slice = &self.slices[idx];
        &self.mem[slice.from .. slice.to]
    }
    
    pub fn slice_mut<'a>(&'a mut self, idx: SliceIndex) -> &'a mut [T] {
        let slice = &self.slices[idx];
        &mut self.mem[slice.from .. slice.to]
    }
}

impl<T: Sized+Clone> Index<usize> for SlicedArena<T> {
    type Output = [T];

	#[inline]
    fn index(&self, index: usize) -> &Self::Output {
		self.slice(index)
    }
}

impl<T: Sized+Clone> IndexMut<usize> for SlicedArena<T> {
	#[inline]
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
		self.slice_mut(index)
    }
}

