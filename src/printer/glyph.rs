//! Glyph notation: lexer, right-to-left parser, and canonical printer.

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
        self.toks.get(self.pos).map(|(_, t)| t)
    }
    fn peek2(&self) -> Option<&Tok> {
        self.toks.get(self.pos + 1).map(|(_, t)| t)
    }
    /// Span of the next token, or the empty end-of-input span.
    fn cur(&self) -> Span {
        self.toks.get(self.pos).map_or(self.eof, |&(s, _)| s)
    }
    fn bump(&mut self) -> Option<(Span, Tok)> {
        let t = self.toks.get(self.pos).cloned();
        self.pos += 1;
        t
    }

    /// expr := operand ( dyad expr )?   (the right operand extends to the end)
    fn expr(&mut self) -> Result<Expr, ParseError> {
        let left = self.operand()?;
        match self.peek() {
            Some(Tok::Func(f)) => {
                let f = *f;
                if matches!(self.peek2(), Some(Tok::Slash | Tok::Backslash)) {
                    return Err(ParseError::new(
                        self.cur(),
                        "a left argument to `f/` or `f\\` (n-wise reduce) is unsupported",
                    ));
                }
                self.bump();
                let right = self.expr()?;
                let (ls, rs) = (left.span, right.span);
                Ok(node(
                    ExprKind::Dyadic(f, Box::new(left), Box::new(right)),
                    ls,
                    rs,
                ))
            }
            Some(Tok::PlusTimes) => {
                self.bump();
                let right = self.expr()?;
                let (ls, rs) = (left.span, right.span);
                Ok(node(
                    ExprKind::MatMul(Box::new(left), Box::new(right)),
                    ls,
                    rs,
                ))
            }
            Some(Tok::Rho) => {
                let dims = match &left.kind {
                    ExprKind::Scalar(n) => vec![n.clone()],
                    ExprKind::Strand(ns) => ns.clone(),
                    _ => {
                        return Err(ParseError::new(
                            left.span,
                            "left argument of dyadic `⍴` must be a literal shape like `2 3`",
                        ));
                    }
                };
                if dims.len() > 2 {
                    return Err(ParseError::new(
                        left.span,
                        "reshape supports rank 1 or 2 only",
                    ));
                }
                let dims = dims
                    .iter()
                    .map(|d| {
                        d.parse::<usize>().map_err(|_| {
                            ParseError::new(
                                left.span,
                                format!("shape `{d}` is not a non-negative integer"),
                            )
                        })
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                self.bump();
                let right = self.expr()?;
                let rs = right.span;
                Ok(node(
                    ExprKind::Reshape(dims, Box::new(right)),
                    left.span,
                    rs,
                ))
            }
            _ => Ok(left),
        }
    }

    fn operand(&mut self) -> Result<Expr, ParseError> {
        let at = self.cur();
        let Some((_, tok)) = self.bump() else {
            return Err(ParseError::new(at, "unexpected end of input"));
        };
        match tok {
            Tok::Func(f) => {
                let kind: fn(Func, Box<Expr>) -> ExprKind = match self.peek() {
                    Some(Tok::Slash) => ExprKind::Reduce,
                    Some(Tok::Backslash) => ExprKind::Scan,
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
                self.bump();
                let x = self.expr()?;
                let xs = x.span;
                Ok(node(kind(f, Box::new(x)), at, xs))
            }
            Tok::Iota => {
                let arg = self.expr()?;
                match &arg.kind {
                    ExprKind::Scalar(n) => match n.parse::<usize>() {
                        Ok(n) => Ok(node(ExprKind::Iota(n), at, arg.span)),
                        Err(_) => Err(ParseError::new(
                            arg.span,
                            format!("`⍳` needs a non-negative integer, got `{n}`"),
                        )),
                    },
                    _ => Err(ParseError::new(
                        arg.span,
                        "`⍳` needs a literal length (its argument is the whole expression to its right)",
                    )),
                }
            }
            Tok::Rho | Tok::Transpose => {
                let x = self.expr()?;
                let xs = x.span;
                let kind = if tok == Tok::Rho {
                    ExprKind::Shape(Box::new(x))
                } else {
                    ExprKind::Transpose(Box::new(x))
                };
                Ok(node(kind, at, xs))
            }
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
                let mut last = at;
                while let Some(Tok::Num(_)) = self.peek() {
                    if let Some((s, Tok::Num(n))) = self.bump() {
                        nums.push(n);
                        last = s;
                    }
                }
                let kind = if nums.len() == 1 {
                    ExprKind::Scalar(nums.pop().unwrap_or_default())
                } else {
                    ExprKind::Strand(nums)
                };
                Ok(node(kind, at, last))
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

/// Left operands need parentheses when they would otherwise swallow the
/// rest of the line (anything that takes a right argument).
fn left(e: &Expr) -> String {
    match &e.kind {
        ExprKind::Ident(_) | ExprKind::Scalar(_) | ExprKind::Strand(_) => print(e),
        _ => format!("({})", print(e)),
    }
}

pub(super) fn print(e: &Expr) -> String {
    match &e.kind {
        ExprKind::Ident(s) => s.clone(),
        ExprKind::Scalar(n) => num(n),
        ExprKind::Strand(ns) => ns.iter().map(|n| num(n)).collect::<Vec<_>>().join(" "),
        ExprKind::Iota(n) => format!("⍳{n}"),
        ExprKind::Shape(x) => format!("⍴{}", print(x)),
        ExprKind::Transpose(x) => format!("⍉{}", print(x)),
        ExprKind::Reshape(dims, x) => {
            let d: Vec<String> = dims.iter().map(ToString::to_string).collect();
            format!("{}⍴{}", d.join(" "), print(x))
        }
        ExprKind::Dyadic(f, a, b) => format!("{} {} {}", left(a), f.glyph(), print(b)),
        ExprKind::Reduce(f, x) => format!("{}/{}", f.glyph(), print(x)),
        ExprKind::Scan(f, x) => format!("{}\\{}", f.glyph(), print(x)),
        ExprKind::MatMul(a, b) => format!("{} +.× {}", left(a), print(b)),
    }
}
