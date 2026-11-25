//! The Bottom Dock's report tabs (M4's U28, Part 2's decision 5): Hak Pak
//! Conflict Analysis, Resources Used and the Claims Table, each a dialog
//! before. Each is a tab of the Bottom Dock while its state is held
//! (`app.hak_report`, `app.area_stats`, `app.claims.table`: the panels'
//! state, by the decision), opened and brought to the front by its command,
//! closed by its Close at the tab bar's end (or by what closed the dialog:
//! the module closed, the area's view gone, the claims no longer kept). Not
//! kept across runs: a report is of the module as it was asked for.
//!
//! Their tables are `report_table`'s: a header that sorts, the rows in
//! sight alone laid out, the arrows choosing one, Ctrl + C copying it.

use std::sync::Arc;

use egui::Ui;
use mg_module::claims::Whose;
use mg_module::haks::HakReport;
use mg_resman::ResKey;

use crate::Moonglow;
use crate::palette_view::Keys;
use crate::report_table::{self, Column};

/// A report tab brought to the front of the Bottom Dock and the dock
/// opened (its command), its table taking the keys.
pub(crate) fn bring(app: &mut Moonglow, panel: crate::shell::Panel) {
    crate::shell::focus_bottom(app, panel);
    app.settings.hide_log = false;
    app.dialog_panel_focus = Some(panel.keys());
}

impl Moonglow {
    /// Hak Pak Conflict Analysis showing `report` (its job's end), brought
    /// to the front of the Bottom Dock, its table taking the keys. Public
    /// for the tests and pictures, whose haks are not on the machine.
    #[doc(hidden)]
    pub fn show_hak_report(&mut self, report: HakReport) {
        self.hak_report = Some(HakConflicts::new(report));
        bring(self, crate::shell::Panel::HakConflicts);
    }

    /// The Claims Table's counters, while it is open (for the tests): how
    /// often its rows' order was made, and the rows laid out last frame.
    #[doc(hidden)]
    pub fn claims_table_counts(&self) -> Option<(u64, (usize, usize))> {
        let w = self.claims.table.as_ref()?;
        Some((w.table.builds(), w.table.seen()))
    }
}

/// The keys given to a report's table where its command (or F6) asked
/// for them, as it is drawn.
fn take_keys(app: &mut Moonglow, ui: &Ui, keys: Keys) {
    // (Given by its command: held while the pointer moves over the area's
    // view, until a press.)
    if app.dialog_panel_focus.take_if(|k| *k == keys).is_some() {
        crate::palette_view::hold_keys(ui.ctx(), keys);
    }
    if ui.input(|i| i.pointer.any_pressed()) && ui.rect_contains_pointer(ui.max_rect()) {
        crate::palette_view::give_keys(ui.ctx(), keys);
    }
    crate::palette_view::keys_outline(ui, ui.max_rect(), keys);
}

/// The report tab in front of the Bottom Dock, where one is and the dock
/// is open.
pub(crate) fn in_front(app: &Moonglow) -> Option<crate::shell::Panel> {
    use crate::shell::Panel;
    let p = crate::shell::bottom_shown(app).filter(|_| !app.settings.hide_log)?;
    matches!(p, Panel::HakConflicts | Panel::ResourcesUsed | Panel::ClaimsTable).then_some(p)
}

/// Its Close at the tab bar's end: the report let go of, its tab gone.
pub(crate) fn close(app: &mut Moonglow, panel: crate::shell::Panel) {
    use crate::shell::Panel;
    match panel {
        Panel::HakConflicts => app.hak_report = None,
        Panel::ResourcesUsed => app.area_stats = None,
        Panel::ClaimsTable => app.claims.table = None,
        _ => {}
    }
}

// ------------------------------------------------------------------ haks

/// Which of the report Hak Pak Conflict Analysis lists.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum HakView {
    /// The resources more than one hak provides.
    #[default]
    Conflicts,
    /// The haks' resources the base game has too.
    Overrides,
    /// Every resource of every hak.
    All,
    /// The haks, in priority order, and where each was found.
    Haks,
    /// What the module uses of its haks (What the Module Uses).
    Uses,
}

/// Hak Pak Conflict Analysis (Module Properties › Custom Content › Check
/// for Conflicts…): the report, the view shown, What the Module Uses' lines
/// once asked for.
#[derive(Debug, Clone, Default)]
pub struct HakConflicts {
    pub report: Arc<HakReport>,
    pub view: HakView,
    /// What the module uses of its haks, once worked out (its lines, as
    /// `mg haks --usage` says them).
    pub usage: Option<String>,
    pub(crate) table: report_table::Table,
    /// The rows of the view shown, made as it is chosen.
    rows: Option<(HakView, Arc<Vec<HakRow>>)>,
    /// How many resources more than one hak has, counted once.
    conflicts: usize,
    /// How many lines What the Module Uses said, counted once.
    uses: usize,
}

/// A row of Hak Pak Conflict Analysis' table.
#[derive(Debug, Clone)]
enum HakRow {
    /// A resource and its joined haks string, the first winning.
    Resource(ResKey, String),
    /// A hak and its file (or what is said of it not found).
    Hak(String, String, bool),
    /// A line of What the Module Uses.
    Line(String),
}

impl HakConflicts {
    /// A report shown on its conflicts.
    pub fn new(report: HakReport) -> HakConflicts {
        let conflicts = report.conflicts().count();
        HakConflicts { report: Arc::new(report), conflicts, ..Default::default() }
    }

    /// What the module uses of its haks, worked out: its lines kept and
    /// counted, the view on them.
    pub(crate) fn set_usage(&mut self, usage: String) {
        self.uses = usage.lines().count();
        self.usage = Some(usage);
        self.view = HakView::Uses;
        // (Its rows made again from these lines.)
        self.rows = None;
    }

    /// How often the rows' order was made (for the tests: once a report,
    /// view and sort, not each frame).
    #[doc(hidden)]
    pub fn order_builds(&self) -> u64 {
        self.table.builds()
    }

    /// The rows laid out last frame (for the tests: those in sight alone).
    #[doc(hidden)]
    pub fn rows_seen(&self) -> (usize, usize) {
        self.table.seen()
    }

    /// How many the view lists (its tab bar's words).
    pub fn count(&self, view: HakView) -> usize {
        match view {
            HakView::Conflicts => self.conflicts,
            HakView::Overrides => self.report.overrides.len(),
            HakView::All => self.report.resources.len(),
            HakView::Haks => self.report.haks.len(),
            HakView::Uses => self.uses,
        }
    }

    fn rows(&mut self) -> Arc<Vec<HakRow>> {
        if let Some((v, rows)) = &self.rows
            && *v == self.view
        {
            return rows.clone();
        }
        let r = &self.report;
        let res = |(k, h): (&ResKey, &Vec<String>)| HakRow::Resource(*k, h.join(", "));
        let rows: Vec<HakRow> = match self.view {
            HakView::Conflicts => r.conflicts().map(res).collect(),
            HakView::Overrides => r.overrides.iter().map(res).collect(),
            HakView::All => r.resources.iter().map(res).collect(),
            HakView::Haks => (r.haks.iter())
                .map(|(name, path)| match path {
                    Some(p) => HakRow::Hak(name.clone(), p.display().to_string(), true),
                    None => {
                        let said = r.not_found.get(name).cloned();
                        HakRow::Hak(name.clone(), said.unwrap_or_else(|| "Not found".into()), false)
                    }
                })
                .collect(),
            HakView::Uses => (self.usage.iter())
                .flat_map(|u| u.lines())
                .map(|l| HakRow::Line(l.into()))
                .collect(),
        };
        let rows = Arc::new(rows);
        self.rows = Some((self.view, rows.clone()));
        self.table.clear();
        rows
    }
}

/// The accessible name of Hak Pak Conflict Analysis' table.
pub const HAK_ROWS: &str = "Hak resources";

const RESOURCE_COLUMNS: [Column; 3] =
    [Column::shared("Resource", 1.0), Column::fixed("Type", 70.0), Column::shared("Haks", 2.0)];
const HAK_COLUMNS: [Column; 2] = [Column::shared("Hak", 1.0), Column::shared("File", 2.0)];
const USE_COLUMNS: [Column; 1] = [Column::shared("What the Module Uses", 1.0).unsorted()];

/// Hak Pak Conflict Analysis' body: what the report lists (Conflicts,
/// Overrides, All, Haks; What the Module Uses once asked for), chosen over
/// its table; What the Module Uses and Copy Report beside them.
pub(crate) fn hak_body(app: &mut Moonglow, ui: &mut Ui) {
    take_keys(app, ui, Keys::HakReport);
    let busy = app.busy();
    let Some(h) = app.hak_report.as_mut() else { return };
    let mut usage = false;
    let mut copy = false;
    header(ui, |ui| {
        let n = |v| h.count(v);
        let words = [
            format!("Conflicts ({})", n(HakView::Conflicts)),
            format!("Overrides ({})", n(HakView::Overrides)),
            format!("All ({})", n(HakView::All)),
            format!("Haks ({})", n(HakView::Haks)),
            "What the Module Uses".to_string(),
        ];
        let mut choices = vec![
            (HakView::Conflicts, words[0].as_str(), "The resources more than one hak provides"),
            (HakView::Overrides, words[1].as_str(), "The haks' resources the game has too"),
            (HakView::All, words[2].as_str(), "Every resource of every hak"),
            (HakView::Haks, words[3].as_str(), "The haks, the first winning, and their files"),
        ];
        if h.usage.is_some() {
            choices.push((HakView::Uses, words[4].as_str(), "What the module uses of its haks"));
        }
        mg_uikit::segmented(ui, "Hak report view", &mut h.view, &choices);
        // (What the Module Uses and Copy Report are plain buttons at the
        // header's right end, apart from the view switch, not more of its
        // segments: on the next line where the room is too narrow.)
        let font = egui::TextStyle::Button.resolve(ui.style());
        let pad = ui.spacing().button_padding.x * 2.0;
        let ppp = ui.ctx().pixels_per_point().to_bits();
        let width_id = egui::Id::new("hak-report-buttons-width");
        let width_key = (font.size.to_bits(), pad.to_bits(), ppp);
        let both = ui
            .data(|d| d.get_temp::<((u32, u32, u32), f32)>(width_id))
            .filter(|(k, _)| *k == width_key)
            .map(|(_, w)| w)
            .unwrap_or_else(|| {
                let width = |words: &str| {
                    let g = ui.fonts_mut(|f| {
                        f.layout_no_wrap(words.into(), font.clone(), egui::Color32::PLACEHOLDER)
                    });
                    g.size().x + pad
                };
                let w = width("What the Module Uses") + width("Copy Report");
                ui.data_mut(|d| d.insert_temp(width_id, (width_key, w)));
                w
            });
        let need = both + ui.spacing().item_spacing.x;
        let left = ui.available_size_before_wrap().x;
        if left >= need + REPORT_GAP {
            ui.add_space(left - need - ui.spacing().item_spacing.x);
        } else {
            ui.end_row();
        }
        let tip = "Which haks' resources the module uses, hak by hak, to trim a release";
        usage = ui
            .add_enabled(!busy, egui::Button::new("What the Module Uses"))
            .on_hover_text(tip)
            .clicked();
        copy = ui.button("Copy Report").on_hover_text("Copy the whole report as text").clicked();
    });
    let rows = h.rows();
    let view = h.view;
    let (cols, sorts): (&[Column], bool) = match view {
        HakView::Haks => (&HAK_COLUMNS, true),
        HakView::Uses => (&USE_COLUMNS, false),
        _ => (&RESOURCE_COLUMNS, true),
    };
    let key = view as u64;
    // (The haks in their priority order, the first winning: their Hak
    // column sorts by it, up or down; File by the file's words.)
    let order =
        h.table.order(
            key,
            rows.len(),
            |_| true,
            |a, b, col| {
                if !sorts {
                    return a.cmp(&b);
                }
                match (&rows[a], &rows[b]) {
                    (HakRow::Resource(ka, ha), HakRow::Resource(kb, hb)) => match col {
                        0 => {
                            let a_s =
                                ka.resref.as_str().map(std::borrow::Cow::Borrowed).unwrap_or_else(
                                    || std::borrow::Cow::Owned(ka.resref.to_string()),
                                );
                            let b_s =
                                kb.resref.as_str().map(std::borrow::Cow::Borrowed).unwrap_or_else(
                                    || std::borrow::Cow::Owned(kb.resref.to_string()),
                                );
                            report_table::by_words(&a_s, &b_s).then(ka.restype.0.cmp(&kb.restype.0))
                        }
                        1 => ext(ka).cmp(ext(kb)).then(ka.resref.cmp(&kb.resref)),
                        _ => report_table::by_words(ha, hb),
                    },
                    (HakRow::Hak(na, fa, _), HakRow::Hak(nb, fb, _)) => match col {
                        0 => a.cmp(&b),
                        _ => report_table::by_words(fa, fb).then(report_table::by_words(na, nb)),
                    },
                    _ => a.cmp(&b),
                }
            },
        );
    let t = mg_uikit::tokens(ui.ctx());
    let spec = report_table::Spec {
        id: egui::Id::new("hak-report-table"),
        name: HAK_ROWS,
        cols,
        keys: Keys::HakReport,
        may_take_keys: true,
    };
    if rows.is_empty() {
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.add_space(8.0);
            ui.weak(match view {
                HakView::Conflicts => "No resource is in more than one hak.",
                HakView::Overrides => "No hak resource is one the game has too.",
                HakView::Haks => "The module uses no haks.",
                _ => "Nothing to list.",
            });
        });
    } else {
        report_table::show(
            ui,
            &mut h.table,
            &spec,
            &order,
            |i| match &rows[i] {
                HakRow::Resource(k, haks) => (k.to_string(), haks.clone()),
                HakRow::Hak(n, f, _) => (n.clone(), f.clone()),
                HakRow::Line(l) => (l.trim().to_string(), String::new()),
            },
            |ui, i, cells, on| {
                let (text, quiet) = if on { (t.on_fill, t.on_fill) } else { (t.text, t.text_2) };
                match &rows[i] {
                    HakRow::Resource(k, haks) => {
                        report_table::text(ui, cells[0], k.resref.to_string(), text, false);
                        report_table::text(ui, cells[1], ext(k), quiet, false);
                        report_table::text(ui, cells[2], haks.clone(), text, false);
                    }
                    HakRow::Hak(n, f, found) => {
                        report_table::text(ui, cells[0], n.clone(), text, false);
                        let color = if *found || on { quiet } else { t.warn };
                        report_table::text(ui, cells[1], f.clone(), color, false);
                    }
                    HakRow::Line(l) => report_table::text(ui, cells[0], l.clone(), text, false),
                }
            },
        );
    }
    let Some(h) = app.hak_report.as_mut() else { return };
    if copy {
        let mut text = String::new();
        if let Some(u) = &h.usage {
            text.push_str(u);
            text.push_str("\n\n");
        }
        text.push_str(&h.report.to_text());
        ui.ctx().copy_text(text);
    }
    if usage {
        app.hak_usage_job();
    }
}

fn ext(k: &ResKey) -> &'static str {
    k.restype.extension().unwrap_or("?")
}

/// The least gap between a header's view switch and its buttons at its
/// right end.
const REPORT_GAP: f32 = 16.0;

/// The tools' row over a tab's table: the panel's margins.
fn header<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
    let margin = egui::Margin { left: 8, right: 8, top: 4, bottom: 4 };
    egui::Frame::new().inner_margin(margin).show(ui, |ui| ui.horizontal_wrapped(add).inner).inner
}

// ---------------------------------------------------------- resources used

/// The accessible name of Resources Used's table.
pub const USED_ROWS: &str = "Resources used";

const USED_COLUMNS: [Column; 2] =
    [Column::fixed("Resource", 140.0).unsorted(), Column::shared("Used", 1.0).unsorted()];

/// Resources Used (Build › Area Statistics, Aurora's Resources Used): what
/// the area's view has loaded, counted once the view has read it (again
/// as it reads anew, not each frame), in a table of the kit's rows. The
/// area's view gone, the tab goes as the window did.
pub(crate) fn used_body(app: &mut Moonglow, ui: &mut Ui) {
    take_keys(app, ui, Keys::ResourcesUsed);
    let Some(area) = app.area_stats else { return };
    let Some(view) = app.area_views.get(&area) else { return };
    let made_for = (area, view.read_at(), view.reading());
    if app.used_rows.as_ref().is_none_or(|(k, _)| *k != made_for) {
        app.used_rows = Some((made_for, Arc::new(crate::area_view::resources_used(view))));
    }
    let rows = app.used_rows.as_ref().map(|(_, r)| r.clone()).unwrap_or_default();
    let label = crate::tabs::area_label(app, area);
    header(ui, |ui| {
        ui.strong(&label);
        if label != area.to_string() {
            ui.weak(area.to_string());
        }
    });
    let order_id = egui::Id::new("used-rows-order");
    let order = ui
        .data(|d| d.get_temp::<Arc<Vec<usize>>>(order_id))
        .filter(|o| o.len() == rows.len())
        .unwrap_or_else(|| {
            let o: Arc<Vec<usize>> = Arc::new((0..rows.len()).collect());
            ui.data_mut(|d| d.insert_temp(order_id, o.clone()));
            o
        });
    let t = mg_uikit::tokens(ui.ctx());
    let spec = report_table::Spec {
        id: egui::Id::new("resources-used-table"),
        name: USED_ROWS,
        cols: &USED_COLUMNS,
        keys: Keys::ResourcesUsed,
        may_take_keys: true,
    };
    let mut table = std::mem::take(&mut app.used_table);
    report_table::show(
        ui,
        &mut table,
        &spec,
        &order,
        |i| (rows[i].0.to_string(), rows[i].1.clone()),
        |ui, i, cells, on| {
            let (text, quiet) = if on { (t.on_fill, t.on_fill) } else { (t.text, t.text_2) };
            report_table::text(ui, cells[0], rows[i].0, quiet, false);
            report_table::text(ui, cells[1], rows[i].1.clone(), text, false);
        },
    );
    app.used_table = table;
}

/// Each frame's start: Resources Used let go of where its area's view is
/// gone (the area closed), as the window was.
pub(crate) fn follow(app: &mut Moonglow) {
    if let Some(area) = app.area_stats
        && !app.area_views.contains_key(&area)
    {
        app.area_stats = None;
    }
    if app.claims.table.is_some() && !crate::claims::kept(app) {
        app.claims.table = None;
    }
}

// ------------------------------------------------------------ claims table

/// The accessible name of the Claims Table's rows.
pub const CLAIM_ROWS: &str = "Claimed areas";

/// The Claims Table's columns in `room` (M4's U28 review): Area, Whose and
/// Notes as wide as they share, and the row's actions `actions` wide; in a
/// narrow room the first three narrowed first, and where the actions still
/// do not fit, folded into a row's menu button (`folded`). Whether they are.
pub(crate) fn claim_columns(room: f32, actions: f32) -> ([Column; 4], bool) {
    let cols = |area: f32, whose: f32, notes: f32, last: f32| {
        [
            Column::fixed("Area", area),
            Column::shared("Whose", 1.4).at_least(whose),
            Column::shared("Notes", 1.0).at_least(notes),
            Column::fixed("", last).unsorted(),
        ]
    };
    let wide = cols(130.0, 90.0, 90.0, actions);
    if Column::least_of(&wide) <= room {
        return (wide, false);
    }
    let narrow = cols(96.0, 64.0, 44.0, actions);
    if Column::least_of(&narrow) <= room {
        return (narrow, false);
    }
    (cols(96.0, 64.0, 44.0, CLAIM_MENU), true)
}

/// A Claims Table row's actions, as its buttons and its menu (folded, at a
/// narrow width) name them.
pub const RELEASE: &str = "Release";
pub const HAND_TO: &str = "Hand To…";
pub const TAKE_OVER: &str = "Take Over…";
pub const CLAIM: &str = "Claim";
/// The menu button a Claims Table row's actions fold into.
pub const ACTIONS: &str = "Actions";

/// The width of a Claims Table row's menu button, its actions folded.
const CLAIM_MENU: f32 = 36.0;

/// The Claims Table (Edit › Claims Table…): every area of the table and
/// the module, whose each is, and what this builder may do with each
/// (Claim, Release, Hand To…, Take Over…), as `mg claims` lists and
/// changes them; Take Over's question and Hand To's field in the header,
/// over the table, while asked.
pub(crate) fn claims_body(app: &mut Moonglow, ui: &mut Ui) {
    take_keys(app, ui, Keys::ClaimsTable);
    crate::claims::table_body(app, ui);
}

/// A claim's holder as the table says it ("yours: ab", "cd", "free").
pub(crate) fn whose_words(l: &mg_module::claims::Listed) -> String {
    match (l.whose, l.claim.as_ref().and_then(|c| c.holder())) {
        (Whose::Mine, Some(h)) => format!("yours: {h}"),
        (_, Some(h)) => h,
        (_, None) => "free".to_string(),
    }
}

/// A claim's notes ("merged", "protected", "not in the module").
pub(crate) fn notes_words(l: &mg_module::claims::Listed) -> String {
    let mut notes = Vec::new();
    if l.claim.as_ref().is_some_and(|c| c.merged()) {
        notes.push("merged");
    } else if l.protected {
        notes.push("protected");
    }
    if !l.in_module {
        notes.push("not in the module");
    }
    notes.join(", ")
}

// ------------------------------------------------------------------ hints

/// The Status Bar's hints while a list of the Bottom Dock holds the keys
/// (M4's U28 review, U13's hints): Search Results' (a script's lines, Find
/// Instance's objects, Find and Replace Text's strings), Problems' and the
/// three reports' tables, each saying only what its keys and pointer do in
/// the state at hand (nothing where it lists nothing; Enter, Space and
/// Ctrl + C only with a row chosen). In the area view's place: its keys
/// are not its own meanwhile. None where no dock list holds the keys, or
/// they act on nothing (a menu, a popover or a dialog over it, a field
/// typed in).
pub(crate) fn hints(
    app: &Moonglow,
    ctx: &egui::Context,
    k: &crate::shell::HintKeys<'_>,
) -> Option<Vec<mg_uikit::status::Hint>> {
    use crate::search_results::Showing;
    use crate::shell::Panel;
    use mg_uikit::status::Hint;
    let front = crate::shell::bottom_shown(app).filter(|_| !app.settings.hide_log)?;
    let lists = [
        Panel::SearchResults,
        Panel::Problems,
        Panel::HakConflicts,
        Panel::ResourcesUsed,
        Panel::ClaimsTable,
    ];
    if !lists.contains(&front) {
        return None;
    }
    // (A field of Search Results' form has them: its keys, not the
    // editor's under it.)
    if front == Panel::SearchResults
        && crate::palette_view::keys_are(ctx, front.keys())
        && report_table::text_has_keys(ctx)
    {
        return Some(vec![
            Hint::keys(["Enter"], "Search"),
            Hint::keys(["Esc"], "Close"),
            Hint::keys(["Tab"], "Next field"),
            Hint::keys(["Shift", "Tab"], "Previous field"),
        ]);
    }
    if !report_table::has_keys(ctx, front.keys()) {
        return None;
    }
    let ctrl = k.ctrl();
    let arrows = || Hint::either(["Up", "Down"], "Move in the list");
    let copy = || Hint::keys([ctrl, "C"], "Copy");
    let mut hints = Vec::new();
    // (Rows, and whether one is chosen.)
    let (rows, chosen) = match front {
        Panel::SearchResults => {
            let r = &app.search_results;
            match r.showing {
                Showing::Instances => {
                    let n = app.find_instance.as_ref().filter(|f| f.searched);
                    let n = n.map_or(0, |f| f.results.len());
                    let chosen = r.instances.selected.is_some();
                    if n > 0 {
                        hints.push(arrows());
                        if chosen {
                            hints.push(Hint::keys(["Enter"], "Go to"));
                        }
                        hints.push(Hint::keys(["Double Click"], "Go to"));
                        if chosen {
                            hints.push(copy());
                        }
                    }
                    return Some(hints);
                }
                Showing::Text => {
                    let n = r.count(app);
                    let chosen = app.text_replace.as_ref().is_some_and(|d| d.current.is_some());
                    if n > 0 {
                        hints.push(arrows());
                        if chosen {
                            hints.push(Hint::keys(["Space"], "Check"));
                            hints.push(Hint::keys(["Enter"], "Go to"));
                        }
                        hints.push(Hint::keys(["Double Click"], "Go to"));
                        if chosen {
                            hints.push(copy());
                        }
                    }
                    return Some(hints);
                }
                Showing::Lines => {
                    if !r.hits.is_empty() {
                        hints.push(arrows());
                        if r.line.is_some() {
                            hints.push(Hint::keys(["Enter"], "Open"));
                        }
                        hints.push(Hint::keys(["Click"], "Open"));
                        if r.line.is_some() {
                            hints.push(copy());
                        }
                    }
                    return Some(hints);
                }
            }
        }
        Panel::Problems => {
            let any = app.problems.sets().iter().any(|s| !s.problems.is_empty());
            let chosen = app.problems.selected();
            if any {
                hints.push(arrows());
                if chosen.is_some_and(|p| p.go.is_some()) {
                    hints.push(Hint::keys(["Enter"], "Go to"));
                }
                hints.push(Hint::keys(["Double Click"], "Go to"));
                if chosen.is_some_and(|p| p.unused.is_some()) {
                    hints.push(Hint::keys(["Right Click"], "Mark as used"));
                }
                if chosen.is_some() {
                    hints.push(copy());
                }
            }
            return Some(hints);
        }
        Panel::HakConflicts => match &app.hak_report {
            Some(h) => (h.count(h.view) > 0, h.table.selected.is_some()),
            None => (false, false),
        },
        Panel::ResourcesUsed => (
            app.used_rows.as_ref().is_some_and(|(_, r)| !r.is_empty()),
            app.used_table.selected.is_some(),
        ),
        Panel::ClaimsTable => match &app.claims.table {
            Some(w) => (w.table.seen().1 > 0, w.table.selected.is_some()),
            None => (false, false),
        },
        _ => return None,
    };
    if rows {
        hints.push(arrows());
        if chosen {
            hints.push(copy());
        }
    }
    Some(hints)
}

#[cfg(test)]
mod tests {
    use super::{CLAIM_MENU, claim_columns};
    use crate::report_table::Column;

    /// The Claims Table's columns: as wide as they share where the room
    /// allows; the Area, Whose and Notes columns narrowed first; and the
    /// row's actions folded into its menu button only where even the
    /// narrowed columns leave too little room for them. Never wider than
    /// the room while anything would fit.
    #[test]
    fn the_claims_columns_narrow_first_and_fold_last() {
        let actions = 150.0;
        let (wide, folded) = claim_columns(1200.0, actions);
        assert!(!folded && wide[0].width == Some(130.0) && wide[3].width == Some(actions));
        let (narrow, folded) = claim_columns(380.0, actions);
        assert!(!folded, "narrowed, not folded, at the least window's dock");
        assert!(narrow[0].width.unwrap() < 130.0 && Column::least_of(&narrow) <= 380.0);
        assert_eq!(narrow[3].width, Some(actions), "the actions kept whole");
        let (fold, folded) = claim_columns(300.0, actions);
        assert!(folded, "folded where the narrowed columns leave too little");
        assert_eq!(fold[3].width, Some(CLAIM_MENU));
        assert!(Column::least_of(&fold) <= 300.0, "the folded columns fit");
    }
}
