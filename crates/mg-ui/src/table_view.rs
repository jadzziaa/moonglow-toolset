//! The 2DA editor: a table's cells changed in place, rows added at the end
//! and emptied (never taken out: a row's number is what everything else
//! knows it by), cells checked by what their column holds as they are
//! typed, and rows taken from another copy of the table (a lower hak's,
//! which this one hides).
//!
//! A table is edited where it lives: in the module, in a hak open in the
//! hak editor (whose Save writes it), or in a file anywhere. The game's
//! own tables are never changed: Edit on one makes a copy somewhere.

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::Arc;

use egui::Ui;
use mg_2da::TwoDa;
use mg_core::{ResRef, ResType};
use mg_edit::{Command, Edit};
use mg_module::doctor::{CellFault, ColumnKind, TalkTables};
use mg_module::table_desc::{Beside, Description, Kind};
use mg_module::table_edit::{Candidate, Editing, Offered};
use mg_resman::ResKey;

use crate::palette_view::{Keys, give_keys, keys_are};
use crate::{Action, Moonglow, Tab};

/// Where a table being edited lives, and is saved to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Home {
    /// The module's own resource: saved as a step of the module's.
    Module,
    /// A hak open in the hak editor (its id): saved into it, and written
    /// with the hak's own Save.
    Hak(u32),
    File(PathBuf),
}

/// Rows of another copy being looked through, to take.
#[derive(Debug, Clone, Default)]
pub struct Taking {
    /// Which of the other copies (by its layer's name).
    pub from: Option<String>,
    pub chosen: BTreeSet<usize>,
    /// The copies in the load order, read once as the window opens.
    copies: Option<Arc<Vec<(String, TwoDa)>>>,
    /// The rows offered, worked out for a copy and the table as it was
    /// (its revision); and whether whose change each is could be told.
    found: Option<(String, u64, Arc<Vec<Candidate>>, bool)>,
}

/// A table open in a tab.
#[derive(Debug)]
pub struct TableDoc {
    pub id: u32,
    pub key: ResKey,
    pub home: Home,
    pub edit: Editing,
    /// The cell the keys are at: its row and column.
    pub selected: Option<(usize, usize)>,
    /// Several cells selected: the corner the selection began at (it
    /// reaches to [`selected`](Self::selected)). None: the one cell.
    anchor: Option<(usize, usize)>,
    /// A drag over the cells is selecting them.
    dragging: bool,
    /// A column's name is being dragged to another place: the column.
    moving: Option<usize>,
    /// What is being typed into the selected cell.
    pub typing: Option<String>,
    pub filter: String,
    go_to: String,
    /// The selected cell is to be brought into view.
    reveal: bool,
    /// How far the rows are scrolled: the column names and the rows'
    /// numbers follow.
    scroll: egui::Vec2,
    /// Saved with one space between cells, not with the columns in line.
    pub compact: bool,
    /// Talk-table columns show their text, not their numbers.
    pub strref_text: bool,
    /// Columns described as a list or as flags show their values' names.
    pub value_names: bool,
    /// The table's description, where a file beside it gives one.
    described: Option<Described>,
    /// When the description's file was last looked at (the time egui
    /// gives): it is read again when it changes.
    described_at: f64,
    /// Bumped at every change: what is worked out from the table is kept
    /// until then.
    revision: u64,
    faults: Option<(u64, Arc<Vec<CellFault>>)>,
    /// How many rows Add Row adds.
    pub add_count: usize,
    /// The name of a column being added, or given to the column
    /// [`renaming`](Self::renaming), as typed.
    pub new_column: Option<String>,
    /// The column Rename Column… is for.
    pub renaming: Option<usize>,
    pub taking: Option<Taking>,
    /// Where a row is named, as last asked: the row, and each place.
    pub references: Option<(usize, Vec<Reference>)>,
    /// What went wrong last (a cell refused, a save that failed).
    pub problem: Option<String>,
}

/// A table's description as read from its file (`name.2da.toml` beside
/// the table: [`mg_module::table_desc`]).
#[derive(Debug, Clone)]
struct Described {
    path: PathBuf,
    modified: Option<std::time::SystemTime>,
    /// The description, or why it could not be read.
    read: Result<Arc<Description>, String>,
    /// The files of the table's repository or project, for the scripts
    /// and tables the description names.
    files: Arc<Beside>,
    /// The project's tables the description names, as read (once, until
    /// the description is read again).
    tables: std::cell::RefCell<std::collections::HashMap<String, Option<Arc<TwoDa>>>>,
}

/// A cell that names a row: the table, the row and the column.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reference {
    pub table: String,
    pub row: usize,
    pub column: String,
}

impl TableDoc {
    fn new(id: u32, key: ResKey, home: Home, edit: Editing) -> TableDoc {
        TableDoc {
            id,
            key,
            home,
            edit,
            selected: None,
            anchor: None,
            dragging: false,
            moving: None,
            typing: None,
            filter: String::new(),
            go_to: String::new(),
            reveal: false,
            scroll: egui::Vec2::ZERO,
            compact: false,
            strref_text: true,
            value_names: true,
            described: None,
            described_at: f64::NEG_INFINITY,
            revision: 0,
            faults: None,
            add_count: 1,
            new_column: None,
            renaming: None,
            taking: None,
            references: None,
            problem: None,
        }
    }

    pub fn name(&self) -> String {
        self.key.resref.to_string()
    }

    /// The table's description, where it has one that reads.
    pub fn description(&self) -> Option<Arc<Description>> {
        self.described.as_ref().and_then(|d| d.read.clone().ok())
    }

    pub fn title(&self) -> String {
        format!("{}{}", self.key, if self.edit.is_dirty() { " *" } else { "" })
    }

    /// Goes to a cell: selected, and brought into view.
    pub fn go_to(&mut self, row: usize, column: usize) {
        self.anchor = None;
        self.selected = Some((row, column));
        self.reveal = true;
    }

    /// The cells selected: the first and last row, the first and last
    /// column (one cell, with no rectangle selected).
    pub fn area(&self) -> Option<(usize, usize, usize, usize)> {
        let (row, column) = self.selected?;
        let t = self.edit.table();
        // (A corner the table no longer has: the one cell.)
        let (r, c) = self
            .anchor
            .filter(|(r, c)| *r < t.len() && *c < t.columns().len())
            .unwrap_or((row, column));
        Some((r.min(row), r.max(row), c.min(column), c.max(column)))
    }

    /// Selects from where the selection began (the selected cell, with
    /// one cell selected) to `to`.
    pub fn select_to(&mut self, to: (usize, usize)) {
        self.anchor = self.anchor.or(self.selected);
        self.selected = Some(to);
    }

    pub(crate) fn changed(&mut self) {
        self.revision += 1;
        self.references = None;
    }

    /// Whether it has changes not saved where it lives: a cell being typed
    /// too (closing the tab took no notice of one, and it was lost).
    pub(crate) fn has_unsaved(&self) -> bool {
        self.edit.is_dirty() || self.typing.is_some()
    }

    /// Sets the selected cell from what was typed.
    fn commit(&mut self) {
        let (Some((row, column)), Some(text)) = (self.selected, self.typing.take()) else { return };
        let value = Some(text.trim().to_string()).filter(|v| !v.is_empty());
        match self.edit.set(row, column, value) {
            Ok(true) => self.changed(),
            Ok(false) => {}
            Err(e) => self.problem = Some(e.to_string()),
        }
    }
}

/// Where a home is, in words.
fn home_text(app: &Moonglow, home: &Home) -> String {
    match home {
        Home::Module => "in the module".to_string(),
        Home::Hak(id) => match app.haks.iter().find(|d| d.id == *id) {
            Some(d) => format!("in the hak {}", d.title().trim_end_matches(" *")),
            None => "in a hak that is closed: Save As File… keeps it".to_string(),
        },
        Home::File(path) => path.display().to_string(),
    }
}

fn codepage(app: &Moonglow) -> mg_core::Codepage {
    app.game.as_deref().map_or_else(mg_core::Codepage::default, |g| g.codepage())
}

/// (`compact`: the table's file had one blank between cells, and is saved
/// so again unless the builder says otherwise.)
fn open_doc(app: &mut Moonglow, key: ResKey, home: Home, edit: Editing, compact: bool) -> u32 {
    if let Some(d) = app.tables.iter().find(|d| d.key == key && d.home == home) {
        let id = d.id;
        app.actions.push(Action::OpenTab(Tab::Table(id)));
        return id;
    }
    app.next_table += 1;
    let id = app.next_table;
    let mut doc = TableDoc::new(id, key, home, edit);
    doc.compact = compact;
    app.tables.push(doc);
    app.actions.push(Action::OpenTab(Tab::Table(id)));
    id
}

fn is_compact(data: &[u8]) -> bool {
    mg_2da::Layout::of(data) == mg_2da::Layout::Compact
}

fn parse(app: &Moonglow, key: ResKey, data: &[u8]) -> Result<TwoDa, String> {
    TwoDa::parse(data, codepage(app)).map_err(|e| format!("{key}: {e}"))
}

/// Opens the module's own table in the editor.
pub fn open_in_module(app: &mut Moonglow, key: ResKey) -> Option<u32> {
    let data = app.ws.as_ref()?.module.get(&key)?.to_vec();
    match parse(app, key, &data) {
        Ok(t) => Some(open_doc(app, key, Home::Module, Editing::new(t), is_compact(&data))),
        Err(e) => {
            app.log.error(e);
            None
        }
    }
}

/// Opens a table of a hak that is open in the hak editor.
pub fn open_in_hak(app: &mut Moonglow, hak: u32, key: ResKey) -> Option<u32> {
    let data = app.haks.iter().find(|d| d.id == hak)?.hak.data(key);
    match data.and_then(|d| Ok((parse(app, key, &d)?, is_compact(&d)))) {
        Ok((t, compact)) => Some(open_doc(app, key, Home::Hak(hak), Editing::new(t), compact)),
        Err(e) => {
            app.log.error(e);
            None
        }
    }
}

/// Opens a 2DA file anywhere.
pub fn open_path(app: &mut Moonglow, path: &std::path::Path) -> Option<u32> {
    let name = path.file_stem().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
    let read = || -> Result<(ResKey, TwoDa, bool), String> {
        let resref = ResRef::from_str(&name).map_err(|e| format!("{}: {e}", path.display()))?;
        let key = ResKey::new(resref, ResType::TWODA);
        let data = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
        Ok((key, parse(app, key, &data)?, is_compact(&data)))
    };
    match read() {
        Ok((key, t, compact)) => {
            Some(open_doc(app, key, Home::File(path.to_path_buf()), Editing::new(t), compact))
        }
        Err(e) => {
            app.log.error(e);
            None
        }
    }
}

/// A 2DA file anywhere, asked for.
pub fn open_file(app: &mut Moonglow) {
    if let Some(path) = app.dialogs.open_file(crate::dialogs::FileKind::Any, None) {
        open_path(app, &path);
    }
}

/// A table Open 2DA lists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Listed {
    /// One of the module's own (and its file, in a project or a folder).
    Module(ResKey, Option<PathBuf>),
    /// A `.2da` file of the project or the module folder that is not the
    /// module's (a hak's sources, kept beside them).
    File(PathBuf),
}

impl Listed {
    fn name(&self) -> String {
        match self {
            Listed::Module(key, _) => key.resref.to_string(),
            Listed::File(path) => {
                path.file_stem().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default()
            }
        }
    }
}

/// The Open 2DA window: the tables there are to open, and a name to find.
#[derive(Debug, Clone, Default)]
pub struct OpenTable {
    pub filter: String,
    pub tables: Vec<Listed>,
    /// The folder the files are named from.
    root: Option<PathBuf>,
}

/// Tools › 2DA File…: the module's tables and the `.2da` files of its
/// project or folder, whatever folder each is in, to open; with no module
/// open (or none of either), a file is asked for.
pub fn open_table_window(app: &mut Moonglow) {
    use mg_module::ModuleLocation;
    let mut tables = Vec::new();
    let mut root = None;
    if let Some(ws) = &app.ws {
        let module = &ws.module;
        root = match &module.location {
            Some(ModuleLocation::Project { root, .. }) => Some(root.clone()),
            Some(ModuleLocation::Folder(path)) => Some(path.clone()),
            _ => None,
        };
        tables.extend(
            module
                .keys_of(ResType::TWODA)
                .map(|k| Listed::Module(*k, module.file_of(k).map(std::path::Path::to_path_buf))),
        );
        if let Some(root) = &root {
            let own: Vec<PathBuf> = tables
                .iter()
                .filter_map(|t| match t {
                    Listed::Module(_, path) => path.clone(),
                    Listed::File(_) => None,
                })
                .collect();
            let files = mg_module::hak_edit::folder_files(root).into_iter().filter(|p| {
                p.extension().is_some_and(|e| e.eq_ignore_ascii_case("2da")) && !own.contains(p)
            });
            tables.extend(files.map(Listed::File));
        }
    }
    if tables.is_empty() {
        return open_file(app);
    }
    tables.sort_by_key(|t| (t.name(), matches!(t, Listed::File(_))));
    app.open_table = Some(OpenTable { filter: String::new(), tables, root });
}

/// The Open 2DA window.
pub(crate) fn open_table_ui(app: &mut Moonglow, ctx: &egui::Context) {
    let Some(mut w) = app.open_table.take() else { return };
    let mut open = true;
    let mut chosen = None;
    let mut browse = false;
    crate::widgets::dialog(ctx, "Open 2DA", &mut open)
        .default_size([460.0, 440.0])
        .collapsible(false)
        .show(ctx, |ui| {
            let field = ui.add(
                egui::TextEdit::singleline(&mut w.filter)
                    .hint_text("Find by name")
                    .desired_width(f32::INFINITY),
            );
            crate::widgets::autofocus(ui, &field);
            let entered = ui.input(|i| i.key_pressed(egui::Key::Enter));
            let needle = mg_core::text::Needle::new(&w.filter);
            let listed: Vec<&Listed> =
                w.tables.iter().filter(|t| needle.finds(&t.name())).collect();
            ui.horizontal(|ui| {
                ui.weak(format!("{} tables", listed.len()));
                if ui.button("Browse…").on_hover_text("A .2da file anywhere else").clicked() {
                    browse = true;
                }
            });
            if entered && let Some(first) = listed.first() {
                chosen = Some((*first).clone());
            }
            let row = ui.spacing().interact_size.y;
            let root = w.root.as_deref();
            egui::ScrollArea::vertical().auto_shrink(false).show_rows(
                ui,
                row,
                listed.len(),
                |ui, range| {
                    for t in &listed[range] {
                        ui.horizontal(|ui| {
                            let r = ui.selectable_label(false, format!("{}.2da", t.name()));
                            // Where it is: its folder in the project.
                            let (path, whose) = match t {
                                Listed::Module(_, path) => (path.as_deref(), "the module's"),
                                Listed::File(path) => (Some(path.as_path()), "a file"),
                            };
                            let folder = path
                                .and_then(|p| p.parent())
                                .map(|p| root.and_then(|r| p.strip_prefix(r).ok()).unwrap_or(p))
                                .map(|p| p.display().to_string())
                                .filter(|p| !p.is_empty());
                            ui.weak(match folder {
                                Some(folder) => format!("{whose}, in {folder}"),
                                None => whose.to_string(),
                            });
                            if r.clicked() {
                                chosen = Some((*t).clone());
                            }
                        });
                    }
                },
            );
        });
    match chosen {
        Some(Listed::Module(key, _)) => _ = open_in_module(app, key),
        Some(Listed::File(path)) => _ = open_path(app, &path),
        None if browse => open_file(app),
        None if open => app.open_table = Some(w),
        None => {}
    }
}

/// A copy of the table the game reads (the highest in the load order),
/// to be edited in `home`: not saved there until Save.
pub fn edit_copy(app: &mut Moonglow, key: ResKey, home: Home) -> Option<u32> {
    let data = app.game.as_ref()?.resman.get(&key).ok()?.into_owned();
    match parse(app, key, &data) {
        Ok(t) => {
            let mut edit = Editing::new(t);
            edit.mark_unsaved();
            Some(open_doc(app, key, home, edit, is_compact(&data)))
        }
        Err(e) => {
            app.log.error(e);
            None
        }
    }
}

/// Edit a Copy In › a file: asks where.
pub fn edit_copy_in_file(app: &mut Moonglow, key: ResKey) {
    let suggested = PathBuf::from(key.to_string());
    if let Some(path) = app.dialogs.save_file(crate::dialogs::FileKind::Any, Some(&suggested)) {
        let path = if path.extension().is_some() { path } else { path.with_extension("2da") };
        edit_copy(app, key, Home::File(path));
    }
}

/// Saves a table where it lives. Whether it was saved.
pub fn save(app: &mut Moonglow, id: u32) -> bool {
    let cp = codepage(app);
    let Some(at) = app.tables.iter().position(|d| d.id == id) else { return false };
    app.tables[at].commit();
    let (key, home) = (app.tables[at].key, app.tables[at].home.clone());
    let layout = match app.tables[at].compact {
        true => mg_2da::Layout::Compact,
        false => mg_2da::Layout::Aligned,
    };
    let bytes = match app.tables[at].edit.table().to_bytes_as(cp, layout) {
        Ok(b) => b,
        Err(e) => {
            app.tables[at].problem = Some(format!("Not saved: {e}"));
            return false;
        }
    };
    let saved = match &home {
        Home::Module => {
            let edit = Edit::SetResource { key, data: Some(bytes) };
            app.apply(Command::new(format!("Edit {key}"), vec![edit])).map_err(|e| e.to_string())
        }
        Home::Hak(hak) => match app.haks.iter_mut().find(|d| d.id == *hak) {
            Some(doc) => {
                let source = mg_module::hak_edit::Source::Bytes(bytes.into());
                let done = doc.hak.add_sources_as(&format!("Edit {key}"), vec![(key, source)]);
                doc.changed();
                done.map(|_| ())
            }
            None => Err("its hak is closed: Save As File… keeps the table".to_string()),
        },
        Home::File(path) => {
            std::fs::write(path, bytes).map_err(|e| format!("{}: {e}", path.display()))
        }
    };
    match saved {
        Ok(()) => {
            let where_ = home_text(app, &home);
            let doc = &mut app.tables[at];
            doc.edit.mark_saved();
            doc.problem = None;
            let more = match home {
                Home::Hak(_) => {
                    " (the hak's own Save writes its file; Tools › Reload Resources then \
                     shows the module the change)"
                }
                _ => "",
            };
            app.log.info(format!("Saved {key} {where_}{more}"));
            true
        }
        Err(e) => {
            app.tables[at].problem = Some(format!("Not saved: {e}"));
            false
        }
    }
}

/// Save As File…: the table written to a file, which is its home from
/// then on.
fn save_as_file(app: &mut Moonglow, id: u32) {
    let Some(doc) = app.tables.iter().find(|d| d.id == id) else { return };
    let suggested = PathBuf::from(doc.key.to_string());
    let Some(path) = app.dialogs.save_file(crate::dialogs::FileKind::Any, Some(&suggested)) else {
        return;
    };
    let path = if path.extension().is_some() { path } else { path.with_extension("2da") };
    if let Some(doc) = app.tables.iter_mut().find(|d| d.id == id) {
        doc.home = Home::File(path);
    }
    save(app, id);
}

/// Where a table's description would be: beside its file (a file's own,
/// the module's table's in a project or a module folder), beside the
/// module's file for a module kept in one, and for a hak's table in the
/// folder the hak is built from (beside the table's file there), else
/// beside the hak's file.
fn description_path(app: &Moonglow, doc: &TableDoc) -> Option<PathBuf> {
    use mg_module::table_desc::{path_beside, path_in_module};
    match &doc.home {
        Home::File(path) => Some(path_beside(path)),
        Home::Module => path_in_module(&app.ws.as_ref()?.module, &doc.key),
        Home::Hak(id) => {
            let hak = app.haks.iter().find(|d| d.id == *id)?;
            let name = doc.key.to_string();
            let in_folder = hak.folder.as_deref().and_then(|folder| {
                mg_module::hak_edit::folder_files(folder).into_iter().find(|p| {
                    p.file_name().is_some_and(|n| n.to_string_lossy().eq_ignore_ascii_case(&name))
                })
            });
            let table = in_folder.or_else(|| Some(hak.hak.path.as_ref()?.with_file_name(&name)))?;
            Some(path_beside(&table))
        }
    }
}

/// Reads the table's description where there is one, and again when its
/// file changes (looked at once a second).
fn describe(app: &mut Moonglow, ctx: &egui::Context, at: usize) {
    let now = ctx.input(|i| i.time);
    if now - app.tables[at].described_at < 1.0 {
        return;
    }
    app.tables[at].described_at = now;
    let path = description_path(app, &app.tables[at]).filter(|p| p.is_file());
    let Some(path) = path else {
        if app.tables[at].described.take().is_some() {
            app.tables[at].changed();
        }
        return;
    };
    let modified = std::fs::metadata(&path).and_then(|m| m.modified()).ok();
    if app.tables[at].described.as_ref().is_some_and(|d| d.path == path && d.modified == modified) {
        return;
    }
    let table_file = path.with_extension("");
    let files = Arc::new(Beside::of(&Beside::root_of(&table_file)));
    let game = app.game.clone();
    let script = |name: &str| {
        let key = ResKey::from_filename(&format!("{name}.nss"))?;
        files.script(name).or_else(|| Some(game.as_deref()?.resman.get(&key).ok()?.into_owned()))
    };
    let read = std::fs::read_to_string(&path)
        .map_err(|e| e.to_string())
        .and_then(|text| Description::parse(&text, &script))
        .map(Arc::new);
    let doc = &mut app.tables[at];
    doc.described = Some(Described { path, modified, read, files, tables: Default::default() });
    doc.changed();
}

/// What the table's columns hold, where the checks know.
fn kinds(app: &Moonglow, name: &str) -> Vec<(&'static str, ColumnKind)> {
    let read = |n: &str| app.game.as_deref().and_then(|g| g.table(n).ok());
    mg_module::doctor::column_kinds_as_named(name, &read)
}

/// How many strings the talk tables have, for the checks.
fn talk_tables(app: &Moonglow) -> TalkTables {
    let base = app.game.as_deref().map_or(0, |g| g.tlk().entries.len());
    let custom = app.game.as_deref().and_then(|g| g.custom_tlk().map(|t| t.entries.len()));
    TalkTables { base, custom }
}

/// The cells that are wrong, kept until the table changes.
fn faults(app: &mut Moonglow, at: usize) -> Arc<Vec<CellFault>> {
    let doc = &app.tables[at];
    if let Some((revision, found)) = &doc.faults
        && *revision == doc.revision
    {
        return found.clone();
    }
    let name = doc.name();
    let kinds = kinds(app, &name);
    let game = app.game.as_deref();
    let read = |n: &str| game.and_then(|g| g.table(n).ok());
    let rows_of = |n: &str| read(n).map(|t| t.len());
    // (With no game, no table is known to be there or not.)
    let tables: Option<&mg_module::doctor::TableOf> = game.map(|_| &read as _);
    let mut found =
        mg_module::doctor::table_faults(&name, doc.edit.table(), &kinds, tables, talk_tables(app));
    // And by the table's own description: a table it names is this one
    // as it is now, one of its repository's, or the game's.
    if let Some(d) = &doc.described
        && let Ok(description) = &d.read
    {
        let cp = codepage(app);
        let rows_of = |n: &str| match n.eq_ignore_ascii_case(&name) {
            true => Some(doc.edit.table().len()),
            false => d.files.rows_of(n, cp).or_else(|| rows_of(n)),
        };
        for f in description.faults(doc.edit.table(), &rows_of, talk_tables(app)) {
            if !found.iter().any(|x| (x.row, x.column) == (f.row, f.column)) {
                found.push(f);
            }
        }
        found.sort_by_key(|f| (f.row, f.column));
    }
    let found = Arc::new(found);
    let doc = &mut app.tables[at];
    doc.faults = Some((doc.revision, found.clone()));
    found
}

/// The other copies of the table in the load order: each layer's name and
/// its copy, highest first.
fn copies(app: &Moonglow, key: ResKey) -> Vec<(String, TwoDa)> {
    let Some(game) = app.game.as_deref() else { return Vec::new() };
    mg_module::table_layers::layered(&game.resman, &key, codepage(app))
        .map(|l| l.copies)
        .unwrap_or_default()
}

/// Where row `row` of the table `name` is named: the cells of the columns
/// known to name its rows, in the tables the game has (and in `own`, the
/// table as it is being edited, in place of the one of its name).
pub fn references(app: &Moonglow, name: &str, own: &TwoDa, row: usize) -> Vec<Reference> {
    use mg_module::doctor::{column_kinds, row_named};
    let Some(game) = app.game.as_deref() else { return Vec::new() };
    let mut out = Vec::new();
    let mut names: Vec<String> = game
        .resman
        .list(ResType::TWODA)
        .into_iter()
        .map(|r| r.to_string().to_lowercase())
        .collect();
    if !names.iter().any(|n| n == name) {
        names.push(name.to_string());
    }
    names.sort();
    for table in names {
        let kinds = column_kinds(&table);
        // (A column whose table another cell chooses: where it may be
        // this one.)
        let names_it = |k: &ColumnKind| match k {
            ColumnKind::Row(t) => t.eq_ignore_ascii_case(name),
            ColumnKind::RowBy { tables, .. } => {
                tables.iter().any(|(_, t)| t.eq_ignore_ascii_case(name))
            }
            _ => false,
        };
        let wanted: Vec<(&str, ColumnKind)> =
            kinds.iter().filter(|(_, k)| names_it(k)).copied().collect();
        if wanted.is_empty() {
            continue;
        }
        let read;
        let t: &TwoDa = if table == name {
            own
        } else {
            match game.table(&table) {
                Ok(t) => {
                    read = t;
                    &read
                }
                Err(_) => continue,
            }
        };
        for (column, kind) in wanted {
            for r in 0..t.len() {
                let here = kind.row_table(t, r).is_some_and(|n| n.eq_ignore_ascii_case(name));
                let named = t.get_int(r, column).and_then(|v| row_named(&table, column, v));
                if here && named == Some(row) {
                    out.push(Reference { table: table.clone(), row: r, column: column.into() });
                }
            }
        }
    }
    out
}

const CELL: f32 = 116.0;
pub(crate) const NUMBER: f32 = 54.0;
/// The narrowest and the widest a column is made.
const NARROWEST: f32 = 24.0;
const WIDEST: f32 = 1600.0;
/// How many tables' column widths are kept.
const WIDTHS_KEPT: usize = 200;

/// Each column's width: the one given it (by dragging the edge of its
/// name, in this table or another of its name), else `default`.
pub(crate) fn column_widths(
    settings: &crate::settings::Settings,
    table: &str,
    columns: &[String],
    default: f32,
) -> Vec<f32> {
    let kept = settings.table_widths.iter().find(|(t, _)| t.eq_ignore_ascii_case(table));
    let kept = kept.map_or(&[][..], |(_, w)| w.as_slice());
    columns
        .iter()
        .map(|c| {
            let given = kept.iter().find(|(k, _)| k.eq_ignore_ascii_case(c));
            given.map_or(default, |(_, w)| *w as f32)
        })
        .collect()
}

/// Gives a column of the table `table` its width, kept for the tables of
/// that name (and between runs, with the settings).
pub(crate) fn set_column_width(
    settings: &mut crate::settings::Settings,
    table: &str,
    column: &str,
    width: f32,
) {
    let width = width.clamp(NARROWEST, WIDEST).round() as u32;
    let kept = &mut settings.table_widths;
    let at = kept.iter().position(|(t, _)| t.eq_ignore_ascii_case(table));
    let mut entry = match at {
        Some(at) => kept.remove(at),
        None => (table.to_ascii_lowercase(), Vec::new()),
    };
    match entry.1.iter_mut().find(|(c, _)| c.eq_ignore_ascii_case(column)) {
        // (As it was: nothing changes, nor which table was changed last.)
        Some((_, w)) if *w == width && at == Some(0) => {
            kept.insert(0, entry);
            return;
        }
        Some((_, w)) => *w = width,
        None => entry.1.push((column.to_string(), width)),
    }
    kept.insert(0, entry);
    kept.truncate(WIDTHS_KEPT);
}

/// A column renamed keeps its width.
fn rename_column_width(
    settings: &mut crate::settings::Settings,
    table: &str,
    from: &str,
    to: &str,
) {
    let kept = settings.table_widths.iter_mut().find(|(t, _)| t.eq_ignore_ascii_case(table));
    if let Some((_, widths)) = kept {
        widths.retain(|(c, _)| !c.eq_ignore_ascii_case(to));
        if let Some((c, _)) = widths.iter_mut().find(|(c, _)| c.eq_ignore_ascii_case(from)) {
            *c = to.to_string();
        }
    }
}

/// What was done to the edge at the right of a column's name.
pub(crate) enum Edge {
    /// Dragged: the column's width now.
    Width(f32),
    /// Double-clicked: as wide as its widest cell.
    Fit,
}

/// The edge at the right of a column's name (`rect`, the name's place):
/// dragged, it sets the column's width; double-clicked, it fits the column
/// to its widest cell. (Over the name's own right end, and the gap to the
/// next: a drag begun on the rest of the name is the name's.)
pub(crate) fn column_edge(
    ui: &mut Ui,
    id: egui::Id,
    rect: egui::Rect,
    column: &str,
) -> Option<Edge> {
    let gap = ui.spacing().item_spacing.x;
    let x = rect.right() + gap / 2.0;
    let zone = egui::Rect::from_x_y_ranges(rect.right() - 4.0..=rect.right() + gap, rect.y_range());
    let r = ui.interact(zone, id, egui::Sense::click_and_drag());
    r.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Other, true, format!("Width of {column}"))
    });
    if r.hovered() || r.dragged() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeColumn);
        let stroke = egui::Stroke::new(2.0, ui.visuals().selection.bg_fill);
        ui.painter().vline(x, rect.y_range(), stroke);
    }
    if r.double_clicked() {
        return Some(Edge::Fit);
    }
    // (The edge follows the pointer from where it was taken hold of.)
    let pointer = ui.input(|i| (i.pointer.press_origin(), i.pointer.interact_pos()));
    if r.drag_started()
        && let Some(origin) = pointer.0
    {
        ui.data_mut(|d| d.insert_temp(id, origin.x - rect.right()));
    }
    let held: f32 = ui.data(|d| d.get_temp(id)).unwrap_or_default();
    match pointer.1.filter(|_| r.dragged()) {
        Some(at) if (at.x - held - rect.right()).abs() > 0.1 => {
            Some(Edge::Width(at.x - held - rect.left()))
        }
        _ => None,
    }
}

/// The width that shows a column's name and its longest cell (`cells`,
/// as shown: in the monospace font) whole.
pub(crate) fn fit_width(ui: &Ui, name: &str, cells: impl Iterator<Item = String>) -> f32 {
    let longest = cells.max_by_key(|s| s.chars().count()).unwrap_or_default();
    let color = ui.visuals().text_color();
    let wide = |text: String, style: egui::TextStyle| {
        ui.painter().layout_no_wrap(text, style.resolve(ui.style()), color).size().x
    };
    let widest = wide(name.to_string(), egui::TextStyle::Body)
        .max(wide(longest, egui::TextStyle::Monospace));
    // (The cell's margins, and a little to spare.)
    (widest + 12.0).clamp(NARROWEST, WIDEST)
}

/// What a cell shows: its talk-table string (`strref`, a column of them),
/// its value's name (`named`, a column described as a list or flags), or
/// its value. And the text shown in place of the value, and whether that
/// is a name.
fn cell_shows(
    raw: Option<&str>,
    strref: bool,
    named: Option<&Kind>,
    game: Option<&mg_rules::GameData>,
) -> (String, Option<String>, bool) {
    let text = raw
        .filter(|_| strref)
        .and_then(|v| v.parse::<u32>().ok())
        .and_then(|n| game?.string(mg_core::StrRef(n)));
    // (A value by its name, where the description gives it one.)
    let name = raw.zip(named).and_then(|(v, k)| k.named(v));
    let was_named = name.is_some();
    let text = text.or(name);
    let label = text.clone().unwrap_or_else(|| raw.unwrap_or("****").into());
    (label, text, was_named)
}

/// The tab of a table.
pub(crate) fn ui(app: &mut Moonglow, ui: &mut Ui, id: u32) {
    let Some(at) = app.tables.iter().position(|d| d.id == id) else {
        ui.label("This table is closed.");
        return;
    };
    // OK and Cancel, as the other editors have: a table is saved when
    // asked, so OK saves it and Cancel leaves it as last saved.
    let tips = [
        "Save the table and close the window",
        "Close the window without saving (unsaved changes are asked about first)",
    ];
    let pressed = crate::blueprint::ok_cancel_saying(ui, egui::Id::new(("table", id)), false, tips);
    match pressed {
        Some(crate::blueprint::Pressed::Ok) => {
            app.tables[at].commit();
            if !app.tables[at].edit.is_dirty() || save(app, id) {
                app.tables.retain(|d| d.id != id);
                app.actions.push(Action::CloseTab(Tab::Table(id)));
                return;
            }
        }
        // (Closed as by its ×: unsaved changes are asked about first,
        // no Undo bringing a table's back.)
        Some(crate::blueprint::Pressed::Cancel) => {
            app.tables[at].commit();
            if app.may_close_side(&Tab::Table(id)) {
                app.actions.push(Action::CloseTab(Tab::Table(id)));
                return;
            }
        }
        None => {}
    }
    describe(app, ui.ctx(), at);
    let faults = faults(app, at);
    let name = app.tables[at].name();
    let kinds = kinds(app, &name);
    let home = home_text(app, &app.tables[at].home);
    // (Read once: every copy of a large table is not parsed each frame.)
    if app.tables[at].taking.as_ref().is_some_and(|t| t.copies.is_none()) {
        let read = Arc::new(copies(app, app.tables[at].key));
        if let Some(t) = app.tables[at].taking.as_mut() {
            t.copies = Some(read);
        }
    }
    let others: Arc<Vec<(String, TwoDa)>> =
        app.tables[at].taking.as_ref().and_then(|t| t.copies.clone()).unwrap_or_default();
    let has_copies = app.game.as_deref().is_some_and(|g| g.resman.contains(&app.tables[at].key));
    let mut act = None;
    header(app, ui, at, &home, has_copies, faults.len(), &mut act);
    keys(app, ui, at);
    let below = 96.0;
    let grid_height = (ui.available_height() - below).max(120.0);
    ui.allocate_ui(egui::vec2(ui.available_width(), grid_height), |ui| {
        grid(app, ui, at, &kinds, &faults);
    });
    ui.separator();
    inspector(app, ui, at, &kinds, &faults);
    taking_window(app, ui.ctx(), at, &others);
    column_window(app, ui.ctx(), at);
    match act {
        Some(Act::Save) => _ = save(app, id),
        Some(Act::SaveAs) => save_as_file(app, id),
        None => {}
    }
}

enum Act {
    Save,
    SaveAs,
}

fn header(
    app: &mut Moonglow,
    ui: &mut Ui,
    at: usize,
    home: &str,
    has_copies: bool,
    wrong: usize,
    act: &mut Option<Act>,
) {
    let doc = &mut app.tables[at];
    ui.horizontal(|ui| {
        ui.strong(doc.key.to_string());
        if doc.edit.is_dirty() {
            ui.weak("(unsaved)");
        }
        let t = doc.edit.table();
        let about = format!("{} rows, {} columns; {home}", t.len(), t.columns().len());
        ui.add(egui::Label::new(egui::RichText::new(&about).weak()).truncate())
            .on_hover_text(about);
    });
    ui.horizontal_wrapped(|ui| {
        if ui.add_enabled(doc.edit.is_dirty(), egui::Button::new("Save")).clicked() {
            *act = Some(Act::Save);
        }
        if ui
            .button("Save As File…")
            .on_hover_text("Write the table to a file, which it is edited in from then on")
            .clicked()
        {
            *act = Some(Act::SaveAs);
        }
        let undo = doc.edit.undo_label().map(str::to_string);
        let tip = undo.clone().unwrap_or_else(|| "Nothing to undo".into());
        if ui.add_enabled(undo.is_some(), egui::Button::new("Undo")).on_hover_text(tip).clicked() {
            doc.typing = None;
            doc.edit.undo();
            doc.changed();
        }
        let redo = doc.edit.redo_label().map(str::to_string);
        let tip = redo.clone().unwrap_or_else(|| "Nothing to redo".into());
        if ui.add_enabled(redo.is_some(), egui::Button::new("Redo")).on_hover_text(tip).clicked() {
            doc.typing = None;
            doc.edit.redo();
            doc.changed();
        }
        ui.separator();
        let count = doc.add_count.max(1);
        let add = if count == 1 { "Add Row".to_string() } else { format!("Add {count} Rows") };
        if ui
            .button(add)
            .on_hover_text(
                "Empty rows at the end, as many as the number beside says (a row's number is \
                 what names it: none is put between others)",
            )
            .clicked()
        {
            doc.commit();
            let row = doc.edit.add_rows(count);
            doc.anchor = None;
            doc.selected = Some((row, doc.selected.map_or(0, |s| s.1)));
            doc.reveal = true;
            doc.changed();
        }
        let mut n = count;
        ui.add(egui::DragValue::new(&mut n).range(1..=10_000).speed(0.2))
            .on_hover_text("How many rows Add Row adds (padding, to begin at a row of your own)");
        doc.add_count = n;
        let row = doc.selected.map(|s| s.0);
        if ui
            .add_enabled(row.is_some(), egui::Button::new("Empty Row"))
            .on_hover_text(
                "Every cell of the selected row emptied (****). The row stays, so that the \
                 rows after it keep their numbers",
            )
            .clicked()
            && let Some(row) = row
        {
            doc.typing = None;
            doc.edit.clear_row(row);
            doc.changed();
        }
        if ui
            .add_enabled(row.is_some(), egui::Button::new("Delete Row"))
            .on_hover_text(
                "The selected row taken away. Every row after it is then one number lower, \
                 and what names those rows by number (other tables, blueprints, scripts) is \
                 not changed to follow: Find References to Row shows the tables that do. To \
                 keep the numbers, Empty Row",
            )
            .clicked()
            && let Some(row) = row
        {
            doc.typing = None;
            doc.edit.remove_row(row);
            doc.anchor = None;
            let column = doc.selected.map_or(0, |s| s.1);
            doc.selected =
                doc.edit.table().len().checked_sub(1).map(|last| (row.min(last), column));
            doc.reveal = true;
            doc.changed();
        }
        if ui.button("Add Column…").clicked() {
            doc.renaming = None;
            doc.new_column = Some(String::new());
        }
        ui.separator();
        if ui
            .add_enabled(has_copies, egui::Button::new("Take Rows…"))
            .on_hover_text(
                "Rows another copy of this table has (a hak below this one, the game's) and \
                 this one lacks or has differently",
            )
            .clicked()
        {
            doc.commit();
            doc.taking = Some(Taking::default());
        }
        let was = doc.compact;
        ui.checkbox(&mut doc.compact, "Compact").on_hover_text(
            "How the table is saved: ticked, with one space between cells (a smaller file, \
             with less for the game to pass over as it reads); not, with the columns in \
             line, as the game's own tables are (easier to read and to copy from). It begins \
             as the file was",
        );
        if doc.compact != was {
            doc.edit.mark_unsaved();
        }
        ui.checkbox(&mut doc.strref_text, "Strings as text").on_hover_text(
            "Columns that hold a talk-table string's number show the string; the number is \
             what is edited",
        );
        if doc.description().is_some() {
            ui.checkbox(&mut doc.value_names, "Values as names").on_hover_text(
                "Columns the table's description says are a list or flags show each value's \
                 name; the value is what the table holds",
            );
        }
    });
    if let Some(d) = &doc.described {
        let file = d.path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        ui.horizontal_wrapped(|ui| match &d.read {
            Ok(description) => {
                let columns = description.columns.len();
                let said = ui.weak(format!("Described by {file}: {columns} columns"));
                said.on_hover_text(format!(
                    "{}\nWhat each column holds, and the lists to choose from. It is read \
                     again when the file changes.",
                    d.path.display()
                ));
                if let Some(table) = &description.rows {
                    ui.weak(format!("its rows are {table}.2da's"));
                }
                for note in &description.notes {
                    ui.colored_label(ui.visuals().warn_fg_color, note);
                }
                let missing = description.missing_columns(doc.edit.table());
                if !missing.is_empty() {
                    ui.colored_label(
                        ui.visuals().warn_fg_color,
                        format!("described, and not in the table: {}", missing.join(", ")),
                    );
                }
            }
            Err(e) => {
                ui.colored_label(ui.visuals().error_fg_color, format!("{file} was not read: {e}"));
            }
        });
    }
    ui.horizontal_wrapped(|ui| {
        ui.label("Find");
        let field = ui.id().with(("table-filter", doc.id));
        crate::widgets::SearchField::new(field, &mut doc.filter)
            .hint("text in any cell")
            .width(170.0)
            .find_over()
            .show(ui);
        ui.label("Go to");
        let go =
            ui.add(egui::TextEdit::singleline(&mut doc.go_to).desired_width(70.0).hint_text("row"));
        if go.lost_focus()
            && ui.input(|i| i.key_pressed(egui::Key::Enter))
            && let Ok(row) = doc.go_to.trim().parse::<usize>()
            && row < doc.edit.table().len()
        {
            doc.filter.clear();
            doc.selected = Some((row, doc.selected.map_or(0, |s| s.1)));
            doc.reveal = true;
            take_keys(ui.ctx(), doc.id);
        }
        if wrong > 0 {
            let text = egui::RichText::new(match wrong {
                1 => "1 cell is wrong".to_string(),
                n => format!("{n} cells are wrong"),
            })
            .color(ui.visuals().warn_fg_color);
            if ui
                .button(text)
                .on_hover_text(
                    "Goes to the next of them. They are what Verify Module would say of this table",
                )
                .clicked()
            {
                *act = None;
                doc.go_to = "!".into();
            }
        }
        if let Some(problem) = &doc.problem {
            ui.colored_label(ui.visuals().error_fg_color, problem);
        }
    });
}

/// The rows shown: all, or those with the filter's text in a cell.
fn shown_rows(doc: &TableDoc) -> Vec<usize> {
    let t = doc.edit.table();
    let needle = mg_core::text::Needle::new(&doc.filter);
    if needle.is_empty() {
        return (0..t.len()).collect();
    }
    // (And by the names shown for the values of a list or of flags.)
    let description = doc.description().filter(|_| doc.value_names);
    let kinds: Vec<Option<&Kind>> =
        t.columns().iter().map(|c| Some(&description.as_deref()?.column(c)?.kind)).collect();
    // (Every word in one of the row's cells, or their names.)
    (0..t.len())
        .filter(|&r| {
            let mut fields = Vec::new();
            for (c, kind) in kinds.iter().enumerate() {
                if let Some(v) = t.cell(r, c) {
                    fields.push(mg_core::text::fold(v));
                    fields.extend(kind.and_then(|k| k.named(v)).map(|n| mg_core::text::fold(&n)));
                }
            }
            needle.finds_in(&fields.iter().map(String::as_str).collect::<Vec<_>>())
        })
        .collect()
}

/// Gives the keys to a table's grid: the arrows, and Undo and Redo, are
/// that table's from then on.
fn take_keys(ctx: &egui::Context, table: u32) {
    give_keys(ctx, Keys::Table);
    ctx.data_mut(|d| d.insert_temp(egui::Id::new("table-keys-of"), table));
}

/// The table whose grid was given the keys last.
fn keys_table(ctx: &egui::Context) -> Option<u32> {
    ctx.data(|d| d.get_temp(egui::Id::new("table-keys-of")))
}

/// The keys in the grid, from a click on a cell: the arrows, Tab, Page Up
/// and Down, Home and End move; Enter or F2 types into the cell, Delete
/// empties it; Ctrl+C copies it, Ctrl+V pastes cells from there on;
/// Undo and Redo (Ctrl+Z and Ctrl+Y, or the keys they have in Options ›
/// Keyboard) are the table's while the keys are in its grid.
fn keys(app: &mut Moonglow, ui: &mut Ui, at: usize) {
    let doc = &mut app.tables[at];
    // "Go to the next wrong cell", from the header's button.
    if doc.go_to == "!" {
        doc.go_to.clear();
        if let Some((_, found)) = &doc.faults
            && !found.is_empty()
        {
            let now = doc.selected.unwrap_or((usize::MAX, usize::MAX));
            let next = (found.iter().find(|f| (f.row, f.column) > now))
                .or(found.first())
                .map(|f| (f.row, f.column));
            doc.anchor = None;
            doc.selected = next;
            doc.filter.clear();
            doc.reveal = true;
            take_keys(ui.ctx(), doc.id);
        }
    }
    // Undo and Redo are the table's while the keys are in its grid (the
    // keymap's, through `Moonglow::local_undo`).
    if keys_are(ui.ctx(), Keys::Table) && keys_table(ui.ctx()) == Some(doc.id) {
        app.undo_keys = Some(Tab::Table(doc.id));
    }
    if !keys_are(ui.ctx(), Keys::Table) || doc.typing.is_some() {
        return;
    }
    if ui.ctx().egui_wants_keyboard_input() {
        return;
    }
    let Some((row, column)) = doc.selected else { return };
    let rows = shown_rows(doc);
    let Some(place) = rows.iter().position(|r| *r == row) else { return };
    let columns = doc.edit.table().columns().len();
    use egui::Key;
    let none = egui::Modifiers::NONE;
    let pressed = |ui: &mut Ui, m: egui::Modifiers, k: Key| ui.input_mut(|i| i.consume_key(m, k));
    let last = rows.len() as isize - 1;
    // (Shift held: the selection reaches to where the keys go.)
    let shift = ui.input(|i| i.modifiers.shift);
    let mut to = None;
    for (key, by) in [
        (Key::ArrowDown, 1),
        (Key::ArrowUp, -1),
        (Key::PageDown, 15),
        (Key::PageUp, -15),
        (Key::End, isize::MAX / 2),
        (Key::Home, isize::MIN / 2),
    ] {
        if pressed(ui, none, key) {
            let next = (place as isize).saturating_add(by).clamp(0, last) as usize;
            to = Some((rows[next], column));
        }
    }
    let (mut tab, mut back_tab) = (false, false);
    if pressed(ui, egui::Modifiers::SHIFT, Key::Tab) {
        back_tab = true;
    } else if pressed(ui, none, Key::Tab) {
        tab = true;
    }
    if pressed(ui, none, Key::ArrowRight) || tab {
        to = Some((row, (column + 1).min(columns.saturating_sub(1))));
    }
    if pressed(ui, none, Key::ArrowLeft) || back_tab {
        to = Some((row, column.saturating_sub(1)));
    }
    if let Some(to) = to {
        match shift && !tab && !back_tab {
            true => doc.select_to(to),
            false => {
                doc.anchor = None;
                doc.selected = Some(to);
            }
        }
        doc.reveal = true;
    }
    let Some((row, column)) = doc.selected else { return };
    // The rows selected, of those shown, and the columns.
    let (first, last_row, left, right) = doc.area().unwrap_or((row, row, column, column));
    let chosen: Vec<usize> =
        rows.iter().copied().filter(|r| (first..=last_row).contains(r)).collect();
    let several = chosen.len() > 1 || right > left;
    if pressed(ui, none, Key::Enter) || pressed(ui, none, Key::F2) {
        let now = doc.edit.table().cell(row, column).unwrap_or_default().to_string();
        doc.typing = Some(now);
    }
    if pressed(ui, none, Key::Delete) && doc.edit.clear_cells(&chosen, left, right) > 0 {
        doc.changed();
    }
    if pressed(ui, none, Key::Escape) {
        give_keys(ui.ctx(), Keys::Area);
    }
    let (copy, cut, paste) = crate::widgets::copy_cut_paste(ui);
    if copy || cut {
        // A line a row, tabs between the cells, as a spreadsheet copies
        // them (and as Paste reads them).
        let t = doc.edit.table();
        let lines: Vec<String> = chosen
            .iter()
            .map(|&r| {
                let cells: Vec<&str> =
                    (left..=right).map(|c| t.cell(r, c).unwrap_or("****")).collect();
                cells.join("\t")
            })
            .collect();
        ui.ctx().copy_text(lines.join("\n"));
        if cut && doc.edit.clear_cells(&chosen, left, right) > 0 {
            doc.changed();
        }
    }
    // (Pasted from the selection's first cell on.)
    let (row, column) = match several {
        true => (chosen.first().copied().unwrap_or(row), left),
        false => (row, column),
    };
    if paste {
        let text = ui.input(|i| {
            i.events.iter().find_map(|e| match e {
                egui::Event::Paste(t) => Some(t.clone()),
                _ => None,
            })
        });
        if let Some(text) = text {
            match doc.edit.paste(row, column, &text) {
                Ok(0) => {}
                Ok(_) => doc.changed(),
                Err(e) => doc.problem = Some(e.to_string()),
            }
        }
    }
}

fn grid(
    app: &mut Moonglow,
    ui: &mut Ui,
    at: usize,
    kinds: &[(&'static str, ColumnKind)],
    faults: &[CellFault],
) {
    let game = app.game.clone();
    let settings = &mut app.settings;
    let doc = &mut app.tables[at];
    let rows = shown_rows(doc);
    let columns: Vec<String> = doc.edit.table().columns().to_vec();
    let table_name = doc.name();
    let widths = column_widths(settings, &table_name, &columns, CELL);
    // Where each column begins, from the first's left.
    let lefts: Vec<f32> = widths
        .iter()
        .scan(0.0, |x, w| {
            let left = *x;
            *x += w + ui.spacing().item_spacing.x;
            Some(left)
        })
        .collect();
    // The edge of a column's name dragged or double-clicked.
    let mut edge = None;
    let description = doc.description();
    let described = |c: &str| description.as_deref().and_then(|d| d.column(c));
    let kind_of = |c: &str| {
        let known = kinds.iter().find(|(k, _)| k.eq_ignore_ascii_case(c)).map(|(_, k)| *k);
        known.or_else(|| matches!(described(c)?.kind, Kind::StrRef).then_some(ColumnKind::StrRef))
    };
    let strref: Vec<bool> =
        columns.iter().map(|c| doc.strref_text && kind_of(c) == Some(ColumnKind::StrRef)).collect();
    // The columns whose values are shown by name.
    let value_names = doc.value_names;
    let named: Vec<Option<&Kind>> = columns
        .iter()
        .map(|c| {
            described(c)
                .map(|d| &d.kind)
                .filter(|k| value_names && matches!(k, Kind::List { .. } | Kind::Flags { .. }))
        })
        .collect();
    let row_height = ui.text_style_height(&egui::TextStyle::Monospace) + 6.0;
    let pitch = row_height + ui.spacing().item_spacing.y;
    let reveal = std::mem::take(&mut doc.reveal);
    let selected = doc.selected;
    let area = doc.area().filter(|a| (a.0, a.2) != (a.1, a.3));
    let dragging = doc.dragging && ui.input(|i| i.pointer.primary_down());
    doc.dragging = dragging;
    let pointer = ui.ctx().pointer_latest_pos();
    let shift = ui.input(|i| i.modifiers.shift);
    // A cell the selection is to reach to (Shift and a click, or a drag).
    let mut reach = None;
    let mut drag_from = None;
    let has_keys = keys_are(ui.ctx(), Keys::Table);
    let (mut clicked, mut begin, mut done, mut cancel) = (None, false, false, false);
    // A column's name, right-clicked: renamed (false) or removed (true).
    let mut column_act = None;
    // A column moved: from which place to which (its menu, or its name
    // dragged onto another's).
    let mut column_move = None;
    let moving = doc.moving.filter(|_| ui.input(|i| i.pointer.primary_down()));
    let released = doc.moving.is_some() && moving.is_none();
    let mut over_column = None;
    let wrong = ui.visuals().error_fg_color;
    // The column names stay in sight over the rows, and the rows' numbers
    // beside them: each goes with the rows' own scroll, a frame behind.
    let hidden = egui::scroll_area::ScrollBarVisibility::AlwaysHidden;
    let still = egui::scroll_area::ScrollSource::NONE;
    let (scroll_x, scroll_y) = (doc.scroll.x, doc.scroll.y);
    let id = doc.id;
    ui.horizontal(|ui| {
        ui.add_sized([NUMBER, row_height], egui::Label::new(""));
        egui::ScrollArea::horizontal()
            .id_salt(("table-head", id))
            .auto_shrink([false, true])
            .scroll_bar_visibility(hidden)
            .scroll_source(still)
            .horizontal_scroll_offset(scroll_x)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    for (index, c) in columns.iter().enumerate() {
                        let text = egui::RichText::new(c).strong();
                        let (rect, r) = ui.allocate_exact_size(
                            egui::vec2(widths[index], row_height),
                            egui::Sense::click_and_drag(),
                        );
                        // Dragged by its name to another column's place.
                        if r.drag_started() {
                            doc.moving = Some(index);
                        }
                        if doc.moving.is_some() && pointer.is_some_and(|p| rect.contains(p)) {
                            over_column = Some(index);
                            if doc.moving != Some(index) {
                                // (Where it would go: before or after this one.)
                                let before = doc.moving.is_some_and(|from| from > index);
                                let x = if before { rect.left() } else { rect.right() };
                                let stroke = egui::Stroke::new(2.0, ui.visuals().selection.bg_fill);
                                ui.painter().vline(x, rect.y_range(), stroke);
                            }
                        }
                        r.context_menu(|ui| {
                            if ui.button("Rename Column…").clicked() {
                                column_act = Some((index, false));
                                ui.close();
                            }
                            let left = ui.add_enabled(index > 0, egui::Button::new("Move Left"));
                            if left.clicked() {
                                column_move = Some((index, index - 1));
                                ui.close();
                            }
                            let last = index + 1 == columns.len();
                            if ui.add_enabled(!last, egui::Button::new("Move Right")).clicked() {
                                column_move = Some((index, index + 1));
                                ui.close();
                            }
                            let more = columns.len() > 1;
                            if ui
                                .add_enabled(more, egui::Button::new("Remove Column"))
                                .on_disabled_hover_text("A table keeps one column at least")
                                .clicked()
                            {
                                column_act = Some((index, true));
                                ui.close();
                            }
                        });
                        let inner = rect.shrink2(egui::vec2(4.0, 0.0));
                        // (In the cell's place, taking none of its own: the
                        // cells after it stay in their columns.)
                        let layout = egui::Layout::left_to_right(egui::Align::Center);
                        ui.new_child(egui::UiBuilder::new().max_rect(inner).layout(layout))
                            .add(egui::Label::new(text).truncate().selectable(false));
                        let about = match (described(c), kind_of(c)) {
                            (Some(d), _) => match &d.note {
                                Some(note) => format!("{c}: {}\n{note}", d.kind.words()),
                                None => format!("{c}: {}", d.kind.words()),
                            },
                            (None, Some(ColumnKind::StrRef)) => {
                                format!("{c}: a talk-table string's number")
                            }
                            (None, Some(ColumnKind::Model)) => format!("{c}: a model's name"),
                            (None, Some(ColumnKind::Row(t))) => format!("{c}: a row of {t}.2da"),
                            (None, Some(ColumnKind::RowBy { by, tables })) => {
                                let of: Vec<String> =
                                    tables.iter().map(|(v, t)| format!("{v}: {t}.2da")).collect();
                                format!(
                                    "{c}: a row of the table its {by} chooses ({})",
                                    of.join(", ")
                                )
                            }
                            (None, Some(ColumnKind::Table)) => {
                                format!("{c}: a table's name, without .2da")
                            }
                            (None, None) => c.clone(),
                        };
                        r.on_hover_text(
                            about
                                + "\nDrag: move the column. Drag its right edge: its width \
                                   (double click there: as wide as its widest cell). \
                                   Right-click: rename, move or remove it",
                        );
                        // (On top of the name: its right end is the edge's.)
                        let edge_id = egui::Id::new(("table-edge", id, index));
                        if doc.moving.is_none()
                            && let Some(act) = column_edge(ui, edge_id, rect, c)
                        {
                            edge = Some((index, act));
                        }
                    }
                    // (Room for the rows' own scroll bar at the right.)
                    ui.add_space(ui.spacing().scroll.bar_width + 12.0);
                });
            });
    });
    match edge {
        Some((c, Edge::Width(w))) => set_column_width(settings, &table_name, &columns[c], w),
        Some((c, Edge::Fit)) => {
            let t = doc.edit.table();
            let cells = rows
                .iter()
                .map(|&r| cell_shows(t.cell(r, c), strref[c], named[c], game.as_deref()).0);
            let w = fit_width(ui, &columns[c], cells);
            set_column_width(settings, &table_name, &columns[c], w);
        }
        None => {}
    }
    if released {
        // (Let go over another column's name: moved there.)
        if let (Some(from), Some(to)) = (doc.moving.take(), over_column) {
            column_move = Some((from, to));
        }
    }
    if let Some((from, to)) = column_move {
        doc.typing = None;
        if doc.edit.move_column(from, to) {
            // (The selected cell goes with its column.)
            doc.anchor = None;
            doc.selected = doc.selected.map(|(r, c)| {
                let c = if c == from {
                    to
                } else if from < c && c <= to {
                    c - 1
                } else if to <= c && c < from {
                    c + 1
                } else {
                    c
                };
                (r, c)
            });
            doc.changed();
        }
        return;
    }
    match column_act {
        Some((column, true)) => {
            doc.typing = None;
            if doc.edit.remove_column(column) {
                doc.anchor = None;
                let last = doc.edit.table().columns().len().checked_sub(1);
                doc.selected = doc.selected.and_then(|(r, c)| Some((r, c.min(last?))));
                doc.changed();
            }
            return;
        }
        Some((column, false)) => {
            doc.commit();
            doc.new_column = Some(columns[column].clone());
            doc.renaming = Some(column);
        }
        None => {}
    }
    let mut list =
        egui::ScrollArea::both().id_salt(("table-rows", doc.id)).auto_shrink([false, false]);
    if reveal && let Some((row, column)) = selected {
        if let Some(place) = rows.iter().position(|r| *r == row) {
            let above = (ui.available_height() / 2.0 - pitch).max(0.0);
            list = list.vertical_scroll_offset((place as f32 * pitch - above).max(0.0));
        }
        // Sideways only where the cell is out of sight.
        let left = lefts.get(column).copied().unwrap_or_default();
        let step = widths.get(column).copied().unwrap_or(CELL) + ui.spacing().item_spacing.x;
        let width = ui.available_width() - NUMBER;
        if left < scroll_x || left + step > scroll_x + width {
            list = list.horizontal_scroll_offset((left - width / 2.0).max(0.0));
        }
    }
    let room = ui.available_height();
    let bar = ui.spacing().scroll.bar_width + ui.spacing().scroll.bar_inner_margin;
    let shown = ui
        .horizontal_top(|ui| {
            let gutter = egui::vec2(NUMBER, (room - bar).max(row_height));
            let down = egui::Layout::top_down(egui::Align::Min);
            ui.allocate_ui_with_layout(gutter, down, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt(("table-numbers", id))
                    .auto_shrink([false, false])
                    .scroll_bar_visibility(hidden)
                    .scroll_source(still)
                    .vertical_scroll_offset(scroll_y)
                    .show_rows(ui, row_height, rows.len(), |ui, range| {
                        for &r in &rows[range] {
                            let number = egui::RichText::new(r.to_string()).weak().monospace();
                            let label = egui::Label::new(number).selectable(false);
                            ui.add_sized([NUMBER - 8.0, row_height], label);
                        }
                    });
            });
            ui.vertical(|ui| {
                list.show_rows(ui, row_height, rows.len(), |ui, range| {
                    for &r in &rows[range] {
                        ui.horizontal(|ui| {
                            for (c, is_strref) in strref.iter().copied().enumerate() {
                                let width = widths[c];
                                let here = selected == Some((r, c));
                                if here && let Some(text) = doc.typing.as_mut() {
                                    let edit = egui::TextEdit::singleline(text)
                                        .id(egui::Id::new(("table-cell", id)))
                                        .font(egui::TextStyle::Monospace)
                                        .margin(egui::Margin::symmetric(4, 0))
                                        .vertical_align(egui::Align::Center);
                                    // (In the cell's own place, as wide.)
                                    let (rect, _) = ui.allocate_exact_size(
                                        egui::vec2(width, row_height),
                                        egui::Sense::hover(),
                                    );
                                    // (A place of its own: what the field takes beyond
                                    // the cell does not move the cells after it.)
                                    let mut within =
                                        ui.new_child(egui::UiBuilder::new().max_rect(rect));
                                    let field = within.add_sized(rect.size(), edit);
                                    crate::widgets::leaves_at_a_press(ui.ctx(), &field);
                                    if !crate::widgets::has_keys(&field) && !field.lost_focus() {
                                        field.request_focus();
                                    }
                                    if field.lost_focus() {
                                        match ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                                            true => cancel = true,
                                            false => done = true,
                                        }
                                    }
                                    continue;
                                }
                                let raw = doc.edit.table().cell(r, c);
                                let fault = faults.iter().find(|f| f.row == r && f.column == c);
                                let (label, text, was_named) =
                                    cell_shows(raw, is_strref, named[c], game.as_deref());
                                let mut rich = egui::RichText::new(label).monospace();
                                if raw.is_none() {
                                    rich = rich.weak();
                                }
                                if fault.is_some() {
                                    rich = rich.color(wrong);
                                }
                                let (rect, cell) = ui.allocate_exact_size(
                                    egui::vec2(width, row_height),
                                    egui::Sense::click_and_drag(),
                                );
                                if cell.drag_started() {
                                    drag_from = Some((r, c));
                                } else if dragging && pointer.is_some_and(|p| rect.contains(p)) {
                                    reach = Some((r, c));
                                }
                                let within = area.is_some_and(|(r0, r1, c0, c1)| {
                                    (r0..=r1).contains(&r) && (c0..=c1).contains(&c)
                                });
                                if within && !here {
                                    let fill = ui.visuals().selection.bg_fill.gamma_multiply(0.45);
                                    ui.painter().rect_filled(rect, 0.0, fill);
                                }
                                if here {
                                    let fill = match has_keys {
                                        true => ui.visuals().selection.bg_fill,
                                        false => ui.visuals().widgets.inactive.weak_bg_fill,
                                    };
                                    ui.painter().rect_filled(rect, 2.0, fill);
                                } else if cell.hovered() {
                                    ui.painter().rect_filled(
                                        rect,
                                        2.0,
                                        ui.visuals().widgets.hovered.weak_bg_fill,
                                    );
                                }
                                if fault.is_some() {
                                    let stroke = egui::Stroke::new(1.0, wrong);
                                    ui.painter().rect_stroke(
                                        rect.shrink(1.0),
                                        2.0,
                                        stroke,
                                        egui::StrokeKind::Inside,
                                    );
                                }
                                let inner = rect.shrink2(egui::vec2(4.0, 0.0));
                                // (In the cell's place, taking none of its own: the
                                // cells after it stay in their columns.)
                                let layout = egui::Layout::left_to_right(egui::Align::Center);
                                ui.new_child(egui::UiBuilder::new().max_rect(inner).layout(layout))
                                    .add(egui::Label::new(rich).truncate().selectable(false));
                                let mut tip = Vec::new();
                                if was_named {
                                    tip.push(raw.unwrap_or_default().to_string());
                                } else if text.is_some() {
                                    tip.push(format!("StrRef {}", raw.unwrap_or_default()));
                                }
                                if let Some(f) = fault {
                                    tip.push(f.message.clone());
                                }
                                let cell = match tip.is_empty() {
                                    true => cell,
                                    false => cell.on_hover_text(tip.join("\n")),
                                };
                                if cell.clicked() && shift {
                                    reach = Some((r, c));
                                } else if cell.clicked() || cell.double_clicked() {
                                    clicked = Some((r, c));
                                    begin = cell.double_clicked();
                                }
                            }
                        });
                    }
                })
            })
            .inner
        })
        .inner;
    doc.scroll = shown.state.offset;
    crate::palette_view::keys_outline(ui, shown.inner_rect, Keys::Table);
    if done {
        doc.commit();
        // (Enter: on to the row below, as a spreadsheet does.)
        if let Some((row, column)) = doc.selected
            && ui.input(|i| i.key_pressed(egui::Key::Enter))
            && let Some(place) = rows.iter().position(|r| *r == row)
            && let Some(next) = rows.get(place + 1)
        {
            doc.anchor = None;
            doc.selected = Some((*next, column));
            doc.reveal = true;
        }
        take_keys(ui.ctx(), doc.id);
    }
    if cancel {
        doc.typing = None;
        take_keys(ui.ctx(), doc.id);
    }
    if let Some(cell) = drag_from {
        doc.commit();
        doc.selected = Some(cell);
        doc.anchor = None;
        doc.dragging = true;
        take_keys(ui.ctx(), doc.id);
    } else if let Some(cell) = reach.filter(|c| Some(*c) != doc.selected) {
        doc.commit();
        doc.select_to(cell);
        take_keys(ui.ctx(), doc.id);
    }
    if let Some(cell) = clicked {
        doc.commit();
        doc.anchor = None;
        doc.selected = Some(cell);
        take_keys(ui.ctx(), doc.id);
        if begin {
            let now = doc.edit.table().cell(cell.0, cell.1).unwrap_or_default().to_string();
            doc.typing = Some(now);
        }
    }
}

/// A row's name for a list of rows to choose from: its number and its
/// first cell that says something.
fn row_label(t: &TwoDa, row: usize) -> String {
    let named = ["Label", "LABEL", "Name", "NAME"].iter().find_map(|c| t.get(row, c));
    let text = named.or_else(|| (0..t.columns().len()).find_map(|c| t.cell(row, c)));
    match text {
        Some(text) => format!("{row}  {text}"),
        None => format!("{row}  ****"),
    }
}

/// Under the grid: the selected cell, by what its column holds.
fn inspector(
    app: &mut Moonglow,
    ui: &mut Ui,
    at: usize,
    kinds: &[(&'static str, ColumnKind)],
    faults: &[CellFault],
) {
    let game = app.game.clone();
    let name = app.tables[at].name();
    let Some((row, column)) = app.tables[at].selected else {
        ui.weak(
            "Click a cell to select it; double click, Enter or F2 to type in it; Delete \
             empties it.",
        );
        return;
    };
    let table: &TwoDa = app.tables[at].edit.table();
    let Some(column_name) = table.columns().get(column).cloned() else { return };
    let value = table.cell(row, column).map(str::to_string);
    let kind = kinds.iter().find(|(k, _)| k.eq_ignore_ascii_case(&column_name)).map(|(_, k)| *k);
    // What the table's own description says of the column comes first.
    let description = app.tables[at].description();
    let described = description.as_deref().and_then(|d| d.column(&column_name));
    let kind = match described.map(|d| &d.kind) {
        Some(Kind::StrRef) => Some(ColumnKind::StrRef),
        Some(Kind::Text) | None => kind,
        Some(_) => None,
    };
    let id = app.tables[at].id;
    // A table the description names: the project's own, else the game's.
    let files = app.tables[at].described.as_ref();
    let cp = codepage(app);
    let named_table = |n: &str| -> Option<Arc<TwoDa>> {
        let own = files.and_then(|d| {
            let mut read = d.tables.borrow_mut();
            read.entry(n.to_ascii_lowercase())
                .or_insert_with(|| d.files.table(n, cp).map(Arc::new))
                .clone()
        });
        own.or_else(|| game.as_deref().and_then(|g| g.table(n).ok()))
    };
    let mut set: Option<Option<String>> = None;
    let mut find_references = false;
    let mut talk = false;
    ui.horizontal_wrapped(|ui| {
        ui.strong(format!("Row {row}, {column_name}"));
        ui.monospace(value.as_deref().unwrap_or("****"));
        // The row of the table whose rows these are.
        if let Some(of) = description.as_deref().and_then(|d| d.rows.as_deref())
            && let Some(other) = named_table(of)
            && row < other.len()
        {
            ui.weak(format!("({} of {of}.2da)", row_label(&other, row)));
        }
        match described.map(|d| &d.kind) {
            Some(Kind::List { entries, other }) => {
                let now = value.as_deref().and_then(|v| described?.kind.named(v));
                let shown = match (&now, &value) {
                    (Some(name), _) => name.clone(),
                    (None, Some(v)) if *other => format!("{v}  (its own value)"),
                    (None, Some(v)) => format!("{v}  (none of the list)"),
                    (None, None) => "none".to_string(),
                };
                ui.weak(if *other { "one of, or another value:" } else { "one of:" });
                // (The value as the list writes it, when it names one of
                // the list's.)
                let was = match &now {
                    Some(name) => entries.iter().find(|e| e.name == *name).map(|e| e.value.clone()),
                    None => value.clone(),
                };
                let mut pick = was.clone();
                let options = || {
                    std::iter::once(None)
                        .chain(entries.iter().map(|e| Some(e.value.clone())))
                        .collect()
                };
                crate::widgets::dropdown(("table-list-choice", id))
                    .selected_text(shown)
                    .width(300.0)
                    .show_with(ui, &mut pick, options, |ui, pick| {
                        if ui.selectable_label(value.is_none(), "none (****)").clicked() {
                            *pick = None;
                            set = Some(None);
                        }
                        for e in entries {
                            let is = now.as_deref() == Some(e.name.as_str());
                            let label = format!("{}  ({})", e.name, e.value);
                            if ui.selectable_label(is, label).clicked() {
                                *pick = Some(e.value.clone());
                                set = Some(Some(e.value.clone()));
                            }
                        }
                    });
                if set.is_none() && pick != was {
                    set = Some(pick);
                }
            }
            Some(kind @ Kind::Flags { entries }) => {
                let bits = value.as_deref().and_then(mg_module::table_desc::whole).unwrap_or(0);
                let (on, rest) = mg_module::table_desc::flags_of(entries, bits);
                let shown = match on.len() {
                    0 => "none".to_string(),
                    1 => on[0].name.clone(),
                    n => format!("{n} flags"),
                };
                ui.weak("flags:");
                // (A set of ticks, not one choice: no arrow keys.)
                crate::widgets::dropdown(("table-flags-choice", id))
                    .selected_text(shown)
                    .width(300.0)
                    .stays_open()
                    .show_ui(ui, |ui| {
                        for e in entries {
                            let Some(v) =
                                mg_module::table_desc::whole(&e.value).filter(|v| *v != 0)
                            else {
                                continue;
                            };
                            let mut ticked = bits & v == v;
                            let label = format!("{}  ({})", e.name, e.value);
                            if ui.checkbox(&mut ticked, label).changed() {
                                let next = if ticked { bits | v } else { bits & !v };
                                set = Some(kind.flags_cell(next, value.as_deref()));
                            }
                        }
                    });
                // (Each by name beside the list, as the cell has them.)
                for e in &on {
                    ui.label(&e.name);
                }
                if rest != 0 {
                    ui.colored_label(ui.visuals().warn_fg_color, format!("{rest:#X}"));
                }
            }
            Some(Kind::Row(target)) => {
                let named = value
                    .as_deref()
                    .and_then(mg_2da::parse_int)
                    .and_then(|v| usize::try_from(v).ok());
                let other = named_table(target);
                row_choice(ui, id, (&name, table), (target, other.as_deref()), named, &mut set);
            }
            Some(kind @ (Kind::Int { .. } | Kind::Float { .. })) => {
                ui.weak(kind.words());
            }
            _ => {}
        }
        if let Some(note) = described.and_then(|d| d.note.as_deref()) {
            ui.weak(note);
        }
        match kind {
            Some(ColumnKind::StrRef) => {
                let text = value
                    .as_deref()
                    .and_then(|v| v.parse::<u32>().ok())
                    .and_then(|n| game.as_deref()?.string(mg_core::StrRef(n)));
                match text {
                    Some(text) => ui.label(format!("\u{201c}{text}\u{201d}")),
                    None => ui.weak("a talk-table string's number"),
                };
                if ui.button("Talk Table").on_hover_text("Open the talk table editor").clicked() {
                    talk = true;
                }
            }
            Some(ColumnKind::Model) => {
                ui.weak("a model's name");
            }
            Some(k @ (ColumnKind::Row(_) | ColumnKind::RowBy { .. })) => {
                // (Of a requirement, the table its kind names, if any.)
                match k.row_table(table, row) {
                    Some(target) => {
                        let now = value.as_deref().and_then(mg_2da::parse_int);
                        let named =
                            now.and_then(|v| mg_module::doctor::row_named(&name, &column_name, v));
                        let other = game.as_deref().and_then(|g| g.table(target).ok());
                        let other = (target, other.as_deref());
                        row_choice(ui, id, (&name, table), other, named, &mut set);
                    }
                    None => {
                        ui.weak("not a row, for this row's kind");
                    }
                }
            }
            Some(ColumnKind::Table) => {
                ui.weak("a table's name, without .2da");
            }
            None => {}
        }
        if let Some(f) = faults.iter().find(|f| f.row == row && f.column == column) {
            ui.colored_label(ui.visuals().error_fg_color, &f.message);
        }
    });
    ui.horizontal_wrapped(|ui| {
        if ui
            .button(format!("Find References to Row {row}"))
            .on_hover_text(
                "The cells of other tables (and this one) that name this row, in the columns \
                 known to name a row of this table",
            )
            .clicked()
        {
            find_references = true;
        }
        if let Some((of, found)) = &app.tables[at].references
            && *of == row
        {
            if found.is_empty() {
                ui.weak("No table's known columns name it.");
            }
            for r in found.iter().take(12) {
                ui.label(format!("{}.2da row {}, {}", r.table, r.row, r.column));
            }
            if found.len() > 12 {
                ui.weak(format!("and {} more", found.len() - 12));
            }
        }
    });
    let found = find_references.then(|| references(app, &name, table, row));
    if let Some(value) = set {
        let doc = &mut app.tables[at];
        doc.typing = None;
        match doc.edit.set(row, column, value) {
            Ok(true) => doc.changed(),
            Ok(false) => {}
            Err(e) => doc.problem = Some(e.to_string()),
        }
    }
    if let Some(found) = found {
        app.tables[at].references = Some((row, found));
    }
    if talk {
        app.actions.push(Action::OpenTab(Tab::TalkTable));
    }
}

/// A row of the table `target` to choose (the table itself as it is now,
/// for a row of its own; else `other`, as read), with the one named now.
fn row_choice(
    ui: &mut Ui,
    id: u32,
    (name, table): (&str, &TwoDa),
    (target, other): (&str, Option<&TwoDa>),
    named: Option<usize>,
    set: &mut Option<Option<String>>,
) {
    let other = if target.eq_ignore_ascii_case(name) { Some(table) } else { other };
    let shown = match (&other, named) {
        (Some(t), Some(n)) if n < t.len() => row_label(t, n),
        (_, Some(n)) => format!("{n}  (no such row)"),
        _ => "none".to_string(),
    };
    ui.weak(format!("a row of {target}.2da:"));
    let Some(t) = other else { return };
    // (Every row's name only while the list is open or has the keys: a
    // table may have thousands.)
    let id = egui::Id::new(("table-row-choice", id));
    let rows = if crate::widgets::RowChoice::in_use(ui, id) {
        0..t.len()
    } else {
        named.filter(|&n| n < t.len()).map_or(0..0, |n| n..n + 1)
    };
    let choices: Vec<mg_rules::Choice> =
        rows.map(|row| mg_rules::Choice { row, text: row_label(t, row) }).collect();
    let picked = crate::widgets::row_choice(id, &choices, named.map(|n| n as i64))
        .width(260.0)
        .ordered()
        .none("none (****)")
        .shown(shown)
        .show_optional(ui);
    if let Some(picked) = picked {
        *set = Some(picked.map(|r| r.to_string()));
    }
}

/// Add Column…: its name.
fn column_window(app: &mut Moonglow, ctx: &egui::Context, at: usize) {
    let settings = &mut app.settings;
    let doc = &mut app.tables[at];
    let Some(mut name) = doc.new_column.clone() else { return };
    let mut open = true;
    let mut cancelled = false;
    let mut add = false;
    let renaming = doc.renaming.filter(|c| *c < doc.edit.table().columns().len());
    let title = if renaming.is_some() { "Rename Column" } else { "Add Column" };
    crate::widgets::dialog(ctx, title, &mut open).collapsible(false).resizable(false).show(
        ctx,
        |ui| {
            ui.label("The column's name (no spaces)");
            let field = ui.add(egui::TextEdit::singleline(&mut name).desired_width(220.0));
            crate::widgets::autofocus(ui, &field);
            let fits = !name.trim().is_empty() && !name.contains([' ', '\t', '"']);
            // (Its own name in other letters is no other column's.)
            let taken = doc.edit.table().column(name.trim()).is_some_and(|c| Some(c) != renaming);
            if taken {
                ui.colored_label(
                    ui.visuals().error_fg_color,
                    "The table has a column of this name.",
                );
            }
            ui.horizontal(|ui| {
                let may = fits && !taken;
                let go = if renaming.is_some() { "Rename" } else { "Add" };
                if ui.add_enabled(may, egui::Button::new(go)).clicked()
                    || (may && crate::widgets::enter(ui))
                {
                    add = true;
                }
                if crate::widgets::cancel(ui) {
                    cancelled = true;
                }
            });
        },
    );
    if add {
        let done = match renaming {
            Some(column) => {
                let was = doc.edit.table().columns()[column].clone();
                let done = doc.edit.rename_column(column, name.trim());
                if done.is_ok() {
                    rename_column_width(settings, &doc.name(), &was, name.trim());
                }
                done.map(|_| ())
            }
            None => doc.edit.add_column(name.trim()),
        };
        match done {
            Ok(()) => doc.changed(),
            Err(e) => doc.problem = Some(e.to_string()),
        }
        doc.new_column = None;
    } else {
        doc.new_column = (open && !cancelled).then_some(name);
    }
    if doc.new_column.is_none() {
        doc.renaming = None;
    }
}

/// Take Rows…: the rows another copy has and this table lacks or has
/// differently, to choose from.
fn taking_window(app: &mut Moonglow, ctx: &egui::Context, at: usize, others: &[(String, TwoDa)]) {
    let doc = &mut app.tables[at];
    let Some(mut taking) = doc.taking.clone() else { return };
    let mut open = true;
    let mut cancelled = false;
    let mut take = false;
    let columns: Vec<String> = doc.edit.table().columns().to_vec();
    // The lowest copy is what the others are told from.
    let base = others.last().filter(|_| others.len() > 1).map(|(_, t)| t);
    if taking.from.as_ref().is_none_or(|f| !others.iter().any(|(l, _)| l == f)) {
        taking.from = others.first().map(|(l, _)| l.clone());
        taking.chosen.clear();
    }
    let from = taking.from.clone();
    let other = from.as_ref().and_then(|f| others.iter().find(|(l, _)| l == f));
    // (Whose change a difference is can be told only against a lower
    // copy. Worked out once for a copy and the table as it is.)
    let kept =
        taking.found.as_ref().filter(|(l, r, _, _)| Some(l) == from.as_ref() && *r == doc.revision);
    let (found, told): (Arc<Vec<Candidate>>, bool) = match (kept, other) {
        (Some((_, _, found, told)), _) => (found.clone(), *told),
        (None, Some((label, t))) => {
            let base = base.filter(|_| others.last().is_some_and(|(l, _)| l != label));
            let found = Arc::new(mg_module::table_edit::candidates(doc.edit.table(), t, base));
            taking.found = Some((label.clone(), doc.revision, found.clone(), base.is_some()));
            (found, base.is_some())
        }
        (None, None) => (Arc::default(), false),
    };
    crate::widgets::dialog(ctx, "Take Rows from Another Copy", &mut open)
        .default_size([760.0, 520.0])
        .show(ctx, |ui| {
            if others.is_empty() {
                ui.label("No other copy of this table is in the load order.");
                return;
            }
            ui.horizontal(|ui| {
                ui.label("From");
                let labels: Vec<Option<String>> =
                    others.iter().map(|(label, _)| Some(label.clone())).collect();
                let mut pick = from.clone();
                crate::widgets::dropdown(("take-from", doc.id))
                    .selected_text(from.clone().unwrap_or_default())
                    .width(260.0)
                    .show(ui, &mut pick, &labels, |l| {
                        let rows = others.iter().find(|(o, _)| Some(o) == l.as_ref());
                        format!(
                            "{} ({} rows)",
                            l.as_deref().unwrap_or(""),
                            rows.map_or(0, |o| o.1.len())
                        )
                    });
                if pick != from {
                    taking.from = pick;
                    taking.chosen.clear();
                }
                let count = |o: Offered| found.iter().filter(|c| c.offered == o).count();
                ui.weak(match told {
                    true => format!(
                        "{} new, {} changed there, {} changed in both",
                        count(Offered::New),
                        count(Offered::Changed),
                        count(Offered::Conflict)
                    ),
                    false => format!(
                        "{} new, {} that differ (the lowest copy: nothing to tell by whose \
                         change it is)",
                        count(Offered::New),
                        count(Offered::Changed)
                    ),
                });
            });
            ui.horizontal(|ui| {
                // A box is ticked while every row of its kind is.
                let kinds = [
                    (Offered::New, "New", "The rows this table lacks"),
                    (
                        Offered::Changed,
                        "Differ",
                        "The rows the other copy has differently, which this table did not \
                         change itself (where that can be told)",
                    ),
                ];
                for (kind, label, hint) in kinds {
                    let rows: Vec<_> =
                        found.iter().filter(|c| c.offered == kind).map(|c| c.row).collect();
                    let mut all =
                        !rows.is_empty() && rows.iter().all(|r| taking.chosen.contains(r));
                    let tick =
                        ui.add_enabled(!rows.is_empty(), egui::Checkbox::new(&mut all, label));
                    if tick.on_hover_text(hint).changed() {
                        match all {
                            true => taking.chosen.extend(rows),
                            false => rows.iter().for_each(|r| {
                                taking.chosen.remove(r);
                            }),
                        }
                    }
                }
                if ui.button("Clear Selections").clicked() {
                    taking.chosen.clear();
                }
            });
            ui.separator();
            if found.is_empty() {
                ui.weak("This table has every row of that copy as it has them.");
            }
            let height = ui.text_style_height(&egui::TextStyle::Body) + 6.0;
            let room = (ui.available_height() - 40.0).max(120.0);
            egui::ScrollArea::vertical().max_height(room).auto_shrink([false, false]).show_rows(
                ui,
                height,
                found.len(),
                |ui, range| {
                    for c in &found[range] {
                        ui.horizontal(|ui| {
                            let mut on = taking.chosen.contains(&c.row);
                            let what = match c.offered {
                                Offered::New => "new",
                                Offered::Changed if told => "changed there",
                                Offered::Changed => "differs",
                                Offered::Conflict => "changed in both",
                            };
                            if ui.checkbox(&mut on, format!("Row {}", c.row)).changed() {
                                match on {
                                    true => _ = taking.chosen.insert(c.row),
                                    false => _ = taking.chosen.remove(&c.row),
                                }
                            }
                            let text = egui::RichText::new(what);
                            ui.label(match c.offered {
                                Offered::Conflict => text.color(ui.visuals().warn_fg_color),
                                _ => text.weak(),
                            });
                            // The cells that differ: here, then there.
                            let cells: Vec<String> = c
                                .cells
                                .iter()
                                .map(|(col, mine, theirs)| {
                                    let (mine, theirs) = (
                                        mine.as_deref().unwrap_or("****"),
                                        theirs.as_deref().unwrap_or("****"),
                                    );
                                    match c.offered {
                                        Offered::New => format!("{}: {theirs}", columns[*col]),
                                        _ => format!("{}: {mine} \u{2192} {theirs}", columns[*col]),
                                    }
                                })
                                .collect();
                            let line = cells.join("   ");
                            ui.add(
                                egui::Label::new(egui::RichText::new(&line).monospace()).truncate(),
                            )
                            .on_hover_text(cells.join("\n"));
                        });
                    }
                },
            );
            ui.separator();
            ui.horizontal(|ui| {
                let n = taking.chosen.len();
                if ui
                    .add_enabled(n > 0, egui::Button::new(format!("Take {n} Rows")))
                    .on_hover_text("Each row whole, as that copy has it; one step to undo")
                    .clicked()
                {
                    take = true;
                }
                if crate::widgets::cancel(ui) {
                    cancelled = true;
                }
            });
        });
    if take && let Some((_, t)) = other {
        let rows: Vec<usize> = taking.chosen.iter().copied().collect();
        if doc.edit.take_rows(t, &rows) > 0 {
            doc.changed();
        }
        doc.taking = None;
    } else {
        doc.taking = (open && !cancelled).then_some(taking);
    }
}
