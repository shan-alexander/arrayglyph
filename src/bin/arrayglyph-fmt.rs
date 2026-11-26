//! `arrayglyph-fmt`: converts between glyph notation and canonical verbose Rust.
//!
//! ```text
//! arrayglyph-fmt [--to-rust | --to-glyph] [EXPR ...]
//! ```
//!
//! * `--to-rust` (default) desugars glyph notation: `+/(a × b)` becomes
//!   `a.zip_with(&b, Mul).reduce_axis::<1>(Add)`.
//! * `--to-glyph` compresses canonical Rust back to glyphs.
//!
//! With no `EXPR` arguments it converts stdin line by line, skipping blank
//! lines. It exits with status 1 if any input fails to parse.

use std::io::{self, BufRead, Write};
use std::process::ExitCode;

use arrayglyph::printer::{glyph_to_rust, rust_to_glyph};

const USAGE: &str = "\
usage: arrayglyph-fmt [--to-rust | --to-glyph] [EXPR ...]

  --to-rust   glyph notation -> canonical verbose Rust (default)
  --to-glyph  canonical verbose Rust -> glyph notation
  -h, --help  show this help

With no EXPR, reads one expression per line from stdin.

examples:
  arrayglyph-fmt '+/(a × b)'
  arrayglyph-fmt --to-glyph 'a.zip_with(&b, Mul).reduce_axis::<1>(Add)'";

fn main() -> ExitCode {
    let mut to_rust = true;
    let mut exprs = Vec::new();
    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "--to-rust" => to_rust = true,
            "--to-glyph" => to_rust = false,
            "-h" | "--help" => {
                println!("{USAGE}");
                return ExitCode::SUCCESS;
            }
            s if s.starts_with("--") => {
                eprintln!("unknown flag `{s}`\n\n{USAGE}");
                return ExitCode::from(2);
            }
            _ => exprs.push(arg),
        }
    }
    let convert = |s: &str| {
        if to_rust {
            glyph_to_rust(s)
        } else {
            rust_to_glyph(s)
        }
    };

    let inputs: Vec<String> = if exprs.is_empty() {
        io::stdin().lock().lines().map_while(Result::ok).collect()
    } else {
        exprs
    };

    let mut ok = true;
    let mut out = io::stdout().lock();
    for line in inputs.iter().map(|l| l.trim()).filter(|l| !l.is_empty()) {
        match convert(line) {
            Ok(s) => {
                let _ = writeln!(out, "{s}");
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
