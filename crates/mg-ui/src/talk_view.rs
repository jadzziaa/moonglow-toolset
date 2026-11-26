//! The Talk Table editor: the module's custom talk table, its lines by the
//! StrRefs the game knows them by (16777216 and up), with the feminine
//! table beside it when there is one. The table is a file of its own in the
//! user's `tlk/` folder, saved with the module, with its own undo; a module
//! kept in a nasher project may keep it there as JSON instead, one a
//! language, saved as `nwn_tlk` writes it; one kept in the module (a
//! `.tlk` among its resources) is saved into it, with it. A table found
//! in a hak is shown read-only. Open File and New File edit a `.tlk` anywhere
//! instead, with or without a module open.
//!
//! The game's own `dialog.tlk` (ruling 7 of the redesign) is shown
//! read-only in the same Talk Table tab, as the Talk board draws it (the
//! owner's ruling of 2026-11-17), its lines numbered by their own StrRefs
//! (from 0): the table the game's data read, the install's language's, one
//! copy made as it is shown and let go of as the tab shows another table or
//! closes ([`GameTalk`]). The module's table stays as it was meanwhile, its
//! unsaved lines, its steps, its chosen line and its list's place with it.
//!
//! M4's U17 (the Talk board): the editor is the kit's header over the list
//! (a header row, the rows in sight alone laid out); the chosen line's
//! fields are the Right Dock's String panel, what names it References, and
//! the table's facts (and, for a module with none, the page that makes one)
//! the Talk Table panel ([`panels`]). Find looks through a long table a
//! slice a frame ([`Sweep`]).

pub mod panels;

use std::sync::Arc;

use crate::{Action, Moonglow, Tab};
use egui::Ui;
use mg_core::StrRef;
use mg_module::talk::{self, Line, Table};
use mg_resman::ResKey;

/// Which talk table the Talk Table tab, its panels and its keys show: the
/// module's (or a file's opened on its own: [`TalkView::outside`]), or the
/// game's `dialog.tlk` ([`Which::shown`]).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum Which {
    #[default]
    Module,
    Game,
}

impl Which {
    /// Its editor's tab: the one Talk Table tab, whichever it shows.
    pub fn tab(self) -> Tab {
        Tab::TalkTable
    }

    /// The talk table a tab shows now, if it is the Talk Table tab.
    pub fn of(app: &Moonglow, tab: &Tab) -> Option<Which> {
        (*tab == Tab::TalkTable).then(|| Which::shown(app))
    }

    /// The talk table the Talk Table tab shows now: the game's while its
    /// copy is made ([`show_game`]), else the module's.
    pub fn shown(app: &Moonglow) -> Which {
        match app.game_talk {
            Some(_) => Which::Game,
            None => Which::Module,
        }
    }

    /// The StrRef the game knows a line of this table by: the module's
    /// lines from 16777216, `dialog.tlk`'s by their own numbers.
    pub fn strref(self, row: usize) -> u32 {
        match self {
            Which::Module => Table::strref(row).0,
            Which::Game => row as u32,
        }
    }
}

/// The game's `dialog.tlk` as the Resources panel and Quick Open name it
/// (no resource of the load order: the install's `lang/<code>/data`).
pub(crate) fn dialog_key() -> ResKey {
    ResKey::parse("dialog", mg_core::ResType::TLK).expect("a name")
}

/// The game's `dialog.tlk`, shown read-only (M4's U17): its lines, made
/// once from the game data's table (a copy at its opening, shared from then
/// on: no frame, scroll or Find copies it), and the view of them.
#[derive(Debug)]
pub struct GameTalk {
    pub table: Arc<Table>,
    pub view: TalkView,
    /// The game data's table it was made from (its place and length): a
    /// game data read again, or a plugin's table put in its place, makes
    /// it again. (Not the game data itself, nor a weak pointer to it: the
    /// window changes the game data in place where nothing else holds it.)
    of: (usize, usize, usize),
    /// Where the game data read it from.
    pub path: Option<std::path::PathBuf>,
}

/// The game's `dialog.tlk`, made where it is not yet or the game data read
/// another (the language changed, a plugin put its own in place): `None`
/// without the game data.
pub(crate) fn game_table(app: &mut Moonglow) -> Option<Arc<Table>> {
    let game = app.game.as_ref()?;
    let tlk = game.tlk();
    let of = (Arc::as_ptr(game) as usize, tlk.entries.as_ptr() as usize, tlk.entries.len());
    if let Some(g) = &app.game_talk
        && g.of == of
    {
        return Some(g.table.clone());
    }
    let path = app.install.as_ref().map(|i| i.talk_table());
    let label = match &path {
        Some(p) => format!("the game's files ({})", p.display()),
        None => "the game's files".to_string(),
    };
    let table = Arc::new(Table::of_game("dialog", talk::Source::Layer(label), tlk.clone()));
    // (Its view kept where it was, the line chosen while it is there.)
    let mut view = app.game_talk.take().map(|g| g.view).unwrap_or_default();
    view.changed_outside();
    view.selected = view.selected.filter(|s| *s < table.len());
    app.game_talk = Some(GameTalk { table: table.clone(), view, of, path });
    Some(table)
}

/// The game's copy shown made again where the game data read another
/// (each frame, before the docks and the Status Bar read it).
pub(crate) fn refresh(app: &mut Moonglow) {
    if app.game_talk.is_some() {
        let _ = game_table(app);
    }
}

/// Shows the game's `dialog.tlk` in the Talk Table tab (the Resources
/// panel's row, Quick Open), the tab brought forward or opened; a second
/// time, nothing more. The module's table stays as it is behind it: a
/// burst of typing open in it ends (what is typed next is a step of its
/// own), and nothing typed while the game's is shown reaches it (String is
/// read-only then).
pub(crate) fn show_game(app: &mut Moonglow) {
    if game_table(app).is_none() {
        app.log.error("dialog.tlk is the game data's: set the game's folder in Tools › Options");
        return;
    }
    if let Some(t) = app.talk.as_mut() {
        t.end_typing();
    }
    app.actions.push(Action::OpenTab(Tab::TalkTable));
}

/// The Talk Table tab back on the module's table (More › Module's Table,
/// Open File…, New File…): the game's copy let go of.
pub(crate) fn hide_game(app: &mut Moonglow) {
    app.game_talk = None;
}

/// The table `which` and its view, where it is open.
pub(crate) fn parts(app: &mut Moonglow, which: Which) -> Option<(&Table, &mut TalkView)> {
    match which {
        Which::Module => {
            let Moonglow { talk, talk_view, .. } = app;
            Some((talk.as_ref()?, talk_view))
        }
        Which::Game => {
            let g = app.game_talk.as_mut()?;
            Some((&*g.table, &mut g.view))
        }
    }
}

/// [`parts`] to look at.
pub(crate) fn parts_of(app: &Moonglow, which: Which) -> Option<(&Table, &TalkView)> {
    match which {
        Which::Module => Some((app.talk.as_ref()?, &app.talk_view)),
        Which::Game => app.game_talk.as_ref().map(|g| (&*g.table, &g.view)),
    }
}

/// Whether the table `which` shown is the one its editor edits: the
/// module's table, or a file's, that can be saved (not a hak's, not the
/// game's).
pub(crate) fn editable(app: &Moonglow, which: Which) -> bool {
    which == Which::Module && app.talk.as_ref().is_some_and(Table::editable)
}

#[derive(Debug, Default)]
pub struct TalkView {
    pub filter: String,
    pub selected: Option<usize>,
    /// Bumped on every change to the table, for the cached search.
    revision: u64,
    found: Option<(String, u64, Vec<usize>)>,
    /// Find under way, a slice of the lines a frame.
    sweep: Option<Sweep>,
    /// The selected line as edited (with plain line ends:
    /// [`shown_line`]), and as the table had it when last applied (an undo
    /// changes the table under it).
    edit: Option<(usize, Line, Line)>,
    error: Option<String>,
    /// New Talk Table: its name and whether it has a feminine table.
    pub new_name: String,
    pub new_feminine: bool,
    /// Only the lines with text or a sound are listed.
    pub only_text: bool,
    /// Those lines, and the revision they are of.
    texted: Option<(u64, Arc<Vec<usize>>)>,
    /// Unfiltered row index list `[0..len)` cached by table length.
    all_rows: Option<(usize, Arc<Vec<usize>>)>,
    /// Cached `ws.module.custom_tlk()` keyed by `(ws.identity(), ws.revision())`.
    named_cache: Option<((u64, u64), Option<String>)>,
    /// The lines listed, for what they were made by (the revision, the
    /// table's length, what is looked for, and with Only Lines with Text
    /// the line selected).
    #[allow(clippy::type_complexity)]
    shown: Option<((u64, usize, String, Option<Option<usize>>), Arc<Vec<usize>>)>,
    /// The list is to scroll to the selected line (the list changed under
    /// it).
    reveal: bool,
    /// The table is a file opened or made on its own (Open File, New
    /// File), not the module's.
    pub outside: bool,
    /// Go to: a StrRef or a line's number as typed, and why it went
    /// nowhere.
    pub go_to: String,
    go_to_problem: Option<String>,
    /// The module's table that was found and could not be read (its name):
    /// said once and not tried again while the module names it.
    unreadable: Option<String>,
    /// The module's table that was not found (its name), and when it was
    /// looked for: looked for again every few seconds, not every frame.
    missing: Option<(String, std::time::Instant)>,
    /// The languages a nasher project keeps the table open in (as JSON, a
    /// file each), where it keeps more than one.
    languages: Vec<(mg_core::Language, std::path::PathBuf)>,
    /// The open table's files and their times when last looked at, to
    /// read it again when they change outside Moonglow ([`read_again`]).
    stamps: Option<Vec<(std::path::PathBuf, Option<std::time::SystemTime>)>>,
    /// Find References for a line, on a thread of its own (the References
    /// panel), and what it found for which line.
    pub(crate) looking: Option<panels::Looking>,
    pub(crate) references: Option<panels::Found>,
    /// The chosen line's Text is to take the keyboard (Enter in the list, a
    /// line clicked twice): taken as the String panel draws it, String
    /// brought to the front for it; let go of a few frames on where it is
    /// not drawn (the pass it was asked at).
    pub(crate) to_text: Option<u64>,
    /// The line chosen as the frame before saw it: another chosen ends a
    /// burst of typing ([`Table::end_typing`]), so that what is typed after
    /// coming back is a step of Undo of its own.
    seen_line: Option<usize>,
}

/// Find looking through a table a slice a frame: what is looked for, the
/// table's revision and whether Only lines with text was on, the next line
/// to look at, the lines found so far, and those of them listed (with Only
/// lines with text, those with text), shared with the list: a slice
/// appends to it, never a copy of it made a frame.
#[derive(Debug, Clone)]
struct Sweep {
    query: String,
    revision: u64,
    only_text: bool,
    next: usize,
    found: Vec<usize>,
    listed: Arc<Vec<usize>>,
}

/// How long Find looks a frame: a long table (`dialog.tlk`'s 112,228
/// lines, each scanned without heap allocations for ASCII queries) is
/// looked through over several frames while keeping under the frame budget.
const SWEEP_BUDGET: std::time::Duration = std::time::Duration::from_micros(800);

/// How many lines Find looks through between two looks at the clock.
const SWEEP_SLICE: usize = 256;

/// Whether a line has text or a sound (Only Lines with Text lists it): not
/// blank, as its entry's flags and bytes say (no line read as text).
fn has_text(t: &Table, row: usize) -> bool {
    !t.is_blank(row)
}

/// Puts `row` in a list of rows in order, or takes it out.
fn keep_row(rows: &mut Vec<usize>, row: usize, listed: bool) {
    match (rows.binary_search(&row), listed) {
        (Err(at), true) => rows.insert(at, row),
        (Ok(at), false) => _ = rows.remove(at),
        _ => {}
    }
}

impl TalkView {
    /// Chooses line `row` and brings it into the list's sight (Go to's,
    /// the kept tabs', a test's).
    pub fn go_to_line(&mut self, row: usize) {
        self.selected = Some(row);
        self.reveal = true;
    }

    fn changed(&mut self) {
        self.revision += 1;
    }

    /// Whether Find is looking through the lines (over several frames).
    pub fn sweeping(&self) -> bool {
        self.sweep.is_some()
    }

    /// Whether Find References is looking for a line's users.
    pub fn looking(&self) -> bool {
        self.looking.is_some()
    }

    /// Whether anything of the view is under way: Find a slice a frame, or
    /// Find References on its thread.
    pub(crate) fn under_way(&self) -> bool {
        self.sweeping() || self.looking.is_some()
    }

    /// Line `row` of the table was set in the editor (a letter typed into
    /// its text): what is kept of the table's lines is brought up to that
    /// line alone. Each letter had every line looked at again, for Only
    /// Lines with Text and for what the Filter finds, and the list made
    /// anew: slow in a table that reaches into a range of a million lines.
    fn line_changed(&mut self, t: &Table, row: usize) {
        let before = self.revision;
        self.revision += 1;
        if let Some((of, texted)) = &mut self.texted
            && *of == before
        {
            let should_keep = has_text(t, row);
            if texted.binary_search(&row).is_ok() != should_keep {
                keep_row(Arc::make_mut(texted), row, should_keep);
            }
            *of = self.revision;
        }
        if let Some((query, of, found)) = &mut self.found
            && *of == before
        {
            keep_row(found, row, t.finds(row, query));
            *of = self.revision;
        }
        // (The list as it is: every line, or with Only Lines with Text
        // those with text and the one selected, which is the one written.
        // With something looked for it is made again, from what is found.)
        if let Some(((of, len, query, selected), _)) = &mut self.shown
            && (*of, *len) == (before, t.len())
            && query.is_empty()
            && selected.is_none_or(|s| s == Some(row))
        {
            *of = self.revision;
        }
    }

    /// The table changed under the editor (a line typed for a field, or
    /// taken back): the list and the line edited show it anew.
    pub(crate) fn changed_outside(&mut self) {
        self.revision += 1;
        self.edit = None;
    }
}

/// Whether the module's nasher project keeps its table in more than one
/// language (a JSON file each).
pub(crate) fn several_languages(app: &Moonglow) -> bool {
    !app.talk_view.languages.is_empty()
}

/// The name the module gives its talk table, if any (cached per workspace
/// revision so panels do not re-parse `module.ifo` on every frame).
fn named(app: &mut Moonglow) -> Option<String> {
    let ws = app.ws.as_ref()?;
    let key = (ws.identity(), ws.revision());
    if let Some((was, cached)) = &app.talk_view.named_cache
        && *was == key
    {
        return cached.clone();
    }
    let name = ws.module.custom_tlk().ok().flatten().filter(|n| !n.trim().is_empty());
    app.talk_view.named_cache = Some((key, name.clone()));
    name
}

/// Opens the module's talk table for editing, if it names one the game
/// finds and another isn't open. Unsaved changes to an open one stay.
/// The name the module gives its table, if any (read from `module.ifo`
/// here, once for those who ask after).
pub(crate) fn load(app: &mut Moonglow) -> Option<String> {
    let name = named(app);
    if let Some(name) = &name
        && !(app.talk_view.outside && app.talk.is_some())
    {
        open(app, name);
    }
    name
}

/// Opens the module's talk table `name`, unless it is the one open, the
/// open one has unsaved changes, or it was tried and could not be read.
fn open(app: &mut Moonglow, name: &str) {
    if app.talk.as_ref().is_some_and(|t| t.name == name || t.is_dirty()) {
        return;
    }
    if app.talk_view.unreadable.as_deref() == Some(name) {
        return;
    }
    let again = std::time::Duration::from_secs(3);
    if (app.talk_view.missing.as_ref()).is_some_and(|(n, at)| n == name && at.elapsed() < again) {
        return;
    }
    let (Some(game), Some(install), Some(ws)) = (&app.game, &app.install, &app.ws) else {
        return;
    };
    let dirs = install.tlk_dirs();
    // (A nasher project's own, in the game's language where it keeps one
    // a language.)
    let language = Some(game.language);
    let find = |name: &str| talk::find_for(&ws.module, &game.resman, &dirs, name, language);
    let found = match find(name) {
        Ok(Some(found)) => found,
        Ok(None) => {
            app.talk = None;
            app.talk_view.missing = Some((name.to_string(), std::time::Instant::now()));
            return;
        }
        Err(e) => {
            app.log.error(format!("Talk table {name}: {e}"));
            app.talk_view.unreadable = Some(name.to_string());
            return;
        }
    };
    let languages = project_languages(app, name);
    let feminine = match &found.source {
        talk::Source::Json { .. } => {
            project_feminine(app, name, found.tlk().ok().map(|t| t.language))
        }
        _ => find(&talk::feminine(name)).ok().flatten(),
    };
    // (A table the game ships, `data/tlk`'s, or a Workshop item's, is read
    // as a hak's is: never written over.)
    let user = install.user_dir.clone();
    let shipped = |mut f: talk::Found| {
        if let talk::Source::File(p) = &f.source
            && !user.as_ref().is_some_and(|u| p.starts_with(u))
        {
            f.source = talk::Source::Layer(format!("the game's files ({})", p.display()));
        }
        f
    };
    let (found, feminine) = (shipped(found), feminine.map(shipped));
    match Table::open(name, found, feminine) {
        Ok(t) => {
            let named_cache = std::mem::take(&mut app.talk_view.named_cache);
            let stamps = Some(stamps_now(&t));
            app.talk = Some(t);
            app.talk_view = TalkView { languages, named_cache, stamps, ..TalkView::default() };
        }
        Err(e) => {
            app.log.error(format!("Talk table {name}: {e}"));
            app.talk_view.unreadable = Some(name.to_string());
        }
    }
}

/// The languages the module's nasher project keeps its table `name` in, a
/// JSON file each: none unless it keeps more than one.
fn project_languages(app: &Moonglow, name: &str) -> Vec<(mg_core::Language, std::path::PathBuf)> {
    let Some(p) = app.ws.as_ref().and_then(|ws| ws.module.project.as_deref()) else {
        return Vec::new();
    };
    let tables = p.talk_tables(name).unwrap_or_default();
    let out: Vec<_> = tables.into_iter().map(|t| (t.language(), t.path)).collect();
    if out.len() > 1 { out } else { Vec::new() }
}

/// The feminine table of the project's table `name` in `language`: its
/// own target's (`<name>f`), or none (not a `.tlk` of the tlk folder,
/// which `nasher install` may have left there).
fn project_feminine(
    app: &Moonglow,
    name: &str,
    language: Option<mg_core::Language>,
) -> Option<talk::Found> {
    let p = app.ws.as_ref()?.module.project.as_deref()?;
    let style = p.settings.tlk_style;
    let tables = p.talk_tables(&talk::feminine(name)).unwrap_or_default();
    let f = tables.into_iter().find(|f| Some(f.language()) == language)?;
    Some(talk::Found { source: talk::Source::Json { path: f.path, style }, data: f.file })
}

/// The project's table in another of its languages (its file `path`), in
/// place of the one open (which has no unsaved changes).
pub(crate) fn open_language(app: &mut Moonglow, path: &std::path::Path) {
    let Some(t) = &app.talk else { return };
    let (name, languages) = (t.name.clone(), std::mem::take(&mut app.talk_view.languages));
    let Some(p) = app.ws.as_ref().and_then(|ws| ws.module.project.as_deref()) else { return };
    let style = p.settings.tlk_style;
    let read = |path: &std::path::Path| {
        let data = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let source = talk::Source::Json { path: path.to_path_buf(), style };
        Ok::<_, String>(talk::Found { source, data })
    };
    let language = languages.iter().find(|(_, p)| p == path).map(|(l, _)| *l);
    let feminine = project_feminine(app, &name, language);
    match read(path).and_then(|f| Table::open(&name, f, feminine)) {
        Ok(t) => {
            app.log.info(format!("Talk table {} ({} lines)", t.source, t.len()));
            let stamps = Some(stamps_now(&t));
            app.talk = Some(t);
            app.talk_view = TalkView { languages, stamps, ..TalkView::default() };
        }
        Err(e) => {
            app.log.error(format!("Talk table: {e}"));
            app.talk_view.languages = languages;
        }
    }
}

/// A table's files and their times now ([`TalkView::stamps`]): taken as
/// it is opened, so that the first Reload Resources after (a hak's save
/// does one) looks at the times, and reads the files only where they
/// changed: it read them whole to see, 50,000 lines in a frame.
fn stamps_now(t: &Table) -> Vec<(std::path::PathBuf, Option<std::time::SystemTime>)> {
    std::iter::once(&t.source)
        .chain(t.feminine.as_ref().map(|(s, _)| s))
        .filter_map(|s| s.file().map(std::path::Path::to_path_buf))
        .map(|p| {
            let at = std::fs::metadata(&p).and_then(|m| m.modified()).ok();
            (p, at)
        })
        .collect()
}

/// The talk table open in the editor, read again from its files where
/// they changed outside Moonglow (a pull of a project's JSON; their times
/// are looked at, as the custom talk table's): the editor shows what the
/// files hold now. One with unsaved changes keeps them (the next Save
/// writes them over the files), and the log says so.
pub(crate) fn read_again(app: &mut Moonglow) {
    let Some(t) = app.talk.as_mut() else {
        app.talk_view.stamps = None;
        return;
    };
    let now = stamps_now(t);
    let was = app.talk_view.stamps.replace(now.clone());
    let same_files =
        was.as_ref().is_some_and(|w| w.iter().map(|(p, _)| p).eq(now.iter().map(|(p, _)| p)));
    // (First looked at, the files are read to see whether they are as
    // the table opened was: they may have changed since.)
    let first = !same_files;
    if !first && was.as_ref() == Some(&now) {
        return;
    }
    if first && t.is_dirty() {
        return;
    }
    match t.read_again() {
        Ok(true) => {
            app.log.info(format!("Talk table {} read again (changed outside Moonglow)", t.source));
            app.talk_view.edit = None;
            app.talk_view.changed();
        }
        Ok(false) => {}
        Err(_) => app.log.warn(format!(
            "Talk table {} changed outside Moonglow while it has unsaved changes here: they are \
             kept, and Save writes them over the file",
            t.source
        )),
    }
}

/// Saves the talk table if it has unsaved changes, and has the game data
/// read it again. Returns false on an error (logged).
pub(crate) fn save(app: &mut Moonglow) -> bool {
    let Some(t) = &mut app.talk else { return true };
    if !t.is_dirty() || !t.editable() {
        return true;
    }
    // (One kept in the module goes into it outside the module's history,
    // which the module's own save then writes: the table is saved only
    // once the module has taken it. Not a step Undo takes back: the
    // table's editor, and the lines typed for the module's fields, are
    // what it is made from, and an undone step put the module's old table
    // back under the lines its fields name.)
    let Some(mut t) = app.talk.take() else { return true };
    let saved = t.save_with_module(&mut |key, data| {
        let Some(ws) = app.ws.as_mut() else { return Err("no module is open".into()) };
        ws.derive(|m| m.set(key, data)).map_err(|e| e.to_string())?;
        ws.mark_modified();
        Ok(())
    });
    let t = app.talk.insert(t);
    match saved {
        Ok(()) => {
            let in_module = t.source == talk::Source::Module;
            match in_module {
                true => app.log.info(format!(
                    "Talk table {} ({} lines) put into the module: the module's Save writes it",
                    t.name,
                    t.len()
                )),
                false => app.log.info(format!("Saved talk table {} ({} lines)", t.source, t.len())),
            }
            // (Written here: not a change made outside, for `read_again`:
            // its files' times now are the ones known.)
            app.talk_view.stamps = Some(stamps_now(t));
            if in_module {
                app.refresh_module_layer();
            }
            app.reload_custom_tlk();
            true
        }
        Err(e) => {
            app.log.error(format!("Talk table: {e}"));
            false
        }
    }
}

/// The line a text moved into the module's talk table would take now
/// ([`add_line`]): the next free one of the project's range, as typed
/// text takes one; the table's end without one.
fn line_to_add(app: &mut Moonglow, taken: &[StrRef]) -> Result<usize, String> {
    load(app);
    let own = !app.talk_view.outside;
    let t = app.talk.as_ref().filter(|t| own && t.editable()).ok_or("no talk table to add to")?;
    let taken: Vec<usize> =
        (taken.iter().filter(|s| s.is_custom())).map(|s| s.index() as usize).collect();
    crate::talk_typed::next_line_but(app, t, &taken)
}

/// The StrRef [`add_line`] would give a line added now (String Edit shows
/// it from Move to Talk Table on, and adds the line at OK).
pub(crate) fn next_strref(app: &mut Moonglow) -> Result<StrRef, String> {
    next_strref_but(app, &[])
}

/// [`next_strref`] once the lines `taken` are added: a window that adds
/// several at its OK numbers each against those before it.
pub(crate) fn next_strref_but(app: &mut Moonglow, taken: &[StrRef]) -> Result<StrRef, String> {
    line_to_add(app, taken).map(Table::strref)
}

/// Lines a window will add to the module's talk table at its OK (Move
/// Name to Talk Table, a table's New Line), each by the StrRef it was
/// shown as taking.
#[derive(Debug, Clone, Default)]
pub(crate) struct Pending(Vec<(StrRef, Line)>);

impl Pending {
    /// A line to add, by the StrRef it will take once those before it
    /// are added (as the window shows it until then).
    pub(crate) fn push(&mut self, app: &mut Moonglow, line: Line) -> Result<StrRef, String> {
        let taken: Vec<StrRef> = self.0.iter().map(|(s, _)| *s).collect();
        let strref = next_strref_but(app, &taken)?;
        self.0.push((strref, line));
        Ok(strref)
    }

    /// The text of the line on its way as `strref`.
    pub(crate) fn text(&self, strref: u32) -> Option<&str> {
        self.0.iter().find(|(s, _)| s.0 == strref).map(|(_, l)| l.text.as_str())
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Adds the lines something still names (`named`), in the order they
    /// were pushed, and lets the others go: each StrRef shown and the one
    /// its line took, where the table changed meanwhile and they differ
    /// (to be named in its place). An error stops there, and leaves the
    /// lines not yet added to be added again.
    pub(crate) fn add(
        &mut self,
        app: &mut Moonglow,
        named: impl Fn(StrRef) -> bool,
    ) -> (Vec<(StrRef, StrRef)>, Result<(), String>) {
        let mut took = Vec::new();
        while let Some((shown, line)) = self.0.first().cloned() {
            if named(shown) {
                match add_line(app, &line) {
                    Ok(strref) => {
                        app.log.info(format!(
                            "Added talk-table line {}; it's saved with the module",
                            strref.0
                        ));
                        took.push((shown, strref));
                    }
                    Err(e) => return (took, Err(e)),
                }
            }
            self.0.remove(0);
        }
        (took, Ok(()))
    }
}

/// Adds a line to the module's talk table (String Edit › Move to Talk
/// Table, at its OK); returns its StrRef.
pub(crate) fn add_line(app: &mut Moonglow, line: &Line) -> Result<StrRef, String> {
    let row = line_to_add(app, &[])?;
    let t = app.talk.as_mut().expect("checked");
    if row == t.len() {
        t.add_line(line)?;
    } else {
        t.put_line(row, line)?;
    }
    app.talk_view.changed();
    Ok(Table::strref(row))
}

/// Whether String Edit can move text into the talk table, and the table's
/// language if so.
pub(crate) fn can_add(app: &mut Moonglow) -> Option<(mg_core::Language, bool)> {
    let name = load(app);
    let t = app.talk.as_ref().filter(|t| t.editable() && !app.talk_view.outside)?;
    (Some(&t.name) == name.as_ref()).then(|| (t.tlk.language, t.feminine.is_some()))
}

/// The module's talk table's tab (or a file's opened on its own): its
/// header over its list; with no table, what makes one is the Talk Table
/// panel's (Module's Table) and the tab says so.
pub(crate) fn ui(app: &mut Moonglow, ui: &mut Ui) {
    // (The game's dialog.tlk shown in it: its own.)
    if app.game_talk.is_some() {
        game_ui(app, ui);
        return;
    }
    let outside = app.talk_view.outside && app.talk.is_some();
    if app.ws.is_none() && !outside {
        ui.label("Open a module to edit its talk table, or a talk table file on its own.");
        ui.horizontal_wrapped(|ui| files(&mut app.actions, ui));
        return;
    }
    let name = load(app);
    let open = app.talk.as_ref().is_some_and(|t| outside || Some(&t.name) == name.as_ref());
    if !open {
        no_table_here(app, ui, name.as_deref());
        return;
    }
    // The line chosen when the module's tabs were kept, chosen again and
    // brought into view (once the table is open).
    let len = app.talk.as_ref().map_or(0, |t| t.len());
    let kept = app.open_tabs.take_view(&Tab::TalkTable).and_then(|v| v.talk_line());
    if let Some(line) = kept.filter(|l| *l < len) {
        app.talk_view.selected = Some(line);
        app.talk_view.reveal = true;
    }
    // (The pointer over the editor: Undo and Redo are the table's there.)
    crate::side_doc::note_pointer(app, ui, &Tab::TalkTable);
    editor(app, ui, Which::Module);
}

/// The Talk Table tab showing the game's `dialog.tlk`: read-only, its
/// lines by their own StrRefs.
fn game_ui(app: &mut Moonglow, ui: &mut Ui) {
    if game_table(app).is_none() {
        ui.label("dialog.tlk is the game data's: set the game's folder in Tools › Options.");
        // (Module's Table and the files, from More, to leave it.)
        let state =
            HeaderState { back: Some(true), back_to_file: back_to_file(app), ..Default::default() };
        let asked = header(app, ui, Which::Game, &state);
        run_asked(app, Which::Game, &asked);
        return;
    }
    editor(app, ui, Which::Game);
}

/// The module's tab with no table of its own (or one the game can't
/// find): the header, its Open File… and New File… in More, and what to do
/// over the empty list. Making one is the Talk Table panel's.
fn no_table_here(app: &mut Moonglow, ui: &mut Ui, name: Option<&str>) {
    let asked = header(app, ui, Which::Module, &HeaderState::default());
    if let Some(pressed) = asked.pressed
        && crate::side_doc::pressed_ok_cancel(app, &Tab::TalkTable, true, pressed, |_| {})
    {
        return;
    }
    run_asked(app, Which::Module, &asked);
    let t = mg_uikit::tokens(ui.ctx());
    egui::Frame::new().inner_margin(egui::Margin::same(12)).show(ui, |ui| {
        let text = match name {
            None => "This module has no talk table of its own: make one in the Talk Table \
                     panel (Module's Table), or open a .tlk file from More."
                .to_string(),
            Some(n) => format!(
                "The module names the talk table {n}, which isn't in its haks, the module or the \
                 tlk folder: the game won't load the module. Make it in the Talk Table panel."
            ),
        };
        let color = if name.is_some() { ui.visuals().error_fg_color } else { t.text_2 };
        ui.add(egui::Label::new(egui::RichText::new(text).color(color)).wrap());
    });
}

/// What the header knows of the table shown.
#[derive(Default)]
struct HeaderState {
    open: bool,
    editable: bool,
    dirty: bool,
    empty: bool,
    selected: Option<usize>,
    /// Module's Table can be gone back to (a file opened on its own, the
    /// game's table, with a module open) and nothing unsaved holds it.
    back: Option<bool>,
    /// Module's Table from the game's table lands on a file's table with
    /// unsaved changes behind it, not the module's (its tip says so).
    back_to_file: bool,
}

/// Whether Module's Table from the game's table goes back to a file's
/// table opened on its own with unsaved changes (kept, nothing lost),
/// rather than to the module's.
fn back_to_file(app: &Moonglow) -> bool {
    app.talk_view.outside && app.talk.as_ref().is_some_and(Table::is_dirty)
}

/// What the header asked for, done after it ([`run_asked`]).
#[derive(Default)]
struct Asked {
    add: bool,
    remove: bool,
    save: bool,
    references: bool,
    export: Option<bool>,
    import: Option<bool>,
    open_file: bool,
    new_file: bool,
    module_s: bool,
    gone_to: bool,
    pressed: Option<crate::blueprint::Pressed>,
}

/// The name of a talk table's header (the kit's group, the tests'), the
/// board's.
pub const TALK_HEADER: &str = "Talk table header";

/// Find's field's id.
fn find_id(which: Which) -> egui::Id {
    egui::Id::new(("talk-find", which))
}

/// The talk table's header (M4's U17, the Talk board's): Find and Go to;
/// a line; Add Line with its word, Remove Last Line and Save as icons; a
/// line; Find References; at the right Only lines with text (a toggle),
/// Export (CSV, JSON) and More (Import CSV…, Import JSON…, Open File…,
/// New File…, Module's Table); then Close where the table is only looked
/// at, else OK and Cancel, in the end slot (ruling 10). Its own Undo and
/// Redo are gone (the Top Bar's: `Moonglow::local_undo`). What has no room
/// loses its words, then goes into More (Find and Go to last).
fn header(app: &mut Moonglow, ui: &mut Ui, which: Which, s: &HeaderState) -> Asked {
    use mg_uikit::Icon;
    use mg_uikit::header::{EditorHeader, Item};
    use std::cell::{Cell, RefCell};
    let over = ui.rect_contains_pointer(ui.max_rect());
    let (filter, go_to, mut only) = match parts(app, which) {
        Some((_, v)) => (std::mem::take(&mut v.filter), std::mem::take(&mut v.go_to), v.only_text),
        None => (String::new(), String::new(), false),
    };
    let filter = RefCell::new(filter);
    let go_to = RefCell::new(go_to);
    let gone = Cell::new(false);
    let typed = Cell::new(false);
    const FIND: f32 = 190.0;
    const GO_TO: f32 = 112.0;
    let find = |ui: &mut Ui| {
        let mut text = filter.borrow_mut();
        crate::widgets::SearchField::new(find_id(which), &mut text)
            .name("Find")
            .hint("words or a StrRef")
            .width(FIND)
            .find_when(over)
            .show(ui);
    };
    let go = |ui: &mut Ui| {
        let mut text = go_to.borrow_mut();
        let field =
            mg_uikit::text_field(&mut *text).desired_width(GO_TO).hint_text("StrRef or line");
        let r = ui
            .add_enabled(s.open, field)
            .on_hover_text("A StrRef or a line's number, then Enter, to go to it");
        r.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::TextEdit, true, "Go to"));
        if r.changed() {
            typed.set(true);
        }
        // (Its Enter taken: the list, given the keys, would go to the line's
        // text with it.)
        if r.lost_focus()
            && ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter))
        {
            gone.set(true);
        }
    };
    let export = Cell::new(None::<bool>);
    let formats = |ui: &mut Ui| {
        mg_uikit::menu_rows(ui);
        let csv = ui
            .add_enabled(s.open, egui::Button::new("Export CSV…"))
            .on_hover_text("The lines as a CSV file, for a spreadsheet or a translator");
        if csv.clicked() {
            export.set(Some(false));
            ui.close();
        }
        let json = ui
            .add_enabled(s.open, egui::Button::new("Export JSON…"))
            .on_hover_text("The lines with text as JSON, as nwn_tlk writes a talk table");
        if json.clicked() {
            export.set(Some(true));
            ui.close();
        }
    };
    let close_tip = (!s.editable).then_some("Close the window");
    let mut h = EditorHeader::new(TALK_HEADER)
        .more_words(true)
        .left(
            Item::custom("Find", FIND, find)
                .more(FIND, |ui| {
                    let mut text = filter.borrow_mut();
                    crate::widgets::SearchField::new(find_id(which), &mut text)
                        .name("Find")
                        .hint("words or a StrRef")
                        .width(FIND)
                        .show(ui);
                })
                .primary(),
        )
        .left(Item::custom("Go to", GO_TO, go).more(GO_TO, go).primary())
        .left(Item::separator())
        .left(
            Item::button(Icon::Plus, ADD_LINE)
                .enabled(s.editable)
                .tip("Add an empty line at the end, and choose it"),
        )
        .left(
            Item::icon(Icon::Minus, REMOVE_LAST_LINE)
                .enabled(s.editable && !s.empty)
                .tip("Remove Last Line: only the last, removing another would renumber those after it"),
        )
        .left(Item::icon(Icon::Save, "Save").enabled(s.editable && s.dirty).tip("Save the talk table"))
        .left(Item::separator())
        .left(
            Item::button(Icon::References, FIND_REFERENCES)
                .enabled(s.selected.is_some() && app.ws.is_some())
                .tip("Where the module's blueprints, areas, conversations and 2DAs name the selected line"),
        )
        .right(
            Item::toggle(Some(Icon::Filter), ONLY_TEXT, &mut only)
                .icon_only()
                .enabled(s.open)
                .tip("Only lines with text: leave empty lines out of the list (reserved ranges)"),
        )
        .right(
            Item::menu(Some(Icon::Export), "Export", formats)
                .words_icon(true)
                .tip("The lines as CSV, for a spreadsheet, or as JSON, as nwn_tlk writes them"),
        )
        .right(
            Item::button(Icon::Import, IMPORT_CSV)
                .in_more()
                .enabled(s.editable)
                .tip("Read lines from a CSV, as Export CSV writes it. Rows past the end add lines."),
        )
        .right(
            Item::button(Icon::Import, IMPORT_JSON)
                .in_more()
                .enabled(s.editable)
                .tip("Read lines from nwn_tlk's JSON, as a nasher repository keeps it"),
        )
        .right(
            Item::button(Icon::Open, OPEN_FILE)
                .in_more()
                .tip("Edit a .tlk file anywhere, with its feminine table if one is beside it"),
        )
        .right(Item::button(Icon::New, NEW_FILE).in_more().tip("Make an empty .tlk file and edit it"));
    if let Some(back) = s.back {
        h = h.right(Item::button(Icon::TalkTable, MODULE_S_TABLE).in_more().enabled(back).tip(
            if s.back_to_file {
                "Back to the file's table, which has unsaved changes"
            } else {
                "Back to the talk table the module names"
            },
        ));
    }
    let out =
        crate::widgets::header_with_ok_cancel(ui, h, close_tip, crate::side_doc::OK_CANCEL_TIPS);
    if let Some((_, v)) = parts(app, which) {
        v.filter = filter.into_inner();
        v.go_to = go_to.into_inner();
        if typed.get() {
            v.go_to_problem = None;
        }
        if only != v.only_text {
            v.only_text = only;
            v.reveal = true;
        }
    }
    Asked {
        add: out.clicked(ADD_LINE),
        remove: out.clicked(REMOVE_LAST_LINE),
        save: out.clicked("Save"),
        references: out.clicked(FIND_REFERENCES),
        export: export.get(),
        import: if out.clicked(IMPORT_CSV) {
            Some(false)
        } else if out.clicked(IMPORT_JSON) {
            Some(true)
        } else {
            None
        },
        open_file: out.clicked(OPEN_FILE),
        new_file: out.clicked(NEW_FILE),
        module_s: out.clicked(MODULE_S_TABLE),
        gone_to: gone.get(),
        pressed: out.end,
    }
}

/// The header's commands' names (its record's, the tests').
pub const ADD_LINE: &str = "Add Line";
pub const REMOVE_LAST_LINE: &str = "Remove Last Line";
pub const FIND_REFERENCES: &str = "Find References";
pub const ONLY_TEXT: &str = "Only lines with text";
pub const IMPORT_CSV: &str = "Import CSV…";
pub const IMPORT_JSON: &str = "Import JSON…";
pub const OPEN_FILE: &str = "Open File…";
pub const NEW_FILE: &str = "New File…";
pub const MODULE_S_TABLE: &str = "Module's Table";

/// What the tab's closing question says of Save, for a table kept in the
/// module (M5's W5): the table's Save puts its lines into the module.
pub const CLOSE_SAVE_NOTE: &str =
    "Save puts its lines into the module, and the module's Save writes them.";

/// What the tab's closing question says of Don't Save, where lines were
/// typed for the module's fields in other editors too.
pub const CLOSE_TYPED_NOTE: &str =
    "Don't Save keeps the lines typed in other editors: the module's Undo takes them back.";

/// What the header asked that needs no table open, or that leaves it:
/// Open File…, New File…, the exports and imports, Module's Table.
fn run_asked(app: &mut Moonglow, which: Which, asked: &Asked) {
    if asked.open_file {
        app.actions.push(Action::TalkFile(false));
    }
    if asked.new_file {
        app.actions.push(Action::TalkFile(true));
    }
    if let Some(json) = asked.export {
        app.actions.push(match (which, json) {
            (Which::Game, json) => Action::GameTalkExport(json),
            (Which::Module, false) => Action::TalkCsv(false),
            (Which::Module, true) => Action::TalkJson(false),
        });
    }
    if let Some(json) = asked.import
        && which == Which::Module
    {
        app.actions.push(if json { Action::TalkJson(true) } else { Action::TalkCsv(true) });
    }
    if asked.module_s {
        match which {
            Which::Module => {
                if app.talk.as_ref().is_some_and(|t| !t.is_dirty()) {
                    app.talk = None;
                    app.talk_view = TalkView::default();
                }
            }
            // (The tab back on the module's table, as it was left; a
            // file's opened on its own behind it goes as the file's own
            // Module's Table would have it go: where it has no unsaved
            // changes.)
            Which::Game => {
                hide_game(app);
                if app.talk_view.outside && app.talk.as_ref().is_some_and(|t| !t.is_dirty()) {
                    app.talk = None;
                    app.talk_view = TalkView::default();
                }
            }
        }
    }
}

/// A talk table's editor (M4's U17): its header, a note where Go to went
/// nowhere or a line could not be set, and the list of its lines under a
/// header row. The chosen line's fields, what names it and the table's
/// facts are its panels'.
fn editor(app: &mut Moonglow, ui: &mut Ui, which: Which) {
    let Some((t, v)) = parts_of(app, which) else { return };
    let state = HeaderState {
        open: true,
        editable: which == Which::Module && t.editable(),
        dirty: t.is_dirty(),
        empty: t.is_empty(),
        selected: v.selected,
        back: match which {
            Which::Module => (v.outside && app.ws.is_some()).then(|| !t.is_dirty()),
            // (Back to the module's table, or what the tab showed before.)
            Which::Game => Some(true),
        },
        back_to_file: which == Which::Game && back_to_file(app),
    };
    let asked = header(app, ui, which, &state);
    // OK and Cancel, as the other editors have: the table is saved when
    // asked (its × keeps what is unsaved for the module's Save; Cancel
    // asks first and drops it). Close alone for one only looked at.
    if let Some(pressed) = asked.pressed
        && crate::side_doc::pressed_ok_cancel(app, &which.tab(), !state.editable, pressed, |_| {})
    {
        return;
    }
    crate::side_doc::failed_foot(app, ui, &which.tab());
    run_asked(app, which, &asked);
    // (What the header did to the table is the table's to undo, More's
    // popover pressed outside the tab: the Top Bar's Undo after it.)
    if which == Which::Module && (asked.add || asked.remove || asked.import.is_some()) {
        app.undo_worked = Some(Tab::TalkTable);
    }
    if asked.module_s && which == Which::Module && app.talk.is_none() {
        return;
    }
    if which == Which::Module {
        let Moonglow { talk: Some(t), talk_view: v, .. } = app else { return };
        let mut changed = false;
        if asked.add && t.editable() {
            let empty = Line {
                text: String::new(),
                feminine: t.feminine.as_ref().map(|_| String::new()),
                sound: String::new(),
                sound_length: 0.0,
            };
            if let Ok(row) = t.add_line(&empty) {
                v.selected = Some(row);
                v.filter.clear();
                // (An empty line: it would not be listed.)
                v.only_text = false;
                v.reveal = true;
                changed = true;
            }
        }
        if asked.remove && t.editable() && !t.is_empty() {
            t.remove_last();
            changed = true;
        }
        if changed {
            v.changed();
        }
        if asked.save {
            // Saved through the app, which reloads the game's copy.
            app.actions.push(Action::SaveTalkTable);
        }
    }
    if asked.references
        && let Some(row) = parts_of(app, which).and_then(|(_, v)| v.selected)
    {
        panels::find_references(app, which, row);
    }
    if asked.gone_to {
        go_to(app, which, ui.ctx());
    }
    notes(app, ui, which);
    list(app, ui, which);
}

/// Each frame, for each talk table: Find References is polled when active,
/// and active Find sweeps continue only while the Talk Table tab is in front
/// and showing that table (otherwise `sweep` is cleared and resumes when shown
/// again).
pub(crate) fn follow(app: &mut Moonglow, ctx: &egui::Context) {
    let pass = ctx.cumulative_pass_nr();
    // (The game's table let go of with its tab, its look stopped.)
    // (Not while its tab is on its way: `show_game` asks for it.)
    let opening = app.actions.iter().any(|a| matches!(a, Action::OpenTab(Tab::TalkTable)));
    if app.game_talk.is_some() && app.dock.find_tab(&Tab::TalkTable).is_none() && !opening {
        app.game_talk = None;
    }
    // Another line chosen: end any in-flight typing burst cleanly.
    if app.talk_view.seen_line != app.talk_view.selected {
        app.talk_view.seen_line = app.talk_view.selected;
        if let Some(t) = app.talk.as_mut() {
            t.end_typing();
        }
    }
    // (The module's view where its table is gone, or its tab closed: its
    // look stopped and Find let go of, not left under way.)
    if app.talk.is_none() || app.dock.find_tab(&Tab::TalkTable).is_none() {
        app.talk_view.looking = None;
        app.talk_view.sweep = None;
    }
    for which in [Which::Module, Which::Game] {
        let Some((_, v)) = parts(app, which) else { continue };
        panels::follow_looking(v, ctx);
        // (Find looks on as the list is drawn, `rows`; where its tab is
        // not, or shows the other table, the look is let go of: begun
        // again as it is drawn.)
        if v.sweeping()
            && (!in_front(app, &which.tab()) || Which::shown(app) != which)
            && let Some((_, v)) = parts(app, which)
        {
            v.sweep = None;
        }
        // (The text not drawn to take the keyboard: let go of.)
        if let Some((_, v)) = parts(app, which)
            && v.to_text.is_some_and(|at| pass > at + 3)
        {
            v.to_text = None;
        }
    }
}

/// Whether `tab` is open and in front of its pane (its body drawn).
fn in_front(app: &Moonglow, tab: &Tab) -> bool {
    let Some(at) = app.dock.find_tab(tab) else { return false };
    let leaf = app.dock.leaf(egui_dock::NodePath { surface: at.surface, node: at.node });
    leaf.is_ok_and(|leaf| leaf.active == at.tab)
}

/// Go to: the line of the StrRef or number typed, among all the lines,
/// the list given the keys; or why it went nowhere.
fn go_to(app: &mut Moonglow, which: Which, ctx: &egui::Context) {
    let Some((t, v)) = parts(app, which) else { return };
    let typed = v.go_to.trim();
    let base = which.strref(0);
    let row = typed.parse::<u32>().ok().map(|n| n.checked_sub(base).unwrap_or(n) as usize);
    match row {
        Some(row) if row < t.len() => {
            v.selected = Some(row);
            // (Among all the lines: one found alone is nowhere to go to.)
            v.filter.clear();
            v.reveal = true;
            v.go_to_problem = None;
            take_keys(ctx, which);
        }
        Some(_) if t.is_empty() => v.go_to_problem = Some("The table has no lines yet.".into()),
        Some(_) => {
            let last = which.strref(t.len() - 1);
            v.go_to_problem =
                Some(format!("The table has {} lines: StrRefs {base} to {last}.", t.len()));
        }
        None if typed.is_empty() => v.go_to_problem = None,
        None => v.go_to_problem = Some("A StrRef or a line's number.".into()),
    }
}

/// What is said under the header: where Go to went nowhere.
fn notes(app: &mut Moonglow, ui: &mut Ui, which: Which) {
    let Some((_, v)) = parts_of(app, which) else { return };
    let Some(problem) = v.go_to_problem.clone() else { return };
    egui::Frame::new().inner_margin(egui::Margin::symmetric(8, 4)).show(ui, |ui| {
        let color = ui.visuals().warn_fg_color;
        ui.add(egui::Label::new(egui::RichText::new(problem).color(color)).wrap());
    });
}

/// The list's keys given to the table `which` (a click on a line, Go to,
/// F6, a test): its editor made the active one as its tab is next drawn.
pub fn take_keys(ctx: &egui::Context, which: Which) {
    crate::palette_view::give_keys(ctx, crate::palette_view::Keys::Talk);
    ctx.data_mut(|d| {
        d.insert_temp(egui::Id::new("talk-keys-of"), which);
        d.insert_temp(egui::Id::new("talk-keys-focus"), which);
    });
}

/// The list's keys given to the table `which` without its editor made the
/// active one (for the tests: the keys left with a list whose editor is
/// not in front, as a press elsewhere leaves them).
#[doc(hidden)]
pub fn keys_left_with(ctx: &egui::Context, which: Which) {
    crate::palette_view::give_keys(ctx, crate::palette_view::Keys::Talk);
    ctx.data_mut(|d| d.insert_temp(egui::Id::new("talk-keys-of"), which));
}

/// The talk table whose list was given the keys last.
pub(crate) fn keys_of(ctx: &egui::Context) -> Option<Which> {
    ctx.data(|d| d.get_temp(egui::Id::new("talk-keys-of")))
}

/// Whether the list of `which` has the keys: they are a talk table's, its,
/// its editor is the active one, and nothing modal or a popover is in
/// front (as a 2DA's grid's, `table_view::grid_has_keys`).
pub(crate) fn list_has_keys(app: &Moonglow, ctx: &egui::Context, which: Which) -> bool {
    crate::palette_view::keys_are(ctx, crate::palette_view::Keys::Talk)
        && keys_of(ctx) == Some(which)
        && crate::table_view::active_tab(app).as_ref() == Some(&which.tab())
        && !crate::shell::keys_blocked(ctx)
        && !mg_uikit::popover::Popover::any_open(ctx)
}

/// Where the list was given the keys, its tab's pane is given the focus
/// (its editor the active one).
fn focus_asked(app: &mut Moonglow, ctx: &egui::Context, which: Which) {
    let key = egui::Id::new("talk-keys-focus");
    if ctx.data(|d| d.get_temp::<Which>(key)) != Some(which) {
        return;
    }
    ctx.data_mut(|d| d.remove_temp::<Which>(key));
    app.focus_asked_tab = Some(which.tab());
}

/// The lines listed: all, or those Find found; with Only Lines with Text,
/// those of them with text or a sound (and the line chosen). Kept while
/// what they were made by is the same: every line's number was listed anew
/// each frame, a million for a table that reaches into a range up there.
/// While Find looks through a long table, those found so far.
fn rows(t: &Table, v: &mut TalkView) -> Arc<Vec<usize>> {
    let query = v.filter.trim().to_string();
    let kept =
        if v.only_text { v.selected.filter(|s| !has_text(t, *s) && *s < t.len()) } else { None };
    let mark = (v.revision, t.len(), query.clone(), v.only_text.then_some(kept));
    if let Some((of, rows)) = &v.shown
        && *of == mark
    {
        return rows.clone();
    }
    if query.is_empty() {
        v.sweep = None;
        let rows = if v.only_text {
            if v.texted.as_ref().is_none_or(|(r, _)| *r != v.revision) {
                v.texted = Some((
                    v.revision,
                    Arc::new((0..t.len()).filter(|row| has_text(t, *row)).collect()),
                ));
            }
            let texted = v.texted.as_ref().expect("set above").1.clone();
            match kept {
                None => texted,
                Some(sel) => {
                    let mut rows = (*texted).clone();
                    keep_row(&mut rows, sel, true);
                    Arc::new(rows)
                }
            }
        } else {
            match &v.all_rows {
                Some((len, rows)) if *len == t.len() => rows.clone(),
                _ => {
                    let rows: Arc<Vec<usize>> = Arc::new((0..t.len()).collect());
                    v.all_rows = Some((t.len(), rows.clone()));
                    rows
                }
            }
        };
        v.shown = Some((mark, rows.clone()));
        return rows;
    }
    let done = v.found.as_ref().is_some_and(|(q, r, _)| *q == query && *r == v.revision);
    if !done {
        // (Looking on: what is listed meanwhile, appended to, shared.)
        if let Some(listed) = sweep(t, v, &query) {
            return listed;
        }
    }
    v.sweep = None;
    let found = v.found.as_ref().map(|f| &f.2);
    let rows: Vec<usize> = match (found, v.only_text) {
        (Some(found), true) => {
            found.iter().copied().filter(|r| has_text(t, *r) || v.selected == Some(*r)).collect()
        }
        (Some(found), false) => found.clone(),
        (None, _) => Vec::new(),
    };
    let rows = Arc::new(rows);
    v.shown = Some((mark, rows.clone()));
    rows
}

/// Find looks on through `t` for `query` from where it was, for at most
/// [`SWEEP_BUDGET`] (the clock asked every [`SWEEP_SLICE`] lines): the
/// lines listed so far where it has not looked through them all (`None`
/// once it has, what it found kept in `found`). A query, the table or Only
/// lines with text changed begins it again.
fn sweep(t: &Table, v: &mut TalkView, query: &str) -> Option<Arc<Vec<usize>>> {
    let TalkView { sweep, found, revision, only_text, .. } = v;
    let fresh = sweep
        .as_ref()
        .is_some_and(|s| s.query == query && s.revision == *revision && s.only_text == *only_text);
    if !fresh {
        *sweep = Some(Sweep {
            query: query.into(),
            revision: *revision,
            only_text: *only_text,
            next: 0,
            found: Vec::new(),
            listed: Arc::new(Vec::new()),
        });
    }
    let s = sweep.as_mut().expect("set");
    let filter_only = s.only_text;
    let began = std::time::Instant::now();
    while s.next < t.len() {
        let end = (s.next + SWEEP_SLICE).min(t.len());
        let new = t.find_in(s.next..end, query);
        let listed = Arc::make_mut(&mut s.listed);
        if filter_only {
            listed.extend(new.iter().copied().filter(|r| has_text(t, *r)));
        } else {
            listed.extend_from_slice(&new);
        }
        s.found.extend(new);
        s.next = end;
        if began.elapsed() >= SWEEP_BUDGET {
            break;
        }
    }
    if s.next < t.len() {
        return Some(s.listed.clone());
    }
    let done = sweep.take().expect("set");
    *found = Some((done.query, done.revision, done.found));
    None
}

/// The list's columns' widths (the board's `.tl-row`): the StrRef's 96, the
/// sound's 150, the text the rest (two halves with a feminine table).
const STRREF_WIDTH: f32 = 96.0;
const SOUND_WIDTH: f32 = 150.0;

/// The list (M4's U17, the Talk board's `.tl`): a header row (StrRef, Text,
/// Feminine where the table has one, Sound) over the lines, each 22 high,
/// only those in sight laid out; the StrRef monospace at its right, the
/// text's first line cut short, the sound monospace and quiet; the chosen
/// line filled. A click chooses a line and gives the list the keys (Up,
/// Down, Page Up, Page Down, Home, End; Enter to its text, as a double
/// click; Escape leaves the list).
fn list(app: &mut Moonglow, ui: &mut Ui, which: Which) {
    use crate::palette_view::{Keys, give_keys};
    let has = list_has_keys(app, ui.ctx(), which);
    // Undo and Redo are the table's while the keys are in its list (the
    // keymap's, through `Moonglow::local_undo`).
    if has && which == Which::Module {
        app.undo_keys = Some(Tab::TalkTable);
    }
    focus_asked(app, ui.ctx(), which);
    let Some((t, v)) = parts(app, which) else { return };
    let rows = rows(t, v);
    let feminine = t.feminine.is_some();
    v.selected = v.selected.filter(|&s| s < t.len());
    let tokens = mg_uikit::tokens(ui.ctx());
    let pitch = mg_uikit::row_metrics(ui).1;
    let seen_id = egui::Id::new(("talk-rows-seen", which));
    let mut to_text = false;
    if has && !ui.ctx().egui_wants_keyboard_input() && !rows.is_empty() {
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
            // (Looked for only as a key is pressed: a hundred thousand
            // rows each frame.)
            let at = v.selected.and_then(|s| rows.iter().position(|r| *r == s));
            let to = at.map_or(0, |at| (at as isize).saturating_add(by).clamp(0, last)) as usize;
            v.selected = Some(rows[to]);
            // (Into view, if it is not among the lines drawn.)
            let seen: Option<(usize, usize)> = ui.data(|d| d.get_temp(seen_id));
            v.reveal = seen.is_none_or(|(from, end)| to <= from || to + 2 >= end);
        }
        to_text = v.selected.is_some()
            && ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::Enter));
        if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::Escape)) {
            give_keys(ui.ctx(), Keys::Area);
        }
    }
    // The header row.
    let width = ui.available_width();
    let (head, _) = ui.allocate_exact_size(
        egui::vec2(width, crate::table_view::look::HEADER_HIGH),
        egui::Sense::hover(),
    );
    let text_width = (width - STRREF_WIDTH - SOUND_WIDTH).max(80.0);
    let text_width = if feminine { text_width / 2.0 } else { text_width };
    let columns = |left: f32| {
        let mut x = left;
        let mut out = [0.0..=0.0, 0.0..=0.0, 0.0..=0.0, 0.0..=0.0];
        let mut push = |idx: usize, w: f32| {
            out[idx] = x..=x + w;
            x += w;
        };
        push(0, STRREF_WIDTH);
        push(1, text_width);
        if feminine {
            push(2, text_width);
            push(3, SOUND_WIDTH);
            (out, 4)
        } else {
            push(2, SOUND_WIDTH);
            (out, 3)
        }
    };
    crate::table_view::look::header_cell(ui, head);
    let words: &[&str] = match feminine {
        true => &["StrRef", "Text (masculine)", "Feminine", "Sound"],
        false => &["StrRef", "Text", "Sound"],
    };
    let font = egui::TextStyle::Body.resolve(ui.style());
    let (head_cols, head_len) = columns(head.left());
    for (i, (x, words)) in head_cols[..head_len].iter().zip(words).enumerate() {
        let cell =
            egui::Rect::from_x_y_ranges(x.clone(), head.y_range()).shrink2(egui::vec2(8.0, 0.0));
        let galley =
            ui.painter().layout((*words).to_string(), font.clone(), tokens.text, cell.width());
        // (The StrRef's heading at its right, over the numbers.)
        let at = match i {
            0 => egui::pos2(
                cell.right() - 4.0 - galley.size().x,
                cell.center().y - galley.size().y / 2.0,
            ),
            _ => egui::pos2(cell.left(), cell.center().y - galley.size().y / 2.0),
        };
        ui.painter().with_clip_rect(cell).galley(at, galley, tokens.text);
    }
    let mut scroll =
        egui::ScrollArea::vertical().id_salt(("talk-rows", which)).auto_shrink([false, false]);
    // The list changed under the selected line: it comes back into view.
    if std::mem::take(&mut v.reveal)
        && let Some(at) = v.selected.and_then(|s| rows.iter().position(|r| *r == s))
    {
        let height = ui.available_height();
        scroll = scroll.vertical_scroll_offset((at as f32 * pitch - height / 2.0).max(0.0));
    }
    let mono = egui::FontId::monospace(12.0);
    let shown = mg_uikit::show_rows(scroll, ui, rows.len(), |ui, range| {
        ui.data_mut(|d| d.insert_temp(seen_id, (range.start, range.end)));
        ui.spacing_mut().item_spacing.y = 0.0;
        let mut clicked = None;
        // (Painted as the 2DA editor's grid is, `table_view::look`: a row
        // a cell of it, the chosen one filled and, while the list has the
        // keys, ringed; the lines under each, as few shapes.)
        let mut lines = crate::table_view::look::Lines::default();
        for &row in &rows[range] {
            let (rect, r) = ui
                .allocate_exact_size(egui::vec2(ui.available_width(), pitch), egui::Sense::click());
            let strref = which.strref(row);
            let line = t.line(row);
            fn first(s: &str) -> &str {
                s.lines().next().unwrap_or_default()
            }
            // (Named by its StrRef, its text its value: a screen reader
            // says both.)
            r.widget_info(|| {
                let mut info = egui::WidgetInfo::selected(
                    egui::WidgetType::SelectableLabel,
                    true,
                    v.selected == Some(row),
                    strref.to_string(),
                );
                info.current_text_value = Some(first(&line.text).to_string());
                info
            });
            let selected = v.selected == Some(row);
            use crate::table_view::look::Cell;
            let state = match (selected, r.hovered()) {
                (true, _) => Cell::Current { ringed: has },
                (false, true) => Cell::Hovered,
                (false, false) => Cell::Plain,
            };
            crate::table_view::look::cell(ui.painter(), &tokens, rect, state);
            lines.cell(rect, state);
            lines.row(rect.left(), rect.right(), rect.bottom());
            let (number, text, quiet) = match selected {
                true => (tokens.on_fill, tokens.on_fill, tokens.on_fill),
                false => (tokens.text_2, tokens.text, tokens.text_3),
            };
            let (xs, xs_len) = columns(rect.left());
            let cut = |s: &str,
                       f: egui::FontId,
                       c: egui::Color32,
                       x: &std::ops::RangeInclusive<f32>,
                       right: bool| {
                if s.is_empty() {
                    return;
                }
                let cell = egui::Rect::from_x_y_ranges(x.clone(), rect.y_range());
                let inner = match right {
                    true => cell.with_max_x(cell.right() - 12.0).with_min_x(cell.left() + 8.0),
                    false => cell.shrink2(egui::vec2(8.0, 0.0)),
                };
                let mut job = egui::text::LayoutJob::simple_singleline(s.to_owned(), f, c);
                job.wrap = egui::text::TextWrapping::truncate_at_width(inner.width().max(1.0));
                let galley = ui.painter().layout_job(job);
                let y = cell.center().y - galley.size().y / 2.0;
                let x = if right { inner.right() - galley.size().x } else { inner.left() };
                ui.painter().galley(egui::pos2(x, y), galley, c);
            };
            let strref_text = strref.to_string();
            cut(&strref_text, mono.clone(), number, &xs[0], true);
            cut(first(&line.text), font.clone(), text, &xs[1], false);
            if let Some(f) = &line.feminine {
                cut(first(f), font.clone(), text, &xs[2], false);
            }
            cut(&line.sound, mono.clone(), quiet, &xs[xs_len - 1], false);
            let r = r.on_hover_cursor(egui::CursorIcon::PointingHand);
            if r.clicked() || r.double_clicked() {
                clicked = Some((row, r.double_clicked()));
            }
        }
        lines.paint(ui.painter(), &tokens);
        clicked
    });
    if let Some((row, twice)) = shown.inner {
        v.selected = Some(row);
        to_text |= twice;
        take_keys(ui.ctx(), which);
    }
    if rows.is_empty() && !v.filter.trim().is_empty() && !v.sweeping() {
        ui.weak("No line has that.");
    }
    if v.sweeping() {
        // (Looking on at the next frame: the window goes on meanwhile.)
        ui.ctx().request_repaint();
    }
    if to_text {
        v.to_text = Some(ui.ctx().cumulative_pass_nr());
        bring_string(app, which);
    }
}

/// String brought to the front where its line's text is to take the
/// keyboard: the Right Dock's (unfolded on it), or a floating table's
/// window's right pane's.
fn bring_string(app: &mut Moonglow, which: Which) {
    let tab = which.tab();
    let floating = app.dock.find_tab(&tab).is_some_and(|at| !at.surface.is_main());
    match floating {
        true => _ = app.window_rights.insert(tab, crate::Panel::String),
        false => app.actions.push(Action::ShowPanel(crate::Panel::String)),
    }
}

/// Undo (or redo) in the talk table, the pointer over its editor
/// (`Moonglow::local_undo`): whether it is open (with nothing to undo,
/// nothing is).
pub(crate) fn undo(app: &mut Moonglow, redo: bool) -> bool {
    let Some(t) = app.talk.as_mut() else { return false };
    if !(if redo { t.can_redo() } else { t.can_undo() }) {
        return true;
    }
    if redo {
        t.redo()
    } else {
        t.undo()
    }
    app.talk_view.changed();
    true
}

/// Export of the game's `dialog.tlk` (its lines by their own numbers):
/// CSV, or nwn_tlk's JSON.
pub(crate) fn export_game(app: &mut Moonglow, json: bool) {
    let Some(t) = game_table(app) else { return };
    let kind = crate::dialogs::FileKind::TalkLines { json };
    let ext = if json { "json" } else { "csv" };
    let start = std::path::PathBuf::from(format!("dialog.{ext}"));
    let Some(path) = app.dialogs.save_file(kind, Some(&start)) else { return };
    let data = if json { t.to_json() } else { t.to_csv_as(false).into_bytes() };
    match std::fs::write(&path, data) {
        Ok(()) => app.log.info(format!("Wrote {} lines to {}", t.len(), path.display())),
        Err(e) => app.log.error(format!("{}: {e}", path.display())),
    }
}

/// Open File… and New File…: a talk table that is a file anywhere. (Put
/// into the row they are called in: in a wrapping row of their own at
/// the end of one, they stood in a sliver past the row's end, one under
/// the other, and held the list a long way down.)
fn files(actions: &mut Vec<Action>, ui: &mut Ui) {
    if ui
        .button("Open File…")
        .on_hover_text("Edit a .tlk file anywhere, with its feminine table if one is beside it")
        .clicked()
    {
        actions.push(Action::TalkFile(false));
    }
    if ui.button("New File…").on_hover_text("Make an empty .tlk file and edit it").clicked() {
        actions.push(Action::TalkFile(true));
    }
}

/// Open File (or, `new`, New File): the editor takes a talk table that is
/// a file of its own, whatever table the module names.
pub(crate) fn file(app: &mut Moonglow, new: bool) {
    if app.talk.as_ref().is_some_and(|t| t.is_dirty()) {
        app.log.error("The talk table has unsaved changes: save it (or undo them) first.");
        return;
    }
    let start = app.install.as_ref().and_then(|i| i.user_folder("tlk"));
    let named = |path: &std::path::Path| {
        path.file_stem().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
    };
    let table = if new {
        let suggested = start.map(|d| d.join("new.tlk"));
        let kind = crate::dialogs::FileKind::TalkTable;
        let Some(path) = app.dialogs.save_file(kind, suggested.as_deref()) else {
            return;
        };
        let path = if path.extension().is_some() { path } else { path.with_extension("tlk") };
        let language = app.game.as_deref().map_or(mg_core::Language::ENGLISH, |g| g.language);
        let dir = path.parent().map(std::path::Path::to_path_buf).unwrap_or_default();
        let mut t = Table::create(&named(&path), &dir, language, false);
        t.source = talk::Source::File(path);
        t.save().map(|()| t)
    } else {
        let kind = crate::dialogs::FileKind::TalkTable;
        let Some(path) = app.dialogs.open_file(kind, start.as_deref()) else {
            return;
        };
        let found = |p: &std::path::Path| {
            let data = std::fs::read(p).map_err(|e| format!("{}: {e}", p.display()))?;
            Ok::<_, String>(talk::Found { source: talk::Source::File(p.to_path_buf()), data })
        };
        let name = named(&path);
        let beside = path.with_file_name(format!("{}.tlk", talk::feminine(&name)));
        let feminine = if beside.is_file() { found(&beside).map(Some) } else { Ok(None) };
        found(&path).and_then(|f| Table::open(&name, f, feminine?))
    };
    match table {
        Ok(t) => {
            app.log.info(format!("Talk table {} ({} lines)", t.source, t.len()));
            let stamps = Some(stamps_now(&t));
            app.talk = Some(t);
            app.talk_view = TalkView { outside: true, stamps, ..Default::default() };
            // (Shown in the tab in place of the game's dialog.tlk too.)
            hide_game(app);
            app.actions.push(Action::OpenTab(Tab::TalkTable));
        }
        Err(e) => app.log.error(format!("Talk table: {e}")),
    }
}

/// Export: the table's lines to a file, as CSV for a spreadsheet or as
/// JSON (`nwn_tlk`'s, nasher's). Import: lines read from one, by their
/// StrRefs, as one undoable step.
pub(crate) fn transfer(app: &mut Moonglow, import: bool, json: bool) {
    let Some(t) = app.talk.as_mut() else { return };
    let ext = if json { "json" } else { "csv" };
    let start =
        t.source.file().and_then(|f| f.parent()).map(|d| d.join(format!("{}.{ext}", t.name)));
    let kind = crate::dialogs::FileKind::TalkLines { json };
    if import {
        let Some(path) = app.dialogs.open_file(kind, start.as_deref()) else {
            return;
        };
        let read = std::fs::read_to_string(&path).map_err(|e| e.to_string());
        let read =
            read.and_then(|text| if json { t.import_json(&text) } else { t.import_csv(&text) });
        match read {
            Ok((changed, added)) => {
                app.talk_view.changed();
                app.log.info(format!(
                    "Read {}: {changed} lines changed, {added} added",
                    path.display()
                ));
            }
            Err(e) => app.log.error(format!("{}: {e}", path.display())),
        }
    } else {
        let Some(path) = app.dialogs.save_file(kind, start.as_deref()) else {
            return;
        };
        match std::fs::write(&path, if json { t.to_json() } else { t.to_csv().into_bytes() }) {
            Ok(()) => app.log.info(format!("Wrote {} lines to {}", t.len(), path.display())),
            Err(e) => app.log.error(format!("{}: {e}", path.display())),
        }
    }
}

/// Makes the talk table `name` in the module's nasher project
/// ([`mg_module::nasher::Project::add_talk_table`]: its file, a target of
/// its own) and opens it from there.
pub(crate) fn in_project_table(
    app: &mut Moonglow,
    name: &str,
    language: mg_core::Language,
    feminine: bool,
) -> Result<Table, String> {
    let p = app.ws.as_mut().and_then(|ws| ws.module.project.as_deref_mut()).ok_or("no project")?;
    let made = p.add_talk_table(name, language, feminine).map_err(|e| e.to_string())?;
    let style = p.settings.tlk_style;
    let warnings = std::mem::take(&mut p.warnings);
    for w in warnings {
        app.log.warn(w);
    }
    let found = |path: &std::path::Path| -> Result<talk::Found, String> {
        let data = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
        Ok(talk::Found { source: talk::Source::Json { path: path.to_path_buf(), style }, data })
    };
    let feminine = made.get(1).map(|f| found(f)).transpose()?;
    app.log.info(format!(
        "nasher.cfg: a target packs {}",
        made.iter().map(|f| f.display().to_string()).collect::<Vec<_>>().join(" and ")
    ));
    Table::open(name, found(&made[0])?, feminine)
}

/// A line as its fields show it: with plain line ends
/// ([`crate::text::to_editor`]).
pub(crate) fn shown_line(line: &Line) -> Line {
    let plain = |s: &str| crate::text::to_editor(s).0;
    Line { text: plain(&line.text), feminine: line.feminine.as_deref().map(plain), ..line.clone() }
}

/// A line as typed in its fields, to set where the table has `stored`:
/// with the line ends of the stored text, those of its lines kept, plain
/// for a text that had none (the game's own talk table has no CRLF).
pub(crate) fn stored_line(typed: &Line, stored: &Line) -> Line {
    let ends = |typed: &str, stored: &str| crate::text::with_line_ends_or(typed, stored, false);
    let feminine = (typed.feminine.as_deref())
        .map(|f| ends(f, stored.feminine.as_deref().unwrap_or_default()));
    Line { text: ends(&typed.text, &stored.text), feminine, ..typed.clone() }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// What the editor keeps of the table's lines (those with text, those
    /// the Filter finds, the list) is, after a line is set, what a look
    /// through every line makes of it: for a text typed, emptied, typed
    /// to what is looked for and away from it, with and without Only
    /// Lines with Text. The list of every line is not made again for a
    /// letter.
    #[test]
    fn a_line_set_keeps_the_lists_as_a_look_through_makes_them() {
        let dir = mg_testkit::scratch_dir("ui-talk-line-changed");
        let mut t = Table::create("talk", &dir, mg_core::Language::ENGLISH, false);
        let line = |text: &str| Line {
            text: text.into(),
            feminine: None,
            sound: String::new(),
            sound_length: 0.0,
        };
        for (row, text) in [(0, "The gate"), (2, "A goblin"), (5, "The gatehouse"), (9, "")] {
            t.put_line(row, &line(text)).unwrap();
        }
        let texted = |t: &Table| (0..t.len()).filter(|r| has_text(t, *r)).collect::<Vec<_>>();
        for query in ["", "gate", "5", "the gate"] {
            let all: Vec<usize> = (0..t.len()).collect();
            let listed = std::sync::Arc::new(all.clone());
            let mut v = TalkView {
                revision: 7,
                texted: Some((7, std::sync::Arc::new(texted(&t)))),
                found: Some((query.to_string(), 7, t.find(query))),
                shown: Some(((7, t.len(), String::new(), None), listed.clone())),
                ..Default::default()
            };
            for (row, text) in
                [(3, "g"), (3, "ga"), (3, "gate"), (3, "gat"), (3, ""), (0, "The door"), (5, "")]
            {
                let before = t.line(row);
                t.set_line(row, &line(text), Some(0)).unwrap();
                v.line_changed(&t, row);
                let (of, kept) = v.texted.as_ref().unwrap();
                assert_eq!((*of, &**kept), (v.revision, &texted(&t)), "{query:?}, {row}: {text:?}");
                let (_, of, found) = v.found.as_ref().unwrap();
                assert_eq!((*of, found), (v.revision, &t.find(query)), "{query:?}, {row}: {text}");
                // (Every line listed: the same list, not one made again.)
                let ((of, ..), rows) = v.shown.as_ref().unwrap();
                assert_eq!(*of, v.revision);
                assert!(std::sync::Arc::ptr_eq(rows, &listed) && **rows == all);
                t.set_line(row, &before, None).unwrap();
                v.line_changed(&t, row);
            }
            // (What was kept of an older table is left for the next frame
            // to make again.)
            v.texted = Some((1, Default::default()));
            v.line_changed(&t, 0);
            assert_eq!(v.texted, Some((1, Default::default())));
        }
        assert!(t.finds(0, "gate") && !t.finds(2, "gate") && !t.finds(99, ""));
    }
}
