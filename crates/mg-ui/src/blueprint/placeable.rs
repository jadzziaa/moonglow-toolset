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

pub(super) const PAGES: [&str; 9] = [
    "Basic",
    "Inventory",
    "Lock",
    "Trap",
    "Scripts",
    "Advanced",
    "Visuals",
    "Description",
    "Comments",
];

/// A placeable's initial states (`AnimationState`).
const STATES: [(i64, &str); 6] = [
    (0, "Default"),
    (1, "Open"),
    (2, "Closed"),
    (3, "Destroyed"),
    (4, "Activated"),
    (5, "Deactivated"),
];

/// The width of the Basic page's fields beside the model.
const BASIC_SIDE: f32 = 440.0;

pub(super) fn page(f: &mut Form<'_>, ui: &mut Ui, page: &str) {
    match page {
        "Basic" => situated::beside_model(f, ui, BASIC_SIDE, false, |f, ui, _| basic(f, ui)),
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
        "Visuals" => super::visuals::page(f, ui, true),
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
        crate::widgets::field_label(ui, "Name");
        f.locstring(ui, "Name", "LocName");
        ui.end_row();
        crate::widgets::field_label(ui, "Tag");
        f.text(ui, "Tag", "Tag", 32);
        ui.end_row();
        crate::widgets::field_label(ui, "Appearance Type");
        ui.horizontal(|ui| {
            f.choice(ui, "Appearance", "Appearance", &appearances, FieldType::Dword);
            f.gallery_button(ui, crate::appearance_gallery::Kind::Placeable);
        });
        ui.end_row();
        crate::widgets::field_label(ui, "Category");
        f.category(ui, BlueprintKind::Placeable);
        ui.end_row();
        // A static placeable is scenery: it cannot be used or hold items.
        crate::widgets::field_label(ui, "");
        let fixed = f.check(ui, "Static", "Static");
        ui.end_row();
        crate::widgets::field_label(ui, "");
        ui.add_enabled_ui(!fixed, |ui| f.check(ui, "Useable", "Useable"));
        ui.end_row();
        crate::widgets::field_label(ui, "");
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
            // The game keeps an inventory on a placeable players can't use
            // (engine_ee_fields.rs): scripts reach it, players don't.
            if has && !fixed && f.int("Useable") == 0 {
                ui.weak("not Useable: only scripts reach it");
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
    let (mut adds, mut taken) = (Vec::new(), Vec::new());
    let mut edits = Vec::new();
    crate::widgets::two_columns(ui, 300.0, |ui, col| {
        if col == 0 {
            add = f.item_picker(ui, "Add Item");
            return;
        }
        let ctx = ui.ctx().clone();
        let icons: Vec<_> = items.iter().map(|it| f.entry_icon(&ctx, it)).collect();
        let name = |it: &Struct| f.entry_name(it, &names);
        let look = inventory::ItemLook {
            icons: &icons,
            name: &name,
            infinite: false,
            selected: None,
            equip: false,
            copied: f.app.item_clip.len(),
        };
        let mut out = inventory::item_list(ui, key, &base, &items, look);
        // Pasted, dragged from the palette, or dragged out of another
        // object's inventory: added here.
        let whose = inventory::owner(key, &base);
        (adds, taken) = f.list_events(ui, &items, &mut out, whose, &base);
        edits = out.edits;
    });
    adds.extend(add);
    if !adds.is_empty() {
        let mut held = items.clone();
        let mut together = taken;
        for r in adds {
            let item = f.inventory_item(&held, r);
            together.push(Edit::InsertItem {
                key,
                path: base.clone(),
                list: "ItemList".into(),
                index: held.len(),
                item: item.clone(),
            });
            held.push(item);
        }
        f.app.actions.push(Action::Apply(Command::new("Add item", together)));
    }
    for (what, edit) in edits {
        f.app.actions.push(Action::Apply(Command::new(what, vec![edit])));
    }
}
