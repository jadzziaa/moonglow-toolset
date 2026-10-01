//! Placeable Object Properties (`TdlgPlaceableEdit`): Basic, Lock, Trap,
//! Scripts, Advanced, Description, Comments; and the placeable's inventory
//! (Aurora's Inventory… dialog) as its own page.

use egui::Ui;
use mg_edit::{Command, Edit};
use mg_gff::{FieldType, Struct};
use mg_module::palette::BlueprintKind;
use mg_rules::ChoiceColumns;

use super::{Form, inventory, situated};
use crate::Action;

pub(super) const PAGES: [&str; 8] =
    ["Basic", "Inventory", "Lock", "Trap", "Scripts", "Advanced", "Description", "Comments"];

/// A placeable's initial states (`AnimationState`).
const STATES: [(i64, &str); 6] = [
    (0, "Default"),
    (1, "Open"),
    (2, "Closed"),
    (3, "Destroyed"),
    (4, "Activated"),
    (5, "Deactivated"),
];

pub(super) fn page(f: &mut Form<'_>, ui: &mut Ui, page: &str) {
    match page {
        "Basic" => basic(f, ui),
        "Inventory" => contents(f, ui),
        "Lock" => situated::lock(f, ui),
        "Trap" => situated::trap(f, ui),
        "Scripts" => situated::scripts(
            f,
            ui,
            &[
                ("OnClick", "OnClick"),
                ("OnClose", "OnClosed"),
                ("OnDamaged", "OnDamaged"),
                ("OnDeath", "OnDeath"),
                ("OnDisturbed", "OnInvDisturbed"),
                ("OnHeartbeat", "OnHeartbeat"),
                ("OnLock", "OnLock"),
                ("OnOpen", "OnOpen"),
                ("OnPhysicalAttacked", "OnMeleeAttacked"),
                ("OnSpellCastAt", "OnSpellCastAt"),
                ("OnUnLock", "OnUnlock"),
                ("OnUsed", "OnUsed"),
                ("OnUserDefined", "OnUserDefined"),
            ],
        ),
        "Advanced" => situated::advanced(f, ui, &STATES, treasure),
        "Description" => f.locstring_memo(ui, "Description", "Description"),
        _ => f.memo(ui, "Comments", "Comment"),
    }
}

fn basic(f: &mut Form<'_>, ui: &mut Ui) {
    let appearances = f
        .app
        .game
        .as_ref()
        .and_then(|g| {
            g.choices("placeables", ChoiceColumns { name: Some("StrRef"), label: Some("Label") })
                .ok()
        })
        .unwrap_or_default();
    egui::Grid::new(("utp-basic", f.key)).num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
        ui.label("Name");
        f.locstring(ui, "Name", "LocName");
        ui.end_row();
        ui.label("Tag");
        f.text(ui, "Tag", "Tag", 32);
        ui.end_row();
        ui.label("Appearance Type");
        f.choice(ui, "Appearance", "Appearance", &appearances, FieldType::Dword);
        ui.end_row();
        ui.label("Category");
        f.category(ui, BlueprintKind::Placeable);
        ui.end_row();
        // A static placeable is scenery: it cannot be used or hold items.
        ui.label("");
        let fixed = f.check(ui, "Static", "Static");
        ui.end_row();
        ui.label("");
        ui.add_enabled_ui(!fixed, |ui| f.check(ui, "Useable", "Useable"));
        ui.end_row();
        ui.label("");
        ui.horizontal(|ui| {
            let has =
                ui.add_enabled_ui(!fixed, |ui| f.check(ui, "Has Inventory", "HasInventory")).inner;
            if ui
                .add_enabled(has, egui::Button::new("Inventory…"))
                .on_hover_text("Edit inventory contents")
                .clicked()
            {
                f.app.blueprint_pages.insert((f.key, f.path.clone()), "Inventory");
            }
        });
        ui.end_row();
        situated::durability(f, ui);
        ui.label("");
        situated::preview_button(f, ui);
        ui.end_row();
    });
}

/// Treasure Model (the body bag left when it is destroyed), with an
/// inventory.
fn treasure(f: &mut Form<'_>, ui: &mut Ui) {
    let bags = f
        .app
        .game
        .as_ref()
        .and_then(|g| {
            g.choices("bodybag", ChoiceColumns { name: Some("Name"), label: Some("LABEL") }).ok()
        })
        .unwrap_or_default();
    let has = f.int("HasInventory") != 0;
    ui.add_enabled_ui(has, |ui| ui.label("Treasure Model"));
    ui.add_enabled_ui(has, |ui| f.choice(ui, "Treasure model", "BodyBag", &bags, FieldType::Byte));
    ui.end_row();
}

/// The items the placeable holds.
fn contents(f: &mut Form<'_>, ui: &mut Ui) {
    let base = f.path.clone();
    if f.int("HasInventory") == 0 {
        ui.weak("The placeable has no inventory (Basic › Has Inventory).");
        return;
    }
    let key = f.key;
    let items: Vec<Struct> = f.root.list("ItemList").unwrap_or(&[]).to_vec();
    let names = f.blueprint_names(BlueprintKind::Item);
    let mut add = None;
    let mut edits = Vec::new();
    ui.columns(2, |cols| {
        add = f.palette_picker(&mut cols[0], BlueprintKind::Item, "Add Item");
        let ctx = cols[1].ctx().clone();
        let icons: Vec<_> = items.iter().map(|it| f.entry_icon(&ctx, it)).collect();
        let name = |it: &Struct| f.entry_name(it, &names);
        edits = inventory::item_list(&mut cols[1], key, &base, &items, &icons, &name, false, &[]);
    });
    if let Some(r) = add {
        let item = f.inventory_item(&items, r);
        let edit = Edit::InsertItem {
            key,
            path: base.clone(),
            list: "ItemList".into(),
            index: items.len(),
            item,
        };
        f.app.actions.push(Action::Apply(Command::new("Add item", vec![edit])));
    }
    for (what, edit) in edits {
        f.app.actions.push(Action::Apply(Command::new(what, vec![edit])));
    }
}
