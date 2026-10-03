//! The module contents tree (Aurora's left pane).

use std::collections::HashMap;

use egui::Ui;
use mg_core::{ResRef, ResType};
use mg_edit::Workspace;
use mg_gff::Gff;
use mg_resman::ResKey;
use mg_rules::GameData;

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

/// What lists of areas show and filter by, from an area's ARE.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct AreaInfo {
    /// Its name in the editing language, else as the game shows it (its
    /// talk-table string); empty when it has none.
    pub name: String,
    pub tileset: ResRef,
    /// `Flags`: 1 interior, 2 underground, 4 natural.
    pub flags: u32,
}

/// The areas' names (and tilesets and flags), each read from its ARE once:
/// again when the ARE is replaced, and from the open document while it is
/// being edited.
#[derive(Debug, Default)]
pub(crate) struct AreaNames {
    /// By area: the stored ARE read (where its data is and its length:
    /// replaced data is elsewhere) and what it says.
    read: HashMap<ResRef, ((usize, usize), AreaInfo)>,
}

impl AreaNames {
    pub(crate) fn info(
        &mut self,
        ws: &Workspace,
        game: Option<&GameData>,
        area: ResRef,
    ) -> AreaInfo {
        let key = ResKey::new(area, ResType::ARE);
        let read = |are: &Gff| {
            let name = are.root.locstring("Name").map_or(String::new(), |name| {
                let edited = crate::text::edited_text(name);
                if !edited.is_empty() {
                    return edited;
                }
                game.and_then(|g| g.locstring(name)).unwrap_or_default()
            });
            AreaInfo {
                name,
                tileset: are.root.resref("Tileset").unwrap_or(ResRef::EMPTY),
                flags: are.root.integer("Flags").unwrap_or(0) as u32,
            }
        };
        if let Some(are) = ws.loaded(&key) {
            return read(are);
        }
        let Some(data) = ws.module.get(&key) else { return AreaInfo::default() };
        let stamp = (data.as_ptr() as usize, data.len());
        if let Some((s, info)) = self.read.get(&area)
            && *s == stamp
        {
            return info.clone();
        }
        let info = Gff::read(data).map(|g| read(&g)).unwrap_or_default();
        self.read.insert(area, (stamp, info.clone()));
        info
    }

    /// The area's name in the editing language, else as the game shows it
    /// (its talk-table string); empty when it has none.
    pub(crate) fn name(&mut self, ws: &Workspace, game: Option<&GameData>, area: ResRef) -> String {
        self.info(ws, game, area).name
    }

    /// What the area is called in lists and tab titles: its name when
    /// `by_name` and it has one, else its ResRef.
    pub(crate) fn label(
        &mut self,
        ws: &Workspace,
        game: Option<&GameData>,
        area: ResRef,
        by_name: bool,
    ) -> String {
        let name = if by_name { self.name(ws, game, area) } else { String::new() };
        if name.trim().is_empty() { area.to_string() } else { name }
    }
}

pub(crate) fn module_tree(app: &mut Moonglow, ui: &mut Ui) {
    let Some(ws) = &app.ws else { return };
    let by_name = app.settings.area_names;
    let filter_id = egui::Id::new("tree-filter");
    let mut filter = app.buffers.get(&filter_id).cloned().unwrap_or_default();
    ui.horizontal(|ui| {
        crate::widgets::field_label(ui, "Filter");
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
        // Areas by name (Options › General): named, and in the names'
        // order; the filter finds a name or a ResRef.
        let mut names: HashMap<ResKey, String> = HashMap::new();
        if by_name && types == &[ResType::ARE] {
            for k in &keys {
                let name = app.area_names.label(ws, app.game.as_deref(), k.resref, true);
                names.insert(*k, name);
            }
            keys.sort_by_cached_key(|k| (names[k].to_lowercase(), *k));
        }
        keys.retain(|k| {
            filter.is_empty()
                || k.to_string().contains(&filter)
                || names.get(k).is_some_and(|n| n.to_lowercase().contains(&filter))
        });
        egui::CollapsingHeader::new(format!("{name} ({})", keys.len()))
            .id_salt(name)
            .default_open(*name == "Areas")
            // Filtering opens every group with a match, whatever was open.
            .open((!filter.is_empty()).then_some(true))
            .show(ui, |ui| {
                for k in keys {
                    let label = match (names.get(&k), k.restype) {
                        (Some(name), _) => name.clone(),
                        (None, ResType::ARE | ResType::DLG | ResType::NSS) => k.resref.to_string(),
                        _ => k.to_string(),
                    };
                    // Blueprints drag into an area, as from the palette.
                    let blueprint = mg_area::ObjectKind::from_restype(k.restype).is_some();
                    let sense = if blueprint {
                        egui::Sense::click_and_drag()
                    } else {
                        egui::Sense::click()
                    };
                    let mut r = ui.add(egui::Button::selectable(false, label).sense(sense));
                    if names.contains_key(&k) {
                        r = r.on_hover_text(k.resref.to_string());
                    }
                    if blueprint && r.drag_started() {
                        r.dnd_set_drag_payload(crate::palette_view::Dragged(k));
                    }
                    if r.double_clicked() {
                        open = Some(k);
                    }
                    r.context_menu(|ui| {
                        if k.restype == ResType::ARE {
                            if ui
                                .button("Edit Areas Together…")
                                .on_hover_text(
                                    "Choose more areas to set their properties with this one's",
                                )
                                .clicked()
                            {
                                app.area_chooser =
                                    Some(crate::area_props::AreaChooser::with(k.resref));
                            }
                            if ui.button("View Area").clicked() {
                                app.actions.push(Action::OpenTab(Tab::Area(k.resref)));
                            }
                            if ui.button("Properties").clicked() {
                                app.actions.push(Action::OpenTab(Tab::AreaProperties(k.resref)));
                            }
                            if ui
                                .button("Export Minimap…")
                                .on_hover_text("The area's map as the game shows it, as a PNG")
                                .clicked()
                            {
                                app.actions.push(Action::ExportMinimap(k.resref));
                            }
                            for t in [ResType::ARE, ResType::GIT] {
                                let g = ResKey::new(k.resref, t);
                                if ui.button(format!("Fields of {g}")).clicked() {
                                    app.actions.push(Action::OpenTab(Tab::Gff(g)));
                                }
                            }
                            ui.separator();
                        }
                        if mg_module::rename::renamable(k.restype) {
                            if ui.button("Find References").clicked() {
                                app.actions.push(Action::FindReferences(k));
                            }
                            if ui.button("Rename…").clicked() {
                                app.actions.push(Action::RenameDialog(k));
                            }
                        }
                        if ui.button("Export…").clicked() {
                            app.actions.push(Action::ExportDialog(vec![k]));
                        }
                        let with = match k.restype {
                            ResType::NSS => " (with its compiled script)",
                            ResType::ARE => " (with its .git and .gic)",
                            _ => "",
                        };
                        if ui
                            .button("Export as Files…")
                            .on_hover_text(format!("Write {k} into a folder{with}"))
                            .clicked()
                        {
                            app.actions.push(Action::ExportFiles {
                                keys: vec![k],
                                dependencies: false,
                                scratch: false,
                            });
                        }
                        let tip = crate::transfer::scratch_tip(
                            app.settings.scratch_dir.as_deref(),
                            &format!("{k}{with}"),
                        );
                        if ui.button("Copy to Scratch Folder").on_hover_text(tip).clicked() {
                            app.actions.push(Action::ExportFiles {
                                keys: vec![k],
                                dependencies: false,
                                scratch: true,
                            });
                        }
                    });
                }
            });
    }
    let others = ws.module.len() - listed;
    ui.weak(format!("{others} other resources"));
    if let Some(k) = open.and_then(Tab::for_resource) {
        app.actions.push(Action::OpenTab(k));
    }
}
