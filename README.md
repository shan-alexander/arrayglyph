# arrayglyph

APL-inspired, compile-time-shaped arrays for Rust. *(Project codename: Iota.)*

`arrayglyph` gives a Rust program the dense vocabulary of APL — `+⌿x` for
column sums, `⌈/x - ⌊/x` for a row-wise range, `(⌈⌿ ∘ +⍀) a` for a named
pipeline — while compiling to ordinary Rust. The shape of every array lives
in the type (`Array<T, ROWS, COLS>`), so a mismatch is a **compile error**,
not a panic or a wrong answer. A bidirectional pretty-printer turns the
glyphs into verbose Rust and back, so a reviewer can read either view.

The crate is a Phase 1 prototype. It is not published to crates.io
(`publish = false`). Add it as a git dependency; see [Install](#install-and-usage).

## Why it exists

Array code is mostly “do this to every row,” “reduce down each column,”
“zip these two, then sum.” In ordinary Rust that becomes loops, index
arithmetic, and shapes that exist only in comments. The compiler will not
stop you from summing a column with a row index, or multiplying a 2×3
matrix by another 2×3. Those bugs show up at runtime, if at all.

APL has spent sixty years naming those whole-array operations with a small,
fixed set of glyphs. `+/` is “sum along the last axis.” `×\` is “running
product.” `+/(a × b)` is a dot product. There are no loop indices, so
off-by-one errors have nowhere to live. The notation is a *tool of thought*:
you write the idea, not the traversal.

`arrayglyph` takes that vocabulary and puts it on top of stable Rust:

* **Zero-cost.** Every glyph lowers to a fold or a loop the optimiser
  already understands. There is no interpreter and no boxed array language
  running beside Rust.
* **Shapes are types.** `Array<f64, 14, 5>` is 14 rows by 5 columns. Elementwise
  ops, `matmul`, and `reshape` that disagree on shape fail to compile.
* **Axes are explicit.** Reductions and scans always name the axis they run
  along, so the result shape is visible at the call site and checked by the
  type system.
* **The compiled form is ordinary Rust.** Glyphs are a notation layer, not a
  second language. What cargo builds is method calls, macros, and iterator
  folds you could have written by hand.

The point is not to make Rust look like APL. It is to make a certain class
of array programs shorter to write, harder to get wrong, and easier for a
human to audit.

## Agents write glyphs, humans read Rust

The crate is designed with **AI agents as the primary authors** of array
code, and humans as the primary reviewers.

An agent pays for every token it emits and every token it reads back. A
column-mean, a range, or a four-stage pipeline is a dozen characters of
glyph text, or a screenful of loops. Dense, regular notation has fewer
places to be wrong, and the same phrase means the same thing every time.

That density is only acceptable in a codebase people still have to own if
it can be opened up. The pretty-printer is the bridge:

* An agent (or a person) writes glyph notation: `+⌿x ÷ 14.0`.
* `arrayglyph-fmt --to-rust` — or `glyph_to_rust` in library form — expands
  it to named, explicit Rust: `x.sum::<0>().zip_with(&Array::splat(14.0), Div)`.
* `--style macro` gives the equally explicit macro spelling
  `reduce!(+[0], x).zip_with(&Array::splat(14.0), Div)`.
* `--style both` prints the glyph and both Rust spellings side by side.
* `--to-glyph` compresses canonical Rust back to glyphs, so a reviewer can
  switch views either way.

What compiles is always ordinary Rust, with visible borrows, const-generic
axes, and trait bounds. Glyphs never hide an allocation or a type. The
agent writes the dense form; the printer shows a person what it stands for;
the type checker keeps both honest.

## When to reach for it (and when not to)

From a pure coding point of view — whether the author is a person or an
agent — arrayglyph is a good fit when most of the following are true:

* The data is a **dense, rectangular, fixed-shape** 2-D array (a table of
  measurements, a small matrix, a batch of vectors). The shape is known
  when you write the program, or at least when it compiles.
* You do **reductions, scans, transposes, reshapes, elementwise ops, or
  matrix products**, and you care which axis each one uses.
* You want a **wrong shape or a wrong axis to be a compile error**, not a
  skipped test.
* You want a pipeline you can **name and reuse** (`train![…]`), not a
  one-off nest of loops.
* You (or an agent) will write **many** such expressions, and the token
  savings plus the printer’s two views are worth a small, unusual vocabulary.

Idiomatic Rust is the better tool when:

* The work is **one-dimensional** and already clear as `iter()`, `zip`, and
  `sum`. A dot product of two `Vec`s is a one-liner either way; the glyph
  form is not a win on its own.
* Shapes are **dynamic or ragged** — runtime lengths, missing cells, lists
  of lists of different sizes. `Array<T, ROWS, COLS>` is const-generic. A
  length that is only known when the file is read belongs in `Vec`,
  `ndarray`, or a type that can fail at runtime.
* You need **rank 3 or higher**, empty reductions with identity elements,
  glyph-level broadcasting, comparisons and compress, or argmax. Those are
  on the roadmap, not in this crate yet.
* You already have a numerical stack (`ndarray`, `nalgebra`, a GPU
  library) and the job is heavy linear algebra, not clearly-shaped
  reductions. arrayglyph is a vocabulary and a type, not a BLAS.
* You would rather never look at a Unicode glyph. The macros and methods
  work without them, but the crate’s intended notation *is* the glyphs.

A fair summary: use it for small, dense, fixed-shape array programs where
getting the axis right matters. Do not use it as a general replacement for
`Vec`, and do not expect it to be a NumPy or an APL interpreter.

## A short APL primer

You do not need to know APL to read the rest of this page, but you do need
the dozen glyphs the crate uses and three rules that are *not* quite APL’s.

### Glyph table

| glyph | name | arrayglyph Rust (method style) | macro style |
|---|---|---|---|
| `+ - × ÷ ⌈ ⌊` | add, sub, mul, div, max, min | `Add Sub Mul Div Max Min` in `zip_with` | tokens `+ - * / max min` |
| `+⌿x` | sum down the columns (axis 0) | `x.sum::<0>()` | `reduce!(+[0], x)` |
| `+/x` | sum along each row (axis 1) | `x.sum::<1>()` | `reduce!(+[1], x)` |
| `×⌿x`, `⌈/x`, `⌊⌿x` | product, max, min (same axis rule) | `x.prod::<0>()`, `x.max::<1>()`, `x.min::<0>()` | `reduce!(*[0], x)`, `reduce!(max[1], x)`, `reduce!(min[0], x)` |
| `-⌿x`, `÷/x` | `-` and `÷` have no named method | `x.reduce_axis::<0>(Sub)`, `x.reduce_axis::<1>(Div)` | `reduce!(-[0], x)`, `reduce!(/[1], x)` |
| `+⍀x`, `×\x`, `⌈\x`, `⌊⍀x` | running sum / product / max / min | `x.cumsum::<0>()`, `x.cumprod::<1>()`, `x.cummax::<1>()`, `x.cummin::<0>()` | `scan!(+[0], x)`, `scan!(*[1], x)`, … |
| `a f b` | elementwise | `a.zip_with(&b, F)` | same |
| `⍳n` | iota, 0-based | `iota::<n>()` | same |
| `⍴x` | shape | `x.shape()` | same |
| `r c⍴x` | reshape | `x.reshape::<r, c>()` | same |
| `⍉x` | transpose | `x.transpose()` | same |
| `a +.× b` | matrix product | `a.matmul(&b)` | same |
| `(f ∘ g ∘ h) x` | compose, apply `h` then `g` then `f` | `train![f, g, h].call(&x)` | same |
| `1 2 3`, `¯3` | strand, high minus | `Array::vector([1, 2, 3])`, `-3` | same |

The bracket forms `f/[0]`, `f/[1]`, `f\[0]`, `f\[1]` are accepted as input
and printed as `⌿ / ⍀ \`. Macro operators are ASCII (`+ - * / max min`)
because Rust’s lexer rejects `×` and `⌈` as tokens.

### Axes

A 2-D array is `R` rows by `C` columns. Axis 0 runs **down each column**;
axis 1 runs **along each row**. Axis 0 is the natural “default” in the
same sense as NumPy’s `axis=0` and APL’s first-axis operators `⌿` and `⍀`.
Canonical arrayglyph code **always writes the axis**, so you never have to
remember a default:

| axis | meaning | reduce | scan | shape |
|---|---|---|---|---|
| 0 | down each column | `+⌿x` → `x.sum::<0>()` | `+⍀x` → `x.cumsum::<0>()` | `R × C → 1 × C` |
| 1 | along each row | `+/x` → `x.sum::<1>()` | `+\x` → `x.cumsum::<1>()` | `R × C → R × 1` |

Reducing both axes is two calls: `+/+⌿x` is `x.sum::<0>().sum::<1>()`.
There are no `reduce_rows!` / `reduce_cols!` macros. A vector is a
`1 × N` array, so its only meaningful axis is 1; `v.sum::<0>()` is a
**compile error** that tells you to use axis 1. An axis other than 0 or 1
is also a compile error.

The older spellings `reduce!(+, x)` (implicit axis 0) and
`reduce!(+, x, axis = 1)` (keyword axis) still compile and still convert
to glyphs. They are deprecated; new code and the printer use the bracket
form `reduce!(+[1], x)` or the method `x.sum::<1>()`.

### Evaluation order and precedence

Glyph text in arrayglyph is read **left to right, with Rust-style
precedence**. That is a deliberate break from APL, which evaluates right
to left and has no precedence. If you already know Rust, `2 × 3 + x` is
`(2 × 3) + x`, not `2 × (3 + x)`.

| binds | forms | example |
|---|---|---|
| loosest | `+ -` | `a - b - c` = `(a - b) - c` |
| | `× ÷ +.×` | `2 × 3 + x` = `(2 × 3) + x` |
| | `⌈ ⌊` (like a method call) | `a ⌈ b + c` = `(a ⌈ b) + c` |
| tightest | prefix `f/ f⌿ f\ f⍀ ⍴ ⍉ r c⍴ ⍳n`, and `(f ∘ g) x` | `+/a × b` = `(+/a) × b` |

So a dot product must be written `+/(a × b)`, with parentheses, just as
you would write `sum(a * b)` rather than `sum(a) * b`.

### Folds run left to right

A reduction is a **left fold**, like Rust’s `Iterator::reduce`, not APL’s
right fold. `-/1 2 3` is `(1 - 2) - 3`, which is `-4`. For associative
ops (`+ × ⌈ ⌊`) the two agree; for `-` and `÷` they do not. The same
rule applies to scans: `-\1 2 3` is `1 ¯1 ¯4`.

### Composition is explicit

APL can hide composition in right-to-left reading: `+/⍉m` *is* “transpose,
then sum.” Under left-to-right evaluation that has to be spelled out.
`(f ∘ g) x` applies `g` first, then `f` — mathematical composition, as in
APL’s `∘` and Haskell’s `.`. The Rust spelling is a flat train in **the
same order as the glyph**, so the rightmost item runs first:

```text
(+/ ∘ ⍉) m
```

```rust,ignore
train![reduce(+[1]), transpose].call(&m)
```

`∘` is associative, so `(+/ ∘ (⍉ ∘ 2 3⍴))` prints as the flat
`(+/ ∘ ⍉ ∘ 2 3⍴)`.

## Worked examples

Each example below is shown as ordinary idiomatic Rust — loops or iterators
over `Vec<Vec<f64>>` — next to the arrayglyph glyph and the verbose Rust
it expands to. The expansions are the pretty-printer’s actual output, not
hand-written paraphrases. The numbers in the weather examples are exactly
what `cargo run --example showcase` prints; the temperatures are
**synthetic** sample data, not real measurements.

A short snippet is a two-column table. A longer one is a pair of labelled
code blocks, which renders more reliably on GitHub.

### A 2×3 warm-up

Take a tiny matrix and ask for column sums, row sums, and the grand total.

```rust
use arrayglyph::prelude::*;

let m = Array::from_rows([[1, 2, 3], [4, 5, 6]]);
assert_eq!(m.sum::<0>().as_slice(), &[5, 7, 9]);      // +⌿m    2×3 → 1×3
assert_eq!(m.sum::<1>().as_slice(), &[6, 15]);         // +/m    2×3 → 2×1
assert_eq!(m.sum::<0>().sum::<1>().item(), 21);        // +/+⌿m  two axes
assert_eq!(reduce!(+[0], m), m.sum::<0>());            // same thing, as a macro
```

The same work as iterators over `Vec<Vec<_>>` is not long, but the shape
is only a convention: `m[row][col]`, sum down `row` for a column total,
sum across `col` for a row total. Swap the two indexes and the program
still compiles. In arrayglyph, `sum::<0>()` returns `Array<T, 1, 3>` and
`sum::<1>()` returns `Array<T, 2, 1>`. Using the wrong one in a later
operation is a type error (E0308), and asking for axis 2 is a type error
(E0277) that says arrays have axes 0 and 1 only.

A left fold of subtraction, and a reusable train that transposes first
and then sums each row — the column sums of `m`, laid out as a column:

```rust
use arrayglyph::prelude::*;

let a = Array::vector([1, 2, 3]);
assert_eq!(reduce!(-[1], a).item(), -4); // -/1 2 3 = (1 - 2) - 3

let m = Array::from_rows([[1, 2, 3], [4, 5, 6]]);
let col_sums = train![reduce(+[1]), transpose];
assert_eq!(col_sums.call(&m).as_slice(), &[5, 7, 9]);
```

Shapes that disagree fail to compile, including inside a train:

```rust
use arrayglyph::prelude::*;
let p = iota::<6>().reshape::<2, 3>();     // ok: 6 == 2*3
let q = p.matmul(&p.transpose());          // (2×3)·(3×2) → 2×2
assert_eq!(q.shape(), (2, 2));
// let bad = p.matmul(&p);                 // compile error: inner dims 3 ≠ 2
// let bad = iota::<6>().reshape::<4, 2>(); // compile error: 6 ≠ 8
```

### The weather dataset

The rest of the examples use the showcase’s table: **14 days × 5 cities**
of synthetic daily highs in °C (Oslo, London, New York, Tokyo, Sydney),
loaded into `Array<f64, 14, 5>`. Rows are days, columns are cities. Run
the full tour with `cargo run --example showcase`. A walkthrough of every
step is in [`examples/SHOWCASE.md`](examples/SHOWCASE.md).

```text
                  Oslo  London  New York   Tokyo  Sydney
  Nov 10           2.6    11.8       9.6    14.5    21.0
  Nov 11           2.2    13.1      10.3    16.2    22.5
  …
  Nov 23           0.4    10.9       3.3    10.0    23.5
```

In the plain-Rust columns below, `rows` is a `Vec<Vec<f64>>` with
`rows[day][city]`, 14 rows of 5.

### Column means (axis 0)

Average high, per city, over the 14 days. That is a sum **down each
column**, then division by 14.

**Idiomatic Rust**

```rust,ignore
let means: Vec<f64> = (0..5)
    .map(|j| rows.iter().map(|row| row[j]).sum::<f64>() / 14.0)
    .collect();
```

**Glyph, and what it stands for**

```text
+⌿x ÷ 14.0
```

```rust,ignore
x.sum::<0>().zip_with(&Array::splat(14.0), Div)
// macro style:
reduce!(+[0], x).zip_with(&Array::splat(14.0), Div)
// hand-written equivalent:
x.mean::<0>()
```

Prefix `+⌿` binds tighter than `÷`, so no parentheses are needed: this is
`(+⌿x) ÷ 14.0`. The result is `1 × 5`, one mean per city. There is no
mean glyph; the printer translates the sum-and-divide literally.
`x.mean::<0>()` is the same value, written by hand — the printer does not
emit `mean`, because that is not what the glyph says.

Why the arrayglyph form is better here: the axis is in the glyph (`⌿`) and
in the Rust (`::<0>()`), so “per city” cannot be confused with “per day.”
The type of the result is `Array<f64, 1, 5>`. Subtracting it from the
14×5 table without broadcasting is a compile error, not a silently
misaligned loop. The iterator version is fine, but `row[j]` and `14.0`
are obligations the compiler does not check.

```text
                  Oslo  London  New York   Tokyo  Sydney
  mean             2.1     9.4       7.5    13.6    24.4
```

### Range: warmest minus coldest, per city

Two reductions, then an elementwise subtract. Infix `-` is looser than
prefix `⌿`, so `⌈⌿x - ⌊⌿x` is `(⌈⌿x) - (⌊⌿x)`.

**Idiomatic Rust**

```rust,ignore
let range: Vec<f64> = (0..5)
    .map(|j| {
        let col = rows.iter().map(|row| row[j]);
        col.clone().fold(f64::NEG_INFINITY, f64::max)
            - col.fold(f64::INFINITY, f64::min)
    })
    .collect();
```

**Glyph, and what it stands for**

```text
⌈⌿x - ⌊⌿x
```

```rust,ignore
x.max::<0>().zip_with(&x.min::<0>(), Sub)
// macro style:
reduce!(max[0], x).zip_with(&reduce!(min[0], x), Sub)
```

The glyph is one phrase: “column max minus column min.” The iterator
version has to say “for each column index, fold max and fold min, then
subtract,” and it has to get `NEG_INFINITY` / `INFINITY` right. Both
reductions in arrayglyph produce `1 × 5`, so their difference is `1 × 5`.
If one of them were accidentally axis 1, the `zip_with` would not compile.

```text
                  Oslo  London  New York   Tokyo  Sydney
  warmest day      3.2    13.1      10.7    16.9    28.6
  range            3.3     7.2       7.4     6.9     7.6
```

Sydney posts the highest average (24.4 °C) and the single warmest reading
(28.6 °C). Oslo is the coolest, and the tightest range (3.3 °C).

### Row means and daily spread (axis 1)

The same two questions, now **per day**: the five-city mean, and the gap
that day between the warmest city and the coldest. The axis flips from
`⌿` to `/`, and in Rust from `::<0>()` to `::<1>()`.

**Idiomatic Rust**

```rust,ignore
let daily: Vec<f64> = rows
    .iter()
    .map(|row| row.iter().sum::<f64>() / 5.0)
    .collect();
let spread: Vec<f64> = rows
    .iter()
    .map(|row| {
        row.iter().copied().fold(f64::NEG_INFINITY, f64::max)
            - row.iter().copied().fold(f64::INFINITY, f64::min)
    })
    .collect();
```

**Glyph, and what it stands for**

```text
+/x ÷ 5.0
⌈/x - ⌊/x
```

```rust,ignore
x.sum::<1>().zip_with(&Array::splat(5.0), Div)
x.max::<1>().zip_with(&x.min::<1>(), Sub)
// macro style:
reduce!(+[1], x).zip_with(&Array::splat(5.0), Div)
reduce!(max[1], x).zip_with(&reduce!(min[1], x), Sub)
```

This is the best argument for explicit axes. The plain-Rust column-mean
loop and the row-mean loop look alike except for which index moves. A
tired author, or an agent that has just written `+/` for a vector, can
flip the axis and still type-check. In arrayglyph the two programs have
different glyphs, different method turbofish, and different result types
(`1 × 5` vs `14 × 1`). Mixing them is a compile error.

```text
                 5-city mean   warmest − coldest
  Nov 10                11.9                18.4
  Nov 11                12.9                20.3
  …
  Nov 17                11.7                27.1
  Nov 18                11.1                27.1
  …
  Nov 23                 9.6                23.1
```

The five-city mean peaks early (12.9 °C on Nov 11) and trends down. The
largest spread, 27.1 °C, falls on Nov 17–18, when Sydney is in the high
twenties and Oslo is near freezing.

### Grand mean

Reducing both axes is two calls. For `+` the order does not matter.

| | first columns, then that row | first rows, then that column |
|---|---|---|
| Glyph | `+/+⌿x` | `+⌿+/x` |
| Method style | `x.sum::<0>().sum::<1>()` | `x.sum::<1>().sum::<0>()` |
| Macro style | `reduce!(+[1], reduce!(+[0], x))` | `reduce!(+[0], reduce!(+[1], x))` |

**Idiomatic Rust**

```rust,ignore
let n = rows.len() * rows[0].len();          // 70
let total: f64 = rows.iter().flatten().sum();
let grand = total / n as f64;                // 11.39
```

The iterator `flatten().sum()` is genuinely shorter for a single scalar
total, and it is a reasonable choice. The arrayglyph form is worth it
when the partial results (the 1×5 column sums, or the 14×1 row sums)
are also going to be used, or when you want the intermediate shapes
checked. A vector is `1 × N`, so the APL habit `+/+/x` would try to
reduce a 14×1 column along axis 1 and **would not compile**; the working
spelling is `+/+⌿x`.

```text
  grand mean over all 70 readings: 11.39 °C
```

### Weekly means, as a train

The longest pipeline in the showcase: two weekly means per city, written
as one composed function. The glyph is read **right to left inside the
parentheses** (because `∘` applies the right-hand side first). The
`train!` list is in the same order.

```text
(5 2⍴ ∘ +/ ∘ 10 7⍴ ∘ ⍉) x ÷ 7.0
```

```rust,ignore
train![reshape(5, 2), reduce(+[1]), reshape(10, 7), transpose]
    .call(&x)
    .zip_with(&Array::splat(7.0), Div)
```

1. `⍉` transposes, 14×5 → 5×14: one row per city.
2. `10 7⍴` reshapes that to 10×7: two consecutive 7-day blocks per city,
   stacked as rows.
3. `+/` sums each of those rows (axis 1): 10×7 → 10×1, the weekly totals.
4. `5 2⍴` reshapes back to 5×2: one row per city, one column per week.
5. `÷ 7.0` turns totals into means. Prefix application binds tighter than
   `÷`, so the division applies to the whole result of the train.

Every intermediate shape is a type. A typo such as `10 8⍴` (or
`reshape(10, 8)`) would not compile, because 5×14 has 70 elements and
10×8 has 80. The same is true of `5 2⍴` against a 10×1 column.

**Idiomatic Rust**

```rust,ignore
let mut weekly = vec![vec![0.0; 2]; 5];
for city in 0..5 {
    for week in 0..2 {
        let start = week * 7;
        weekly[city][week] =
            (0..7).map(|d| rows[start + d][city]).sum::<f64>() / 7.0;
    }
}
```

Honesty is required here. For this particular question, the nested loop
is easier to read than the reshape dance. “For each city and each week,
average days `week*7 .. week*7+7`” is what the computation *is*. The
train is denser, it is one reusable value, and the compiler will reject a
wrong element count — but it asks you to think in transposes and
block-reshapes rather than in city and week. That is a real trade: reach
for the train when the pipeline will be reused or the shape checks matter
more than the narrative; reach for the loop when the narrative *is* the
code. (A helper that split an axis into chunks would narrow this gap; it
is on the wishlist, not in the crate today.)

```text
                  week 1    week 2    change
  Oslo               2.6       1.6      -1.0
  London            10.3       8.6      -1.7
  New York           8.5       6.4      -2.1
  Tokyo             14.9      12.3      -2.6
  Sydney            24.0      24.8      +0.8
```

Four of the five cities cool off in the second week; only Sydney warms
slightly (+0.8 °C).

### The strongest warm spell

A “warm spell” here is the running sum of each city’s daily **anomaly**
— the departure from that city’s own mean — and then the maximum that
running sum reaches. In °C·days, that is how far above “normal” the
warmest stretch got.

Glyph text cannot yet broadcast a 1×5 row across a 14×5 array, so the
anomaly itself is written in Rust. `mean.broadcast()` stretches the
column means down all 14 rows; `zip_with(..., Sub)` subtracts them
elementwise.

```rust,ignore
let a = x.zip_with(&mean.broadcast(), Sub);
```

**Idiomatic Rust** (anomaly and peak in one go)

```rust,ignore
let mut peak = vec![f64::NEG_INFINITY; 5];
for j in 0..5 {
    let mut run = 0.0;
    for row in &rows {
        run += row[j] - means[j];
        if run > peak[j] {
            peak[j] = run;
        }
    }
}
```

**Glyph, and what it stands for**

```text
(⌈⌿ ∘ +⍀) a
```

```rust,ignore
train![reduce(max[0]), scan(+[0])].call(&a)
```

`+⍀` is the running sum **down each column** (an axis-0 scan: the shape
stays 14×5). Then `⌈⌿` takes the largest value of that running sum in
each city. The two items name their axes, `+[0]` and `max[0]`, matching
the glyphs `⍀` and `⌿`. The train is a value: you can name it, copy it,
and apply it to another anomaly table without rewriting the loop.

The plain-Rust version is a textbook running-max, and it is clear. Its
risks are the usual ones: the wrong index on `means[j]`, a `run`
initialised to the wrong thing, a `>` that should have been `>=`. The
arrayglyph version cannot mix a column scan with a row max without a
shape error, and it states the idea in the same order as a reviewer would
say it: “running column sum, then column max.”

```text
                  Oslo  London  New York   Tokyo  Sydney
  peak warmth      4.3     9.1       8.2     9.1     5.5
  strongest warm spell: Tokyo (+9.1 °C·days above its own mean)
```

Tokyo and London tie on the printed one-decimal figures; Tokyo is
slightly ahead and is the one the showcase reports.

## The pretty-printer and `arrayglyph-fmt`

Glyphs are the primary notation. The printer has two Rust styles; both
**always write the axis**. Method style is the default.

| glyph | `--style method` (default) | `--style macro` |
|---|---|---|
| `+⌿m` | `m.sum::<0>()` | `reduce!(+[0], m)` |
| `+/m` | `m.sum::<1>()` | `reduce!(+[1], m)` |
| `×\m` | `m.cumprod::<1>()` | `scan!(*[1], m)` |
| `⌈⌿m - ⌊⌿m` | `m.max::<0>().zip_with(&m.min::<0>(), Sub)` | `reduce!(max[0], m).zip_with(&reduce!(min[0], m), Sub)` |
| `-⌿m` | `m.reduce_axis::<0>(Sub)` | `reduce!(-[0], m)` |
| `(+/ ∘ ⍉) m` | `train![reduce(+[1]), transpose].call(&m)` | same |
| `(+/ ∘ ⍉ ∘ 2 3⍴) v` | `train![reduce(+[1]), transpose, reshape(2, 3)].call(&v)` | same |

The named methods use a **const-generic axis** (`m.sum::<0>()`, not
`m.sum(0)`), so the result type is known at compile time. `-` and `÷`
have no named method; method style falls back to `reduce_axis` /
`scan_axis`, and macro style covers them uniformly.

From a checkout:

```sh
cargo run --bin arrayglyph-fmt -- '+/(a × b)'
# a.zip_with(&b, Mul).sum::<1>()

cargo run --bin arrayglyph-fmt -- --style macro '+/(a × b)'
# reduce!(+[1], a.zip_with(&b, Mul))

cargo run --bin arrayglyph-fmt -- --style both '(+/ ∘ ⍉ ∘ 2 3⍴) v'
# glyph:  (+/ ∘ ⍉ ∘ 2 3⍴) v
# method: train![reduce(+[1]), transpose, reshape(2, 3)].call(&v)
# macro:  train![reduce(+[1]), transpose, reshape(2, 3)].call(&v)

cargo run --bin arrayglyph-fmt -- -- '-⌿m'
# m.reduce_axis::<0>(Sub)

cargo run --bin arrayglyph-fmt -- --to-glyph 'm.max::<0>()'
# ⌈⌿m
```

`--to-rust` (default) and `--to-glyph` pick the direction and cannot be
combined. `--to-glyph` accepts either style and the older spellings
(`reduce!(+, x, axis = 1)`, `compose(…)`). Put `--` before an expression
that starts with `-`, as with any clap CLI. With no expressions, the tool
reads one per line from stdin.

From Rust, use `arrayglyph::printer::{glyph_to_rust, glyph_to_rust_with,
rust_to_glyph, Style}`. A side-by-side of the two styles, with every
snippet taken from the printer, is
[`examples/printer_styles.md`](examples/printer_styles.md).

## `train!` and `prof!`

`train![f, g, h]` is the glyph `(f ∘ g ∘ h)`. Items are listed in **glyph
order**, so the rightmost runs first. That is a judgment call: listing
them in data-flow order would read like a Unix pipe, but it would reverse
every train relative to its glyph. A reader who knows `∘` already knows
how to read `train!`.

Items: `reduce(op[k])`, `scan(op[k])`, `transpose`, `shape`,
`reshape(r, c)`, or any function value in parentheses (for example a
named train). The macro expands to left-nested `compose` calls;
`compose(f, g)` stays public as the lower-level API, and the printer
still reads it.

```rust
use arrayglyph::prelude::*;

// glyph: (+/ ∘ ⍉ ∘ 2 3⍴) ⍳6
let f = train![reduce(+[1]), transpose, reshape(2, 3)];
assert_eq!(f.call(&iota::<6>()).as_slice(), &[3, 5, 7]);
```

`prof!(expr)` evaluates the expression once, times it with
`std::time::Instant`, prints a one-line report to **stderr**, and returns
the value unchanged. `prof!(glyph = "…", expr)` also prints the glyph the
expression stands for. The shape comes from the type (`2×3`, `scalar`,
`shape (2, 3)`). It is a stopwatch, not a benchmark harness — one run,
wall-clock time, no warm-up.

```rust
use arrayglyph::prelude::*;

let m = Array::from_rows([[1, 2, 3], [4, 5, 6]]);
let cols = prof!(glyph = "+⌿m", m.sum::<0>());
assert_eq!(cols.as_slice(), &[5, 7, 9]);
```

```text
[prof] src/main.rs:5  +⌿m  ⇐  m.sum::<0>()  → 1×3  in 1.2µs
```

The elapsed time varies from run to run.

## Install and usage

The crate is **not on crates.io**. Add it as a git dependency. You need
Rust 1.85 or later (the crate is edition 2024).

```toml
[dependencies]
arrayglyph = { git = "https://github.com/shan-alexander/arrayglyph" }
```

Or: `cargo add arrayglyph --git https://github.com/shan-alexander/arrayglyph`.

```rust
use arrayglyph::prelude::*;

let a = Array::vector([1, 2, 3]);
let b = Array::vector([4, 5, 6]);
// +/(a × b)  — a dot product; vectors are 1×N, so they fold along axis 1
assert_eq!(a.zip_with(&b, Mul).sum::<1>().item(), 32);
```

From a clone of the repository:

```sh
git clone https://github.com/shan-alexander/arrayglyph
cd arrayglyph
cargo test
cargo run --example showcase
cargo run --bin arrayglyph-fmt -- --style both '+/a × b'
```

The binary can also be installed with
`cargo install --git https://github.com/shan-alexander/arrayglyph --bin arrayglyph-fmt`.

## Status and roadmap

**Phase 1** (this milestone) is a working prototype: `Array<T, R, C>`,
views, elementwise ops, matmul, transpose, reshape, axis-explicit
reductions and scans (`m.sum::<K>()`, `reduce!(+[K], m)` and the `cum*` /
`prod` / `max` / `min` family), flat `train![…]`, `prof!`, the
bidirectional printer with method and macro styles, and the
`arrayglyph-fmt` CLI. Convenience APIs that do not yet have glyph forms:
`broadcast::<R, C>()`, `mean::<K>()`, scalar operators (`&m * 2.0`),
precision-aware `Display`, and `table::Table`, a small labelled-CSV
loader that checks the data’s shape once at load time.

**Phase 2** is planned to add a general inner product and outer product
(`a ∘.× b`), forks and atops such as `(+/ ÷ ≢)`, optional Rayon
parallel variants for associative reductions, richer span rendering in
the printer, and property tests against a loop-level reference.

**Phase 3+** would add rank-N arrays, identity elements for empty
reductions, more glyphs (catenate, take/drop, reverse, tally), and
possibly a `glyph!{ … }` proc-macro that parses glyph source at compile
time through the same parser the printer uses.

[`examples/WISHLIST.md`](examples/WISHLIST.md) lists features the
showcase wanted and does not have yet, including broadcasting in glyph
text, argmax, comparisons, and labels that survive a reduction. Open
design questions and the full rationale live in [DESIGN.md](DESIGN.md).

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT), at your option.

## Further reading

| | |
|---|---|
| [`examples/SHOWCASE.md`](examples/SHOWCASE.md) | walkthrough of the weather example, with every glyph next to the Rust that runs |
| `cargo run --example showcase` | the same tour, executed |
| `cargo run --example basics` | a short tour of shapes, reductions, trains, and the printer |
| `cargo run --example reduce_axis` | axis 0 vs axis 1, as methods and as macros |
| `cargo run --example compose` | building and reusing a `train!` |
| `cargo run --example pretty_print` | glyph → Rust (both styles) and back |
| [`examples/printer_styles.md`](examples/printer_styles.md) | method style vs macro style, all snippets from the printer |
| [DESIGN.md](DESIGN.md) | rationale, semantics, decisions, and the roadmap |
| [`docs/AGENT_API.md`](docs/AGENT_API.md) | agent-facing API notes: what is decided, and what is still a sketch |
