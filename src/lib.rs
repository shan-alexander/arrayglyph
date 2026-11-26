//! # arrayglyph
//!
//! APL-inspired, compile-time-shaped arrays for Rust.
//!
//! * [`Array<T, ROWS, COLS>`](Array) is an owned, `Vec`-backed 2-D array. Shape
//!   mismatches in elementwise ops, `matmul`, and `reshape` are **compile errors**.
//! * [`reduce!`] and [`scan!`] fold along an axis, left to right like Rust.
//!   `reduce!(+, m)` uses the default **axis 0** and sums down each column
//!   (`R × C → 1 × C`, glyph `+⌿m`); `reduce!(+, m, axis = 1)` sums each row
//!   (`R × C → R × 1`, glyph `+/m`). Vectors are `1 × N`, so they use
//!   `axis = 1`.
//! * [`train`] has function values and the explicit composition operator
//!   `∘`: `(+/ ∘ ⍉) m` is `compose(Reduce(Add).axis::<1>(), Transpose).call(&m)`.
//! * [`printer`] translates glyph notation (`+/(a × b)`) to verbose Rust
//!   (`a.zip_with(&b, Mul).reduce_axis::<1>(Add)`) and back.
//!
//! ```
//! use arrayglyph::prelude::*;
//!
//! let a = Array::vector([1, 2, 3]);
//! let b = Array::vector([4, 5, 6]);
//! // +/(a × b)   (dot product; prefix `+/` binds tighter than `×`)
//! let dot = reduce!(+, a.zip_with(&b, Mul), axis = 1).item();
//! assert_eq!(dot, 32);
//!
//! let m = Array::from_rows([[1, 2, 3], [4, 5, 6]]);
//! assert_eq!(reduce!(+, m).as_slice(), &[5, 7, 9]);         // +⌿m  per column
//! assert_eq!(reduce!(+, m, axis = 1).as_slice(), &[6, 15]); // +/m  per row
//! ```
//!
//! See `DESIGN.md` in the repository for the full design and roadmap.

mod array;
mod macros;
pub mod op;
pub mod printer;
pub mod train;
pub mod view;

pub use array::{AlongAxis, Array, Axis, ShapeError, Vector, iota};

/// Convenience re-exports: the array type, `iota`, every op marker, and the
/// macros.
///
/// ```
/// use arrayglyph::prelude::*;
/// assert_eq!(scan!(+, iota::<4>(), axis = 1).as_slice(), &[0, 1, 3, 6]);
/// ```
pub mod prelude {
    pub use crate::op::{Add, Div, Dyadic, Max, Min, Mul, Sub};
    pub use crate::train::{
        Compose, Monadic, Reduce, ReduceAxis, Reshape, Scan, ScanAxis, Shape, Transpose, compose,
    };
    pub use crate::view::Col;
    pub use crate::{AlongAxis, Array, Axis, ShapeError, Vector, iota, reduce, scan};
}

/// Compiles and runs the README's Rust examples as doctests.
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
struct ReadmeDoctests;
