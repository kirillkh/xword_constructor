use xword::sliced_arena::SlicedArena;
use xword::Word;
use std::mem;

pub fn dic_arena(dic: Vec<Vec<u8>>) -> (Vec<Word>, SlicedArena<u8>) {
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
