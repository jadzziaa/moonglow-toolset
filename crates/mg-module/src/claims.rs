//! Who is building which area: the team's claims table, named by the
//! project file (`[claims] file`), and what Moonglow makes of it.
//!
//! The table is a 2DA in the checkout, as The Domains of Dread's hand-back
//! proposal has it (`builders/claims.2da`): a row an area, by its ResRef,
//! with the builder's code that holds it, whether it is merged (in the
//! game: its Tag and ResRef are kept from then on), when it was last
//! claimed or released, and the Tag it was merged with.
//!
//! ```text
//! 2DA V2.0
//!
//!     Area             Builder  Status   Date        Tag
//! 0   kz_square        ab       claimed  2026-10-22  KZ_SQUARE
//! 1   kz_abbey_gate    ****     merged   2026-10-01  KZ_ABBEY_GATE
//! 2   kz_bridge        cd       merged   2026-10-20  KZ_BRIDGE
//! ```
//!
//! `Area` and `Builder` are needed; `Status`, `Date` and `Tag` are read
//! where the table has them, and any other column (a team's `World`) is
//! kept as it is. Claim and Release change one row's line, or add one at
//! the end, and leave every other line as written: two builders claiming
//! two areas at once make two changes git merges by itself.

use std::path::{Path, PathBuf};

use mg_core::ResRef;

use crate::project_file::Patterns;

/// The columns a table Moonglow makes has.
pub const COLUMNS: [&str; 5] = ["Area", "Builder", "Status", "Date", "Tag"];

/// What an area another builder holds is to the builder who opens it
/// (`[claims] others`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Others {
    /// It can be edited: the area says whose it is, and saving a change to
    /// it asks first.
    #[default]
    Warn,
    /// No change to it is taken.
    ReadOnly,
}

/// What an area no one holds is (`[claims] unclaimed`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Unclaimed {
    /// Anyone's to edit, as without a table.
    #[default]
    Edit,
    /// It says so, and saving a change to it asks first.
    Warn,
}

/// The project's `[claims]`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ClaimRules {
    /// The table, from the project's root, as the file writes it (none:
    /// no claims are kept).
    pub file: Option<String>,
    pub others: Others,
    pub unclaimed: Unclaimed,
    /// Areas whose Tag and ResRef are kept, besides those the table says
    /// are merged.
    pub protected: Patterns,
}

impl ClaimRules {
    /// The table's path, for a project's root.
    pub fn path_in(&self, root: &Path) -> Option<PathBuf> {
        self.file.as_deref().map(|f| root.join(f))
    }
}

/// One area's row.
#[derive(Debug, Clone, PartialEq)]
pub struct Claim {
    pub area: ResRef,
    /// The builder's code that holds it (lower case), if one does.
    pub builder: Option<String>,
    /// As written (`claimed`, `free`, `merged`), lower case.
    pub status: Option<String>,
    pub date: Option<String>,
    /// The Tag it was merged with.
    pub tag: Option<String>,
    /// Its row in the table.
    pub row: usize,
}

impl Claim {
    /// Whether the area is in the game (`merged`): its Tag and ResRef keep
    /// players' positions and first visits.
    pub fn merged(&self) -> bool {
        self.status.as_deref() == Some("merged")
    }

    /// Who holds it and since when, in a few words: `cd since 2026-10-20`.
    pub fn holder(&self) -> Option<String> {
        let code = self.builder.as_deref()?;
        Some(match &self.date {
            Some(d) => format!("{code} since {d}"),
            None => code.to_string(),
        })
    }
}

/// The table as read.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Claims {
    pub rows: Vec<Claim>,
}

impl Claims {
    /// The row of an area (the first, should it have two).
    pub fn of(&self, area: ResRef) -> Option<&Claim> {
        self.rows.iter().find(|c| c.area == area)
    }

    /// Reads a table's text: what it doesn't understand is a warning
    /// (`claims.2da:5: …`); a text that is no 2DA, or lacks `Area` or
    /// `Builder`, an error.
    pub fn parse(name: &str, text: &str, warnings: &mut Vec<String>) -> Result<Claims, String> {
        let table = Table::read(text).map_err(|e| format!("{name}: {e}"))?;
        let mut rows = Vec::new();
        for (row, cells) in table.rows.iter().enumerate() {
            let line = table.data_line(row).map_or(0, |l| l + 1);
            let at = |c: Option<usize>| c.and_then(|c| cells.get(c).cloned().flatten());
            let Some(area) = at(Some(table.area)) else {
                // (A row of nothing: one left blank, or released by hand.)
                if cells.iter().any(Option::is_some) {
                    warnings.push(format!("{name}:{line}: a row without an Area"));
                }
                continue;
            };
            let area = match ResRef::from_str(&area) {
                Ok(r) => r,
                Err(e) => {
                    warnings.push(format!("{name}:{line}: {e}"));
                    continue;
                }
            };
            let builder = at(Some(table.builder)).map(|b| b.to_ascii_lowercase());
            let status = at(table.status).map(|s| s.to_ascii_lowercase());
            if let Some(s) = &status
                && !["claimed", "free", "merged"].contains(&s.as_str())
            {
                warnings.push(format!(
                    "{name}:{line}: Status \"{s}\" is none of claimed, free and merged (taken as \
                     not merged)"
                ));
            }
            if rows.iter().any(|c: &Claim| c.area == area) {
                warnings.push(format!("{name}:{line}: {area} has a row above: that one is read"));
            }
            rows.push(Claim {
                area,
                builder,
                status,
                date: at(table.date),
                tag: at(table.tag),
                row,
            });
        }
        Ok(Claims { rows })
    }
}

/// Whose an area is, to the builder with `code`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Standing<'a> {
    /// The builder's own.
    Mine(&'a Claim),
    /// Another builder's.
    Others(&'a Claim),
    /// No one's (its row, where it has one).
    Free(Option<&'a Claim>),
}

impl<'a> Standing<'a> {
    pub fn of(claims: &'a Claims, area: ResRef, code: Option<&str>) -> Standing<'a> {
        match claims.of(area) {
            Some(c) => match c.builder.as_deref() {
                Some(b) if Some(b) == code => Standing::Mine(c),
                Some(_) => Standing::Others(c),
                None => Standing::Free(Some(c)),
            },
            None => Standing::Free(None),
        }
    }

    pub fn claim(self) -> Option<&'a Claim> {
        match self {
            Standing::Mine(c) | Standing::Others(c) => Some(c),
            Standing::Free(c) => c,
        }
    }
}

/// What a builder may do with an area.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Access {
    /// Edit it, as without a table.
    Edit,
    /// Edit it, warned first (before a save): why.
    Warn(String),
    /// Look at it only: why.
    ReadOnly(String),
}

/// What a change to an area's claim is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Change<'a> {
    /// The builder `code` takes it.
    Claim(&'a str),
    /// The builder `code` lets it go.
    Release(&'a str),
}

/// A table's text with an area claimed or released (`date` the day,
/// `YYYY-MM-DD`), every other line kept as written; a table made where
/// there is none (`text` empty). `force` takes another builder's claim
/// (or releases it): without it, that is refused, saying whose it is.
pub fn change(
    text: &str,
    area: ResRef,
    change: Change,
    date: &str,
    force: bool,
) -> Result<String, String> {
    let made;
    let text = if text.trim().is_empty() {
        made = new_table();
        made.as_str()
    } else {
        text
    };
    let table = Table::read(text)?;
    let mut warnings = Vec::new();
    let claims = Claims::parse("", text, &mut warnings)?;
    let row = claims.of(area);
    let holder = row.and_then(|c| c.builder.as_deref());
    let merged = row.is_some_and(Claim::merged);
    let (builder, status) = match change {
        Change::Claim(code) => {
            if let Some(other) = holder.filter(|h| *h != code && !force) {
                return Err(format!(
                    "{area} is claimed by {other}{}: they release it first",
                    row.and_then(|c| c.date.as_deref())
                        .map_or(String::new(), |d| format!(" (since {d})"))
                ));
            }
            (Some(code), if merged { "merged" } else { "claimed" })
        }
        Change::Release(code) => {
            match holder {
                None => return Err(format!("{area} is not claimed")),
                Some(h) if h != code && !force => {
                    return Err(format!("{area} is claimed by {h}, not by {code}"));
                }
                _ => {}
            }
            (None, if merged { "merged" } else { "free" })
        }
    };
    let mut set: Vec<(usize, Option<String>)> = vec![(table.builder, builder.map(String::from))];
    set.extend(table.status.map(|c| (c, Some(status.to_string()))));
    set.extend(table.date.map(|c| (c, Some(date.to_string()))));
    match row {
        Some(c) => table.with_row(c.row, &set),
        None => {
            set.push((table.area, Some(area.to_string().to_ascii_lowercase())));
            table.with_new_row(&set)
        }
    }
}

/// Claims or releases an area in the table at `path` (made, with its
/// folder, where there is none), today: [`change`] on the file.
pub fn write_change(path: &Path, area: ResRef, how: Change, force: bool) -> Result<(), String> {
    rewrite(path, |text| change(text, area, how, &mg_core::time::today(), force))
}

/// A claims table's text: UTF-8 where it is that, else Windows-1252 (a
/// table saved by a Windows editor: "José" read as UTF-8 was written back
/// as "Jos\u{fffd}" for good).
pub(crate) fn decode(bytes: &[u8]) -> (std::borrow::Cow<'_, str>, bool) {
    match std::str::from_utf8(bytes) {
        Ok(text) => (text.into(), true),
        Err(_) => (mg_core::Codepage::WINDOWS_1252.decode(bytes), false),
    }
}

/// The table at `path` (none: empty) made over by `f`, in its own
/// encoding, written whole through a temporary file: the team's table is
/// never left cut short.
fn rewrite(path: &Path, f: impl FnOnce(&str) -> Result<String, String>) -> Result<(), String> {
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Vec::new(),
        Err(e) => return Err(format!("{}: {e}", path.display())),
    };
    let (text, utf8) = decode(&bytes);
    let out = f(&text)?;
    let out =
        if utf8 { out.into_bytes() } else { mg_core::Codepage::WINDOWS_1252.encode_lossy(&out) };
    if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    let tmp = crate::sibling(path, ".moonglow-tmp");
    std::fs::write(&tmp, out)
        .and_then(|()| std::fs::rename(&tmp, path))
        .map_err(|e| format!("{}: {e}", path.display()))
}

/// A table's text with an area marked merged (in the game from now on),
/// with the Tag it was merged with, its holder kept (a row added where it
/// has none), every other line kept as written.
pub fn merged(text: &str, area: ResRef, tag: &str, date: &str) -> Result<String, String> {
    let made;
    let text = if text.trim().is_empty() {
        made = new_table();
        made.as_str()
    } else {
        text
    };
    let table = Table::read(text)?;
    let mut warnings = Vec::new();
    let claims = Claims::parse("", text, &mut warnings)?;
    let Some(status) = table.status else { return Err("the table has no Status column".into()) };
    let mut set = vec![(status, Some("merged".to_string()))];
    set.extend(table.tag.map(|c| (c, Some(tag.to_string()))));
    set.extend(table.date.map(|c| (c, Some(date.to_string()))));
    match claims.of(area) {
        Some(c) => table.with_row(c.row, &set),
        None => {
            set.push((table.area, Some(area.to_string().to_ascii_lowercase())));
            table.with_new_row(&set)
        }
    }
}

/// [`merged`] on the table at `path`, today.
pub fn write_merged(path: &Path, area: ResRef, tag: &str) -> Result<(), String> {
    rewrite(path, |text| merged(text, area, tag, &mg_core::time::today()))
}

/// The text of a table with no rows yet.
fn new_table() -> String {
    // (Room for a 16-character ResRef, a code of 8, a status, a date.)
    let mut header = String::new();
    for (name, at) in COLUMNS.iter().zip([4, 21, 30, 39, 51]) {
        while header.len() < at {
            header.push(' ');
        }
        header.push_str(name);
    }
    format!("2DA V2.0\n\n{header}\n")
}

/// A 2DA's text, read as the game reads it, with where each row's line is.
struct Table<'t> {
    text: &'t str,
    /// Each line's start and end (without its line ending).
    lines: Vec<(usize, usize)>,
    /// The header's line, and where each column's name begins in it.
    header: usize,
    starts: Vec<usize>,
    /// The first row's line.
    first: usize,
    rows: Vec<Vec<Option<String>>>,
    area: usize,
    builder: usize,
    status: Option<usize>,
    date: Option<usize>,
    tag: Option<usize>,
    newline: &'static str,
}

/// A 2DA line's cells, as the game splits it, with where each begins.
fn cells_of(line: &str) -> Vec<(usize, Option<String>)> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let (mut quoted, mut in_cell, mut began) = (false, false, 0);
    for (i, c) in line.char_indices() {
        match c {
            '"' => {
                if !in_cell {
                    began = i;
                }
                quoted = !quoted;
                in_cell = true;
            }
            ' ' | '\t' if !quoted => {
                if in_cell {
                    let v = std::mem::take(&mut cur);
                    out.push((began, (!v.is_empty() && v != "****").then_some(v)));
                    in_cell = false;
                }
            }
            _ => {
                if !in_cell {
                    began = i;
                }
                cur.push(c);
                in_cell = true;
            }
        }
    }
    if in_cell {
        out.push((began, (!cur.is_empty() && cur != "****").then_some(cur)));
    }
    out
}

impl<'t> Table<'t> {
    fn read(text: &'t str) -> Result<Table<'t>, String> {
        let mut lines = Vec::new();
        let mut at = 0;
        for piece in text.split_inclusive('\n') {
            let body = piece.trim_end_matches(['\n', '\r']);
            lines.push((at, at + body.len()));
            at += piece.len();
        }
        let newline = if text.contains("\r\n") { "\r\n" } else { "\n" };
        let line = |i: usize| -> &str { lines.get(i).map_or("", |&(a, b)| &text[a..b]) };
        if line(0).trim() != "2DA V2.0" {
            return Err("not a 2DA (its first line is not 2DA V2.0)".into());
        }
        let mut i = 1;
        while i < lines.len() && line(i).trim().is_empty() {
            i += 1;
        }
        if line(i).trim().starts_with("DEFAULT:") {
            i += 1;
            while i < lines.len() && line(i).trim().is_empty() {
                i += 1;
            }
        }
        if i >= lines.len() {
            return Err("no column names".into());
        }
        let header = i;
        let names = cells_of(line(header));
        let starts: Vec<usize> = names.iter().map(|(s, _)| *s).collect();
        let names: Vec<String> = names.into_iter().map(|(_, n)| n.unwrap_or_default()).collect();
        let column = |n: &str| names.iter().position(|c| c.eq_ignore_ascii_case(n));
        let (Some(area), Some(builder)) = (column("Area"), column("Builder")) else {
            return Err("its columns are to have Area and Builder".into());
        };
        let mut first = header + 1;
        while first < lines.len() && line(first).trim().is_empty() {
            first += 1;
        }
        let mut rows: Vec<Vec<Option<String>>> = (first..lines.len())
            .map(|l| {
                let mut cells: Vec<Option<String>> =
                    cells_of(line(l)).into_iter().skip(1).map(|(_, v)| v).collect();
                cells.resize(names.len(), None);
                cells
            })
            .collect();
        while rows.last().is_some_and(|r| r.iter().all(Option::is_none)) {
            rows.pop();
        }
        Ok(Table {
            text,
            header,
            starts,
            first,
            rows,
            area,
            builder,
            status: column("Status"),
            date: column("Date"),
            tag: column("Tag"),
            newline,
            lines,
        })
    }

    /// The line (from 0) a row is on.
    fn data_line(&self, row: usize) -> Option<usize> {
        let l = self.first + row;
        (l < self.lines.len()).then_some(l)
    }

    /// A row's line, its label and cells laid out under the column names.
    fn line_of(&self, label: &str, cells: &[Option<String>]) -> Result<String, String> {
        let mut out = String::from(label);
        for (i, c) in cells.iter().enumerate() {
            let cell = match c {
                None => "****".to_string(),
                Some(v) if v.contains('"') => {
                    return Err(format!("{v}: a 2DA cell can't hold a double quote"));
                }
                Some(v) if v.is_empty() || v.contains([' ', '\t']) => format!("\"{v}\""),
                Some(v) => v.clone(),
            };
            // (Under its name, where the line has come no further.)
            let start = self.starts.get(i).copied().unwrap_or(0);
            let header_line = &self.text[self.lines[self.header].0..self.lines[self.header].1];
            let column = header_line[..start.min(header_line.len())].chars().count();
            let mut width = out.chars().count();
            if width < column {
                out.extend(std::iter::repeat_n(' ', column - width));
                width = column;
            }
            if width > 0 && !out.ends_with([' ', '\t']) {
                out.push(' ');
            }
            out.push_str(&cell);
        }
        Ok(out)
    }

    /// The text with row `row`'s cells `set`, its line laid out again.
    fn with_row(&self, row: usize, set: &[(usize, Option<String>)]) -> Result<String, String> {
        let mut cells = self.rows.get(row).cloned().ok_or("no such row")?;
        for (c, v) in set {
            if let Some(cell) = cells.get_mut(*c) {
                *cell = v.clone();
            }
        }
        let l = self.data_line(row).ok_or("no such row")?;
        let (a, b) = self.lines[l];
        let label = cells_of(&self.text[a..b])
            .first()
            .and_then(|(_, v)| v.clone())
            .unwrap_or_else(|| row.to_string());
        let line = self.line_of(&label, &cells)?;
        Ok(format!("{}{line}{}", &self.text[..a], &self.text[b..]))
    }

    /// The text with a row added after the last (its cells `set`, the rest
    /// empty).
    fn with_new_row(&self, set: &[(usize, Option<String>)]) -> Result<String, String> {
        let mut cells = vec![None; self.starts.len()];
        for (c, v) in set {
            if let Some(cell) = cells.get_mut(*c) {
                *cell = v.clone();
            }
        }
        let n = self.rows.len();
        let line = self.line_of(&n.to_string(), &cells)?;
        // After the last row (or the column names), before blank lines.
        let after = match n {
            0 => self.header,
            n => self.data_line(n - 1).unwrap_or(self.header),
        };
        let end = self.lines[after].1;
        let rest = &self.text[end..];
        let ending = if rest.starts_with("\r\n") {
            2
        } else if rest.starts_with('\n') {
            1
        } else {
            0
        };
        let (head, tail) = self.text.split_at(end + ending);
        let lead = if ending == 0 { self.newline } else { "" };
        Ok(format!("{head}{lead}{line}{}{tail}", self.newline))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TABLE: &str = "2DA V2.0\n\n    Area             Builder  Status   Date        Tag            World\n\
                         0   kz_square        ab       claimed  2026-10-22  KZ_SQUARE      1\n\
                         1   kz_abbey_gate    ****     merged   2026-10-01  KZ_ABBEY_GATE  1\n\
                         2   kz_bridge        cd       merged   2026-10-20  KZ_BRIDGE      ****\n";

    fn r(s: &str) -> ResRef {
        ResRef::from_str(s).unwrap()
    }

    fn read(text: &str) -> (Claims, Vec<String>) {
        let mut w = Vec::new();
        (Claims::parse("claims.2da", text, &mut w).unwrap(), w)
    }

    #[test]
    fn the_table_reads_as_the_game_reads_it() {
        let (c, w) = read(TABLE);
        assert_eq!(w, Vec::<String>::new());
        assert_eq!(c.rows.len(), 3);
        let square = c.of(r("KZ_SQUARE")).unwrap();
        assert_eq!(square.builder.as_deref(), Some("ab"));
        assert!(!square.merged());
        assert_eq!(square.holder().as_deref(), Some("ab since 2026-10-22"));
        let gate = c.of(r("kz_abbey_gate")).unwrap();
        assert!(gate.merged() && gate.builder.is_none());
        assert_eq!(gate.tag.as_deref(), Some("KZ_ABBEY_GATE"));
        // The game's own reading agrees, cell for cell.
        let game = mg_2da::TwoDa::parse(TABLE.as_bytes(), mg_core::Codepage::WINDOWS_1252).unwrap();
        for claim in &c.rows {
            assert_eq!(
                game.get(claim.row, "Area").map(str::to_ascii_lowercase),
                Some(claim.area.to_string().to_ascii_lowercase())
            );
            assert_eq!(game.get(claim.row, "Builder").map(String::from), claim.builder);
            assert_eq!(game.get(claim.row, "Tag").map(String::from), claim.tag);
        }
    }

    /// A table saved by a Windows editor (Windows-1252, a builder named
    /// José) keeps its bytes where a claim is written into it, and no
    /// temporary file is left beside it.
    #[test]
    fn a_windows_table_keeps_its_letters() {
        let dir = mg_testkit::scratch_dir("mg-claims-1252");
        let path = dir.join("claims.2da");
        let text = TABLE.replace("cd       merged", "José     merged");
        let bytes = mg_core::Codepage::WINDOWS_1252.encode_lossy(&text);
        assert!(std::str::from_utf8(&bytes).is_err());
        std::fs::write(&path, &bytes).unwrap();
        write_change(&path, r("kz_new"), Change::Claim("ab"), false).unwrap();
        let after = std::fs::read(&path).unwrap();
        assert!(after.windows(4).any(|w| w == b"Jos\xe9"), "{}", String::from_utf8_lossy(&after));
        assert!(String::from_utf8_lossy(&after).contains("kz_new"));
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 1);
    }

    #[test]
    fn whose_an_area_is() {
        let (c, _) = read(TABLE);
        assert!(matches!(Standing::of(&c, r("kz_square"), Some("ab")), Standing::Mine(_)));
        assert!(matches!(Standing::of(&c, r("kz_square"), Some("cd")), Standing::Others(_)));
        assert!(matches!(Standing::of(&c, r("kz_square"), None), Standing::Others(_)));
        assert!(matches!(
            Standing::of(&c, r("kz_abbey_gate"), Some("ab")),
            Standing::Free(Some(_))
        ));
        assert!(matches!(Standing::of(&c, r("elsewhere"), Some("ab")), Standing::Free(None)));
    }

    #[test]
    fn a_claim_changes_its_row_s_line_only() {
        let out =
            change(TABLE, r("kz_abbey_gate"), Change::Claim("ef"), "2026-10-23", false).unwrap();
        let before: Vec<&str> = TABLE.lines().collect();
        let after: Vec<&str> = out.lines().collect();
        assert_eq!(before.len(), after.len());
        let differ: Vec<usize> = (0..before.len()).filter(|&i| before[i] != after[i]).collect();
        assert_eq!(differ, [4]);
        assert_eq!(
            after[4], "1   kz_abbey_gate    ef       merged   2026-10-23  KZ_ABBEY_GATE  1",
            "laid out under the column names, merged kept, World kept"
        );
        let (c, w) = read(&out);
        assert!(w.is_empty(), "{w:?}");
        assert_eq!(c.of(r("kz_abbey_gate")).unwrap().builder.as_deref(), Some("ef"));
        // Released: ****, merged kept; a new area's is free.
        let out =
            change(&out, r("kz_abbey_gate"), Change::Release("ef"), "2026-10-24", false).unwrap();
        assert_eq!(
            out.lines().nth(4).unwrap(),
            "1   kz_abbey_gate    ****     merged   2026-10-24  KZ_ABBEY_GATE  1"
        );
        let out =
            change(TABLE, r("kz_square"), Change::Release("ab"), "2026-10-24", false).unwrap();
        assert_eq!(
            out.lines().nth(3).unwrap(),
            "0   kz_square        ****     free     2026-10-24  KZ_SQUARE      1"
        );
    }

    #[test]
    fn a_new_area_is_a_row_at_the_end() {
        let out = change(TABLE, r("KZ_Mill"), Change::Claim("ab"), "2026-10-23", false).unwrap();
        assert_eq!(&out[..TABLE.len()], TABLE, "every line above kept");
        assert_eq!(
            &out[TABLE.len()..],
            "3   kz_mill          ab       claimed  2026-10-23  ****           ****\n"
        );
        // No table yet: one is made.
        let made = change("", r("kz_mill"), Change::Claim("ab"), "2026-10-23", false).unwrap();
        let (c, w) = read(&made);
        assert!(w.is_empty());
        assert_eq!(c.rows[0].area, r("kz_mill"));
        assert_eq!(c.rows[0].status.as_deref(), Some("claimed"));
        let game = mg_2da::TwoDa::parse(made.as_bytes(), mg_core::Codepage::WINDOWS_1252).unwrap();
        assert_eq!(game.columns(), COLUMNS);
        assert_eq!(game.get(0, "Builder"), Some("ab"));
        // A file without a last line ending, with CRLF, and blank lines
        // after the rows: the row goes after the last, in its endings.
        let crlf = "2DA V2.0\r\n\r\n Area Builder\r\n0 a ab\r\n\r\n";
        let out = change(crlf, r("b"), Change::Claim("cd"), "x", false).unwrap();
        assert_eq!(out, "2DA V2.0\r\n\r\n Area Builder\r\n0 a ab\r\n1 b   cd\r\n\r\n");
        let bare = "2DA V2.0\n\n Area Builder\n0 a ab";
        let out = change(bare, r("b"), Change::Claim("cd"), "x", false).unwrap();
        assert_eq!(out, "2DA V2.0\n\n Area Builder\n0 a ab\n1 b   cd\n");
    }

    #[test]
    fn another_s_claim_is_refused_unless_forced() {
        let e = change(TABLE, r("kz_bridge"), Change::Claim("ab"), "x", false).unwrap_err();
        assert_eq!(e, "kz_bridge is claimed by cd (since 2026-10-20): they release it first");
        let e = change(TABLE, r("kz_bridge"), Change::Release("ab"), "x", false).unwrap_err();
        assert_eq!(e, "kz_bridge is claimed by cd, not by ab");
        let e = change(TABLE, r("kz_abbey_gate"), Change::Release("ab"), "x", false).unwrap_err();
        assert_eq!(e, "kz_abbey_gate is not claimed");
        let out = change(TABLE, r("kz_bridge"), Change::Claim("ab"), "x", true).unwrap();
        assert_eq!(read(&out).0.of(r("kz_bridge")).unwrap().builder.as_deref(), Some("ab"));
        // Claimed again by the one who holds it: the date moves.
        let out = change(TABLE, r("kz_square"), Change::Claim("ab"), "2027-01-01", false).unwrap();
        assert_eq!(read(&out).0.of(r("kz_square")).unwrap().date.as_deref(), Some("2027-01-01"));
    }

    #[test]
    fn what_the_table_does_not_understand_is_said() {
        let (_, w) =
            read("2DA V2.0\n\n Area Builder Status\n0 a ab merged\n1 a cd done\n2 **** x\n");
        assert_eq!(
            w,
            [
                "claims.2da:5: Status \"done\" is none of claimed, free and merged (taken as not \
                 merged)",
                "claims.2da:5: a has a row above: that one is read",
                "claims.2da:6: a row without an Area",
            ]
        );
        let mut w = Vec::new();
        let e = Claims::parse("claims.2da", "2DA V2.0\n\n Area Who\n", &mut w).unwrap_err();
        assert_eq!(e, "claims.2da: its columns are to have Area and Builder");
        assert!(Claims::parse("claims.2da", "area builder\n", &mut w).is_err());
        // A quoted cell, and one too long for a ResRef.
        let (c, w) =
            read("2DA V2.0\n\n Area Builder Date\n0 a ab \"22 Oct\"\n1 much_too_long_a_name x\n");
        assert_eq!(c.rows[0].date.as_deref(), Some("22 Oct"));
        assert_eq!(w.len(), 1);
        assert!(w[0].starts_with("claims.2da:5: "), "{w:?}");
    }

    #[test]
    fn merged_keeps_the_holder_and_records_the_tag() {
        let out = merged(TABLE, r("kz_square"), "KZ_SQ", "2026-10-30").unwrap();
        assert_eq!(
            out.lines().nth(3).unwrap(),
            "0   kz_square        ab       merged   2026-10-30  KZ_SQ          1"
        );
        let out = merged(TABLE, r("kz_mill"), "KZ_MILL", "2026-10-30").unwrap();
        assert_eq!(
            out.lines().last().unwrap(),
            "3   kz_mill          ****     merged   2026-10-30  KZ_MILL        ****"
        );
        let e = merged("2DA V2.0\n\n Area Builder\n", r("a"), "A", "x").unwrap_err();
        assert_eq!(e, "the table has no Status column");
    }

    #[test]
    fn every_cut_of_a_table_reads_without_a_panic() {
        for end in (0..=TABLE.len()).filter(|&i| TABLE.is_char_boundary(i)) {
            let text = &TABLE[..end];
            let mut w = Vec::new();
            let _ = Claims::parse("claims.2da", text, &mut w);
            let _ = change(text, r("kz_mill"), Change::Claim("ab"), "x", false);
            let _ = change(text, r("kz_square"), Change::Release("ab"), "x", true);
            let _ = merged(text, r("kz_square"), "T", "x");
        }
        let mut w = Vec::new();
        let _ = Claims::parse("claims.2da", "2DA V2.0\n\"\n\"\"\"", &mut w);
    }
}
