//! A 2DA as the game reads it, the copy highest in the load order, with
//! where each row and cell comes from: the lowest layer from which its
//! value has come down unchanged. A hak that adds rows to the game's table
//! shows as the game's rows and the hak's; one that changes a cell shows
//! which, and what it was.

use mg_2da::TwoDa;
use mg_core::Codepage;
use mg_resman::{ResKey, ResMan};

/// Every copy of a table in the load order, and each cell's origin.
#[derive(Debug, Clone)]
pub struct Layered {
    /// The layers that have the table, highest first (the first is the one
    /// the game reads), with their copies. Copies below one that doesn't
    /// parse are left out.
    pub copies: Vec<(String, TwoDa)>,
    /// For each row of the table in effect, for each of its columns, the
    /// index in `copies` of the layer the value comes from.
    cells: Vec<Vec<usize>>,
}

/// Whether a cell has the same value in a lower copy (by column name, as
/// the game reads it; a missing cell is an empty one).
fn same(top: &TwoDa, lower: &TwoDa, row: usize, column: usize) -> bool {
    row < lower.len() && top.cell(row, column) == lower.get(row, &top.columns()[column])
}

impl Layered {
    /// The table in effect.
    pub fn table(&self) -> &TwoDa {
        &self.copies[0].1
    }

    /// The layer a cell's value comes from (an index in `copies`).
    pub fn cell_origin(&self, row: usize, column: usize) -> usize {
        self.cells.get(row).and_then(|r| r.get(column)).copied().unwrap_or(0)
    }

    /// The layer that last changed a row: the highest of its cells'.
    pub fn row_origin(&self, row: usize) -> usize {
        self.cells.get(row).and_then(|r| r.iter().min()).copied().unwrap_or(0)
    }

    /// What a cell was in the copy below the one it comes from: that layer
    /// and its value (`None`: empty, or no such row). `None` when no copy
    /// lies below.
    pub fn before(&self, row: usize, column: usize) -> Option<(&str, Option<&str>)> {
        let (label, lower) = self.copies.get(self.cell_origin(row, column) + 1)?;
        let name = &self.table().columns()[column];
        Some((label.as_str(), lower.get(row, name)))
    }

    /// For each layer, the rows it adds (that no lower copy has; all of the
    /// lowest copy's) and the rows it changes, as row origins.
    pub fn summary(&self) -> Vec<(&str, usize, usize)> {
        let mut out: Vec<(&str, usize, usize)> =
            self.copies.iter().map(|(l, _)| (l.as_str(), 0, 0)).collect();
        for row in 0..self.table().len() {
            let o = self.row_origin(row);
            let added = self.copies[o + 1..].iter().all(|(_, t)| row >= t.len());
            if added {
                out[o].1 += 1;
            } else {
                out[o].2 += 1;
            }
        }
        out
    }
}

/// Reads every copy of a 2DA in the load order. `None` when the game has
/// none, or the one it reads doesn't parse.
pub fn layered(rm: &ResMan, key: &ResKey, codepage: Codepage) -> Option<Layered> {
    let mut copies = Vec::new();
    for layer in rm.layers() {
        if !layer.container.contains(key) {
            continue;
        }
        // A copy the game can't read (past 2 GiB) hides the ones below.
        let Ok(data) = layer.container.read(key) else { break };
        let Ok(t) = TwoDa::parse(&data, codepage) else { break };
        copies.push((layer.label.clone(), t));
    }
    let top = &copies.first()?.1;
    let cells = (0..top.len())
        .map(|row| {
            (0..top.columns().len())
                .map(|c| {
                    let mut origin = 0;
                    while copies.get(origin + 1).is_some_and(|(_, t)| same(top, t, row, c)) {
                        origin += 1;
                    }
                    origin
                })
                .collect()
        })
        .collect();
    Some(Layered { copies, cells })
}

/// Whether a 2DA column holds talk-table references, by its name: those
/// the game's tables use (`Name`, `Description`, `StrRef`, `Plural`...).
pub fn is_strref_column(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    n.contains("strref")
        || n.contains("stringref")
        || n.contains("string_ref")
        || n.contains("str_ref")
        || n.contains("desc")
        || matches!(
            n.as_str(),
            "name"
                | "plural"
                | "lower"
                | "nameplural"
                | "convername"
                | "convernamelower"
                | "gamestring"
                | "trapname"
                | "baseitemstatref"
                | "feat"
        )
}

#[cfg(test)]
mod tests {
    use super::*;
    use mg_core::{ResRef, ResType};
    use mg_resman::{LayerClass, MemContainer, priority};

    fn layer(rm: &mut ResMan, priority: u32, label: &str, table: &str) {
        let mut c = MemContainer::new();
        let key = ResKey::new(ResRef::from_str("things").unwrap(), ResType::TWODA);
        c.insert(key, format!("2DA V2.0\n\n{table}").into_bytes());
        rm.add(priority, label, LayerClass::Erf, c);
    }

    #[test]
    fn rows_and_cells_come_from_the_lowest_layer_unchanged() {
        let mut rm = ResMan::new();
        layer(&mut rm, priority::KEY, "game", "  Label Name\n0 Rock 10\n1 Tree 11\n");
        layer(&mut rm, priority::HAK, "low.hak", "  Label Name\n0 Rock 10\n1 Oak 11\n2 Bush 12\n");
        let top =
            "  Label Name Model\n0 Rock 10 ****\n1 Oak 11 tree\n2 Bush 12 ****\n3 Moss 13 m\n";
        layer(&mut rm, priority::HAK_USER, "top.hak", top);
        let key = ResKey::new(ResRef::from_str("things").unwrap(), ResType::TWODA);
        let l = layered(&rm, &key, Codepage::default()).unwrap();
        let labels: Vec<&str> = l.copies.iter().map(|(l, _)| l.as_str()).collect();
        assert_eq!(labels, ["top.hak", "low.hak", "game"]);
        // Row 0 is the game's throughout (a column it lacks reads empty).
        assert_eq!(l.row_origin(0), 2);
        // Row 1: low.hak renamed it, top.hak gave it a model.
        assert_eq!((l.cell_origin(1, 0), l.cell_origin(1, 1), l.cell_origin(1, 2)), (1, 2, 0));
        assert_eq!(l.row_origin(1), 0);
        assert_eq!(l.before(1, 0), Some(("game", Some("Tree"))));
        assert_eq!(l.before(1, 2), Some(("low.hak", None)));
        // Row 2 comes from low.hak; row 3 is top.hak's own.
        assert_eq!((l.row_origin(2), l.row_origin(3)), (1, 0));
        assert_eq!(l.before(3, 0), Some(("low.hak", None)));
        assert_eq!(l.summary(), [("top.hak", 1, 1), ("low.hak", 1, 0), ("game", 1, 0)]);
    }

    #[test]
    fn strref_columns_by_name() {
        for c in ["Name", "Description", "STRING_REF", "StrRef", "FeedbackStrRefFail", "Plural"] {
            assert!(is_strref_column(c), "{c}");
        }
        for c in ["Label", "ModelName", "NameLabel", "Cost"] {
            assert!(!is_strref_column(c), "{c}");
        }
    }
}
