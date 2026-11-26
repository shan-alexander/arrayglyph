//! A tour of arrayglyph: shaped arrays, reduce!/scan!, and the printer.
//!
//! Run with `cargo run --example basics`.

use arrayglyph::prelude::*;
use arrayglyph::printer::{glyph_to_rust, rust_to_glyph};

fn main() {
    // Shapes live in the type.
    let prices: Array<f64, 2, 3> = Array::from_rows([[9.5, 12.0, 3.25], [4.0, 4.0, 10.0]]);
    let qty: Array<f64, 2, 3> = Array::from_rows([[2.0, 1.0, 4.0], [3.0, 0.0, 1.0]]);

    // +/(prices × qty)     (per-row totals)
    let totals = reduce!(+, prices.zip_with(&qty, Mul));
    println!("totals per order (+/(prices × qty)):\n{totals}\n");

    // ⌈\ running max along each row
    println!("running max (⌈\\prices):\n{}\n", scan!(max, prices));

    // Left to right, like Rust: -/ 1 2 3 = (1 - 2) - 3 = -4
    println!("-/1 2 3 = {}", reduce!(-, Array::vector([1, 2, 3])).item());

    // Matrix product; the inner dimension is checked at compile time.
    let m = iota::<6>().reshape::<2, 3>();
    let mmt = m.matmul(&m.transpose());
    println!("\nm +.× ⍉m where m←2 3⍴⍳6:\n{mmt}\n");

    // Explicit composition: (+/ ∘ ⍉) applies ⍉ first, then +/.
    let col_sums = compose(Reduce(Add), Transpose);
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
    ] {
        let rust = glyph_to_rust(glyph).expect("valid glyph");
        let back = rust_to_glyph(&rust).expect("canonical rust");
        println!("{glyph:<12} => {rust:<45} => {back}");
    }
}
