//! The pretty-printer: dense glyph notation <-> the verbose Rust it means.
//!
//! Run with `cargo run --example pretty_print`.

use arrayglyph::printer::{glyph_to_rust, rust_to_glyph};

fn main() {
    // Glyph -> verbose Rust. Left to right, Rust-style precedence.
    let glyphs = [
        "+/(a × b)",  // dot product
        "+/a × b",    // prefix binds tighter: (+/a) × b
        "2 × 3 + x",  // (2 × 3) + x, as in Rust
        "+⌿m ÷ 3",    // column sums, divided by 3
        "(+/ ∘ ⍉) m", // explicit composition
        "a +.× ⍉b",   // matrix product with a transpose
        "⌈\\(a - b)", // running max of a difference
    ];
    println!("glyph -> Rust");
    for g in glyphs {
        println!("  {g:<12} => {}", glyph_to_rust(g).unwrap());
    }

    // Verbose Rust (including macro calls) -> glyph.
    let rust = "reduce!(+, prices.zip_with(&qty, Mul), axis = 1)";
    println!(
        "\nRust -> glyph\n  {rust}\n  => {}",
        rust_to_glyph(rust).unwrap()
    );

    // Errors carry a line/column span.
    let err = glyph_to_rust("a * b").unwrap_err();
    println!("\nerror: {err}");
}
