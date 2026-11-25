# arrayglyph — design notes (Phase 1)

*Project codename: Iota. Crate name: `arrayglyph`.*

`arrayglyph` is an APL-inspired array vocabulary embedded in stable Rust. It has
three layers:

1. **`Array<T, ROWS, COLS>`**, an owned, `Vec`-backed array whose shape is part
   of its type, so shape mismatches are compile errors.
2. **A small macro surface** (`reduce!`, `scan!`) that gives APL's `f/` and `f\`
   a compact spelling and lowers to ordinary iterator folds.
3. **A pretty-printer** (`arrayglyph::printer` + the `arrayglyph-fmt` binary)
   that turns dense glyph notation (`+/a × b`) into the verbose Rust it stands
   for (`a.zip_with(&b, Mul).reduce(Add)`), and back again.

---

## 1. Why APL glyphs?

APL's notation is a *tool of thought*. One glyph names a whole-array operation:
`+/` is "sum", `×\` is "running product", `⌈/` is "maximum". Compositions read
as one phrase: `+/a × b` is a dot product. Because there are no loop indices,
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
| dot product | `+/a × b` | `a.zip_with(&b, Mul).reduce(Add)` |
| running max | `⌈\v` | `v.scan(Max)` |
| row sums of a matrix product | `+/a +.× b` | `a.matmul(&b).reduce(Add)` |

A hand-written loop version of the first row runs 5–8 lines. The intended
workflow is:

* **Agents emit glyph form.** It's short, regular, and has few ways to be wrong.
  Shape errors still surface as compile errors once it's desugared.
* **Humans inspect with the pretty-printer.** `arrayglyph-fmt --to-rust` expands
  any glyph phrase into named, explicit method calls. `--to-glyph` compresses
  canonical verbose Rust back down, so a reviewer can switch views either way.
* **What compiles is always ordinary Rust.** The glyph text is a notation layer.
  It is not a second language with its own semantics.

The printer is the bridge that makes dense notation acceptable in a codebase
humans still have to own.

## 4. The cultural tension

Rust values explicitness: named methods, visible borrows (`&b`), and no implicit
conversions. APL values terseness: one character per idea, implicit iteration,
and right-to-left evaluation with no precedence. The two pull against each other,
and this design doesn't pretend otherwise:

* The **canonical, compiled form is verbose Rust**. Glyphs never hide a borrow,
  an allocation, or a trait bound. The verbose form shows all of them.
* The macros are deliberately **few and boring**. `reduce!(+, a)` expands to
  `a.reduce(Add)`, which you can also write directly. There are no custom
  operators, no `proc-macro` parsing of glyph source in Phase 1, and no
  operator-precedence games.
* Glyph notation lives in **strings, docs, comments, and agent output**, and the
  printer translates it. Rust's lexer rejects most APL glyphs (`×`, `⌈`, `⍴`)
  as tokens, so they can't appear inside `macro_rules!` input anyway. The macros
  use ASCII spellings (`+ - * / max min`).

Some Rustaceans will still find `+/a × b` alien. The bet is that a reversible
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
* Reduction keeps the reduced axis as length 1 (`R × C → R × 1`). We can't
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
| `f/x` | reduce (last axis) | `x.reduce(F)` · `reduce!(f, x)` |
| `f\x` | scan (last axis) | `x.scan(F)` · `scan!(f, x)` |
| `a f b` | elementwise | `a.zip_with(&b, F)` (operators `&a + &b` also work) |
| `n` with arrays | scalar extension | `Array::splat(n)` (shape inferred from the other operand) |
| `1 2 3` | strand / vector literal | `Array::vector([1, 2, 3])` |
| `⍳n` | iota (0-based) | `iota::<n>()` |
| `⍴x` | shape | `x.shape()` |
| `r c⍴x` / `n⍴x` | reshape | `x.reshape::<r, c>()` / `x.reshape::<1, n>()` |
| `⍉x` | transpose | `x.transpose()` |
| `a +.× b` | inner (matrix) product | `a.matmul(&b)` |
| `¯3` | negative literal | `-3` |

## 7. Reduce / scan semantics

* **Axis.** Phase 1 always reduces/scans along the **last axis** (APL's default
  for `/` and `\`). An `R × C` array reduces to `R × 1`. To reduce columns,
  `⍉` first: `+/⍉m` ≡ `m.transpose().reduce(Add)`.
* **Right-to-left.** `f/ x₁ x₂ … xₙ` = `x₁ f (x₂ f (… f xₙ))`, implemented as
  `DoubleEndedIterator::rfold`. For associative ops this equals a left fold.
  For non-associative ops it doesn't: `-/1 2 3` = `1 - (2 - 3)` = **2** (not
  −4), and `÷/8 4 2` = `8 ÷ (4 ÷ 2)` = **4**.
* **Scan.** Element *i* of `f\x` is `f/` of the first *i*+1 elements
  (`-\1 2 3` = `1 ¯1 2`). For associative ops this is the O(n) running fold
  (`Iterator::scan`). For non-associative ops we evaluate each prefix by
  right-fold, which is O(n²) and exactly faithful to APL. This choice is made
  by `Dyadic::ASSOCIATIVE`.
* **Empty axis.** APL returns the identity element for `+/⍬`. Here
  `reduce`/`scan` require `COLS > 0` at compile time (const assert), so there is
  no identity-element machinery yet.
* **Floating point.** `Add` and `Mul` are treated as associative even for floats.
  Scan results may differ from a strict right-fold in the last ulp.
* **`Max`/`Min`** use `PartialOrd`. With NaN, they return the left operand.

## 8. Macro syntax

```rust
reduce!(+, x)      // -> (x).reduce(arrayglyph::op::Add)
scan!(max, x)      // -> (x).scan(arrayglyph::op::Max)
```

The accepted operators are `+ - * / max min`. Anything else is a compile error
with a clear "unsupported operator" message. The array argument is any
expression. Both macros expand to plain method calls on `Array`, which are
implemented as iterator folds. Phase 1 has no `proc-macro` crate.

## 9. Pretty-printer

`arrayglyph::printer::{glyph_to_rust, rust_to_glyph}`, plus the
`arrayglyph-fmt` CLI:

* **Glyph → Rust (desugaring).** This is a hand-written lexer plus a
  right-to-left recursive-descent parser with APL rules: no precedence, the
  right operand extends as far right as possible, and parentheses group. It
  emits canonical verbose Rust (the table in §6).
* **Rust → glyph.** This is a small parser for exactly the canonical forms the
  desugarer emits (method chains `.zip_with(&_, Op)`, `.reduce(Op)`, `.scan(Op)`,
  `.matmul(&_)`, `.reshape::<r, c>()`, `.transpose()`, `.shape()`, plus
  `Array::vector([..])`, `Array::splat(n)`, `iota::<n>()`, identifiers,
  parentheses, and `reduce!(op, x)` / `scan!(op, x)` macro calls). It is
  **not** a general Rust parser. Arbitrary Rust, including operator forms like
  `&a + &b`, is rejected with an error, not guessed at.
* **Round trip.** `rust_to_glyph(glyph_to_rust(g)) == g` for every canonical
  glyph string in the test corpus. Canonical spacing is `a + b`, `+/x`, `⍳5`,
  `2 3⍴x`, and `a +.× b`. Non-canonical input is normalised. The one lossy
  case is a one-element vector `Array::vector([5])`, which has no strand form
  (APL would write `,5`) and is rejected.
* The output is checked: `tests/printer_compiles.rs` pastes desugared output
  into real Rust and evaluates it.
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

1. **Axis specification.** APL uses `f/[k]` or `f⌿` (first axis). Should the
   macro be `reduce!(+, x, axis = 0)`, `reduce!(+[0], x)`, or separate
   `reduce_rows!`/`reduce_cols!`? Should the default stay "last axis" (APL) or
   change to "all axes" (NumPy `sum()`)?
2. **Rank polymorphism.** APL functions work at any rank. Should we go through
   shape types (`Array<T, S: Shape>`, fully static but complex type signatures),
   dynamic rank (`ArrayD`, simple but runtime shape errors), or both, with
   conversions? This also decides whether reductions can drop a dimension.
3. **Right-to-left vs Rust precedence.** In glyph text `2 × 3 + 4` is 14 (APL),
   but in Rust `2 * 3 + 4` is 10. The printer always produces explicit method
   chains, so the compiled code is unambiguous. But anyone mixing operator
   overloads (`&a * &b + &c`) in Rust gets Rust precedence. Should glyph text in
   docs require full parenthesisation, or should we lint for it?
4. Smaller ones: empty reductions with identity elements (`+/⍬ = 0`), float
   `÷` on integer arrays (APL always yields floats), and `⎕IO` = 0 vs 1.

## 11. Roadmap

**Phase 1 (this milestone).** `Array<T, R, C>`, views, elementwise ops, matmul,
transpose, reshape, `reduce!`/`scan!`, the bidirectional printer, and the
`arrayglyph-fmt` CLI.

**Phase 2**

* **Axis-aware reduce/scan** via an `[axis]` macro parameter
  (`reduce!(+[0], m)` ≡ APL `+⌿m`). Glyph syntax `+/[0] m` and `+⌿ m`.
* **Outer and inner product macros.** `outer!(*, a, b)` (`a ∘.× b`) and a
  general `inner!(+, *, a, b)` (`a +.× b` with any pair of functions; today
  only `+.×` / `matmul` exists).
* **Operator trains / forks.** `(+/ ÷ ≢)` (mean) as a composable value: a
  `Train` type plus printer support for tacit forms.
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
