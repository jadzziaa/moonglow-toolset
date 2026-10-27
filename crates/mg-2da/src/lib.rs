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

pub mod diff;

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
    #[error(
        "the table's short rows would be padded with more than {0} empty cells (a header of \
         many columns over many short lines)"
    )]
    TooManyCells(usize),
}

/// A cap on the empty cells short rows are padded with: a header of 200,000
/// columns over 300,000 one-cell lines, a megabyte of text, would make 60
/// billion cells. Real tables pad a few cells per byte at most; the cap
/// allows [`FACTOR`](Self::FACTOR) per byte of the file plus
/// [`SLACK`](Self::SLACK).
struct PaddingBudget {
    left: usize,
    limit: usize,
}

impl PaddingBudget {
    const FACTOR: usize = 8;
    const SLACK: usize = 1 << 20;

    fn for_input(len: usize) -> Self {
        let limit = len.saturating_mul(Self::FACTOR).saturating_add(Self::SLACK);
        PaddingBudget { left: limit, limit }
    }

    fn take(&mut self, cells: usize) -> Result<(), TwoDaError> {
        self.left = self.left.checked_sub(cells).ok_or(TwoDaError::TooManyCells(self.limit))?;
        Ok(())
    }
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
        let mut padding = PaddingBudget::for_input(data.len());
        for line in lines {
            let mut cells = split_cells(line, width + 1);
            // The first cell is the row label, which the game ignores.
            if !cells.is_empty() {
                cells.remove(0);
            }
            padding.take(width - cells.len())?;
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

    /// The index of a column (case-insensitive). (Asked for on every
    /// cell read: a name of up to 64 bytes is lowered on the stack.)
    pub fn column(&self, name: &str) -> Option<usize> {
        let mut buf = [0u8; 64];
        let Some(lowered) = buf.get_mut(..name.len()) else {
            return self.lookup.get(&name.to_ascii_lowercase()).copied();
        };
        lowered.copy_from_slice(name.as_bytes());
        lowered.make_ascii_lowercase();
        // (ASCII lowered, UTF-8 stays UTF-8.)
        let lowered = std::str::from_utf8(lowered).ok()?;
        self.lookup.get(lowered).copied()
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

    /// Sets the cell at (`row`, `column`): `None`, or `****`, empties it.
    /// An error for a value a 2DA cannot hold (a double quote); nothing
    /// for a cell the table does not have.
    pub fn set(
        &mut self,
        row: usize,
        column: usize,
        value: Option<String>,
    ) -> Result<(), TwoDaError> {
        let value = value.filter(|v| v != "****");
        if value.as_ref().is_some_and(|v| v.contains('"')) {
            return Err(TwoDaError::Unquotable);
        }
        if let Some(cell) = self.rows.get_mut(row).and_then(|r| r.get_mut(column)) {
            *cell = value;
        }
        Ok(())
    }

    /// Adds a row of empty cells at the end; its number.
    pub fn add_row(&mut self) -> usize {
        self.rows.push(vec![None; self.columns.len()]);
        self.rows.len() - 1
    }

    /// Adds a column at the end, its cells empty.
    pub fn add_column(&mut self, name: &str) -> Result<(), TwoDaError> {
        let mut columns = self.columns.clone();
        columns.push(name.to_string());
        self.set_columns(columns)?;
        for row in &mut self.rows {
            row.push(None);
        }
        Ok(())
    }

    /// Takes a column out, with its cells (a row's cell, where the row has
    /// one); `None` for a column the table does not have.
    pub fn remove_column(&mut self, column: usize) -> Option<Vec<Option<String>>> {
        if column >= self.columns.len() {
            return None;
        }
        let mut columns = self.columns.clone();
        columns.remove(column);
        self.set_columns(columns).ok()?;
        let cells = self.rows.iter_mut().map(|r| (column < r.len()).then(|| r.remove(column)));
        Some(cells.map(Option::flatten).collect())
    }

    /// Puts a column in at `column` (the end, past it), with its cells: one
    /// for each row, the rows beyond `cells` empty.
    pub fn insert_column(
        &mut self,
        column: usize,
        name: &str,
        cells: Vec<Option<String>>,
    ) -> Result<(), TwoDaError> {
        let at = column.min(self.columns.len());
        let mut columns = self.columns.clone();
        columns.insert(at, name.to_string());
        self.set_columns(columns)?;
        let mut cells = cells.into_iter();
        for row in &mut self.rows {
            let cell = cells.next().flatten();
            row.insert(at.min(row.len()), cell);
        }
        Ok(())
    }

    /// Moves a column to another place among the columns, with its cells:
    /// the columns between move one place the other way. Nothing for a
    /// place the table does not have.
    pub fn move_column(&mut self, from: usize, to: usize) -> bool {
        let count = self.columns.len();
        if from >= count || to >= count || from == to {
            return false;
        }
        let mut columns = self.columns.clone();
        let name = columns.remove(from);
        columns.insert(to, name);
        if self.set_columns(columns).is_err() {
            return false;
        }
        for row in &mut self.rows {
            // (A row with fewer cells than columns is given the rest.)
            row.resize(count.max(row.len()), None);
            let cell = row.remove(from);
            row.insert(to, cell);
        }
        true
    }

    /// Gives a column another name (an error for one the table has).
    pub fn rename_column(&mut self, column: usize, name: &str) -> Result<(), TwoDaError> {
        let mut columns = self.columns.clone();
        if let Some(c) = columns.get_mut(column) {
            *c = name.to_string();
        }
        self.set_columns(columns)
    }

    /// Empties every cell of a row, which stays (the rows after it keep
    /// their numbers: a row is never taken out of a table).
    pub fn clear_row(&mut self, row: usize) {
        if let Some(r) = self.rows.get_mut(row) {
            r.iter_mut().for_each(|c| *c = None);
        }
    }

    /// A cell as a float, parsed as the game does (see [`parse_float`]).
    pub fn get_float(&self, row: usize, column: &str) -> Option<f32> {
        self.get(row, column).and_then(parse_float)
    }

    /// Serializes the table with aligned columns and CRLF line ends, as the
    /// game's own files are laid out.
    pub fn to_bytes(&self, codepage: Codepage) -> Result<Vec<u8>, TwoDaError> {
        self.to_bytes_as(codepage, Layout::Aligned)
    }

    /// Serializes the table as `layout` says: its columns in line, or one
    /// space between cells (a smaller file, with less for the game to
    /// pass over as it reads).
    pub fn to_bytes_as(&self, codepage: Codepage, layout: Layout) -> Result<Vec<u8>, TwoDaError> {
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
            if layout == Layout::Compact {
                // (The column names' line begins with a space, as the
                // rows begin with their numbers.)
                let mut l = label.to_string();
                for c in cells {
                    l.push(' ');
                    l += c;
                }
                return l + "\r\n";
            }
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

/// How a table's file is laid out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Layout {
    /// Columns in line, padded with spaces: the game's own files.
    #[default]
    Aligned,
    /// One space between cells.
    Compact,
}

impl Layout {
    /// How a file is laid out: compact where no row has two blanks
    /// together between its cells (one space or one tab between each).
    pub fn of(data: &[u8]) -> Layout {
        let padded = data
            .split(|b| *b == b'\n')
            .skip(3)
            .any(|line| line.trim_ascii().windows(2).any(|w| w.iter().all(|b| b" \t".contains(b))));
        let rows = data.split(|b| *b == b'\n').skip(3).any(|l| !l.trim_ascii().is_empty());
        if rows && !padded { Layout::Compact } else { Layout::Aligned }
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

    /// A header of many columns over many one-cell lines would pad to
    /// billions of cells: refused past the cap; a table of the same shape
    /// within it reads, padded.
    #[test]
    fn padding_short_rows_is_capped() {
        let table = |columns: usize, rows: usize| {
            let mut text = String::from("2DA V2.0\n\n");
            text.extend((0..columns).map(|c| format!("c{c} ")));
            text.push('\n');
            text.extend((0..rows).map(|r| format!("{r}\n")));
            text
        };
        let t = TwoDa::parse(table(1000, 10).as_bytes(), Codepage::default()).unwrap();
        assert_eq!((t.columns().len(), t.rows.len()), (1000, 0), "trailing empty rows go");
        let t = TwoDa::parse(format!("{}9 x\n", table(1000, 10)).as_bytes(), Codepage::default());
        assert_eq!(t.unwrap().rows.iter().map(Vec::len).collect::<Vec<_>>(), [1000; 11]);
        let e = TwoDa::parse(table(20_000, 30_000).as_bytes(), Codepage::default()).unwrap_err();
        assert!(matches!(e, TwoDaError::TooManyCells(_)), "{e}");
    }

    /// Every prefix of a table reads as a table or is an error: never a
    /// panic.
    #[test]
    fn truncated_input_is_an_error_or_a_table() {
        for len in 0..SAMPLE.len() {
            let _ = TwoDa::parse(&SAMPLE.as_bytes()[..len], Codepage::default());
            let _ = overlong_rows(&SAMPLE.as_bytes()[..len], Codepage::default());
        }
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

    #[test]
    fn cells_rows_and_columns_are_changed_in_place() {
        let mut t =
            TwoDa::parse(b"2DA V2.0\n\n  Label Cost\n0 a 1\n1 b 2\n", Codepage::default()).unwrap();
        t.set(1, 1, Some("5".into())).unwrap();
        t.set(0, 0, Some("****".into())).unwrap();
        assert_eq!((t.get(1, "Cost"), t.get(0, "Label")), (Some("5"), None));
        assert_eq!(t.set(0, 0, Some("a \"b\"".into())), Err(TwoDaError::Unquotable));
        // (A cell that is not there: nothing.)
        t.set(9, 0, Some("x".into())).unwrap();
        assert_eq!(t.add_row(), 2);
        t.set(2, 0, Some("two words".into())).unwrap();
        t.add_column("Name").unwrap();
        assert_eq!(t.add_column("name"), Err(TwoDaError::DuplicateColumn("name".into())));
        t.set(2, 2, Some("7".into())).unwrap();
        t.clear_row(1);
        // One space between cells reads back as the same table, and a
        // file's own layout is known.
        let compact = t.to_bytes_as(Codepage::default(), Layout::Compact).unwrap();
        let aligned = t.to_bytes(Codepage::default()).unwrap();
        assert!(compact.len() < aligned.len());
        assert!(!String::from_utf8_lossy(&compact).lines().skip(2).any(|l| l.contains("  ")));
        assert_eq!(TwoDa::parse(&compact, Codepage::default()).unwrap(), t);
        assert_eq!(
            (Layout::of(&compact), Layout::of(&aligned)),
            (Layout::Compact, Layout::Aligned)
        );
        assert_eq!(Layout::of(b"2DA V2.0\n\n\tA\tB\n0\tx\ty\n"), Layout::Compact);
        assert_eq!(Layout::of(b"2DA V2.0\n\n  A B\n"), Layout::Aligned, "no rows: the game's way");
        // A column moved takes its cells along, and moved back is as it was.
        let before = t.clone();
        let (first, last) = (t.columns()[0].clone(), t.columns().len() - 1);
        let cell = t.cell(0, 0).map(str::to_string);
        assert!(t.move_column(0, last) && !t.move_column(0, 99) && !t.move_column(1, 1));
        assert_eq!((t.column(&first), t.cell(0, last).map(str::to_string)), (Some(last), cell));
        assert_eq!(t.columns()[0], before.columns()[1]);
        assert!(t.move_column(last, 0));
        assert_eq!(t, before);
        // A column taken out and put back where it was, and renamed.
        let before = t.clone();
        let first = t.columns()[0].clone();
        let cells = t.remove_column(0).unwrap();
        assert_eq!((t.columns().len(), cells.len()), (before.columns().len() - 1, t.len()));
        assert!(t.column(&first).is_none() && t.remove_column(99).is_none());
        t.insert_column(0, &first, cells).unwrap();
        assert_eq!(t, before);
        let second = t.columns()[1].clone();
        assert!(t.rename_column(0, &second).is_err(), "a name the table has");
        t.rename_column(0, "Other").unwrap();
        assert_eq!((t.column("other"), t.column(&first)), (Some(0), None));
        t.rename_column(0, &first).unwrap();
        assert_eq!(t.len(), 3, "a cleared row stays");
        let text = String::from_utf8(t.to_bytes(Codepage::default()).unwrap()).unwrap();
        let rows: Vec<&str> = text.lines().skip(3).map(str::trim).collect();
        assert_eq!(
            rows,
            ["0  ****        1    ****", "1  ****        **** ****", "2  \"two words\" **** 7"]
        );
        assert_eq!(TwoDa::parse(text.as_bytes(), Codepage::default()).unwrap(), t);
    }
}
