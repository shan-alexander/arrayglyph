//! Round-trip tests: glyph -> verbose Rust -> glyph is the identity on a
//! corpus of canonical glyph strings, and the verbose form is pinned.
//!
//! Glyph evaluation is left to right with Rust-style precedence:
//! `+ -` < `× ÷ +.×` < `⌈ ⌊` < prefix forms (`f/ f⌿ f\ f⍀ ⍴ ⍉ r c⍴ ⍳n`).
//!
//! Axes: `f⌿`/`f⍀` work along axis 0 (the Rust default, `x.reduce(F)`),
//! `f/`/`f\` along axis 1 (`x.reduce_axis::<1>(F)`).

use arrayglyph::printer::{Expr, glyph_to_rust, rust_to_glyph};

/// (canonical glyph, expected verbose Rust)
const CORPUS: &[(&str, &str)] = &[
    ("x", "x"),
    ("+/x", "x.reduce_axis::<1>(Add)"),
    ("-/x", "x.reduce_axis::<1>(Sub)"),
    ("×/x", "x.reduce_axis::<1>(Mul)"),
    ("÷/x", "x.reduce_axis::<1>(Div)"),
    ("⌈/x", "x.reduce_axis::<1>(Max)"),
    ("⌊/x", "x.reduce_axis::<1>(Min)"),
    ("+\\x", "x.scan_axis::<1>(Add)"),
    ("-\\1 2 3", "Array::vector([1, 2, 3]).scan_axis::<1>(Sub)"),
    ("⌈\\v", "v.scan_axis::<1>(Max)"),
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
    ("+/a × b", "a.reduce_axis::<1>(Add).zip_with(&b, Mul)"),
    ("+/(a × b)", "a.zip_with(&b, Mul).reduce_axis::<1>(Add)"),
    ("+/x - y", "x.reduce_axis::<1>(Add).zip_with(&y, Sub)"),
    (
        "-/x ÷ +/y",
        "x.reduce_axis::<1>(Sub).zip_with(&y.reduce_axis::<1>(Add), Div)",
    ),
    (
        "⌊/((a - b) × c)",
        "a.zip_with(&b, Sub).zip_with(&c, Mul).reduce_axis::<1>(Min)",
    ),
    ("+\\+\\x", "x.scan_axis::<1>(Add).scan_axis::<1>(Add)"),
    (
        "⌈\\(a × 2)",
        "a.zip_with(&Array::splat(2), Mul).scan_axis::<1>(Max)",
    ),
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
    ("+/⍳5", "iota::<5>().reduce_axis::<1>(Add)"),
    ("⍳5 + x", "iota::<5>().zip_with(&x, Add)"),
    ("⍴m", "m.shape()"),
    ("⍉m", "m.transpose()"),
    ("+/⍉m", "m.transpose().reduce_axis::<1>(Add)"),
    ("⍉+/m", "m.reduce_axis::<1>(Add).transpose()"),
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
    ("+/a +.× b", "a.reduce_axis::<1>(Add).matmul(&b)"),
    ("+/(a +.× b)", "a.matmul(&b).reduce_axis::<1>(Add)"),
    ("a +.× b + c", "a.matmul(&b).zip_with(&c, Add)"),
    ("a +.× (b + c)", "a.matmul(&b.zip_with(&c, Add))"),
    ("a +.× b +.× c", "a.matmul(&b).matmul(&c)"),
    (
        "2 2⍴m +.× ⍉2 2⍴m",
        "m.reshape::<2, 2>().matmul(&m.reshape::<2, 2>().transpose())",
    ),
    // Explicit composition: `f ∘ g` applies g, then f.
    (
        "(+/ ∘ ⍉) m",
        "compose(Reduce(Add).axis::<1>(), Transpose).call(&m)",
    ),
    (
        "(⌈\\ ∘ ⍉) m",
        "compose(Scan(Max).axis::<1>(), Transpose).call(&m)",
    ),
    ("(⍴ ∘ ⍉) m", "compose(Shape, Transpose).call(&m)"),
    (
        "(⍉ ∘ 2 3⍴) ⍳6",
        "compose(Transpose, Reshape::<2, 3>).call(&iota::<6>())",
    ),
    (
        "(+/ ∘ 6⍴) m",
        "compose(Reduce(Add).axis::<1>(), Reshape::<1, 6>).call(&m)",
    ),
    (
        "(+/ ∘ ⍉ ∘ 2 3⍴) ⍳6",
        "compose(compose(Reduce(Add).axis::<1>(), Transpose), Reshape::<2, 3>).call(&iota::<6>())",
    ),
    (
        "(+/ ∘ (⍉ ∘ 2 3⍴)) ⍳6",
        "compose(Reduce(Add).axis::<1>(), compose(Transpose, Reshape::<2, 3>)).call(&iota::<6>())",
    ),
    (
        "(+/ ∘ ⍉) m + 1",
        "compose(Reduce(Add).axis::<1>(), Transpose).call(&m).zip_with(&Array::splat(1), Add)",
    ),
    (
        "(+/ ∘ ⍉) (m + 1)",
        "compose(Reduce(Add).axis::<1>(), Transpose).call(&m.zip_with(&Array::splat(1), Add))",
    ),
    (
        "2 × (-/ ∘ ⍉) m",
        "Array::splat(2).zip_with(&compose(Reduce(Sub).axis::<1>(), Transpose).call(&m), Mul)",
    ),
    (
        "(+/ ∘ ⍉) (⍉ ∘ ⍉) m",
        "compose(Reduce(Add).axis::<1>(), Transpose).call(&compose(Transpose, Transpose).call(&m))",
    ),
    (
        "+/(⍉ ∘ ⍉) m",
        "compose(Transpose, Transpose).call(&m).reduce_axis::<1>(Add)",
    ),
    // Axis 0: `f⌿` and `f⍀` (down the columns; the Rust default).
    ("+⌿m", "m.reduce(Add)"),
    ("-⌿m", "m.reduce(Sub)"),
    ("⌈⌿m", "m.reduce(Max)"),
    ("+⍀m", "m.scan(Add)"),
    ("⌊⍀m", "m.scan(Min)"),
    ("+⌿m × 2", "m.reduce(Add).zip_with(&Array::splat(2), Mul)"),
    ("+⌿(m × 2)", "m.zip_with(&Array::splat(2), Mul).reduce(Add)"),
    ("+/+⌿m", "m.reduce(Add).reduce_axis::<1>(Add)"),
    ("+⌿+/m", "m.reduce_axis::<1>(Add).reduce(Add)"),
    ("+⌿⍉m", "m.transpose().reduce(Add)"),
    ("+⌿m ÷ 3", "m.reduce(Add).zip_with(&Array::splat(3), Div)"),
    ("m - +⌿m", "m.zip_with(&m.reduce(Add), Sub)"),
    ("(+⌿ ∘ ⍉) m", "compose(Reduce(Add), Transpose).call(&m)"),
    (
        "(⌈⍀ ∘ 3 2⍴) ⍳6",
        "compose(Scan(Max), Reshape::<3, 2>).call(&iota::<6>())",
    ),
    (
        "(+/ ∘ +⌿) m",
        "compose(Reduce(Add).axis::<1>(), Reduce(Add)).call(&m)",
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
        ("+/ ∘ ⍉ m", "(+/ ∘ ⍉) m"), // unparenthesised train
        ("((+/ ∘ ⍉)) m", "(+/ ∘ ⍉) m"),
        ("(+/ ∘ ⍉ m)", "(+/ ∘ ⍉) m"),
        ("(+/) x", "+/x"), // a lone function needs no compose
        ("(+/ ∘ ⍉)m", "(+/ ∘ ⍉) m"),
        // APL-style axis brackets normalise to the canonical operator.
        ("+/[0] m", "+⌿m"),
        ("+/[1] m", "+/m"),
        ("+⌿[1] m", "+/m"),
        ("+\\[0] m", "+⍀m"),
        ("⌈⍀[1] m", "⌈\\m"),
        ("(+/[0] ∘ ⍉) m", "(+⌿ ∘ ⍉) m"),
        ("(+⌿) m", "+⌿m"),
    ];
    for (input, canon) in cases {
        assert_eq!(Expr::parse_glyph(input).unwrap().to_glyph(), canon);
    }
}

#[test]
fn rust_side_accepts_macros_paths_and_parens() {
    let cases = [
        ("reduce!(+, a.zip_with(&b, Mul), axis = 1)", "+/(a × b)"),
        ("scan!(max, v, axis = 1)", "⌈\\v"),
        ("reduce!(/, v, axis = 1)", "÷/v"),
        ("x.reduce_axis::<1>(op::Add)", "+/x"),
        ("x.reduce_axis::<1>(arrayglyph::op::Min)", "⌊/x"),
        ("(a).zip_with(&(b), Add)", "a + b"),
        ("Array::vector([1, 2, 3,])", "1 2 3"),
        (
            "train::compose(train::Reduce(op::Add).axis::<1>(), train::Transpose).call(&m)",
            "(+/ ∘ ⍉) m",
        ),
        (
            "compose(Reduce(Add).axis::<1>(), Transpose,).call(m)",
            "(+/ ∘ ⍉) m",
        ),
        ("Reduce(Add).axis::<1>().call(&x)", "+/x"),
        ("Reshape::<2, 3>.call(&x)", "2 3⍴x"),
        // Axis forms: macro keyword, explicit axis 0, and train `.axis`.
        ("reduce!(+, m)", "+⌿m"),
        ("reduce!(+, m, axis = 0)", "+⌿m"),
        ("reduce!(max, m, axis = 1,)", "⌈/m"),
        ("scan!(+, m)", "+⍀m"),
        ("scan!(min, m, axis = 1)", "⌊\\m"),
        ("m.reduce_axis::<0>(Add)", "+⌿m"),
        ("m.scan_axis::<0>(Add)", "+⍀m"),
        ("Reduce(Add).axis::<0>().call(&m)", "+⌿m"),
        ("Scan(Max).axis::<1>().call(&m)", "⌈\\m"),
    ];
    for (input, glyph) in cases {
        assert_eq!(rust_to_glyph(input).unwrap(), glyph, "{input}");
    }
}

#[test]
fn glyph_errors() {
    let bad = [
        "a * b",      // ASCII * is APL power
        "-x",         // monadic minus unsupported
        "+/",         // missing operand
        "a + (b",     // unbalanced
        "a b",        // trailing input
        "⍳x",         // iota needs a literal
        "⍳¯3",        // iota needs a non-negative literal
        "⍳5 6",       // iota takes one length
        "x⍴y",        // reshape needs a literal shape (x⍴ is not a function)
        "2 3 4⍴x",    // rank 3 not supported
        "1.5⍴x",      // non-integer shape
        "a +/ b",     // n-wise reduce
        "a ∘.× b",    // outer product: Phase 2
        "(+/ ∘) m",   // dangling compose
        "(∘ ⍉) m",    // compose needs a left function
        "(+/ ∘ ⍉)",   // a train must be applied
        "(+/ ∘ x) m", // x is not a function
        "(- ∘ ⍉) m",  // monadic minus is not a function
        "+/[2] m",    // arrays have axes 0 and 1 only
        "+/[] m",     // empty axis
        "+/[0 m",     // unclosed axis bracket
        "[0] m",      // an axis needs an operator
        "a +",        // dangling operator
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
        "x.reduce_axis::<1>(Add) y",
        "Array::splat(2i32)",
        "compose(Reduce(Add).axis::<1>(), Transpose)", // not applied
        "compose(Reduce(Add).axis::<1>(), x).call(&m)", // x is not a function value
        "compose(Reduce(Pow), Transpose).call(&m)",
        "Transpose.apply(&m)",
        "m.reduce_axis::<2>(Add)",
        "reduce!(+, m, axis = 2)",
        "reduce!(+, m, axes = 1)",
        "Reduce(Add).axis::<3>().call(&m)",
        "scan!(+, m, axis = x)",
    ];
    for r in bad {
        assert!(rust_to_glyph(r).is_err(), "{r:?} should be rejected");
    }
}
