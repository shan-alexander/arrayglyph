//! The pretty-printer: dense glyph notation <-> the verbose Rust it means.
//!
//! Run with `cargo run --example pretty_print`.

use arrayglyph::printer::{Style, glyph_to_rust, glyph_to_rust_with, rust_to_glyph};

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
    println!("glyph -> Rust (method style, the default)");
    for g in glyphs {
        println!("  {g:<12} => {}", glyph_to_rust(g).unwrap());
    }
    println!("\nglyph -> Rust (macro style)");
    for g in glyphs {
        println!(
            "  {g:<12} => {}",
            glyph_to_rust_with(g, Style::Macro).unwrap()
        );
    }

    // Verbose Rust (either style, or the older keyword form) -> glyph.
    println!("\nRust -> glyph");
    for rust in [
        "prices.zip_with(&qty, Mul).sum::<1>()",
        "reduce!(+[1], prices.zip_with(&qty, Mul))",
        "reduce!(+, prices.zip_with(&qty, Mul), axis = 1)",
    ] {
        println!("  {rust:<50} => {}", rust_to_glyph(rust).unwrap());
    }

    // Errors carry a line/column span.
    let err = glyph_to_rust("a * b").unwrap_err();
    println!("\nerror: {err}");
}
