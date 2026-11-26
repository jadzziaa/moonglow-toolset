//! The 2DA editor: a table's cells changed in place, rows added at the end
//! and emptied (never taken out: a row's number is what everything else
//! knows it by), cells checked by what their column holds as they are
//! typed, and rows taken from another copy of the table (a lower hak's,
//! which this one hides).
//!
//! A table is edited where it lives: in the module, in a hak open in the
//! hak editor (whose Save writes it), or in a file anywhere. The game's
//! own tables are never changed: Edit on one makes a copy somewhere.

use mg_uikit::Controls as _;
use mg_uikit::TokensExt as _;
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::Arc;

use egui::Ui;
use mg_2da::TwoDa;
use mg_core::{ResRef, ResType, StrRef};
use mg_edit::{Command, Edit};
use mg_module::doctor::{CellFault, ColumnKind, TalkTables};
use mg_module::table_desc::{Beside, Description, Kind};
use mg_module::table_edit::{Candidate, Editing, Offered, Whose};
use mg_resman::ResKey;

use crate::palette_view::{Keys, give_keys, keys_are};

pub(crate) mod panels;
use crate::talk_view::Pending;
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
    /// Cells chosen one by one (a row and this table's column), of rows
    /// both copies changed: taken where their row is not taken whole.
    pub cells: BTreeSet<(usize, usize)>,
    /// The row whose cells are being chosen.
    pub open_row: Option<usize>,
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
    /// Such a drag past the grid's edge: how far to scroll it at the
    /// next frame.
    edge_scroll: egui::Vec2,
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
    /// The file as it was read or last saved: a save writes the table
    /// as it is laid out (its widths, row labels and tabs), and what it
    /// was read from changing under it reads it again.
    read: Arc<[u8]>,
    /// When what the table was read from was last looked at (the time
    /// egui gives).
    read_at: f64,
    /// What told, when it was last read, whether what the table was read
    /// from had changed, without reading it ([`seen_now`]).
    seen: Option<Seen>,
    /// Talk-table columns show their text, not their numbers.
    pub strref_text: bool,
    /// The columns hidden from view, by name in lower case (Hide Column;
    /// Show Hidden Columns brings them back).
    pub hidden: std::collections::BTreeSet<String>,
    /// Columns described as a list or as flags show their values' names.
    pub value_names: bool,
    /// The table's description, where a file beside it gives one.
    described: Option<Described>,
    /// The table's description being looked for on a thread of its own
    /// ([`look_for_description`]: a hak's source folder walked, a folder's
    /// descriptions read), and where it is another than the one in hand,
    /// the folder it names scripts and tables from walked too (a
    /// repository may have a hundred thousand files): the look, and
    /// whether it has come to that walk (which the editor says: it can
    /// take a while).
    walking: Option<(Arc<std::sync::atomic::AtomicBool>, std::thread::JoinHandle<Looked>)>,
    /// When the description's file was last looked at (the time egui
    /// gives): it is read again when it changes.
    described_at: f64,
    /// Bumped at every change: what is worked out from the table is kept
    /// until then.
    revision: u64,
    /// Bumped as the table is read again where it was read from: another
    /// history, whose count of changes starts again (what is worked out
    /// for the changes since a count is not carried across it).
    history: u64,
    /// The rows it is narrowed to besides a Find's (the resource
    /// browser's Rows from: those a layer last changed).
    pub(crate) within: Option<Arc<Vec<usize>>>,
    /// The rows shown, for the table and the Find they were worked out
    /// for ([`shown_rows`]).
    #[allow(clippy::type_complexity)]
    shown: Option<(ShownFor, Arc<Vec<usize>>)>,
    /// The cells that are wrong, for the table's revision and the other
    /// open tables' changes (a table the description names may be one).
    #[allow(clippy::type_complexity)]
    faults: Option<((u64, u64, u64, u64, u64, TalkTables, Same<Description>), Arc<Vec<CellFault>>)>,
    /// Cached base-game table copy and its own fault coordinates so edits do
    /// not re-parse and re-validate the base game's copy on every change.
    #[allow(clippy::type_complexity)]
    base_faults: Option<(
        (String, u64, TalkTables),
        Option<Arc<(TwoDa, std::collections::HashSet<(usize, usize)>)>>,
    )>,
    /// The columns, kept for the revision and the count of changes they
    /// were taken at ([`TableDoc::columns_cached`]): not copied each frame.
    #[allow(clippy::type_complexity)]
    columns_cache: Option<((u64, u64), Arc<[String]>)>,
    /// How many rows Add Row adds.
    pub add_count: usize,
    /// The name of a column being added, or given to the column
    /// [`renaming`](Self::renaming), as typed.
    pub new_column: Option<String>,
    /// The column Rename Column… is for.
    pub renaming: Option<usize>,
    pub taking: Option<Taking>,
    /// Where a row is named, as last asked: the row, and each place.
    /// The module's blueprints, areas and scripts that name it.
    pub references: Option<(usize, Vec<Reference>, Vec<mg_module::rename::Usage>)>,
    /// What went wrong last (a cell refused, a save that failed).
    pub problem: Option<String>,
    /// New Line's lines, added to the module's talk table as the table is
    /// saved, where a cell still names them ([`add_pending_lines`]).
    pending: Pending,
    /// What is being typed in the Cell panel's Value (M4's U16), kept by
    /// its cell: set there as the field is left, never in the cell that
    /// took its number ([`panels::follow_typed`]).
    pub(crate) cell_typed: Option<panels::CellTyped>,
    /// Find References to Row being run, on a thread of its own: let go of
    /// as another row is chosen or the table changes ([`panels::Looking`]).
    looking: Option<panels::Looking>,
}

/// A table's description as read from its file (`name.2da.toml` beside
/// the table, or a family's: [`mg_module::table_desc`]), or Moonglow's of
/// the game's table of its name.
#[derive(Debug, Clone)]
struct Described {
    /// Its file; `None`: Moonglow's description of the game's table.
    path: Option<PathBuf>,
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
            edge_scroll: egui::Vec2::ZERO,
            moving: None,
            typing: None,
            filter: String::new(),
            go_to: String::new(),
            reveal: false,
            scroll: egui::Vec2::ZERO,
            compact: false,
            read: Arc::from(&[][..]),
            read_at: f64::NEG_INFINITY,
            seen: None,
            strref_text: true,
            hidden: Default::default(),
            value_names: true,
            described: None,
            walking: None,
            described_at: f64::NEG_INFINITY,
            revision: 0,
            history: 0,
            within: None,
            shown: None,
            faults: None,
            base_faults: None,
            columns_cache: None,
            add_count: 1,
            new_column: None,
            renaming: None,
            taking: None,
            references: None,
            problem: None,
            pending: Pending::default(),
            cell_typed: None,
            looking: None,
        }
    }

    /// The table as it is now, to hand to another table's editor (a
    /// description that names its rows): the editor's own, shared, not a
    /// copy (`Editing::shared`). It was copied whole every frame a cell
    /// naming its rows was selected, a million cells for a table of
    /// 20,000 rows, and then once a change.
    fn shared(&self) -> Arc<TwoDa> {
        self.edit.shared()
    }

    /// The table's columns, kept until it changes: by an edit, or read
    /// again (a new history, whose count of changes starts again at 0,
    /// and [`TableDoc::changed`] counts a revision).
    fn columns_cached(&mut self) -> Arc<[String]> {
        let key = (self.revision, self.edit.changes());
        if let Some((was, cols)) = &self.columns_cache
            && *was == key
        {
            return cols.clone();
        }
        let cols: Arc<[String]> = Arc::from(self.edit.table().columns());
        self.columns_cache = Some((key, cols.clone()));
        cols
    }

    pub fn name(&self) -> String {
        self.key.resref.to_string()
    }

    /// A table to look at, not to edit: the resource browser's view of a
    /// table the game reads (its own id, of those counted down from the
    /// highest).
    pub(crate) fn read_only(id: u32, key: ResKey, table: TwoDa) -> TableDoc {
        TableDoc::new(id, key, Home::Module, Editing::new(table))
    }

    /// What its columns' widths are kept by ([`column_widths`]): the
    /// module's table by its name; a hak's by the hak's file and its
    /// name (`cep_2da.hak/appearance`); a file's by its folder and its
    /// name.
    pub(crate) fn widths_key(&self, app: &Moonglow) -> String {
        let name = self.name();
        match &self.home {
            Home::Module => name,
            Home::Hak(hak) => {
                let hak = app.haks.iter().find(|d| d.id == *hak);
                let file = hak.and_then(|d| {
                    d.hak.path.as_ref()?.file_name().map(|f| f.to_string_lossy().to_lowercase())
                });
                file.map_or(name.clone(), |f| format!("{f}/{name}"))
            }
            Home::File(path) => match path.parent() {
                Some(dir) => format!("{}/{name}", dir.display()),
                None => name,
            },
        }
    }

    /// The table's description, where it has one that reads.
    pub fn description(&self) -> Option<Arc<Description>> {
        self.described.as_ref().and_then(|d| d.read.clone().ok())
    }

    /// Whether its description's folder is being walked (a UI test waits
    /// for it: [`crate::Moonglow::under_way`]).
    pub fn walking(&self) -> bool {
        self.walking.is_some()
    }

    /// What is being typed in the Cell panel's Value, where something is
    /// (for the tests: it is kept by its cell, `panels::CellTyped`).
    #[doc(hidden)]
    pub fn cell_typed_text(&self) -> Option<&str> {
        self.cell_typed.as_ref().map(|t| t.text.as_str())
    }

    /// Whether Find References to Row is under way for it (a UI test waits
    /// for it: [`crate::Moonglow::under_way`]).
    pub fn looking(&self) -> bool {
        self.looking.is_some()
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

    /// How far the grid is scrolled, across and down (points).
    pub fn scrolled(&self) -> egui::Vec2 {
        self.scroll
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

    /// Hides column `c` from view (its menu's Hide Column, its eye in the
    /// Outliner): its cells kept and saved, the keys passing over it; the
    /// selection let go of where it is in it.
    pub(crate) fn hide_column(&mut self, c: usize) {
        let Some(name) = self.edit.table().columns().get(c) else { return };
        self.hidden.insert(name.to_lowercase());
        if self.selected.is_some_and(|(_, sc)| sc == c) {
            self.selected = None;
            self.anchor = None;
        }
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
        // (A look of the table as it was: let go of.)
        if let Some(looking) = self.looking.take() {
            looking.stop();
        }
        // (A selection on a row or column that is gone, an Undo having
        // taken it back, goes to the last there is: the arrows, Enter and
        // Delete did nothing until a click, and Empty Row and Delete Row
        // stayed on for a row that wasn't there.)
        let table = self.edit.table();
        let (rows, columns) = (table.len(), table.columns().len());
        let within = |(row, column): (usize, usize)| {
            Some((row.min(rows.checked_sub(1)?), column.min(columns.checked_sub(1)?)))
        };
        if self.selected.is_some_and(|at| within(at) != Some(at)) {
            self.selected = self.selected.and_then(within);
            self.anchor = None;
        }
    }

    /// Whether it has changes not saved where it lives: a cell being typed
    /// too (closing the tab took no notice of one, and it was lost).
    pub(crate) fn has_unsaved(&self) -> bool {
        self.edit.is_dirty() || self.typing.is_some() || self.cell_typed.is_some()
    }

    /// Sets the selected cell from what was typed, in the grid's cell and
    /// in the Cell panel's Value (each in its own cell).
    fn commit(&mut self) {
        panels::commit_typed(self);
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
fn open_doc(app: &mut Moonglow, key: ResKey, home: Home, edit: Editing, data: &[u8]) -> u32 {
    if let Some(d) = app.tables.iter().find(|d| d.key == key && d.home == home) {
        let id = d.id;
        app.actions.push(Action::OpenTab(Tab::Table(id)));
        return id;
    }
    app.next_table += 1;
    let id = app.next_table;
    let mut doc = TableDoc::new(id, key, home, edit);
    doc.compact = is_compact(data);
    doc.read = Arc::from(data);
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
        Ok(t) => Some(open_doc(app, key, Home::Module, Editing::new(t), &data)),
        Err(e) => {
            app.log.error(e);
            None
        }
    }
}

/// Opens a table of a hak that is open in the hak editor.
pub fn open_in_hak(app: &mut Moonglow, hak: u32, key: ResKey) -> Option<u32> {
    let data = app.haks.iter().find(|d| d.id == hak)?.hak.data(key);
    match data.and_then(|d| Ok((parse(app, key, &d)?, d))) {
        Ok((t, d)) => Some(open_doc(app, key, Home::Hak(hak), Editing::new(t), &d)),
        Err(e) => {
            app.log.error(e);
            None
        }
    }
}

/// Opens a 2DA file anywhere.
pub fn open_path(app: &mut Moonglow, path: &std::path::Path) -> Option<u32> {
    let name = path.file_stem().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
    let read = || -> Result<(ResKey, TwoDa, Vec<u8>), String> {
        let resref = ResRef::from_str(&name).map_err(|e| format!("{}: {e}", path.display()))?;
        let key = ResKey::new(resref, ResType::TWODA);
        let data = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
        Ok((key, parse(app, key, &data)?, data))
    };
    match read() {
        Ok((key, t, data)) => {
            Some(open_doc(app, key, Home::File(path.to_path_buf()), Editing::new(t), &data))
        }
        Err(e) => {
            app.log.error(e);
            None
        }
    }
}

/// A 2DA file anywhere, asked for.
pub fn open_file(app: &mut Moonglow) {
    if let Some(path) = app.dialogs.open_file(crate::dialogs::FileKind::Table, None) {
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
    let footed = crate::widgets::dialog(ctx, "Open 2DA", &mut open)
        .default_size([460.0, 440.0])
        .collapsible(false)
        .show_foot(ctx, |ui| {
            let field =
                crate::widgets::SearchField::new(egui::Id::new("open-2da-find"), &mut w.filter)
                    .hint("Find by name")
                    .find_over()
                    .show(ui)
                    .response;
            crate::widgets::autofocus(ui, &field);
            let needle = mg_core::text::Needle::new(&w.filter);
            let listed: Vec<&Listed> =
                w.tables.iter().filter(|t| needle.finds(&t.name())).collect();
            let (count, first) = (listed.len(), listed.first().map(|t| (*t).clone()));
            let root = w.root.as_deref();
            mg_uikit::show_rows(
                egui::ScrollArea::vertical().auto_shrink(false),
                ui,
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
            let foot = crate::widgets::Foot::new()
                .note(format!("{count} tables"))
                .main("Open", first.is_some())
                .tip("Open the first table listed (Enter)")
                .button("Browse…", true)
                .tip("A .2da file anywhere else")
                .cancel();
            (first, foot)
        });
    if let Some(footed) = &footed {
        if footed.pressed.main() {
            chosen = footed.inner.clone().flatten();
        }
        browse = footed.pressed.clicked("Browse…");
        if footed.pressed.cancelled() {
            open = false;
        }
    }
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
            let id = open_doc(app, key, home, edit, &data);
            // (Laid out as the game's copy, not read from its home: nothing
            // there to follow until it is saved.)
            if let Some(doc) = app.tables.iter_mut().find(|d| d.id == id) {
                doc.read_at = f64::INFINITY;
            }
            Some(id)
        }
        Err(e) => {
            app.log.error(e);
            None
        }
    }
}

/// Edit a Copy In › a file: asks where.
pub fn edit_copy_in_file(app: &mut Moonglow, key: ResKey) -> Option<u32> {
    let suggested = PathBuf::from(key.to_string());
    let path = app.dialogs.save_file(crate::dialogs::FileKind::Table, Some(&suggested))?;
    let path = if path.extension().is_some() { path } else { path.with_extension("2da") };
    edit_copy(app, key, Home::File(path))
}

/// Adds a row with `cells` (by column name) at the end of the table open
/// as `id`, as one step to take back, and goes to it: its number.
pub fn add_row_of(
    app: &mut Moonglow,
    id: u32,
    label: &str,
    cells: &[(String, String)],
) -> Option<usize> {
    let doc = app.tables.iter_mut().find(|d| d.id == id)?;
    doc.commit();
    let row = doc.edit.add_row_of(label, cells);
    doc.changed();
    doc.go_to(row, 0);
    Some(row)
}

/// Adds New Line's lines that a cell of the table at `at` still names to
/// the module's talk table, and gives each such cell the number its line
/// took, where the talk table changed meanwhile. (Before the table is
/// saved; and before the module's Save saves the talk table, which saves
/// the tables after it.) An error leaves the table unsaved, saying why.
pub(crate) fn add_pending_lines(app: &mut Moonglow, at: usize) -> Result<(), String> {
    if app.tables[at].pending.is_empty() {
        return Ok(());
    }
    app.tables[at].commit();
    let named = |doc: &TableDoc, n: StrRef| -> Vec<(usize, usize)> {
        let (t, n) = (doc.edit.table(), n.0.to_string());
        (0..t.len())
            .flat_map(|r| (0..t.columns().len()).map(move |c| (r, c)))
            .filter(|&(r, c)| t.cell(r, c).is_some_and(|v| v.trim() == n))
            .collect()
    };
    let mut pending = std::mem::take(&mut app.tables[at].pending);
    let t = app.tables[at].edit.table();
    let numbers: BTreeSet<u32> = (0..t.len())
        .flat_map(|r| (0..t.columns().len()).filter_map(move |c| t.cell(r, c)))
        .filter_map(|v| v.trim().parse().ok())
        .collect();
    let (took, added) = pending.add(app, |n| numbers.contains(&n.0));
    app.tables[at].pending = pending;
    let doc = &mut app.tables[at];
    // (All at once: a line that took the number another was shown with
    // is not given that one's cells too.)
    let cells: Vec<(Vec<(usize, usize)>, StrRef)> = took
        .iter()
        .filter(|(shown, to)| shown != to)
        .map(|(s, to)| (named(doc, *s), *to))
        .collect();
    for (places, to) in cells {
        for (r, c) in places {
            doc.edit.set(r, c, Some(to.0.to_string())).map_err(|e| e.to_string())?;
        }
        doc.changed();
    }
    added.map_err(|e| format!("New Line: {e}"))
}

/// Saves a table where it lives. Whether it was saved.
pub fn save(app: &mut Moonglow, id: u32) -> bool {
    let cp = codepage(app);
    let Some(at) = app.tables.iter().position(|d| d.id == id) else { return false };
    if let Err(e) = add_pending_lines(app, at) {
        app.tables[at].problem = Some(format!("Not saved: {e}"));
        return false;
    }
    app.tables[at].commit();
    let (key, home) = (app.tables[at].key, app.tables[at].home.clone());
    let layout = match app.tables[at].compact {
        true => mg_2da::Layout::Compact,
        false => mg_2da::Layout::Aligned,
    };
    // As its file is laid out (its widths, row labels, tabs and line
    // ends), where it is saved as the file was (in line, or compact);
    // else anew in the layout chosen.
    let doc = &app.tables[at];
    let like = !doc.read.is_empty() && mg_2da::Layout::of(&doc.read) == layout;
    let written = match like {
        true => doc.edit.table().to_bytes_like(cp, &doc.read),
        false => doc.edit.table().to_bytes_as(cp, layout),
    };
    let bytes = match written {
        Ok(b) => b,
        Err(e) => {
            app.tables[at].problem = Some(format!("Not saved: {e}"));
            return false;
        }
    };
    let kept: Arc<[u8]> = Arc::from(&bytes[..]);
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
        // (Beside it and moved over it: written in place, a full disk left
        // half a table.)
        Home::File(path) => {
            mg_module::write_replacing(path, &bytes).map_err(|e| format!("{}: {e}", path.display()))
        }
    };
    match saved {
        Ok(()) => {
            let where_ = home_text(app, &home);
            let doc = &mut app.tables[at];
            doc.edit.mark_saved();
            doc.read = kept;
            doc.read_at = f64::NEG_INFINITY;
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
    let Some(path) = app.dialogs.save_file(crate::dialogs::FileKind::Table, Some(&suggested))
    else {
        return;
    };
    let path = if path.extension().is_some() { path } else { path.with_extension("2da") };
    let Some(doc) = app.tables.iter_mut().find(|d| d.id == id) else { return };
    let was = std::mem::replace(&mut doc.home, Home::File(path));
    // (Its home once it is written there: a write that failed left a
    // module's table pointing at a file that isn't there, and its next
    // Save went there again, not into the module.)
    if !save(app, id)
        && let Some(doc) = app.tables.iter_mut().find(|d| d.id == id)
    {
        doc.home = was;
    }
}

/// Where a table is, for its description to be looked for: what the
/// window's thread knows at once, the looking left to another.
#[derive(Debug, Clone, PartialEq)]
enum TableAt {
    /// Its file, or where its file would be (a file's own, the module's
    /// table's in a project or a module folder, beside the module's file
    /// for a module kept in one).
    File(PathBuf),
    /// A hak's table, `name`: in the folder the hak is built from (beside
    /// the table's file there), else beside the hak's file.
    Hak { folder: Option<PathBuf>, beside: Option<PathBuf>, name: String },
}

impl TableAt {
    fn of(app: &Moonglow, doc: &TableDoc) -> Option<TableAt> {
        match &doc.home {
            Home::File(path) => Some(TableAt::File(path.clone())),
            Home::Module => {
                let module = &app.ws.as_ref()?.module;
                mg_module::table_desc::table_in_module(module, &doc.key).map(TableAt::File)
            }
            Home::Hak(id) => {
                let hak = app.haks.iter().find(|d| d.id == *id)?;
                let name = doc.key.to_string();
                let beside = hak.hak.path.as_ref().map(|p| p.with_file_name(&name));
                Some(TableAt::Hak { folder: hak.folder.clone(), beside, name })
            }
        }
    }

    /// Where the table's description is: `name.2da.toml` beside the
    /// table, or a family's in that folder (`tables = ["loot_*"]`). It
    /// walks a hak's source folder, and lists the table's folder and reads
    /// every description in it for a family's: not for the window's
    /// thread.
    fn description(&self) -> Option<PathBuf> {
        use mg_module::table_desc::find_for;
        match self {
            TableAt::File(path) => find_for(path),
            TableAt::Hak { folder, beside, name } => {
                let in_folder = folder.as_deref().and_then(|folder| {
                    mg_module::hak_edit::folder_files(folder).into_iter().find(|p| {
                        p.file_name()
                            .is_some_and(|n| n.to_string_lossy().eq_ignore_ascii_case(name))
                    })
                });
                find_for(&in_folder.or_else(|| beside.clone())?)
            }
        }
    }
}

/// What a look for a table's description found.
#[derive(Debug)]
enum Looked {
    /// None of the builder's.
    None,
    /// The one in hand, its file as it was read.
    Same,
    /// Another, or the same changed: its file, the file's time, and the
    /// files of its repository or project.
    New(PathBuf, Option<std::time::SystemTime>, Beside),
}

/// Looks for the description of the table at `at` (`had`: the one in
/// hand and its file's time): for a thread of its own. Once a second the
/// window's thread walked a hak's source folder for it, or listed the
/// table's folder and read and parsed every description there: a hitch a
/// second, with a folder of 20,000 files, while the tab was in front.
fn look_for_description(
    at: Option<TableAt>,
    had: Option<(PathBuf, Option<std::time::SystemTime>)>,
    walking: &std::sync::atomic::AtomicBool,
    hold: &std::sync::atomic::AtomicBool,
) -> Looked {
    // (A test's hold: the look kept under way, as a large folder keeps it.)
    while hold.load(std::sync::atomic::Ordering::Relaxed) {
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    let Some(path) = at.and_then(|at| at.description()).filter(|p| p.is_file()) else {
        return Looked::None;
    };
    let modified = std::fs::metadata(&path).and_then(|m| m.modified()).ok();
    if had.is_some_and(|(p, m)| p == path && m == modified) {
        return Looked::Same;
    }
    // (The description's folder: a repository's or project's, which may be
    // large.)
    walking.store(true, std::sync::atomic::Ordering::Relaxed);
    let files = Beside::of(&Beside::root_of(&path.with_extension("")));
    Looked::New(path, modified, files)
}

/// What the table was read from, as it is now: the module's resource (an
/// Undo of the module may take its save back), the hak's (replaced in the
/// hak editor) or the file's (changed outside). `None`: it is no more.
fn source_now(app: &Moonglow, doc: &TableDoc) -> Option<Vec<u8>> {
    match &doc.home {
        Home::Module => Some(app.ws.as_ref()?.module.get(&doc.key)?.to_vec()),
        Home::Hak(hak) => app.haks.iter().find(|d| d.id == *hak)?.hak.data(doc.key).ok(),
        Home::File(path) => std::fs::read(path).ok(),
    }
}

/// What tells whether a table's source changed, without reading it.
#[derive(Debug, Clone, PartialEq)]
enum Seen {
    /// A file's time and size.
    File(std::time::SystemTime, u64),
    /// A hak's count of changes in its editor.
    Hak(u64),
}

/// What tells, now, whether the table's source changed since it was
/// read; none where only its bytes tell (the module's, a file that can't
/// be looked at).
fn seen_now(app: &Moonglow, doc: &TableDoc) -> Option<Seen> {
    match &doc.home {
        Home::Module => None,
        Home::Hak(hak) => Some(Seen::Hak(app.haks.iter().find(|d| d.id == *hak)?.revision())),
        Home::File(path) => {
            let about = std::fs::metadata(path).ok()?;
            Some(Seen::File(about.modified().ok()?, about.len()))
        }
    }
}

/// Reads the table again when what it was read from changes under it
/// (looked at once a second): with no change of the builder's not saved,
/// the editor shows the table as it is now; with one, it says so, and a
/// save writes over it. A copy not yet saved anywhere (Edit a Copy) has
/// nothing to follow.
fn read_again(app: &mut Moonglow, ctx: &egui::Context, at: usize) {
    let now = ctx.input(|i| i.time);
    if now - app.tables[at].read_at < 1.0 || app.tables[at].read.is_empty() {
        return;
    }
    app.tables[at].read_at = now;
    // (Looked at before it is read: a file by its time and size, a hak by
    // its changes, the module's bytes where they are. The whole of it was
    // read and copied every second the tab was in front.)
    let seen = seen_now(app, &app.tables[at]);
    if seen.is_some() && seen == app.tables[at].seen {
        return;
    }
    let same = |app: &Moonglow| {
        let doc = &app.tables[at];
        let module = app.ws.as_ref().map(|ws| &ws.module);
        doc.home == Home::Module && module.and_then(|m| m.get(&doc.key)) == Some(&doc.read[..])
    };
    if same(app) {
        return;
    }
    let Some(data) = source_now(app, &app.tables[at]) else { return };
    app.tables[at].seen = seen;
    if data[..] == app.tables[at].read[..] {
        return;
    }
    let key = app.tables[at].key;
    let parsed = parse(app, key, &data);
    let doc = &mut app.tables[at];
    doc.read = Arc::from(&data[..]);
    match parsed {
        Ok(t) if !doc.has_unsaved() => {
            doc.edit = Editing::new(t);
            doc.history += 1;
            doc.compact = is_compact(&data);
            doc.changed();
            doc.problem = None;
            app.log.info(format!("{key} was changed where it was read from: read again"));
        }
        Ok(_) => {
            doc.problem = Some(
                "Changed where it was read from since it was opened: Save writes this table \
                 over that one"
                    .into(),
            );
        }
        Err(e) => doc.problem = Some(format!("Changed where it was read from, unreadable: {e}")),
    }
}

/// Reads the table's description where there is one, and again when its
/// file changes (looked for once a second); with none of the builder's,
/// Moonglow's of the game's table of its name, if it has one. The look,
/// and the walk of the folder the description names scripts and tables
/// from, are on a thread of their own ([`look_for_description`]), the
/// window going on meanwhile.
fn describe(app: &mut Moonglow, ctx: &egui::Context, at: usize) {
    if take_look(app, ctx, at) {
        return;
    }
    let now = ctx.input(|i| i.time);
    if now - app.tables[at].described_at < 1.0 {
        return;
    }
    app.tables[at].described_at = now;
    let table = TableAt::of(app, &app.tables[at]);
    let doc = &mut app.tables[at];
    let had = (doc.described.as_ref()).and_then(|d| Some((d.path.clone()?, d.modified)));
    let walking = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let hold = app.hold_description_looks.clone();
    let doc = &mut app.tables[at];
    let look = {
        let walking = walking.clone();
        std::thread::spawn(move || look_for_description(table, had, &walking, &hold))
    };
    doc.walking = Some((walking, look));
    app.frame_after(ctx, std::time::Duration::from_millis(30));
}

/// Each frame, for every table: a look for its description that has
/// ended is taken in, whether its tab is drawn or not. (Taken in only as
/// its tab was drawn, a look started before the tab went behind another
/// was never taken in, and the window's work never ended: `under_way`
/// said "a table's description's folder being read" for good.)
pub(crate) fn follow_looks(app: &mut Moonglow, ctx: &egui::Context) {
    for at in 0..app.tables.len() {
        take_look(app, ctx, at);
        // (And Find References to Row, M4's U16: `panels::follow_looking`.)
        panels::follow_looking(&mut app.tables[at], ctx);
    }
}

/// Takes in table `at`'s look for its description where it has ended;
/// where one is under way, asks for a frame soon ([`Moonglow::frame_after`]:
/// a test's run of frames doesn't wait for the look's thread, which on
/// Windows CI outlasted `Harness::run`'s four steps). Whether one was
/// under way (ended now or not).
fn take_look(app: &mut Moonglow, ctx: &egui::Context, at: usize) -> bool {
    use mg_module::table_desc::game_description;
    // (A look under way is looked at every frame.)
    if let Some((_, look)) = &app.tables[at].walking {
        if !look.is_finished() {
            app.frame_after(ctx, std::time::Duration::from_millis(30));
            return true;
        }
        let (_, look) = app.tables[at].walking.take().expect("looking");
        match look.join().unwrap_or(Looked::Same) {
            Looked::Same => {}
            Looked::New(path, modified, files) => {
                read_description(app, at, Some(path), modified, Arc::new(files));
            }
            Looked::None => {
                // Moonglow's, of the game's table of the name.
                let had = app.tables[at].described.as_ref().map(|d| d.path.is_none());
                match game_description(&app.tables[at].name()) {
                    Some(_) if had == Some(true) => {}
                    Some(_) => read_description(app, at, None, None, Arc::default()),
                    None if had.is_some() => {
                        app.tables[at].described = None;
                        app.tables[at].changed();
                    }
                    None => {}
                }
            }
        }
        return true;
    }
    false
}

/// Reads a description (`path`'s file; `None`: Moonglow's of the game's
/// table), its lists from the scripts of `files` or the game's.
fn read_description(
    app: &mut Moonglow,
    at: usize,
    path: Option<PathBuf>,
    modified: Option<std::time::SystemTime>,
    files: Arc<Beside>,
) {
    let game = app.game.clone();
    let script = |name: &str| {
        let key = ResKey::from_filename(&format!("{name}.nss"))?;
        files.script(name).or_else(|| Some(game.as_deref()?.resman.get(&key).ok()?.into_owned()))
    };
    let text = match &path {
        Some(path) => std::fs::read_to_string(path).map_err(|e| e.to_string()),
        None => mg_module::table_desc::game_description(&app.tables[at].name())
            .map(str::to_string)
            .ok_or_else(|| "no description".to_string()),
    };
    let read = text.and_then(|text| Description::read(&text, &script)).map(Arc::new);
    let doc = &mut app.tables[at];
    doc.described = Some(Described { path, modified, read, files, tables: Default::default() });
    doc.changed();
}

/// A table a description names, of its project: as it is open in the
/// editor (a file's, or the module's from that file), else as its file is
/// read (once, until the description is read again).
fn project_table(app: &Moonglow, d: &Described, name: &str) -> Option<Arc<TwoDa>> {
    let file = d.files.table_file(name)?;
    // (The same file however its path is written: `a/../b`.)
    let same = |p: &std::path::Path| {
        p == file || p.canonicalize().is_ok_and(|p| file.canonicalize().is_ok_and(|f| f == p))
    };
    let named = |t: &&TableDoc| t.name().eq_ignore_ascii_case(name);
    let open = app.tables.iter().filter(named).find(|t| match &t.home {
        Home::File(p) => same(p),
        Home::Module => app.ws.as_ref().and_then(|ws| ws.module.file_of(&t.key)).is_some_and(same),
        Home::Hak(_) => false,
    });
    if let Some(t) = open {
        return Some(t.shared());
    }
    let mut read = d.tables.borrow_mut();
    let cp = codepage(app);
    read.entry(name.to_ascii_lowercase())
        .or_insert_with(|| d.files.table(name, cp).map(Arc::new))
        .clone()
}

/// What the table's columns hold, where the checks know.
pub(crate) fn kinds(app: &Moonglow, name: &str) -> Vec<(&'static str, ColumnKind)> {
    let read = |n: &str| app.game.as_deref().and_then(|g| g.table(n).ok());
    mg_module::doctor::column_kinds_as_named(name, &read)
}

/// How many strings the talk tables have, for the checks.
fn talk_tables(app: &Moonglow) -> TalkTables {
    let base = app.game.as_deref().map_or(0, |g| g.tlk().entries.len());
    let custom = app.game.as_deref().and_then(|g| g.custom_tlk().map(|t| t.entries.len()));
    TalkTables { base, custom }
}

type BaseFaults = (TwoDa, std::collections::HashSet<(usize, usize)>);

/// Returns the game's base copy of `name` and its own fault coordinates, cached
/// on `doc.base_faults` per `(name, load_order, talk_tables)`.
fn base_faults_for(
    doc: &mut TableDoc,
    game: Option<&mg_rules::GameData>,
    name: &str,
    kinds: &[(&'static str, ColumnKind)],
    load_order: u64,
    tlk: TalkTables,
) -> Option<Arc<BaseFaults>> {
    let key = (name.to_ascii_lowercase(), load_order, tlk);
    if let Some((was, cached)) = &doc.base_faults
        && *was == key
    {
        return cached.clone();
    }
    let cached = game.and_then(|g| {
        let base = mg_module::doctor::game_copy(&g.resman, name)?;
        let read = std::cell::RefCell::new(std::collections::HashMap::new());
        let game_read = |n: &str| -> Option<Arc<TwoDa>> {
            let mut read = read.borrow_mut();
            let t = read
                .entry(n.to_ascii_lowercase())
                .or_insert_with(|| mg_module::doctor::game_copy(&g.resman, n).map(Arc::new));
            t.clone()
        };
        let theirs = mg_module::doctor::table_faults(name, &base, kinds, Some(&game_read), tlk);
        let set = cells_of(&theirs);
        Some(Arc::new((base, set)))
    });
    doc.base_faults = Some((key, cached.clone()));
    cached
}

fn filter_base_faults(
    found: &mut Vec<CellFault>,
    table: &TwoDa,
    base: &TwoDa,
    theirs: &std::collections::HashSet<(usize, usize)>,
) {
    found.retain(|f| {
        let column = &table.columns()[f.column];
        let kept = base.column(column).is_some_and(|c| {
            base.cell(f.row, c) == table.cell(f.row, f.column) && theirs.contains(&(f.row, c))
        });
        !kept
    });
}

struct DescFaultsCtx<'a> {
    app: &'a Moonglow,
    doc: &'a TableDoc,
    described: &'a Described,
    description: &'a Description,
    name: &'a str,
    tlk: TalkTables,
    base: Option<&'a TwoDa>,
}

fn description_faults_append(
    found: &mut Vec<CellFault>,
    ctx: &DescFaultsCtx<'_>,
    rows: Option<&[usize]>,
) {
    let table = ctx.doc.edit.table();
    let read = |n: &str| ctx.app.game.as_deref().and_then(|g| g.table(n).ok());
    let rows_of = |n: &str| match n.eq_ignore_ascii_case(ctx.name) {
        true => Some(table.len()),
        false => project_table(ctx.app, ctx.described, n)
            .map(|t| t.len())
            .or_else(|| read(n).map(|t| t.len())),
    };
    let desc_faults = match rows {
        Some(rows) => ctx.description.faults_rows(table, rows, &rows_of, ctx.tlk),
        None => ctx.description.faults(table, &rows_of, ctx.tlk),
    };
    let mut said = cells_of(found);
    for f in desc_faults {
        let column = &table.columns()[f.column];
        let the_game_s =
            ctx.base.is_some_and(|b| b.get(f.row, column) == table.cell(f.row, f.column));
        if !the_game_s && said.insert((f.row, f.column)) {
            found.push(f);
        }
    }
}

/// The cells that are wrong, kept until the table changes.
fn faults(app: &mut Moonglow, at: usize) -> Arc<Vec<CellFault>> {
    // (Another open table's change may be one a description names.)
    let others: u64 = (app.tables.iter().enumerate())
        .filter(|(i, _)| *i != at)
        .map(|(_, t)| t.revision.wrapping_add(t.edit.changes()))
        .fold(0, u64::wrapping_add);
    let doc = &app.tables[at];
    let changes = doc.edit.changes();
    let described = Same(doc.description());
    let tlk = talk_tables(app);
    let key = (doc.revision, doc.history, changes, others, app.load_order, tlk, described);
    if let Some((mark, found)) = &doc.faults
        && *mark == key
    {
        return found.clone();
    }
    let name = doc.name();
    let kinds = kinds(app, &name);

    // Incremental path: only small cell edits occurred since previous faults,
    // external factors match, and no self-referencing row flipped emptiness.
    let incremental = doc.faults.as_ref().and_then(|(mark, prev)| {
        let (_, prev_history, prev_changes, prev_others, prev_lo, prev_tlk, prev_desc) = mark;
        // (The same history: a table read again starts another, whose
        // changes count from 0 again.)
        if *prev_changes < changes
            && (*prev_history, *prev_others, *prev_lo, *prev_tlk, prev_desc)
                == (doc.history, others, app.load_order, tlk, &key.6)
        {
            let (touched, emptiness_changed) = doc.edit.cell_rows_since(*prev_changes)?;
            let self_ref = kinds.iter().any(|(_, k)| {
                matches!(k, ColumnKind::Row(target) if target.eq_ignore_ascii_case(&name))
                    || matches!(k, ColumnKind::RowBy { .. })
            });
            (!self_ref || !emptiness_changed).then(|| (touched, prev.clone()))
        } else {
            None
        }
    });

    if let Some((touched, prev_found)) = incremental {
        let mut touched_faults = {
            let game = app.game.as_deref();
            let read = |n: &str| game.and_then(|g| g.table(n).ok());
            let tables: Option<&mg_module::doctor::TableOf> = game.map(|_| &read as _);
            mg_module::doctor::table_faults_rows(
                &name,
                doc.edit.table(),
                &touched,
                &kinds,
                tables,
                tlk,
            )
        };
        // (The game's own faults taken out of what the rows edited have,
        // as the whole table's are: whenever they have any, though the
        // table had none before.)
        let base_pair = if app.game.is_some()
            && (!touched_faults.is_empty()
                || doc.described.as_ref().is_some_and(|d| d.path.is_none() && d.read.is_ok()))
        {
            let game = app.game.clone();
            base_faults_for(
                &mut app.tables[at],
                game.as_deref(),
                &name,
                &kinds,
                app.load_order,
                tlk,
            )
        } else {
            None
        };
        let doc = &app.tables[at];
        if !touched_faults.is_empty()
            && let Some(pair) = &base_pair
        {
            filter_base_faults(&mut touched_faults, doc.edit.table(), &pair.0, &pair.1);
        }
        if let Some(d) = &doc.described
            && let Ok(description) = &d.read
        {
            let base = if d.path.is_none() { base_pair.as_ref().map(|p| &p.0) } else { None };
            let ctx = DescFaultsCtx { app, doc, described: d, description, name: &name, tlk, base };
            description_faults_append(&mut touched_faults, &ctx, Some(&touched));
        }
        touched_faults.sort_by_key(|f| (f.row, f.column));
        let unchanged = touched.iter().all(|&r| {
            let new_slice = row_faults(&touched_faults, r);
            let old_slice = row_faults(&prev_found, r);
            new_slice == old_slice
        });
        let found = if unchanged {
            prev_found
        } else {
            let mut next = Vec::with_capacity(prev_found.len());
            let mut pi = 0;
            let mut ni = 0;
            for &r in &touched {
                while pi < prev_found.len() && prev_found[pi].row < r {
                    next.push(prev_found[pi].clone());
                    pi += 1;
                }
                while pi < prev_found.len() && prev_found[pi].row == r {
                    pi += 1;
                }
                while ni < touched_faults.len() && touched_faults[ni].row < r {
                    ni += 1;
                }
                while ni < touched_faults.len() && touched_faults[ni].row == r {
                    next.push(touched_faults[ni].clone());
                    ni += 1;
                }
            }
            next.extend_from_slice(&prev_found[pi..]);
            Arc::new(next)
        };
        app.tables[at].faults = Some((key, found.clone()));
        return found;
    }

    let game = app.game.as_deref();
    let read = |n: &str| game.and_then(|g| g.table(n).ok());
    // (With no game, no table is known to be there or not.)
    let tables: Option<&mg_module::doctor::TableOf> = game.map(|_| &read as _);
    let mut found = mg_module::doctor::table_faults(&name, doc.edit.table(), &kinds, tables, tlk);
    let needs_base = (!found.is_empty()
        || doc.described.as_ref().is_some_and(|d| d.path.is_none() && d.read.is_ok()))
        && app.game.is_some();
    let base_pair = if needs_base {
        let game = app.game.clone();
        base_faults_for(&mut app.tables[at], game.as_deref(), &name, &kinds, app.load_order, tlk)
    } else {
        None
    };
    if !found.is_empty()
        && let Some(pair) = &base_pair
    {
        filter_base_faults(&mut found, app.tables[at].edit.table(), &pair.0, &pair.1);
    }
    // And by the table's own description: a table it names is this one
    // as it is now, one of its repository's, or the game's.
    let doc = &app.tables[at];
    if let Some(d) = &doc.described
        && let Ok(description) = &d.read
    {
        let base = if d.path.is_none() { base_pair.as_ref().map(|p| &p.0) } else { None };
        let ctx = DescFaultsCtx { app, doc, described: d, description, name: &name, tlk, base };
        description_faults_append(&mut found, &ctx, None);
    }
    // (By row, then column, always: the grid takes each row's with
    // `row_faults`.)
    found.sort_by_key(|f| (f.row, f.column));
    let found = Arc::new(found);
    let doc = &mut app.tables[at];
    doc.faults = Some((key, found.clone()));
    found
}

/// The cells faults are in, to look one up.
fn cells_of(faults: &[CellFault]) -> std::collections::HashSet<(usize, usize)> {
    faults.iter().map(|f| (f.row, f.column)).collect()
}

/// The faults of row `row`, of faults in the order [`faults`] gives them
/// (by row, then column): found by halving, not by looking through them.
fn row_faults(faults: &[CellFault], row: usize) -> &[CellFault] {
    let from = faults.partition_point(|f| f.row < row);
    let to = from + faults[from..].partition_point(|f| f.row == row);
    &faults[from..to]
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
    references_until(app.game.as_deref(), name, own, row, &|| false).unwrap_or_default()
}

/// [`references`] in the game's tables `game` reads, given up where
/// `stop` says so (asked between tables): `None` then. Run on a thread
/// of its own by the References panel.
pub(crate) fn references_until(
    game: Option<&mg_rules::GameData>,
    name: &str,
    own: &TwoDa,
    row: usize,
    stop: &dyn Fn() -> bool,
) -> Option<Vec<Reference>> {
    use mg_module::doctor::{column_kinds, row_named};
    let Some(game) = game else { return Some(Vec::new()) };
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
        if stop() {
            return None;
        }
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
    Some(out)
}

/// Where the module names row `row` of the table `name`: its blueprints',
/// areas' and placed objects' fields, and its scripts
/// ([`mg_module::row_uses`]).
pub fn module_row_uses(app: &Moonglow, name: &str, row: usize) -> Vec<mg_module::rename::Usage> {
    let Some(ws) = app.ws.as_ref() else { return Vec::new() };
    mg_module::row_uses::row_uses(&ws.module, name, row, app.game.as_deref(), codepage(app))
}

const CELL: f32 = 116.0;
pub(crate) const NUMBER: f32 = 54.0;
/// The narrowest and the widest a column is made.
const NARROWEST: f32 = 24.0;
const WIDEST: f32 = 1600.0;
/// How many tables' column widths are kept.
const WIDTHS_KEPT: usize = 200;

/// Each column's width: the one given it (by dragging the edge of its
/// name) in this table, else in a table of its name, else `default`.
/// `table` is a table's name (`appearance`), or where it is and its name
/// (`cep_2da.hak/appearance`, [`TableDoc::widths_key`]): two haks'
/// `custom.2da` keep widths of their own, and take those of the name
/// until given one.
pub(crate) fn column_widths(
    settings: &crate::settings::Settings,
    table: &str,
    columns: &[String],
    default: f32,
) -> Vec<f32> {
    let kept = |t: &str| {
        let kept = settings.table_widths.iter().find(|(k, _)| k.eq_ignore_ascii_case(t));
        kept.map_or(&[][..], |(_, w)| w.as_slice())
    };
    let name = table.rsplit('/').next().unwrap_or(table);
    let (own, named) = (kept(table), kept(name));
    columns
        .iter()
        .map(|c| {
            let given = |list: &[(String, u32)]| {
                list.iter().find(|(k, _)| k.eq_ignore_ascii_case(c)).map(|(_, w)| *w as f32)
            };
            given(own).or_else(|| given(named)).unwrap_or(default)
        })
        .collect()
}

/// Puts a column back to its first width (`None`: every column of the
/// table), in this table and in the tables of its name.
pub(crate) fn reset_column_width(
    settings: &mut crate::settings::Settings,
    table: &str,
    column: Option<&str>,
) {
    let name = table.rsplit('/').next().unwrap_or(table).to_string();
    for t in [table.to_string(), name] {
        if let Some((_, widths)) =
            settings.table_widths.iter_mut().find(|(k, _)| k.eq_ignore_ascii_case(&t))
        {
            widths.retain(|(c, _)| column.is_some_and(|col| !c.eq_ignore_ascii_case(col)));
        }
    }
    settings.table_widths.retain(|(_, w)| !w.is_empty());
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

/// What a column's menu asks of how it is shown.
enum ColumnView {
    /// Hidden from view (its place among the columns).
    Hide(usize),
    /// Its width put back to the first (`None`: every column's).
    Reset(Option<usize>),
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
    (talk, game): (Option<&mg_module::talk::Table>, Option<&mg_rules::GameData>),
    pending: &Pending,
) -> (String, Option<String>, bool) {
    let text = raw
        .filter(|_| strref)
        .and_then(|v| v.parse::<u32>().ok())
        .and_then(|n| pending_or_talk_string(pending, talk, game, n));
    // (A value by its name, where the description gives it one.)
    let name = raw.zip(named).and_then(|(v, k)| k.named(v));
    let was_named = name.is_some();
    let text = text.or(name);
    let label = text.clone().unwrap_or_else(|| raw.unwrap_or("****").into());
    (label, text, was_named)
}

/// The tab of a table (M4's U16, the Table board): its header (the kit's
/// [`mg_uikit::header::EditorHeader`], [`TABLE_HEADER`]) over its grid, the
/// whole of the rest. Its cell's value and choosers are the Right Dock's
/// Cell panel and its row's users its References panel, its columns the
/// Outliner's (`panels`), drawn before it: in its window's right and left
/// panes where it floats. Its counts and keys are the Status Bar's.
pub(crate) fn ui(app: &mut Moonglow, ui: &mut Ui, id: u32) {
    let Some(at) = app.tables.iter().position(|d| d.id == id) else {
        ui.label("This table is closed.");
        return;
    };
    read_again(app, ui.ctx(), at);
    describe(app, ui.ctx(), at);
    let faults = faults(app, at);
    let name = app.tables[at].name();
    let kinds = kinds(app, &name);
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
    let asked = header(app, ui, at, has_copies, faults.len());
    // OK and Cancel at the header's end, as the other editors have them: a
    // table is saved when asked, so OK saves it and Cancel leaves it as
    // last saved (asked about first, as its × asks: no Undo brings a
    // table's changes back), its panels' changes with it. (A cell being
    // typed, in the grid or the Cell panel, is set first.)
    let commit = |app: &mut Moonglow| {
        let doc = &mut app.tables[at];
        doc.commit();
        panels::commit_typed(doc);
    };
    if let Some(pressed) = asked.pressed
        && crate::side_doc::pressed_ok_cancel(app, &Tab::Table(id), false, pressed, commit)
    {
        return;
    }
    // (Why it could not be saved, at the foot.)
    crate::side_doc::failed_foot(app, ui, &Tab::Table(id));
    notes(app, ui, at);
    // (A floating table's Outliner folded to a strip at the grid's left,
    // where its window has not the room for its pane: `shell::window_panes`.)
    let strip = app.outliner.strips_now.contains(&crate::outliner::Owner::Table(id));
    if strip {
        crate::outliner::strip(app, ui, crate::outliner::Owner::Table(id));
    }
    keys(app, ui, at);
    grid(app, ui, at, &kinds, &faults);
    if strip {
        crate::outliner::over(app, ui, crate::outliner::Owner::Table(id));
    }
    taking_window(app, ui.ctx(), at, &others);
    column_window(app, ui.ctx(), at);
    if asked.save {
        save(app, id);
    } else if asked.save_as {
        save_as_file(app, id);
    }
    focus_asked(app, ui.ctx(), id, &Tab::Table(id));
}

/// The name of a table's header (`mg_uikit::header`; a screen reader's and
/// the tests' group), the Table board's.
pub const TABLE_HEADER: &str = "2DA table header";

/// What a table's header asked for that [`ui`] does.
#[derive(Default)]
struct Asked {
    save: bool,
    save_as: bool,
    /// OK or Cancel, at the header's end.
    pressed: Option<crate::blueprint::Pressed>,
}

/// The tip of the header's "N cells are wrong".
const WRONG_TIP: &str = "Go to the next of what Verify Module would say of this table";

/// The words of "N cells are wrong".
fn wrong_words(wrong: usize) -> String {
    match wrong {
        1 => "1 cell is wrong".to_string(),
        n => format!("{n} cells are wrong"),
    }
}

/// Add Row's words: "Add Row", or "Add 3 Rows".
fn add_words(count: usize) -> String {
    if count == 1 { "Add Row".to_string() } else { format!("Add {count} Rows") }
}

/// Show Hidden Columns' words, with how many are hidden.
pub(crate) fn show_hidden_words(hidden: usize) -> String {
    match hidden {
        0 => "Show Hidden Columns".to_string(),
        n => format!("Show Hidden Columns ({n})"),
    }
}

/// The table header (M4's U16, the Table board's): Save, the first
/// command (into More last); a line; Add Row with its count beside it
/// (joined), Empty Row, Delete Row and Add Column… as icons; a line; Find,
/// Go to and "N cells are wrong" (where some are); at the right Values as
/// names (where a description names values) and Strings as text, joined
/// as "How cells show"; More, with its words, always holding Save As
/// File…, Take Rows…, Compact, Show Hidden Columns and Reset All Widths;
/// then OK and Cancel in the end slot (ruling 10 of section 15.2), which
/// never yields. What has no room loses its words, then goes into More.
/// Its own Undo and Redo are gone: the Top Bar's and the keys reach the
/// table (`Moonglow::local_undo`). It does what changes the table itself;
/// Save, Save As File… and OK or Cancel it asks [`ui`] to do.
fn header(app: &mut Moonglow, ui: &mut Ui, at: usize, has_copies: bool, wrong: usize) -> Asked {
    use mg_uikit::Icon;
    use mg_uikit::header::{EditorHeader, Item, Joined};
    use std::cell::{Cell, RefCell};
    let ctx = ui.ctx().clone();
    // (Find's keys go to its field while the pointer is over the table,
    // as they did over its old rows: the header's part is cut to its row.)
    let over = ui.rect_contains_pointer(ui.max_rect());
    let doc = &mut app.tables[at];
    let id = doc.id;
    let dirty = doc.edit.is_dirty();
    let row = doc.selected.map(|s| s.0);
    let hidden = doc.hidden.len();
    let hidden_names: Vec<String> = (doc.edit.table().columns().iter())
        .filter(|c| doc.hidden.contains(&c.to_lowercase()))
        .cloned()
        .collect();
    let described = doc.description().is_some();
    let mut value_names = doc.value_names;
    let mut strref_text = doc.strref_text;
    let mut compact = doc.compact;
    // (The parts of the caller's: Add Row and its count, Find, Go to and
    // the wrong cells, each drawn on the row or in More; what they ask is
    // read after.)
    let count = Cell::new(doc.add_count.max(1));
    let add = Cell::new(false);
    let to_wrong = Cell::new(false);
    let filter = RefCell::new(std::mem::take(&mut doc.filter));
    let go_to = RefCell::new(std::mem::take(&mut doc.go_to));
    let gone_to = Cell::new(None::<usize>);
    let rows = doc.edit.table().len();
    let measure = |text: &str| {
        let font = egui::FontId::proportional(mg_uikit::metrics::BODY_SIZE);
        ui.painter().layout_no_wrap(text.to_string(), font, egui::Color32::WHITE).size().x
    };
    // Add Row with its count, joined as the board draws them.
    const COUNT: f32 = 40.0;
    let add_button = |ui: &mut Ui, icon_only: bool| {
        let words = add_words(count.get());
        // (Named Add Row whatever its count says: "Add 5 Rows".)
        let b = mg_uikit::KitButton::new(Some(Icon::Row), words)
            .name(ADD_ROW)
            .icon_only(icon_only)
            .tip("Add empty rows at the end, as many as the number beside says");
        ui.add(b)
    };
    let add_width = |icon_only: bool| {
        let b =
            mg_uikit::KitButton::new(Some(Icon::Row), add_words(count.get())).icon_only(icon_only);
        b.width(ui) + 2.0 + COUNT
    };
    let (add_full, add_icon) = (add_width(false), add_width(true));
    let add_row = |ui: &mut Ui, icon_only: bool| {
        ui.spacing_mut().item_spacing.x = 2.0;
        if add_button(ui, icon_only).clicked() {
            add.set(true);
        }
        let mut n = count.get();
        let field = egui::DragValue::new(&mut n).range(1..=10_000).speed(0.2);
        ui.add_sized([COUNT, mg_uikit::metrics::CONTROL_HEIGHT], field)
            .on_hover_text("How many rows Add Row adds")
            .widget_info(|| {
                egui::WidgetInfo::labeled(egui::WidgetType::DragValue, true, ADD_COUNT)
            });
        count.set(n.max(1));
    };
    // Find: text in any cell, its rows alone shown.
    const FIND: f32 = 170.0;
    let find = |ui: &mut Ui| {
        let field = egui::Id::new(("table-filter", id));
        let mut text = filter.borrow_mut();
        crate::widgets::SearchField::new(field, &mut text)
            .name("Find")
            .hint("text in any cell")
            .width(FIND)
            .find_when(over)
            .show(ui);
    };
    // Go to: a row by its number, Enter going there.
    const GO_TO: f32 = 84.0;
    let go = |ui: &mut Ui| {
        let mut text = go_to.borrow_mut();
        let field = mg_uikit::text_field(&mut *text).desired_width(GO_TO).hint_text("Go to row");
        let r = ui.add(field).on_hover_text("Go to a row by its number: Enter");
        r.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::TextEdit, true, "Go to"));
        // (Its Enter taken: the grid, given the keys, began typing in the
        // row's cell with it, as it did before M4's U16.)
        if r.lost_focus()
            && ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter))
            && let Ok(row) = text.trim().parse::<usize>()
            && row < rows
        {
            gone_to.set(Some(row));
        }
    };
    // "N cells are wrong": a click goes to the next.
    let wrong_text = wrong_words(wrong);
    let wrong_width = 4.0 + mg_uikit::metrics::ICON + 6.0 + measure(&wrong_text) + 8.0;
    let warn = |ui: &mut Ui| {
        if warn_button(ui, &wrong_text).clicked() {
            to_wrong.set(true);
        }
    };
    let mut how = Vec::new();
    if described {
        how.push(Joined::toggle(Icon::Tag, VALUES_AS_NAMES, &mut value_names).tip(
            "Values as names: show the names of list and flag values. The value is what is saved.",
        ));
    }
    how.push(Joined::toggle(Icon::TalkTable, STRINGS_AS_TEXT, &mut strref_text).tip(
        "Strings as text: show the string in talk-table columns. The number is what is edited.",
    ));
    let hidden_tip = match hidden_names.is_empty() {
        true => {
            "No column is hidden: Hide Column on a column's menu or its eye hides one".to_string()
        }
        false => format!("Hidden from view: {}", hidden_names.join(", ")),
    };
    let mut h = EditorHeader::new(TABLE_HEADER)
        .more_words(true)
        .left(Item::button(Icon::Save, "Save").primary().enabled(dirty).tip("Save the table"))
        .left(Item::separator())
        .left(
            Item::custom(ADD_ROW, add_full, |ui| add_row(ui, false))
                .compact(add_icon, |ui| add_row(ui, true))
                .more(add_full, |ui| add_row(ui, false)),
        )
        .left(
            Item::icon(Icon::Eraser, EMPTY_ROW).enabled(row.is_some()).tip(
                "Empty Row: empty every cell of the selected row. Later rows keep their numbers.",
            ),
        )
        .left(
            Item::icon(Icon::Delete, DELETE_ROW)
                .enabled(row.is_some())
                .tip("Delete Row: later rows renumber, and what names them doesn't follow"),
        )
        .left(
            Item::icon(Icon::Column, ADD_COLUMN).tip("Add a column at the end, named as you type"),
        )
        .left(Item::separator())
        .left(
            Item::custom("Find", FIND, find)
                .more(FIND, |ui| {
                    let field = egui::Id::new(("table-filter", id));
                    let mut text = filter.borrow_mut();
                    crate::widgets::SearchField::new(field, &mut text)
                        .name("Find")
                        .hint("text in any cell")
                        .width(FIND)
                        .find_when(over)
                        .show(ui);
                })
                .primary(),
        )
        .left(
            Item::custom("Go to", GO_TO, go)
                .more(GO_TO, |ui| {
                    let mut text = go_to.borrow_mut();
                    let field = mg_uikit::text_field(&mut *text)
                        .desired_width(GO_TO)
                        .hint_text("Go to row");
                    let r = ui.add(field);
                    r.widget_info(|| {
                        egui::WidgetInfo::labeled(egui::WidgetType::TextEdit, true, "Go to")
                    });
                    if r.lost_focus()
                        && ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter))
                        && let Ok(row) = text.trim().parse::<usize>()
                        && row < rows
                    {
                        gone_to.set(Some(row));
                    }
                })
                .primary(),
        );
    if wrong > 0 {
        h = h.left(Item::custom(wrong_words(wrong), wrong_width, warn).more(wrong_width, |ui| {
            if warn_button(ui, &wrong_words(wrong)).clicked() {
                to_wrong.set(true);
            }
        }));
    }
    let h = h
        .right(Item::joined(HOW_CELLS_SHOW, how))
        .right(
            Item::button(Icon::SaveAs, SAVE_AS_FILE)
                .in_more()
                .tip("Write the table to a file, which it is edited in from then on"),
        )
        .right(
            Item::button(Icon::Import, TAKE_ROWS)
                .in_more()
                .enabled(has_copies)
                .tip("Take rows from another copy of this table, such as the game's"),
        )
        .right(
            Item::toggle(None, COMPACT, &mut compact)
                .in_more()
                .tip("Save with one space between cells. Off, with the columns aligned."),
        )
        .right(
            Item::button(Icon::Eye, SHOW_HIDDEN)
                .in_more()
                .enabled(hidden > 0)
                .shown_words(show_hidden_words(hidden))
                .tip(hidden_tip),
        )
        .right(
            Item::button(Icon::Column, RESET_ALL_WIDTHS)
                .in_more()
                .tip("Every column of this table back to its first width"),
        );
    let out = crate::widgets::header_with_ok_cancel(ui, h, None, crate::side_doc::OK_CANCEL_TIPS);
    let widths_key = app.tables[at].widths_key(app);
    let doc = &mut app.tables[at];
    doc.filter = filter.into_inner();
    doc.go_to = go_to.into_inner();
    doc.add_count = count.get();
    doc.value_names = value_names;
    doc.strref_text = strref_text;
    if compact != doc.compact {
        doc.compact = compact;
        doc.edit.mark_unsaved();
    }
    if add.get() {
        doc.commit();
        let row = doc.edit.add_rows(doc.add_count.max(1));
        doc.anchor = None;
        doc.selected = Some((row, doc.selected.map_or(0, |s| s.1)));
        doc.reveal = true;
        doc.changed();
    }
    if out.clicked(EMPTY_ROW)
        && let Some(row) = row
    {
        doc.typing = None;
        doc.edit.clear_row(row);
        doc.changed();
    }
    if out.clicked(DELETE_ROW)
        && let Some(row) = row
    {
        doc.typing = None;
        doc.edit.remove_row(row);
        doc.anchor = None;
        let column = doc.selected.map_or(0, |s| s.1);
        doc.selected = doc.edit.table().len().checked_sub(1).map(|last| (row.min(last), column));
        doc.reveal = true;
        doc.changed();
    }
    if out.clicked(ADD_COLUMN) {
        doc.renaming = None;
        doc.new_column = Some(String::new());
    }
    if let Some(row) = gone_to.get() {
        doc.filter.clear();
        doc.anchor = None;
        doc.selected = Some((row, doc.selected.map_or(0, |s| s.1)));
        doc.reveal = true;
        take_keys(&ctx, doc.id);
    }
    if to_wrong.get() {
        doc.go_to = "!".into();
    }
    if out.clicked(TAKE_ROWS) {
        doc.commit();
        doc.taking = Some(Taking::default());
    }
    if out.clicked(SHOW_HIDDEN) {
        doc.hidden.clear();
    }
    if out.clicked(RESET_ALL_WIDTHS) {
        reset_column_width(&mut app.settings, &widths_key, None);
    }
    Asked { save: out.clicked("Save"), save_as: out.clicked(SAVE_AS_FILE), pressed: out.end }
}

/// The header's names (its controls' accessible names, and the tests').
pub const ADD_ROW: &str = "Add Row";
/// Add Row's count's name.
pub const ADD_COUNT: &str = "How many rows Add Row adds";
pub const EMPTY_ROW: &str = "Empty Row";
pub const DELETE_ROW: &str = "Delete Row";
pub const ADD_COLUMN: &str = "Add Column…";
pub const HOW_CELLS_SHOW: &str = "How cells show";
pub const VALUES_AS_NAMES: &str = "Values as names";
pub const STRINGS_AS_TEXT: &str = "Strings as text";
pub const SAVE_AS_FILE: &str = "Save As File…";
pub const TAKE_ROWS: &str = "Take Rows…";
pub const COMPACT: &str = "Compact";
pub const SHOW_HIDDEN: &str = "Show Hidden Columns";
pub const RESET_ALL_WIDTHS: &str = "Reset All Widths";

/// "N cells are wrong" as the board draws it: a flat button, its warning
/// icon and words in `warn`.
fn warn_button(ui: &mut Ui, words: &str) -> egui::Response {
    let t = mg_uikit::tokens(ui.ctx());
    let font = egui::FontId::proportional(mg_uikit::metrics::BODY_SIZE);
    let galley = ui.painter().layout_no_wrap(words.to_string(), font, t.warn);
    let size = egui::vec2(
        4.0 + mg_uikit::metrics::ICON + 6.0 + galley.size().x + 8.0,
        mg_uikit::metrics::CONTROL_HEIGHT,
    );
    let (rect, r) = ui.allocate_exact_size(size, egui::Sense::click());
    if r.hovered() {
        ui.painter().rect_filled(rect, 4.0, t.raised);
    }
    if r.has_focus() {
        let ring = egui::Stroke::new(1.0, t.accent);
        ui.painter().rect_stroke(rect, 4.0, ring, egui::StrokeKind::Inside);
    }
    let icon = egui::Rect::from_min_size(
        egui::pos2(rect.left() + 4.0, rect.center().y - mg_uikit::metrics::ICON / 2.0),
        egui::Vec2::splat(mg_uikit::metrics::ICON),
    );
    mg_uikit::Icon::Warning.paint(ui.painter(), icon, t.warn);
    let at = egui::pos2(icon.right() + 6.0, rect.center().y - galley.size().y / 2.0);
    ui.painter().galley(at, galley, t.warn);
    r.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, words));
    r.on_hover_text(WRONG_TIP)
}

/// Under the header, where there is something to say: a cell refused, a
/// paste or a save that failed (the table's problem), and the folder a
/// description names being read.
fn notes(app: &Moonglow, ui: &mut Ui, at: usize) {
    let doc = &app.tables[at];
    let walks = |w: &(Arc<std::sync::atomic::AtomicBool>, _)| {
        w.0.load(std::sync::atomic::Ordering::Relaxed)
    };
    let walking = doc.walking.as_ref().is_some_and(walks);
    if doc.problem.is_none() && !walking {
        return;
    }
    egui::Frame::new().inner_margin(egui::Margin::symmetric(8, 2)).show(ui, |ui| {
        if walking {
            ui.weak("Reading the folder the table's description names scripts and tables from…");
        }
        if let Some(problem) = &doc.problem {
            ui.colored_label(ui.visuals().error_fg_color, problem);
        }
    });
}

/// What the rows shown were worked out for: the revision, the history,
/// the rows looked within, the changes, the description, the Find and
/// whether by value names.
type ShownFor = (u64, u64, Same<Vec<usize>>, u64, Same<Description>, String, bool);

/// An `Arc` in a cache key, told by `Arc::ptr_eq` and held: by its address
/// alone, another made where a freed one was was taken for it (review C-19).
#[derive(Debug, Clone)]
struct Same<T>(Option<Arc<T>>);

impl<T> PartialEq for Same<T> {
    fn eq(&self, other: &Self) -> bool {
        match (&self.0, &other.0) {
            (Some(a), Some(b)) => Arc::ptr_eq(a, b),
            (None, None) => true,
            _ => false,
        }
    }
}

/// The rows shown: those a Find finds, else every row. Worked out once
/// for the table as it is and the Find as it is typed (a large table's
/// every cell was looked through at each frame), and again when either
/// changes.
fn shown_rows(doc: &mut TableDoc) -> Arc<Vec<usize>> {
    let within = Same(doc.within.clone());
    let description = doc.description();
    let changes = doc.edit.changes();
    let key: ShownFor = (
        doc.revision,
        doc.history,
        within,
        changes,
        Same(description.clone()),
        doc.filter.clone(),
        doc.value_names,
    );
    if let Some((k, rows)) = &doc.shown
        && *k == key
    {
        return rows.clone();
    }
    if let Some((prev_k, prev_rows)) = &doc.shown {
        let (_, history, prev_within, prev_changes, prev_desc, prev_filter, prev_names) = prev_k;
        // (The same history: a table read again starts another.)
        if *prev_changes < changes
            && (*history, prev_within, prev_desc, prev_filter.as_str(), *prev_names)
                == (doc.history, &key.2, &key.4, doc.filter.as_str(), doc.value_names)
            && let Some((touched, _)) = doc.edit.cell_rows_since(*prev_changes)
        {
            let mut next = prev_rows.clone();
            let t = doc.edit.table();
            let needle = mg_core::text::Needle::new(&doc.filter);
            if !needle.is_empty() {
                let desc = description.filter(|_| doc.value_names);
                let kinds: Vec<Option<&Kind>> =
                    t.columns().iter().map(|c| Some(&desc.as_deref()?.column(c)?.kind)).collect();
                let mut fields = Vec::new();
                for r in touched {
                    if r >= t.len() || doc.within.as_ref().is_some_and(|w| !w.contains(&r)) {
                        continue;
                    }
                    let matches = row_matches(t, r, &needle, &kinds, &mut fields);
                    match next.binary_search(&r) {
                        Ok(_) if matches => {}
                        Err(_) if !matches => {}
                        Ok(idx) => {
                            Arc::make_mut(&mut next).remove(idx);
                        }
                        Err(idx) => {
                            Arc::make_mut(&mut next).insert(idx, r);
                        }
                    }
                }
            }
            doc.shown = Some((key, next.clone()));
            return next;
        }
    }
    let rows = Arc::new(rows_found(doc));
    doc.shown = Some((key, rows.clone()));
    rows
}

fn row_matches(
    t: &TwoDa,
    r: usize,
    needle: &mg_core::text::Needle,
    kinds: &[Option<&Kind>],
    fields: &mut Vec<String>,
) -> bool {
    if needle.is_empty() {
        return true;
    }
    if needle.is_single_word() {
        for (c, kind) in kinds.iter().enumerate() {
            if let Some(v) = t.cell(r, c)
                && (needle.finds(v)
                    || kind.and_then(|k| k.named(v)).is_some_and(|n| needle.finds(&n)))
            {
                return true;
            }
        }
        return false;
    }
    fields.clear();
    for (c, kind) in kinds.iter().enumerate() {
        if let Some(v) = t.cell(r, c) {
            fields.push(mg_core::text::fold(v));
            fields.extend(kind.and_then(|k| k.named(v)).map(|n| mg_core::text::fold(&n)));
        }
    }
    needle.finds_in_strings(fields)
}

fn rows_found(doc: &TableDoc) -> Vec<usize> {
    let t = doc.edit.table();
    let needle = mg_core::text::Needle::new(&doc.filter);
    let all: Vec<usize> = match &doc.within {
        Some(within) => within.iter().copied().filter(|r| *r < t.len()).collect(),
        None => (0..t.len()).collect(),
    };
    if needle.is_empty() {
        return all;
    }
    // (And by the names shown for the values of a list or of flags.)
    let description = doc.description().filter(|_| doc.value_names);
    let kinds: Vec<Option<&Kind>> =
        t.columns().iter().map(|c| Some(&description.as_deref()?.column(c)?.kind)).collect();
    // (Every word in one of the row's cells, or their names.)
    let mut fields = Vec::new();
    all.into_iter().filter(|&r| row_matches(t, r, &needle, &kinds, &mut fields)).collect()
}

/// Gives the keys to a table's grid: the arrows, and Undo and Redo, are
/// that table's from then on.
pub fn take_keys(ctx: &egui::Context, table: u32) {
    give_keys(ctx, Keys::Table);
    ctx.data_mut(|d| {
        d.insert_temp(egui::Id::new("table-keys-of"), table);
        // (Its editor made the active one as its tab is next drawn:
        // `focus_asked`.)
        d.insert_temp(egui::Id::new("table-keys-focus"), table);
    });
}

/// Where a grid was given the keys (a click on a cell, by the pointer or a
/// screen reader, F6, a press in its panels, a test), its tab's pane is
/// given the focus, so that its editor is the active one
/// ([`grid_has_keys`]): `tab` drawing the grid `id`.
pub(crate) fn focus_asked(app: &mut Moonglow, ctx: &egui::Context, id: u32, tab: &Tab) {
    let key = egui::Id::new("table-keys-focus");
    if ctx.data(|d| d.get_temp::<u32>(key)) != Some(id) {
        return;
    }
    ctx.data_mut(|d| d.remove_temp::<u32>(key));
    // (Set once the editor area is drawn: the dock is the area's while
    // its tabs are, `Moonglow::ui`.)
    app.focus_asked_tab = Some(tab.clone());
}

/// Tab and Shift+Tab are the grid's while it has the keys and nothing has
/// the keyboard's focus (its Next column and the one before): egui would
/// give the focus to the first control drawn in the frame (the Top Bar's,
/// a panel's), and the grid, drawn after, would then see a control with
/// the focus and leave the key to it, as it did before M4's U16 (the
/// board's "Tab Next column" did nothing). Asked as the frame begins,
/// before any control is drawn.
pub(crate) fn keep_tab_for_the_grid(app: &Moonglow, ctx: &egui::Context) {
    let tab = ctx.input(|i| {
        let m = i.modifiers;
        i.key_pressed(egui::Key::Tab) && (!m.any() || m.shift_only())
    });
    if !tab || ctx.memory(|m| m.focused().is_some()) {
        return;
    }
    // (Only for a grid of the active editor, nothing typed in it: the
    // editor's, or a table only looked at, the resource tab's or the hak
    // editor's view, `browser::two_da`.)
    let Some(id) = keys_table(ctx) else { return };
    let editor = app.tables.iter().find(|d| d.id == id);
    let drawn = match editor {
        Some(d) => d.typing.is_none() && grid_has_keys(app, ctx, id, &Tab::Table(id)),
        None => crate::browser::view_tab(app, id).is_some_and(|t| grid_has_keys(app, ctx, id, &t)),
    };
    if drawn {
        ctx.memory_mut(|m| m.move_focus(egui::FocusDirection::None));
    }
}

/// The tab of the active editor: the one in front of the pane with the
/// focus (a group's, or a floating window's, the one last pressed in),
/// else the active docked editor; as the frame began.
pub(crate) fn active_tab(app: &Moonglow) -> Option<Tab> {
    app.active_tab_now.clone()
}

/// [`active_tab`] worked out from the dock (as the frame begins:
/// `Moonglow::ui`, before the editor area takes the dock to draw it).
pub(crate) fn front_of_focused(app: &Moonglow) -> Option<Tab> {
    let front = app.dock.focused_leaf().and_then(|pane| {
        let leaf = app.dock.leaf(pane).ok()?;
        leaf.tabs.get(leaf.active.0).cloned()
    });
    front.or_else(|| app.active_editor().cloned())
}

/// Whether the grid `id` (drawn in `tab`) has the keys: they were given
/// to it last (a click on a cell, F6, its panels), its editor is the
/// active one (a press in another editor's group or window ends it, a
/// press in a dock's panel gives the keys to that panel), and nothing
/// modal, no dialog, no menu and no popover is in front. Its keys (the arrows, Tab,
/// Enter, Delete, Ctrl+C and Ctrl+V) act and are hinted only then.
pub(crate) fn grid_has_keys(app: &Moonglow, ctx: &egui::Context, id: u32, tab: &Tab) -> bool {
    keys_are(ctx, Keys::Table)
        && keys_table(ctx) == Some(id)
        && active_tab(app).as_ref() == Some(tab)
        && !crate::shell::keys_blocked(ctx)
        // (A kit popover open over it, the header's More's: not egui's
        // popups, which `keys_blocked` sees.)
        && !mg_uikit::popover::Popover::any_open(ctx)
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
    let id = app.tables[at].id;
    let has = grid_has_keys(app, ui.ctx(), id, &Tab::Table(id));
    // Undo and Redo are the table's while the keys are in its grid (the
    // keymap's, through `Moonglow::local_undo`).
    if has {
        app.undo_keys = Some(Tab::Table(id));
    }
    keys_in(&mut app.tables[at], ui, false, has);
}

/// [`keys`] in a table's grid; `read_only` (the resource browser's view):
/// the keys that move and Ctrl+C alone; `has`: whether its grid has the
/// keys ([`grid_has_keys`]).
pub(crate) fn keys_in(doc: &mut TableDoc, ui: &mut Ui, read_only: bool, has: bool) {
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
    if !has || doc.typing.is_some() {
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
    // (Past the columns hidden from view, to the next one shown.)
    let names = doc.edit.table().columns().to_vec();
    let shown = |c: usize| !names.get(c).is_some_and(|n| doc.hidden.contains(&n.to_lowercase()));
    if pressed(ui, none, Key::ArrowRight) || tab {
        let next = (column + 1..columns).find(|c| shown(*c)).unwrap_or(column);
        to = Some((row, next));
    }
    if pressed(ui, none, Key::ArrowLeft) || back_tab {
        let next = (0..column).rev().find(|c| shown(*c)).unwrap_or(column);
        to = Some((row, next));
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
    if pressed(ui, none, Key::Escape) {
        give_keys(ui.ctx(), Keys::Area);
    }
    if read_only {
        let (copy, _, _) = crate::widgets::copy_cut_paste(ui);
        if copy {
            ui.ctx().copy_text(copied(doc.edit.table(), &chosen, left, right));
        }
        return;
    }
    if pressed(ui, none, Key::Enter) || pressed(ui, none, Key::F2) {
        let now = doc.edit.table().cell(row, column).unwrap_or_default().to_string();
        doc.typing = Some(now);
    } else if let Some(typed) = typed_text(ui) {
        // A letter typed begins typing: the cell's text replaced by it,
        // the cursor after it, as a spreadsheet does.
        let id = egui::Id::new(("table-cell", doc.id));
        let mut state = egui::text_edit::TextEditState::load(ui.ctx(), id).unwrap_or_default();
        let end = egui::text::CCursor::new(typed.chars().count());
        state.cursor.set_char_range(Some(egui::text::CCursorRange::one(end)));
        state.store(ui.ctx(), id);
        doc.typing = Some(typed);
    }
    if pressed(ui, none, Key::Delete) && doc.edit.clear_cells(&chosen, left, right) > 0 {
        doc.changed();
    }
    let (copy, cut, paste) = crate::widgets::copy_cut_paste(ui);
    if copy || cut {
        ui.ctx().copy_text(copied(doc.edit.table(), &chosen, left, right));
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
            // Under a Find the lines go into the rows shown, from the
            // cursor's on (what was copied or cut there is of those rows):
            // counted on from the cursor's number, they went into the rows
            // between that the Find hides. A paste of more lines than the
            // Find shows rows for is refused: where the rest would go is
            // not in sight.
            let at = rows.iter().position(|r| *r == row).unwrap_or(place);
            let narrowed = rows.len() < doc.edit.table().len();
            let (lines, room) = (Editing::pasted_lines(&text), rows.len() - at);
            if narrowed && lines > room {
                doc.problem = Some(format!(
                    "Not pasted: {lines} lines, and the rows shown have room for {room} from \
                     here. Clear Find to paste past them."
                ));
            } else {
                let into: &[usize] = if narrowed { &rows[at..] } else { &[row] };
                match doc.edit.paste_into(into, column, &text) {
                    Ok(0) => {}
                    Ok(_) => doc.changed(),
                    Err(e) => doc.problem = Some(e.to_string()),
                }
            }
        }
    }
}

/// Over a read-only view's grid (the resource browser's): Find, Go to
/// and Show Hidden Columns, as the editor has them.
pub(crate) fn view_header(ui: &mut Ui, doc: &mut TableDoc) {
    ui.horizontal_wrapped(|ui| {
        ui.label("Find");
        let field = ui.id().with(("table-filter", doc.id));
        crate::widgets::SearchField::new(field, &mut doc.filter)
            .hint("text in any cell")
            .width(170.0)
            .find_over()
            .show(ui);
        ui.label("Go to");
        let go = ui.add(mg_uikit::text_field(&mut doc.go_to).desired_width(70.0).hint_text("row"));
        if go.lost_focus()
            && ui.input(|i| i.key_pressed(egui::Key::Enter))
            && let Ok(row) = doc.go_to.trim().parse::<usize>()
            && row < doc.edit.table().len()
        {
            doc.filter.clear();
            doc.anchor = None;
            doc.selected = Some((row, doc.selected.map_or(0, |s| s.1)));
            doc.reveal = true;
            take_keys(ui.ctx(), doc.id);
        }
        if !doc.hidden.is_empty() {
            let names: Vec<&str> = doc.hidden.iter().map(String::as_str).collect();
            if ui
                .button(format!("Show Hidden Columns ({})", doc.hidden.len()))
                .on_hover_text(format!("Hidden from view: {}", names.join(", ")))
                .clicked()
            {
                doc.hidden.clear();
            }
        }
        if let Some((row, column)) = doc.selected
            && let Some(name) = doc.edit.table().columns().get(column)
        {
            // (The whole value, which its cell may cut short.)
            let value = doc.edit.table().cell(row, column).unwrap_or("****");
            ui.weak(format!("Row {row}, {name}: {value}"));
        }
    });
}

/// The cells `chosen` (rows) from `left` to `right`, as a spreadsheet
/// copies them (and Paste reads them): a line a row, tabs between cells.
fn copied(t: &TwoDa, chosen: &[usize], left: usize, right: usize) -> String {
    let lines: Vec<String> = chosen
        .iter()
        .map(|&r| {
            let cells: Vec<&str> = (left..=right).map(|c| t.cell(r, c).unwrap_or("****")).collect();
            cells.join("\t")
        })
        .collect();
    lines.join("\n")
}

fn grid(
    app: &mut Moonglow,
    ui: &mut Ui,
    at: usize,
    kinds: &[(&'static str, ColumnKind)],
    faults: &[CellFault],
) {
    let game = app.game.clone();
    let table_name = app.tables[at].widths_key(app);
    let id = app.tables[at].id;
    let has_keys = grid_has_keys(app, ui.ctx(), id, &Tab::Table(id));
    let talk = app.talk.as_ref().filter(|_| !app.talk_view.outside);
    let doc = &mut app.tables[at];
    let settings = &mut app.settings;
    let of = GridOf { settings, table_name: &table_name, talk, game: game.as_deref(), has_keys };
    grid_in(ui, doc, of, kinds, faults, None);
}

/// What a grid is drawn with besides its table.
pub(crate) struct GridOf<'a> {
    pub settings: &'a mut crate::settings::Settings,
    /// What its columns' widths are kept by ([`TableDoc::widths_key`]).
    pub table_name: &'a str,
    /// The module's talk table, open for editing (its lines not yet saved
    /// shown too).
    pub talk: Option<&'a mg_module::talk::Table>,
    pub game: Option<&'a mg_rules::GameData>,
    /// Whether its grid has the keys ([`grid_has_keys`]): the cell with
    /// them ringed in the accent.
    pub has_keys: bool,
}

/// The name a view's column of layers ("From") keeps its width by: no
/// table's column is named so.
pub(crate) const FROM: &str = "<from>";

/// A table's grid: the editor's, or (`layers` given) the resource
/// browser's read-only view of a table the game reads, each row's layer
/// beside it and the cells a layer changed marked; there a cell is
/// selected, the keys move, Find narrows and Ctrl+C copies, and nothing
/// is changed (a column hidden or its width aside).
/// A run of a row's cells out of sight sideways, not yet passed over:
/// their width and how many.
#[derive(Default)]
struct Unseen {
    width: f32,
    cells: usize,
}

impl Unseen {
    /// Passes over the run in one place: the ids of two widgets for each
    /// cell, as a cell out of sight took one by one.
    fn pass(&mut self, ui: &mut Ui, height: f32) {
        if self.cells > 0 {
            ui.allocate_space(egui::vec2(self.width, height));
            ui.skip_ahead_auto_ids(2 * self.cells - 1);
        }
        *self = Self::default();
    }
}

pub(crate) fn grid_in(
    ui: &mut Ui,
    doc: &mut TableDoc,
    of: GridOf<'_>,
    kinds: &[(&'static str, ColumnKind)],
    faults: &[CellFault],
    layers: Option<&mg_module::table_layers::Layered>,
) {
    // (The board's grid, `Table.html`'s `.tbl .gr`: no gap between the
    // cells, a line at each one's right and foot.)
    ui.spacing_mut().item_spacing.x = 0.0;
    let GridOf { settings, table_name, talk, game, has_keys } = of;
    let read_only = layers.is_some();
    // (The layer each row comes from, where the table has several.)
    let layered = layers.filter(|l| l.copies.len() > 1);
    let from_width = match layered {
        Some(_) => column_widths(settings, table_name, &[FROM.to_string()], 110.0)[0],
        None => 0.0,
    };
    let lead = if layered.is_some() { from_width + ui.spacing().item_spacing.x } else { 0.0 };
    let rows = shown_rows(doc);
    let columns = doc.columns_cached();
    // (A column hidden from view is drawn in no room: its cells are kept
    // and saved, and the keys pass over it.)
    let widths: Vec<f32> = column_widths(settings, table_name, &columns, CELL)
        .into_iter()
        .zip(columns.iter())
        .map(|(w, c)| if doc.hidden.contains(&c.to_lowercase()) { 0.0 } else { w })
        .collect();
    // Where each column begins, from the first's left.
    let lefts: Vec<f32> = widths
        .iter()
        .scan(lead, |x, w| {
            let left = *x;
            if *w > 0.0 {
                *x += w + ui.spacing().item_spacing.x;
            }
            Some(left)
        })
        .collect();
    // The edge of a column's name dragged or double-clicked (or of From).
    let mut edge = None;
    let mut from_edge = None;
    let description = doc.description();
    let described = |c: &str| description.as_deref().and_then(|d| d.column(c));
    let kind_of = |c: &str| {
        let known = kinds.iter().find(|(k, _)| k.eq_ignore_ascii_case(c)).map(|(_, k)| *k);
        known.or_else(|| matches!(described(c)?.kind, Kind::StrRef).then_some(ColumnKind::StrRef))
    };
    // (A view takes a column by its name too: Name, Description…)
    let is_strref = |c: &str| {
        kind_of(c) == Some(ColumnKind::StrRef)
            || (read_only && mg_module::table_layers::is_strref_column(c))
    };
    let strref: Vec<bool> = columns.iter().map(|c| doc.strref_text && is_strref(c)).collect();
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
    // A row of the grid: a table's row (22, none between), or its text
    // and a margin where that is taller. The row numbers beside the rows
    // are as high, in the same rows' style: they were the text's height
    // and its gap apart, the cells a control's and its gap, and drifted
    // half a row from their rows by the fifteenth.
    let (control, step) = mg_uikit::row_metrics(ui);
    let row_height = control.max(ui.text_style_height(&egui::TextStyle::Monospace) + 6.0);
    let pitch = row_height + (step - control);
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
    let (mut clicked, mut begin, mut done, mut cancel) = (None, false, false, false);
    // A column's name, right-clicked: renamed (false) or removed (true).
    let mut column_act = None;
    // A column moved: from which place to which (its menu, or its name
    // dragged onto another's).
    let mut column_move = None;
    // A column hidden, or its width (or every column's) put back.
    let mut column_view = None;
    let moving = doc.moving.filter(|_| ui.input(|i| i.pointer.primary_down()));
    let released = doc.moving.is_some() && moving.is_none();
    let mut over_column = None;
    let wrong = ui.visuals().error_fg_color;
    let changed = ui.visuals().warn_fg_color;
    // (The tokens read once for the grid's every cell.)
    let tokens = ui.tokens();
    // The column names stay in sight over the rows, and the rows' numbers
    // beside them: each goes with the rows' own scroll, a frame behind.
    let hidden = egui::scroll_area::ScrollBarVisibility::AlwaysHidden;
    let still = egui::scroll_area::ScrollSource::NONE;
    let (scroll_x, scroll_y) = (doc.scroll.x, doc.scroll.y);
    let spacing_x = ui.spacing().item_spacing.x;
    let bar = ui.spacing().scroll.bar_width + ui.spacing().scroll.bar_inner_margin;
    let view_width = (ui.available_width() - NUMBER).max(0.0);
    let rows_height = (ui.available_height() - look::HEADER_HIGH - bar).max(row_height);
    let scroll_edge = std::mem::take(&mut doc.edge_scroll);
    let mut target_x =
        (dragging && scroll_edge.x != 0.0).then_some((scroll_x + scroll_edge.x).max(0.0));
    let mut target_y =
        (dragging && scroll_edge.y != 0.0).then_some((scroll_y + scroll_edge.y).max(0.0));
    if reveal && let Some((row, column)) = selected {
        if let Some(place) = rows.iter().position(|r| *r == row) {
            let above = (rows_height / 2.0 - pitch).max(0.0);
            target_y = Some((place as f32 * pitch - above).max(0.0));
        }
        // Sideways only where the cell is out of sight, snapped to column edges.
        let left = lefts.get(column).copied().unwrap_or_default();
        let step = widths.get(column).copied().unwrap_or(CELL) + spacing_x;
        if left - lead < scroll_x || left + step > scroll_x + view_width {
            target_x = Some(snap_reveal_x(&lefts, &widths, column, view_width));
        }
    }
    let head_x = target_x.unwrap_or(scroll_x);
    let gutter_y = target_y.unwrap_or(scroll_y);
    let id = doc.id;
    let selected_column = selected.map(|s| s.1);
    ui.horizontal(|ui| {
        // (The corner over the rows' numbers, the gutter's.)
        let (corner, _) =
            ui.allocate_exact_size(egui::vec2(NUMBER, look::HEADER_HIGH), egui::Sense::hover());
        look::header_cell(ui, corner);
        egui::ScrollArea::horizontal()
            .id_salt(("table-head", id))
            .auto_shrink([false, true])
            .scroll_bar_visibility(hidden)
            .scroll_source(still)
            .horizontal_scroll_offset(head_x)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    if layered.is_some() {
                        let (rect, _) = ui.allocate_exact_size(
                            egui::vec2(from_width, look::HEADER_HIGH),
                            egui::Sense::hover(),
                        );
                        look::header_cell(ui, rect);
                        let text = egui::RichText::new("From").color(ui.tokens().text);
                        let layout = egui::Layout::left_to_right(egui::Align::Center);
                        let inner = rect.shrink2(egui::vec2(4.0, 0.0));
                        ui.new_child(egui::UiBuilder::new().max_rect(inner).layout(layout))
                            .add(egui::Label::new(text).truncate().selectable(false))
                            .on_hover_text("The layer each row comes from");
                        let edge_id = egui::Id::new(("table-from-edge", id));
                        if let Some(act) = column_edge(ui, edge_id, rect, "From") {
                            from_edge = Some(act);
                        }
                    }
                    for (index, c) in columns.iter().enumerate() {
                        if widths[index] == 0.0 {
                            continue;
                        }
                        // (The cell's column's name in the accent, the
                        // board's `.ch.cur`.)
                        let t = ui.tokens();
                        let color = if selected_column == Some(index) { t.accent } else { t.text };
                        let text = egui::RichText::new(c).color(color);
                        let sense = match read_only {
                            true => egui::Sense::click(),
                            false => egui::Sense::click_and_drag(),
                        };
                        let (rect, r) = ui.allocate_exact_size(
                            egui::vec2(widths[index], look::HEADER_HIGH),
                            sense,
                        );
                        look::header_cell(ui, rect);
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
                        mg_uikit::context_menu(&r, |ui| {
                            if read_only {
                                if ui.button("Hide Column").clicked() {
                                    column_view = Some(ColumnView::Hide(index));
                                    ui.close();
                                }
                                if ui.button("Reset Width").clicked() {
                                    column_view = Some(ColumnView::Reset(Some(index)));
                                    ui.close();
                                }
                                if ui.button("Reset All Widths").clicked() {
                                    column_view = Some(ColumnView::Reset(None));
                                    ui.close();
                                }
                                return;
                            }
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
                            mg_uikit::menu_separator(ui);
                            if ui
                                .button("Hide Column")
                                .on_hover_text(
                                    "Hide it while the table is open. Its cells are kept and saved.",
                                )
                                .clicked()
                            {
                                column_view = Some(ColumnView::Hide(index));
                                ui.close();
                            }
                            if ui
                                .button("Reset Width")
                                .on_hover_text("This column back to the width it began with")
                                .clicked()
                            {
                                column_view = Some(ColumnView::Reset(Some(index)));
                                ui.close();
                            }
                            if ui
                                .button("Reset All Widths")
                                .on_hover_text("Every column of this table back to its first width")
                                .clicked()
                            {
                                column_view = Some(ColumnView::Reset(None));
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
                            (None, Some(ColumnKind::Script)) => {
                                format!("{c}: a script's name, which the game runs")
                            }
                            (None, None) => c.clone(),
                        };
                        r.on_hover_text(match read_only {
                            true => {
                                about
                                    + "\nDrag its right edge to resize, double-click there to fit. \
                                       Right-click to hide it."
                            }
                            false => {
                                about
                                    + "\nDrag to move the column, its right edge to resize. \
                                       Right-click for more."
                            }
                        });
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
        Some((c, Edge::Width(w))) => set_column_width(settings, table_name, &columns[c], w),
        Some((c, Edge::Fit)) => {
            // (To every row's cell, as shown: not those of a Find alone.)
            let t = doc.edit.table();
            let cells = (0..t.len()).map(|r| {
                cell_shows(t.cell(r, c), strref[c], named[c], (talk, game), &doc.pending).0
            });
            let w = fit_width(ui, &columns[c], cells);
            set_column_width(settings, table_name, &columns[c], w);
        }
        None => {}
    }
    match (from_edge, layered) {
        (Some(Edge::Width(w)), _) => set_column_width(settings, table_name, FROM, w),
        (Some(Edge::Fit), Some(l)) => {
            let w = fit_width(ui, "From", l.copies.iter().map(|c| c.0.clone()));
            set_column_width(settings, table_name, FROM, w);
        }
        _ => {}
    }
    match column_view {
        Some(ColumnView::Hide(c)) => doc.hide_column(c),
        Some(ColumnView::Reset(c)) => {
            reset_column_width(settings, table_name, c.map(|c| columns[c].as_str()));
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
    if let Some(y) = target_y {
        list = list.vertical_scroll_offset(y);
    }
    if let Some(x) = target_x {
        list = list.horizontal_scroll_offset(x);
    }
    let room = ui.available_height();
    let shown = mg_uikit::with_rows(ui, |ui| {
        ui.horizontal_top(|ui| {
            let gutter = egui::vec2(NUMBER, (room - bar).max(row_height));
            let down = egui::Layout::top_down(egui::Align::Min);
            ui.allocate_ui_with_layout(gutter, down, |ui| {
                // (The board's gutter: `bar`, a line at its right.)
                look::gutter(ui, ui.max_rect());
                egui::ScrollArea::vertical()
                    .id_salt(("table-numbers", id))
                    .auto_shrink([false, false])
                    .scroll_bar_visibility(hidden)
                    .scroll_source(still)
                    .vertical_scroll_offset(gutter_y)
                    .show_rows(ui, row_height, rows.len(), |ui, range| {
                        for &r in &rows[range] {
                            let (rect, _) = ui.allocate_exact_size(
                                egui::vec2(NUMBER, row_height),
                                egui::Sense::hover(),
                            );
                            look::number(ui, &tokens, rect, r);
                        }
                    });
            });
            ui.vertical(|ui| {
                list.show_rows(ui, row_height, rows.len(), |ui, range| {
                    // (The cells' lines drawn once a row and once a
                    // column over the rows shown, not two to a cell: and
                    // the ring of the cell with the keys over them.)
                    let mut lines = look::Lines::default();
                    for &r in &rows[range] {
                        // (The row's own faults: every cell laid out
                        // looked through all of the table's.)
                        let wrong_here = row_faults(faults, r);
                        let row = ui.horizontal(|ui| {
                            // (The layer the row comes from, in a view.)
                            if let Some(l) = layered {
                                let origin = l.row_origin(r);
                                let text = egui::RichText::new(&l.copies[origin].0);
                                let text =
                                    if origin + 1 < l.copies.len() { text } else { text.weak() };
                                let (rect, _) = ui.allocate_exact_size(
                                    egui::vec2(from_width, row_height),
                                    egui::Sense::hover(),
                                );
                                let layout = egui::Layout::left_to_right(egui::Align::Center);
                                let inner = rect.shrink2(egui::vec2(4.0, 0.0));
                                ui.new_child(egui::UiBuilder::new().max_rect(inner).layout(layout))
                                    .add(egui::Label::new(text).truncate().selectable(false));
                            }
                            // (The cells out of sight in a run, not yet
                            // passed over.)
                            let mut unseen = Unseen::default();
                            for (c, is_strref) in strref.iter().copied().enumerate() {
                                let width = widths[c];
                                if width == 0.0 {
                                    continue;
                                }
                                let here = selected == Some((r, c));
                                if here && let Some(text) = doc.typing.as_mut() {
                                    unseen.pass(ui, row_height);
                                    let edit = mg_uikit::text_field(text)
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
                                    lines.cell(rect, look::Cell::Plain);
                                    continue;
                                }
                                // A cell scrolled out of sight sideways takes
                                // its place and no more: no text made, nothing
                                // looked up (spells.2da's 55 columns were all
                                // laid out for every row shown), and a run of
                                // them one place (each a widget of its own
                                // cost 0.3 ms a frame). It counts as many
                                // widgets as one drawn, so the ids of those
                                // after it stay theirs.
                                let size = egui::vec2(width, row_height);
                                let at = ui.cursor().min + egui::vec2(unseen.width, 0.0);
                                if !ui.is_rect_visible(egui::Rect::from_min_size(at, size)) {
                                    unseen.width += width;
                                    unseen.cells += 1;
                                    continue;
                                }
                                unseen.pass(ui, row_height);
                                let raw = doc.edit.table().cell(r, c);
                                let fault = wrong_here.iter().find(|f| f.column == c);
                                let (label, text, was_named) = cell_shows(
                                    raw,
                                    is_strref,
                                    named[c],
                                    (talk, game),
                                    &doc.pending,
                                );
                                let mut rich = egui::RichText::new(label).monospace();
                                if raw.is_none() {
                                    rich = rich.weak();
                                }
                                if fault.is_some() {
                                    rich = rich.color(wrong);
                                }
                                // (A cell a layer changed, not added: the
                                // row is in the copy below.)
                                let was = layered.and_then(|l| {
                                    let origin = l.cell_origin(r, c);
                                    let below = l.copies.get(origin + 1)?;
                                    (r < below.1.len()).then(|| l.before(r, c)).flatten().map(
                                        |(below, value)| {
                                            (l.copies[origin].0.as_str(), below, value)
                                        },
                                    )
                                });
                                if was.is_some() {
                                    rich = rich.color(changed);
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
                                // (The board's grid: cells chosen `cell-range`,
                                // the one with the keys `accent-fill` with
                                // `on-fill` text, ringed in `accent` while the
                                // grid has the keys; a hovered cell `hover-row`;
                                // lines at each cell's right and foot.)
                                let t = &tokens;
                                let state = match (here, within) {
                                    (true, _) => look::Cell::Current { ringed: has_keys },
                                    (false, true) => look::Cell::Chosen,
                                    _ if cell.hovered() => look::Cell::Hovered,
                                    _ => look::Cell::Plain,
                                };
                                look::cell(ui.painter(), t, rect, state);
                                lines.cell(rect, state);
                                // (Its text `on-fill` whatever it is: a wrong
                                // cell's red, or a layer's change's, read
                                // poorly on the fill in the light theme; the
                                // wrong cell keeps its red outline.)
                                if here {
                                    rich = match raw {
                                        Some(_) => rich.color(t.on_fill),
                                        None => rich.color(t.on_fill.gamma_multiply(0.6)),
                                    };
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
                                let has_tip =
                                    was_named || text.is_some() || fault.is_some() || was.is_some();
                                let cell = match has_tip {
                                    false => cell,
                                    true => cell.on_hover_ui(|ui| {
                                        let mut tip = Vec::new();
                                        if was_named {
                                            tip.push(raw.unwrap_or_default().to_string());
                                        } else if text.is_some() {
                                            tip.push(format!("StrRef {}", raw.unwrap_or_default()));
                                        }
                                        if let Some(f) = fault {
                                            tip.push(f.message.clone());
                                        }
                                        if let Some((by, below, value)) = was {
                                            let value = value.unwrap_or("****");
                                            tip.push(format!("Set by {by} ({value} in {below})"));
                                        }
                                        ui.label(tip.join("\n"));
                                    }),
                                };
                                if cell.clicked() && shift {
                                    reach = Some((r, c));
                                } else if cell.clicked() || cell.double_clicked() {
                                    clicked = Some((r, c));
                                    begin = cell.double_clicked() && !read_only;
                                }
                            }
                            unseen.pass(ui, row_height);
                        });
                        let at = row.response.rect;
                        lines.row(at.left(), at.right(), at.bottom());
                    }
                    lines.paint(ui.painter(), &tokens);
                })
            })
            .inner
        })
        .inner
    });
    doc.scroll = shown.state.offset;
    if (shown.state.offset - egui::vec2(head_x, gutter_y)).length_sq() > 0.25 {
        ui.ctx().request_repaint();
    }
    // A drag selecting cells past the grid's edge scrolls it that way,
    // the faster the further, and the selection reaches the row and
    // column at that edge, as a spreadsheet's does.
    if dragging
        && let Some(p) = pointer
        && let Some((_, column)) = doc.selected
    {
        let inner = shown.inner_rect;
        let past = |at: f32, low: f32, high: f32, most: f32| {
            if at > high {
                (at - high).min(most)
            } else if at < low {
                -(low - at).min(most)
            } else {
                0.0
            }
        };
        let by = egui::vec2(
            past(p.x, inner.left(), inner.right(), 3.0 * CELL),
            past(p.y, inner.top(), inner.bottom(), 3.0 * pitch),
        );
        if by != egui::Vec2::ZERO && !rows.is_empty() {
            doc.edge_scroll = by;
            ui.ctx().request_repaint();
            let (offset, seen) = (doc.scroll, inner.size());
            let place = match by.y {
                y if y > 0.0 => ((offset.y + seen.y) / pitch) as usize,
                y if y < 0.0 => (offset.y / pitch).ceil() as usize,
                _ => doc.selected.and_then(|(r, _)| rows.iter().position(|x| *x == r)).unwrap_or(0),
            };
            let column = match by.x {
                x if x > 0.0 => {
                    lefts.iter().rposition(|l| *l < offset.x + seen.x).unwrap_or(column)
                }
                x if x < 0.0 => lefts.iter().position(|l| *l >= offset.x).unwrap_or(column),
                _ => column,
            };
            let to = (rows[place.min(rows.len() - 1)], column);
            if Some(to) != doc.selected {
                doc.select_to(to);
            }
        }
    }
    // (The grid's keys are said by the ring on the cell that has them, the
    // board's, not an outline round the whole grid.)
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

/// How a table's grid is painted (M4's U16 review, the Table board's
/// `.tbl .gr`): the header row's cells, the rows' numbers' gutter, and a
/// cell by its state; for the 2DA editor's grid and the read-only views'
/// (`grid_in`), and for another grid of rows and columns (U17's talk
/// table) to call.
pub(crate) mod look {
    use mg_uikit::TokensExt as _;

    /// The header row's height (`.gh`, 24).
    pub(crate) const HEADER_HIGH: f32 = 24.0;

    /// A cell of the header row (`.gh .ch`): `bar`, a `line` at its right
    /// and foot.
    pub(crate) fn header_cell(ui: &egui::Ui, rect: egui::Rect) {
        let t = ui.tokens();
        let p = ui.painter();
        p.rect_filled(rect, 0.0, t.bar);
        let line = egui::Stroke::new(1.0, t.line);
        p.vline(rect.right() - 0.5, rect.y_range(), line);
        p.hline(rect.x_range(), rect.bottom() - 0.5, line);
    }

    /// The rows' numbers' gutter (`.gr .rn`): `bar`, a `line` at its
    /// right.
    pub(crate) fn gutter(ui: &egui::Ui, rect: egui::Rect) {
        let t = ui.tokens();
        ui.painter().rect_filled(rect, 0.0, t.bar);
        ui.painter().vline(rect.right() - 0.5, rect.y_range(), egui::Stroke::new(1.0, t.line));
    }

    /// A row's number in the gutter: monospace 12 in `text-3`, at its
    /// right with 8 to spare, a `cell-line` at its foot.
    pub(crate) fn number(ui: &mut egui::Ui, t: &mg_uikit::Tokens, rect: egui::Rect, row: usize) {
        ui.painter().hline(
            rect.x_range(),
            rect.bottom() - 0.5,
            egui::Stroke::new(1.0, t.cell_line),
        );
        let text = egui::RichText::new(row.to_string())
            .font(egui::FontId::monospace(12.0))
            .color(t.text_3);
        let inner = rect.shrink2(egui::vec2(8.0, 0.0));
        let layout = egui::Layout::right_to_left(egui::Align::Center);
        ui.new_child(egui::UiBuilder::new().max_rect(inner).layout(layout))
            .add(egui::Label::new(text).selectable(false));
    }

    /// A cell's state, as it is painted.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub(crate) enum Cell {
        Plain,
        Hovered,
        /// Among those chosen (`.c.rng`).
        Chosen,
        /// The one with the keys (`.c.act`), ringed where the grid has them.
        Current {
            ringed: bool,
        },
    }

    /// A cell (`.gr .c`): its fill by its state. (Its lines, `cell-line`
    /// at its right and foot, and the ring of the one with the keys are
    /// [`Lines`]'.)
    pub(crate) fn cell(p: &egui::Painter, t: &mg_uikit::Tokens, rect: egui::Rect, state: Cell) {
        match state {
            Cell::Plain => {}
            Cell::Hovered => _ = p.rect_filled(rect, 0.0, t.hover_row),
            Cell::Chosen => _ = p.rect_filled(rect, 0.0, t.cell_range),
            Cell::Current { .. } => _ = p.rect_filled(rect, 0.0, t.accent_fill),
        }
    }

    /// The lines of the cells drawn in a pass, painted after them: one
    /// `cell-line` at each column's right down the rows shown and one at
    /// each row's foot across them (the board's lines at each cell's right
    /// and foot, as few shapes), then the ring of the cell with the keys.
    #[derive(Default)]
    pub(crate) struct Lines {
        /// The columns' right edges, from the first row drawn.
        rights: Vec<f32>,
        /// The rows' feet, each with its cells' left and right.
        feet: Vec<(f32, f32, f32)>,
        /// The top of the first row drawn.
        top: Option<f32>,
        /// The ring's cell.
        ring: Option<egui::Rect>,
        /// Whether the cells taken so far are of the first row drawn.
        first: bool,
    }

    impl Lines {
        /// A cell drawn (of the row begun last).
        pub(crate) fn cell(&mut self, rect: egui::Rect, state: Cell) {
            if self.top.is_none() {
                self.top = Some(rect.top());
                self.first = true;
            }
            if self.first {
                self.rights.push(rect.right());
            }
            if state == (Cell::Current { ringed: true }) {
                self.ring = Some(rect);
            }
        }

        /// A row ended, its cells from `left` to `right` with its foot at
        /// `foot`.
        pub(crate) fn row(&mut self, left: f32, right: f32, foot: f32) {
            self.first = false;
            if self.top.is_some() {
                self.feet.push((left, right, foot));
            }
        }

        /// Paints them.
        pub(crate) fn paint(&self, p: &egui::Painter, t: &mg_uikit::Tokens) {
            let line = egui::Stroke::new(1.0, t.cell_line);
            if let (Some(top), Some((.., foot))) = (self.top, self.feet.last()) {
                for x in &self.rights {
                    p.vline(x - 0.5, top..=*foot, line);
                }
            }
            for (left, right, foot) in &self.feet {
                p.hline(*left..=*right, foot - 0.5, line);
            }
            if let Some(rect) = self.ring {
                let ring = egui::Stroke::new(1.0, t.accent);
                p.rect_stroke(rect, 0.0, ring, egui::StrokeKind::Inside);
            }
        }
    }
}

/// Text typed this frame (not with Ctrl, Alt or Command), taken from the
/// input: what begins typing into a cell of the grid.
fn typed_text(ui: &mut Ui) -> Option<String> {
    ui.input_mut(|i| {
        if i.modifiers.command || i.modifiers.ctrl || i.modifiers.alt {
            return None;
        }
        let mut typed = String::new();
        i.events.retain(|e| match e {
            egui::Event::Text(t) if !t.chars().any(char::is_control) => {
                typed.push_str(t);
                false
            }
            _ => true,
        });
        (!typed.is_empty()).then_some(typed)
    })
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

/// What a column holds, where the checks know: the table's own
/// description first (a `strref` column of it is a talk-table string's
/// cell; one of another kind is the description's alone), else the kinds
/// the checks know of the game's tables.
fn column_kind(
    kinds: &[(&'static str, ColumnKind)],
    description: Option<&Description>,
    column: &str,
) -> Option<ColumnKind> {
    let known = kinds.iter().find(|(k, _)| k.eq_ignore_ascii_case(column)).map(|(_, k)| *k);
    match description.and_then(|d| d.column(column)).map(|d| &d.kind) {
        Some(Kind::StrRef) => Some(ColumnKind::StrRef),
        Some(Kind::Text) | None => known,
        Some(_) => None,
    }
}

/// [`talk_string`], or the text of a line New Line will add as the table
/// is saved (`pending`).
fn pending_or_talk_string(
    pending: &Pending,
    talk: Option<&mg_module::talk::Table>,
    game: Option<&mg_rules::GameData>,
    n: u32,
) -> Option<String> {
    pending.text(n).map(str::to_string).or_else(|| talk_string(talk, game, n))
}

/// A talk-table string's text: a custom one (16777216 and on) as the
/// module's talk table open for editing has it (a line added and not yet
/// saved too), else as the game reads it.
pub fn talk_string(
    talk: Option<&mg_module::talk::Table>,
    game: Option<&mg_rules::GameData>,
    n: u32,
) -> Option<String> {
    let strref = mg_core::StrRef(n);
    if strref.is_custom()
        && let Some(t) = talk
    {
        let row = strref.index() as usize;
        return (row < t.len()).then(|| t.line(row).text).filter(|t| !t.is_empty());
    }
    game?.string(strref)
}

/// How many strings a list of them shows at most.
const STRINGS_SHOWN: usize = 200;

/// The talk tables' strings whose text holds `find` (as Find reads it):
/// the module's own table's first (as it is being edited, else as the
/// game reads it), then the game's `dialog.tlk`'s, which is looked
/// through only for something typed. Each string's number and text, at
/// most [`STRINGS_SHOWN`], and how many there are in all.
pub fn strings_found(
    talk: Option<&mg_module::talk::Table>,
    game: Option<&mg_rules::GameData>,
    find: &str,
) -> (Vec<(u32, String)>, usize) {
    let needle = mg_core::text::Needle::new(find.trim());
    let mut found = Vec::new();
    let mut all = 0;
    let mut offer = |n: u32, text: String| {
        if !text.is_empty() && needle.finds(&text) {
            all += 1;
            if found.len() < STRINGS_SHOWN {
                found.push((n, text));
            }
        }
    };
    match (talk, game.and_then(|g| g.custom_tlk())) {
        (Some(t), _) => (0..t.len())
            .for_each(|row| offer(mg_module::talk::Table::strref(row).0, t.line(row).text)),
        (None, Some(tlk)) => {
            let cp = game.map(|g| g.codepage()).unwrap_or_default();
            for row in 0..tlk.entries.len() {
                let strref = mg_core::StrRef(mg_core::StrRef::CUSTOM_FLAG | row as u32);
                offer(strref.0, tlk.text_in(strref, cp).unwrap_or_default());
            }
        }
        (None, None) => {}
    }
    if let Some(g) = game
        && !needle.is_empty()
    {
        let (tlk, cp) = (g.tlk(), g.codepage());
        for row in 0..tlk.entries.len() as u32 {
            offer(row, tlk.text_in(mg_core::StrRef(row), cp).unwrap_or_default());
        }
    }
    (found, all)
}

/// A talk-table string's cell, chosen: **Choose String…**, the strings
/// found by a part of their text, one clicked going in the cell; and
/// **New Line…**, a line of text added to the module's own talk table,
/// whose number goes in the cell (`addable`: the module has a table to
/// add to; the line is then `new_line`'s, added by the caller).
fn string_choice(
    ui: &mut Ui,
    id: u32,
    (talk, game): (Option<&mg_module::talk::Table>, Option<&mg_rules::GameData>),
    value: Option<&str>,
    addable: bool,
    set: &mut Option<Option<String>>,
    new_line: &mut Option<String>,
) {
    let base = egui::Id::new(("table-string", id));
    let (choose_id, new_id) = (base.with("choose"), base.with("new"));
    let mut choosing: bool = ui.data(|d| d.get_temp(choose_id)).unwrap_or(false);
    let mut adding: bool = ui.data(|d| d.get_temp(new_id)).unwrap_or(false);
    let choose = ui
        .selectable_label(choosing, "Choose String…")
        .on_hover_text("The strings of the module's talk table and the game's, by their text");
    if choose.clicked() {
        choosing = !choosing;
        adding = false;
    }
    let new = ui
        .add_enabled(addable, egui::Button::selectable(adding, "New Line…"))
        .on_hover_text("Add a line to the module's talk table, its number in the cell")
        .on_disabled_hover_text("Needs the module's own talk table, open for editing");
    if new.clicked() {
        adding = !adding;
        choosing = false;
    }
    if choosing {
        // (On a line of its own under the cell's: the panel's row wraps.)
        ui.end_row();
        ui.vertical(|ui| {
            ui.set_width(ui.available_width().min(560.0));
            let field_id = base.with("find");
            let mut find: String = ui.data(|d| d.get_temp(field_id)).unwrap_or_default();
            let _ = crate::widgets::SearchField::new(field_id.with("field"), &mut find)
                .hint("Part of a string's text")
                .show(ui);
            ui.data_mut(|d| d.insert_temp(field_id, find.clone()));
            // (Looked through again when what is typed, or the module's
            // table, changes: the game's has a hundred thousand strings.)
            let mark = (find.clone(), talk.map(|t| (t.len(), t.steps())));
            let kept_id = base.with("found");
            type Kept = (String, Option<(usize, (usize, usize))>, Arc<(Vec<(u32, String)>, usize)>);
            let kept: Option<Kept> = ui.data(|d| d.get_temp(kept_id));
            let found = match kept.filter(|(f, t, _)| (f, t) == (&mark.0, &mark.1)) {
                Some((_, _, found)) => found,
                None => {
                    let found = Arc::new(strings_found(talk, game, &find));
                    ui.data_mut(|d| d.insert_temp(kept_id, (mark.0, mark.1, found.clone())));
                    found
                }
            };
            let (names, all) = (&found.0, found.1);
            match (all, find.trim().is_empty()) {
                (0, true) => _ = ui.weak("Type a part of a string's text"),
                (0, false) => _ = ui.weak("No string has it"),
                (n, _) if n > names.len() => {
                    ui.weak(format!("{} of {n}: type more of the text", names.len()));
                }
                _ => {}
            }
            let list = egui::ScrollArea::vertical()
                .id_salt(base.with("list"))
                .max_height(240.0)
                .min_scrolled_height(200.0);
            list.show(ui, |ui| {
                mg_uikit::list_rows(ui);
                for (n, text) in names {
                    let now = value.and_then(|v| v.trim().parse::<u32>().ok()) == Some(*n);
                    let line = text.lines().next().unwrap_or_default();
                    let label = egui::Button::selectable(now, format!("{n}  {line}")).truncate();
                    if ui.add(label).on_hover_text(text).clicked() {
                        *set = Some(Some(n.to_string()));
                        choosing = false;
                    }
                }
            });
        });
    }
    if adding && addable {
        ui.end_row();
        ui.vertical(|ui| {
            let text_id = base.with("text");
            let mut text: String = ui.data(|d| d.get_temp(text_id)).unwrap_or_default();
            ui.label("The new line's text");
            let field = ui.add(
                egui::TextEdit::multiline(&mut text)
                    .desired_rows(2)
                    .desired_width(ui.available_width().min(420.0)),
            );
            crate::widgets::autofocus(ui, &field);
            let typed = !text.trim().is_empty();
            ui.horizontal(|ui| {
                let add =
                    ui.add_enabled(typed, egui::Button::new("Add to Talk Table")).on_hover_text(
                        "The cell takes the new line's number, added as the table is saved.",
                    );
                if add.clicked() {
                    *new_line = Some(text.trim().to_string());
                    text.clear();
                    adding = false;
                }
                if crate::widgets::cancel(ui) {
                    text.clear();
                    adding = false;
                }
            });
            ui.data_mut(|d| d.insert_temp(text_id, text));
        });
    }
    ui.data_mut(|d| {
        d.insert_temp(choose_id, choosing);
        d.insert_temp(new_id, adding);
    });
}

/// A row of the table `target` to choose (the table itself as it is now,
/// for a row of its own; else `other`, as read), with the one named now.
/// The models the game reads (its own, the haks', the module's) whose
/// names hold `find`, by name, at most `most`.
pub(crate) fn models_named(
    game: &mg_rules::GameData,
    find: &str,
    most: usize,
) -> (Vec<String>, usize) {
    let find = find.trim().to_ascii_lowercase();
    let mut names: Vec<String> = (game.resman.layers().iter())
        .flat_map(|l| l.container.keys().filter(|k| k.restype == ResType::MDL).map(|k| k.resref))
        .map(|r| r.to_string())
        .filter(|n| n.contains(&find))
        .collect();
    names.sort();
    names.dedup();
    let all = names.len();
    names.truncate(most);
    (names, all)
}

/// What tells one load order from another, for a list made from it and
/// kept: its layers by name, each with how many resources it has (a file
/// added to a hak, the module or `override` changes one).
fn load_order_mark(game: &mg_rules::GameData) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    for l in game.resman.layers() {
        (&l.label, l.priority, l.container.len()).hash(&mut h);
    }
    h.finish()
}

/// A model's cell, chosen: whether the game has the model it names (and
/// where: the layer the game reads it from), checked for the cell chosen
/// (not for every row, which the game's own tables would fail: they name
/// models the game never shipped), and **Choose Model…**, a list of the
/// models the game reads, found by a part of the name.
fn model_choice(
    ui: &mut Ui,
    id: u32,
    game: Option<&mg_rules::GameData>,
    value: Option<&str>,
    set: &mut Option<Option<String>>,
) {
    let Some(game) = game else {
        ui.weak("a model's name");
        return;
    };
    let key = value.and_then(|v| ResRef::from_str(v).ok()).map(|r| ResKey::new(r, ResType::MDL));
    // (Each said on a line of its own, wrapping at the panel's width, the
    // chooser under it: M4's U16, the Cell panel's.)
    let said = |text: String, color: egui::Color32| {
        egui::Label::new(egui::RichText::new(text).color(color)).wrap()
    };
    let weak = ui.visuals().weak_text_color();
    match key.map(|k| (k, game.resman.copies(&k))) {
        None => _ = ui.add(said("a model's name".into(), weak)),
        Some((_, copies)) if copies.is_empty() => {
            let warn = ui.visuals().warn_fg_color;
            let text = "no model of this name in the game, its haks or the module".to_string();
            ui.add(said(text, warn))
                .on_hover_text("Used, the row shows nothing (or the game's stand-in)");
        }
        Some((_, copies)) => {
            let layer = &game.resman.layers()[copies[0]].label;
            ui.add(said(format!("a model, read from {layer}"), weak));
        }
    }
    ui.end_row();
    let find_id = egui::Id::new(("table-model-find", id));
    let open_id = find_id.with("open");
    let mut open: bool = ui.data(|d| d.get_temp(open_id)).unwrap_or(false);
    if ui.selectable_label(open, "Choose Model…").clicked() {
        open = !open;
    }
    ui.data_mut(|d| d.insert_temp(open_id, open));
    if !open {
        return;
    }
    // (On a line of its own under the cell's: the panel's row wraps.)
    ui.end_row();
    ui.vertical(|ui| {
        ui.set_width(ui.available_width().min(360.0));
        let mut find: String = ui.data(|d| d.get_temp(find_id)).unwrap_or_default();
        let _ = crate::widgets::SearchField::new(find_id.with("field"), &mut find)
            .hint("Part of a model's name")
            .show(ui);
        ui.data_mut(|d| d.insert_temp(find_id, find.clone()));
        // (Looked through again when what is typed, or the load order,
        // changes, as the strings are: every frame the list showed, each
        // layer's every key was gone through, a name made of each model's
        // and all of them sorted, 50,000 with CEP's haks.)
        let mark = (find.clone(), load_order_mark(game));
        let kept_id = find_id.with("found");
        type Kept = (String, u64, Arc<(Vec<String>, usize)>);
        let kept: Option<Kept> = ui.data(|d| d.get_temp(kept_id));
        let found = match kept.filter(|(f, m, _)| (f, *m) == (&mark.0, mark.1)) {
            Some((_, _, found)) => found,
            None => {
                let found = Arc::new(models_named(game, &find, 200));
                ui.data_mut(|d| d.insert_temp(kept_id, (mark.0, mark.1, found.clone())));
                found
            }
        };
        let (names, all) = (&found.0, found.1);
        if all > names.len() {
            ui.weak(format!("{} of {all}: type more of the name", names.len()));
        }
        let list = egui::ScrollArea::vertical()
            .id_salt(find_id.with("list"))
            .max_height(240.0)
            .min_scrolled_height(200.0);
        // (The rows in sight alone: the 200 laid out each frame took
        // 0.12 ms while it was open.)
        mg_uikit::show_rows(list, ui, names.len(), |ui, range| {
            for n in &names[range] {
                let now = value.is_some_and(|v| v.eq_ignore_ascii_case(n));
                if ui.selectable_label(now, n).clicked() {
                    *set = Some(Some(n.clone()));
                    ui.data_mut(|d| d.insert_temp(open_id, false));
                }
            }
        });
    });
}

fn row_choice(
    ui: &mut Ui,
    id: u32,
    (name, table): (&str, &TwoDa),
    (target, other): (&str, Option<&TwoDa>),
    named: Option<usize>,
    width: f32,
    set: &mut Option<Option<String>>,
) {
    let other = if target.eq_ignore_ascii_case(name) { Some(table) } else { other };
    let shown = match (&other, named) {
        (Some(t), Some(n)) if n < t.len() => row_label(t, n),
        (_, Some(n)) => format!("{n}  (no such row)"),
        _ => "none".to_string(),
    };
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
        .width(width.min(300.0))
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
    // (What the table's widths are kept by: a hak's or a file's table's
    // not by its name alone, and a column renamed in one lost its width.)
    let widths_of = app.tables[at].widths_key(app);
    let settings = &mut app.settings;
    let doc = &mut app.tables[at];
    let Some(mut name) = doc.new_column.clone() else { return };
    let mut open = true;
    let mut cancelled = false;
    let mut add = false;
    let renaming = doc.renaming.filter(|c| *c < doc.edit.table().columns().len());
    let title = if renaming.is_some() { "Rename Column" } else { "Add Column" };
    let footed = crate::widgets::dialog(ctx, title, &mut open)
        .collapsible(false)
        .resizable(false)
        .show_foot(ctx, |ui| {
            ui.label("The column's name (no spaces)");
            let field = ui.add(mg_uikit::text_field(&mut name).desired_width(220.0));
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
            let go = if renaming.is_some() { "Rename" } else { "Add" };
            ((), crate::widgets::Foot::new().main(go, fits && !taken).cancel())
        });
    if let Some(footed) = &footed {
        add = footed.pressed.main();
        cancelled = footed.pressed.cancelled();
    }
    if add {
        let done = match renaming {
            Some(column) => {
                let was = doc.edit.table().columns()[column].clone();
                let done = doc.edit.rename_column(column, name.trim());
                if done.is_ok() {
                    rename_column_width(settings, &widths_of, &was, name.trim());
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
        taking.cells.clear();
        taking.open_row = None;
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
    let footed = crate::widgets::dialog(ctx, "Take Rows from Another Copy", &mut open)
        .default_size([760.0, 520.0])
        .show_foot(ctx, |ui| {
            if others.is_empty() {
                ui.label("No other copy of this table is in the load order.");
                return ((), crate::widgets::Foot::new().main("Take Rows", false).cancel());
            }
            ui.horizontal(|ui| {
                ui.label("From");
                let labels: Vec<Option<String>> =
                    others.iter().map(|(label, _)| Some(label.clone())).collect();
                let mut pick = from.clone();
                let width = 260.0_f32.min(ui.available_width());
                crate::widgets::dropdown(("take-from", doc.id))
                    .selected_text(from.clone().unwrap_or_default())
                    .width(width)
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
                    taking.cells.clear();
                    taking.open_row = None;
                }
            });
            // (Under the copy chosen, wrapped: beside it, it ran past a
            // narrow window's edge.)
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
            ui.horizontal_wrapped(|ui| {
                // A box is ticked while every row of its kind is.
                let kinds = [
                    (Offered::New, "New", "The rows this table lacks"),
                    (
                        Offered::Changed,
                        "Differ",
                        "Rows the other copy has differently, which this table didn't change \
                         itself",
                    ),
                ];
                for (kind, label, hint) in kinds {
                    let rows: Vec<_> =
                        found.iter().filter(|c| c.offered == kind).map(|c| c.row).collect();
                    let mut all =
                        !rows.is_empty() && rows.iter().all(|r| taking.chosen.contains(r));
                    let tick =
                        ui.add_enabled(!rows.is_empty(), mg_uikit::Checkbox::new(&mut all, label));
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
                    taking.cells.clear();
                }
            });
            ui.separator();
            if found.is_empty() {
                ui.weak("This table has every row of that copy as it has them.");
            }
            // (Room under the list for a row's cells being chosen.)
            let under = if taking.open_row.is_some() { 110.0 } else { 0.0 };
            let room = (ui.available_height() - under).max(120.0);
            // (Rows of a check box and words, 22 apart: they were given the
            // text's height and a margin, and their check boxes were taller.)
            mg_uikit::show_rows(
                egui::ScrollArea::vertical().max_height(room).auto_shrink([false, false]),
                ui,
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
                            if ui.check_box(&mut on, format!("Row {}", c.row)).changed() {
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
                            // (A row both changed: its cells, chosen one by
                            // one under the list.)
                            if c.offered == Offered::Conflict {
                                let open = taking.open_row == Some(c.row);
                                let some = taking.cells.iter().filter(|(r, _)| *r == c.row).count();
                                let label = match some {
                                    0 => "Cells…".to_string(),
                                    n => format!("Cells… ({n})"),
                                };
                                if ui
                                    .selectable_label(open, label)
                                    .on_hover_text("Choose the cells of this row to take")
                                    .clicked()
                                {
                                    taking.open_row = if open { None } else { Some(c.row) };
                                }
                            }
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
                                        _ => format!("{}: {mine} -> {theirs}", columns[*col]),
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
            // The cells of the row both changed, to take one by one.
            if let Some(c) = taking.open_row.and_then(|r| found.iter().find(|c| c.row == r)) {
                ui.separator();
                let whole = taking.chosen.contains(&c.row);
                ui.label(match whole {
                    true => format!("Row {}: taken whole (uncheck it to choose its cells)", c.row),
                    false => format!("Row {}'s cells: check those to take from that copy", c.row),
                });
                // (A line each, in a list of its own: a row may differ in
                // many cells.)
                let cells = egui::ScrollArea::vertical().id_salt("take-cells").max_height(100.0);
                ui.add_enabled_ui(!whole, |ui| {
                    cells.auto_shrink([false, true]).show(ui, |ui| {
                        mg_uikit::list_rows(ui);
                        for (i, (col, mine, theirs)) in c.cells.iter().enumerate() {
                            let (mine, theirs) = (
                                mine.as_deref().unwrap_or("****"),
                                theirs.as_deref().unwrap_or("****"),
                            );
                            let mut on = whole || taking.cells.contains(&(c.row, *col));
                            let line = format!("{}: {mine} -> {theirs}", columns[*col]);
                            ui.horizontal(|ui| {
                                let text = egui::RichText::new(line).monospace();
                                let r = ui.check_box(&mut on, text);
                                let whose = match c.whose.get(i) {
                                    Some(Whose::Theirs) => "changed there",
                                    Some(Whose::Ours) => "changed here",
                                    Some(Whose::Both) => "changed in both",
                                    _ => "",
                                };
                                if !whose.is_empty() {
                                    ui.weak(whose);
                                }
                                if r.changed() {
                                    match on {
                                        true => _ = taking.cells.insert((c.row, *col)),
                                        false => _ = taking.cells.remove(&(c.row, *col)),
                                    }
                                }
                            });
                        }
                    });
                });
            }
            let n = taking.chosen.len();
            let m = taking.cells.iter().filter(|(r, _)| !taking.chosen.contains(r)).count();
            let label = match (n, m) {
                (n, 0) => format!("Take {n} Rows"),
                (0, m) => format!("Take {m} Cells"),
                (n, m) => format!("Take {n} Rows and {m} Cells"),
            };
            let foot = crate::widgets::Foot::new()
                .main(label, n + m > 0)
                .tip("Take each checked row whole, and each chosen cell alone. One step to undo.")
                .cancel();
            ((), foot)
        });
    if let Some(footed) = &footed {
        take = footed.pressed.main();
        cancelled = footed.pressed.cancelled();
    }
    if take && let Some((_, t)) = other {
        let rows: Vec<usize> = taking.chosen.iter().copied().collect();
        let cells: Vec<(usize, usize)> = taking.cells.iter().copied().collect();
        if doc.edit.take(t, &rows, &cells) != (0, 0) {
            doc.changed();
        }
        doc.taking = None;
    } else {
        doc.taking = (open && !cancelled).then_some(taking);
    }
}

/// Snaps a horizontal reveal scroll offset to a column left edge so `column`
/// is visible without slicing the leftmost column header mid-text.
fn snap_reveal_x(lefts: &[f32], widths: &[f32], column: usize, width: f32) -> f32 {
    let col_left = lefts.get(column).copied().unwrap_or_default();
    let col_w = widths.get(column).copied().unwrap_or(CELL);
    if width <= 0.0 || col_w <= 0.0 {
        return 0.0;
    }
    let need_min_x = (col_left + col_w - width).max(0.0);
    std::iter::once(0.0)
        .chain(lefts.iter().zip(widths.iter()).filter(|&(_, &w)| w > 0.0).map(|(&l, _)| l))
        .find(|&x| x <= col_left && x >= need_min_x)
        .unwrap_or_else(|| need_min_x.min(col_left))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The rows shown and the faults are kept for the description they
    /// were worked out with, held: by its address alone, another read
    /// where a freed one was was taken for it (review C-19).
    #[test]
    fn the_rows_shown_and_the_faults_hold_their_description() {
        let key = ResKey::parse("spells", mg_core::ResType::TWODA).unwrap();
        let table =
            TwoDa::parse(b"2DA V2.0\n\nLABEL\n0 a\n", mg_core::Codepage::default()).unwrap();
        let mut doc = TableDoc::read_only(u32::MAX, key, table);
        let text = mg_module::table_desc::game_description("spells").expect("Moonglow's own");
        let desc = Arc::new(Description::read(text, &|_| None).unwrap());
        doc.described = Some(Described {
            path: None,
            modified: None,
            read: Ok(desc.clone()),
            files: Default::default(),
            tables: Default::default(),
        });
        shown_rows(&mut doc);
        assert_eq!(Arc::strong_count(&desc), 3, "the rows shown hold it");
        let mut app = Moonglow::new(None, Box::new(crate::NoDialogs::default()));
        app.tables.push(doc);
        faults(&mut app, 0);
        assert_eq!(Arc::strong_count(&desc), 4, "and the faults");
    }

    /// A row's faults, taken by halving from the faults in their order,
    /// are those a look through all of them finds (the grid looked every
    /// cell laid out up so: the review of November 2026, D20).
    #[test]
    fn a_row_s_faults_are_those_a_look_through_all_finds() {
        let fault = |row, column| CellFault { row, column, message: format!("{row}:{column}") };
        let mut all = vec![fault(7, 2), fault(0, 3), fault(7, 0), fault(3, 1), fault(9, 9)];
        all.sort_by_key(|f| (f.row, f.column));
        for row in 0..12 {
            let by_look: Vec<&CellFault> = all.iter().filter(|f| f.row == row).collect();
            let halved: Vec<&CellFault> = row_faults(&all, row).iter().collect();
            assert_eq!(halved, by_look, "row {row}");
        }
        assert!(row_faults(&[], 3).is_empty());
        assert_eq!(cells_of(&all).len(), 5);
        assert!(cells_of(&all).contains(&(7, 0)) && !cells_of(&all).contains(&(0, 7)));
    }

    /// The look for a table's description, as a thread of its own makes
    /// it: the table's own beside its file, a family's in its folder, a
    /// hak's table's in the folder the hak is built from (else beside the
    /// hak); the one in hand, its file as it was, is not read again; and
    /// none where there is none (the window's thread made this look once
    /// a second: the editors' page's smaller line).
    #[test]
    fn a_table_s_description_is_looked_for_where_it_would_be() {
        let dir = mg_testkit::scratch_dir("ui-table-description-look");
        let (src, built) = (dir.join("src").join("tables"), dir.join("built"));
        std::fs::create_dir_all(&src).unwrap();
        std::fs::create_dir_all(&built).unwrap();
        let own = src.join("mg_t.2da.toml");
        std::fs::write(&own, "[columns.Label]\nhelp = \"A label\"\n").unwrap();
        let family = src.join("loot.2da.toml");
        std::fs::write(&family, "tables = [\"loot_*\"]\n").unwrap();
        let file = |name: &str| TableAt::File(src.join(name));
        let walked = std::sync::atomic::AtomicBool::new(false);
        let no_hold = std::sync::atomic::AtomicBool::new(false);
        assert!(matches!(
            look_for_description(Some(file("other.2da")), None, &walked, &no_hold),
            Looked::None
        ));
        assert!(!walked.load(std::sync::atomic::Ordering::Relaxed), "no folder walked for none");
        let found = |at: TableAt| match look_for_description(Some(at), None, &walked, &no_hold) {
            Looked::New(path, modified, _) => Some((path, modified)),
            _ => None,
        };
        let (path, modified) = found(file("mg_t.2da")).expect("its own");
        assert_eq!(path, own);
        assert!(modified.is_some());
        assert_eq!(found(file("loot_gems.2da")).map(|f| f.0), Some(family.clone()));
        assert!(matches!(
            look_for_description(Some(file("other.2da")), None, &walked, &no_hold),
            Looked::None
        ));
        assert!(matches!(look_for_description(None, None, &walked, &no_hold), Looked::None));
        // The one in hand, as it was: not read again; changed, it is.
        let had = Some((own.clone(), modified));
        assert!(matches!(
            look_for_description(Some(file("mg_t.2da")), had, &walked, &no_hold),
            Looked::Same
        ));
        let earlier = modified.map(|m| m - std::time::Duration::from_secs(60));
        let had = Some((own.clone(), earlier));
        assert!(matches!(
            look_for_description(Some(file("mg_t.2da")), had, &walked, &no_hold),
            Looked::New(..)
        ));
        let had = Some((family.clone(), modified));
        assert!(matches!(
            look_for_description(Some(file("mg_t.2da")), had, &walked, &no_hold),
            Looked::New(..)
        ));
        // A hak's table: in the folder it is built from, beside the
        // table's file there, in any case of its name; else beside the hak.
        std::fs::write(src.join("mg_t.2da"), "2DA V2.0\n\n  Label\n0 a\n").unwrap();
        let hak = |folder: Option<&std::path::Path>| TableAt::Hak {
            folder: folder.map(|f| f.to_path_buf()),
            beside: Some(built.join("mg_t.2da")),
            name: "MG_T.2da".into(),
        };
        assert_eq!(found(hak(Some(&dir.join("src")))).map(|f| f.0), Some(own.clone()));
        assert_eq!(found(hak(None)), None, "none beside the hak");
        let beside = built.join("mg_t.2da.toml");
        std::fs::write(&beside, "[columns.Label]\nhelp = \"A label\"\n").unwrap();
        assert_eq!(found(hak(None)).map(|f| f.0), Some(beside));
    }

    /// A table open in the editor is handed to another table's (whose
    /// description names its rows) as it is, copied once for each change
    /// of it, not every time it is asked for (the review of November
    /// 2026, D21).
    #[test]
    fn an_open_table_is_shared_until_it_changes() {
        let table = TwoDa::parse(b"2DA V2.0\n\n  Label\n0 a\n", mg_core::Codepage::default());
        let key = ResKey::parse("mg_t", ResType::TWODA).unwrap();
        let mut doc = TableDoc::new(1, key, Home::Module, Editing::new(table.unwrap()));
        let first = doc.shared();
        assert!(Arc::ptr_eq(&first, &doc.shared()), "the same copy while nothing changed");
        doc.edit.set(0, 0, Some("b".into())).unwrap();
        doc.changed();
        let next = doc.shared();
        assert!(!Arc::ptr_eq(&first, &next));
        assert_eq!((first.cell(0, 0), next.cell(0, 0)), (Some("a"), Some("b")));
    }

    /// The review of November 2026's smaller notes on the 2DA editor: a
    /// file's table is written through a temporary file, and a Save As
    /// File… that fails leaves the table where it lived (its next Save
    /// went to the file that wasn't there); a selection on a row an Undo
    /// took back goes to the last row; a column renamed in a file's table
    /// keeps its width (kept by the table's place, not its name alone).
    #[test]
    fn a_table_s_save_as_selection_and_widths_keep_their_place() {
        let dir = mg_testkit::scratch_dir("ui-table-smaller");
        let file = dir.join("mg_t.2da");
        std::fs::write(&file, "2DA V2.0\n\n  Label\n0 a\n").unwrap();
        // (No folder can be made under a file: the write fails.)
        std::fs::write(dir.join("a_file"), "x").unwrap();
        let bad = dir.join("a_file").join("mg_t.2da");
        let dialogs = crate::NoDialogs { save: vec![bad], ..Default::default() };
        let mut app = Moonglow::new(None, Box::new(dialogs));
        let id = open_path(&mut app, &file).expect("opened");
        let home = Home::File(file.clone());
        save_as_file(&mut app, id);
        assert_eq!(app.tables[0].home, home, "still its own file's");
        assert!(app.tables[0].problem.as_deref().is_some_and(|p| p.starts_with("Not saved")));
        // Saved: whole, and nothing left beside it.
        app.tables[0].edit.set(0, 0, Some("b".into())).unwrap();
        assert!(save(&mut app, id));
        let written = TwoDa::parse(&std::fs::read(&file).unwrap(), mg_core::Codepage::default());
        assert_eq!(written.unwrap().cell(0, 0), Some("b"));
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 2, "the table and the other file");

        // Rows added and taken back with the selection on the last.
        let doc = &mut app.tables[0];
        doc.edit.add_rows(3);
        doc.changed();
        doc.selected = Some((3, 0));
        doc.edit.undo();
        doc.changed();
        assert_eq!(doc.selected, Some((0, 0)), "on the last row there is");

        // A column renamed keeps its width, under the file's key.
        let key = app.tables[0].widths_key(&app);
        assert_ne!(key, "mg_t", "a file's table's is not kept by its name alone");
        app.settings.table_widths = vec![(key.clone(), vec![("Label".into(), 222)])];
        rename_column_width(&mut app.settings, &key, "Label", "Name");

        // Its file is looked at (its time and size) before it is read
        // again, and read when those change.
        let ctx = egui::Context::default();
        let look = |app: &mut Moonglow| {
            app.tables[0].read_at = f64::NEG_INFINITY;
            read_again(app, &ctx, 0);
        };
        look(&mut app);
        let seen = app.tables[0].seen.clone();
        assert!(matches!(seen, Some(Seen::File(..))), "{seen:?}");
        look(&mut app);
        assert_eq!(app.tables[0].seen, seen, "as it was: not read");
        std::fs::write(&file, "2DA V2.0\n\n  Label\n0 changed_outside\n").unwrap();
        look(&mut app);
        assert_eq!(app.tables[0].edit.table().cell(0, 0), Some("changed_outside"));
        assert_ne!(app.tables[0].seen, seen);
        assert_eq!(app.settings.table_widths, [(key, vec![("Name".to_string(), 222)])]);
    }

    #[test]
    fn snap_reveal_x_aligns_to_column_edges() {
        let widths = [80.0, 150.0, 90.0, 120.0, 110.0];
        let lefts = [0.0, 80.0, 230.0, 320.0, 440.0];
        // Revealing column 4 inside a 400 px viewport lands cleanly on column 2's left edge.
        let x = snap_reveal_x(&lefts, &widths, 4, 400.0);
        assert_eq!(x, 230.0);
        assert!(lefts[4] + widths[4] <= x + 400.0);
        // Column 0 stays at 0.0.
        assert_eq!(snap_reveal_x(&lefts, &widths, 0, 400.0), 0.0);
        // A column wider than the viewport aligns its left edge to the gutter.
        assert_eq!(snap_reveal_x(&lefts, &[80.0, 500.0], 1, 300.0), 80.0);
    }
}
