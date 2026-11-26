//! A tour of arrayglyph: shaped arrays, axis reductions, trains, and the printer.
//!
//! Run with `cargo run --example basics`.

use arrayglyph::prelude::*;
use arrayglyph::printer::{Style, glyph_to_rust, glyph_to_rust_with, rust_to_glyph};

fn main() {
    // Shapes live in the type.
    let prices: Array<f64, 2, 3> = Array::from_rows([[9.5, 12.0, 3.25], [4.0, 4.0, 10.0]]);
    let qty: Array<f64, 2, 3> = Array::from_rows([[2.0, 1.0, 4.0], [3.0, 0.0, 1.0]]);

    // +/(prices × qty)     (per-row totals)
    let totals = prices.zip_with(&qty, Mul).sum::<1>();
    assert_eq!(totals, reduce!(+[1], prices.zip_with(&qty, Mul))); // macro spelling
    println!("totals per order (+/(prices × qty)):\n{totals}\n");

    // ⌈\ running max along each row
    println!("running max (⌈\\prices):\n{}\n", prices.cummax::<1>());

    // Left to right, like Rust: -/ 1 2 3 = (1 - 2) - 3 = -4
    println!(
        "-/1 2 3 = {}",
        reduce!(-[1], Array::vector([1, 2, 3])).item()
    );

    // Matrix product; the inner dimension is checked at compile time.
    let m = iota::<6>().reshape::<2, 3>();
    let mmt = m.matmul(&m.transpose());
    println!("\nm +.× ⍉m where m←2 3⍴⍳6:\n{mmt}\n");

    // Explicit composition: (+/ ∘ ⍉) applies ⍉ first, then +/.
    let col_sums = train![reduce(+[1]), transpose];
    println!("(+/ ∘ ⍉) m:\n{}\n", col_sums.call(&m));

    // The pretty-printer, both directions.
    for glyph in [
        "+/(a × b)",
        "2 × 3 + x",
        "⌈\\v",
        "2 3⍴⍳6",
        "+/x - y",
        "a +.× ⍉b",
        "(+/ ∘ ⍉) m",
        "+⌿m",
        "+/[1] m",
    ] {
        let rust = glyph_to_rust(glyph).expect("valid glyph");
        let back = rust_to_glyph(&rust).expect("canonical rust");
        println!("{glyph:<12} => {rust:<45} => {back}");
    }

    // Macro style writes the axis in brackets.
    for glyph in ["+/(a × b)", "×\\m", "+/+⌿m"] {
        let rust = glyph_to_rust_with(glyph, Style::Macro).expect("valid glyph");
        println!("{glyph:<12} => {rust}");
    }
}
