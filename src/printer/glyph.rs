//! Glyph notation: lexer, left-to-right precedence parser, and canonical
//! printer.
//!
//! Binding strength, loosest first (see [`Prec`]):
//!
//! | level | forms | associativity |
//! |---|---|---|
//! | `Sum` | `a + b`, `a - b` | left |
//! | `Product` | `a × b`, `a ÷ b`, `a +.× b` | left |
//! | `MaxMin` | `a ⌈ b`, `a ⌊ b` | left |
//! | `Prefix` | `f/x`, `f\x`, `⍴x`, `⍉x`, `r c⍴x`, `⍳n` | nests to the right |
//! | `Atom` | `x`, `3`, `1 2 3`, `( … )` | — |

use super::{Expr, ExprKind, Func, LineIndex, ParseError, Span};

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Ident(String),
    /// Number in Rust literal form (`-3`, `2.5`).
    Num(String),
    Func(Func),
    Slash,
    Backslash,
    Iota,
    Rho,
    Transpose,
    /// `+.×`
    PlusTimes,
    LParen,
    RParen,
}

fn lex(src: &str, idx: &LineIndex<'_>) -> Result<Vec<(Span, Tok)>, ParseError> {
    let chars: Vec<(usize, char)> = src.char_indices().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let (pos, c) = chars[i];
        let peek = |k: usize| chars.get(i + k).map(|&(_, c)| c);
        // Byte offset just past char index `j` (exclusive end).
        let end_of = |j: usize| chars.get(j).map_or(src.len(), |&(o, _)| o);
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        if c.is_ascii_alphabetic() || c == '_' {
            let start = i;
            while i < chars.len() && (chars[i].1.is_ascii_alphanumeric() || chars[i].1 == '_') {
                i += 1;
            }
            let s: String = chars[start..i].iter().map(|&(_, c)| c).collect();
            out.push((idx.span(pos, end_of(i)), Tok::Ident(s)));
            continue;
        }
        if c.is_ascii_digit() || (c == '¯' && peek(1).is_some_and(|d| d.is_ascii_digit())) {
            let mut s = String::new();
            if c == '¯' {
                s.push('-');
                i += 1;
            }
            while i < chars.len() && chars[i].1.is_ascii_digit() {
                s.push(chars[i].1);
                i += 1;
            }
            if i + 1 < chars.len() && chars[i].1 == '.' && chars[i + 1].1.is_ascii_digit() {
                s.push('.');
                i += 1;
                while i < chars.len() && chars[i].1.is_ascii_digit() {
                    s.push(chars[i].1);
                    i += 1;
                }
            }
            out.push((idx.span(pos, end_of(i)), Tok::Num(s)));
            continue;
        }
        if c == '+' && peek(1) == Some('.') && peek(2) == Some('×') {
            i += 3;
            out.push((idx.span(pos, end_of(i)), Tok::PlusTimes));
            continue;
        }
        let tok = match c {
            '/' => Tok::Slash,
            '\\' => Tok::Backslash,
            '⍳' => Tok::Iota,
            '⍴' => Tok::Rho,
            '⍉' => Tok::Transpose,
            '(' => Tok::LParen,
            ')' => Tok::RParen,
            _ => match Func::from_glyph(c) {
                Some(f) => Tok::Func(f),
                None => {
                    let hint = match c {
                        '*' => " (multiply is `×`; `*` is APL power, unsupported)",
                        '.' => " (only the inner product `+.×` is supported)",
                        '¯' => " (`¯` must be followed by digits)",
                        _ => "",
                    };
                    return Err(ParseError::new(
                        idx.span(pos, end_of(i + 1)),
                        format!("unsupported glyph `{c}`{hint}"),
                    ));
                }
            },
        };
        i += 1;
        out.push((idx.span(pos, end_of(i)), tok));
    }
    Ok(out)
}

/// How tightly a glyph form binds, loosest first. Mirrors Rust: products
/// bind tighter than sums, and prefix forms bind like Rust's unary operators
/// and method calls (tighter than any infix operator).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Prec {
    /// `+ -`
    Sum,
    /// `× ÷ +.×`
    Product,
    /// `⌈ ⌊`
    MaxMin,
    /// `f/x f\x ⍴x ⍉x r c⍴x ⍳n`
    Prefix,
    /// identifiers, numbers, strands, parenthesised expressions
    Atom,
}

impl Prec {
    fn tighter(self) -> Prec {
        match self {
            Prec::Sum => Prec::Product,
            Prec::Product => Prec::MaxMin,
            Prec::MaxMin => Prec::Prefix,
            Prec::Prefix | Prec::Atom => Prec::Atom,
        }
    }
}

fn infix_prec(t: &Tok) -> Option<Prec> {
    match t {
        Tok::Func(Func::Add | Func::Sub) => Some(Prec::Sum),
        Tok::Func(Func::Mul | Func::Div) | Tok::PlusTimes => Some(Prec::Product),
        Tok::Func(Func::Max | Func::Min) => Some(Prec::MaxMin),
        _ => None,
    }
}

/// A monadic function in prefix position.
#[derive(Debug, Clone, PartialEq)]
enum FnAtom {
    Reduce(Func),
    Scan(Func),
    Shape,
    Transpose,
    Reshape(Vec<usize>),
}

impl FnAtom {
    fn apply(self, x: Expr) -> ExprKind {
        let x = Box::new(x);
        match self {
            FnAtom::Reduce(f) => ExprKind::Reduce(f, x),
            FnAtom::Scan(f) => ExprKind::Scan(f, x),
            FnAtom::Shape => ExprKind::Shape(x),
            FnAtom::Transpose => ExprKind::Transpose(x),
            FnAtom::Reshape(d) => ExprKind::Reshape(d, x),
        }
    }
}

struct Parser {
    toks: Vec<(Span, Tok)>,
    pos: usize,
    /// Empty span at end of input.
    eof: Span,
}

fn node(kind: ExprKind, start: Span, end: Span) -> Expr {
    Expr::new(kind, Span::new(start.start, end.end))
}

impl Parser {
    fn peek(&self) -> Option<&Tok> {
        self.peek_at(0)
    }
    fn peek_at(&self, k: usize) -> Option<&Tok> {
        self.toks.get(self.pos + k).map(|(_, t)| t)
    }
    /// Span of the next token, or the empty end-of-input span.
    fn cur(&self) -> Span {
        self.toks.get(self.pos).map_or(self.eof, |&(s, _)| s)
    }
    /// Span of the most recently consumed token.
    fn prev(&self) -> Span {
        self.pos
            .checked_sub(1)
            .and_then(|p| self.toks.get(p))
            .map_or(self.eof, |&(s, _)| s)
    }
    fn bump(&mut self) -> Option<(Span, Tok)> {
        let t = self.toks.get(self.pos).cloned();
        self.pos += 1;
        t
    }

    fn expr(&mut self) -> Result<Expr, ParseError> {
        self.binary(Prec::Sum)
    }

    /// Precedence climbing: `level := tighter (op_at_level tighter)*`, folded
    /// to the left, so `a - b - c` is `(a - b) - c`.
    fn binary(&mut self, level: Prec) -> Result<Expr, ParseError> {
        if level >= Prec::Prefix {
            return self.prefix();
        }
        let mut left = self.binary(level.tighter())?;
        while let Some(tok) = self.peek() {
            if infix_prec(tok) != Some(level) {
                break;
            }
            if matches!(self.peek_at(1), Some(Tok::Slash | Tok::Backslash)) {
                return Err(ParseError::new(
                    self.cur(),
                    "a left argument to `f/` or `f\\` (n-wise reduce) is unsupported; \
                     for a reduction on the right, write e.g. `a + +/b`",
                ));
            }
            let (_, tok) = self.bump().expect("peeked");
            let right = self.binary(level.tighter())?;
            let (ls, rs) = (left.span, right.span);
            let (l, r) = (Box::new(left), Box::new(right));
            let kind = match tok {
                Tok::PlusTimes => ExprKind::MatMul(l, r),
                Tok::Func(f) => ExprKind::Dyadic(f, l, r),
                _ => unreachable!("infix_prec only accepts functions"),
            };
            left = node(kind, ls, rs);
        }
        Ok(left)
    }

    /// Tries to read a monadic function at the cursor. Returns `None`, and
    /// consumes nothing, if the next tokens start an ordinary operand.
    fn fn_atom(&mut self) -> Result<Option<(FnAtom, Span)>, ParseError> {
        let at = self.cur();
        let atom = match self.peek() {
            Some(Tok::Func(f)) => {
                let f = *f;
                let atom = match self.peek_at(1) {
                    Some(Tok::Slash) => FnAtom::Reduce(f),
                    Some(Tok::Backslash) => FnAtom::Scan(f),
                    _ => {
                        return Err(ParseError::new(
                            at,
                            format!(
                                "monadic `{}` is unsupported (expected `{0}/` or `{0}\\`)",
                                f.glyph()
                            ),
                        ));
                    }
                };
                self.pos += 2;
                atom
            }
            Some(Tok::Rho) => {
                self.pos += 1;
                FnAtom::Shape
            }
            Some(Tok::Transpose) => {
                self.pos += 1;
                FnAtom::Transpose
            }
            Some(Tok::Num(_)) => {
                // `r c⍴`: a literal shape directly followed by `⍴`.
                let mut k = 0;
                while let Some(Tok::Num(_)) = self.peek_at(k) {
                    k += 1;
                }
                if self.peek_at(k) != Some(&Tok::Rho) {
                    return Ok(None);
                }
                let dims_span = Span::new(at.start, self.toks[self.pos + k - 1].0.end);
                if k > 2 {
                    return Err(ParseError::new(
                        dims_span,
                        "reshape supports rank 1 or 2 only",
                    ));
                }
                let mut dims = Vec::with_capacity(k);
                for j in 0..k {
                    let (span, Tok::Num(d)) = &self.toks[self.pos + j] else {
                        unreachable!("counted Num tokens")
                    };
                    dims.push(d.parse::<usize>().map_err(|_| {
                        ParseError::new(*span, format!("shape `{d}` is not a non-negative integer"))
                    })?);
                }
                self.pos += k + 1;
                FnAtom::Reshape(dims)
            }
            _ => return Ok(None),
        };
        Ok(Some((atom, Span::new(at.start, self.prev().end))))
    }

    /// prefix := fn_atom prefix | `⍳` NUM | primary
    fn prefix(&mut self) -> Result<Expr, ParseError> {
        let at = self.cur();
        if let Some((f, fspan)) = self.fn_atom()? {
            let x = self.prefix()?;
            let xs = x.span;
            return Ok(node(f.apply(x), fspan, xs));
        }
        if self.peek() == Some(&Tok::Iota) {
            self.bump();
            let arg = self.cur();
            return match (self.bump(), self.peek()) {
                (Some((_, Tok::Num(_))), Some(Tok::Num(_))) => {
                    Err(ParseError::new(self.cur(), "`⍳` takes a single length"))
                }
                (Some((_, Tok::Num(n))), _) => match n.parse::<usize>() {
                    Ok(n) => Ok(node(ExprKind::Iota(n), at, arg)),
                    Err(_) => Err(ParseError::new(
                        arg,
                        format!("`⍳` needs a non-negative integer, got `{n}`"),
                    )),
                },
                _ => Err(ParseError::new(
                    arg,
                    "`⍳` needs a literal length, like `⍳5`",
                )),
            };
        }
        self.primary()
    }

    /// primary := ident | number+ | `(` expr `)`
    fn primary(&mut self) -> Result<Expr, ParseError> {
        let at = self.cur();
        let Some((_, tok)) = self.bump() else {
            return Err(ParseError::new(at, "unexpected end of input"));
        };
        match tok {
            Tok::LParen => {
                let mut e = self.expr()?;
                let close = self.cur();
                match self.bump() {
                    Some((_, Tok::RParen)) => {
                        e.span = Span::new(at.start, close.end);
                        Ok(e)
                    }
                    _ => Err(ParseError::new(close, "expected `)`")),
                }
            }
            Tok::Num(n) => {
                let mut nums = vec![n];
                while let Some(Tok::Num(_)) = self.peek() {
                    if let Some((_, Tok::Num(n))) = self.bump() {
                        nums.push(n);
                    }
                }
                let kind = if nums.len() == 1 {
                    ExprKind::Scalar(nums.pop().unwrap_or_default())
                } else {
                    ExprKind::Strand(nums)
                };
                Ok(node(kind, at, self.prev()))
            }
            Tok::Ident(s) => Ok(Expr::new(ExprKind::Ident(s), at)),
            Tok::RParen => Err(ParseError::new(at, "unexpected `)`")),
            t => Err(ParseError::new(at, format!("unexpected {t:?}"))),
        }
    }
}

pub(super) fn parse(src: &str) -> Result<Expr, ParseError> {
    let idx = LineIndex::new(src);
    let toks = lex(src, &idx)?;
    let mut p = Parser {
        toks,
        pos: 0,
        eof: idx.span(src.len(), src.len()),
    };
    let e = p.expr()?;
    if p.pos < p.toks.len() {
        return Err(ParseError::new(p.cur(), "unexpected trailing input"));
    }
    Ok(e)
}

/// Lexes glyph text and returns only the token spans. Exposed to the parent
/// module's tests so that token spans can be checked directly.
#[cfg(test)]
pub(super) fn token_spans(src: &str) -> Result<Vec<Span>, ParseError> {
    let idx = LineIndex::new(src);
    Ok(lex(src, &idx)?.into_iter().map(|(s, _)| s).collect())
}

fn num(n: &str) -> String {
    match n.strip_prefix('-') {
        Some(rest) => format!("¯{rest}"),
        None => n.to_string(),
    }
}

/// The binding strength of an already-built node.
pub(super) fn prec(e: &Expr) -> Prec {
    match &e.kind {
        ExprKind::Dyadic(Func::Add | Func::Sub, ..) => Prec::Sum,
        ExprKind::Dyadic(Func::Mul | Func::Div, ..) | ExprKind::MatMul(..) => Prec::Product,
        ExprKind::Dyadic(Func::Max | Func::Min, ..) => Prec::MaxMin,
        ExprKind::Reduce(..)
        | ExprKind::Scan(..)
        | ExprKind::Shape(_)
        | ExprKind::Transpose(_)
        | ExprKind::Reshape(..)
        | ExprKind::Iota(_) => Prec::Prefix,
        ExprKind::Ident(_) | ExprKind::Scalar(_) | ExprKind::Strand(_) => Prec::Atom,
    }
}

/// Prints `e`, parenthesised if it binds looser than `min`.
fn at(e: &Expr, min: Prec) -> String {
    let s = print(e);
    if prec(e) < min { format!("({s})") } else { s }
}

pub(super) fn print(e: &Expr) -> String {
    let p = prec(e);
    match &e.kind {
        ExprKind::Ident(s) => s.clone(),
        ExprKind::Scalar(n) => num(n),
        ExprKind::Strand(ns) => ns.iter().map(|n| num(n)).collect::<Vec<_>>().join(" "),
        ExprKind::Iota(n) => format!("⍳{n}"),
        ExprKind::Shape(x) => format!("⍴{}", at(x, Prec::Prefix)),
        ExprKind::Transpose(x) => format!("⍉{}", at(x, Prec::Prefix)),
        ExprKind::Reshape(dims, x) => {
            let d: Vec<String> = dims.iter().map(ToString::to_string).collect();
            format!("{}⍴{}", d.join(" "), at(x, Prec::Prefix))
        }
        // Left-associative: the left operand may sit at the same level, the
        // right operand must bind strictly tighter.
        ExprKind::Dyadic(f, a, b) => {
            format!("{} {} {}", at(a, p), f.glyph(), at(b, p.tighter()))
        }
        ExprKind::MatMul(a, b) => format!("{} +.× {}", at(a, p), at(b, p.tighter())),
        ExprKind::Reduce(f, x) => format!("{}/{}", f.glyph(), at(x, Prec::Prefix)),
        ExprKind::Scan(f, x) => format!("{}\\{}", f.glyph(), at(x, Prec::Prefix)),
    }
}
