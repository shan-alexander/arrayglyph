//! The compile-time-shaped [`Array`] type.

use core::fmt;
use core::ops::{Index, IndexMut};

use crate::op::Dyadic;
use crate::view::Col;

/// An owned, row-major, `Vec`-backed 2-D array whose shape is part of its
/// type.
///
/// `ROWS` and `COLS` are const generics. Operations that need matching shapes
/// take the *same* const parameters, so a mismatch is a **compile error**,
/// not a runtime panic. Vectors are `1 × N` arrays; see [`Vector`].
///
/// ```
/// use arrayglyph::Array;
///
/// let a: Array<i32, 2, 3> = Array::from_rows([[1, 2, 3], [4, 5, 6]]);
/// assert_eq!(a.shape(), (2, 3));
/// assert_eq!(a[(1, 2)], 6);
/// let b = &a + &a;
/// assert_eq!(b.row(0), &[2, 4, 6]);
/// ```
///
/// Mismatched shapes don't compile:
///
/// ```compile_fail,E0308
/// use arrayglyph::Array;
/// let a: Array<i32, 2, 3> = Array::splat(1);
/// let b: Array<i32, 3, 2> = Array::splat(1);
/// let _ = a + b; // ERROR: expected `Array<i32, 2, 3>`, found `Array<i32, 3, 2>`
/// ```
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Array<T, const ROWS: usize, const COLS: usize> {
    /// Row-major storage. Invariant: `data.len() == ROWS * COLS`.
    data: Vec<T>,
}

/// A rank-1 array of length `N`, represented as a `1 × N` [`Array`].
///
/// ```
/// use arrayglyph::{Array, Vector};
/// let v: Vector<f64, 3> = Array::vector([1.0, 2.0, 3.0]);
/// assert_eq!(v[2], 3.0);
/// ```
pub type Vector<T, const N: usize> = Array<T, 1, N>;

/// Error returned by [`Array::from_vec`] when the runtime length doesn't match
/// the compile-time shape.
///
/// ```
/// use arrayglyph::{Array, ShapeError};
/// let err = Array::<i32, 2, 2>::from_vec(vec![1, 2, 3]).unwrap_err();
/// assert_eq!(err, ShapeError { expected: 4, found: 3 });
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShapeError {
    /// `ROWS * COLS` for the target type.
    pub expected: usize,
    /// Length of the `Vec` that was supplied.
    pub found: usize,
}

impl fmt::Display for ShapeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "LENGTH ERROR: expected {} elements, found {}",
            self.expected, self.found
        )
    }
}

impl std::error::Error for ShapeError {}

/// `⍳N`: the vector `0 1 … N-1`. The index origin is 0, as in Rust.
///
/// ```
/// use arrayglyph::iota;
/// assert_eq!(iota::<4>().as_slice(), &[0, 1, 2, 3]);
/// ```
pub fn iota<const N: usize>() -> Vector<usize, N> {
    Array {
        data: (0..N).collect(),
    }
}

impl<T, const ROWS: usize, const COLS: usize> Array<T, ROWS, COLS> {
    /// The number of rows, known at compile time.
    ///
    /// ```
    /// use arrayglyph::Array;
    /// assert_eq!(Array::<u8, 2, 5>::ROWS, 2);
    /// ```
    pub const ROWS: usize = ROWS;

    /// The number of columns, known at compile time.
    ///
    /// ```
    /// use arrayglyph::Array;
    /// assert_eq!(Array::<u8, 2, 5>::COLS, 5);
    /// ```
    pub const COLS: usize = COLS;

    /// The total number of elements, `ROWS * COLS`.
    ///
    /// ```
    /// use arrayglyph::Array;
    /// assert_eq!(Array::<u8, 2, 5>::LEN, 10);
    /// ```
    pub const LEN: usize = ROWS * COLS;

    /// Builds an array from row-major data whose length is only known at
    /// runtime.
    ///
    /// # Errors
    ///
    /// Returns [`ShapeError`] if `data.len() != ROWS * COLS`.
    ///
    /// ```
    /// use arrayglyph::Array;
    /// let m = Array::<i32, 2, 2>::from_vec(vec![1, 2, 3, 4]).unwrap();
    /// assert_eq!(m[(1, 0)], 3);
    /// ```
    pub fn from_vec(data: Vec<T>) -> Result<Self, ShapeError> {
        if data.len() == ROWS * COLS {
            Ok(Self { data })
        } else {
            Err(ShapeError {
                expected: ROWS * COLS,
                found: data.len(),
            })
        }
    }

    /// Builds an array from nested fixed-size arrays. The shape is checked at
    /// compile time.
    ///
    /// ```
    /// use arrayglyph::Array;
    /// let m = Array::from_rows([[1, 2, 3], [4, 5, 6]]);
    /// assert_eq!(m.shape(), (2, 3));
    /// ```
    ///
    /// Ragged literals don't compile:
    ///
    /// ```compile_fail,E0308
    /// use arrayglyph::Array;
    /// let m: Array<i32, 2, 3> = Array::from_rows([[1, 2, 3], [4, 5]]);
    /// ```
    pub fn from_rows(rows: [[T; COLS]; ROWS]) -> Self {
        Self {
            data: rows.into_iter().flatten().collect(),
        }
    }

    /// Builds an array by calling `f(row, col)` for each position, in
    /// row-major order.
    ///
    /// ```
    /// use arrayglyph::Array;
    /// let eye: Array<i32, 3, 3> = Array::from_fn(|r, c| i32::from(r == c));
    /// assert_eq!(eye.row(1), &[0, 1, 0]);
    /// ```
    pub fn from_fn(mut f: impl FnMut(usize, usize) -> T) -> Self {
        let mut data = Vec::with_capacity(ROWS * COLS);
        for r in 0..ROWS {
            for c in 0..COLS {
                data.push(f(r, c));
            }
        }
        Self { data }
    }

    /// `⍴x`: the shape as `(rows, cols)`.
    ///
    /// ```
    /// use arrayglyph::Array;
    /// let m: Array<u8, 4, 2> = Array::splat(0);
    /// assert_eq!(m.shape(), (4, 2));
    /// ```
    pub const fn shape(&self) -> (usize, usize) {
        (ROWS, COLS)
    }

    /// Returns the total number of elements.
    ///
    /// ```
    /// use arrayglyph::Array;
    /// assert_eq!(Array::<u8, 4, 2>::splat(0).len(), 8);
    /// ```
    pub const fn len(&self) -> usize {
        ROWS * COLS
    }

    /// Returns `true` if the array has no elements.
    ///
    /// ```
    /// use arrayglyph::Array;
    /// assert!(Array::<u8, 0, 3>::splat(0).is_empty());
    /// ```
    pub const fn is_empty(&self) -> bool {
        ROWS * COLS == 0
    }

    /// Borrows the row-major storage.
    ///
    /// ```
    /// use arrayglyph::Array;
    /// assert_eq!(Array::from_rows([[1, 2], [3, 4]]).as_slice(), &[1, 2, 3, 4]);
    /// ```
    pub fn as_slice(&self) -> &[T] {
        &self.data
    }

    /// Mutably borrows the row-major storage. The length can't change, so the
    /// shape invariant holds.
    ///
    /// ```
    /// use arrayglyph::Array;
    /// let mut m = Array::from_rows([[1, 2], [3, 4]]);
    /// m.as_mut_slice()[3] = 9;
    /// assert_eq!(m[(1, 1)], 9);
    /// ```
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        &mut self.data
    }

    /// Consumes the array and returns its row-major storage.
    ///
    /// ```
    /// use arrayglyph::Array;
    /// assert_eq!(Array::vector([1, 2]).into_vec(), vec![1, 2]);
    /// ```
    pub fn into_vec(self) -> Vec<T> {
        self.data
    }

    /// Iterates over every element in row-major order.
    ///
    /// ```
    /// use arrayglyph::Array;
    /// let m = Array::from_rows([[1, 2], [3, 4]]);
    /// assert_eq!(m.iter().sum::<i32>(), 10);
    /// ```
    pub fn iter(&self) -> core::slice::Iter<'_, T> {
        self.data.iter()
    }

    /// Mutably iterates over every element in row-major order.
    ///
    /// ```
    /// use arrayglyph::Array;
    /// let mut m = Array::from_rows([[1, 2], [3, 4]]);
    /// m.iter_mut().for_each(|x| *x *= 10);
    /// assert_eq!(m.row(1), &[30, 40]);
    /// ```
    pub fn iter_mut(&mut self) -> core::slice::IterMut<'_, T> {
        self.data.iter_mut()
    }

    /// Returns the element at `(row, col)`, or `None` if it is out of bounds.
    ///
    /// ```
    /// use arrayglyph::Array;
    /// let m = Array::from_rows([[1, 2], [3, 4]]);
    /// assert_eq!(m.get(0, 1), Some(&2));
    /// assert_eq!(m.get(0, 2), None);
    /// ```
    pub fn get(&self, row: usize, col: usize) -> Option<&T> {
        if row < ROWS && col < COLS {
            self.data.get(row * COLS + col)
        } else {
            None
        }
    }

    /// Mutable version of [`get`](Self::get).
    ///
    /// ```
    /// use arrayglyph::Array;
    /// let mut m = Array::from_rows([[1, 2], [3, 4]]);
    /// *m.get_mut(1, 0).unwrap() = 7;
    /// assert_eq!(m[(1, 0)], 7);
    /// ```
    pub fn get_mut(&mut self, row: usize, col: usize) -> Option<&mut T> {
        if row < ROWS && col < COLS {
            self.data.get_mut(row * COLS + col)
        } else {
            None
        }
    }

    /// Borrows row `i` as a contiguous slice of length `COLS`.
    ///
    /// # Panics
    ///
    /// Panics if `i >= ROWS`.
    ///
    /// ```
    /// use arrayglyph::Array;
    /// let m = Array::from_rows([[1, 2], [3, 4]]);
    /// assert_eq!(m.row(1), &[3, 4]);
    /// ```
    pub fn row(&self, i: usize) -> &[T] {
        assert!(i < ROWS, "row index {i} out of bounds for {ROWS} rows");
        &self.data[i * COLS..(i + 1) * COLS]
    }

    /// Mutably borrows row `i`.
    ///
    /// # Panics
    ///
    /// Panics if `i >= ROWS`.
    ///
    /// ```
    /// use arrayglyph::Array;
    /// let mut m = Array::from_rows([[1, 2], [3, 4]]);
    /// m.row_mut(0).reverse();
    /// assert_eq!(m.row(0), &[2, 1]);
    /// ```
    pub fn row_mut(&mut self, i: usize) -> &mut [T] {
        assert!(i < ROWS, "row index {i} out of bounds for {ROWS} rows");
        &mut self.data[i * COLS..(i + 1) * COLS]
    }

    /// Iterates over the rows as slices.
    ///
    /// ```
    /// use arrayglyph::Array;
    /// let m = Array::from_rows([[1, 2], [3, 4]]);
    /// let sums: Vec<i32> = m.rows().map(|r| r.iter().sum()).collect();
    /// assert_eq!(sums, vec![3, 7]);
    /// ```
    pub fn rows(&self) -> impl DoubleEndedIterator<Item = &[T]> + ExactSizeIterator {
        let data = &self.data;
        (0..ROWS).map(move |i| &data[i * COLS..(i + 1) * COLS])
    }

    /// Borrows column `j` as a strided [`Col`] view.
    ///
    /// # Panics
    ///
    /// Panics if `j >= COLS`.
    ///
    /// ```
    /// use arrayglyph::Array;
    /// let m = Array::from_rows([[1, 2], [3, 4]]);
    /// assert_eq!(m.col(1).to_vec(), vec![2, 4]);
    /// ```
    pub fn col(&self, j: usize) -> Col<'_, T> {
        assert!(
            j < COLS,
            "column index {j} out of bounds for {COLS} columns"
        );
        Col::new(&self.data, j, COLS, ROWS)
    }

    /// Iterates over the columns as [`Col`] views.
    ///
    /// ```
    /// use arrayglyph::Array;
    /// let m = Array::from_rows([[1, 2], [3, 4]]);
    /// let sums: Vec<i32> = m.cols().map(|c| c.iter().sum()).collect();
    /// assert_eq!(sums, vec![4, 6]);
    /// ```
    pub fn cols(&self) -> impl DoubleEndedIterator<Item = Col<'_, T>> + ExactSizeIterator {
        let data = &self.data;
        (0..COLS).map(move |j| Col::new(data, j, COLS, ROWS))
    }

    /// Applies `f` to every element. The result has the same shape.
    ///
    /// ```
    /// use arrayglyph::Array;
    /// let v = Array::vector([1, 2, 3]).map(|x| x * x);
    /// assert_eq!(v.as_slice(), &[1, 4, 9]);
    /// ```
    pub fn map<U>(self, f: impl FnMut(T) -> U) -> Array<U, ROWS, COLS> {
        Array {
            data: self.data.into_iter().map(f).collect(),
        }
    }

    /// `a f b`: combines two arrays of the **same shape** elementwise with
    /// a [`Dyadic`] function.
    ///
    /// ```
    /// use arrayglyph::{op::Max, Array};
    /// let a = Array::vector([1, 5, 3]);
    /// let b = Array::vector([4, 2, 6]);
    /// assert_eq!(a.zip_with(&b, Max).as_slice(), &[4, 5, 6]);
    /// ```
    ///
    /// ```compile_fail,E0308
    /// use arrayglyph::{op::Add, Array};
    /// let a = Array::vector([1, 2, 3]);
    /// let b = Array::vector([1, 2]);
    /// let _ = a.zip_with(&b, Add); // ERROR: shapes 1×3 and 1×2 differ
    /// ```
    pub fn zip_with<O: Dyadic<T>>(&self, other: &Self, _op: O) -> Self
    where
        T: Clone,
    {
        Self {
            data: self
                .data
                .iter()
                .zip(&other.data)
                .map(|(a, b)| O::apply(a.clone(), b.clone()))
                .collect(),
        }
    }

    /// `⍉x`: transpose.
    ///
    /// ```
    /// use arrayglyph::Array;
    /// let m = Array::from_rows([[1, 2, 3], [4, 5, 6]]);
    /// let t: Array<i32, 3, 2> = m.transpose();
    /// assert_eq!(t.row(0), &[1, 4]);
    /// ```
    pub fn transpose(&self) -> Array<T, COLS, ROWS>
    where
        T: Clone,
    {
        Array::from_fn(|r, c| self.data[c * COLS + r].clone())
    }

    /// `R2 C2⍴x`: reinterprets the data with a new shape. The element count
    /// must be equal, and this is checked **at compile time**.
    ///
    /// ```
    /// use arrayglyph::iota;
    /// let m = iota::<6>().reshape::<2, 3>();
    /// assert_eq!(m.row(1), &[3, 4, 5]);
    /// ```
    ///
    /// ```compile_fail,E0080
    /// use arrayglyph::iota;
    /// let _ = iota::<6>().reshape::<4, 2>(); // ERROR: 6 != 8 elements
    /// ```
    pub fn reshape<const R2: usize, const C2: usize>(self) -> Array<T, R2, C2> {
        const {
            assert!(
                ROWS * COLS == R2 * C2,
                "reshape: element count must be preserved (ROWS*COLS == R2*C2)"
            )
        };
        Array { data: self.data }
    }

    /// `a +.× b`: matrix product. The inner dimensions must match **at
    /// compile time**: `(R × C) · (C × K) → (R × K)`.
    ///
    /// ```
    /// use arrayglyph::Array;
    /// let a = Array::from_rows([[1, 2], [3, 4]]);
    /// let b = Array::from_rows([[5, 6, 7], [8, 9, 10]]);
    /// let c: Array<i32, 2, 3> = a.matmul(&b);
    /// assert_eq!(c.row(0), &[21, 24, 27]);
    /// ```
    ///
    /// ```compile_fail,E0308
    /// use arrayglyph::Array;
    /// let a: Array<i32, 2, 3> = Array::splat(1);
    /// let b: Array<i32, 2, 3> = Array::splat(1);
    /// let _ = a.matmul(&b); // ERROR: inner dimensions 3 and 2 differ
    /// ```
    pub fn matmul<const K: usize>(&self, other: &Array<T, COLS, K>) -> Array<T, ROWS, K>
    where
        T: Clone + Default + core::ops::Add<Output = T> + core::ops::Mul<Output = T>,
    {
        Array::from_fn(|r, k| {
            (0..COLS).fold(T::default(), |acc, c| {
                acc + self.data[r * COLS + c].clone() * other.data[c * K + k].clone()
            })
        })
    }

    /// `f⌿ x`: reduces along **axis 0**, down each column, as a **left
    /// fold**: `f⌿` over rows `a b c` is `(a f b) f c`, element by element.
    /// An `R × C` array becomes `1 × C`. This is the default axis, so
    /// `reduce!(+, m)` gives column totals.
    ///
    /// For associative ops (`+ × ⌈ ⌊`) the fold direction doesn't matter.
    /// For `-` and `÷` it does, and arrayglyph folds left (like
    /// [`Iterator::reduce`]) where APL folds right.
    ///
    /// For the other axis use [`reduce_axis`](Array::reduce_axis)`::<1>`.
    ///
    /// ```
    /// use arrayglyph::{op::{Add, Max}, Array};
    /// let m = Array::from_rows([[1, 2, 3], [4, 5, 6]]);
    /// assert_eq!(m.reduce(Add).as_slice(), &[5, 7, 9]); // 1×3
    /// assert_eq!(m.reduce(Max).shape(), (1, 3));
    /// ```
    ///
    /// Axis 0 must have at least two rows. Reducing a 1-row array (such as a
    /// [`Vector`](crate::Vector)) down its columns would return it unchanged,
    /// which is almost always a mistake, so it is a compile error. Reduce a
    /// vector with `reduce_axis::<1>`:
    ///
    /// ```compile_fail,E0080
    /// use arrayglyph::{op::Add, Array};
    /// let v = Array::vector([1, 2, 3]);
    /// let _ = v.reduce(Add); // ERROR: axis 0 of a 1-row array; use axis 1
    /// ```
    pub fn reduce<O: Dyadic<T>>(&self, op: O) -> Array<T, 1, COLS>
    where
        T: Clone,
    {
        <Axis<0> as AlongAxis<T, ROWS, COLS>>::reduce(self, op)
    }

    /// `f/ x` (axis 1) or `f⌿ x` (axis 0): reduces along `AXIS` as a left
    /// fold. `AXIS = 0` goes down each column (`R × C → 1 × C`, the same as
    /// [`reduce`](Array::reduce)); `AXIS = 1` goes along each row
    /// (`R × C → R × 1`). The reduced axis is kept with length 1, so
    /// reducing both axes is two calls, and a vector's total is
    /// `.reduce_axis::<1>(op).item()`.
    ///
    /// ```
    /// use arrayglyph::{op::{Add, Sub}, Array};
    /// let m = Array::from_rows([[1, 2, 3], [4, 5, 6]]);
    /// assert_eq!(m.reduce_axis::<0>(Add).as_slice(), &[5, 7, 9]);
    /// assert_eq!(m.reduce_axis::<1>(Add).as_slice(), &[6, 15]);
    /// // Grand total: one axis, then the other.
    /// assert_eq!(m.reduce(Add).reduce_axis::<1>(Add).item(), 21);
    /// // Left fold: -/ 1 2 3  =  (1 - 2) - 3  =  -4
    /// assert_eq!(Array::vector([1, 2, 3]).reduce_axis::<1>(Sub).item(), -4);
    /// ```
    ///
    /// Arrays have two axes, so any other `AXIS` is a compile error:
    ///
    /// ```compile_fail,E0277
    /// use arrayglyph::{op::Add, Array};
    /// let m = Array::from_rows([[1, 2], [3, 4]]);
    /// let _ = m.reduce_axis::<2>(Add); // ERROR: no axis 2
    /// ```
    ///
    /// Reducing a length-1 axis is a no-op, and therefore a compile error:
    ///
    /// ```compile_fail,E0080
    /// use arrayglyph::{op::Add, Array};
    /// let col: Array<i32, 3, 1> = Array::splat(1);
    /// let _ = col.reduce_axis::<1>(Add); // ERROR: axis 1 has length 1
    /// ```
    pub fn reduce_axis<const AXIS: usize>(
        &self,
        op: impl Dyadic<T>,
    ) -> <Axis<AXIS> as AlongAxis<T, ROWS, COLS>>::Reduced
    where
        T: Clone,
        Axis<AXIS>: AlongAxis<T, ROWS, COLS>,
    {
        <Axis<AXIS> as AlongAxis<T, ROWS, COLS>>::reduce(self, op)
    }

    /// `f⍀ x`: scans along **axis 0**, down each column (the default axis).
    /// Row `i` of the result is `f⌿` of rows `0..=i`: running totals per
    /// column. The shape doesn't change. Computed in O(n) for every op.
    ///
    /// ```
    /// use arrayglyph::{op::Add, Array};
    /// let m = Array::from_rows([[1, 2], [3, 4], [5, 6]]);
    /// assert_eq!(m.scan(Add).as_slice(), &[1, 2, 4, 6, 9, 12]);
    /// ```
    ///
    /// Like [`reduce`](Array::reduce), axis 0 needs at least two rows; scan
    /// a vector with `scan_axis::<1>`:
    ///
    /// ```compile_fail,E0080
    /// use arrayglyph::{op::Add, Array};
    /// let _ = Array::vector([1, 2, 3]).scan(Add); // ERROR: use axis 1
    /// ```
    pub fn scan<O: Dyadic<T>>(&self, op: O) -> Self
    where
        T: Clone,
    {
        <Axis<0> as AlongAxis<T, ROWS, COLS>>::scan(self, op)
    }

    /// `f\ x` (axis 1) or `f⍀ x` (axis 0): scans along `AXIS`. Element `i`
    /// along the axis is the left fold of elements `0..=i`.
    ///
    /// ```
    /// use arrayglyph::{op::{Add, Sub}, Array};
    /// assert_eq!(Array::vector([1, 2, 3, 4]).scan_axis::<1>(Add).as_slice(), &[1, 3, 6, 10]);
    /// // -\ 1 2 3  is  1, 1-2, (1-2)-3  =  1 ¯1 ¯4
    /// assert_eq!(Array::vector([1, 2, 3]).scan_axis::<1>(Sub).as_slice(), &[1, -1, -4]);
    /// ```
    ///
    /// ```compile_fail,E0277
    /// use arrayglyph::{op::Add, Array};
    /// let _ = Array::from_rows([[1, 2], [3, 4]]).scan_axis::<7>(Add); // ERROR
    /// ```
    pub fn scan_axis<const AXIS: usize>(&self, op: impl Dyadic<T>) -> Self
    where
        T: Clone,
        Axis<AXIS>: AlongAxis<T, ROWS, COLS>,
    {
        <Axis<AXIS> as AlongAxis<T, ROWS, COLS>>::scan(self, op)
    }
}

/// A type-level axis number, used by [`Array::reduce_axis`] and
/// [`Array::scan_axis`] (and by `axis = N` in [`reduce!`](crate::reduce) and
/// [`scan!`](crate::scan)). `Axis<0>` runs down the columns (the rows axis,
/// like APL's first axis `⌿`) and `Axis<1>` runs along each row (APL's last
/// axis `/`). Only those two implement [`AlongAxis`].
///
/// ```
/// use arrayglyph::{Array, Axis, AlongAxis, op::Add};
/// let m = Array::from_rows([[1, 2], [3, 4]]);
/// assert_eq!(<Axis<1> as AlongAxis<_, 2, 2>>::reduce(&m, Add).as_slice(), &[3, 7]);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Axis<const N: usize>;

/// Reduction and scan along one axis of an `R × C` array. Implemented only
/// for [`Axis<0>`] and [`Axis<1>`]; the reduced shape is the associated
/// type, so the result shape is known at compile time.
///
/// Both impls require the axis to have length at least 2 (checked at compile
/// time): reducing or scanning a length-1 axis returns the input unchanged,
/// which in practice means the wrong axis was picked.
///
/// ```
/// use arrayglyph::{Array, Axis, AlongAxis, op::Add};
/// let m = Array::from_rows([[1, 2, 3], [4, 5, 6]]);
/// let cols: Array<i32, 1, 3> = <Axis<0> as AlongAxis<_, 2, 3>>::reduce(&m, Add);
/// assert_eq!(cols.as_slice(), &[5, 7, 9]);
/// ```
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not an axis of a 2-D array",
    label = "use axis 0 (down columns) or axis 1 (along rows)",
    note = "arrayglyph arrays have exactly two axes: 0 (rows axis, `⌿`) and 1 (columns axis, `/`)"
)]
pub trait AlongAxis<T, const ROWS: usize, const COLS: usize> {
    /// The shape after reducing this axis to length 1.
    type Reduced;
    /// Left-fold reduction along this axis.
    fn reduce<O: Dyadic<T>>(a: &Array<T, ROWS, COLS>, op: O) -> Self::Reduced
    where
        T: Clone;
    /// Running left fold along this axis.
    fn scan<O: Dyadic<T>>(a: &Array<T, ROWS, COLS>, op: O) -> Array<T, ROWS, COLS>
    where
        T: Clone;
}

impl<T, const ROWS: usize, const COLS: usize> AlongAxis<T, ROWS, COLS> for Axis<0> {
    type Reduced = Array<T, 1, COLS>;

    fn reduce<O: Dyadic<T>>(a: &Array<T, ROWS, COLS>, _op: O) -> Self::Reduced
    where
        T: Clone,
    {
        const {
            assert!(
                ROWS >= 2,
                "reduce along axis 0 needs at least 2 rows: a 1-row array (such as a \
                 vector) is unchanged by it; use axis = 1"
            )
        };
        let mut rows = a.rows();
        let mut acc: Vec<T> = rows.next().expect("ROWS >= 2").to_vec();
        for row in rows {
            acc = acc
                .into_iter()
                .zip(row.iter().cloned())
                .map(|(x, y)| O::apply(x, y))
                .collect();
        }
        Array { data: acc }
    }

    fn scan<O: Dyadic<T>>(a: &Array<T, ROWS, COLS>, _op: O) -> Array<T, ROWS, COLS>
    where
        T: Clone,
    {
        const {
            assert!(
                ROWS >= 2,
                "scan along axis 0 needs at least 2 rows: a 1-row array (such as a \
                 vector) is unchanged by it; use axis = 1"
            )
        };
        let mut data: Vec<T> = Vec::with_capacity(ROWS * COLS);
        data.extend_from_slice(a.row(0));
        for i in 1..ROWS {
            for (j, x) in a.row(i).iter().enumerate() {
                let next = O::apply(data[(i - 1) * COLS + j].clone(), x.clone());
                data.push(next);
            }
        }
        Array { data }
    }
}

impl<T, const ROWS: usize, const COLS: usize> AlongAxis<T, ROWS, COLS> for Axis<1> {
    type Reduced = Array<T, ROWS, 1>;

    fn reduce<O: Dyadic<T>>(a: &Array<T, ROWS, COLS>, _op: O) -> Self::Reduced
    where
        T: Clone,
    {
        const {
            assert!(
                COLS >= 2,
                "reduce along axis 1 needs at least 2 columns: a 1-column array is \
                 unchanged by it; use axis = 0"
            )
        };
        Array {
            data: a
                .rows()
                .map(|row| row.iter().cloned().reduce(O::apply).expect("COLS >= 2"))
                .collect(),
        }
    }

    fn scan<O: Dyadic<T>>(a: &Array<T, ROWS, COLS>, _op: O) -> Array<T, ROWS, COLS>
    where
        T: Clone,
    {
        const {
            assert!(
                COLS >= 2,
                "scan along axis 1 needs at least 2 columns: a 1-column array is \
                 unchanged by it; use axis = 0"
            )
        };
        let mut data = Vec::with_capacity(ROWS * COLS);
        for row in a.rows() {
            let mut it = row.iter().cloned();
            let first = it.next().expect("COLS >= 2");
            data.push(first.clone());
            data.extend(it.scan(first, |acc, x| {
                *acc = O::apply(acc.clone(), x);
                Some(acc.clone())
            }));
        }
        Array { data }
    }
}

impl<T: Clone, const ROWS: usize, const COLS: usize> Array<T, ROWS, COLS> {
    /// Fills the whole array with `value`. This is APL scalar extension.
    /// The shape usually comes from type inference.
    ///
    /// ```
    /// use arrayglyph::{op::Mul, Array};
    /// let v = Array::vector([1, 2, 3]);
    /// let doubled = Array::splat(2).zip_with(&v, Mul); // shape inferred as 1×3
    /// assert_eq!(doubled.as_slice(), &[2, 4, 6]);
    /// ```
    pub fn splat(value: T) -> Self {
        Self {
            data: vec![value; ROWS * COLS],
        }
    }

    /// Stretches length-1 axes to a larger shape, like NumPy's
    /// `broadcast_to`. A `1 × C` row repeats down `R2` rows; an `R × 1`
    /// column repeats across `C2` columns. Axes that aren't length 1 must
    /// already match; that is checked at compile time.
    ///
    /// This is what makes "subtract the column means" type-check: the means
    /// are `1 × C`, the data is `R × C`.
    ///
    /// ```
    /// use arrayglyph::{op::Sub, Array};
    /// let m = Array::from_rows([[1, 2, 3], [4, 5, 6]]);
    /// let first_row = Array::vector([1, 2, 3]);
    /// // Shape R2 × C2 is inferred from `zip_with`'s other operand (2×3).
    /// let shifted = m.zip_with(&first_row.broadcast(), Sub);
    /// assert_eq!(shifted.as_slice(), &[0, 0, 0, 3, 3, 3]);
    ///
    /// let col: Array<i32, 2, 1> = Array::from_rows([[10], [20]]);
    /// assert_eq!(col.broadcast::<2, 3>().as_slice(), &[10, 10, 10, 20, 20, 20]);
    /// ```
    ///
    /// ```compile_fail,E0080
    /// use arrayglyph::Array;
    /// let v = Array::vector([1, 2, 3]);
    /// let _ = v.broadcast::<2, 4>(); // ERROR: 3 columns can't become 4
    /// ```
    pub fn broadcast<const R2: usize, const C2: usize>(&self) -> Array<T, R2, C2> {
        const {
            assert!(
                (ROWS == R2 || ROWS == 1) && (COLS == C2 || COLS == 1),
                "broadcast: each axis must already match or have length 1"
            )
        };
        Array::from_fn(|r, c| {
            let (sr, sc) = (if ROWS == 1 { 0 } else { r }, if COLS == 1 { 0 } else { c });
            self.data[sr * COLS + sc].clone()
        })
    }
}

impl<T: Default, const ROWS: usize, const COLS: usize> Default for Array<T, ROWS, COLS> {
    /// Fills the array with `T::default()`.
    ///
    /// ```
    /// use arrayglyph::Array;
    /// let z: Array<f32, 2, 2> = Array::default();
    /// assert!(z.iter().all(|&x| x == 0.0));
    /// ```
    fn default() -> Self {
        Self::from_fn(|_, _| T::default())
    }
}

impl<T, const N: usize> Array<T, 1, N> {
    /// Builds a vector (a `1 × N` array) from a fixed-size array. This is the
    /// glyph strand `1 2 3`.
    ///
    /// ```
    /// use arrayglyph::Array;
    /// let v = Array::vector([1, 2, 3]);
    /// assert_eq!(v.shape(), (1, 3));
    /// ```
    pub fn vector(items: [T; N]) -> Self {
        Self { data: items.into() }
    }
}

impl<T> Array<T, 1, 1> {
    /// Unwraps a `1 × 1` array, such as the result of reducing a vector,
    /// into its single element.
    ///
    /// ```
    /// use arrayglyph::{op::Add, Array};
    /// let total: i32 = Array::vector([1, 2, 3]).reduce_axis::<1>(Add).item();
    /// assert_eq!(total, 6);
    /// ```
    pub fn item(self) -> T {
        self.data
            .into_iter()
            .next()
            .expect("1x1 array has one element")
    }
}

impl<T, const ROWS: usize, const COLS: usize> Index<(usize, usize)> for Array<T, ROWS, COLS> {
    type Output = T;

    /// Indexes with `(row, col)`. Panics if either index is out of bounds.
    ///
    /// ```
    /// use arrayglyph::Array;
    /// let m = Array::from_rows([[1, 2], [3, 4]]);
    /// assert_eq!(m[(1, 0)], 3);
    /// ```
    fn index(&self, (r, c): (usize, usize)) -> &T {
        self.get(r, c)
            .unwrap_or_else(|| panic!("index ({r}, {c}) out of bounds for shape {ROWS}×{COLS}"))
    }
}

impl<T, const ROWS: usize, const COLS: usize> IndexMut<(usize, usize)> for Array<T, ROWS, COLS> {
    /// Mutably indexes with `(row, col)`.
    ///
    /// ```
    /// use arrayglyph::Array;
    /// let mut m = Array::from_rows([[1, 2], [3, 4]]);
    /// m[(0, 0)] = 9;
    /// assert_eq!(m.row(0), &[9, 2]);
    /// ```
    fn index_mut(&mut self, (r, c): (usize, usize)) -> &mut T {
        self.get_mut(r, c)
            .unwrap_or_else(|| panic!("index ({r}, {c}) out of bounds for shape {ROWS}×{COLS}"))
    }
}

impl<T, const N: usize> Index<usize> for Array<T, 1, N> {
    type Output = T;

    /// Indexes a vector with a single index.
    ///
    /// ```
    /// use arrayglyph::Array;
    /// assert_eq!(Array::vector([10, 20, 30])[1], 20);
    /// ```
    fn index(&self, i: usize) -> &T {
        &self.data[i]
    }
}

impl<T, const N: usize> IndexMut<usize> for Array<T, 1, N> {
    /// Mutably indexes a vector with a single index.
    ///
    /// ```
    /// use arrayglyph::Array;
    /// let mut v = Array::vector([10, 20, 30]);
    /// v[1] = 0;
    /// assert_eq!(v.as_slice(), &[10, 0, 30]);
    /// ```
    fn index_mut(&mut self, i: usize) -> &mut T {
        &mut self.data[i]
    }
}

impl<T, const ROWS: usize, const COLS: usize> IntoIterator for Array<T, ROWS, COLS> {
    type Item = T;
    type IntoIter = std::vec::IntoIter<T>;

    /// Consumes the array and yields its elements in row-major order.
    ///
    /// ```
    /// use arrayglyph::Array;
    /// let v: Vec<i32> = Array::vector([1, 2]).into_iter().collect();
    /// assert_eq!(v, vec![1, 2]);
    /// ```
    fn into_iter(self) -> Self::IntoIter {
        self.data.into_iter()
    }
}

impl<'a, T, const ROWS: usize, const COLS: usize> IntoIterator for &'a Array<T, ROWS, COLS> {
    type Item = &'a T;
    type IntoIter = core::slice::Iter<'a, T>;

    /// Yields references to the elements in row-major order.
    ///
    /// ```
    /// use arrayglyph::Array;
    /// let m = Array::vector([1, 2]);
    /// let mut n = 0;
    /// for x in &m { n += x; }
    /// assert_eq!(n, 3);
    /// ```
    fn into_iter(self) -> Self::IntoIter {
        self.data.iter()
    }
}

macro_rules! impl_binop {
    ($trait:ident, $method:ident, $marker:ident, $sym:literal) => {
        #[doc = concat!("Elementwise `", $sym, "` on two owned arrays of the same shape.")]
        ///
        /// ```
        /// use arrayglyph::Array;
        /// let a = Array::vector([8, 6]);
        /// let b = Array::vector([2, 3]);
        #[doc = concat!("let c = a ", $sym, " b;")]
        /// assert_eq!(c.len(), 2);
        /// ```
        impl<T, const ROWS: usize, const COLS: usize> core::ops::$trait for Array<T, ROWS, COLS>
        where
            T: core::ops::$trait<Output = T>,
        {
            type Output = Self;
            fn $method(self, rhs: Self) -> Self {
                Self {
                    data: self
                        .data
                        .into_iter()
                        .zip(rhs.data)
                        .map(|(a, b)| core::ops::$trait::$method(a, b))
                        .collect(),
                }
            }
        }

        #[doc = concat!("Elementwise `", $sym, "` on two borrowed arrays of the same shape.")]
        ///
        /// ```
        /// use arrayglyph::Array;
        /// let a = Array::vector([8, 6]);
        /// let b = Array::vector([2, 3]);
        #[doc = concat!("let c = &a ", $sym, " &b;")]
        /// assert_eq!(c.len(), 2);
        /// ```
        impl<T, const ROWS: usize, const COLS: usize> core::ops::$trait for &Array<T, ROWS, COLS>
        where
            T: Clone + core::ops::$trait<Output = T>,
        {
            type Output = Array<T, ROWS, COLS>;
            fn $method(self, rhs: Self) -> Array<T, ROWS, COLS> {
                self.zip_with(rhs, crate::op::$marker)
            }
        }
    };
}

impl_binop!(Add, add, Add, "+");
impl_binop!(Sub, sub, Sub, "-");
impl_binop!(Mul, mul, Mul, "*");
impl_binop!(Div, div, Div, "/");

impl<T: core::ops::Neg<Output = T>, const ROWS: usize, const COLS: usize> core::ops::Neg
    for Array<T, ROWS, COLS>
{
    type Output = Self;

    /// Elementwise negation (APL monadic `-`).
    ///
    /// ```
    /// use arrayglyph::Array;
    /// assert_eq!((-Array::vector([1, -2])).as_slice(), &[-1, 2]);
    /// ```
    fn neg(self) -> Self {
        self.map(|x| -x)
    }
}

impl<T: fmt::Debug, const ROWS: usize, const COLS: usize> fmt::Debug for Array<T, ROWS, COLS> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Array<{ROWS}×{COLS}>")?;
        f.debug_list().entries(self.rows()).finish()
    }
}

impl<T: fmt::Display, const ROWS: usize, const COLS: usize> fmt::Display for Array<T, ROWS, COLS> {
    /// Prints APL-style: right-aligned columns, one line per row.
    ///
    /// A precision (`{:.2}`) is applied to every element, which keeps
    /// floating-point tables readable, and a width (`{:6}`) sets the minimum
    /// column width.
    ///
    /// ```
    /// use arrayglyph::Array;
    /// let m = Array::from_rows([[1, 20], [300, 4]]);
    /// assert_eq!(m.to_string(), "  1 20\n300  4");
    ///
    /// let t = Array::from_rows([[1.0, 2.25], [-0.5, 10.0]]);
    /// assert_eq!(format!("{t:.1}"), " 1.0  2.2\n-0.5 10.0");
    /// assert_eq!(format!("{:4}", Array::vector([1, 2])), "   1    2");
    /// ```
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let cells: Vec<String> = match f.precision() {
            Some(p) => self.data.iter().map(|x| format!("{x:.p$}")).collect(),
            None => self.data.iter().map(ToString::to_string).collect(),
        };
        let min_width = f.width().unwrap_or(0);
        let widths: Vec<usize> = (0..COLS)
            .map(|c| {
                (0..ROWS)
                    .map(|r| cells[r * COLS + c].chars().count())
                    .max()
                    .unwrap_or(0)
                    .max(min_width)
            })
            .collect();
        for r in 0..ROWS {
            if r > 0 {
                writeln!(f)?;
            }
            for c in 0..COLS {
                if c > 0 {
                    write!(f, " ")?;
                }
                write!(f, "{:>w$}", cells[r * COLS + c], w = widths[c])?;
            }
        }
        Ok(())
    }
}
