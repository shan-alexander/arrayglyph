//! # arrayglyph
//!
//! APL-inspired, compile-time-shaped arrays for Rust.
//!
//! * [`Array<T, ROWS, COLS>`](Array) is an owned, `Vec`-backed 2-D array. Shape
//!   mismatches in elementwise ops, `matmul`, and `reshape` are **compile errors**.
//! * Axis reductions and scans fold left to right, like Rust, and **always
//!   name their axis**. Axis 0 runs down each column (`R × C → 1 × C`, glyph
//!   `+⌿m`), axis 1 along each row (`R × C → R × 1`, glyph `+/m`). Write
//!   them as methods, `m.sum::<0>()` / `m.cumprod::<1>()`, or as macros with
//!   a bracketed axis, `reduce!(+[0], m)` / `scan!(*[1], m)` (APL's
//!   `+/[0] m`). Vectors are `1 × N`, so they use axis 1.
//! * [`mod@train`] has function values and composition `∘`. [`train!`] writes a
//!   flat train in glyph order: `(+/ ∘ ⍉) m` is
//!   `train![reduce(+[1]), transpose].call(&m)`.
//! * [`prof!`] times an expression and prints its shape to stderr.
//! * [`printer`] translates glyph notation (`+/(a × b)`) to verbose Rust
//!   (`a.zip_with(&b, Mul).sum::<1>()`, or `reduce!(+[1], …)` in macro
//!   style) and back. Glyphs are the primary notation.
//!
//! ```
//! use arrayglyph::prelude::*;
//!
//! let a = Array::vector([1, 2, 3]);
//! let b = Array::vector([4, 5, 6]);
//! // +/(a × b)   (dot product; prefix `+/` binds tighter than `×`)
//! let dot = a.zip_with(&b, Mul).sum::<1>().item();
//! assert_eq!(dot, 32);
//! assert_eq!(reduce!(+[1], a.zip_with(&b, Mul)).item(), 32); // same, macro style
//!
//! let m = Array::from_rows([[1, 2, 3], [4, 5, 6]]);
//! assert_eq!(m.sum::<0>().as_slice(), &[5, 7, 9]);       // +⌿m  per column
//! assert_eq!(reduce!(+[1], m).as_slice(), &[6, 15]);     // +/m  per row
//! assert_eq!(m.cumsum::<1>().as_slice(), &[1, 3, 6, 4, 9, 15]); // +\m
//! ```
//!
//! See `DESIGN.md` in the repository for the full design and roadmap.

mod array;
mod macros;
pub mod op;
pub mod printer;
pub mod prof;
pub mod table;
pub mod train;
pub mod view;

pub use array::{AlongAxis, Array, Axis, ShapeError, Vector, iota};

/// Convenience re-exports: the array type, `iota`, every op marker, and the
/// macros.
///
/// ```
/// use arrayglyph::prelude::*;
/// assert_eq!(scan!(+[1], iota::<4>()).as_slice(), &[0, 1, 3, 6]);
/// assert_eq!(train![reduce(+[1])].call(iota::<4>()).item(), 6);
/// ```
pub mod prelude {
    pub use crate::op::{Add, Div, Dyadic, Max, Min, Mul, Sub};
    pub use crate::train::{
        Compose, Monadic, Reduce, ReduceAxis, Reshape, Scan, ScanAxis, Shape, Transpose, compose,
    };
    pub use crate::view::Col;
    pub use crate::{AlongAxis, Array, Axis, ShapeError, Vector, iota, prof, reduce, scan, train};
}

/// Compiles and runs the README's Rust examples as doctests.
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
struct ReadmeDoctests;
