//! The inventory of several placeables, creatures or stores edited
//! together (`docs/lists-together-proposal.md`): the item blueprints any
//! of them holds, ≠ with how many where some do. An item added (picked,
//! pasted or dropped from the palette) goes to each, at its own first free
//! place (a store's on the page of its kind); one removed is taken from
//! each, every one of it; its flags are set on each one's. A creature's
//! equipment shows each slot's item where all wear the same, ≠ where they
//! differ; an item equipped in a slot is worn by each, what each wore
//! there going to its backpack.

use egui::Ui;
use mg_core::ResRef;
use mg_edit::{Edit, GffPath, lists};
use mg_gff::{Struct, Value};
use mg_module::palette::BlueprintKind;

use super::Form;
use super::creature_lists::{ITEM_FLAGS, SLOTS, equip_edits, item_slots};
use super::inventory::{self, Added};

/// What holds the items.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Holder {
    Placeable,
    Creature,
    Store,
}

/// The places a target keeps its items in: its `ItemList` (a creature's
/// backpack), or each of a store's pages'; each the path of the struct
/// the list is in, and the items.
fn bags(t: &lists::Target, holder: Holder) -> Vec<(GffPath, &[Struct])> {
    match holder {
        Holder::Store => (t.list("StoreList").iter().enumerate())
            .map(|(i, p)| (t.path.clone().item("StoreList", i), p.list("ItemList").unwrap_or(&[])))
            .collect(),
        _ => vec![(t.path.clone(), t.list("ItemList"))],
    }
}

/// The items the targets hold, counted once for a page's rows: how many
/// of each blueprint each target holds, and every target's items of each
/// (for the flags they have).
struct Holdings<'a> {
    counts: Vec<std::collections::HashMap<ResRef, usize>>,
    of: std::collections::HashMap<ResRef, Vec<&'a Struct>>,
}

impl<'a> Holdings<'a> {
    /// Of each target's items (`items`, a target's bags in one).
    fn of(items: &[Vec<&'a Struct>]) -> Holdings<'a> {
        let mut h = Holdings { counts: Vec::with_capacity(items.len()), of: Default::default() };
        for held in items {
            let mut counts = std::collections::HashMap::new();
            for &e in held {
                let Some(r) = lists::item_of(e) else { continue };
                *counts.entry(r).or_insert(0) += 1;
                h.of.entry(r).or_default().push(e);
            }
            h.counts.push(counts);
        }
        h
    }

    /// How many items of blueprint `r` each target holds.
    fn held(&self, r: ResRef) -> Vec<usize> {
        self.counts.iter().map(|c| c.get(&r).copied().unwrap_or(0)).collect()
    }

    /// Every target's items of blueprint `r`.
    fn items(&self, r: ResRef) -> &[&'a Struct] {
        self.of.get(&r).map_or(&[], Vec::as_slice)
    }
}

/// Runs `with` on the form as if it edited target `t` alone (the edits a
/// page builds for its own object, made for each), and puts it back.
fn as_target<R>(f: &mut Form<'_>, t: &lists::Target, with: impl FnOnce(&mut Form<'_>) -> R) -> R {
    let was = (f.key, std::mem::replace(&mut f.path, t.path.clone()));
    let root = std::mem::replace(&mut f.root, t.at.clone());
    f.key = t.key;
    let out = with(f);
    f.key = was.0;
    f.path = was.1;
    f.root = root;
    out
}

/// Why target `t` takes no item: a placeable without an inventory
/// (Basic › Has Inventory).
fn takes_none(t: &lists::Target, holder: Holder) -> Option<&'static str> {
    let none = holder == Holder::Placeable && t.at.integer("HasInventory").unwrap_or(0) == 0;
    none.then_some("no inventory (Basic › Has Inventory)")
}

/// The edits that add `added` to target `t`: at the end of its
/// `ItemList`, at the first free place; a store's on the page of its kind
/// (made where it has none of that page yet).
fn adding(f: &mut Form<'_>, t: &lists::Target, holder: Holder, added: &Added) -> Vec<Edit> {
    if holder != Holder::Store {
        let items = t.list("ItemList").to_vec();
        let item = f.inventory_item(&items, added);
        return vec![Edit::insert_item(t.key, t.path.clone(), "ItemList", items.len(), item)];
    }
    let fit = f.added_fit(added);
    let id = fit.panel.and_then(|p| super::store::PANEL_PAGE.get(p as usize).copied()).unwrap_or(1);
    let pages = t.list("StoreList");
    let mut edits = Vec::new();
    let (path, items) = match pages.iter().position(|p| p.id == id) {
        Some(i) => {
            (t.path.clone().item("StoreList", i), pages[i].list("ItemList").unwrap_or(&[]).to_vec())
        }
        None => {
            let at = pages.len();
            edits.push(Edit::insert_item(t.key, t.path.clone(), "StoreList", at, Struct::new(id)));
            (t.path.clone().item("StoreList", at), Vec::new())
        }
    };
    let mut item = f.inventory_item(&items, added);
    if !item.contains("Infinite") {
        item.set("Infinite", Value::Byte(0));
    }
    edits.push(Edit::insert_item(t.key, path, "ItemList", items.len(), item));
    edits
}

/// The inventory page of several objects edited together.
pub(super) fn page(f: &mut Form<'_>, ui: &mut Ui, holder: Holder) {
    let key = f.key;
    let targets = f.list_targets();
    let total = targets.len();
    let names = f.blueprint_names(BlueprintKind::Item);
    let name_of = |r: ResRef| names.get(&r).cloned().unwrap_or_else(|| r.to_string());
    // Every item blueprint any of them holds, in the first one's order.
    let lists_of: Vec<Vec<&Struct>> = targets
        .iter()
        .map(|t| bags(t, holder).into_iter().flat_map(|(_, items)| items).collect())
        .collect();
    let merged = lists::merge_refs(&lists_of, lists::item_of);
    let holdings = Holdings::of(&lists_of);
    let flags: &[(&'static str, &str)] = match holder {
        Holder::Creature => &ITEM_FLAGS,
        Holder::Store => &[("Infinite", "The store never runs out of it")],
        Holder::Placeable => &[],
    };
    let mut adds: Vec<Added> = Vec::new();
    let mut change: Option<(&str, lists::Outcome, &str)> = None;
    let mut equip: Option<(u32, ResRef)> = None;
    let mut slot_out: Option<(u32, bool)> = None;
    let mut chosen = None;
    crate::widgets::two_columns(ui, 300.0, |ui, col| {
        if col == 0 {
            if let Some(r) = f.item_picker(ui, "Add Item") {
                adds.push(Added::Blueprint(r));
            }
            chosen = f.palette_chosen(ui, BlueprintKind::Item);
            let copied = f.app.item_clip.len();
            if ui
                .add_enabled(copied > 0, egui::Button::new("Paste"))
                .on_hover_text("Add the items copied to each")
                .clicked()
            {
                adds.extend(f.app.item_clip.iter().cloned().map(Added::Item));
            }
            return;
        }
        if holder == Holder::Creature {
            crate::widgets::section_heading(ui, "Equipment");
            equipment(f, ui, &targets, &names, chosen, &mut equip, &mut slot_out);
            ui.add_space(crate::widgets::SECTION_GAP);
            crate::widgets::section_heading(ui, "Backpack");
        }
        if merged.is_empty() {
            ui.weak("None of them holds an item.");
        }
        egui::Grid::new(("inventory-together", key))
            .num_columns(3 + flags.len())
            .striped(true)
            .show(ui, |ui| {
                ui.strong("Item");
                ui.strong("Held");
                for (flag, tip) in flags {
                    ui.strong(*flag).on_hover_text(*tip);
                }
                ui.label("");
                ui.end_row();
                for m in &merged {
                    let r = m.key;
                    ui.horizontal(|ui| {
                        ui.label(name_of(r)).on_hover_text(r.to_string());
                        super::entry_mark(ui, m.having.len(), total);
                    });
                    let counts = holdings.held(r);
                    let first = counts[m.having[0]];
                    ui.horizontal(|ui| {
                        if counts.iter().any(|c| *c != first && *c > 0) {
                            ui.colored_label(ui.visuals().warn_fg_color, super::MIXED)
                                .on_hover_text(
                                    "They hold other numbers of it: this is the first's",
                                );
                        }
                        ui.label(format!("×{first}"));
                    });
                    for (flag, tip) in flags {
                        let of_it = holdings.items(r);
                        let (on, mixed) = inventory::flag_state(of_it.iter().copied(), flag);
                        let mut v = on;
                        let r_box = ui
                            .add(mg_uikit::Checkbox::new(&mut v, "").indeterminate(mixed))
                            .on_hover_text(*tip);
                        if r_box.changed() {
                            let v = v || mixed;
                            change = Some((
                                "Item flag",
                                set_flag(&targets, holder, r, flag, v),
                                "Set on",
                            ));
                        }
                    }
                    ui.horizontal(|ui| {
                        if ui
                            .small_button("Remove")
                            .on_hover_text("Take every one of it from each")
                            .clicked()
                        {
                            change =
                                Some(("Remove Item", removing(&targets, holder, r), "Taken from"));
                        }
                        if m.having.len() < total
                            && ui
                                .small_button("Give to All")
                                .on_hover_text(super::creature_lists::GIVE_TIP)
                                .clicked()
                        {
                            let mut out = lists::Outcome::default();
                            for (t, c) in targets.iter().zip(&counts) {
                                if *c > 0 {
                                    continue;
                                }
                                if let Some(why) = takes_none(t, holder) {
                                    out.left.push(format!("{}: {why}", t.name));
                                    continue;
                                }
                                out.edits.extend(adding(f, t, holder, &Added::Blueprint(r)));
                                out.changed += 1;
                            }
                            change = Some(("Add item", out, "Added to"));
                        }
                    });
                    ui.end_row();
                }
            });
        // Dropped from the palette on the page: added to each.
        let rect = ui.min_rect();
        if let Some(inventory::Dragged::Palette(r)) = inventory::dropped_on(ui, rect) {
            adds.push(Added::Blueprint(r));
        }
    });
    if !adds.is_empty() {
        let mut out = lists::Outcome::default();
        for t in targets.iter() {
            if let Some(why) = takes_none(t, holder) {
                out.left.push(format!("{}: {why}", t.name));
                continue;
            }
            // (Each added after those before it, at its own free place.)
            let mut t = t.clone();
            for added in &adds {
                let edits = adding(f, &t, holder, added);
                apply_to(&mut t, &edits);
                out.edits.extend(edits);
            }
            out.changed += 1;
        }
        change = Some(("Add item", out, "Added to"));
    }
    if let Some((bit, r)) = equip {
        let mut out = lists::Outcome::default();
        for t in targets.iter() {
            if item_slots(f, r, None) & bit == 0 {
                out.left.push(format!("{}: it doesn't go in that slot", t.name));
                continue;
            }
            let equipped = t.list("Equip_ItemList").to_vec();
            let backpack = t.list("ItemList").to_vec();
            let edits = as_target(f, t, |f| equip_edits(f, (&equipped, &backpack), bit, r, None));
            out.edits.extend(edits);
            out.changed += 1;
        }
        change = Some(("Equip", out, "Equipped on"));
    }
    if let Some((bit, stow)) = slot_out {
        let mut out = lists::Outcome::default();
        for t in targets.iter() {
            let equipped = t.list("Equip_ItemList");
            let Some(i) = equipped.iter().position(|s| s.id == bit) else { continue };
            out.edits.push(Edit::remove_item(t.key, t.path.clone(), "Equip_ItemList", i));
            if stow {
                let backpack = t.list("ItemList").to_vec();
                let item = equipped[i].clone();
                let item = as_target(f, t, |f| f.inventory_item(&backpack, &Added::Item(item)));
                out.edits.push(Edit::insert_item(
                    t.key,
                    t.path.clone(),
                    "ItemList",
                    backpack.len(),
                    item,
                ));
            }
            out.changed += 1;
        }
        let what = if stow { "Unequip to backpack" } else { "Unequip" };
        change = Some((what, out, "Done on"));
    }
    if let Some((what, out, done)) = change {
        f.apply_each(what, out, done);
    }
}

/// `t` as `edits` (inserts into its own lists) leave it, for the next
/// item added to it to find its place among them.
fn apply_to(t: &mut lists::Target, edits: &[Edit]) {
    for e in edits {
        let Edit::InsertItem { path, list, index, item, .. } = e else { continue };
        let rel = GffPath(path.0[t.path.0.len().min(path.0.len())..].to_vec());
        let Some(s) = rel.get_mut(&mut t.at) else { continue };
        if s.get(list).is_none() {
            s.set(list, Value::List(Vec::new()));
        }
        if let Some(items) = s.list_mut(list)
            && *index <= items.len()
        {
            items.insert(*index, item.clone());
        }
    }
}

/// Every item of blueprint `r` taken from each target.
fn removing(targets: &[lists::Target], holder: Holder, r: ResRef) -> lists::Outcome {
    let mut out = lists::Outcome::default();
    for t in targets {
        let mut any = false;
        for (path, items) in bags(t, holder) {
            for (i, _) in
                items.iter().enumerate().rev().filter(|(_, e)| lists::item_of(e) == Some(r))
            {
                out.edits.push(Edit::remove_item(t.key, path.clone(), "ItemList", i));
                any = true;
            }
        }
        out.changed += usize::from(any);
    }
    out
}

/// Flag `flag` set (or cleared) on every item of blueprint `r` each
/// target holds: a creature's flags written as 1 or left out, as Aurora
/// writes them; a store's Infinite as a byte.
fn set_flag(
    targets: &[lists::Target],
    holder: Holder,
    r: ResRef,
    flag: &'static str,
    on: bool,
) -> lists::Outcome {
    let mut out = lists::Outcome::default();
    for t in targets {
        let mut any = false;
        for (path, items) in bags(t, holder) {
            for (i, e) in items.iter().enumerate().filter(|(_, e)| lists::item_of(e) == Some(r)) {
                let at = path.clone().item("ItemList", i);
                let edit = if holder == Holder::Store {
                    let now = e.integer(flag).unwrap_or(0) != 0;
                    (now != on).then(|| Edit::SetField {
                        key: t.key,
                        path: at,
                        label: flag.into(),
                        value: Some(Value::Byte(u8::from(on))),
                    })
                } else {
                    inventory::flag_edit(t.key, at, e, flag, on)
                };
                if let Some(edit) = edit {
                    out.edits.push(edit);
                    any = true;
                }
            }
        }
        out.changed += usize::from(any);
    }
    out
}

/// A creature's equipment slots, of several creatures: the item all of
/// them wear there, or ≠ with what the first wears; Equip (the palette's
/// item chosen, `chosen`), To Backpack and Remove for each.
fn equipment(
    f: &mut Form<'_>,
    ui: &mut Ui,
    targets: &[lists::Target],
    names: &std::collections::HashMap<ResRef, String>,
    chosen: Option<ResRef>,
    equip: &mut Option<(u32, ResRef)>,
    slot_out: &mut Option<(u32, bool)>,
) {
    let name_of = |r: ResRef| names.get(&r).cloned().unwrap_or_else(|| r.to_string());
    let fits = chosen.map_or(0, |r| item_slots(f, r, None));
    egui::Grid::new(("equipment-together", f.key)).num_columns(3).striped(true).show(ui, |ui| {
        for (bit, slot) in SLOTS {
            let worn: Vec<Option<ResRef>> = targets
                .iter()
                .map(|t| {
                    let e = t.list("Equip_ItemList").iter().find(|s| s.id == bit)?;
                    lists::item_of(e)
                })
                .collect();
            ui.label(slot);
            ui.horizontal(|ui| {
                if worn.iter().any(|w| *w != worn[0]) {
                    let n = worn.iter().filter(|w| w.is_some()).count();
                    let tip =
                        format!("{n} of the {} wear something here, not all the same", worn.len());
                    ui.colored_label(ui.visuals().warn_fg_color, super::MIXED).on_hover_text(tip);
                }
                match worn[0] {
                    Some(r) => ui.label(name_of(r)),
                    None => ui.weak("(none)"),
                };
            });
            ui.horizontal(|ui| {
                let can = chosen.is_some() && fits & bit != 0;
                if ui
                    .add_enabled(can, egui::Button::new("Equip").small())
                    .on_hover_text("Equip the item chosen in the palette here, on each")
                    .clicked()
                    && let Some(r) = chosen
                {
                    *equip = Some((bit, r));
                }
                let any = worn.iter().any(Option::is_some);
                if ui.add_enabled(any, egui::Button::new("To Backpack").small()).clicked() {
                    *slot_out = Some((bit, true));
                }
                if ui.add_enabled(any, egui::Button::new("Remove").small()).clicked() {
                    *slot_out = Some((bit, false));
                }
            });
            ui.end_row();
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use mg_core::ResType;
    use mg_resman::ResKey;

    fn item(resref: &str, flag: u8) -> Struct {
        let mut s = Struct::new(0);
        s.set("InventoryRes", Value::resref(ResRef::from_str(resref).unwrap()));
        s.set("Dropable", Value::Byte(flag));
        s
    }

    /// The items counted once for the page (the review's B20: each row
    /// copied every target's whole list, and walked every item of every
    /// target for each flag) say what counting them a row at a time said.
    #[test]
    fn holdings_counted_once_are_those_counted_a_row_at_a_time() {
        let key = ResKey::parse("area", ResType::GIT).unwrap();
        let creature = |i: usize, items: Vec<Struct>| {
            let mut s = Struct::new(0);
            s.set("ItemList", Value::List(items));
            lists::Target::new(key, GffPath::root().item("Creature List", i), s)
        };
        let targets = [
            creature(0, vec![item("sword", 1), item("potion", 0), item("potion", 1)]),
            creature(1, vec![item("potion", 0)]),
            creature(2, Vec::new()),
        ];
        let lists_of: Vec<Vec<&Struct>> = (targets.iter())
            .map(|t| bags(t, Holder::Creature).into_iter().flat_map(|(_, items)| items).collect())
            .collect();
        let holdings = Holdings::of(&lists_of);
        let merged = lists::merge_refs(&lists_of, lists::item_of);
        let slices: Vec<&[Struct]> = targets.iter().map(|t| t.list("ItemList")).collect();
        assert_eq!(merged, lists::merge(&slices, lists::item_of));
        for m in &merged {
            let r = m.key;
            let a_row_at_a_time: Vec<usize> = (targets.iter())
                .map(|t| t.list("ItemList").iter().filter(|e| lists::item_of(e) == Some(r)).count())
                .collect();
            assert_eq!(holdings.held(r), a_row_at_a_time, "{r}");
            let of_it: Vec<&Struct> = lists_of
                .iter()
                .flatten()
                .copied()
                .filter(|e| lists::item_of(e) == Some(r))
                .collect();
            assert_eq!(holdings.items(r), of_it, "{r}");
        }
        let potion = ResRef::from_str("potion").unwrap();
        assert_eq!(holdings.held(potion), [2, 1, 0]);
        assert_eq!(holdings.held(ResRef::from_str("none").unwrap()), [0, 0, 0]);
        // A store's pages: each its own bag, at its own path.
        let mut store = Struct::new(0);
        let page = |id: u32, items: Vec<Struct>| {
            let mut p = Struct::new(id);
            p.set("ItemList", Value::List(items));
            p
        };
        store.set(
            "StoreList",
            Value::List(vec![page(0, vec![item("sword", 0)]), page(4, vec![item("potion", 0)])]),
        );
        let t = lists::Target::new(key, GffPath::root().item("StoreList", 0), store);
        let bags = bags(&t, Holder::Store);
        assert_eq!(bags.len(), 2);
        assert_eq!(bags[1].0, t.path.clone().item("StoreList", 1));
        assert_eq!(bags[1].1.len(), 1);
    }
}
