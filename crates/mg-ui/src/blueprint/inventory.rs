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
use crate::images::{ICON_MAX, Picture};

/// An inventory is a grid this many cells wide.
pub(super) const GRID_WIDTH: u32 = 10;

/// What an item's base item says about it: its baseitems.2da `StorePanel`
/// and its size in cells.
pub(super) struct ItemFit {
    pub panel: Option<u32>,
    pub w: u32,
    pub h: u32,
}

/// The item blueprint an inventory entry names: a blueprint's
/// `InventoryRes`, or a placed object's whole item's `TemplateResRef`.
pub(super) fn entry_resref(entry: &Struct) -> ResRef {
    entry
        .resref("InventoryRes")
        .or_else(|| entry.resref("EquippedRes"))
        .or_else(|| entry.resref("TemplateResRef"))
        .unwrap_or(ResRef::EMPTY)
}

impl Form<'_> {
    /// An entry's inventory icon: a whole item's own, else its blueprint's.
    pub(super) fn entry_icon(&mut self, ctx: &egui::Context, entry: &Struct) -> Vec<Picture> {
        if entry.integer("BaseItem").is_some() {
            return self.app.item_icon(ctx, entry);
        }
        match self.blueprint(BlueprintKind::Item, entry_resref(entry)) {
            Some(item) => self.app.item_icon(ctx, &item),
            None => Vec::new(),
        }
    }

    /// An entry's name: a whole item's own, else its blueprint's.
    pub(super) fn entry_name(&self, entry: &Struct, names: &HashMap<ResRef, String>) -> String {
        let own = entry
            .locstring("LocalizedName")
            .and_then(|ls| self.app.game.as_ref().and_then(|g| g.locstring(ls)))
            .filter(|n| !n.is_empty());
        let r = entry_resref(entry);
        own.or_else(|| names.get(&r).cloned()).unwrap_or_else(|| r.to_string())
    }

    /// The item palette picker (see [`Form::palette_picker`]) with the chosen
    /// item beside it ([`Form::chosen_item`]), or below it in a narrow column.
    pub(super) fn item_picker(&mut self, ui: &mut Ui, add: &str) -> Option<ResRef> {
        const TREE: f32 = 240.0;
        if ui.available_width() < 2.0 * TREE {
            let taken = self.palette_picker(ui, BlueprintKind::Item, add);
            self.chosen_item(ui);
            return taken;
        }
        ui.horizontal_top(|ui| {
            let taken = ui
                .vertical(|ui| {
                    ui.set_width(TREE);
                    self.palette_picker(ui, BlueprintKind::Item, add)
                })
                .inner;
            ui.vertical(|ui| self.chosen_item(ui));
            taken
        })
        .inner
    }

    /// The item chosen in the palette picker, for a look before adding it:
    /// its icon, name, base item and statistics, and its properties.
    pub(super) fn chosen_item(&mut self, ui: &mut Ui) {
        let Some(resref) = self.palette_chosen(ui, BlueprintKind::Item) else { return };
        let Some(item) = self.blueprint(BlueprintKind::Item, resref) else { return };
        let icon = self.app.item_icon(ui.ctx(), &item);
        let Some(game) = self.app.game.as_ref() else { return };
        let name = item
            .locstring("LocalizedName")
            .and_then(|ls| game.locstring(ls))
            .filter(|n| !n.is_empty())
            .unwrap_or_else(|| resref.to_string());
        let base = item.integer("BaseItem").unwrap_or(0).max(0) as usize;
        let base_name = game
            .table("baseitems")
            .ok()
            .and_then(|t| t.get_int(base, "Name"))
            .and_then(|s| game.string(mg_core::StrRef(s as u32)))
            .unwrap_or_default();
        let stats = super::item::statistics(game, &item);
        let properties: Vec<String> = item
            .list("PropertiesList")
            .unwrap_or(&[])
            .iter()
            .map(|p| game.property_text(&mg_rules::items::ItemProperty::from_gff(p)))
            .collect();
        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.weak("Chosen in the palette");
            ui.horizontal_top(|ui| {
                crate::images::icon_box(ui, &icon, ICON_MAX, &name);
                ui.vertical(|ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.strong(&name);
                        ui.weak(format!("{base_name} · {resref}"));
                    });
                    // Two statistics to a row where they fit.
                    let per_row = if ui.available_width() >= 420.0 { 2 } else { 1 };
                    egui::Grid::new(("chosen-item", self.key))
                        .num_columns(4)
                        .spacing([12.0, 2.0])
                        .show(ui, |ui| {
                            for pair in stats.chunks(per_row) {
                                for (label, value) in pair {
                                    ui.label(*label);
                                    ui.strong(value);
                                }
                                ui.end_row();
                            }
                        });
                });
            });
            if !properties.is_empty() {
                ui.label("Properties");
                for p in &properties {
                    ui.horizontal_wrapped(|ui| {
                        ui.add_space(8.0);
                        ui.strong(p);
                    });
                }
            }
        });
    }

    /// Where an inventory entry fits (a whole item's own base item, else
    /// its blueprint's).
    fn entry_fit(&mut self, entry: &Struct) -> ItemFit {
        match entry.integer("BaseItem") {
            Some(base) => self.base_item_fit(base),
            None => self.item_fit(entry_resref(entry)),
        }
    }

    fn base_item_fit(&self, base: i64) -> ItemFit {
        let table = self.app.game.as_ref().and_then(|g| g.table("baseitems").ok());
        let cell = |col: &str| {
            let t = table.as_ref()?;
            t.get_int(usize::try_from(base).ok()?, col).and_then(|v| u32::try_from(v).ok())
        };
        ItemFit {
            panel: cell("StorePanel"),
            w: cell("InvSlotWidth").unwrap_or(1).clamp(1, GRID_WIDTH),
            h: cell("InvSlotHeight").unwrap_or(1).max(1),
        }
    }

    /// The whole item a placed object holds for blueprint `resref` (`id`:
    /// its struct id), as Aurora expands it; `None` if not found.
    pub(super) fn held_item(&mut self, resref: ResRef, id: u32) -> Option<Struct> {
        let bp = self.blueprint(BlueprintKind::Item, resref)?;
        let game = self.app.game.as_ref()?;
        let ws = self.app.ws.as_ref();
        let item = |r: ResRef| -> Option<Struct> {
            let k = ResKey::new(r, mg_core::ResType::UTI);
            let data = ws
                .and_then(|w| w.module.get(&k).map(<[u8]>::to_vec))
                .or_else(|| game.resman.get(&k).ok().map(|d| d.into_owned()))?;
            mg_gff::Gff::read(&data).ok().map(|g| g.root)
        };
        let placing = mg_module::instances::Placing { game, item: &item };
        Some(mg_module::instances::held(&placing, &bp, id))
    }

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
    /// For a placed object, the whole item instead of its resref.
    pub(super) fn inventory_item(&mut self, items: &[Struct], resref: ResRef) -> Struct {
        let taken: Vec<_> = items
            .iter()
            .map(|it| {
                let fit = self.entry_fit(it);
                let x = it.integer("Repos_PosX").unwrap_or(0) as u32;
                let y = it.integer("Repos_Posy").unwrap_or(0) as u32;
                (x, y, fit.w, fit.h)
            })
            .collect();
        let fit = self.item_fit(resref);
        let (x, y) = place(&taken, fit.w, fit.h);
        let id = items.len() as u32;
        let mut item = if self.is_instance()
            && let Some(whole) = self.held_item(resref, id)
        {
            whole
        } else {
            let mut entry = Struct::new(id);
            entry.set("InventoryRes", Value::resref(resref));
            entry
        };
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

/// A creature's item flag (`Dropable`, `Pickpocketable`) as a checkbox:
/// written as 1 when set and left out when not, as the game's blueprints
/// have it. The edit, if it changed.
pub(super) fn flag_box(
    ui: &mut Ui,
    key: ResKey,
    path: GffPath,
    item: &Struct,
    flag: &'static str,
) -> Option<(&'static str, Edit)> {
    let mut on = item.integer(flag).unwrap_or(0) != 0;
    if !ui.checkbox(&mut on, flag).changed() {
        return None;
    }
    let value = on.then_some(Value::Byte(1));
    Some((flag, Edit::SetField { key, path, label: flag.into(), value }))
}

/// How an item list shows its items.
pub(super) struct ItemLook<'a> {
    /// Each item's icon layers.
    pub icons: &'a [Vec<Picture>],
    pub name: &'a dyn Fn(&Struct) -> String,
    /// A store's Infinite column.
    pub infinite: bool,
    /// The selected item (a creature's, for its options).
    pub selected: Option<usize>,
}

/// The width of a store's Infinite column.
const FLAG_WIDTH: f32 = 60.0;

/// The items of the `ItemList` at `path`, a row each across the whole
/// width: icon, name, (with `infinite`, a store's Infinite flag) and Remove.
/// The edits asked for, and the item clicked.
pub(super) fn item_list(
    ui: &mut Ui,
    key: ResKey,
    path: &GffPath,
    items: &[Struct],
    look: ItemLook<'_>,
) -> (Vec<(&'static str, Edit)>, Option<usize>) {
    let ItemLook { icons, name, infinite, selected } = look;
    let mut clicked = None;
    let mut edits = Vec::new();
    let gap = ui.spacing().item_spacing.x;
    let header = egui::vec2(ui.available_width(), ui.spacing().interact_size.y);
    let (rect, _) = ui.allocate_exact_size(header, egui::Sense::hover());
    let layout = egui::Layout::left_to_right(egui::Align::Center);
    let mut head = ui.new_child(egui::UiBuilder::new().max_rect(rect).layout(layout));
    head.add_space(ICON_MAX.x + gap);
    head.strong("Item");
    if infinite {
        head.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            // Over the checkboxes: past the Remove buttons.
            let font = egui::TextStyle::Button.resolve(ui.style());
            let remove = ui.painter().layout_no_wrap("Remove".into(), font, egui::Color32::WHITE);
            ui.add_space(remove.size().x + 2.0 * ui.spacing().button_padding.x + gap);
            let cell = egui::vec2(FLAG_WIDTH, header.y);
            let centered = egui::Layout::centered_and_justified(egui::Direction::LeftToRight);
            ui.allocate_ui_with_layout(cell, centered, |ui| ui.strong("Infinite"));
        });
    }
    // The list fills what is left of the page's visible height.
    let height = (ui.clip_rect().bottom() - ui.cursor().top()).max(200.0);
    egui::ScrollArea::vertical()
        .id_salt(("items", key, path.to_string()))
        .max_height(height)
        .auto_shrink([false, true])
        .show(ui, |ui| {
            for (i, it) in items.iter().enumerate() {
                let resref = entry_resref(it);
                let n = name(it);
                let layers = icons.get(i).map_or(&[][..], |v| v);
                // Each row as tall as its icon: armor at its own size, rings
                // and potions compact.
                let height = crate::images::icon_size(layers, ICON_MAX).y + 4.0;
                let row =
                    egui::vec2(ui.available_width(), height.max(ui.spacing().interact_size.y));
                let (rect, _) = ui.allocate_exact_size(row, egui::Sense::hover());
                if i % 2 == 1 {
                    ui.painter().rect_filled(rect, 0.0, ui.visuals().faint_bg_color);
                }
                let layout = egui::Layout::left_to_right(egui::Align::Center);
                let mut ui = ui.new_child(egui::UiBuilder::new().max_rect(rect).layout(layout));
                crate::images::icon_box(&mut ui, layers, ICON_MAX, &n);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
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
                    if infinite {
                        let cell = egui::vec2(FLAG_WIDTH, row.y);
                        let centered =
                            egui::Layout::centered_and_justified(egui::Direction::LeftToRight);
                        ui.allocate_ui_with_layout(cell, centered, |ui| {
                            let mut on = it.integer("Infinite").unwrap_or(0) != 0;
                            if ui.checkbox(&mut on, "").on_hover_text("Infinite").changed() {
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
                        });
                    }
                    let width = ui.available_width();
                    let label = egui::Button::selectable(
                        selected == Some(i),
                        (n.as_str(), egui::Atom::grow()),
                    )
                    .min_size(egui::vec2(width, 0.0))
                    .truncate();
                    if ui.add(label).on_hover_text(resref.to_string()).clicked() {
                        clicked = Some(i);
                    }
                });
            }
        });
    (edits, clicked)
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
