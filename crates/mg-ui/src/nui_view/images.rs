//! A window's pictures: where the game finds each, and pictures added from
//! disk, into the module while you work or into a hak the module uses
//! (which is how players get them).
use super::*;
use crate::FileKind;
use crate::images::origin_name;
use mg_module::hak_edit::{Source, key_for, write_into};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// The picture types NUI reads, in the order it looks for them.
const TYPES: [ResType; 5] = [ResType::JPG, ResType::TGA, ResType::PNG, ResType::BMP, ResType::DDS];

/// The pictures the window and its variants show.
fn used(doc: &Value, s: &Settings) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    skin::names(doc, s, &mut names);
    for view in s.variants.values() {
        skin::names(view, s, &mut names);
    }
    names
}

/// A picture's resources in the module.
fn in_module(module: &mg_module::Module, name: &str) -> Vec<ResKey> {
    TYPES.iter().filter_map(|ty| ResKey::parse(name, *ty)).filter(|k| module.contains(k)).collect()
}

/// A file from disk as a resource: its name is the resource name.
fn resource(path: &Path) -> Result<ResKey, String> {
    let file = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    key_for(&file).map_err(|e| format!("{file}: {e}"))
}

/// The largest file taken as a picture: a 4096 by 4096 TGA with alpha is
/// 64 MB.
const LARGEST: u64 = 64 * 1024 * 1024;

/// A picture file as the module resource it becomes, and the name it takes
/// when the file's own is one the game can't read (over 16 characters, other
/// symbols): its letters and digits, shortened, one the module doesn't use.
fn from_file(
    path: &Path,
    module: &mg_module::Module,
) -> Result<(ResKey, Vec<u8>, Option<String>), String> {
    // (A picture, not whatever file was chosen: the bytes go into the
    // module and its history.)
    let size = std::fs::metadata(path).map_err(|e| format!("{}: {e}", path.display()))?.len();
    if size > LARGEST {
        return Err(format!(
            "{}: {} MB is more than a picture for a window is ({} MB at most)",
            path.display(),
            size / (1024 * 1024),
            LARGEST / (1024 * 1024)
        ));
    }
    let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let why = match resource(path) {
        Ok(key) => return Ok((key, bytes, None)),
        Err(why) => why,
    };
    let ext = path.extension().map(|e| e.to_string_lossy().into_owned()).unwrap_or_default();
    let restype = ResType::from_extension(&ext).ok_or(why)?;
    let stem: String = path
        .file_stem()
        .map(|s| s.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' { c } else { '_' })
        .collect();
    let stem = if stem.trim_matches('_').is_empty() { "picture".to_owned() } else { stem };
    for n in 1..10_000 {
        let suffix = if n == 1 { String::new() } else { format!("_{n}") };
        let name = format!("{}{suffix}", &stem[..stem.len().min(16 - suffix.len())]);
        let key = ResKey::parse(&name, restype).ok_or_else(|| format!("{name}: not a name"))?;
        if module.get(&key).is_none_or(|b| b == bytes.as_slice()) {
            return Ok((key, bytes, Some(name)));
        }
    }
    Err(format!("{}: the module has too many pictures named like it", path.display()))
}

/// One file into the module: its edit, or why not (logged).
fn add_file(app: &mut Moonglow, path: &Path) -> Option<(ResKey, Edit)> {
    let module = &app.ws.as_ref()?.module;
    match from_file(path, module) {
        Ok((key, bytes, renamed)) => {
            // (A file of a name the module has, with other bytes: said,
            // as it takes that picture's place.)
            if renamed.is_none() && module.get(&key).is_some_and(|b| b != bytes.as_slice()) {
                app.log.warn(format!(
                    "{key} takes the place of the picture of that name the module had (Undo \
                     brings it back)"
                ));
            }
            if let Some(name) = renamed {
                let file = path.file_name().unwrap_or_default().to_string_lossy();
                app.log.info(format!(
                    "{file} is added as {name}: the game reads names of up to 16 letters, \
                     digits, _ and -"
                ));
            }
            Some((key, Edit::SetResource { key, data: Some(bytes) }))
        }
        Err(e) => {
            app.log.error(e);
            None
        }
    }
}

fn asked() -> egui::Id {
    egui::Id::new("nui-picture-from-disk")
}

/// A picture field's **From disk…**: asks for the file (the editors have no
/// app), and returns the picture's name on the frame after it was added.
pub(super) fn from_disk(ui: &mut Ui) -> Option<String> {
    let field = ui.make_persistent_id("from-disk");
    let picked = ui.ctx().data_mut(|d| d.remove_temp::<String>(field));
    if ui
        .button("From disk…")
        .on_hover_text("Add a picture file to the module and use it")
        .clicked()
    {
        ui.ctx().data_mut(|d| d.insert_temp(asked(), Some(field)));
    }
    picked
}

/// Answers a **From disk…**: the file into the module, its name to the field.
pub(super) fn answer_from_disk(ctx: &egui::Context, app: &mut Moonglow) -> Option<Edit> {
    let field = ctx.data_mut(|d| d.remove_temp::<Option<egui::Id>>(asked()))??;
    let path = app.dialogs.open_file(FileKind::Images, None)?;
    let (key, edit) = add_file(app, &path)?;
    ctx.data_mut(|d| d.insert_temp(field, key.resref.to_string()));
    Some(edit)
}

/// What Export… leaves behind of a window's pictures (`names`, those the
/// module does not hold) that a client could miss, as a line for the log:
/// a picture from a hak (a client needs that hak), from this machine's
/// override, development or user folders, the Workshop, or one the game
/// finds nowhere. The game's own pictures are on every client and go
/// unsaid. `None` with nothing to say.
pub(crate) fn pictures_left_behind(
    resman: Option<&mg_resman::ResMan>,
    names: &[String],
) -> Option<String> {
    let every_client = |layer: &str| {
        layer.starts_with("key:")
            || layer.starts_with("module")
            || matches!(layer, "portraits" | "ambient" | "music")
    };
    let mut missed = Vec::new();
    for name in names {
        let origin = resman.and_then(|rm| {
            TYPES.iter().find_map(|ty| rm.origin(&ResKey::parse(name, *ty)?).map(str::to_owned))
        });
        match origin {
            Some(layer) if every_client(&layer) => {}
            Some(layer) => missed.push(format!("{name} ({})", origin_name(&layer))),
            None => missed.push(format!("{name} (found nowhere)")),
        }
    }
    (!missed.is_empty()).then(|| {
        format!(
            "Not exported, as the module doesn't hold {}: {}. A client without {} shows the \
             game's gui_error picture in {} place: give players the hak, or add the picture to \
             the module (Images › Add images from disk…)",
            if missed.len() == 1 { "it" } else { "them" },
            missed.join(", "),
            if missed.len() == 1 { "it" } else { "them" },
            if missed.len() == 1 { "its" } else { "their" },
        )
    })
}

/// The picture the root's first draw layer stretches under the controls.
fn current_background(doc: &Value) -> Option<&str> {
    let first = doc["root"]["draw_list"].get(0)?;
    (first["type"] == 5 && first["order"] == -1).then(|| first["image"].as_str()).flatten()
}

/// A stretched picture under the window's controls: a draw layer of the
/// root column, painted before it.
pub(super) fn background(doc: &mut Value, s: &Settings, name: &str) {
    let geometry = resolved(&doc["geometry"], s);
    let title = doc["title"] != json!(false);
    let w = geometry["w"].as_f64().unwrap_or(400.0) - 16.0;
    let h = geometry["h"].as_f64().unwrap_or(300.0) - 16.0 - if title { 33.0 } else { 0.0 };
    let item = json!({"type":5,"enabled":true,"order":-1,"render":0,"arrayBinds":false,
        "rect":{"x":0.0,"y":0.0,"w":w.max(1.0),"h":h.max(1.0)},
        "image":name,"image_aspect":5,"image_halign":0,"image_valign":0});
    let Some(root) = doc.get_mut("root").filter(|r| r.is_object()) else { return };
    if !root["draw_list"].is_array() {
        root["draw_list"] = json!([]);
    }
    if let Some(layers) = root["draw_list"].as_array_mut() {
        layers.insert(0, item);
    }
}

/// The hak pictures go into: the one chosen (in the user's hak folder) or a
/// new one asked for. A hak saved elsewhere is copied into the hak folder.
fn chosen_hak(app: &mut Moonglow, name: &str, choice: &Option<String>) -> Option<PathBuf> {
    let Some(folder) = app.install.as_ref().and_then(mg_resman::GameInstall::user_hak_dir) else {
        app.log.error("Haks need the game's user folder (Tools › Options)");
        return None;
    };
    let path = match choice {
        // A listed hak the game finds elsewhere (its data, the Workshop) isn't
        // ours to write: one made in the hak folder would hide it.
        Some(hak) => {
            let path = folder.join(format!("{hak}.hak"));
            if !path.is_file() {
                app.log.error(format!(
                    "{hak}.hak isn't in the game's hak folder, so it can't take pictures: choose \
                     a new hak"
                ));
                return None;
            }
            path
        }
        None => app
            .dialogs
            .save_file(FileKind::Hak, Some(&folder.join(format!("{name}_images.hak"))))?,
    };
    // As the hak editor's Save: never into the game's folder.
    if app.install.as_ref().is_some_and(|gi| path.starts_with(&gi.root)) {
        app.log.error("Moonglow doesn't write into the game's folder: save the hak elsewhere");
        return None;
    }
    Some(path)
}

/// Writes pictures into the hak at `path`, at once (a hak isn't the
/// module's: Undo doesn't take it back), and says in the log which hak was
/// written and where the copy of what it was is kept. Whether it was written.
///
/// A hak open in the hak editor takes them in that document instead (its
/// file written under it would be lost at its Save): saved at once, as the
/// editor's Save saves it (a `.bak` kept, as here), when it had no unsaved
/// changes; else, one step of its Undo, left for you to save there, which
/// is said (and is not written: false).
fn write_hak(app: &mut Moonglow, path: &Path, pictures: Vec<(ResKey, Source)>) -> bool {
    if let Some(doc) = app.haks.iter_mut().find(|d| d.hak.path.as_deref() == Some(path)) {
        let (id, unsaved) = (doc.id, doc.hak.is_dirty());
        let file = path.file_name().unwrap_or_default().to_string_lossy().into_owned();
        let added = match doc.hak.add_sources_as("Add pictures", pictures) {
            Ok(added) => added,
            Err(e) => {
                app.log.error(format!("Could not add the pictures to {file}: {e}"));
                return false;
            }
        };
        doc.changed();
        let what = format!(
            "Added {} to {file} in the hak editor, replaced {}",
            count(added.added, "picture"),
            added.replaced
        );
        if unsaved {
            app.log.warn(format!(
                "{what} (Undo there takes them out). It has other unsaved changes, so it isn't \
                 saved: save it there for the game to find them"
            ));
            return false;
        }
        // (As a hak written here: the copy of what it was kept beside it.)
        let bak = path.with_extension("hak.bak");
        if let Err(e) = std::fs::copy(path, &bak) {
            app.log.error(format!("Could not keep {}: {e}", bak.display()));
            return false;
        }
        app.log.info(format!("{what}; the hak as it was is kept as {}", bak.display()));
        return crate::hak_view::save(app, id, false);
    }
    let kept = path.exists().then(|| path.with_extension("hak.bak"));
    match write_into(path, pictures) {
        Ok(added) => {
            let kept = kept.map_or(String::new(), |bak| {
                format!("; the hak as it was is kept as {}", bak.display())
            });
            app.log.info(format!(
                "Added {} to {}, replaced {}{kept}",
                count(added.added, "picture"),
                path.display(),
                added.replaced
            ));
            true
        }
        Err(e) => {
            app.log.error(format!("Could not write {}: {e}", path.display()));
            false
        }
    }
}

/// After writing a hak: one outside the hak folder is copied there; one the
/// module doesn't list yet is added at the top of its list (Undo takes it
/// off), one it lists is read again. False when the hak can't be used.
fn use_hak(app: &mut Moonglow, path: &Path, edits: &mut Vec<Edit>) -> bool {
    let Some(folder) = app.install.as_ref().and_then(mg_resman::GameInstall::user_hak_dir) else {
        return false;
    };
    let hak = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    if path.parent() != Some(folder.as_path()) {
        let user = folder.parent().unwrap_or(&folder).to_owned();
        match mg_module::attach::placements(&user, &[path.to_owned()]) {
            Ok(places) => {
                if places.iter().any(|p| p.there == mg_module::attach::There::Different) {
                    app.log.error(format!("Another {hak}.hak is in the hak folder: not replaced"));
                    return false;
                }
                if let Err(e) = mg_module::attach::copy(&places, false) {
                    app.log.error(e);
                    return false;
                }
            }
            Err(e) => {
                app.log.error(e);
                return false;
            }
        }
    }
    let Some(ws) = &mut app.ws else { return false };
    if crate::listed_haks(ws).iter().any(|h| h.eq_ignore_ascii_case(&hak)) {
        app.actions.push(Action::ReloadResources);
    } else if let Ok(info) = ws.doc(&crate::module_props::info_key()) {
        edits.push(Edit::SetField {
            key: crate::module_props::info_key(),
            path: mg_edit::GffPath::root(),
            label: mg_schema::ifo::MOD_HAK_LIST.label.to_string(),
            value: Some(mg_module::attach::hak_list(&info.root, &[hak])),
        });
    }
    true
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
    let haks = crate::listed_haks(ws);
    // Pictures added to the module that no control shows yet.
    let unused: BTreeSet<String> = TYPES
        .iter()
        .flat_map(|ty| ws.module.keys_of(*ty))
        .map(|k| k.resref.to_string().to_lowercase())
        .filter(|n| !names.contains(n))
        .collect();
    egui::ScrollArea::vertical().id_salt("nui-images-page").show(ui, |ui| {
        widgets::section_heading(ui, "Images");
        ui.label(
            "The pictures this window shows and where the game finds each. Pictures from disk go \
             into the module while you work; players get them from a hak the module uses.",
        );
        if names.is_empty() && unused.is_empty() {
            ui.weak(
                "No pictures yet: an Image, an Image button or a draw layer's Image names one.",
            );
        }
        let background_now = current_background(doc).map(str::to_owned);
        let module_names: BTreeSet<String> =
            module_images.iter().map(|k| k.resref.to_string().to_lowercase()).collect();
        egui::Grid::new("nui-images").striped(true).num_columns(3).spacing([12.0, 6.0]).show(
            ui,
            |ui| {
                if !names.is_empty() || !unused.is_empty() {
                    widgets::table_heading(ui, "Picture");
                    widgets::table_heading(ui, "Found in");
                    ui.label("");
                    ui.end_row();
                }
                for picture in names.iter().chain(&unused) {
                    ui.label(picture);
                    let origin = app
                        .nui_assets
                        .origins
                        .iter()
                        .find(|(k, _)| k.rsplit_once('.').is_some_and(|(n, _)| n == picture))
                        .map(|(_, o)| o.clone());
                    let in_module = module_names.contains(picture);
                    let missing = origin.is_none() && !in_module && !unused.contains(picture);
                    match origin {
                        Some(origin) => ui.label(origin_name(&origin)).on_hover_text(origin),
                        None if unused.contains(picture) => ui.label("Module (not shown yet)"),
                        None if in_module => ui.label("Module"),
                        None => ui.colored_label(
                            ui.visuals().error_fg_color,
                            "Missing: the game shows its gui_error picture",
                        ),
                    };
                    let is_background = background_now.as_deref() == Some(picture.as_str());
                    if ui
                        .add_enabled(
                            !missing && !is_background,
                            egui::Button::new("Use as background").small(),
                        )
                        .on_hover_text("Stretched under the window's controls")
                        .on_disabled_hover_text(if is_background {
                            "It is the background"
                        } else {
                            "The picture is missing"
                        })
                        .clicked()
                    {
                        background(doc, s, picture);
                    }
                    ui.end_row();
                }
            },
        );
        if ui
            .button("Add images from disk…")
            .on_hover_text("Into the module, named as their files")
            .clicked()
        {
            for path in app.dialogs.open_files(FileKind::Images, None) {
                edits.extend(add_file(app, &path).map(|(_, edit)| edit));
            }
        }
        ui.separator();
        // A hak no longer listed isn't a choice.
        if state.images_hak.as_ref().is_some_and(|h| !haks.contains(h)) {
            state.images_hak = None;
        }
        let target =
            state.images_hak.as_ref().map_or("a new hak".to_owned(), |h| format!("{h}.hak"));
        ui.horizontal_wrapped(|ui| {
            // Only a module with haks has a choice to make.
            if !haks.is_empty() {
                let label = widgets::field_label(ui, "Into");
                let options: Vec<Option<String>> =
                    haks.iter().cloned().map(Some).chain(std::iter::once(None)).collect();
                widgets::dropdown("nui-images-hak")
                    .selected_text(&target)
                    .show(ui, &mut state.images_hak, &options, |hak| {
                        hak.as_ref().map_or_else(|| "a new hak".to_string(), |h| format!("{h}.hak"))
                    })
                    .labelled_by(label.id);
            }
            if ui.button(format!("Add images to {target}…")).clicked() {
                let files = app.dialogs.open_files(FileKind::Images, None);
                if !files.is_empty()
                    && let Some(path) = chosen_hak(app, name, &state.images_hak)
                    && let Some(ws) = &app.ws
                {
                    // Named as in the module: one the game can't read is shortened.
                    let resources: Result<Vec<_>, _> = files
                        .iter()
                        .map(|f| {
                            from_file(f, &ws.module).map(|(k, b, _)| (k, Source::Bytes(b.into())))
                        })
                        .collect();
                    match resources {
                        Ok(pictures) => {
                            if write_hak(app, &path, pictures) {
                                use_hak(app, &path, &mut edits);
                            }
                        }
                        Err(e) => app.log.error(e),
                    }
                }
            }
            if ui
                .add_enabled(
                    !module_images.is_empty(),
                    egui::Button::new(format!("Move module images into {target}")),
                )
                .on_hover_text(
                    "This window's pictures leave the module for the hak; Undo brings them back, \
                     the hak keeps them",
                )
                .on_disabled_hover_text("None of this window's pictures is in the module")
                .clicked()
                && let Some(path) = chosen_hak(app, name, &state.images_hak)
                && let Some(ws) = &app.ws
            {
                let resources: Vec<_> = module_images
                    .iter()
                    .filter_map(|k| Some((*k, Source::Bytes(ws.module.get(k)?.to_vec().into()))))
                    .collect();
                // The module keeps its pictures unless the hak is in use.
                if write_hak(app, &path, resources) {
                    if use_hak(app, &path, &mut edits) {
                        edits.extend(
                            module_images.iter().map(|k| Edit::SetResource { key: *k, data: None }),
                        );
                    }
                } else if app.haks.iter().any(|d| d.hak.path.as_deref() == Some(path.as_path())) {
                    app.log.info(
                        "The module keeps its pictures until the hak with them is saved: Move \
                         them again once it is",
                    );
                }
            }
        });
        ui.weak(
            "A hak's file is written at once, and Undo doesn't take it back: a copy of the hak \
             as it was is kept beside it (.hak.bak). A hak open in the hak editor takes them \
             there, saved unless it has other unsaved changes. A new hak is made in the game's \
             hak folder and added at the top of the module's haks.",
        );
    });
    edits
}

#[cfg(test)]
mod tests {
    use super::*;
    use mg_resman::{LayerClass, MemContainer, ResMan};

    /// Export… says which pictures it leaves behind that a client could
    /// miss: a hak's, this machine's override's, one found nowhere; the
    /// game's own go unsaid.
    #[test]
    fn export_says_the_pictures_a_client_could_miss() {
        let mut rm = ResMan::new();
        let layer = |name: &str, ty: ResType| {
            let mut c = MemContainer::new();
            c.insert(ResKey::parse(name, ty).unwrap(), b"picture".to_vec());
            c
        };
        rm.add(30, "hak:tdod_gui", LayerClass::Erf, layer("pic_hak", ResType::TGA));
        rm.add(20, "override", LayerClass::Directory, layer("pic_over", ResType::PNG));
        rm.add(10, "key:nwn_base", LayerClass::Key, layer("pic_game", ResType::DDS));
        let names = |n: &[&str]| n.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert_eq!(pictures_left_behind(Some(&rm), &names(&["pic_game"])), None);
        let said = pictures_left_behind(
            Some(&rm),
            &names(&["pic_game", "pic_hak", "pic_over", "pic_gone"]),
        )
        .unwrap();
        assert!(
            said.contains(
                "pic_hak (Hak: tdod_gui), pic_over (Override folder), pic_gone (found nowhere)"
            ),
            "{said}"
        );
        assert!(!said.contains("pic_game"), "{said}");
        assert!(said.contains("give players the hak"), "{said}");
    }
}
