# Pretty-printer styles: a visual comparison

The printer turns one glyph expression into Rust in two styles. Both are
real, compile, and are produced by `arrayglyph-fmt`. Every Rust snippet below
is printer output, copied verbatim.

| | **Method style** (default) | **Macro style** |
|---|---|---|
| Spelling | named methods with a const-generic axis: `m.sum::<0>()`, `m.cumprod::<1>()` | macros with the axis in brackets: `reduce!(+[0], m)`, `scan!(*[1], m)` |
| CLI | `arrayglyph-fmt '+⌿m'` or `--style method` | `arrayglyph-fmt --style macro '+⌿m'` |
| Library | `glyph_to_rust(src)` / `Expr::to_rust()` | `glyph_to_rust_with(src, Style::Macro)` / `Expr::to_rust_styled(Style::Macro)` |
| `-` and `÷` | no named method: `m.reduce_axis::<0>(Sub)` | `reduce!(-[0], m)`, `reduce!(/[0], m)` |

`arrayglyph-fmt --style both` prints the glyph and both spellings as labelled
lines. Glyphs stay the primary notation; the two styles are ways of reading
them as Rust.

Shared rules:

- **The axis is always written.** Axis 0 = down the columns (`R × C → 1 × C`,
  glyph `⌿` / `⍀`), axis 1 = along each row (`R × C → R × 1`, glyph `/` /
  `\`). The bracket is APL's own axis syntax: `reduce!(+[0], m)` is `+/[0] m`.
- **Const-generic axis, not a runtime argument.** `m.sum::<0>()` rather than
  `m.sum(0)`, so the result type (`1 × C` vs `R × 1`) is known at compile time
  and a wrong axis or a shape mismatch is a compile error.
- **Trains are the same in both styles:** a flat `train![…]` listed in glyph
  order, so the rightmost item runs first (see §6).
- The older keyword form `reduce!(+, m, axis = 0)` and the default-axis form
  `reduce!(+, m)` still compile and still convert to glyphs, but they are
  deprecated and the printer never emits them. There are no
  `reduce_rows!` / `reduce_cols!` macros.

---

## 1. Sum over axis 0 (down the columns)

`R × C → 1 × C`.

| Glyph | Method style | Macro style |
|---|---|---|
| `+⌿m` | `m.sum::<0>()` | `reduce!(+[0], m)` |

## 2. Product scan over axis 1 (along each row)

Running product; the shape is unchanged.

| Glyph | Method style | Macro style |
|---|---|---|
| `×\m` | `m.cumprod::<1>()` | `scan!(*[1], m)` |

## 3. Mean over axis 0 (per column)

There is no mean glyph; the glyph is "sum, then divide by the axis length".
The printer translates exactly that. The library also has `m.mean::<0>()`
(and `m.mean::<1>()`), which has no glyph form, so the printer neither emits
nor reads it.

| Glyph | Method style | Macro style |
|---|---|---|
| `+⌿m ÷ 3` | `m.sum::<0>().zip_with(&Array::splat(3), Div)` | `reduce!(+[0], m).zip_with(&Array::splat(3), Div)` |

Hand-written equivalent: `m.mean::<0>()` (for a 3-row `m`).

## 4. Nested: mean of the column sums

Column sums first (`+⌿m`, `R × C → 1 × C`), then the sum of that row
(`+/`, `1 × C → 1 × 1`), divided by the number of columns (3 here). Prefix
binds tighter than `÷`.

| Glyph | Method style | Macro style |
|---|---|---|
| `+/+⌿m ÷ 3` | `m.sum::<0>().sum::<1>().zip_with(&Array::splat(3), Div)` | `reduce!(+[1], reduce!(+[0], m)).zip_with(&Array::splat(3), Div)` |

Hand-written equivalent: `m.sum::<0>().mean::<1>()`. Method style reads left
to right in evaluation order; macro style nests inside out, like the glyph.

## 5. Range: max minus min, axis 0

Two reductions, then an elementwise subtract. Infix `-` is looser than prefix
`⌿`, so this is `(⌈⌿m) - (⌊⌿m)`.

| Glyph | Method style | Macro style |
|---|---|---|
| `⌈⌿m - ⌊⌿m` | `m.max::<0>().zip_with(&m.min::<0>(), Sub)` | `reduce!(max[0], m).zip_with(&reduce!(min[0], m), Sub)` |

## 6. Compose: row sums of a transpose (column sums as a column)

`(f ∘ g) m` applies `g` first, then `f`. `+/` is axis 1.

| Glyph | Method style | Macro style |
|---|---|---|
| `(+/ ∘ ⍉) m` | `train![reduce(+[1]), transpose].call(&m)` | same |
| `(+/ ∘ ⍉ ∘ 2 3⍴) v` | `train![reduce(+[1]), transpose, reshape(2, 3)].call(&v)` | same |

`train![f, g, h]` lists the functions **in glyph order** (rightmost runs
first), the same order as `f ∘ g ∘ h`. That was a judgment call: data-flow
order (`train![reshape(2, 3), transpose, reduce(+[1])]`) reads in execution
order but would reverse every train relative to its glyph. The value is the
same as `m.transpose().sum::<1>()`. Nested grouping such as `(+/ ∘ (⍉ ∘ 2
3⍴))` prints flat, because `∘` is associative. `compose(f, g)` is still
available as the lower-level API that `train!` expands to.

---

## Short take

- **Method style** (the default) is shorter for single reductions and reads
  left to right like the evaluation. It is what `glyph_to_rust` and the CLI
  print unless asked otherwise.
- **Macro style** keeps the operator glyph-like (`+[0]`, `max[1]`) and nests
  in the same order as the glyph. It covers `-` and `÷` without falling back
  to `reduce_axis`.
- Both always name the axis, both keep compile-time shapes, and both round
  trip: glyph → either style → glyph is the identity on the test corpus.
- See `docs/AGENT_API.md` for the agent-facing API sketch, which now builds on
  these forms.
