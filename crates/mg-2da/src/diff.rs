//! Differences between 2DA tables, by row and column, and merging two
//! tables that were both made from a third.
//!
//! A row is its number (what everything that names a row names it by) and
//! a column its name, whatever the case and wherever it stands. An empty
//! cell (`****`) and a cell that isn't there are the same.

use crate::TwoDa;

/// A cell that differs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CellChange {
    pub row: usize,
    /// The column's name, as the newer table spells it.
    pub column: String,
    pub from: Option<String>,
    pub to: Option<String>,
}

/// What differs from one table to another.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Diff {
    pub columns_added: Vec<String>,
    pub columns_removed: Vec<String>,
    /// The rows the newer table has past the older one's end.
    pub rows_added: std::ops::Range<usize>,
    /// The rows the older table has past the newer one's end.
    pub rows_removed: std::ops::Range<usize>,
    /// Cells that differ, by row then column; those of added and removed
    /// rows and columns among them.
    pub cells: Vec<CellChange>,
    /// The `DEFAULT:` value, if it differs.
    pub default: Option<(Option<String>, Option<String>)>,
}

impl Diff {
    pub fn is_empty(&self) -> bool {
        self == &Diff::default()
    }
}

/// The columns of both, the newer table's first (in its order and
/// spelling), then those only the older has.
fn all_columns(from: &TwoDa, to: &TwoDa) -> Vec<String> {
    let mut out: Vec<String> = to.columns().to_vec();
    out.extend(from.columns().iter().filter(|c| to.column(c).is_none()).cloned());
    out
}

fn cell_at(t: &TwoDa, col: Option<usize>, row: usize) -> Option<&str> {
    col.and_then(|c| t.cell(row, c))
}

/// What differs from `from` to `to`.
pub fn diff(from: &TwoDa, to: &TwoDa) -> Diff {
    let only = |a: &TwoDa, b: &TwoDa| -> Vec<String> {
        a.columns().iter().filter(|c| b.column(c).is_none()).cloned().collect()
    };
    let mut d = Diff {
        columns_added: only(to, from),
        columns_removed: only(from, to),
        rows_added: from.len().min(to.len())..to.len(),
        rows_removed: to.len().min(from.len())..from.len(),
        ..Default::default()
    };
    if d.rows_added.is_empty() {
        d.rows_added = 0..0;
    }
    if d.rows_removed.is_empty() {
        d.rows_removed = 0..0;
    }
    let columns = all_columns(from, to);
    let col_map: Vec<(&str, Option<usize>, Option<usize>)> =
        columns.iter().map(|c| (c.as_str(), from.column(c), to.column(c))).collect();
    for row in 0..from.len().max(to.len()) {
        for &(column, fc, tc) in &col_map {
            let (a, b) = (cell_at(from, fc, row), cell_at(to, tc, row));
            if a != b {
                d.cells.push(CellChange {
                    row,
                    column: column.to_string(),
                    from: a.map(str::to_string),
                    to: b.map(str::to_string),
                });
            }
        }
    }
    if from.default != to.default {
        d.default = Some((from.default.clone(), to.default.clone()));
    }
    d
}

/// A cell both tables changed, each to something else.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Conflict {
    pub row: usize,
    pub column: String,
    pub base: Option<String>,
    pub ours: Option<String>,
    pub theirs: Option<String>,
}

/// Whose value a conflict takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Prefer {
    Ours,
    Theirs,
}

/// Two tables merged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Merged {
    pub table: TwoDa,
    /// The cells both changed differently (the table has the preferred
    /// side's value). Two rows added under one number are these: a row's
    /// number cannot be given away for it.
    pub conflicts: Vec<Conflict>,
}

/// Merges `ours` and `theirs`, both made from `base`: what either changed
/// is taken; where both changed a cell, the same change is one, and
/// different changes are a conflict (`prefer`'s is taken).
///
/// A column either removed is gone; one either added is there (ours
/// before theirs). The table is as long as the longer of the two, unless
/// one of them alone made it shorter.
pub fn merge(base: &TwoDa, ours: &TwoDa, theirs: &TwoDa, prefer: Prefer) -> Merged {
    let kept = |c: &&String| ours.column(c).is_some() && theirs.column(c).is_some();
    let mut columns: Vec<String> = base.columns().iter().filter(kept).cloned().collect();
    for t in [ours, theirs] {
        for c in t.columns() {
            let known = |x: &String| x.eq_ignore_ascii_case(c);
            if base.column(c).is_none() && !columns.iter().any(known) {
                columns.push(c.clone());
            }
        }
    }
    // (Spelled as ours spells them, where it has them.)
    for c in &mut columns {
        if let Some(i) = ours.column(c) {
            *c = ours.columns()[i].clone();
        }
    }
    let len = match (ours.len() == base.len(), theirs.len() == base.len()) {
        (true, _) => theirs.len(),
        (_, true) => ours.len(),
        _ => ours.len().max(theirs.len()),
    };
    type ColMap<'a> = (&'a str, Option<usize>, Option<usize>, Option<usize>);
    let col_map: Vec<ColMap<'_>> = columns
        .iter()
        .map(|c| (c.as_str(), base.column(c), ours.column(c), theirs.column(c)))
        .collect();
    let mut rows = Vec::with_capacity(len);
    let mut conflicts = Vec::new();
    fn pick_borrow<'a>(
        b: Option<&'a str>,
        o: Option<&'a str>,
        t: Option<&'a str>,
        prefer: Prefer,
    ) -> (Option<&'a str>, bool) {
        if o == b {
            (t, false)
        } else if t == b || o == t {
            (o, false)
        } else {
            (if prefer == Prefer::Ours { o } else { t }, true)
        }
    }
    for row in 0..len {
        let mut cells = Vec::with_capacity(col_map.len());
        for &(column, bc, oc, tc) in &col_map {
            let (b, o, t) =
                (cell_at(base, bc, row), cell_at(ours, oc, row), cell_at(theirs, tc, row));
            let (taken, conflict) = pick_borrow(b, o, t, prefer);
            if conflict {
                conflicts.push(Conflict {
                    row,
                    column: column.to_string(),
                    base: b.map(str::to_string),
                    ours: o.map(str::to_string),
                    theirs: t.map(str::to_string),
                });
            }
            cells.push(taken.map(str::to_string));
        }
        rows.push(cells);
    }
    let mut table = TwoDa::new(columns).expect("columns of tables that were read");
    table.rows = rows;
    table.default = if ours.default == base.default {
        theirs.default.clone()
    } else if theirs.default == base.default
        || ours.default == theirs.default
        || prefer == Prefer::Ours
    {
        ours.default.clone()
    } else {
        theirs.default.clone()
    };
    Merged { table, conflicts }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mg_core::Codepage;

    fn table(text: &str) -> TwoDa {
        TwoDa::parse(format!("2DA V2.0\n\n{text}").as_bytes(), Codepage::default()).unwrap()
    }

    const BASE: &str = "  Label Name Cost\n0 a 10 1\n1 b 11 2\n2 c **** 3\n";

    #[test]
    fn a_table_differs_from_itself_in_nothing() {
        assert!(diff(&table(BASE), &table(BASE)).is_empty());
        // Nor by its layout, the case of a column or the place it stands.
        let same = "  LABEL   Cost Name\n0  a 1 10\n1  b 2 11\n2  c 3 ****\n";
        assert!(diff(&table(BASE), &table(same)).is_empty());
    }

    #[test]
    fn differences_are_cells_rows_and_columns() {
        let to = "  Label Name Weight\n0 a 10 5\n1 b 12 ****\n2 c 13 ****\n3 d 14 7\n";
        let d = diff(&table(BASE), &table(to));
        assert_eq!(d.columns_added, ["Weight"]);
        assert_eq!(d.columns_removed, ["Cost"]);
        assert_eq!((d.rows_added.clone(), d.rows_removed.clone()), (3..4, 0..0));
        let at = |row, column: &str| {
            d.cells.iter().find(|c| c.row == row && c.column == column).cloned()
        };
        let name = at(1, "Name").unwrap();
        assert_eq!((name.from.as_deref(), name.to.as_deref()), (Some("11"), Some("12")));
        assert_eq!(at(2, "Name").unwrap().from, None);
        assert_eq!(at(0, "Cost").unwrap().to, None);
        assert_eq!(at(3, "Label").unwrap().to.as_deref(), Some("d"));
        assert!(at(0, "Name").is_none() && at(1, "Weight").is_none());
        let back = diff(&table(to), &table(BASE));
        assert_eq!((back.rows_added, back.rows_removed), (0..0, 3..4));
    }

    #[test]
    fn two_tables_made_from_one_are_merged() {
        let ours = "  Label Name Cost\n0 a 10 1\n1 b 99 2\n2 c **** 3\n3 d 13 4\n";
        let theirs = "  Label Name Cost Weight\n0 a 10 7\n1 b 11 2\n2 c 12 3\n";
        let m = merge(&table(BASE), &table(ours), &table(theirs), Prefer::Ours);
        assert!(m.conflicts.is_empty(), "{:?}", m.conflicts);
        let want = "  Label Name Cost Weight\n0 a 10 7 ****\n1 b 99 2 ****\n2 c 12 3 ****\n3 d 13 4 ****\n";
        assert_eq!(m.table, table(want));
        // The other way about, the same table.
        let n = merge(&table(BASE), &table(theirs), &table(ours), Prefer::Ours);
        assert!(diff(&m.table, &n.table).is_empty());
    }

    #[test]
    fn a_cell_both_changed_is_a_conflict() {
        let ours = "  Label Name Cost\n0 a 10 1\n1 b 20 2\n2 c **** 3\n3 x 1 1\n";
        let theirs = "  Label Name Cost\n0 a 10 1\n1 b 30 2\n2 c **** 3\n3 y 1 1\n";
        let m = merge(&table(BASE), &table(ours), &table(theirs), Prefer::Theirs);
        let at: Vec<(usize, &str)> =
            m.conflicts.iter().map(|c| (c.row, c.column.as_str())).collect();
        // (Two rows added under one number: only where they differ.)
        assert_eq!(at, [(1, "Name"), (3, "Label")]);
        assert_eq!(m.conflicts[0].base.as_deref(), Some("11"));
        assert_eq!(m.table.get(1, "Name"), Some("30"));
        assert_eq!(m.table.get(3, "Label"), Some("y"));
        let ours_kept = merge(&table(BASE), &table(ours), &table(theirs), Prefer::Ours);
        assert_eq!(ours_kept.table.get(1, "Name"), Some("20"));
        // Both to the same: no conflict.
        let m = merge(&table(BASE), &table(ours), &table(ours), Prefer::Ours);
        assert!(m.conflicts.is_empty());
        assert_eq!(m.table, table(ours));
    }

    #[test]
    fn rows_and_columns_one_side_took_away_are_gone() {
        let shorter = "  Label Name\n0 a 10\n1 b 11\n";
        let other = "  Label Name Cost\n0 a 10 1\n1 b 11 5\n2 c **** 3\n";
        let m = merge(&table(BASE), &table(shorter), &table(other), Prefer::Ours);
        assert_eq!(m.table, table(shorter));
        assert!(m.conflicts.is_empty());
    }
}
