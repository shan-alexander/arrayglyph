//! Dyadic scalar functions: the `f` in `a f b`, `f/ x`, and `f\ x`.
//!
//! Each function is a zero-sized marker type that implements [`Dyadic`].
//! Markers are passed by value, for example `v.reduce(Add)`.
//!
//! | glyph | marker | associative |
//! |---|---|---|
//! | `+` | [`Add`] | yes |
//! | `-` | [`Sub`] | no |
//! | `×` | [`Mul`] | yes |
//! | `÷` | [`Div`] | no |
//! | `⌈` | [`Max`] | yes |
//! | `⌊` | [`Min`] | yes |

/// A dyadic (two-argument) scalar function usable in elementwise ops,
/// reductions, and scans.
///
/// # Example
///
/// ```
/// use arrayglyph::op::{Dyadic, Sub};
///
/// assert_eq!(<Sub as Dyadic<i32>>::apply(5, 3), 2);
/// assert_eq!(<Sub as Dyadic<i32>>::GLYPH, '-');
/// assert!(!<Sub as Dyadic<i32>>::ASSOCIATIVE);
/// ```
pub trait Dyadic<T> {
    /// The APL glyph for this function, e.g. `'×'`.
    const GLYPH: char;
    /// The verbose Rust name of the marker type, e.g. `"Mul"`.
    const NAME: &'static str;
    /// Whether `(a f b) f c == a f (b f c)`. This picks the O(n) scan.
    const ASSOCIATIVE: bool;
    /// Applies the function to `a` and `b`, in that order.
    fn apply(a: T, b: T) -> T;
}

macro_rules! arith_op {
    ($(#[$doc:meta])* $name:ident, $trait:ident, $method:ident, $glyph:literal, $assoc:literal) => {
        $(#[$doc])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
        pub struct $name;

        impl<T: core::ops::$trait<Output = T>> Dyadic<T> for $name {
            const GLYPH: char = $glyph;
            const NAME: &'static str = stringify!($name);
            const ASSOCIATIVE: bool = $assoc;
            #[inline]
            fn apply(a: T, b: T) -> T {
                core::ops::$trait::$method(a, b)
            }
        }
    };
}

arith_op!(
    /// Addition, glyph `+`.
    ///
    /// ```
    /// use arrayglyph::{op::Add, Array};
    /// assert_eq!(Array::vector([1, 2, 3]).reduce(Add).item(), 6);
    /// ```
    Add, Add, add, '+', true
);
arith_op!(
    /// Subtraction, glyph `-`. This is **not associative**, so reductions
    /// follow APL's right-to-left rule: `-/ 1 2 3` is `1 - (2 - 3)`, which is `2`.
    ///
    /// ```
    /// use arrayglyph::{op::Sub, Array};
    /// assert_eq!(Array::vector([1, 2, 3]).reduce(Sub).item(), 2);
    /// ```
    Sub, Sub, sub, '-', false
);
arith_op!(
    /// Multiplication, glyph `×`.
    ///
    /// ```
    /// use arrayglyph::{op::Mul, Array};
    /// assert_eq!(Array::vector([2, 3, 4]).reduce(Mul).item(), 24);
    /// ```
    Mul, Mul, mul, '×', true
);
arith_op!(
    /// Division, glyph `÷`. It uses `T`'s own `Div`, so integer division
    /// truncates (APL would give a float). It is **not associative**:
    /// `÷/ 8 4 2` is `8 ÷ (4 ÷ 2)`, which is `4`.
    ///
    /// ```
    /// use arrayglyph::{op::Div, Array};
    /// assert_eq!(Array::vector([8, 4, 2]).reduce(Div).item(), 4);
    /// ```
    Div, Div, div, '÷', false
);

/// Maximum, glyph `⌈`. Uses `PartialOrd`. If the comparison is false
/// (for example with NaN), it returns the left operand.
///
/// ```
/// use arrayglyph::{op::Max, Array};
/// assert_eq!(Array::vector([3, 9, 2]).reduce(Max).item(), 9);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Max;

impl<T: PartialOrd> Dyadic<T> for Max {
    const GLYPH: char = '⌈';
    const NAME: &'static str = "Max";
    const ASSOCIATIVE: bool = true;
    #[inline]
    fn apply(a: T, b: T) -> T {
        if b > a { b } else { a }
    }
}

/// Minimum, glyph `⌊`. Uses `PartialOrd`. If the comparison is false
/// (for example with NaN), it returns the left operand.
///
/// ```
/// use arrayglyph::{op::Min, Array};
/// assert_eq!(Array::vector([3, 9, 2]).reduce(Min).item(), 2);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Min;

impl<T: PartialOrd> Dyadic<T> for Min {
    const GLYPH: char = '⌊';
    const NAME: &'static str = "Min";
    const ASSOCIATIVE: bool = true;
    #[inline]
    fn apply(a: T, b: T) -> T {
        if b < a { b } else { a }
    }
}
