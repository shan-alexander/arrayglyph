# Syn research notes for arrayglyph

Research on dtolnay's [syn](https://github.com/dtolnay/syn) (docs: [docs.rs/syn](https://docs.rs/syn/latest/syn/); studied **v3.0.10**, current `master` as of 2026-11-25). Feeds DESIGN.md and Phase 2; does not change Phase 1 scope. Download/dependent-crate counts omitted (not re-verified here).

## 1. Module organization

Public surface is feature-gated in [`src/lib.rs`](https://github.com/dtolnay/syn/blob/master/src/lib.rs): `parse`, `punctuated`, `token`, `spanned`, `buffer` (parsing), `ext`, plus `visit` / `visit_mut` / `fold`. Supporting modules (`error`, `lookahead`, `precedence`, `expr`, `op`) stay private except for re-exports.

- **`parse`**: `Parse` / `ParseStream` (`&ParseBuffer`) / `Parser` / `Lookahead1`. Parser fns are `fn(ParseStream) -> Result<T>`.
- **`buffer`**: `TokenBuffer` + cheap `Cursor` over a flattened token array; forking does not copy the buffer ([`src/buffer.rs`](https://github.com/dtolnay/syn/blob/master/src/buffer.rs)).
- **`token`**: typed keywords/punct/delims; `Token![+]`, `parenthesized!` / `bracketed!` / `braced!`; `custom_keyword!` / `custom_punctuation!`.
- **`punctuated`**: `Punctuated<T, P>` = `Vec<(T, P)>` plus optional trailing `T`.
- **`spanned`**: `Spanned::span()` over any `ToTokens` node.
- **`visit` / `visit_mut` / `fold`**: hook traits (shared walk / in-place mut / owned transform). Default methods recurse; override a node and call the free `visit_*` / `fold_*` to continue.
- **`ext`**: sealed helpers (`IdentExt::parse_any` for keyword-as-ident, etc.).

**Codegen.** Internal crate [`codegen/`](https://github.com/dtolnay/syn/blob/master/codegen/README.md) reads a syntax-tree description (`syn.json`) and writes `src/gen/{visit,visit_mut,fold,clone,debug,eq,hash}.rs`. It is **not** run from `build.rs` (too slow); maintainers run `cargo run` in `codegen/`. This keeps traversal in lockstep with the AST.

**Applies to arrayglyph.** Mirror the layout, not the code generator, until the glyph AST is large: `token` (glyphs + spans), `parse`, `print` (verbose Rust *and* glyph), `punctuated` (operator-separated array exprs), `visit`/`fold` for the pretty-printer, `error`/`span`. A `syn.json`-style description + codegen is worth it only once node kinds proliferate (trains, axis, inner/outer product). Do not invent a second Rust parser; reuse syn for Rust fragments.

## 2. Parse / ParseStream / ToTokens

Bidirectional core ([`parse` module](https://docs.rs/syn/latest/syn/parse/index.html)):

- **`Parse`**: default parse. Types with context-dependent rules (`Punctuated` trailing punct, outer vs inner attrs) omit `Parse` and expose named parsers invoked via **`Parser`**.
- **`ParseStream`**: `parse::<T>()`, `call(fn)`, `peek` / `peek2` / `peek3`, `lookahead1()` (peek alternatives → “expected one of: …” via `lookahead.error()`), `fork()` (cheap; do not parse unbounded input on a fork — use `step` or `discouraged::Speculative` if you must commit), `error(msg)`, `parse_terminated`.
- **`ToTokens`** (from [quote](https://docs.rs/quote), enabled by syn's `printing` feature): AST → `TokenStream`. Quasi-quote (`quote!` / `parse_quote!`) is the usual codegen path. `parse` + `ToTokens` is the round-trip.

**Error combination.** `Error` holds a `Vec` of messages; `combine` concatenates them so `to_compile_error()` emits every diagnostic at once ([`src/error.rs`](https://github.com/dtolnay/syn/blob/master/src/error.rs)).

**Applies to arrayglyph.** Define a small `Glyph` AST with `Parse`-like (`fn parse(GlyphStream) -> Result<T>`) and two printers: `ToRustTokens` (verbose, parenthesized Rust) and `ToGlyph` (dense form). Fork/lookahead matter for distinguishing reduction/scan/product/train syntax. If Phase 2 compiles glyph files or a `glyph! { ... }` proc-macro, depend on **syn + quote + proc-macro2** for the Rust parts; implement the glyph sublanguage ourselves. Do not depend on them in Phase 1 (declarative macros + a hand-rolled desugarer are enough).

## 3. Span tracking

Every token carries a `proc_macro2::Span`. `Spanned` joins first/last token spans, but `Span::join` is nightly-only; on stable it often returns only the first token. **`Error::new_spanned(tokens, msg)`** records start *and* end spans from a `ToTokens` value and is the recommended stable way to underline a whole node. `Error::new(span, msg)` / `ParseStream::error` for a point; `Error::new_range(cursor_range, msg)` for a parse-buffer range. `to_compile_error()` / `into_compile_error()` emit a spanned `compile_error!` invocation — the correct proc-macro error path, not a panic ([docs](https://docs.rs/syn/latest/syn/struct.Error.html)).

**Applies to arrayglyph.** Put a `Span` (or file + byte range, if we also parse `.ag` files outside rustc) on every glyph token and AST node **from the first printer implementation**, even if Phase 1 only desugars. Retrofitting spans is costly. Desugared Rust should use `quote_spanned!` (Phase 2) so rustc errors (shape mismatches, trait bounds) point at the glyph, not the expansion. Phase 2 pretty-printer polish (“span tracking so desugared output maps back to glyph source”) is exactly this plus a source-map for editor tooling.

## 4. Feature-flagged API

From [`Cargo.toml`](https://github.com/dtolnay/syn/blob/master/Cargo.toml) and [lib.rs “Optional features”](https://docs.rs/syn/latest/syn/#optional-features). **Defaults:** `derive`, `parsing`, `printing`, `clone-impls`, `proc-macro`. **Opt-in:** `full` (items/exprs, not just derive input), `visit`, `visit-mut`, `fold`, `extra-traits` (Debug/Eq/Hash). `printing` pulls `quote`. `proc-macro` forwards to `proc-macro2`/`quote` (disable for non-macro contexts). Goal: consumers do not compile the full AST / traversals if they only need `DeriveInput`.

**Applies to arrayglyph.** Gate similarly, e.g. default = shaped `Array` + `reduce!`/`scan!`; opt-in `printer` (desugar), `parsing` (verbose→glyph, Phase 2), `rayon` (parallel reduce/scan), later `proc-macro` / `full` (glyph files, trains). Keep the Phase 1 default surface small so a data-pipeline user does not pay for the pretty-printer.

## 5. Punctuated lists and binary-operator precedence

`Punctuated<T, P>` models operator- or comma-separated sequences, retaining the punctuation tokens (and their spans). Parsers: `parse_terminated` (optional trailing punct), `parse_separated_nonempty` (no trailing). Good for `f, g, h` or `a ⍴ b` lists; **not** sufficient alone for mixed-precedence expression grammar.

Expression parsing is a **precedence climber** in [`src/expr.rs`](https://github.com/dtolnay/syn/blob/master/src/expr.rs) (`parse_expr` / `parse_binop_rhs`), with levels in [`src/precedence.rs`](https://github.com/dtolnay/syn/blob/master/src/precedence.rs) taken from the [Rust reference](https://doc.rust-lang.org/reference/expressions.html#expression-precedence) (Jump < Assign < Range < Or < And < Compare < BitOr < … < Sum < Product < Cast < Prefix < Unambiguous). Most binops are **left-associative**; assignment is **right-associative** (`next == precedence && precedence == Assign` continues climbing). Comparisons cannot be chained. Printing re-inserts parentheses from the same table (`print_expr_binary`). Precedence is tested by parenthesizing every subexpr and comparing to rustc (`tests/test_precedence.rs`).

**Right-to-left vs Rust (critical).** APL dyadic functions share one precedence and associate **right-to-left**: `2×3+4` is `2×(3+4)`. Rust is left-associative with ranked precedence: `2*3+4` is `(2*3)+4`. These disagree. Glyph expressions must be parsed in a **delimited context** (`glyph! { ... }` or a `.ag` file), never dropped bare into ordinary Rust where rustc's grammar wins. The glyph climber should default to APL: one dyadic level, right-associative. `ToRustTokens` **must parenthesize** so the verbose form preserves that meaning under rustc. Document this as a hard semantic rule; do not “split the difference.”

**Applies to arrayglyph.** Use a `Punctuated`-like type for homogeneous operator lists (e.g. train items). For mixed glyph expressions, copy the climber but invert associativity and flatten dyadic precedence. `custom_punctuation!` helps only for ASCII multi-char ops; many APL glyphs will arrive as `proc_macro2` `Ident` or `Punct` — define a `GlyphToken` enum with spans rather than forcing everything through syn's `Token!`.

## 6. Testing strategy

Syn itself does **not** use trybuild. It **recommends** [trybuild](https://github.com/dtolnay/trybuild) for *macro authors*: compile-fail UI tests so diagnostics do not regress ([README “Testing”](https://github.com/dtolnay/syn#testing)). Syn's own suite:

- **insta snapshots** of a compact `Lite` Debug form (`tests/snapshot/`, `tests/debug/`).
- **Round-trip** (`tests/test_round_trip.rs`): parse rust-lang/rust sources with syn, print via `ToTokens`, reparse with **librustc** (`rustc_private`, nightly), compare ASTs spanlessly.
- **Precedence** (`tests/test_precedence.rs`): same corpus; fully parenthesize via syn vs rustc and compare.
- Unit tests per node kind; feature-matrix crate under `tests/features`.

**Applies to arrayglyph.**

- Phase 1 / ongoing: **trybuild `compile_fail`** for `Array<T, R, C>` shape mismatches (these are rustc const-generic errors — trybuild is the right tool even without a proc-macro).
- Round-trip: glyph → verbose Rust → (Phase 2) parse back; plus property tests vs explicit loops (already in the Phase 2 plan).
- insta snapshots of desugared Rust and, later, of parse errors.
- When a proc-macro exists: trybuild for glyph parse errors (span + message).
- Do not take on rustc_private corpus tests.

## Recommendations for DESIGN.md / Phase 2

1. **AST + spans first.** `GlyphToken` / `GlyphExpr` with spans; `Parse` + `ToRustTokens` + `ToGlyph`; `Visit`/`Fold` for the pretty-printer. Stub `ToGlyph` in Phase 1 if needed, but keep span fields now.
2. **Delimit glyph syntax; APL associativity is law.** Parse only inside a macro or glyph file. Right-to-left, single dyadic precedence; verbose output fully parenthesized. Record this in DESIGN.md open questions as a proposed resolution, not an open fight with Rust precedence.
3. **Depend on syn/quote/proc-macro2 in Phase 2+ only**, when compiling mixed Rust+glyph input. Mirror patterns for the glyph language; do not fork syn. Phase 1 stays declarative macros.
4. **Feature flags:** default core arrays/macros; `printer`, `parsing`, `rayon` opt-in.
5. **trybuild** for shape-mismatch (and later glyph) diagnostics; insta for printer snapshots; proptest for semantics. No rustc_private.
6. **Codegen for visit/fold** only after the primitive set grows (axis, inner/outer product, trains). Until then, hand-written walks are fine.
