//! Bidirectional pretty-printer between APL-style glyph notation and the
//! canonical verbose Rust that it stands for.
//!
//! Glyphs are the primary notation; the Rust side is what they stand for.
//!
//! * [`glyph_to_rust`] desugars glyph text into Rust method chains
//!   ([`Style::Method`], the default): `+⌿m` is `m.sum::<0>()`.
//! * [`glyph_to_rust_with`] picks a [`Style`]. [`Style::Macro`] writes
//!   reductions and scans as macros with a bracketed axis:
//!   `reduce!(+[0], m)`.
//! * [`rust_to_glyph`] compresses either style (and the older spellings,
//!   such as `reduce!(+, m, axis = 1)`, `x.reduce_axis::<1>(Add)` and
//!   `compose(…)`) back to glyphs.
//!
//! Both styles **always write the axis**, so the result shape is visible.
//!
//! ```
//! use arrayglyph::printer::{glyph_to_rust, glyph_to_rust_with, rust_to_glyph, Style};
//!
//! let rust = glyph_to_rust("+/(a × b)").unwrap();
//! assert_eq!(rust, "a.zip_with(&b, Mul).sum::<1>()");
//! assert_eq!(rust_to_glyph(&rust).unwrap(), "+/(a × b)");
//! assert_eq!(
//!     glyph_to_rust_with("+/(a × b)", Style::Macro).unwrap(),
//!     "reduce!(+[1], a.zip_with(&b, Mul))"
//! );
//!
//! // Rust-style precedence, evaluated left to right:
//! assert_eq!(glyph_to_rust("+/a × b").unwrap(), "a.sum::<1>().zip_with(&b, Mul)");
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
//! * Prefix forms (`f/`, `f⌿`, `f\`, `f⍀`, `⍴`, `⍉`, `r c⍴`, `⍳n`) bind tighter than any
//!   infix operator, like Rust's unary operators. So `+/a × b` is
//!   `(+/a) × b`, and the dot product is written `+/(a × b)`.
//! * `∘` composes functions explicitly: `(+/ ∘ ⍉) m` applies `⍉`, then `+/`,
//!   and becomes the flat train `train![reduce(+[1]), transpose].call(&m)`
//!   (same order as the glyph; the rightmost runs first). `∘` is
//!   associative, so `(f ∘ (g ∘ h))` prints flat as `(f ∘ g ∘ h)`. An
//!   applied composition binds like any other prefix form.
//!
//! * Reductions and scans name their axis APL-style: `f⌿` and `f⍀` work
//!   along **axis 0** (down each column: `x.sum::<0>()`), `f/` and `f\`
//!   along **axis 1** (along each row: `x.sum::<1>()`). The bracket forms
//!   `f/[0]`, `f/[1]`, `f\[0]` and `f\[1]` are accepted as input and
//!   printed in the canonical form.
//!
//! | glyph | [`Style::Method`] (default) | [`Style::Macro`] |
//! |---|---|---|
//! | `+⌿m` / `+/m` | `m.sum::<0>()` / `m.sum::<1>()` | `reduce!(+[0], m)` / `reduce!(+[1], m)` |
//! | `×⌿m`, `⌈⌿m`, `⌊⌿m` | `m.prod::<0>()`, `m.max::<0>()`, `m.min::<0>()` | `reduce!(*[0], m)`, `reduce!(max[0], m)`, `reduce!(min[0], m)` |
//! | `-⌿m`, `÷⌿m` | `m.reduce_axis::<0>(Sub)`, `m.reduce_axis::<0>(Div)` | `reduce!(-[0], m)`, `reduce!(/[0], m)` |
//! | `+⍀m`, `×\m` | `m.cumsum::<0>()`, `m.cumprod::<1>()` | `scan!(+[0], m)`, `scan!(*[1], m)` |
//! | `⌈⍀m`, `⌊\m` | `m.cummax::<0>()`, `m.cummin::<1>()` | `scan!(max[0], m)`, `scan!(min[1], m)` |
//! | `-⍀m`, `÷\m` | `m.scan_axis::<0>(Sub)`, `m.scan_axis::<1>(Div)` | `scan!(-[0], m)`, `scan!(/[1], m)` |
//! | `(+/ ∘ ⍉) m` | `train![reduce(+[1]), transpose].call(&m)` | same |
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

/// Which Rust spelling [`glyph_to_rust_with`] and [`Expr::to_rust_styled`]
/// emit for reductions and scans. Everything else (`zip_with`, `matmul`,
/// `transpose`, `reshape`, `train![…]`) is the same in both styles, and
/// both always write the axis.
///
/// ```
/// use arrayglyph::printer::{glyph_to_rust_with, Style};
/// assert_eq!(glyph_to_rust_with("×\\m", Style::Method).unwrap(), "m.cumprod::<1>()");
/// assert_eq!(glyph_to_rust_with("×\\m", Style::Macro).unwrap(), "scan!(*[1], m)");
/// assert_eq!(Style::default(), Style::Method);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Style {
    /// Named methods with a const-generic axis: `m.sum::<0>()`,
    /// `m.cumprod::<1>()`. `-` and `÷` have no named method, so they use
    /// `m.reduce_axis::<K>(Sub)` / `m.scan_axis::<K>(Div)`. The default.
    #[default]
    Method,
    /// Macros with a bracketed axis: `reduce!(+[0], m)`, `scan!(*[1], m)`.
    Macro,
}

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

    /// The ASCII token used by `reduce!`/`scan!`/`train!`: one of
    /// `+ - * / max min`.
    ///
    /// ```
    /// use arrayglyph::printer::Func;
    /// assert_eq!(Func::Mul.macro_token(), "*");
    /// assert_eq!(Func::from_macro_token(Func::Max.macro_token()), Some(Func::Max));
    /// ```
    pub const fn macro_token(self) -> &'static str {
        match self {
            Func::Add => "+",
            Func::Sub => "-",
            Func::Mul => "*",
            Func::Div => "/",
            Func::Max => "max",
            Func::Min => "min",
        }
    }

    /// The named reduction method (`sum`, `prod`, `max`, `min`), if this
    /// function has one. `Sub` and `Div` don't.
    ///
    /// ```
    /// use arrayglyph::printer::Func;
    /// assert_eq!(Func::Add.reduce_method(), Some("sum"));
    /// assert_eq!(Func::Div.reduce_method(), None);
    /// ```
    pub const fn reduce_method(self) -> Option<&'static str> {
        match self {
            Func::Add => Some("sum"),
            Func::Mul => Some("prod"),
            Func::Max => Some("max"),
            Func::Min => Some("min"),
            Func::Sub | Func::Div => None,
        }
    }

    /// The named scan method (`cumsum`, `cumprod`, `cummax`, `cummin`), if
    /// this function has one. `Sub` and `Div` don't.
    ///
    /// ```
    /// use arrayglyph::printer::Func;
    /// assert_eq!(Func::Max.scan_method(), Some("cummax"));
    /// assert_eq!(Func::Sub.scan_method(), None);
    /// ```
    pub const fn scan_method(self) -> Option<&'static str> {
        match self {
            Func::Add => Some("cumsum"),
            Func::Mul => Some("cumprod"),
            Func::Max => Some("cummax"),
            Func::Min => Some("cummin"),
            Func::Sub | Func::Div => None,
        }
    }

    /// Parses a named method: returns the function and `true` for a
    /// reduction (`sum`…), `false` for a scan (`cumsum`…).
    ///
    /// ```
    /// use arrayglyph::printer::Func;
    /// assert_eq!(Func::from_method("prod"), Some((Func::Mul, true)));
    /// assert_eq!(Func::from_method("cummin"), Some((Func::Min, false)));
    /// assert_eq!(Func::from_method("mean"), None);
    /// ```
    pub fn from_method(name: &str) -> Option<(Func, bool)> {
        Some(match name {
            "sum" => (Func::Add, true),
            "prod" => (Func::Mul, true),
            "max" => (Func::Max, true),
            "min" => (Func::Min, true),
            "cumsum" => (Func::Add, false),
            "cumprod" => (Func::Mul, false),
            "cummax" => (Func::Max, false),
            "cummin" => (Func::Min, false),
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
/// let ExprKind::Reduce(Func::Add, 1, inner) = &e.kind else { panic!() };
/// assert_eq!(inner.kind, ExprKind::Ident("x".into()));
/// assert_eq!(inner.span.start.col, 3);
/// assert_eq!(e.to_rust(), "x.sum::<1>()");
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
    /// A reduction along an axis (`0` or `1`). `+⌿x` (axis 0, down the
    /// columns) becomes `x.sum::<0>()` (or `reduce!(+[0], x)` in macro
    /// style); `+/x` (axis 1, along the rows) becomes `x.sum::<1>()`.
    /// `f/[k]x` is accepted as input.
    Reduce(Func, usize, Box<Expr>),
    /// A scan along an axis. `+⍀x` (axis 0) becomes `x.cumsum::<0>()`;
    /// `+\x` (axis 1) becomes `x.cumsum::<1>()` (macro style:
    /// `scan!(+[1], x)`).
    Scan(Func, usize, Box<Expr>),
    /// `a +.× b`, which becomes `a.matmul(&b)`.
    MatMul(Box<Expr>, Box<Expr>),
    /// `(f ∘ g) x`: applies a composed function, which becomes
    /// `train![f, g].call(&x)`. Only used when the function contains `∘`;
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
/// assert_eq!(f.kind, TrainKind::Scan(Func::Max, 1));
/// assert_eq!(g.kind, TrainKind::Reshape(vec![2, 3]));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TrainKind {
    /// `f⌿` (axis 0) or `f/` (axis 1), which becomes the train item
    /// `reduce(op[k])` (the value `Reduce(F).axis::<k>()`).
    Reduce(Func, usize),
    /// `f⍀` (axis 0) or `f\` (axis 1), which becomes `scan(op[k])`.
    Scan(Func, usize),
    /// `⍴`, which becomes `shape`.
    Shape,
    /// `⍉`, which becomes `transpose`.
    Transpose,
    /// `r c⍴` or `n⍴`, which becomes `reshape(r, c)`.
    Reshape(Vec<usize>),
    /// `f ∘ g` (apply `g`, then `f`): the items of `f` followed by the
    /// items of `g` in one flat `train![…]`.
    Compose(Box<Train>, Box<Train>),
}

impl Train {
    /// The operands of a `∘` chain, left to right, whatever the grouping:
    /// `(f ∘ (g ∘ h))` and `((f ∘ g) ∘ h)` both give `[f, g, h]`. A
    /// non-composition is a one-element list.
    ///
    /// ```
    /// use arrayglyph::printer::{Expr, ExprKind, TrainKind};
    /// let e = Expr::parse_glyph("(+/ ∘ (⍉ ∘ 2 3⍴)) v").unwrap();
    /// let ExprKind::Apply(t, _) = &e.kind else { panic!() };
    /// let kinds: Vec<_> = t.parts().into_iter().map(|p| p.kind.clone()).collect();
    /// assert_eq!(kinds[1], TrainKind::Transpose);
    /// assert_eq!(kinds.len(), 3);
    /// ```
    pub fn parts(&self) -> Vec<&Train> {
        match &self.kind {
            TrainKind::Compose(f, g) => {
                let mut v = f.parts();
                v.extend(g.parts());
                v
            }
            _ => vec![self],
        }
    }
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

    /// Parses canonical verbose Rust in either [`Style`], plus the older
    /// spellings (`reduce!(op, x, axis = k)`, `reduce!(op, x)`,
    /// `x.reduce(F)`, `x.reduce_axis::<k>(F)`, `compose(…)`).
    ///
    /// # Errors
    ///
    /// Returns [`ParseError`] for anything outside the canonical subset.
    ///
    /// ```
    /// use arrayglyph::printer::Expr;
    /// let e = Expr::parse_rust("reduce!(max[1], v)").unwrap();
    /// assert_eq!(e.to_glyph(), "⌈/v");
    /// assert_eq!(Expr::parse_rust("v.max::<1>()").unwrap().to_glyph(), "⌈/v");
    /// // compatibility forms
    /// assert_eq!(Expr::parse_rust("reduce!(max, v, axis = 1)").unwrap().to_glyph(), "⌈/v");
    /// assert_eq!(Expr::parse_rust("reduce!(max, m)").unwrap().to_glyph(), "⌈⌿m"); // axis 0
    /// // named methods must write their axis
    /// assert!(Expr::parse_rust("m.sum()").is_err());
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

    /// Renders canonical verbose Rust in the default [`Style::Method`].
    ///
    /// ```
    /// use arrayglyph::printer::Expr;
    /// let e = Expr::parse_glyph("2 3⍴⍳6").unwrap();
    /// assert_eq!(e.to_rust(), "iota::<6>().reshape::<2, 3>()");
    /// let e = Expr::parse_glyph("+⌿2 3⍴⍳6").unwrap();
    /// assert_eq!(e.to_rust(), "iota::<6>().reshape::<2, 3>().sum::<0>()");
    /// ```
    pub fn to_rust(&self) -> String {
        self.to_rust_styled(Style::Method)
    }

    /// Renders canonical verbose Rust in the given [`Style`].
    ///
    /// ```
    /// use arrayglyph::printer::{Expr, Style};
    /// let e = Expr::parse_glyph("⌈/(+⍀m)").unwrap();
    /// assert_eq!(e.to_rust_styled(Style::Method), "m.cumsum::<0>().max::<1>()");
    /// assert_eq!(e.to_rust_styled(Style::Macro), "reduce!(max[1], scan!(+[0], m))");
    /// ```
    pub fn to_rust_styled(&self, style: Style) -> String {
        rust::print(self, style)
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

/// Desugars glyph notation into canonical verbose Rust, in the default
/// [`Style::Method`].
///
/// # Errors
///
/// Returns [`ParseError`] if the glyph text is malformed or unsupported.
///
/// ```
/// use arrayglyph::printer::glyph_to_rust;
/// assert_eq!(glyph_to_rust("+\\⍳5").unwrap(), "iota::<5>().cumsum::<1>()");
/// assert_eq!(glyph_to_rust("-⌿m").unwrap(), "m.reduce_axis::<0>(Sub)");
/// assert_eq!(glyph_to_rust("a +.× ⍉b").unwrap(), "a.matmul(&b.transpose())");
/// assert_eq!(
///     glyph_to_rust("(+/ ∘ ⍉ ∘ 2 3⍴) v").unwrap(),
///     "train![reduce(+[1]), transpose, reshape(2, 3)].call(&v)"
/// );
/// ```
pub fn glyph_to_rust(src: &str) -> Result<String, ParseError> {
    glyph_to_rust_with(src, Style::Method)
}

/// Desugars glyph notation into canonical verbose Rust in the given
/// [`Style`].
///
/// # Errors
///
/// Returns [`ParseError`] if the glyph text is malformed or unsupported.
///
/// ```
/// use arrayglyph::printer::{glyph_to_rust_with, Style};
/// assert_eq!(glyph_to_rust_with("+/+⌿m", Style::Macro).unwrap(), "reduce!(+[1], reduce!(+[0], m))");
/// assert_eq!(glyph_to_rust_with("+/+⌿m", Style::Method).unwrap(), "m.sum::<0>().sum::<1>()");
/// ```
pub fn glyph_to_rust_with(src: &str, style: Style) -> Result<String, ParseError> {
    Ok(Expr::parse_glyph(src)?.to_rust_styled(style))
}

/// Compresses canonical verbose Rust (as emitted by [`glyph_to_rust`] or
/// [`glyph_to_rust_with`], in either style) back into glyph notation.
///
/// # Errors
///
/// Returns [`ParseError`] if the input is outside the canonical subset.
///
/// ```
/// use arrayglyph::printer::rust_to_glyph;
/// assert_eq!(
///     rust_to_glyph("a.zip_with(&Array::splat(2), Mul).cummax::<1>()").unwrap(),
///     "⌈\\(a × 2)"
/// );
/// assert_eq!(rust_to_glyph("scan!(max[1], a.zip_with(&Array::splat(2), Mul))").unwrap(), "⌈\\(a × 2)");
/// assert_eq!(rust_to_glyph("train![reduce(+[1]), transpose].call(&m)").unwrap(), "(+/ ∘ ⍉) m");
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

        let ExprKind::Reduce(_, _, body) = &e.kind else {
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
    fn axis_operator_spans_cover_the_bracket() {
        let src = "+/[0] m\n + ⌈⍀ m";
        let spans = glyph::token_spans(src).unwrap();
        let texts: Vec<&str> = spans.iter().map(|s| s.slice(src)).collect();
        assert_eq!(texts, ["+", "/[0]", "m", "+", "⌈", "⍀", "m"]);
        let err = Expr::parse_glyph("x +\n +/[7] m").unwrap_err();
        assert_eq!(err.span.slice("x +\n +/[7] m"), "[7]");
        assert_eq!(lc(err.span.start), (2, 4));
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
        let src = "a.zip_with(\n  &b, Mul)\n .sum::<1>()";
        let e = Expr::parse_rust(src).unwrap();
        assert_eq!(e.span.slice(src), src);
        let ExprKind::Reduce(_, _, inner) = &e.kind else {
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
