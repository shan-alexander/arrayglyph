//! The crate's macros:
//!
//! * [`reduce!`] and [`scan!`]: APL's `f⌿`/`f/` and `f⍀`/`f\`. The canonical
//!   form writes the axis in brackets after the operator, `reduce!(+[0], m)`,
//!   exactly like APL's `+/[0] m`. The older `reduce!(+, m, axis = 1)` and
//!   default-axis `reduce!(+, m)` forms still work for compatibility.
//! * [`train!`]: a flat composition of function values, `train![f, g, h]`
//!   (the glyph `(f ∘ g ∘ h)`).
//! * [`prof!`]: evaluates an expression, times it, and prints a one-line
//!   profile to stderr.
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
/// **Canonical form: `reduce!(op[axis], x)`.** The axis is written in
/// brackets right after the operator, like APL's `+/[0] m`:
///
/// * `reduce!(+[0], m)` folds down each column (`R × C → 1 × C`). Glyph `+⌿m`.
/// * `reduce!(+[1], m)` folds along each row (`R × C → R × 1`). Glyph `+/m`.
///
/// Both expand to [`m.reduce_axis::<AXIS>(Op)`](crate::Array::reduce_axis).
/// The axis must be a literal `0` or `1`; anything else is a compile error.
/// This is the form the pretty-printer emits in macro style, and it always
/// writes the axis.
///
/// **Compatibility forms** (still accepted, not emitted by the printer):
/// `reduce!(op, m, axis = N)` means `reduce!(op[N], m)`, and
/// `reduce!(op, m)` means `reduce!(op[0], m)` (axis 0, the default).
/// Prefer the bracket form in new code, so the axis is always visible.
///
/// Reducing both axes is two calls. The reduced axis is kept with length 1;
/// call `.item()` on a `1 × 1` result to get the scalar. **Vectors** are
/// `1 × N`, so their only meaningful axis is 1: `reduce!(+[1], v)`.
/// Reducing a length-1 axis would return the array unchanged, so it is a
/// compile error that says which axis to use.
///
/// **Semantics:** a left fold, `f/ a b c = (a f b) f c`, as in Rust. For
/// `-` and `/` this differs from APL's right fold.
///
/// Operator tokens: `+ - * / max min` (`*` is multiply, `/` is divide).
///
/// ```
/// use arrayglyph::{reduce, Array};
///
/// let m = Array::from_rows([[1, 2, 3], [4, 5, 6]]);
/// assert_eq!(reduce!(+[0], m).as_slice(), &[5, 7, 9]);       // +⌿m: per column
/// assert_eq!(reduce!(+[1], m).as_slice(), &[6, 15]);         // +/m: per row
/// assert_eq!(reduce!(+[1], reduce!(+[0], m)).item(), 21);    // both axes
/// assert_eq!(reduce!(max[0], m).as_slice(), &[4, 5, 6]);
///
/// let v = Array::vector([1, 2, 3]);
/// assert_eq!(reduce!(-[1], v).item(), -4); // (1 - 2) - 3
///
/// // Compatibility forms mean the same thing:
/// assert_eq!(reduce!(+, m, axis = 1), reduce!(+[1], m));
/// assert_eq!(reduce!(+, m), reduce!(+[0], m));
/// ```
///
/// A vector needs axis 1:
///
/// ```compile_fail,E0080
/// use arrayglyph::{reduce, Array};
/// let _ = reduce!(+[0], Array::vector([1, 2, 3])); // ERROR: use +[1]
/// ```
///
/// There is no axis 2:
///
/// ```compile_fail,E0277
/// use arrayglyph::{reduce, Array};
/// let _ = reduce!(+[2], Array::from_rows([[1, 2], [3, 4]]));
/// ```
///
/// Unsupported operators are compile errors:
///
/// ```compile_fail
/// use arrayglyph::{reduce, Array};
/// let _ = reduce!(%[1], Array::vector([1, 2]));
/// ```
#[macro_export]
macro_rules! reduce {
    ($op:tt [$axis:literal], $array:expr $(,)?) => {
        ($array).reduce_axis::<$axis>($crate::__glyph_op!($op))
    };
    ($op:tt, $array:expr, axis = $axis:literal $(,)?) => {
        ($array).reduce_axis::<$axis>($crate::__glyph_op!($op))
    };
    ($op:tt, $array:expr $(,)?) => {
        ($array).reduce($crate::__glyph_op!($op))
    };
}

/// `f⍀ x` / `f\ x`: scans along an axis (running left fold).
///
/// **Canonical form: `scan!(op[axis], x)`**, as for [`reduce!`](crate::reduce!):
/// `scan!(+[0], m)` gives running totals down each column (glyph `+⍀m`),
/// `scan!(*[1], m)` running products along each row (glyph `×\m`). The
/// shape doesn't change. The compatibility forms `scan!(op, m, axis = N)`
/// and `scan!(op, m)` (axis 0) still work. Vectors need axis 1.
///
/// ```
/// use arrayglyph::{scan, Array};
///
/// let m = Array::from_rows([[1, 2], [3, 4], [5, 6]]);
/// assert_eq!(scan!(+[0], m).as_slice(), &[1, 2, 4, 6, 9, 12]);  // down columns
/// assert_eq!(scan!(*[1], m).as_slice(), &[1, 2, 3, 12, 5, 30]); // along rows
///
/// let v = Array::vector([3, 1, 4, 1, 5]);
/// assert_eq!(scan!(max[1], v).as_slice(), &[3, 3, 4, 4, 5]);
/// assert_eq!(scan!(-[1], Array::vector([1, 2, 3])).as_slice(), &[1, -1, -4]);
///
/// // Compatibility forms:
/// assert_eq!(scan!(+, m, axis = 1), scan!(+[1], m));
/// assert_eq!(scan!(+, m), scan!(+[0], m));
/// ```
///
/// ```compile_fail
/// use arrayglyph::{scan, Array};
/// let _ = scan!(^[1], Array::vector([1, 2]));
/// ```
#[macro_export]
macro_rules! scan {
    ($op:tt [$axis:literal], $array:expr $(,)?) => {
        ($array).scan_axis::<$axis>($crate::__glyph_op!($op))
    };
    ($op:tt, $array:expr, axis = $axis:literal $(,)?) => {
        ($array).scan_axis::<$axis>($crate::__glyph_op!($op))
    };
    ($op:tt, $array:expr $(,)?) => {
        ($array).scan($crate::__glyph_op!($op))
    };
}

/// `(f ∘ g ∘ h)`: a flat **train** (composition) of function values.
///
/// `train![f, g, h]` lists the functions **in the same order as the glyph
/// `(f ∘ g ∘ h)`**, so the **rightmost runs first**, as in mathematical
/// composition: `train![f, g, h].call(&x)` is `f(g(h(x)))`. Applying a
/// train is `.call(&x)` (borrowing) or `.call(x)` (by value), via
/// [`Monadic`](crate::train::Monadic).
///
/// Items:
///
/// | item | glyph | value |
/// |---|---|---|
/// | `reduce(op[k])` | `f⌿` (k = 0) / `f/` (k = 1) | [`Reduce`](crate::train::Reduce)`(Op).axis::<k>()` |
/// | `scan(op[k])` | `f⍀` / `f\` | [`Scan`](crate::train::Scan)`(Op).axis::<k>()` |
/// | `transpose` | `⍉` | [`Transpose`](crate::train::Transpose) |
/// | `shape` | `⍴` | [`Shape`](crate::train::Shape) |
/// | `reshape(r, c)` | `r c⍴` | [`Reshape`](crate::train::Reshape)`::<r, c>` |
/// | `(expr)` | — | any function value, e.g. a named train |
///
/// Operators inside `reduce(…)` / `scan(…)` use the same tokens as
/// [`reduce!`](crate::reduce!): `+ - * / max min`. The axis is required in canonical
/// form; `reduce(+)` (axis 0) is accepted for symmetry with `reduce!(+, m)`.
///
/// The expansion is left-nested [`compose`](crate::train::compose) calls,
/// `compose(compose(f, g), h)`; composition is associative, so grouping
/// never changes the result. Shapes are still checked at compile time
/// through the whole train.
///
/// ```
/// use arrayglyph::prelude::*;
///
/// let m = Array::from_rows([[1, 2, 3], [4, 5, 6]]);
/// // glyph: (+/ ∘ ⍉) m   -- transpose first, then sum each row
/// let col_sums = train![reduce(+[1]), transpose];
/// assert_eq!(col_sums.call(&m).as_slice(), &[5, 7, 9]);
///
/// // glyph: (+/ ∘ ⍉ ∘ 2 3⍴) ⍳6
/// let three = train![reduce(+[1]), transpose, reshape(2, 3)];
/// assert_eq!(three.call(&iota::<6>()).as_slice(), &[3, 5, 7]);
///
/// // A named train can be an item, in parentheses.
/// let grand = train![reduce(+[0]), (col_sums)]; // 3×1 → 1×1
/// assert_eq!(grand.call(&m).as_slice(), &[21]);
///
/// // A one-item train is just that function value.
/// assert_eq!(train![scan(max[1])].call(&m), m.cummax::<1>());
/// ```
///
/// Shape errors inside a train are still compile errors:
///
/// ```compile_fail,E0080
/// use arrayglyph::prelude::*;
/// let _ = train![transpose, reshape(4, 2)].call(iota::<6>()); // 6 != 8
/// ```
#[macro_export]
macro_rules! train {
    ($($items:tt)+) => {
        $crate::__train!(@go [] $($items)+)
    };
}

/// Implementation detail of [`train!`]: munches items into expressions,
/// then folds them left into `compose` calls.
///
/// ```
/// let _ = arrayglyph::__train!(@go [] transpose);
/// ```
#[doc(hidden)]
#[macro_export]
macro_rules! __train {
    (@go [$($acc:expr),*]) => {
        $crate::__train!(@fold $($acc),*)
    };
    (@go [$($acc:expr),*] , $($rest:tt)*) => {
        $crate::__train!(@go [$($acc),*] $($rest)*)
    };
    (@go [$($acc:expr),*] transpose $($rest:tt)*) => {
        $crate::__train!(@go [$($acc,)* $crate::train::Transpose] $($rest)*)
    };
    (@go [$($acc:expr),*] shape $($rest:tt)*) => {
        $crate::__train!(@go [$($acc,)* $crate::train::Shape] $($rest)*)
    };
    (@go [$($acc:expr),*] reshape($r:literal, $c:literal) $($rest:tt)*) => {
        $crate::__train!(@go [$($acc,)* $crate::train::Reshape::<$r, $c>] $($rest)*)
    };
    (@go [$($acc:expr),*] reduce($op:tt [$k:literal]) $($rest:tt)*) => {
        $crate::__train!(@go
            [$($acc,)* $crate::train::Reduce($crate::__glyph_op!($op)).axis::<$k>()] $($rest)*)
    };
    (@go [$($acc:expr),*] reduce($op:tt) $($rest:tt)*) => {
        $crate::__train!(@go [$($acc,)* $crate::train::Reduce($crate::__glyph_op!($op))] $($rest)*)
    };
    (@go [$($acc:expr),*] scan($op:tt [$k:literal]) $($rest:tt)*) => {
        $crate::__train!(@go
            [$($acc,)* $crate::train::Scan($crate::__glyph_op!($op)).axis::<$k>()] $($rest)*)
    };
    (@go [$($acc:expr),*] scan($op:tt) $($rest:tt)*) => {
        $crate::__train!(@go [$($acc,)* $crate::train::Scan($crate::__glyph_op!($op))] $($rest)*)
    };
    (@go [$($acc:expr),*] ($e:expr) $($rest:tt)*) => {
        $crate::__train!(@go [$($acc,)* $e] $($rest)*)
    };
    (@go [$($acc:expr),*] $bad:tt $($rest:tt)*) => {
        compile_error!(concat!(
            "train!: unknown item `",
            stringify!($bad),
            "`; expected reduce(op[k]), scan(op[k]), transpose, shape, reshape(r, c), or (expr)"
        ))
    };
    (@fold $f:expr) => {
        $f
    };
    (@fold $f:expr, $g:expr $(, $rest:expr)*) => {
        $crate::__train!(@fold $crate::train::compose($f, $g) $(, $rest)*)
    };
}

/// Profiles one expression: evaluates it once, times it with
/// [`std::time::Instant`], prints a one-line report to **stderr**, and
/// returns the value unchanged.
///
/// * `prof!(expr)` reports the source location, the expression text, the
///   result's shape, and the elapsed time.
/// * `prof!(glyph = "+⌿m", expr)` also prints the glyph the expression
///   stands for, so a profile line ties the dense notation to the Rust that
///   ran. The glyph is only printed; it is not parsed or checked (use
///   [`printer::glyph_to_rust`](crate::printer::glyph_to_rust) for that).
///
/// The result must implement [`Profile`](crate::prof::Profile), which
/// describes its shape: every [`Array`](crate::Array) (`R×C`), the `(rows,
/// cols)` tuple from `shape()`, and the primitive numbers (`scalar`).
///
/// Output looks like this (the timing varies):
///
/// ```text
/// [prof] src/main.rs:7  +⌿m  ⇐  m.sum::<0>()  → 1×3  in 1.2µs
/// ```
///
/// ```
/// use arrayglyph::prelude::*;
///
/// let m = Array::from_rows([[1, 2, 3], [4, 5, 6]]);
/// let cols = prof!(glyph = "+⌿m", m.sum::<0>());
/// assert_eq!(cols.as_slice(), &[5, 7, 9]); // the value passes through
///
/// let total = prof!(cols.sum::<1>().item());
/// assert_eq!(total, 21);
/// ```
///
/// Types without a [`Profile`](crate::prof::Profile) impl are rejected at
/// compile time:
///
/// ```compile_fail,E0277
/// use arrayglyph::prof;
/// let _ = prof!(String::from("not an array"));
/// ```
#[macro_export]
macro_rules! prof {
    (glyph = $glyph:expr, $e:expr $(,)?) => {
        $crate::prof::__run(file!(), line!(), Some($glyph), stringify!($e), || $e)
    };
    ($e:expr $(,)?) => {
        $crate::prof::__run(file!(), line!(), None, stringify!($e), || $e)
    };
}
