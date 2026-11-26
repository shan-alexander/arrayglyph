//! Canonical verbose Rust: emitter and a parser for exactly the emitted subset.

use super::{Expr, ExprKind, Func, LineIndex, ParseError, Span, Style, Train, TrainKind};

pub(super) fn print(e: &Expr, style: Style) -> String {
    let p = |x: &Expr| print(x, style);
    match &e.kind {
        ExprKind::Ident(s) => s.clone(),
        ExprKind::Scalar(n) => format!("Array::splat({n})"),
        ExprKind::Strand(ns) => format!("Array::vector([{}])", ns.join(", ")),
        ExprKind::Iota(n) => format!("iota::<{n}>()"),
        ExprKind::Shape(x) => format!("{}.shape()", p(x)),
        ExprKind::Transpose(x) => format!("{}.transpose()", p(x)),
        ExprKind::Reshape(dims, x) => match dims.as_slice() {
            [n] => format!("{}.reshape::<1, {n}>()", p(x)),
            [r, c] => format!("{}.reshape::<{r}, {c}>()", p(x)),
            _ => unreachable!("parser guarantees rank 1 or 2"),
        },
        ExprKind::Dyadic(f, a, b) => {
            format!("{}.zip_with(&{}, {})", p(a), p(b), f.rust_name())
        }
        ExprKind::Reduce(f, k, x) => match (style, f.reduce_method()) {
            (Style::Macro, _) => format!("reduce!({}[{k}], {})", f.macro_token(), p(x)),
            (Style::Method, Some(m)) => format!("{}.{m}::<{k}>()", p(x)),
            (Style::Method, None) => format!("{}.reduce_axis::<{k}>({})", p(x), f.rust_name()),
        },
        ExprKind::Scan(f, k, x) => match (style, f.scan_method()) {
            (Style::Macro, _) => format!("scan!({}[{k}], {})", f.macro_token(), p(x)),
            (Style::Method, Some(m)) => format!("{}.{m}::<{k}>()", p(x)),
            (Style::Method, None) => format!("{}.scan_axis::<{k}>({})", p(x), f.rust_name()),
        },
        ExprKind::MatMul(a, b) => format!("{}.matmul(&{})", p(a), p(b)),
        ExprKind::Apply(t, x) => format!("{}.call(&{})", print_train(t), p(x)),
    }
}

/// A train as a flat `train![…]` list, in glyph (`∘`) order.
fn print_train(t: &Train) -> String {
    let items: Vec<String> = t.parts().into_iter().map(train_item).collect();
    format!("train![{}]", items.join(", "))
}

fn train_item(t: &Train) -> String {
    match &t.kind {
        TrainKind::Reduce(f, k) => format!("reduce({}[{k}])", f.macro_token()),
        TrainKind::Scan(f, k) => format!("scan({}[{k}])", f.macro_token()),
        TrainKind::Shape => "shape".to_string(),
        TrainKind::Transpose => "transpose".to_string(),
        TrainKind::Reshape(dims) => match dims.as_slice() {
            [n] => format!("reshape(1, {n})"),
            [r, c] => format!("reshape({r}, {c})"),
            _ => unreachable!("parser guarantees rank 1 or 2"),
        },
        TrainKind::Compose(..) => unreachable!("parts() flattens compositions"),
    }
}

/// Names of the `arrayglyph::train` function values.
const FN_VALUES: [&str; 6] = ["compose", "Reduce", "Scan", "Transpose", "Shape", "Reshape"];

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Ident(String),
    Num(String),
    Punct(&'static str),
}

fn lex(src: &str, idx: &LineIndex<'_>) -> Result<Vec<(Span, Tok)>, ParseError> {
    const PUNCT: [&str; 16] = [
        "::", ".", "(", ")", ",", "&", "<", ">", "[", "]", "-", "!", "+", "*", "/", "=",
    ];
    let mut out = Vec::new();
    let bytes = src.as_bytes();
    let mut i = 0;
    while i < src.len() {
        let c = bytes[i];
        let start = i;
        let tok = if c.is_ascii_whitespace() {
            i += 1;
            continue;
        } else if c.is_ascii_alphabetic() || c == b'_' {
            while i < src.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                i += 1;
            }
            Tok::Ident(src[start..i].to_string())
        } else if c.is_ascii_digit() {
            while i < src.len() && bytes[i].is_ascii_digit() {
                i += 1;
            }
            if i + 1 < src.len() && bytes[i] == b'.' && bytes[i + 1].is_ascii_digit() {
                i += 1;
                while i < src.len() && bytes[i].is_ascii_digit() {
                    i += 1;
                }
            }
            if i < src.len() && (bytes[i].is_ascii_alphabetic() || bytes[i] == b'_') {
                return Err(ParseError::new(
                    idx.span(start, i + 1),
                    "numeric suffixes are not supported",
                ));
            }
            Tok::Num(src[start..i].to_string())
        } else if let Some(p) = PUNCT.iter().find(|p| src[i..].starts_with(**p)) {
            i += p.len();
            Tok::Punct(p)
        } else {
            let ch = src[i..].chars().next().unwrap_or('?');
            return Err(ParseError::new(
                idx.span(i, i + ch.len_utf8()),
                format!("unexpected character `{ch}`"),
            ));
        };
        out.push((idx.span(start, i), tok));
    }
    Ok(out)
}

struct Parser {
    toks: Vec<(Span, Tok)>,
    pos: usize,
    eof: Span,
}

impl Parser {
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
    fn peek(&self) -> Option<&Tok> {
        self.peek_at(0)
    }
    fn peek_at(&self, k: usize) -> Option<&Tok> {
        self.toks.get(self.pos + k).map(|(_, t)| t)
    }
    /// The last segment of the `a::b::c` path at the cursor, without
    /// consuming it. A `::` before `<` (turbofish) ends the path.
    fn path_tail(&self) -> Option<(usize, &str)> {
        let mut k = 0;
        let mut last = match self.peek_at(0) {
            Some(Tok::Ident(s)) => s.as_str(),
            _ => return None,
        };
        while self.peek_at(k + 1) == Some(&Tok::Punct("::")) {
            match self.peek_at(k + 2) {
                Some(Tok::Ident(s)) => {
                    last = s.as_str();
                    k += 2;
                }
                _ => break,
            }
        }
        Some((k + 1, last))
    }
    /// fn_value := `train![` item, … `]` | `(` fn_value `)`
    ///           | path::(compose `(` fn_value `,` fn_value `)`
    ///           | (Reduce | Scan) `(` op `)` [`.axis::<` k `>()`]
    ///           | Transpose | Shape | Reshape `::<` r `,` c `>`)
    fn fn_value(&mut self) -> Result<Train, ParseError> {
        let at = self.cur();
        if self.at_train_macro() {
            return self.train_macro();
        }
        if self.is_punct("(") {
            self.pos += 1;
            let mut t = self.fn_value()?;
            self.eat(")")?;
            t.span = Span::new(at.start, self.prev().end);
            return Ok(t);
        }
        let Some((len, name)) = self.path_tail() else {
            return self.err("expected a function value (e.g. `Reduce(Add)`)");
        };
        let name = name.to_string();
        self.pos += len;
        let kind = match name.as_str() {
            "compose" => {
                self.eat("(")?;
                let f = self.fn_value()?;
                self.eat(",")?;
                let g = self.fn_value()?;
                if self.is_punct(",") {
                    self.pos += 1;
                }
                self.eat(")")?;
                TrainKind::Compose(Box::new(f), Box::new(g))
            }
            "Reduce" | "Scan" => {
                self.eat("(")?;
                let f = self.op_path()?;
                self.eat(")")?;
                let mut axis = 0;
                if self.is_punct(".") && self.peek_at(1) == Some(&Tok::Ident("axis".into())) {
                    self.pos += 2;
                    axis = self.turbofish_axis()?;
                    self.eat("(")?;
                    self.eat(")")?;
                }
                if name == "Reduce" {
                    TrainKind::Reduce(f, axis)
                } else {
                    TrainKind::Scan(f, axis)
                }
            }
            "Transpose" => TrainKind::Transpose,
            "Shape" => TrainKind::Shape,
            "Reshape" => {
                self.eat("::")?;
                self.eat("<")?;
                let r = self.usize_lit()?;
                self.eat(",")?;
                let c = self.usize_lit()?;
                self.eat(">")?;
                TrainKind::Reshape(if r == 1 { vec![c] } else { vec![r, c] })
            }
            other => {
                return Err(ParseError::new(
                    Span::new(at.start, self.prev().end),
                    format!("`{other}` is not an arrayglyph function value"),
                ));
            }
        };
        Ok(Train {
            kind,
            span: Span::new(at.start, self.prev().end),
        })
    }
    /// Is the cursor at `train!` (optionally `arrayglyph::train!`)?
    fn at_train_macro(&self) -> bool {
        self.path_tail().is_some_and(|(len, name)| {
            name == "train" && self.peek_at(len) == Some(&Tok::Punct("!"))
        })
    }

    /// `train![item, …]`, folded left into compositions (glyph order: the
    /// rightmost item runs first). Brackets, parentheses, or braces.
    fn train_macro(&mut self) -> Result<Train, ParseError> {
        let at = self.cur();
        let (len, _) = self.path_tail().expect("checked by at_train_macro");
        self.pos += len;
        self.eat("!")?;
        let close = if self.is_punct("[") {
            "]"
        } else if self.is_punct("(") {
            ")"
        } else {
            return self.err("expected `[` after `train!`");
        };
        self.pos += 1;
        let mut items = Vec::new();
        while !self.is_punct(close) {
            items.push(self.train_item()?);
            if self.is_punct(",") {
                self.pos += 1;
            } else if !self.is_punct(close) {
                return self.err(format!("expected `,` or `{close}` in `train!`"));
            }
        }
        self.eat(close)?;
        let span = Span::new(at.start, self.prev().end);
        let mut items = items.into_iter();
        let Some(first) = items.next() else {
            return Err(ParseError::new(span, "`train![]` needs at least one item"));
        };
        let mut t = first;
        for g in items {
            t = Train {
                kind: TrainKind::Compose(Box::new(t), Box::new(g)),
                span,
            };
        }
        t.span = span;
        Ok(t)
    }

    /// item := reduce `(` op [`[` k `]`] `)` | scan `(` … `)` | transpose
    ///       | shape | reshape `(` r `,` c `)` | `(` fn_value `)`
    fn train_item(&mut self) -> Result<Train, ParseError> {
        let at = self.cur();
        if self.is_punct("(") {
            return self.fn_value();
        }
        let name = self.ident()?;
        let kind = match name.as_str() {
            "reduce" | "scan" => {
                self.eat("(")?;
                let f = self.macro_op()?;
                let axis = self.bracket_axis()?.unwrap_or(0);
                self.eat(")")?;
                if name == "reduce" {
                    TrainKind::Reduce(f, axis)
                } else {
                    TrainKind::Scan(f, axis)
                }
            }
            "transpose" => TrainKind::Transpose,
            "shape" => TrainKind::Shape,
            "reshape" => {
                self.eat("(")?;
                let r = self.usize_lit()?;
                self.eat(",")?;
                let c = self.usize_lit()?;
                self.eat(")")?;
                TrainKind::Reshape(if r == 1 { vec![c] } else { vec![r, c] })
            }
            other => {
                return Err(ParseError::new(
                    at,
                    format!(
                        "`{other}` is not a train item (expected reduce(op[k]), scan(op[k]), \
                         transpose, shape, reshape(r, c), or a parenthesised function value)"
                    ),
                ));
            }
        };
        Ok(Train {
            kind,
            span: Span::new(at.start, self.prev().end),
        })
    }

    /// An optional `[k]` axis after a macro operator, as in `reduce!(+[1], m)`.
    fn bracket_axis(&mut self) -> Result<Option<usize>, ParseError> {
        if !self.is_punct("[") {
            return Ok(None);
        }
        self.pos += 1;
        let k = self.axis_lit()?;
        self.eat("]")?;
        Ok(Some(k))
    }

    fn err<T>(&self, msg: impl Into<String>) -> Result<T, ParseError> {
        Err(ParseError::new(self.cur(), msg))
    }
    fn is_punct(&self, p: &str) -> bool {
        matches!(self.peek(), Some(Tok::Punct(q)) if *q == p)
    }
    fn eat(&mut self, p: &str) -> Result<(), ParseError> {
        if self.is_punct(p) {
            self.pos += 1;
            Ok(())
        } else {
            self.err(format!("expected `{p}`"))
        }
    }
    fn ident(&mut self) -> Result<String, ParseError> {
        match self.peek() {
            Some(Tok::Ident(s)) => {
                let s = s.clone();
                self.pos += 1;
                Ok(s)
            }
            _ => self.err("expected identifier"),
        }
    }
    /// A possibly negative number literal.
    fn number(&mut self) -> Result<String, ParseError> {
        let neg = self.is_punct("-");
        if neg {
            self.pos += 1;
        }
        match self.peek() {
            Some(Tok::Num(n)) => {
                let n = if neg { format!("-{n}") } else { n.clone() };
                self.pos += 1;
                Ok(n)
            }
            _ => self.err("expected number literal"),
        }
    }
    fn usize_lit(&mut self) -> Result<usize, ParseError> {
        let at = self.cur();
        let n = self.number()?;
        n.parse().map_err(|_| {
            ParseError::new(
                Span::new(at.start, self.prev().end),
                format!("expected a non-negative integer, found `{n}`"),
            )
        })
    }
    /// An axis literal, which must be `0` or `1`.
    fn axis_lit(&mut self) -> Result<usize, ParseError> {
        let at = self.cur();
        let k = self.usize_lit()?;
        if k > 1 {
            return Err(ParseError::new(
                Span::new(at.start, self.prev().end),
                format!("axis must be 0 or 1, found {k}"),
            ));
        }
        Ok(k)
    }
    /// `::<k>` with `k` an axis literal.
    fn turbofish_axis(&mut self) -> Result<usize, ParseError> {
        self.eat("::")?;
        self.eat("<")?;
        let k = self.axis_lit()?;
        self.eat(">")?;
        Ok(k)
    }
    /// `Add`, `op::Add`, or `arrayglyph::op::Add`.
    fn op_path(&mut self) -> Result<Func, ParseError> {
        let at = self.cur();
        let mut name = self.ident()?;
        while self.is_punct("::") {
            self.pos += 1;
            name = self.ident()?;
        }
        Func::from_rust_name(&name).ok_or_else(|| {
            ParseError::new(
                Span::new(at.start, self.prev().end),
                format!("unknown op marker `{name}`"),
            )
        })
    }
    /// `+ - * / max min`, as in `reduce!(+, x)`.
    fn macro_op(&mut self) -> Result<Func, ParseError> {
        let at = self.cur();
        let tok = match self.peek() {
            Some(Tok::Punct(p)) => p.to_string(),
            Some(Tok::Ident(s)) => s.clone(),
            _ => return self.err("expected macro operator"),
        };
        self.pos += 1;
        Func::from_macro_token(&tok)
            .ok_or_else(|| ParseError::new(at, format!("unsupported macro operator `{tok}`")))
    }

    /// Finishes a node that started at `start` and ends at the last token.
    fn node(&self, kind: ExprKind, start: Span) -> Expr {
        Expr::new(kind, Span::new(start.start, self.prev().end))
    }

    /// postfix := primary ( '.' method )*
    fn postfix(&mut self) -> Result<Expr, ParseError> {
        let start = self.cur();
        let mut e = self.primary()?;
        while self.is_punct(".") {
            self.pos += 1;
            let at = self.cur();
            let m = self.ident()?;
            let kind = match m.as_str() {
                "zip_with" => {
                    self.eat("(")?;
                    self.eat("&")?;
                    let rhs = self.postfix()?;
                    self.eat(",")?;
                    let f = self.op_path()?;
                    self.eat(")")?;
                    ExprKind::Dyadic(f, Box::new(e), Box::new(rhs))
                }
                "matmul" => {
                    self.eat("(")?;
                    self.eat("&")?;
                    let rhs = self.postfix()?;
                    self.eat(")")?;
                    ExprKind::MatMul(Box::new(e), Box::new(rhs))
                }
                "reduce" | "scan" | "reduce_axis" | "scan_axis" => {
                    let axis = if m.ends_with("_axis") {
                        self.turbofish_axis()?
                    } else {
                        0
                    };
                    self.eat("(")?;
                    let f = self.op_path()?;
                    self.eat(")")?;
                    if m.starts_with("reduce") {
                        ExprKind::Reduce(f, axis, Box::new(e))
                    } else {
                        ExprKind::Scan(f, axis, Box::new(e))
                    }
                }
                named if Func::from_method(named).is_some() => {
                    let (f, is_reduce) = Func::from_method(named).expect("checked");
                    let axis = if self.is_punct("::") {
                        self.turbofish_axis()?
                    } else {
                        return self.err(format!(
                            "`{named}` needs an explicit axis: `.{named}::<0>()` or `.{named}::<1>()`"
                        ));
                    };
                    self.eat("(")?;
                    self.eat(")")?;
                    if is_reduce {
                        ExprKind::Reduce(f, axis, Box::new(e))
                    } else {
                        ExprKind::Scan(f, axis, Box::new(e))
                    }
                }
                "transpose" | "shape" => {
                    self.eat("(")?;
                    self.eat(")")?;
                    if m == "shape" {
                        ExprKind::Shape(Box::new(e))
                    } else {
                        ExprKind::Transpose(Box::new(e))
                    }
                }
                "reshape" => {
                    self.eat("::")?;
                    self.eat("<")?;
                    let r = self.usize_lit()?;
                    self.eat(",")?;
                    let c = self.usize_lit()?;
                    self.eat(">")?;
                    self.eat("(")?;
                    self.eat(")")?;
                    let dims = if r == 1 { vec![c] } else { vec![r, c] };
                    ExprKind::Reshape(dims, Box::new(e))
                }
                other => {
                    return Err(ParseError::new(
                        at,
                        format!("method `{other}` has no glyph form"),
                    ));
                }
            };
            e = self.node(kind, start);
        }
        Ok(e)
    }

    fn primary(&mut self) -> Result<Expr, ParseError> {
        let at = self.cur();
        if self.is_punct("(") {
            self.pos += 1;
            let mut e = self.postfix()?;
            self.eat(")")?;
            e.span = Span::new(at.start, self.prev().end);
            return Ok(e);
        }
        if self.at_train_macro()
            || self
                .path_tail()
                .is_some_and(|(_, name)| FN_VALUES.contains(&name))
        {
            // `fn_value.call(&x)`
            let t = self.fn_value()?;
            self.eat(".")?;
            let call_at = self.cur();
            if self.ident()? != "call" {
                return Err(ParseError::new(
                    call_at,
                    "expected `.call(&…)` on a function value",
                ));
            }
            self.eat("(")?;
            if self.is_punct("&") {
                self.pos += 1;
            }
            let x = Box::new(self.postfix()?);
            self.eat(")")?;
            let kind = match t.kind {
                TrainKind::Reduce(f, k) => ExprKind::Reduce(f, k, x),
                TrainKind::Scan(f, k) => ExprKind::Scan(f, k, x),
                TrainKind::Shape => ExprKind::Shape(x),
                TrainKind::Transpose => ExprKind::Transpose(x),
                TrainKind::Reshape(d) => ExprKind::Reshape(d, x),
                TrainKind::Compose(..) => ExprKind::Apply(t, x),
            };
            return Ok(self.node(kind, at));
        }
        let name = self.ident()?;
        let kind = match name.as_str() {
            "Array" => {
                self.eat("::")?;
                let ctor_at = self.cur();
                match self.ident()?.as_str() {
                    "splat" => {
                        self.eat("(")?;
                        let n = self.number()?;
                        self.eat(")")?;
                        ExprKind::Scalar(n)
                    }
                    "vector" => {
                        self.eat("(")?;
                        self.eat("[")?;
                        let mut ns = vec![self.number()?];
                        while self.is_punct(",") {
                            self.pos += 1;
                            if self.is_punct("]") {
                                break;
                            }
                            ns.push(self.number()?);
                        }
                        self.eat("]")?;
                        self.eat(")")?;
                        if ns.len() < 2 {
                            return Err(ParseError::new(
                                Span::new(at.start, self.prev().end),
                                "a one-element vector has no glyph strand form",
                            ));
                        }
                        ExprKind::Strand(ns)
                    }
                    other => {
                        return Err(ParseError::new(
                            ctor_at,
                            format!("`Array::{other}` has no glyph form"),
                        ));
                    }
                }
            }
            "iota" => {
                self.eat("::")?;
                self.eat("<")?;
                let n = self.usize_lit()?;
                self.eat(">")?;
                self.eat("(")?;
                self.eat(")")?;
                ExprKind::Iota(n)
            }
            "reduce" | "scan" if self.is_punct("!") => {
                self.pos += 1;
                self.eat("(")?;
                let f = self.macro_op()?;
                let bracket = self.bracket_axis()?;
                self.eat(",")?;
                let x = self.postfix()?;
                let mut axis = bracket.unwrap_or(0);
                if bracket.is_some() {
                    if self.is_punct(",") {
                        self.pos += 1;
                    }
                } else if self.is_punct(",") {
                    self.pos += 1;
                    if self.peek() == Some(&Tok::Ident("axis".into())) {
                        self.pos += 1;
                        self.eat("=")?;
                        axis = self.axis_lit()?;
                        if self.is_punct(",") {
                            self.pos += 1;
                        }
                    }
                }
                self.eat(")")?;
                if name == "reduce" {
                    ExprKind::Reduce(f, axis, Box::new(x))
                } else {
                    ExprKind::Scan(f, axis, Box::new(x))
                }
            }
            _ if self.is_punct("!") || self.is_punct("(") || self.is_punct("::") => {
                return Err(ParseError::new(
                    at,
                    format!("`{name}` is not a canonical arrayglyph form"),
                ));
            }
            _ => ExprKind::Ident(name),
        };
        Ok(self.node(kind, at))
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
    let e = p.postfix()?;
    if p.pos < p.toks.len() {
        return p.err("unexpected trailing input (only one canonical expression is supported)");
    }
    Ok(e)
}
