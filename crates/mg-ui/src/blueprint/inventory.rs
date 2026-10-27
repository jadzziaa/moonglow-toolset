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
        match self.kept_blueprint(BlueprintKind::Item, entry_resref(entry)) {
            Some(item) => self.app.item_icon(ctx, &item),
            None => Vec::new(),
        }
    }

    /// An entry's cost in gold (`GameData::item_cost`): a whole item's
    /// own, else its blueprint's; nothing without the game or the
    /// blueprint.
    pub(super) fn entry_cost(&mut self, entry: &Struct) -> Option<u32> {
        let kept;
        let item = match entry.integer("BaseItem") {
            Some(_) => entry,
            None => {
                kept = self.kept_blueprint(BlueprintKind::Item, entry_resref(entry))?;
                &*kept
            }
        };
        let game = self.app.game.as_deref()?;
        Some(game.item_cost(&mg_rules::ItemValue::from_gff(item)))
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
            // (A hak's 2DA may say anything: kept within reason, so the
            // places' sums in `place` cannot overflow.)
            h: cell("InvSlotHeight").unwrap_or(1).clamp(1, 255),
        }
    }

    /// The whole item a placed object holds for blueprint `resref` (`id`:
    /// its struct id), as Aurora expands it; `None` if not found.
    pub(super) fn held_item(&mut self, resref: ResRef, id: u32) -> Option<Struct> {
        let bp = self.blueprint(BlueprintKind::Item, resref)?;
        let game = self.app.game.as_deref()?;
        let module = self.app.ws.as_ref().map(|w| &w.module);
        let item =
            |r: ResRef| mg_module::gff_root(module, game, &ResKey::new(r, mg_core::ResType::UTI));
        let placing = mg_module::instances::Placing { game, item: &item };
        Some(mg_module::instances::held(&placing, &bp, id))
    }

    pub(super) fn item_fit(&mut self, resref: ResRef) -> ItemFit {
        let item = self.kept_blueprint(BlueprintKind::Item, resref);
        let base = item.and_then(|u| u.integer("BaseItem"));
        // (No blueprint or no base item: no row, as a negative one.)
        self.base_item_fit(base.unwrap_or(-1))
    }

    /// Where an item to add fits: a moved one's own, else its blueprint's.
    pub(super) fn added_fit(&mut self, added: &Added) -> ItemFit {
        match added {
            Added::Blueprint(r) => self.item_fit(*r),
            Added::Item(item) => self.entry_fit(item),
        }
    }

    /// A new entry in an inventory holding `items`, at its end: at the
    /// first free place, its struct id the list position (as Aurora writes
    /// them). A blueprint's item is made: for a placed object, the whole
    /// item instead of its resref. An item moved here is itself.
    pub(super) fn inventory_item(&mut self, items: &[Struct], added: &Added) -> Struct {
        let taken: Vec<_> = items
            .iter()
            .map(|it| {
                let fit = self.entry_fit(it);
                let x = it.integer("Repos_PosX").unwrap_or(0) as u32;
                let y = it.integer("Repos_Posy").unwrap_or(0) as u32;
                (x, y, fit.w, fit.h)
            })
            .collect();
        let id = items.len() as u32;
        let item = match added {
            Added::Blueprint(resref) => self.new_item(*resref, id),
            Added::Item(item) => self.carried(item, id),
        };
        let fit = self.added_fit(added);
        let (x, y) = place(&taken, fit.w, fit.h);
        mg_module::instances::listed_item(&item, id, x as u16, y as u16)
    }

    /// A new item of blueprint `resref` as this object holds one (`id`: its
    /// struct id): a placed object the whole item, a blueprint the resref.
    fn new_item(&mut self, resref: ResRef, id: u32) -> Struct {
        if self.is_instance()
            && let Some(whole) = self.held_item(resref, id)
        {
            return whole;
        }
        let mut entry = Struct::new(id);
        entry.set("InventoryRes", Value::resref(resref));
        entry
    }

    /// `item`, from another place, as this object holds it (`id`: its
    /// struct id): itself, every field it has kept, where it is of the
    /// kind this object holds (a whole item into a placed object, a
    /// blueprint's entry into a blueprint), as Aurora moves one. Otherwise
    /// it is made from its blueprint, keeping its flags.
    pub(super) fn carried(&mut self, item: &Struct, id: u32) -> Struct {
        if is_whole(item) == self.is_instance() {
            let mut s = item.clone();
            s.id = id;
            return s;
        }
        let mut s = self.new_item(entry_resref(item), id);
        for flag in ["Dropable", "Pickpocketable"] {
            if let Some(v) = item.get(flag) {
                s.set(flag, v.clone());
            }
        }
        s
    }
}

/// Whether an inventory's entry is a whole item (a placed object's), not
/// a blueprint's entry naming one.
pub(super) fn is_whole(entry: &Struct) -> bool {
    entry.integer("BaseItem").is_some()
}

/// What goes into an inventory: a new item of a blueprint, or an item
/// moved (or copied) from another place, as it is.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum Added {
    Blueprint(ResRef),
    Item(Struct),
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

/// Whether a creature's item has its flag (`Dropable`, `Pickpocketable`)
/// set: a byte, 0 (or left out, as the engine reads it) when not.
pub(super) fn flag_on(item: &Struct, flag: &str) -> bool {
    item.integer(flag).unwrap_or(0) != 0
}

/// Several items' flag, for one checkbox: (on: set on every one, mixed:
/// set on some and not on others). None at all: off, not mixed.
pub(super) fn flag_state<'a>(
    items: impl IntoIterator<Item = &'a Struct>,
    flag: &str,
) -> (bool, bool) {
    let (mut set, mut unset) = (0, 0);
    for item in items {
        if flag_on(item, flag) {
            set += 1;
        } else {
            unset += 1;
        }
    }
    (set > 0 && unset == 0, set > 0 && unset > 0)
}

/// The edit that sets a creature's item's flag (at `path`) to `on`:
/// written as 1, cleared by leaving it out, as Aurora and the game's
/// blueprints write them. None if it is so already (an item left as it
/// was keeps its bytes, a 0 the file has included).
pub(super) fn flag_edit(
    key: ResKey,
    path: GffPath,
    item: &Struct,
    flag: &'static str,
    on: bool,
) -> Option<Edit> {
    if flag_on(item, flag) == on {
        return None;
    }
    let value = on.then_some(Value::Byte(1));
    Some(Edit::SetField { key, path, label: flag.into(), value })
}

/// An item dragged in an inventory page: from the item palette, from a
/// list of items (a backpack, a container, a store's page), or from a
/// creature's equipment. `owner` tells whose list it is from (a drop on
/// another object's page is not taken).
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Dragged {
    Palette(ResRef),
    /// `source`: the document and the object in it whose `ItemList` it is;
    /// `item`: the item itself, as it is there.
    Listed {
        owner: egui::Id,
        source: (ResKey, GffPath),
        index: usize,
        resref: ResRef,
        item: Struct,
    },
    /// `source`: the creature whose `Equip_ItemList` it is.
    Equipped {
        owner: egui::Id,
        source: (ResKey, GffPath),
        index: usize,
        resref: ResRef,
        item: Struct,
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
        Some(Edit::remove_item(*key, path.clone(), list, index))
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

/// A row of an item list: the item's icon layers and its name.
pub(super) type Row = (Vec<Picture>, String);

/// How an item list shows its items.
pub(super) struct ItemLook<'a> {
    /// An item's icon layers and name (its index, the item): asked for
    /// the rows in sight only, as they are drawn (a store's page may hold
    /// thousands).
    pub row: &'a mut dyn FnMut(usize, &Struct) -> Row,
    /// A store's Infinite column.
    pub infinite: bool,
    /// The selected items (a creature's, for their options).
    pub selected: &'a [usize],
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
    let ItemLook { row: make, infinite, selected, equip, copied } = look;
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
    // Each row's height as it was last drawn (0: not yet), for the rows
    // out of sight, which take their room only; one never drawn takes
    // the middle height of those that were.
    let heights_id = egui::Id::new(("item-heights", key, path.to_string()));
    let mut heights: Vec<f32> = ui.data(|d| d.get_temp(heights_id)).unwrap_or_default();
    heights.resize(items.len(), 0.0);
    let mut drawn: Vec<f32> = heights.iter().copied().filter(|h| *h > 0.0).collect();
    drawn.sort_by(f32::total_cmp);
    let guess = drawn.get(drawn.len() / 2).copied().unwrap_or(ICON_MAX.y / 2.0 + 4.0);
    let list = egui::ScrollArea::vertical()
        .id_salt(("items", key, path.to_string()))
        .max_height(height)
        .min_scrolled_height(120.0)
        .auto_shrink([false, true])
        .show(ui, |ui| {
            // (Room to drop on, in an empty list too.)
            ui.set_min_height(48.0);
            for (i, it) in items.iter().enumerate() {
                let width = ui.available_width() - gutter;
                let known = if heights[i] > 0.0 { heights[i] } else { guess };
                let room = egui::Rect::from_min_size(ui.cursor().min, egui::vec2(width, known));
                if !ui.is_rect_visible(room) {
                    ui.allocate_space(egui::vec2(width, known));
                    continue;
                }
                let resref = entry_resref(it);
                let (layers, n) = make(i, it);
                let layers = &layers[..];
                // Each row as tall as its icon: armor at its own size, rings
                // and potions compact.
                let height = crate::images::icon_size(layers, ICON_MAX).y + 4.0;
                let row = egui::vec2(width, height.max(ui.spacing().interact_size.y));
                heights[i] = row.y;
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
                            Edit::remove_item(key, path.clone(), "ItemList", i),
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
                        selected.contains(&i),
                        (n.as_str(), egui::Atom::grow()),
                    )
                    .min_size(egui::vec2(width, 0.0))
                    .truncate()
                    .sense(egui::Sense::click_and_drag());
                    let r = ui.add(label).on_hover_text(format!("{n} ({resref})"));
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
                            item: it.clone(),
                        });
                    }
                    r.context_menu(|ui| {
                        if equip && ui.button("Equip").clicked() {
                            asked = Some((i, Ask::Equip));
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
                                Edit::remove_item(key, path.clone(), "ItemList", i),
                            ));
                            ui.close();
                        }
                        // Properties last, as in Aurora's menus.
                        ui.separator();
                        if ui
                            .button("Properties")
                            .on_hover_text(
                                "The item's properties: a placed object's item is its own; a \
                                 blueprint's is its blueprint (the module's to edit, the \
                                 game's to look at)",
                            )
                            .clicked()
                        {
                            asked = Some((i, Ask::Open));
                            ui.close();
                        }
                    });
                });
            }
        });
    ui.data_mut(|d| d.insert_temp(heights_id, heights));
    // A row moved within the list: out of its place and in before the row
    // it was let go on.
    if let Some((from, to)) = reorder
        && let Some(item) = items.get(from)
    {
        egui::DragAndDrop::clear_payload(ui.ctx());
        let at = if from < to { to - 1 } else { to };
        moved = vec![
            Edit::remove_item(key, path.clone(), "ItemList", from),
            Edit::insert_item(key, path.clone(), "ItemList", at, item.clone()),
        ];
    }
    // Ctrl+C on the row under the pointer (else the one selected), Ctrl+V
    // over the list: while nothing is typed in.
    let over = ui.rect_contains_pointer(list.inner_rect);
    if over && ui.ctx().memory(|m| m.focused().is_none()) {
        let event = |f: fn(&egui::Event) -> bool| ui.input(|i| i.events.iter().any(f));
        if event(|e| matches!(e, egui::Event::Copy))
            && let Some(i) = hovered.or(selected.first().copied())
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
    /// What an inventory page does with what its list came to: the items
    /// to add (picked, pasted or dropped) go to the end of the object's
    /// `ItemList` at `base`, with `taken` (what took a moved one from where
    /// it was), as one step; the list's own `edits`, each a step.
    pub(super) fn apply_list(
        &mut self,
        base: &GffPath,
        items: &[Struct],
        adds: Vec<Added>,
        taken: Vec<Edit>,
        edits: Vec<(&'static str, Edit)>,
    ) {
        let key = self.key;
        if !adds.is_empty() {
            let mut held = items.to_vec();
            let what = if taken.is_empty() { "Add item" } else { "Move item" };
            let mut together = taken;
            for added in adds {
                let item = self.inventory_item(&held, &added);
                together.push(Edit::insert_item(
                    key,
                    base.clone(),
                    "ItemList",
                    held.len(),
                    item.clone(),
                ));
                held.push(item);
            }
            self.app.actions.push(Action::Apply(Command::new(what, together)));
        }
        for (what, edit) in edits {
            self.app.actions.push(Action::Apply(Command::new(what, vec![edit])));
        }
    }

    /// Copy and Properties from a row's menu, a row moved within the
    /// list, and what there is to add to it: the items pasted, one dragged
    /// from the palette, one dragged from another object's inventory
    /// (moved out of it, unless Ctrl is held: then a copy; either way the
    /// item as it is there). What to add, and the edits that take the
    /// moved ones from where they were.
    pub(super) fn list_events(
        &mut self,
        ui: &Ui,
        items: &[Struct],
        out: &mut ListOut,
        whose: egui::Id,
        at: &GffPath,
    ) -> (Vec<Added>, Vec<Edit>) {
        let (mut adds, mut removals) = (Vec::new(), Vec::new());
        match out.asked {
            Some((i, Ask::Open)) => {
                if let Some(entry) = items.get(i) {
                    self.item_properties(at, "ItemList", i, entry);
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
            adds.extend(self.app.item_clip.iter().copied().map(Added::Blueprint));
        }
        match &out.dropped {
            Some(Dragged::Palette(r)) => adds.push(Added::Blueprint(*r)),
            Some(
                d @ (Dragged::Listed { owner, item, .. } | Dragged::Equipped { owner, item, .. }),
            ) if *owner != whose => {
                adds.push(Added::Item(item.clone()));
                removals.extend(self.taking(ui, d));
            }
            _ => {}
        }
        (adds, removals)
    }

    /// The edit that takes an item dropped here from where it was dragged
    /// from: none with Ctrl held (a copy), nor from the game's blueprint
    /// shown to look at (which nothing changes: a copy too).
    pub(super) fn taking(&self, ui: &Ui, dragged: &Dragged) -> Vec<Edit> {
        if ui.input(|i| i.modifiers.command) {
            return Vec::new();
        }
        let viewed = |e: &Edit| self.app.ws.as_ref().is_some_and(|ws| ws.is_viewed(e.key()));
        dragged.removal().into_iter().filter(|e| !viewed(e)).collect()
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

    fn item(fields: &[(&str, u8)]) -> Struct {
        let mut s = Struct::new(0);
        s.set("InventoryRes", Value::resref(ResRef::from_str("nw_it_torch001").unwrap()));
        for (label, v) in fields {
            s.set(label, Value::Byte(*v));
        }
        s
    }

    #[test]
    fn item_flags_read_as_the_engine_reads_them() {
        // Left out or 0: not set; any other byte: set.
        assert!(!flag_on(&item(&[]), "Dropable"));
        assert!(!flag_on(&item(&[("Dropable", 0)]), "Dropable"));
        assert!(flag_on(&item(&[("Dropable", 1)]), "Dropable"));
        assert!(flag_on(&item(&[("Pickpocketable", 2)]), "Pickpocketable"));
        // Several items: on when every one is, mixed when some are.
        let (on, off) = (item(&[("Dropable", 1)]), item(&[]));
        assert_eq!(flag_state([&on, &on], "Dropable"), (true, false));
        assert_eq!(flag_state([&on, &off], "Dropable"), (false, true));
        assert_eq!(flag_state([&off, &off], "Dropable"), (false, false));
        assert_eq!(flag_state([], "Dropable"), (false, false), "none selected");
    }

    #[test]
    fn item_flags_are_written_as_aurora_writes_them() {
        let key = ResKey::parse("bandit", mg_core::ResType::UTC).unwrap();
        let path = GffPath::root().item("ItemList", 0);
        let set = |s: &Struct, flag, on| flag_edit(key, path.clone(), s, flag, on);
        // Set: a byte 1. Cleared: left out.
        let edit = set(&item(&[]), "Dropable", true).unwrap();
        let Edit::SetField { label, value, path: at, .. } = &edit else { panic!("{edit:?}") };
        assert_eq!((label.as_str(), value, at), ("Dropable", &Some(Value::Byte(1)), &path));
        let edit = set(&item(&[("Pickpocketable", 1)]), "Pickpocketable", false).unwrap();
        let Edit::SetField { value, .. } = &edit else { panic!("{edit:?}") };
        assert_eq!(value, &None);
        // An item that has it so already is left as it is: an explicit 0
        // stays, as a 1 does.
        assert!(set(&item(&[("Dropable", 0)]), "Dropable", false).is_none());
        assert!(set(&item(&[]), "Dropable", false).is_none());
        assert!(set(&item(&[("Dropable", 1)]), "Dropable", true).is_none());
        // Applied and read back from the file: the bytes, and nothing else
        // changed.
        let mut m = mg_module::Module::new();
        let mut g = mg_gff::Gff::new(*b"UTC ");
        g.root.set("ItemList", Value::List(vec![item(&[("Dropable", 0)])]));
        m.set_gff(key, &g).unwrap();
        let mut ws = mg_edit::Workspace::new(m);
        let before = ws.module.get(&key).unwrap().to_vec();
        let doc = ws.doc(&key).unwrap().root.clone();
        let it = &doc.list("ItemList").unwrap()[0];
        let on = set(it, "Pickpocketable", true).unwrap();
        ws.apply(Command::new("Pickpocketable", vec![on])).unwrap();
        let saved = |ws: &mut mg_edit::Workspace| {
            ws.flush().unwrap();
            mg_gff::Gff::read(ws.module.get(&key).unwrap()).unwrap().root
        };
        let read = saved(&mut ws);
        let it = &read.list("ItemList").unwrap()[0];
        assert_eq!(it.get("Pickpocketable"), Some(&Value::Byte(1)));
        assert_eq!(it.get("Dropable"), Some(&Value::Byte(0)), "the 0 kept");
        ws.undo().unwrap();
        ws.flush().unwrap();
        assert_eq!(ws.module.get(&key).unwrap(), &before[..], "undone: the bytes it had");
    }

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
