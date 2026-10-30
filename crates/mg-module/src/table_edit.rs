//! A 2DA as it is edited: every change a step to take back, rows added
//! at the end and emptied but never taken out (a row's number is what
//! everything else knows it by), and rows taken from another copy of the
//! table (a lower hak's, which this one hides).

use mg_2da::{TwoDa, TwoDaError};

/// A cell as it was and as it is: its row and column, the two values.
type CellChange = (usize, usize, Option<String>, Option<String>);

/// One step.
#[derive(Debug, Clone, PartialEq)]
enum Change {
    Cells(Vec<CellChange>),
    /// Rows added at the end, with the cells they were given.
    Rows(usize, Vec<CellChange>),
    Column(String),
    /// A column taken out: where it was, its name and its cells.
    ColumnOut(usize, String, Vec<Option<String>>),
    /// A column moved: from which place to which.
    ColumnMoved(usize, usize),
    /// A column renamed: which, from what, to what.
    ColumnNamed(usize, String, String),
    /// A row taken away, with the cells it had: the rows after it are
    /// each one number lower.
    Dropped(usize, Vec<Option<String>>),
}

/// A table being edited.
#[derive(Debug, Clone, PartialEq)]
pub struct Editing {
    table: TwoDa,
    undo: Vec<(String, Change)>,
    redo: Vec<(String, Change)>,
    /// How many steps were done when it was last saved (`None`: a step
    /// that was saved has been taken back and another made in its place).
    saved: Option<usize>,
}

/// Why a row of another copy is offered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Offered {
    /// This table has no such row, or an empty one.
    New,
    /// The other copy changed the row and this one did not (it is as the
    /// lowest copy has it), or there is no lowest copy to tell by.
    Changed,
    /// Both changed it, differently.
    Conflict,
}

/// A row of another copy that this table lacks or has differently.
#[derive(Debug, Clone, PartialEq)]
pub struct Candidate {
    pub row: usize,
    pub offered: Offered,
    /// The cells that differ: this table's column, its value, the other's.
    pub cells: Vec<(usize, Option<String>, Option<String>)>,
}

/// The other copy's row as this table's columns have it (by name: a
/// column the other lacks is `None`, as the game reads it).
fn row_of(ours: &TwoDa, other: &TwoDa, row: usize) -> Vec<Option<String>> {
    ours.columns().iter().map(|c| other.get(row, c).map(str::to_string)).collect()
}

/// The rows of `other` that `ours` could take: those it lacks, and those
/// it has differently. With `base` (the lowest copy, usually the game's),
/// a row `other` left as the base has it is not offered (only this table
/// changed it), and one both changed is a conflict.
pub fn candidates(ours: &TwoDa, other: &TwoDa, base: Option<&TwoDa>) -> Vec<Candidate> {
    let mut out = Vec::new();
    for row in 0..other.len() {
        let theirs = row_of(ours, other, row);
        if theirs.iter().all(Option::is_none) {
            continue;
        }
        let mine: Vec<Option<String>> =
            (0..ours.columns().len()).map(|c| ours.cell(row, c).map(str::to_string)).collect();
        if mine == theirs {
            continue;
        }
        let based = base.filter(|b| row < b.len()).map(|b| row_of(ours, b, row));
        let offered = if row >= ours.len() || mine.iter().all(Option::is_none) {
            Offered::New
        } else {
            match &based {
                Some(b) if *b == theirs => continue,
                Some(b) if *b != mine => Offered::Conflict,
                _ => Offered::Changed,
            }
        };
        let cells = (0..mine.len())
            .filter(|&c| mine[c] != theirs[c])
            .map(|c| (c, mine[c].clone(), theirs[c].clone()))
            .collect();
        out.push(Candidate { row, offered, cells });
    }
    out
}

impl Editing {
    pub fn new(table: TwoDa) -> Editing {
        Editing { table, undo: Vec::new(), redo: Vec::new(), saved: Some(0) }
    }

    pub fn table(&self) -> &TwoDa {
        &self.table
    }

    /// Whether it differs from what was last saved.
    pub fn is_dirty(&self) -> bool {
        self.saved != Some(self.undo.len())
    }

    /// Notes it as never saved where it is (a copy made to be edited).
    pub fn mark_unsaved(&mut self) {
        self.saved = None;
    }

    /// Notes it saved as it is.
    pub fn mark_saved(&mut self) {
        self.saved = Some(self.undo.len());
    }

    pub fn undo_label(&self) -> Option<&str> {
        self.undo.last().map(|(l, _)| l.as_str())
    }

    pub fn redo_label(&self) -> Option<&str> {
        self.redo.last().map(|(l, _)| l.as_str())
    }

    fn done(&mut self, label: impl Into<String>, change: Change) {
        if self.saved.is_some_and(|s| s > self.undo.len()) {
            self.saved = None;
        }
        self.undo.push((label.into(), change));
        self.redo.clear();
    }

    /// Takes a column out, with its cells; whether it was there.
    pub fn remove_column(&mut self, column: usize) -> bool {
        let Some(name) = self.table.columns().get(column).cloned() else { return false };
        let Some(cells) = self.table.remove_column(column) else { return false };
        self.done(format!("Remove column {name}"), Change::ColumnOut(column, name, cells));
        true
    }

    /// Moves a column to another place among the columns; whether it moved.
    pub fn move_column(&mut self, from: usize, to: usize) -> bool {
        let Some(name) = self.table.columns().get(from).cloned() else { return false };
        if !self.table.move_column(from, to) {
            return false;
        }
        self.done(format!("Move column {name}"), Change::ColumnMoved(from, to));
        true
    }

    /// Renames a column; whether its name changed.
    pub fn rename_column(&mut self, column: usize, name: &str) -> Result<bool, TwoDaError> {
        let Some(old) = self.table.columns().get(column).cloned() else { return Ok(false) };
        if old == name {
            return Ok(false);
        }
        self.table.rename_column(column, name)?;
        let label = format!("Rename column {old} to {name}");
        self.done(label, Change::ColumnNamed(column, old, name.to_string()));
        Ok(true)
    }

    /// Applies cell changes (each as it is to be), keeping those that
    /// change something.
    fn put(
        &mut self,
        cells: Vec<(usize, usize, Option<String>)>,
    ) -> Result<Vec<CellChange>, TwoDaError> {
        let cells: Vec<(usize, usize, Option<String>)> =
            cells.into_iter().map(|(r, c, v)| (r, c, v.filter(|v| v != "****"))).collect();
        if cells.iter().any(|(_, _, v)| v.as_ref().is_some_and(|v| v.contains('"'))) {
            return Err(TwoDaError::Unquotable);
        }
        let mut changed = Vec::new();
        for (row, column, value) in cells {
            let before = self.table.cell(row, column).map(str::to_string);
            let there = row < self.table.len() && column < self.table.columns().len();
            if there && before != value {
                self.table.set(row, column, value.clone())?;
                changed.push((row, column, before, value));
            }
        }
        Ok(changed)
    }

    /// Sets a cell (`None` or `****`: empty). Whether it changed.
    pub fn set(
        &mut self,
        row: usize,
        column: usize,
        value: Option<String>,
    ) -> Result<bool, TwoDaError> {
        let changed = self.put(vec![(row, column, value)])?;
        let any = !changed.is_empty();
        if any {
            let name = &self.table.columns()[column];
            self.done(format!("Set row {row}, {name}"), Change::Cells(changed));
        }
        Ok(any)
    }

    /// Adds `count` empty rows at the end; the first one's number.
    pub fn add_rows(&mut self, count: usize) -> usize {
        let first = self.table.len();
        for _ in 0..count {
            self.table.add_row();
        }
        if count > 0 {
            let label =
                if count == 1 { format!("Add row {first}") } else { format!("Add {count} rows") };
            self.done(label, Change::Rows(count, Vec::new()));
        }
        first
    }

    /// Adds a row at the end with `cells` (by column name; a column the
    /// table lacks is left out, as the game would read nothing there), as
    /// one step named `label`; the row's number.
    pub fn add_row_of(&mut self, label: &str, cells: &[(String, String)]) -> usize {
        let row = self.table.len();
        self.table.add_row();
        let cells = (cells.iter())
            .filter_map(|(name, value)| Some((row, self.table.column(name)?, Some(value.clone()))))
            .filter(|(_, _, v)| !v.as_ref().is_some_and(|v| v.contains('"')))
            .collect();
        let changed = self.put(cells).unwrap_or_default();
        self.done(label, Change::Rows(1, changed));
        row
    }

    /// Empties a row's cells; the row stays.
    pub fn clear_row(&mut self, row: usize) {
        let cells = (0..self.table.columns().len()).map(|c| (row, c, None)).collect();
        if let Ok(changed) = self.put(cells)
            && !changed.is_empty()
        {
            self.done(format!("Empty row {row}"), Change::Cells(changed));
        }
    }

    /// Empties the cells of a rectangle (`rows`, and the columns from
    /// `first` to `last`), as one step; how many had something.
    pub fn clear_cells(&mut self, rows: &[usize], first: usize, last: usize) -> usize {
        let cells = rows.iter().flat_map(|&r| (first..=last).map(move |c| (r, c, None))).collect();
        match self.put(cells) {
            Ok(changed) if !changed.is_empty() => {
                let count = changed.len();
                let what = if count == 1 { "1 cell".to_string() } else { format!("{count} cells") };
                self.done(format!("Empty {what}"), Change::Cells(changed));
                count
            }
            _ => 0,
        }
    }

    /// Takes a row away; whether it was there. The rows after it are
    /// each one number lower: what names them by number (other tables,
    /// blueprints, scripts) is not followed.
    pub fn remove_row(&mut self, row: usize) -> bool {
        if row >= self.table.len() {
            return false;
        }
        let cells = self.table.rows.remove(row);
        self.done(format!("Delete row {row}"), Change::Dropped(row, cells));
        true
    }

    pub fn add_column(&mut self, name: &str) -> Result<(), TwoDaError> {
        self.table.add_column(name)?;
        self.done(format!("Add column {name}"), Change::Column(name.to_string()));
        Ok(())
    }

    /// Pastes cells copied as text (a line a row, tabs between cells) from
    /// (`row`, `column`) on, adding rows at the end as needed; cells past
    /// the last column are left out. How many cells changed.
    pub fn paste(&mut self, row: usize, column: usize, text: &str) -> Result<usize, TwoDaError> {
        let lines: Vec<&str> = text.lines().filter(|l| !l.is_empty()).collect();
        let mut cells = Vec::new();
        for (dr, line) in lines.iter().enumerate() {
            for (dc, value) in line.split('\t').enumerate() {
                let value = Some(value.trim().to_string()).filter(|v| !v.is_empty());
                cells.push((row + dr, column + dc, value));
            }
        }
        if cells.iter().any(|(_, _, v)| v.as_ref().is_some_and(|v| v.contains('"'))) {
            return Err(TwoDaError::Unquotable);
        }
        let added = (row + lines.len()).saturating_sub(self.table.len());
        for _ in 0..added {
            self.table.add_row();
        }
        let changed = self.put(cells)?;
        let n = changed.len();
        if added > 0 {
            self.done("Paste", Change::Rows(added, changed));
        } else if n > 0 {
            self.done("Paste", Change::Cells(changed));
        }
        Ok(n)
    }

    /// Takes rows of another copy of the table, each whole (by column
    /// name), as one step; rows past this table's end are added, with
    /// empty ones between where the other copy's numbers skip. How many
    /// rows changed.
    pub fn take_rows(&mut self, other: &TwoDa, rows: &[usize]) -> usize {
        let last = rows.iter().copied().filter(|r| *r < other.len()).max();
        let Some(last) = last else { return 0 };
        let added = (last + 1).saturating_sub(self.table.len());
        for _ in 0..added {
            self.table.add_row();
        }
        let mut cells = Vec::new();
        for &row in rows.iter().filter(|r| **r < other.len()) {
            for (c, value) in row_of(&self.table, other, row).into_iter().enumerate() {
                // (A cell another table's rules cannot hold is left as it is.)
                if !value.as_ref().is_some_and(|v| v.contains('"')) {
                    cells.push((row, c, value));
                }
            }
        }
        let changed = self.put(cells).unwrap_or_default();
        let mut taken: Vec<usize> = changed.iter().map(|c| c.0).collect();
        taken.dedup();
        let label = format!("Take {} rows", taken.len());
        if added > 0 {
            self.done(label, Change::Rows(added, changed));
        } else if !changed.is_empty() {
            self.done(label, Change::Cells(changed));
        }
        taken.len()
    }

    fn back(&mut self, change: &Change) {
        match change {
            Change::Cells(cells) => {
                for (row, column, before, _) in cells.iter().rev() {
                    let _ = self.table.set(*row, *column, before.clone());
                }
            }
            Change::Rows(count, cells) => {
                for (row, column, before, _) in cells.iter().rev() {
                    let _ = self.table.set(*row, *column, before.clone());
                }
                let left = self.table.len().saturating_sub(*count);
                self.table.rows.truncate(left);
            }
            Change::Dropped(row, cells) => self.table.rows.insert(*row, cells.clone()),
            Change::ColumnOut(column, name, cells) => {
                let _ = self.table.insert_column(*column, name, cells.clone());
            }
            Change::ColumnMoved(from, to) => _ = self.table.move_column(*to, *from),
            Change::ColumnNamed(column, old, _) => _ = self.table.rename_column(*column, old),
            Change::Column(_) => {
                let mut columns = self.table.columns().to_vec();
                columns.pop();
                for row in &mut self.table.rows {
                    row.pop();
                }
                let _ = self.table.set_columns(columns);
            }
        }
    }

    fn again(&mut self, change: &Change) {
        match change {
            Change::Cells(cells) => {
                for (row, column, _, after) in cells {
                    let _ = self.table.set(*row, *column, after.clone());
                }
            }
            Change::Rows(count, cells) => {
                for _ in 0..*count {
                    self.table.add_row();
                }
                for (row, column, _, after) in cells {
                    let _ = self.table.set(*row, *column, after.clone());
                }
            }
            Change::Dropped(row, _) => _ = self.table.rows.remove(*row),
            Change::ColumnOut(column, ..) => _ = self.table.remove_column(*column),
            Change::ColumnMoved(from, to) => _ = self.table.move_column(*from, *to),
            Change::ColumnNamed(column, _, new) => _ = self.table.rename_column(*column, new),
            Change::Column(name) => _ = self.table.add_column(name),
        }
    }

    /// Takes the last step back; whether there was one.
    pub fn undo(&mut self) -> bool {
        let Some((label, change)) = self.undo.pop() else { return false };
        self.back(&change);
        self.redo.push((label, change));
        true
    }

    pub fn redo(&mut self) -> bool {
        let Some((label, change)) = self.redo.pop() else { return false };
        self.again(&change);
        self.undo.push((label, change));
        true
    }
}

#[cfg(test)]
mod tests {
    use mg_core::Codepage;

    use super::*;

    fn table(text: &str) -> TwoDa {
        TwoDa::parse(format!("2DA V2.0\n\n{text}").as_bytes(), Codepage::default()).unwrap()
    }

    /// A row added with its cells (by column name) is one step: a column
    /// the table lacks, and `****`, give nothing; Undo takes the row away
    /// and Redo puts it back with them.
    #[test]
    fn a_row_is_added_with_its_cells_as_one_step() {
        let mut e = Editing::new(table("  Label Cost\n0 a 1\n"));
        let cells = |pairs: &[(&str, &str)]| -> Vec<(String, String)> {
            pairs.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect()
        };
        let row =
            e.add_row_of("Add dusk", &cells(&[("Cost", "5"), ("Label", "Dusk"), ("Nope", "x")]));
        assert_eq!(row, 1);
        assert_eq!(
            (e.table().get(1, "Label"), e.table().get(1, "Cost")),
            (Some("Dusk"), Some("5"))
        );
        assert_eq!(e.undo_label(), Some("Add dusk"));
        e.undo();
        assert_eq!(e.table().len(), 1);
        e.redo();
        assert_eq!(e.table().get(1, "Label"), Some("Dusk"));
        e.add_row_of("Empty", &cells(&[("Label", "****")]));
        assert_eq!((e.table().len(), e.table().get(2, "Label")), (3, None));
    }

    #[test]
    fn every_change_is_a_step_to_take_back() {
        let start = table("  Label Cost\n0 a 1\n1 b 2\n");
        let mut e = Editing::new(start.clone());
        assert!(!e.is_dirty());
        assert!(e.set(0, 1, Some("9".into())).unwrap());
        assert!(!e.set(0, 1, Some("9".into())).unwrap(), "as it was: no step");
        assert_eq!(e.undo_label(), Some("Set row 0, Cost"));
        assert_eq!(e.add_rows(2), 2);
        e.set(3, 0, Some("d".into())).unwrap();
        e.clear_row(1);
        e.add_column("Name").unwrap();
        assert_eq!(e.set(0, 2, Some("\"".into())), Err(TwoDaError::Unquotable));
        assert_eq!(e.table().len(), 4);
        assert_eq!(e.table().get(1, "Label"), None);
        // A rectangle of cells is emptied in one step.
        assert_eq!(e.clear_cells(&[0, 3], 0, 1), 3);
        assert_eq!((e.table().get(0, "Label"), e.table().get(3, "Label")), (None, None));
        assert_eq!(e.clear_cells(&[0, 3], 0, 1), 0, "empty already: no step");
        e.undo();
        assert_eq!((e.table().get(0, "Cost"), e.table().get(3, "Label")), (Some("9"), Some("d")));
        // A column is moved, with its cells, and back.
        assert!(e.move_column(0, 2) && !e.move_column(0, 0) && !e.move_column(0, 9));
        assert_eq!(e.table().columns(), ["Cost", "Name", "Label"]);
        assert_eq!((e.undo_label(), e.table().cell(3, 2)), (Some("Move column Label"), Some("d")));
        e.undo();
        assert_eq!(
            (e.table().columns()[0].as_str(), e.table().get(3, "Label")),
            ("Label", Some("d"))
        );
        e.redo();
        e.undo();
        // A column is renamed, and taken out with its cells.
        assert_eq!(e.rename_column(0, "Name"), Err(TwoDaError::DuplicateColumn("Name".into())));
        assert_eq!(e.rename_column(0, "Tag"), Ok(true));
        assert_eq!(e.undo_label(), Some("Rename column Label to Tag"));
        assert!(e.remove_column(1) && !e.remove_column(9));
        assert_eq!(e.table().columns(), ["Tag", "Name"]);
        e.undo();
        assert_eq!((e.table().columns().len(), e.table().get(0, "Cost")), (3, Some("9")));
        e.undo();
        assert_eq!(e.table().get(3, "Label"), Some("d"));
        // A row is taken away with its cells: those after it move up.
        assert!(e.remove_row(2) && !e.remove_row(9));
        assert_eq!(e.undo_label(), Some("Delete row 2"));
        assert_eq!((e.table().len(), e.table().get(2, "Label")), (3, Some("d")));
        e.undo();
        assert_eq!((e.table().len(), e.table().get(3, "Label")), (4, Some("d")));
        assert_eq!(e.table().get(2, "Label"), None);
        e.redo();
        assert!(e.is_dirty());
        e.mark_saved();
        assert!(!e.is_dirty());
        // Back to the start, and forward again.
        let end = e.table().clone();
        while e.undo() {}
        assert_eq!(e.table(), &start);
        assert!(e.is_dirty(), "not as it was saved");
        while e.redo() {}
        assert_eq!(e.table(), &end);
        assert!(!e.is_dirty());
        // A saved step taken back and another made in its place: never as
        // saved again, however far back.
        e.undo();
        e.set(0, 0, Some("z".into())).unwrap();
        e.undo();
        assert!(e.is_dirty());
    }

    #[test]
    fn cells_are_pasted_from_text() {
        let mut e = Editing::new(table("  Label Cost\n0 a 1\n1 b 2\n"));
        // Two rows of two cells from row 1 on: a row is added for the
        // second; a cell past the last column is left out.
        assert_eq!(e.paste(1, 0, "x\t7\textra\ny\t\n").unwrap(), 3);
        assert_eq!(e.table().len(), 3);
        assert_eq!((e.table().get(1, "Label"), e.table().get(1, "Cost")), (Some("x"), Some("7")));
        assert_eq!((e.table().get(2, "Label"), e.table().get(2, "Cost")), (Some("y"), None));
        assert!(e.undo());
        assert_eq!(e.table().len(), 2);
        assert_eq!(e.table().get(1, "Label"), Some("b"));
        assert_eq!(e.paste(0, 0, "a \"b\""), Err(TwoDaError::Unquotable));
    }

    #[test]
    fn rows_are_offered_and_taken_from_another_copy() {
        let base = table("  Label Cost\n0 a 1\n1 b 2\n2 c 3\n");
        // Ours changed row 0 and row 2; theirs changed row 1 and row 2,
        // and added rows 4 and 5 (3 is padding), with a column of its own.
        let ours = table("  Label Cost\n0 A 1\n1 b 2\n2 c 30\n");
        let theirs = table(
            "  Label Cost Extra\n0 a 1 x\n1 b 20 x\n2 C 3 x\n3 **** **** ****\n4 e 5 x\n5 f 6 x\n",
        );
        let found = candidates(&ours, &theirs, Some(&base));
        let kinds: Vec<(usize, Offered)> = found.iter().map(|c| (c.row, c.offered)).collect();
        assert_eq!(
            kinds,
            [(1, Offered::Changed), (2, Offered::Conflict), (4, Offered::New), (5, Offered::New)]
        );
        assert_eq!(
            found[1].cells,
            [(0, Some("c".into()), Some("C".into())), (1, Some("30".into()), Some("3".into()))]
        );
        // Without a base every difference is offered as a change.
        assert_eq!(candidates(&ours, &theirs, None).len(), 5);

        let mut e = Editing::new(ours.clone());
        assert_eq!(e.take_rows(&theirs, &[1, 5]), 2);
        assert_eq!(e.undo_label(), Some("Take 2 rows"));
        let t = e.table();
        assert_eq!(t.len(), 6, "rows are added up to the last one taken");
        assert_eq!(t.get(1, "Cost"), Some("20"));
        assert_eq!((t.get(5, "Label"), t.get(4, "Label")), (Some("f"), None));
        assert_eq!(t.columns().len(), 2, "the other's own column is not taken");
        assert!(e.undo());
        assert_eq!(e.table(), &ours);
        assert_eq!(e.take_rows(&theirs, &[99]), 0);
    }
}
