//! Reductions and scans along axis 0 (down each column) and axis 1 (along
//! each row). The axis is always written: as a bracket in the macros,
//! `reduce!(+[0], m)` (APL's `+/[0] m`), or as a const generic in the named
//! methods, `m.sum::<0>()`.
//!
//! Run with `cargo run --example reduce_axis`.

use arrayglyph::prelude::*;
use arrayglyph::printer::{Style, glyph_to_rust, glyph_to_rust_with};

fn main() {
    // 3 rows × 4 columns
    let m = Array::from_rows([[1, 2, 3, 4], [5, 6, 7, 8], [9, 10, 11, 12]]);
    println!("m =\n{m}\n");

    // Axis 0: fold down each column, 3×4 -> 1×4. Glyph +⌿m.
    let cols = reduce!(+[0], m);
    assert_eq!(cols, m.sum::<0>()); // the same thing, as a method
    println!(
        "reduce!(+[0], m)  m.sum::<0>()  +⌿m   shape {:?}\n{cols}\n",
        cols.shape()
    );

    // Axis 1: fold along each row, 3×4 -> 3×1. Glyph +/m.
    let rows = reduce!(+[1], m);
    assert_eq!(rows, m.sum::<1>());
    println!(
        "reduce!(+[1], m)  m.sum::<1>()  +/m   shape {:?}\n{rows}\n",
        rows.shape()
    );

    // Both axes is two calls.
    let total = cols.sum::<1>().item(); // glyph +/+⌿m
    println!("grand total +/+⌿m = {total}\n");

    // Non-associative ops fold left: per column, (1 - 5) - 9 = -13.
    // `-` and `÷` have no named method; use the macro or `reduce_axis`.
    println!("reduce!(-[0], m)  -⌿m\n{}\n", reduce!(-[0], m));
    println!("m.max::<1>()  ⌈/m\n{}\n", m.max::<1>());

    // Scans: running totals down columns / products along rows.
    println!("m.cumsum::<0>()  +⍀m\n{}\n", m.cumsum::<0>());
    println!("scan!(*[1], m)  ×\\m\n{}\n", scan!(*[1], m));

    // Vectors are 1×N, so their only meaningful axis is 1.
    let v = Array::vector([3, 1, 4, 1, 5]);
    println!("+/v = {}", v.sum::<1>().item());
    // `v.sum::<0>()` would be a compile error: axis 0 of a 1-row array is a no-op.

    // The older keyword form still works (kept for compatibility):
    assert_eq!(reduce!(+, m, axis = 1), rows);

    // The glyph forms and the Rust they stand for, in both styles:
    for g in ["+⌿m", "+/m", "+/[0] m", "+/+⌿m", "+⍀m", "-⌿m"] {
        println!(
            "{g:<8} => {:<26} | {}",
            glyph_to_rust(g).unwrap(),
            glyph_to_rust_with(g, Style::Macro).unwrap()
        );
    }
}
