//! The const-generic shaped array: the shape lives in the type.
//!
//! Run with `cargo run --example shaped_array`.

use arrayglyph::prelude::*;

fn main() {
    // Construction. Each constructor fixes ROWS and COLS at compile time.
    let a: Array<i32, 2, 3> = Array::from_rows([[1, 2, 3], [4, 5, 6]]);
    // 2 3⍴⍳6 (the element count is checked: 6 = 2×3). iota yields usize; map to i32.
    let b = iota::<6>().reshape::<2, 3>().map(|x| x as i32);
    let v = Array::vector([10, 20, 30]); // a Vector is a 1×N array
    let z: Array<i32, 3, 2> = Array::from_fn(|r, c| (r * 10 + c) as i32);

    println!("a =\n{a}\n");
    println!("b = 2 3⍴⍳6 =\n{b}\n");
    println!(
        "shape of a: {:?}, of v: {:?}, of z: {:?}\n",
        a.shape(),
        v.shape(),
        z.shape()
    );

    // Elementwise ops need identical shapes: a + b (both 2×3).
    let sum = a.zip_with(&b, Add); // glyph: a + b
    println!("a + b =\n{sum}\n");
    let doubled = Array::splat(2).zip_with(&a, Mul); // glyph: 2 × a (scalar extension)
    println!("2 × a =\n{doubled}\n");

    // Matrix product: (2×3) +.× (3×2) -> 2×2. The inner dimensions must agree.
    let p = a.matmul(&z); // glyph: a +.× z
    println!("a +.× z  (2×3 · 3×2 → {:?}) =\n{p}\n", p.shape());

    // Transpose swaps the type-level shape: 2×3 -> 3×2.
    let t: Array<i32, 3, 2> = a.transpose();
    println!("⍉a =\n{t}");

    // A shape mismatch never reaches run time. Uncommenting this line:
    //
    //     let bad = a.zip_with(&t, Add);
    //
    // fails to compile with
    //
    //     error[E0308]: mismatched types
    //         let bad = a.zip_with(&t, Add);
    //                              ^^ expected `2`, found `3`
    //
    // and `a.matmul(&a)` (2×3 · 2×3) fails the same way: `matmul` wants an
    // `Array<i32, 3, K>` on the right. A wrong reshape such as
    // `iota::<6>().reshape::<4, 2>()` is rejected at build time by a const
    // assertion (error[E0080]: "reshape: element count must be preserved").
}
