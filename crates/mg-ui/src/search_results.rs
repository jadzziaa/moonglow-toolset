//! Search Results (M4's U13 and U28): the Bottom Dock's global panel,
//! shown whatever the editor in front, with its switch in the Status Bar
//! (every board draws it). It holds the last search's hits, one search's at
//! a time ([`Showing`]; Part 2's decision 8: "Search Results is one panel,
//! showing the last search, its form as its header"):
//!
//! - a script's searches (Find In Files, Find In Currently Open Scripts,
//!   References), what was searched over their lines; a click on one opens
//!   the script at it;
//! - Find Instance (Edit › Find Instance…; before U28 a window of its own):
//!   its form (the kinds, the area, the blueprint and the tag, Search and
//!   Clear, the count) as the panel's header, the placed objects found as
//!   a table (Type, Tag, Area, Blueprint), each going to its object;
//! - Find and Replace Text (Edit › Find and Replace Text…, Ctrl+H; before
//!   U28 a window of its own): its form (Find what, Replace with, the
//!   options and kinds, Find and Replace in N, "6 strings, 5 times") as the
//!   header, the strings found as the Dialogs board's rows (a tick, the
//!   place as a link, the string under it).
//!
//! A search's form keeps what was typed while another's results are shown;
//! a search runs as a job (its progress and Cancel in the job's window).
//! Floating, a script's window shows the panel on its bottom pane after the
//! script's own panels (Floating.png).

use egui::Ui;
use mg_resman::ResKey;
use mg_uikit::Controls as _;

use crate::Moonglow;
use crate::palette_view::Keys;
use crate::report_table::{self, Column};
use crate::row_copy;
use crate::script_tools::{self as tools, InfoTab};

/// The accessible name of Search Results' body (its hits or the words
/// saying what fills it): a screen reader's, and the tests' way to it
/// apart from the panel's tab and switch, which share its name.
pub const SEARCH_HITS: &str = "Search hits";

/// The accessible name of Find Instance's table of objects found.
pub const INSTANCES_FOUND: &str = "Objects found";

/// The accessible name of Find and Replace Text's strings found.
pub const STRINGS_FOUND: &str = "Strings found";

/// One hit of a script's search, as Search Results lists it and opens it.
/// (Find Instance's and Find and Replace Text's are their own states', the
/// objects and strings found: `area_tools::FindInstance`,
/// `bulk::TextReplace`.)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Hit {
    /// A line of a script: the script, its line (from 0) and its text.
    Line { script: ResKey, line: usize, text: String },
}

impl Hit {
    /// Its row's words: `hello(3): int n = 1;` for a script's line.
    pub fn row(&self) -> String {
        match self {
            Hit::Line { script, line, text } => format!("{}({}): {text}", script.resref, line + 1),
        }
    }
}

/// Whose results Search Results shows: the last search's.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Showing {
    /// A script's search: [`SearchResults::hits`].
    #[default]
    Lines,
    /// Find Instance's form and the objects it found (`app.find_instance`).
    Instances,
    /// Find and Replace Text's form and the strings it found
    /// (`app.text_replace`).
    Text,
}

/// What Search Results shows: whose results ([`Showing`]); a script's
/// search's, said by `what` (`"Twice": 3 lines found`), and its hits, at
/// most [`tools::MAX_RESULTS`].
#[derive(Debug, Clone, Default)]
pub struct SearchResults {
    pub what: String,
    pub hits: Vec<Hit>,
    pub showing: Showing,
    /// The script's line the keys are on.
    pub(crate) line: Option<usize>,
    /// Find Instance's table: its sort, the row chosen, its order.
    pub(crate) instances: report_table::Table,
    /// The form's first field asked to take the keyboard (its command).
    pub(crate) focus_form: bool,
    /// The module's areas and their labels (Find Instance's).
    pub(crate) area_labels: AreaLabels,
    /// Where the form shown was drawn last (its scroll's room, which the
    /// accessibility tree has no rect of): the tests'.
    form_rect: Option<egui::Rect>,
}

impl SearchResults {
    /// How many it shows: the lines, objects or strings found.
    pub fn count(&self, app: &Moonglow) -> usize {
        match self.showing {
            Showing::Lines => self.hits.len(),
            Showing::Instances => app.find_instance.as_ref().map_or(0, |f| f.results.len()),
            // (Those shown: none while the module changed since, or what's
            // asked is not what they were found for.)
            Showing::Text => app.text_replace.as_ref().map_or(0, |d| {
                let shown = d.asked_now() && app.ws.as_ref().is_some_and(|ws| !d.stale(ws));
                if shown { d.hits.len() } else { 0 }
            }),
        }
    }

    /// The Find Instance table's rows' order made, how many times (the
    /// tests': nothing is sorted each frame).
    #[doc(hidden)]
    pub fn instance_order_builds(&self) -> u64 {
        self.instances.builds()
    }

    /// The Find Instance table's rows laid out at the last frame, by their
    /// places in its order (the tests').
    #[doc(hidden)]
    pub fn instance_rows_seen(&self) -> (usize, usize) {
        self.instances.seen()
    }

    /// The Find Instance row chosen, by its place among the results.
    #[doc(hidden)]
    pub fn instance_chosen(&self) -> Option<usize> {
        self.instances.selected
    }

    /// Where the form shown was last drawn: the room it scrolls in (the
    /// tests': a line of it is whole in it or out of it, never cut).
    #[doc(hidden)]
    pub fn form_rect(&self) -> Option<egui::Rect> {
        self.form_rect
    }

    /// How many times In Area's list (the module's areas and their labels)
    /// was made (the tests': as the module changes, not each frame).
    #[doc(hidden)]
    pub fn area_list_builds(&self) -> u64 {
        self.area_labels.revision
    }
}

impl Moonglow {
    /// A script's search's hits into Search Results, in place of the last
    /// search's, said by `what`; the panel brought to the front and the
    /// Bottom Dock opened on it where anything was found (or the window's
    /// bottom pane of the script `from`, where it floats).
    pub(crate) fn show_search_results(
        &mut self,
        what: String,
        hits: Vec<Hit>,
        from: Option<ResKey>,
    ) {
        // (Nothing found opens no closed dock: the log says so.)
        let open = !hits.is_empty();
        let r = &mut self.search_results;
        (r.what, r.hits, r.showing, r.line) = (what, hits, Showing::Lines, None);
        self.script_tools.ask_forward(from, InfoTab::SearchResults, open);
    }

    /// Search Results shows `showing`'s form and results, brought to the
    /// front of the main window's Bottom Dock, opened (a search's command,
    /// its results found). One search's results at a time: a script's
    /// lines are let go of; Find Instance's and Find and Replace Text's
    /// forms keep what was typed.
    pub(crate) fn show_search(&mut self, showing: Showing) {
        let r = &mut self.search_results;
        if showing != Showing::Lines {
            r.hits.clear();
            r.what.clear();
        }
        r.showing = showing;
        self.script_tools.ask_forward(None, InfoTab::SearchResults, true);
    }

    /// Edit › Find Instance…: its form in Search Results, the Bottom Dock
    /// opened on it, From Blueprint taking the keys.
    pub(crate) fn open_find_instance(&mut self) {
        self.find_instance.get_or_insert_with(Default::default);
        self.show_search(Showing::Instances);
        self.search_results.focus_form = true;
    }

    /// Edit › Find and Replace Text… (Ctrl+H): its form in Search Results,
    /// the Bottom Dock opened on it, Find what taking the keys.
    pub(crate) fn open_find_text(&mut self) {
        self.text_replace.get_or_insert_with(Default::default);
        self.show_search(Showing::Text);
        self.search_results.focus_form = true;
    }
}

/// Each frame's start: Find and Replace Text's Find again after a Replace
/// in N was applied (what is left), as a job.
pub(crate) fn follow(app: &mut Moonglow) {
    if app.text_refind && app.actions.is_empty() && !app.busy() {
        app.text_refind = false;
        crate::bulk::start_find_text(app);
    }
}

/// The badge of Search Results' switch: what it shows, where any.
pub(crate) fn badge(app: &Moonglow) -> Option<mg_uikit::Badge> {
    match app.search_results.count(app) {
        0 => None,
        n => Some(mg_uikit::Badge::count(n)),
    }
}

/// Search Results' body (`shell::panel_body`, in the Bottom Dock or in a
/// script's window's bottom pane): the search shown's form, if it has one,
/// then its results by the rows in sight; with nothing searched yet, what
/// fills it.
pub(crate) fn body(app: &mut Moonglow, ui: &mut Ui) {
    let keys = Keys::SearchResults;
    crate::script_panels::begin(app, ui, SEARCH_HITS, keys);
    if ui.input(|i| i.pointer.any_pressed()) && ui.rect_contains_pointer(ui.max_rect()) {
        crate::palette_view::give_keys(ui.ctx(), keys);
    }
    match app.search_results.showing {
        Showing::Instances if app.find_instance.is_some() => instances(app, ui),
        Showing::Text if app.text_replace.is_some() => text(app, ui),
        _ => lines(app, ui),
    }
}

/// What a form asked for this frame.
#[derive(Default)]
struct Asked {
    search: bool,
    clear: bool,
    replace: bool,
    /// Escape in one of its fields: the dock closed, as the window was.
    close: bool,
    /// A field took Enter or Escape this frame: not the rows' too.
    took_keys: bool,
}

/// A field of a form, `width` wide: Enter in it searches, Escape closes
/// the dock; the first takes the keys where its command asked.
fn field(
    ui: &mut Ui,
    text: &mut String,
    hint: &str,
    width: f32,
    mono: bool,
    focus: &mut bool,
    asked: &mut Asked,
) -> egui::Response {
    let mut edit = mg_uikit::text_field(text).hint_text(hint).desired_width(width);
    if mono {
        edit = edit.font(egui::TextStyle::Monospace);
    }
    let r = ui.add(edit);
    if std::mem::take(focus) {
        r.request_focus();
    }
    if r.lost_focus() {
        let take = |ui: &mut Ui, k| ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, k));
        if take(ui, egui::Key::Enter) {
            (asked.search, asked.took_keys) = (true, true);
            report_table::rows_took_keys_from_field(ui.ctx(), Keys::SearchResults, r.id);
        }
        if take(ui, egui::Key::Escape) {
            (asked.close, asked.took_keys) = (true, true);
        }
    }
    r
}

/// A label and its field (`width` wide, or as wide as the row has room
/// for) on a wrapped row, kept together: on a row of their own where what
/// is before them leaves too little room, so that nothing is cut at the
/// least window.
fn labeled<R>(ui: &mut Ui, label: &str, width: f32, add: impl FnOnce(&mut Ui, f32) -> R) -> R {
    let font = egui::TextStyle::Body.resolve(ui.style());
    let words = ui.fonts_mut(|f| f.layout_no_wrap(label.to_string(), font, egui::Color32::WHITE));
    let gap = ui.spacing().item_spacing.x;
    let need = words.size().x + gap + width;
    let at_start = ui.cursor().left() <= ui.max_rect().left() + 0.5;
    // (What is left of the row: a wrapping row's `available_width` is the
    // whole row's.)
    if !at_start && ui.available_size_before_wrap().x < need {
        ui.end_row();
    }
    let room = (ui.available_size_before_wrap().x - words.size().x - gap).max(40.0);
    ui.horizontal(|ui| {
        crate::widgets::field_label(ui, label);
        add(ui, width.min(room))
    })
    .inner
}

/// The main button of a form (`accent-fill`, the dialog pattern's).
fn main_button(ui: &mut Ui, words: &str, enabled: bool) -> egui::Response {
    let b = mg_uikit::KitButton::new(None, words).look(mg_uikit::ButtonLook::Primary);
    ui.add_enabled(enabled, b)
}

/// The form's frame: the panel's margins; where the dock is lower than the
/// form and a row under it (the least window), the form scrolls in what
/// the rows leave it, so that a row stays in sight, and by whole lines
/// ([`form_line`]: M4's U28 review, the orchestrator's ruling): as many as
/// fit are shown, never a line cut in half at rest (a scroll that stops
/// between two is moved on to the next, the way it went), and a field or
/// button given the keyboard's focus (Tab) is brought wholly into sight.
/// What it asked for, and the rect it shows the form in.
fn form_frame<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> (R, egui::Rect) {
    let margin = egui::Margin { left: 8, right: 8, top: 4, bottom: 4 };
    let rows = crate::table_view::look::HEADER_HIGH + 2.0 * mg_uikit::row_pitch(ui);
    let line = form_line_high(ui);
    let pitch = line + ui.spacing().item_spacing.y;
    let top = f32::from(margin.top);
    // (At the least window the form scrolls down to a row of its own, the
    // header and a whole hit under it; in what is left, whole lines.)
    let room = ui.available_height() - rows;
    let lines = ((room - top - line) / pitch).floor().max(0.0) + 1.0;
    let most = top + (lines - 1.0) * pitch + line;
    let id = ui.id().with("search-form-lines");
    let mut state: FormScroll = ui.data(|d| d.get_temp(id)).unwrap_or_default();
    let mut area = egui::ScrollArea::vertical()
        .id_salt("search-form")
        .max_height(most)
        // (egui's least, 64, would be taller than `most` there.)
        .min_scrolled_height(0.0)
        .auto_shrink([false, true]);
    if let Some(want) = state.want.take() {
        area = area.vertical_scroll_offset(want);
    }
    let out = area.show(ui, |ui| egui::Frame::new().inner_margin(margin).show(ui, add).inner);
    let shown = out.inner_rect;
    let at = out.state.offset.y;
    let most_offset = (out.content_size.y - shown.height()).max(0.0);
    // (Content's top, in the window: where line 0 is, less the margin.)
    let content_top = shown.top() - at;
    let line_of = |y: f32| ((y - content_top - top) / pitch).floor().max(0.0);
    // A field or button given the keyboard's focus: its line whole in
    // sight.
    // (Not in a frame the pointer is pressed or clicked in: egui's
    // `read_response` takes the focus from a widget pressed elsewhere, and
    // the field a menu's command has just given it would lose it.)
    let focused = ui.memory(|m| m.focused());
    let pressed = ui.input(|i| i.pointer.any_pressed() || i.pointer.any_click());
    if focused != state.focused && !pressed {
        state.focused = focused;
        let rect = focused.and_then(|f| ui.ctx().read_response(f)).map(|r| r.rect);
        if let Some(r) = rect.filter(|r| r.top() >= content_top && r.top() < shown.bottom() + 1e4)
            && (r.top() < shown.top() - 0.5 || r.bottom() > shown.bottom() + 0.5)
            && r.top() <= content_top + out.content_size.y
        {
            let j = line_of(r.center().y);
            let first = if r.top() < shown.top() { j } else { j - (lines - 1.0) };
            state.want = Some((first.max(0.0) * pitch).min(most_offset));
        }
    }
    // At rest between two lines: on to the next whole one, the way it went
    // (back where it went back).
    let moving = ui.input(|i| i.smooth_scroll_delta != egui::Vec2::ZERO || i.pointer.any_down());
    let off_line = (at / pitch).fract() > 0.01 && (at - most_offset).abs() > 0.5;
    if state.want.is_none() && !moving && off_line && (at - state.last).abs() < 0.01 {
        let below = (at / pitch).floor() * pitch;
        let to = if state.went_down { below + pitch } else { below };
        state.want = Some(to.min(most_offset));
    }
    if (at - state.last).abs() >= 0.01 {
        state.went_down = at > state.last;
    }
    state.last = at;
    if state.want.is_some() {
        ui.ctx().request_repaint();
    }
    ui.data_mut(|d| d.insert_temp(id, state));
    // (The Bottom Dock's least: the form whole, not as cut here, and a row
    // under it; the dock is never held lower by the form's own scrolling.)
    let whole = out.inner_rect.top() + out.content_size.y;
    crate::shell::first_row_ends(ui, whole + rows);
    (out.inner, shown)
}

/// The form's scroll between frames: where it was, which way it went last,
/// the control focused last, and where it is asked to go next.
#[derive(Clone, Copy, Debug, Default)]
struct FormScroll {
    last: f32,
    went_down: bool,
    focused: Option<egui::Id>,
    want: Option<f32>,
}

/// A form's line's height: its tallest control's (a field, as its font
/// and margins make it at this zoom; a button; a tick), and a pixel more
/// for egui's rounding to the pixel grid (at 125% a 22-point field is
/// drawn 22.2 high, a fraction below where it was laid out), so that the
/// lines are a pitch apart and the form scrolls by whole ones.
fn form_line_high(ui: &Ui) -> f32 {
    let font = egui::TextStyle::Body.resolve(ui.style());
    let margin = mg_uikit::controls::FIELD_MARGIN;
    let field = ui.fonts_mut(|f| f.row_height(&font)) + f32::from(margin.top + margin.bottom);
    let tallest = ui.spacing().interact_size.y.max(mg_uikit::metrics::CONTROL_HEIGHT).max(field);
    tallest + 2.0 / ui.ctx().pixels_per_point()
}

/// A line of a form, wrapped where the room is narrower than it: each of
/// its rows [`form_line_high`] high.
fn form_line<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
    let high = form_line_high(ui);
    ui.horizontal_wrapped(|ui| {
        ui.set_row_height(high);
        add(ui)
    })
    .inner
}

/// Escape in a form's field: the Bottom Dock closed (where it is in it),
/// as Escape closed the window the form was in.
fn close_dock(app: &mut Moonglow, ui: &Ui) {
    if app.bottom_rect.is_some_and(|r| r.intersects(ui.max_rect())) {
        app.settings.hide_log = true;
    }
}

/// A script's search's lines: what was searched, then each a row high,
/// cut at the panel's edge and whole in its tip; a click opens one, as
/// Enter does the one the arrows are on. Its menu (a right click, or
/// Shift+F10 on the one the arrows are on) copies it or every line, as
/// Ctrl+C does the one the arrows are on.
fn lines(app: &mut Moonglow, ui: &mut Ui) {
    let results = &app.search_results;
    if results.hits.is_empty() && results.what.is_empty() {
        ui.weak(
            "Find In Files, Find In Currently Open Scripts (a script's Find…) and References list \
             what they find here. Find Instance and Find and Replace Text (Edit) search here too.",
        );
        return;
    }
    ui.add(egui::Label::new(egui::RichText::new(&results.what).weak()).truncate());
    let has = report_table::has_keys(ui.ctx(), Keys::SearchResults);
    let len = results.hits.len();
    let mut current = results.line;
    let mut open = None;
    let page = list_page(ui);
    if has && let Some(enter) = list_keys(ui, len, &mut current, page) {
        open = enter;
    }
    let mut keyed = false;
    if has && let Some(at) = current.filter(|at| *at < len) {
        if row_copy::copy_pressed(ui) {
            ui.ctx().copy_text(results.hits[at].row());
        }
        keyed = row_copy::menu_key(ui);
    }
    let mut copy = None;
    let mut chosen = None;
    let results = &app.search_results;
    let moved = current != results.line;
    let mut area = egui::ScrollArea::vertical()
        .id_salt("search-hits")
        .auto_shrink([false, false])
        // (No taller than its room: egui's least would lay rows out past
        // the dock's bottom.)
        .min_scrolled_height(0.0);
    // (Shift+F10's row brought into sight, its menu opened under it.)
    if (moved || keyed)
        && let Some(at) = current
    {
        area = area.vertical_scroll_offset(
            (at as f32 * mg_uikit::row_pitch(ui) - ui.available_height() / 2.0).max(0.0),
        );
    }
    crate::shell::first_row_ends(ui, ui.cursor().top() + mg_uikit::row_pitch(ui));
    mg_uikit::show_rows(area, ui, len, |ui, range| {
        for at in range {
            let hit = &results.hits[at];
            // (A long line cut at the panel's edge, on one row.)
            let on = current == Some(at);
            let words = hit.row();
            let r = ui.add(egui::Button::selectable(on, words.as_str()).truncate());
            if r.clicked() {
                open = Some(at);
            } else if r.secondary_clicked() {
                // (Chosen, not opened: Ctrl+C after its menu copies it.)
                chosen = Some(at);
            }
            match row_copy::row_menu(ui, &r, keyed && on, |_| false) {
                Some(row_copy::Asked::Row) => copy = Some(words.clone()),
                Some(row_copy::Asked::All) => {
                    copy = Some(row_copy::lines(results.hits.iter().map(Hit::row)));
                }
                None => {}
            }
            r.on_hover_text(words);
        }
    });
    if let Some(text) = copy {
        ui.ctx().copy_text(text);
    }
    if let Some(at) = open.or(chosen) {
        current = Some(at);
    }
    app.search_results.line = current;
    match open.and_then(|at| app.search_results.hits.get(at).cloned()) {
        Some(Hit::Line { script, line, .. }) => {
            crate::script_view::open_line(app, &script.resref.to_string(), line + 1);
        }
        None => {}
    }
}

/// How many rows a page is in `ui`'s room.
fn list_page(ui: &Ui) -> isize {
    ((ui.available_height() / mg_uikit::row_pitch(ui)) as isize - 1).max(1)
}

/// The arrows on a list of `len` rows (Up, Down, Page Up, Page Down, Home,
/// End), `current` moved; `Some(Some(at))` where Enter asked to open the
/// one at `at`, `Some(None)` where the keys did anything else.
fn list_keys(
    ui: &mut Ui,
    len: usize,
    current: &mut Option<usize>,
    page: isize,
) -> Option<Option<usize>> {
    use egui::Key;
    if len == 0 {
        return None;
    }
    let last = len as isize - 1;
    let steps = [
        (Key::ArrowDown, 1),
        (Key::ArrowUp, -1),
        (Key::PageDown, page),
        (Key::PageUp, -page),
        (Key::End, isize::MAX / 2),
        (Key::Home, isize::MIN / 2),
    ];
    let pressed =
        ui.input_mut(|i| steps.into_iter().find(|(k, _)| i.consume_key(egui::Modifiers::NONE, *k)));
    if let Some((_, by)) = pressed {
        // (From none: Down and Page Down and End as from before the first,
        // the others as from past the last.)
        let from = current.map_or(if by > 0 { -1 } else { last + 1 }, |at| at as isize);
        let to = from.saturating_add(by).clamp(0, last) as usize;
        *current = Some(to);
        return Some(None);
    }
    let at = (*current)?;
    ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::Enter)).then_some(Some(at))
}

/// Find Instance's columns (the window's: Type 76, then Tag, Area and
/// Blueprint).
const INSTANCE_COLUMNS: [Column; 4] = [
    Column::fixed("Type", 96.0),
    Column::shared("Tag", 1.0),
    Column::shared("Area", 1.0),
    Column::shared("Blueprint", 1.0),
];

/// What Find Instance found, by number: "No objects found", "1 object
/// found", "120 objects found" (the window's "120 found" had no noun; its
/// "double click one to go to it" is the Status Bar's hint now: M4's U28
/// review).
pub fn instances_words(n: usize) -> String {
    match n {
        0 => "No objects found".to_string(),
        1 => "1 object found".to_string(),
        n => format!("{n} objects found"),
    }
}

/// The module's areas and their labels as lists show them (Options ›
/// General's names or resrefs), made again only as the module or that
/// option changes: In Area's list, the Area column and its sort.
#[derive(Debug, Clone, Default)]
pub(crate) struct AreaLabels {
    made_for: Option<(u64, bool)>,
    pub(crate) labels: std::collections::HashMap<mg_core::ResRef, String>,
    /// In Area's choices ((all areas), then each area), made with them.
    options: std::sync::Arc<Vec<Option<mg_core::ResRef>>>,
    /// Counted up each time they are made again.
    pub(crate) revision: u64,
}

impl AreaLabels {
    fn follow(app: &mut Moonglow) {
        let Some(ws) = app.ws.as_ref() else {
            let l = &mut app.search_results.area_labels;
            if l.made_for.is_some() {
                *l = AreaLabels { revision: l.revision + 1, ..Default::default() };
            }
            return;
        };
        let now = (ws.revision(), app.settings.area_names);
        if app.search_results.area_labels.made_for == Some(now) {
            return;
        }
        let mut areas: Vec<mg_core::ResRef> =
            ws.module.keys_of(mg_core::ResType::ARE).map(|k| k.resref).collect();
        areas.sort();
        let labels = areas.iter().map(|a| (*a, crate::tabs::area_label(app, *a))).collect();
        let options = std::iter::once(None).chain(areas.iter().copied().map(Some)).collect();
        let options = std::sync::Arc::new(options);
        let l = &mut app.search_results.area_labels;
        *l = AreaLabels { made_for: Some(now), labels, options, revision: l.revision + 1 };
    }

    fn of(&self, a: &mg_core::ResRef) -> String {
        self.labels.get(a).cloned().unwrap_or_else(|| a.to_string())
    }
}

/// Find Instance's form and the objects it found.
fn instances(app: &mut Moonglow, ui: &mut Ui) {
    AreaLabels::follow(app);
    let Some(mut f) = app.find_instance.take() else { return };
    let mut focus = std::mem::take(&mut app.search_results.focus_form);
    // (Its command gave the panel the keys: held while the pointer moves
    // over an area's view, for the rows once the form has searched.)
    if focus {
        crate::palette_view::hold_keys(ui.ctx(), Keys::SearchResults);
    }
    let busy = app.busy();
    // (The table's keys as the frame began: a field's Enter, taken by the
    // form, is not the table's too.)
    let r = &mut app.search_results;
    let names = &r.area_labels;
    let (mut asked, form) = form_frame(ui, |ui| {
        let mut asked = Asked::default();
        form_line(ui, |ui| {
            crate::widgets::field_label(ui, "Search For");
            for kind in mg_area::ObjectKind::ALL {
                ui.check_box(&mut f.kinds[kind.index()], kind.label());
            }
        });
        form_line(ui, |ui| {
            labeled(ui, "In Area", 160.0, |ui, w| {
                let all = || "(all areas)".to_string();
                let shown = f.area.map_or_else(all, |a| names.of(&a));
                // (Made with the labels, as the module changes: not each
                // frame.)
                crate::widgets::dropdown("find-area").width(w).selected_text(shown).show(
                    ui,
                    &mut f.area,
                    names.options.as_slice(),
                    |a| a.map_or_else(all, |a| names.of(&a)),
                );
            });
            labeled(ui, "From Blueprint", 140.0, |ui, w| {
                field(ui, &mut f.template, "blueprint resref", w, false, &mut focus, &mut asked)
            });
            labeled(ui, "With Tag", 140.0, |ui, w| {
                field(ui, &mut f.tag, "tag", w, false, &mut false, &mut asked)
            });
            asked.search |= main_button(ui, "Search", !busy)
                .on_hover_text("Search the module's areas for these objects")
                .clicked();
            asked.clear |=
                ui.button("Clear").on_hover_text("Clear the search and what it found").clicked();
            if f.searched {
                let t = mg_uikit::tokens(ui.ctx());
                // (Whole on the form's line, never wrapped under it.)
                let words = egui::RichText::new(instances_words(f.results.len())).color(t.text_2);
                ui.add(egui::Label::new(words).wrap_mode(egui::TextWrapMode::Extend));
            }
        });
        asked
    });
    r.form_rect = Some(form);
    let mut go = None;
    if f.searched {
        let results = &f.results;
        let key = f.revision.wrapping_mul(1 << 20) ^ names.revision;
        let order = r.instances.order(
            key,
            results.len(),
            |_| true,
            |a, b, col| {
                let (a, b) = (&results[a], &results[b]);
                match col {
                    0 => kind_word(a.kind).cmp(kind_word(b.kind)),
                    1 => report_table::by_words(&a.tag, &b.tag),
                    2 => report_table::by_words(&names.of(&a.area), &names.of(&b.area)),
                    _ => report_table::by_words(&a.template, &b.template),
                }
            },
        );
        let t = mg_uikit::tokens(ui.ctx());
        let spec = report_table::Spec {
            id: egui::Id::new("find-instance-table"),
            name: INSTANCES_FOUND,
            cols: &INSTANCE_COLUMNS,
            keys: Keys::SearchResults,
            may_take_keys: !asked.took_keys,
        };
        let shown = report_table::show(
            ui,
            &mut r.instances,
            &spec,
            &order,
            |i| {
                let x = &results[i];
                let tag = if x.tag.is_empty() { "(no tag)" } else { &x.tag };
                let area = names.of(&x.area);
                (format!("{} {tag}", kind_word(x.kind)), format!("in {area}, from {}", x.template))
            },
            |ui, i, cells, on| {
                let x = &results[i];
                let (text, quiet) = if on { (t.on_fill, t.on_fill) } else { (t.text, t.text_2) };
                report_table::text(ui, cells[0], kind_word(x.kind), quiet, false);
                report_table::text(ui, cells[1], &x.tag, text, false);
                report_table::text(ui, cells[2], names.of(&x.area), text, false);
                report_table::text(ui, cells[3], &x.template, text, false);
            },
        );
        go = shown.activated.and_then(|i| f.results.get(i).cloned());
    } else {
        ui.weak(
            "Find Instance looks for the objects placed in the module's areas by kind, area, \
             blueprint and tag. Search lists them here.",
        );
    }
    if asked.clear {
        f = crate::area_tools::FindInstance::default();
        r.instances.clear();
        asked.search = false;
    }
    if asked.close {
        close_dock(app, ui);
    }
    app.find_instance = Some(f);
    if asked.search {
        crate::area_tools::start_find(app);
    }
    if let Some(r) = go {
        crate::area_tools::go_to_found(app, &r);
    }
}

/// An object's kind as its Type cell and its row's name say it, in the
/// singular ("Placeable"), as Find Instance's list did.
fn kind_word(kind: mg_area::ObjectKind) -> &'static str {
    use mg_area::ObjectKind as K;
    match kind {
        K::Creature => "Creature",
        K::Door => "Door",
        K::Encounter => "Encounter",
        K::Item => "Item",
        K::Placeable => "Placeable",
        K::Sound => "Sound",
        K::Store => "Store",
        K::Trigger => "Trigger",
        K::Waypoint => "Waypoint",
    }
}

/// Find and Replace Text's form and the strings it found.
fn text(app: &mut Moonglow, ui: &mut Ui) {
    use mg_module::text::TextKind;
    let Some(mut d) = app.text_replace.take() else { return };
    let mut focus = std::mem::take(&mut app.search_results.focus_form);
    // (Its command gave the panel the keys: held while the pointer moves
    // over an area's view, for the rows once the form has searched.)
    if focus {
        crate::palette_view::hold_keys(ui.ctx(), Keys::SearchResults);
    }
    let busy = app.busy();
    let stale = app.ws.as_ref().is_some_and(|ws| d.stale(ws));
    let (asked, form) = form_frame(ui, |ui| {
        let mut asked = Asked::default();
        form_line(ui, |ui| {
            labeled(ui, "Find what", 200.0, |ui, w| {
                let r = field(ui, &mut d.find, "", w, true, &mut focus, &mut asked);
                r.on_hover_text(
                    "Searches the module's names, descriptions, conversation lines and journal \
                     in every language, not the game's talk table. Check tags, ResRefs or \
                     variables below to search them too.",
                );
            });
            labeled(ui, "Replace with", 200.0, |ui, w| {
                field(ui, &mut d.with, "", w, true, &mut false, &mut asked)
            });
            ui.check_box(&mut d.options.match_case, "Match case");
            ui.check_box(&mut d.options.whole_word, "Whole words");
            ui.check_box(&mut d.options.regex, "Regular expression").on_hover_text(
                "Find what is a regular expression. Replace with may use its groups ($1, ${name}).",
            );
        });
        let mut tick = |ui: &mut Ui, k: TextKind, hover: Option<&str>| {
            let mut on = d.kinds.contains(&k);
            let r = ui.check_box(&mut on, k.label());
            let r = match hover {
                Some(h) => r.on_hover_text(h),
                None => r,
            };
            if r.changed() {
                d.kinds.retain(|x| *x != k);
                if on {
                    d.kinds.push(k);
                }
            }
        };
        form_line(ui, |ui| {
            for k in TextKind::PLAYERS {
                tick(ui, k, None);
            }
            for k in TextKind::ALL.into_iter().filter(|k| !k.players()) {
                let hover = match k {
                    TextKind::Tag => "Objects' and areas' tags. A tag made another's is warned of.",
                    TextKind::ResRef => {
                        "Fields naming a script, blueprint or conversation. Rename… renames the \
                         resource itself."
                    }
                    TextKind::Variable => "Local variables' names and string values",
                    _ => "The other text fields (a door's Linked To, a key's tag, comments)",
                };
                tick(ui, k, Some(hover));
            }
        });
        let bad = d.regex_problem();
        let asked_now = d.asked_now();
        let can = !d.find.is_empty() && bad.is_none() && !busy;
        let n = if asked_now && !stale { d.ticked().0 } else { 0 };
        form_line(ui, |ui| {
            asked.search |= main_button(ui, "Find", can).clicked();
            asked.replace |= ui
                .add_enabled(n > 0 && !busy, egui::Button::new(format!("Replace in {n}")))
                .on_hover_text("Replace the text in the checked strings, as one step to undo")
                .clicked();
            let t = mg_uikit::tokens(ui.ctx());
            if let Some(e) = &bad {
                ui.colored_label(t.warn, format!("Not a regular expression: {e}"));
            } else if asked_now && stale {
                ui.label(
                    egui::RichText::new(
                        "The module changed since. Find again for what it has now.",
                    )
                    .color(t.warn),
                );
            } else if asked_now {
                ui.label(egui::RichText::new(d.count_words()).color(t.text_2));
            } else if d.searched.is_some() {
                ui.label(egui::RichText::new("Find again for what's asked now.").color(t.text_2));
            }
        });
        asked.search &= can;
        asked
    });
    app.search_results.form_rect = Some(form);
    let mut go = None;
    if d.asked_now() && !stale {
        go = strings(ui, &mut d);
    } else if d.searched.is_none() {
        ui.weak(
            "Find lists here the module's strings with the text in them, each checked to be \
             replaced. Replace in N replaces it in those checked, as one step to undo.",
        );
    }
    if asked.close {
        close_dock(app, ui);
    }
    app.text_replace = Some(d);
    if asked.search {
        crate::bulk::start_find_text(app);
    }
    if asked.replace {
        crate::bulk::replace(app);
    }
    if let Some(at) = go {
        crate::bulk::go_to_hit(app, at);
    }
}

/// The strings found, as the Dialogs board's rows: each two lines high (a
/// tick and the place as a link, the string under it, cut), a `line` under
/// each, only those in sight laid out; the arrows move the one the keys
/// are on, Space ticks it, Enter opens it, Ctrl+C copies it. A string cut
/// short is whole in its tip; a row's menu (a right click, or Shift+F10 on
/// the one the keys are on) copies it or every string found, ticked or not
/// (the ticks say what is replaced, not what is chosen). The one asked to
/// be opened.
fn strings(ui: &mut Ui, d: &mut crate::bulk::TextReplace) -> Option<usize> {
    let has = report_table::has_keys(ui.ctx(), Keys::SearchResults);
    let line = ui.spacing().interact_size.y;
    let tall = line + ui.text_style_height(&egui::TextStyle::Body) + 4.0;
    let page = ((ui.available_height() / tall) as isize - 1).max(1);
    let mut current = d.current;
    let mut go = None;
    let mut tick = None;
    if has {
        if let Some(enter) = list_keys(ui, d.hits.len(), &mut current, page) {
            go = enter;
        }
        if let Some(at) = current
            && ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Space))
        {
            tick = Some((at, !d.hits[at].1));
        }
    }
    let mut keyed = false;
    if has && let Some(at) = current.filter(|at| *at < d.hits.len()) {
        if row_copy::copy_pressed(ui) {
            ui.ctx().copy_text(string_line(&d.hits[at].0));
        }
        keyed = row_copy::menu_key(ui);
    }
    let mut copy = None;
    let moved = current != d.current || keyed;
    let t = mg_uikit::tokens(ui.ctx());
    let mut rows = egui::ScrollArea::vertical()
        .id_salt("strings-found")
        .auto_shrink([false, false])
        // (No taller than its room: egui's least would lay rows out past
        // the dock's bottom.)
        .min_scrolled_height(0.0);
    if moved && let Some(at) = current {
        rows =
            rows.vertical_scroll_offset((at as f32 * tall - ui.available_height() / 2.0).max(0.0));
    }
    mg_uikit::name_group(ui, STRINGS_FOUND);
    crate::shell::first_row_ends(ui, ui.cursor().top() + tall);
    let margin = egui::Margin { left: 8, right: 8, top: 0, bottom: 2 };
    let mut chosen = None;
    let mut seen = (0, 0);
    egui::Frame::new().inner_margin(margin).show(ui, |ui| {
        // (No gap between rows: the pitch `show_rows` takes from `ui` is
        // the row's height, so that the 50,000th is where the scroll bar
        // says it is.)
        ui.spacing_mut().item_spacing.y = 0.0;
        rows.show_rows(ui, tall, d.hits.len(), |ui, range| {
            seen = (range.start, range.end);
            for i in range {
                let (hit, on) = &d.hits[i];
                let mut ticked = *on;
                let size = egui::vec2(ui.available_width(), tall);
                // (The row first, under what is in it: a click on it, not
                // on its tick or link, puts the keys on it.)
                let (rect, row) = ui.allocate_exact_size(size, egui::Sense::click());
                let is = current == Some(i);
                row.widget_info(|| {
                    let said = format!("{}: {}", hit.place, hit.text);
                    egui::WidgetInfo::selected(egui::WidgetType::SelectableLabel, true, is, &said)
                });
                if row.clicked() || row.secondary_clicked() {
                    chosen = Some(i);
                }
                if row.double_clicked() || row.triple_clicked() {
                    go = Some(i);
                }
                match row_copy::row_menu(ui, &row, keyed && is, |_| false) {
                    Some(row_copy::Asked::Row) => copy = Some(string_line(hit)),
                    Some(row_copy::Asked::All) => {
                        copy = Some(row_copy::lines(d.hits.iter().map(|h| string_line(&h.0))));
                    }
                    None => {}
                }
                if is {
                    ui.painter().rect_filled(rect, 0.0, t.accent_fill);
                }
                let down = egui::Layout::top_down(egui::Align::Min);
                let mut inner = ui.new_child(egui::UiBuilder::new().max_rect(rect).layout(down));
                inner.style_mut().wrap_mode = Some(egui::TextWrapMode::Truncate);
                // (On the chosen fill, its words in its own color, as the
                // tables'.)
                if is {
                    inner.visuals_mut().override_text_color = Some(t.on_fill);
                    inner.visuals_mut().hyperlink_color = t.on_fill;
                }
                inner.horizontal(|ui| {
                    let r = ui.add(mg_uikit::Checkbox::without_text(&mut ticked));
                    r.widget_info(|| {
                        let said = format!("Replace in {}", hit.place);
                        egui::WidgetInfo::selected(egui::WidgetType::Checkbox, true, ticked, &said)
                    });
                    if r.changed() {
                        tick = Some((i, ticked));
                    }
                    if ui.link(&hit.place).on_hover_text("Open it").clicked() {
                        go = Some(i);
                    }
                });
                let end = hit.text.char_indices().nth(160).map_or(hit.text.len(), |(b, _)| b);
                let text = &hit.text[..end];
                // (Not selectable: a click on the string chooses its row.
                // Whole in its tip, the row's words cut.)
                inner.indent(("hit", i), |ui| {
                    let text = egui::RichText::new(text);
                    let text = if is { text.color(t.on_fill) } else { text.weak() };
                    ui.add(egui::Label::new(text).selectable(false)).on_hover_text(&hit.text)
                });
                if is && has {
                    let ring = egui::Stroke::new(1.0, t.accent);
                    ui.painter().rect_stroke(rect, 2.0, ring, egui::StrokeKind::Inside);
                }
                // (A `line` under each hit, the board's `.hit`.)
                let stroke = egui::Stroke::new(1.0, t.line);
                ui.painter().hline(rect.x_range(), rect.bottom() - 0.5, stroke);
            }
        })
    });
    d.strings_seen = seen;
    if let Some(text) = copy {
        ui.ctx().copy_text(text);
    }
    if let Some(at) = chosen {
        current = Some(at);
        crate::palette_view::give_keys(ui.ctx(), Keys::SearchResults);
    }
    if let Some(at) = go {
        current = Some(at);
    }
    d.current = current;
    if let Some((at, on)) = tick {
        d.current = Some(at);
        d.tick(at, on);
    }
    go
}

/// A string found as one line of text (Copy, Ctrl+C, a line of Copy
/// All): its place and the whole string, "keep › creature GUARD ›
/// FirstName: Guard", its line breaks as spaces.
pub fn string_line(hit: &mg_module::text::Hit) -> String {
    format!("{}: {}", hit.place, hit.text.replace(['\r', '\n'], " "))
}

/// The lines of the scripts `keys` that `find` matches (as the Find
/// window's options say), at most [`tools::MAX_RESULTS`] of them, and how
/// many there were.
pub(crate) fn lines_found(
    app: &Moonglow,
    keys: &[ResKey],
    find: &str,
    options: &tools::SearchOptions,
) -> (Vec<Hit>, usize) {
    let mut hits = Vec::new();
    let mut found = 0;
    let ws = app.ws.as_ref();
    for k in keys {
        let text = crate::script_view::script_source(&app.scripts, ws, None, k).unwrap_or_default();
        for (line, text) in tools::find_lines(&text, find, options) {
            found += 1;
            if hits.len() < tools::MAX_RESULTS {
                hits.push(Hit::Line { script: *k, line, text });
            }
        }
    }
    (hits, found)
}
