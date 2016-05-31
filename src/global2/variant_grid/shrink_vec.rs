use std::ops::{Deref, DerefMut};
use std::{ptr, slice};


pub struct ShrinkVec<'a, T: 'a+Clone> {
    slice: &'a mut [T]
}

impl<'a, T: 'a+Clone> ShrinkVec<'a, T> {
    pub fn new(slice: &'a mut [T]) -> ShrinkVec<'a, T> {
        ShrinkVec { slice:slice }
    }
    
    pub unsafe fn swap_remove_unchecked(&mut self, idx: usize) -> T {
        if self.len() == 0 {
            panic!("cannot swap_remove() when len==0");
        }
        let new_len = self.len() - 1;
        
        
        let ptr = self.slice.as_mut_ptr();
        let replacement = ptr::read(ptr.offset(new_len as isize));
        let old = ptr::replace(ptr.offset(idx as isize), replacement);
//            self.slice = mem::transmute(&mut self.slice[..new_len]);
        
        let ptr = self.slice.as_mut_ptr();
        self.slice = slice::from_raw_parts_mut(ptr, new_len);
        old
    }
}


impl<'a, T: 'a+Clone>  Clone for ShrinkVec<'a, T> {
    fn clone(&self) -> ShrinkVec<'a, T> {
        unsafe {
            let ptr = &self.slice as *const &mut [T];
            let slice: &mut [T] = ptr::read(ptr);
            
            ShrinkVec::new(slice)
        }
    }
}

impl<'a, T: 'a+Clone> Deref for ShrinkVec<'a, T> {
    type Target = [T];
    
    fn deref(&self) -> &[T] {
        self.slice
    }
}

impl<'a, T: 'a+Clone> DerefMut for ShrinkVec<'a, T> {
    fn deref_mut(&mut self) -> &mut [T] {
        self.slice
    }
}


