//! `reduce!` and `scan!` along axis 0 (the default: down each column) and
//! axis 1 (along each row).
//!
//! Run with `cargo run --example reduce_axis`.

use arrayglyph::prelude::*;
use arrayglyph::printer::glyph_to_rust;

fn main() {
    // 3 rows × 4 columns
    let m = Array::from_rows([[1, 2, 3, 4], [5, 6, 7, 8], [9, 10, 11, 12]]);
    println!("m =\n{m}\n");

    // axis = 0 (default): fold down each column, 3×4 -> 1×4.
    let cols = reduce!(+, m); // same as reduce!(+, m, axis = 0); glyph +⌿m
    println!(
        "reduce!(+, m)           +⌿m   shape {:?}\n{cols}\n",
        cols.shape()
    );

    // axis = 1: fold along each row, 3×4 -> 3×1.
    let rows = reduce!(+, m, axis = 1); // glyph +/m
    println!(
        "reduce!(+, m, axis = 1) +/m   shape {:?}\n{rows}\n",
        rows.shape()
    );

    // Both axes is two calls.
    let total = reduce!(+, cols, axis = 1).item(); // glyph +/+⌿m
    println!("grand total +/+⌿m = {total}\n");

    // Non-associative ops fold left: per column, (1 - 5) - 9 = -13.
    println!("reduce!(-, m)  -⌿m\n{}\n", reduce!(-, m));
    println!(
        "reduce!(max, m, axis = 1)  ⌈/m\n{}\n",
        reduce!(max, m, axis = 1)
    );

    // scan! takes the same keyword: running totals down columns / along rows.
    println!("scan!(+, m)  +⍀m\n{}\n", scan!(+, m));
    println!("scan!(+, m, axis = 1)  +\\m\n{}\n", scan!(+, m, axis = 1));

    // Vectors are 1×N, so their only meaningful axis is 1.
    let v = Array::vector([3, 1, 4, 1, 5]);
    println!("+/v = {}", reduce!(+, v, axis = 1).item());
    // `reduce!(+, v)` would be a compile error: axis 0 of a 1-row array is a no-op.

    // The glyph forms and the Rust they stand for:
    for g in ["+⌿m", "+/m", "+/[0] m", "+/+⌿m", "+⍀m"] {
        println!("{g:<8} => {}", glyph_to_rust(g).unwrap());
    }
}
