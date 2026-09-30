//! Item lists as inventories (a store's pages, a placeable's contents):
//! where a new item goes, and the list with its Remove buttons.

use std::collections::HashMap;

use egui::Ui;
use mg_core::ResRef;
use mg_edit::{Edit, GffPath};
use mg_gff::{Struct, Value};
use mg_module::palette::BlueprintKind;
use mg_resman::ResKey;

use super::Form;

/// An inventory is a grid this many cells wide.
pub(super) const GRID_WIDTH: u32 = 10;

/// What an item's base item says about it: its baseitems.2da `StorePanel`
/// and its size in cells.
pub(super) struct ItemFit {
    pub panel: Option<u32>,
    pub w: u32,
    pub h: u32,
}

impl Form<'_> {
    pub(super) fn item_fit(&mut self, resref: ResRef) -> ItemFit {
        let base = self.blueprint(BlueprintKind::Item, resref).and_then(|u| u.integer("BaseItem"));
        let table = self.app.game.as_ref().and_then(|g| g.table("baseitems").ok());
        let cell = |col: &str| {
            let t = table.as_ref()?;
            t.get_int(usize::try_from(base?).ok()?, col).and_then(|v| u32::try_from(v).ok())
        };
        ItemFit {
            panel: cell("StorePanel"),
            w: cell("InvSlotWidth").unwrap_or(1).clamp(1, GRID_WIDTH),
            h: cell("InvSlotHeight").unwrap_or(1).max(1),
        }
    }

    /// A new entry for `resref` in an inventory holding `items`: at the
    /// first free place, its struct id the list position (as Aurora writes
    /// them).
    pub(super) fn inventory_item(&mut self, items: &[Struct], resref: ResRef) -> Struct {
        let taken: Vec<_> = items
            .iter()
            .map(|it| {
                let fit = self.item_fit(it.resref("InventoryRes").unwrap_or(ResRef::EMPTY));
                let x = it.integer("Repos_PosX").unwrap_or(0) as u32;
                let y = it.integer("Repos_Posy").unwrap_or(0) as u32;
                (x, y, fit.w, fit.h)
            })
            .collect();
        let fit = self.item_fit(resref);
        let (x, y) = place(&taken, fit.w, fit.h);
        let mut item = Struct::new(items.len() as u32);
        item.set("InventoryRes", Value::resref(resref));
        item.set("Repos_PosX", Value::Word(x as u16));
        item.set("Repos_Posy", Value::Word(y as u16));
        item
    }
}

/// The first free place for a `w`×`h` item, row by row, among `taken`
/// rectangles (x, y, w, h).
pub(super) fn place(taken: &[(u32, u32, u32, u32)], w: u32, h: u32) -> (u32, u32) {
    let free = |x: u32, y: u32| {
        taken
            .iter()
            .all(|&(tx, ty, tw, th)| x + w <= tx || tx + tw <= x || y + h <= ty || ty + th <= y)
    };
    (0..)
        .find_map(|y| (0..=GRID_WIDTH - w).find(|&x| free(x, y)).map(|x| (x, y)))
        .expect("an empty row fits any item")
}

/// The items of the `ItemList` at `path`, by name, each with Remove (and
/// with `infinite`, a store's Infinite flag); the edits asked for.
pub(super) fn item_list(
    ui: &mut Ui,
    key: ResKey,
    path: &GffPath,
    items: &[Struct],
    names: &HashMap<ResRef, String>,
    infinite: bool,
) -> Vec<(&'static str, Edit)> {
    let mut edits = Vec::new();
    let columns = if infinite { 3 } else { 2 };
    egui::ScrollArea::vertical().id_salt(("items", key, path.to_string())).max_height(400.0).show(
        ui,
        |ui| {
            egui::Grid::new(("items-grid", key, path.to_string()))
                .num_columns(columns)
                .striped(true)
                .show(ui, |ui| {
                    ui.strong("Item");
                    if infinite {
                        ui.strong("Infinite");
                    }
                    ui.label("");
                    ui.end_row();
                    for (i, it) in items.iter().enumerate() {
                        let resref = it.resref("InventoryRes").unwrap_or(ResRef::EMPTY);
                        let name =
                            names.get(&resref).cloned().unwrap_or_else(|| resref.to_string());
                        ui.label(name).on_hover_text(resref.to_string());
                        if infinite {
                            let mut on = it.integer("Infinite").unwrap_or(0) != 0;
                            if ui.checkbox(&mut on, "").changed() {
                                edits.push((
                                    "Infinite store item",
                                    Edit::SetField {
                                        key,
                                        path: path.item("ItemList", i),
                                        label: "Infinite".into(),
                                        value: Some(Value::Byte(u8::from(on))),
                                    },
                                ));
                            }
                        }
                        if ui.small_button("Remove").clicked() {
                            edits.push((
                                "Remove item",
                                Edit::RemoveItem {
                                    key,
                                    path: path.clone(),
                                    list: "ItemList".into(),
                                    index: i,
                                },
                            ));
                        }
                        ui.end_row();
                    }
                });
        },
    );
    edits
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn items_take_the_first_free_place() {
        // The jewelry page of x2_storethief003: rings (1×1) fill the gaps
        // amulets (1×2) leave in the second row.
        let mut taken = Vec::new();
        for h in [2, 2, 2, 1, 1, 2, 2, 2, 2, 1] {
            let at = place(&taken, 1, h);
            taken.push((at.0, at.1, 1, h));
        }
        let row: Vec<_> = (0..10).map(|x| (x, 0)).collect();
        assert_eq!(taken.iter().map(|t| (t.0, t.1)).collect::<Vec<_>>(), row);
        assert_eq!(place(&taken, 1, 1), (3, 1));
        taken.push((3, 1, 1, 1));
        assert_eq!(place(&taken, 1, 1), (4, 1));
        taken.push((4, 1, 1, 1));
        assert_eq!(place(&taken, 1, 1), (9, 1));
        taken.push((9, 1, 1, 1));
        assert_eq!(place(&taken, 1, 2), (0, 2));
        // Wide items go to the next row that fits them.
        assert_eq!(place(&[(0, 0, 9, 1)], 2, 1), (0, 1));
    }
}
