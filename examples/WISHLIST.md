# Wishlist: features the showcase wanted

Notes from building `examples/showcase.rs` (two weeks × five cities of
synthetic temperatures). Each entry: **what**, **why** (where the showcase
felt it), and a **sketch**.

## Built while writing the showcase

| feature | why | where |
|---|---|---|
| `Display` honours `{:.1}` / `{:6}` | float tables printed as `13.428571428571429` | `impl Display for Array` |
| `broadcast::<R2, C2>()` | subtracting the 1×5 city means from the 14×5 data didn't type-check | `Array::broadcast` |
| `mean::<K>()` (was `mean()` / `mean_axis::<K>()`) | "average per city" is the first thing anyone asks | `Array::mean`, `AlongAxis::mean` |
| scalar operators `&m * 2.0`, `m - 1` | `zip_with(&Array::splat(2.0), Mul)` is noisy in hand-written Rust | `impl Add<f64> for Array<f64, …>` etc. |
| `table::Table::parse_csv` | loading a labelled CSV into a shaped array with no dependencies | `src/table.rs` |

## Built in the explicit-axis round

| feature | why | where |
|---|---|---|
| named axis methods `sum`/`prod`/`max`/`min`/`cumsum`/`cumprod`/`cummax`/`cummin::<K>()` | `x.reduce_axis::<1>(Add)` was the noisiest part of every printed line | `Array::sum` etc. |
| bracket-axis macros `reduce!(+[0], m)` | the axis was optional, so `reduce!(+, m)` hid which way it folded | `reduce!`, `scan!` |
| flat trains `train![…]` | the weekly-means pipeline was three nested `compose(…)` calls | `train!` |
| `prof!` | "which step is slow?" had no one-line answer | `prof!`, `prof::Profile` |
| `arrayglyph-fmt --style <method\|macro\|both>` | reviewers wanted the macro spelling next to the method one | `src/bin/arrayglyph-fmt.rs` |

## Not built yet

1. **Broadcasting in glyph text.** *Why:* `x - +⌿x ÷ 14.0` (centre each city
   on its mean) is the natural glyph, but `14×5 - 1×5` is a shape error, so
   the showcase computes anomalies in plain Rust. *Sketch:* make `zip_with`
   take `&impl BroadcastTo<R, C>` (implemented for same-shape, `1×C`, `R×1`
   and `1×1` arrays, with the same compile-time check as `broadcast`), so the
   printer's existing `a.zip_with(&b, F)` output just works.
2. **Forks and tally: `(+/ ÷ ≢)`, `(⌈⌿ - ⌊⌿)`.** *Why:* mean and range are
   classic three-function trains; today they're spelled `+⌿x ÷ 14.0` (with
   the length typed by hand) and `⌈⌿x - ⌊⌿x`. *Sketch:* `TrainKind::Fork(f,
   g, h)` ↔ `fork(F, Div, H)`, a `Tally` function value (`≢`) that returns the
   axis length as `T`, and Rust `Monadic` impls like `Compose`'s.
3. **Index of the max (`argmax`, or APL grade `⍒`).** *Why:* "which city /
   day was warmest?" needed `iter().max_by(total_cmp)` and a manual zip with
   labels. *Sketch:* `m.argmax_axis::<K>() -> Array<usize, …>` with the same
   `AlongAxis` shapes, and later `⍋ ⍒` returning permutation vectors.
4. **Comparisons and compress: `+⌿x > 25.0`, `mask / x`.** *Why:* "how many
   days above 25 °C?" has no array spelling. *Sketch:* `Gt Lt Ge Le Eq`
   markers producing `Array<bool, R, C>`, `count`/`+⌿` over bools, and
   compress returning a `Vec` (its length isn't known at compile time).
5. **Splitting an axis (weeks) without the transpose dance.** *Why:* weekly
   means took `(5 2⍴ ∘ +/ ∘ 10 7⍴ ∘ ⍉)` (now at least a flat
   `train![reshape(5, 2), reduce(+[1]), reshape(10, 7), transpose]`),
   correct and type-checked but hard to read. *Sketch:* `m.chunks_axis::<0, 7>()` returning `[Array<T, 7, C>;
   N]`, or rank-3 arrays (Phase 3).
6. **Labels that survive computation.** *Why:* `Table` keeps row/column
   labels, but every result is a bare `Array`, so the report re-zips labels
   by hand. *Sketch:* `Labeled<A>` with `reduce` dropping the reduced axis's
   labels and keeping the other's; `Display` printing them as headers.
7. **Numeric literals that adapt to the element type.** *Why:* glyph `÷ 14`
   desugars to `Array::splat(14)`, an integer, so with `f64` data you must
   write `14.0`. *Sketch:* emit `Array::splat(14 as _)`-style conversions, or
   have the printer accept a type hint (`--elem f64`).
8. **`item()` without consuming.** *Why:* `total.item()` moved `total`.
   *Sketch:* `fn item(&self) -> T where T: Clone` (a breaking change, so batch
   it with other API changes) or a `get_item(&self) -> &T`.
9. **Compile-time CSV shape.** *Why:* `Table<f64, 14, 5>` repeats a shape the
   file already knows. *Sketch:* an `include_table!("data/weather.csv")`
   proc-macro that reads the file at build time and emits
   `Table<f64, 14, 5>`; it fits with the planned `glyph!{}` proc-macro.
10. **More scalar functions.** *Why:* standard deviation needs `sqrt` and
    power. *Sketch:* monadic `|`, `*` (power), `√`-style helpers as
    `map` wrappers with glyph forms, and a `Pow` dyadic marker.
