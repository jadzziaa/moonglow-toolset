//! A 2DA as it is edited: every change a step to take back, rows added
//! at the end and emptied but never taken out (a row's number is what
//! everything else knows it by), and rows taken from another copy of the
//! table (a lower hak's, which this one hides).

use std::sync::Arc;

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
    /// (Shared with those who read it beside the editor:
    /// [`Editing::shared`].)
    table: Arc<TwoDa>,
    undo: Vec<(String, Change)>,
    redo: Vec<(String, Change)>,
    /// How many steps were done when it was last saved (`None`: a step
    /// that was saved has been taken back and another made in its place).
    saved: Option<usize>,
    /// Bumped at every change, an Undo and a Redo too: what is worked out
    /// from the table (the rows a Find shows) is kept until it moves.
    changes: u64,
    /// How rows and columns were numbered anew, each with the change it
    /// came with ([`Editing::changes`]): what [`Editing::follow`] takes a
    /// cell through. The last [`MOVES_KEPT`] alone.
    moves: std::collections::VecDeque<(u64, Move)>,
    /// Recent cell-only edits (without row/column renumbering): `(changes_after,
    /// sorted_rows, emptiness_changed)` for incremental UI re-filtering/faults.
    cell_edits: std::collections::VecDeque<(u64, Vec<usize>, bool)>,
}

/// How many of the latest renumberings [`Editing::follow`] can go back
/// through.
const MOVES_KEPT: usize = 1024;
/// How many of the latest cell-only edits [`Editing::cell_rows_since`] keeps.
const CELL_EDITS_KEPT: usize = 64;

/// A renumbering of rows or columns, as a change or its Undo made it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Move {
    /// A row taken away: those after it one lower.
    RowOut(usize),
    /// A row put in at its place: those from it on one higher.
    RowIn(usize),
    /// The rows from this number on taken away (rows added, undone).
    RowsTo(usize),
    ColumnOut(usize),
    ColumnIn(usize),
    /// From which place to which.
    ColumnMoved(usize, usize),
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

/// Whose change a cell that differs is, against the lowest copy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Whose {
    /// The other copy changed it; this table has it as the lowest copy.
    Theirs,
    /// This table changed it; the other has it as the lowest copy.
    Ours,
    /// Both changed it, differently.
    Both,
    /// There is no lowest copy to tell by (or the row is not in it).
    Untold,
}

/// A row of another copy that this table lacks or has differently.
#[derive(Debug, Clone, PartialEq)]
pub struct Candidate {
    pub row: usize,
    pub offered: Offered,
    /// The cells that differ: this table's column, its value, the other's.
    pub cells: Vec<(usize, Option<String>, Option<String>)>,
    /// Whose change each of [`cells`](Self::cells) is.
    pub whose: Vec<Whose>,
}

/// The other copy's row as this table's columns have it (by name: a
/// column the other lacks is `None`, as the game reads it).
fn row_of(ours: &TwoDa, other: &TwoDa, row: usize) -> Vec<Option<String>> {
    ours.columns().iter().map(|c| other.get(row, c).map(str::to_string)).collect()
}

fn cell_by<'a>(table: &'a TwoDa, cols: &[Option<usize>], row: usize, c: usize) -> Option<&'a str> {
    cols[c].and_then(|col| table.cell(row, col))
}

/// The rows of `other` that `ours` could take: those it lacks, and those
/// it has differently. With `base` (the lowest copy, usually the game's),
/// a row `other` left as the base has it is not offered (only this table
/// changed it), and one both changed is a conflict.
pub fn candidates(ours: &TwoDa, other: &TwoDa, base: Option<&TwoDa>) -> Vec<Candidate> {
    let other_cols: Vec<Option<usize>> = ours.columns().iter().map(|c| other.column(c)).collect();
    let base_cols: Option<Vec<Option<usize>>> =
        base.map(|b| ours.columns().iter().map(|c| b.column(c)).collect());
    let mut out = Vec::new();
    for row in 0..other.len() {
        let mut any_theirs = false;
        let mut any_diff = false;
        let mut all_none_mine = true;
        for c in 0..ours.columns().len() {
            let m = ours.cell(row, c);
            let t = cell_by(other, &other_cols, row, c);
            if t.is_some() {
                any_theirs = true;
            }
            if m.is_some() {
                all_none_mine = false;
            }
            if m != t {
                any_diff = true;
            }
        }
        if !any_theirs || !any_diff {
            continue;
        }
        let base_row = base.zip(base_cols.as_deref()).filter(|(b, _)| row < b.len());
        let offered = if row >= ours.len() || all_none_mine {
            Offered::New
        } else if let Some((b, bcols)) = base_row {
            let theirs_eq_base = (0..ours.columns().len())
                .all(|c| cell_by(b, bcols, row, c) == cell_by(other, &other_cols, row, c));
            if theirs_eq_base {
                continue;
            }
            let mine_eq_base =
                (0..ours.columns().len()).all(|c| cell_by(b, bcols, row, c) == ours.cell(row, c));
            if mine_eq_base { Offered::Changed } else { Offered::Conflict }
        } else {
            Offered::Changed
        };
        let mut cells = Vec::new();
        let mut whose = Vec::new();
        for c in 0..ours.columns().len() {
            let m = ours.cell(row, c);
            let t = cell_by(other, &other_cols, row, c);
            if m == t {
                continue;
            }
            cells.push((c, m.map(str::to_string), t.map(str::to_string)));
            whose.push(match base_row {
                Some((b, bcols)) => {
                    let bv = cell_by(b, bcols, row, c);
                    if bv == m {
                        Whose::Theirs
                    } else if bv == t {
                        Whose::Ours
                    } else {
                        Whose::Both
                    }
                }
                None => Whose::Untold,
            });
        }
        out.push(Candidate { row, offered, cells, whose });
    }
    out
}

impl Editing {
    pub fn new(table: TwoDa) -> Editing {
        let table = Arc::new(table);
        Editing {
            table,
            undo: Vec::new(),
            redo: Vec::new(),
            saved: Some(0),
            changes: 0,
            moves: Default::default(),
            cell_edits: Default::default(),
        }
    }

    /// Where the cell `(row, column)` of the table as it was at `since`
    /// ([`Editing::changes`]) is now: its row and column followed through
    /// every row and column taken away, put back or moved since (a Delete
    /// Row, its Undo, a column moved). `None` where its row or column is
    /// gone, or where `since` is older than what is kept. What is typed
    /// for a cell is kept by it across such a change (the 2DA editor's
    /// Cell panel), never set in the cell that took its number.
    pub fn follow(&self, (row, column): (usize, usize), since: u64) -> Option<(usize, usize)> {
        if since > self.changes {
            return None;
        }
        // (Older than the oldest kept: where it went is not known.)
        let forgotten = self.moves.len() == MOVES_KEPT
            && self.moves.front().is_some_and(|(at, _)| *at > since + 1);
        if forgotten {
            return None;
        }
        let (mut row, mut column) = (row, column);
        for (_, m) in self.moves.iter().filter(|(at, _)| *at > since) {
            match *m {
                Move::RowOut(r) if row == r => return None,
                Move::RowOut(r) if row > r => row -= 1,
                Move::RowIn(r) if row >= r => row += 1,
                Move::RowsTo(len) if row >= len => return None,
                Move::ColumnOut(c) if column == c => return None,
                Move::ColumnOut(c) if column > c => column -= 1,
                Move::ColumnIn(c) if column >= c => column += 1,
                Move::ColumnMoved(from, to) => {
                    column = if column == from {
                        to
                    } else if from < column && column <= to {
                        column - 1
                    } else if to <= column && column < from {
                        column + 1
                    } else {
                        column
                    };
                }
                _ => {}
            }
        }
        Some((row, column))
    }

    /// Notes how `change`, done (`forward`) or undone, numbered rows or
    /// columns anew, at the change just counted.
    fn note_moves(&mut self, change: &Change, forward: bool) {
        let m = match (change, forward) {
            (Change::Dropped(row, _), true) => Move::RowOut(*row),
            (Change::Dropped(row, _), false) => Move::RowIn(*row),
            (Change::Rows(..), false) => Move::RowsTo(self.table.len()),
            (Change::ColumnOut(column, ..), true) => Move::ColumnOut(*column),
            (Change::ColumnOut(column, ..), false) => Move::ColumnIn(*column),
            (Change::ColumnMoved(from, to), true) => Move::ColumnMoved(*from, *to),
            (Change::ColumnMoved(from, to), false) => Move::ColumnMoved(*to, *from),
            // (The last column, added, taken away again.)
            (Change::Column(_), false) => Move::ColumnOut(self.table.columns().len()),
            _ => return,
        };
        if self.moves.len() == MOVES_KEPT {
            self.moves.pop_front();
        }
        self.moves.push_back((self.changes, m));
    }

    /// Records touched rows and whether any touched row transitioned between
    /// blank and non-blank, for cell-only edits (`Change::Cells`); clears the
    /// log on structural or large bulk changes.
    fn note_cell_edit(&mut self, change: &Change, forward: bool) {
        let Change::Cells(cells) = change else {
            self.cell_edits.clear();
            return;
        };
        if cells.len() > 256 {
            self.cell_edits.clear();
            return;
        }
        let mut rows: Vec<usize> = cells.iter().map(|c| c.0).collect();
        rows.sort_unstable();
        rows.dedup();
        let cols = self.table.columns().len();
        let mut emptiness_changed = false;
        for &r in &rows {
            if r >= self.table.len() {
                continue;
            }
            let is_empty_now = (0..cols).all(|c| self.table.cell(r, c).is_none());
            let was_empty = (0..cols).all(|c| {
                let prior = if forward {
                    cells.iter().find(|x| x.0 == r && x.1 == c).map(|x| x.2.as_deref())
                } else {
                    cells.iter().rfind(|x| x.0 == r && x.1 == c).map(|x| x.3.as_deref())
                };
                match prior {
                    Some(v) => v.is_none(),
                    None => self.table.cell(r, c).is_none(),
                }
            });
            emptiness_changed |= was_empty != is_empty_now;
        }
        if self.cell_edits.len() == CELL_EDITS_KEPT {
            self.cell_edits.pop_front();
        }
        self.cell_edits.push_back((self.changes, rows, emptiness_changed));
    }

    /// If every change strictly after `since` up to [`Self::changes`] was a
    /// small cell-only edit, returns `(sorted_deduped_rows, any_emptiness_changed)`.
    pub fn cell_rows_since(&self, since: u64) -> Option<(Vec<usize>, bool)> {
        if since == self.changes {
            return Some((Vec::new(), false));
        }
        if since > self.changes {
            return None;
        }
        let needed = usize::try_from(self.changes - since).ok()?;
        if needed > self.cell_edits.len() {
            return None;
        }
        let start = self.cell_edits.len() - needed;
        if self.cell_edits[start].0 != since + 1 {
            return None;
        }
        let mut rows = Vec::new();
        let mut emptiness_changed = false;
        for (_, r, empty) in self.cell_edits.range(start..) {
            rows.extend_from_slice(r);
            emptiness_changed |= *empty;
        }
        rows.sort_unstable();
        rows.dedup();
        Some((rows, emptiness_changed))
    }

    pub fn table(&self) -> &TwoDa {
        &self.table
    }

    /// The table as it is now, to hold beside the editor (another table's
    /// editor, whose description names this one's rows): not a copy. A
    /// change made while it is held leaves it as it was (the editor then
    /// goes on in a copy of its own); with nothing holding it, a change
    /// copies nothing.
    pub fn shared(&self) -> Arc<TwoDa> {
        self.table.clone()
    }

    /// The table to change: the editor's own (copied first where
    /// [`shared`](Self::shared) handed it out and it is still held).
    fn table_mut(&mut self) -> &mut TwoDa {
        Arc::make_mut(&mut self.table)
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

    /// How many changes the table has had (every step, Undo and Redo):
    /// the same number, the same table.
    pub fn changes(&self) -> u64 {
        self.changes
    }

    fn done(&mut self, label: impl Into<String>, change: Change) {
        self.changes += 1;
        self.note_moves(&change, true);
        self.note_cell_edit(&change, true);
        if self.saved.is_some_and(|s| s > self.undo.len()) {
            self.saved = None;
        }
        self.undo.push((label.into(), change));
        self.redo.clear();
    }

    /// Takes a column out, with its cells; whether it was there.
    pub fn remove_column(&mut self, column: usize) -> bool {
        let Some(name) = self.table.columns().get(column).cloned() else { return false };
        let Some(cells) = self.table_mut().remove_column(column) else { return false };
        self.done(format!("Remove column {name}"), Change::ColumnOut(column, name, cells));
        true
    }

    /// Moves a column to another place among the columns; whether it moved.
    pub fn move_column(&mut self, from: usize, to: usize) -> bool {
        let Some(name) = self.table.columns().get(from).cloned() else { return false };
        if !self.table_mut().move_column(from, to) {
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
        self.table_mut().rename_column(column, name)?;
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
        // (All of them before any is put: a value no cell holds, a double
        // quote or a line break, changes nothing.)
        for (_, _, value) in &cells {
            value.as_deref().map_or(Ok(()), TwoDa::holds)?;
        }
        let mut changed = Vec::new();
        for (row, column, value) in cells {
            let before = self.table.cell(row, column).map(str::to_string);
            let there = row < self.table.len() && column < self.table.columns().len();
            if there && before != value {
                self.table_mut().set(row, column, value.clone())?;
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
            self.table_mut().add_row();
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
        self.table_mut().add_row();
        let cells = (cells.iter())
            .filter_map(|(name, value)| Some((row, self.table.column(name)?, Some(value.clone()))))
            .filter(|(_, _, v)| v.as_deref().is_none_or(|v| mg_2da::TwoDa::holds(v).is_ok()))
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
        let cells = self.table_mut().rows.remove(row);
        self.done(format!("Delete row {row}"), Change::Dropped(row, cells));
        true
    }

    pub fn add_column(&mut self, name: &str) -> Result<(), TwoDaError> {
        self.table_mut().add_column(name)?;
        self.done(format!("Add column {name}"), Change::Column(name.to_string()));
        Ok(())
    }

    /// Pastes cells copied as text (a line a row, tabs between cells) from
    /// (`row`, `column`) on, adding rows at the end as needed; cells past
    /// the last column are left out. How many cells changed.
    pub fn paste(&mut self, row: usize, column: usize, text: &str) -> Result<usize, TwoDaError> {
        self.paste_into(&[row], column, text)
    }

    /// How many rows a text pasted would go into: its lines that have
    /// text.
    pub fn pasted_lines(text: &str) -> usize {
        text.lines().filter(|l| !l.is_empty()).count()
    }

    /// [`paste`](Self::paste) into the rows given, a line each in their
    /// order (the rows an editor shows under a Find, from the cursor on:
    /// what is copied or cut there is of those rows, and goes back into
    /// them, not into the rows between that the Find hides); lines past
    /// the last of them go into the rows after it. With no row, nothing
    /// is pasted.
    pub fn paste_into(
        &mut self,
        rows: &[usize],
        column: usize,
        text: &str,
    ) -> Result<usize, TwoDaError> {
        let Some(&end) = rows.last() else { return Ok(0) };
        let lines: Vec<&str> = text.lines().filter(|l| !l.is_empty()).collect();
        let row_of =
            |dr: usize| rows.get(dr).copied().unwrap_or_else(|| end + (dr + 1 - rows.len()));
        let mut cells = Vec::new();
        for (dr, line) in lines.iter().enumerate() {
            for (dc, value) in line.split('\t').enumerate() {
                let value = Some(value.trim().to_string()).filter(|v| !v.is_empty());
                cells.push((row_of(dr), column + dc, value));
            }
        }
        if cells.iter().any(|(_, _, v)| v.as_ref().is_some_and(|v| v.contains('"'))) {
            return Err(TwoDaError::Unquotable);
        }
        let past = cells.iter().map(|c| c.0 + 1).max().unwrap_or(0);
        let added = past.saturating_sub(self.table.len());
        for _ in 0..added {
            self.table_mut().add_row();
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
        self.take(other, rows, &[]).0
    }

    /// Takes whole rows of `other` (as [`take_rows`](Self::take_rows))
    /// and single cells of it (a row and this table's column: a row both
    /// copies changed, taken cell by cell), in one step. How many rows and
    /// how many of the cells alone changed.
    pub fn take(
        &mut self,
        other: &TwoDa,
        rows: &[usize],
        cells: &[(usize, usize)],
    ) -> (usize, usize) {
        let rows: Vec<usize> = rows.iter().copied().filter(|r| *r < other.len()).collect();
        // (The rows taken whole, to look one up: each changed cell was
        // looked for in the list of them, twice, and a module's
        // placeables.2da taking CEP's 20,000 rows made four billion
        // comparisons of it.)
        let whole: std::collections::HashSet<usize> = rows.iter().copied().collect();
        let cells: Vec<(usize, usize)> = cells
            .iter()
            .copied()
            .filter(|(r, c)| *r < other.len() && *c < self.table.columns().len())
            .filter(|(r, _)| !whole.contains(r))
            .collect();
        let last = rows.iter().chain(cells.iter().map(|(r, _)| r)).copied().max();
        let Some(last) = last else { return (0, 0) };
        let added = (last + 1).saturating_sub(self.table.len());
        for _ in 0..added {
            self.table_mut().add_row();
        }
        let mut put = Vec::new();
        for &row in &rows {
            for (c, value) in row_of(&self.table, other, row).into_iter().enumerate() {
                put.push((row, c, value));
            }
        }
        for &(row, c) in &cells {
            let value = other.get(row, &self.table.columns()[c]).map(str::to_string);
            put.push((row, c, value));
        }
        // (A cell another table's rules cannot hold is left as it is.)
        put.retain(|(_, _, value)| {
            value.as_deref().is_none_or(|v| mg_2da::TwoDa::holds(v).is_ok())
        });
        let changed = self.put(put).unwrap_or_default();
        let mut taken: Vec<usize> =
            changed.iter().map(|c| c.0).filter(|r| whole.contains(r)).collect();
        taken.dedup();
        let one_by_one = changed.iter().filter(|c| !whole.contains(&c.0)).count();
        let label = match (taken.len(), one_by_one) {
            (n, 0) => format!("Take {n} rows"),
            (0, m) => format!("Take {m} cells"),
            (n, m) => format!("Take {n} rows and {m} cells"),
        };
        if added > 0 {
            self.done(label, Change::Rows(added, changed));
        } else if !changed.is_empty() {
            self.done(label, Change::Cells(changed));
        }
        (taken.len(), one_by_one)
    }

    fn back(&mut self, change: &Change) {
        match change {
            Change::Cells(cells) => {
                for (row, column, before, _) in cells.iter().rev() {
                    let _ = self.table_mut().set(*row, *column, before.clone());
                }
            }
            Change::Rows(count, cells) => {
                for (row, column, before, _) in cells.iter().rev() {
                    let _ = self.table_mut().set(*row, *column, before.clone());
                }
                let left = self.table.len().saturating_sub(*count);
                self.table_mut().rows.truncate(left);
            }
            Change::Dropped(row, cells) => self.table_mut().rows.insert(*row, cells.clone()),
            Change::ColumnOut(column, name, cells) => {
                let _ = self.table_mut().insert_column(*column, name, cells.clone());
            }
            Change::ColumnMoved(from, to) => _ = self.table_mut().move_column(*to, *from),
            Change::ColumnNamed(column, old, _) => _ = self.table_mut().rename_column(*column, old),
            Change::Column(_) => {
                let mut columns = self.table.columns().to_vec();
                columns.pop();
                for row in &mut self.table_mut().rows {
                    row.pop();
                }
                let _ = self.table_mut().set_columns(columns);
            }
        }
    }

    fn again(&mut self, change: &Change) {
        match change {
            Change::Cells(cells) => {
                for (row, column, _, after) in cells {
                    let _ = self.table_mut().set(*row, *column, after.clone());
                }
            }
            Change::Rows(count, cells) => {
                for _ in 0..*count {
                    self.table_mut().add_row();
                }
                for (row, column, _, after) in cells {
                    let _ = self.table_mut().set(*row, *column, after.clone());
                }
            }
            Change::Dropped(row, _) => _ = self.table_mut().rows.remove(*row),
            Change::ColumnOut(column, ..) => _ = self.table_mut().remove_column(*column),
            Change::ColumnMoved(from, to) => _ = self.table_mut().move_column(*from, *to),
            Change::ColumnNamed(column, _, new) => _ = self.table_mut().rename_column(*column, new),
            Change::Column(name) => _ = self.table_mut().add_column(name),
        }
    }

    /// Takes the last step back; whether there was one.
    pub fn undo(&mut self) -> bool {
        let Some((label, change)) = self.undo.pop() else { return false };
        self.changes += 1;
        self.back(&change);
        self.note_moves(&change, false);
        self.note_cell_edit(&change, false);
        self.redo.push((label, change));
        true
    }

    pub fn redo(&mut self) -> bool {
        let Some((label, change)) = self.redo.pop() else { return false };
        self.changes += 1;
        self.again(&change);
        self.note_moves(&change, true);
        self.note_cell_edit(&change, true);
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

    /// The table handed out beside the editor ([`Editing::shared`]) is
    /// the editor's own, not a copy: a change with nothing holding it
    /// copies nothing (it was copied whole once a change for a description
    /// that names its rows: the review of November 2026, D21), and one
    /// made while it is held leaves what is held as it was, an Undo too.
    #[test]
    fn the_table_is_shared_and_copied_only_where_it_is_held() {
        let mut e = Editing::new(table("  Label Cost\n0 a 1\n1 b 2\n"));
        let at = |e: &Editing| Arc::as_ptr(&e.shared());
        let first = at(&e);
        assert_eq!(first, at(&e), "the same table each time");
        e.set(0, 0, Some("changed".into())).unwrap();
        e.add_rows(2);
        e.undo();
        e.redo();
        assert_eq!(at(&e), first, "not held: changed where it is");
        assert_eq!((e.table().cell(0, 0), e.table().len()), (Some("changed"), 4));
        // Held (another editor reads it): the change is made in a copy.
        let held = e.shared();
        e.set(1, 0, Some("other".into())).unwrap();
        assert_ne!(at(&e), Arc::as_ptr(&held));
        assert_eq!((held.cell(1, 0), e.table().cell(1, 0)), (Some("b"), Some("other")));
        e.undo();
        assert_eq!(*held, *e.table(), "as it was held, and as Undo has it again");
        // Let go of: changed where it is again.
        drop(held);
        let now = at(&e);
        e.set(1, 1, Some("9".into())).unwrap();
        assert_eq!(at(&e), now);
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

    /// Taking many rows looks each changed cell's row up, not through the
    /// list of the rows taken: 20,000 rows of ten cells took four billion
    /// comparisons, a freeze of the window (the review of November 2026,
    /// D22). What is taken, and what it says, are as for a few.
    #[test]
    fn many_rows_are_taken_in_time() {
        use std::fmt::Write;
        const ROWS: usize = 20_000;
        let columns = "  A B C D E F G H I J";
        let mut theirs = format!("{columns}\n");
        for r in 0..ROWS {
            writeln!(theirs, "{r} a{r} b c d e f g h i j").unwrap();
        }
        let theirs = table(&theirs);
        let mut e = Editing::new(table(&format!("{columns}\n0 mine b c d e f g h i j\n")));
        let all: Vec<usize> = (0..ROWS).collect();
        // (A cell of a row taken whole is the row's, not counted alone;
        // one past the other copy's end is none.)
        let cells = [(5, 0), (ROWS + 5, 0)];
        let began = std::time::Instant::now();
        assert_eq!(e.take(&theirs, &all, &cells), (ROWS, 0));
        let took = began.elapsed();
        assert!(took < std::time::Duration::from_secs(10), "{took:?}");
        assert_eq!(e.table().len(), ROWS);
        assert_eq!(e.table().get(0, "A"), Some("a0"));
        assert_eq!(e.table().get(ROWS - 1, "A"), Some(format!("a{}", ROWS - 1).as_str()));
        assert_eq!(e.undo_label(), Some(format!("Take {ROWS} rows").as_str()));
        assert!(e.undo());
        assert_eq!((e.table().len(), e.table().get(0, "A")), (1, Some("mine")));
        // Rows and cells alone, in one step.
        assert_eq!(e.take(&theirs, &[3], &[(1, 0), (3, 1)]), (1, 1));
        assert_eq!(e.undo_label(), Some("Take 1 rows and 1 cells"));
        assert_eq!((e.table().get(1, "A"), e.table().get(1, "B")), (Some("a1"), None));
    }

    /// Pasted into the rows an editor shows under a Find, a line goes into
    /// each of those rows, not into the rows between them that the Find
    /// hides (the review of November 2026, D8); lines past the last row
    /// given go on after it.
    #[test]
    fn cells_are_pasted_into_the_rows_given() {
        let mut e = Editing::new(table("  Label\n0 a\n1 b\n2 c\n3 d\n"));
        assert_eq!(Editing::pasted_lines("x\n\ny\n"), 2);
        assert_eq!(e.paste_into(&[1, 3], 0, "x\ny\n").unwrap(), 2);
        let labels = |e: &Editing| -> Vec<String> {
            (0..e.table().len()).map(|r| e.table().get(r, "Label").unwrap_or("").into()).collect()
        };
        assert_eq!(labels(&e), ["a", "x", "c", "y"]);
        assert!(e.undo());
        assert_eq!(labels(&e), ["a", "b", "c", "d"]);
        // More lines than rows: on from the last, a row added at the end.
        assert_eq!(e.paste_into(&[0, 2], 0, "p\nq\nr\ns\n").unwrap(), 4);
        assert_eq!(labels(&e), ["p", "b", "q", "r", "s"]);
        assert_eq!(e.paste_into(&[], 0, "p\n").unwrap(), 0);
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

    /// A row both copies changed is taken cell by cell: each cell says
    /// whose change it is, and the cells chosen alone are taken, with
    /// whole rows, in one step.
    #[test]
    fn a_row_both_changed_is_taken_cell_by_cell() {
        let base = table("  Label Cost Weight\n0 a 1 5\n1 b 2 6\n");
        // Row 1: ours changed Cost, theirs Weight, and both Label.
        let ours = table("  Label Cost Weight\n0 a 1 5\n1 B 20 6\n");
        let theirs = table("  Label Cost Weight\n0 a 1 50\n1 X 2 60\n");
        let found = candidates(&ours, &theirs, Some(&base));
        let row = found.iter().find(|c| c.row == 1).unwrap();
        assert_eq!(row.offered, Offered::Conflict);
        let cells: Vec<usize> = row.cells.iter().map(|c| c.0).collect();
        assert_eq!(cells, [0, 1, 2]);
        assert_eq!(row.whose, [Whose::Both, Whose::Ours, Whose::Theirs]);
        assert_eq!(candidates(&ours, &theirs, None)[1].whose, [Whose::Untold; 3]);
        let mut e = Editing::new(ours.clone());
        // Row 0 whole, and row 1's Weight alone.
        assert_eq!(e.take(&theirs, &[0], &[(1, 2)]), (1, 1));
        assert_eq!(e.undo_label(), Some("Take 1 rows and 1 cells"));
        let t = e.table();
        assert_eq!((t.get(0, "Weight"), t.get(1, "Weight")), (Some("50"), Some("60")));
        assert_eq!((t.get(1, "Label"), t.get(1, "Cost")), (Some("B"), Some("20")), "kept");
        assert!(e.undo());
        assert_eq!(e.table(), &ours, "one step");
        // A cell of a row taken whole is the row's; cells alone, labeled so.
        assert_eq!(e.take(&theirs, &[], &[(1, 0), (9, 0)]), (0, 1));
        assert_eq!(e.undo_label(), Some("Take 1 cells"));
    }

    /// A cell is followed through the rows and columns numbered anew since
    /// a change (what the Cell panel keeps typed text by): a row taken
    /// away above it, its Undo, a column moved, rows added and undone; and
    /// let go of where its own row or column goes.
    #[test]
    fn a_cell_is_followed_through_rows_and_columns_numbered_anew() {
        let mut e = Editing::new(table("  A B C\n0 a b c\n1 d e f\n2 g h i\n3 j k l\n"));
        let since = e.changes();
        assert_eq!(e.follow((2, 1), since), Some((2, 1)), "nothing changed");
        e.set(0, 0, Some("x".into())).unwrap();
        assert_eq!(e.follow((2, 1), since), Some((2, 1)), "a cell set renumbers nothing");
        e.remove_row(0);
        assert_eq!(e.follow((2, 1), since), Some((1, 1)), "a row above taken away");
        assert_eq!(e.follow((0, 1), since), None, "its own row taken away");
        e.undo();
        assert_eq!(e.follow((2, 1), since), Some((2, 1)), "and put back");
        let before = e.changes();
        e.move_column(0, 2);
        assert_eq!(e.follow((2, 1), before), Some((2, 0)), "a column moved past it");
        assert_eq!(e.follow((2, 0), before), Some((2, 2)), "its own column moved");
        e.undo();
        assert_eq!(e.follow((2, 1), before), Some((2, 1)));
        let before = e.changes();
        e.add_rows(2);
        assert_eq!(e.follow((5, 0), before), Some((5, 0)), "rows added at the end");
        let added = e.changes();
        e.undo();
        assert_eq!(e.follow((5, 0), added), None, "an added row undone");
        assert_eq!(e.follow((3, 0), added), Some((3, 0)));
        let before = e.changes();
        e.remove_column(1);
        assert_eq!((e.follow((0, 1), before), e.follow((0, 2), before)), (None, Some((0, 1))));
        assert_eq!(e.follow((0, 0), e.changes() + 1), None, "a change not made yet");
    }

    #[test]
    fn add_row_of_and_take_filter_line_breaks_without_dropping_valid_cells() {
        let mut e = Editing::new(table("  A B C\n0 x y z\n"));
        let row = e.add_row_of(
            "add",
            &[
                ("A".into(), "ok".into()),
                ("B".into(), "bad\nline".into()),
                ("C".into(), "also\rbad".into()),
            ],
        );
        assert_eq!(e.table().get(row, "A"), Some("ok"));
        assert_eq!(e.table().get(row, "B"), None);

        let mut other = TwoDa::new(vec!["A".into(), "B".into(), "C".into()]).unwrap();
        other.rows.push(vec![
            Some("valid".into()),
            Some("cr\rlf\n".into()),
            Some("q\"uote".into()),
        ]);
        let (_, changed) = e.take(&other, &[], &[(0, 0), (0, 1), (0, 2)]);
        assert_eq!(changed, 1);
        assert_eq!(e.table().get(0, "A"), Some("valid"));
        assert_eq!(e.table().get(0, "B"), Some("y"));
        assert_eq!(e.table().get(0, "C"), Some("z"));
    }

    #[test]
    fn cell_rows_since_tracks_small_edits_undo_redo_and_resets_on_structure() {
        let mut e = Editing::new(table("  A B\n0 a b\n1 c ****\n2 **** ****\n"));
        let s0 = e.changes();
        assert_eq!(e.cell_rows_since(s0), Some((vec![], false)));
        assert_eq!(e.cell_rows_since(s0 + 1), None);

        // Single-cell edit not changing emptiness.
        e.set(0, 1, Some("z".into())).unwrap();
        assert_eq!(e.cell_rows_since(s0), Some((vec![0], false)));

        // Editing an all-blank row flips emptiness.
        let s1 = e.changes();
        e.set(2, 0, Some("fill".into())).unwrap();
        assert_eq!(e.cell_rows_since(s1), Some((vec![2], true)));
        assert_eq!(e.cell_rows_since(s0), Some((vec![0, 2], true)));

        // Undoing back to blank reports the touched row and emptiness transition.
        let s2 = e.changes();
        assert!(e.undo());
        assert_eq!(e.cell_rows_since(s2), Some((vec![2], true)));
        let s3 = e.changes();
        assert!(e.redo());
        assert_eq!(e.cell_rows_since(s3), Some((vec![2], true)));

        // Exceeding CELL_EDITS_KEPT invalidates older history while keeping recent window.
        let before_burst = e.changes();
        for i in 0..70 {
            e.set(1, 0, Some(format!("v{i}"))).unwrap();
        }
        assert_eq!(e.cell_rows_since(before_burst), None);
        assert_eq!(e.cell_rows_since(e.changes() - 2), Some((vec![1], false)));

        // Structural edit clears the cell log.
        let before_add = e.changes();
        e.add_rows(1);
        assert_eq!(e.cell_rows_since(before_add), None);
        let after_add = e.changes();
        e.set(3, 0, Some("x".into())).unwrap();
        assert_eq!(e.cell_rows_since(before_add), None);
        assert_eq!(e.cell_rows_since(after_add), Some((vec![3], true)));

        let before_col = e.changes();
        e.add_column("C").unwrap();
        assert_eq!(e.cell_rows_since(before_col), None);
        assert_eq!(e.cell_rows_since(after_add), None);
    }
}
