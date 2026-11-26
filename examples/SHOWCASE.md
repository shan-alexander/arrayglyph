# Showcase walkthrough

This is a guided reading of [`showcase.rs`](showcase.rs): two weeks of daily
highs, computed once as dense APL-style glyphs and once as the verbose Rust
they stand for. A `show!` macro in the example asserts that the
pretty-printer's translation of each glyph string (method style, the
default) is token-for-token the Rust that runs, so the two sides cannot
drift. It also prints the macro-style spelling (`arrayglyph-fmt --style
macro`), which is shown below as **Macro style**. Both styles always write
the axis.

## The dataset

[`data/weather.csv`](data/weather.csv) holds **synthetic** daily high
temperatures in °C: **14 days × 5 cities** (Oslo, London, New York, Tokyo,
Sydney), from 10–23 November. The file itself is labeled as sample data made
up for this demo; these are **not** real measurements.

It is loaded with `include_str!` (no file I/O, no extra crates) into a
[`table::Table`](../src/table.rs), which checks the CSV's shape once and then
hands over a const-shaped array. From that point the shape lives in the type:

```rust
const WEATHER_CSV: &str = include_str!("data/weather.csv");
const DAYS: usize = 14;
const CITIES: usize = 5;

let table: Table<f64, DAYS, CITIES> =
    Table::parse_csv(WEATHER_CSV).expect("weather.csv is 14 days × 5 cities");
let x: Array<f64, DAYS, CITIES> = table.data;
```

Run the example with:

```sh
cargo run --example showcase
```

The printed table starts like this (rows are days, columns are cities):

```text
                  Oslo  London  New York   Tokyo  Sydney
  Nov 10           2.6    11.8       9.6    14.5    21.0
  Nov 11           2.2    13.1      10.3    16.2    22.5
  Nov 12           2.8    12.4       7.2    16.9    24.0
  …
  Nov 23           0.4    10.9       3.3    10.0    23.5
```

A few glyph-reading rules come up throughout. `⌿` and `⍀` work **down each
column** (axis 0: a 14×5 array becomes 1×5; Rust `x.sum::<0>()` or
`reduce!(+[0], x)`). `/` and `\` work **along each row** (axis 1: 14×5
becomes 14×1; `x.sum::<1>()` or `reduce!(+[1], x)`). Prefix forms bind tighter
than any infix operator, so `+⌿x ÷ 14.0` is `(+⌿x) ÷ 14.0`. `∘` is
mathematical composition: `(f ∘ g) x` applies `g` first, then `f`. In Rust
a train is `train![f, g]`, listed in the same order as the glyph.

## 1. Per city: means, warmest day, and range

Axis 0 folds **down each column**, so each city gets one number.

**Glyph**

```text
+⌿x ÷ 14.0
```

**Rust**

```rust
x.sum::<0>().zip_with(&Array::splat(14.0), Div)
```

**Macro style**

```rust
reduce!(+[0], x).zip_with(&Array::splat(14.0), Div)
```

`+⌿x` is the column sums (14×5 → 1×5). Division by 14.0, the number of days,
turns those into means. The `÷` is looser than the prefix `+⌿`, so no
parentheses are needed. The same values are also `x.mean::<0>()`.

**Glyph**

```text
⌈⌿x
```

**Rust**

```rust
x.max::<0>()
```

**Macro style**

```rust
reduce!(max[0], x)
```

`⌈` is max. This is the warmest day in each city over the two weeks.

**Glyph**

```text
⌈⌿x - ⌊⌿x
```

**Rust**

```rust
x.max::<0>().zip_with(&x.min::<0>(), Sub)
```

**Macro style**

```rust
reduce!(max[0], x).zip_with(&reduce!(min[0], x), Sub)
```

`⌊` is min. The two reductions are 1×5, so their difference is the range of
each city's temperatures. Infix `-` is looser than the prefix `⌿` forms, so
this is `(⌈⌿x) - (⌊⌿x)`.

```text
                  Oslo  London  New York   Tokyo  Sydney
  mean             2.1     9.4       7.5    13.6    24.4
  warmest day      3.2    13.1      10.7    16.9    28.6
  range            3.3     7.2       7.4     6.9     7.6
```

Sydney is the warmest on average (24.4 °C) and also posts the single highest
reading (28.6 °C). Oslo is the coolest, and the tightest range (3.3 °C).

## 2. Per day: five-city mean and spread

Axis 1 folds **along each row**, so each day gets one number.

**Glyph**

```text
+/x ÷ 5.0
```

**Rust**

```rust
x.sum::<1>().zip_with(&Array::splat(5.0), Div)
```

**Macro style**

```rust
reduce!(+[1], x).zip_with(&Array::splat(5.0), Div)
```

`+/x` is the row sums (14×5 → 14×1). Dividing by 5.0, the number of cities,
gives the five-city mean for each day. In Rust the axis is written out:
`sum::<1>()`, or `+[1]` in the macro.

**Glyph**

```text
⌈/x - ⌊/x
```

**Rust**

```rust
x.max::<1>().zip_with(&x.min::<1>(), Sub)
```

**Macro style**

```rust
reduce!(max[1], x).zip_with(&reduce!(min[1], x), Sub)
```

Same pattern as the per-city range, but with `/` (axis 1) instead of `⌿`:
the gap, each day, between the warmest city and the coldest.

```text
                 5-city mean   warmest − coldest
  Nov 10                11.9                18.4
  Nov 11                12.9                20.3
  …
  Nov 17                11.7                27.1
  …
  Nov 23                 9.6                23.1
```

The five-city mean peaks early (12.9 °C on Nov 11) and trends down. The
largest spread, 27.1 °C, falls on Nov 17–18, when Sydney is in the high
twenties and Oslo is near freezing.

## 3. Grand mean

Reducing both axes is two calls. For `+` the order does not matter.

**Glyph**

```text
+/+⌿x
```

**Rust**

```rust
x.sum::<0>().sum::<1>()
```

**Macro style**

```rust
reduce!(+[1], reduce!(+[0], x))
```

**Glyph**

```text
+⌿+/x
```

**Rust**

```rust
x.sum::<1>().sum::<0>()
```

**Macro style**

```rust
reduce!(+[0], reduce!(+[1], x))
```

`+/+⌿x` first sums down the columns (`+⌿`, 14×5 → 1×5) and then along that
remaining row (`+/`, 1×5 → 1×1). `+⌿+/x` does the row sums first. Either
total, divided by 70 readings, is the grand mean:

```text
  grand mean over all 70 readings: 11.39 °C
```

A vector is 1×N, so a length-1 axis is a compile error rather than a silent
no-op. The APL habit `+/+/x` would try to reduce a 14×1 column along axis 1
and would not compile; the working spelling is `+/+⌿x`.

## 4. Weekly means per city, as one train

This is the longest pipeline in the example: one composed function that
reshapes two weeks into two weekly means per city.

**Glyph**

```text
(5 2⍴ ∘ +/ ∘ 10 7⍴ ∘ ⍉) x ÷ 7.0
```

**Rust**

```rust
train![reshape(5, 2), reduce(+[1]), reshape(10, 7), transpose]
    .call(&x)
    .zip_with(&Array::splat(7.0), Div)
```

The train is the same in both styles. `train![…]` lists its functions in
the same order as the glyph, and `∘` is applied from the right, so read both
right to left:

1. `⍉` transposes, 14×5 → 5×14: one row per city.
2. `10 7⍴` reshapes that to 10×7: two consecutive 7-day blocks per city,
   stacked as rows.
3. `+/` sums each of those rows (axis 1): 10×7 → 10×1, the weekly totals.
4. `5 2⍴` reshapes back to 5×2: one row per city, one column per week.

Then `÷ 7.0` turns totals into means. Prefix application binds tighter than
`÷`, so the division applies to the whole result of the train, not only to
`x`. Every intermediate shape is a type: a typo such as `10 8⍴` (or
`reshape(10, 8)`) would not compile.

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

## 5. Strongest warm spell

The anomaly `a` is each day's departure from that city's own mean. Glyph
text cannot yet broadcast a 1×5 row across a 14×5 array (see
[WISHLIST.md](WISHLIST.md)), so this step is written in Rust:

```rust
let a = x.zip_with(&mean.broadcast(), Sub);
```

`mean.broadcast()` stretches the 1×5 city means down all 14 rows, and
`zip_with(..., Sub)` subtracts them elementwise.

The spell itself is a two-function train:

**Glyph**

```text
(⌈⌿ ∘ +⍀) a
```

**Rust**

```rust
train![reduce(max[0]), scan(+[0])].call(&a)
```

`+⍀` is the running sum **down each column** (axis-0 scan: the shape stays
14×5). Then `⌈⌿` takes the largest value of that running sum in each city.
The result is how far, in °C·days, the warmest spell sat above that city's
own mean. The items name their axes, `max[0]` and `+[0]`, matching the
glyphs `⌿` and `⍀`.

```text
                  Oslo  London  New York   Tokyo  Sydney
  peak warmth      4.3     9.1       8.2     9.1     5.5
  strongest warm spell: Tokyo (+9.1 °C·days above its own mean)
```

Tokyo and London tie on the printed one-decimal figures; Tokyo is slightly
ahead and is the one the example reports.

## Rust → glyph

The printer also compresses canonical verbose Rust back to glyphs. It reads
either style and the older spellings (`reduce!(+, x, axis = 1)`,
`compose(…)`), so code written before the canonical forms still converts:

**Rust**

```rust
x.max::<0>().zip_with(&x.min::<0>(), Sub)
```

**Glyph**

```text
⌈⌿x - ⌊⌿x
```

**Rust**

```rust
reduce!(+[1], x)
reduce!(+, x, axis = 1)   // deprecated spelling, same glyph
```

**Glyph**

```text
+/x
```

**Rust**

```rust
train![reduce(max[0]), scan(+[0])].call(&a)
compose(Reduce(Max), Scan(Add)).call(&a)   // lower-level API, same glyph
```

**Glyph**

```text
(⌈⌿ ∘ +⍀) a
```

The bracket in `reduce!(+[1], x)` is APL's own axis syntax, `+/[1] x`, so
`[1]` selects `/` and `[0]` selects `⌿`.

## 6. Profiling one step

`prof!` evaluates an expression once, times it with `std::time::Instant`,
prints one line to stderr, and returns the value. The optional `glyph = "…"`
puts the glyph next to the Rust in the report:

```rust
let weekly_again = prof!(
    glyph = "(5 2⍴ ∘ +/ ∘ 10 7⍴ ∘ ⍉) x",
    train![reshape(5, 2), reduce(+[1]), reshape(10, 7), transpose].call(&x)
);
```

```text
[prof] examples/showcase.rs:180  (5 2⍴ ∘ +/ ∘ 10 7⍴ ∘ ⍉) x  ⇐  train![reshape(5, 2), reduce(+[1]), reshape(10, 7), transpose].call(&x)  → 5×2  in 2.781µs
```

The time varies from run to run; the shape (`5×2`) comes from the type.

## Earlier additions

Five small APIs landed while the showcase was being written. Each has rustdoc
and tests; [WISHLIST.md](WISHLIST.md) lists them alongside the features the
showcase still wanted.

- **`Display` honours precision and width.** `{:.1}` and `{:6}` apply to
  every element, and columns stay right-aligned:
  `format!("{temps:.1}")`.
- **`Array::broadcast`.** Stretch a length-1 axis to a larger shape, checked
  at compile time: `x.zip_with(&mean.broadcast(), Sub)`.
- **`mean::<K>()`.** `m.mean::<0>()` for per-column means, `m.mean::<1>()`
  for per-row means. `+⌿x ÷ 14.0` is also `x.mean::<0>()`. (This was
  `mean()` / `mean_axis::<K>()` until the axis became explicit everywhere.)
- **Scalar operators.** Elementwise `+ - * /` with a scalar on the right,
  without spelling out `Array::splat`: `&m * 2.0`, `m - 1.0`.
- **`table::Table`.** A tiny labelled-CSV loader with no new dependencies:
  `Table::<f64, 14, 5>::parse_csv(include_str!("data/weather.csv"))`.
