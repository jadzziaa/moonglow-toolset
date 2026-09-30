//! The module contents tree (Aurora's left pane).

use egui::Ui;
use mg_core::ResType;
use mg_resman::ResKey;

use crate::{Action, Moonglow, Tab};

const GROUPS: &[(&str, &[ResType])] = &[
    ("Areas", &[ResType::ARE]),
    ("Conversations", &[ResType::DLG]),
    ("Scripts", &[ResType::NSS]),
    ("Creatures", &[ResType::UTC]),
    ("Doors", &[ResType::UTD]),
    ("Encounters", &[ResType::UTE]),
    ("Items", &[ResType::UTI]),
    ("Merchants", &[ResType::UTM]),
    ("Placeables", &[ResType::UTP]),
    ("Sounds", &[ResType::UTS]),
    ("Triggers", &[ResType::UTT]),
    ("Waypoints", &[ResType::UTW]),
    ("Journal and factions", &[ResType::JRL, ResType::FAC]),
];

pub(crate) fn module_tree(app: &mut Moonglow, ui: &mut Ui) {
    let Some(ws) = &app.ws else { return };
    let filter_id = egui::Id::new("tree-filter");
    let mut filter = app.buffers.get(&filter_id).cloned().unwrap_or_default();
    ui.horizontal(|ui| {
        ui.label("Filter");
        ui.text_edit_singleline(&mut filter);
    });
    app.buffers.insert(filter_id, filter.clone());
    let filter = filter.to_ascii_lowercase();
    if ui.selectable_label(false, "Module Properties").clicked() {
        app.actions.push(Action::OpenTab(Tab::ModuleProperties));
    }
    let mut open = None;
    let mut listed = 0;
    for (name, types) in GROUPS {
        let mut keys: Vec<ResKey> =
            ws.module.keys().filter(|k| types.contains(&k.restype)).copied().collect();
        keys.sort();
        listed += keys.len();
        keys.retain(|k| filter.is_empty() || k.to_string().contains(&filter));
        egui::CollapsingHeader::new(format!("{name} ({})", keys.len()))
            .id_salt(name)
            .default_open(*name == "Areas" || !filter.is_empty())
            .show(ui, |ui| {
                for k in keys {
                    let label = match k.restype {
                        ResType::ARE | ResType::DLG | ResType::NSS => k.resref.to_string(),
                        _ => k.to_string(),
                    };
                    if ui.selectable_label(false, label).double_clicked() {
                        open = Some(k);
                    }
                }
            });
    }
    let others = ws.module.len() - listed;
    ui.weak(format!("{others} other resources"));
    if let Some(k) = open.and_then(Tab::for_resource) {
        app.actions.push(Action::OpenTab(k));
    }
}
