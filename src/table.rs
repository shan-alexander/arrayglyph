//! A tiny labelled-CSV loader: row labels, column labels, and a shaped
//! [`Array`] of values.
//!
//! The text's shape is checked when it is parsed (CSV is run-time data), and
//! from then on the array's shape is part of its type, so everything you
//! compute from it is shape-checked at compile time.
//!
//! The format is deliberately small: comma-separated, one header line, the
//! first column holds row labels, `#` starts a comment line, blank lines are
//! skipped, and cells are trimmed. Quoted fields are not supported.
//!
//! ```
//! use arrayglyph::table::Table;
//!
//! let csv = "# synthetic example\n\
//!            city,  Jan, Feb\n\
//!            Oslo,  -4,  -3\n\
//!            Rome,  12,  13\n";
//! let t: Table<i32, 2, 2> = Table::parse_csv(csv).unwrap();
//! assert_eq!(t.col_labels, ["Jan", "Feb"]);
//! assert_eq!(t.row_labels, ["Oslo", "Rome"]);
//! assert_eq!(t.data.row(1), &[12, 13]);
//! assert_eq!(t.index_name, "city");
//! ```

use core::fmt;
use core::str::FromStr;

use crate::Array;

/// A labelled table: `ROWS × COLS` values plus a label for every row and
/// column.
///
/// ```
/// use arrayglyph::table::Table;
/// let t: Table<f64, 1, 2> = Table::parse_csv("k,a,b\nx,1.5,2").unwrap();
/// assert_eq!(t.data.as_slice(), &[1.5, 2.0]);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Table<T, const ROWS: usize, const COLS: usize> {
    /// The header's first cell (the name of the label column).
    pub index_name: String,
    /// One label per row, from the first column.
    pub row_labels: Vec<String>,
    /// One label per column, from the header.
    pub col_labels: Vec<String>,
    /// The values.
    pub data: Array<T, ROWS, COLS>,
}

/// Why CSV text didn't load, with the 1-based line it was found on.
///
/// ```
/// use arrayglyph::table::Table;
/// let err = Table::<i32, 1, 2>::parse_csv("k,a,b\nx,1,oops").unwrap_err();
/// assert_eq!(err.line, 2);
/// assert!(err.message.contains("oops"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CsvError {
    /// 1-based line number (0 when the problem is the whole input, such as
    /// too few rows).
    pub line: usize,
    /// What went wrong.
    pub message: String,
}

impl fmt::Display for CsvError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.line == 0 {
            write!(f, "csv: {}", self.message)
        } else {
            write!(f, "csv line {}: {}", self.line, self.message)
        }
    }
}

impl std::error::Error for CsvError {}

impl<T: FromStr, const ROWS: usize, const COLS: usize> Table<T, ROWS, COLS> {
    /// Parses labelled CSV text into a `ROWS × COLS` table.
    ///
    /// # Errors
    ///
    /// Returns [`CsvError`] if the header doesn't have `COLS + 1` cells, a
    /// row doesn't have `COLS + 1` cells, a value doesn't parse as `T`, or
    /// there aren't exactly `ROWS` data rows.
    ///
    /// ```
    /// use arrayglyph::table::Table;
    /// let csv = "day,a,b\nMon,1,2\nTue,3,4\nWed,5,6";
    /// let err = Table::<i32, 2, 2>::parse_csv(csv).unwrap_err();
    /// assert_eq!(err.to_string(), "csv line 4: expected 2 data rows, found more");
    /// ```
    pub fn parse_csv(text: &str) -> Result<Self, CsvError> {
        let err = |line: usize, message: String| CsvError { line, message };
        let mut lines = text
            .lines()
            .enumerate()
            .map(|(i, l)| (i + 1, l.trim()))
            .filter(|(_, l)| !l.is_empty() && !l.starts_with('#'));

        let (hline, header) = lines
            .next()
            .ok_or_else(|| err(0, "no header line".into()))?;
        let mut head = header.split(',').map(str::trim);
        let index_name = head.next().unwrap_or_default().to_string();
        let col_labels: Vec<String> = head.map(String::from).collect();
        if col_labels.len() != COLS {
            return Err(err(
                hline,
                format!("expected {COLS} value columns, found {}", col_labels.len()),
            ));
        }

        let mut row_labels = Vec::with_capacity(ROWS);
        let mut values = Vec::with_capacity(ROWS * COLS);
        for (n, line) in lines {
            if row_labels.len() == ROWS {
                return Err(err(n, format!("expected {ROWS} data rows, found more")));
            }
            let mut cells = line.split(',').map(str::trim);
            row_labels.push(cells.next().unwrap_or_default().to_string());
            let before = values.len();
            for cell in cells {
                let v = cell
                    .parse()
                    .map_err(|_| err(n, format!("`{cell}` is not a valid number")))?;
                values.push(v);
            }
            if values.len() - before != COLS {
                return Err(err(
                    n,
                    format!("expected {COLS} values, found {}", values.len() - before),
                ));
            }
        }
        if row_labels.len() != ROWS {
            return Err(err(
                0,
                format!("expected {ROWS} data rows, found {}", row_labels.len()),
            ));
        }
        let data = Array::from_vec(values).map_err(|e| err(0, e.to_string()))?;
        Ok(Self {
            index_name,
            row_labels,
            col_labels,
            data,
        })
    }
}
