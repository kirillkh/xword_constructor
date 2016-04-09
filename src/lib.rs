extern crate ndarray;
extern crate rand;

//pub mod board;
pub mod fastmath;

mod constructor;
mod common;
mod board;

pub use self::constructor::Constructor;
pub use self::board::Board;
pub use self::common::{dim, Word, WordId, Orientation, Placement, PlacementId, MatrixDim, LineDim, Problem};

pub mod util {
	pub use common::{make_rng, AbstractRng};
}
