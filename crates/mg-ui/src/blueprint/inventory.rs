//! Item lists as inventories (a store's pages, a placeable's contents):
//! where a new item goes, and the list with its Remove buttons.

use std::collections::HashMap;

use egui::Ui;
use mg_core::ResRef;
use mg_edit::{Command, Edit, GffPath};
use mg_gff::{Struct, Value};
use mg_module::palette::BlueprintKind;
use mg_resman::ResKey;

use super::Form;
use crate::Action;
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
            .and_then(|ls| self.app.game.as_deref().and_then(|g| g.locstring(ls)))
            .filter(|n| !n.is_empty());
        let r = entry_resref(entry);
        own.or_else(|| names.get(&r).cloned()).unwrap_or_else(|| r.to_string())
    }

    /// The item palette picker (see [`Form::palette_picker`]) with the chosen
    /// item beside it ([`Form::chosen_item`]), or below it in a narrow column.
    pub(super) fn item_picker(&mut self, ui: &mut Ui, add: &str) -> Option<ResRef> {
        const TREE: f32 = 240.0;
        if ui.available_width() < 2.0 * TREE {
            return self
                .palette_picker_over(ui, BlueprintKind::Item, add, |f, ui| f.chosen_item(ui));
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
        let Some(game) = self.app.game.as_deref() else { return };
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
                                    crate::widgets::field_label(ui, *label);
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
        let table = self.app.game.as_deref().and_then(|g| g.table("baseitems").ok());
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
        let game = self.app.game.as_deref()?;
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
        let table = self.app.game.as_deref().and_then(|g| g.table("baseitems").ok());
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

/// An item dragged in an inventory page: from the item palette, from a
/// list of items (a backpack, a container, a store's page), or from a
/// creature's equipment. `owner` tells whose list it is from (a drop on
/// another object's page is not taken).
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Dragged {
    Palette(ResRef),
    /// `source`: the document and the object in it whose `ItemList` it is.
    Listed {
        owner: egui::Id,
        source: (ResKey, GffPath),
        index: usize,
        resref: ResRef,
    },
    /// `source`: the creature whose `Equip_ItemList` it is.
    Equipped {
        owner: egui::Id,
        source: (ResKey, GffPath),
        index: usize,
        resref: ResRef,
    },
}

impl Dragged {
    /// The edit that takes a dragged item out of where it came from (a
    /// move to another object's inventory).
    pub(super) fn removal(&self) -> Option<Edit> {
        let (list, (key, path), index) = match self {
            Dragged::Palette(_) => return None,
            Dragged::Listed { source, index, .. } => ("ItemList", source, *index),
            Dragged::Equipped { source, index, .. } => ("Equip_ItemList", source, *index),
        };
        Some(Edit::RemoveItem { key: *key, path: path.clone(), list: list.into(), index })
    }
}

/// Whose items a page's are: the document and the object in it.
pub(super) fn owner(key: ResKey, path: &GffPath) -> egui::Id {
    egui::Id::new(("inventory-of", key, path.to_string()))
}

/// The item dragged that is let go over `rect` this frame (taken: it is
/// dropped here). While one is dragged over `rect`, the place is outlined.
pub(super) fn dropped_on(ui: &Ui, rect: egui::Rect) -> Option<Dragged> {
    let dragged = egui::DragAndDrop::payload::<Dragged>(ui.ctx())?;
    // (Where it shows, under the pointer: not a part scrolled out of sight,
    // nor under another window.)
    let over = ui.rect_contains_pointer(rect);
    if !over {
        return None;
    }
    let stroke = egui::Stroke::new(1.5, ui.visuals().selection.stroke.color);
    ui.painter().rect_stroke(rect, 2.0, stroke, egui::StrokeKind::Inside);
    if ui.input(|i| i.pointer.any_released()) {
        egui::DragAndDrop::clear_payload(ui.ctx());
        return Some((*dragged).clone());
    }
    None
}

/// What a list row's right-click menu asks of its item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Ask {
    Equip,
    Open,
    Copy,
}

/// What an item list reports: the edits asked for, the item clicked, an
/// item dropped on it, and what a row's menu asked.
pub(super) struct ListOut {
    pub edits: Vec<(&'static str, Edit)>,
    pub clicked: Option<usize>,
    pub dropped: Option<Dragged>,
    pub asked: Option<(usize, Ask)>,
    /// A row dragged to another place in the list: the edits that move it,
    /// to make together.
    pub moved: Vec<Edit>,
    /// Paste was asked for (the header's button, a row's menu, Ctrl+V over
    /// the list).
    pub paste: bool,
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
    /// Its items can be equipped (a creature's backpack): the rows' menu
    /// offers it.
    pub equip: bool,
    /// How many items are copied, to paste (0: none).
    pub copied: usize,
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
) -> ListOut {
    let ItemLook { icons, name, infinite, selected, equip, copied } = look;
    let mut clicked = None;
    let mut asked = None;
    let (mut paste, mut moved) = (false, Vec::new());
    // The row under the pointer (for Ctrl+C), and a row let go over another
    // (from, to).
    let (mut hovered, mut reorder) = (None, None);
    let mut edits = Vec::new();
    let whose = owner(key, path);
    let gap = ui.spacing().item_spacing.x;
    // Clear of the scroll bar (which floats over the list's right edge).
    let scroll = &ui.spacing().scroll;
    let gutter = scroll.bar_width + scroll.bar_inner_margin + scroll.bar_outer_margin + 4.0;
    let header = egui::vec2(ui.available_width() - gutter, ui.spacing().interact_size.y);
    let (rect, _) = ui.allocate_exact_size(header, egui::Sense::hover());
    let layout = egui::Layout::left_to_right(egui::Align::Center);
    let mut head = ui.new_child(egui::UiBuilder::new().max_rect(rect).layout(layout));
    head.add_space(ICON_MAX.x + gap);
    head.strong("Item");
    if copied > 0
        && head
            .small_button(format!("Paste ({copied})"))
            .on_hover_text("Add the items copied (Ctrl+V with the pointer over the list)")
            .clicked()
    {
        paste = true;
    }
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
    let list = egui::ScrollArea::vertical()
        .id_salt(("items", key, path.to_string()))
        .max_height(height)
        .min_scrolled_height(120.0)
        .auto_shrink([false, true])
        .show(ui, |ui| {
            // (Room to drop on, in an empty list too.)
            ui.set_min_height(48.0);
            for (i, it) in items.iter().enumerate() {
                let resref = entry_resref(it);
                let n = name(it);
                let layers = icons.get(i).map_or(&[][..], |v| v);
                // Each row as tall as its icon: armor at its own size, rings
                // and potions compact.
                let height = crate::images::icon_size(layers, ICON_MAX).y + 4.0;
                let row = egui::vec2(
                    ui.available_width() - gutter,
                    height.max(ui.spacing().interact_size.y),
                );
                let (rect, _) = ui.allocate_exact_size(row, egui::Sense::hover());
                if ui.rect_contains_pointer(rect) {
                    hovered = Some(i);
                    // One of the list's own rows let go here takes this
                    // place (a line shows where).
                    if let Some(Dragged::Listed { owner, index, .. }) =
                        egui::DragAndDrop::payload::<Dragged>(ui.ctx()).as_deref()
                        && *owner == whose
                        && *index != i
                    {
                        let line = egui::Stroke::new(2.0, ui.visuals().selection.stroke.color);
                        ui.painter().hline(rect.x_range(), rect.top(), line);
                        if ui.input(|i| i.pointer.any_released()) {
                            reorder = Some((*index, i));
                        }
                    }
                }
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
                    .truncate()
                    .sense(egui::Sense::click_and_drag());
                    let r = ui.add(label).on_hover_text(resref.to_string());
                    if r.clicked() {
                        clicked = Some(i);
                    }
                    // Dragged: onto a slot to equip it, or out of a slot's
                    // list into another.
                    if r.drag_started() {
                        r.dnd_set_drag_payload(Dragged::Listed {
                            owner: whose,
                            source: (key, path.clone()),
                            index: i,
                            resref,
                        });
                    }
                    r.context_menu(|ui| {
                        if equip && ui.button("Equip").clicked() {
                            asked = Some((i, Ask::Equip));
                            ui.close();
                        }
                        if ui.button("Open Blueprint").clicked() {
                            asked = Some((i, Ask::Open));
                            ui.close();
                        }
                        if ui.button("Copy").clicked() {
                            asked = Some((i, Ask::Copy));
                            ui.close();
                        }
                        if ui.add_enabled(copied > 0, egui::Button::new("Paste")).clicked() {
                            paste = true;
                            ui.close();
                        }
                        if ui.button("Remove").clicked() {
                            edits.push((
                                "Remove item",
                                Edit::RemoveItem {
                                    key,
                                    path: path.clone(),
                                    list: "ItemList".into(),
                                    index: i,
                                },
                            ));
                            ui.close();
                        }
                    });
                });
            }
        });
    // A row moved within the list: out of its place and in before the row
    // it was let go on.
    if let Some((from, to)) = reorder
        && let Some(item) = items.get(from)
    {
        egui::DragAndDrop::clear_payload(ui.ctx());
        let at = if from < to { to - 1 } else { to };
        moved = vec![
            Edit::RemoveItem { key, path: path.clone(), list: "ItemList".into(), index: from },
            Edit::InsertItem {
                key,
                path: path.clone(),
                list: "ItemList".into(),
                index: at,
                item: item.clone(),
            },
        ];
    }
    // Ctrl+C on the row under the pointer (else the one selected), Ctrl+V
    // over the list: while nothing is typed in.
    let over = ui.rect_contains_pointer(list.inner_rect);
    if over && ui.ctx().memory(|m| m.focused().is_none()) {
        let event = |f: fn(&egui::Event) -> bool| ui.input(|i| i.events.iter().any(f));
        if event(|e| matches!(e, egui::Event::Copy))
            && let Some(i) = hovered.or(selected)
        {
            asked = Some((i, Ask::Copy));
        }
        if copied > 0 && event(|e| matches!(e, egui::Event::Paste(_))) {
            paste = true;
        }
    }
    // An item let go over the list (not one of its own).
    let dropped = dropped_on(ui, list.inner_rect)
        .filter(|d| !matches!(d, Dragged::Listed { owner, .. } if *owner == whose));
    ListOut { edits, clicked, dropped, asked, moved, paste }
}

impl Form<'_> {
    /// What an item list reported that is the same for every inventory:
    /// Copy and Open Blueprint from a row's menu, a row moved within the
    /// list, and what there is to add to it: the items pasted, one dragged
    /// from the palette, one dragged from another object's inventory
    /// (moved out of it, unless Ctrl is held: then a copy). The blueprints
    /// to add, and the edits that take the moved ones from where they were.
    pub(super) fn list_events(
        &mut self,
        ui: &Ui,
        items: &[Struct],
        out: &mut ListOut,
        whose: egui::Id,
    ) -> (Vec<ResRef>, Vec<Edit>) {
        let (mut adds, mut removals) = (Vec::new(), Vec::new());
        match out.asked {
            Some((i, Ask::Open)) => {
                if let Some(r) = items.get(i).map(entry_resref).filter(|r| !r.is_empty()) {
                    let item = ResKey::new(r, mg_core::ResType::UTI);
                    crate::palette_view::view_blueprint(self.app, item);
                }
            }
            Some((i, Ask::Copy)) => {
                if let Some(r) = items.get(i).map(entry_resref).filter(|r| !r.is_empty()) {
                    self.copy_item(ui, r);
                }
            }
            _ => {}
        }
        if !out.moved.is_empty() {
            let moved = std::mem::take(&mut out.moved);
            self.app.actions.push(Action::Apply(Command::new("Move item", moved)));
        }
        if out.paste {
            adds.extend(self.app.item_clip.iter().copied());
        }
        match &out.dropped {
            Some(Dragged::Palette(r)) => adds.push(*r),
            Some(
                d @ (Dragged::Listed { owner, resref, .. }
                | Dragged::Equipped { owner, resref, .. }),
            ) if *owner != whose => {
                adds.push(*resref);
                if !ui.input(|i| i.modifiers.command) {
                    removals.extend(d.removal());
                }
            }
            _ => {}
        }
        (adds, removals)
    }

    /// Copies an item (its blueprint), to paste into an inventory.
    pub(super) fn copy_item(&mut self, ui: &Ui, resref: ResRef) {
        self.app.item_clip = vec![resref];
        crate::widgets::mark_clipboard(ui.ctx(), "1 item");
    }
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
