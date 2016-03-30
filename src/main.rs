extern crate ndarray;
extern crate rand;

use std::cmp::max;

use self::board::Board;
use self::common::{dim, Word, Orientation, Placement};
use self::constructor::Constructor;

mod board;
mod common;
mod constructor;

fn main() {
	
	
	let seq = Constructor::new(10, 10).nrpa(3, moves);
	println!("seq = {:?}", seq);
}
