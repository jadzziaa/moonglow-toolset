//! The resource browser: every resource in the load order (game, haks,
//! module) with the layer it comes from, and a read-only view of any of
//! them that can be copied into the module or saved to a file.

use std::path::PathBuf;

use egui::Ui;
use mg_core::ResType;
use mg_edit::{Command, Edit};
use mg_gff::{Gff, Struct, Value};
use mg_resman::ResKey;

use crate::text::decode;
use crate::{Action, FileKind, Moonglow, Tab};

/// How far an overridden copy's row is set in under its resource's.
const COPY_INDENT: f32 = 18.0;

/// A resource's bytes from the load order: the copy the game reads, or
/// (`layer`, by its label) the copy that layer has.
fn read_copy<'a>(
    rm: &'a mg_resman::ResMan,
    key: &ResKey,
    layer: Option<&str>,
) -> Option<std::borrow::Cow<'a, [u8]>> {
    match layer {
        None => rm.get(key).ok(),
        Some(label) => {
            let i = rm.layers().iter().position(|l| l.label == label)?;
            rm.get_in(i, key).ok()
        }
    }
}

/// A row of the browser's list: a copy of a resource and its layer's
/// label; for a copy that a higher layer's overrides, that layer's.
#[derive(Debug, Clone)]
struct Row {
    key: ResKey,
    layer: String,
    over: Option<String>,
}

/// The browser's index and filters.
#[derive(Debug, Default)]
pub struct Browser {
    /// Every copy of each resource, the one the game reads first and those
    /// it overrides under it; rebuilt when stale.
    entries: Vec<Row>,
    /// How many resources there are (the copies the game reads).
    resources: usize,
    types: Vec<ResType>,
    pub restype: Option<ResType>,
    pub filter: String,
    /// The copies that others override are listed under them (a builder's
    /// report: of a table in two haks, only the higher one was found).
    pub show_copies: bool,
    /// Indices into `entries` matching the filters, for `query`.
    shown: Vec<usize>,
    /// How many of those are overridden copies.
    shown_copies: usize,
    query: Option<(Option<ResType>, String, bool)>,
    /// The load order changed since the index was built.
    pub(crate) stale: bool,
    /// Export as Files writes compiled models as text.
    pub models_as_text: bool,
    /// 2DA views: StrRefs shown as numbers rather than their text.
    pub(crate) raw_strrefs: bool,
    /// 2DA views: only the rows a layer last changed (its index in the
    /// table's copies).
    pub rows_from: Option<usize>,
    /// The row the keys are at, among those listed.
    pub cursor: Option<usize>,
    /// The cursor moved: the list goes to it.
    reveal: bool,
}

impl Browser {
    pub fn new() -> Browser {
        Browser { stale: true, show_copies: true, ..Default::default() }
    }

    fn refresh(&mut self, app_resman: Option<&mg_resman::ResMan>) {
        if !self.stale {
            return;
        }
        self.stale = false;
        self.query = None;
        self.entries = match app_resman {
            Some(rm) => {
                let label = |i: usize| rm.layers()[i].label.clone();
                rm.every_copy()
                    .into_iter()
                    .flat_map(|(key, layers)| {
                        let top = label(layers[0]);
                        layers.into_iter().enumerate().map(move |(n, i)| Row {
                            key,
                            layer: label(i),
                            over: (n > 0).then(|| top.clone()),
                        })
                    })
                    .collect()
            }
            None => Vec::new(),
        };
        self.resources = self.entries.iter().filter(|r| r.over.is_none()).count();
        let mut types: Vec<ResType> = self.entries.iter().map(|r| r.key.restype).collect();
        types.sort_by_key(|t| t.extension().unwrap_or_default());
        types.dedup();
        self.types = types;
    }

    fn filtered(&mut self) -> &[usize] {
        let needle = mg_core::text::Needle::new(&self.filter);
        let query = (self.restype, needle.as_str().to_string(), self.show_copies);
        if self.query.as_ref() != Some(&query) {
            self.shown = self
                .entries
                .iter()
                .enumerate()
                .filter(|(_, r)| {
                    (query.2 || r.over.is_none())
                        && query.0.is_none_or(|t| r.key.restype == t)
                        && needle.finds(&r.key.resref.to_string())
                })
                .map(|(i, _)| i)
                .collect();
            self.shown_copies =
                self.shown.iter().filter(|&&i| self.entries[i].over.is_some()).count();
            self.query = Some(query);
            // (Another list: the keys start at its top.)
            self.cursor = self.cursor.map(|_| 0);
            self.reveal = self.cursor.is_some();
        }
        &self.shown
    }
}

pub(crate) fn ui(app: &mut Moonglow, ui: &mut Ui) {
    use crate::palette_view::{Keys, give_keys, keys_are};
    let pane = ui.max_rect();
    let Moonglow { browser, game, ws, actions, .. } = app;
    browser.refresh(game.as_ref().map(|g| &g.resman));
    if game.is_none() {
        ui.label("The browser needs the game data (Tools > Options).");
        return;
    }
    ui.horizontal(|ui| {
        let current = browser.restype.and_then(|t| t.extension()).unwrap_or("all types");
        let types: Vec<Option<ResType>> =
            std::iter::once(None).chain(browser.types.iter().copied().map(Some)).collect();
        crate::widgets::dropdown("browser-type").selected_text(current).show(
            ui,
            &mut browser.restype,
            &types,
            |t| t.map_or("all types", |t| t.extension().unwrap_or("?")),
        );
        crate::widgets::field_label(ui, "Name");
        // The Filter's keys, as the side panes have them (GitHub issue
        // 12): Find's (Ctrl+F) with the pointer over the browser go to it,
        // Escape clears it and then leaves it, Down goes into the list.
        let field_id = egui::Id::new("browser-filter-field");
        let over = ui.ctx().pointer_hover_pos().is_some_and(|p| pane.contains(p));
        let width = ui.spacing().text_edit_width;
        let left = crate::widgets::SearchField::new(field_id, &mut browser.filter)
            .find_when(over)
            .down()
            .width(width)
            .show(ui)
            .left;
        if left == Some(crate::widgets::FilterLeft::Down) {
            browser.cursor = Some(0);
            browser.reveal = true;
            give_keys(ui.ctx(), Keys::Browser);
        }
        if ui.button("Refresh").clicked() {
            browser.stale = true;
        }
        ui.checkbox(&mut browser.show_copies, "Overridden copies").on_hover_text(
            "Under each resource, the copies of it lower in the load order (another hak's, \
             override's, the game's), which the one above overrides: each can be opened, \
             read, copied to the module and saved",
        );
    });
    let count = browser.filtered().len();
    // The keys in the list, from a click on a row or Down from the
    // Filter: a row at a time, a page, the ends; Enter opens.
    let mut entered = false;
    if keys_are(ui.ctx(), Keys::Browser) && !ui.ctx().egui_wants_keyboard_input() && count > 0 {
        use egui::Key;
        let steps = [
            (Key::ArrowDown, 1),
            (Key::ArrowUp, -1),
            (Key::PageDown, 10),
            (Key::PageUp, -10),
            (Key::End, isize::MAX / 2),
            (Key::Home, isize::MIN / 2),
        ];
        let pressed = ui.input_mut(|i| {
            steps.into_iter().find(|(k, _)| i.consume_key(egui::Modifiers::NONE, *k))
        });
        if let Some((_, by)) = pressed {
            let last = count as isize - 1;
            let at = browser.cursor.map_or(0, |at| (at as isize).saturating_add(by));
            browser.cursor = Some(at.clamp(0, last) as usize);
            browser.reveal = true;
        }
        entered = browser.cursor.is_some()
            && ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::Enter));
        if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::Escape)) {
            give_keys(ui.ctx(), Keys::Area);
        }
    }
    browser.cursor = browser.cursor.filter(|c| *c < count);
    ui.horizontal(|ui| {
        // (The resources, the copies the game reads; and the copies they
        // override, listed under them.)
        let found = count - browser.shown_copies;
        let copies = match browser.shown_copies {
            0 => String::new(),
            1 => ", 1 overridden copy".into(),
            n => format!(", {n} overridden copies"),
        };
        ui.weak(format!(
            "{found} of {} resources{copies}; double-click to open",
            browser.resources
        ));
        // (Narrowed by a type or a name: not the whole game by a slip.)
        let narrowed = found > 0 && found < browser.resources;
        if ui
            .add_enabled(narrowed, egui::Button::new(format!("Export {found} as Files…")))
            .on_hover_text(
                "The resources listed, as loose files in a folder (name.ext), as the load \
                 order has them (not the copies they override). Choose a type or type part \
                 of a name first.",
            )
            .clicked()
        {
            let keys = (browser.shown.iter().map(|&i| &browser.entries[i]))
                .filter(|r| r.over.is_none())
                .map(|r| r.key)
                .collect();
            actions.push(Action::SaveResources(keys));
        }
        if browser.shown.iter().any(|&i| browser.entries[i].key.restype == ResType::MDL) {
            ui.checkbox(&mut browser.models_as_text, "Models as text").on_hover_text(
                "Export as Files writes compiled models as the text they compile from",
            );
        }
    });
    ui.separator();
    let row_height = ui.text_style_height(&egui::TextStyle::Body) + 4.0;
    let in_module = |k: &ResKey| ws.as_ref().is_some_and(|w| w.module.contains(k));
    let shown = &browser.shown;
    let mut list = egui::ScrollArea::vertical().auto_shrink([false, false]);
    if std::mem::take(&mut browser.reveal)
        && let Some(at) = browser.cursor
    {
        let pitch = row_height + ui.spacing().item_spacing.y;
        let above = (ui.available_height() / 2.0 - pitch).max(0.0);
        list = list.vertical_scroll_offset((at as f32 * pitch - above).max(0.0));
    }
    let cursor = browser.cursor;
    let mut clicked = None;
    let out = list.show_rows(ui, row_height, count, |ui, range| {
        for (at, &i) in range.clone().zip(&shown[range]) {
            let Row { key: k, layer, over } = &browser.entries[i];
            ui.horizontal(|ui| {
                let at_cursor = cursor == Some(at) && keys_are(ui.ctx(), Keys::Browser);
                // A copy another overrides: under it, set in, its own
                // layer's (opened as that layer has it).
                let mut name = egui::RichText::new(k.to_string()).monospace();
                if over.is_some() {
                    ui.add_space(COPY_INDENT);
                    name = name.weak();
                }
                let r = ui.selectable_label(at_cursor, name);
                match over {
                    Some(over) => _ = ui.weak(format!("{layer}, overridden by {over}")),
                    None => _ = ui.weak(layer),
                }
                let copy = over.as_ref().map(|_| layer.clone());
                let tab = || match (&copy, Tab::for_resource(*k)) {
                    (Some(layer), _) => Tab::ResourceCopy { key: *k, layer: layer.clone() },
                    (None, Some(t @ Tab::Model(_))) => t,
                    (None, Some(t)) if in_module(k) => t,
                    _ => Tab::Resource(*k),
                };
                if r.clicked() {
                    clicked = Some(at);
                }
                if r.double_clicked() || (entered && cursor == Some(at)) {
                    actions.push(Action::OpenTab(tab()));
                }
                r.context_menu(|ui| {
                    if ui.button("Open").clicked() {
                        actions.push(Action::OpenTab(tab()));
                    }
                    // (The viewer shows the copy the game reads.)
                    if copy.is_none()
                        && k.restype != ResType::MDL
                        && crate::model_view::previewable(k.restype)
                        && ui.button("Preview").on_hover_text("See it in the 3D viewer").clicked()
                    {
                        actions.push(Action::OpenTab(Tab::Model(*k)));
                    }
                    if ui.button("Save As…").on_hover_text("As a file, as it is").clicked() {
                        actions.push(Action::SaveResource(*k, copy.clone()));
                    }
                    if k.restype == ResType::MDL
                        && ui
                            .button("Save As Text…")
                            .on_hover_text("A compiled model as the text it compiles from")
                            .clicked()
                    {
                        actions.push(Action::SaveModelText(*k, copy.clone()));
                    }
                });
            });
        }
    });
    if let Some(at) = clicked {
        browser.cursor = Some(at);
        give_keys(ui.ctx(), Keys::Browser);
    }
    crate::palette_view::keys_outline(ui, out.inner_rect, Keys::Browser);
}

/// Where a table shown in the browser is to be edited.
enum EditTable {
    InModule,
    Copy(crate::table_view::Home),
    CopyInFile,
    /// The table a hak of the module has: the hak's file.
    InHak(std::path::PathBuf),
}

/// The module's haks that have `key`, highest first: each one's name and
/// its file, where that is the builder's own (not one of the game's).
fn haks_with(app: &Moonglow, key: ResKey) -> Vec<(String, std::path::PathBuf)> {
    let (Some(game), Some(install)) = (&app.game, &app.install) else { return Vec::new() };
    let dirs = install.hak_dirs();
    let mut layers: Vec<_> = game.resman.layers().iter().collect();
    layers.sort_by_key(|l| std::cmp::Reverse(l.priority));
    layers
        .into_iter()
        .filter_map(|l| {
            let name = l.label.strip_prefix("hak:")?;
            if !l.container.contains(&key) {
                return None;
            }
            let path = dirs.iter().map(|d| d.join(format!("{name}.hak"))).find(|p| p.is_file())?;
            (!path.starts_with(&install.root)).then(|| (name.to_string(), path))
        })
        .collect()
}

/// A 2DA's Edit menu: the module's own table as it is, or a copy of the
/// one the game reads, in the module, in a hak open in the hak editor,
/// or in a file. (The game's own tables are never changed.)
fn table_menu(app: &Moonglow, ui: &mut Ui, key: ResKey) -> Option<EditTable> {
    use crate::table_view::Home;
    let mut out = None;
    let in_module = app.ws.as_ref().is_some_and(|w| w.module.contains(&key));
    ui.menu_button("Edit…", |ui| {
        if in_module {
            if ui.button("The Module's Table").clicked() {
                out = Some(EditTable::InModule);
            }
        } else if app.ws.is_some()
            && ui
                .button("A Copy in the Module")
                .on_hover_text("The module's own table from then on, over the haks'")
                .clicked()
        {
            out = Some(EditTable::Copy(Home::Module));
        }
        // The table as a hak of the module has it: edited in the hak, which
        // the hak editor opens.
        for (name, path) in haks_with(app, key) {
            let open = app.haks.iter().any(|d| d.hak.path.as_deref() == Some(&path));
            let tip = match open {
                true => "The hak's own table: saved into the hak, which its Save writes",
                false => {
                    "The hak's own table: the hak is opened in the hak editor, the table is \
                     saved into it, and the hak's Save writes its file"
                }
            };
            if ui.button(format!("The Table of {name}.hak")).on_hover_text(tip).clicked() {
                out = Some(EditTable::InHak(path));
            }
        }
        let own: Vec<_> = haks_with(app, key).into_iter().map(|(_, path)| path).collect();
        for doc in &app.haks {
            // (Its own table is offered above.)
            if doc.hak.path.as_ref().is_some_and(|p| own.contains(p)) {
                continue;
            }
            let name = doc.title();
            let name = name.trim_end_matches(" *");
            let has = doc.hak.items().iter().any(|i| i.key == key);
            let label = match has {
                true => format!("A Copy in {name} (in place of its own)"),
                false => format!("A Copy in {name}"),
            };
            if ui.button(label).on_hover_text("A hak open in the hak editor").clicked() {
                out = Some(EditTable::Copy(Home::Hak(doc.id)));
            }
        }
        if ui.button("A Copy in a File…").clicked() {
            out = Some(EditTable::CopyInFile);
        }
    });
    out
}

/// A copy of a table that a hak of the module has (`layer`, `hak:name`),
/// where the hak is the builder's own: a button to edit it in the hak.
fn hak_table_button(app: &Moonglow, ui: &mut Ui, key: ResKey, layer: &str) -> Option<EditTable> {
    let name = layer.strip_prefix("hak:")?;
    let (_, path) = haks_with(app, key).into_iter().find(|(n, _)| n == name)?;
    let tip = "The hak's own table: the hak is opened in the hak editor (if it isn't), the \
               table is saved into it, and the hak's Save writes its file";
    let edit = ui.button(format!("Edit in {name}.hak")).on_hover_text(tip).clicked();
    edit.then_some(EditTable::InHak(path))
}

/// Save As on a resource of the load order (the copy the game reads, or
/// the one `layer` has): its bytes to a file; or, `as_text`, a compiled
/// model as its text (one that is text already is saved as it is).
pub(crate) fn save_as(app: &mut Moonglow, key: ResKey, layer: Option<&str>, as_text: bool) {
    let Some(game) = &app.game else { return };
    let Some(data) = read_copy(&game.resman, &key, layer) else {
        match layer {
            Some(layer) => app.log.error(format!("{layer} has no {key}.")),
            None => app.log.error(format!("{key} is not in the load order.")),
        }
        return;
    };
    let data = match as_text.then(|| mg_mdl::decompile(&data)) {
        Some(Ok(text)) => text.into_bytes().into(),
        Some(Err(mg_mdl::DecompileError::NotBinary)) | None => data,
        Some(Err(e)) => {
            app.log.error(format!("{key} can't be written as text: {e}"));
            return;
        }
    };
    let Some(path) = app.dialogs.save_file(FileKind::Any, Some(&PathBuf::from(key.to_string())))
    else {
        return;
    };
    match std::fs::write(&path, &data) {
        Ok(()) => app.log.info(format!("Saved {key} to {}", path.display())),
        Err(e) => app.log.error(format!("Could not write {}: {e}", path.display())),
    }
}

/// Export as Files: the resources listed, each as `name.ext` in a folder
/// the user chooses.
pub(crate) fn export(app: &mut Moonglow, keys: &[ResKey]) {
    let Some(game) = &app.game else { return };
    let Some(dir) = app.dialogs.pick_folder("Export as files into", app.export_dir.as_deref())
    else {
        return;
    };
    let (mut written, mut as_text) = (0, 0);
    for key in keys {
        let Ok(data) = game.resman.get(key) else { continue };
        // (Asked for: a compiled model as its text; one that can't be
        // read so goes out as it is.)
        let text = (app.browser.models_as_text).then(|| model_text(*key, &data)).flatten();
        as_text += usize::from(text.is_some());
        let data = text.map_or(data, |t| t.into_bytes().into());
        if let Err(e) = key.path_in(&dir).and_then(|path| std::fs::write(path, &data)) {
            app.log.error(format!("Could not write to {}: {e}", dir.display()));
            return;
        }
        written += 1;
    }
    let models = match as_text {
        0 => String::new(),
        n => format!(" ({n} compiled models as text)"),
    };
    app.log.info(format!("Exported {written} resources to {}{models}", dir.display()));
    app.export_dir = Some(dir);
}

/// A resource read and parsed once for its viewer.
#[derive(Debug)]
pub(crate) struct Viewed {
    data: Vec<u8>,
    origin: String,
    content: Content,
}

#[derive(Debug)]
enum Content {
    Gff(Gff),
    TwoDa(mg_module::table_layers::Layered),
    Script(String),
    Text(String),
    Binary(String),
    Error(String),
}

impl Viewed {
    /// The copy the game reads, or (`layer`, by its label) the one that
    /// layer has.
    fn load(game: &mg_rules::GameData, key: ResKey, layer: Option<&str>) -> Option<Viewed> {
        let rm = &game.resman;
        let at = match layer {
            Some(label) => rm.layers().iter().position(|l| l.label == label)?,
            None => rm.find(&key)?,
        };
        let data = rm.get_in(at, &key).ok()?.into_owned();
        let origin = rm.layers()[at].label.clone();
        let codepage = game.codepage();
        let content = if key.restype.is_gff() {
            Gff::read(&data).map_or_else(|e| Content::Error(e.to_string()), Content::Gff)
        } else if key.restype == ResType::TWODA {
            // (A copy that another overrides: as it stands on those below.)
            match mg_module::table_layers::layered_from(rm, &key, codepage, at) {
                Some(l) => Content::TwoDa(l),
                None => match mg_2da::TwoDa::parse(&data, codepage) {
                    Err(e) => Content::Error(e.to_string()),
                    Ok(_) => Content::Error("the copy the game reads can't be read".into()),
                },
            }
        } else if key.restype == ResType::NSS {
            // (Shown and copied with plain newlines, as the script editor
            // has them; Copy to Module and Save copy the bytes.)
            Content::Script(crate::text::to_editor(&crate::text::decode_script(&data)).0)
        } else if is_text(key.restype, &data) {
            Content::Text(crate::text::to_editor(&codepage.decode(&data)).0)
        } else if let Some(text) = model_text(key, &data) {
            Content::Text(text)
        } else {
            Content::Binary(hex(&data[..data.len().min(512)]))
        };
        Some(Viewed { data, origin, content })
    }
}

/// A resource's bytes as something to read: a GFF's fields, text, or the
/// first bytes in hexadecimal.
#[derive(Debug)]
pub(crate) enum Plain {
    Gff(Box<Gff>),
    Text(String),
    Error(String),
}

/// What `data`, a resource of `key`'s type, shows as.
pub(crate) fn plain(key: ResKey, data: &[u8]) -> Plain {
    if key.restype.is_gff() {
        match Gff::read(data) {
            Ok(g) => Plain::Gff(Box::new(g)),
            Err(e) => Plain::Error(e.to_string()),
        }
    } else if is_text(key.restype, data) {
        // (Its bytes as Latin-1 where they aren't UTF-8.)
        let latin = || data.iter().map(|&b| b as char).collect();
        Plain::Text(String::from_utf8(data.to_vec()).unwrap_or_else(|_| latin()))
    } else if let Some(text) = model_text(key, data) {
        Plain::Text(text)
    } else {
        Plain::Text(hex(&data[..data.len().min(512)]))
    }
}

/// A compiled model as the text it compiles from (`mg_mdl::decompile`);
/// nothing for anything else, or a model that can't be read.
pub(crate) fn model_text(key: ResKey, data: &[u8]) -> Option<String> {
    (key.restype == ResType::MDL).then(|| mg_mdl::decompile(data).ok()).flatten()
}

/// Draws a [`Plain`] view, scrolling.
pub(crate) fn plain_ui(ui: &mut Ui, key: ResKey, view: &Plain) {
    egui::ScrollArea::both().id_salt(("plain", key)).auto_shrink([false, false]).show(ui, |ui| {
        match view {
            Plain::Gff(g) => gff_tree(ui, &g.root, &key.to_string()),
            Plain::Text(text) => {
                ui.monospace(text);
            }
            Plain::Error(e) => {
                ui.colored_label(ui.visuals().error_fg_color, e);
            }
        }
    });
}

/// Read-only view of a resource from the load order: the copy the game
/// reads, or (`layer`, by its label) the copy that layer has, which a
/// higher one overrides.
pub(crate) fn resource_ui(app: &mut Moonglow, ui: &mut Ui, key: ResKey, layer: Option<&str>) {
    let Some(game) = &app.game else { return };
    let at = (key, layer.map(str::to_string));
    let viewed = match app.viewed.get(&at) {
        Some(v) => v.clone(),
        None => match Viewed::load(game, key, layer) {
            Some(v) => {
                let v = std::sync::Arc::new(v);
                app.viewed.insert(at, v.clone());
                v
            }
            None => {
                match layer {
                    Some(layer) => ui.label(format!("{layer} has no {key} now.")),
                    None => ui.label(format!("{key} is not in the load order.")),
                };
                return;
            }
        },
    };
    // (A copy's: what overrides it, which the game reads.)
    let over = layer.and_then(|_| game.resman.origin(&key)).map(str::to_string);
    let mut copy = false;
    let mut save = false;
    let mut edit_table = None;
    ui.horizontal(|ui| {
        ui.strong(key.to_string());
        ui.weak(format!("from {}, {} bytes", viewed.origin, viewed.data.len()));
        let can_copy = app.ws.as_ref().is_some_and(|w| !w.module.contains(&key));
        copy = ui.add_enabled(can_copy, egui::Button::new("Copy to Module")).clicked();
        save = ui.button("Save As…").clicked();
        if key.restype == ResType::TWODA {
            edit_table = match layer {
                None => table_menu(app, ui, key),
                // A copy a hak of the module has: edited in that hak.
                Some(layer) => hak_table_button(app, ui, key, layer),
            };
        }
    });
    if let Some(over) = &over {
        ui.weak(format!("Overridden by {over}, whose copy the game reads."));
    }
    ui.separator();
    match edit_table {
        Some(EditTable::InModule) => _ = crate::table_view::open_in_module(app, key),
        Some(EditTable::Copy(home)) => _ = crate::table_view::edit_copy(app, key, home),
        Some(EditTable::CopyInFile) => crate::table_view::edit_copy_in_file(app, key),
        Some(EditTable::InHak(path)) => {
            if let Some(hak) = crate::hak_view::open_at(app, &path) {
                crate::table_view::open_in_hak(app, hak, key);
            }
        }
        None => {}
    }
    if copy {
        app.actions.push(Action::Apply(Command::new(
            format!("Copy {key} to the module"),
            vec![Edit::SetResource { key, data: Some(viewed.data.clone()) }],
        )));
        if let Some(tab) = Tab::for_resource(key) {
            app.actions.push(Action::OpenTab(tab));
        }
    }
    if save
        && let Some(path) =
            app.dialogs.save_file(FileKind::Any, Some(&PathBuf::from(key.to_string())))
    {
        match std::fs::write(&path, &viewed.data) {
            Ok(()) => app.log.info(format!("Saved {key} to {}", path.display())),
            Err(e) => app.log.error(format!("Could not write {}: {e}", path.display())),
        }
    }
    if let Content::TwoDa(l) = &viewed.content {
        two_da(app, ui, key, layer, l);
        return;
    }
    let palette = crate::script_view::Palette::for_ui(&app.settings.script_style, ui);
    egui::ScrollArea::both().auto_shrink([false, false]).show(ui, |ui| match &viewed.content {
        Content::Gff(g) => gff_tree(ui, &g.root, &key.to_string()),
        Content::TwoDa(_) => {}
        Content::Script(text) => {
            let job = crate::script_view::highlight(text, &palette);
            // (On the script editor's background, where one is chosen:
            // its colors are chosen for it.)
            match palette.background {
                Some(fill) => _ = egui::Frame::new().fill(fill).show(ui, |ui| ui.label(job)),
                None => _ = ui.label(job),
            }
        }
        Content::Text(text) | Content::Binary(text) => {
            ui.monospace(text);
        }
        Content::Error(e) => {
            ui.colored_label(ui.visuals().error_fg_color, e);
        }
    });
}

fn is_text(t: ResType, data: &[u8]) -> bool {
    matches!(
        t,
        ResType::NSS
            | ResType::JUI
            | ResType::TXT
            | ResType::INI
            | ResType::SET
            | ResType::TXI
            | ResType::MTR
            | ResType::SHD
            | ResType::LUA
    ) || (!data.is_empty()
        && data.iter().take(4096).all(|b| b.is_ascii_graphic() || b.is_ascii_whitespace()))
}

fn hex(data: &[u8]) -> String {
    data.chunks(16)
        .enumerate()
        .map(|(i, row)| {
            let bytes: Vec<String> = row.iter().map(|b| format!("{b:02x}")).collect();
            let text: String =
                row.iter().map(|&b| if b.is_ascii_graphic() { b as char } else { '.' }).collect();
            format!("{:08x}  {:<47}  {text}", i * 16, bytes.join(" "))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn value_text(v: &Value) -> String {
    match v {
        Value::Byte(x) => x.to_string(),
        Value::Char(x) => x.to_string(),
        Value::Word(x) => x.to_string(),
        Value::Short(x) => x.to_string(),
        Value::Dword(x) => x.to_string(),
        Value::Int(x) => x.to_string(),
        Value::Dword64(x) => x.to_string(),
        Value::Int64(x) => x.to_string(),
        Value::Float(x) => x.to_string(),
        Value::Double(x) => x.to_string(),
        Value::String(b) | Value::ResRef(b) => format!("{:?}", decode(b)),
        Value::LocString(ls) => {
            let mut parts: Vec<String> = Vec::new();
            if !ls.strref.is_none() {
                parts.push(format!("StrRef {}", ls.strref.0));
            }
            parts.extend(ls.strings.iter().map(|(k, b)| format!("[{}] {:?}", k.0, decode(b))));
            parts.join(", ")
        }
        Value::Void(b) => format!("{} bytes", b.len()),
        Value::Struct(_) | Value::List(_) => String::new(),
    }
}

/// A GFF struct as a read-only tree.
fn gff_tree(ui: &mut Ui, s: &Struct, id: &str) {
    for (i, f) in s.fields.iter().enumerate() {
        let label = f.label.to_string_lossy();
        let child_id = format!("{id}/{i}");
        match &f.value {
            Value::Struct(child) => {
                egui::CollapsingHeader::new(format!("{label} (struct {})", child.id))
                    .id_salt(&child_id)
                    .show(ui, |ui| gff_tree(ui, child, &child_id));
            }
            Value::List(items) => {
                egui::CollapsingHeader::new(format!("{label} (list, {} items)", items.len()))
                    .id_salt(&child_id)
                    .show(ui, |ui| {
                        for (j, item) in items.iter().enumerate() {
                            let item_id = format!("{child_id}/{j}");
                            egui::CollapsingHeader::new(format!("[{j}] struct {}", item.id))
                                .id_salt(&item_id)
                                .show(ui, |ui| gff_tree(ui, item, &item_id));
                        }
                    });
            }
            v => {
                ui.horizontal(|ui| {
                    ui.label(&label);
                    ui.weak(v.field_type().json_name());
                    ui.monospace(value_text(v));
                });
            }
        }
    }
}

/// A 2DA as a table; only the rows in view are laid out. With copies of
/// it in several layers (haks over the game's), each row says which layer
/// it comes from, and the cells a layer changed are marked, with what they
/// were below. Cells of StrRef columns show their text. As in the 2DA
/// editor, the column names stay in sight over the rows and the rows'
/// numbers beside them, and a column is as wide as the edge of its name
/// was dragged to (here or there: the widths are the table's).
fn two_da(
    app: &mut Moonglow,
    ui: &mut Ui,
    key: ResKey,
    layer: Option<&str>,
    l: &mg_module::table_layers::Layered,
) {
    use crate::table_view::{
        Edge, NUMBER, column_edge, column_widths, fit_width, set_column_width,
    };
    use mg_module::table_layers::is_strref_column;
    let Moonglow { browser, game, settings, .. } = app;
    let t = l.table();
    let layered = l.copies.len() > 1;
    ui.horizontal(|ui| {
        ui.checkbox(&mut browser.raw_strrefs, "StrRefs as numbers").on_hover_text(
            "Name, Description and other StrRef columns show their text unless this is on",
        );
        if layered {
            let label = |i: Option<usize>| match i {
                None => "All rows".to_string(),
                Some(i) => format!("Rows from {}", l.copies[i].0),
            };
            browser.rows_from = browser.rows_from.filter(|&i| i < l.copies.len());
            let options: Vec<Option<usize>> =
                std::iter::once(None).chain((0..l.copies.len()).map(Some)).collect();
            crate::widgets::dropdown("2da-rows-from").selected_text(label(browser.rows_from)).show(
                ui,
                &mut browser.rows_from,
                &options,
                |i| label(*i),
            );
        }
    });
    if layered {
        let parts: Vec<String> = l
            .summary()
            .iter()
            .rev()
            .map(|(label, added, changed)| match (added, changed) {
                (a, 0) => format!("{label}: {a} rows"),
                (0, c) => format!("{label}: changes {c}"),
                (a, c) => format!("{label}: adds {a}, changes {c}"),
            })
            .collect();
        ui.weak(format!("{} copies, lowest first. {}", l.copies.len(), parts.join("; ")));
    }
    let rows: Vec<usize> = match browser.rows_from.filter(|_| layered) {
        Some(o) => (0..t.len()).filter(|&r| l.row_origin(r) == o).collect(),
        None => (0..t.len()).collect(),
    };
    let strrefs: Vec<bool> =
        t.columns().iter().map(|c| !browser.raw_strrefs && is_strref_column(c)).collect();
    let game = game.as_deref();
    // What a cell shows: its string's text, in a StrRef column (and that
    // text), else its value.
    let shows = |r: usize, c: usize| {
        let raw = t.cell(r, c);
        let text = raw
            .filter(|_| strrefs[c])
            .and_then(|v| v.parse::<u32>().ok())
            .and_then(|n| game?.string(mg_core::StrRef(n)));
        (text.clone().unwrap_or_else(|| raw.unwrap_or("****").into()), text)
    };
    let bottom = l.copies.len() - 1;
    let changed = ui.visuals().warn_fg_color;
    let width = 110.0;
    let name = key.resref.to_string();
    let columns: Vec<String> = t.columns().to_vec();
    let widths = column_widths(settings, &name, &columns, width);
    let row_height = ui.text_style_height(&egui::TextStyle::Monospace) + 4.0;
    let fixed = |ui: &mut Ui, w: f32, text: egui::RichText| {
        ui.add_sized([w, row_height], egui::Label::new(text).truncate())
    };
    // The column names over the rows and the rows' numbers beside them go
    // with the rows' own scroll (a frame behind), kept here.
    let id = egui::Id::new(("2da-view", key, layer));
    let scroll: egui::Vec2 = ui.data(|d| d.get_temp(id)).unwrap_or_default();
    let hidden = egui::scroll_area::ScrollBarVisibility::AlwaysHidden;
    let still = egui::scroll_area::ScrollSource::NONE;
    let mut edge = None;
    ui.horizontal(|ui| {
        fixed(ui, NUMBER, egui::RichText::new(""));
        egui::ScrollArea::horizontal()
            .id_salt(id.with("head"))
            .auto_shrink([false, true])
            .scroll_bar_visibility(hidden)
            .scroll_source(still)
            .horizontal_scroll_offset(scroll.x)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    if layered {
                        fixed(ui, width, egui::RichText::new("From").strong());
                    }
                    for (c, column) in columns.iter().enumerate() {
                        let size = egui::vec2(widths[c], row_height);
                        let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
                        let text = egui::RichText::new(column).strong();
                        ui.put(rect, egui::Label::new(text).truncate().selectable(false))
                            .on_hover_text(format!(
                                "{column}\nDrag its right edge: its width (double click there: \
                                 as wide as its widest cell)"
                            ));
                        if let Some(act) = column_edge(ui, id.with(("edge", c)), rect, column) {
                            edge = Some((c, act));
                        }
                    }
                    // (Room for the rows' own scroll bar at the right.)
                    ui.add_space(ui.spacing().scroll.bar_width + 12.0);
                });
            });
    });
    match edge {
        Some((c, Edge::Width(w))) => set_column_width(settings, &name, &columns[c], w),
        Some((c, Edge::Fit)) => {
            let w = fit_width(ui, &columns[c], rows.iter().map(|&r| shows(r, c).0));
            set_column_width(settings, &name, &columns[c], w);
        }
        None => {}
    }
    let room = ui.available_height();
    let bar = ui.spacing().scroll.bar_width + ui.spacing().scroll.bar_inner_margin;
    let shown = ui
        .horizontal_top(|ui| {
            let gutter = egui::vec2(NUMBER, (room - bar).max(row_height));
            let down = egui::Layout::top_down(egui::Align::Min);
            ui.allocate_ui_with_layout(gutter, down, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt(id.with("numbers"))
                    .auto_shrink([false, false])
                    .scroll_bar_visibility(hidden)
                    .scroll_source(still)
                    .vertical_scroll_offset(scroll.y)
                    .show_rows(ui, row_height, rows.len(), |ui, range| {
                        for &r in &rows[range] {
                            fixed(
                                ui,
                                NUMBER,
                                egui::RichText::new(r.to_string()).weak().monospace(),
                            );
                        }
                    });
            });
            ui.vertical(|ui| {
                egui::ScrollArea::both()
                    .id_salt(id.with("rows"))
                    .auto_shrink([false, false])
                    .show_rows(ui, row_height, rows.len(), |ui, range| {
                        for &r in &rows[range] {
                            ui.horizontal(|ui| {
                                let origin = l.row_origin(r);
                                if layered {
                                    let text = egui::RichText::new(&l.copies[origin].0);
                                    fixed(
                                        ui,
                                        width,
                                        if origin < bottom { text } else { text.weak() },
                                    );
                                }
                                for (c, &w) in widths.iter().enumerate() {
                                    let raw = t.cell(r, c);
                                    let (shown, text) = shows(r, c);
                                    let mut rich = egui::RichText::new(shown).monospace();
                                    let cell_origin = l.cell_origin(r, c);
                                    // Changed, not added: the row is in the copy below.
                                    let was = (layered
                                        && cell_origin < bottom
                                        && r < l.copies[cell_origin + 1].1.len())
                                    .then(|| l.before(r, c))
                                    .flatten();
                                    if was.is_some() {
                                        rich = rich.color(changed);
                                    }
                                    let resp = fixed(ui, w, rich);
                                    if text.is_some() || was.is_some() {
                                        let mut tip = Vec::new();
                                        if text.is_some() {
                                            tip.push(format!("StrRef {}", raw.unwrap_or_default()));
                                        }
                                        if let Some((below, value)) = was {
                                            let from = &l.copies[cell_origin].0;
                                            let value = value.unwrap_or("****");
                                            tip.push(format!("Set by {from}; {value} in {below}"));
                                        }
                                        resp.on_hover_text(tip.join("\n"));
                                    }
                                }
                            });
                        }
                    })
            })
            .inner
        })
        .inner;
    ui.data_mut(|d| d.insert_temp(id, shown.state.offset));
}
