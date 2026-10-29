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
    /// The description as typed (with plain line ends).
    description: Option<String>,
    /// Bumped on every change, for the cached rows.
    revision: u64,
    rows: Option<(String, Sort, u64, Vec<usize>)>,
    /// The order of the list.
    pub sort: Sort,
    /// The resource shown under the list.
    viewing: Option<(ResKey, Viewing)>,
    /// The folder the hak was built from: Update from Folder reads it
    /// again.
    pub folder: Option<PathBuf>,
    /// Files chosen to add, some of which the hak has already: asked
    /// about before anything is replaced.
    pub adding: Option<Adding>,
}

/// A hak's resource as it is shown under the list: a picture (a TGA, DDS
/// or PLT), a sound to play, a 2DA's rows and columns, a model drawn (the
/// copy the game reads being this hak's), or as the resource browser shows
/// the rest (fields, text, a compiled model as its text, the first bytes).
pub(crate) enum Viewing {
    Picture(egui::TextureHandle),
    Sound(std::sync::Arc<[u8]>),
    Table(Box<mg_module::table_layers::Layered>),
    /// Drawn in the model viewer, from the load order.
    Model,
    Plain(crate::browser::Plain),
}

impl std::fmt::Debug for Viewing {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Viewing::{}", self.kind())
    }
}

impl Viewing {
    /// What a word names the view by (for tests and the log).
    pub(crate) fn kind(&self) -> &'static str {
        match self {
            Viewing::Picture(_) => "picture",
            Viewing::Sound(_) => "sound",
            Viewing::Table(_) => "table",
            Viewing::Model => "model",
            Viewing::Plain(_) => "plain",
        }
    }
}

/// How hak `doc`'s resource `key` (its bytes `data`) is viewed.
fn viewing(
    app: &mut Moonglow,
    ctx: &egui::Context,
    doc: usize,
    key: ResKey,
    data: Vec<u8>,
) -> Viewing {
    use mg_core::ResType;
    let game = app.game.clone();
    let rm = game.as_deref().map(|g| &g.resman);
    let plain = |data: &[u8]| Viewing::Plain(crate::browser::plain(key, data, rm));
    let picture = |rgba: mg_image::Rgba| {
        let rgba = rgba.top_down();
        let size = [rgba.width as usize, rgba.height as usize];
        let image = egui::ColorImage::from_rgba_unmultiplied(size, &rgba.data);
        let name = format!("hak-view-{key}");
        Viewing::Picture(ctx.load_texture(name, image, egui::TextureOptions::LINEAR))
    };
    match key.restype {
        ResType::TGA | ResType::DDS => match mg_image::read(key.restype, &data) {
            Ok(t) => picture(t.to_rgba()),
            Err(e) => Viewing::Plain(crate::browser::Plain::Error(e.to_string())),
        },
        ResType::PLT => match mg_image::plt::Plt::read(&data) {
            // (In each layer's first color, from the game's palettes.)
            Ok(plt) => {
                let palettes: Vec<Option<std::sync::Arc<mg_image::Rgba>>> = match app.loader() {
                    Some(mut l) => mg_image::plt::PALETTES.iter().map(|p| l.palette(p)).collect(),
                    None => vec![None; 10],
                };
                let refs: [Option<&mg_image::Rgba>; 10] =
                    std::array::from_fn(|i| palettes.get(i).and_then(|p| p.as_deref()));
                picture(plt.colorize(&refs, [0; 10]))
            }
            Err(e) => Viewing::Plain(crate::browser::Plain::Error(e.to_string())),
        },
        ResType::WAV | ResType::BMU => {
            let data: std::sync::Arc<[u8]> = data.into();
            match mg_audio::Stream::open(data.clone()) {
                Ok(_) => Viewing::Sound(data),
                Err(e) => Viewing::Plain(crate::browser::Plain::Error(e.to_string())),
            }
        }
        ResType::TWODA => {
            let codepage = app.game.as_deref().map(|g| g.codepage()).unwrap_or_default();
            match mg_2da::TwoDa::parse(&data, codepage) {
                Ok(t) => Viewing::Table(Box::new(mg_module::table_layers::Layered::single(
                    app.haks[doc].title().trim_end_matches(" *"),
                    t,
                ))),
                Err(_) => plain(&data),
            }
        }
        // Drawn where the game reads this copy (the hak is one of the
        // module's, as saved): the viewer draws from the load order.
        ResType::MDL
            if app.game.as_deref().is_some_and(|g| {
                let stem = app.haks[doc].hak.path.as_deref().and_then(|p| p.file_stem());
                let layer = stem.map(|s| format!("hak:{}", s.to_string_lossy()));
                g.resman.get(&key).is_ok_and(|d| *d == data[..])
                    && layer.is_some_and(|l| {
                        g.resman.origin(&key).is_some_and(|o| o.eq_ignore_ascii_case(&l))
                    })
            }) =>
        {
            Viewing::Model
        }
        _ => plain(&data),
    }
}

/// Files chosen to add to a hak that has some of them already.
#[derive(Debug, Clone, PartialEq)]
pub struct Adding {
    pub files: Vec<PathBuf>,
    /// Those that would replace a resource of the hak, and which.
    pub replaced: Vec<(PathBuf, ResKey)>,
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
            adding: None,
        }
    }

    /// What the resource shown under the list is shown as (`picture`,
    /// `sound`, `table`, `model`, `plain`), and which.
    pub fn viewing(&self) -> Option<(ResKey, &'static str)> {
        self.viewing.as_ref().map(|(k, v)| (*k, v.kind()))
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
        let needle = mg_core::text::Needle::new(&self.filter);
        let filter = needle.as_str().to_string();
        let fresh = self
            .rows
            .as_ref()
            .is_some_and(|(f, s, r, _)| *f == filter && *s == self.sort && *r == self.revision);
        if !fresh {
            let items = self.hak.items();
            let mut rows: Vec<usize> =
                (0..items.len()).filter(|&i| needle.finds(&items[i].key.to_string())).collect();
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

    pub(crate) fn changed(&mut self) {
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
    open_at(app, &path);
}

/// Opens a hak's file in the hak editor (its tab brought to the front if
/// it is open already); its id.
pub fn open_at(app: &mut Moonglow, path: &std::path::Path) -> Option<u32> {
    if let Some(d) = app.haks.iter().find(|d| d.hak.path.as_deref() == Some(path)) {
        let id = d.id;
        app.actions.push(crate::Action::OpenTab(Tab::Hak(id)));
        return Some(id);
    }
    match Hak::open(path) {
        Ok(h) => {
            let id = open_tab(app, h);
            // (Built from a folder before: Update from Folder knows it.)
            let folder = app.settings.hak_folder(path).map(std::path::Path::to_path_buf);
            if let Some(doc) = app.haks.iter_mut().find(|d| d.id == id) {
                doc.folder = folder;
            }
            Some(id)
        }
        Err(e) => {
            app.log.error(format!("Could not open {}: {e}", path.display()));
            None
        }
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
    app.install.as_ref()?.user_hak_dir()
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
        EditTable(ResKey),
        UpdateFromFolder,
        CompileModels,
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
        // (Here, not among the buttons under: that row is full.)
        if ui
            .button("Compile Models")
            .on_hover_text(
                "Compiles the hak's models kept as text (ASCII), each against its supermodel \
                 from this hak, the module's haks or the game; one that can't be stays as text",
            )
            .clicked()
        {
            todo = Some(Do::CompileModels);
        }
        ui.weak(where_);
    });
    ui.horizontal(|ui| {
        ui.label("Description");
        let stored = &doc.hak.description;
        let text = doc.description.get_or_insert_with(|| crate::text::to_editor(stored).0);
        let field = egui::TextEdit::multiline(text)
            .desired_rows(2)
            .desired_width(480.0)
            .hint_text("for people: the game doesn't show it");
        let r =
            crate::widgets::text_area(ui, ui.id().with("hak-description"), field).response.response;
        crate::widgets::leaves_at_a_press(ui.ctx(), &r);
        if r.lost_focus() && *text != crate::text::to_editor(stored).0 {
            let t = crate::text::with_line_ends(text, stored);
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
        let field = ui.id().with("hak-filter-field");
        crate::widgets::SearchField::new(field, &mut doc.filter)
            .hint("name or .type")
            .width(200.0)
            .find_over()
            .show(ui);
        let sorts = [Sort::Name, Sort::Type, Sort::Size];
        crate::widgets::dropdown(("hak-sort", id))
            .selected_text(match doc.sort {
                Sort::Name => "By name",
                Sort::Type => "By type",
                Sort::Size => "By size",
            })
            .show_with(
                ui,
                &mut doc.sort,
                || sorts.to_vec(),
                |ui, sort| {
                    ui.selectable_value(sort, Sort::Name, "By name");
                    ui.selectable_value(sort, Sort::Type, "By type");
                    ui.selectable_value(sort, Sort::Size, "By size")
                        .on_hover_text("The largest first");
                },
            );
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
                    let listed: Vec<ResKey> =
                        rows.iter().map(|&r| doc.hak.items()[r].key).collect();
                    crate::widgets::select_in_list(
                        &mut doc.selected,
                        &mut doc.anchor,
                        &listed,
                        item.key,
                        command,
                        shift,
                    );
                }
                if r.double_clicked() {
                    todo = Some(Do::View(item.key));
                }
                r.context_menu(|ui| {
                    if ui.button("View").clicked() {
                        todo = Some(Do::View(item.key));
                    }
                    if item.key.restype == mg_core::ResType::TWODA
                        && ui
                            .button("Edit Table")
                            .on_hover_text(
                                "In the 2DA editor: saved into this hak, and written with the \
                                 hak's own Save",
                            )
                            .clicked()
                    {
                        todo = Some(Do::EditTable(item.key));
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
    // (Drawn below, with the window's state: a table's columns, the model
    // viewer, the speakers.)
    let shown = doc.viewing.take();
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
    } else if let Some((key, view)) = shown {
        ui.separator();
        let mut close = false;
        ui.horizontal(|ui| {
            ui.strong(key.to_string());
            close = ui.small_button("Close").clicked();
            if let Viewing::Sound(data) = &view {
                let name = key.resref;
                let sound = crate::audio::Sound { name, data: data.clone() };
                let playing = app.speaker.playing(crate::audio::Channel::Preview);
                if ui.button(if playing { "Stop" } else { "Play" }).clicked() {
                    if playing {
                        app.stop_sound(crate::audio::Channel::Preview);
                    } else {
                        app.speaker.play(crate::audio::Channel::Preview, sound, 1.0, false);
                    }
                }
            }
        });
        match &view {
            Viewing::Picture(texture) => {
                let size = texture.size_vec2();
                ui.weak(format!("{} by {} pixels", size.x, size.y));
                // (At its own size, or smaller to fit the room under the
                // list.)
                let room = ui.available_size();
                let fit = (room.x / size.x.max(1.0)).min(room.y / size.y.max(1.0)).min(1.0);
                ui.add(egui::Image::new(texture).fit_to_exact_size(size * fit.max(0.01)));
            }
            Viewing::Sound(data) => {
                ui.weak(format!("A sound, {} bytes: Play plays it", data.len()));
            }
            Viewing::Table(table) => {
                let label = table.copies[0].0.clone();
                crate::browser::two_da(app, ui, key, Some(&label), table);
            }
            Viewing::Model => {
                use crate::model_view::Source;
                if crate::model_view::embedded(app, ui, Source::Resource(key)) {
                    app.actions.push(crate::Action::OpenTab(Tab::Model(key)));
                }
            }
            Viewing::Plain(plain) => crate::browser::plain_ui(ui, key, plain),
        }
        let doc = &mut app.haks[i];
        if !close && doc.viewing.is_none() {
            doc.viewing = Some((key, view));
        }
    }
    match todo {
        None => {}
        Some(Do::AddFiles) => {
            let files = app.dialogs.open_files(FileKind::HakFiles, None);
            add(app, i, files);
        }
        Some(Do::AddFolder) => {
            if let Some(dir) = app.dialogs.pick_folder("Add Folder", None) {
                add(app, i, mg_module::hak_edit::folder_files(&dir));
            }
        }
        Some(Do::EditTable(key)) => {
            let id = app.haks[i].id;
            crate::table_view::open_in_hak(app, id, key);
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
        Some(Do::View(key)) => match app.haks[i].hak.data(key) {
            Ok(data) => {
                let view = viewing(app, ui.ctx(), i, key, data);
                app.haks[i].viewing = Some((key, view));
            }
            Err(e) => app.log.error(format!("{key}: {e}")),
        },
        Some(Do::CompileModels) => compile_models(app, i),
        Some(Do::UpdateFromFolder) => {
            // What the folder has now: its files in place of the hak's, in
            // one step to undo; what the folder has no file of goes, and
            // is named (a hak added to by hand, or built from several
            // folders, loses it: Undo puts it back).
            let doc = &mut app.haks[i];
            if let Some(dir) = doc.folder.clone() {
                let (added, gone) = doc.hak.update_from_folder(&dir);
                doc.changed();
                app.log.info(format!("Read {} again", dir.display()));
                report(app, &added);
                if !gone.is_empty() {
                    let names: Vec<String> = gone.iter().take(12).map(ResKey::to_string).collect();
                    let more = gone.len().saturating_sub(names.len());
                    let more = if more > 0 { format!(" and {more} more") } else { String::new() };
                    app.log.warn(format!(
                        "Took out {} resources the folder has no file of (Undo puts them back): \
                         {}{more}",
                        gone.len(),
                        names.join(", ")
                    ));
                }
            }
        }
    }
}

/// Compile Models: the models of hak `i` kept as text are compiled
/// (`mg_module::models`) as a job, off the window's thread, and put into
/// the hak as one step to undo; those that can't be are named in the log
/// and stay as they are.
fn compile_models(app: &mut Moonglow, i: usize) {
    use mg_module::models::{Compiled, compile_models_each};
    let (id, hak) = (app.haks[i].id, app.haks[i].hak.clone());
    app.start_job_anywhere(
        "Compile Models",
        move |job| -> Vec<Compiled> {
            job.progress.say("Reading the hak's models");
            // (Its materials too: a mesh with no render hint takes its
            // material's.)
            let models: Vec<(ResKey, Vec<u8>)> = (hak.items().iter())
                .filter(|item| {
                    matches!(item.key.restype, mg_core::ResType::MDL | mg_core::ResType::MTR)
                })
                .filter_map(|item| Some((item.key, hak.data(item.key).ok()?)))
                .collect();
            let files: Vec<(ResKey, &[u8])> =
                models.iter().map(|(k, d)| (*k, d.as_slice())).collect();
            let lookup = |name: &str, t: mg_core::ResType| {
                let rm = &job.game.as_deref()?.resman;
                rm.get_named(name, t).ok().map(|d| d.into_owned())
            };
            compile_models_each(&files, &lookup, &|done, total| {
                job.progress.step(done, total);
                job.progress.say(format!("{done} of {total} models"));
                !job.progress.cancelled()
            })
        },
        move |app, results| {
            // (The window took no input meanwhile: the hak is as it was.)
            let Some(i) = app.haks.iter().position(|d| d.id == id) else { return };
            let sorted = crate::model_view::CompiledModels::sort(&mut app.log, "the hak", results);
            let Some(sorted) = sorted else { return };
            let made: Vec<_> = (sorted.made.iter())
                .map(|(k, b)| (*k, mg_module::hak_edit::Source::Bytes(b.clone().into())))
                .collect();
            if !made.is_empty() {
                match app.haks[i].hak.add_sources_as("Compile Models", made) {
                    Ok(_) => app.haks[i].changed(),
                    Err(e) => {
                        app.log.error(format!("Compile Models: {e}"));
                        return;
                    }
                }
            }
            sorted.say(&mut app.log);
        },
    );
}

/// Adds files to hak `i`: at once, unless the hak has some of them
/// already, which is asked about first (as Aurora's hak editor asks).
fn add(app: &mut Moonglow, i: usize, files: Vec<PathBuf>) {
    let replaced = app.haks[i].hak.replaced_by(&files);
    if replaced.is_empty() {
        let added = app.haks[i].hak.add_files(&files);
        app.haks[i].changed();
        report(app, &added);
    } else {
        app.haks[i].adding = Some(Adding { files, replaced });
    }
}

/// Asks about files being added that a hak has already: replace them,
/// leave those out, or add nothing.
pub(crate) fn adding_window(app: &mut Moonglow, ctx: &egui::Context) {
    let Some(i) = app.haks.iter().position(|d| d.adding.is_some()) else { return };
    let adding = app.haks[i].adding.clone().expect("found");
    let title = app.haks[i].title();
    let (n, all) = (adding.replaced.len(), adding.files.len());
    let said =
        format!("{} already has {n} of the {all} files chosen:", title.trim_end_matches(" *"));
    let files: Vec<_> = (adding.replaced.iter())
        .map(|(path, key)| (key.to_string(), Some(path.display().to_string())))
        .collect();
    let tips = [
        "Add all the files: these take the place of the hak's",
        "Add the other files and leave the hak's as they are",
    ];
    use crate::widgets::Already;
    let Some(answer) = crate::widgets::files_already(ctx, "Hak", &said, &files, tips) else {
        return;
    };
    app.haks[i].adding = None;
    let replace = match answer {
        Already::Replace => true,
        Already::Skip => false,
        Already::Cancel => return,
    };
    let files: Vec<PathBuf> = if replace {
        adding.files
    } else {
        let skip: Vec<&PathBuf> = adding.replaced.iter().map(|(p, _)| p).collect();
        adding.files.into_iter().filter(|f| !skip.contains(&f)).collect()
    };
    let added = app.haks[i].hak.add_files(&files);
    app.haks[i].changed();
    report(app, &added);
}
