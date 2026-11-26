# Agent-first public API (sketch)

**Status:** design sketch, **partly superseded**. On 2026-11-26 S decided
the canonical spellings for axes, trains, and the printer, and those are now
implemented (§0). The rest of this document (keyword macros such as `zip!`
and `reshape!`, `AG00xx` error codes, `api.json`, the rustdoc section
convention) is still a sketch and not implemented. Where an older example
below used the keyword form for reductions or trains, it has been updated to
the decided form.

## 0. Decided and implemented (S, 2026-11-26)

What an agent should emit today:

| intent | glyph | canonical Rust (method style, printer default) | macro style |
|---|---|---|---|
| sum down columns | `+⌿x` | `x.sum::<0>()` | `reduce!(+[0], x)` |
| sum along rows | `+/x` | `x.sum::<1>()` | `reduce!(+[1], x)` |
| product / max / min | `×⌿x`, `⌈/x`, `⌊⌿x` | `x.prod::<0>()`, `x.max::<1>()`, `x.min::<0>()` | `reduce!(*[0], x)`, `reduce!(max[1], x)`, `reduce!(min[0], x)` |
| `-` / `÷` reductions | `-⌿x`, `÷/x` | `x.reduce_axis::<0>(Sub)`, `x.reduce_axis::<1>(Div)` | `reduce!(-[0], x)`, `reduce!(/[1], x)` |
| scans | `+⍀x`, `×\x`, `⌈\x`, `⌊⍀x` | `x.cumsum::<0>()`, `x.cumprod::<1>()`, `x.cummax::<1>()`, `x.cummin::<0>()` | `scan!(+[0], x)`, `scan!(*[1], x)`, … |
| mean (no glyph) | `+⌿x ÷ 14.0` | `x.mean::<0>()` (hand-written) | — |
| train | `(+/ ∘ ⍉ ∘ 2 3⍴) v` | `train![reduce(+[1]), transpose, reshape(2, 3)].call(&v)` | same |
| profile a step | — | `prof!(glyph = "+⌿x", x.sum::<0>())` | same |

Rules that follow from the decisions:

* **The axis is always explicit.** There is no implicit default in canonical
  code or printer output. Macros write it in brackets after the operator
  (`+[0]`, APL's `+/[0]`); methods take it as a const generic (`::<0>`) so
  the result shape stays in the type. `reduce!(+, x, axis = 1)` and
  `reduce!(+, x)` still compile and still convert to glyphs, but are
  deprecated. There are no `reduce_rows!` / `reduce_cols!`.
* **Trains are flat** and listed in glyph order (rightmost runs first):
  `train![f, g, h]` is `(f ∘ g ∘ h)`. Items are `reduce(op[k])`,
  `scan(op[k])`, `transpose`, `shape`, `reshape(r, c)`, or `(expr)`.
  `compose(f, g)` remains as the lower-level API. It is a `macro_rules!`
  tt-muncher, not a proc-macro (this answers §6 Q3).
* **Printer default is method style** (`glyph_to_rust`); macro style via
  `glyph_to_rust_with(src, Style::Macro)` or `arrayglyph-fmt --style macro`;
  `--style both` prints glyph, method and macro lines (this answers §6
  Q5). Glyphs remain the primary notation.
* **Operator tokens** in the implemented macros are `+ - * / max min`. The
  word aliases (`add`, `mul`, …) below are still only a sketch.

Agents writing arrayglyph care about **unambiguous syntax**, **predictable
errors**, and **parseable docs**, not APL nostalgia. This sketch makes
explicit **keyword forms** the primary public names. Glyph notation is a
shorthand: macros or the pretty-printer expand it into those keyword forms,
and the keyword forms expand to the methods that already exist
(`reduce_axis::<K>`, `compose(f, g).call(&x)`, …).

The compiled surface stays method-shaped. What changes is what agents are
told to emit, what rustdoc leads with, and what the printer treats as the
canonical ASCII spelling.

## 1. Keyword-first macros

### Conventions

* **Operator names** in keyword forms are lowercase ASCII words: `add`,
  `sub`, `mul`, `div`, `max`, `min`. The symbols `+ - * /` are accepted as
  aliases (`*` is multiply, `/` is divide — the same tokens the current
  macros use). Marker types stay `Add`, `Sub`, `Mul`, `Div`, `Max`, `Min`.
* **Keywords** configure a call (`op`, `axis`, `of`, `rows`, `cols`, …).
  The array operand may be written `of = x` or as a single positional
  expression. Canonical written order is `op` (when present), then the
  array, then optional keywords. `macro_rules!` can implement that order
  plus a few permutations; freely ordered keywords are a proc-macro
  enhancement (see §3 and §6).
* **The axis is always written** on every reduction-like form: `op[0]`
  means down the columns (`R × C → 1 × C`), `op[1]` along each row
  (`R × C → R × 1`). Anything else is an error. Vectors are `1 × N` and
  need axis 1. (Superseded: this sketch originally proposed a keyword
  `axis = N` with default 0.)
* Trailing commas are allowed. All macros are rustdoc'd under the keyword
  spelling; glyphs appear in a `Glyph` section, not in the signature line.

### Grammar (EBNF-ish)

```ebnf
op-word  = "add" | "sub" | "mul" | "div" | "max" | "min"
op-sym   = "+" | "-" | "*" | "/"          (* aliases; * = mul, / = div *)
op       = op-word | op-sym
axis     = "0" | "1"
array    = "of" "=" rust-expr | rust-expr
n        = rust-usize-literal             (* const-generic bound *)

(* implemented: bracket axis, always written *)
reduce!  = "reduce!"  "(" op-sym-or-minmax "[" axis "]" "," rust-expr ")"
scan!    = "scan!"    "(" op-sym-or-minmax "[" axis "]" "," rust-expr ")"
op-sym-or-minmax = "+" | "-" | "*" | "/" | "max" | "min"
(* deprecated, still accepted: reduce!(op, x, axis = k) and reduce!(op, x) *)

(* implemented as a method instead of a macro: x.mean::<axis>() *)

zip!     = "zip!"     "(" "op" "=" op "," array "," array ")"
(* two arrays, or one array and a scalar; shapes must match or broadcast *)

reshape!    = "reshape!"    "(" array "," "rows" "=" n "," "cols" "=" n ")"
transpose!  = "transpose!"  "(" array ")"
iota!       = "iota!"       "(" "n" "=" n ")"
broadcast!  = "broadcast!"  "(" array "," "rows" "=" n "," "cols" "=" n ")"
matmul!     = "matmul!"     "(" array "," array ")"

(* general inner product; matmul! is inner!(reduce = add, zip = mul, …) *)
inner!   = "inner!" "(" "reduce" "=" op "," "zip" "=" op "," array "," array ")"

(* Phase 2 *)
outer!   = "outer!" "(" "op" "=" op "," array "," array ")"

(* function values; apply with .call(&x) *)
(* implemented: flat train, glyph order, rightmost item runs first *)
train!      = "train!" "[" item ("," item)* ","? "]"
item        = "reduce" "(" op-sym-or-minmax "[" axis "]" ")"
            | "scan"   "(" op-sym-or-minmax "[" axis "]" ")"
            | "transpose" | "shape" | "reshape" "(" n "," n ")"
            | "(" rust-expr ")"                (* any function value *)

(* Phase 2: fork (f g h) x  =  (f x) g (h x); likely a train! item *)
fork!       = "fork!" "(" "f" "=" item "," "g" "=" op "," "h" "=" item ")"
```

Inside `train![…]`, `reduce(op[k])` / `scan(op[k])` are function values
(train operands). That is how `(+/ ∘ ⍉) m` is spelled:
`train![reduce(+[1]), transpose].call(&m)`.

### One-line examples

| form | example | expands to |
|---|---|---|
| `reduce!` **(implemented)** | `reduce!(+[0], m)` | `m.reduce_axis::<0>(Add)` = `m.sum::<0>()` |
| `reduce!` axis 1 **(implemented)** | `reduce!(+[1], v)` | `v.sum::<1>()` |
| `scan!` **(implemented)** | `scan!(max[1], m)` | `m.cummax::<1>()` |
| mean **(implemented as a method)** | `m.mean::<0>()` / `m.mean::<1>()` | — |
| `zip!` | `zip!(op = mul, a, b)` | `a.zip_with(&b, Mul)` |
| `zip!` (scalar) | `zip!(op = div, of = s, 14.0)` | `s.zip_with(&Array::splat(14.0), Div)` |
| `reshape!` | `reshape!(of = x, rows = 2, cols = 3)` | `x.reshape::<2, 3>()` |
| `transpose!` | `transpose!(of = m)` | `m.transpose()` |
| `iota!` | `iota!(n = 6)` | `iota::<6>()` |
| `broadcast!` | `broadcast!(of = mean, rows = 14, cols = 5)` | `mean.broadcast::<14, 5>()` |
| `matmul!` | `matmul!(a, b)` | `a.matmul(&b)` |
| `inner!` | `inner!(reduce = add, zip = mul, a, b)` | same as `matmul!(a, b)` today |
| `outer!` | `outer!(op = mul, a, b)` | Phase 2 (`a ∘.× b`) |
| `train!` **(implemented)** | `train![reduce(+[1]), transpose]` | `compose(Reduce(Add).axis::<1>(), Transpose)` |
| apply | `col_sums.call(&m)` (no `apply!` macro) | `col_sums.call(&m)` |

`map` stays a method, `m.map(|x| …)`, because the function is a Rust
closure, not an arrayglyph operator. There is no `map!` macro unless we
later add monadic glyph ops (`√`, `∣`) with keyword names.

### Relation to today's API, and migration

| older spelling (still accepted) | canonical now | still a sketch |
|---|---|---|
| `reduce!(+, m)` | `reduce!(+[0], m)` / `m.sum::<0>()` | — |
| `reduce!(+, m, axis = 1)` | `reduce!(+[1], m)` / `m.sum::<1>()` | — |
| `scan!(max, m)` | `scan!(max[0], m)` / `m.cummax::<0>()` | — |
| `compose(Reduce(Add).axis::<1>(), Transpose).call(&m)` | `train![reduce(+[1]), transpose].call(&m)` | — |
| `m.mean()` / `m.mean_axis::<1>()` (removed) | `m.mean::<0>()` / `m.mean::<1>()` | — |
| `a.zip_with(&b, Mul)` | unchanged | `zip!(op = mul, a, b)` |
| `iota::<6>()` / `x.reshape::<2, 3>()` | unchanged | `iota!(n = 6)` / `reshape!(of = x, rows = 2, cols = 3)` |
| `m.broadcast()` | unchanged | `broadcast!(of = m, rows = R, cols = C)` |

**Compat.** The keyword-axis macros `reduce!(+, m, axis = 1)` and the
default-axis `scan!(max, m)` still compile and the printer still reads
them; they are deprecated in the docs and never emitted. Methods and
`train::compose` stay public; `train!` expands to them. `mean()` /
`mean_axis` were folded into `mean::<K>()` (a breaking change, accepted
because the crate is pre-1.0 and unpublished).

## 2. Glyphs map onto keyword forms

**Implemented (2026-11-26):** the printer has two Rust targets, selected by
`printer::Style` / `arrayglyph-fmt --style <method|macro|both>`: method
style (default) and macro style. Both round-trip with glyphs. The table
below shows them for the reduction/scan/train rows; the `zip!`, `reshape!`,
`iota!` and similar keyword columns are still only a sketch (the printer
emits the method calls for those in both styles).

| glyph | macro style (implemented) / keyword form (sketch) | method style (implemented, default) |
|---|---|---|
| `+⌿x` / `+/[0] x` | `reduce!(+[0], x)` | `x.sum::<0>()` |
| `+/x` / `+/[1] x` | `reduce!(+[1], x)` | `x.sum::<1>()` |
| `-⌿x` `-/x` | `reduce!(-[0], x)` / `reduce!(-[1], x)` | `x.reduce_axis::<0>(Sub)` / `x.reduce_axis::<1>(Sub)` |
| `×⌿x` `×/x` | `reduce!(*[0], x)` / `reduce!(*[1], x)` | `x.prod::<0>()` / `x.prod::<1>()` |
| `÷⌿x` `÷/x` | `reduce!(/[0], x)` / `reduce!(/[1], x)` | `x.reduce_axis::<0>(Div)` / `x.reduce_axis::<1>(Div)` |
| `⌈⌿x` `⌈/x` | `reduce!(max[0], x)` / `reduce!(max[1], x)` | `x.max::<0>()` / `x.max::<1>()` |
| `⌊⌿x` `⌊/x` | `reduce!(min[0], x)` / `reduce!(min[1], x)` | `x.min::<0>()` / `x.min::<1>()` |
| `+⍀x` / `+\[0] x` | `scan!(+[0], x)` | `x.cumsum::<0>()` |
| `+\x` / `+\[1] x` | `scan!(+[1], x)` | `x.cumsum::<1>()` |
| `⌈⍀x` `⌈\x` `⌊⍀x` `⌊\x` | `scan!(max[0], x)` … `scan!(min[1], x)` | `x.cummax::<0>()` … `x.cummin::<1>()` |
| `a + b` `a - b` | `zip!(op = add, a, b)` / `zip!(op = sub, a, b)` | `a.zip_with(&b, Add)` etc. |
| `a × b` `a ÷ b` | `zip!(op = mul, a, b)` / `zip!(op = div, a, b)` | `a.zip_with(&b, Mul)` / `Div` |
| `a ⌈ b` `a ⌊ b` | `zip!(op = max, a, b)` / `zip!(op = min, a, b)` | `a.zip_with(&b, Max)` / `Min` |
| `+⌿x ÷ 14.0` | `reduce!(+[0], x).zip_with(&Array::splat(14.0), Div)` (sketch: `zip!(op = div, reduce!(+[0], x), 14.0)`) | `x.sum::<0>().zip_with(&Array::splat(14.0), Div)` |
| `⍴x` | `shape!(of = x)` (or `x.shape()`, no macro required) | `x.shape()` |
| `2 3⍴x` | `reshape!(of = x, rows = 2, cols = 3)` | `x.reshape::<2, 3>()` |
| `6⍴x` | `reshape!(of = x, rows = 1, cols = 6)` | `x.reshape::<1, 6>()` |
| `⍉x` | `transpose!(of = x)` | `x.transpose()` |
| `⍳6` | `iota!(n = 6)` | `iota::<6>()` |
| `a +.× b` | `matmul!(a, b)` | `a.matmul(&b)` |
| `a ∘.× b` | `outer!(op = mul, a, b)` | Phase 2 |
| `(+/ ∘ ⍉) m` | `train![reduce(+[1]), transpose].call(&m)` | same |
| `(⌈⌿ ∘ +⍀) a` | `train![reduce(max[0]), scan(+[0])].call(&a)` | same |
| `1 2 3` | `vector!(1, 2, 3)` (thin wrapper over `Array::vector`) | `Array::vector([1, 2, 3])` |
| `¯3` | `-3` | `-3` (high minus is glyph orthography only) |

Bracket axis (`f/[0]`, `f/[1]`, `f\[0]`, `f\[1]`) is input-only in glyph
text and normalises to `⌿ / ⍀ \`. In Rust the axis is **always** written,
in both styles, including axis 0, so a reader never has to remember which
slash is the default.

## 3. Errors name the keyword form

Every diagnostic an agent is expected to act on should (a) carry a stable
code `AG00xx`, (b) name the keyword form that failed, and (c) suggest the
keyword-form fix. Glyph text may appear as a hint, not as the only
spelling.

### Sample messages

**Shape mismatch** (`AG0001`). Today this is rustc `E0308` on
`zip_with`/`matmul`. On stable we can tighten the wording with
`#[diagnostic::on_unimplemented]` on a helper trait; we cannot rewrite
E0308 itself without a proc-macro that type-checks shapes before rustc
does.

```text
error[AG0001]: shape mismatch: 2×3 cannot zip with 3×2
 --> src/main.rs:8:17
  |
8 |     zip!(op = add, of = a, b)
  |                    ------  ^ expected 2 rows and 3 columns
  |
  = help: transpose one side: zip!(op = add, of = a, transpose!(of = b))
  = help: or broadcast a length-1 axis: broadcast!(of = b, rows = 2, cols = 3)
```

**Length-1 axis reduce** (`AG0002`). Implemented today as a const
assertion (`E0080`) whose message already suggests the bracket and method
spellings; the `AG0002` code itself is still a sketch.

```text
error[AG0002]: cannot reduce along axis 0 of a 1-row array
 --> src/main.rs:4:5
  |
4 |     reduce!(+[0], v)
  |     ^^^^^^^^^^^^^^^^ axis 0 has length 1; this would be a no-op
  |
  = note: a vector is 1×N; its only meaningful axis is 1
  = help: write `reduce!(+[1], v)` or `v.sum::<1>()`
```

**Axis out of range** (`AG0003`). Achievable today with `AlongAxis` and
`#[diagnostic::on_unimplemented]` (`E0277`).

```text
error[AG0003]: axis 2 is not an axis of a 2-D array
 --> src/main.rs:6:36
  |
6 |     reduce!(+[2], m)
  |               ^
  |
  = help: use [0] (down columns) or [1] (along rows)
```

**Unknown operator** (`AG0004`). Achievable today with `compile_error!`.

```text
error[AG0004]: unknown operator `pow`
 --> src/main.rs:3:18
  |
3 |     reduce!(pow[0], x)
  |             ^^^
  |
  = help: expected one of: + - * / max min
```

**Glyph parse error with span** (`AG0005`). The printer already returns
`ParseError { message, span }` with 1-based `line:col`. Attach the code
and a keyword-form suggestion.

```text
error[AG0005]: unsupported glyph `*` (multiply is `×`; `*` is APL power) (at 1:3)
 --> <glyph>:1:3
  |
1 | a * b
  |   ^
  |
  = help: keyword form: zip!(op = mul, a, b)
```

**Integer / float literal mismatch** (`AG0006`). Today this is a rustc
type error when `÷ 14` becomes `Array::splat(14)` against an `f64` array.
A proc-macro (or a printer lint) can catch it; `macro_rules!` cannot see
the element type.

```text
error[AG0006]: integer literal `14` does not match an f64 array
 --> src/main.rs:9:44
  |
9 |     zip!(op = div, reduce!(+[0], x), 14)
  |                                     ^^ expected f64
  |
  = help: write `14.0` so the literal matches the element type
  = help: zip!(op = div, reduce!(+[0], x), 14.0)
```

### What stable Rust can do vs. a proc-macro

| already doable (`macro_rules!`, const asserts, diagnostics) | needs a proc-macro (or a rustc-side lint) |
|---|---|
| `AG0002` length-1 axis (`const { assert!(…) }`, E0080) | `AG0001` rewritten as a first-class arrayglyph error instead of E0308 |
| `AG0003` bad axis (`#[diagnostic::on_unimplemented]`, E0277) | `AG0006` int/float literal vs. element type |
| `AG0004` unknown `op` (`compile_error!`) | freely ordered keyword arguments |
| `AG0005` glyph spans (existing `ParseError`) | expanding glyph *source* at compile time (`glyph! { +⌿x }`) with those spans in rustc |
| attaching `AG00xx` codes to our own error types and `compile_error!` text | injecting `AG00xx` into rustc's E0308/E0080 labels (we can only wrap or precede them) |

A proc-macro crate is justified once we want `glyph! { … }` or
type-aware literal checks. It is not required to *parse* the keyword
macros or to ship `AG0002`–`AG0005`.

## 4. Glyph-only vs. keyword alias

**Rule:** everything an agent is expected to emit has a keyword form.
Glyphs are a shorthand, never the only spelling of an operation.

| stays without a dedicated keyword macro | reason |
|---|---|
| high minus `¯3` | orthography in glyph *text* only; the keyword/Rust spelling is `-3` |
| numeric strands `1 2 3` as juxtaposition | keyword form is `vector!(1, 2, 3)`; no reason to invent a second literal syntax in Rust |
| `m.map(\|x\| …)` | the function is a Rust closure; a macro adds nothing |
| array-array operators `&a + &b`, scalar `&m * 2.0` | idiomatic Rust, already unambiguous; `zip!` is the named alias, not a replacement |

Trains, forks, outer product, and inner product are **not** glyph-only,
even if the glyph is nicer for humans. They get `train!` (implemented),
`fork!`, `outer!`, `inner!` so an agent never has to emit `∘` or `+.×` to
be correct. Docs may *introduce* a train from its glyph, then give the
keyword form as the signature.

Until Phase 2 lands, `outer!` and `fork!` are specified (as above) and
rejected with a stable “not yet implemented” error (`AG0010`), not left
as undocumented glyph syntax.

## 5. Parseable docs

### rustdoc convention

Every public item (macro, method, function value, error type) uses the
same four sections, in this order, so an agent can split a doc comment
reliably:

```text
# Signature
reduce!(op[axis], x)        e.g. reduce!(+[0], x)
x.sum::<axis>()             (method form)

# Glyph
+⌿x          (axis 0)
+/x          (axis 1)
+/[0] x, +/[1] x   (accepted; normalised)

# Errors
- AG0002: axis length is 1 (e.g. reduce!(+[0], v) on a vector)
- AG0003: axis is not 0 or 1
- AG0004: unknown op

# Example
```

`Signature` is always the canonical form (§0). `Glyph` is omitted only for items
with no glyph (`broadcast!`, `Table::parse_csv`). `Errors` lists codes,
not just rustc error numbers. Existing rustdoc examples stay under
`Example`.

### Machine-readable index

A generated file `docs/api.json` (committed, built by a small
`xtask` or test so it cannot drift) lists every public form:

```json
{
  "version": 1,
  "forms": [
    {
      "name": "reduce!",
      "kind": "macro",
      "signature": "reduce!(op[0 | 1], array)",
      "deprecated_forms": ["reduce!(op, array, axis = k)", "reduce!(op, array)"],
      "glyphs": ["f⌿x", "f/x", "f/[0] x", "f/[1] x"],
      "expands_to": "array.reduce_axis::<axis>(Op)",
      "method_forms": ["array.sum::<axis>()", "array.prod::<axis>()", "array.max::<axis>()", "array.min::<axis>()"],
      "errors": ["AG0002", "AG0003", "AG0004"]
    }
  ],
  "errors": [
    {
      "code": "AG0002",
      "title": "cannot reduce or scan a length-1 axis",
      "fix": "use axis 1 for a vector (1×N), e.g. reduce!(+[1], v); axis 0 for a column (R×1)"
    }
  ]
}
```

### Stable error codes

Codes are sticky: once published, a code is never reused for a different
failure. New failures get a new number. Suggested starting set:

| code | failure |
|---|---|
| `AG0001` | shape mismatch (zip, matmul, broadcast target) |
| `AG0002` | reduce/scan/mean on a length-1 axis |
| `AG0003` | axis not in `{0, 1}` |
| `AG0004` | unknown operator |
| `AG0005` | glyph parse error (span attached) |
| `AG0006` | integer/float literal does not match the element type |
| `AG0007` | reshape does not preserve the element count (today E0080) |
| `AG0008` | n-wise reduce (`a +/ b`) is unsupported |
| `AG0009` | unknown `train!` item (today a `compile_error!` naming the allowed items) |
| `AG0010` | form reserved but not implemented (outer, fork, `glyph! { }`) |

## 6. Open questions

1. ~~Must agents always write the axis?~~ **Decided (S, 2026-11-26): yes,
   for everyone.** The axis is always written (`reduce!(+[0], m)`,
   `m.sum::<0>()`), and the printer always emits it.
2. **`of = x` versus a positional array.** All-keyword is the least
   ambiguous (`zip!(op = mul, of = a, b)`). Reductions settled on a
   positional array (`reduce!(+[1], x)`). A single positional
   array is shorter. Do we require `of =` in the canonical agent form, or
   only recommend it?
3. ~~Nested `compose!` vs. a flat train?~~ **Decided (S, 2026-11-26) and
   implemented:** flat `train![reduce(+[1]), transpose]`, in glyph order
   (rightmost runs first). It turned out not to need a proc-macro: a
   `macro_rules!` tt-muncher builds left-nested `compose` calls.
4. **When to add a proc-macro crate.** Keyword macros, `AG0002`–`AG0005`,
   and `api.json` do not need one. `glyph! { +⌿x }`, `AG0001`/`AG0006` as
   first-class errors, and free keyword order do. Is that a Phase 2 item,
   or part of implementing this sketch?
5. ~~What the printer emits by default.~~ **Decided (S, 2026-11-26) and
   implemented:** method style (`m.sum::<0>()`) by default; macro style
   (`reduce!(+[0], m)`) with `--style macro`; both with `--style both`.
   Glyphs remain the primary notation.

