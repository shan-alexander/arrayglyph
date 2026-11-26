//! The `∘` compose operator: building a reusable function value (a train).
//!
//! `(f ∘ g) x` applies `g`, then `f`. In Rust it is the flat
//! `train![f, g]`, listed in the same order as the glyph (the rightmost
//! item runs first). `compose(f, g)` is the lower-level API it expands to.
//!
//! Run with `cargo run --example compose`.

use arrayglyph::prelude::*;
use arrayglyph::printer::glyph_to_rust;

fn main() {
    let m = Array::from_rows([[1, 2, 3], [4, 5, 6]]);
    println!("m =\n{m}\n");

    // (+/ ∘ ⍉) m : transpose (2×3 -> 3×2), then sum each row -> 3×1.
    // That is the column sums of m, laid out as a column.
    let col_sums = train![reduce(+[1]), transpose];
    let out = col_sums.call(&m);
    println!("(+/ ∘ ⍉) m  shape {:?}\n{out}\n", out.shape());
    // Same numbers as the axis-0 reduction +⌿m (which keeps them as a row).
    assert_eq!(out.as_slice(), m.sum::<0>().as_slice());

    // A train is an ordinary value: name it once, reuse it.
    let other = Array::from_rows([[10, 20, 30], [1, 1, 1]]);
    println!("(+/ ∘ ⍉) other\n{}\n", col_sums.call(&other));

    // (⌈/ ∘ +⍀) m: running totals down each column (+⍀) first, then the
    // largest value in each row of that (⌈/).
    let peak = train![reduce(max[1]), scan(+[0])];
    println!("(⌈/ ∘ +⍀) m\n{}\n", peak.call(&m));

    // Three stages: (+/ ∘ ⍉ ∘ 2 3⍴) ⍳6 — one flat list, no nesting.
    let three = train![reduce(+[1]), transpose, reshape(2, 3)];
    println!("(+/ ∘ ⍉ ∘ 2 3⍴) ⍳6\n{}\n", three.call(&iota::<6>()));
    // The same value with the lower-level API:
    let nested = compose(compose(Reduce(Add).axis::<1>(), Transpose), Reshape::<2, 3>);
    assert_eq!(three.call(&iota::<6>()), nested.call(&iota::<6>()));

    // What the glyph forms desugar to:
    for g in ["(+/ ∘ ⍉) m", "(⌈/ ∘ +⍀) m", "(+/ ∘ ⍉ ∘ 2 3⍴) ⍳6"] {
        println!("{g:<20} => {}", glyph_to_rust(g).unwrap());
    }
}
