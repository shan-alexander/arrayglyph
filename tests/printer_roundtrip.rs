//! Round-trip tests: glyph -> verbose Rust -> glyph is the identity on a
//! corpus of canonical glyph strings, and the verbose form is pinned.
//!
//! Glyph evaluation is left to right with Rust-style precedence:
//! `+ -` < `× ÷ +.×` < `⌈ ⌊` < prefix forms (`f/ f\ ⍴ ⍉ r c⍴ ⍳n`).

use arrayglyph::printer::{Expr, glyph_to_rust, rust_to_glyph};

/// (canonical glyph, expected verbose Rust)
const CORPUS: &[(&str, &str)] = &[
    ("x", "x"),
    ("+/x", "x.reduce(Add)"),
    ("-/x", "x.reduce(Sub)"),
    ("×/x", "x.reduce(Mul)"),
    ("÷/x", "x.reduce(Div)"),
    ("⌈/x", "x.reduce(Max)"),
    ("⌊/x", "x.reduce(Min)"),
    ("+\\x", "x.scan(Add)"),
    ("-\\1 2 3", "Array::vector([1, 2, 3]).scan(Sub)"),
    ("⌈\\v", "v.scan(Max)"),
    ("a + b", "a.zip_with(&b, Add)"),
    ("a - b", "a.zip_with(&b, Sub)"),
    ("a × b", "a.zip_with(&b, Mul)"),
    ("a ÷ b", "a.zip_with(&b, Div)"),
    ("a ⌈ b", "a.zip_with(&b, Max)"),
    ("a ⌊ b", "a.zip_with(&b, Min)"),
    // Precedence: × ÷ over + -, left-associative.
    ("a + b × c", "a.zip_with(&b.zip_with(&c, Mul), Add)"),
    ("a × b + c", "a.zip_with(&b, Mul).zip_with(&c, Add)"),
    ("(a + b) × c", "a.zip_with(&b, Add).zip_with(&c, Mul)"),
    ("a × (b + c)", "a.zip_with(&b.zip_with(&c, Add), Mul)"),
    ("a - b - c", "a.zip_with(&b, Sub).zip_with(&c, Sub)"),
    ("a - (b - c)", "a.zip_with(&b.zip_with(&c, Sub), Sub)"),
    ("a ÷ b ÷ c", "a.zip_with(&b, Div).zip_with(&c, Div)"),
    ("a ÷ b × c", "a.zip_with(&b, Div).zip_with(&c, Mul)"),
    ("a - b + c", "a.zip_with(&b, Sub).zip_with(&c, Add)"),
    ("a - (b + c)", "a.zip_with(&b.zip_with(&c, Add), Sub)"),
    // ⌈ ⌊ bind tighter than × (like a method call).
    ("a ⌈ b + c", "a.zip_with(&b, Max).zip_with(&c, Add)"),
    ("a ⌈ (b + c)", "a.zip_with(&b.zip_with(&c, Add), Max)"),
    ("a × b ⌊ c", "a.zip_with(&b.zip_with(&c, Min), Mul)"),
    ("(a × b) ⌊ c", "a.zip_with(&b, Mul).zip_with(&c, Min)"),
    // Prefix forms bind tightest.
    ("+/a × b", "a.reduce(Add).zip_with(&b, Mul)"),
    ("+/(a × b)", "a.zip_with(&b, Mul).reduce(Add)"),
    ("+/x - y", "x.reduce(Add).zip_with(&y, Sub)"),
    ("-/x ÷ +/y", "x.reduce(Sub).zip_with(&y.reduce(Add), Div)"),
    (
        "⌊/((a - b) × c)",
        "a.zip_with(&b, Sub).zip_with(&c, Mul).reduce(Min)",
    ),
    ("+\\+\\x", "x.scan(Add).scan(Add)"),
    ("⌈\\(a × 2)", "a.zip_with(&Array::splat(2), Mul).scan(Max)"),
    // Scalars and strands.
    ("2 × v", "Array::splat(2).zip_with(&v, Mul)"),
    ("v × 2.5", "v.zip_with(&Array::splat(2.5), Mul)"),
    ("v + ¯3", "v.zip_with(&Array::splat(-3), Add)"),
    (
        "2 × 3 + v",
        "Array::splat(2).zip_with(&Array::splat(3), Mul).zip_with(&v, Add)",
    ),
    ("1 2 3 + v", "Array::vector([1, 2, 3]).zip_with(&v, Add)"),
    ("¯1 0 1 × v", "Array::vector([-1, 0, 1]).zip_with(&v, Mul)"),
    // Iota, shape, transpose, reshape.
    ("⍳5", "iota::<5>()"),
    ("+/⍳5", "iota::<5>().reduce(Add)"),
    ("⍳5 + x", "iota::<5>().zip_with(&x, Add)"),
    ("⍴m", "m.shape()"),
    ("⍉m", "m.transpose()"),
    ("+/⍉m", "m.transpose().reduce(Add)"),
    ("⍉+/m", "m.reduce(Add).transpose()"),
    ("2 3⍴⍳6", "iota::<6>().reshape::<2, 3>()"),
    (
        "2 3⍴⍳6 + 1",
        "iota::<6>().reshape::<2, 3>().zip_with(&Array::splat(1), Add)",
    ),
    (
        "2 3⍴(⍳6 + 1)",
        "iota::<6>().zip_with(&Array::splat(1), Add).reshape::<2, 3>()",
    ),
    ("6⍴m", "m.reshape::<1, 6>()"),
    ("⍉2 3⍴x", "x.reshape::<2, 3>().transpose()"),
    // Matrix product sits at the × level.
    ("a +.× b", "a.matmul(&b)"),
    ("a +.× ⍉b", "a.matmul(&b.transpose())"),
    ("+/a +.× b", "a.reduce(Add).matmul(&b)"),
    ("+/(a +.× b)", "a.matmul(&b).reduce(Add)"),
    ("a +.× b + c", "a.matmul(&b).zip_with(&c, Add)"),
    ("a +.× (b + c)", "a.matmul(&b.zip_with(&c, Add))"),
    ("a +.× b +.× c", "a.matmul(&b).matmul(&c)"),
    (
        "2 2⍴m +.× ⍉2 2⍴m",
        "m.reshape::<2, 2>().matmul(&m.reshape::<2, 2>().transpose())",
    ),
];

#[test]
fn glyph_to_rust_matches_corpus() {
    for (g, r) in CORPUS {
        assert_eq!(glyph_to_rust(g).unwrap(), *r, "desugaring {g:?}");
    }
}

#[test]
fn glyph_rust_glyph_is_identity() {
    for (g, _) in CORPUS {
        let r = glyph_to_rust(g).unwrap();
        assert_eq!(rust_to_glyph(&r).unwrap(), *g, "round trip via {r:?}");
    }
}

#[test]
fn rust_glyph_rust_is_identity() {
    for (_, r) in CORPUS {
        let g = rust_to_glyph(r).unwrap();
        assert_eq!(glyph_to_rust(&g).unwrap(), *r, "round trip via {g:?}");
    }
}

#[test]
fn ast_kinds_survive_round_trip() {
    // Spans differ between the two sources; the tree shape must not.
    for (g, _) in CORPUS {
        let a = Expr::parse_glyph(g).unwrap();
        let b = Expr::parse_rust(&a.to_rust()).unwrap();
        assert_eq!(a.to_rust(), b.to_rust());
        assert_eq!(a.to_glyph(), b.to_glyph());
    }
}

#[test]
fn non_canonical_glyph_input_is_normalised() {
    let cases = [
        ("+/ (a  ×  b)", "+/(a × b)"),
        ("( a )", "a"),
        ("((+/x))", "+/x"),
        ("(+/x) - y", "+/x - y"),
        ("(a × b) + c", "a × b + c"),
        ("(a - b) - c", "a - b - c"),
        ("a + (b × c)", "a + b × c"),
        ("a − b", "a - b"), // U+2212 minus
        ("2 3 ⍴ ⍳ 6", "2 3⍴⍳6"),
        ("a+.×b", "a +.× b"),
        ("(⍳5) + x", "⍳5 + x"),
    ];
    for (input, canon) in cases {
        assert_eq!(Expr::parse_glyph(input).unwrap().to_glyph(), canon);
    }
}

#[test]
fn rust_side_accepts_macros_paths_and_parens() {
    let cases = [
        ("reduce!(+, a.zip_with(&b, Mul))", "+/(a × b)"),
        ("scan!(max, v)", "⌈\\v"),
        ("reduce!(/, v,)", "÷/v"),
        ("x.reduce(op::Add)", "+/x"),
        ("x.reduce(arrayglyph::op::Min)", "⌊/x"),
        ("(a).zip_with(&(b), Add)", "a + b"),
        ("Array::vector([1, 2, 3,])", "1 2 3"),
    ];
    for (input, glyph) in cases {
        assert_eq!(rust_to_glyph(input).unwrap(), glyph, "{input}");
    }
}

#[test]
fn glyph_errors() {
    let bad = [
        "a * b",   // ASCII * is APL power
        "-x",      // monadic minus unsupported
        "+/",      // missing operand
        "a + (b",  // unbalanced
        "a b",     // trailing input
        "⍳x",      // iota needs a literal
        "⍳¯3",     // iota needs a non-negative literal
        "⍳5 6",    // iota takes one length
        "x⍴y",     // reshape needs a literal shape (x⍴ is not a function)
        "2 3 4⍴x", // rank 3 not supported
        "1.5⍴x",   // non-integer shape
        "a +/ b",  // n-wise reduce
        "a ∘.× b", // outer product: Phase 2
        "a +",     // dangling operator
        ")",
    ];
    for g in bad {
        assert!(glyph_to_rust(g).is_err(), "{g:?} should be rejected");
    }
}

#[test]
fn rust_errors() {
    let bad = [
        "for i in 0..n {}",
        "&a + &b",
        "x.sum()",
        "x.reduce(Pow)",
        "Array::vector([5])",
        "Array::zeros()",
        "foo(x)",
        "reduce!(%, x)",
        "x.reduce(Add) y",
        "Array::splat(2i32)",
    ];
    for r in bad {
        assert!(rust_to_glyph(r).is_err(), "{r:?} should be rejected");
    }
}
