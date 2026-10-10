//! A window's pictures: where the game finds each, and pictures added from
//! disk, into the module while you work or into a hak the module uses
//! (which is how players get them).
use super::*;
use crate::FileKind;
use mg_module::hak_edit::{Source, key_for, write_into};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// The picture types NUI reads, in the order it looks for them.
const TYPES: [ResType; 5] = [ResType::JPG, ResType::TGA, ResType::PNG, ResType::BMP, ResType::DDS];

/// The pictures the window and its variants show.
fn used(doc: &Value, s: &Settings) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    skin::names(doc, s, &mut names);
    for view in s.views.values() {
        skin::names(view, s, &mut names);
    }
    names
}

/// A picture's resources in the module.
fn in_module(module: &mg_module::Module, name: &str) -> Vec<ResKey> {
    TYPES.iter().filter_map(|ty| ResKey::parse(name, *ty)).filter(|k| module.contains(k)).collect()
}

/// The module's haks as Module Properties lists them, unsaved edits included.
pub(super) fn listed_haks(ws: &mut mg_edit::Workspace) -> Vec<String> {
    use mg_schema::StructExt;
    let Ok(info) = ws.doc(&crate::module_props::info_key()) else { return Vec::new() };
    info.root
        .items(&mg_schema::ifo::MOD_HAK_LIST)
        .iter()
        .map(|h| {
            String::from_utf8_lossy(h.read(&mg_schema::ifo::mod_hak_list::MOD_HAK).as_bytes())
                .trim()
                .to_owned()
        })
        .filter(|h| !h.is_empty())
        .collect()
}

fn hak_folder(app: &Moonglow) -> Option<PathBuf> {
    app.install.as_ref()?.user_dir.as_ref().map(|u| u.join("hak"))
}

/// A file from disk as a resource: its name is the resource name.
fn resource(path: &Path) -> Result<ResKey, String> {
    let file = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    key_for(&file).map_err(|e| format!("{file}: {e}"))
}

/// A stretched picture under the window's controls: a draw layer of the
/// root column, painted before it.
fn background(doc: &mut Value, s: &Settings, name: &str) {
    let geometry = resolved(&doc["geometry"], s);
    let title = doc["title"] != json!(false);
    let w = geometry["w"].as_f64().unwrap_or(400.0) - 16.0;
    let h = geometry["h"].as_f64().unwrap_or(300.0) - 16.0 - if title { 33.0 } else { 0.0 };
    let item = json!({"type":5,"enabled":true,"order":-1,"render":0,"arrayBinds":false,
        "rect":{"x":0.0,"y":0.0,"w":w.max(1.0),"h":h.max(1.0)},
        "image":name,"image_aspect":5,"image_halign":0,"image_valign":0});
    let root = &mut doc["root"];
    if !root["draw_list"].is_array() {
        root["draw_list"] = json!([]);
    }
    root["draw_list"].as_array_mut().unwrap().insert(0, item);
}

/// The hak pictures go into: the one chosen (in the user's hak folder) or a
/// new one asked for. A hak saved elsewhere is copied into the hak folder.
fn chosen_hak(app: &mut Moonglow, name: &str, choice: &Option<String>) -> Option<PathBuf> {
    let Some(folder) = hak_folder(app) else {
        app.log.error("Haks need the game's user folder (Tools › Options)");
        return None;
    };
    match choice {
        Some(hak) => Some(folder.join(format!("{hak}.hak"))),
        None => {
            app.dialogs.save_file(FileKind::Hak, Some(&folder.join(format!("{name}_images.hak"))))
        }
    }
}

/// After writing a hak: one outside the hak folder is copied there; one the
/// module doesn't list yet is added at the top of its list (Undo takes it
/// off), one it lists is read again.
fn use_hak(app: &mut Moonglow, path: &Path, edits: &mut Vec<Edit>) {
    let Some(folder) = hak_folder(app) else { return };
    let hak = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    if path.parent() != Some(folder.as_path()) {
        let user = folder.parent().unwrap_or(&folder).to_owned();
        match mg_module::attach::placements(&user, &[path.to_owned()]) {
            Ok(places) => {
                if places.iter().any(|p| p.there == mg_module::attach::There::Different) {
                    app.log.error(format!("Another {hak}.hak is in the hak folder: not replaced"));
                    return;
                }
                if let Err(e) = mg_module::attach::copy(&places, false) {
                    app.log.error(e);
                    return;
                }
            }
            Err(e) => {
                app.log.error(e);
                return;
            }
        }
    }
    let Some(ws) = &mut app.ws else { return };
    if listed_haks(ws).iter().any(|h| h.eq_ignore_ascii_case(&hak)) {
        app.actions.push(Action::ReloadResources);
    } else if let Ok(info) = ws.doc(&crate::module_props::info_key()) {
        edits.push(Edit::SetField {
            key: crate::module_props::info_key(),
            path: mg_edit::GffPath::root(),
            label: mg_schema::ifo::MOD_HAK_LIST.label.to_string(),
            value: Some(mg_module::attach::hak_list(&info.root, &[hak])),
        });
    }
}

/// The Images page. Returns the module's changes to make with the window's.
pub(super) fn page(
    ui: &mut Ui,
    app: &mut Moonglow,
    name: &str,
    doc: &mut Value,
    s: &Settings,
    state: &mut State,
) -> Vec<Edit> {
    let mut edits = Vec::new();
    let Some(ws) = &mut app.ws else { return edits };
    let names = used(doc, s);
    let module_images: Vec<ResKey> = names.iter().flat_map(|n| in_module(&ws.module, n)).collect();
    let haks = listed_haks(ws);
    egui::ScrollArea::vertical().id_salt("nui-images-page").show(ui, |ui| {
        ui.heading("Images");
        ui.label("The pictures this window shows and where the game finds each. Add your own from disk into the module while you work; players get them from a hak the module uses.");
        ui.add_space(6.0);
        if names.is_empty() {
            ui.weak("No pictures yet: an Image, an Image button or a draw layer's Image names one.");
        }
        egui::Grid::new("nui-images").striped(true).num_columns(3).spacing([12.0, 6.0]).show(ui, |ui| {
            for picture in &names {
                ui.label(picture);
                let origin = app
                    .nui_assets
                    .origins
                    .iter()
                    .find(|(k, _)| k.rsplit_once('.').is_some_and(|(n, _)| n == picture))
                    .map(|(_, o)| o.clone());
                match origin {
                    Some(origin) => ui.label(origin),
                    None => ui.colored_label(egui::Color32::LIGHT_RED, "Missing: the game shows its gui_error picture"),
                };
                if ui.small_button("Use as background").on_hover_text("Stretched under the window's controls").clicked() {
                    background(doc, s, picture);
                }
                ui.end_row();
            }
        });
        ui.add_space(8.0);
        if ui.button("Add images from disk…").on_hover_text("Into the module, named as their files").clicked() {
            for path in app.dialogs.open_files(FileKind::Images, None) {
                match resource(&path).and_then(|k| Ok((k, std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?))) {
                    Ok((key, bytes)) => edits.push(Edit::SetResource { key, data: Some(bytes) }),
                    Err(e) => app.log.error(e),
                }
            }
        }
        ui.separator();
        ui.strong("Hak");
        ui.horizontal_wrapped(|ui| {
            egui::ComboBox::from_id_salt("nui-images-hak")
                .selected_text(state.images_hak.as_deref().unwrap_or("New hak…"))
                .show_ui(ui, |ui| {
                    for hak in &haks {
                        ui.selectable_value(&mut state.images_hak, Some(hak.clone()), hak);
                    }
                    ui.selectable_value(&mut state.images_hak, None, "New hak…");
                });
            if ui.button("Add images to the hak…").clicked() {
                let files = app.dialogs.open_files(FileKind::Images, None);
                if !files.is_empty()
                    && let Some(path) = chosen_hak(app, name, &state.images_hak)
                {
                    let resources: Result<Vec<_>, _> =
                        files.iter().map(|f| resource(f).map(|k| (k, Source::File(f.clone())))).collect();
                    match resources.and_then(|r| write_into(&path, r)) {
                        Ok(_) => use_hak(app, &path, &mut edits),
                        Err(e) => app.log.error(e),
                    }
                }
            }
            if ui
                .add_enabled(!module_images.is_empty(), egui::Button::new("Move module images into the hak"))
                .on_hover_text("This window's pictures leave the module for the hak; Undo brings them back, the hak keeps them")
                .on_disabled_hover_text("None of this window's pictures is in the module")
                .clicked()
                && let Some(path) = chosen_hak(app, name, &state.images_hak)
                && let Some(ws) = &app.ws
            {
                let resources: Vec<_> = module_images
                    .iter()
                    .filter_map(|k| Some((*k, Source::Bytes(ws.module.get(k)?.to_vec().into()))))
                    .collect();
                match write_into(&path, resources) {
                    Ok(_) => {
                        edits.extend(module_images.iter().map(|k| Edit::SetResource { key: *k, data: None }));
                        use_hak(app, &path, &mut edits);
                    }
                    Err(e) => app.log.error(e),
                }
            }
        });
        ui.weak("A new hak is made in the game's hak folder and added at the top of the module's haks.");
    });
    edits
}
