//! `reduce!` and `scan!`: APL's `f⌿`/`f/` and `f⍀`/`f\`, with an optional
//! `axis = 0 | 1` keyword (default 0, down the columns).
//!
//! Both macros take an ASCII spelling of the function, because Rust's lexer
//! rejects APL glyphs such as `×` and `⌈`:
//!
//! | macro token | glyph | marker |
//! |---|---|---|
//! | `+` | `+` | [`Add`](crate::op::Add) |
//! | `-` | `-` | [`Sub`](crate::op::Sub) |
//! | `*` | `×` | [`Mul`](crate::op::Mul) |
//! | `/` | `÷` | [`Div`](crate::op::Div) |
//! | `max` | `⌈` | [`Max`](crate::op::Max) |
//! | `min` | `⌊` | [`Min`](crate::op::Min) |

/// Maps a macro operator token to its `op` marker. This is an internal
/// detail; use [`reduce!`] or [`scan!`].
///
/// ```
/// let _ = arrayglyph::__glyph_op!(max);
/// ```
#[doc(hidden)]
#[macro_export]
macro_rules! __glyph_op {
    (+) => {
        $crate::op::Add
    };
    (-) => {
        $crate::op::Sub
    };
    (*) => {
        $crate::op::Mul
    };
    (/) => {
        $crate::op::Div
    };
    (max) => {
        $crate::op::Max
    };
    (min) => {
        $crate::op::Min
    };
    ($other:tt) => {
        compile_error!(concat!(
            "arrayglyph: unsupported operator `",
            stringify!($other),
            "`; expected one of + - * / max min"
        ))
    };
}

/// `f⌿ x` / `f/ x`: reduces along an axis as a left fold.
///
/// * `reduce!(op, m)` reduces along **axis 0**, the default: down each
///   column, so an `R × C` array becomes `1 × C` (column totals). It expands
///   to [`m.reduce(Op)`](crate::Array::reduce).
/// * `reduce!(op, m, axis = 1)` reduces along each row: `R × C` becomes
///   `R × 1`. It expands to [`m.reduce_axis::<1>(Op)`](crate::Array::reduce_axis).
///   `axis = 0` is accepted too and means the default.
///
/// The axis must be a literal `0` or `1`; anything else is a compile error.
/// Reducing both axes is two calls. The reduced axis is kept with length 1;
/// call `.item()` on a `1 × 1` result to get the scalar.
///
/// **Vectors** are `1 × N`, so their only meaningful axis is 1:
/// `reduce!(+, v, axis = 1)`. Reducing a length-1 axis would return the
/// array unchanged, so `reduce!(+, v)` on a vector is a compile error that
/// tells you to use `axis = 1`, rather than a silent no-op.
///
/// **Semantics:** a left fold, `f/ a b c = (a f b) f c`, as in Rust. For
/// `-` and `/` this differs from APL's right fold.
///
/// ```
/// use arrayglyph::{reduce, Array};
///
/// let m = Array::from_rows([[1, 2, 3], [4, 5, 6]]);
/// assert_eq!(reduce!(+, m).as_slice(), &[5, 7, 9]);           // axis 0: per column
/// assert_eq!(reduce!(+, m, axis = 0).as_slice(), &[5, 7, 9]); // same, explicit
/// assert_eq!(reduce!(+, m, axis = 1).as_slice(), &[6, 15]);   // per row
/// assert_eq!(reduce!(+, reduce!(+, m), axis = 1).item(), 21); // both axes
///
/// let v = Array::vector([1, 2, 3]);
/// assert_eq!(reduce!(-, v, axis = 1).item(), -4); // (1 - 2) - 3
/// assert_eq!(reduce!(max, v, axis = 1).item(), 3);
/// ```
///
/// A vector needs `axis = 1`:
///
/// ```compile_fail,E0080
/// use arrayglyph::{reduce, Array};
/// let _ = reduce!(+, Array::vector([1, 2, 3])); // ERROR: use axis = 1
/// ```
///
/// There is no axis 2:
///
/// ```compile_fail,E0277
/// use arrayglyph::{reduce, Array};
/// let _ = reduce!(+, Array::from_rows([[1, 2], [3, 4]]), axis = 2);
/// ```
///
/// Unsupported operators are compile errors:
///
/// ```compile_fail
/// use arrayglyph::{reduce, Array};
/// let _ = reduce!(%, Array::vector([1, 2]), axis = 1);
/// ```
#[macro_export]
macro_rules! reduce {
    ($op:tt, $array:expr, axis = $axis:literal $(,)?) => {
        ($array).reduce_axis::<$axis>($crate::__glyph_op!($op))
    };
    ($op:tt, $array:expr $(,)?) => {
        ($array).reduce($crate::__glyph_op!($op))
    };
}

/// `f⍀ x` / `f\ x`: scans along an axis (running left fold).
///
/// `scan!(op, m)` scans along **axis 0**, down each column (running column
/// totals), like [`reduce!`]. `scan!(op, m, axis = 1)` scans along each row.
/// The shape doesn't change. As with `reduce!`, vectors need `axis = 1`.
///
/// ```
/// use arrayglyph::{scan, Array};
///
/// let m = Array::from_rows([[1, 2], [3, 4], [5, 6]]);
/// assert_eq!(scan!(+, m).as_slice(), &[1, 2, 4, 6, 9, 12]);          // down columns
/// assert_eq!(scan!(+, m, axis = 1).as_slice(), &[1, 3, 3, 7, 5, 11]); // along rows
///
/// let v = Array::vector([3, 1, 4, 1, 5]);
/// assert_eq!(scan!(+, v, axis = 1).as_slice(), &[3, 4, 8, 9, 14]);
/// assert_eq!(scan!(max, v, axis = 1).as_slice(), &[3, 3, 4, 4, 5]);
/// assert_eq!(scan!(-, Array::vector([1, 2, 3]), axis = 1).as_slice(), &[1, -1, -4]);
/// ```
///
/// ```compile_fail
/// use arrayglyph::{scan, Array};
/// let _ = scan!(^, Array::vector([1, 2]), axis = 1);
/// ```
#[macro_export]
macro_rules! scan {
    ($op:tt, $array:expr, axis = $axis:literal $(,)?) => {
        ($array).scan_axis::<$axis>($crate::__glyph_op!($op))
    };
    ($op:tt, $array:expr $(,)?) => {
        ($array).scan($crate::__glyph_op!($op))
    };
}
