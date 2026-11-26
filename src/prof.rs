//! Support for [`prof!`](crate::prof!): shape descriptions and the timing
//! runner.

use std::time::{Duration, Instant};

use crate::Array;

/// A value whose shape [`prof!`](crate::prof!) can describe.
///
/// ```
/// use arrayglyph::{prof::Profile, Array};
/// assert_eq!(Array::from_rows([[1, 2, 3], [4, 5, 6]]).shape_desc(), "2×3");
/// assert_eq!(7.5_f64.shape_desc(), "scalar");
/// assert_eq!((2_usize, 3_usize).shape_desc(), "shape (2, 3)");
/// ```
#[diagnostic::on_unimplemented(
    message = "`prof!` cannot describe the shape of `{Self}`",
    label = "this expression's type has no `Profile` impl",
    note = "`prof!` works on arrays, `shape()` tuples, and primitive numbers; implement `arrayglyph::prof::Profile` for other types"
)]
pub trait Profile {
    /// A short shape description, such as `"2×3"` or `"scalar"`.
    fn shape_desc(&self) -> String;
}

impl<T, const ROWS: usize, const COLS: usize> Profile for Array<T, ROWS, COLS> {
    fn shape_desc(&self) -> String {
        format!("{ROWS}×{COLS}")
    }
}

impl Profile for (usize, usize) {
    fn shape_desc(&self) -> String {
        format!("shape {self:?}")
    }
}

macro_rules! scalar_profile {
    ($($t:ty),*) => {$(
        impl Profile for $t {
            fn shape_desc(&self) -> String {
                "scalar".to_string()
            }
        }
    )*};
}
scalar_profile!(
    i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize, f32, f64, bool
);

/// Formats one profile line. Separate from printing so it can be tested.
///
/// ```
/// use std::time::Duration;
/// use arrayglyph::prof::format_line;
/// let line = format_line("src/main.rs", 7, Some("+⌿m"), "m.sum::<0>()", "1×3", Duration::from_micros(3));
/// assert_eq!(line, "[prof] src/main.rs:7  +⌿m  ⇐  m.sum::<0>()  → 1×3  in 3µs");
/// let line = format_line("a.rs", 1, None, "x", "scalar", Duration::from_nanos(40));
/// assert_eq!(line, "[prof] a.rs:1  x  → scalar  in 40ns");
/// ```
pub fn format_line(
    file: &str,
    line: u32,
    glyph: Option<&str>,
    expr: &str,
    shape: &str,
    elapsed: Duration,
) -> String {
    let what = match glyph {
        Some(g) => format!("{g}  ⇐  {expr}"),
        None => expr.to_string(),
    };
    format!("[prof] {file}:{line}  {what}  → {shape}  in {elapsed:?}")
}

/// Runs `f`, times it, prints the profile line to stderr, and returns the
/// value. Called by [`prof!`](crate::prof!); not meant to be called
/// directly.
///
/// ```
/// let v = arrayglyph::prof::__run("x.rs", 1, None, "40 + 2", || 40 + 2);
/// assert_eq!(v, 42);
/// ```
#[doc(hidden)]
pub fn __run<R: Profile>(
    file: &str,
    line: u32,
    glyph: Option<&str>,
    expr: &str,
    f: impl FnOnce() -> R,
) -> R {
    let start = Instant::now();
    let value = f();
    let elapsed = start.elapsed();
    eprintln!(
        "{}",
        format_line(file, line, glyph, expr, &value.shape_desc(), elapsed)
    );
    value
}
