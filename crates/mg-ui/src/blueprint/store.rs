//! Store Properties (`TdlgStoreEdit`): Basic (name, pricing, stolen
//! goods, gold), Advanced, Restrictions (base items the store will not, or
//! will only, buy), Comments; and the store's inventory (Aurora's
//! Inventory… dialog) as its own page.

use egui::Ui;
use mg_edit::{Command, Edit};
use mg_gff::{FieldType, Struct, Value};
use mg_module::palette::BlueprintKind;
use mg_rules::Choice;

use super::inventory::{self, Added, Bin, NewPage};
use super::{Form, situated};
use crate::Action;
use crate::widgets::commit_number;

pub(super) const PAGES: [&str; 5] = ["Basic", "Inventory", "Advanced", "Restrictions", "Comments"];

/// The store's inventory pages as Aurora writes them: (`StoreList` struct
/// id, name).
pub(crate) use mg_rules::rows::STORE_PAGES;

/// The `StoreList` struct id of each baseitems.2da `StorePanel`.
const PANEL_PAGE: [u32; 5] = [0, 4, 2, 3, 1];

/// The struct id of `WillNotBuy` and `WillOnlyBuy` items.
const BASE_ITEM_ID: u32 = mg_schema::utm::WILL_NOT_BUY.item_id;

pub(super) fn page(f: &mut Form<'_>, ui: &mut Ui, page: &str) {
    match page {
        "Basic" => basic(f, ui),
        "Inventory" => inventory(f, ui),
        "Advanced" => advanced(f, ui),
        "Restrictions" => restrictions(f, ui),
        _ => f.memo(ui, "Comments", "Comment"),
    }
}

/// A price that −1 turns off: a checkbox and the amount. `default` is the
/// value when the field is missing, `on` the amount a check sets.
fn optional_price(
    f: &mut Form<'_>,
    ui: &mut Ui,
    check: &str,
    label: &str,
    what: &str,
    default: i64,
    on: i64,
) {
    let current = f.root.integer(label).unwrap_or(default);
    let mut enabled = current != -1;
    ui.horizontal(|ui| {
        f.mark_mixed_inline(ui, &[label]);
        if ui.checkbox(&mut enabled, check).changed() {
            f.set_int(check, label, if enabled { on } else { -1 }, FieldType::Int);
        }
        ui.add_enabled_ui(current != -1, |ui| {
            if let Some(v) = commit_number(ui, current.max(0), 0..=999_999_999) {
                f.set_int(what, label, v, FieldType::Int);
            }
        });
    });
}

fn basic(f: &mut Form<'_>, ui: &mut Ui) {
    egui::Grid::new(("utm-basic", f.key)).num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
        crate::widgets::field_label(ui, "Name");
        f.locstring(ui, "Name", "LocName");
        ui.end_row();
        crate::widgets::field_label(ui, "Tag");
        f.text(ui, "Tag", "Tag", 32);
        ui.end_row();
        crate::widgets::field_label(ui, "Category");
        f.category(ui, BlueprintKind::Store);
        ui.end_row();
        crate::widgets::field_label(ui, "");
        if ui.button("Inventory…").on_hover_text("Edit inventory contents").clicked() {
            f.app.blueprint_pages.insert((f.key, f.path.clone()), "Inventory");
        }
        ui.end_row();
    });
    ui.add_space(crate::widgets::SECTION_GAP);
    crate::widgets::section_heading(ui, "Pricing");
    egui::Grid::new(("utm-pricing", f.key)).num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
        crate::widgets::field_label(ui, "Sell Mark Up (%)");
        f.spin(ui, "Sell mark up", "MarkUp", 1..=1000);
        ui.end_row();
        crate::widgets::field_label(ui, "Buy Mark Down (%)");
        f.spin(ui, "Buy mark down", "MarkDown", 0..=100);
        ui.end_row();
        crate::widgets::field_label(ui, "Identify Price");
        optional_price(f, ui, "Will Identify Items", "IdentifyPrice", "Identify price", 100, 100);
        ui.end_row();
    });
    ui.add_space(crate::widgets::SECTION_GAP);
    crate::widgets::section_heading(ui, "Stolen Goods");
    let black_market = f.check(ui, "Buy Stolen Goods", "BlackMarket");
    ui.horizontal(|ui| {
        ui.add_enabled_ui(black_market, |ui| {
            crate::widgets::field_label(ui, "Buy Mark Down (%)");
            f.spin(ui, "Stolen goods mark down", "BM_MarkDown", 0..=100);
        });
    });
    ui.add_space(crate::widgets::SECTION_GAP);
    crate::widgets::section_heading(ui, "Restrictions");
    egui::Grid::new(("utm-limits", f.key)).num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
        crate::widgets::field_label(ui, "Max Buy Price");
        optional_price(f, ui, "Has Maximum Buy Price", "MaxBuyPrice", "Max buy price", -1, 100);
        ui.end_row();
        crate::widgets::field_label(ui, "Gold Amount");
        optional_price(f, ui, "Has Limited Gold", "StoreGold", "Store gold", -1, 1000);
        ui.end_row();
    });
}

fn advanced(f: &mut Form<'_>, ui: &mut Ui) {
    egui::Grid::new(("utm-advanced", f.key)).num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
        crate::widgets::field_label(ui, "Blueprint ResRef");
        f.blueprint_resref(ui);
        ui.end_row();
        crate::widgets::field_label(ui, "Variables");
        f.variables(ui);
        ui.end_row();
    });
    f.update_instances(ui);
    ui.separator();
    situated::scripts(f, ui, &[("OnOpenStore", "OnOpenStore"), ("OnStoreClosed", "OnStoreClosed")]);
}

/// Adds items, each to the page its base item belongs on (made where the
/// store has none of that page yet), at the first free place, with
/// `taken` (what took a moved one from where it was) as one step; the
/// struct id of the last one's page.
fn add_items(f: &mut Form<'_>, adds: &[Added], taken: Vec<Edit>) -> Option<u32> {
    let base = f.path.clone();
    let pages = f.root.list("StoreList").unwrap_or(&[]).to_vec();
    let mut last = None;
    let bin = |f: &mut Form<'_>, added: &Added| {
        let fit = f.added_fit(added);
        let page = fit.panel.and_then(|p| PANEL_PAGE.get(p as usize).copied()).unwrap_or(1);
        last = Some(page);
        match pages.iter().position(|p| p.id == page) {
            Some(i) => Bin {
                path: base.item("StoreList", i),
                items: pages[i].list("ItemList").unwrap_or(&[]).to_vec(),
                page: None,
            },
            None => Bin {
                path: base.clone(),
                items: Vec::new(),
                page: Some(NewPage {
                    at: base.clone(),
                    list: "StoreList",
                    index: pages.len(),
                    page: Struct::new(page),
                }),
            },
        }
    };
    // (A store's item moved from another keeps its own.)
    let infinite = |item: &mut Struct| {
        if !item.contains("Infinite") {
            item.set("Infinite", Value::Byte(0));
        }
    };
    let what = if taken.is_empty() { "Add store item" } else { "Move item" };
    f.add_items(what, adds, taken, bin, infinite);
    last
}

fn inventory(f: &mut Form<'_>, ui: &mut Ui) {
    let base = f.path.clone();
    let key = f.key;
    let pages = f.root.list("StoreList").unwrap_or(&[]).to_vec();
    let names = f.blueprint_names(BlueprintKind::Item);
    let shown_id = egui::Id::new(("utm-page", key));
    let first = STORE_PAGES
        .iter()
        .map(|(id, _)| *id)
        .find(|id| {
            pages.iter().any(|p| p.id == *id && p.list("ItemList").is_some_and(|l| !l.is_empty()))
        })
        .unwrap_or(0);
    let mut shown: u32 = ui.data(|d| d.get_temp(shown_id)).unwrap_or(first);
    let mut add = None;
    let (mut adds, mut taken) = (Vec::new(), Vec::new());
    let mut edits = Vec::new();
    crate::widgets::two_columns(ui, 300.0, |ui, col| {
        if col == 0 {
            add = f.item_picker(ui, "Add Item");
            return;
        }
        ui.horizontal_wrapped(|ui| {
            for (id, name) in STORE_PAGES {
                let count = pages
                    .iter()
                    .find(|p| p.id == id)
                    .and_then(|p| p.list("ItemList"))
                    .map_or(0, <[Struct]>::len);
                ui.selectable_value(&mut shown, id, format!("{name} ({count})"));
            }
        });
        let Some(index) = pages.iter().position(|p| p.id == shown) else {
            ui.weak("No items on this page.");
            return;
        };
        let path = base.clone().item("StoreList", index);
        let items = pages[index].list("ItemList").unwrap_or(&[]);
        let items = items.to_vec();
        // What the store sells each for and buys it for: its cost at the
        // store's mark up and mark down, as Aurora shows them (a buyer's
        // Appraise is the game's to add).
        let mark_up = f.root.integer("MarkUp").unwrap_or(100).max(0) as u64;
        let mark_down = f.root.integer("MarkDown").unwrap_or(0).max(0) as u64;
        let copied = f.app.item_clip.len();
        let ctx = ui.ctx().clone();
        let mut row = |_: usize, it: &Struct| {
            let icon = f.entry_icon(&ctx, it);
            let name = f.entry_name(it, &names);
            let name = match f.entry_cost(it) {
                Some(cost) => {
                    let (sells, buys) = (price(cost, mark_up), price(cost, mark_down));
                    format!("{name}  ·  {sells} gp, buys at {buys} gp")
                }
                None => name,
            };
            (icon, name)
        };
        let look = inventory::ItemLook {
            row: &mut row,
            infinite: true,
            selected: &[],
            equip: false,
            copied,
        };
        let mut out = inventory::item_list(ui, key, &path, &items, look);
        // Pasted, dragged from the palette, or dragged out of another
        // object's inventory: added (to the page its kind goes on).
        let whose = inventory::owner(key, &path);
        (adds, taken) = f.list_events(ui, &items, &mut out, whose, &path);
        edits = out.edits;
    });
    // (A moved item's removal goes with it, as one step; the page shown,
    // the last one's.)
    adds.extend(add.map(Added::Blueprint));
    if let Some(page) = add_items(f, &adds, taken) {
        shown = page;
    }
    for (what, edit) in edits {
        f.app.actions.push(Action::Apply(Command::new(what, vec![edit])));
    }
    ui.data_mut(|d| d.insert_temp(shown_id, shown));
}

/// An item's price at a store's mark up or mark down (a percentage of its
/// cost): Aurora's Inventory shows the cost so, rounded down and never
/// less than 1 gp (an item of no cost sells and buys at 1).
pub(crate) fn price(cost: u32, percent: u64) -> u64 {
    (u64::from(cost) * percent / 100).max(1)
}

/// The base items a store can list (those with a store panel), by name.
fn base_items(f: &Form<'_>) -> Vec<Choice> {
    let Some(game) = f.app.game.as_deref() else { return Vec::new() };
    let Ok(table) = game.table("baseitems") else { return Vec::new() };
    let named = f.choices("baseitems", Some("Name"), Some("label"));
    let items = (0..table.len())
        .filter(|&row| table.get_int(row, "StorePanel").is_some())
        .map(|row| Choice { row, text: mg_rules::choice_text(&named, row as i64) });
    mg_rules::by_name(items.collect())
}

fn restrictions(f: &mut Form<'_>, ui: &mut Ui) {
    let base = f.path.clone();
    let key = f.key;
    let list = |label: &str| -> Vec<i64> {
        f.root.list(label).unwrap_or(&[]).iter().filter_map(|s| s.integer("BaseItem")).collect()
    };
    let (not_buy, only_buy) = (list("WillNotBuy"), list("WillOnlyBuy"));
    // The mode is the list in use; with both empty, the one last chosen.
    let state_id = egui::Id::new(("utm-restrict", key));
    let (mut only, mut filter, mut left, mut right): (bool, String, Option<i64>, Option<i64>) =
        ui.data(|d| d.get_temp(state_id)).unwrap_or_default();
    if !only_buy.is_empty() {
        only = true;
    } else if !not_buy.is_empty() {
        only = false;
    }
    let (label, current) = if only { ("WillOnlyBuy", only_buy) } else { ("WillNotBuy", not_buy) };
    let entries = |rows: &[i64]| -> Value {
        Value::List(
            rows.iter()
                .map(|&r| {
                    let mut s = Struct::new(BASE_ITEM_ID);
                    s.set("BaseItem", Value::Int(r as i32));
                    s
                })
                .collect(),
        )
    };
    let mut set: Option<(&str, Vec<(&str, Value)>)> = None;
    for (text, mode) in [
        ("Store will NOT buy the following items", false),
        ("Store will ONLY buy the following items", true),
    ] {
        if ui.radio(only == mode, text).clicked() && only != mode {
            only = mode;
            if !current.is_empty() {
                // The items move to the other list.
                let (to, from) = if mode {
                    ("WillOnlyBuy", "WillNotBuy")
                } else {
                    ("WillNotBuy", "WillOnlyBuy")
                };
                set =
                    Some(("Restriction mode", vec![(to, entries(&current)), (from, entries(&[]))]));
            }
        }
    }
    ui.separator();
    let items = base_items(f);
    let name = |row: i64| mg_rules::choice_text(&items, row);
    let mut add = None;
    let mut remove = None;
    ui.horizontal_top(|ui| {
        ui.vertical(|ui| {
            ui.set_width(240.0);
            ui.label("Base Items");
            crate::widgets::SearchField::new(egui::Id::new(("utm-bases-find", key)), &mut filter)
                .hint("Find")
                .show(ui);
            let needle = mg_core::text::Needle::new(&filter);
            egui::ScrollArea::vertical().id_salt(("utm-bases", key)).max_height(360.0).show(
                ui,
                |ui| {
                    for c in &items {
                        let row = c.row as i64;
                        if current.contains(&row) || !needle.finds(&c.text) {
                            continue;
                        }
                        let r = ui.selectable_label(left == Some(row), &c.text);
                        if r.clicked() {
                            left = Some(row);
                        }
                        if r.double_clicked() {
                            add = Some(row);
                        }
                    }
                },
            );
        });
        ui.vertical(|ui| {
            ui.add_space(48.0);
            let button = |text| egui::Button::new(text).min_size(egui::vec2(90.0, 0.0));
            if ui.add_enabled(left.is_some(), button("Add")).clicked() {
                add = left;
            }
            if ui.add_enabled(right.is_some(), button("Remove")).clicked() {
                remove = right;
            }
            if ui.add_enabled(!current.is_empty(), button("Remove All")).clicked() {
                set = Some(("Clear restricted items", vec![(label, entries(&[]))]));
            }
        });
        ui.vertical(|ui| {
            ui.set_width(240.0);
            ui.label("Restricted Items");
            egui::ScrollArea::vertical().id_salt(("utm-restricted", key)).max_height(390.0).show(
                ui,
                |ui| {
                    for row in &current {
                        let r = ui.selectable_label(right == Some(*row), name(*row));
                        if r.clicked() {
                            right = Some(*row);
                        }
                        if r.double_clicked() {
                            remove = Some(*row);
                        }
                    }
                },
            );
        });
    });
    if let Some(row) = add.filter(|r| !current.contains(r)) {
        let mut s = Struct::new(BASE_ITEM_ID);
        s.set("BaseItem", Value::Int(row as i32));
        f.app.actions.push(Action::Apply(Command::new(
            "Add restricted item",
            vec![Edit::insert_item(key, base.clone(), label, current.len(), s)],
        )));
        left = None;
    }
    if let Some(i) = remove.and_then(|row| current.iter().position(|&r| r == row)) {
        f.app.actions.push(Action::Apply(Command::new(
            "Remove restricted item",
            vec![Edit::remove_item(key, base, label, i)],
        )));
        right = None;
    }
    if let Some((what, fields)) = set {
        f.set_fields(what, fields);
    }
    ui.data_mut(|d| d.insert_temp(state_id, (only, filter, left, right)));
}

#[cfg(test)]
mod tests {
    /// As Aurora's store Inventory showed them (captured 2026-10-29): a
    /// longsword of 30 gp at a mark up of 137 sells at 41, and buys at 10
    /// at a mark down of 35; 50 arrows of no cost, at 1 each way.
    #[test]
    fn prices_are_rounded_down_and_one_gold_at_the_least() {
        assert_eq!(super::price(30, 137), 41);
        assert_eq!(super::price(30, 35), 10);
        assert_eq!(super::price(0, 137), 1);
        assert_eq!(super::price(0, 35), 1);
    }
}
