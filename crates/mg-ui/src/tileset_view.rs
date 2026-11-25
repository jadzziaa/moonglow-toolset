//! The tileset editor (Tools › Tilesets): a `.set` file on disk, edited in
//! place line by line (`mg_set::edit`), with undo: its general settings,
//! terrains and crossers, painting rules, tiles (and their door hooks) and
//! groups. It also makes the tileset's palette beside it, renders its
//! minimap pictures and checks it with the content doctor.
//!
//! As the Tileset board draws it (M4's U20): a header (the five pages as a
//! switch, Save, Make Palette, Check, Render Minimap Pictures, OK and
//! Cancel at its end); on Tiles, the bar over the tile list (Find,
//! Duplicate Tile, Add Tile, Remove Last Tile), the list as a table (#,
//! Model, Corners, Edges) and the chosen tile's form at its right, its
//! doors under it. It brings the Right Dock's Preview (the tile's diagram,
//! its doors in a sentence and its model, with the model in a tab of its
//! own a click away) over Textures (what the model draws), and its parts
//! to the Outliner (General, Terrains, Crossers, the rules, Tiles, Groups
//! with their counts); Check's findings go to Problems, the Bottom Dock's
//! global panel (`panels`).

use mg_uikit::Controls as _;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use egui::Ui;
use mg_core::{Codepage, ResType};
use mg_resman::ResKey;
use mg_set::edit::{self, SetFile};
use mg_set::{CORNERS, EDGES, Tileset};

use crate::dialogs::FileKind;
use crate::{Action, Moonglow, Tab};

mod panels;
pub use panels::{DIAGRAM_NAME, OPEN_MODEL, Place, go_to};
pub(crate) use panels::{
    OUTLINE_GROUPS, OutlineRow, outline_counts, outline_index, outline_rows, part_label, part_words,
};
pub(crate) use panels::{
    open_model, open_model_button, panel_detail, preview, shown, textures, tileset_of,
};

/// The editor's pages.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum Page {
    #[default]
    General,
    Terrain,
    Rules,
    Tiles,
    Groups,
}

/// The pages in the switch's order.
pub const PAGES: [Page; 5] = [Page::General, Page::Terrain, Page::Rules, Page::Tiles, Page::Groups];

/// A pending overwrite confirmation for Make Palette or Render Minimap Pictures.
#[derive(Debug, Clone)]
pub(crate) enum TilesetAsk {
    Palette {
        id: u32,
    },
    /// The tileset's `id`, and the picture names to give its unnamed
    /// tiles once answered (not before: Cancel changes nothing).
    Minimaps {
        id: u32,
        names: Vec<(usize, String)>,
        dir: PathBuf,
        tiles: Vec<(String, PathBuf)>,
        failed: Vec<String>,
    },
}

/// Incremental per-frame state for Render Minimap Pictures so rendering many
/// tiles never freezes the UI thread.
#[derive(Debug, Clone)]
pub(crate) struct MinimapJob {
    pub(crate) dir: PathBuf,
    pub(crate) tiles: Vec<(String, PathBuf)>,
    pub(crate) overwrite: bool,
    pub(crate) next: usize,
    pub(crate) written: usize,
    pub(crate) skipped: usize,
    pub(crate) failed: Vec<String>,
    pub(crate) mounted: bool,
    pub(crate) hold: bool,
    /// Rendered and said, the folder still to unmount: something held
    /// the game data at the end, so it is let go of on a later frame.
    pub(crate) unmounting: bool,
}

/// A tileset open in a tab.
#[derive(Debug)]
pub struct TilesetDoc {
    pub id: u32,
    pub path: PathBuf,
    pub file: SetFile,
    /// The file as read, after each change.
    set: Result<Arc<Tileset>, String>,
    undo: Vec<Step>,
    redo: Vec<Step>,
    /// The sections read this revision of the file, each key (upper case)
    /// with its value, and each tile's door sections: a field asks for
    /// its value every frame, and each asking went through the file's
    /// lines from the first to find the section.
    read: std::cell::RefCell<Read>,
    saved: Option<usize>,
    /// The terrain and crosser names as last read from or written to disk,
    /// so Save updates `<tileset>palstd.itp` without re-reading and parsing
    /// the `.set` from disk.
    saved_names: Option<[Vec<String>; 2]>,
    pub page: Page,
    pub tile: Option<usize>,
    pub group: Option<usize>,
    pub filter: String,
    /// A terrain's or crosser's name, while it's being added.
    pub new_type: String,
    /// A terrain (`false`) or crosser (`true`) being renamed: which, its
    /// index, and the name typed.
    pub renaming: Option<(bool, usize, String)>,
    /// The tile the list was last scrolled to or clicked: a tile chosen
    /// otherwise (a finding's place, Add Tile, the Outliner) is scrolled
    /// into sight.
    scrolled_to: Option<usize>,
    /// Counted up at every change of the file (a change, an Undo, a Redo):
    /// what is made of the file (the list's rows, the Outliner's) is made
    /// again only then.
    pub revision: u64,
    /// The text's fingerprint (`problems::fingerprint`) at a revision:
    /// made again only as the revision changes, to tell whether Check's
    /// findings in Problems are of the text as it is.
    hashed: std::cell::Cell<Option<(u64, u64)>>,
    /// A field to give the keyboard to once drawn (a finding's place):
    /// its section and key.
    pub focus_field: Option<(String, String)>,
    /// The tiles listed for the revision and the filter they were made by.
    rows_seen: std::cell::RefCell<Option<Seen<String, usize>>>,
    /// The Outliner's groups open (General has none): Terrains, Crossers,
    /// Primary rules, Secondary rules, Tiles, Groups.
    pub outline_open: [bool; 6],
    /// The Outliner's rows for the revision and the groups open.
    pub(crate) outline_seen: std::cell::RefCell<Option<Seen<[bool; 6], OutlineRow>>>,
    /// How many times the file was walked for what is made of it (the
    /// list's rows, the Outliner's): for the tests, which see that a frame
    /// that changes nothing walks it never.
    walks: std::cell::Cell<u64>,
    /// The pointer was over the Tiles page as it was last drawn: Find's
    /// keys go to its field (and its hint is shown).
    pub(crate) hovered: bool,
    /// The file's fingerprint (`problems::fingerprint`) as it was last
    /// read or written here: a change heard of outside Moonglow is one
    /// where the file is something else now (M5's W5; `None`: it could
    /// not be read when opened).
    disk: Option<u64>,
    /// The file's fingerprint as another program wrote it while this has
    /// unsaved changes: waiting for Changed Outside Moonglow's answer,
    /// meanwhile kept from being written over by Save.
    pub(crate) theirs: Option<u64>,
    /// What the log was last told of the file changed outside (its
    /// fingerprint, `u64::MAX` for a file that could not be read): said
    /// once, not at every change heard of.
    said: Option<u64>,
    /// The file's time and length as last read or written here: a look
    /// at every file (the watch's start or walk) reads only those that
    /// differ, not every open tileset on the frame.
    seen: Option<(Option<std::time::SystemTime>, u64)>,
}

/// Rows made of the file at a revision and by `K` (the words of Find, the
/// Outliner's groups open), kept for them.
pub(crate) type Seen<K, R> = ((u64, K), Arc<Vec<R>>);

/// What [`TilesetDoc`] keeps of the file as it is: sections' values,
/// tiles' doors, and cached terrain/crosser option lists.
#[derive(Debug, Default)]
struct Read {
    sections: HashMap<String, Arc<HashMap<String, String>>>,
    doors: HashMap<usize, Arc<Vec<(usize, String)>>>,
    /// `[terrains, crossers, terrains_with_none, crossers_with_none]`.
    names: [Option<Arc<Vec<String>>>; 4],
}

fn type_names(t: &Tileset) -> [Vec<String>; 2] {
    let of = |l: &[mg_set::NamedType]| l.iter().map(|x| x.name.clone()).collect();
    [of(&t.terrains), of(&t.crossers)]
}

/// A step of the tileset editor's undo: the lines it changed, as they
/// were before it (`lines`, from line `at` on), in place of `now` lines
/// of the file after it. Single-key edits use `TilesetDoc::set_key` in
/// place; structural edits diff via `TilesetDoc::change`.
#[derive(Debug, Clone, PartialEq)]
struct Step {
    at: usize,
    now: usize,
    lines: Vec<String>,
}

impl Step {
    /// The step that makes `after` into `before` again: the lines between
    /// what the two begin and end alike with.
    fn back(before: &SetFile, after: &SetFile) -> Step {
        let (a, b) = (before.lines(), after.lines());
        let head = a.iter().zip(b).take_while(|(x, y)| x == y).count();
        let most = a.len().min(b.len()) - head;
        let tail =
            a.iter().rev().zip(b.iter().rev()).take(most).take_while(|(x, y)| x == y).count();
        Step { at: head, now: b.len() - head - tail, lines: a[head..a.len() - tail].to_vec() }
    }

    /// Takes the step on `file`: the step that takes it back again.
    fn take(self, file: &mut SetFile) -> Step {
        let now = self.lines.len();
        let lines = file.splice_lines(self.at, self.now, self.lines);
        Step { at: self.at, now, lines }
    }
}

impl TilesetDoc {
    /// A key's value in a section of the file (`SetFile::get`), the
    /// section read once for each revision of the file.
    fn get(&self, section: &str, key: &str) -> Option<String> {
        let mut read = self.read.borrow_mut();
        let values = read.sections.entry(section.to_ascii_uppercase()).or_insert_with(|| {
            let mut values = HashMap::new();
            for (k, v) in self.file.entries(section) {
                // (The first of a key written twice, as the file reads it.)
                values.entry(k.to_ascii_uppercase()).or_insert(v);
            }
            Arc::new(values)
        });
        values.get(&key.to_ascii_uppercase()).cloned()
    }

    /// A tile's doors (`SetFile::doors`), found once for each revision of
    /// the file.
    fn doors(&self, tile: usize) -> Arc<Vec<(usize, String)>> {
        let mut read = self.read.borrow_mut();
        read.doors.entry(tile).or_insert_with(|| Arc::new(self.file.doors(tile))).clone()
    }

    /// Terrain (`crosser: false`) or crosser (`true`) names cached per
    /// revision, optionally prefixed with `""` for `(none)`.
    fn type_options(&self, crosser: bool, none: bool) -> Arc<Vec<String>> {
        let slot = usize::from(crosser) + 2 * usize::from(none);
        let mut read = self.read.borrow_mut();
        read.names[slot]
            .get_or_insert_with(|| {
                let list = self.tileset().map(|t| if crosser { &t.crossers } else { &t.terrains });
                let names: Vec<String> = none
                    .then(String::new)
                    .into_iter()
                    .chain(list.into_iter().flatten().map(|x| x.name.clone()))
                    .collect();
                Arc::new(names)
            })
            .clone()
    }

    fn new(id: u32, path: PathBuf, file: SetFile, disk: Option<u64>) -> TilesetDoc {
        let set = read(&file);
        let saved_names = disk.and_then(|_| set.as_deref().ok().map(type_names));
        let seen = stat(&path);
        TilesetDoc {
            id,
            path,
            file,
            set,
            undo: Vec::new(),
            redo: Vec::new(),
            read: Default::default(),
            saved: Some(0),
            saved_names,
            page: Page::default(),
            tile: None,
            group: None,
            filter: String::new(),
            new_type: String::new(),
            renaming: None,
            scrolled_to: None,
            revision: 0,
            hashed: Default::default(),
            focus_field: None,
            rows_seen: Default::default(),
            // (The board's: Terrains and Crossers open, the long lists
            // closed.)
            outline_open: [true, true, false, false, false, false],
            outline_seen: Default::default(),
            hovered: false,
            walks: std::cell::Cell::new(0),
            seen,
            disk,
            theirs: None,
            said: None,
        }
    }

    /// Shows the page named `page` (as [`page_name`] names it), the tile
    /// and the group, as a session kept them (those past the file's end
    /// left out).
    pub(crate) fn show(&mut self, page: Option<&str>, tile: Option<usize>, group: Option<usize>) {
        if let Some(p) = PAGES.into_iter().find(|p| Some(page_name(*p)) == page) {
            self.page = p;
        }
        let (tiles, groups) = match &self.set {
            Ok(t) => (t.tiles.len(), t.groups.len()),
            Err(_) => (0, 0),
        };
        self.tile = tile.filter(|&t| t < tiles);
        self.group = group.filter(|&g| g < groups);
    }

    pub fn is_dirty(&self) -> bool {
        self.saved != Some(self.undo.len())
    }

    pub fn title(&self) -> String {
        let name =
            self.path.file_name().map_or("tileset".into(), |n| n.to_string_lossy().into_owned());
        format!("{name}{}", if self.is_dirty() { " *" } else { "" })
    }

    pub fn tileset(&self) -> Option<&Tileset> {
        self.set.as_deref().ok()
    }

    /// The tileset as read, shared (a panel holds it while the file changes
    /// under it, without a copy).
    pub(crate) fn shared(&self) -> Option<Arc<Tileset>> {
        self.set.as_ref().ok().cloned()
    }

    /// Changes the file, as one undo step.
    pub fn change(&mut self, f: impl FnOnce(&mut SetFile)) {
        let before = self.file.clone();
        f(&mut self.file);
        if self.file == before {
            return;
        }
        self.record_step(Step::back(&before, &self.file));
    }

    /// Sets one key in one section in place without cloning the file.
    pub fn set_key(&mut self, section: &str, key: &str, value: &str) {
        if let Some((at, now, lines)) = self.file.set_in_place(section, key, value) {
            self.record_step(Step { at, now, lines });
        }
    }

    fn record_step(&mut self, step: Step) {
        if self.saved.is_some_and(|s| s > self.undo.len()) {
            self.saved = None;
        }
        self.undo.push(step);
        self.redo.clear();
        self.reread();
    }

    /// Whether it has a step to undo.
    pub fn has_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    /// Whether it has a step to redo.
    pub fn has_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    pub fn undo(&mut self) {
        if let Some(step) = self.undo.pop() {
            self.redo.push(step.take(&mut self.file));
            self.reread();
        }
    }

    pub fn redo(&mut self) {
        if let Some(step) = self.redo.pop() {
            self.undo.push(step.take(&mut self.file));
            self.reread();
        }
    }

    /// After the file changed: its tileset read again, what was read of it
    /// forgotten, and its revision counted up (Check's findings are then
    /// compared with the text by its fingerprint: `follow_problems`).
    fn reread(&mut self) {
        self.set = read(&self.file);
        *self.read.borrow_mut() = Read::default();
        self.revision += 1;
    }

    /// The file's text `text` (fingerprint `disk`), read again from disk
    /// (changed outside Moonglow, M5's W5), taken as the tileset's: its
    /// history goes (its steps were of the text before), it is as saved;
    /// the page, Find and the scroll stay, and the tile, the group and a
    /// terrain being renamed where they still are.
    fn take_disk(&mut self, text: &str, disk: u64) {
        self.file = SetFile::parse(text);
        self.undo.clear();
        self.redo.clear();
        self.saved = Some(0);
        self.disk = Some(disk);
        self.theirs = None;
        self.said = None;
        self.reread();
        self.saved_names = self.tileset().map(type_names);
        let counts = self
            .tileset()
            .map(|t| [t.tiles.len(), t.groups.len(), t.terrains.len(), t.crossers.len()]);
        let [tiles, groups, terrains, crossers] = counts.unwrap_or_default();
        self.tile = self.tile.filter(|t| *t < tiles);
        self.group = self.group.filter(|g| *g < groups);
        self.renaming = (self.renaming.take())
            .filter(|(crosser, i, _)| *i < if *crosser { crossers } else { terrains });
    }

    /// Changed Outside Moonglow's Keep Moonglow's: the file as another
    /// program wrote it counts as known, and what is here as not saved
    /// over it (Save writes it).
    pub(crate) fn keep_mine(&mut self) {
        if let Some(theirs) = self.theirs.take() {
            self.disk = Some(theirs);
            self.saved = None;
            // (The names the file has now, which Save's palette follows
            // from: theirs, not those of the text last read or written.)
            if let Some(names) = self.names_on_disk() {
                self.saved_names = Some(names);
            }
        }
    }

    /// The terrain and crosser names of the file as it is on disk (`None`
    /// where it can't be read as a tileset).
    fn names_on_disk(&self) -> Option<[Vec<String>; 2]> {
        let data = std::fs::read(&self.path).ok()?;
        // (Read as the editor's text is, `read`: from the bytes, a
        // non-ASCII name read otherwise than Save's and was taken as renamed.)
        let file = SetFile::parse(&Codepage::default().decode(&data));
        read(&file).ok().as_deref().map(type_names)
    }

    /// How many times the file was walked for its list's and Outliner's
    /// rows (for the tests).
    #[doc(hidden)]
    pub fn walks(&self) -> u64 {
        self.walks.get()
    }

    /// The text's fingerprint as it is, of the bytes Save writes
    /// (`bytes_of`, as the file's on disk are; made once a revision).
    pub fn fingerprint(&self) -> u64 {
        if let Some((at, hash)) = self.hashed.get()
            && at == self.revision
        {
            return hash;
        }
        let hash = crate::problems::fingerprint(&bytes_of(&self.file.text()));
        self.hashed.set(Some((self.revision, hash)));
        hash
    }

    /// Writes the file; a palette beside it (`<tileset>palstd.itp`) follows
    /// the terrains and crossers renamed since it was last saved, and what
    /// it says of that is returned.
    pub fn save(&mut self) -> Result<Option<String>, String> {
        // (Changed outside Moonglow, the question not answered yet: not
        // written over, as the module's files are not.)
        if self.theirs.is_some() {
            return Err(format!(
                "{}: changed outside Moonglow while it has unsaved changes here: choose which \
                 to keep in Changed Outside Moonglow first",
                self.path.display()
            ));
        }
        // (Written in the codepage it was read in, `open_path`; a letter
        // that has none there is not written as another.)
        let text = self.file.text();
        let Some(data) = Codepage::default().encode(&text) else {
            let lacking = crate::text::unwritable(Codepage::default(), &text);
            let shown: String = lacking.iter().take(12).collect();
            let more = if lacking.len() > 12 { "…" } else { "" };
            return Err(format!(
                "{}: {shown}{more} can't be written in {}, which the game reads a tileset in: \
                 change them to save",
                self.path.display(),
                crate::text::codepage_name(Codepage::default())
            ));
        };
        let was = self.saved_names.clone();
        // (Whole or not at all, as every file Moonglow keeps: the `.set`,
        // and the palette beside it below.)
        mg_module::write_replacing(&self.path, &data)
            .map_err(|e| format!("{}: {e}", self.path.display()))?;
        self.saved = Some(self.undo.len());
        // (Written here: not a change outside when the watch hears it.)
        self.disk = Some(crate::problems::fingerprint(&data));
        self.seen = stat(&self.path);
        self.said = None;
        let now = self.tileset().map(type_names);
        self.saved_names = now.clone();
        let (Some(was), Some(now)) = (was, now) else { return Ok(None) };
        let renamed: Vec<(&String, &String)> = (0..2)
            .flat_map(|k| was[k].iter().zip(&now[k]))
            .filter(|(a, b)| !a.eq_ignore_ascii_case(b))
            .collect();
        let palette = self.palette_path();
        if renamed.is_empty() || !palette.is_file() {
            return Ok(None);
        }
        let mut gff = std::fs::read(&palette)
            .map_err(|e| e.to_string())
            .and_then(|d| mg_gff::Gff::read(&d).map_err(|e| e.to_string()))
            .map_err(|e| format!("{}: {e}", palette.display()))?;
        let changed: usize = renamed
            .iter()
            .map(|(a, b)| mg_module::tileset::rename_in_palette(&mut gff, a, b))
            .sum();
        if changed == 0 {
            return Ok(None);
        }
        gff.to_bytes()
            .map_err(|e| e.to_string())
            .and_then(|b| mg_module::write_replacing(&palette, &b).map_err(|e| e.to_string()))
            .map_err(|e| format!("{}: {e}", palette.display()))?;
        let names: Vec<String> = renamed.iter().map(|(a, b)| format!("{a} → {b}")).collect();
        Ok(Some(format!("{}: renamed {}", palette.display(), names.join(", "))))
    }

    /// The palette beside the `.set`, as Make Palette writes it.
    pub fn palette_path(&self) -> PathBuf {
        let stem =
            self.path.file_stem().map(|s| s.to_string_lossy().to_lowercase()).unwrap_or_default();
        self.path.with_file_name(format!("{stem}palstd.itp"))
    }

    /// The tiles the list shows: all, or those whose model or a terrain or
    /// crosser has every word of Find; made once for the file's revision
    /// and the words (a tileset's thousands each frame).
    fn rows(&self) -> Arc<Vec<usize>> {
        let Some(t) = self.tileset() else { return Arc::default() };
        let mark = (self.revision, self.filter.clone());
        if let Some((was, rows)) = &*self.rows_seen.borrow()
            && *was == mark
        {
            return rows.clone();
        }
        let filter = mg_core::text::Needle::new(&self.filter);
        let rows: Vec<usize> = (0..t.tiles.len())
            .filter(|&i| {
                let tile = &t.tiles[i];
                if filter.is_empty() {
                    return true;
                }
                let corners = tile.corners.iter().map(|(c, _)| c.as_str());
                let fields: Vec<String> = std::iter::once(tile.model.as_str())
                    .chain(corners)
                    .chain(tile.edges.iter().map(String::as_str))
                    .map(mg_core::text::fold)
                    .collect();
                filter.finds_in(&fields.iter().map(String::as_str).collect::<Vec<_>>())
            })
            .collect();
        let rows = Arc::new(rows);
        self.walks.set(self.walks.get() + 1);
        *self.rows_seen.borrow_mut() = Some((mark, rows.clone()));
        rows
    }
}

/// The file as a tileset (shared: a page holds it while it changes the
/// file, without a copy of every tile each frame).
fn read(file: &SetFile) -> Result<Arc<Tileset>, String> {
    // (The text itself, as UTF-8: decoded once already, from the file.)
    let utf8 = Codepage::for_label("utf-8").unwrap_or_default();
    Tileset::parse(file.text().as_bytes(), utf8).map(Arc::new).map_err(|e| e.to_string())
}

/// The text as the file's bytes: Windows-1252, as `open_path` read it (a
/// letter it has none for as `?`, which Save refuses to write).
fn bytes_of(text: &str) -> Vec<u8> {
    Codepage::default().encode_lossy(text)
}

/// Tools › Tilesets › New Tileset…: a new `.set` with its general
/// settings, to add terrains, tiles and groups to.
pub fn new_tileset(app: &mut Moonglow) {
    let suggested = PathBuf::from("zzz01.set");
    let Some(path) = app.dialogs.save_file(FileKind::Tileset, Some(&suggested)) else { return };
    let stem = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    let name = match mg_core::ResRef::new_name(&stem) {
        Ok(name) => name.to_string_lossy(),
        Err(e) => {
            app.log.error(format!("{stem}: a tileset's name: {e}"));
            return;
        }
    };
    let file = edit::skeleton(&name);
    if let Err(e) = mg_module::write_replacing(&path, &bytes_of(&file.text())) {
        app.log.error(format!("{}: {e}", path.display()));
        return;
    }
    open_path(app, path);
}

/// Tools › Tilesets › Open Tileset….
pub fn open(app: &mut Moonglow) {
    let Some(path) = app.dialogs.open_file(FileKind::Tileset, None) else { return };
    open_path(app, path);
}

pub fn open_path(app: &mut Moonglow, path: PathBuf) {
    if let Some(d) = app.tilesets.iter().find(|d| d.path == path) {
        app.actions.push(Action::OpenTab(Tab::Tileset(d.id)));
        return;
    }
    match std::fs::read(&path) {
        Ok(data) => {
            let text = Codepage::default().decode(&data).into_owned();
            app.next_tileset += 1;
            let id = app.next_tileset;
            let disk = Some(crate::problems::fingerprint(&data));
            let doc = TilesetDoc::new(id, path, SetFile::parse(&text), disk);
            // (Opened again where Problems holds what Check found: current
            // where the file is the text checked, else said not to be.)
            follow_problems(app, &doc);
            app.tilesets.push(doc);
            app.actions.push(Action::OpenTab(Tab::Tileset(id)));
        }
        Err(e) => {
            app.log.error(format!("{}: {e}", path.display()));
            // (A file gone: what Check found of it says so.)
            let source = crate::problems::Source::Tileset(path);
            app.problems.set_stale(&source, Some(crate::problems::Stale::Missing));
        }
    }
}

/// A tileset's file at `path` heard to have changed outside Moonglow (the
/// watch of the open tilesets' files, `outside.rs`; M5's W5, the owner's
/// ruling of 2026-11-21): as a script's file is. Without unsaved changes
/// here the editor shows the file as it is now (`TilesetDoc::take_disk`);
/// with some, Moonglow's are kept and Changed Outside Moonglow asks which
/// to keep (`outside::window`), Save not writing over the file meanwhile.
/// A file that can't be read, or read as a tileset (half written, gone),
/// leaves the editor as it was, and the log says so once. What Check found
/// of it in Problems follows by its fingerprint (`follow_all`, at the next
/// frame's start). `take`: take the file whatever is unsaved here (Take
/// the Files').
pub(crate) fn changed_on_disk(app: &mut Moonglow, path: &Path, take: bool) {
    let Some(at) = app.tilesets.iter().position(|d| d.path == path) else { return };
    let shown = path.display();
    app.tilesets[at].seen = stat(path);
    let data = match std::fs::read(path) {
        Ok(data) => data,
        Err(e) => {
            let d = &mut app.tilesets[at];
            if d.said != Some(u64::MAX) {
                d.said = Some(u64::MAX);
                app.log.warn(format!(
                    "{shown}: changed outside Moonglow, and can't be read now ({e}): the editor \
                     keeps what it has"
                ));
            }
            // (What Check found stays of the text in the editor, which is
            // as it was: `follow_problems`.)
            return;
        }
    };
    let disk = crate::problems::fingerprint(&data);
    let d = &mut app.tilesets[at];
    // (As known: Moonglow's own save, or the file put back as it was. A
    // question waiting about another text has nothing left to ask.)
    if d.disk == Some(disk) {
        d.theirs = None;
        d.said = None;
        return;
    }
    let text = Codepage::default().decode(&data).into_owned();
    let unread = match Tileset::parse(&data, Codepage::default()) {
        Err(e) => Some(e.to_string()),
        Ok(_) => cut_short(&SetFile::parse(&text)),
    };
    if let Some(e) = unread {
        if d.said != Some(disk) {
            d.said = Some(disk);
            app.log.warn(format!(
                "{shown}: changed outside Moonglow, and can't be read as a tileset ({e}): the \
                 editor keeps what it had"
            ));
        }
        return;
    }
    // (The text here already: the file is as saved.)
    if text == d.file.text() {
        d.disk = Some(disk);
        d.theirs = None;
        d.saved = Some(d.undo.len());
        return;
    }
    if d.is_dirty() && !take {
        if d.theirs != Some(disk) {
            d.theirs = Some(disk);
            app.ask_outside_again();
            app.log.warn(format!(
                "{shown} changed outside Moonglow and has unsaved changes here: Moonglow's is \
                 kept (Changed Outside Moonglow asks which to keep)"
            ));
        }
        return;
    }
    d.take_disk(&text, disk);
    app.log.info(format!("{shown} read again (changed outside Moonglow)"));
}

/// Why a tileset's text read from outside looks cut off (written half
/// way, as a program that writes in place leaves it for a moment): no
/// `[TILES]`, or a list whose count names more sections than it has (the
/// game's reader takes such a file without a word).
fn cut_short(file: &SetFile) -> Option<String> {
    if !file.has_section("TILES") {
        return Some("missing section [TILES]".into());
    }
    let lists = [
        ("TERRAIN TYPES", "TERRAIN"),
        ("CROSSER TYPES", "CROSSER"),
        ("PRIMARY RULES", "PRIMARY RULE"),
        ("SECONDARY RULES", "SECONDARY RULE"),
        ("TILES", "TILE"),
        ("GROUPS", "GROUP"),
    ];
    for (list, item) in lists {
        let count = file.get(list, "Count").and_then(|c| c.trim().parse::<usize>().ok());
        if let Some(last) = count.and_then(|n| n.checked_sub(1))
            && !file.has_section(&format!("{item}{last}"))
        {
            return Some(format!("[{list}] counts {} but there is no [{item}{last}]", last + 1));
        }
    }
    None
}

/// Changed Outside Moonglow's answer for the tilesets it asked about:
/// Keep Moonglow's (`mine`) or Take the Files'. The files' names.
/// Where each tileset's editor was drawn: in which pass, layer and rect.
type Areas = HashMap<u32, (u64, egui::LayerId, egui::Rect)>;

/// A file's modified time and length, as `TilesetDoc::seen` keeps them.
fn stat(path: &Path) -> Option<(Option<std::time::SystemTime>, u64)> {
    std::fs::metadata(path).ok().map(|m| (m.modified().ok(), m.len()))
}

/// Whether the file at `path`, open in the tileset editor, may have
/// changed since it was last read or written here (its time or length
/// differ): a look at every file reads only those.
pub(crate) fn may_have_changed(app: &Moonglow, path: &Path) -> bool {
    app.tilesets.iter().find(|d| d.path == path).is_some_and(|d| stat(path) != d.seen)
}

fn areas_id() -> egui::Id {
    egui::Id::new("tileset-editor-areas")
}

/// Whether the text field with the keyboard is one of the editors of the
/// tilesets at `paths` (all where None), by where each editor was drawn
/// in the last pass or this one (not a tab out of sight since): a field of
/// another editor keeps the keyboard when a tileset's file changes
/// outside.
pub(crate) fn typing_in(app: &Moonglow, ctx: &egui::Context, paths: Option<&[PathBuf]>) -> bool {
    let Some(focus) = ctx.memory(|m| m.focused()) else { return false };
    let Some(r) = ctx.read_response(focus) else { return false };
    let areas: Areas = ctx.data(|m| m.get_temp(areas_id())).unwrap_or_default();
    let recent = ctx.cumulative_pass_nr().saturating_sub(1);
    app.tilesets
        .iter()
        .filter(|d| paths.is_none_or(|p| p.contains(&d.path)))
        .filter_map(|d| areas.get(&d.id))
        .any(|(pass, layer, rect)| {
            *pass >= recent && *layer == r.layer_id && rect.intersects(r.rect)
        })
}

pub(crate) fn answered_outside(app: &mut Moonglow, mine: bool) -> Vec<String> {
    let paths: Vec<PathBuf> =
        (app.tilesets.iter()).filter(|d| d.theirs.is_some()).map(|d| d.path.clone()).collect();
    for path in &paths {
        if mine {
            if let Some(d) = app.tilesets.iter_mut().find(|d| d.path == *path) {
                d.keep_mine();
            }
        } else {
            changed_on_disk(app, path, true);
        }
    }
    paths.iter().map(|p| p.display().to_string()).collect()
}

/// The tilesets' files waiting for Changed Outside Moonglow's answer.
pub(crate) fn waiting_outside(app: &Moonglow) -> impl Iterator<Item = &Path> {
    app.tilesets.iter().filter(|d| d.theirs.is_some()).map(|d| d.path.as_path())
}

/// Whether what Check found of `d`'s file in Problems is of its text as it
/// is: current where it is, else said to have changed (by fingerprints:
/// an Undo back to the text checked makes them current again).
fn follow_problems(app: &mut Moonglow, d: &TilesetDoc) {
    let source = crate::problems::Source::Tileset(d.path.clone());
    let Some(checked) = app.problems.of(&source).and_then(|s| s.checked) else { return };
    let why = (d.fingerprint() != checked).then_some(crate::problems::Stale::Changed);
    app.problems.set_stale(&source, why);
}

/// Every open tileset's findings followed at the frame's start, whether its
/// tab is drawn or not (one changed behind another tab, or read again from
/// disk, said stale before Problems draws).
pub(crate) fn follow_all(app: &mut Moonglow) {
    if app.problems.sets().is_empty() {
        return;
    }
    let docs = std::mem::take(&mut app.tilesets);
    for d in &docs {
        follow_problems(app, d);
    }
    app.tilesets = docs;
}

/// A tileset `d` closed (`side_doc::forget_side`): what Check found of it is
/// current where the file on disk is the text checked; else said to be of
/// changes let go of (`dirty`: closed without saving) or of the file as it
/// was.
pub(crate) fn closed(app: &mut Moonglow, path: &Path, dirty: bool) {
    let source = crate::problems::Source::Tileset(path.to_path_buf());
    let Some(checked) = app.problems.of(&source).and_then(|s| s.checked) else { return };
    use crate::problems::Stale;
    let why = match std::fs::read(path) {
        Ok(data) if crate::problems::fingerprint(&data) == checked => None,
        Ok(_) if dirty => Some(Stale::Closed),
        Ok(_) => Some(Stale::Changed),
        Err(_) => Some(Stale::Missing),
    };
    app.problems.set_stale(&source, why);
}

/// Check's findings of tileset `id` recorded as Check records them (for the
/// tests without the game's data).
#[doc(hidden)]
pub fn record_check(app: &mut Moonglow, id: u32, findings: &[mg_module::doctor::Finding]) {
    let Some(at) = app.tilesets.iter().position(|d| d.id == id) else { return };
    let d = app.tilesets.remove(at);
    panels::to_problems(app, &d, findings);
    app.tilesets.insert(at, d);
}

/// The name of the editor's header (the kit's group, the tests').
pub const TILESET_HEADER: &str = "Tileset header";
/// The pages' switch's name.
pub const PAGE: &str = "Page";
/// The header's commands' names.
pub const MAKE_PALETTE: &str = "Make Palette";
pub const CHECK: &str = "Check";
pub const RENDER_MINIMAPS: &str = "Render Minimap Pictures";
/// The bar's commands' names (the bar over the tile list).
pub const DUPLICATE_TILE: &str = "Duplicate Tile";
pub const ADD_TILE: &str = "Add Tile";
pub const REMOVE_LAST_TILE: &str = "Remove Last Tile";
/// The tile list's name (a screen reader's, the tests').
pub const TILES_TABLE: &str = "Tiles table";
/// The name of a tileset's Textures rows (a screen reader's, the tests').
pub const TEXTURE_ROWS: &str = "Texture rows";
/// The tile list's Find field's name.
pub const FIND: &str = "Find";

/// The pages' words and tips, in the switch's order.
const PAGE_TIPS: [(Page, &str); 5] = [
    (Page::General, "The tileset's name, its terrains for new areas, its height step and grass"),
    (Page::Terrain, "The terrains and crossers the tiles name: add and rename them"),
    (Page::Rules, "What the terrain brush changes beside the corners it paints"),
    (Page::Tiles, "Each tile's model, corners, edges, flags and doors"),
    (Page::Groups, "The tiles placed together as one, as the palette lists them"),
];

pub(crate) fn ui(app: &mut Moonglow, ui: &mut Ui, id: u32) {
    let Some(i) = app.tilesets.iter().position(|d| d.id == id) else {
        ui.label("This tileset was closed.");
        return;
    };
    let tab = Tab::Tileset(id);
    // (Where its fields are drawn this frame: `typing_in`.)
    let area = (ui.ctx().cumulative_pass_nr(), ui.layer_id(), ui.max_rect());
    ui.ctx().data_mut(|m| m.get_temp_mut_or_default::<Areas>(areas_id()).insert(id, area));
    // The doc is taken out while drawn (its fields edit through the app's
    // buffers), and put back.
    let mut doc = app.tilesets.remove(i);
    let asked = header(app, ui, &doc);
    if let Some(page) = asked.page {
        doc.page = page;
    }
    if asked.save {
        save_now(app, &mut doc);
    }
    if asked.make_palette {
        ask_make_palette(app, &mut doc);
    }
    if asked.check {
        check(app, &mut doc);
    }
    if asked.render {
        ask_render_minimaps(app, &mut doc);
    }
    // (What Check found, of a text not as it is: said so in Problems.)
    follow_problems(app, &doc);
    app.tilesets.insert(i, doc);
    // OK and Cancel at the header's end, as the other editors have them: a
    // tileset is saved when asked, so OK saves it and closes it, Cancel
    // closes it as last saved (asked about first, as its × asks).
    if let Some(pressed) = asked.pressed
        && crate::side_doc::pressed_ok_cancel(app, &tab, false, pressed, |_| {})
    {
        return;
    }
    // (Why it could not be saved, at the foot.)
    crate::side_doc::failed_foot(app, ui, &tab);
    let Some(i) = app.tilesets.iter().position(|d| d.id == id) else { return };
    let mut doc = app.tilesets.remove(i);
    // (A floating tileset's Outliner folded to a strip at the body's left,
    // where its window has not the room for its pane: `shell::window_panes`.)
    let owner = crate::outliner::Owner::Tileset(id);
    let strip = app.outliner.strips_now.contains(&owner);
    if strip {
        crate::outliner::strip(app, ui, owner);
    }
    draw(app, ui, &mut doc);
    app.tilesets.insert(i, doc);
    if strip {
        crate::outliner::over(app, ui, owner);
    }
}

/// What the header asked for, done after it.
#[derive(Default)]
struct HeaderAsked {
    page: Option<Page>,
    save: bool,
    make_palette: bool,
    check: bool,
    render: bool,
    pressed: Option<crate::blueprint::Pressed>,
}

/// The editor's header (the Tileset board's, `Tileset_fix_hdr2.png` and
/// `Tileset_fix_hdr3.png`): the pages' switch (General, Terrains and
/// Crossers, Painting Rules, Tiles, Groups); a line; Save, the primary
/// command; a line; Make Palette, Check, Render Minimap Pictures; then OK
/// and Cancel at its end. What has no room loses its words, then goes into
/// More, Save and the switch last.
fn header(app: &Moonglow, ui: &mut Ui, d: &TilesetDoc) -> HeaderAsked {
    use mg_uikit::Icon;
    use mg_uikit::header::{EditorHeader, Item};
    use std::cell::Cell;
    let shown = Some(d.page);
    let page = Cell::new(shown);
    let choices: Vec<(Option<Page>, &str, &str)> =
        PAGE_TIPS.iter().map(|(p, tip)| (Some(*p), page_name(*p), *tip)).collect();
    // (Measured as the kit lays it out: each choice its words and the
    // padding at each side, a line between them.)
    let switch_width = {
        let font = egui::TextStyle::Body.resolve(ui.style());
        let words = |w: &str| {
            ui.fonts_mut(|f| f.layout_no_wrap(w.to_string(), font.clone(), egui::Color32::WHITE))
                .size()
                .x
        };
        let pad = 2.0 * mg_uikit::metrics::CONTROL_PADDING_X;
        let widths: f32 = PAGES.iter().map(|p| words(page_name(*p)) + pad).sum();
        widths.ceil() + (PAGES.len() - 1) as f32
    };
    let switch = |ui: &mut Ui| {
        let mut chosen = page.get();
        let room = egui::vec2(switch_width, mg_uikit::metrics::CONTROL_HEIGHT);
        let layout = egui::Layout::left_to_right(egui::Align::Center);
        ui.allocate_ui_with_layout(room, layout, |ui| {
            mg_uikit::segmented(ui, PAGE, &mut chosen, &choices);
        });
        page.set(chosen);
    };
    let readable = d.set.is_ok();
    let file = d.path.file_name().map_or_else(String::new, |n| n.to_string_lossy().into_owned());
    let stem = d.path.file_stem().map(|s| s.to_string_lossy().to_lowercase()).unwrap_or_default();
    let gpu = app.viewport.is_some();
    let render_tip = match gpu {
        true => "Save each tile's minimap picture (ImageMap2D), seen from above, beside the .set",
        false => "Rendering needs a GPU",
    };
    let header = EditorHeader::new(TILESET_HEADER)
        .left(Item::custom(PAGE, switch_width, switch).more(switch_width, switch).primary())
        .left(Item::separator())
        .left(
            Item::button(Icon::Save, "Save")
                .primary()
                .enabled(d.is_dirty())
                .tip(format!("Save the tileset file, {file}")),
        )
        .left(Item::separator())
        .left(Item::button(Icon::Palette, MAKE_PALETTE).enabled(readable).tip(format!(
            "Write {stem}palstd.itp beside the .set: its groups, features and terrains"
        )))
        .left(
            Item::button(Icon::Verify, CHECK)
                .enabled(readable)
                .tip("The content doctor's checks of tilesets: what they find goes to Problems"),
        )
        .left(
            Item::button(Icon::Minimap, RENDER_MINIMAPS).enabled(gpu && readable).tip(render_tip),
        );
    let out =
        crate::widgets::header_with_ok_cancel(ui, header, None, crate::side_doc::OK_CANCEL_TIPS);
    HeaderAsked {
        page: page.get().filter(|p| Some(*p) != shown),
        save: out.clicked("Save"),
        make_palette: out.clicked(MAKE_PALETTE),
        check: out.clicked(CHECK),
        render: out.clicked(RENDER_MINIMAPS),
        pressed: out.end,
    }
}

/// The header's Save: the file written, what it says logged; why not,
/// under the editor too (`side_doc::failed_foot`).
fn save_now(app: &mut Moonglow, d: &mut TilesetDoc) {
    let tab = Tab::Tileset(d.id);
    match d.save() {
        Ok(palette) => {
            app.clear_failed(&tab);
            app.log.info(format!("Saved {}", d.path.display()));
            if let Some(note) = palette {
                app.log.info(note);
            }
        }
        Err(e) => {
            // (Not written over a file changed outside: asked again which
            // to keep, though Later was answered.)
            if d.theirs.is_some() {
                app.ask_outside_again();
            }
            app.note_failed(&tab, e.clone());
            app.log.error(e);
        }
    }
}

/// The page under the header: on Tiles the list and the form; the others
/// scrolled both ways in the editor's room, as the whole window was.
fn draw(app: &mut Moonglow, ui: &mut Ui, d: &mut TilesetDoc) {
    if let Err(e) = &d.set {
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.add_space(12.0);
            ui.colored_label(
                ui.visuals().error_fg_color,
                format!("The file can't be read as a tileset: {e}"),
            );
        });
        return;
    }
    if d.page == Page::Tiles {
        tiles(app, ui, d);
        return;
    }
    d.hovered = false;
    let page = egui::ScrollArea::both().id_salt(("set-editor", d.id, d.page)).auto_shrink(false);
    page.show(ui, |ui| {
        egui::Frame::new().inner_margin(egui::Margin::symmetric(12, 10)).show(ui, |ui| {
            match d.page {
                Page::General => general(app, ui, d),
                Page::Terrain => terrain(ui, d),
                Page::Rules => rules(ui, d),
                Page::Groups => groups(app, ui, d),
                Page::Tiles => {}
            }
        });
    });
}

/// Gives the keyboard to `section`'s `key` field (`id`) where a finding's
/// place asked for it, and brings it into sight.
fn focus_asked(ui: &Ui, d: &mut TilesetDoc, section: &str, key: &str, id: egui::Id) -> bool {
    let asked = d
        .focus_field
        .as_ref()
        .is_some_and(|(s, k)| s.eq_ignore_ascii_case(section) && k.eq_ignore_ascii_case(key));
    if !asked {
        return false;
    }
    // (Not in the frame of the click that asked: egui lets go of the
    // focus at a click elsewhere as the frame ends. The next one, asked
    // for.)
    if ui.input(|i| i.pointer.any_click() || i.pointer.any_pressed() || i.pointer.any_released()) {
        ui.ctx().request_repaint();
        return false;
    }
    d.focus_field = None;
    ui.memory_mut(|m| m.request_focus(id));
    true
}

/// A text value of a section, committed when focus leaves the field.
fn text_key(
    app: &mut Moonglow,
    ui: &mut Ui,
    d: &mut TilesetDoc,
    section: &str,
    key: &str,
    width: f32,
) {
    let current = d.get(section, key).unwrap_or_default();
    let id = egui::Id::new(("set", d.id, section, key));
    let focus = focus_asked(ui, d, section, key, id);
    if let Some(v) = crate::widgets::commit_text(app, ui, id, &current, false, width) {
        d.set_key(section, key, v.trim());
    }
    if focus {
        ui.scroll_to_cursor(Some(egui::Align::Center));
    }
}

fn flag_key(ui: &mut Ui, d: &mut TilesetDoc, section: &str, key: &str, label: &str) {
    let mut on = d.get(section, key).and_then(|v| v.trim().parse::<i32>().ok()).unwrap_or(0) != 0;
    if ui.check_box(&mut on, label).changed() {
        d.set_key(section, key, if on { "1" } else { "0" });
    }
}

fn int_key(
    ui: &mut Ui,
    d: &mut TilesetDoc,
    section: &str,
    key: &str,
    range: std::ops::RangeInclusive<i32>,
) {
    let current = d.get(section, key).and_then(|v| v.trim().parse::<i32>().ok()).unwrap_or(0);
    if let Some(v) = crate::widgets::commit_number(ui, current, range) {
        d.set_key(section, key, &v.to_string());
    }
}

/// A terrain or crosser name chosen from `options` (`TilesetDoc::type_options`).
fn name_key(
    ui: &mut Ui,
    d: &mut TilesetDoc,
    section: &str,
    key: &str,
    options: &[String],
    none: bool,
) {
    name_key_width(ui, d, section, key, options, none, None);
}

/// [`name_key`], its box `width` wide where given. `options` is built by
/// [`TilesetDoc::type_options`] (already prefixed with `""` when `none` is set).
fn name_key_width(
    ui: &mut Ui,
    d: &mut TilesetDoc,
    section: &str,
    key: &str,
    options: &[String],
    none: bool,
    width: Option<f32>,
) {
    let current = d.get(section, key).unwrap_or_default();
    // (The tileset's own name for the one set, as it may be written in
    // another case.)
    let named = options.iter().skip(usize::from(none)).find(|n| n.eq_ignore_ascii_case(&current));
    let was = named.unwrap_or(&current);
    let mut pick = was.clone();
    let mut dropdown = crate::widgets::dropdown(("set-name", d.id, section, key));
    if let Some(w) = width {
        dropdown = dropdown.width(w);
    }
    dropdown.selected_text(if current.is_empty() { "(none)" } else { &current }).show(
        ui,
        &mut pick,
        options,
        |n| {
            if n.is_empty() { "(none)".to_string() } else { n.clone() }
        },
    );
    if pick != *was && pick != current {
        d.set_key(section, key, &pick);
    }
}

fn general(app: &mut Moonglow, ui: &mut Ui, d: &mut TilesetDoc) {
    let terrains = d.type_options(false, false);
    egui::Grid::new(("set-general", d.id)).num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
        for (key, label) in [
            ("Name", "Name"),
            ("UnlocalizedName", "Name shown (no talk-table string)"),
            ("EnvMap", "Environment map"),
        ] {
            crate::widgets::field_label(ui, label);
            text_key(app, ui, d, "GENERAL", key, 220.0);
            ui.end_row();
        }
        crate::widgets::field_label(ui, "Name's talk-table string");
        int_key(ui, d, "GENERAL", "DisplayName", -1..=i32::MAX);
        ui.end_row();
        crate::widgets::field_label(ui, "Height step (m)");
        text_key(app, ui, d, "GENERAL", "Transition", 80.0);
        ui.end_row();
        crate::widgets::field_label(ui, "Selector height (m)");
        text_key(app, ui, d, "GENERAL", "SelectorHeight", 80.0);
        ui.end_row();
        for (key, label) in [
            ("Border", "Border terrain"),
            ("Default", "New areas' terrain"),
            ("Floor", "Floor terrain"),
        ] {
            crate::widgets::field_label(ui, label);
            name_key(ui, d, "GENERAL", key, &terrains, false);
            ui.end_row();
        }
    });
    flag_key(ui, d, "GENERAL", "Interior", "Interior");
    flag_key(ui, d, "GENERAL", "HasHeightTransition", "Height transitions (raise and lower)");
    ui.add_space(crate::widgets::SECTION_GAP);
    crate::widgets::section_heading(ui, "Grass");
    flag_key(ui, d, "GRASS", "Grass", "Grass on tiles' grass faces");
    egui::Grid::new(("set-grass", d.id)).num_columns(2).show(ui, |ui| {
        for (key, label) in
            [("GrassTextureName", "Texture"), ("Density", "Density"), ("Height", "Height")]
        {
            crate::widgets::field_label(ui, label);
            text_key(app, ui, d, "GRASS", key, 160.0);
            ui.end_row();
        }
    });
}

/// A page's name, as its tab shows it (and a session keeps it).
pub(crate) fn page_name(page: Page) -> &'static str {
    match page {
        Page::General => "General",
        Page::Terrain => "Terrains and Crossers",
        Page::Rules => "Painting Rules",
        Page::Tiles => "Tiles",
        Page::Groups => "Groups",
    }
}

/// Why tile `i` is a hole, as the game reads the tileset (it draws such a
/// tile as nothing, with no walkmesh; `engine_tileset_holes`): no section
/// of its own, or no model in it. `None` for a whole tile.
pub fn tile_hole(t: &Tileset, i: usize) -> Option<String> {
    let section = format!("TILE{i}");
    if t.missing_sections.contains(&section) {
        Some(format!("no [{section}] section"))
    } else if t.tiles.get(i).is_some_and(|x| x.model.trim().is_empty()) {
        Some("no Model=".into())
    } else {
        None
    }
}

/// The holes of the lists whose sections begin with one of `prefixes`
/// (`TERRAIN`, `PRIMARY RULE`), said under a page: the game leaves each
/// out, and the numbers the page gives count the entries there are.
fn list_holes(ui: &mut Ui, t: &Tileset, prefixes: &[&str]) {
    let gone: Vec<&String> = (t.missing_sections.iter())
        .filter(|s| {
            prefixes.iter().any(|p| {
                s.strip_prefix(p)
                    .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
            })
        })
        .collect();
    if gone.is_empty() {
        return;
    }
    let names: Vec<String> = gone.iter().take(12).map(|s| format!("[{s}]")).collect();
    let more =
        if gone.len() > 12 { format!(" and {} more", gone.len() - 12) } else { String::new() };
    ui.add_space(crate::widgets::SECTION_GAP);
    ui.add(
        egui::Label::new(egui::RichText::new(format!(
            "Holes: the file has no {}{more}. The game leaves them out, and the numbers here count \
             the entries the file has.",
            names.join(", ")
        ))
        .color(ui.visuals().warn_fg_color))
        .wrap(),
    );
}

fn terrain(ui: &mut Ui, d: &mut TilesetDoc) {
    let Ok(t) = d.set.clone() else { return };
    list_holes(ui, &t, &["TERRAIN", "CROSSER"]);
    ui.columns(2, |cols| {
        for (col, crosser) in [(0, false), (1, true)] {
            let ui = &mut cols[col];
            crate::widgets::section_heading(ui, if crosser { "Crossers" } else { "Terrains" });
            let list = if crosser { &t.crossers } else { &t.terrains };
            mg_uikit::list_rows(ui);
            for (i, x) in list.iter().enumerate() {
                let label = if x.strref.is_none() {
                    x.unlocalized_name.clone().unwrap_or_default()
                } else {
                    format!("string {}", x.strref.0)
                };
                ui.horizontal(|ui| {
                    ui.monospace(format!("{i:>3}"));
                    match &mut d.renaming {
                        Some((c, k, typed)) if *c == crosser && *k == i => {
                            let r = ui.add(mg_uikit::text_field(typed).desired_width(120.0));
                            let why = rename_refused(&t, crosser, i, typed);
                            let ok = why.is_none();
                            let enter =
                                r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                            let renamed = ui
                                .add_enabled(ok, egui::Button::new("Rename"))
                                .on_hover_text(
                                    "Rename it in every tile, painting rule and general terrain",
                                )
                                .clicked();
                            if (renamed || enter) && ok {
                                let to = typed.trim().to_string();
                                d.change(|f| {
                                    edit::rename_type(f, crosser, i, &to);
                                });
                                d.renaming = None;
                            } else if ui.button("Cancel").clicked() {
                                d.renaming = None;
                            } else if let Some(why) = why {
                                ui.weak(why);
                            }
                        }
                        _ => {
                            ui.label(&x.name);
                            ui.weak(label);
                            let what = if crosser { "crosser" } else { "terrain" };
                            if ui
                                .small_button("Rename…")
                                .on_hover_text(format!(
                                    "Rename this {what}: every tile naming it follows"
                                ))
                                .clicked()
                            {
                                d.renaming = Some((crosser, i, x.name.clone()));
                            }
                        }
                    }
                });
            }
        }
    });
    ui.separator();
    ui.horizontal(|ui| {
        ui.label("Name");
        ui.add(mg_uikit::text_field(&mut d.new_type).desired_width(160.0));
        let name = d.new_type.trim().to_string();
        let refused = type_name_refused(&t, None, &d.new_type);
        let ok = refused.is_none();
        if ui.add_enabled(ok, egui::Button::new("Add Terrain")).clicked() {
            d.change(|f| {
                edit::add_type(f, false, &name, None);
            });
            d.new_type.clear();
        }
        if ui.add_enabled(ok, egui::Button::new("Add Crosser")).clicked() {
            d.change(|f| {
                edit::add_type(f, true, &name, None);
            });
            d.new_type.clear();
        }
        if !name.is_empty()
            && let Some(why) = refused
        {
            ui.weak(format!("{why}."));
        }
    });
}

/// Why adding or renaming a terrain (`crosser`: crosser) `skip` to `typed`
/// is refused: empty, whitespace, longer than 16 characters (palettes store it
/// as a ResRef), or collides case-insensitively with another terrain/crosser.
fn type_name_refused(
    t: &Tileset,
    skip: Option<(bool, usize)>,
    typed: &str,
) -> Option<&'static str> {
    let name = typed.trim();
    let other = |c: bool, k: usize, x: &mg_set::NamedType| {
        skip != Some((c, k)) && x.name.eq_ignore_ascii_case(name)
    };
    let taken = t.terrains.iter().enumerate().any(|(k, x)| other(false, k, x))
        || t.crossers.iter().enumerate().any(|(k, x)| other(true, k, x));
    if name.is_empty() {
        Some("A name, please")
    } else if name.contains(char::is_whitespace) {
        Some("No spaces: the file's lines name it")
    } else if name.len() > 16 {
        Some("16 characters at most: a palette names it so")
    } else if taken {
        Some("The tileset has one of that name")
    } else {
        None
    }
}

/// Why a terrain (`crosser`: crosser) `index` can't be renamed `typed`:
/// no name, a space, more than a palette's 16 characters (a palette names
/// it as a ResRef), or another terrain's or crosser's name; `None` when
/// it can.
fn rename_refused(t: &Tileset, crosser: bool, index: usize, typed: &str) -> Option<&'static str> {
    type_name_refused(t, Some((crosser, index)), typed)
}

/// The painting rules, primary and secondary: what the area view's
/// terrain brush changes beside the corners it paints.
fn rules(ui: &mut Ui, d: &mut TilesetDoc) {
    // (Scrolled: a game tileset's are a hundred or more.)
    egui::ScrollArea::vertical()
        .id_salt(("set-rules", d.id))
        .auto_shrink([false, false])
        .show(ui, |ui| rules_in(ui, d));
}

fn rule_row(ui: &mut Ui, cols: [f32; 7], row_h: f32, mut cell: impl FnMut(&mut Ui, usize)) {
    let gap_x = 10.0;
    let total_w: f32 = cols.iter().sum::<f32>() + gap_x * (cols.len() - 1) as f32;
    ui.allocate_ui_with_layout(
        egui::vec2(total_w, row_h),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            ui.spacing_mut().item_spacing.x = gap_x;
            for (col, w) in cols.into_iter().enumerate() {
                ui.allocate_ui_with_layout(
                    egui::vec2(w, row_h),
                    egui::Layout::left_to_right(egui::Align::Center),
                    |ui| cell(ui, col),
                );
            }
        },
    );
}

fn rules_in(ui: &mut Ui, d: &mut TilesetDoc) {
    const KEYS: [&str; 3] = ["Placed", "Adjacent", "Changed"];
    const HEIGHT_KEYS: [&str; 3] = ["PlacedHeight", "AdjacentHeight", "ChangedHeight"];
    const HEADS: [&str; 7] = ["", "Placed", "height", "Adjacent", "height", "Changed", "height"];
    let Ok(t) = d.set.clone() else { return };
    let terrains = d.type_options(false, false);
    let a11y = ui.ctx().accesskit_node_builder(ui.id(), |_| ()).is_some();
    let note = "Painting a corner Placed beside one Adjacent turns that one into Changed, each at \
                its height. The terrain brush follows the primary rules.";
    ui.add(egui::Label::new(egui::RichText::new(note).weak()).wrap());
    list_holes(ui, &t, &["PRIMARY RULE", "SECONDARY RULE"]);
    for secondary in [false, true] {
        let (_, each) = edit::rule_sections(secondary);
        let list = if secondary { &t.secondary_rules } else { &t.primary_rules };
        ui.add_space(crate::widgets::SECTION_GAP);
        crate::widgets::section_heading(
            ui,
            if secondary { "Secondary rules" } else { "Primary rules" },
        );
        if list.is_empty() {
            mg_uikit::with_table_rows(ui, |ui| {
                let row_h = ui.spacing().interact_size.y;
                let combo_w = ui.spacing().combo_width;
                let cols = [32.0, combo_w, 48.0, combo_w, 48.0, combo_w, 48.0];
                rule_row(ui, cols, row_h, |ui, col| {
                    crate::widgets::field_label(ui, HEADS[col]);
                });
            });
            ui.weak("None.");
        } else {
            mg_uikit::with_table_rows(ui, |ui| {
                let gap = mg_uikit::metrics::TABLE_RULE;
                ui.spacing_mut().item_spacing.y = gap;
                let row_h = ui.spacing().interact_size.y;
                let combo_w = ui.spacing().combo_width;
                let cols = [32.0, combo_w, 48.0, combo_w, 48.0, combo_w, 48.0];
                let total_w: f32 = cols.iter().sum::<f32>() + 10.0 * (cols.len() - 1) as f32;
                let pitch = row_h + gap;
                rule_row(ui, cols, row_h, |ui, col| {
                    crate::widgets::field_label(ui, HEADS[col]);
                });
                let top_y = ui.cursor().top();
                let (first, last) = if a11y {
                    (0, list.len())
                } else {
                    let clip = ui.clip_rect();
                    let first = ((clip.top() - top_y) / pitch).floor().max(0.0) as usize;
                    let last = (((clip.bottom() - top_y) / pitch).ceil().max(0.0) as usize + 1)
                        .min(list.len());
                    (first.min(last), last)
                };
                if first == last {
                    ui.allocate_space(egui::vec2(total_w, list.len() as f32 * pitch - gap));
                } else {
                    if first > 0 {
                        ui.allocate_space(egui::vec2(total_w, first as f32 * pitch - gap));
                    }
                    for i in first..last {
                        let section = format!("{each}{i}");
                        rule_row(ui, cols, row_h, |ui, col| match col {
                            0 => {
                                ui.monospace(format!("{i:>3}"));
                            }
                            1 | 3 | 5 => {
                                name_key_width(
                                    ui,
                                    d,
                                    &section,
                                    KEYS[(col - 1) / 2],
                                    &terrains,
                                    false,
                                    Some(cols[col]),
                                );
                            }
                            _ => {
                                int_key(ui, d, &section, HEIGHT_KEYS[(col - 2) / 2], -1..=4);
                            }
                        });
                    }
                    let after = list.len() - last;
                    if after > 0 {
                        ui.allocate_space(egui::vec2(total_w, after as f32 * pitch - gap));
                    }
                }
            });
        }
        ui.horizontal(|ui| {
            let first = terrains.first().cloned().unwrap_or_default();
            if ui
                .add_enabled(!terrains.is_empty(), egui::Button::new("Add Rule"))
                .on_hover_text("A rule at the end, each terrain the first, at height 0")
                .on_disabled_hover_text("The tileset has no terrains to name")
                .clicked()
            {
                d.change(|f| {
                    edit::add_rule(f, secondary, &first, &first, &first);
                });
            }
            if ui.add_enabled(!list.is_empty(), egui::Button::new("Remove Last Rule")).clicked() {
                d.change(|f| {
                    edit::remove_last_rule(f, secondary);
                });
            }
        });
    }
}

/// The chosen tile's door hooks, under its form (the Tileset board's
/// `.doors`): where a door placed on the tile stands, of which
/// `doortypes.2da` row; the table's header row, a row a door (its number,
/// Type, the row's label, X, Y, Z and Orientation), Add Door and Remove
/// Last Door under it.
fn door_hooks(app: &Moonglow, ui: &mut Ui, d: &mut TilesetDoc, i: usize) {
    use crate::table_view::look;
    crate::widgets::section_heading(ui, "Doors");
    let doors = d.doors(i);
    let types = app.game.as_deref().and_then(|g| g.table("doortypes").ok());
    let t = mg_uikit::tokens(ui.ctx());
    // (The board's columns: 26, 52, 62, 64, 64, 64 and the rest.)
    let width = ui.available_width();
    let fixed = [26.0, 52.0, 62.0, 64.0, 64.0, 64.0];
    let heads = ["", "Type", "", "X", "Y", "Z", "Orientation (°)"];
    let font = egui::TextStyle::Body.resolve(ui.style());
    // (The last column keeps its heading whole, "Orientation (°)", its
    // padding 8 each side, the board's; where the form is narrower than
    // the board's columns, the others narrow alike: the table never runs
    // past the form's edge.)
    let heading = ui
        .fonts_mut(|f| f.layout_no_wrap(heads[6].to_string(), font.clone(), egui::Color32::WHITE))
        .size()
        .x;
    let least = (heading + 16.0).ceil();
    let last = (width - fixed.iter().sum::<f32>()).max(least);
    let scale = ((width - last) / fixed.iter().sum::<f32>()).clamp(0.4, 1.0);
    let widths: Vec<f32> =
        (fixed.iter().map(|w| (w * scale).floor())).chain(std::iter::once(last.floor())).collect();
    let high = look::HEADER_HIGH;
    let (head, _) =
        ui.allocate_exact_size(egui::vec2(widths.iter().sum(), high), egui::Sense::hover());
    let mut x = head.left();
    for (at, (w, words)) in widths.iter().zip(heads).enumerate() {
        // (Type's heading over its number and the row's label, the label's
        // column having none: "Type" whole where the form is narrow.)
        if at == 2 {
            continue;
        }
        let w = if at == 1 { w + widths[2] } else { *w };
        let cell = egui::Rect::from_x_y_ranges(x..=x + w, head.y_range());
        look::header_cell(ui, cell);
        let inner = cell.shrink2(egui::vec2(8.0, 0.0));
        let mut job =
            egui::text::LayoutJob::simple_singleline(words.to_string(), font.clone(), t.text_2);
        job.wrap = egui::text::TextWrapping::truncate_at_width(inner.width().max(1.0));
        let galley = ui.painter().layout_job(job);
        let at = egui::pos2(inner.left(), cell.center().y - galley.size().y / 2.0);
        ui.painter().with_clip_rect(cell).galley(at, galley, t.text_2);
        x += w;
    }
    let row = mg_uikit::metrics::CONTROL_HEIGHT + 4.0;
    for (n, section) in doors.iter() {
        let (rect, _) =
            ui.allocate_exact_size(egui::vec2(widths.iter().sum(), row), egui::Sense::hover());
        ui.painter().hline(
            rect.x_range(),
            rect.bottom() - 0.5,
            egui::Stroke::new(1.0, t.cell_line),
        );
        let mut x = rect.left();
        let mut cells = Vec::new();
        for w in &widths {
            cells.push(
                egui::Rect::from_x_y_ranges(x..=x + w, rect.y_range())
                    .shrink2(egui::vec2(3.0, 2.0)),
            );
            x += w;
        }
        let mut cell = |c: usize| {
            let layout = egui::Layout::left_to_right(egui::Align::Center);
            ui.new_child(
                egui::UiBuilder::new().max_rect(cells[c]).layout(layout).id_salt((section, c)),
            )
        };
        cell(0).label(egui::RichText::new(n.to_string()).monospace().color(t.text_3));
        {
            let mut ui = cell(1);
            ui.spacing_mut().interact_size.x = cells[1].width();
            int_key(&mut ui, d, section, "Type", 0..=i32::MAX);
        }
        // (Its row's label, where the game data has the table.)
        let row = d.get(section, "Type").and_then(|v| v.trim().parse::<usize>().ok());
        let label = row.zip(types.as_ref()).and_then(|(r, t)| t.get(r, "Label"));
        cell(2).add(
            egui::Label::new(egui::RichText::new(label.unwrap_or_default()).color(t.text_3))
                .truncate(),
        );
        for (c, key) in [(3, "X"), (4, "Y"), (5, "Z"), (6, "Orientation")] {
            let mut ui = cell(c);
            ui.spacing_mut().interact_size.x = cells[c].width();
            float_key(&mut ui, d, section, key);
        }
    }
    if doors.is_empty() {
        ui.add_space(4.0);
        ui.weak("None: a door can't be placed on this tile.");
    }
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        let add = mg_uikit::KitButton::new(Some(mg_uikit::Icon::Plus), "Add Door")
            .tip("Add a door hook at the tile's middle, doortypes.2da row 0");
        if ui.add(add).clicked() {
            d.change(|f| {
                edit::add_door(f, i, 0);
            });
        }
        let remove = mg_uikit::KitButton::new(None, "Remove Last Door")
            .tip("Take the tile's last door hook away");
        if ui.add_enabled(!doors.is_empty(), remove).clicked() {
            d.change(|f| {
                edit::remove_last_door(f, i);
            });
        }
    });
}

/// A number with a fraction (meters, degrees), committed when it changes
/// (a drag: when released), written as the game's files write one
/// (`5.0`).
fn float_key(ui: &mut Ui, d: &mut TilesetDoc, section: &str, key: &str) {
    let current = d.get(section, key).and_then(|v| v.trim().parse::<f32>().ok()).unwrap_or(0.0);
    // (With a fraction always, as the file writes one: "3.0".)
    let range = -10_000.0..=10_000.0;
    if let Some(v) = crate::widgets::drag_number(ui, current, |d| d.range(range).min_decimals(1)) {
        let mut text = v.to_string();
        if !text.contains('.') {
            text.push_str(".0");
        }
        d.set_key(section, key, &text);
    }
}

/// The tile list's width beside the form where the editor is wide enough
/// for both (the board's 548), and the least the form keeps beside it.
const LIST_WIDTH: f32 = 548.0;
const FORM_LEAST: f32 = 380.0;
/// The list's columns' widths: #, Model and Edges (the board's 36, 108 and
/// 134), Corners the rest. The number's padding is narrower than the
/// others' (2 and 6, against 8), so that three digits are whole in the
/// board's 36 (tcn01's 408 tiles).
const NUMBER_WIDTH: f32 = 36.0;
const MODEL_WIDTH: f32 = 108.0;
const EDGES_WIDTH: f32 = 134.0;

/// The Tiles page: the bar over the list (Find, Duplicate Tile, Add Tile,
/// Remove Last Tile) and the list at the left, the chosen tile's form and
/// its doors at the right (the Tileset board's); under each other where
/// the editor is narrower than both (decided by the editor's own width,
/// never by a width a scroll bar takes).
fn tiles(app: &mut Moonglow, ui: &mut Ui, d: &mut TilesetDoc) {
    let Some(t) = d.shared() else { return };
    let room = ui.available_rect_before_wrap();
    // (Find's keys go to its field while the pointer is over the page, as
    // they did over the whole window.)
    d.hovered = ui.rect_contains_pointer(room);
    let beside = room.width() >= FORM_LEAST + 300.0;
    let t2 = mg_uikit::tokens(ui.ctx());
    let (list, form) = if beside {
        let list_width = LIST_WIDTH.min(room.width() - FORM_LEAST).floor();
        let list = room.with_max_x(room.left() + list_width);
        let form = room.with_min_x(list.right() + 1.0);
        ui.painter().vline(list.right() + 0.5, room.y_range(), egui::Stroke::new(1.0, t2.line));
        (list, form)
    } else {
        // (The list's bar and a few rows over the form.)
        let high = (room.height() * 0.42).clamp(140.0, 360.0).min(room.height());
        let list = room.with_max_y(room.top() + high);
        let form = room.with_min_y(list.bottom() + 1.0);
        ui.painter().hline(room.x_range(), list.bottom() + 0.5, egui::Stroke::new(1.0, t2.line));
        (list, form)
    };
    ui.advance_cursor_after_rect(room);
    let mut left = ui.new_child(egui::UiBuilder::new().max_rect(list).id_salt("tile-list"));
    left.set_clip_rect(list.intersect(ui.clip_rect()));
    // (What the bar asks is done after the list and the form are drawn,
    // so that both are drawn from one reading of the file: the list's rows
    // made of a reading the bar's change had made stale indexed past the
    // tiles' end. A field typed in sets its text before the press acts
    // whatever the order: `widgets::text_begin`, tested.)
    let asked = tile_bar(&mut left, d, &t);
    // (Down out of Find: the list has the keys, at its first row.)
    if asked.down {
        take_keys(ui.ctx(), d.id);
        if let Some(&first) = d.rows().first() {
            d.tile = Some(first);
            // (Brought into sight, though it was chosen already.)
            d.scrolled_to = None;
        }
    }
    tile_table(app, &mut left, d, &t);
    let mut right = ui.new_child(egui::UiBuilder::new().max_rect(form).id_salt("tile-form"));
    right.set_clip_rect(form.intersect(ui.clip_rect()));
    match d.tile.filter(|&i| i < t.tiles.len()) {
        Some(i) => {
            egui::ScrollArea::vertical()
                .id_salt(("set-tile-form", d.id))
                .auto_shrink([false, false])
                .show(&mut right, |ui| {
                    let margin = egui::Margin { left: 14, right: 12, top: 10, bottom: 12 };
                    egui::Frame::new()
                        .inner_margin(margin)
                        .show(ui, |ui| tile_form(app, ui, d, &t, i));
                });
        }
        None => {
            right.add_space(10.0);
            right.horizontal(|ui| {
                ui.add_space(14.0);
                ui.weak("Choose a tile to edit it.");
            });
        }
    }
    asked.run(d, &t);
}

/// What the bar over the tile list asked for ([`tile_bar`]).
#[derive(Default)]
struct BarAsked {
    duplicate: bool,
    add: bool,
    remove: bool,
    /// Down in Find: on to the tile list, at its first row.
    down: bool,
}

impl BarAsked {
    /// Done (after the list and the form are drawn), each one Undo step:
    /// Duplicate Tile and Add Tile choose the
    /// tile they add; Remove Last Tile lets go of the tile it takes away,
    /// where it was chosen.
    fn run(self, d: &mut TilesetDoc, t: &Tileset) {
        if self.duplicate
            && let Some(s) = d.tile
        {
            let mut new = None;
            d.change(|f| new = edit::duplicate_tile(f, s));
            d.tile = new.or(d.tile);
        }
        if self.add {
            let model = format!("{}_a01_{:02}", t.general.name.to_lowercase(), t.tiles.len() + 1);
            let terrain = t.terrains.first().map(|x| x.name.clone()).unwrap_or_default();
            let mut new = 0;
            d.change(|f| new = edit::add_tile(f, &model, &terrain));
            d.tile = Some(new);
        }
        if self.remove {
            d.change(|f| {
                edit::remove_last_tile(f);
            });
            let left = d.tileset().map_or(0, |t| t.tiles.len());
            d.tile = d.tile.filter(|&i| i < left);
        }
    }
}

/// The bar over the tile list (`.panel-bar`): Find, filling the room the
/// buttons leave; Duplicate Tile, Add Tile and Remove Last Tile (its icon
/// alone, the board's minus). Narrower, the buttons lose their words.
fn tile_bar(ui: &mut Ui, d: &mut TilesetDoc, t: &Tileset) -> BarAsked {
    use mg_uikit::{Icon, KitButton};
    let t2 = mg_uikit::tokens(ui.ctx());
    let bar = egui::vec2(ui.available_width(), mg_uikit::metrics::CONTROL_HEIGHT + 8.0);
    let (rect, _) = ui.allocate_exact_size(bar, egui::Sense::hover());
    ui.painter().hline(rect.x_range(), rect.bottom() - 0.5, egui::Stroke::new(1.0, t2.line_soft));
    let inner = rect.shrink2(egui::vec2(8.0, 4.0));
    let layout = egui::Layout::left_to_right(egui::Align::Center);
    let mut ui = ui.new_child(egui::UiBuilder::new().max_rect(inner).layout(layout));
    ui.spacing_mut().item_spacing.x = 4.0;
    let sel = d.tile;
    let words = |icon_only: bool| {
        [
            KitButton::new(Some(Icon::Copy), DUPLICATE_TILE)
                .icon_only(icon_only)
                .tip("Add a copy at the end, since areas store tiles by number"),
            KitButton::new(Some(Icon::Plus), ADD_TILE)
                .icon_only(icon_only)
                .tip("Add a tile at the end, its model named after the tileset"),
            KitButton::new(Some(Icon::Minus), REMOVE_LAST_TILE)
                .icon_only(true)
                .look(mg_uikit::ButtonLook::Flat)
                .tip("Take the last tile away"),
        ]
    };
    let width = |b: &[KitButton; 3]| b.iter().map(|b| b.width(&ui)).sum::<f32>() + 3.0 * 4.0;
    const FIND_LEAST: f32 = 120.0;
    let icon_only = width(&words(false)) + FIND_LEAST > inner.width();
    let buttons = words(icon_only);
    let find = (inner.width() - width(&buttons)).max(60.0);
    let field = egui::Id::new(("tileset-filter-field", d.id));
    let left = crate::widgets::SearchField::new(field, &mut d.filter)
        .name(FIND)
        .hint("Model or terrain")
        .width(find - 24.0)
        .find_when(d.hovered)
        .down()
        .show(&mut ui)
        .left;
    let [duplicate, add, remove] = buttons;
    BarAsked {
        down: left == Some(crate::widgets::FilterLeft::Down),
        duplicate: ui.add_enabled(sel.is_some(), duplicate).clicked(),
        add: ui.add(add).clicked(),
        remove: ui.add_enabled(!t.tiles.is_empty(), remove).clicked(),
    }
}

/// The tileset whose tile list was given the keys last.
fn keys_of(ctx: &egui::Context) -> Option<u32> {
    ctx.data(|d| d.get_temp(egui::Id::new("tileset-keys-of")))
}

/// The keys given to tileset `id`'s tile list (a click on a row, F6).
pub(crate) fn take_keys(ctx: &egui::Context, id: u32) {
    crate::palette_view::give_keys(ctx, crate::palette_view::Keys::Tiles);
    ctx.data_mut(|d| d.insert_temp(egui::Id::new("tileset-keys-of"), id));
}

/// Whether tileset `id`'s tile list has the keys: they are a tile list's,
/// its, its editor is the active one, and nothing modal or a popover is in
/// front (as a 2DA's grid's, `table_view::grid_has_keys`).
pub(crate) fn list_has_keys(app: &Moonglow, ctx: &egui::Context, id: u32) -> bool {
    crate::palette_view::keys_are(ctx, crate::palette_view::Keys::Tiles)
        && keys_of(ctx) == Some(id)
        && crate::table_view::active_tab(app).as_ref() == Some(&Tab::Tileset(id))
        && !crate::shell::keys_blocked(ctx)
        && !mg_uikit::popover::Popover::any_open(ctx)
}

/// The tile list as a table (the Tileset board's `.tile-list .table`, on
/// the 2DA grid's look, `table_view::look`): a header row (#, Model,
/// Corners, Edges) over the tiles, each 22 high, only those in sight laid
/// out; the number quiet at its right, the model and the edges monospace,
/// a hole's words in `warn`; the chosen tile filled (ringed while the list
/// has the keys). A click chooses a tile and gives the list the keys: Up,
/// Down, Page Up, Page Down, Home and End, while the tileset is the active
/// editor (Undo and Redo its own then), the list outlined meanwhile; Down in
/// Find goes to its first tile. (Enter does nothing: choosing a tile shows
/// it in the form.)
fn tile_table(app: &mut Moonglow, ui: &mut Ui, d: &mut TilesetDoc, t: &Tileset) {
    use crate::table_view::look;
    let ctx = ui.ctx().clone();
    let has = list_has_keys(app, &ctx, d.id);
    if has {
        app.undo_keys = Some(Tab::Tileset(d.id));
    }
    let rows = d.rows();
    let tokens = mg_uikit::tokens(&ctx);
    mg_uikit::name_group(ui, TILES_TABLE);
    if has && !ctx.egui_wants_keyboard_input() && !rows.is_empty() {
        use egui::Key;
        let last = rows.len() as isize - 1;
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
            let at = d.tile.and_then(|s| rows.iter().position(|r| *r == s));
            let to = at.map_or(0, |at| (at as isize).saturating_add(by).clamp(0, last)) as usize;
            d.tile = Some(rows[to]);
        }
    }
    let width = ui.available_width();
    let (head, _) =
        ui.allocate_exact_size(egui::vec2(width, look::HEADER_HIGH), egui::Sense::hover());
    let corners_width = (width - NUMBER_WIDTH - MODEL_WIDTH - EDGES_WIDTH).max(60.0);
    let columns = |left: f32| {
        let mut x = left;
        [NUMBER_WIDTH, MODEL_WIDTH, corners_width, EDGES_WIDTH].map(|w| {
            let r = x..=x + w;
            x += w;
            r
        })
    };
    let font = egui::TextStyle::Body.resolve(ui.style());
    for (i, (x, words)) in
        columns(head.left()).into_iter().zip(["#", "Model", "Corners", "Edges"]).enumerate()
    {
        let cell = egui::Rect::from_x_y_ranges(x, head.y_range());
        look::header_cell(ui, cell);
        let inner = match i {
            0 => cell.with_min_x(cell.left() + 2.0).with_max_x(cell.right() - 6.0),
            _ => cell.shrink2(egui::vec2(8.0, 0.0)),
        };
        let galley =
            ui.painter().layout(words.to_string(), font.clone(), tokens.text_2, inner.width());
        let x = match i {
            0 => inner.right() - galley.size().x,
            _ => inner.left(),
        };
        let at = egui::pos2(x, cell.center().y - galley.size().y / 2.0);
        ui.painter().with_clip_rect(cell).galley(at, galley, tokens.text_2);
    }
    let pitch = mg_uikit::row_metrics(ui).1;
    let mut scroll =
        egui::ScrollArea::vertical().id_salt(("set-tiles", d.id)).auto_shrink([false, false]);
    // (A tile chosen otherwise than by a click in the list, its keys among
    // them: brought into sight.)
    if d.tile != d.scrolled_to {
        d.scrolled_to = d.tile;
        if let Some(at) = d.tile.and_then(|t| rows.iter().position(|&r| r == t)) {
            let seen: Option<(usize, usize)> =
                ui.data(|m| m.get_temp(egui::Id::new(("set-tiles-seen", d.id))));
            if seen.is_none_or(|(from, end)| at <= from || at + 2 >= end) {
                let height = ui.available_height();
                scroll = scroll.vertical_scroll_offset((at as f32 * pitch - height / 2.0).max(0.0));
            }
        }
    }
    let mono = egui::FontId::monospace(12.0);
    let chosen = d.tile;
    let shown = mg_uikit::show_rows(scroll, ui, rows.len(), |ui, range| {
        ui.data_mut(|m| {
            m.insert_temp(egui::Id::new(("set-tiles-seen", d.id)), (range.start, range.end))
        });
        ui.spacing_mut().item_spacing.y = 0.0;
        let mut clicked = None;
        let mut lines = look::Lines::default();
        for &i in &rows[range] {
            let tile = &t.tiles[i];
            let (rect, r) = ui
                .allocate_exact_size(egui::vec2(ui.available_width(), pitch), egui::Sense::click());
            let selected = chosen == Some(i);
            let hole = tile_hole(t, i);
            let mut edges = String::from("[");
            for (k, e) in tile.edges.iter().enumerate() {
                if k > 0 {
                    edges.push('/');
                }
                edges.push_str(if e.is_empty() { "-" } else { e.as_str() });
            }
            edges.push(']');
            let (model, corners) = match &hole {
                Some(why) => ("a hole".to_string(), why.clone()),
                None => {
                    let mut c = String::new();
                    for (k, (name, _)) in tile.corners.iter().enumerate() {
                        if k > 0 {
                            c.push('/');
                        }
                        c.push_str(name);
                    }
                    (tile.model.clone(), c)
                }
            };
            // (Named by its number, a hole's words with it; its model,
            // corners and edges its value: a screen reader says both.)
            r.widget_info(|| {
                let said = match &hole {
                    Some(why) => format!("Tile {i}, a hole: {why}"),
                    None => format!("Tile {i}"),
                };
                let mut info = egui::WidgetInfo::selected(
                    egui::WidgetType::SelectableLabel,
                    true,
                    selected,
                    &said,
                );
                info.current_text_value = Some(format!("{model} {corners} {edges}"));
                info
            });
            let state = match (selected, r.hovered()) {
                (true, _) => look::Cell::Current { ringed: has },
                (false, true) => look::Cell::Hovered,
                (false, false) => look::Cell::Plain,
            };
            look::cell(ui.painter(), &tokens, rect, state);
            lines.cell(rect, state);
            lines.row(rect.left(), rect.right(), rect.bottom());
            let (quiet, text, warn) = match selected {
                true => (tokens.on_fill, tokens.on_fill, tokens.on_fill),
                false => (tokens.text_3, tokens.text, tokens.warn),
            };
            let words = if hole.is_some() { warn } else { text };
            let xs = columns(rect.left());
            let cut = |s: String,
                       f: egui::FontId,
                       c: egui::Color32,
                       x: &std::ops::RangeInclusive<f32>,
                       right: bool| {
                let cell = egui::Rect::from_x_y_ranges(x.clone(), rect.y_range());
                let inner = match right {
                    true => cell.with_min_x(cell.left() + 2.0).with_max_x(cell.right() - 6.0),
                    false => cell.shrink2(egui::vec2(8.0, 0.0)),
                };
                let mut job = egui::text::LayoutJob::simple_singleline(s, f, c);
                job.wrap = egui::text::TextWrapping::truncate_at_width(inner.width().max(1.0));
                let galley = ui.painter().layout_job(job);
                let y = cell.center().y - galley.size().y / 2.0;
                let x = if right { inner.right() - galley.size().x } else { inner.left() };
                ui.painter().galley(egui::pos2(x, y), galley, c);
            };
            cut(i.to_string(), mono.clone(), quiet, &xs[0], true);
            cut(model, mono.clone(), words, &xs[1], false);
            cut(corners, font.clone(), words, &xs[2], false);
            cut(edges, mono.clone(), text, &xs[3], false);
            if r.clicked() {
                clicked = Some(i);
            }
        }
        lines.paint(ui.painter(), &tokens);
        clicked
    });
    crate::palette_view::keys_outline_when(ui, shown.inner_rect, has);
    if let Some(i) = shown.inner {
        d.tile = Some(i);
        d.scrolled_to = Some(i);
        take_keys(&ctx, d.id);
    }
    if rows.is_empty() {
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.add_space(8.0);
            match d.filter.trim().is_empty() {
                true => ui.weak("No tiles: Add Tile adds one."),
                false => ui.weak("No tile has that."),
            };
        });
    }
}

/// The room a form row's label takes (the board's `--lbl-w: 104px`).
const LABEL: f32 = 104.0;

/// A row of the tile's form: its label in its column, then what `add`
/// draws in the room the row gives it (the width it is told is the room
/// left after the label and the gap, as laid out).
fn form_row(ui: &mut Ui, label: &str, add: impl FnOnce(&mut Ui, f32)) {
    ui.horizontal(|ui| {
        ui.set_min_height(mg_uikit::metrics::CONTROL_HEIGHT + 4.0);
        let (rect, _) = ui.allocate_exact_size(
            egui::vec2(LABEL, mg_uikit::metrics::CONTROL_HEIGHT),
            egui::Sense::hover(),
        );
        let layout = egui::Layout::left_to_right(egui::Align::Center);
        let mut cell = ui.new_child(egui::UiBuilder::new().max_rect(rect).layout(layout));
        cell.add(
            egui::Label::new(egui::RichText::new(label).color(mg_uikit::tokens(ui.ctx()).text_2))
                .truncate(),
        );
        let room = ui.available_width();
        add(ui, room);
    });
}

/// The chosen tile's form (the Tileset board's `.tile-form`): "Tile N" and
/// its model; the hole's note; Model, Walkmesh, Minimap picture, Path node
/// and Orientation; its lights' and loops' flags in four columns; each
/// corner's terrain and height, each edge's crosser; then its doors.
fn tile_form(app: &mut Moonglow, ui: &mut Ui, d: &mut TilesetDoc, t: &Tileset, i: usize) {
    let section = format!("TILE{i}");
    let terrains = d.type_options(false, false);
    let crossers = d.type_options(true, true);
    let tokens = mg_uikit::tokens(ui.ctx());
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(format!("Tile {i}")).strong().size(15.0).color(tokens.text));
        ui.add(
            egui::Label::new(egui::RichText::new(&t.tiles[i].model).color(tokens.text_3))
                .truncate(),
        );
    });
    ui.add_space(6.0);
    if let Some(why) = tile_hole(t, i) {
        ui.add(
            egui::Label::new(
                egui::RichText::new(format!(
                    "A hole: {why}. The game draws the tile as nothing, with no walkmesh, and \
                     painting never chooses it; a model typed here fills it in."
                ))
                .color(ui.visuals().warn_fg_color),
            )
            .wrap(),
        );
        ui.add_space(6.0);
    }
    for (key, label) in [
        ("Model", "Model"),
        ("WalkMesh", "Walkmesh"),
        ("ImageMap2D", "Minimap picture"),
        ("PathNode", "Path node"),
    ] {
        form_row(ui, label, |ui, room| {
            ui.style_mut().override_font_id = Some(egui::FontId::monospace(12.0));
            text_key(app, ui, d, &section, key, room);
        });
    }
    form_row(ui, "Orientation", |ui, room| {
        // Degrees, a quarter turn at a time.
        let current =
            d.get(&section, "Orientation").and_then(|v| v.trim().parse::<i32>().ok()).unwrap_or(0);
        let mut chosen = current;
        let r = crate::widgets::dropdown(("set-orientation", d.id, i))
            .selected_text(format!("{current}°"))
            .width(room)
            .show(ui, &mut chosen, &[0, 90, 180, 270], |a| format!("{a}°"));
        r.on_hover_text("Orientation, a quarter turn at a time");
        if chosen != current {
            d.set_key(&section, "Orientation", &chosen.to_string());
        }
    });
    ui.add_space(6.0);
    // (The flags in four columns, the board's, fewer where the form is
    // narrower than four of them.)
    let flags = [
        "MainLight1",
        "MainLight2",
        "SourceLight1",
        "SourceLight2",
        "AnimLoop1",
        "AnimLoop2",
        "AnimLoop3",
    ];
    let each = 104.0;
    let columns = ((ui.available_width() / each).floor() as usize).clamp(1, 4);
    egui::Grid::new(("set-flags", d.id)).num_columns(columns).min_col_width(each - 8.0).show(
        ui,
        |ui| {
            for (n, key) in flags.iter().enumerate() {
                flag_key(ui, d, &section, key, key);
                if (n + 1) % columns == 0 {
                    ui.end_row();
                }
            }
        },
    );
    ui.add_space(10.0);
    for corner in CORNERS {
        form_row(ui, corner, |ui, room| {
            // (The terrain box takes what the height's words and number
            // leave, at most the board's 196.)
            let height_words = 50.0;
            let number = 40.0;
            let gap = ui.spacing().item_spacing.x;
            let combo = (room - height_words - number - 2.0 * gap).clamp(60.0, 196.0);
            name_key_width(ui, d, &section, corner, &terrains, false, Some(combo));
            ui.label(egui::RichText::new("height").color(tokens.text_2));
            ui.spacing_mut().interact_size.x = number;
            int_key(ui, d, &section, &format!("{corner}Height"), -1..=4);
        });
    }
    for edge in EDGES {
        form_row(ui, &format!("{edge} edge"), |ui, room| {
            name_key_width(ui, d, &section, edge, &crossers, true, Some(room));
        });
    }
    ui.add_space(14.0);
    door_hooks(app, ui, d, i);
}

fn groups(app: &mut Moonglow, ui: &mut Ui, d: &mut TilesetDoc) {
    let Ok(t) = d.set.clone() else { return };
    ui.horizontal(|ui| {
        let tile = d.tile;
        if ui
            .button("Add Group")
            .on_hover_text(
                "Add a one-tile group of the tile chosen on the Tiles page. Make it bigger below.",
            )
            .clicked()
        {
            let mut n = 0;
            d.change(|f| {
                n = edit::add_group(f, "New group", 1, 1, &[Some(tile.unwrap_or(0) as u32)])
            });
            d.group = Some(n);
        }
        if ui.add_enabled(!t.groups.is_empty(), egui::Button::new("Remove Last Group")).clicked() {
            d.change(|f| {
                edit::remove_last_group(f);
            });
        }
    });
    ui.columns(2, |cols| {
        egui::ScrollArea::vertical().id_salt(("set-groups", d.id)).max_height(360.0).show(
            &mut cols[0],
            |ui| {
                mg_uikit::list_rows(ui);
                for (i, g) in t.groups.iter().enumerate() {
                    let text = format!("{i:>3}  {} ({}×{})", g.name, g.columns, g.rows);
                    if ui.selectable_label(d.group == Some(i), text).clicked() {
                        d.group = Some(i);
                    }
                }
            },
        );
        let ui = &mut cols[1];
        let Some(i) = d.group.filter(|&i| i < t.groups.len()) else {
            ui.weak("Choose a group.");
            return;
        };
        let g = &t.groups[i];
        let section = format!("GROUP{i}");
        ui.horizontal(|ui| {
            ui.label("Name");
            text_key(app, ui, d, &section, "Name", 220.0);
        });
        let (mut rows, mut columns) = (g.rows, g.columns);
        ui.horizontal(|ui| {
            ui.label("Rows");
            let r = ui.add(egui::DragValue::new(&mut rows).range(1..=16));
            ui.label("Columns");
            let c = ui.add(egui::DragValue::new(&mut columns).range(1..=16));
            if (r.changed() || c.changed()) && (rows, columns) != (g.rows, g.columns) {
                d.change(|f| {
                    f.set(&section, "Rows", &rows.to_string());
                    f.set(&section, "Columns", &columns.to_string());
                    for k in 0..(rows * columns) as usize {
                        if f.get(&section, &format!("Tile{k}")).is_none() {
                            f.set(&section, &format!("Tile{k}"), "-1");
                        }
                    }
                });
            }
        });
        ui.weak("Tile numbers, the bottom row first (-1: none)");
        // Rows from the top, as on the map.
        for r in (0..g.rows as usize).rev() {
            ui.horizontal(|ui| {
                for c in 0..g.columns as usize {
                    let k = r * g.columns as usize + c;
                    let current = g.tiles.get(k).copied().flatten().map_or(-1, |t| t as i64);
                    let last = t.tiles.len() as i64 - 1;
                    if let Some(v) = crate::widgets::commit_number(ui, current, -1..=last) {
                        d.set_key(&section, &format!("Tile{k}"), &v.to_string());
                    }
                }
            });
        }
    });
}

/// Asks before overwriting an existing `<stem>palstd.itp`, then writes it.
fn ask_make_palette(app: &mut Moonglow, d: &mut TilesetDoc) {
    let path = d.palette_path();
    if path.exists() {
        match app.settings.files_there {
            crate::settings::FilesThere::Replace => make_palette(app, d),
            crate::settings::FilesThere::Skip => palette_skipped(app, &path),
            crate::settings::FilesThere::Ask => {
                app.tileset_ask = Some(TilesetAsk::Palette { id: d.id });
            }
        }
    } else {
        make_palette(app, d);
    }
}

/// Make Palette with its file in the folder, set or answered to skip it:
/// the log says so, as Render Minimap Pictures says what it skipped.
fn palette_skipped(app: &mut Moonglow, path: &Path) {
    let dir = path.parent().map_or_else(String::new, |p| p.display().to_string());
    let name = path.file_name().map_or_else(String::new, |n| n.to_string_lossy().into_owned());
    app.log.info(format!("Wrote no palette into {dir} (1 existing skipped: {name})"));
}

/// Writes the palette beside the `.set`.
fn make_palette(app: &mut Moonglow, d: &mut TilesetDoc) {
    let Some(t) = d.tileset() else { return };
    let (gff, warnings) = mg_module::tileset::palette(t);
    let path = d.palette_path();
    for w in warnings {
        app.log.warn(format!("Palette: {w}"));
    }
    match gff
        .to_bytes()
        .map_err(|e| e.to_string())
        .and_then(|b| mg_module::write_replacing(&path, &b).map_err(|e| e.to_string()))
    {
        Ok(()) => app.log.info(format!("Wrote {}", path.display())),
        Err(e) => app.log.error(format!("{}: {e}", path.display())),
    }
}

/// How wide and high a minimap picture is rendered, in pixels.
pub const MINIMAP_SIZE: u32 = 32;

const MINIMAP_LAYER: &str = "tileset folder";

/// What the log says of Render Minimap Pictures where the game data is in
/// use (its folder can't be mounted).
const MINIMAP_HELD: &str = "No minimap pictures rendered: the tileset folder can't be loaded \
                            while the game data is in use. Try again once it is done.";

fn mount_tileset_dir(app: &mut Moonglow, dir: &Path) -> bool {
    app.release_game_data();
    let Some(game) = crate::exclusive(&mut app.game) else {
        app.log.error(MINIMAP_HELD);
        return false;
    };
    // (A folder a job before could not let go of, replaced.)
    game.resman.remove(MINIMAP_LAYER);
    game.resman.add(
        mg_resman::priority::TEMP,
        MINIMAP_LAYER,
        mg_resman::LayerClass::Directory,
        mg_resman::DirContainer::open(dir),
    );
    true
}

/// Unmounts the tileset's folder; false while something else holds the
/// game data (and the folder is still mounted).
fn unmount_tileset_dir(app: &mut Moonglow) -> bool {
    app.release_game_data();
    match crate::exclusive(&mut app.game) {
        Some(game) => {
            game.resman.remove(MINIMAP_LAYER);
            true
        }
        None => app.game.is_none(),
    }
}

struct MinimapPlan {
    /// Picture names (`mi_<model>`) for the tiles without one, by index.
    names: Vec<(usize, String)>,
    dir: PathBuf,
    tiles: Vec<(String, PathBuf)>,
    failed: Vec<String>,
}

impl MinimapJob {
    fn from_plan(plan: MinimapPlan, overwrite: bool) -> Self {
        Self {
            dir: plan.dir,
            tiles: plan.tiles,
            overwrite,
            next: 0,
            written: 0,
            skipped: 0,
            failed: plan.failed,
            mounted: false,
            hold: false,
            unmounting: false,
        }
    }
}

/// Gives `d`'s unnamed tiles their picture names, as one undoable change.
fn name_tiles(d: &mut TilesetDoc, names: &[(usize, String)]) {
    if names.is_empty() {
        return;
    }
    d.change(|f| {
        for (i, name) in names {
            f.set(&format!("TILE{i}"), "ImageMap2D", name);
        }
    });
}

/// Collects non-hole tiles to render, with the `mi_<model>` names unnamed
/// tiles are to get (not given here: [`name_tiles`] once it goes ahead).
fn prepare_minimap_plan(app: &mut Moonglow, d: &TilesetDoc) -> Option<MinimapPlan> {
    if app.game.is_none() {
        app.log.error("Rendering minimaps needs the game data");
        return None;
    }
    // (The folder is mounted only into game data the window alone holds:
    // refused now, before anything is asked or named.)
    app.release_game_data();
    if crate::exclusive(&mut app.game).is_none() {
        app.log.error(MINIMAP_HELD);
        return None;
    }
    let dir = d.path.parent().map(PathBuf::from).unwrap_or_default();
    let Ok(t) = d.set.clone() else { return None };
    let unnamed = |i: usize| {
        t.tiles[i]
            .image_map_2d
            .as_deref()
            .is_none_or(|n| n.is_empty() || n.eq_ignore_ascii_case("(null)"))
    };
    let names: Vec<(usize, String)> = (0..t.tiles.len())
        .filter(|&i| !t.tiles[i].is_hole() && unnamed(i))
        .map(|i| (i, format!("mi_{}", t.tiles[i].model.to_lowercase())))
        .collect();
    let mut tiles = Vec::new();
    let mut failed = Vec::new();
    let mut done = std::collections::HashSet::new();
    for (i, tile) in t.tiles.iter().enumerate() {
        if tile.is_hole() {
            continue;
        }
        // (The name it is to be given, for one without.)
        let given = names.iter().find(|(n, _)| *n == i).map(|(_, name)| name.clone());
        let Some(name) = given.or_else(|| tile.image_map_2d.clone()).filter(|n| !n.is_empty())
        else {
            continue;
        };
        if !done.insert(name.to_lowercase()) {
            continue;
        }
        let named = mg_core::ResRef::from_str(&name.to_lowercase())
            .map_err(|e| e.to_string())
            .and_then(|r| crate::transfer::named_in(&dir, &r, "tga"));
        match named {
            Ok(path) => tiles.push((tile.model.clone(), path)),
            Err(e) => failed.push(format!("{name}: {e}")),
        }
    }
    Some(MinimapPlan { names, dir, tiles, failed })
}

fn finish_minimap_job(app: &mut Moonglow, mut job: MinimapJob, canceled: bool) {
    // (Held by something else, the folder is let go of on a later frame:
    // `minimap_progress`.)
    job.unmounting = job.mounted && !unmount_tileset_dir(app);
    let noun = if job.written == 1 { "picture" } else { "pictures" };
    if canceled {
        app.log.info(format!(
            "Render Minimap Pictures canceled ({} of {} written)",
            job.written,
            job.tiles.len()
        ));
    } else if job.skipped > 0 {
        app.log.info(format!(
            "Rendered {} minimap {noun} into {} ({} existing skipped)",
            job.written,
            job.dir.display(),
            job.skipped
        ));
    } else {
        app.log.info(format!("Rendered {} minimap {noun} into {}", job.written, job.dir.display()));
    }
    for f in job.failed.iter().take(20) {
        app.log.warn(format!("Minimap picture: {f}"));
    }
    if job.failed.len() > 20 {
        app.log.warn(format!("… and {} more", job.failed.len() - 20));
    }
    if job.unmounting {
        app.minimap_job = Some(job);
    }
}

fn step_minimap_tile(app: &mut Moonglow, job: &mut MinimapJob) {
    let Some((model, path)) = job.tiles.get(job.next).cloned() else { return };
    job.next += 1;
    if !job.overwrite && path.exists() {
        job.skipped += 1;
        return;
    }
    match crate::model_view::render_tile_from_above(app, &model, MINIMAP_SIZE) {
        Some(image) => match mg_module::write_replacing(&path, &mg_image::write_tga(&image)) {
            Ok(()) => job.written += 1,
            Err(e) => job.failed.push(format!("{}: {e}", path.display())),
        },
        None => job.failed.push(format!("{model}: model not found")),
    }
}

/// Mounts `job`'s folder: the job, mounted, to [`run_minimap_job`];
/// `None` where it can't be (the log says why, and nothing is changed).
/// A job before still letting go of its folder is dropped only once this
/// one's is mounted (which replaces it), so no folder is left mounted
/// unknown.
fn start_minimap_job(app: &mut Moonglow, mut job: MinimapJob) -> Option<MinimapJob> {
    let before = app.minimap_job.take();
    if !mount_tileset_dir(app, &job.dir) {
        app.minimap_job = before;
        return None;
    }
    job.mounted = true;
    Some(job)
}

/// Renders a started job's pictures: all at once without background jobs
/// (or with none to render), else a few at every frame.
fn run_minimap_job(app: &mut Moonglow, mut job: MinimapJob) {
    if !app.background_jobs || job.tiles.is_empty() {
        while job.next < job.tiles.len() {
            step_minimap_tile(app, &mut job);
        }
        finish_minimap_job(app, job, false);
    } else {
        app.minimap_job = Some(job);
    }
}

/// Interactive Render Minimap Pictures: checks for existing `.tga` files first,
/// asks before overwriting, and steps incrementally across frames.
fn ask_render_minimaps(app: &mut Moonglow, d: &mut TilesetDoc) {
    let Some(mut plan) = prepare_minimap_plan(app, d) else { return };
    let has_existing = plan.tiles.iter().any(|(_, p)| p.exists());
    let overwrite = match app.settings.files_there {
        _ if !has_existing => true,
        crate::settings::FilesThere::Replace => true,
        crate::settings::FilesThere::Skip => false,
        crate::settings::FilesThere::Ask => {
            app.tileset_ask = Some(TilesetAsk::Minimaps {
                id: d.id,
                names: plan.names,
                dir: plan.dir,
                tiles: plan.tiles,
                failed: plan.failed,
            });
            return;
        }
    };
    let names = std::mem::take(&mut plan.names);
    let Some(job) = start_minimap_job(app, MinimapJob::from_plan(plan, overwrite)) else { return };
    name_tiles(d, &names);
    run_minimap_job(app, job);
}

/// Draws overwrite prompts and incremental minimap rendering progress.
pub(crate) fn windows(app: &mut Moonglow, ctx: &egui::Context) {
    if let Some(ask) = app.tileset_ask.clone() {
        let prompt = match &ask {
            TilesetAsk::Palette { id } => {
                let Some(idx) = app.tilesets.iter().position(|d| d.id == *id) else {
                    app.tileset_ask = None;
                    minimap_progress(app, ctx);
                    return;
                };
                let name = app.tilesets[idx]
                    .palette_path()
                    .file_name()
                    .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
                (
                    "This palette file is already in the folder:",
                    vec![(name, None)],
                    ["Overwrite the palette file", "Leave the palette file as it is"],
                )
            }
            TilesetAsk::Minimaps { tiles, .. } => {
                let existing: Vec<(String, Option<String>)> = tiles
                    .iter()
                    .filter(|(_, p)| p.exists())
                    .filter_map(|(_, p)| {
                        Some((p.file_name()?.to_string_lossy().into_owned(), None))
                    })
                    .collect();
                if existing.is_empty() {
                    // (Gone from the folder since: nothing to ask.)
                    answer_tileset_ask(app, crate::widgets::Already::Replace);
                    minimap_progress(app, ctx);
                    return;
                }
                let said = if existing.len() == 1 {
                    "This minimap picture is already in the folder:"
                } else {
                    "These minimap pictures are already in the folder:"
                };
                (
                    said,
                    existing,
                    [
                        "Overwrite the existing minimap pictures",
                        "Leave the existing minimap pictures as they are",
                    ],
                )
            }
        };
        if let Some(answer) = crate::widgets::files_already(
            ctx,
            &mut app.settings.files_there,
            "Folder",
            prompt.0,
            &prompt.1,
            prompt.2,
        ) {
            answer_tileset_ask(app, answer);
        }
    }
    minimap_progress(app, ctx);
}

/// The overwrite question answered: Replace writes over the files there,
/// Skip Those leaves them as they are, Cancel does nothing at all.
fn answer_tileset_ask(app: &mut Moonglow, answer: crate::widgets::Already) {
    let Some(ask) = app.tileset_ask.take() else { return };
    match ask {
        TilesetAsk::Palette { id } => {
            if answer == crate::widgets::Already::Replace
                && let Some(idx) = app.tilesets.iter().position(|d| d.id == id)
            {
                let mut doc = app.tilesets.remove(idx);
                make_palette(app, &mut doc);
                app.tilesets.insert(idx, doc);
            } else if answer == crate::widgets::Already::Skip
                && let Some(path) =
                    app.tilesets.iter().find(|d| d.id == id).map(|d| d.palette_path())
            {
                palette_skipped(app, &path);
            }
        }
        TilesetAsk::Minimaps { id, names, dir, tiles, failed } => {
            if answer == crate::widgets::Already::Cancel {
                return;
            }
            let overwrite = answer == crate::widgets::Already::Replace;
            let plan = MinimapPlan { names: Vec::new(), dir, tiles, failed };
            let Some(job) = start_minimap_job(app, MinimapJob::from_plan(plan, overwrite)) else {
                return;
            };
            if let Some(d) = app.tilesets.iter_mut().find(|d| d.id == id) {
                name_tiles(d, &names);
            }
            run_minimap_job(app, job);
        }
    }
}

/// Cancel in the minimap progress window: the job stopped where it is.
fn cancel_minimap_job(app: &mut Moonglow) {
    if let Some(job) = app.minimap_job.take() {
        finish_minimap_job(app, job, true);
    }
}

fn minimap_progress(app: &mut Moonglow, ctx: &egui::Context) {
    let Some(mut job) = app.minimap_job.take() else { return };
    if job.unmounting {
        // (Rendered and said: the folder let go of once nothing else
        // holds the game data.)
        // (Not `release_game_data` at every frame, which waits for the
        // holders: a frame asks whether they have let go. But where only
        // the 3D views and the haks' model views hold it, which take it
        // again whenever they draw, it is let go of here: with no job and
        // no thread holding it, nothing is waited for.)
        let had_game = app.game.is_some();
        if crate::exclusive(&mut app.game).is_none() && !app.busy() && !app.game_data_held() {
            app.release_game_data();
        }
        match crate::exclusive(&mut app.game) {
            Some(game) => {
                game.resman.remove(MINIMAP_LAYER);
            }
            None if had_game => {
                app.minimap_job = Some(job);
                ctx.request_repaint_after(std::time::Duration::from_millis(250));
            }
            None => {}
        }
        return;
    }
    if !job.hold {
        let started = std::time::Instant::now();
        let budget = std::time::Duration::from_millis(8);
        while job.next < job.tiles.len() {
            step_minimap_tile(app, &mut job);
            if started.elapsed() >= budget {
                break;
            }
        }
    }
    if job.next >= job.tiles.len() {
        finish_minimap_job(app, job, false);
        return;
    }
    let total = job.tiles.len().max(1);
    let done = job.next;
    let footed = crate::widgets::modal(ctx, "Render Minimap Pictures").show_foot(ctx, |ui| {
        crate::widgets::fit_min_width(ui, 300.0);
        ui.label("Rendering tile minimap pictures…");
        ui.add(
            egui::ProgressBar::new(done as f32 / total as f32).text(format!("{done} of {total}")),
        );
        ((), crate::widgets::Foot::new().cancel())
    });
    app.minimap_job = Some(job);
    if footed.is_some_and(|f| f.pressed.cancelled()) {
        cancel_minimap_job(app);
        return;
    }
    if !app.minimap_job.as_ref().is_some_and(|j| j.hold) {
        ctx.request_repaint();
    }
}

/// Starts a held minimap progress job for dialog-pattern tests.
#[doc(hidden)]
pub fn hold_minimap_for_tests(app: &mut Moonglow) {
    app.minimap_job = Some(MinimapJob {
        dir: PathBuf::from("."),
        tiles: vec![("t0".into(), PathBuf::from("mi_t0.tga"))],
        overwrite: true,
        next: 0,
        written: 0,
        skipped: 0,
        failed: Vec::new(),
        mounted: false,
        hold: true,
        unmounting: false,
    });
}

/// Renders every tile's minimap picture ([`MINIMAP_SIZE`] square) into the
/// `.set`'s folder, its models found there or in the game data. Tiles
/// without a picture's name get `mi_<model>` (one undoable change).
pub fn render_minimaps(app: &mut Moonglow, d: &mut TilesetDoc) {
    let Some(mut plan) = prepare_minimap_plan(app, d) else { return };
    let names = std::mem::take(&mut plan.names);
    let Some(job) = start_minimap_job(app, MinimapJob::from_plan(plan, true)) else { return };
    name_tiles(d, &names);
    run_minimap_job(app, job);
}

/// The content doctor's tileset checks, models looked for in the game
/// data: what they find replaces this tileset's findings in Problems (the
/// Bottom Dock opened on them where they find anything; nothing found is
/// said in the log and in Problems).
fn check(app: &mut Moonglow, d: &mut TilesetDoc) {
    let Some(game) = &app.game else {
        app.log.error("Checking a tileset needs the game data");
        return;
    };
    let stem = d.path.file_stem().map(|s| s.to_string_lossy().to_lowercase()).unwrap_or_default();
    let Some(key) = ResKey::parse(&stem, ResType::SET) else { return };
    let findings = mg_module::doctor::examine_tileset(&game.resman, key, &bytes_of(&d.file.text()));
    panels::to_problems(app, d, &findings);
}

#[cfg(test)]
mod tests {
    use super::*;

    const SET: &str = "[GENERAL]\r\nName=TST01\r\n\r\n[TILES]\r\nCount=2\r\n\r\n\
[TILE0]\r\nModel=t0\r\nmodel=second\r\nDoors=1\r\n\r\n[TILE0DOOR0]\r\nType=3\r\nX=1.0\r\n\r\n\
[TILE1]\r\nModel=t1\r\nTopLeft=A\r\n";

    fn doc() -> TilesetDoc {
        TilesetDoc::new(1, PathBuf::from("tst01.set"), SetFile::parse(SET), None)
    }

    /// The review's A22: a field's value is asked of the section read
    /// once for a revision of the file, and is what the file itself says
    /// (a key in any case, the first of one written twice, none for a
    /// key or a section that is not there), before and after a change,
    /// an Undo and a Redo.
    #[test]
    fn a_field_s_value_is_the_file_s_through_changes() {
        let mut d = doc();
        let same = |d: &TilesetDoc| {
            for section in ["GENERAL", "general", "TILE0", "TILE0DOOR0", "TILE1", "TILE7"] {
                for key in ["Name", "Model", "MODEL", "Doors", "Type", "X", "TopLeft", "Nope"] {
                    let file = d.file.get(section, key).map(str::to_string);
                    assert_eq!(d.get(section, key), file, "[{section}] {key}");
                }
            }
            assert_eq!(*d.doors(0), d.file.doors(0));
            assert_eq!(*d.doors(1), d.file.doors(1));
        };
        same(&d);
        assert_eq!(d.get("TILE0", "Model").as_deref(), Some("t0"));
        d.change(|f| f.set("TILE0", "Model", "other"));
        assert_eq!(d.get("tile0", "model").as_deref(), Some("other"));
        same(&d);
        d.change(|f| {
            edit::add_door(f, 1, 2);
        });
        assert_eq!(d.doors(1).len(), 1);
        same(&d);
        d.undo();
        assert!(d.doors(1).is_empty());
        same(&d);
        d.undo();
        assert_eq!(d.get("TILE0", "Model").as_deref(), Some("t0"));
        d.redo();
        same(&d);
    }

    /// Undo keeps the lines a step changed, not the file: each step taken
    /// back and made again leaves the file as it was, byte for byte, and
    /// a step that sets one value keeps one line.
    #[test]
    fn undo_keeps_the_lines_a_step_changed() {
        let mut d = doc();
        let mut texts = vec![d.file.text()];
        type Change = Box<dyn Fn(&mut SetFile)>;
        let steps: Vec<Change> = vec![
            Box::new(|f| f.set("TILE1", "TopLeft", "B")),
            Box::new(|f| {
                edit::add_tile(f, "t2", "A");
            }),
            Box::new(|f| f.set("GENERAL", "Name", "TST02")),
            Box::new(|f| {
                edit::remove_last_tile(f);
            }),
            Box::new(|f| f.remove_key("TILE0", "model")),
            Box::new(|f| {
                edit::add_door(f, 0, 1);
            }),
        ];
        for step in &steps {
            d.change(step);
            texts.push(d.file.text());
        }
        assert_eq!(d.undo.len(), steps.len());
        assert_eq!((d.undo[0].lines.len(), d.undo[0].now), (1, 1), "one line of one value");
        assert_eq!((d.undo[2].lines.len(), d.undo[2].now), (1, 1));
        for back in (0..steps.len()).rev() {
            d.undo();
            assert_eq!(d.file.text(), texts[back], "undone to {back}");
            assert!(d.file == SetFile::parse(&texts[back]));
        }
        assert!(!d.has_undo() && !d.is_dirty());
        for (forward, text) in texts.iter().enumerate().skip(1) {
            d.redo();
            assert_eq!(d.file.text(), *text, "redone to {forward}");
        }
        // (Lines alike at both ends of a change are counted once.)
        let before = SetFile::parse("a\nb\nb\nc\n");
        let after = SetFile::parse("a\nb\nc\n");
        let step = Step::back(&before, &after);
        let mut file = after.clone();
        let again = step.take(&mut file);
        assert!(file == before);
        again.take(&mut file);
        assert!(file == after);
    }

    #[test]
    fn set_key_edits_in_place_and_undoes() {
        let mut d = doc();
        let before = d.file.text();
        d.set_key("TILE0", "Model", "other");
        assert_eq!(d.get("TILE0", "Model").as_deref(), Some("other"));
        assert_eq!(d.undo.len(), 1);

        // No-op repeat does not push a duplicate undo step.
        d.set_key("TILE0", "Model", "other");
        assert_eq!(d.undo.len(), 1);

        // Missing key in existing section and brand-new section creation.
        d.set_key("TILE1", "PathNode", "p1");
        d.set_key("NEWSEC", "Foo", "Bar");
        assert_eq!(d.undo.len(), 3);
        let after = d.file.text();
        assert_eq!(d.get("TILE1", "PathNode").as_deref(), Some("p1"));
        assert_eq!(d.get("NEWSEC", "Foo").as_deref(), Some("Bar"));

        while d.has_undo() {
            d.undo();
        }
        assert_eq!(d.file.text(), before);
        assert!(!d.is_dirty());

        while d.has_redo() {
            d.redo();
        }
        assert_eq!(d.file.text(), after);
    }

    #[test]
    fn type_name_refused_enforces_16_char_limit_for_add_and_rename() {
        let d = doc();
        let t = d.set.unwrap();
        assert!(type_name_refused(&t, None, "abcdefghijklmnop").is_none());
        assert_eq!(
            type_name_refused(&t, None, "abcdefghijklmnopq"),
            Some("16 characters at most: a palette names it so")
        );
        assert_eq!(
            rename_refused(&t, false, 0, "abcdefghijklmnopq"),
            Some("16 characters at most: a palette names it so")
        );
    }

    #[test]
    fn render_minimaps_skips_holes_and_aborts_if_game_data_is_shared() {
        let dir = mg_testkit::scratch_dir("tileset-minimap-holes");
        let set_path = dir.join("tst01.set");
        let with_hole = "[GENERAL]\r\nName=TST01\r\n\r\n[TILES]\r\nCount=2\r\n\r\n\
                         [TILE0]\r\nModel=t0\r\n\r\n[TILE1]\r\nTopLeft=A\r\n";
        let mut d = TilesetDoc::new(1, set_path, SetFile::parse(with_hole), None);

        let mut rm = mg_resman::ResMan::new();
        rm.add(
            mg_resman::priority::KEY,
            "game",
            mg_resman::LayerClass::Key,
            mg_resman::MemContainer::new(),
        );
        let mut app = Moonglow::new(None, Box::new(crate::NoDialogs::default()));
        let game = std::sync::Arc::new(mg_rules::GameData::new(
            rm,
            mg_tlk::Tlk::new(mg_core::Language::ENGLISH),
        ));
        app.game = Some(game);
        // A job holding the game data prevents exclusive access: aborts
        // before mutating doc. (A job, so that nothing waits for it.)
        let held = hold_with_job(&mut app);
        render_minimaps(&mut app, &mut d);
        assert!(!d.has_undo());
        assert!(d.get("TILE0", "ImageMap2D").is_none());
        assert!(app.log.entries.iter().any(|(_, m)| m.contains("in use")));
        end_job(&mut app, &egui::Context::default(), held);

        // With exclusive access, TILE0 gets `mi_t0` while hole TILE1 does not get `mi_`.
        render_minimaps(&mut app, &mut d);
        assert_eq!(d.get("TILE0", "ImageMap2D").as_deref(), Some("mi_t0"));
        assert!(d.get("TILE1", "ImageMap2D").is_none());
    }

    /// Keep Moonglow's over a file another program changed: Save's
    /// palette follows from the names the file has now (theirs), not from
    /// those last read here (the review's D6).
    #[test]
    fn keep_mine_takes_the_names_the_file_has_now() {
        let dir = mg_testkit::scratch_dir("ui-tileset-keep-mine-names");
        let path = dir.join("tst01.set");
        let with = |terrain: &str| {
            format!("{SET}\r\n[TERRAIN TYPES]\r\nCount=1\r\n\r\n[TERRAIN0]\r\nName={terrain}\r\n")
        };
        std::fs::write(&path, with("Grass")).unwrap();
        let disk = crate::problems::fingerprint(with("Grass").as_bytes());
        let mut d = TilesetDoc::new(1, path.clone(), SetFile::parse(&with("Grass")), Some(disk));
        assert_eq!(d.saved_names.as_ref().map(|n| n[0].clone()), Some(vec!["Grass".to_string()]));
        d.change(|f| f.set("GENERAL", "Transition", "5"));
        // Another program renames the terrain; Moonglow's is kept.
        std::fs::write(&path, with("Moss")).unwrap();
        d.theirs = Some(crate::problems::fingerprint(with("Moss").as_bytes()));
        d.keep_mine();
        assert_eq!(d.saved_names.as_ref().map(|n| n[0].clone()), Some(vec!["Moss".to_string()]));
    }

    /// Keep Moonglow's over a file with a terrain named in Windows-1252
    /// (`Forêt`): its names are read as the editor reads its own text, so
    /// Save takes nothing as renamed (review U's 3).
    #[test]
    fn keep_mine_reads_a_non_ascii_name_as_the_editor_does() {
        let dir = mg_testkit::scratch_dir("ui-tileset-keep-mine-1252");
        let path = dir.join("tst01.set");
        let text =
            format!("{SET}\r\n[TERRAIN TYPES]\r\nCount=1\r\n\r\n[TERRAIN0]\r\nName=Forêt\r\n");
        let data = Codepage::default().encode(&text).unwrap().into_owned();
        std::fs::write(&path, &data).unwrap();
        let disk = crate::problems::fingerprint(&data);
        let opened = SetFile::parse(&Codepage::default().decode(&data));
        let mut d = TilesetDoc::new(1, path.clone(), opened, Some(disk));
        d.change(|f| f.set("GENERAL", "Transition", "5"));
        d.theirs = Some(disk ^ 1);
        d.keep_mine();
        assert_eq!(d.saved_names, d.tileset().map(type_names), "nothing renamed");
    }

    /// A file in Windows-1252 (`Forêt`, its 0xEA) opened and saved
    /// untouched is written back byte for byte, its fingerprint the file's
    /// (lossless editing: it was written as UTF-8).
    #[test]
    fn save_writes_back_the_windows_1252_it_read() {
        let dir = mg_testkit::scratch_dir("ui-tileset-save-1252");
        let path = dir.join("tst01.set");
        let text =
            format!("{SET}\r\n[TERRAIN TYPES]\r\nCount=1\r\n\r\n[TERRAIN0]\r\nName=Forêt\r\n");
        let data = Codepage::default().encode(&text).unwrap().into_owned();
        assert!(data.contains(&0xEA));
        std::fs::write(&path, &data).unwrap();
        let disk = crate::problems::fingerprint(&data);
        let opened = SetFile::parse(&Codepage::default().decode(&data));
        let mut d = TilesetDoc::new(1, path.clone(), opened, Some(disk));
        assert_eq!(d.fingerprint(), disk, "the text's fingerprint is the file's");
        assert_eq!(d.tileset().map(|t| t.terrains[0].name.as_str()), Some("Forêt"));
        d.save().unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), data);
        assert_eq!(d.disk, Some(disk));
    }

    /// A name Windows-1252 can't write (`Łąka`): Save refuses, saying
    /// which letters, and the file stays as it was.
    #[test]
    fn save_refuses_letters_windows_1252_cannot_write() {
        let dir = mg_testkit::scratch_dir("ui-tileset-save-unwritable");
        let path = dir.join("tst01.set");
        let text =
            format!("{SET}\r\n[TERRAIN TYPES]\r\nCount=1\r\n\r\n[TERRAIN0]\r\nName=Grass\r\n");
        std::fs::write(&path, &text).unwrap();
        let disk = crate::problems::fingerprint(text.as_bytes());
        let mut d = TilesetDoc::new(1, path.clone(), SetFile::parse(&text), Some(disk));
        d.change(|f| f.set("TERRAIN0", "Name", "Łąka"));
        let e = d.save().unwrap_err();
        assert!(e.contains("Łą") && e.contains("Windows-1252"), "{e}");
        assert_eq!(std::fs::read_to_string(&path).unwrap(), text);
        assert!(d.is_dirty());
        assert_eq!(d.tileset().map(|t| t.terrains[0].name.as_str()), Some("Łąka"));
    }

    /// Another program writes the very text that is unsaved here, with a
    /// Windows-1252 letter (`Forêt`): the file is as saved, nothing asked.
    #[test]
    fn a_file_written_outside_as_the_text_here_is_as_saved() {
        let dir = mg_testkit::scratch_dir("ui-tileset-outside-1252");
        let path = dir.join("tst01.set");
        let with = |terrain: &str| {
            format!("{SET}\r\n[TERRAIN TYPES]\r\nCount=1\r\n\r\n[TERRAIN0]\r\nName={terrain}\r\n")
        };
        std::fs::write(&path, with("Grass")).unwrap();
        let disk = crate::problems::fingerprint(with("Grass").as_bytes());
        let mut d = TilesetDoc::new(1, path.clone(), SetFile::parse(&with("Grass")), Some(disk));
        d.change(|f| f.set("TERRAIN0", "Name", "Forêt"));
        assert!(d.is_dirty());
        let data = Codepage::default().encode(&d.file.text()).unwrap().into_owned();
        std::fs::write(&path, &data).unwrap();
        let mut app = Moonglow::new(None, Box::new(crate::NoDialogs::default()));
        app.tilesets.push(d);
        changed_on_disk(&mut app, &path, false);
        let d = &app.tilesets[0];
        assert!(!d.is_dirty() && d.theirs.is_none());
        assert_eq!(d.disk, Some(crate::problems::fingerprint(&data)));
    }

    #[test]
    fn overwrite_prompts_and_skip_protect_existing_tileset_outputs() {
        let dir = mg_testkit::scratch_dir("ui-tileset-overwrite-prompts");
        let set_path = dir.join("tst01.set");
        let pal_path = dir.join("tst01palstd.itp");
        std::fs::write(&pal_path, b"EXISTING_PAL").unwrap();

        let tga_path = dir.join("mi_t0.tga");
        std::fs::write(&tga_path, b"EXISTING_TGA").unwrap();

        let mut d = doc();
        d.path = set_path;

        let mut rm = mg_resman::ResMan::new();
        rm.add(
            mg_resman::priority::KEY,
            "game",
            mg_resman::LayerClass::Key,
            mg_resman::MemContainer::new(),
        );
        let mut app = Moonglow::new(None, Box::new(crate::NoDialogs::default()));
        app.game = Some(std::sync::Arc::new(mg_rules::GameData::new(
            rm,
            mg_tlk::Tlk::new(mg_core::Language::ENGLISH),
        )));
        app.settings.files_there = crate::settings::FilesThere::Ask;

        // Existing palette triggers overwrite prompt rather than silently overwriting.
        ask_make_palette(&mut app, &mut d);
        assert!(matches!(app.tileset_ask, Some(TilesetAsk::Palette { id: 1 })));
        assert_eq!(std::fs::read(&pal_path).unwrap(), b"EXISTING_PAL");
        app.tileset_ask = None;

        // Existing minimap triggers overwrite prompt.
        ask_render_minimaps(&mut app, &mut d);
        assert!(matches!(app.tileset_ask, Some(TilesetAsk::Minimaps { .. })));
        assert_eq!(std::fs::read(&tga_path).unwrap(), b"EXISTING_TGA");
        app.tileset_ask = None;

        // Choosing Skip leaves existing `.tga` bytes intact and counts skipped.
        app.settings.files_there = crate::settings::FilesThere::Skip;
        ask_render_minimaps(&mut app, &mut d);
        assert!(app.tileset_ask.is_none());
        assert_eq!(std::fs::read(&tga_path).unwrap(), b"EXISTING_TGA");
        assert!(app.log.entries.iter().any(|(_, m)| m.contains("1 existing skipped")));
    }

    /// An app with game data of its own (no other holder of it), its
    /// questions on `files_there`.
    fn app_with_game(files_there: crate::settings::FilesThere) -> Moonglow {
        let mut rm = mg_resman::ResMan::new();
        rm.add(
            mg_resman::priority::KEY,
            "game",
            mg_resman::LayerClass::Key,
            mg_resman::MemContainer::new(),
        );
        let mut app = Moonglow::new(None, Box::new(crate::NoDialogs::default()));
        app.game = Some(std::sync::Arc::new(mg_rules::GameData::new(
            rm,
            mg_tlk::Tlk::new(mg_core::Language::ENGLISH),
        )));
        app.settings.files_there = files_there;
        app
    }

    fn mounted(app: &Moonglow) -> bool {
        app.game
            .as_ref()
            .is_some_and(|g| g.resman.layers().iter().any(|l| l.label == MINIMAP_LAYER))
    }

    /// A tileset of `n` tiles, none with a minimap picture's name, in `dir`.
    fn tiles_doc(dir: &Path, n: usize) -> TilesetDoc {
        let mut text = format!("[GENERAL]\r\nName=TST01\r\n\r\n[TILES]\r\nCount={n}\r\n");
        for i in 0..n {
            text.push_str(&format!("\r\n[TILE{i}]\r\nModel=t{i}\r\n"));
        }
        TilesetDoc::new(1, dir.join("tst01.set"), SetFile::parse(&text), None)
    }

    /// Make Palette with the palette in the folder and the questions set
    /// to Skip: the file is left as it is, and the log says so.
    #[test]
    fn make_palette_set_to_skip_says_what_it_skipped() {
        let dir = mg_testkit::scratch_dir("ui-tileset-palette-skip");
        let pal_path = dir.join("tst01palstd.itp");
        std::fs::write(&pal_path, b"EXISTING_PAL").unwrap();
        let mut d = doc();
        d.path = dir.join("tst01.set");
        let mut app = app_with_game(crate::settings::FilesThere::Skip);
        ask_make_palette(&mut app, &mut d);
        assert_eq!(std::fs::read(&pal_path).unwrap(), b"EXISTING_PAL");
        let said: Vec<_> = app.log.entries.iter().map(|(_, m)| m.clone()).collect();
        assert!(
            said.iter().any(|m| m.contains("tst01palstd.itp") && m.contains("existing skipped")),
            "{said:?}"
        );
    }

    /// Render Minimap Pictures asked about pictures in the folder, and
    /// the question cancelled: the tileset is as it was, no picture names
    /// given, nothing to undo, nothing mounted.
    #[test]
    fn cancelling_the_minimap_question_changes_nothing() {
        let dir = mg_testkit::scratch_dir("ui-tileset-minimap-cancel");
        std::fs::write(dir.join("mi_t0.tga"), b"EXISTING_TGA").unwrap();
        let mut d = tiles_doc(&dir, 2);
        let mut app = app_with_game(crate::settings::FilesThere::Ask);
        ask_render_minimaps(&mut app, &mut d);
        assert!(matches!(app.tileset_ask, Some(TilesetAsk::Minimaps { .. })));
        assert!(!d.has_undo() && !d.is_dirty(), "asking changed the tileset");
        app.tilesets.push(d);
        answer_tileset_ask(&mut app, crate::widgets::Already::Cancel);
        let d = &app.tilesets[0];
        assert!(!d.has_undo() && !d.is_dirty(), "cancel changed the tileset");
        assert!(d.get("TILE0", "ImageMap2D").is_none());
        assert!(app.tileset_ask.is_none() && app.minimap_job.is_none() && !mounted(&app));
        assert_eq!(std::fs::read(dir.join("mi_t0.tga")).unwrap(), b"EXISTING_TGA");
    }

    /// The same question answered Skip Those: the picture names are given
    /// then (one undoable change), and the job runs.
    #[test]
    fn answering_the_minimap_question_names_the_pictures() {
        let dir = mg_testkit::scratch_dir("ui-tileset-minimap-answer");
        std::fs::write(dir.join("mi_t0.tga"), b"EXISTING_TGA").unwrap();
        let mut d = tiles_doc(&dir, 2);
        let mut app = app_with_game(crate::settings::FilesThere::Ask);
        ask_render_minimaps(&mut app, &mut d);
        app.tilesets.push(d);
        answer_tileset_ask(&mut app, crate::widgets::Already::Skip);
        let d = &mut app.tilesets[0];
        assert_eq!(d.get("TILE0", "ImageMap2D").as_deref(), Some("mi_t0"));
        assert_eq!(d.get("TILE1", "ImageMap2D").as_deref(), Some("mi_t1"));
        d.undo();
        assert!(!d.has_undo() && d.get("TILE0", "ImageMap2D").is_none());
        assert!(!mounted(&app) && app.minimap_job.is_none());
        assert_eq!(std::fs::read(dir.join("mi_t0.tga")).unwrap(), b"EXISTING_TGA");
        assert!(app.log.entries.iter().any(|(_, m)| m.contains("1 existing skipped")));
    }

    /// Runs the window's frames until the minimap job is done (or `limit`
    /// frames have gone by).
    fn frames(app: &mut Moonglow, ctx: &egui::Context, limit: usize) {
        for _ in 0..limit {
            if app.minimap_job.is_none() {
                return;
            }
            let mut out = ctx.run_ui(egui::RawInput::default(), |ui| windows(app, ui.ctx()));
            out.textures_delta.clear();
        }
    }

    /// The stepped render (background jobs on) mounts the tileset's folder
    /// while it runs, and lets go of it at its end: the folder unmounted,
    /// no job left, the log's summary written.
    #[test]
    fn the_stepped_minimap_render_unmounts_the_folder_at_its_end() {
        let dir = mg_testkit::scratch_dir("ui-tileset-minimap-stepped");
        let mut d = tiles_doc(&dir, 3);
        let mut app = app_with_game(crate::settings::FilesThere::Ask);
        app.background_jobs = true;
        ask_render_minimaps(&mut app, &mut d);
        assert!(app.minimap_job.is_some() && mounted(&app), "stepped, the folder mounted");
        let ctx = egui::Context::default();
        frames(&mut app, &ctx, 100);
        assert!(app.minimap_job.is_none(), "the job ends");
        assert!(!mounted(&app), "the folder is let go of");
        assert!(crate::exclusive(&mut app.game).is_some(), "nothing holds the game data");
        let said: Vec<_> = app.log.entries.iter().map(|(_, m)| m.clone()).collect();
        assert!(said.iter().any(|m| m.starts_with("Rendered 0 minimap pictures")), "{said:?}");
    }

    /// A job that holds the game data (its share, as a job's thread has
    /// one) until the sender is dropped. (A job, not a bare clone: while
    /// one runs, `release_game_data` doesn't wait for the holders.)
    fn hold_with_job(app: &mut Moonglow) -> std::sync::mpsc::Sender<()> {
        let (tx, rx) = std::sync::mpsc::channel::<()>();
        app.start_job_for_tests("Hold", move |_| {
            let _ = rx.recv();
        });
        assert!(app.busy());
        tx
    }

    /// Lets the job of [`hold_with_job`] end, and waits for it.
    fn end_job(app: &mut Moonglow, ctx: &egui::Context, tx: std::sync::mpsc::Sender<()>) {
        drop(tx);
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while app.busy() {
            assert!(std::time::Instant::now() < deadline, "the job didn't end");
            app.poll_job(ctx);
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
    }

    /// The stepped render ending while something else holds the game data
    /// (a job, a search's thread): the folder can't be unmounted then,
    /// so it is let go of on a later frame, once the game data is free,
    /// and the log's summary is written once.
    #[test]
    fn the_minimap_folder_is_unmounted_once_the_game_data_is_free() {
        let dir = mg_testkit::scratch_dir("ui-tileset-minimap-held");
        let mut d = tiles_doc(&dir, 2);
        let mut app = app_with_game(crate::settings::FilesThere::Ask);
        app.background_jobs = true;
        ask_render_minimaps(&mut app, &mut d);
        assert!(mounted(&app));
        let held = hold_with_job(&mut app);
        let ctx = egui::Context::default();
        frames(&mut app, &ctx, 3);
        assert!(mounted(&app), "held, the folder can't be let go of yet");
        end_job(&mut app, &ctx, held);
        frames(&mut app, &ctx, 3);
        assert!(!mounted(&app), "the folder is let go of once the game data is free");
        assert!(app.minimap_job.is_none());
        let summaries = app.log.entries.iter().filter(|(_, m)| m.starts_with("Rendered ")).count();
        assert_eq!(summaries, 1);
    }

    /// The stepped render cancelled after its first tile: the folder
    /// unmounted, no job left, the log says how far it got.
    #[test]
    fn the_stepped_minimap_render_cancelled_midway_unmounts_the_folder() {
        let dir = mg_testkit::scratch_dir("ui-tileset-minimap-stepped-cancel");
        let mut d = tiles_doc(&dir, 3);
        let mut app = app_with_game(crate::settings::FilesThere::Ask);
        app.background_jobs = true;
        ask_render_minimaps(&mut app, &mut d);
        let mut job = app.minimap_job.take().expect("a stepped job");
        step_minimap_tile(&mut app, &mut job);
        assert_eq!(job.next, 1);
        assert!(mounted(&app));
        app.minimap_job = Some(job);
        cancel_minimap_job(&mut app);
        assert!(app.minimap_job.is_none(), "no job left");
        assert!(!mounted(&app), "the folder is let go of");
        assert!(crate::exclusive(&mut app.game).is_some(), "nothing holds the game data");
        let said: Vec<_> = app.log.entries.iter().map(|(_, m)| m.clone()).collect();
        assert!(said.iter().any(|m| m.contains("canceled (0 of 3 written)")), "{said:?}");
    }

    /// The folder left to unmount while a job holds the game data, and a
    /// hak's model view taking it again at every frame (as the 3D views
    /// do): once the job is done, the folder is still let go of, those
    /// views' shares dropped as `release_game_data` drops them.
    #[test]
    fn the_minimap_folder_is_unmounted_though_a_view_takes_the_game_data_again() {
        let dir = mg_testkit::scratch_dir("ui-tileset-minimap-retaken");
        let mut d = tiles_doc(&dir, 2);
        let mut app = app_with_game(crate::settings::FilesThere::Ask);
        app.background_jobs = true;
        crate::hak_view::new_hak(&mut app);
        ask_render_minimaps(&mut app, &mut d);
        assert!(mounted(&app));
        let held = hold_with_job(&mut app);
        let ctx = egui::Context::default();
        let frame = |app: &mut Moonglow| {
            // (The hak's model drawn: its load order, made again.)
            let _ = crate::hak_view::resources(app, 0);
            let mut out = ctx.run_ui(egui::RawInput::default(), |ui| windows(app, ui.ctx()));
            out.textures_delta.clear();
        };
        for _ in 0..3 {
            frame(&mut app);
        }
        assert!(mounted(&app) && app.minimap_job.as_ref().is_some_and(|j| j.unmounting));
        end_job(&mut app, &ctx, held);
        for _ in 0..5 {
            if app.minimap_job.is_none() {
                break;
            }
            frame(&mut app);
        }
        assert!(!mounted(&app), "the folder is let go of once the job is done");
        assert!(app.minimap_job.is_none());
    }

    /// Render Minimap Pictures answered while the game data is held (the
    /// folder can't be mounted): nothing is rendered, the tileset is as it
    /// was (no picture names given, nothing to undo), and the log says why.
    #[test]
    fn a_minimap_render_that_cannot_start_names_no_tiles() {
        let dir = mg_testkit::scratch_dir("ui-tileset-minimap-unstarted");
        std::fs::write(dir.join("mi_t0.tga"), b"EXISTING_TGA").unwrap();
        let mut d = tiles_doc(&dir, 2);
        let mut app = app_with_game(crate::settings::FilesThere::Ask);
        ask_render_minimaps(&mut app, &mut d);
        app.tilesets.push(d);
        let held = hold_with_job(&mut app);
        answer_tileset_ask(&mut app, crate::widgets::Already::Skip);
        let d = &app.tilesets[0];
        assert!(!d.has_undo() && !d.is_dirty(), "the tileset changed");
        assert!(d.get("TILE1", "ImageMap2D").is_none());
        assert!(app.minimap_job.is_none() && !mounted(&app));
        let said: Vec<_> = app.log.entries.iter().map(|(_, m)| m.clone()).collect();
        assert!(said.iter().any(|m| m.starts_with("No minimap pictures rendered")), "{said:?}");
        end_job(&mut app, &egui::Context::default(), held);
    }

    /// A job's folder left to unmount (the game data held as it ended),
    /// and another render that can't mount its own: the first folder is
    /// still known, and let go of once the game data is free.
    #[test]
    fn a_minimap_render_that_cannot_start_keeps_the_folder_before_it() {
        let dir = mg_testkit::scratch_dir("ui-tileset-minimap-two");
        let other = dir.join("other");
        std::fs::create_dir_all(&other).unwrap();
        std::fs::write(other.join("mi_t0.tga"), b"EXISTING_TGA").unwrap();
        let mut a = tiles_doc(&dir, 2);
        let mut b = tiles_doc(&other, 2);
        b.id = 2;
        let mut app = app_with_game(crate::settings::FilesThere::Ask);
        app.background_jobs = true;
        ask_render_minimaps(&mut app, &mut a);
        let held = hold_with_job(&mut app);
        let ctx = egui::Context::default();
        frames(&mut app, &ctx, 3);
        assert!(app.minimap_job.as_ref().is_some_and(|j| j.unmounting && j.dir == dir));
        // (Asked before the game data was held; answered while it is.)
        app.tileset_ask = Some(TilesetAsk::Minimaps {
            id: 2,
            names: vec![(0, "mi_t0".into()), (1, "mi_t1".into())],
            dir: other.clone(),
            tiles: vec![("t0".into(), other.join("mi_t0.tga"))],
            failed: Vec::new(),
        });
        app.tilesets.push(b);
        answer_tileset_ask(&mut app, crate::widgets::Already::Skip);
        assert!(
            app.minimap_job.as_ref().is_some_and(|j| j.unmounting && j.dir == dir),
            "the folder before is still to unmount"
        );
        assert!(mounted(&app));
        end_job(&mut app, &ctx, held);
        frames(&mut app, &ctx, 3);
        assert!(!mounted(&app), "the folder before is let go of");
        assert!(app.minimap_job.is_none());
    }
}
