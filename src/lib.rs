#![feature(test)]
#![feature(repr_simd)]
#![feature(specialization)]

extern crate ndarray;
extern crate rand;
extern crate test;
extern crate fnv;

//pub mod board;
pub mod fastmath;

#[macro_use] mod common;
mod fixed_grid;
mod global2;

pub use self::global2::constructor2::Constructor;
pub use self::fixed_grid::FixedGrid;
pub use self::common::{dim, Word, WordId, Orientation, Placement, PlacementId, MatrixDim, LineDim, Problem, Align64};
// TODO: move sliced_arena to the top level
pub use self::global2::sliced_arena;

pub mod util {
	pub use common::{make_rng, tl_rng, xor_rng, AbstractRng};
}
