//! 2DA V2.0 tables: the game's rules data (appearances, classes, feats,
//! placeables, item properties, ...).
//!
//! Parsing is lenient in the same ways as the game and neverwinter.nim: the
//! `DEFAULT:` line and blank lines are optional, row labels are ignored (rows
//! are numbered by position), short rows are padded with empty cells, extra
//! cells are dropped, `****` and `""` mean empty, double quotes group text with
//! spaces, and empty rows at the end are removed.
//!
//! ```
//! let t = mg_2da::TwoDa::parse(b"2DA V2.0\n\n   LABEL  COST\n0  Sword  10\n1  \"Big Axe\" ****\n", Default::default()).unwrap();
//! assert_eq!(t.get(1, "label"), Some("Big Axe"));
//! assert_eq!(t.get_int(0, "Cost"), Some(10));
//! assert_eq!(t.get(1, "COST"), None);
//! ```

use std::collections::HashMap;

use mg_core::Codepage;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum TwoDaError {
    #[error("not a 2DA V2.0 file")]
    NotTwoDa,
    #[error("the column header line is missing")]
    NoColumns,
    #[error("column {0:?} is not unique")]
    DuplicateColumn(String),
    #[error("a cell value contains a double quote, which 2DA cannot express")]
    Unquotable,
}

/// A 2DA table. Cells are `None` when empty (`****`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TwoDa {
    pub default: Option<String>,
    columns: Vec<String>,
    lookup: HashMap<String, usize>,
    pub rows: Vec<Vec<Option<String>>>,
}

fn split_cells(line: &str, max: usize) -> Vec<Option<String>> {
    let mut cells = Vec::new();
    let mut cur = String::new();
    let mut quoted = false;
    let mut in_cell = false;
    let push = |cur: &mut String, cells: &mut Vec<Option<String>>| {
        let v = std::mem::take(cur);
        cells.push(if v.is_empty() || v == "****" { None } else { Some(v) });
    };
    for c in line.chars() {
        match c {
            '"' => {
                quoted = !quoted;
                in_cell = true;
            }
            ' ' | '\t' if !quoted => {
                if in_cell {
                    push(&mut cur, &mut cells);
                    in_cell = false;
                    if cells.len() >= max {
                        return cells;
                    }
                }
            }
            _ => {
                cur.push(c);
                in_cell = true;
            }
        }
    }
    if in_cell && cells.len() < max {
        push(&mut cur, &mut cells);
    }
    cells
}

/// Rows with more cells than the table has columns (after the row label),
/// as `(row, cells)`: the game and [`TwoDa::parse`] drop the extra cells,
/// and Aurora fails on them. Usually two rows run together.
pub fn overlong_rows(data: &[u8], codepage: Codepage) -> Vec<(usize, usize)> {
    let text = codepage.decode(data);
    let mut lines = text.lines().map(str::trim);
    if lines.next() != Some("2DA V2.0") {
        return Vec::new();
    }
    let mut lines = lines.skip_while(|l| l.is_empty()).peekable();
    if lines.peek().is_some_and(|l| l.starts_with("DEFAULT:")) {
        lines.next();
    }
    let mut lines = lines.skip_while(|l| l.is_empty());
    let Some(header) = lines.next() else { return Vec::new() };
    let width = split_cells(header, usize::MAX).into_iter().flatten().count();
    lines
        .skip_while(|l| l.is_empty())
        .enumerate()
        .filter_map(|(row, line)| {
            let cells = split_cells(line, usize::MAX).len().saturating_sub(1);
            (cells > width).then_some((row, cells))
        })
        .collect()
}

impl TwoDa {
    /// A table with the given columns and no rows.
    pub fn new(columns: Vec<String>) -> Result<TwoDa, TwoDaError> {
        let mut t = TwoDa::default();
        t.set_columns(columns)?;
        Ok(t)
    }

    /// Parses a table; text is decoded with `codepage`.
    pub fn parse(data: &[u8], codepage: Codepage) -> Result<TwoDa, TwoDaError> {
        let text = codepage.decode(data);
        let mut lines = text.lines().map(str::trim);
        if lines.next() != Some("2DA V2.0") {
            return Err(TwoDaError::NotTwoDa);
        }
        let mut lines = lines.skip_while(|l| l.is_empty()).peekable();
        let mut default = None;
        if let Some(l) = lines.peek()
            && let Some(rest) = l.strip_prefix("DEFAULT:")
        {
            default = split_cells(rest, 1).into_iter().next().flatten();
            lines.next();
        }
        let mut lines = lines.skip_while(|l| l.is_empty());
        let header = lines.next().ok_or(TwoDaError::NoColumns)?;
        let columns: Vec<String> = split_cells(header, usize::MAX).into_iter().flatten().collect();
        if columns.is_empty() {
            return Err(TwoDaError::NoColumns);
        }
        let mut t = TwoDa::new(columns)?;
        t.default = default;
        let width = t.columns.len();
        let lines: Vec<&str> = lines.skip_while(|l| l.is_empty()).collect();
        for line in lines {
            let mut cells = split_cells(line, width + 1);
            // The first cell is the row label, which the game ignores.
            if !cells.is_empty() {
                cells.remove(0);
            }
            cells.resize(width, None);
            t.rows.push(cells);
        }
        while t.rows.last().is_some_and(|r| r.iter().all(Option::is_none)) {
            t.rows.pop();
        }
        Ok(t)
    }

    pub fn columns(&self) -> &[String] {
        &self.columns
    }

    /// Replaces the column names (rows are not changed).
    pub fn set_columns(&mut self, columns: Vec<String>) -> Result<(), TwoDaError> {
        let mut lookup = HashMap::with_capacity(columns.len());
        for (i, c) in columns.iter().enumerate() {
            if lookup.insert(c.to_ascii_lowercase(), i).is_some() {
                return Err(TwoDaError::DuplicateColumn(c.clone()));
            }
        }
        self.columns = columns;
        self.lookup = lookup;
        Ok(())
    }

    /// The index of a column (case-insensitive).
    pub fn column(&self, name: &str) -> Option<usize> {
        self.lookup.get(&name.to_ascii_lowercase()).copied()
    }

    pub fn len(&self) -> usize {
        self.rows.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// A cell by column index; `None` for empty cells and out-of-range rows.
    pub fn cell(&self, row: usize, column: usize) -> Option<&str> {
        self.rows.get(row)?.get(column)?.as_deref()
    }

    /// A cell by column name; `None` for empty cells, unknown columns and
    /// out-of-range rows. (The table's `DEFAULT:` is not applied; see
    /// [`TwoDa::get_or_default`].)
    pub fn get(&self, row: usize, column: &str) -> Option<&str> {
        self.cell(row, self.column(column)?)
    }

    /// Like [`TwoDa::get`], falling back to the table's `DEFAULT:` value.
    pub fn get_or_default(&self, row: usize, column: &str) -> Option<&str> {
        self.get(row, column).or(self.default.as_deref())
    }

    /// A cell as an integer, parsed as the game does (see [`parse_int`]).
    pub fn get_int(&self, row: usize, column: &str) -> Option<i32> {
        self.get(row, column).and_then(parse_int)
    }

    /// A cell as a float, parsed as the game does (see [`parse_float`]).
    pub fn get_float(&self, row: usize, column: &str) -> Option<f32> {
        self.get(row, column).and_then(parse_float)
    }

    /// Serializes the table with aligned columns and CRLF line ends, as the
    /// game's own files are laid out.
    pub fn to_bytes(&self, codepage: Codepage) -> Result<Vec<u8>, TwoDaError> {
        let esc = |c: &Option<String>| -> Result<String, TwoDaError> {
            match c {
                None => Ok("****".into()),
                Some(v) if v.contains('"') => Err(TwoDaError::Unquotable),
                Some(v) if v.is_empty() || v.contains([' ', '\t']) => Ok(format!("\"{v}\"")),
                Some(v) => Ok(v.clone()),
            }
        };
        let rows: Vec<Vec<String>> =
            self.rows.iter().map(|r| r.iter().map(esc).collect()).collect::<Result<_, _>>()?;
        let widths: Vec<usize> = (0..self.columns.len())
            .map(|i| {
                rows.iter()
                    .filter_map(|r| r.get(i).map(|c| c.chars().count()))
                    .chain([self.columns[i].chars().count()])
                    .max()
                    .unwrap_or(0)
            })
            .collect();
        let id_width = rows.len().saturating_sub(1).to_string().len().max(1) + 2;
        let mut out = String::from("2DA V2.0\r\n");
        if let Some(d) = &self.default {
            out += &format!("DEFAULT: {}", esc(&Some(d.clone()))?);
        }
        out += "\r\n";
        let line = |label: &str, cells: &[String]| {
            let mut l = format!("{label:id_width$}");
            for (i, c) in cells.iter().enumerate() {
                if i + 1 == cells.len() {
                    l += c;
                } else {
                    l += &format!("{c:w$}", w = widths[i] + 1);
                }
            }
            l.truncate(l.trim_end().len());
            l + "\r\n"
        };
        out += &line("", &self.columns);
        for (i, r) in rows.iter().enumerate() {
            out += &line(&i.to_string(), r);
        }
        codepage.encode(&out).map(|b| b.into_owned()).ok_or(TwoDaError::Unquotable)
    }
}

/// Parses an integer cell like the game: optional sign, then decimal digits
/// or a `0x` hexadecimal number; trailing garbage is ignored. `None` if no
/// digits.
pub fn parse_int(s: &str) -> Option<i32> {
    let s = s.trim();
    let (neg, rest) = match s.as_bytes().first() {
        Some(b'-') => (true, &s[1..]),
        Some(b'+') => (false, &s[1..]),
        _ => (false, s),
    };
    let (radix, digits) = match rest.get(..2) {
        Some("0x" | "0X") => (16, &rest[2..]),
        _ => (10, rest),
    };
    let end = digits.find(|c: char| !c.is_digit(radix)).unwrap_or(digits.len());
    if end == 0 {
        return None;
    }
    let v = i64::from_str_radix(&digits[..end], radix).ok()?;
    let v = if neg { -v } else { v };
    // Hex values up to 0xFFFFFFFF are bit patterns (e.g. slot masks).
    i32::try_from(v).ok().or_else(|| u32::try_from(v).ok().map(|u| u as i32))
}

/// Parses a float cell like C's `atof`: the longest valid prefix. `None` if
/// there is none.
pub fn parse_float(s: &str) -> Option<f32> {
    let s = s.trim();
    let b = s.as_bytes();
    let mut end = 0;
    if matches!(b.first(), Some(b'+' | b'-')) {
        end = 1;
    }
    let digits_start = end;
    while end < b.len() && b[end].is_ascii_digit() {
        end += 1;
    }
    if end < b.len() && b[end] == b'.' {
        end += 1;
        while end < b.len() && b[end].is_ascii_digit() {
            end += 1;
        }
    }
    if end == digits_start || (end == digits_start + 1 && b[digits_start] == b'.') {
        return None;
    }
    // Optional exponent.
    if end < b.len() && matches!(b[end], b'e' | b'E') {
        let mut e = end + 1;
        if matches!(b.get(e), Some(b'+' | b'-')) {
            e += 1;
        }
        if b.get(e).is_some_and(u8::is_ascii_digit) {
            while b.get(e).is_some_and(u8::is_ascii_digit) {
                e += 1;
            }
            end = e;
        }
    }
    s[..end].parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "2DA V2.0\r\nDEFAULT: 7\r\n\r\n      LABEL     HOSTILE  Name\r\n0     Player    0        \"Two words\"\r\n1     Hostile   0x10\r\n  \r\n2 **** -5 x extra\r\n\r\n3\r\n";

    #[test]
    fn lenient_parse() {
        let t = TwoDa::parse(SAMPLE.as_bytes(), Codepage::default()).unwrap();
        assert_eq!(t.default.as_deref(), Some("7"));
        assert_eq!(t.columns(), ["LABEL", "HOSTILE", "Name"]);
        // Blank lines inside the table are rows; trailing empty rows go.
        assert_eq!(t.len(), 4);
        assert_eq!(t.get(0, "name"), Some("Two words"));
        assert_eq!(t.get_int(1, "HOSTILE"), Some(16));
        assert_eq!(t.get(1, "Name"), None);
        assert_eq!(t.rows[2], vec![None, None, None]);
        assert_eq!(t.get(3, "label"), None);
        assert_eq!(t.get_int(3, "Hostile"), Some(-5));
        assert_eq!(t.get(3, "Name"), Some("x"));
        assert_eq!(t.get_or_default(1, "Name"), Some("7"));
        assert_eq!(t.get(99, "Name"), None);
        assert_eq!(t.get(0, "nope"), None);
    }

    #[test]
    fn header_without_blank_line() {
        let t = TwoDa::parse(b"2DA V2.0\n A B\n0 1 2\n", Codepage::default()).unwrap();
        assert_eq!(t.columns(), ["A", "B"]);
        assert_eq!(t.get(0, "b"), Some("2"));
    }

    #[test]
    fn write_round_trip() {
        let t = TwoDa::parse(SAMPLE.as_bytes(), Codepage::default()).unwrap();
        let bytes = t.to_bytes(Codepage::default()).unwrap();
        assert_eq!(TwoDa::parse(&bytes, Codepage::default()).unwrap(), t);
    }

    #[test]
    fn numbers_like_the_game() {
        assert_eq!(parse_int("12abc"), Some(12));
        assert_eq!(parse_int("0x1C030"), Some(0x1C030));
        assert_eq!(parse_int("0xFFFFFFFF"), Some(-1));
        assert_eq!(parse_int("-3"), Some(-3));
        assert_eq!(parse_int("abc"), None);
        assert_eq!(parse_float("1.5f"), Some(1.5));
        assert_eq!(parse_float("-.25"), Some(-0.25));
        assert_eq!(parse_float("2e3"), Some(2000.0));
        assert_eq!(parse_float("."), None);
        assert_eq!(parse_float("x"), None);
    }

    #[test]
    fn rejects_non_2da() {
        assert_eq!(TwoDa::parse(b"hello", Codepage::default()), Err(TwoDaError::NotTwoDa));
        assert_eq!(TwoDa::parse(b"2DA V2.0\n\n", Codepage::default()), Err(TwoDaError::NoColumns));
    }

    #[test]
    fn overlong_rows_are_found() {
        let t = b"2DA V2.0\n\n   LABEL Model\n0  a     m_a\n1  b     m_b 2 c m_c\n2  \"d e\" m_d\n";
        assert_eq!(overlong_rows(t, Codepage::default()), [(1, 5)]);
        assert!(overlong_rows(b"nope", Codepage::default()).is_empty());
    }
}
