//! The module contents tree (Aurora's left pane).

use std::collections::HashMap;

use egui::Ui;
use mg_core::{ResRef, ResType};
use mg_edit::Workspace;
use mg_gff::Gff;
use mg_resman::ResKey;
use mg_rules::GameData;

use crate::{Action, Moonglow, Tab};

/// What Make Placeables Static does, for its menu items.
const STATIC_TIP: &str = "Sets Static on the placeables that lose nothing by it: the game loads and \
    draws static placeables more cheaply. Left dynamic: those that are Useable, are tilted, \
    scaled or moved by a visual transform, or have a script, a conversation, a trap, an \
    inventory or an animation switched on";

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
    /// Blueprints' names, likewise.
    names: HashMap<ResKey, ((usize, usize), String)>,
}

/// A blueprint's name in the editing language, else as the game shows it:
/// a creature's first and last names, another's `LocalizedName` or
/// `LocName`.
fn blueprint_name(game: Option<&GameData>, root: &mg_gff::Struct) -> String {
    let text = |label: &str| {
        root.locstring(label).map_or(String::new(), |name| {
            let edited = crate::text::edited_text(name);
            if !edited.is_empty() {
                return edited;
            }
            game.and_then(|g| g.locstring(name)).unwrap_or_default()
        })
    };
    let full = [text("FirstName"), text("LastName")];
    let full: Vec<&str> = full.iter().map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
    if !full.is_empty() {
        return full.join(" ");
    }
    let name = text("LocalizedName");
    if name.trim().is_empty() { text("LocName") } else { name }
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

    /// A blueprint's name (empty when it has none), read once as the
    /// areas' are.
    pub(crate) fn blueprint(
        &mut self,
        ws: &Workspace,
        game: Option<&GameData>,
        key: ResKey,
    ) -> String {
        if let Some(doc) = ws.loaded(&key) {
            return blueprint_name(game, &doc.root);
        }
        let Some(data) = ws.module.get(&key) else { return String::new() };
        let stamp = (data.as_ptr() as usize, data.len());
        if let Some((s, name)) = self.names.get(&key)
            && *s == stamp
        {
            return name.clone();
        }
        let name = Gff::read(data).map(|g| blueprint_name(game, &g.root)).unwrap_or_default();
        self.names.insert(key, (stamp, name.clone()));
        name
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

/// What is placed in an area, for the module tree: each kind of object
/// and its objects' names (by their place in the area's list).
pub(crate) type AreaContents = Vec<(mg_area::ObjectKind, Vec<String>)>;

/// What is placed in an area, as the workspace has it now: every kind,
/// its objects by name (a tag for one without a name).
fn area_contents(ws: &mut Workspace, game: Option<&GameData>, area: ResRef) -> AreaContents {
    let Ok(git) = ws.doc(&ResKey::new(area, ResType::GIT)) else { return Vec::new() };
    mg_area::ObjectKind::ALL
        .into_iter()
        .map(|kind| {
            let names = git.root.list(kind.list()).unwrap_or(&[]).iter().map(|o| {
                let name = blueprint_name(game, o);
                let tag = || String::from_utf8_lossy(o.string("Tag").unwrap_or_default()).into();
                if name.trim().is_empty() { tag() } else { name }
            });
            (kind, names.collect())
        })
        .collect()
}

/// An area's placed objects under its row of the module tree, kind by
/// kind as Aurora lists them. A click goes to the object in the area's
/// view; a double click opens its Properties.
fn contents_ui(
    ui: &mut Ui,
    area: ResRef,
    contents: &AreaContents,
    go: &mut Option<(ResRef, mg_area::ObjectKind, usize, bool)>,
) {
    ui.indent(("area-contents", area), |ui| {
        for (kind, names) in contents {
            let title = format!("{} ({})", kind.plural(), names.len());
            if names.is_empty() {
                ui.weak(title);
                continue;
            }
            egui::CollapsingHeader::new(title).id_salt(("area-kind", area, *kind)).show(ui, |ui| {
                for (index, name) in names.iter().enumerate() {
                    let shown = if name.is_empty() { "(no name)" } else { name.as_str() };
                    let r = ui.selectable_label(false, shown).on_hover_text(
                        "Click to go to it in the area; double-click for its Properties",
                    );
                    if r.double_clicked() {
                        *go = Some((area, *kind, index, true));
                    } else if r.clicked() {
                        *go = Some((area, *kind, index, false));
                    }
                }
            });
        }
    });
}

pub(crate) fn module_tree(app: &mut Moonglow, ui: &mut Ui) {
    let Some(ws) = &app.ws else { return };
    let revision = ws.revision();
    // Areas opened out whose contents are to be read (after the tree is
    // drawn), and the object clicked.
    let mut read = Vec::new();
    let mut go = None;
    let (by_name, resrefs) = (app.settings.area_names, app.settings.name_resrefs);
    let filter_id = egui::Id::new("tree-filter");
    let mut filter = app.buffers.get(&filter_id).cloned().unwrap_or_default();
    ui.horizontal(|ui| {
        crate::widgets::field_label(ui, "Filter");
        // (As wide as the pane has room for, not wider.)
        ui.add(egui::TextEdit::singleline(&mut filter).desired_width(f32::INFINITY));
    });
    app.buffers.insert(filter_id, filter.clone());
    let filter = filter.to_ascii_lowercase();
    // Every group opened or closed at once (this frame).
    let fold = crate::widgets::fold_buttons(ui, "group");
    if ui.selectable_label(false, "Module Properties").clicked() {
        app.actions.push(Action::OpenTab(Tab::ModuleProperties));
    }
    let mut open = None;
    let mut make = None;
    let mut listed = 0;
    for (name, types) in GROUPS {
        let mut keys: Vec<ResKey> =
            ws.module.keys().filter(|k| types.contains(&k.restype)).copied().collect();
        keys.sort();
        listed += keys.len();
        // Areas by name (Options › General): named, and in the names'
        // order; the filter finds a name or a ResRef.
        let mut names: HashMap<ResKey, String> = HashMap::new();
        let named = types == &[ResType::ARE]
            || types.iter().all(|t| mg_area::ObjectKind::from_restype(*t).is_some());
        if by_name && named {
            let game = app.game.as_deref();
            for k in &keys {
                let name = if k.restype == ResType::ARE {
                    app.area_names.name(ws, game, k.resref)
                } else {
                    app.area_names.blueprint(ws, game, *k)
                };
                // (One without a name keeps its ResRef.)
                if !name.trim().is_empty() {
                    names.insert(*k, named_label(&name, k.resref, resrefs));
                }
            }
            let shown =
                |k: &ResKey| names.get(k).map_or_else(|| k.resref.to_string(), String::clone);
            keys.sort_by_cached_key(|k| (shown(k).to_lowercase(), *k));
        }
        keys.retain(|k| {
            filter.is_empty()
                || k.to_string().contains(&filter)
                || names.get(k).is_some_and(|n| n.to_lowercase().contains(&filter))
        });
        // What makes a new resource of the group (its wizard, or the New
        // window): on the group's and its resources' right-click menus.
        let new = new_command(types);
        let new_label = new.map(|id| new_label(&id.name()));
        let header = egui::CollapsingHeader::new(format!("{name} ({})", keys.len()))
            .id_salt(name)
            .default_open(*name == "Areas")
            // Filtering opens every group with a match, whatever was open.
            .open((!filter.is_empty()).then_some(true).or(fold))
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
                    // An area opens out to what is placed in it.
                    let opened = egui::Id::new(("tree-area-open", k.resref));
                    let is_area = k.restype == ResType::ARE;
                    let mut out = is_area && ui.data(|d| d.get_temp(opened).unwrap_or(false));
                    let row = ui.horizontal(|ui| {
                        if is_area {
                            let arrow = if out { "⏷" } else { "⏵" };
                            let toggle = ui
                                .small_button(arrow)
                                .on_hover_text("What is placed in the area, by kind");
                            if toggle.clicked() {
                                out = !out;
                                ui.data_mut(|d| d.insert_temp(opened, out));
                            }
                        }
                        ui.add(egui::Button::selectable(false, label).sense(sense))
                    });
                    let mut r = row.inner;
                    if out {
                        match app.area_contents.get(&k.resref).filter(|c| c.0 == revision) {
                            Some((_, contents)) => contents_ui(ui, k.resref, contents, &mut go),
                            None => read.push(k.resref),
                        }
                    }
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
                        if let (Some(id), Some(label)) = (new, &new_label) {
                            if ui.button(label).clicked() {
                                make = Some(id);
                            }
                            ui.separator();
                        }
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
                            if ui
                                .button("Make Placeables Static")
                                .on_hover_text(STATIC_TIP)
                                .clicked()
                            {
                                app.actions.push(Action::StaticPlaceables(vec![k.resref]));
                            }
                            if ui
                                .button("Make Placeables Dynamic")
                                .on_hover_text("Clears Static on every placeable of the area")
                                .clicked()
                            {
                                app.actions.push(Action::DynamicPlaceables(vec![k.resref]));
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
                        if ui.button("Delete…").clicked() {
                            app.actions.push(Action::DeleteDialog(k));
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
        if let (Some(id), Some(label)) = (new, &new_label) {
            header.header_response.context_menu(|ui| {
                if ui.button(label).clicked() {
                    make = Some(id);
                }
                if types.contains(&ResType::ARE)
                    && ui
                        .button("Make Placeables Static in Every Area")
                        .on_hover_text(STATIC_TIP)
                        .clicked()
                {
                    let areas = ws.module.keys_of(ResType::ARE).map(|k| k.resref).collect();
                    app.actions.push(Action::StaticPlaceables(areas));
                }
                if types.contains(&ResType::ARE)
                    && ui
                        .button("Make Placeables Dynamic in Every Area")
                        .on_hover_text("Clears Static on every placeable of the module")
                        .clicked()
                {
                    let areas = ws.module.keys_of(ResType::ARE).map(|k| k.resref).collect();
                    app.actions.push(Action::DynamicPlaceables(areas));
                }
            });
        }
    }
    let others = ws.module.len() - listed;
    ui.weak(format!("{others} other resources"));
    if let Some(k) = open {
        app.actions.push(Action::OpenResource(k));
    }
    // What the areas opened out hold, read for the next frame.
    if !read.is_empty()
        && let Some(ws) = app.ws.as_mut()
    {
        for area in read {
            let contents = area_contents(ws, app.game.as_deref(), area);
            app.area_contents.insert(area, (revision, contents));
        }
        ui.ctx().request_repaint();
    }
    // An object clicked in an area's list: gone to in the area's view, or
    // (a double click) its Properties opened.
    if let Some((area, kind, index, properties)) = go {
        if properties {
            let path = mg_edit::GffPath::root().item(kind.list(), index);
            app.actions.push(Action::OpenTab(Tab::Instance { area, path }));
        } else {
            app.area_focus = Some((area, kind, index));
            app.actions.push(Action::OpenTab(Tab::Area(area)));
        }
    }
    if let Some(id) = make {
        id.run(app, ui.ctx());
    }
}

/// A named resource in the tree: its name, and its ResRef in parentheses
/// when asked for (Options › General).
fn named_label(name: &str, resref: ResRef, resrefs: bool) -> String {
    if resrefs { format!("{name} ({resref})") } else { name.to_string() }
}

/// The menu entry for a command that makes something new, from its name:
/// "Area Wizard" is "New Area…", "New Script" is "New Script…".
fn new_label(command: &str) -> String {
    let what = command.trim_end_matches(" Wizard");
    if what.starts_with("New ") { format!("{what}…") } else { format!("New {what}…") }
}

/// The command that makes a new resource of a tree group's types: the
/// Wizards menu's for it.
fn new_command(types: &[ResType]) -> Option<crate::commands::Id> {
    use crate::commands::Id;
    use mg_module::palette::BlueprintKind;
    Some(match *types.first()? {
        ResType::ARE => Id::AreaWizard,
        ResType::DLG => Id::NewConversation,
        ResType::NSS => Id::NewScript,
        ResType::UTC => Id::CreatureWizard,
        ResType::UTD => Id::Wizard(BlueprintKind::Door),
        ResType::UTE => Id::Wizard(BlueprintKind::Encounter),
        ResType::UTI => Id::Wizard(BlueprintKind::Item),
        ResType::UTM => Id::Wizard(BlueprintKind::Store),
        ResType::UTP => Id::Wizard(BlueprintKind::Placeable),
        ResType::UTS => Id::Wizard(BlueprintKind::Sound),
        ResType::UTT => Id::Wizard(BlueprintKind::Trigger),
        ResType::UTW => Id::Wizard(BlueprintKind::Waypoint),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_group_of_resources_has_its_new_command() {
        use crate::commands::Id;
        let label = |types: &[ResType]| new_command(types).map(|id| new_label(&id.name()));
        assert_eq!(label(&[ResType::ARE]).as_deref(), Some("New Area…"));
        assert_eq!(label(&[ResType::UTC]).as_deref(), Some("New Creature…"));
        assert_eq!(label(&[ResType::UTM]).as_deref(), Some("New Store…"));
        assert_eq!(label(&[ResType::NSS]).as_deref(), Some("New Script…"));
        assert_eq!(label(&[ResType::DLG]).as_deref(), Some("New Conversation…"));
        // The journal and factions are one of each: nothing to make.
        assert_eq!(new_command(&[ResType::JRL, ResType::FAC]), None);
        // Every group of blueprints has one.
        for (_, types) in GROUPS.iter().filter(|(n, _)| *n != "Journal and factions") {
            assert!(new_command(types).is_some(), "{types:?}");
        }
        assert_eq!(
            new_command(&[ResType::UTI]),
            Some(Id::Wizard(mg_module::palette::BlueprintKind::Item))
        );
    }
}
