# arrayglyph

APL-inspired, compile-time-shaped arrays for Rust. *(Project codename: Iota.)*

* **`Array<T, ROWS, COLS>`** is an owned, `Vec`-backed array with its shape in
  the type. Shape mismatches in elementwise ops, `matmul`, and `reshape` are
  **compile errors**.
* **Reductions and scans always name their axis** and fold as **left
  folds**, left to right like Rust. Axis 0 runs down each column (`R × C →
  1 × C`, glyph `+⌿m`), axis 1 along each row (`R × 1`, glyph `+/m`).
  Write them as methods, `m.sum::<0>()` / `m.cumprod::<1>()`, or as
  macros with the axis in brackets, `reduce!(+[0], m)` / `scan!(*[1], m)`
  (APL's `+/[0] m`). The result shape is part of the type either way.
* **`∘` composes functions explicitly**, written as a flat train in glyph
  order: `(+/ ∘ ⍉ ∘ 2 3⍴) v` is
  `train![reduce(+[1]), transpose, reshape(2, 3)].call(&v)`.
* **`prof!(expr)`** times one expression and prints its shape and elapsed
  time to stderr, then returns the value.
* **A pretty-printer and the `arrayglyph-fmt` CLI** convert dense glyph
  notation (the primary notation) to verbose Rust and back, in method style
  (default) or macro style (`--style macro`).
* **Small conveniences:** `broadcast`, `mean::<K>()`, scalar operators
  (`&m * 2.0`), `{:.1}`-aware printing, and `table::Table` for loading a
  labelled CSV into a shaped array.

Status: Phase 1 prototype. Not published to crates.io (`publish = false`).
See [DESIGN.md](DESIGN.md) for the rationale, semantics, open questions, and
roadmap.

## Quick start

```rust
use arrayglyph::prelude::*;

let a = Array::vector([1, 2, 3]);
let b = Array::vector([4, 5, 6]);

// +/(a × b)   (dot product). Vectors are 1×N, so they fold along axis 1.
assert_eq!(a.zip_with(&b, Mul).sum::<1>().item(), 32);
assert_eq!(reduce!(+[1], a.zip_with(&b, Mul)).item(), 32); // macro spelling

// Left fold, like Rust: -/1 2 3 = (1 - 2) - 3 = -4
assert_eq!(reduce!(-[1], a).item(), -4);

// Axis 0 folds down each column; axis 1 along each row.
let m = Array::from_rows([[3, 1, 4], [1, 5, 9]]);
assert_eq!(m.sum::<0>().as_slice(), &[4, 6, 13]);       // +⌿m: 2×3 → 1×3
assert_eq!(m.sum::<1>().as_slice(), &[8, 15]);          // +/m: 2×3 → 2×1
assert_eq!(m.sum::<0>().sum::<1>().item(), 23);         // both axes: two calls
assert_eq!(reduce!(+[0], m), m.sum::<0>());             // same thing, as a macro

// Running max along each row: ⌈\m
assert_eq!(m.cummax::<1>().as_slice(), &[3, 3, 4, 1, 5, 9]);
assert_eq!(scan!(max[1], m), m.cummax::<1>());

// Shapes are checked by the compiler:
let p = iota::<6>().reshape::<2, 3>();   // ok: 6 == 2*3
let q = p.matmul(&p.transpose());        // (2×3)·(3×2) → 2×2
// let bad = p.matmul(&p);               // compile error: inner dims 3 ≠ 2
// let bad = iota::<6>().reshape::<4, 2>(); // compile error: 6 ≠ 8
assert_eq!(q.shape(), (2, 2));

// Explicit composition: (+/ ∘ ⍉) applies ⍉, then +/ (column sums, as a column).
// train! lists functions in glyph order: the rightmost runs first.
let col_sums = train![reduce(+[1]), transpose];
assert_eq!(col_sums.call(&m).as_slice(), &[4, 6, 13]);

// Time one expression: prints `[prof] file:line  +⌿m  ⇐  m.sum::<0>()  → 1×3  in …`
// to stderr and returns the value.
let cols = prof!(glyph = "+⌿m", m.sum::<0>());
assert_eq!(cols.as_slice(), &[4, 6, 13]);
```

**Axes.** The axis is always written, so the result shape is visible at the
call site:

| | axis 0 (down columns, `R×C → 1×C`) | axis 1 (along rows, `R×C → R×1`) |
|---|---|---|
| glyph | `+⌿m`, `+⍀m` (or `+/[0] m`) | `+/m`, `+\m` (or `+/[1] m`) |
| method | `m.sum::<0>()`, `m.cumsum::<0>()` | `m.sum::<1>()`, `m.cumsum::<1>()` |
| macro | `reduce!(+[0], m)`, `scan!(+[0], m)` | `reduce!(+[1], m)`, `scan!(+[1], m)` |

Named methods: `sum`, `prod`, `max`, `min`, `mean` (reductions) and
`cumsum`, `cumprod`, `cummax`, `cummin` (scans), each `::<K>()` with a
const-generic axis so a wrong axis or a shape mismatch is a compile error.
`-` and `÷` have no named method: use `reduce!(-[1], m)` or
`m.reduce_axis::<1>(Sub)`. There are no `reduce_rows!`/`reduce_cols!`
variants, and reducing both axes is two calls. A vector is a `1 × N` array,
so its only meaningful axis is 1; `v.sum::<0>()` or `reduce!(+[0], v)` on a
vector is a **compile error** that says to use axis 1 (reducing a length-1
axis would silently return the input). An axis other than 0 or 1 is also a
compile error.

*Deprecated spellings, still accepted:* `reduce!(+, m, axis = 1)` (keyword
axis) and `reduce!(+, m)` (implicit axis 0), and likewise for `scan!`. New
code and the printer always use the bracket form.

Macro operators are ASCII, because Rust's lexer rejects `×`, `⌈`, and similar
glyphs: `+ - * / max min`.

## Glyph ↔ Rust

```text
$ arrayglyph-fmt '+/(a × b)' '+/a × b' '2 × 3 + x' '2 3⍴⍳6' 'a +.× ⍉b'
a.zip_with(&b, Mul).sum::<1>()
a.sum::<1>().zip_with(&b, Mul)
Array::splat(2).zip_with(&Array::splat(3), Mul).zip_with(&x, Add)
iota::<6>().reshape::<2, 3>()
a.matmul(&b.transpose())

$ arrayglyph-fmt '+⌿m' '+/m' '+/[0] m' '⌈⍀m'
m.sum::<0>()
m.sum::<1>()
m.sum::<0>()
m.cummax::<0>()

$ arrayglyph-fmt --style macro '+⌿m' '+/m' '×\m' '+/+⌿m'
reduce!(+[0], m)
reduce!(+[1], m)
scan!(*[1], m)
reduce!(+[1], reduce!(+[0], m))

$ arrayglyph-fmt --style both '(+/ ∘ ⍉ ∘ 2 3⍴) v'
glyph:  (+/ ∘ ⍉ ∘ 2 3⍴) v
method: train![reduce(+[1]), transpose, reshape(2, 3)].call(&v)
macro:  train![reduce(+[1]), transpose, reshape(2, 3)].call(&v)

$ arrayglyph-fmt -- '-⌿m'
m.reduce_axis::<0>(Sub)

$ arrayglyph-fmt --to-glyph 'm.max::<0>()' 'reduce!(max[1], v)' 'reduce!(max, v, axis = 1)' 'train![reduce(+[1]), transpose].call(&m)'
⌈⌿m
⌈/v
⌈/v
(+/ ∘ ⍉) m
```

`--style <method|macro|both>` (default `method`) picks the Rust spelling of
reductions and scans; `both` prints labelled glyph, method and macro lines.
`--to-rust` (default) and `--to-glyph` pick the direction. `--to-glyph`
accepts either style and the older spellings. Put `--` before an expression
that starts with `-`.

From Rust, use `arrayglyph::printer::{glyph_to_rust, glyph_to_rust_with,
rust_to_glyph, Style}`.
[`examples/printer_styles.md`](examples/printer_styles.md) compares the two
styles side by side, and [`docs/AGENT_API.md`](docs/AGENT_API.md) records the
agent-facing API decisions (and what is still a sketch).

**Trains.** `train![f, g, h]` is the glyph `(f ∘ g ∘ h)`, with the items **in
the same order as the glyph**, so the rightmost item runs first. Items:
`reduce(op[k])`, `scan(op[k])`, `transpose`, `shape`, `reshape(r, c)`, or any
function value in parentheses (for example a named train). `∘` is
associative, so the printer flattens any grouping: `(+/ ∘ (⍉ ∘ 2 3⍴))` prints
as `(+/ ∘ ⍉ ∘ 2 3⍴)`. `compose(f, g)` remains available as the lower-level API
that `train!` expands to.

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
| `+⌿x`, `+⍀x` (axis 0, down columns) | `x.sum::<0>()`, `x.cumsum::<0>()` · `reduce!(+[0], x)`, `scan!(+[0], x)` |
| `+/x`, `+\x` (axis 1, along rows) | `x.sum::<1>()`, `x.cumsum::<1>()` · `reduce!(+[1], x)`, `scan!(+[1], x)` |
| `× ⌈ ⌊` with `⌿ / ⍀ \` | `prod`/`max`/`min`, `cumprod`/`cummax`/`cummin` (same axis rule) |
| `-⌿x`, `÷\x` | `x.reduce_axis::<0>(Sub)`, `x.scan_axis::<1>(Div)` · `reduce!(-[0], x)`, `scan!(/[1], x)` |
| `f/[0]x`, `f/[1]x` | accepted as input; printed as `f⌿x`, `f/x` |
| `a f b` | `a.zip_with(&b, F)` |
| `⍳n`, `⍴x`, `⍉x` | `iota::<n>()`, `x.shape()`, `x.transpose()` |
| `r c⍴x` | `x.reshape::<r, c>()` |
| `a +.× b` | `a.matmul(&b)` |
| `(f ∘ g ∘ h) x` | `train![f, g, h].call(&x)` with items `reduce(op[k])`, `scan(op[k])`, `transpose`, `shape`, `reshape(r, c)` |
| `1 2 3`, `¯3` | `Array::vector([1, 2, 3])`, `-3` |

## Commands

```sh
cargo test                       # unit, integration, doc and compile_fail tests
cargo clippy --all-targets       # lint (clean)
cargo run --example basics       # quick tour
cargo run --bin arrayglyph-fmt -- '+/a × b'
cargo run --bin arrayglyph-fmt -- --style both '+/a × b'
```

## Examples

Each one is short, commented, and self-contained. The showcase's data,
[`examples/data/weather.csv`](examples/data/weather.csv), is **synthetic
sample data** made up for the demo, not real measurements. A walkthrough of
the showcase, with each glyph next to the Rust it stands for, is
[`examples/SHOWCASE.md`](examples/SHOWCASE.md).
[`examples/WISHLIST.md`](examples/WISHLIST.md) lists the features the
showcase wanted, and which of them now exist.

| example | shows |
|---|---|
| **`cargo run --example showcase`** | **the full tour on a small dataset: a labelled CSV (14 days × 5 cities of synthetic temperatures) loaded into `Array<f64, 14, 5>`, per-city (axis 0) and per-day (axis 1) statistics, a `∘` train for weekly means, `prof!`, and every computation shown as glyph next to the verbose Rust that runs (method style) and its macro spelling** |
| `cargo run --example shaped_array` | construction, shapes, elementwise ops, `matmul`, and the compile error a shape mismatch causes |
| `cargo run --example reduce_axis` | axis 0 (columns) vs axis 1 (rows), as `m.sum::<K>()` and `reduce!(+[K], m)` |
| `cargo run --example compose` | building trains with `∘` / `train![…]` (and the lower-level `compose`) |
| `cargo run --example pretty_print` | glyph → verbose Rust (both styles) and back |
| `cargo run --example basics` | a quick tour of all of the above |

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT), at your option.
