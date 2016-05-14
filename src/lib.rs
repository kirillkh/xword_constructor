#![feature(test)]

extern crate ndarray;
extern crate rand;
extern crate test;
extern crate fnv;

//pub mod board;
pub mod fastmath;

#[macro_use] mod common;
mod constructor;
mod board;
mod global2;

pub use self::constructor::Constructor;
pub use self::board::Board;
pub use self::common::{dim, Word, WordId, Orientation, Placement, PlacementId, MatrixDim, LineDim, Problem};

pub mod util {
	pub use common::{make_rng, AbstractRng};
}
