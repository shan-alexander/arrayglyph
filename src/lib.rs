//! # arrayglyph
//!
//! APL-inspired, compile-time-shaped arrays for Rust.
//!
//! * [`Array<T, ROWS, COLS>`](Array) is an owned, `Vec`-backed 2-D array. Shape
//!   mismatches in elementwise ops, `matmul`, and `reshape` are **compile errors**.
//! * [`reduce!`] and [`scan!`] are `f/` and `f\` along the last axis, as
//!   left folds (left to right, like Rust).
//! * [`train`] has function values and the explicit composition operator
//!   `∘`: `(+/ ∘ ⍉) m` is `compose(Reduce(Add), Transpose).call(&m)`.
//! * [`printer`] translates glyph notation (`+/a × b`) to verbose Rust
//!   (`a.zip_with(&b, Mul).reduce(Add)`) and back.
//!
//! ```
//! use arrayglyph::prelude::*;
//!
//! let a = Array::vector([1, 2, 3]);
//! let b = Array::vector([4, 5, 6]);
//! // +/(a × b)   (dot product; prefix `+/` binds tighter than `×`)
//! let dot = reduce!(+, a.zip_with(&b, Mul)).item();
//! assert_eq!(dot, 32);
//! ```
//!
//! See `DESIGN.md` in the repository for the full design and roadmap.

mod array;
mod macros;
pub mod op;
pub mod printer;
pub mod train;
pub mod view;

pub use array::{Array, ShapeError, Vector, iota};

/// Convenience re-exports: the array type, `iota`, every op marker, and the
/// macros.
///
/// ```
/// use arrayglyph::prelude::*;
/// assert_eq!(scan!(+, iota::<4>()).as_slice(), &[0, 1, 3, 6]);
/// ```
pub mod prelude {
    pub use crate::op::{Add, Div, Dyadic, Max, Min, Mul, Sub};
    pub use crate::train::{Compose, Monadic, Reduce, Reshape, Scan, Shape, Transpose, compose};
    pub use crate::view::Col;
    pub use crate::{Array, ShapeError, Vector, iota, reduce, scan};
}

/// Compiles and runs the README's Rust examples as doctests.
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
struct ReadmeDoctests;
