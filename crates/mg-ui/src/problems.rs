//! Problems (M4's U20): the Bottom Dock's global panel of findings, shown
//! whatever the editor in front, with its switch in the Status Bar between
//! the Log's and the editor's own (every board draws it; the NUI board's
//! before Checks). It holds what each **source** found last, a source's
//! findings replacing what it found before (never appended to for ever):
//! today the tileset editor's Check, one source for each tileset file; U28
//! adds Build Module's results and Verify's findings as sources of their
//! own (Q4 and Q5). A finding is its severity, where it is (in words), its
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
//! module; a file no longer found is said to be.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use egui::Ui;
use mg_module::doctor::Severity;

use crate::Moonglow;

/// The accessible name of Problems' body (its rows, or the words saying
/// what fills it): apart from the panel's tab and switch, which share its
/// name.
pub const PROBLEM_ROWS: &str = "Problem rows";

/// Who found a set of findings: what replaces them when it finds again.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Source {
    /// The tileset editor's Check of the tileset file at this path.
    Tileset(PathBuf),
}

impl Source {
    /// Its name over its findings: the tileset's file name.
    pub fn title(&self) -> String {
        match self {
            Source::Tileset(path) => file_name(path),
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
    /// The rows as listed, for the revision they were made at.
    rows: Option<(u64, Arc<Vec<Row>>)>,
    /// The pass the panel was last drawn in and the revision it showed:
    /// a change made after it in that pass (a tileset drawn after the
    /// Bottom Dock marked its findings stale) asks for the next frame, once
    /// (`shell::follow_bottom`).
    pub(crate) drawn: Option<(u64, u64)>,
}

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
        let before = self.sets.len();
        self.sets.retain(|s| s.source != *source);
        if self.sets.len() != before {
            self.changed();
        }
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
pub const FILLED_BY: &str = "A tileset's Check lists what it finds here.";

/// Problems' body (`shell::panel_body`, the Bottom Dock's): each source's
/// heading (its name, its counts, and whether its findings may be stale),
/// then its findings, errors first, each a row of its kind's mark and word,
/// its place (a link that goes there, where it has one) and its message cut
/// to its line and whole in its tip; only the rows in sight laid out. With
/// nothing found anywhere, the kit's empty state.
pub(crate) fn body(app: &mut Moonglow, ui: &mut Ui) {
    mg_uikit::name_group(ui, PROBLEM_ROWS);
    let keys = crate::palette_view::Keys::Problems;
    crate::palette_view::keys_outline(ui, ui.max_rect(), keys);
    if ui.input(|i| i.pointer.any_pressed()) && ui.rect_contains_pointer(ui.max_rect()) {
        crate::palette_view::give_keys(ui.ctx(), keys);
    }
    let focus = app.dialog_panel_focus.take_if(|k| *k == keys).is_some();
    let t = mg_uikit::tokens(ui.ctx());
    app.problems.drawn = Some((ui.ctx().cumulative_pass_nr(), app.problems.revision()));
    if app.problems.sets.is_empty() {
        empty(ui, &t);
        return;
    }
    let rows = app.problems.rows();
    let line = mg_uikit::metrics::CONTROL_HEIGHT + 6.0;
    let mut go = None;
    let mut remove = None;
    let mut first = focus;
    let sets = &app.problems.sets;
    let margin = egui::Margin { left: 8, right: 8, top: 2, bottom: 2 };
    egui::Frame::new().inner_margin(margin).show(ui, |ui| {
        let area = egui::ScrollArea::vertical().id_salt("problem-rows").auto_shrink([false, false]);
        mg_uikit::show_rows(area, ui, rows.len(), |ui, shown| {
            for row in &rows[shown] {
                match *row {
                    Row::Heading { set } => {
                        if ui
                            .push_id(("heading", set), |ui| heading(ui, &t, &sets[set], line))
                            .inner
                        {
                            remove = Some(sets[set].source.clone());
                        }
                    }
                    Row::Problem { set, at } => {
                        let p = &sets[set].problems[at];
                        ui.push_id((set, at), |ui| {
                            if problem(ui, &t, p, line, &mut first) {
                                go = p.go.clone();
                            }
                        });
                    }
                }
            }
        });
    });
    if let Some(source) = remove {
        app.problems.remove(&source);
    }
    if let Some(to) = go {
        go_to(app, &to);
    }
}

/// The name of a heading's Remove from Problems (its menu's row).
pub const REMOVE: &str = "Remove from Problems";

/// The kit's empty state (`.empty`): the problems icon, what it is, what
/// fills it, in the middle of the panel.
fn empty(ui: &mut Ui, t: &mg_uikit::Tokens) {
    let room = ui.available_rect_before_wrap();
    ui.scope_builder(egui::UiBuilder::new().max_rect(room), |ui| {
        ui.vertical_centered(|ui| {
            let high = 32.0 + 8.0 + 2.0 * ui.text_style_height(&egui::TextStyle::Body);
            ui.add_space(((room.height() - high) / 2.0).max(4.0));
            let (rect, _) = ui.allocate_exact_size(egui::vec2(32.0, 32.0), egui::Sense::hover());
            mg_uikit::Icon::Problems.paint(ui.painter(), rect, t.text_3);
            ui.add_space(4.0);
            ui.label(egui::RichText::new(NOTHING).color(t.text_2));
            ui.label(egui::RichText::new(FILLED_BY).color(t.text_3));
        });
    });
}

/// A source's heading row: its name, its counts or "No problems found",
/// and why its findings may be stale (then not counted, it says). A row of
/// its own to focus (Tab): a right click, or Delete with it focused,
/// offers or does Remove from Problems. Whether it was asked to go.
fn heading(ui: &mut Ui, t: &mg_uikit::Tokens, set: &Set, line: f32) -> bool {
    let (errors, warnings) = set.counts();
    let words = counted(errors, warnings).unwrap_or_else(|| NOTHING.to_string());
    let shown = ui.horizontal(|ui| {
        ui.set_min_height(line);
        ui.label(egui::RichText::new(set.source.title()).strong().color(t.text));
        ui.label(egui::RichText::new(&words).color(t.text_3));
        if let Some(why) = set.stale {
            let said = format!("{}: not counted", why.words());
            ui.label(egui::RichText::new(said).color(t.warn)).on_hover_text(
                "What it says is of the file as it was checked: Check again to see it now",
            );
        }
    });
    let rect = shown.response.rect.with_max_x(ui.max_rect().right());
    let id = ui.id().with("problems-heading");
    let r = ui.interact(rect, id, egui::Sense::click() | egui::Sense::FOCUSABLE);
    let said = format!("{}, {words}", set.source.title());
    r.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Other, true, &said));
    if r.has_focus() {
        ui.painter().rect_stroke(
            rect,
            2.0,
            egui::Stroke::new(1.0, t.accent),
            egui::StrokeKind::Inside,
        );
    }
    let mut asked =
        r.has_focus() && ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Delete));
    mg_uikit::context_menu(&r, |ui| {
        if ui
            .button(REMOVE)
            .on_hover_text("Take these findings away; Check again brings them back")
            .clicked()
        {
            asked = true;
            ui.close();
        }
    });
    asked
}

/// A finding's row: its kind's mark and word, its place (a link where it
/// goes somewhere), its message cut to the row, whole in its tip. Whether
/// its place was clicked.
fn problem(ui: &mut Ui, t: &mg_uikit::Tokens, p: &Problem, line: f32, first: &mut bool) -> bool {
    let mut clicked = false;
    ui.horizontal(|ui| {
        ui.set_min_height(line);
        let error = p.severity == Severity::Error;
        let (icon, color, word) = match error {
            true => (mg_uikit::Icon::Error, t.error, "Error"),
            false => (mg_uikit::Icon::Warning, t.warn, "Warning"),
        };
        let (rect, _) = ui.allocate_exact_size(egui::vec2(16.0, 16.0), egui::Sense::hover());
        icon.paint(ui.painter(), rect, color);
        // (The kind's word in a column of its own, as Checks' 54.)
        let size = egui::vec2(54.0, line);
        ui.allocate_ui_with_layout(size, egui::Layout::left_to_right(egui::Align::Center), |ui| {
            ui.set_min_size(size);
            ui.colored_label(color, word);
        });
        let place = egui::RichText::new(&p.at).color(t.accent);
        // (No link of no words: a finding of the whole file names it.)
        match &p.go {
            Some(_) if !p.at.is_empty() => {
                let link = ui.link(place).on_hover_text("Go to it");
                if std::mem::take(first) {
                    link.request_focus();
                }
                clicked = link.clicked();
            }
            _ if !p.at.is_empty() => _ = ui.label(place),
            _ => {}
        }
        ui.add(egui::Label::new(&p.message).truncate()).on_hover_text(&p.message);
    });
    clicked
}

/// Goes where a finding is: a tileset's place (its editor opened or
/// brought forward on the page, row and field).
pub(crate) fn go_to(app: &mut Moonglow, to: &Go) {
    match to {
        Go::Tileset { path, place } => crate::tileset_view::go_to(app, path, place),
    }
}
