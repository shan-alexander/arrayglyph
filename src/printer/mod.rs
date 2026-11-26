//! Bidirectional pretty-printer between APL-style glyph notation and the
//! canonical verbose Rust that it stands for.
//!
//! * [`glyph_to_rust`] desugars glyph text into Rust method chains.
//! * [`rust_to_glyph`] compresses those canonical Rust forms, plus
//!   `reduce!`/`scan!` macro calls, back to glyphs.
//!
//! ```
//! use arrayglyph::printer::{glyph_to_rust, rust_to_glyph};
//!
//! let rust = glyph_to_rust("+/(a × b)").unwrap();
//! assert_eq!(rust, "a.zip_with(&b, Mul).reduce(Add)");
//! assert_eq!(rust_to_glyph(&rust).unwrap(), "+/(a × b)");
//!
//! // Rust-style precedence, evaluated left to right:
//! assert_eq!(glyph_to_rust("+/a × b").unwrap(), "a.reduce(Add).zip_with(&b, Mul)");
//! assert_eq!(
//!     glyph_to_rust("2 × 3 + x").unwrap(),
//!     "Array::splat(2).zip_with(&Array::splat(3), Mul).zip_with(&x, Add)"
//! );
//! ```
//!
//! Glyph text is read **left to right with Rust-style precedence**, not with
//! APL's right-to-left rule:
//!
//! * `× ÷ +.×` bind tighter than `+ -`, so `2 × 3 + x` is `(2 × 3) + x`.
//! * `⌈ ⌊` bind tighter still, like a method call: `a ⌈ b + c` is
//!   `(a ⌈ b) + c`.
//! * All infix operators are left-associative: `a - b - c` is `(a - b) - c`.
//! * Prefix forms (`f/`, `f\`, `⍴`, `⍉`, `r c⍴`, `⍳n`) bind tighter than any
//!   infix operator, like Rust's unary operators. So `+/a × b` is
//!   `(+/a) × b`, and the dot product is written `+/(a × b)`.
//! * `∘` composes functions explicitly: `(+/ ∘ ⍉) m` applies `⍉`, then `+/`,
//!   and becomes `compose(Reduce(Add), Transpose).call(&m)`. An applied
//!   composition binds like any other prefix form.
//!
//! Parentheses group. The glyph printer emits only the parentheses that this
//! table needs. See the crate's `DESIGN.md` §6 and §7 for the full glyph
//! table and evaluation rules.
//!
//! The Rust side is **not** a general Rust parser. It accepts exactly the
//! canonical forms that [`glyph_to_rust`] emits, and rejects anything else
//! with a [`ParseError`].

mod glyph;
mod rust;
mod span;

pub use span::{LineIndex, Pos, Span};

use core::fmt;

/// A dyadic scalar function in the printer's AST.
///
/// ```
/// use arrayglyph::printer::Func;
/// assert_eq!(Func::Mul.glyph(), '×');
/// assert_eq!(Func::Mul.rust_name(), "Mul");
/// assert_eq!(Func::from_rust_name("Max"), Some(Func::Max));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Func {
    /// `+` / `Add`
    Add,
    /// `-` / `Sub`
    Sub,
    /// `×` / `Mul`
    Mul,
    /// `÷` / `Div`
    Div,
    /// `⌈` / `Max`
    Max,
    /// `⌊` / `Min`
    Min,
}

impl Func {
    /// The canonical glyph.
    ///
    /// ```
    /// use arrayglyph::printer::Func;
    /// assert_eq!(Func::Min.glyph(), '⌊');
    /// ```
    pub const fn glyph(self) -> char {
        match self {
            Func::Add => '+',
            Func::Sub => '-',
            Func::Mul => '×',
            Func::Div => '÷',
            Func::Max => '⌈',
            Func::Min => '⌊',
        }
    }

    /// The name of the corresponding `arrayglyph::op` marker type.
    ///
    /// ```
    /// use arrayglyph::printer::Func;
    /// assert_eq!(Func::Div.rust_name(), "Div");
    /// ```
    pub const fn rust_name(self) -> &'static str {
        match self {
            Func::Add => "Add",
            Func::Sub => "Sub",
            Func::Mul => "Mul",
            Func::Div => "Div",
            Func::Max => "Max",
            Func::Min => "Min",
        }
    }

    /// Parses a glyph character. Both ASCII `-` and `−` (U+2212) map to
    /// [`Func::Sub`].
    ///
    /// ```
    /// use arrayglyph::printer::Func;
    /// assert_eq!(Func::from_glyph('⌈'), Some(Func::Max));
    /// assert_eq!(Func::from_glyph('−'), Some(Func::Sub));
    /// assert_eq!(Func::from_glyph('*'), None);
    /// ```
    pub const fn from_glyph(c: char) -> Option<Func> {
        Some(match c {
            '+' => Func::Add,
            '-' | '−' => Func::Sub,
            '×' => Func::Mul,
            '÷' => Func::Div,
            '⌈' => Func::Max,
            '⌊' => Func::Min,
            _ => return None,
        })
    }

    /// Parses an `op` marker name such as `"Add"`.
    ///
    /// ```
    /// use arrayglyph::printer::Func;
    /// assert_eq!(Func::from_rust_name("Sub"), Some(Func::Sub));
    /// assert_eq!(Func::from_rust_name("Pow"), None);
    /// ```
    pub fn from_rust_name(name: &str) -> Option<Func> {
        Some(match name {
            "Add" => Func::Add,
            "Sub" => Func::Sub,
            "Mul" => Func::Mul,
            "Div" => Func::Div,
            "Max" => Func::Max,
            "Min" => Func::Min,
            _ => return None,
        })
    }

    /// Parses the ASCII macro token used by `reduce!`/`scan!`
    /// (`+ - * / max min`).
    ///
    /// ```
    /// use arrayglyph::printer::Func;
    /// assert_eq!(Func::from_macro_token("*"), Some(Func::Mul));
    /// assert_eq!(Func::from_macro_token("max"), Some(Func::Max));
    /// ```
    pub fn from_macro_token(tok: &str) -> Option<Func> {
        Some(match tok {
            "+" => Func::Add,
            "-" => Func::Sub,
            "*" => Func::Mul,
            "/" => Func::Div,
            "max" => Func::Max,
            "min" => Func::Min,
            _ => return None,
        })
    }
}

/// A syntax-tree node: an [`ExprKind`] plus the [`Span`] of source text it
/// was parsed from.
///
/// Both notations parse into and print from this tree. Spans refer to
/// whichever source was parsed: glyph text for [`Expr::parse_glyph`], Rust
/// text for [`Expr::parse_rust`]. A parenthesised expression's span includes
/// its parentheses.
///
/// Numbers are stored as Rust literal text (for example `"-3"` or `"2.5"`).
/// The glyph printer renders a leading `-` as APL's high minus `¯`.
///
/// ```
/// use arrayglyph::printer::{Expr, ExprKind, Func};
///
/// let e = Expr::parse_glyph("+/x").unwrap();
/// let ExprKind::Reduce(Func::Add, inner) = &e.kind else { panic!() };
/// assert_eq!(inner.kind, ExprKind::Ident("x".into()));
/// assert_eq!(inner.span.start.col, 3);
/// assert_eq!(e.to_rust(), "x.reduce(Add)");
/// assert_eq!(e.to_glyph(), "+/x");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Expr {
    /// What the node is.
    pub kind: ExprKind,
    /// Where it came from in the parsed source.
    pub span: Span,
}

/// The node types of the printer's syntax tree.
///
/// ```
/// use arrayglyph::printer::{Expr, ExprKind};
/// let e = Expr::parse_glyph("⍳5").unwrap();
/// assert_eq!(e.kind, ExprKind::Iota(5));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ExprKind {
    /// A Rust variable holding an `Array`.
    Ident(String),
    /// A single number. With arrays it becomes `Array::splat(n)`.
    Scalar(String),
    /// A numeric strand `1 2 3`, which becomes `Array::vector([1, 2, 3])`. It
    /// always has at least 2 elements.
    Strand(Vec<String>),
    /// `⍳n`, which becomes `iota::<n>()`.
    Iota(usize),
    /// `⍴x`, which becomes `x.shape()`.
    Shape(Box<Expr>),
    /// `⍉x`, which becomes `x.transpose()`.
    Transpose(Box<Expr>),
    /// `r c⍴x` or `n⍴x`, which becomes `x.reshape::<r, c>()`. Holds 1 or 2 dims.
    Reshape(Vec<usize>, Box<Expr>),
    /// `a f b`, which becomes `a.zip_with(&b, F)`.
    Dyadic(Func, Box<Expr>, Box<Expr>),
    /// `f/x`, which becomes `x.reduce(F)`.
    Reduce(Func, Box<Expr>),
    /// `f\x`, which becomes `x.scan(F)`.
    Scan(Func, Box<Expr>),
    /// `a +.× b`, which becomes `a.matmul(&b)`.
    MatMul(Box<Expr>, Box<Expr>),
    /// `(f ∘ g) x`: applies a composed function, which becomes
    /// `compose(F, G).call(&x)`. Only used when the function contains `∘`;
    /// a lone function such as `(+/) x` is normalised to [`ExprKind::Reduce`].
    Apply(Train, Box<Expr>),
}

/// A function value in a train: an operand of `∘`, plus its [`Span`].
///
/// ```
/// use arrayglyph::printer::{Expr, ExprKind, TrainKind};
/// let e = Expr::parse_glyph("(+/ ∘ ⍉) m").unwrap();
/// let ExprKind::Apply(train, _) = &e.kind else { panic!() };
/// assert!(matches!(train.kind, TrainKind::Compose(..)));
/// // Like parenthesised expressions, a train's span includes its parentheses.
/// assert_eq!(train.span.slice("(+/ ∘ ⍉) m"), "(+/ ∘ ⍉)");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Train {
    /// What the function is.
    pub kind: TrainKind,
    /// Where it came from in the parsed source.
    pub span: Span,
}

/// The function forms that can appear in a train.
///
/// ```
/// use arrayglyph::printer::{Expr, ExprKind, Func, TrainKind};
/// let e = Expr::parse_glyph("(⌈\\ ∘ 2 3⍴) v").unwrap();
/// let ExprKind::Apply(t, _) = &e.kind else { panic!() };
/// let TrainKind::Compose(f, g) = &t.kind else { panic!() };
/// assert_eq!(f.kind, TrainKind::Scan(Func::Max));
/// assert_eq!(g.kind, TrainKind::Reshape(vec![2, 3]));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TrainKind {
    /// `f/`, which becomes `Reduce(F)`.
    Reduce(Func),
    /// `f\`, which becomes `Scan(F)`.
    Scan(Func),
    /// `⍴`, which becomes `Shape`.
    Shape,
    /// `⍉`, which becomes `Transpose`.
    Transpose,
    /// `r c⍴` or `n⍴`, which becomes `Reshape::<r, c>`.
    Reshape(Vec<usize>),
    /// `f ∘ g` (apply `g`, then `f`), which becomes `compose(f, g)`.
    Compose(Box<Train>, Box<Train>),
}

impl Expr {
    /// Builds a node from its kind and span.
    ///
    /// ```
    /// use arrayglyph::printer::{Expr, ExprKind, Span};
    /// let e = Expr::new(ExprKind::Iota(3), Span::default());
    /// assert_eq!(e.to_glyph(), "⍳3");
    /// ```
    pub const fn new(kind: ExprKind, span: Span) -> Self {
        Self { kind, span }
    }

    /// Parses glyph notation.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError`] for unknown glyphs, unsupported forms, or
    /// unbalanced parentheses.
    ///
    /// ```
    /// use arrayglyph::printer::Expr;
    /// assert!(Expr::parse_glyph("⌈\\v").is_ok());
    /// assert!(Expr::parse_glyph("a * b").is_err()); // APL × not *
    /// ```
    pub fn parse_glyph(src: &str) -> Result<Expr, ParseError> {
        glyph::parse(src)
    }

    /// Parses canonical verbose Rust, including `reduce!`/`scan!` calls.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError`] for anything outside the canonical subset.
    ///
    /// ```
    /// use arrayglyph::printer::Expr;
    /// let e = Expr::parse_rust("reduce!(max, v)").unwrap();
    /// assert_eq!(e.to_glyph(), "⌈/v");
    /// ```
    pub fn parse_rust(src: &str) -> Result<Expr, ParseError> {
        rust::parse(src)
    }

    /// Renders canonical glyph notation.
    ///
    /// ```
    /// use arrayglyph::printer::Expr;
    /// // Redundant parentheses and spacing are normalised away...
    /// let e = Expr::parse_glyph("( +/ x )  -  y").unwrap();
    /// assert_eq!(e.to_glyph(), "+/x - y");
    /// // ...but parentheses that change the meaning are kept.
    /// let e = Expr::parse_glyph("a - (b - c)").unwrap();
    /// assert_eq!(e.to_glyph(), "a - (b - c)");
    /// ```
    pub fn to_glyph(&self) -> String {
        glyph::print(self)
    }

    /// Renders canonical verbose Rust.
    ///
    /// ```
    /// use arrayglyph::printer::Expr;
    /// let e = Expr::parse_glyph("2 3⍴⍳6").unwrap();
    /// assert_eq!(e.to_rust(), "iota::<6>().reshape::<2, 3>()");
    /// ```
    pub fn to_rust(&self) -> String {
        rust::print(self)
    }
}

/// A parse failure, with the [`Span`] of the source where it was detected.
///
/// ```
/// use arrayglyph::printer::glyph_to_rust;
/// let err = glyph_to_rust("a +\n (b").unwrap_err();
/// assert!(err.message.contains("expected `)`"));
/// assert_eq!((err.span.start.line, err.span.start.col), (2, 4)); // end of input
/// assert_eq!(err.offset(), 7);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    /// Human-readable description.
    pub message: String,
    /// The offending token, or an empty span at the point where input ran out.
    pub span: Span,
}

impl ParseError {
    pub(crate) fn new(span: Span, message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            span,
        }
    }

    /// Shorthand for `self.span.start.offset`.
    ///
    /// ```
    /// use arrayglyph::printer::glyph_to_rust;
    /// assert_eq!(glyph_to_rust("a * b").unwrap_err().offset(), 2);
    /// ```
    pub const fn offset(&self) -> usize {
        self.span.start.offset
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} (at {}:{})",
            self.message, self.span.start.line, self.span.start.col
        )
    }
}

impl std::error::Error for ParseError {}

/// Desugars glyph notation into canonical verbose Rust.
///
/// # Errors
///
/// Returns [`ParseError`] if the glyph text is malformed or unsupported.
///
/// ```
/// use arrayglyph::printer::glyph_to_rust;
/// assert_eq!(glyph_to_rust("+\\⍳5").unwrap(), "iota::<5>().scan(Add)");
/// assert_eq!(glyph_to_rust("a +.× ⍉b").unwrap(), "a.matmul(&b.transpose())");
/// ```
pub fn glyph_to_rust(src: &str) -> Result<String, ParseError> {
    Ok(Expr::parse_glyph(src)?.to_rust())
}

/// Compresses canonical verbose Rust (as emitted by [`glyph_to_rust`]) back
/// into glyph notation.
///
/// # Errors
///
/// Returns [`ParseError`] if the input is outside the canonical subset.
///
/// ```
/// use arrayglyph::printer::rust_to_glyph;
/// assert_eq!(
///     rust_to_glyph("a.zip_with(&Array::splat(2), Mul).scan(Max)").unwrap(),
///     "⌈\\(a × 2)"
/// );
/// assert!(rust_to_glyph("for i in 0..n {}").is_err());
/// ```
pub fn rust_to_glyph(src: &str) -> Result<String, ParseError> {
    Ok(Expr::parse_rust(src)?.to_glyph())
}

#[cfg(test)]
mod span_tests {
    use super::*;

    fn lc(p: Pos) -> (usize, usize) {
        (p.line, p.col)
    }

    #[test]
    fn token_spans_on_multibyte_glyphs() {
        // `⌈` is 3 bytes, `×` is 2 bytes; columns count chars.
        let src = "⌈/a × b";
        let spans = glyph::token_spans(src).unwrap();
        let got: Vec<_> = spans
            .iter()
            .map(|s| (s.start.offset, s.end.offset, s.start.col, s.end.col))
            .collect();
        assert_eq!(
            got,
            vec![
                (0, 3, 1, 2),  // ⌈
                (3, 4, 2, 3),  // /
                (4, 5, 3, 4),  // a
                (6, 8, 5, 6),  // ×
                (9, 10, 7, 8), // b
            ]
        );
        for s in &spans {
            assert!(src.is_char_boundary(s.start.offset) && src.is_char_boundary(s.end.offset));
        }
        assert_eq!(spans[3].slice(src), "×");
    }

    #[test]
    fn inner_product_token_is_one_span() {
        let src = "a +.× b";
        let spans = glyph::token_spans(src).unwrap();
        assert_eq!(spans[1].slice(src), "+.×");
        assert_eq!((spans[1].start.col, spans[1].end.col), (3, 6));
    }

    #[test]
    fn ast_spans_on_multiline_input() {
        let src = "+/(a\n  × ⍳3)";
        let e = Expr::parse_glyph(src).unwrap();
        assert_eq!(e.span.slice(src), src);
        assert_eq!((lc(e.span.start), lc(e.span.end)), ((1, 1), (2, 8)));

        let ExprKind::Reduce(_, body) = &e.kind else {
            panic!("expected reduce")
        };
        assert_eq!(body.span.slice(src), "(a\n  × ⍳3)");
        let ExprKind::Dyadic(Func::Mul, a, iota) = &body.kind else {
            panic!("expected dyadic ×")
        };
        assert_eq!((lc(a.span.start), lc(a.span.end)), ((1, 4), (1, 5)));
        assert_eq!(iota.span.slice(src), "⍳3");
        assert_eq!((lc(iota.span.start), lc(iota.span.end)), ((2, 5), (2, 7)));
        assert_eq!(iota.span.start.offset, 10); // 5 (line 1) + 2 spaces + 3 bytes for ×

        // Without parentheses, prefix `+/` binds tighter than `×`.
        let e = Expr::parse_glyph("+/a\n  × ⍳3").unwrap();
        assert!(matches!(e.kind, ExprKind::Dyadic(Func::Mul, ..)));
    }

    #[test]
    fn paren_span_includes_parens() {
        let src = "(+/x) - y";
        let e = Expr::parse_glyph(src).unwrap();
        let ExprKind::Dyadic(_, l, r) = &e.kind else {
            panic!()
        };
        assert_eq!(l.span.slice(src), "(+/x)");
        assert_eq!(r.span.slice(src), "y");
    }

    #[test]
    fn strand_span_covers_all_numbers() {
        let src = "¯1 2  3";
        let e = Expr::parse_glyph(src).unwrap();
        assert_eq!(e.span.slice(src), src);
        assert_eq!(e.span.end.col, 8); // `¯` is 2 bytes but 1 column
    }

    #[test]
    fn error_spans_point_at_offending_token() {
        let err = Expr::parse_glyph("a +\n  b * c").unwrap_err();
        assert_eq!((err.span.start.line, err.span.start.col), (2, 5));
        assert_eq!(err.span.len(), 1);

        let err = Expr::parse_glyph("⍴⍴\n⍳x").unwrap_err();
        assert_eq!(err.span.slice("⍴⍴\n⍳x"), "x");
        assert_eq!((err.span.start.line, err.span.start.col), (2, 2));

        // End of input: an empty span at the very end.
        let src = "+/\n";
        let err = Expr::parse_glyph(src).unwrap_err();
        assert!(err.span.is_empty());
        assert_eq!((err.span.start.line, err.span.start.col), (2, 1));
    }

    #[test]
    fn train_spans_on_multibyte_multiline_input() {
        let src = "(⌈\\ ∘\n ⍉) m";
        let e = Expr::parse_glyph(src).unwrap();
        assert_eq!(e.span.slice(src), src);
        let ExprKind::Apply(t, x) = &e.kind else {
            panic!("expected a train application")
        };
        assert_eq!(t.span.slice(src), "(⌈\\ ∘\n ⍉)");
        let TrainKind::Compose(f, g) = &t.kind else {
            panic!("expected compose")
        };
        assert_eq!(f.span.slice(src), "⌈\\");
        assert_eq!((lc(f.span.start), lc(f.span.end)), ((1, 2), (1, 4)));
        assert_eq!(g.span.slice(src), "⍉");
        assert_eq!((lc(g.span.start), lc(g.span.end)), ((2, 2), (2, 3)));
        assert_eq!((lc(x.span.start), lc(x.span.end)), ((2, 5), (2, 6)));

        let err = Expr::parse_glyph("(+/ ∘\n) m").unwrap_err();
        assert_eq!((err.span.start.line, err.span.start.col), (2, 1));
    }

    #[test]
    fn rust_side_spans() {
        let src = "a.zip_with(\n  &b, Mul)\n .reduce(Add)";
        let e = Expr::parse_rust(src).unwrap();
        assert_eq!(e.span.slice(src), src);
        let ExprKind::Reduce(_, inner) = &e.kind else {
            panic!()
        };
        assert_eq!(inner.span.slice(src), "a.zip_with(\n  &b, Mul)");
        let ExprKind::Dyadic(_, _, b) = &inner.kind else {
            panic!()
        };
        assert_eq!((lc(b.span.start), lc(b.span.end)), ((2, 4), (2, 5)));

        let err = Expr::parse_rust("x\n  .reduce(Pow)").unwrap_err();
        assert_eq!(err.span.slice("x\n  .reduce(Pow)"), "Pow");
        assert_eq!((err.span.start.line, err.span.start.col), (2, 11));
    }

    #[test]
    fn line_index_maps_multibyte_columns() {
        let src = "⍳3\n÷/v\n";
        let idx = LineIndex::new(src);
        assert_eq!(idx.line_count(), 3);
        assert_eq!(lc(idx.pos(0)), (1, 1));
        assert_eq!(lc(idx.pos(3)), (1, 2)); // `3`
        assert_eq!(lc(idx.pos(5)), (2, 1)); // `÷`
        assert_eq!(lc(idx.pos(7)), (2, 2)); // `/` after 2-byte `÷`
        assert_eq!(lc(idx.pos(src.len())), (3, 1));
        assert_eq!(idx.pos(999).offset, src.len());
    }
}
