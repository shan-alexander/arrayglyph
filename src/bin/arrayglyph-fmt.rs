//! `arrayglyph-fmt`: converts between glyph notation and canonical verbose Rust.
//!
//! ```text
//! arrayglyph-fmt [--to-rust | --to-glyph] [--style <method|macro|both>] [EXPR ...]
//! ```
//!
//! * `--to-rust` (default) desugars glyph notation. `--style` picks the Rust
//!   spelling of reductions and scans:
//!   * `method` (default): `+/(a × b)` → `a.zip_with(&b, Mul).sum::<1>()`
//!   * `macro`: `+/(a × b)` → `reduce!(+[1], a.zip_with(&b, Mul))`
//!   * `both`: prints the glyph, method and macro forms as labelled lines.
//! * `--to-glyph` compresses canonical Rust (either style, or the older
//!   spellings) back to glyphs. With `--style both` it also prints the two
//!   Rust forms; `method` and `macro` don't change glyph output.
//!
//! With no `EXPR` arguments it converts stdin line by line, skipping blank
//! lines. Put `--` before expressions that start with `-` (`-- -⍀m`), as
//! with any clap CLI. It exits with status 1 if any input fails to parse, and 2 on a
//! usage error.

use std::io::{self, BufRead, Write};
use std::process::ExitCode;

use arrayglyph::printer::{Expr, Style};
use clap::{Parser, ValueEnum};

/// Convert between arrayglyph glyph notation and canonical verbose Rust.
#[derive(Debug, Parser)]
#[command(
    name = "arrayglyph-fmt",
    version,
    after_help = "With no EXPR, reads one expression per line from stdin.\n\n\
examples:\n  \
arrayglyph-fmt '+/(a × b)'\n  \
arrayglyph-fmt --style macro '+/(a × b)'\n  \
arrayglyph-fmt --style both '(+/ ∘ ⍉) m'\n  \
arrayglyph-fmt -- '-⍀m'\n  \
arrayglyph-fmt --to-glyph 'a.zip_with(&b, Mul).sum::<1>()'"
)]
struct Cli {
    /// Glyph notation -> canonical verbose Rust (the default).
    #[arg(long, conflicts_with = "to_glyph")]
    to_rust: bool,

    /// Canonical verbose Rust -> glyph notation.
    #[arg(long)]
    to_glyph: bool,

    /// Rust spelling for reductions and scans.
    #[arg(long, value_enum, default_value_t = StyleArg::Method)]
    style: StyleArg,

    /// Expressions to convert. Reads stdin, one per line, if none are given.
    /// Put `--` before an expression that starts with `-`, e.g. `-- -⍀m`.
    exprs: Vec<String>,
}

/// The `--style` values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum StyleArg {
    /// Named methods with a const axis: `m.sum::<0>()`, `m.cumprod::<1>()`.
    Method,
    /// Macros with a bracketed axis: `reduce!(+[0], m)`, `scan!(*[1], m)`.
    Macro,
    /// Glyph, method and macro forms on three labelled lines.
    Both,
}

fn render(expr: &Expr, to_glyph: bool, style: StyleArg) -> String {
    match (style, to_glyph) {
        (StyleArg::Both, _) => format!(
            "glyph:  {}\nmethod: {}\nmacro:  {}",
            expr.to_glyph(),
            expr.to_rust_styled(Style::Method),
            expr.to_rust_styled(Style::Macro)
        ),
        (_, true) => expr.to_glyph(),
        (StyleArg::Method, false) => expr.to_rust_styled(Style::Method),
        (StyleArg::Macro, false) => expr.to_rust_styled(Style::Macro),
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let parse = |s: &str| {
        if cli.to_glyph {
            Expr::parse_rust(s)
        } else {
            Expr::parse_glyph(s)
        }
    };

    let inputs: Vec<String> = if cli.exprs.is_empty() {
        io::stdin().lock().lines().map_while(Result::ok).collect()
    } else {
        cli.exprs.clone()
    };

    let mut ok = true;
    let mut out = io::stdout().lock();
    for line in inputs.iter().map(|l| l.trim()).filter(|l| !l.is_empty()) {
        match parse(line) {
            Ok(expr) => {
                let _ = writeln!(out, "{}", render(&expr, cli.to_glyph, cli.style));
            }
            Err(e) => {
                ok = false;
                // Minimal caret; rich span rendering is Phase 2.
                eprintln!(
                    "error: {e}\n  {line}\n  {}^",
                    " ".repeat(e.span.start.col - 1)
                );
            }
        }
    }
    if ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn cli_definition_is_valid() {
        Cli::command().debug_assert();
    }

    #[test]
    fn style_flag_spellings() {
        let cli = Cli::try_parse_from(["fmt", "--style", "macro", "+/m"]).unwrap();
        assert_eq!(cli.style, StyleArg::Macro);
        let cli = Cli::try_parse_from(["fmt", "+/m"]).unwrap();
        assert_eq!(cli.style, StyleArg::Method);
        assert!(Cli::try_parse_from(["fmt", "--style", "keyword"]).is_err());
        assert!(Cli::try_parse_from(["fmt", "--to-rust", "--to-glyph"]).is_err());
        // Flags may follow expressions; `--` passes expressions that start with `-`.
        let cli = Cli::try_parse_from(["fmt", "+/m", "--style", "macro"]).unwrap();
        assert_eq!(
            (cli.style, cli.exprs.as_slice()),
            (StyleArg::Macro, &["+/m".to_string()][..])
        );
        let cli = Cli::try_parse_from(["fmt", "--style", "both", "--", "-⍀m", "-/v"]).unwrap();
        assert_eq!(cli.exprs, ["-⍀m", "-/v"]);
    }

    #[test]
    fn render_each_style() {
        let e = Expr::parse_glyph("+/(a × b)").unwrap();
        assert_eq!(
            render(&e, false, StyleArg::Method),
            "a.zip_with(&b, Mul).sum::<1>()"
        );
        assert_eq!(
            render(&e, false, StyleArg::Macro),
            "reduce!(+[1], a.zip_with(&b, Mul))"
        );
        assert_eq!(
            render(&e, false, StyleArg::Both),
            "glyph:  +/(a × b)\nmethod: a.zip_with(&b, Mul).sum::<1>()\nmacro:  reduce!(+[1], a.zip_with(&b, Mul))"
        );
        let r = Expr::parse_rust("reduce!(+[1], m)").unwrap();
        assert_eq!(render(&r, true, StyleArg::Macro), "+/m");
    }
}
