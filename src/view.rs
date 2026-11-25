//! Borrowed views into an [`Array`](crate::Array).
//!
//! Rows are contiguous, so a row view is just a `&[T]`
//! ([`Array::row`](crate::Array::row)). Columns are strided, so they get the
//! [`Col`] view type.

use core::ops::Index;

/// A borrowed, strided view of one column of an [`Array`](crate::Array).
///
/// You get one from [`Array::col`](crate::Array::col) or
/// [`Array::cols`](crate::Array::cols). Nothing is copied.
///
/// ```
/// use arrayglyph::Array;
///
/// let m = Array::from_rows([[1, 2, 3], [4, 5, 6]]);
/// let c = m.col(1);
/// assert_eq!(c.len(), 2);
/// assert_eq!(c[1], 5);
/// assert_eq!(c.iter().copied().collect::<Vec<_>>(), vec![2, 5]);
/// ```
#[derive(Debug, Clone, Copy)]
pub struct Col<'a, T> {
    data: &'a [T],
    start: usize,
    stride: usize,
    len: usize,
}

impl<'a, T> Col<'a, T> {
    pub(crate) fn new(data: &'a [T], start: usize, stride: usize, len: usize) -> Self {
        Self {
            data,
            start,
            stride,
            len,
        }
    }

    /// Returns the number of elements in the column (the array's `ROWS`).
    ///
    /// ```
    /// use arrayglyph::Array;
    /// let m = Array::from_rows([[1, 2], [3, 4], [5, 6]]);
    /// assert_eq!(m.col(0).len(), 3);
    /// ```
    pub fn len(&self) -> usize {
        self.len
    }

    /// Returns `true` if the column has no elements (`ROWS == 0`).
    ///
    /// ```
    /// use arrayglyph::Array;
    /// let m = Array::from_rows([[1, 2]]);
    /// assert!(!m.col(0).is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Returns the element in row `i` of this column, or `None` if it is
    /// out of bounds.
    ///
    /// ```
    /// use arrayglyph::Array;
    /// let m = Array::from_rows([[1, 2], [3, 4]]);
    /// assert_eq!(m.col(1).get(1), Some(&4));
    /// assert_eq!(m.col(1).get(2), None);
    /// ```
    pub fn get(&self, i: usize) -> Option<&'a T> {
        if i < self.len {
            self.data.get(self.start + i * self.stride)
        } else {
            None
        }
    }

    /// Iterates over the column from top to bottom.
    ///
    /// ```
    /// use arrayglyph::Array;
    /// let m = Array::from_rows([[1, 2], [3, 4]]);
    /// assert_eq!(m.col(0).iter().sum::<i32>(), 4);
    /// ```
    pub fn iter(&self) -> impl DoubleEndedIterator<Item = &'a T> + ExactSizeIterator + 'a {
        let (data, start, stride) = (self.data, self.start, self.stride);
        (0..self.len).map(move |i| &data[start + i * stride])
    }

    /// Copies the column into a `Vec`.
    ///
    /// ```
    /// use arrayglyph::Array;
    /// let m = Array::from_rows([[1, 2], [3, 4]]);
    /// assert_eq!(m.col(1).to_vec(), vec![2, 4]);
    /// ```
    pub fn to_vec(&self) -> Vec<T>
    where
        T: Clone,
    {
        self.iter().cloned().collect()
    }
}

impl<T> Index<usize> for Col<'_, T> {
    type Output = T;

    /// Indexes into the column. Panics if `i >= len()`.
    ///
    /// ```
    /// use arrayglyph::Array;
    /// let m = Array::from_rows([[1, 2], [3, 4]]);
    /// assert_eq!(m.col(0)[1], 3);
    /// ```
    fn index(&self, i: usize) -> &T {
        self.get(i).unwrap_or_else(|| {
            panic!("column index {i} out of bounds for length {}", self.len)
        })
    }
}
