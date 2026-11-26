//! Round-trip tests: glyph -> verbose Rust -> glyph is the identity on a
//! corpus of canonical glyph strings, and the verbose form is pinned.
//!
//! Glyph evaluation is left to right with Rust-style precedence:
//! `+ -` < `× ÷ +.×` < `⌈ ⌊` < prefix forms (`f/ f⌿ f\ f⍀ ⍴ ⍉ r c⍴ ⍳n`).
//!
//! Axes: `f⌿`/`f⍀` work along axis 0 (`x.sum::<0>()`), `f/`/`f\` along
//! axis 1 (`x.sum::<1>()`). Both Rust styles always write the axis.
//! Trains print as flat `train![…]` lists in glyph order.

use arrayglyph::printer::{Expr, Style, glyph_to_rust, glyph_to_rust_with, rust_to_glyph};

/// (canonical glyph, expected method-style Rust)
const CORPUS: &[(&str, &str)] = &[
    ("x", "x"),
    ("+/x", "x.sum::<1>()"),
    ("-/x", "x.reduce_axis::<1>(Sub)"),
    ("×/x", "x.prod::<1>()"),
    ("÷/x", "x.reduce_axis::<1>(Div)"),
    ("⌈/x", "x.max::<1>()"),
    ("⌊/x", "x.min::<1>()"),
    ("+\\x", "x.cumsum::<1>()"),
    ("-\\1 2 3", "Array::vector([1, 2, 3]).scan_axis::<1>(Sub)"),
    ("⌈\\v", "v.cummax::<1>()"),
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
    ("+/a × b", "a.sum::<1>().zip_with(&b, Mul)"),
    ("+/(a × b)", "a.zip_with(&b, Mul).sum::<1>()"),
    ("+/x - y", "x.sum::<1>().zip_with(&y, Sub)"),
    (
        "-/x ÷ +/y",
        "x.reduce_axis::<1>(Sub).zip_with(&y.sum::<1>(), Div)",
    ),
    (
        "⌊/((a - b) × c)",
        "a.zip_with(&b, Sub).zip_with(&c, Mul).min::<1>()",
    ),
    ("+\\+\\x", "x.cumsum::<1>().cumsum::<1>()"),
    (
        "⌈\\(a × 2)",
        "a.zip_with(&Array::splat(2), Mul).cummax::<1>()",
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
    ("+/⍳5", "iota::<5>().sum::<1>()"),
    ("⍳5 + x", "iota::<5>().zip_with(&x, Add)"),
    ("⍴m", "m.shape()"),
    ("⍉m", "m.transpose()"),
    ("+/⍉m", "m.transpose().sum::<1>()"),
    ("⍉+/m", "m.sum::<1>().transpose()"),
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
    ("+/a +.× b", "a.sum::<1>().matmul(&b)"),
    ("+/(a +.× b)", "a.matmul(&b).sum::<1>()"),
    ("a +.× b + c", "a.matmul(&b).zip_with(&c, Add)"),
    ("a +.× (b + c)", "a.matmul(&b.zip_with(&c, Add))"),
    ("a +.× b +.× c", "a.matmul(&b).matmul(&c)"),
    (
        "2 2⍴m +.× ⍉2 2⍴m",
        "m.reshape::<2, 2>().matmul(&m.reshape::<2, 2>().transpose())",
    ),
    // Explicit composition: `f ∘ g` applies g, then f.
    ("(+/ ∘ ⍉) m", "train![reduce(+[1]), transpose].call(&m)"),
    ("(⌈\\ ∘ ⍉) m", "train![scan(max[1]), transpose].call(&m)"),
    ("(⍴ ∘ ⍉) m", "train![shape, transpose].call(&m)"),
    (
        "(⍉ ∘ 2 3⍴) ⍳6",
        "train![transpose, reshape(2, 3)].call(&iota::<6>())",
    ),
    (
        "(+/ ∘ 6⍴) m",
        "train![reduce(+[1]), reshape(1, 6)].call(&m)",
    ),
    (
        "(+/ ∘ ⍉ ∘ 2 3⍴) ⍳6",
        "train![reduce(+[1]), transpose, reshape(2, 3)].call(&iota::<6>())",
    ),
    (
        "(+/ ∘ ⍉) m + 1",
        "train![reduce(+[1]), transpose].call(&m).zip_with(&Array::splat(1), Add)",
    ),
    (
        "(+/ ∘ ⍉) (m + 1)",
        "train![reduce(+[1]), transpose].call(&m.zip_with(&Array::splat(1), Add))",
    ),
    (
        "2 × (-/ ∘ ⍉) m",
        "Array::splat(2).zip_with(&train![reduce(-[1]), transpose].call(&m), Mul)",
    ),
    (
        "(+/ ∘ ⍉) (⍉ ∘ ⍉) m",
        "train![reduce(+[1]), transpose].call(&train![transpose, transpose].call(&m))",
    ),
    (
        "+/(⍉ ∘ ⍉) m",
        "train![transpose, transpose].call(&m).sum::<1>()",
    ),
    // Axis 0: `f⌿` and `f⍀` (down the columns; the Rust default).
    ("+⌿m", "m.sum::<0>()"),
    ("-⌿m", "m.reduce_axis::<0>(Sub)"),
    ("⌈⌿m", "m.max::<0>()"),
    ("+⍀m", "m.cumsum::<0>()"),
    ("⌊⍀m", "m.cummin::<0>()"),
    ("+⌿m × 2", "m.sum::<0>().zip_with(&Array::splat(2), Mul)"),
    ("+⌿(m × 2)", "m.zip_with(&Array::splat(2), Mul).sum::<0>()"),
    ("+/+⌿m", "m.sum::<0>().sum::<1>()"),
    ("+⌿+/m", "m.sum::<1>().sum::<0>()"),
    ("+⌿⍉m", "m.transpose().sum::<0>()"),
    ("+⌿m ÷ 3", "m.sum::<0>().zip_with(&Array::splat(3), Div)"),
    ("m - +⌿m", "m.zip_with(&m.sum::<0>(), Sub)"),
    ("(+⌿ ∘ ⍉) m", "train![reduce(+[0]), transpose].call(&m)"),
    (
        "(⌈⍀ ∘ 3 2⍴) ⍳6",
        "train![scan(max[0]), reshape(3, 2)].call(&iota::<6>())",
    ),
    ("(+/ ∘ +⌿) m", "train![reduce(+[1]), reduce(+[0])].call(&m)"),
];

/// (canonical glyph, expected macro-style Rust). Every `CORPUS` entry also
/// round-trips in macro style; these pin the exact text.
const MACRO_CORPUS: &[(&str, &str)] = &[
    ("+⌿m", "reduce!(+[0], m)"),
    ("+/m", "reduce!(+[1], m)"),
    ("-/x", "reduce!(-[1], x)"),
    ("×⌿x", "reduce!(*[0], x)"),
    ("÷/x", "reduce!(/[1], x)"),
    ("⌈⌿m", "reduce!(max[0], m)"),
    ("⌊/m", "reduce!(min[1], m)"),
    ("+⍀m", "scan!(+[0], m)"),
    ("×\\m", "scan!(*[1], m)"),
    ("-\\1 2 3", "scan!(-[1], Array::vector([1, 2, 3]))"),
    ("⌈\\v", "scan!(max[1], v)"),
    ("⌊⍀m", "scan!(min[0], m)"),
    ("+/(a × b)", "reduce!(+[1], a.zip_with(&b, Mul))"),
    ("+/a × b", "reduce!(+[1], a).zip_with(&b, Mul)"),
    ("+/+⌿m", "reduce!(+[1], reduce!(+[0], m))"),
    (
        "+⌿m ÷ 3",
        "reduce!(+[0], m).zip_with(&Array::splat(3), Div)",
    ),
    ("m - +⌿m", "m.zip_with(&reduce!(+[0], m), Sub)"),
    ("+/⍉m", "reduce!(+[1], m.transpose())"),
    ("⍉+/m", "reduce!(+[1], m).transpose()"),
    ("+/a +.× b", "reduce!(+[1], a).matmul(&b)"),
    ("+\\+\\x", "scan!(+[1], scan!(+[1], x))"),
    (
        "(+/ ∘ ⍉ ∘ 2 3⍴) ⍳6",
        "train![reduce(+[1]), transpose, reshape(2, 3)].call(&iota::<6>())",
    ),
    (
        "+/(⍉ ∘ ⍉) m",
        "reduce!(+[1], train![transpose, transpose].call(&m))",
    ),
];

#[test]
fn macro_style_matches_corpus() {
    for (g, r) in MACRO_CORPUS {
        assert_eq!(
            glyph_to_rust_with(g, Style::Macro).unwrap(),
            *r,
            "desugaring {g:?}"
        );
        assert_eq!(rust_to_glyph(r).unwrap(), *g, "compressing {r:?}");
    }
}

#[test]
fn macro_style_round_trips_whole_corpus() {
    for (g, method) in CORPUS {
        let mac = glyph_to_rust_with(g, Style::Macro).unwrap();
        assert_eq!(
            rust_to_glyph(&mac).unwrap(),
            *g,
            "glyph -> macro -> glyph via {mac:?}"
        );
        // Both styles parse to the same tree.
        assert_eq!(
            Expr::parse_rust(&mac).unwrap().to_rust(),
            *method,
            "macro -> method for {g:?}"
        );
        let back = Expr::parse_rust(method)
            .unwrap()
            .to_rust_styled(Style::Macro);
        assert_eq!(back, mac, "method -> macro for {g:?}");
        // Both styles always write the axis.
        assert!(
            !mac.contains("reduce!(+, ") && !mac.contains("axis ="),
            "{mac}"
        );
    }
}

#[test]
fn method_style_always_writes_the_axis() {
    for (g, r) in CORPUS {
        for bare in [".reduce(", ".scan(", "Reduce(", "Scan(", "compose("] {
            assert!(
                !r.contains(bare),
                "{g:?} printed {r:?} without an explicit axis"
            );
        }
    }
}

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
        // `∘` is associative: any grouping prints as one flat train.
        ("(+/ ∘ (⍉ ∘ 2 3⍴)) ⍳6", "(+/ ∘ ⍉ ∘ 2 3⍴) ⍳6"),
        ("((+/ ∘ ⍉) ∘ 2 3⍴) ⍳6", "(+/ ∘ ⍉ ∘ 2 3⍴) ⍳6"),
        ("(+/ ∘ (⍉ ∘ (⍉ ∘ ⍉))) m", "(+/ ∘ ⍉ ∘ ⍉ ∘ ⍉) m"),
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
        // Canonical bracket-axis macros, with trailing commas and paths.
        ("reduce!(+[0], m)", "+⌿m"),
        ("reduce!(*[1], m,)", "×/m"),
        ("scan!(min[0], m)", "⌊⍀m"),
        ("scan!(/[1], v)", "÷\\v"),
        // Named const-axis methods.
        ("m.sum::<0>()", "+⌿m"),
        ("m.prod::<1>()", "×/m"),
        ("m.min::<0>().max::<1>()", "⌈/⌊⌿m"),
        ("m.cumsum::<1>()", "+\\m"),
        ("m.cummin::<0>()", "⌊⍀m"),
        // Flat trains: brackets or parens, paths, nesting, trailing comma,
        // by-value call, and a one-item train normalised to its function.
        ("train![reduce(+[1]), transpose].call(&m)", "(+/ ∘ ⍉) m"),
        (
            "arrayglyph::train!(reduce(+[1]), transpose,).call(m)",
            "(+/ ∘ ⍉) m",
        ),
        (
            "train![reduce(max[0]), (train![transpose, shape])].call(&m)",
            "(⌈⌿ ∘ ⍉ ∘ ⍴) m",
        ),
        ("train![reduce(+), transpose].call(&m)", "(+⌿ ∘ ⍉) m"),
        ("train![scan(*[1])].call(&m)", "×\\m"),
        ("train![reshape(1, 6), (Transpose)].call(&m)", "(6⍴ ∘ ⍉) m"),
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
        // New forms: axis required on named methods and in range.
        "m.sum()",
        "m.cumsum::<2>()",
        "m.mean::<0>()", // no glyph form
        "reduce!(+[2], m)",
        "reduce!(+[0] m)",
        "reduce!(+[0], m, axis = 1)",
        "train![].call(&m)",
        "train![transpose]",                    // not applied
        "train![frobnicate].call(&m)",          // unknown item
        "train![reduce(%[1])].call(&m)",        // unknown op
        "train![reshape(2)].call(&m)",          // reshape needs r, c
        "train![transpose transpose].call(&m)", // missing comma
    ];
    for r in bad {
        assert!(rust_to_glyph(r).is_err(), "{r:?} should be rejected");
    }
}
