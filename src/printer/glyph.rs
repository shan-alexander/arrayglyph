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
//! | `Prefix` | `f/x`, `f⌿x`, `f\x`, `f⍀x`, `⍴x`, `⍉x`, `r c⍴x`, `⍳n`, `(f ∘ g) x` | nests to the right |
//! | `Atom` | `x`, `3`, `1 2 3`, `( … )` | — |

use super::{Expr, ExprKind, Func, LineIndex, ParseError, Span, Train, TrainKind};

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Ident(String),
    /// Number in Rust literal form (`-3`, `2.5`).
    Num(String),
    Func(Func),
    /// `/` (axis 1), `⌿` (axis 0), or `/[k]`: reduce along the given axis.
    Slash(usize),
    /// `\` (axis 1), `⍀` (axis 0), or `\[k]`: scan along the given axis.
    Backslash(usize),
    Iota,
    Rho,
    Transpose,
    /// `+.×`
    PlusTimes,
    /// `∘` (compose)
    Jot,
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
        if let Some((default_axis, is_reduce)) = match c {
            '/' => Some((1, true)),
            '⌿' => Some((0, true)),
            '\\' => Some((1, false)),
            '⍀' => Some((0, false)),
            _ => None,
        } {
            // Optional APL-style axis bracket written directly after the
            // operator: `+/[0] m`. Arrays have axes 0 and 1 only.
            let mut axis = default_axis;
            let mut len = 1;
            if peek(1) == Some('[') {
                let digit = peek(2);
                if peek(3) != Some(']') || !matches!(digit, Some('0' | '1')) {
                    let close = (i + 2..chars.len())
                        .find(|&j| chars[j].1 == ']')
                        .map_or(chars.len(), |j| j + 1);
                    return Err(ParseError::new(
                        idx.span(chars[i + 1].0, end_of(close)),
                        "axis must be `[0]` (down the columns) or `[1]` (along the rows)",
                    ));
                }
                axis = usize::from(digit == Some('1'));
                len = 4;
            }
            i += len;
            let tok = if is_reduce {
                Tok::Slash(axis)
            } else {
                Tok::Backslash(axis)
            };
            out.push((idx.span(pos, end_of(i)), tok));
            continue;
        }
        let tok = match c {
            '⍳' => Tok::Iota,
            '⍴' => Tok::Rho,
            '⍉' => Tok::Transpose,
            '∘' if peek(1) == Some('.') => {
                return Err(ParseError::new(
                    idx.span(pos, end_of(i + 2)),
                    "outer product `∘.f` is not supported yet (Phase 2)",
                ));
            }
            '∘' => Tok::Jot,
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
    /// `f/x f\x ⍴x ⍉x r c⍴x ⍳n (f ∘ g) x`
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
    Reduce(Func, usize),
    Scan(Func, usize),
    Shape,
    Transpose,
    Reshape(Vec<usize>),
}

impl FnAtom {
    fn into_train(self, span: Span) -> Train {
        let kind = match self {
            FnAtom::Reduce(f, k) => TrainKind::Reduce(f, k),
            FnAtom::Scan(f, k) => TrainKind::Scan(f, k),
            FnAtom::Shape => TrainKind::Shape,
            FnAtom::Transpose => TrainKind::Transpose,
            FnAtom::Reshape(d) => TrainKind::Reshape(d),
        };
        Train { kind, span }
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
            if matches!(self.peek_at(1), Some(Tok::Slash(_) | Tok::Backslash(_))) {
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
                    Some(&Tok::Slash(k)) => FnAtom::Reduce(f, k),
                    Some(&Tok::Backslash(k)) => FnAtom::Scan(f, k),
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

    /// train_atom := fn_atom | `(` train `)`
    ///
    /// Returns `None`, consuming nothing, if the cursor doesn't start a
    /// function. `(` is ambiguous between `(f ∘ g)` and `(expr)`, so it
    /// backtracks.
    fn train_atom(&mut self) -> Result<Option<Train>, ParseError> {
        if self.peek() == Some(&Tok::LParen) {
            let save = self.pos;
            let open = self.cur();
            self.pos += 1;
            if let Ok(Some(mut t)) = self.train() {
                if self.peek() == Some(&Tok::RParen) {
                    self.pos += 1;
                    t.span = Span::new(open.start, self.prev().end);
                    return Ok(Some(t));
                }
            }
            self.pos = save;
            return Ok(None);
        }
        Ok(self.fn_atom()?.map(|(f, span)| f.into_train(span)))
    }

    /// train := train_atom (`∘` train_atom)*   (left-associative)
    fn train(&mut self) -> Result<Option<Train>, ParseError> {
        let Some(mut t) = self.train_atom()? else {
            return Ok(None);
        };
        while self.peek() == Some(&Tok::Jot) {
            self.pos += 1;
            let Some(g) = self.train_atom()? else {
                return Err(ParseError::new(
                    self.cur(),
                    "expected a function after `∘` (e.g. `+/`, `⍉`, `2 3⍴`)",
                ));
            };
            let span = Span::new(t.span.start, g.span.end);
            t = Train {
                kind: TrainKind::Compose(Box::new(t), Box::new(g)),
                span,
            };
        }
        Ok(Some(t))
    }

    /// prefix := train prefix | `⍳` NUM | primary
    fn prefix(&mut self) -> Result<Expr, ParseError> {
        let at = self.cur();
        if let Some(t) = self.train()? {
            let x = self.prefix()?;
            let span = Span::new(at.start, x.span.end);
            return Ok(Expr::new(apply_train(t, x), span));
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

/// Applies a train. A lone function becomes the ordinary prefix node, so
/// `(+/) x` and `+/x` parse to the same tree.
fn apply_train(t: Train, x: Expr) -> ExprKind {
    let x = Box::new(x);
    match t.kind {
        TrainKind::Reduce(f, k) => ExprKind::Reduce(f, k, x),
        TrainKind::Scan(f, k) => ExprKind::Scan(f, k, x),
        TrainKind::Shape => ExprKind::Shape(x),
        TrainKind::Transpose => ExprKind::Transpose(x),
        TrainKind::Reshape(d) => ExprKind::Reshape(d, x),
        TrainKind::Compose(..) => ExprKind::Apply(t, x),
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
        | ExprKind::Iota(_)
        | ExprKind::Apply(..) => Prec::Prefix,
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
        ExprKind::Reduce(f, k, x) => {
            format!("{}{}{}", f.glyph(), reduce_glyph(*k), at(x, Prec::Prefix))
        }
        ExprKind::Scan(f, k, x) => {
            format!("{}{}{}", f.glyph(), scan_glyph(*k), at(x, Prec::Prefix))
        }
        ExprKind::Apply(t, x) => format!("({}) {}", print_train(t), at(x, Prec::Prefix)),
    }
}

/// Prints a train. `∘` is left-associative, so only a composition on the
/// right of `∘` needs parentheses.
fn print_train(t: &Train) -> String {
    match &t.kind {
        TrainKind::Reduce(f, k) => format!("{}{}", f.glyph(), reduce_glyph(*k)),
        TrainKind::Scan(f, k) => format!("{}{}", f.glyph(), scan_glyph(*k)),
        TrainKind::Shape => "⍴".to_string(),
        TrainKind::Transpose => "⍉".to_string(),
        TrainKind::Reshape(dims) => {
            let d: Vec<String> = dims.iter().map(ToString::to_string).collect();
            format!("{}⍴", d.join(" "))
        }
        TrainKind::Compose(f, g) => {
            let right = if matches!(g.kind, TrainKind::Compose(..)) {
                format!("({})", print_train(g))
            } else {
                print_train(g)
            };
            format!("{} ∘ {}", print_train(f), right)
        }
    }
}

/// The canonical reduce operator for an axis: `⌿` (axis 0) or `/` (axis 1).
fn reduce_glyph(axis: usize) -> char {
    if axis == 0 { '⌿' } else { '/' }
}

/// The canonical scan operator for an axis: `⍀` (axis 0) or `\` (axis 1).
fn scan_glyph(axis: usize) -> char {
    if axis == 0 { '⍀' } else { '\\' }
}
