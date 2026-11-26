//! `reduce!` and `scan!`: APL's `f/` and `f\`.
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

/// `f/ x`: reduces along the **last axis** as a left fold.
///
/// `reduce!(op, array)` expands to `array.reduce(Op)` (see
/// [`Array::reduce`](crate::Array::reduce)), which is a
/// left fold ([`Iterator::reduce`]) over each row. An `R × C` array
/// becomes `R × 1`. For vectors, call `.item()` to get the scalar.
///
/// **Semantics:** `f/ a b c = (a f b) f c`, left to right as in Rust. For
/// `-` and `/` this differs from APL's right fold:
///
/// ```
/// use arrayglyph::{reduce, Array};
///
/// let v = Array::vector([1, 2, 3]);
/// assert_eq!(reduce!(+, v).item(), 6);
/// assert_eq!(reduce!(-, v).item(), -4);    // (1 - 2) - 3
/// assert_eq!(reduce!(max, v).item(), 3);
///
/// let m = Array::from_rows([[1, 2], [3, 4]]);
/// assert_eq!(reduce!(*, m).as_slice(), &[2, 12]); // one result per row
/// ```
///
/// Unsupported operators are compile errors:
///
/// ```compile_fail
/// use arrayglyph::{reduce, Array};
/// let _ = reduce!(%, Array::vector([1, 2]));
/// ```
#[macro_export]
macro_rules! reduce {
    ($op:tt, $array:expr $(,)?) => {
        ($array).reduce($crate::__glyph_op!($op))
    };
}

/// `f\ x`: scans along the **last axis**.
///
/// `scan!(op, array)` expands to `array.scan(Op)` (see
/// [`Array::scan`](crate::Array::scan)). Element `i` of each row is `f/` of the
/// first `i + 1` elements, which is the running left fold. The shape
/// doesn't change.
///
/// ```
/// use arrayglyph::{scan, Array};
///
/// let v = Array::vector([3, 1, 4, 1, 5]);
/// assert_eq!(scan!(+, v).as_slice(), &[3, 4, 8, 9, 14]);
/// assert_eq!(scan!(max, v).as_slice(), &[3, 3, 4, 4, 5]);
/// assert_eq!(scan!(-, Array::vector([1, 2, 3])).as_slice(), &[1, -1, -4]);
/// ```
///
/// ```compile_fail
/// use arrayglyph::{scan, Array};
/// let _ = scan!(^, Array::vector([1, 2]));
/// ```
#[macro_export]
macro_rules! scan {
    ($op:tt, $array:expr $(,)?) => {
        ($array).scan($crate::__glyph_op!($op))
    };
}
