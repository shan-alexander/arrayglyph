//! Source positions and spans for the printer.
//!
//! Every glyph/Rust token and every [`Expr`](super::Expr) node carries a
//! [`Span`]. A span is a half-open `[start, end)` range made of two [`Pos`]
//! values. Each `Pos` stores the **byte offset** plus a 1-based **line** and
//! a 1-based, **char-based column**. Columns count Unicode scalar values, not
//! bytes, so `⍴` (3 bytes in UTF-8) moves the column by exactly 1.
//!
//! Positions are computed through a [`LineIndex`], a table of line-start
//! byte offsets built once per source string.

/// One position in a source string.
///
/// ```
/// use arrayglyph::printer::{LineIndex, Pos};
/// let idx = LineIndex::new("+/x\n⍴y");
/// assert_eq!(idx.pos(7), Pos { offset: 7, line: 2, col: 2 }); // the `y` after 3-byte `⍴`
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct Pos {
    /// Byte offset into the source (always on a char boundary).
    pub offset: usize,
    /// 1-based line number. Lines are separated by `\n`.
    pub line: usize,
    /// 1-based column, counted in chars (Unicode scalar values) from the
    /// start of the line.
    pub col: usize,
}

/// A half-open source range `[start, end)`.
///
/// ```
/// use arrayglyph::printer::Expr;
/// let e = Expr::parse_glyph("+/x").unwrap();
/// assert_eq!((e.span.start.offset, e.span.end.offset), (0, 3));
/// assert_eq!((e.span.start.col, e.span.end.col), (1, 4));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Span {
    /// First position covered.
    pub start: Pos,
    /// Position just past the last char covered.
    pub end: Pos,
}

impl Span {
    /// Builds a span from two positions.
    ///
    /// ```
    /// use arrayglyph::printer::{LineIndex, Span};
    /// let idx = LineIndex::new("abc");
    /// let s = Span::new(idx.pos(1), idx.pos(3));
    /// assert_eq!(s.len(), 2);
    /// ```
    pub const fn new(start: Pos, end: Pos) -> Self {
        Self { start, end }
    }

    /// The length in bytes.
    ///
    /// ```
    /// use arrayglyph::printer::Expr;
    /// assert_eq!(Expr::parse_glyph("⍳5").unwrap().span.len(), 4); // `⍳` is 3 bytes
    /// ```
    pub const fn len(&self) -> usize {
        self.end.offset - self.start.offset
    }

    /// Returns `true` if the span covers no bytes, as with an
    /// end-of-input error.
    ///
    /// ```
    /// use arrayglyph::printer::Expr;
    /// let err = Expr::parse_glyph("+/").unwrap_err();
    /// assert!(err.span.is_empty());
    /// ```
    pub const fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the covered text of `src`, which must be the source the span
    /// came from.
    ///
    /// ```
    /// use arrayglyph::printer::Expr;
    /// let src = "a +.× b";
    /// let e = Expr::parse_glyph(src).unwrap();
    /// assert_eq!(e.span.slice(src), "a +.× b");
    /// ```
    pub fn slice<'s>(&self, src: &'s str) -> &'s str {
        &src[self.start.offset..self.end.offset]
    }
}

/// A line-start table that maps byte offsets to [`Pos`] (line and column).
///
/// ```
/// use arrayglyph::printer::LineIndex;
/// let idx = LineIndex::new("ab\n⌈/c");
/// let p = idx.pos(6); // byte of `/` after 3-byte `⌈`
/// assert_eq!((p.line, p.col), (2, 2));
/// ```
#[derive(Debug, Clone)]
pub struct LineIndex<'s> {
    src: &'s str,
    line_starts: Vec<usize>,
}

impl<'s> LineIndex<'s> {
    /// Builds the table in one pass over `src`.
    ///
    /// ```
    /// use arrayglyph::printer::LineIndex;
    /// assert_eq!(LineIndex::new("a\nb\nc").line_count(), 3);
    /// ```
    pub fn new(src: &'s str) -> Self {
        let line_starts = std::iter::once(0)
            .chain(src.match_indices('\n').map(|(i, _)| i + 1))
            .collect();
        Self { src, line_starts }
    }

    /// The number of lines. Empty input counts as one line.
    ///
    /// ```
    /// use arrayglyph::printer::LineIndex;
    /// assert_eq!(LineIndex::new("").line_count(), 1);
    /// ```
    pub fn line_count(&self) -> usize {
        self.line_starts.len()
    }

    /// Converts a byte offset to a [`Pos`]. Offsets past the end clamp to
    /// `src.len()`. An offset inside a multi-byte char is rounded down to
    /// that char's start.
    ///
    /// ```
    /// use arrayglyph::printer::LineIndex;
    /// let idx = LineIndex::new("×y");
    /// assert_eq!(idx.pos(2).col, 2); // `×` is 2 bytes; `y` is column 2
    /// assert_eq!(idx.pos(1).offset, 0); // mid-char rounds down
    /// ```
    pub fn pos(&self, offset: usize) -> Pos {
        let mut offset = offset.min(self.src.len());
        while !self.src.is_char_boundary(offset) {
            offset -= 1;
        }
        let line_idx = self.line_starts.partition_point(|&s| s <= offset) - 1;
        let line_start = self.line_starts[line_idx];
        Pos {
            offset,
            line: line_idx + 1,
            col: self.src[line_start..offset].chars().count() + 1,
        }
    }

    /// Builds a [`Span`] from two byte offsets.
    ///
    /// ```
    /// use arrayglyph::printer::LineIndex;
    /// let s = LineIndex::new("x\n⍳3").span(2, 6);
    /// assert_eq!((s.start.line, s.start.col, s.end.col), (2, 1, 3));
    /// ```
    pub fn span(&self, start: usize, end: usize) -> Span {
        Span::new(self.pos(start), self.pos(end))
    }
}
