# arrayglyph

APL-inspired, compile-time-shaped arrays for Rust. *(Project codename: Iota.)*

* **`Array<T, ROWS, COLS>`** is an owned, `Vec`-backed array with its shape in
  the type. Shape mismatches in elementwise ops, `matmul`, and `reshape` are
  **compile errors**.
* **`reduce!(+, x)` / `scan!(max, x)`** are the glyphs `+/x` and `⌈\x`. They
  work along the last axis as **left folds**, left to right like Rust.
* **A pretty-printer and the `arrayglyph-fmt` CLI** convert dense glyph
  notation to verbose Rust and back.

Status: Phase 1 prototype. Not published to crates.io (`publish = false`).
See [DESIGN.md](DESIGN.md) for the rationale, semantics, open questions, and
roadmap.

## Quick start

```rust
use arrayglyph::prelude::*;

let a = Array::vector([1, 2, 3]);
let b = Array::vector([4, 5, 6]);

// +/(a × b)   (dot product)
assert_eq!(reduce!(+, a.zip_with(&b, Mul)).item(), 32);

// Left fold, like Rust: -/1 2 3 = (1 - 2) - 3 = -4
assert_eq!(reduce!(-, a).item(), -4);

// Running max, per row of a matrix
let m = Array::from_rows([[3, 1, 4], [1, 5, 9]]);
assert_eq!(scan!(max, m).as_slice(), &[3, 3, 4, 1, 5, 9]);

// Shapes are checked by the compiler:
let p = iota::<6>().reshape::<2, 3>();   // ok: 6 == 2*3
let q = p.matmul(&p.transpose());        // (2×3)·(3×2) → 2×2
// let bad = p.matmul(&p);               // compile error: inner dims 3 ≠ 2
// let bad = iota::<6>().reshape::<4, 2>(); // compile error: 6 ≠ 8
assert_eq!(q.shape(), (2, 2));
```

Macro operators are ASCII, because Rust's lexer rejects `×`, `⌈`, and similar
glyphs: `+ - * / max min`.

## Glyph ↔ Rust

```text
$ arrayglyph-fmt '+/(a × b)' '+/a × b' '2 × 3 + x' '2 3⍴⍳6' 'a +.× ⍉b'
a.zip_with(&b, Mul).reduce(Add)
a.reduce(Add).zip_with(&b, Mul)
Array::splat(2).zip_with(&Array::splat(3), Mul).zip_with(&x, Add)
iota::<6>().reshape::<2, 3>()
a.matmul(&b.transpose())

$ arrayglyph-fmt --to-glyph 'reduce!(max, v)' 'v.zip_with(&Array::splat(2), Mul).scan(Add)'
⌈/v
+\(v × 2)
```

From Rust, use `arrayglyph::printer::{glyph_to_rust, rust_to_glyph}`.

**Evaluation order.** Glyph text reads **left to right with Rust-style
precedence**. This is a deliberate break from APL's right-to-left rule:

| binds | forms | example |
|---|---|---|
| loosest | `+ -` | `a - b - c` = `(a - b) - c` |
| | `× ÷ +.×` | `2 × 3 + x` = `(2 × 3) + x` |
| | `⌈ ⌊` (like a method call) | `a ⌈ b + c` = `(a ⌈ b) + c` |
| tightest | prefix `f/ f\ ⍴ ⍉ r c⍴ ⍳n` | `+/a × b` = `(+/a) × b` |

So the dot product is written `+/(a × b)`. Reductions are left folds:
`-/1 2 3` is `(1 - 2) - 3`, which is `-4`.

| glyph | Rust |
|---|---|
| `+ - × ÷ ⌈ ⌊` | `Add Sub Mul Div Max Min` (in `arrayglyph::op`) |
| `f/x`, `f\x` | `x.reduce(F)`, `x.scan(F)` |
| `a f b` | `a.zip_with(&b, F)` |
| `⍳n`, `⍴x`, `⍉x` | `iota::<n>()`, `x.shape()`, `x.transpose()` |
| `r c⍴x` | `x.reshape::<r, c>()` |
| `a +.× b` | `a.matmul(&b)` |
| `1 2 3`, `¯3` | `Array::vector([1, 2, 3])`, `-3` |

## Commands

```sh
cargo test                       # unit, integration, doc and compile_fail tests
cargo clippy --all-targets       # lint (clean)
cargo run --example basics       # tour
cargo run --bin arrayglyph-fmt -- '+/a × b'
```

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT), at your option.
