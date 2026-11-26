//! Function values and explicit composition: the Rust side of glyph `∘`.
//!
//! Glyph text evaluates left to right, so composition isn't implied by
//! juxtaposition. It is written explicitly: `(+/ ∘ ⍉) m` applies `⍉`, then
//! `+/`. This is mathematical composition, `(f ∘ g)(x) = f(g(x))`, as in APL
//! and Haskell. It desugars to [`compose`] over small function-value types:
//!
//! | glyph | Rust value |
//! |---|---|
//! | `f⌿` (axis 0) | [`Reduce`]`(F)` |
//! | `f/` (axis 1) | [`Reduce`]`(F).axis::<1>()` |
//! | `f⍀` (axis 0) | [`Scan`]`(F)` |
//! | `f\` (axis 1) | [`Scan`]`(F).axis::<1>()` |
//! | `⍉` | [`Transpose`] |
//! | `⍴` | [`Shape`] |
//! | `r c⍴` | [`Reshape`]`::<r, c>` |
//! | `f ∘ g` | [`compose`]`(f, g)` |
//!
//! Apply a function value with [`Monadic::call`]. Every value accepts both
//! `&Array` (borrowing, which is what the printer emits) and `Array` (by
//! value).
//!
//! ```
//! use arrayglyph::prelude::*;
//!
//! let m = Array::from_rows([[1, 2, 3], [4, 5, 6]]);
//! // glyph: (+/ ∘ ⍉) m     row sums of the transpose = column sums, as a column
//! let col_sums = compose(Reduce(Add).axis::<1>(), Transpose);
//! assert_eq!(col_sums.call(&m).as_slice(), &[5, 7, 9]);
//! assert_eq!(col_sums.call(&m).shape(), (3, 1));
//! // The composed value is reusable, and equals the prefix form +/⍉m.
//! assert_eq!(col_sums.call(&m), m.transpose().reduce_axis::<1>(Add));
//! ```
//!
//! ## Flat trains: `train![…]`
//!
//! Nested `compose(compose(f, g), h)` gets hard to read quickly, so the
//! canonical spelling (and what the printer emits) is the flat
//! [`train!`](crate::train!) macro. Items are listed **in glyph order**, so
//! the rightmost runs first, exactly like `∘`:
//!
//! ```
//! use arrayglyph::prelude::*;
//! // glyph: (+/ ∘ ⍉ ∘ 2 3⍴) ⍳6
//! let f = train![reduce(+[1]), transpose, reshape(2, 3)];
//! assert_eq!(f.call(&iota::<6>()).as_slice(), &[3, 5, 7]);
//! // the same value, spelled with the lower-level API
//! let g = compose(compose(Reduce(Add).axis::<1>(), Transpose), Reshape::<2, 3>);
//! assert_eq!(f.call(&iota::<6>()), g.call(&iota::<6>()));
//! ```
//!
//! [`compose`] and the function-value types stay public as the lower-level
//! API; `train!` expands to them.

use crate::op::Dyadic;
use crate::{AlongAxis, Array, Axis};

/// A one-argument function value that can be applied to an array.
///
/// It is implemented for `X = &Array<…>` and `X = Array<…>` by every function
/// value in this module.
///
/// ```
/// use arrayglyph::prelude::*;
/// let v = Array::vector([3, 1, 2]);
/// assert_eq!(Reduce(Max).axis::<1>().call(&v).item(), 3);
/// ```
pub trait Monadic<X> {
    /// The result type.
    type Output;
    /// Applies the function to `x`.
    fn call(&self, x: X) -> Self::Output;
}

/// `f⌿` as a value: reduce along axis 0 (down each column) with `F`, a left
/// fold. See [`Array::reduce`]. For axis 1 (`f/`), use
/// [`axis`](Reduce::axis)`::<1>()`.
///
/// ```
/// use arrayglyph::prelude::*;
/// let m = Array::from_rows([[1, 2], [3, 4]]);
/// assert_eq!(Reduce(Add).call(&m).as_slice(), &[4, 6]);
/// assert_eq!(Reduce(Sub).axis::<1>().call(&Array::vector([1, 2, 3])).item(), -4);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Reduce<F>(pub F);

impl<F> Reduce<F> {
    /// Picks the axis to reduce along: `0` (the default, `f⌿`) or `1`
    /// (`f/`). Mirrors `reduce!(op[AXIS], x)`; inside [`train!`](crate::train!)
    /// it is written `reduce(op[AXIS])`.
    ///
    /// ```
    /// use arrayglyph::prelude::*;
    /// let m = Array::from_rows([[1, 2], [3, 4]]);
    /// assert_eq!(Reduce(Add).axis::<1>().call(&m).as_slice(), &[3, 7]);
    /// ```
    pub fn axis<const AXIS: usize>(self) -> ReduceAxis<F, AXIS> {
        ReduceAxis(self.0)
    }
}

/// `f/` or `f⌿` as a value with an explicit axis. Build it with
/// [`Reduce::axis`]. See [`Array::reduce_axis`].
///
/// ```
/// use arrayglyph::prelude::*;
/// let rows = Reduce(Max).axis::<1>();
/// assert_eq!(rows.call(&Array::from_rows([[3, 9], [7, 1]])).as_slice(), &[9, 7]);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct ReduceAxis<F, const AXIS: usize>(pub F);

/// `f⍀` as a value: scan along axis 0 (down each column) with `F`. See
/// [`Array::scan`]. For axis 1 (`f\`), use [`axis`](Scan::axis)`::<1>()`.
///
/// ```
/// use arrayglyph::prelude::*;
/// assert_eq!(Scan(Add).axis::<1>().call(&Array::vector([1, 2, 3])).as_slice(), &[1, 3, 6]);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Scan<F>(pub F);

impl<F> Scan<F> {
    /// Picks the axis to scan along: `0` (the default, `f⍀`) or `1` (`f\`).
    ///
    /// ```
    /// use arrayglyph::prelude::*;
    /// let m = Array::from_rows([[1, 2], [3, 4]]);
    /// assert_eq!(Scan(Add).axis::<0>().call(&m), Scan(Add).call(&m));
    /// ```
    pub fn axis<const AXIS: usize>(self) -> ScanAxis<F, AXIS> {
        ScanAxis(self.0)
    }
}

/// `f\` or `f⍀` as a value with an explicit axis. Build it with
/// [`Scan::axis`]. See [`Array::scan_axis`].
///
/// ```
/// use arrayglyph::prelude::*;
/// let running = Scan(Add).axis::<1>();
/// assert_eq!(running.call(&iota::<4>()).as_slice(), &[0, 1, 3, 6]);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct ScanAxis<F, const AXIS: usize>(pub F);

/// `⍉` as a value. See [`Array::transpose`].
///
/// ```
/// use arrayglyph::prelude::*;
/// let m = Array::from_rows([[1, 2]]);
/// assert_eq!(Transpose.call(&m).shape(), (2, 1));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Transpose;

/// `⍴` as a value. See [`Array::shape`].
///
/// ```
/// use arrayglyph::prelude::*;
/// assert_eq!(Shape.call(&iota::<4>()), (1, 4));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Shape;

/// `r c⍴` as a value. See [`Array::reshape`]. The element count is checked
/// at compile time.
///
/// ```
/// use arrayglyph::prelude::*;
/// let m = Reshape::<2, 3>.call(iota::<6>());
/// assert_eq!(m.row(1), &[3, 4, 5]);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Reshape<const R: usize, const C: usize>;

/// `f ∘ g` as a value: applies `g`, then `f`. Build one with [`compose`].
///
/// ```
/// use arrayglyph::prelude::*;
/// let c = compose(Scan(Add), Transpose);
/// assert_eq!(c.f, Scan(Add));
/// assert_eq!(c.g, Transpose);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Compose<F, G> {
    /// Applied second (the left operand of `∘`).
    pub f: F,
    /// Applied first (the right operand of `∘`).
    pub g: G,
}

/// `f ∘ g`: composes two function values. `compose(f, g).call(x)` is
/// `f.call(g.call(x))`.
///
/// Compositions nest. `f ∘ g ∘ h` groups to the left as
/// `compose(compose(f, g), h)`, and since composition is associative,
/// grouping never changes the result.
///
/// ```
/// use arrayglyph::prelude::*;
/// let m = iota::<6>();
/// // glyph: (+/ ∘ ⍉ ∘ 2 3⍴) ⍳6
/// let f = compose(compose(Reduce(Add).axis::<1>(), Transpose), Reshape::<2, 3>);
/// assert_eq!(f.call(&m).as_slice(), &[3, 5, 7]);
/// ```
///
/// Shape errors inside a composition are still compile errors:
///
/// ```compile_fail,E0308
/// use arrayglyph::prelude::*;
/// let m: Array<i32, 2, 3> = Array::splat(1);
/// let col_sums = compose(Reduce(Add).axis::<1>(), Transpose).call(&m); // 3×1
/// let _ = col_sums + m; // ERROR: 3×1 and 2×3 differ
/// ```
///
/// ```compile_fail,E0080
/// use arrayglyph::prelude::*;
/// // Reshape inside a composition still checks the element count: 6 != 8.
/// let _ = compose(Transpose, Reshape::<4, 2>).call(iota::<6>());
/// ```
pub fn compose<F, G>(f: F, g: G) -> Compose<F, G> {
    Compose { f, g }
}

impl<X, F, G> Monadic<X> for Compose<F, G>
where
    G: Monadic<X>,
    F: Monadic<G::Output>,
{
    type Output = F::Output;
    fn call(&self, x: X) -> Self::Output {
        self.f.call(self.g.call(x))
    }
}

/// Implements `Monadic` for both `&Array` and `Array` by delegating the owned
/// case to the borrowed one (or the reverse, for consuming ops).
macro_rules! by_ref_and_value {
    ($ty:ty, [$($gen:tt)*], where [$($bounds:tt)*], $out:ty, |$this:ident, $x:ident| $body:expr) => {
        impl<'a, T, $($gen)* const R: usize, const C: usize> Monadic<&'a Array<T, R, C>> for $ty
        where
            $($bounds)*
        {
            type Output = $out;
            fn call(&self, $x: &'a Array<T, R, C>) -> $out {
                let $this = self;
                $body
            }
        }

        impl<T, $($gen)* const R: usize, const C: usize> Monadic<Array<T, R, C>> for $ty
        where
            $($bounds)*
        {
            type Output = $out;
            fn call(&self, x: Array<T, R, C>) -> $out {
                self.call(&x)
            }
        }
    };
}

by_ref_and_value!(Reduce<F>, [F,], where [T: Clone, F: Dyadic<T> + Copy], Array<T, 1, C>,
    |this, x| x.reduce(this.0));
by_ref_and_value!(ReduceAxis<F, A>, [F, const A: usize,],
    where [T: Clone, F: Dyadic<T> + Copy, Axis<A>: AlongAxis<T, R, C>],
    <Axis<A> as AlongAxis<T, R, C>>::Reduced,
    |this, x| x.reduce_axis::<A>(this.0));
by_ref_and_value!(Scan<F>, [F,], where [T: Clone, F: Dyadic<T> + Copy], Array<T, R, C>,
    |this, x| x.scan(this.0));
by_ref_and_value!(ScanAxis<F, A>, [F, const A: usize,],
    where [T: Clone, F: Dyadic<T> + Copy, Axis<A>: AlongAxis<T, R, C>],
    Array<T, R, C>,
    |this, x| x.scan_axis::<A>(this.0));
by_ref_and_value!(Transpose, [], where [T: Clone], Array<T, C, R>, |_this, x| x.transpose());
by_ref_and_value!(Shape, [], where [], (usize, usize), |_this, x| x.shape());

impl<'a, T: Clone, const R: usize, const C: usize, const R2: usize, const C2: usize>
    Monadic<&'a Array<T, R, C>> for Reshape<R2, C2>
{
    type Output = Array<T, R2, C2>;
    fn call(&self, x: &'a Array<T, R, C>) -> Self::Output {
        x.clone().reshape()
    }
}

impl<T, const R: usize, const C: usize, const R2: usize, const C2: usize> Monadic<Array<T, R, C>>
    for Reshape<R2, C2>
{
    type Output = Array<T, R2, C2>;
    fn call(&self, x: Array<T, R, C>) -> Self::Output {
        x.reshape()
    }
}
