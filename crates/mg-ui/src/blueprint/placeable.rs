//! Placeable Object Properties (`TdlgPlaceableEdit`): Basic, Lock, Trap,
//! Scripts, Advanced, Description, Comments; and the placeable's inventory
//! (Aurora's Inventory… dialog) as its own page.

use egui::Ui;
use mg_gff::{FieldType, Struct};
use mg_module::palette::BlueprintKind;

use super::{Form, inventory, situated};

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
        "Advanced" => situated::advanced(f, ui, &STATES, advanced),
        "Visuals" => super::visuals::page(f, ui, true),
        "Description" => f.locstring_memo(ui, "Description", "Description"),
        _ => f.memo(ui, "Comments", "Comment"),
    }
}

fn basic(f: &mut Form<'_>, ui: &mut Ui) {
    let appearances = f.choices("placeables", Some("StrRef"), Some("Label"));
    egui::Grid::new(("utp-basic", f.key)).num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
        crate::widgets::field_label(ui, "Name");
        f.locstring(ui, "Name", "LocName");
        ui.end_row();
        crate::widgets::field_label(ui, "Tag");
        f.text(ui, "Tag", "Tag", 32);
        ui.end_row();
        crate::widgets::field_label(ui, "Appearance Type");
        ui.horizontal(|ui| {
            f.appearance_choice(ui, "Appearance", &appearances, FieldType::Dword, false);
            f.gallery_button(ui, crate::appearance_gallery::Kind::Placeable);
        });
        ui.end_row();
        f.model_warning(ui, mg_rules::ObjectKind::Placeable);
        crate::widgets::field_label(ui, "Category");
        f.category(ui, BlueprintKind::Placeable);
        ui.end_row();
        // A static placeable is scenery: it cannot be used or hold items.
        // Ticked, its visual transform goes too, as in Aurora: the game
        // draws a static placeable without one.
        crate::widgets::field_label(ui, "");
        let fixed =
            f.check_taking(ui, "Static", "Static", &mg_module::areas::VISUAL_TRANSFORM_FIELDS);
        ui.end_row();
        crate::widgets::field_label(ui, "");
        ui.add_enabled_ui(!fixed, |ui| f.check(ui, "Useable", "Useable"));
        ui.end_row();
        crate::widgets::field_label(ui, "");
        ui.horizontal(|ui| {
            let has =
                ui.add_enabled_ui(!fixed, |ui| f.check(ui, "Has Inventory", "HasInventory")).inner;
            // (Several edited together: an inventory is one's own.)
            let one = f.also.is_empty();
            let button = ui
                .add_enabled(has && one, egui::Button::new("Inventory…"))
                .on_hover_text("Edit inventory contents");
            let button = if one {
                button
            } else {
                button.on_disabled_hover_text("Inventories are edited one at a time")
            };
            if button.clicked() {
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

/// The Advanced page's own rows: Treasure Model (the body bag left when
/// it is destroyed), with an inventory; and Illumination.
fn advanced(f: &mut Form<'_>, ui: &mut Ui) {
    let bags = f.choices("bodybag", Some("Name"), Some("LABEL"));
    let has = f.int("HasInventory") != 0;
    ui.add_enabled_ui(has, |ui| ui.label("Treasure Model"));
    ui.add_enabled_ui(has, |ui| f.choice(ui, "Treasure model", "BodyBag", &bags, FieldType::Byte));
    ui.end_row();
    illumination(f, ui);
}

/// Whether its light is on (`LightState`, read by the game and not shown
/// by Aurora: `mg_rules::placeables`). Without the field it is on, and
/// the field is written only when the builder changes it. It can be set
/// whatever the appearance (Jasperre's report): one without a light
/// (placeables.2da `LightColor`) isn't lit by the game, but its row may
/// gain one later, and a placeable unlit on purpose stays so. It turns off
/// the appearance's light only; the model's own light nodes stay on, as
/// they do in the game (`model_light_uniforms` in `client_render.rs`).
fn illumination(f: &mut Form<'_>, ui: &mut Ui) {
    use mg_rules::placeables::{LIGHT_STATE, has_light, illuminated};
    let appearance = f.int("Appearance");
    let lit = f.app.game.as_deref().is_some_and(|g| has_light(g, appearance));
    crate::widgets::field_label(ui, "");
    f.mark_mixed(ui, LIGHT_STATE);
    let mut on = illuminated(&f.root);
    let r = ui.checkbox(&mut on, "Illuminated");
    let r = if lit {
        r.on_hover_text(
            "Whether the placeable's light (placeables.2da LightColor) is on when the game \
             loads it (GetPlaceableIllumination; scripts turn it on and off with \
             SetPlaceableIllumination). The model's own light nodes stay on either way. \
             Aurora doesn't show this field.",
        )
    } else {
        r.on_hover_text(
            "Its appearance has no light (placeables.2da LightColor), so the game doesn't \
             light it; this is kept for when the appearance's row gains one. On unless \
             unticked. Aurora doesn't show this field.",
        )
    };
    if r.changed() {
        f.set_int("Illumination", LIGHT_STATE, i64::from(on), FieldType::Byte);
    }
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
        let copied = f.app.item_clip.len();
        let mut row = |_: usize, it: &Struct| (f.entry_icon(&ctx, it), f.entry_name(it, &names));
        let look = inventory::ItemLook {
            row: &mut row,
            infinite: false,
            selected: &[],
            equip: false,
            copied,
        };
        let mut out = inventory::item_list(ui, key, &base, &items, look);
        // Pasted, dragged from the palette, or dragged out of another
        // object's inventory: added here.
        let whose = inventory::owner(key, &base);
        (adds, taken) = f.list_events(ui, &items, &mut out, whose, &base);
        edits = out.edits;
    });
    adds.extend(add.map(inventory::Added::Blueprint));
    f.apply_list(&base, &items, adds, taken, edits);
}
