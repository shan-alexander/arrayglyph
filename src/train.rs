//! Function values and explicit composition: the Rust side of glyph `∘`.
//!
//! Glyph text evaluates left to right, so composition isn't implied by
//! juxtaposition. It is written explicitly: `(+/ ∘ ⍉) m` applies `⍉`, then
//! `+/`. This is mathematical composition, `(f ∘ g)(x) = f(g(x))`, as in APL
//! and Haskell. It desugars to [`compose`] over small function-value types:
//!
//! | glyph | Rust value |
//! |---|---|
//! | `f/` | [`Reduce`]`(F)` |
//! | `f\` | [`Scan`]`(F)` |
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
//! // glyph: (+/ ∘ ⍉) m     column sums
//! let col_sums = compose(Reduce(Add), Transpose);
//! assert_eq!(col_sums.call(&m).as_slice(), &[5, 7, 9]);
//! // The composed value is reusable, and equals the prefix form +/⍉m.
//! assert_eq!(col_sums.call(&m), m.transpose().reduce(Add));
//! ```
//!
//! The Rust spelling *is* the ASCII spelling. There is no `compose!` macro.

use crate::Array;
use crate::op::Dyadic;

/// A one-argument function value that can be applied to an array.
///
/// It is implemented for `X = &Array<…>` and `X = Array<…>` by every function
/// value in this module.
///
/// ```
/// use arrayglyph::prelude::*;
/// let v = Array::vector([3, 1, 2]);
/// assert_eq!(Reduce(Max).call(&v).item(), 3);
/// ```
pub trait Monadic<X> {
    /// The result type.
    type Output;
    /// Applies the function to `x`.
    fn call(&self, x: X) -> Self::Output;
}

/// `f/` as a value: reduce along the last axis with `F` (a left fold). See
/// [`Array::reduce`].
///
/// ```
/// use arrayglyph::prelude::*;
/// assert_eq!(Reduce(Sub).call(&Array::vector([1, 2, 3])).item(), -4);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Reduce<F>(pub F);

/// `f\` as a value: scan along the last axis with `F`. See [`Array::scan`].
///
/// ```
/// use arrayglyph::prelude::*;
/// assert_eq!(Scan(Add).call(&Array::vector([1, 2, 3])).as_slice(), &[1, 3, 6]);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Scan<F>(pub F);

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
/// let f = compose(compose(Reduce(Add), Transpose), Reshape::<2, 3>);
/// assert_eq!(f.call(&m).as_slice(), &[3, 5, 7]);
/// ```
///
/// Shape errors inside a composition are still compile errors:
///
/// ```compile_fail,E0308
/// use arrayglyph::prelude::*;
/// let m: Array<i32, 2, 3> = Array::splat(1);
/// let col_sums = compose(Reduce(Add), Transpose).call(&m); // 3×1
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

by_ref_and_value!(Reduce<F>, [F,], where [T: Clone, F: Dyadic<T> + Copy], Array<T, R, 1>,
    |this, x| x.reduce(this.0));
by_ref_and_value!(Scan<F>, [F,], where [T: Clone, F: Dyadic<T> + Copy], Array<T, R, C>,
    |this, x| x.scan(this.0));
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
