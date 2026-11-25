//! Item lists as inventories (a store's pages, a placeable's contents):
//! where a new item goes, and the list with its Remove buttons.

use mg_uikit::Controls as _;
use std::collections::HashMap;

use egui::Ui;
use mg_core::ResRef;
use mg_edit::{Command, Edit, GffPath, lists};
use mg_gff::{Struct, Value};
use mg_module::palette::BlueprintKind;
use mg_resman::ResKey;

use super::Form;
use crate::Action;
use crate::images::{ICON_MAX, Picture};

/// An inventory is a grid this many cells wide.
pub(super) const GRID_WIDTH: u32 = mg_rules::levelup::INVENTORY_WIDTH as u32;

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
    /// An entry's inventory icon: a whole item's own, else its blueprint's;
    /// a woman's where armor is shown on one (Options › General).
    pub(super) fn entry_icon(&mut self, ctx: &egui::Context, entry: &Struct) -> Vec<Picture> {
        let female = self.app.settings.armor_on_woman;
        self.entry_icon_of(ctx, entry, female)
    }

    /// [`Form::entry_icon`] as a woman's inventory shows it where `female`,
    /// else as a man's: in a creature's own inventory, by its Gender.
    pub(super) fn entry_icon_of(
        &mut self,
        ctx: &egui::Context,
        entry: &Struct,
        female: bool,
    ) -> Vec<Picture> {
        if entry.integer("BaseItem").is_some() {
            return self.app.item_icon_of(ctx, entry, female);
        }
        match self.kept_blueprint(BlueprintKind::Item, entry_resref(entry)) {
            Some(item) => self.app.item_icon_of(ctx, &item, female),
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
    /// Why the creature may not wear an item it has equipped (`entry`, of
    /// its `Equip_ItemList`), said for a builder: the game takes such an
    /// item off, into the backpack, as it loads the creature
    /// ([`mg_rules::GameData::wear_problem`]).
    pub(super) fn wear_problem(&mut self, entry: &Struct) -> Option<String> {
        let kept = match entry.integer("BaseItem") {
            Some(_) => None,
            None => Some(self.kept_blueprint(BlueprintKind::Item, entry_resref(entry))?),
        };
        let item = kept.as_deref().unwrap_or(entry);
        let game = self.app.game.as_deref()?;
        let problem = game.wear_problem(&self.root, item)?;
        Some(game.wear_problem_text(&problem))
    }

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
        let game = self.app.game.as_deref();
        let row = usize::try_from(base).ok();
        let panel = game.zip(row).and_then(|(g, row)| {
            let v = g.table("baseitems").ok()?.get_int(row, "StorePanel")?;
            u32::try_from(v).ok()
        });
        // (The room it takes by the one reading of it, which keeps what a
        // hak's 2DA says within the grid: `GameData::inventory_size`.)
        let (w, h) = game.zip(row).map_or((1, 1), |(g, row)| g.inventory_size(row));
        ItemFit { panel, w: u32::from(w), h: u32::from(h) }
    }

    /// The whole item a placed object holds for blueprint `resref` (`id`:
    /// its struct id), as Aurora expands it; `None` if not found.
    pub(super) fn held_item(&mut self, resref: ResRef, id: u32) -> Option<Struct> {
        let bp = self.blueprint(BlueprintKind::Item, resref)?;
        let game = self.app.game.as_deref()?;
        let module = self.app.ws.as_ref().map(|w| &w.module);
        let held = mg_module::instances::with_placing(game, module, |placing, _| {
            mg_module::instances::held(placing, &bp, id)
        });
        Some(held)
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
                let (x, y) = lists::placed_at(it);
                (x, y, fit.w, fit.h)
            })
            .collect();
        let id = items.len() as u32;
        let item = match added {
            Added::Blueprint(resref) => self.new_item(*resref, id),
            Added::Item(item) => self.carried(item, id),
        };
        let fit = self.added_fit(added);
        let (x, y) = lists::place(&taken, GRID_WIDTH, fit.w, fit.h);
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

/// One of an object's items that a drag, Copy or Cut carries: the list
/// it is in (`ItemList`, or a creature's `Equip_ItemList`), its place
/// there, and the item itself, as it is.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Carried {
    pub list: &'static str,
    pub index: usize,
    pub item: Struct,
}

impl Carried {
    /// Its place among an object's items: a creature's slots first (in
    /// their order), then its list.
    fn order(&self) -> (bool, usize) {
        (self.list != "Equip_ItemList", self.index)
    }
}

/// What a row's drag, Copy or Cut carries (`row`): the items `selected`
/// (every list's of the object) where the row is one of them, in their
/// order; else the row alone.
pub(super) fn carried_with(row: Carried, selected: &[Carried]) -> Vec<Carried> {
    let is_row = |c: &Carried| (c.list, c.index) == (row.list, row.index);
    if !selected.iter().any(is_row) {
        return vec![row];
    }
    let mut all = selected.to_vec();
    all.sort_by_key(Carried::order);
    all
}

/// The edits that take `carried` (each with the item it takes) out of the
/// object at `source`: within a list from the last place to the first, so
/// that each place is still the item's as its edit runs.
pub(super) fn removals(source: &(ResKey, GffPath), carried: &[Carried]) -> Vec<(Edit, Struct)> {
    let mut sorted: Vec<&Carried> = carried.iter().collect();
    sorted.sort_by_key(|c| (c.list, std::cmp::Reverse(c.index)));
    let (key, path) = source;
    let take =
        |c: &&Carried| (Edit::remove_item(*key, path.clone(), c.list, c.index), c.item.clone());
    sorted.iter().map(take).collect()
}

/// An item dragged in an inventory page: from the item palette, from a
/// list of items (a backpack, a container, a store's page), or from a
/// creature's equipment. `owner` tells whose list it is from (a drop on
/// another object's page is not taken).
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Dragged {
    Palette(ResRef),
    /// `source`: the document and the object in it whose `ItemList` it is;
    /// `item`: the item itself, as it is there; `more`: the other items
    /// of the object selected with it, carried along.
    Listed {
        owner: egui::Id,
        source: (ResKey, GffPath),
        index: usize,
        resref: ResRef,
        item: Struct,
        more: Vec<Carried>,
    },
    /// `source`: the creature whose `Equip_ItemList` it is.
    Equipped {
        owner: egui::Id,
        source: (ResKey, GffPath),
        index: usize,
        resref: ResRef,
        item: Struct,
        more: Vec<Carried>,
    },
}

impl Dragged {
    /// How many items it carries.
    pub(super) fn count(&self) -> usize {
        match self {
            Dragged::Palette(_) => 1,
            Dragged::Listed { more, .. } | Dragged::Equipped { more, .. } => 1 + more.len(),
        }
    }

    /// Whose items it carries (none: the palette's).
    pub(super) fn owner(&self) -> Option<egui::Id> {
        match self {
            Dragged::Palette(_) => None,
            Dragged::Listed { owner, .. } | Dragged::Equipped { owner, .. } => Some(*owner),
        }
    }

    /// The item the drag began on: its list and its place there.
    pub(super) fn lead(&self) -> Option<(&'static str, usize)> {
        match self {
            Dragged::Palette(_) => None,
            Dragged::Listed { index, .. } => Some(("ItemList", *index)),
            Dragged::Equipped { index, .. } => Some(("Equip_ItemList", *index)),
        }
    }

    /// The items it carries, in their order (a creature's slots', then its
    /// list's); the one the drag began on among them. None: the palette's.
    pub(super) fn carried(&self) -> Vec<Carried> {
        let (list, index, item, more) = match self {
            Dragged::Palette(_) => return Vec::new(),
            Dragged::Listed { index, item, more, .. } => ("ItemList", index, item, more),
            Dragged::Equipped { index, item, more, .. } => ("Equip_ItemList", index, item, more),
        };
        let mut all = more.clone();
        all.push(Carried { list, index: *index, item: item.clone() });
        all.sort_by_key(Carried::order);
        all
    }

    /// The edits that take the dragged items out of where they came from
    /// (a move to another object's inventory), each with the item it
    /// takes.
    pub(super) fn removals(&self) -> Vec<(Edit, Struct)> {
        match self {
            Dragged::Palette(_) => Vec::new(),
            Dragged::Listed { source, .. } | Dragged::Equipped { source, .. } => {
                removals(source, &self.carried())
            }
        }
    }
}

/// While several items are dragged, how many, beside the pointer (once a
/// frame, whichever inventory asks first). It takes no pointer: a drop
/// lands under it.
pub(super) fn show_count(ctx: &egui::Context) {
    let Some(dragged) = egui::DragAndDrop::payload::<Dragged>(ctx) else { return };
    let n = dragged.count();
    let Some(at) = ctx.pointer_latest_pos().filter(|_| n > 1) else { return };
    let id = egui::Id::new("inventory-carried-count");
    let pass = ctx.cumulative_pass_nr();
    if ctx.data(|d| d.get_temp::<u64>(id)) == Some(pass) {
        return;
    }
    ctx.data_mut(|d| d.insert_temp(id, pass));
    egui::Area::new(id)
        .order(egui::Order::Tooltip)
        .interactable(false)
        .fixed_pos(at + egui::vec2(16.0, 16.0))
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| ui.label(format!("{n} items")));
        });
}

/// The name of the step that adds `n` items to an inventory: `one` for a
/// single item (each inventory's own), or one that says how many and how
/// (`moved` from another place, `pasted`).
pub(super) fn adding_name(one: &str, n: usize, moved: bool, pasted: bool) -> String {
    match (n > 1, moved, pasted) {
        (false, true, _) => "Move item".into(),
        (false, ..) => one.into(),
        (true, true, _) => format!("Move {n} items"),
        (true, false, true) => format!("Paste {n} items"),
        (true, false, false) => format!("Add {n} items"),
    }
}

/// The rows selected in an item list that keeps its own selection (a
/// placeable's, a store's page): Ctrl and Shift select several. Let go
/// when the list's length changes (its places then mean other items).
#[derive(Clone, Default)]
struct Rows {
    rows: std::collections::BTreeSet<usize>,
    anchor: Option<usize>,
    len: usize,
}

/// The rows selected in the list kept at `id`, which has `len` items now.
pub(super) fn selected_rows(ui: &Ui, id: egui::Id, len: usize) -> Vec<usize> {
    let rows: Option<Rows> = ui.data(|d| d.get_temp(id));
    match rows {
        Some(r) if r.len == len => r.rows.into_iter().collect(),
        // (Let go for good: the list back at its old length later holds
        // other items at those places.)
        Some(_) => {
            ui.data_mut(|d| d.remove::<Rows>(id));
            Vec::new()
        }
        None => Vec::new(),
    }
}

/// What the list kept at `id` (`len` items) does with its rows: `clicked`
/// selected (alone; with Ctrl, added or taken away; with Shift, the rows
/// from the one clicked last); all let go once some were `moved`.
pub(super) fn select_rows(ui: &Ui, id: egui::Id, len: usize, clicked: Option<usize>, moved: bool) {
    if moved {
        ui.data_mut(|d| d.remove::<Rows>(id));
        return;
    }
    let Some(row) = clicked else { return };
    let mut r: Rows =
        ui.data(|d| d.get_temp(id)).filter(|r: &Rows| r.len == len).unwrap_or_default();
    r.len = len;
    let listed: Vec<usize> = (0..len).collect();
    let m = ui.input(|i| i.modifiers);
    crate::widgets::select_in_list(&mut r.rows, &mut r.anchor, &listed, row, m.command, m.shift);
    ui.data_mut(|d| d.insert_temp(id, r));
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
    let stroke = egui::Stroke::new(1.5, mg_uikit::tokens(ui.ctx()).accent);
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
    Find,
}

/// What an item list reports: the edits asked for, the item clicked, an
/// item dropped on it, and what a row's menu asked.
pub(super) struct ListOut {
    pub edits: Vec<(&'static str, Edit)>,
    pub clicked: Option<usize>,
    pub dropped: Option<Dragged>,
    pub asked: Option<(usize, Ask)>,
    /// Rows dragged to another place in the list: the edits that move
    /// them, to make together.
    pub moved: Vec<Edit>,
    /// What Copy (a row's menu, Ctrl+C over the list) and Cut (a row's
    /// menu, Ctrl+X) were asked for: the rows selected, or the one row.
    pub copy: Vec<Carried>,
    pub cut: Vec<Carried>,
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
    /// The selected items: a drag, Copy or Cut of one of them carries
    /// them all.
    pub selected: &'a [usize],
    /// The object's items selected in its other lists (a creature's
    /// equipment), carried along with the list's.
    pub also: &'a [Carried],
    /// Its items can be equipped (a creature's backpack): the rows' menu
    /// offers it.
    pub equip: bool,
    /// How many items are copied, to paste (0: none).
    pub copied: usize,
    /// The item blueprints in the palettes, keyed by resref. A row offers
    /// Find in Palette only if its blueprint is listed here.
    pub palette: &'a HashMap<ResRef, String>,
}

/// Find in Palette's tip.
const FIND_IN_PALETTE_TIP: &str = "Choose the item's blueprint in the item palette";

/// Adds Find in Palette to an item's right-click menu. The button is
/// enabled only if the item's blueprint is in a palette (`found`).
/// Returns whether it was clicked.
pub(super) fn find_in_palette_button(ui: &mut Ui, found: bool) -> bool {
    ui.add_enabled(found, egui::Button::new("Find in Palette"))
        .on_hover_text(FIND_IN_PALETTE_TIP)
        .on_disabled_hover_text(
            "The item's blueprint is in neither palette. It may be gone, or in an unused hak.",
        )
        .clicked()
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
    let ItemLook { row: make, infinite, selected, also, equip, copied, palette } = look;
    show_count(ui.ctx());
    // Every item of the object selected, as a drag, Copy or Cut carries
    // them.
    let mut chosen: Vec<Carried> = also.to_vec();
    let listed = |&i: &usize| {
        let item = items.get(i)?.clone();
        Some(Carried { list: "ItemList", index: i, item })
    };
    chosen.extend(selected.iter().filter_map(listed));
    let carry = |i: usize| carried_with(listed(&i).expect("a row"), &chosen);
    let (mut copy, mut cut) = (Vec::new(), Vec::new());
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
    // The list fills what is left of the page's visible height (or a fixed
    // cap inside the Properties panel's all-pages scroll so it never depends
    // on scroll position).
    let height = if super::panel_scroll(ui) {
        240.0
    } else {
        (ui.clip_rect().bottom() - ui.cursor().top()).max(200.0)
    };
    // Each row's height as it was last drawn (0: not yet), for the rows
    // out of sight, which take their room only; one never drawn takes
    // the middle height of those that were.
    #[derive(Clone)]
    struct ItemHeights {
        heights: Vec<f32>,
        guess: f32,
    }
    let heights_id = egui::Id::new(("item-heights", key, path.to_string()));
    let mut stored =
        ui.data(|d| d.get_temp::<std::sync::Arc<ItemHeights>>(heights_id)).unwrap_or_else(|| {
            std::sync::Arc::new(ItemHeights { heights: Vec::new(), guess: ICON_MAX.y / 2.0 + 4.0 })
        });
    let mut modified = stored.heights.len() != items.len();
    if modified {
        let mut next = stored.heights.clone();
        next.resize(items.len(), 0.0);
        let mut drawn: Vec<f32> = next.iter().copied().filter(|h| *h > 0.0).collect();
        drawn.sort_by(f32::total_cmp);
        let guess = drawn.get(drawn.len() / 2).copied().unwrap_or(ICON_MAX.y / 2.0 + 4.0);
        stored = std::sync::Arc::new(ItemHeights { heights: next, guess });
    }
    let guess = stored.guess;
    let mut updated: Vec<(usize, f32)> = Vec::new();
    let list = egui::ScrollArea::vertical()
        .id_salt(("items", key, path.to_string()))
        .max_height(height)
        .min_scrolled_height(120.0)
        .auto_shrink([false, true])
        .show(ui, |ui| {
            // (Room to drop on, in an empty list too.)
            ui.set_min_height(48.0);
            // (Rows that meet: each as tall as its icon, at least a row.)
            mg_uikit::list_rows(ui);
            let width = ui.available_width() - gutter;
            let clip_top = ui.clip_rect().top();
            let clip_bottom = ui.clip_rect().bottom();
            let spacing_y = ui.spacing().item_spacing.y;
            let x_vis = ui
                .clip_rect()
                .x_range()
                .intersects(egui::Rangef::new(ui.cursor().min.x, ui.cursor().min.x + width));
            let mut y = ui.cursor().top();
            let mut skipped_h = 0.0f32;
            let mut skipped_n = 0usize;
            for (i, it) in items.iter().enumerate() {
                let known = if stored.heights[i] > 0.0 { stored.heights[i] } else { guess };
                let visible = x_vis && y + known >= clip_top && y <= clip_bottom;
                if !visible {
                    skipped_h += known;
                    skipped_n += 1;
                    y += known + spacing_y;
                    continue;
                }
                if skipped_n > 0 {
                    ui.allocate_space(egui::vec2(
                        width,
                        skipped_h + (skipped_n - 1) as f32 * spacing_y,
                    ));
                    skipped_h = 0.0;
                    skipped_n = 0;
                }
                let resref = entry_resref(it);
                let (layers, n) = make(i, it);
                let layers = &layers[..];
                // Each row as tall as its icon: armor at its own size, rings
                // and potions compact.
                let height = crate::images::icon_size(layers, ICON_MAX).y + 4.0;
                let row = egui::vec2(width, height.max(ui.spacing().interact_size.y));
                if (stored.heights[i] - row.y).abs() > f32::EPSILON {
                    updated.push((i, row.y));
                }
                let (rect, _) = ui.allocate_exact_size(row, egui::Sense::hover());
                y = ui.cursor().top();
                if ui.rect_contains_pointer(rect) {
                    hovered = Some(i);
                    // The list's own rows let go here take this place (a
                    // line shows where), the one dragged and those
                    // selected with it, in their order.
                    if let Some(d) = egui::DragAndDrop::payload::<Dragged>(ui.ctx())
                        && let Some(rows) = own_rows(&d, whose)
                        && !rows.contains(&i)
                    {
                        let line = egui::Stroke::new(2.0, mg_uikit::tokens(ui.ctx()).accent);
                        ui.painter().hline(rect.x_range(), rect.top(), line);
                        if ui.input(|i| i.pointer.any_released()) {
                            reorder = Some((rows, i));
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
                            "Remove Item",
                            Edit::remove_item(key, path.clone(), "ItemList", i),
                        ));
                    }
                    if infinite {
                        let cell = egui::vec2(FLAG_WIDTH, row.y);
                        let centered =
                            egui::Layout::centered_and_justified(egui::Direction::LeftToRight);
                        ui.allocate_ui_with_layout(cell, centered, |ui| {
                            let mut on = it.integer("Infinite").unwrap_or(0) != 0;
                            if ui.check_box(&mut on, "").on_hover_text("Infinite").changed() {
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
                        let mut more = carry(i);
                        more.retain(|c| (c.list, c.index) != ("ItemList", i));
                        r.dnd_set_drag_payload(Dragged::Listed {
                            owner: whose,
                            source: (key, path.clone()),
                            index: i,
                            resref,
                            item: it.clone(),
                            more,
                        });
                    }
                    mg_uikit::context_menu(&r, |ui| {
                        if equip && ui.button("Equip").clicked() {
                            asked = Some((i, Ask::Equip));
                            ui.close();
                        }
                        if ui.button("Cut").clicked() {
                            cut = carry(i);
                            ui.close();
                        }
                        if ui.button("Copy").clicked() {
                            copy = carry(i);
                            ui.close();
                        }
                        if ui.add_enabled(copied > 0, egui::Button::new("Paste")).clicked() {
                            paste = true;
                            ui.close();
                        }
                        if ui.button("Remove").clicked() {
                            edits.push((
                                "Remove Item",
                                Edit::remove_item(key, path.clone(), "ItemList", i),
                            ));
                            ui.close();
                        }
                        if find_in_palette_button(ui, palette.contains_key(&resref)) {
                            asked = Some((i, Ask::Find));
                            ui.close();
                        }
                        // Properties last, as in Aurora's menus.
                        mg_uikit::menu_separator(ui);
                        if ui
                            .button("Properties")
                            .on_hover_text(
                                "Open the item's properties (a blueprint's item opens its \
                                 blueprint)",
                            )
                            .clicked()
                        {
                            asked = Some((i, Ask::Open));
                            ui.close();
                        }
                    });
                });
            }
            if skipped_n > 0 {
                ui.allocate_space(egui::vec2(
                    width,
                    skipped_h + (skipped_n - 1) as f32 * spacing_y,
                ));
            }
        });
    if !updated.is_empty() {
        modified = true;
        let inner = std::sync::Arc::make_mut(&mut stored);
        for (i, h) in updated {
            inner.heights[i] = h;
        }
        let mut drawn: Vec<f32> = inner.heights.iter().copied().filter(|h| *h > 0.0).collect();
        drawn.sort_by(f32::total_cmp);
        inner.guess = drawn.get(drawn.len() / 2).copied().unwrap_or(ICON_MAX.y / 2.0 + 4.0);
    }
    if modified {
        ui.data_mut(|d| d.insert_temp(heights_id, stored));
    }
    // Rows moved within the list: out of their places and in, in their
    // order, before the row they were let go on.
    if let Some((from, to)) = reorder
        && from.iter().all(|&i| i < items.len())
    {
        egui::DragAndDrop::clear_payload(ui.ctx());
        let at = to - from.iter().filter(|&&i| i < to).count();
        let source = (key, path.clone());
        let rows: Vec<Carried> = from.iter().filter_map(listed).collect();
        moved = removals(&source, &rows).into_iter().map(|(e, _)| e).collect();
        for (k, c) in rows.into_iter().enumerate() {
            moved.push(Edit::insert_item(key, path.clone(), "ItemList", at + k, c.item));
        }
    }
    // Ctrl+C and Ctrl+X on the row under the pointer (with those selected
    // where it is one of them; with none under it, those selected), Ctrl+V
    // over the list: while nothing is typed in.
    let over = ui.rect_contains_pointer(list.inner_rect);
    if over && ui.ctx().memory(|m| m.focused().is_none()) {
        let event = |f: fn(&egui::Event) -> bool| ui.input(|i| i.events.iter().any(f));
        let under = || match hovered {
            Some(i) => carry(i),
            None => {
                let mut all = chosen.clone();
                all.sort_by_key(Carried::order);
                all
            }
        };
        if event(|e| matches!(e, egui::Event::Copy)) {
            copy = under();
        }
        if event(|e| matches!(e, egui::Event::Cut)) {
            cut = under();
        }
        if copied > 0 && event(|e| matches!(e, egui::Event::Paste(_))) {
            paste = true;
        }
    }
    // An item let go over the list (not of its own rows alone).
    let dropped = dropped_on(ui, list.inner_rect).filter(|d| own_rows(d, whose).is_none());
    ListOut { edits, clicked, dropped, asked, moved, paste, copy, cut }
}

/// The rows of the list `whose` that `dragged` carries, in their order,
/// where it carries that list's rows alone (a move within the list).
fn own_rows(dragged: &Dragged, whose: egui::Id) -> Option<Vec<usize>> {
    let Dragged::Listed { owner, .. } = dragged else { return None };
    let carried = dragged.carried();
    let rows = carried.iter().all(|c| c.list == "ItemList");
    (*owner == whose && rows).then(|| carried.iter().map(|c| c.index).collect())
}

/// Where an item added to an inventory goes ([`Form::add_items`]): the
/// `ItemList` of the struct at `path`, which holds `items` now; or, where
/// `page` is given, that page's, made first (a store's page of a kind it
/// has none of yet; `path` is then the page's place).
pub(super) struct Bin {
    pub path: GffPath,
    pub items: Vec<Struct>,
    pub page: Option<NewPage>,
}

/// A page an added item makes: `page` (its `ItemList` left out), put at
/// `index` of list `list` of the struct at `at`.
pub(super) struct NewPage {
    pub at: GffPath,
    pub list: &'static str,
    pub index: usize,
    pub page: Struct,
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
        incoming: Incoming,
        edits: Vec<(&'static str, Edit)>,
    ) {
        let Incoming { adds, taken, pasted } = incoming;
        let what = adding_name("Add item", adds.len(), !taken.is_empty(), pasted);
        let here =
            |_: &mut Self, _: &Added| Bin { path: base.clone(), items: items.to_vec(), page: None };
        self.add_items(&what, &adds, taken, here, |_| {});
        for (what, edit) in edits {
            self.app.actions.push(Action::Apply(Command::new(what, vec![edit])));
        }
    }

    /// Items added to an inventory (picked, pasted or dropped), with
    /// `taken` (what took a moved one from where it was), as one step
    /// named `what`: each where `bin` puts it, at the end of that
    /// `ItemList`, at the first free place in its grid among the items
    /// there and those added before it; `adjust` gives each what its
    /// inventory adds (a store's `Infinite`). The placeable's, the
    /// creature's and the store's inventories add their items so.
    pub(super) fn add_items(
        &mut self,
        what: &str,
        adds: &[Added],
        taken: Vec<Edit>,
        mut bin: impl FnMut(&mut Self, &Added) -> Bin,
        adjust: impl Fn(&mut Struct),
    ) {
        if adds.is_empty() {
            return;
        }
        let key = self.key;
        // What each list holds as the step goes on, by its path; the pages
        // made in it (where, the page's id, the path it is at).
        let mut held: Vec<(GffPath, Vec<Struct>)> = Vec::new();
        let mut made: Vec<(GffPath, &'static str, u32, GffPath)> = Vec::new();
        let mut together = taken;
        for added in adds {
            let Bin { mut path, items, page } = bin(self, added);
            if let Some(p) = page {
                let id = p.page.id;
                let same = |m: &&(GffPath, &str, u32, GffPath)| m.0 == p.at && m.1 == p.list;
                match made.iter().filter(same).find(|m| m.2 == id) {
                    Some(m) => path = m.3.clone(),
                    None => {
                        let index = p.index + made.iter().filter(same).count();
                        path = p.at.item(p.list, index);
                        together.push(Edit::insert_item(key, p.at.clone(), p.list, index, p.page));
                        made.push((p.at, p.list, id, path.clone()));
                    }
                }
            }
            let at = match held.iter().position(|(p, _)| *p == path) {
                Some(i) => i,
                None => {
                    held.push((path.clone(), items));
                    held.len() - 1
                }
            };
            let mut item = self.inventory_item(&held[at].1, added);
            adjust(&mut item);
            let index = held[at].1.len();
            together.push(Edit::insert_item(key, path, "ItemList", index, item.clone()));
            held[at].1.push(item);
        }
        self.app.actions.push(Action::Apply(Command::new(what, together)));
    }

    /// Copy, Cut, Find in Palette and Properties from a row's menu, rows
    /// moved within the list, and what there is to add to it: the items
    /// pasted, one dragged from the palette, those dragged from another
    /// object's inventory (moved out of it, unless Ctrl is held: then
    /// copies; either way each item as it is there). What to add, and the
    /// edits that take the moved ones from where they were.
    pub(super) fn list_events(
        &mut self,
        ui: &Ui,
        items: &[Struct],
        out: &mut ListOut,
        whose: egui::Id,
        at: &GffPath,
    ) -> Incoming {
        let (mut adds, mut removals) = (Vec::new(), Vec::new());
        if !out.copy.is_empty() {
            let _ = self.copy_items(ui, &out.copy);
        }
        if !out.cut.is_empty() {
            let cut = std::mem::take(&mut out.cut);
            self.cut_items(ui, &(self.key, at.clone()), &cut);
        }
        match out.asked {
            Some((i, Ask::Open)) => {
                if let Some(entry) = items.get(i) {
                    self.item_properties(at, "ItemList", i, entry);
                }
            }
            Some((i, Ask::Find)) => {
                if let Some(entry) = items.get(i) {
                    self.find_in_palette(ui, BlueprintKind::Item, entry_resref(entry));
                }
            }
            _ => {}
        }
        if !out.moved.is_empty() {
            let moved = std::mem::take(&mut out.moved);
            let n = moved.len() / 2;
            let what = if n > 1 { format!("Move {n} items") } else { "Move item".into() };
            self.app.actions.push(Action::Apply(Command::new(what, moved)));
        }
        if out.paste {
            adds.extend(self.app.item_clip.iter().cloned().map(Added::Item));
        }
        match &out.dropped {
            Some(Dragged::Palette(r)) => adds.push(Added::Blueprint(*r)),
            Some(d) if d.owner() != Some(whose) => {
                adds.extend(d.carried().into_iter().map(|c| Added::Item(c.item)));
                removals.extend(self.taking(ui, d).into_iter().map(|(e, _)| e));
            }
            _ => {}
        }
        Incoming { adds, taken: removals, pasted: out.paste }
    }

    /// The edits that take the items dropped here from where they were
    /// dragged from, each with the item it takes: none with Ctrl held
    /// (copies), nor from the game's blueprint shown to look at (which
    /// nothing changes: copies too).
    pub(super) fn taking(&self, ui: &Ui, dragged: &Dragged) -> Vec<(Edit, Struct)> {
        if ui.input(|i| i.modifiers.command) {
            return Vec::new();
        }
        let viewed = |e: &Edit| self.app.ws.as_ref().is_some_and(|ws| ws.is_viewed(e.key()));
        dragged.removals().into_iter().filter(|(e, _)| !viewed(e)).collect()
    }

    /// Copies items as this inventory holds them, in their order, to paste
    /// into an inventory: pasted, each is carried as a drag carries it
    /// ([`carried`]: as it is into a placed object from one, made from its
    /// blueprint between a placed object and a blueprint). An entry that
    /// names no blueprint is left out.
    ///
    /// [`carried`]: Self::carried
    /// Returns those copied.
    pub(super) fn copy_items(&mut self, ui: &Ui, items: &[Carried]) -> Vec<Carried> {
        let named = items.iter().filter(|c| !entry_resref(&c.item).is_empty());
        let copied: Vec<Carried> = named.cloned().collect();
        if copied.is_empty() {
            return copied;
        }
        let n = copied.len();
        self.app.item_clip = copied.iter().map(|c| c.item.clone()).collect();
        let what = if n == 1 { "1 item".to_string() } else { format!("{n} items") };
        crate::widgets::mark_clipboard(ui.ctx(), &what);
        copied
    }

    /// Cut: `items` of the object at `source` copied ([`Form::copy_items`])
    /// and taken out of it, all in one step; one not copied stays.
    pub(super) fn cut_items(&mut self, ui: &Ui, source: &(ResKey, GffPath), items: &[Carried]) {
        let copied = self.copy_items(ui, items);
        let edits: Vec<Edit> = removals(source, &copied).into_iter().map(|(e, _)| e).collect();
        let n = edits.len();
        if n == 0 {
            return;
        }
        let what = if n > 1 { format!("Cut {n} items") } else { "Cut item".into() };
        self.app.actions.push(Action::Apply(Command::new(what, edits)));
    }
}

/// What an inventory's list brings in to add: the items (pasted,
/// dropped), the edits that take those moved from where they were, and
/// whether they were pasted.
#[derive(Default)]
pub(super) struct Incoming {
    pub adds: Vec<Added>,
    pub taken: Vec<Edit>,
    pub pasted: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Find in Palette's tip names no side: the page's item palette is
    /// at the left of the slots in a wide page, over them in a narrow one.
    #[test]
    fn find_in_palette_s_tip_names_no_side() {
        let tip = FIND_IN_PALETTE_TIP;
        assert!(!tip.contains("left") && !tip.contains("right"), "{tip}");
    }

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

    /// An item whose file has a place below 0 (an INT of -1) is at 0 for
    /// the next one's place: cast to an unsigned number it was at the far
    /// end of what the field holds, and its place and width overflowed
    /// when added (the review's B29). Placed as Advanced Search's Add an
    /// Item places one: by the one rule (`mg_edit::lists`).
    #[test]
    fn an_item_at_a_place_below_zero_is_at_the_grid_s_start() {
        let mut at_minus = item(&[]);
        at_minus.set("Repos_PosX", Value::Int(-1));
        at_minus.set("Repos_Posy", Value::Int(-7));
        assert_eq!(lists::placed_at(&at_minus), (0, 0));
        assert_eq!(lists::placed_at(&item(&[])), (0, 0), "left out");
        let (x, y) = lists::placed_at(&at_minus);
        assert_eq!(place(&[(x, y, 1, 1)], 1, 1), (1, 0), "beside it");
        // (The far end of what a field holds: no row wraps.)
        assert_eq!(place(&[(u32::MAX, u32::MAX, 2, 2)], 1, 1), (0, 0));
    }

    /// Several carried: the selection in its order (the slots', then the
    /// list's) where the row is in it, else the row alone; taken out from
    /// the last place to the first within each list; the step named for
    /// how many, a single item's as before.
    #[test]
    fn several_items_are_carried_in_their_order() {
        let c = |list, index| Carried { list, index, item: item(&[("Dropable", index as u8)]) };
        let selected = [c("ItemList", 4), c("Equip_ItemList", 3), c("ItemList", 1)];
        let at = |all: &[Carried]| all.iter().map(|c| (c.list, c.index)).collect::<Vec<_>>();
        let all = carried_with(c("ItemList", 1), &selected);
        assert_eq!(at(&all), [("Equip_ItemList", 3), ("ItemList", 1), ("ItemList", 4)]);
        assert_eq!(at(&carried_with(c("ItemList", 2), &selected)), [("ItemList", 2)]);
        let key = ResKey::parse("bandit", mg_core::ResType::UTC).unwrap();
        let source = (key, GffPath::root());
        let taken: Vec<_> = removals(&source, &all)
            .into_iter()
            .map(|(e, it)| match e {
                Edit::RemoveItem { list, index, .. } => (list, index, it == c("x", index).item),
                other => panic!("{other:?}"),
            })
            .collect();
        let want = [("Equip_ItemList", 3, true), ("ItemList", 4, true), ("ItemList", 1, true)];
        assert_eq!(taken, want.map(|(l, i, s)| (l.to_string(), i, s)));
        // The drag that began on the list's row 4 carries the three.
        let d = Dragged::Listed {
            owner: egui::Id::new("o"),
            source,
            index: 4,
            resref: ResRef::EMPTY,
            item: c("ItemList", 4).item,
            more: vec![c("Equip_ItemList", 3), c("ItemList", 1)],
        };
        assert_eq!((d.count(), d.lead()), (3, Some(("ItemList", 4))));
        assert_eq!(at(&d.carried()), at(&all));
        // Of its own list, not the list's rows alone: not a move within it.
        assert_eq!(own_rows(&d, egui::Id::new("o")), None);
        assert_eq!(own_rows(&d, egui::Id::new("other")), None);
        // Names.
        assert_eq!(adding_name("Add item", 1, false, true), "Add item");
        assert_eq!(adding_name("Add store item", 1, true, false), "Move item");
        assert_eq!(adding_name("Add item", 3, true, false), "Move 3 items");
        assert_eq!(adding_name("Add item", 2, false, true), "Paste 2 items");
        assert_eq!(adding_name("Add store item", 2, false, false), "Add 2 items");
    }

    /// (The grid's rule, at the inventory's width.)
    fn place(taken: &[(u32, u32, u32, u32)], w: u32, h: u32) -> (u32, u32) {
        lists::place(taken, GRID_WIDTH, w, h)
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
