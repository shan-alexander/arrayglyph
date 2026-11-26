//! arrayglyph showcase: two weeks of (synthetic) daily highs in five cities.
//!
//! Every computation below is written twice, on purpose:
//!
//! * as **glyph notation** (dense, APL-style, easy for an agent to write and
//!   for a reviewer to skim), and
//! * as the **verbose Rust** that actually runs.
//!
//! The `show!` macro asserts that the pretty-printer's translation of the
//! glyph text (method style, the default) is token-for-token the Rust that
//! runs, so the two can't drift. It also prints the macro-style spelling
//! (`arrayglyph-fmt --style macro`) for comparison.
//!
//! The data in `examples/data/weather.csv` is SYNTHETIC sample data made up
//! for this demo. It is not a record of real temperatures.
//!
//! Run with `cargo run --example showcase`.

use arrayglyph::prelude::*;
use arrayglyph::printer::{Style, glyph_to_rust, glyph_to_rust_with};
use arrayglyph::table::Table;

/// 14 days × 5 cities, embedded at compile time (no file I/O, no deps).
const WEATHER_CSV: &str = include_str!("data/weather.csv");
const DAYS: usize = 14;
const CITIES: usize = 5;

fn squash(s: &str) -> String {
    s.chars().filter(|c| !c.is_whitespace()).collect()
}

/// Prints a glyph expression and the verbose Rust the printer turns it into,
/// checks that this Rust is exactly the code written after `=>`, then runs it.
macro_rules! show {
    ($glyph:literal => $($rust:tt)*) => {{
        let printed = glyph_to_rust($glyph).expect("valid glyph");
        assert_eq!(squash(&printed), squash(stringify!($($rust)*)), "printer drifted");
        let as_macro = glyph_to_rust_with($glyph, Style::Macro).expect("valid glyph");
        println!("  glyph  │ {}", $glyph);
        println!("  rust   │ {printed}");
        if as_macro != printed {
            println!("  macro  │ {as_macro}");
        }
        $($rust)*
    }};
}

fn heading(title: &str) {
    println!("\n{title}\n{}", "─".repeat(title.chars().count()));
}

/// One labelled row of numbers, aligned under the city names.
fn labelled(label: &str, labels: &[String], values: &[f64]) {
    let cells: Vec<String> = labels
        .iter()
        .zip(values)
        .map(|(l, v)| format!("{:>w$.1}", v, w = l.len().max(6)))
        .collect();
    println!("  {label:<14}{}", cells.join("  "));
}

fn header(labels: &[String]) {
    let cells: Vec<String> = labels.iter().map(|l| format!("{l:>6}")).collect();
    println!("  {:<14}{}", "", cells.join("  "));
}

fn main() {
    // ── Load: shape checked once here, then carried in the type ─────────────
    let table: Table<f64, DAYS, CITIES> =
        Table::parse_csv(WEATHER_CSV).expect("weather.csv is 14 days × 5 cities");
    let cities = &table.col_labels;
    let days = &table.row_labels;
    let x: Array<f64, DAYS, CITIES> = table.data;

    println!("arrayglyph showcase · daily highs, °C · SYNTHETIC sample data");
    println!("x : Array<f64, {DAYS}, {CITIES}>   (rows = days, columns = cities)\n");
    header(cities);
    for (d, row) in days.iter().zip(x.rows()) {
        labelled(d, cities, row);
    }

    // ── Axis 0: one result per city ─────────────────────────────────────────
    heading("1 · Per city: reduce down each column (axis 0)");
    let mean = show!("+⌿x ÷ 14.0" => x.sum::<0>().zip_with(&Array::splat(14.0), Div));
    assert_eq!(mean, x.mean::<0>()); // the `mean::<0>()` helper says the same thing
    let hi = show!("⌈⌿x" => x.max::<0>());
    let range = show!("⌈⌿x - ⌊⌿x" => x.max::<0>().zip_with(&x.min::<0>(), Sub));
    println!();
    header(cities);
    labelled("mean", cities, mean.as_slice());
    labelled("warmest day", cities, hi.as_slice());
    labelled("range", cities, range.as_slice());

    // ── Axis 1: one result per day ──────────────────────────────────────────
    heading("2 · Per day: reduce along each row (axis 1)");
    let daily = show!("+/x ÷ 5.0" => x.sum::<1>().zip_with(&Array::splat(5.0), Div));
    let spread = show!("⌈/x - ⌊/x" => x.max::<1>().zip_with(&x.min::<1>(), Sub));
    println!(
        "\n  {:<14}{:>12}  {:>18}",
        "", "5-city mean", "warmest − coldest"
    );
    for (i, d) in days.iter().enumerate() {
        println!("  {d:<14}{:>12.1}  {:>18.1}", daily[(i, 0)], spread[(i, 0)]);
    }

    // Both axes is two calls, and the order doesn't matter for `+`.
    let total = show!("+/+⌿x" => x.sum::<0>().sum::<1>()).item();
    let total2 = show!("+⌿+/x" => x.sum::<1>().sum::<0>()).item();
    assert!((total - total2).abs() < 1e-9);
    println!(
        "  grand mean over all {} readings: {:.2} °C",
        DAYS * CITIES,
        total / (DAYS * CITIES) as f64
    );

    // ── Compose: a reusable pipeline built with ∘ ───────────────────────────
    heading("3 · Compose: weekly means per city, as one train");
    // `train![…]` lists the functions in the same order as the glyph, so
    // read both right to left:
    //   ⍉      14×5 → 5×14   one row per city
    //   10 7⍴  5×14 → 10×7   one row per city-week
    //   +/     10×7 → 10×1   weekly totals
    //   5 2⍴   10×1 → 5×2    back to one row per city, one column per week
    // and then ÷ 7.0 turns totals into means. Every intermediate shape is a
    // type, so a wrong reshape here would not compile.
    let weekly = show!("(5 2⍴ ∘ +/ ∘ 10 7⍴ ∘ ⍉) x ÷ 7.0"
        => train![reshape(5, 2), reduce(+[1]), reshape(10, 7), transpose]
            .call(&x)
            .zip_with(&Array::splat(7.0), Div));
    println!(
        "\n  {:<14}{:>8}  {:>8}  {:>8}",
        "", "week 1", "week 2", "change"
    );
    for (c, row) in cities.iter().zip(weekly.rows()) {
        println!(
            "  {c:<14}{:>8.1}  {:>8.1}  {:>+8.1}",
            row[0],
            row[1],
            row[1] - row[0]
        );
    }

    // ── Anomalies and warm spells ───────────────────────────────────────────
    heading("4 · Warm spells: running sum of each city's daily anomaly");
    // Departure from each city's own mean. `broadcast` stretches the 1×5
    // means down all 14 rows. (Glyph text has no implicit broadcasting yet;
    // see examples/WISHLIST.md.)
    let a = x.zip_with(&mean.broadcast(), Sub);
    // (⌈⌿ ∘ +⍀) a: running total of the anomaly down each column, then the
    // largest value it reaches: how far above "normal" the warmest spell got.
    let spell = show!("(⌈⌿ ∘ +⍀) a" => train![reduce(max[0]), scan(+[0])].call(&a));
    let peak = cities
        .iter()
        .zip(spell.as_slice())
        .max_by(|p, q| p.1.total_cmp(q.1))
        .expect("five cities");
    println!();
    header(cities);
    labelled("peak warmth", cities, spell.as_slice());
    println!(
        "  strongest warm spell: {} (+{:.1} °C·days above its own mean)",
        peak.0, peak.1
    );

    // ── The printer goes the other way too ─────────────────────────────────
    heading("5 · Verbose Rust → glyph (either style, and older spellings)");
    for rust in [
        "x.max::<0>().zip_with(&x.min::<0>(), Sub)",
        "reduce!(+[1], x)",
        "reduce!(+, x, axis = 1)",
        "train![reduce(max[0]), scan(+[0])].call(&a)",
        "compose(Reduce(Max), Scan(Add)).call(&a)",
    ] {
        let glyph = arrayglyph::printer::rust_to_glyph(rust).expect("canonical Rust");
        println!("  {glyph:<14} ⇐ {rust}");
    }

    // ── Profiling ──────────────────────────────────────────────────────────
    heading("6 · prof!: time one expression (report goes to stderr)");
    let weekly_again = prof!(
        glyph = "(5 2⍴ ∘ +/ ∘ 10 7⍴ ∘ ⍉) x",
        train![reshape(5, 2), reduce(+[1]), reshape(10, 7), transpose].call(&x)
    );
    assert_eq!(weekly_again.zip_with(&Array::splat(7.0), Div), weekly);
    println!("  (see the [prof] line on stderr: location, glyph, Rust, shape 5×2, time)");
}
