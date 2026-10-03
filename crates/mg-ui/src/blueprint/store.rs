//! Merchant Properties (`TdlgStoreEdit`): Basic (name, pricing, stolen
//! goods, gold), Advanced, Restrictions (base items the store will not, or
//! will only, buy), Comments; and the store's inventory (Aurora's
//! Inventory… dialog) as its own page.

use egui::Ui;
use mg_core::ResRef;
use mg_edit::{Command, Edit};
use mg_gff::{FieldType, Struct, Value};
use mg_module::palette::BlueprintKind;

use super::{Form, inventory, situated};
use crate::Action;
use crate::widgets::commit_number;

pub(super) const PAGES: [&str; 5] = ["Basic", "Inventory", "Advanced", "Restrictions", "Comments"];

/// The store's inventory pages as Aurora writes them: (`StoreList` struct
/// id, name).
pub(crate) const STORE_PAGES: [(u32, &str); 5] = [
    (0, "Armor"),
    (4, "Weapons"),
    (2, "Potions & Scrolls"),
    (3, "Rings & Amulets"),
    (1, "Miscellaneous"),
];

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
    ui.separator();
    crate::widgets::section_heading(ui, "Pricing");
    egui::Grid::new(("utm-pricing", f.key)).num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
        crate::widgets::field_label(ui, "Sell Mark Up (%)");
        f.number(ui, "Sell mark up", "MarkUp", 1..=1000);
        ui.end_row();
        crate::widgets::field_label(ui, "Buy Mark Down (%)");
        f.number(ui, "Buy mark down", "MarkDown", 0..=100);
        ui.end_row();
        crate::widgets::field_label(ui, "Identify Price");
        optional_price(f, ui, "Will Identify Items", "IdentifyPrice", "Identify price", 100, 100);
        ui.end_row();
    });
    ui.separator();
    crate::widgets::section_heading(ui, "Stolen Goods");
    let black_market = f.check(ui, "Buy Stolen Goods", "BlackMarket");
    ui.horizontal(|ui| {
        ui.add_enabled_ui(black_market, |ui| {
            crate::widgets::field_label(ui, "Buy Mark Down (%)");
            f.number(ui, "Stolen goods mark down", "BM_MarkDown", 0..=100);
        });
    });
    ui.separator();
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

/// Adds an item to the page its base item belongs on, at the first free
/// place; the page's struct id.
fn add_item(f: &mut Form<'_>, resref: ResRef) -> u32 {
    let base = f.path.clone();
    let page =
        f.item_fit(resref).panel.and_then(|p| PANEL_PAGE.get(p as usize).copied()).unwrap_or(1);
    let pages = f.root.list("StoreList").unwrap_or(&[]).to_vec();
    let index = pages.iter().position(|p| p.id == page);
    let items = index.and_then(|i| pages[i].list("ItemList")).unwrap_or(&[]).to_vec();
    let mut item = f.inventory_item(&items, resref);
    item.set("Infinite", Value::Byte(0));
    let edit = match index {
        Some(i) => Edit::InsertItem {
            key: f.key,
            path: base.clone().item("StoreList", i),
            list: "ItemList".into(),
            index: items.len(),
            item,
        },
        None => {
            let mut p = Struct::new(page);
            p.set("ItemList", Value::List(vec![item]));
            Edit::InsertItem {
                key: f.key,
                path: base.clone(),
                list: "StoreList".into(),
                index: pages.len(),
                item: p,
            }
        }
    };
    f.app.actions.push(Action::Apply(Command::new("Add store item", vec![edit])));
    page
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
        let icons: Vec<_> = items.iter().map(|it| f.entry_icon(ui.ctx(), it)).collect();
        let name = |it: &Struct| f.entry_name(it, &names);
        let look =
            inventory::ItemLook { icons: &icons, name: &name, infinite: true, selected: None };
        edits = inventory::item_list(ui, key, &path, &items, look).0;
    });
    if let Some(r) = add {
        shown = add_item(f, r);
    }
    for (what, edit) in edits {
        f.app.actions.push(Action::Apply(Command::new(what, vec![edit])));
    }
    ui.data_mut(|d| d.insert_temp(shown_id, shown));
}

/// The base items a store can list (those with a store panel): (row, name),
/// by name.
fn base_items(f: &Form<'_>) -> Vec<(i64, String)> {
    let Some(game) = f.app.game.as_ref() else { return Vec::new() };
    let Ok(table) = game.table("baseitems") else { return Vec::new() };
    let mut out: Vec<(i64, String)> = (0..table.len())
        .filter(|&row| table.get_int(row, "StorePanel").is_some())
        .map(|row| {
            let name = table
                .get_int(row, "Name")
                .and_then(|s| game.string(mg_core::StrRef(s as u32)))
                .filter(|s| !s.is_empty())
                .or_else(|| table.get(row, "label").map(str::to_string))
                .unwrap_or_else(|| format!("({row})"));
            (row as i64, name)
        })
        .collect();
    out.sort_by_key(|(_, name)| name.to_lowercase());
    out
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
    let (label, current) =
        if only { ("WillOnlyBuy", only_buy.clone()) } else { ("WillNotBuy", not_buy.clone()) };
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
    let name = |row: i64| {
        items.iter().find(|(r, _)| *r == row).map_or_else(|| format!("({row})"), |(_, n)| n.clone())
    };
    let mut add = None;
    let mut remove = None;
    ui.horizontal_top(|ui| {
        ui.vertical(|ui| {
            ui.set_width(240.0);
            ui.label("Base Items");
            ui.add(egui::TextEdit::singleline(&mut filter).hint_text("Find"));
            let needle = filter.to_lowercase();
            egui::ScrollArea::vertical().id_salt(("utm-bases", key)).max_height(360.0).show(
                ui,
                |ui| {
                    for (row, text) in &items {
                        if current.contains(row) || !text.to_lowercase().contains(&needle) {
                            continue;
                        }
                        let r = ui.selectable_label(left == Some(*row), text);
                        if r.clicked() {
                            left = Some(*row);
                        }
                        if r.double_clicked() {
                            add = Some(*row);
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
            vec![Edit::InsertItem {
                key,
                path: base.clone(),
                list: label.into(),
                index: current.len(),
                item: s,
            }],
        )));
        left = None;
    }
    if let Some(i) = remove.and_then(|row| current.iter().position(|&r| r == row)) {
        f.app.actions.push(Action::Apply(Command::new(
            "Remove restricted item",
            vec![Edit::RemoveItem { key, path: base.clone(), list: label.into(), index: i }],
        )));
        right = None;
    }
    if let Some((what, fields)) = set {
        f.set_fields(what, fields);
    }
    ui.data_mut(|d| d.insert_temp(state_id, (only, filter, left, right)));
}
