//! Problems (M4's U20): the Bottom Dock's global panel of findings, shown
//! whatever the editor in front, with its switch in the Status Bar between
//! the Log's and the editor's own (every board draws it; the NUI board's
//! before Checks). It holds what each **source** found last, a source's
//! findings replacing what it found before (never appended to for ever):
//! the tileset editor's Check, one source for each tileset file, and (M4's
//! U28, the owner's Q4 and Q5) Build Module's results and Verify Module's
//! findings, each a source of its own: Verify's with the fix it offers
//! beside it (the log keeps its lines and their fixes); Build's heading
//! with Counted as Used, and its Unused results' Mark as Used (their menu,
//! or the Build dialog's button with one chosen). Their findings are the
//! module's: stale once it changed (its revision), let go of as it closes.
//! A finding is its severity, where it is (in words), its
//! message and a way to go there ([`Go`]); a click on its place goes there
//! (a tileset's file opened or brought forward, its page, its row, the field
//! it names). The switch's badge counts the errors and warnings of every
//! source whose findings are current, in `error` where one is an error, else
//! in `warn`.
//!
//! What a source found stays until it finds again, or is removed (its
//! heading's menu, or Delete on it): a tileset whose text is no longer what
//! was checked keeps its findings listed, marked ([`Stale`]) and left out of
//! the count, until Check runs again or the text is what it was; a tileset
//! closed keeps them too (a click opens the file again), as does closing the
//! module (a tileset's file is not the module's); a file no longer found is
//! said to be.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use egui::Ui;
use mg_module::doctor::Severity;

use crate::Moonglow;
use crate::row_copy::{self, Asked as RowCopy};

/// The accessible name of Problems' body (its rows, or the words saying
/// what fills it): apart from the panel's tab and switch, which share its
/// name.
pub const PROBLEM_ROWS: &str = "Problem rows";

/// Who found a set of findings: what replaces them when it finds again.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Source {
    /// The tileset editor's Check of the tileset file at this path.
    Tileset(PathBuf),
    /// Build › Build Module's results (M4's U28, the owner's Q4): what it
    /// found missing, failing to compile and, asked, unused.
    Build,
    /// Build › Verify Module's findings (U28, Q5), each with the fix it
    /// offers; the log keeps its lines.
    Verify,
}

impl Source {
    /// Its name over its findings: the tileset's file name.
    pub fn title(&self) -> String {
        match self {
            Source::Tileset(path) => file_name(path),
            Source::Build => "Build Module".into(),
            Source::Verify => "Verify Module".into(),
        }
    }
}

fn file_name(path: &Path) -> String {
    path.file_name().map_or_else(|| path.display().to_string(), |n| n.to_string_lossy().into())
}

/// Where a finding goes when its place is clicked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Go {
    /// A place in a tileset file: its editor opened (or brought forward) on
    /// the page, row and field of `place`.
    Tileset { path: PathBuf, place: crate::tileset_view::Place },
    /// A resource of the module: its editor opened (Build's and Verify's,
    /// as a double click on a Build result opened it).
    Resource(mg_resman::ResKey),
}

/// One finding as Problems lists it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Problem {
    pub severity: Severity,
    /// Where it is, in words ("Tile 40, Model").
    pub at: String,
    pub message: String,
    /// Where a click on its place goes (none: a finding of the whole file).
    pub go: Option<Go>,
    /// The log's line whose fix it offers (Verify's: `Log::fix_at`), the
    /// fix's button beside it while the log has it.
    pub fix: Option<usize>,
    /// Of a Build's Unused result, what is unused (Mark as Used takes it).
    pub unused: Option<mg_module::verify::Unused>,
}

impl Problem {
    /// A finding with no fix and nothing unused.
    pub fn new(severity: Severity, at: String, message: String, go: Option<Go>) -> Problem {
        Problem { severity, at, message, go, fix: None, unused: None }
    }
}

/// Why a source's findings may no longer be so.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stale {
    /// The text is not what was checked (open, or the file on disk).
    Changed,
    /// Closed with changes let go of while they were not what was checked.
    Closed,
    /// The file is not found where it was.
    Missing,
}

impl Stale {
    /// Its words, in its set's heading.
    pub fn words(self) -> &'static str {
        match self {
            Stale::Changed => "Changed since the check",
            Stale::Closed => "Closed without saving",
            Stale::Missing => "File not found",
        }
    }
}

/// What one source found last.
#[derive(Debug, Clone)]
pub struct Set {
    pub source: Source,
    pub problems: Vec<Problem>,
    /// Why the findings may no longer be so; none while they are current.
    pub stale: Option<Stale>,
    /// What the source was when it found them: a tileset's text, hashed
    /// ([`fingerprint`]), so that its findings are current again where the
    /// text is what was checked.
    pub checked: Option<u64>,
    /// Its errors, counted as it was set.
    errors: usize,
}

impl Set {
    /// Its errors and warnings.
    pub fn counts(&self) -> (usize, usize) {
        (self.errors, self.problems.len() - self.errors)
    }
}

/// A text's fingerprint, what a source's findings were found of.
pub fn fingerprint(bytes: &[u8]) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    bytes.hash(&mut h);
    h.finish()
}

/// Problems' contents: each source's findings, in the order the sources
/// first reported.
#[derive(Debug, Default)]
pub struct Problems {
    sets: Vec<Set>,
    /// Counted up at every change: the rows are made again only then.
    revision: u64,
    /// The errors and warnings of every current source, as last counted.
    counted: (usize, usize),
    /// How many times the rows were made (for the tests: a frame that
    /// changes nothing makes none).
    row_builds: u64,
    /// Problems asked to the front by a source that found something,
    /// opening the Bottom Dock where `true` (`shell::bottom_forward`, at the
    /// next frame's start).
    pub(crate) forward: Option<bool>,
    /// Build's or Verify's findings brought to the front by their command
    /// (M4's U28 review): the panel takes the keys as it is drawn next, held
    /// while the pointer moves over an area's view.
    pub(crate) take_keys: bool,
    /// Whether the build shown in Problems was asked for its Unused results
    /// (its Counted as Used shown then, as the tip's dialog showed it with
    /// Unused ticked).
    #[doc(hidden)]
    pub build_unused: bool,
    /// The rows as listed, for the revision they were made at.
    rows: Option<(u64, Arc<Vec<Row>>)>,
    /// The pass the panel was last drawn in and the revision it showed:
    /// a change made after it in that pass (a tileset drawn after the
    /// Bottom Dock marked its findings stale) asks for the next frame, once
    /// (`shell::follow_bottom`).
    pub(crate) drawn: Option<(u64, u64)>,
    /// The finding chosen (a click on its row, the arrows): its source and
    /// its place among the source's findings. Build Module's Mark as Used
    /// takes it.
    selected: Option<(Source, usize)>,
    /// The selection moved by the keys this frame: scrolled into sight.
    scroll_to: bool,
    /// Shift+F10 asked for the chosen finding's menu: opened as its row
    /// is drawn (brought into sight first, where it was not).
    menu_keyed: bool,
    /// Verify's Fix All: the first finding of each check with more than
    /// one fix to make, and how many, for the log's fixes as they were
    /// (`Log::fixes_revision`): worked out as they change, not each frame.
    fix_all: Option<(u64, Arc<FixAll>)>,
}

/// Verify's Fix All buttons: the finding each stands beside, its check and
/// how many fixes.
type FixAll = Vec<(usize, String, usize)>;

/// A row of the panel: a source's heading, or one of its findings.
#[derive(Debug, Clone)]
enum Row {
    Heading { set: usize },
    Problem { set: usize, at: usize },
}

impl Problems {
    /// What `source` found, in place of what it found before (none: it
    /// found nothing, said in the panel).
    /// `checked`: the fingerprint of what it found them in, where it has
    /// one.
    pub fn replace(&mut self, source: Source, mut problems: Vec<Problem>, checked: Option<u64>) {
        // (Errors first: they are what stops the game.)
        problems.sort_by_key(|p| p.severity != Severity::Error);
        let errors = problems.iter().filter(|p| p.severity == Severity::Error).count();
        let set = Set { source, problems, stale: None, checked, errors };
        if self.selected.as_ref().is_some_and(|(s, _)| *s == set.source) {
            self.selected = None;
        }
        match self.sets.iter_mut().find(|s| s.source == set.source) {
            Some(was) => *was = set,
            None => self.sets.push(set),
        }
        self.changed();
    }

    /// `source`'s findings said to be no longer so (`why`), or current again
    /// (`None`), where it has any.
    pub fn set_stale(&mut self, source: &Source, why: Option<Stale>) {
        if let Some(set) = self.sets.iter_mut().find(|s| s.source == *source)
            && set.stale != why
        {
            set.stale = why;
            self.changed();
        }
    }

    /// `source`'s findings taken away (its heading's Remove from Problems).
    pub fn remove(&mut self, source: &Source) {
        if self.selected.as_ref().is_some_and(|(s, _)| s == source) {
            self.selected = None;
        }
        let before = self.sets.len();
        self.sets.retain(|s| s.source != *source);
        if self.sets.len() != before {
            self.changed();
        }
    }

    /// `source`'s findings that `keep` keeps, the others taken away
    /// (Build's Unused results a Mark as Used counts as used now), the
    /// chosen one kept chosen where it stays.
    pub fn retain(&mut self, source: &Source, keep: impl Fn(&Problem) -> bool) {
        let Some(set) = self.sets.iter_mut().find(|s| s.source == *source) else { return };
        let chosen = (self.selected.as_ref())
            .filter(|(s, _)| s == source)
            .and_then(|(_, at)| set.problems.get(*at).cloned());
        let before = set.problems.len();
        set.problems.retain(|p| keep(p));
        if set.problems.len() == before {
            return;
        }
        set.errors = set.problems.iter().filter(|p| p.severity == Severity::Error).count();
        if self.selected.as_ref().is_some_and(|(s, _)| s == source) {
            self.selected = (chosen.and_then(|c| set.problems.iter().position(|p| *p == c)))
                .map(|at| (source.clone(), at));
        }
        self.changed();
    }

    /// The finding chosen, if one is.
    pub fn selected(&self) -> Option<&Problem> {
        let (source, at) = self.selected.as_ref()?;
        self.of(source)?.problems.get(*at)
    }

    /// The finding chosen, by its source and place among its findings.
    pub fn select(&mut self, at: Option<(Source, usize)>) {
        self.selected = at;
    }

    /// What `source` found last.
    pub fn of(&self, source: &Source) -> Option<&Set> {
        self.sets.iter().find(|s| s.source == *source)
    }

    /// Every source's findings, in the order the sources first reported.
    pub fn sets(&self) -> &[Set] {
        &self.sets
    }

    /// Counted up at every change.
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// How many times the panel's rows were made.
    #[doc(hidden)]
    pub fn row_builds(&self) -> u64 {
        self.row_builds
    }

    /// The errors and warnings of every source whose findings are current
    /// (counted as they change: the switch's badge asks each frame); stale
    /// ones are listed, not counted.
    pub fn counts(&self) -> (usize, usize) {
        self.counted
    }

    /// The errors and warnings counted again, the rows to be made again.
    fn changed(&mut self) {
        let current = self.sets.iter().filter(|s| s.stale.is_none()).map(Set::counts);
        self.counted = current.fold((0, 0), |a, b| (a.0 + b.0, a.1 + b.1));
        self.revision += 1;
    }

    /// Problems brought to the front (the Bottom Dock opened on it where
    /// `open`), at the next frame's start.
    pub(crate) fn ask_forward(&mut self, open: bool) {
        self.forward = Some(self.forward.unwrap_or(false) || open);
    }

    /// Every finding listed, as a finding's Copy All copies them: one a
    /// line ([`copy_line`]), in the order listed (each source's,
    /// errors first), each saying who found it, so that a line pasted
    /// alone still says it.
    pub fn text(&self) -> String {
        let all = self.sets.iter().flat_map(|s| s.problems.iter().map(move |p| copy_line(s, p)));
        row_copy::lines(all)
    }

    /// One source's findings, as its heading's Copy These Findings copies
    /// them and its Save These Findings As… writes them: [`Problems::text`]'s
    /// lines of that source alone (none listed: empty).
    pub fn text_of(&self, source: &Source) -> String {
        let Some(s) = self.of(source) else { return String::new() };
        row_copy::lines(s.problems.iter().map(|p| copy_line(s, p)))
    }

    /// The rows as listed: each source with findings or a word of its own
    /// (a heading), then its findings.
    fn rows(&mut self) -> Arc<Vec<Row>> {
        if let Some((at, rows)) = &self.rows
            && *at == self.revision
        {
            return rows.clone();
        }
        let mut rows = Vec::new();
        for (set, s) in self.sets.iter().enumerate() {
            rows.push(Row::Heading { set });
            rows.extend((0..s.problems.len()).map(|at| Row::Problem { set, at }));
        }
        let rows = Arc::new(rows);
        self.row_builds += 1;
        self.rows = Some((self.revision, rows.clone()));
        rows
    }
}

/// Each frame's start: Build's and Verify's findings marked stale while
/// the module is not what they were found in (its revision), current again
/// where it is (two numbers compared, nothing walked).
pub(crate) fn follow(app: &mut Moonglow) {
    let now = app.ws.as_ref().map(|ws| ws.revision());
    for source in [Source::Build, Source::Verify] {
        let Some(set) = app.problems.of(&source) else { continue };
        let why = (set.checked != now).then_some(Stale::Changed);
        app.problems.set_stale(&source, why);
    }
}

/// A finding as one line of text (its menu's Copy, Ctrl+C, and a line of
/// Copy All, Copy These Findings and Save These Findings As…): its kind, its place, its whole message (never as
/// the row cuts it) and who found it, "Error: chest_01.utp, The model is
/// missing (Verify Module)"; its place said once where the message says it
/// (Verify's say the check and the resource), and the source's findings
/// said to be stale where they are.
pub fn copy_line(set: &Set, p: &Problem) -> String {
    let word = if p.severity == Severity::Error { "Error" } else { "Warning" };
    let source = match set.stale {
        Some(why) => format!("{}, {}", set.source.title(), why.words().to_lowercase()),
        None => set.source.title(),
    };
    match p.at.is_empty() || p.message.contains(&p.at) {
        true => format!("{word}: {} ({source})", p.message),
        false => format!("{word}: {}, {} ({source})", p.at, p.message),
    }
}

/// Problems' Save These Findings As… (a heading's menu): that source's
/// findings ([`Problems::text_of`]) into a text file, where the builder
/// says (beside the module, at first).
pub(crate) fn save_as(app: &mut Moonglow, source: &Source) {
    let dir = app.module_path().and_then(|p| p.parent().map(Path::to_path_buf));
    let suggested = dir.map(|d| d.join("problems.txt"));
    let kind = crate::dialogs::FileKind::Problems;
    let Some(path) = app.dialogs.save_file(kind, suggested.as_deref()) else { return };
    match mg_module::write_replacing(&path, app.problems.text_of(source).as_bytes()) {
        Ok(()) => app.log.info(format!("Saved Problems to {}", path.display())),
        Err(e) => app.log.error(format!("Could not write {}: {e}", path.display())),
    }
}

/// "1 error", "3 warnings".
pub(crate) fn count(n: usize, what: &str) -> String {
    match n {
        1 => format!("1 {what}"),
        n => format!("{n} {what}s"),
    }
}

/// The Problems switch's badge: every source's errors and warnings
/// together, in `error` where one is an error, else in `warn`; none with
/// nothing found.
pub(crate) fn badge(app: &Moonglow) -> Option<mg_uikit::Badge> {
    let (errors, warnings) = app.problems.counts();
    let kind = if errors > 0 { mg_uikit::BadgeKind::Error } else { mg_uikit::BadgeKind::Warn };
    (errors + warnings > 0).then(|| mg_uikit::Badge::count(errors + warnings).kind(kind))
}

/// "2 errors · 1 warning", "2 errors", "1 warning" (none: `None`).
pub(crate) fn counted(errors: usize, warnings: usize) -> Option<String> {
    match (errors, warnings) {
        (0, 0) => None,
        (e, 0) => Some(count(e, "error")),
        (0, w) => Some(count(w, "warning")),
        (e, w) => Some(format!("{} · {}", count(e, "error"), count(w, "warning"))),
    }
}

/// What the panel's bar says after its tab: "2 errors · 1 warning", of the
/// current findings.
pub(crate) fn detail(app: &Moonglow) -> Option<String> {
    let (errors, warnings) = app.problems.counts();
    counted(errors, warnings)
}

/// What Problems says with nothing in it: its heading and what fills it.
pub const NOTHING: &str = "No problems found";
pub const FILLED_BY: &str =
    "A tileset's Check, Build Module and Verify Module list what they find here.";

/// Problems' body (`shell::panel_body`, the Bottom Dock's): each source's
/// heading (its name, its counts, and whether its findings may be stale),
/// then its findings, errors first, each a row of its kind's mark and word,
/// its place (a link that goes there, where it has one), its message cut
/// to its line and whole in its tip, and Verify's fix beside it; only the
/// rows in sight laid out. A click on a finding's row chooses it, as the
/// arrows do with the panel's keys (Enter goes to it); Build Module's Mark
/// as Used takes the one chosen; Ctrl+C copies it ([`copy_line`]), its
/// menu Copy All. With nothing found anywhere, the kit's empty state.
pub(crate) fn body(app: &mut Moonglow, ui: &mut Ui) {
    mg_uikit::name_group(ui, PROBLEM_ROWS);
    let keys = crate::palette_view::Keys::Problems;
    crate::palette_view::keys_outline(ui, ui.max_rect(), keys);
    if ui.input(|i| i.pointer.any_pressed()) && ui.rect_contains_pointer(ui.max_rect()) {
        crate::palette_view::give_keys(ui.ctx(), keys);
    }
    let focus = app.dialog_panel_focus.take_if(|k| *k == keys).is_some();
    if std::mem::take(&mut app.problems.take_keys) && !app.problems.sets.is_empty() {
        crate::palette_view::hold_keys(ui.ctx(), keys);
    }
    let t = mg_uikit::tokens(ui.ctx());
    app.problems.drawn = Some((ui.ctx().cumulative_pass_nr(), app.problems.revision()));
    let running = running(app);
    if app.problems.sets.is_empty() {
        empty(ui, &t, running.as_ref());
        return;
    }
    let rows = app.problems.rows();
    let fix_all = fix_all(app);
    let place = crate::build_view::used_place(app);
    let build_unused = app.problems.build_unused;
    let has = crate::report_table::has_keys(ui.ctx(), keys);
    let line = mg_uikit::metrics::CONTROL_HEIGHT + 6.0;
    let mut went = keyed(app, ui, &rows, has, line);
    let mut remove = None;
    let mut chosen = None;
    let mut asked = Asked::default();
    let mut first = focus;
    let scroll_to = std::mem::take(&mut app.problems.scroll_to);
    let menu_keyed = app.problems.menu_keyed;
    let mut menu_drawn = false;
    let probs = &app.problems;
    let sets = &probs.sets;
    let log = &app.log;
    let margin = egui::Margin { left: 8, right: 8, top: 2, bottom: 2 };
    egui::Frame::new().inner_margin(margin).show(ui, |ui| {
        let mut area = egui::ScrollArea::vertical()
            .id_salt("problem-rows")
            .auto_shrink([false, false])
            // (No taller than its room: egui's least would lay rows out past
            // the dock's bottom.)
            .min_scrolled_height(0.0);
        if scroll_to && let Some(at) = selected_row(probs, &rows) {
            let room = ui.available_height();
            area = area.vertical_scroll_offset((at as f32 * line - room / 2.0 + line).max(0.0));
        }
        mg_uikit::with_rows(ui, |ui| {
            // (Each row `line` high and that far apart: the room
            // `show_rows` gives the rows out of sight is theirs, and the
            // last one is in reach.)
            ui.spacing_mut().interact_size.y = line;
            // (The Bottom Dock's least height: a heading and a finding.)
            crate::shell::first_row_ends(ui, ui.cursor().top() + 2.0 * line);
            area.show_rows(ui, line, rows.len(), |ui, shown| {
                for row in &rows[shown] {
                    match *row {
                        Row::Heading { set } => {
                            let s = &sets[set];
                            let look = HeadingLook {
                                placed: place.is_some(),
                                running: running.as_ref() == Some(&s.source),
                                counted: s.source == Source::Build && build_unused,
                            };
                            let r = ui.push_id(("heading", set), |ui| {
                                heading(ui, &t, s, line, &look, &mut asked)
                            });
                            match r.inner {
                                Some(HeadingAsked::Remove) => remove = Some(s.source.clone()),
                                Some(HeadingAsked::CopyAll) => {
                                    asked.copy_these = Some(s.source.clone());
                                }
                                Some(HeadingAsked::Save) => asked.save = Some(s.source.clone()),
                                None => {}
                            }
                        }
                        Row::Problem { set, at } => {
                            let s = &sets[set];
                            let p = &s.problems[at];
                            let is = probs.selected.as_ref() == Some(&(s.source.clone(), at));
                            let fix = p.fix.and_then(|l| log.fix_at(l));
                            let all = p.fix.and_then(|l| fix_all.iter().find(|f| f.0 == l));
                            let look = Look {
                                selected: is,
                                ringed: is && has,
                                keyed: is && menu_keyed,
                                fix,
                                all,
                                place: place.is_some(),
                            };
                            menu_drawn |= is;
                            let did = ui.push_id((set, at), |ui| {
                                problem(ui, &t, p, line, &mut first, &look)
                            });
                            let did = did.inner;
                            if did.go {
                                went = p.go.clone();
                            }
                            if did.chosen {
                                chosen = Some((s.source.clone(), at));
                            }
                            if did.fix {
                                asked.fix = p.fix;
                            }
                            if did.fix_all {
                                asked.fix_all = all.map(|f| f.1.clone());
                            }
                            if did.mark
                                && let Some(u) = p.unused
                            {
                                asked.mark = Some(u);
                            }
                            match did.copy {
                                Some(RowCopy::Row) => asked.copy = Some(copy_line(s, p)),
                                Some(RowCopy::All) => asked.copy_all = true,
                                None => {}
                            }
                        }
                    }
                }
            });
        });
    });
    // (The keyed menu opened where its row was drawn; else asked for again
    // next frame, the row brought into sight.)
    if menu_keyed && (menu_drawn || app.problems.selected.is_none()) {
        app.problems.menu_keyed = false;
    } else if menu_keyed {
        app.problems.scroll_to = true;
        ui.ctx().request_repaint();
    }
    if let Some(at) = chosen {
        app.problems.selected = Some(at);
        crate::palette_view::give_keys(ui.ctx(), keys);
    }
    if let Some(text) = asked.copy {
        ui.ctx().copy_text(text);
    }
    if asked.copy_all {
        ui.ctx().copy_text(app.problems.text());
    }
    if let Some(source) = asked.copy_these {
        ui.ctx().copy_text(app.problems.text_of(&source));
    }
    if let Some(source) = asked.save {
        save_as(app, &source);
    }
    if let Some(source) = remove {
        app.problems.remove(&source);
    }
    if let Some(line) = asked.fix {
        app.actions.push(crate::Action::ApplyFix(line));
    }
    if let Some(check) = asked.fix_all {
        app.actions.push(crate::Action::ApplyFixesOf(check));
    }
    if let Some(u) = asked.mark {
        crate::build_view::mark_used(app, u);
    }
    if let Some(button) = &asked.counted {
        crate::build_view::counted_as_used(app, button);
    }
    if let Some(to) = went {
        go_to(app, &to);
    }
}

/// What a frame's rows asked for.
#[derive(Default)]
struct Asked {
    /// The fix beside this log line made.
    fix: Option<usize>,
    /// Every fix of this check made (Fix All).
    fix_all: Option<String>,
    /// A Build's Unused result counted as used (its row's menu).
    mark: Option<mg_module::verify::Unused>,
    /// Build's heading's Counted as Used button: its popup, drawn after
    /// the rows.
    counted: Option<egui::Response>,
    /// A finding's line copied (its menu's Copy).
    copy: Option<String>,
    /// Every finding copied (a finding's Copy All).
    copy_all: bool,
    /// A source's findings copied (its heading's Copy These Findings).
    copy_these: Option<Source>,
    /// A source's findings saved into a file (its heading's Save These
    /// Findings As…).
    save: Option<Source>,
}

/// The place of the finding chosen among the rows.
fn selected_row(p: &Problems, rows: &[Row]) -> Option<usize> {
    let (source, at) = p.selected.as_ref()?;
    let set = p.sets.iter().position(|s| s.source == *source)?;
    rows.iter().position(|r| matches!(*r, Row::Problem { set: s, at: a } if s == set && a == *at))
}

/// The panel's keys, where it has them (nothing over it): Up and Down,
/// Page Up and Page Down, Home and End move the finding chosen; Enter
/// goes where it is. Where to go.
fn keyed(app: &mut Moonglow, ui: &mut Ui, rows: &[Row], has: bool, line: f32) -> Option<Go> {
    use egui::Key;
    if !has {
        return None;
    }
    let problem_rows = || rows.iter().enumerate().filter(|(_, r)| matches!(r, Row::Problem { .. }));
    let page = ((ui.available_height() / line) as isize - 1).max(1);
    let now = selected_row(&app.problems, rows);
    let steps = [
        (Key::ArrowDown, 1isize),
        (Key::ArrowUp, -1),
        (Key::PageDown, page),
        (Key::PageUp, -page),
        (Key::End, isize::MAX / 2),
        (Key::Home, isize::MIN / 2),
    ];
    let pressed =
        ui.input_mut(|i| steps.into_iter().find(|(k, _)| i.consume_key(egui::Modifiers::NONE, *k)));
    if let Some((_, by)) = pressed {
        // (By findings, the headings passed over: the place among the
        // findings is worked out on the rows made for this revision, only
        // as a key is pressed.)
        let all: Vec<usize> = problem_rows().map(|(i, _)| i).collect();
        if all.is_empty() {
            return None;
        }
        let last = all.len() as isize - 1;
        let from = now
            .and_then(|r| all.iter().position(|x| *x == r))
            .map_or(if by > 0 { -1 } else { last + 1 }, |x| x as isize);
        let to = all[from.saturating_add(by).clamp(0, last) as usize];
        if let Row::Problem { set, at } = rows[to] {
            app.problems.selected = Some((app.problems.sets[set].source.clone(), at));
            app.problems.scroll_to = true;
        }
        return None;
    }
    if now.is_some() && ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::Enter)) {
        return app.problems.selected().and_then(|p| p.go.clone());
    }
    // Ctrl+C copies the finding chosen (its line made only then);
    // Shift+F10 opens its menu.
    if let Some((source, at)) = app.problems.selected.clone()
        && let Some(set) = app.problems.of(&source)
        && let Some(p) = set.problems.get(at)
    {
        if row_copy::copy_pressed(ui) {
            ui.ctx().copy_text(copy_line(set, p));
        }
        if row_copy::menu_key(ui) {
            app.problems.menu_keyed = true;
        }
    }
    None
}

/// Verify's Fix All beside the first finding of each check with more than
/// one fix to make: made again only as the log's fixes change.
fn fix_all(app: &mut Moonglow) -> Arc<FixAll> {
    let now = app.log.fixes_revision;
    if let Some((at, firsts)) = &app.problems.fix_all
        && *at == now
    {
        return firsts.clone();
    }
    let mut firsts: Vec<(usize, String, usize)> = Vec::new();
    for o in app.log.fixes.iter().filter(|o| !o.made && o.fix.asks.is_none()) {
        match firsts.iter_mut().find(|(_, c, _)| *c == o.check) {
            Some(f) => f.2 += 1,
            None => firsts.push((o.line, o.check.clone(), 1)),
        }
    }
    firsts.retain(|f| f.2 > 1);
    let firsts = Arc::new(firsts);
    app.problems.fix_all = Some((now, firsts.clone()));
    firsts
}

/// The name of a heading's Remove from Problems (its menu's row).
pub const REMOVE: &str = "Remove from Problems";

/// The source whose job runs now (Build Module's, Verify Module's): what
/// it found is not known yet.
fn running(app: &Moonglow) -> Option<Source> {
    match app.job_shown()?.title.as_str() {
        "Build Module" => Some(Source::Build),
        "Verify Module" => Some(Source::Verify),
        _ => None,
    }
}

/// What Problems says while `source`'s job runs, in place of "No problems
/// found" ("Verify Module is running").
pub fn running_words(source: &Source) -> String {
    format!("{} is running", source.title())
}

/// The kit's empty state (`.empty`): the problems icon, what it is, what
/// fills it, in the middle of the panel; while Build or Verify runs, that
/// it runs (nothing is known found or not yet).
fn empty(ui: &mut Ui, t: &mg_uikit::Tokens, running: Option<&Source>) {
    let room = ui.available_rect_before_wrap();
    ui.scope_builder(egui::UiBuilder::new().max_rect(room), |ui| {
        ui.vertical_centered(|ui| {
            let high = 32.0 + 8.0 + 2.0 * ui.text_style_height(&egui::TextStyle::Body);
            ui.add_space(((room.height() - high) / 2.0).max(4.0));
            let (rect, _) = ui.allocate_exact_size(egui::vec2(32.0, 32.0), egui::Sense::hover());
            mg_uikit::Icon::Problems.paint(ui.painter(), rect, t.text_3);
            ui.add_space(4.0);
            match running {
                Some(source) => {
                    ui.label(egui::RichText::new(running_words(source)).color(t.text_2));
                }
                None => {
                    ui.label(egui::RichText::new(NOTHING).color(t.text_2));
                    ui.label(egui::RichText::new(FILLED_BY).color(t.text_3));
                }
            }
        });
    });
}

/// Why a source's findings may be stale, in its heading's tip: true of
/// the source (Check, Build, Verify again).
fn again(source: &Source) -> &'static str {
    match source {
        Source::Tileset(_) => {
            "What it says is of the file as it was checked: Check again to see it now"
        }
        Source::Build => "What it says is of the module as it was built: Build again to see it now",
        Source::Verify => {
            "What it says is of the module as it was verified: Verify again to see it now"
        }
    }
}

/// The words of Build's heading's button, which shows what the project
/// file counts as used.
pub const COUNTED_AS_USED: &str = "Counted as Used";

/// How a source's heading is drawn: whether the module has a place for
/// the project file (Counted as Used's tip), whether its job runs now, and
/// whether it shows Counted as Used (Build's, asked for Unused).
struct HeadingLook {
    placed: bool,
    running: bool,
    counted: bool,
}

/// A source's heading row: its name (its counts are the tab bar's, said
/// once: M4's U28 review), "No problems found" where it found none or that
/// it runs again, and why its findings may be stale (then not counted, it
/// says); Build's, its Counted as Used where the build was asked for its
/// Unused results (as the tip's dialog showed it, with Unused ticked); at
/// its end its More button. A row of its own to focus (Tab): its menu (the
/// More button, a right click, or Shift+F10 with it focused) offers Copy
/// All and Save As… (every finding listed: Verify writes no file of its
/// own) and Remove from Problems, which Delete with it focused does. What
/// it was asked for.
fn heading(
    ui: &mut Ui,
    t: &mg_uikit::Tokens,
    set: &Set,
    line: f32,
    look: &HeadingLook,
    asked: &mut Asked,
) -> Option<HeadingAsked> {
    let words = match (look.running, set.problems.is_empty()) {
        (true, _) => Some(running_words(&set.source)),
        (false, true) => Some(NOTHING.to_string()),
        (false, false) => None,
    };
    let placed = look.placed;
    let mut button = None;
    let mut more = None;
    // (The heading's own response first, under what is on it: Counted as
    // Used's click is the button's, not the heading's.)
    let top = ui.cursor().min;
    let rect = egui::Rect::from_min_max(top, egui::pos2(ui.max_rect().right(), top.y + line));
    let id = ui.id().with("problems-heading");
    let r = ui.interact(rect, id, egui::Sense::click() | egui::Sense::FOCUSABLE);
    ui.horizontal(|ui| {
        ui.set_min_height(line);
        ui.label(egui::RichText::new(set.source.title()).strong().color(t.text));
        if let Some(words) = &words {
            ui.label(egui::RichText::new(words).color(t.text_3));
        }
        if let Some(why) = set.stale {
            let said = format!("{}: not counted", why.words());
            ui.label(egui::RichText::new(said).color(t.warn)).on_hover_text(again(&set.source));
        }
        if look.counted {
            let b = ui.add(egui::Button::new(COUNTED_AS_USED).small());
            let tip = if placed {
                "What the project file counts as used, and a field to add a name or glob"
            } else {
                crate::build_view::NO_PLACE
            };
            button = Some(b.on_hover_text(tip));
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            // (The icon alone, its name in its tip and for a screen reader:
            // a heading's words and Counted as Used keep the room.)
            let icon = mg_uikit::IconButton::new(mg_uikit::Icon::MoreHorizontal, MORE);
            let b = icon.icon_only(true).small();
            more = Some(
                ui.add(b)
                    .on_hover_text("Problems actions: copy or save every finding, or remove these"),
            );
        });
    });
    asked.counted = asked.counted.take().or(button);
    let said = match &words {
        Some(w) => format!("{}, {w}", set.source.title()),
        None => set.source.title(),
    };
    r.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Other, true, &said));
    if r.has_focus() {
        ui.painter().rect_stroke(
            rect,
            2.0,
            egui::Stroke::new(1.0, t.accent),
            egui::StrokeKind::Inside,
        );
    }
    let deleted =
        r.has_focus() && ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Delete));
    let mut asked = deleted.then_some(HeadingAsked::Remove);
    // (Its menu by a right click or Shift+F10, at the pointer or under its
    // start; the More button's under the button.)
    let keyed = r.has_focus() && row_copy::menu_key(ui);
    let set_open = (r.secondary_clicked() || keyed).then_some(egui::SetOpenCommand::Bool(true));
    let at_id = r.id.with("menu-at");
    if set_open.is_some() {
        let pointer = ui.ctx().pointer_interact_pos().filter(|_| !keyed);
        ui.data_mut(|d| d.insert_temp(at_id, pointer.unwrap_or(rect.left_bottom())));
    }
    let at = ui.data(|d| d.get_temp::<egui::Pos2>(at_id)).unwrap_or(rect.left_bottom());
    let menu = egui::Popup::menu(&r).open_memory(set_open).at_position(at);
    menu.style(mg_uikit::menu_style).show(|ui| heading_menu(ui, &set.source, keyed, &mut asked));
    if let Some(more) = &more {
        let menu = egui::Popup::menu(more).style(mg_uikit::menu_style);
        menu.show(|ui| heading_menu(ui, &set.source, false, &mut asked));
    }
    asked
}

/// What closes Verify's line in the log where it found anything: where
/// its findings are, as Verify writes no file of its own.
pub const LISTED: &str = ". Listed in Problems, where a right click copies or saves them";

/// The name of a heading's More button.
pub const MORE: &str = "Problems Actions";
/// The heading's menu's Copy These Findings (its source's findings, one a
/// line).
pub const COPY_THESE: &str = "Copy These Findings";
/// The heading's menu's Save These Findings As… (its source's findings
/// into a text file).
pub const SAVE_AS: &str = "Save These Findings As…";

/// What a source's heading was asked for (its menu, its keys).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HeadingAsked {
    Remove,
    CopyAll,
    Save,
}

/// A heading's menu, each item on that heading's findings alone: Copy
/// These Findings and Save These Findings As…, then Remove from Problems;
/// the first item taking the keyboard where Shift+F10 opened it.
fn heading_menu(ui: &mut Ui, source: &Source, keyed: bool, asked: &mut Option<HeadingAsked>) {
    mg_uikit::menu_rows(ui);
    let whose = source.title();
    let copy = ui
        .button(COPY_THESE)
        .on_hover_text(format!("Copy the findings of {whose}, each on a line of its own"));
    if keyed {
        copy.request_focus();
    }
    if copy.clicked() {
        *asked = Some(HeadingAsked::CopyAll);
        ui.close();
    }
    let save =
        ui.button(SAVE_AS).on_hover_text(format!("Save the findings of {whose} into a text file"));
    if save.clicked() {
        *asked = Some(HeadingAsked::Save);
        ui.close();
    }
    mg_uikit::menu_separator(ui);
    let again = match source {
        Source::Tileset(_) => "Check again brings them back",
        Source::Build => "Build again brings them back",
        Source::Verify => "Verify again brings them back",
    };
    if ui.button(REMOVE).on_hover_text(format!("Take these findings away. {again}")).clicked() {
        *asked = Some(HeadingAsked::Remove);
        ui.close();
    }
}

/// A finding's row's accessible name: its kind, its place and its message
/// ("Error: mg_bad.nss, unexpected token").
pub fn row_name(p: &Problem) -> String {
    let word = if p.severity == Severity::Error { "Error" } else { "Warning" };
    match p.at.is_empty() {
        true => format!("{word}: {}", p.message),
        false => format!("{word}: {}, {}", p.at, p.message),
    }
}

/// How a finding's row is drawn: chosen, ringed (chosen, with the panel's
/// keys), Verify's fix beside it and its check's Fix All where it is the
/// first of several, and whether Mark as Used can be (the module saved).
struct Look<'a> {
    selected: bool,
    ringed: bool,
    /// Shift+F10 asked for its menu.
    keyed: bool,
    fix: Option<&'a crate::OfferedFix>,
    all: Option<&'a (usize, String, usize)>,
    place: bool,
}

/// What a finding's row was asked for.
#[derive(Default)]
struct Did {
    /// Its place clicked.
    go: bool,
    /// The row clicked elsewhere: chosen.
    chosen: bool,
    fix: bool,
    fix_all: bool,
    /// Its menu's Mark as Used.
    mark: bool,
    /// Its menu's Copy or Copy All.
    copy: Option<RowCopy>,
}

/// The name of an Unused result's menu row and of the Build dialog's
/// button.
pub const MARK_AS_USED: &str = "Mark as Used";

/// A finding's row: its kind's mark and word, its place (a link where it
/// goes somewhere), its message cut to the row, whole in its tip, and
/// Verify's fix as a button after it (Fixed, once made). A click on the row
/// chooses it, a double click goes where it is; its menu (a right click, or
/// Shift+F10 on the one chosen) offers Copy and Copy All, after an Unused
/// result of Build's Mark as Used. (Its words are not selectable: a click
/// on them is the row's, and its menu's Copy takes the whole message.)
fn problem(
    ui: &mut Ui,
    t: &mg_uikit::Tokens,
    p: &Problem,
    line: f32,
    first: &mut bool,
    look: &Look<'_>,
) -> Did {
    let mut did = Did::default();
    let room = egui::Rect::from_min_size(ui.cursor().min, egui::vec2(ui.available_width(), line));
    // (The row first, under what is in it: a click on its link or button
    // is theirs.)
    let row = ui.interact(room, ui.id().with("problem-row"), egui::Sense::click());
    let error = p.severity == Severity::Error;
    let (icon, color, word) = match error {
        true => (mg_uikit::Icon::Error, t.error, "Error"),
        false => (mg_uikit::Icon::Warning, t.warn, "Warning"),
    };
    // (On the chosen fill, its words in its own color, as the tables'.)
    let (color, link) = if look.selected { (t.on_fill, t.on_fill) } else { (color, t.accent) };
    let selected = look.selected;
    row.widget_info(|| {
        let said = row_name(p);
        egui::WidgetInfo::selected(egui::WidgetType::SelectableLabel, true, selected, &said)
    });
    if look.selected {
        // (The lists' chosen fill, as the tables and the tree: `select` is
        // the 3D view's.)
        ui.painter().rect_filled(room, 2.0, t.accent_fill);
    }
    if look.ringed {
        let (ring_rect, ring) = if look.selected {
            (room.shrink(1.5), egui::Stroke::new(1.5, t.on_fill))
        } else {
            (room, egui::Stroke::new(1.0, t.accent))
        };
        ui.painter().rect_stroke(ring_rect, 2.0, ring, egui::StrokeKind::Inside);
    }
    // (A right click chooses it too: Ctrl+C after its menu copies it.)
    did.chosen = row.clicked() || row.secondary_clicked();
    did.go = (row.double_clicked() || row.triple_clicked()) && p.go.is_some();
    // Its menu: an Unused result's Mark as Used first, then Copy and Copy
    // All.
    let place = look.place;
    let all = "Copy every finding listed, under every heading, each on a line of its own";
    did.copy = row_copy::row_menu_saying(ui, &row, look.keyed, all, |ui| {
        if p.unused.is_none() {
            return false;
        }
        let r = ui.add_enabled(place, egui::Button::new(MARK_AS_USED));
        if r.on_disabled_hover_text(crate::build_view::NO_PLACE)
            .on_hover_text(crate::build_view::MARK_HOVER)
            .clicked()
        {
            did.mark = true;
            ui.close();
        }
        true
    });
    ui.horizontal(|ui| {
        ui.set_min_height(line);
        let (rect, _) = ui.allocate_exact_size(egui::vec2(16.0, 16.0), egui::Sense::hover());
        icon.paint(ui.painter(), rect, color);
        // (The kind's word in a column of its own, as Checks' 54.)
        let size = egui::vec2(54.0, line);
        ui.allocate_ui_with_layout(size, egui::Layout::left_to_right(egui::Align::Center), |ui| {
            ui.set_min_size(size);
            ui.colored_label(color, word);
        });
        let place = egui::RichText::new(&p.at).color(link);
        // (No link of no words: a finding of the whole file names it.)
        match &p.go {
            Some(_) if !p.at.is_empty() => {
                let link = ui.link(place).on_hover_text("Go to it");
                if std::mem::take(first) {
                    link.request_focus();
                }
                did.go |= link.clicked();
            }
            _ if !p.at.is_empty() => _ = ui.label(place),
            _ => {}
        }
        // (The fix's buttons at the row's end, the message cut before
        // them.)
        let buttons = |ui: &mut Ui, did: &mut Did| {
            let Some(offer) = look.fix else { return };
            if offer.made {
                let fixed = mg_uikit::IconButton::new(mg_uikit::Icon::Check, "Fixed");
                ui.add_enabled(false, fixed.small()).on_disabled_hover_text(&offer.fix.label);
                return;
            }
            if let Some((_, check, n)) = look.all {
                let all =
                    ui.add(egui::Button::new(format!("Fix All ({n})")).small()).on_hover_ui(|ui| {
                        ui.label(format!(
                            "Fix every finding of {check} that has a fix, as one Undo step"
                        ));
                    });
                did.fix_all = all.clicked();
            }
            did.fix = ui
                .add(egui::Button::new(&offer.fix.label).small())
                .on_hover_ui(|ui| {
                    let hint = match &offer.fix.asks {
                        Some(a) => format!("Runs the plugin's command {} on it", a.command),
                        None => {
                            let resources: Vec<String> =
                                offer.fix.resources().iter().map(ToString::to_string).collect();
                            format!("Changes {} (one step to undo)", resources.join(", "))
                        }
                    };
                    ui.label(hint);
                })
                .clicked();
        };
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            buttons(ui, &mut did);
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                let message: egui::WidgetText = match look.selected {
                    true => egui::RichText::new(&p.message).color(t.on_fill).into(),
                    false => (&p.message).into(),
                };
                // (Not selectable: a click on the words, either button,
                // is the row's.)
                let words = egui::Label::new(message).truncate().selectable(false);
                ui.add(words).on_hover_text(&p.message);
            });
        });
    });
    did
}

/// Goes where a finding is: a tileset's place (its editor opened or
/// brought forward on the page, row and field).
pub(crate) fn go_to(app: &mut Moonglow, to: &Go) {
    match to {
        Go::Tileset { path, place } => crate::tileset_view::go_to(app, path, place),
        Go::Resource(key) => {
            if let Some(t) = crate::Tab::for_resource(*key) {
                app.actions.push(crate::Action::OpenTab(t));
            }
        }
    }
}
