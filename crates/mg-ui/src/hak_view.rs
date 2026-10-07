//! The hak editor: a hak's resources, to add files and folders to, remove,
//! rename and extract, with undo, saved as a new archive moved over the old
//! one (`mg_module::hak_edit`). Tools › New Hak, Open Hak… and Build Hak
//! from Folder… open it. Saving a hak the module uses reloads it.

use std::collections::BTreeSet;
use std::path::PathBuf;

use egui::Ui;
use mg_module::hak_edit::{Added, Hak, Source};
use mg_resman::ResKey;

use crate::dialogs::FileKind;
use crate::{Moonglow, Tab};

/// A hak open in a tab.
#[derive(Debug)]
pub struct HakDoc {
    pub id: u32,
    pub hak: Hak,
    pub filter: String,
    pub selected: BTreeSet<ResKey>,
    /// The resource clicked last: Shift + click selects from it to the one
    /// clicked.
    anchor: Option<ResKey>,
    /// The resource being renamed, and its new name as typed.
    pub rename: Option<(ResKey, String)>,
    description: Option<String>,
    /// Bumped on every change, for the cached rows.
    revision: u64,
    rows: Option<(String, Sort, u64, Vec<usize>)>,
    /// The order of the list.
    pub sort: Sort,
    /// The resource shown under the list, as something to read.
    viewing: Option<(ResKey, crate::browser::Plain)>,
    /// The folder the hak was built from: Update from Folder reads it
    /// again.
    pub folder: Option<PathBuf>,
}

/// The order of a hak's list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Sort {
    #[default]
    Name,
    /// By type, then name.
    Type,
    /// The largest first.
    Size,
}

impl HakDoc {
    fn new(id: u32, hak: Hak) -> HakDoc {
        HakDoc {
            id,
            hak,
            filter: String::new(),
            selected: BTreeSet::new(),
            anchor: None,
            rename: None,
            description: None,
            revision: 0,
            rows: None,
            sort: Sort::Name,
            viewing: None,
            folder: None,
        }
    }

    pub fn title(&self) -> String {
        let name = self
            .hak
            .path
            .as_ref()
            .and_then(|p| p.file_name())
            .map_or("New hak".into(), |n| n.to_string_lossy().into_owned());
        format!("{name}{}", if self.hak.is_dirty() { " *" } else { "" })
    }

    /// The items shown, in the list's order: indices into the hak's items.
    fn rows(&mut self) -> Vec<usize> {
        let filter = self.filter.trim().to_ascii_lowercase();
        let fresh = self
            .rows
            .as_ref()
            .is_some_and(|(f, s, r, _)| *f == filter && *s == self.sort && *r == self.revision);
        if !fresh {
            let items = self.hak.items();
            let mut rows: Vec<usize> = (0..items.len())
                .filter(|&i| filter.is_empty() || items[i].key.to_string().contains(&filter))
                .collect();
            rows.sort_by_key(|&i| items[i].key.to_string());
            match self.sort {
                Sort::Name => {}
                // (Stable: by name within a type or a size.)
                Sort::Type => rows.sort_by_key(|&i| items[i].key.restype.to_string()),
                Sort::Size => rows.sort_by_key(|&i| std::cmp::Reverse(items[i].size)),
            }
            self.rows = Some((filter, self.sort, self.revision, rows));
        }
        self.rows.as_ref().map(|r| r.3.clone()).unwrap_or_default()
    }

    fn changed(&mut self) {
        self.revision += 1;
        self.description = None;
        let keys: BTreeSet<ResKey> = self.hak.items().iter().map(|i| i.key).collect();
        self.selected.retain(|k| keys.contains(k));
        self.viewing = None;
    }
}

fn open_tab(app: &mut Moonglow, hak: Hak) -> u32 {
    app.next_hak += 1;
    let id = app.next_hak;
    app.haks.push(HakDoc::new(id, hak));
    app.actions.push(crate::Action::OpenTab(Tab::Hak(id)));
    id
}

fn report(app: &mut Moonglow, added: &Added) {
    app.log.info(format!("Added {} files, replaced {}", added.added, added.replaced));
    for (path, why) in &added.skipped {
        app.log.warn(format!("Left out {}: {why}", path.display()));
    }
}

/// Tools › New Hak.
pub fn new_hak(app: &mut Moonglow) {
    open_tab(app, Hak::new());
}

/// Tools › Open Hak….
pub fn open_hak(app: &mut Moonglow) {
    let start = hak_dir(app);
    let Some(path) = app.dialogs.open_file(FileKind::Hak, start.as_deref()) else { return };
    if let Some(d) = app.haks.iter().find(|d| d.hak.path.as_deref() == Some(&path)) {
        let id = d.id;
        app.actions.push(crate::Action::OpenTab(Tab::Hak(id)));
        return;
    }
    match Hak::open(&path) {
        Ok(h) => {
            let id = open_tab(app, h);
            // (Built from a folder before: Update from Folder knows it.)
            let folder = app.settings.hak_folder(&path).map(std::path::Path::to_path_buf);
            if let Some(doc) = app.haks.iter_mut().find(|d| d.id == id) {
                doc.folder = folder;
            }
        }
        Err(e) => app.log.error(format!("Could not open {}: {e}", path.display())),
    }
}

/// Tools › Build Hak from Folder…: a new hak with a folder's files (and
/// those of the folders in it), to look over and save.
pub fn build_from_folder(app: &mut Moonglow) {
    let Some(dir) = app.dialogs.pick_folder("Build Hak from Folder", None) else { return };
    let mut hak = Hak::new();
    let added = hak.add_folder(&dir);
    report(app, &added);
    // Saved by default in the user's hak folder, named after the folder.
    let name = dir.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
    let id = open_tab(app, hak);
    if let Some(doc) = app.haks.iter_mut().find(|d| d.id == id) {
        doc.folder = Some(dir.clone());
    }
    app.suggested_hak = hak_dir(app).map(|d| d.join(format!("{name}.hak")));
}

/// The user's hak folder.
fn hak_dir(app: &Moonglow) -> Option<PathBuf> {
    app.install.as_ref()?.user_dir.as_ref().map(|u| u.join("hak"))
}

/// Saves a hak (asking where for a new one, or with `ask`). A hak the
/// module uses is read again.
pub(crate) fn save(app: &mut Moonglow, id: u32, ask: bool) -> bool {
    let Some(i) = app.haks.iter().position(|d| d.id == id) else { return false };
    let in_game = app.haks[i]
        .hak
        .path
        .as_ref()
        .zip(app.install.as_ref())
        .is_some_and(|(p, gi)| p.starts_with(&gi.root));
    let to = if ask || in_game || app.haks[i].hak.path.is_none() {
        let suggested = app
            .suggested_hak
            .take()
            .or_else(|| app.haks[i].hak.path.clone())
            .or_else(|| hak_dir(app).map(|d| d.join("new.hak")));
        match app.dialogs.save_file(FileKind::Hak, suggested.as_deref()) {
            Some(p) if app.install.as_ref().is_some_and(|gi| p.starts_with(&gi.root)) => {
                app.log
                    .error("Moonglow doesn't write into the game's folder: save the hak elsewhere");
                return false;
            }
            Some(p) => Some(p),
            None => return false,
        }
    } else {
        None
    };
    let doc = &mut app.haks[i];
    match doc.hak.save(to.as_deref()) {
        Ok(()) => {
            doc.changed();
            let path = doc.hak.path.clone().expect("saved");
            if let Some(folder) = &doc.folder {
                app.settings.remember_hak_folder(&path, folder);
            }
            let past = doc.hak.past_read_limit();
            app.log.info(format!("Saved {} ({} resources)", path.display(), doc.hak.items().len()));
            if let Some(first) = past.first() {
                app.log.warn(format!(
                    "{} resources from {first} on start past 2 GiB into the hak, where the game \
                     stops reading: split it into two",
                    past.len()
                ));
            }
            // The module's haks are read again if this is one.
            app.reload_resources(false);
            true
        }
        Err(e) => {
            app.log.error(format!("Could not save the hak: {e}"));
            false
        }
    }
}

/// Whether any hak has unsaved changes.
pub(crate) fn unsaved(app: &Moonglow) -> bool {
    app.haks.iter().any(|d| d.hak.is_dirty())
}

/// Saves every hak with unsaved changes that has a file (not the game's).
pub(crate) fn save_all(app: &mut Moonglow) {
    let root = app.install.as_ref().map(|gi| gi.root.clone());
    let ids: Vec<u32> = app
        .haks
        .iter()
        .filter(|d| d.hak.is_dirty())
        .filter(|d| {
            d.hak.path.as_ref().is_some_and(|p| root.as_ref().is_none_or(|r| !p.starts_with(r)))
        })
        .map(|d| d.id)
        .collect();
    for id in ids {
        save(app, id, false);
    }
}

fn size(n: u64) -> String {
    match n {
        n if n >= 1 << 30 => format!("{:.2} GB", n as f64 / f64::from(1u32 << 30)),
        n if n >= 1 << 20 => format!("{:.1} MB", n as f64 / f64::from(1u32 << 20)),
        n if n >= 1 << 10 => format!("{:.1} KB", n as f64 / 1024.0),
        n => format!("{n} bytes"),
    }
}

pub(crate) fn ui(app: &mut Moonglow, ui: &mut Ui, id: u32) {
    let Some(i) = app.haks.iter().position(|d| d.id == id) else {
        ui.label("This hak was closed.");
        return;
    };
    // What the toolbar asks for, done once the doc is let go.
    enum Do {
        AddFiles,
        AddFolder,
        Extract(Vec<ResKey>),
        Save(bool),
        View(ResKey),
        UpdateFromFolder,
    }
    let mut todo = None;
    // The game's own haks are never written: Save As makes a copy.
    let game_root = app.install.as_ref().map(|gi| gi.root.clone());
    let doc = &mut app.haks[i];
    let in_game = doc.hak.path.as_ref().zip(game_root).is_some_and(|(p, r)| p.starts_with(r));
    let mut changed = false;
    if in_game {
        ui.colored_label(
            ui.visuals().warn_fg_color,
            "This hak is part of the game: Save As… keeps your changes in a copy.",
        );
    }
    ui.horizontal(|ui| {
        let where_ =
            doc.hak.path.as_ref().map_or("not saved yet".into(), |p| p.display().to_string());
        ui.strong(format!("{} resources, {}", doc.hak.items().len(), size(doc.hak.size())));
        let folder = doc.folder.as_ref().map(|f| f.display().to_string());
        if ui
            .add_enabled(folder.is_some(), egui::Button::new("Update from Folder"))
            .on_hover_text(match &folder {
                Some(f) => format!("The hak's files again from {f}: what the folder has now"),
                None => "For a hak built with Build Hak from Folder".into(),
            })
            .clicked()
        {
            todo = Some(Do::UpdateFromFolder);
        }
        ui.weak(where_);
    });
    ui.horizontal(|ui| {
        ui.label("Description");
        let text = doc.description.get_or_insert_with(|| doc.hak.description.clone());
        let r = ui.add(
            egui::TextEdit::multiline(text)
                .desired_rows(2)
                .desired_width(480.0)
                .hint_text("for people: the game doesn't show it"),
        );
        if r.lost_focus() && *text != doc.hak.description {
            let t = text.clone();
            doc.hak.set_description(&t);
            changed = true;
        }
    });
    ui.horizontal(|ui| {
        if ui.button("Add Files…").clicked() {
            todo = Some(Do::AddFiles);
        }
        if ui
            .button("Add Folder…")
            .on_hover_text("Its files and those of the folders in it")
            .clicked()
        {
            todo = Some(Do::AddFolder);
        }
        let selected: Vec<ResKey> = doc.selected.iter().copied().collect();
        if ui
            .add_enabled(
                !selected.is_empty(),
                egui::Button::new(format!("Remove ({})", selected.len())),
            )
            .clicked()
        {
            doc.hak.remove(&selected);
            changed = true;
        }
        if ui.add_enabled(!selected.is_empty(), egui::Button::new("Extract…")).clicked() {
            todo = Some(Do::Extract(selected));
        }
        if ui.add_enabled(!doc.hak.items().is_empty(), egui::Button::new("Extract All…")).clicked()
        {
            todo = Some(Do::Extract(doc.hak.items().iter().map(|i| i.key).collect()));
        }
        let undo = doc.hak.undo_label().map(|l| format!("Undo {l}"));
        let redo = doc.hak.redo_label().map(|l| format!("Redo {l}"));
        if ui
            .add_enabled(undo.is_some(), egui::Button::new("Undo"))
            .on_hover_text(undo.unwrap_or_default())
            .clicked()
        {
            doc.hak.undo();
            changed = true;
        }
        if ui
            .add_enabled(redo.is_some(), egui::Button::new("Redo"))
            .on_hover_text(redo.unwrap_or_default())
            .clicked()
        {
            doc.hak.redo();
            changed = true;
        }
        if ui.add_enabled(doc.hak.is_dirty() && !in_game, egui::Button::new("Save")).clicked() {
            todo = Some(Do::Save(false));
        }
        if ui.button("Save As…").clicked() {
            todo = Some(Do::Save(true));
        }
    });
    let past = doc.hak.past_read_limit();
    if !past.is_empty() {
        ui.colored_label(
            ui.visuals().warn_fg_color,
            format!("{} resources would start past 2 GiB into the hak, where the game stops reading: split it into two.", past.len()),
        );
    }
    ui.horizontal(|ui| {
        ui.label("Find");
        ui.add(
            egui::TextEdit::singleline(&mut doc.filter)
                .desired_width(200.0)
                .hint_text("name or .type"),
        );
        egui::ComboBox::from_id_salt(("hak-sort", id))
            .selected_text(match doc.sort {
                Sort::Name => "By name",
                Sort::Type => "By type",
                Sort::Size => "By size",
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut doc.sort, Sort::Name, "By name");
                ui.selectable_value(&mut doc.sort, Sort::Type, "By type");
                ui.selectable_value(&mut doc.sort, Sort::Size, "By size")
                    .on_hover_text("The largest first");
            });
        ui.weak("Click to select, Ctrl+click to add; double-click to view; right-click for more");
    });
    if changed {
        doc.changed();
        changed = false;
    }
    let rows = doc.rows();
    let row_height = ui.text_style_height(&egui::TextStyle::Body) + 4.0;
    let mut rename_done = None;
    // The resource being viewed takes the lower part of the tab.
    let list_height = match doc.viewing {
        Some(_) => (ui.available_height() * 0.45).max(120.0),
        None => ui.available_height(),
    };
    let list = egui::ScrollArea::vertical().id_salt(("hak-rows", id)).max_height(list_height);
    list.auto_shrink([false, false]).show_rows(ui, row_height, rows.len(), |ui, range| {
        for &r in &rows[range] {
            let item = doc.hak.items()[r].clone();
            ui.horizontal(|ui| {
                if let Some((key, name)) = doc.rename.as_mut().filter(|(k, _)| *k == item.key) {
                    let edit = ui.add(egui::TextEdit::singleline(name).desired_width(160.0));
                    edit.request_focus();
                    ui.label(format!(".{}", key.restype));
                    if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                        rename_done = Some(None);
                    } else if edit.lost_focus() {
                        rename_done = Some(Some((*key, name.clone())));
                    }
                    return;
                }
                let selected = doc.selected.contains(&item.key);
                let name = egui::RichText::new(item.key.to_string()).monospace();
                let layout = egui::Layout::left_to_right(egui::Align::Center);
                let r = ui
                    .allocate_ui_with_layout(egui::vec2(240.0, row_height), layout, |ui| {
                        ui.set_min_width(240.0);
                        ui.add(egui::Button::selectable(selected, name))
                    })
                    .inner;
                if r.clicked() {
                    let (shift, command) = ui.input(|i| (i.modifiers.shift, i.modifiers.command));
                    let place = |k: ResKey| rows.iter().position(|&r| doc.hak.items()[r].key == k);
                    let from = doc.anchor.filter(|_| shift).and_then(place);
                    if let (Some(a), Some(b)) = (from, place(item.key)) {
                        // From the one clicked last to this one, as listed
                        // (with Ctrl, added to what is selected).
                        if !command {
                            doc.selected.clear();
                        }
                        let range = &rows[a.min(b)..=a.max(b)];
                        doc.selected.extend(range.iter().map(|&r| doc.hak.items()[r].key));
                    } else {
                        if command {
                            if !doc.selected.remove(&item.key) {
                                doc.selected.insert(item.key);
                            }
                        } else {
                            doc.selected = [item.key].into();
                        }
                        doc.anchor = Some(item.key);
                    }
                }
                if r.double_clicked() {
                    todo = Some(Do::View(item.key));
                }
                r.context_menu(|ui| {
                    if ui.button("View").clicked() {
                        todo = Some(Do::View(item.key));
                    }
                    if ui.button("Rename…").clicked() {
                        doc.rename = Some((item.key, item.key.resref.to_string()));
                    }
                    if ui.button("Extract…").clicked() {
                        todo = Some(Do::Extract(vec![item.key]));
                    }
                    if ui.button("Remove").clicked() {
                        doc.hak.remove(&[item.key]);
                        changed = true;
                    }
                });
                ui.add_sized([90.0, row_height], egui::Label::new(size(item.size)));
                match &item.source {
                    Source::Archive | Source::Bytes(_) => {}
                    Source::File(p) => {
                        ui.weak(format!("from {}", p.display()));
                    }
                }
            });
        }
    });
    let mut close_view = false;
    if let Some((key, view)) = &doc.viewing {
        ui.separator();
        ui.horizontal(|ui| {
            ui.strong(key.to_string());
            close_view = ui.small_button("Close").clicked();
        });
        crate::browser::plain_ui(ui, *key, view);
    }
    if close_view {
        doc.viewing = None;
    }
    if let Some(done) = rename_done {
        if let Some((key, name)) = done.filter(|(k, n)| *n != k.resref.to_string()) {
            match doc.hak.rename(key, name.trim()) {
                Ok(to) => {
                    doc.selected = [to].into();
                    changed = true;
                }
                Err(e) => app.log.error(format!("Rename {key}: {e}")),
            }
        }
        let doc = &mut app.haks[i];
        doc.rename = None;
    }
    if changed {
        app.haks[i].changed();
    }
    match todo {
        None => {}
        Some(Do::AddFiles) => {
            let files = app.dialogs.open_files(FileKind::HakFiles, None);
            let added = app.haks[i].hak.add_files(&files);
            app.haks[i].changed();
            report(app, &added);
        }
        Some(Do::AddFolder) => {
            if let Some(dir) = app.dialogs.pick_folder("Add Folder", None) {
                let added = app.haks[i].hak.add_folder(&dir);
                app.haks[i].changed();
                report(app, &added);
            }
        }
        Some(Do::Extract(keys)) => {
            if let Some(dir) = app.dialogs.pick_folder("Extract to", None) {
                match app.haks[i].hak.extract(&keys, &dir) {
                    Ok(n) => app.log.info(format!("Extracted {n} files to {}", dir.display())),
                    Err(e) => app.log.error(format!("Extract: {e}")),
                }
            }
        }
        Some(Do::Save(ask)) => {
            save(app, id, ask);
        }
        Some(Do::View(key)) => {
            let doc = &mut app.haks[i];
            match doc.hak.data(key) {
                Ok(data) => doc.viewing = Some((key, crate::browser::plain(key, &data))),
                Err(e) => app.log.error(format!("{key}: {e}")),
            }
        }
        Some(Do::UpdateFromFolder) => {
            // What the folder has now: its files in place of the hak's.
            let doc = &mut app.haks[i];
            if let Some(dir) = doc.folder.clone() {
                let all: Vec<ResKey> = doc.hak.items().iter().map(|i| i.key).collect();
                doc.hak.remove(&all);
                let added = doc.hak.add_folder(&dir);
                doc.changed();
                app.log.info(format!("Read {} again", dir.display()));
                report(app, &added);
            }
        }
    }
}

/// Asks what to do with a hak's unsaved changes when its tab is closed.
pub(crate) fn closing_window(app: &mut Moonglow, ctx: &egui::Context) {
    let Some(id) = app.hak_closing else { return };
    let Some(doc) = app.haks.iter().find(|d| d.id == id) else {
        app.hak_closing = None;
        return;
    };
    let title = doc.title();
    let mut answer = None;
    egui::Window::new("Unsaved Hak")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.label(format!("{} has unsaved changes.", title.trim_end_matches(" *")));
            ui.horizontal(|ui| {
                if ui.button("Save").clicked() {
                    answer = Some(true);
                }
                if ui.button("Discard").clicked() {
                    answer = Some(false);
                }
                if crate::widgets::cancel(ui) {
                    app.hak_closing = None;
                }
            });
        });
    let Some(keep) = answer else { return };
    app.hak_closing = None;
    if keep && !save(app, id, false) {
        return;
    }
    app.haks.retain(|d| d.id != id);
    if let Some(t) = app.dock.find_tab(&Tab::Hak(id)) {
        app.dock.remove_tab(t);
    }
}
