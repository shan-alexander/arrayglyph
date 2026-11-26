# arrayglyph

APL-inspired, compile-time-shaped arrays for Rust. *(Project codename: Iota.)*

* **`Array<T, ROWS, COLS>`** is an owned, `Vec`-backed array with its shape in
  the type. Shape mismatches in elementwise ops, `matmul`, and `reshape` are
  **compile errors**.
* **`reduce!(+, m)` / `scan!(max, m)`** fold along an axis as **left
  folds**, left to right like Rust. The default is **axis 0**: down each
  column, so an `R × C` array becomes `1 × C` (glyph `+⌿m`). Pass
  `axis = 1` to fold along each row instead (`R × 1`, glyph `+/m`).
* **`∘` composes functions explicitly**: `(+/ ∘ ⍉) m` is
  `compose(Reduce(Add).axis::<1>(), Transpose).call(&m)`.
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

// +/(a × b)   (dot product). Vectors are 1×N, so they fold along axis 1.
assert_eq!(reduce!(+, a.zip_with(&b, Mul), axis = 1).item(), 32);

// Left fold, like Rust: -/1 2 3 = (1 - 2) - 3 = -4
assert_eq!(reduce!(-, a, axis = 1).item(), -4);

// Axis 0 (the default) folds down each column; axis 1 along each row.
let m = Array::from_rows([[3, 1, 4], [1, 5, 9]]);
assert_eq!(reduce!(+, m).as_slice(), &[4, 6, 13]);          // +⌿m: 2×3 → 1×3
assert_eq!(reduce!(+, m, axis = 1).as_slice(), &[8, 15]);   // +/m: 2×3 → 2×1
assert_eq!(reduce!(+, reduce!(+, m), axis = 1).item(), 23); // both axes: two calls

// Running max along each row
assert_eq!(scan!(max, m, axis = 1).as_slice(), &[3, 3, 4, 1, 5, 9]);

// Shapes are checked by the compiler:
let p = iota::<6>().reshape::<2, 3>();   // ok: 6 == 2*3
let q = p.matmul(&p.transpose());        // (2×3)·(3×2) → 2×2
// let bad = p.matmul(&p);               // compile error: inner dims 3 ≠ 2
// let bad = iota::<6>().reshape::<4, 2>(); // compile error: 6 ≠ 8
assert_eq!(q.shape(), (2, 2));

// Explicit composition: (+/ ∘ ⍉) applies ⍉, then +/ (column sums, as a column)
let col_sums = compose(Reduce(Add).axis::<1>(), Transpose);
assert_eq!(col_sums.call(&m).as_slice(), &[4, 6, 13]);
```

**Axes.** `reduce!` and `scan!` take an optional `axis = 0 | 1` keyword.
Axis 0 (the default) runs down the columns, and axis 1 along the rows. There
are no `reduce_rows!`/`reduce_cols!` variants, and reducing both axes is two
calls. A vector is a `1 × N` array, so its only meaningful axis is 1, and
`reduce!(+, v)` on a vector is a **compile error** that says to use
`axis = 1` (reducing a length-1 axis would silently return the input). An axis
other than 0 or 1 is also a compile error.

Macro operators are ASCII, because Rust's lexer rejects `×`, `⌈`, and similar
glyphs: `+ - * / max min`.

## Glyph ↔ Rust

```text
$ arrayglyph-fmt '+/(a × b)' '+/a × b' '2 × 3 + x' '2 3⍴⍳6' 'a +.× ⍉b'
a.zip_with(&b, Mul).reduce_axis::<1>(Add)
a.reduce_axis::<1>(Add).zip_with(&b, Mul)
Array::splat(2).zip_with(&Array::splat(3), Mul).zip_with(&x, Add)
iota::<6>().reshape::<2, 3>()
a.matmul(&b.transpose())

$ arrayglyph-fmt '+⌿m' '+/m' '+/[0] m' '⌈⍀m'
m.reduce(Add)
m.reduce_axis::<1>(Add)
m.reduce(Add)
m.scan(Max)

$ arrayglyph-fmt '(+/ ∘ ⍉) m'
compose(Reduce(Add).axis::<1>(), Transpose).call(&m)

$ arrayglyph-fmt --to-glyph 'reduce!(max, m)' 'reduce!(max, v, axis = 1)' 'v.zip_with(&Array::splat(2), Mul).scan_axis::<1>(Add)'
⌈⌿m
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
| tightest | prefix `f/ f⌿ f\ f⍀ ⍴ ⍉ r c⍴ ⍳n`, `(f ∘ g) x` | `+/a × b` = `(+/a) × b` |

So the dot product is written `+/(a × b)`. Reductions are left folds:
`-/1 2 3` is `(1 - 2) - 3`, which is `-4`.

| glyph | Rust |
|---|---|
| `+ - × ÷ ⌈ ⌊` | `Add Sub Mul Div Max Min` (in `arrayglyph::op`) |
| `f⌿x`, `f⍀x` (axis 0, down columns) | `x.reduce(F)`, `x.scan(F)` · `reduce!(f, x)` |
| `f/x`, `f\x` (axis 1, along rows) | `x.reduce_axis::<1>(F)`, `x.scan_axis::<1>(F)` · `reduce!(f, x, axis = 1)` |
| `f/[0]x`, `f/[1]x` | accepted as input; printed as `f⌿x`, `f/x` |
| `a f b` | `a.zip_with(&b, F)` |
| `⍳n`, `⍴x`, `⍉x` | `iota::<n>()`, `x.shape()`, `x.transpose()` |
| `r c⍴x` | `x.reshape::<r, c>()` |
| `a +.× b` | `a.matmul(&b)` |
| `(f ∘ g) x` | `compose(F, G).call(&x)` with `F`, `G` in `Reduce(Op)` (axis 0), `Reduce(Op).axis::<1>()`, `Scan(Op)`, `Scan(Op).axis::<1>()`, `Transpose`, `Shape`, `Reshape::<r, c>` |
| `1 2 3`, `¯3` | `Array::vector([1, 2, 3])`, `-3` |

## Commands

```sh
cargo test                       # unit, integration, doc and compile_fail tests
cargo clippy --all-targets       # lint (clean)
cargo run --example basics       # quick tour
cargo run --bin arrayglyph-fmt -- '+/a × b'
```

## Examples

Each one is short, commented, and self-contained. The showcase's data,
[`examples/data/weather.csv`](examples/data/weather.csv), is **synthetic
sample data** made up for the demo, not real measurements.
[`examples/WISHLIST.md`](examples/WISHLIST.md) lists the features the
showcase wanted, and which of them now exist.

| example | shows |
|---|---|
| **`cargo run --example showcase`** | **the full tour on a small dataset: a labelled CSV (14 days × 5 cities of synthetic temperatures) loaded into `Array<f64, 14, 5>`, per-city (axis 0) and per-day (axis 1) statistics, a `∘` train for weekly means, and every computation shown as glyph next to the verbose Rust that runs** |
| `cargo run --example shaped_array` | construction, shapes, elementwise ops, `matmul`, and the compile error a shape mismatch causes |
| `cargo run --example reduce_axis` | `reduce!`/`scan!` with `axis = 0` (default, columns) vs `axis = 1` (rows) |
| `cargo run --example compose` | building trains with `∘` / `compose(f, g)` |
| `cargo run --example pretty_print` | glyph → verbose Rust and back |
| `cargo run --example basics` | a quick tour of all of the above |

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT), at your option.
