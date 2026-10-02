//! Door Properties (`TdlgDoorEdit`): Basic, Lock, Trap, Area Transition,
//! Scripts, Advanced, Description, Comments.

use egui::Ui;
use mg_gff::FieldType;
use mg_module::palette::BlueprintKind;
use mg_rules::{Choice, ChoiceColumns};

use super::{Form, situated};

pub(super) const PAGES: [&str; 9] = [
    "Basic",
    "Lock",
    "Trap",
    "Area Transition",
    "Scripts",
    "Advanced",
    "Visuals",
    "Description",
    "Comments",
];

/// A door's initial states (`AnimationState`).
const STATES: [(i64, &str); 2] = [(0, "Closed"), (1, "Open")];

pub(super) fn page(f: &mut Form<'_>, ui: &mut Ui, page: &str) {
    match page {
        "Basic" => basic(f, ui),
        "Lock" => situated::lock(f, ui),
        "Trap" => situated::trap(f, ui),
        "Area Transition" => situated::transition(f, ui),
        "Scripts" => situated::scripts(
            f,
            ui,
            &[
                ("OnAreaTransitionClick", "OnClick"),
                ("OnClose", "OnClosed"),
                ("OnDamaged", "OnDamaged"),
                ("OnDeath", "OnDeath"),
                ("OnFailToOpen", "OnFailToOpen"),
                ("OnHeartbeat", "OnHeartbeat"),
                ("OnLock", "OnLock"),
                ("OnOpen", "OnOpen"),
                ("OnPhysicalAttacked", "OnMeleeAttacked"),
                ("OnSpellCastAt", "OnSpellCastAt"),
                ("OnUnLock", "OnUnlock"),
                ("OnUserDefined", "OnUserDefined"),
            ],
        ),
        "Advanced" => situated::advanced(f, ui, &STATES, |_, _| {}),
        "Visuals" => super::visuals::page(f, ui, true),
        "Description" => f.locstring_memo(ui, "Description", "Description"),
        _ => f.memo(ui, "Comments", "Comment"),
    }
}

/// Tileset doors (doortypes.2da; row 0 is the generic door): the name and
/// the tileset.
fn door_types(f: &Form<'_>) -> Vec<Choice> {
    let Some(game) = f.app.game.as_ref() else { return Vec::new() };
    let Ok(table) = game.table("doortypes") else { return Vec::new() };
    let names = game
        .choices("doortypes", ChoiceColumns { name: Some("StringRefGame"), label: Some("Label") })
        .unwrap_or_default();
    names
        .into_iter()
        .map(|c| {
            let tileset = table.get(c.row, "TileSet").filter(|t| *t != "****");
            match tileset {
                Some(t) => Choice { row: c.row, text: format!("{} ({t})", c.text) },
                None => c,
            }
        })
        .collect()
}

fn basic(f: &mut Form<'_>, ui: &mut Ui) {
    let types = door_types(f);
    let generic = f
        .app
        .game
        .as_ref()
        .and_then(|g| {
            g.choices("genericdoors", ChoiceColumns { name: Some("Name"), label: Some("Label") })
                .ok()
        })
        .unwrap_or_default();
    egui::Grid::new(("utd-basic", f.key)).num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
        crate::widgets::field_label(ui, "Name");
        f.locstring(ui, "Name", "LocName");
        ui.end_row();
        crate::widgets::field_label(ui, "Tag");
        f.text(ui, "Tag", "Tag", 32);
        ui.end_row();
        crate::widgets::field_label(ui, "Appearance Type");
        f.choice(ui, "Appearance", "Appearance", &types, FieldType::Dword);
        ui.end_row();
        // The generic door's model (EE: GenericType_New; the old byte field
        // follows it where the blueprint has one).
        crate::widgets::field_label(ui, "Generic Appearance");
        let current =
            f.root.integer("GenericType_New").or(f.root.integer("GenericType")).unwrap_or(0);
        ui.add_enabled_ui(f.int("Appearance") == 0, |ui| {
            if let Some(v) = situated::pick(ui, f.key, "generic", &generic, current) {
                let mut fields = vec![("GenericType_New", v, FieldType::Dword)];
                if f.root.get("GenericType").is_some() && v <= 255 {
                    fields.push(("GenericType", v, FieldType::Byte));
                }
                f.set_many("Generic appearance", &fields);
            }
        });
        ui.end_row();
        crate::widgets::field_label(ui, "Category");
        f.category(ui, BlueprintKind::Door);
        ui.end_row();
        situated::durability(f, ui);
        ui.label("");
        situated::preview_button(f, ui);
        ui.end_row();
    });
}
