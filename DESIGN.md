# arrayglyph — design notes (Phase 1)

*Project codename: Iota. Crate name: `arrayglyph`.*

`arrayglyph` is an APL-inspired array vocabulary embedded in stable Rust. It has
three layers:

1. **`Array<T, ROWS, COLS>`**, an owned, `Vec`-backed array whose shape is part
   of its type, so shape mismatches are compile errors.
2. **A small, axis-explicit API surface**: named methods with a
   const-generic axis (`m.sum::<0>()`, `m.cumprod::<1>()`), the macros
   `reduce!(+[0], m)` / `scan!(*[1], m)` (APL's `f/[k]`), the flat train
   macro `train![…]` for `∘`, and `prof!` for timing. All of it lowers to
   ordinary iterator folds.
3. **A pretty-printer** (`arrayglyph::printer` + the `arrayglyph-fmt` binary)
   that turns dense glyph notation (`+/(a × b)`) into the verbose Rust it stands
   for (`a.zip_with(&b, Mul).sum::<1>()`, or `reduce!(+[1], a.zip_with(&b, Mul))`
   in macro style), and back again. Glyphs are the primary notation.

---

## 1. Why APL glyphs?

APL's notation is a *tool of thought*. One glyph names a whole-array operation:
`+/` is "sum", `×\` is "running product", `⌈/` is "maximum". Compositions read
as one phrase: `+/(a × b)` is a dot product. Because there are no loop indices,
off-by-one and aliasing bugs have nowhere to live. The glyph set is small, fixed,
and has 60 years of established meaning, so we don't have to invent a vocabulary.

## 2. Why Rust underneath?

* **Zero-cost.** Every glyph lowers to a fold or loop the optimiser understands.
  No interpreter or boxed values.
* **Static shapes.** Const generics let the *type* carry the shape. An APL
  `LENGTH ERROR` becomes a compile error.
* **Ecosystem and tooling.** cargo, clippy, rustdoc, Rayon (later), proptest
  (later). The glyph layer is a thin veneer and never a separate runtime.
* **Ownership.** Borrowed row/column views give APL-style slicing without copies
  or GC.

## 3. The token-density argument (agents write glyphs, humans read Rust)

An AI agent that writes array code pays for every token it emits and reads.
APL-style notation is very dense:

| intent | glyph form | verbose Rust (what it means) |
|---|---|---|
| dot product | `+/(a × b)` | `a.zip_with(&b, Mul).sum::<1>()` |
| running max | `⌈\v` | `v.cummax::<1>()` |
| row sums of a matrix product | `+/(a +.× b)` | `a.matmul(&b).sum::<1>()` |
| column means | `+⌿m ÷ 3` | `m.sum::<0>().zip_with(&Array::splat(3), Div)` |

A hand-written loop version of the first row runs 5–8 lines. The intended
workflow is:

* **Agents emit glyph form.** It's short, regular, and has few ways to be wrong.
  Shape errors still surface as compile errors once it's desugared.
* **Humans inspect with the pretty-printer.** `arrayglyph-fmt --to-rust` expands
  any glyph phrase into named, explicit method calls (`--style macro` for the
  macro spelling, `--style both` to see glyph, method and macro side by side).
  `--to-glyph` compresses canonical verbose Rust back down, so a reviewer can
  switch views either way.
* **What compiles is always ordinary Rust.** The glyph text is a notation layer.
  It is not a second language with its own semantics.

The printer is the bridge that makes dense notation acceptable in a codebase
humans still have to own.

## 4. The cultural tension

Rust values explicitness: named methods, visible borrows (`&b`), and no implicit
conversions. APL values terseness: one character per idea, implicit iteration,
and right-to-left evaluation with no precedence. The two pull against each other,
and this design doesn't pretend otherwise:

* **We borrow APL's vocabulary, not its grammar.** Glyph text evaluates **left
  to right with Rust-style precedence** (§7.1), so a Rust reader's instincts
  about `2 × 3 + 4` are right. Composition, which APL gets implicitly from
  right-to-left scope, is an explicit, visible operator (`∘`, §7.2).

* The **canonical, compiled form is verbose Rust**. Glyphs never hide a borrow,
  an allocation, or a trait bound. The verbose form shows all of them.
* The macros are deliberately **few and boring**. `reduce!(+[0], a)` expands
  to `a.reduce_axis::<0>(Add)`, the same as the named method `a.sum::<0>()`,
  which you can also write directly. There are no custom operators and no
  `proc-macro` parsing of glyph source in Phase 1.
* Glyph notation lives in **strings, docs, comments, and agent output**, and the
  printer translates it. Rust's lexer rejects most APL glyphs (`×`, `⌈`, `⍴`)
  as tokens, so they can't appear inside `macro_rules!` input anyway. The macros
  use ASCII spellings (`+ - * / max min`).

Some Rustaceans will still find `+/(a × b)` alien. The bet is that a reversible
printer turns the notation into a reading aid rather than a dialect.

## 5. The shaped-array type

```rust
pub struct Array<T, const ROWS: usize, const COLS: usize> { data: Vec<T> } // row-major
pub type Vector<T, const N: usize> = Array<T, 1, N>;
```

**Decisions**

* **Rank 2 now, vectors as `1 × N`.** One type covers vectors and matrices, so
  each op is written once. `Index<usize>` is implemented only for `Array<T, 1, N>`,
  and `item()` only for `Array<T, 1, 1>`.
* **`Vec` backing, not `[[T; C]; R]`.** Large arrays don't blow the stack, and
  stable Rust can't write `[T; R * C]` (that needs `generic_const_exprs`). The
  invariant `data.len() == ROWS * COLS` is enforced by every constructor.
* **Shape errors at compile time, three ways:**
  1. *Type equality.* `+ - * /`, `zip_with`, and `==` require both operands to
     have identical `ROWS`/`COLS`, so a mismatch is `error[E0308]`.
  2. *Shared const params.* `matmul<const K>(&Array<T, R, C>, &Array<T, C, K>)`.
     If the inner dimension doesn't match, the call doesn't type-check
     (`E0308`).
  3. *Inline `const { assert!(..) }`* (post-monomorphisation, stable since 1.79)
     handles relations stable Rust can't express in types: `reshape` requires
     `R*C == R2*C2`, and `reduce`/`scan` require `COLS > 0`. These are real
     compile errors (`error[E0080]: evaluation panicked: …`). They only fire
     when the function is instantiated. **`cargo check` doesn't report them.**
     Only `cargo build`/`cargo test` do, because they need monomorphisation.
* Every rule above has a `compile_fail` doctest pinned to its error code.
* **Runtime escape hatch.** `Array::from_vec` returns `Result<_, ShapeError>`
  for data whose length is only known at runtime.
* **Views.** `row(i) -> &[T]` is contiguous. `col(j) -> Col<'_, T>` is a strided
  borrowed view with `Index` and an iterator. Both borrow, so neither copies.
* **Index origin is 0** (`⎕IO←0`), to match Rust. `iota::<5>()` is `0 1 2 3 4`.

**Trade-offs given stable Rust (no `generic_const_exprs`)**

* We can't write `Array<T, {R*C}>` or compute result shapes like
  `Array<T, R, {C+1}>`. So concatenation (`,`) and take/drop are deferred.
  Reshape spells out its target shape, and a const assertion checks it.
* Reduction keeps the reduced axis as length 1 (`R × C → 1 × C` for axis 0,
  `R × 1` for axis 1). We can't
  "drop a dimension" generically without a rank parameter, so a vector sum is
  `Array<T, 1, 1>` and you call `.item()` to get the scalar. This is NumPy's
  `keepdims=True`. It's consistent but a little noisy.
* **Room for rank N.** The const pair `(ROWS, COLS)` is private to the struct's
  layout. Phase 3 could move to `Array<T, S: Shape>` with `S = (Const<2>, Const<3>)`
  shape types, or to an `ArrayN<T, const D: usize>` with a `[usize; D]` runtime
  shape for dynamic rank. The public method names (`reduce`, `scan`, `zip_with`,
  `reshape`) were chosen to carry over.

## 6. Glyph table and verbose mapping

Dyadic scalar functions (the `Dyadic<T>` trait, with marker types in
`arrayglyph::op`):

| glyph | ASCII (macro) | marker | meaning | associative? |
|---|---|---|---|---|
| `+` | `+` | `Add` | add | yes |
| `-` (also `−`) | `-` | `Sub` | subtract | **no** |
| `×` | `*` | `Mul` | multiply | yes |
| `÷` | `/` | `Div` | divide (T's `Div`; integer division truncates) | **no** |
| `⌈` | `max` | `Max` | maximum | yes |
| `⌊` | `min` | `Min` | minimum | yes |

Operators and other functions:

| glyph | name | verbose Rust |
|---|---|---|
| `f⌿x` | reduce, axis 0 (down columns) | `x.sum::<0>()` (`prod`/`max`/`min`) · `reduce!(f[0], x)` |
| `f/x` | reduce, axis 1 (along rows) | `x.sum::<1>()` · `reduce!(f[1], x)` |
| `f⍀x` | scan, axis 0 | `x.cumsum::<0>()` (`cumprod`/`cummax`/`cummin`) · `scan!(f[0], x)` |
| `f\x` | scan, axis 1 | `x.cumsum::<1>()` · `scan!(f[1], x)` |
| `-⌿x`, `÷/x`, … | `-` and `÷` have no named method | `x.reduce_axis::<0>(Sub)`, `x.scan_axis::<1>(Div)` · `reduce!(-[0], x)` |
| `f/[k]x`, `f\[k]x` | explicit axis (input only) | normalised to `⌿ / ⍀ \` |
| `a f b` | elementwise | `a.zip_with(&b, F)` (operators `&a + &b` also work) |
| `n` with arrays | scalar extension | `Array::splat(n)` (shape inferred from the other operand) |
| `1 2 3` | strand / vector literal | `Array::vector([1, 2, 3])` |
| `⍳n` | iota (0-based) | `iota::<n>()` |
| `⍴x` | shape | `x.shape()` |
| `r c⍴x` / `n⍴x` | reshape | `x.reshape::<r, c>()` / `x.reshape::<1, n>()` |
| `⍉x` | transpose | `x.transpose()` |
| `a +.× b` | inner (matrix) product | `a.matmul(&b)` |
| `(f ∘ g ∘ h) x` | compose, apply | `train![f, g, h].call(&x)` (see §7.2) |
| `f⌿`, `f/`, `f⍀`, `f\`, `⍉`, `⍴`, `r c⍴` (as `∘` operands) | train items | `reduce(f[0])`, `reduce(f[1])`, `scan(f[0])`, `scan(f[1])`, `transpose`, `shape`, `reshape(r, c)` |
| `¯3` | negative literal | `-3` |

## 7. Evaluation order, reduce and scan

### 7.1 Evaluation order: left to right, Rust-style precedence

**Decision (S, 2026-11-25; see `research/syn-notes.md`, "Design decisions").**
Glyph expressions evaluate **left to right, matching Rust convention**. APL's
right-to-left rule is not used. Of the two left-to-right options, strict
left-to-right with no precedence (Nial-style, `2 × 3 + 4` = 20) or
**Rust-style precedence**, arrayglyph uses **Rust-style precedence**:

| level (loosest → tightest) | glyph forms | associativity | Rust analogue |
|---|---|---|---|
| Sum | `a + b`, `a - b` | left | `+ -` |
| Product | `a × b`, `a ÷ b`, `a +.× b` | left | `* /` (and Python's `@` for matmul) |
| MaxMin | `a ⌈ b`, `a ⌊ b` | left | method call `a.max(b)` |
| Prefix | `f/x`, `f⌿x`, `f\x`, `f⍀x`, `⍴x`, `⍉x`, `r c⍴x`, `⍳n`, `(f ∘ g) x` | nests rightward | unary operators / calls |
| Atom | `x`, `3`, `1 2 3`, `( … )` | — | — |

Worked examples:

| glyph | means | value |
|---|---|---|
| `2 × 3 + 4` | `(2 × 3) + 4` | 10 |
| `2 + 3 × 4` | `2 + (3 × 4)` | 14 |
| `8 - 4 - 2` | `(8 - 4) - 2` | 2 |
| `8 ÷ 4 ÷ 2` | `(8 ÷ 4) ÷ 2` | 1 |
| `+/a × b` | `(+/a) × b` | sum of `a`, times `b` |
| `+/(a × b)` | dot product | — |
| `a ⌈ b + c` | `(a ⌈ b) + c` | — |
| `+/⍉m` | `+/(⍉m)` | column sums |

**Why Rust-style precedence instead of strict left-to-right.** The decision
was to match Rust convention, and Rust *has* precedence. Strict
left-to-right would make `2 + 3 × 4` = 20, which surprises Rust readers as
much as APL's 14 does, and would defeat the reason for switching. Precedence
is coherent for these glyphs: `× ÷` and `+ -` map one-to-one onto Rust's
`* /` and `+ -`. The less obvious placements are:

* **`⌈ ⌊` bind tighter than `×`.** Rust has no max/min operator. Its
  spelling is a method call, `a.max(b)`, which binds tighter than any binary
  operator. So `a ⌈ b + c` reads as `a.max(b) + c`. The common
  clamp-at-zero idiom must therefore be parenthesised: `0 ⌈ (x - 1)`. This
  is the least settled choice (see §10).
* **`+.×` sits at the `×` level**, like Python's `@` (and like matrix
  product in mathematical notation).
* **Prefix forms (`f/`, `⍴`, `⍉`, reshape, applied trains) bind tightest**, like
  Rust's unary operators and function calls. So `+/a × b` is `(+/a) × b`. A
  reduction over an expression needs parentheses: `+/(a × b)`. Stacked
  prefixes still nest naturally: `+/⍉m` is `+/(⍉m)`, just as `-!x` in Rust
  is `-(!x)`. Nothing else is coherent for prefix forms.

**Consequences.**

* The printer emits only the parentheses the table requires.
  `(a × b) + c` is printed `a × b + c`, but `a - (b - c)` keeps its
  parentheses. Round trips are still the identity on canonical text.
* Glyph text and Rust operator overloads now agree: `&a * &b + &c` and
  `a × b + c` mean the same thing.
* APL users must unlearn right-to-left. `+/a × b` is no longer a dot product.
  This is the deliberate cost of the decision.

### 7.2 Explicit composition (`∘`) and trains

**Decision (S, 2026-11-25):** composition is an explicit, visible operator.
Under right-to-left evaluation, `+/⍉m` *is* a composition. Under
left-to-right, composing is spelled out:

* **Meaning.** `f ∘ g` applies `g` first, then `f`. This is mathematical
  composition, `(f ∘ g)(x) = f(g(x))`, as in APL's `∘` and Haskell's `.`.
  `(+/ ∘ ⍉) m` has the same value as `+/⍉m`. The difference is that `+/ ∘ ⍉`
  is a value you can name and reuse.
* **Operands** are function forms: `f/`, `f\`, `⍉`, `⍴`, `r c⍴`, a
  parenthesised train, or another composition. `∘` is left-associative:
  `f ∘ g ∘ h` is `(f ∘ g) ∘ h`. Composition is associative, so the
  grouping never changes the value, and **the printer flattens it**:
  `(+/ ∘ (⍉ ∘ 2 3⍴))` prints as `(+/ ∘ ⍉ ∘ 2 3⍴)` (decision S,
  2026-11-26). Round trips are exact on canonical (flat) text.
* **Application.** `(f ∘ g) x` binds like any prefix form. Parentheses around
  the train are canonical, because they make the function boundary visible.
  `+/ ∘ ⍉ m` (no parentheses) is accepted and normalised. A lone function in
  parentheses, `(+/) x`, normalises to `+/x`. A train that is never applied is
  an error.
* **Rust spelling: a flat train (decision S, 2026-11-26).** `(f ∘ g ∘ h) x`
  is `train![f, g, h].call(&x)`. Items: `reduce(op[k])`, `scan(op[k])`,
  `transpose`, `shape`, `reshape(r, c)`, or `(expr)` for any function value
  (such as a named train). So `(+/ ∘ ⍉ ∘ 2 3⍴) ⍳6` becomes
  `train![reduce(+[1]), transpose, reshape(2, 3)].call(&iota::<6>())`, where
  it used to be `compose(compose(Reduce(Add).axis::<1>(), Transpose),
  Reshape::<2, 3>)`.
  * **Order: glyph order, not data-flow order.** Items are listed exactly as
    in the glyph, so the **rightmost item runs first**, like `∘` itself.
    Judgment call: the alternative (data-flow order, `train![reshape(2, 3),
    transpose, reduce(+[1])]`, like a Unix pipe) reads in execution order,
    but it would reverse every train relative to its glyph and break the
    one-to-one visual mapping the printer is for. A reader who knows `∘`
    already knows how to read `train!`.
  * **Expansion.** `train!` is a `macro_rules!` tt-muncher that builds each
    item's value (`Reduce(Add).axis::<1>()`, `Transpose`, …) and folds them
    left into `compose(compose(f, g), h)`. The function values in
    `arrayglyph::train` (`Reduce`, `Scan`, `Transpose`, `Shape`, `Reshape`,
    `Compose`, `Monadic`) and the helper `compose(f, g)` stay public as the
    lower-level API, and the printer's Rust parser still accepts
    `compose(…)`. Shapes are still checked at compile time through the
    whole train.
  * Closures were rejected because Rust can't infer a closure's argument
    type from a later `.call(&m)`, so every closure would need a full
    `Array<T, R, C>` annotation.
* **Not in this pass:** forks/atops (`(+/ ÷ ≢)`), dyadic trains, and outer
  product `∘.f`. Glyph `∘.` is rejected with an explicit "Phase 2" error.

### 7.3 Reduce and scan

* **Axis: always explicit (decision S, 2026-11-26; supersedes the
  2026-11-25 keyword form).** **Axis 0 runs down the columns**: an `R × C`
  array reduces to `1 × C` (per-column totals, like NumPy's `axis=0`). Axis
  1 runs along each row: `R × 1`. The canonical spellings always write the
  axis, and the printer always emits it:
  * **Macros: bracket axis.** `reduce!(+[0], m)`, `scan!(*[1], m)`. This is
    APL's own `+/[0] m` bracket-axis syntax with ASCII operators. The axis
    must be a literal and expands to the turbofish,
    `(m).reduce_axis::<0>(Add)`.
  * **Named methods (printer default).** `m.sum::<K>()`, `prod`, `max`,
    `min`, `mean` (reductions) and `cumsum`, `cumprod`, `cummax`, `cummin`
    (scans; NumPy's names). `-` and `÷` have no named method, so they are
    `m.reduce_axis::<K>(Sub)` / `m.scan_axis::<K>(Div)`. **Judgment call:
    a const-generic axis (`m.sum::<0>()`), not a runtime argument
    (`m.sum(0)`).** A runtime axis can't pick the result type, so `sum(0)`
    would have to return a dynamically shaped array or panic on a bad axis,
    losing the compile-time shape errors that are the point of the crate.
    The turbofish costs five characters and keeps `1 × C` vs `R × 1` in the
    type.
  * **Compatibility.** `reduce!(+, m, axis = N)` and the default-axis
    `reduce!(+, m)` (axis 0), and the same for `scan!`, still work but are
    **deprecated** in docs; the printer reads them and never writes them.
    `m.reduce(F)` / `m.scan(F)` (axis 0) remain as general methods.
  * **Breaking change.** `mean()` (implicit axis 0) and `mean_axis::<K>()`
    became one method, `mean::<K>()`. The crate is pre-1.0 and unpublished,
    so the old names were removed rather than deprecated.
  * There are no `reduce_rows!`/`reduce_cols!` macros, and reducing both
    axes is two calls (`m.sum::<0>().sum::<1>()`).
  * **Types.** `m.reduce_axis::<K>(F)` / `m.scan_axis::<K>(F)` and every
    named method take the axis as a const generic. The result shape comes
    from the trait `AlongAxis<T, R, C>`, implemented only for the marker
    types `Axis<0>` (`Reduced = Array<T, 1, C>`) and `Axis<1>` (`Reduced =
    Array<T, R, 1>`). Any other axis is a type error (E0277) with a custom
    `#[diagnostic::on_unimplemented]` message.
  * **Glyph spelling: APL's own.** `f⌿` / `f⍀` are axis 0 (APL's "first
    axis" operators) and `f/` / `f\` are axis 1 (APL's last axis). The APL
    bracket forms `f/[0]`, `f/[1]`, `f\[0]`, `f\[1]` are accepted as input
    (0-origin) and printed canonically as `⌿ / ⍀ \`. This keeps every
    existing glyph program meaning what it did (`+/v` is still a vector sum,
    `+/(a × b)` is still a dot product) and reads correctly to APL users,
    and the Rust spelling names the axis explicitly in every form.
  * **Vectors.** A vector is `1 × N`, so axis 0 has length 1, and reducing it
    would return the input unchanged. That is almost always a bug (it is
    what the old default would have silently done to `reduce!(+, v)`), so
    **reducing or scanning a length-1 axis is a compile error** (a const
    assertion, E0080) whose message says to use the other axis. Vectors are
    reduced along axis 1 (`v.sum::<1>().item()` or `reduce!(+[1], v)`, glyph
    `+/v`).
    The same check catches the APL habit `+/+/m`, which here would be a no-op
    on an `R × 1` column; the grand total is `+/+⌿m`.
  * **Trains.** In `train!` the item is `reduce(op[k])` / `scan(op[k])`.
    Underneath, `Reduce(F)` / `Scan(F)` are axis 0 and `.axis::<K>()` turns
    them into `ReduceAxis<F, K>` / `ScanAxis<F, K>`. A const-generic default
    (`Reduce<F, const A: usize = 0>`) was rejected because stable Rust doesn't
    use defaults for inference, so `Reduce(Add)` would need annotations.
* **Left fold (decision, follows §7.1).** `f/ x₁ x₂ … xₙ` =
  `((x₁ f x₂) f …) f xₙ`, implemented with `Iterator::reduce`. Writing the
  operator out between the items gives the same answer under §7.1's
  left-associativity: `-/1 2 3` = `1 - 2 - 3` = `(1 - 2) - 3` = **−4**, and
  `÷/8 4 2` = **1**. APL's right fold gives 2 and 4. For associative ops
  (`+ × ⌈ ⌊`) the two agree.
* **Scan.** Element *i* of `f\x` is `f/` of the first *i*+1 elements, which
  is the running left fold: `-\1 2 3` = `1 ¯1 ¯4`. It is O(n) for every op
  (`Iterator::scan`). The old O(n²) prefix right fold is gone.
* **`Dyadic::ASSOCIATIVE`** no longer affects results. It remains as metadata
  for future parallel (Rayon) reductions, which are only valid for
  associative ops.
* **Empty axis.** APL returns the identity element for `+/⍬`. Here
  `reduce`/`scan` require the chosen axis to have length ≥ 2 at compile time
  (const assert), so there is no identity-element machinery yet.
* **Floating point.** Results are exactly the sequential left fold. A future
  parallel variant may differ from it in the last ulp.
* **`Max`/`Min`** use `PartialOrd`. With NaN, they return the left operand.

## 8. Macro syntax

```rust
// canonical: the axis is always written, in brackets after the operator
reduce!(+[0], x)         // -> (x).reduce_axis::<0>(arrayglyph::op::Add)   glyph +⌿x
reduce!(+[1], x)         // -> (x).reduce_axis::<1>(arrayglyph::op::Add)   glyph +/x
scan!(max[1], x)         // -> (x).scan_axis::<1>(arrayglyph::op::Max)     glyph ⌈\x

// deprecated, still accepted
reduce!(+, x, axis = 1)  // -> (x).reduce_axis::<1>(arrayglyph::op::Add)
reduce!(+, x)            // -> (x).reduce(arrayglyph::op::Add)             axis 0

// flat trains, in glyph order (rightmost runs first)
train![reduce(+[1]), transpose, reshape(2, 3)]
                         // -> compose(compose(Reduce(Add).axis::<1>(), Transpose), Reshape::<2, 3>)

// profiling
prof!(m.sum::<0>())                     // times it, prints to stderr, returns the value
prof!(glyph = "+⌿m", m.sum::<0>())      // same, with the glyph in the report
```

The accepted operators are `+ - * / max min`. Anything else is a compile error
with a clear "unsupported operator" message. The array argument is any
expression. The macros expand to plain method calls on `Array`, which are
implemented as iterator folds. Phase 1 has no `proc-macro` crate.

**`prof!` (decision S, 2026-11-26).** `prof!(expr)` evaluates `expr` once
inside a closure, times it with `std::time::Instant`, and prints one line to
**stderr**: `[prof] file:line  <glyph  ⇐>  <expr text>  → <shape>  in
<elapsed>`, then returns the value unchanged, so it can wrap any
subexpression. The expression text is `stringify!(expr)`; the optional
`glyph = "…"` context is printed as given (not parsed). The shape comes from
the small trait `arrayglyph::prof::Profile` (`"R×C"` for arrays, `"shape (r,
c)"` for `shape()` tuples, `"scalar"` for primitives); other types are a
compile error with an `on_unimplemented` note. `prof::format_line` is public
so the report format is testable. It is deliberately not a benchmark
harness: one run, wall-clock time, no warm-up (use criterion for that).

## 9. Pretty-printer

`arrayglyph::printer::{glyph_to_rust, glyph_to_rust_with, rust_to_glyph,
Style}`, plus the `arrayglyph-fmt` CLI. Glyphs are the primary notation; the
Rust is what they stand for.

* **Glyph → Rust (desugaring).** This is a hand-written lexer plus a
  precedence-climbing parser for §7.1. Infix levels are folded left. Prefix
  forms are parsed as unary operators. It emits canonical verbose Rust (the
  table in §6) in one of two **styles** (decision S, 2026-11-26):
  * `Style::Method` (**default**): `m.sum::<0>()`, `m.cumprod::<1>()`,
    `m.reduce_axis::<0>(Sub)`.
  * `Style::Macro`: `reduce!(+[0], m)`, `scan!(*[1], m)`,
    `reduce!(-[0], m)`.
  Both always write the axis and emit the same `zip_with`, `matmul`,
  `transpose`, `reshape`, `iota`, `splat`, and `train![…].call(&x)` forms.
  `glyph_to_rust` / `Expr::to_rust` use the default;
  `glyph_to_rust_with(src, style)` / `Expr::to_rust_styled(style)` choose.
* **CLI.** `arrayglyph-fmt [--to-rust | --to-glyph] [--style
  <method|macro|both>] [EXPR ...]`, built with clap's derive API (`clap` is
  the crate's one dependency). `--style` is a `ValueEnum`, default
  `method`; `both` prints three labelled lines (`glyph:`, `method:`,
  `macro:`). With `--to-glyph`, `method`/`macro` don't change the glyph
  output and `both` also prints the two Rust spellings. Expressions that
  begin with `-` (such as `-⌿m`) go after `--`, the usual clap/POSIX rule.
* **Rust → glyph.** This is a small parser for exactly the canonical forms the
  desugarer emits in either style (method chains `.zip_with(&_, Op)`,
  `.sum::<k>()` and the other named methods (the axis is required),
  `.reduce_axis::<k>(Op)`, `.scan_axis::<k>(Op)`,
  `.matmul(&_)`, `.reshape::<r, c>()`, `.transpose()`, `.shape()`, plus
  `Array::vector([..])`, `Array::splat(n)`, `iota::<n>()`, identifiers,
  parentheses, `reduce!(op[k], x)` / `scan!(op[k], x)` macro calls, and
  `train![…].call(&x)`), plus the older spellings (`.reduce(Op)`,
  `.scan(Op)`, `reduce!(op, x[, axis = k])`, `compose(…).call(&x)` over
  `train` function values). It is
  **not** a general Rust parser. Arbitrary Rust, including operator forms like
  `&a + &b`, is rejected with an error, not guessed at.
* **Round trip.** `rust_to_glyph(glyph_to_rust(g)) == g` for every canonical
  glyph string in the test corpus, which covers every precedence level and
  associativity case. Canonical spacing is `a + b`, `+/x`, `⍳5`, `2 3⍴x`, and
  `a +.× b`. Canonical parentheses are the minimal ones §7.1 needs, and
  `∘` chains are flat. Non-canonical input is normalised. The same holds in
  macro style: glyph → macro → glyph is the identity on the corpus, and
  method and macro text parse to the same tree. The one lossy
  case is a one-element vector `Array::vector([5])`, which has no strand form
  (APL would write `,5`) and is rejected.
* The output is checked: `tests/printer_compiles.rs` pastes desugared output
  (both styles) into real Rust and evaluates it.
* **Known limitation.** A strand that mixes integer and float literals
  (`¯1.5 2`) desugars to `Array::vector([-1.5, 2])`, which Rust rejects
  because the literals have different types. Write `¯1.5 2.0` instead.
  Numbers are kept as literal text, so the printer doesn't rewrite them.
* **Not yet:** comments and multi-statement programs.

### 9.1 Spans (data model ready, rendering is Phase 2)

The printer's data model already carries structured source locations. This
keeps Phase 2's span work additive.

```rust
pub struct Pos  { pub offset: usize, pub line: usize, pub col: usize }
pub struct Span { pub start: Pos, pub end: Pos }   // half-open [start, end)
pub struct Expr { pub kind: ExprKind, pub span: Span }
pub struct ParseError { pub message: String, pub span: Span }
```

* `offset` is a **byte** offset, always on a UTF-8 char boundary. `line` is
  1-based and lines split on `\n`. `col` is 1-based and **counts chars, not
  bytes**: `⍴` (3 bytes), `×` (2 bytes), and `¯` (2 bytes) each advance the
  column by exactly 1.
* `LineIndex` builds a line-start table once per source string. Each
  `pos(offset)` is a binary search plus a char count within one line.
  `LineIndex` is public so tools can map their own offsets.
* **Tokens** (glyph and Rust lexers) and **every AST node** carry a span.
  Node spans cover their whole source extent. A parenthesised expression
  includes its parentheses. `+.×` is a single token span. A strand spans from
  its first number to its last.
* `Expr::parse_glyph` spans point into the glyph text. `Expr::parse_rust`
  spans point into the Rust text. Printing doesn't keep spans, because
  `to_rust` and `to_glyph` produce fresh text.
* **Errors** carry the offending token's span. For "unexpected end of input"
  and unclosed `(` errors, the span is empty and sits at end of input.
  `ParseError::offset()` is a shorthand for `span.start.offset`. `Display`
  prints `message (at line:col)`.
* Unit tests cover multi-line input, multi-byte glyphs, error spans, and
  Rust-side spans (`src/printer/mod.rs`, `span_tests`).
* **Deliberately not done yet (Phase 2):** span joining and merging APIs,
  mapping emitted Rust fragments back to glyph spans (a source map),
  multi-line caret or underline error rendering, and IDE/LSP integration. The
  CLI prints only a minimal single-line caret.

## 10. Open questions

1. ~~Axis specification.~~ **Decided and implemented:** axis 0 = columns,
   no separate row/col macros, no all-axes reduction in one call, glyphs
   `⌿ ⍀` (axis 0) and `/ \` (axis 1), vectors reduce along axis 1. Revised
   (S, 2026-11-26): the axis is always explicit, as `reduce!(+[k], m)` or
   `m.sum::<k>()`; the `axis = N` keyword is kept only for compatibility
   (§7.3).
2. **Rank polymorphism.** APL functions work at any rank. Should we go through
   shape types (`Array<T, S: Shape>`, fully static but complex type signatures),
   dynamic rank (`ArrayD`, simple but runtime shape errors), or both, with
   conversions? This also decides whether reductions can drop a dimension.
3. ~~Right-to-left vs Rust precedence.~~ **Resolved (S, 2026-11-25):** left
   to right with Rust-style precedence, left folds, and explicit `∘` (§7).
   Still open within it: should `⌈ ⌊` bind tighter than `×` (the current
   method-call analogy) or looser than `+ -` (so that `0 ⌈ x - 1` clamps
   without parentheses)?
4. Smaller ones: empty reductions with identity elements (`+/⍬ = 0`), float
   `÷` on integer arrays (APL always yields floats), and `⎕IO` = 0 vs 1.

## 11. Roadmap

**Phase 1 (this milestone).** `Array<T, R, C>`, views, elementwise ops, matmul,
transpose, reshape, axis-explicit reductions and scans (`m.sum::<K>()`,
`reduce!(+[K], m)`), flat `train![…]`, `prof!`, the bidirectional printer with
method and macro styles, and the `arrayglyph-fmt` CLI (`--style`).
Convenience APIs added while building the showcase (no glyph forms yet):
`broadcast::<R, C>()` for length-1 axes, `mean::<K>()`, scalar
operators (`&m * 2.0`), precision-aware `Display`, and `table::Table`, a
tiny labelled-CSV loader that checks the data's shape once at load time.
`examples/WISHLIST.md` tracks what the showcase still wanted (glyph-level
broadcasting, forks and `≢`, argmax, comparisons, labelled results).

**Phase 2**

* **Outer and inner product macros.** `outer!(*, a, b)` (`a ∘.× b`) and a
  general `inner!(+, *, a, b)` (`a +.× b` with any pair of functions; today
  only `+.×` / `matmul` exists).
* **Operator trains / forks.** Beyond `∘` (done, §7.2): forks such as
  `(+/ ÷ ≢)` (mean), atops, and dyadic trains.
* **Opt-in Rayon parallel variants** behind a `rayon` cargo feature:
  `par_reduce`, `par_zip_with`, and `par_matmul`. Parallel reduce is only
  offered for `ASSOCIATIVE` ops, which the type system already tracks.
* **Span tracking in the printer.** Build on the §9.1 data model: span join
  helpers, a source map from each emitted Rust fragment back to its glyph
  `Span` (for IDE hover and diff views), and rich multi-line error rendering.
* **Property tests (proptest)** comparing every glyph op against an explicit
  nested-loop reference implementation, including round-trip
  `glyph → rust → glyph` on generated ASTs.

**Phase 3+.** Rank-N (see §5), identity elements for empty reductions, more
glyphs (`,` catenate, `↑`/`↓` take/drop, `⌽` reverse, `≢` tally), and possibly a
`glyph!{ … }` proc-macro that parses glyph source at compile time via the same
parser as the printer.
