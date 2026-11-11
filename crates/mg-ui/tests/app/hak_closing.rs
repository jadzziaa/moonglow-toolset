//! Closing the hak editor after a file was added to its hak, by every
//! route the window has (the tab's ×, its window's ×, Close Tab, the
//! foot's OK and Cancel, Escape, the window's X, File › Exit and File ›
//! Close), answering every question each asks: a builder's report against
//! 4.0 said the editor would not close. The window's X asked of "the new
//! module" with none open; once the hak's question was answered, Exit and
//! Close were dropped with nothing on screen; a save that failed (the
//! added file gone from disk by then) said why only in the log, and its
//! question came back for ever; Don't Save on the module's question
//! dropped the hak's changes unasked; and a changed hak outlived its tab.
//!
//! One test runs the matrix of cells, each on a window of its own, holds
//! each to what the fixed window does, and writes what each ended in to
//! `target/test-output/hak-closing/matrix.md`; it fails once, at the end,
//! naming the cells that deviate. `HAK_CLOSING_ONLY=<text>` runs the cells
//! whose name has that text. The tests after it are the cases the matrix
//! leaves out: several changed documents at once, a failed save put right,
//! the documents following their tabs, and the 2DA and tileset editors.
//!
//! (Jobs as the app runs them, in the background, everywhere; at once
//! (`bg0`) for a sample: they gave the same end in every cell. A module
//! listing the hak in its hak list was a setting of its own; without the
//! game installed nothing reads the list, and it ended as the module that
//! does not, so it is left out.)

use std::path::{Path, PathBuf};

use crate::common::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Setting {
    /// No module open.
    NoModule,
    /// A saved module that does not list the hak.
    Module,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    /// A hak opened from its file, a new file added by Add Files….
    Opened,
    /// The same, the file added one the hak has, Replace answered.
    Replaced,
    /// A new hak, a file added; its Save asks where and is given a path.
    NewSaved,
    /// The same, the save dialog canceled.
    NewCanceled,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Route {
    /// egui_dock's × on the hak's tab.
    TabX,
    /// The × of the window the tab is in (egui_dock's close-all).
    DockWindowX,
    /// `Action::CloseTabs` (middle click, Close Others / All).
    CloseTabs,
    /// Ctrl+W, the close-tab command.
    CtrlW,
    /// The foot's OK.
    FootOk,
    /// The foot's Cancel.
    FootCancel,
    /// Escape over the tab: nothing, by design (Escape cancels the
    /// windows `tabs.rs` lists, Properties windows, not an editor).
    Escape,
    /// The OS window's X (`Action::CloseWindow`, as `apps/moonglow` pushes).
    WindowX,
    /// File › Exit.
    FileExit,
    /// File › Close.
    FileClose,
}

impl Route {
    fn closes_tab(self) -> bool {
        !matches!(self, Route::WindowX | Route::FileExit | Route::FileClose)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Answer {
    Save,
    DontSave,
    Cancel,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Variant {
    Plain,
    /// The editor's own Save pressed before the route.
    EditorSaveFirst,
    /// The route pressed while the Save Hak job runs (window routes): it
    /// is refused with a line in the log, by design (nothing replaces the
    /// module or closes the window under a job, whose window is on
    /// screen); the route again once it is done.
    WhileSaving,
    /// The added file deleted from disk before the route.
    FileDeleted,
    /// The hak's target can't be written (under a plain file).
    Unwritable,
    /// The module changed too (a table added to it), so that its own
    /// question comes first.
    ModuleChanged,
}

#[derive(Clone, Debug)]
struct Cell {
    variant: Variant,
    setting: Setting,
    kind: Kind,
    route: Route,
    answers: Vec<Answer>,
    background: bool,
}

impl Cell {
    fn name(&self) -> String {
        let answers: Vec<String> = self.answers.iter().map(|a| format!("{a:?}")).collect();
        format!(
            "{:?}/{:?}/{:?}/{:?}/{}/bg{}",
            self.variant,
            self.setting,
            self.kind,
            self.route,
            answers.join("+"),
            u8::from(self.background)
        )
    }

    fn failing(&self) -> bool {
        matches!(self.variant, Variant::FileDeleted | Variant::Unwritable)
    }
}

/// A question as asked.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Q {
    /// "Save changes to" the hak, by its name.
    Hak,
    /// "Save changes to sample.mod?"
    Module,
    /// The window's X with nothing unsaved.
    CloseMoonglow,
    /// Anything else ("the new module").
    Other(String),
}

fn q_of(text: &str) -> Q {
    match text {
        "Close Moonglow?" => Q::CloseMoonglow,
        "Save changes to sample.mod?" => Q::Module,
        t if ["mine.hak?", "new.hak?", "New hak?"].iter().any(|n| t.ends_with(n)) => Q::Hak,
        t => Q::Other(t.to_string()),
    }
}

/// What a cell ended in.
#[derive(Debug, Default)]
struct Obs {
    clean_before: bool,
    route_note: String,
    asked: Vec<Q>,
    answered: Vec<Answer>,
    on_screen: Vec<String>,
    quit_requested: bool,
    tab_open: bool,
    haks: usize,
    /// Side documents left without a tab.
    orphans: usize,
    dirty: Option<bool>,
    confirm_discard: Option<String>,
    side_then: Option<String>,
    busy: bool,
    module_open: bool,
    on_disk: String,
    goal: bool,
    /// Why a save failed, seen outside the log panel (the question, the
    /// editor's foot) at some point.
    reason_seen: bool,
    retry: Option<String>,
    during_job: Option<String>,
    log_tail: Vec<String>,
}

/// What the fixed window does in a cell.
#[derive(Debug, PartialEq)]
struct Want {
    asked: Vec<Q>,
    goal: bool,
    /// A question still on screen at the end (a Save that can't be done,
    /// answered five times).
    asking: bool,
    on_disk: &'static str,
    reason_seen: bool,
    /// The route done a second time (after a job refused it).
    retried: bool,
}

const ADDED: &str = "mg_new.2da";
const REPLACED: &str = "mg_a.2da";
const NEW_TABLE: &str = "2DA V2.0\n\n   Label\n0  added\n";
/// The questions answered in a cell at most.
const MOST: usize = 5;

/// The hak a test opens: two tables, written by Moonglow's own hak code.
fn author_hak(path: &Path, src: &Path) {
    let a = src.join("orig_a").join("mg_a.2da");
    let b = src.join("orig_b").join("mg_b.2da");
    for (f, t) in [(&a, "a"), (&b, "b")] {
        std::fs::create_dir_all(f.parent().unwrap()).unwrap();
        std::fs::write(f, format!("2DA V2.0\n\n   Label\n0  {t}\n")).unwrap();
    }
    let mut hak = mg_module::hak_edit::Hak::new();
    hak.add_files(&[a, b]);
    hak.save(Some(path)).unwrap();
}

/// A saved module.
fn module(dir: &Path) -> PathBuf {
    let mut m = Module::new();
    let mut info = Gff::new(*b"IFO ");
    info.root.write(&ifo::MOD_TAG, ExoString::from("SAMPLE"));
    m.set_info(&info).unwrap();
    m.set(ResKey::parse("hello", ResType::NSS).unwrap(), b"void main() { }\n".to_vec());
    let path = dir.join("sample.mod");
    m.save_as(&ModuleLocation::Archive(path.clone())).unwrap();
    path
}

/// The rect of the dock leaf `tab` is in, and of its body.
fn leaf_rects(h: &Harness<'_, Moonglow>, tab: &Tab) -> Option<(egui::Rect, egui::Rect)> {
    h.state()
        .dock
        .iter_leaves()
        .find(|(_, l)| l.tabs.contains(tab))
        .map(|(_, l)| (l.rect, l.viewport))
}

/// Clicks the last button `label` inside the hak's leaf: whether one was.
fn click_in_tab(h: &mut Harness<'_, Moonglow>, tab: &Tab, label: &str) -> bool {
    let Some((rect, _)) = leaf_rects(h, tab) else { return false };
    let node = h.query_all_by_label(label).rfind(|n| rect.contains_rect(n.rect()));
    match node {
        Some(n) => {
            n.click();
            true
        }
        None => false,
    }
}

/// A node's text: a label's is its value (AccessKit's way).
fn text_of(n: &egui_kittest::Node<'_>) -> Option<String> {
    let a = n.accesskit_node();
    a.label().or_else(|| a.value())
}

/// The questions on screen.
fn questions(h: &Harness<'_, Moonglow>) -> Vec<String> {
    let mut out: Vec<String> =
        h.query_all_by_label_contains("Save changes to").filter_map(|n| text_of(&n)).collect();
    if h.query_all_by_label("Close Moonglow?").next().is_some() {
        out.push("Close Moonglow?".into());
    }
    out
}

/// What is on screen of the windows a close can show.
fn on_screen(h: &Harness<'_, Moonglow>) -> Vec<String> {
    let mut out = questions(h);
    for probe in ["Unsaved Changes", "more tab", "Save Hak", "still running", "could not be saved"]
    {
        let found: Vec<String> = h
            .query_all_by_label_contains(probe)
            .filter_map(|n| text_of(&n))
            .filter(|l| !out.contains(l))
            .collect();
        out.extend(found);
    }
    out
}

/// Whether why a save failed is on screen outside the log panel (whose
/// lines begin with their time): in a question, under an editor.
fn reason_shown(h: &Harness<'_, Moonglow>) -> usize {
    let in_log = |t: &str| {
        let b = t.as_bytes();
        b.len() > 9 && b[2] == b':' && b[5] == b':' && b[8] == b' '
    };
    h.query_all_by_label_contains("could not be saved")
        .filter_map(|n| text_of(&n))
        .filter(|t| !in_log(t))
        .count()
}

/// Settles, without a panic hiding the rest of a cell (said in `note`).
fn settle_noting(h: &mut Harness<'_, Moonglow>, note: &mut String) {
    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| settle(h)));
    if let Err(e) = r {
        note.push_str(&format!(" [settle: {}]", panic_text(&e)));
    }
}

fn panic_text(e: &Box<dyn std::any::Any + Send>) -> String {
    e.downcast_ref::<String>()
        .cloned()
        .or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string()))
        .unwrap_or_else(|| "panic".into())
}

/// Answers the questions that come, in turn, `answers` (the last for the
/// rest), at most [`MOST`] in all: what was asked and answered.
fn answer_all(h: &mut Harness<'_, Moonglow>, answers: &[Answer], obs: &mut Obs, note: &mut String) {
    loop {
        settle_noting(h, note);
        obs.reason_seen |= reason_shown(h) > 0;
        let q = questions(h);
        if q.is_empty() || obs.answered.len() >= MOST {
            return;
        }
        let n = obs.answered.len();
        let answer = answers[n.min(answers.len() - 1)];
        obs.asked.push(match q.as_slice() {
            [one] => q_of(one),
            more => Q::Other(more.join(" & ")),
        });
        obs.answered.push(answer);
        let label = match (answer, q.iter().any(|q| q == "Close Moonglow?")) {
            (Answer::Cancel, _) => "Cancel",
            (_, true) => "Close Moonglow",
            (Answer::Save, _) => "Save",
            (Answer::DontSave, _) => "Don't Save",
        };
        let Some(node) = h.query_all_by_label(label).last() else {
            note.push_str(&format!(" [no {label} button]"));
            return;
        };
        node.click();
        h.step();
    }
}

/// Opens a menu of the menu bar and clicks its item: whether it was there
/// (what was found instead goes into `note`).
fn menu(h: &mut Harness<'_, Moonglow>, name: &str, item: &str, note: &mut String) -> bool {
    let Some(m) = h.query_all_by_label(name).next() else {
        note.push_str(&format!(" [no {name} menu]"));
        return false;
    };
    m.click();
    h.step();
    h.step();
    let found = h.query_all_by_label(item).last().or_else(|| {
        h.query_all_by_label_contains(item)
            .rfind(|n| text_of(n).is_some_and(|t| t.starts_with(item)))
    });
    let Some(i) = found else {
        let seen: Vec<String> =
            h.query_all_by_label_contains(&item[..2]).filter_map(|n| text_of(&n)).collect();
        note.push_str(&format!(" [{name} › {item} not found; seen {seen:?}]"));
        return false;
    };
    i.click();
    h.step();
    true
}

/// Does the route once.
fn do_route(h: &mut Harness<'_, Moonglow>, route: Route, tab: &Tab, note: &mut String) {
    match route {
        Route::TabX => {
            if leaf_rects(h, tab).is_some() {
                let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    click_tab_close(h, tab);
                }));
                if let Err(e) = r {
                    note.push_str(&format!(" [tab ×: {}]", panic_text(&e)));
                }
            }
        }
        Route::DockWindowX => {
            if leaf_rects(h, tab).is_some() {
                let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    click_window_close(h, tab);
                }));
                if let Err(e) = r {
                    note.push_str(&format!(" [window ×: {}]", panic_text(&e)));
                }
            }
        }
        Route::CloseTabs => {
            h.state_mut().actions.push(mg_ui::Action::CloseTabs(vec![tab.clone()]));
            h.step();
        }
        Route::CtrlW => {
            if let Some((_, body)) = leaf_rects(h, tab) {
                click_pos(h, body.center());
                h.step();
            }
            h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::W);
            h.step();
        }
        Route::FootOk | Route::FootCancel => {
            let label = if route == Route::FootOk { "OK" } else { "Cancel" };
            if !click_in_tab(h, tab, label) {
                note.push_str(&format!(" [no {label} in the tab]"));
            }
            h.step();
        }
        Route::Escape => {
            if let Some((_, body)) = leaf_rects(h, tab) {
                h.hover_at(body.center());
                h.step();
            }
            h.key_press(egui::Key::Escape);
            h.step();
        }
        Route::WindowX => {
            h.state_mut().actions.push(mg_ui::Action::CloseWindow);
            h.step();
        }
        Route::FileExit => {
            if !menu(h, "File", "Exit", note) {
                h.state_mut().actions.push(mg_ui::Action::Quit);
                h.step();
            }
        }
        Route::FileClose => {
            if !menu(h, "File", "Close", note) {
                h.state_mut().actions.push(mg_ui::Action::Close);
                h.step();
            }
        }
    }
}

fn goal(h: &Harness<'_, Moonglow>, route: Route, tab: &Tab) -> bool {
    match route {
        Route::WindowX | Route::FileExit => h.state().quit_requested,
        Route::FileClose => h.state().ws.is_none(),
        _ => h.state().dock.find_tab(tab).is_none(),
    }
}

/// Side documents whose tab is not in the dock.
fn orphans(app: &Moonglow) -> usize {
    let tabs = (app.haks.iter().map(|d| Tab::Hak(d.id)))
        .chain(app.tables.iter().map(|d| Tab::Table(d.id)))
        .chain(app.tilesets.iter().map(|d| Tab::Tileset(d.id)));
    tabs.filter(|t| app.dock.find_tab(t).is_none()).count()
}

fn run_cell(cell: &Cell, dir: &Path) -> Obs {
    let mut obs = Obs::default();
    let mut note = String::new();
    std::fs::create_dir_all(dir).unwrap();
    let src = dir.join("src");
    std::fs::create_dir_all(&src).unwrap();
    let hak_path = dir.join("mine.hak");
    let new_path = dir.join("new.hak");
    std::fs::write(dir.join("a_file"), "x").unwrap();
    let (added, key_name) = match cell.kind {
        Kind::Replaced => (src.join(REPLACED), "mg_a"),
        _ => (src.join(ADDED), "mg_new"),
    };
    std::fs::write(&added, NEW_TABLE).unwrap();
    if matches!(cell.kind, Kind::Opened | Kind::Replaced) {
        author_hak(&hak_path, &src);
    }
    let save = match (cell.kind, cell.variant) {
        (Kind::NewSaved, Variant::Unwritable) => vec![dir.join("a_file").join("new.hak")],
        (Kind::NewSaved, _) => vec![new_path.clone()],
        _ => Vec::new(),
    };
    let dialogs = NoDialogs { open_many: vec![vec![added.clone()]], save, ..Default::default() };
    let mut app = Moonglow::new(None, Box::new(dialogs));
    app.settings.no_last_area = true;
    if cell.setting == Setting::Module {
        app.open_module(&module(dir));
    }
    app.background_jobs = cell.background;
    let mut h = harness(app, 1600.0, 1000.0);
    settle_noting(&mut h, &mut note);
    obs.clean_before = !h.state().has_unsaved_work();
    let id = match cell.kind {
        Kind::Opened | Kind::Replaced => mg_ui::hak_view::open_at(h.state_mut(), &hak_path),
        Kind::NewSaved | Kind::NewCanceled => {
            mg_ui::hak_view::new_hak(h.state_mut());
            h.state().haks.last().map(|d| d.id)
        }
    }
    .expect("the hak opened");
    let tab = Tab::Hak(id);
    settle_noting(&mut h, &mut note);
    // Add Files…, through the editor's button.
    assert!(click_in_tab(&mut h, &tab, "Add Files…"), "the editor's Add Files… button");
    h.step();
    settle_noting(&mut h, &mut note);
    if cell.kind == Kind::Replaced {
        let replace = h.query_all_by_label("Replace 1").last();
        assert!(replace.is_some(), "the Replace question: {:?}", on_screen(&h));
        replace.unwrap().click();
        h.step();
        settle_noting(&mut h, &mut note);
    }
    let dirty = |h: &Harness<'_, Moonglow>| {
        h.state().haks.iter().find(|d| d.id == id).map(|d| d.hak.is_dirty())
    };
    assert_eq!(dirty(&h), Some(true), "the file added: {:?}", h.state().log.entries);
    match cell.variant {
        Variant::EditorSaveFirst => {
            assert!(click_in_tab(&mut h, &tab, "Save"), "the editor's Save");
            h.step();
            settle_noting(&mut h, &mut note);
        }
        Variant::FileDeleted => std::fs::remove_file(&added).unwrap(),
        Variant::Unwritable if cell.kind != Kind::NewSaved => {
            let d = h.state_mut().haks.iter_mut().find(|d| d.id == id).unwrap();
            d.hak.path = Some(dir.join("a_file").join("mine.hak"));
        }
        Variant::ModuleChanged => {
            let key = ResKey::parse("mystore", ResType::TWODA).unwrap();
            h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new(
                "A table",
                vec![mg_edit::Edit::SetResource { key, data: Some(SMALL_TABLE.to_vec()) }],
            )));
            h.step();
            settle_noting(&mut h, &mut note);
            let modified = h.state().ws.as_ref().is_some_and(|ws| ws.is_modified());
            assert!(modified, "the module changed: {:?}", h.state().log.entries);
        }
        _ => {}
    }
    if cell.variant == Variant::WhileSaving {
        // The editor's Save as a job, the route pushed before it ends.
        assert!(click_in_tab(&mut h, &tab, "Save"), "the editor's Save");
        h.step();
        let busy = h.state().busy();
        let action = match cell.route {
            Route::WindowX => mg_ui::Action::CloseWindow,
            _ => mg_ui::Action::Quit,
        };
        h.state_mut().actions.push(action);
        h.step();
        let refused = h.state().log.entries.iter().any(|(_, m)| m.contains("is still running"));
        obs.during_job = Some(format!(
            "busy {busy}, refused {refused}, quit {}, confirm {:?}",
            h.state().quit_requested,
            h.state().confirm_discard,
        ));
    } else {
        do_route(&mut h, cell.route, &tab, &mut note);
    }
    answer_all(&mut h, &cell.answers, &mut obs, &mut note);
    obs.goal = goal(&h, cell.route, &tab);
    // (Refused under the job: the route again, once it is done.)
    let refused = obs.during_job.as_deref().is_some_and(|d| d.contains("refused true"));
    if cell.variant == Variant::WhileSaving && refused && !obs.goal {
        let before = obs.answered.len();
        do_route(&mut h, cell.route, &tab, &mut note);
        answer_all(&mut h, &cell.answers, &mut obs, &mut note);
        obs.goal = goal(&h, cell.route, &tab);
        obs.retry = Some(format!("again: {} more questions", obs.answered.len() - before));
    }
    let s = h.state();
    obs.on_screen = on_screen(&h);
    obs.quit_requested = s.quit_requested;
    obs.tab_open = s.dock.find_tab(&tab).is_some();
    obs.haks = s.haks.len();
    obs.orphans = orphans(s);
    obs.dirty = dirty(&h);
    obs.confirm_discard = s.confirm_discard.as_ref().map(|a| format!("{a:?}"));
    obs.side_then = s.side_then.as_ref().map(|a| format!("{a:?}"));
    obs.busy = s.busy();
    obs.module_open = s.ws.is_some();
    obs.log_tail = s.log.entries.iter().rev().take(4).rev().map(|(_, m)| m.clone()).collect();
    let file = match cell.kind {
        Kind::Opened | Kind::Replaced => hak_path.clone(),
        _ => new_path.clone(),
    };
    let key = ResKey::parse(key_name, ResType::TWODA).unwrap();
    obs.on_disk = match mg_module::hak_edit::Hak::open(&file) {
        Err(_) if !file.exists() => "no file".into(),
        Err(e) => format!("unreadable: {e}"),
        Ok(hak) => match hak.data(key) {
            Ok(d) if d == NEW_TABLE.as_bytes() => "has the added file".into(),
            Ok(_) => "old copy only".into(),
            Err(_) => "without it".into(),
        },
    };
    obs.route_note = note.trim().to_string();
    obs
}

/// What the window does in `cell` once fixed.
fn want(cell: &Cell) -> Want {
    use Answer::*;
    let failing = cell.failing();
    // (A Save of the hak that is done: not where its file or its target
    // fails, nor where the save dialog is canceled.)
    let saves = !failing && cell.kind != Kind::NewCanceled;
    let answer = |n: usize| cell.answers[n.min(cell.answers.len() - 1)];
    let mut w = Want {
        asked: Vec::new(),
        goal: false,
        asking: false,
        on_disk: "",
        reason_seen: false,
        retried: false,
    };
    let mut saved = false;
    // The hak's question, from the `n`th answer on: Save again and again
    // where it can't be done, at most [`MOST`] questions in all.
    let hak_question = |w: &mut Want, saved: &mut bool| loop {
        if w.asked.len() >= MOST {
            w.asking = true;
            return;
        }
        let a = answer(w.asked.len());
        w.asked.push(Q::Hak);
        match a {
            Save if saves => {
                *saved = true;
                w.goal = true;
                return;
            }
            Save => w.reason_seen |= failing,
            DontSave => {
                w.goal = true;
                return;
            }
            Cancel => return,
        }
    };
    let clean = cell.variant == Variant::EditorSaveFirst && cell.kind != Kind::NewCanceled;
    if clean || cell.variant == Variant::WhileSaving {
        // Saved before the route: nothing of the hak's is asked.
        saved = true;
        match cell.route {
            Route::Escape => {}
            Route::WindowX => {
                w.asked.push(Q::CloseMoonglow);
                w.goal = answer(0) != Cancel;
            }
            _ => w.goal = true,
        }
        w.retried = cell.variant == Variant::WhileSaving;
    } else {
        match cell.route {
            Route::Escape => {}
            // OK saves without a question: one that fails asks it, saying
            // why; a canceled save dialog leaves the editor as it was.
            Route::FootOk if saves => {
                saved = true;
                w.goal = true;
            }
            Route::FootOk if failing => {
                w.reason_seen = true;
                hak_question(&mut w, &mut saved);
            }
            Route::FootOk => {}
            route if !route.closes_tab() && cell.variant == Variant::ModuleChanged => {
                w.asked.push(Q::Module);
                match answer(0) {
                    Cancel => {}
                    // (The module's Save saves a hak that has its file;
                    // a new one asks where, by its own question.)
                    Save if cell.kind == Kind::Opened => {
                        saved = true;
                        w.goal = true;
                    }
                    Save | DontSave => hak_question(&mut w, &mut saved),
                }
            }
            _ => hak_question(&mut w, &mut saved),
        }
    }
    w.on_disk = match (saved, cell.kind) {
        (true, _) => "has the added file",
        (false, Kind::Opened) => "without it",
        (false, Kind::Replaced) => "old copy only",
        (false, _) => "no file",
    };
    w
}

/// How `o` deviates from what the window does in `cell`.
fn deviations(cell: &Cell, o: &Obs) -> Vec<String> {
    let w = want(cell);
    let mut out = Vec::new();
    if o.asked != w.asked {
        out.push(format!("asked {:?}, not {:?}", o.asked, w.asked));
    }
    if o.goal != w.goal {
        out.push(format!("done {}, not {}", o.goal, w.goal));
    }
    let asking = !questions_in(&o.on_screen).is_empty();
    if asking != w.asking {
        out.push(format!("still asking {asking}, not {}", w.asking));
    }
    if o.on_disk != w.on_disk {
        out.push(format!("the hak's file {}, not {}", o.on_disk, w.on_disk));
    }
    if w.reason_seen && !o.reason_seen {
        out.push("the save failed and why was said only in the log".into());
    }
    if !w.reason_seen && o.reason_seen {
        out.push("a reason shown where no save failed".into());
    }
    // (Under the job the route is refused; on a busy machine the job can
    // end within the frame the route was pushed in, before it ran: then it
    // is done at once, as with nothing under way.)
    let refused = o.during_job.as_deref().is_some_and(|d| d.contains("refused true"));
    if (w.retried && refused) != o.retry.is_some() {
        out.push(format!("the route again {:?}", o.retry));
    }
    if let Some(during) = &o.during_job
        && !(refused && during.contains("quit false"))
        && !(!refused && o.goal)
    {
        out.push(format!("under the job: {during}"));
    }
    if o.orphans > 0 {
        out.push(format!("{} side documents without a tab", o.orphans));
    }
    if o.confirm_discard.is_some() {
        out.push(format!("the module's question left up: {:?}", o.confirm_discard));
    }
    if !asking && o.side_then.is_some() {
        out.push(format!("something still waits for an answer: {:?}", o.side_then));
    }
    if !o.route_note.is_empty() && o.route_note.contains('[') {
        out.push(format!("note {}", o.route_note));
    }
    out
}

fn questions_in(on_screen: &[String]) -> Vec<&String> {
    on_screen.iter().filter(|q| q.starts_with("Save changes") || *q == "Close Moonglow?").collect()
}

fn cells() -> Vec<Cell> {
    use Answer::*;
    let routes = [
        Route::TabX,
        Route::DockWindowX,
        Route::CloseTabs,
        Route::CtrlW,
        Route::FootOk,
        Route::FootCancel,
        Route::Escape,
        Route::WindowX,
        Route::FileExit,
        Route::FileClose,
    ];
    // (A failed save asks the same question whichever tab route asked
    // it: the ×, Close Tab and Cancel stand for the others.)
    let failing_routes = [
        Route::TabX,
        Route::CloseTabs,
        Route::FootOk,
        Route::FootCancel,
        Route::WindowX,
        Route::FileExit,
        Route::FileClose,
    ];
    let settings = [Setting::NoModule, Setting::Module];
    let kinds = [Kind::Opened, Kind::Replaced, Kind::NewSaved, Kind::NewCanceled];
    let mut out = Vec::new();
    let mut push = |variant, setting, kind, route, answers: Vec<Answer>, background| {
        if route == Route::FileClose && setting == Setting::NoModule {
            return;
        }
        out.push(Cell { variant, setting, kind, route, answers, background });
    };
    for setting in settings {
        for kind in kinds {
            // (A file replaced asks as one added: once, with no module.)
            if kind == Kind::Replaced && setting == Setting::Module {
                continue;
            }
            for route in routes {
                let mut answers = vec![vec![Save], vec![DontSave], vec![Cancel]];
                // (The window's × and Ctrl+W ask as the tab's × does.)
                if matches!(route, Route::DockWindowX | Route::CtrlW) {
                    answers = vec![vec![Save]];
                }
                if !route.closes_tab() {
                    answers.push(vec![Save, DontSave]);
                    answers.push(vec![DontSave, Save]);
                }
                for a in answers {
                    push(Variant::Plain, setting, kind, route, a, true);
                }
                push(Variant::EditorSaveFirst, setting, kind, route, vec![Save], true);
            }
        }
        for kind in [Kind::Opened, Kind::Replaced, Kind::NewSaved] {
            for route in failing_routes {
                for variant in [Variant::FileDeleted, Variant::Unwritable] {
                    if variant == Variant::Unwritable && kind == Kind::Replaced {
                        continue;
                    }
                    for a in [Save, DontSave, Cancel] {
                        push(variant, setting, kind, route, vec![a], true);
                    }
                }
            }
        }
        for kind in [Kind::Opened, Kind::NewSaved] {
            for route in [Route::WindowX, Route::FileExit] {
                for a in [Save, Cancel] {
                    push(Variant::WhileSaving, setting, kind, route, vec![a], true);
                }
            }
        }
    }
    for kind in [Kind::Opened, Kind::NewSaved, Kind::NewCanceled] {
        for route in [Route::TabX, Route::WindowX, Route::FileExit, Route::FileClose] {
            for a in [
                vec![Save],
                vec![DontSave],
                vec![Cancel],
                vec![Save, DontSave],
                vec![DontSave, Save],
            ] {
                push(Variant::ModuleChanged, Setting::Module, kind, route, a, true);
            }
        }
    }
    // Jobs run at once, a sample.
    for route in [Route::TabX, Route::FootOk, Route::WindowX, Route::FileExit] {
        for a in [vec![Save], vec![DontSave]] {
            push(Variant::Plain, Setting::NoModule, Kind::Opened, route, a.clone(), false);
            push(Variant::FileDeleted, Setting::NoModule, Kind::Opened, route, a, false);
        }
    }
    push(Variant::Plain, Setting::Module, Kind::NewSaved, Route::FileClose, vec![Save], false);
    push(
        Variant::ModuleChanged,
        Setting::Module,
        Kind::NewSaved,
        Route::WindowX,
        vec![Save],
        false,
    );
    out
}

fn row(name: &str, o: &Obs, off: &[String]) -> String {
    let answered: Vec<String> = o.answered.iter().map(|a| format!("{a:?}")).collect();
    format!(
        "| {name} | {} | done {} quit {} tab {} haks {} orphans {} dirty {:?} module {} busy {} | asked {:?} answered [{}] | on screen {:?} | {} | {} | log {:?} | clean before {}; {} |",
        if off.is_empty() { "ok".to_string() } else { format!("DEVIATES: {}", off.join("; ")) },
        o.goal,
        o.quit_requested,
        o.tab_open,
        o.haks,
        o.orphans,
        o.dirty,
        o.module_open,
        o.busy,
        o.asked,
        answered.join(", "),
        o.on_screen,
        o.on_disk,
        o.retry.as_deref().or(o.during_job.as_deref()).unwrap_or("-"),
        o.log_tail,
        o.clean_before,
        o.route_note,
    )
}

#[test]
fn every_way_of_closing_a_hak_with_a_file_added() {
    let only = std::env::var("HAK_CLOSING_ONLY").ok().filter(|s| !s.is_empty());
    // (The cells' scratch beside the matrix, which is kept.)
    let root = mg_testkit::scratch_dir("hak-closing-cells");
    let mut rows = Vec::new();
    let mut off_rows = Vec::new();
    let mut panics = Vec::new();
    let cells: Vec<Cell> = cells()
        .into_iter()
        .filter(|c| only.as_deref().is_none_or(|o| c.name().contains(o)))
        .collect();
    let started = std::time::Instant::now();
    for (n, cell) in cells.iter().enumerate() {
        let name = cell.name();
        let dir = root.join(format!("cell-{n:04}"));
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| run_cell(cell, &dir)));
        match r {
            Ok(o) => {
                let off = deviations(cell, &o);
                let line = row(&name, &o, &off);
                if !off.is_empty() {
                    off_rows.push(line.clone());
                }
                rows.push(line);
            }
            Err(e) => {
                let line = format!("| {name} | PANIC: {} |", panic_text(&e));
                panics.push(line.clone());
                rows.push(line);
            }
        }
        let _ = std::fs::remove_dir_all(&dir);
    }
    let _ = std::fs::remove_dir_all(&root);
    let header = "| cell | verdict | end state | questions | on screen | hak file | again / under the job | log | notes |\n|---|---|---|---|---|---|---|---|---|";
    let mut text = format!(
        "# Hak closing matrix\n\n{} cells in {:.0?}: {} deviate, {} panicked.\n\n## Deviating\n\n{header}\n{}\n\n## Panicked\n\n{}\n\n## All\n\n{header}\n{}\n",
        cells.len(),
        started.elapsed(),
        off_rows.len(),
        panics.len(),
        off_rows.join("\n"),
        panics.join("\n"),
        rows.join("\n")
    );
    text = text.replace("\n\n\n", "\n\n");
    let out = mg_testkit::output_dir("hak-closing").join("matrix.md");
    std::fs::write(&out, &text).unwrap();
    eprintln!("{} cells in {:.0?}, written to {}", cells.len(), started.elapsed(), out.display());
    assert!(
        off_rows.is_empty() && panics.is_empty(),
        "{} deviating and {} panicked cells of {} (see {}):\n{}\n{}",
        off_rows.len(),
        panics.len(),
        cells.len(),
        out.display(),
        off_rows.join("\n"),
        panics.join("\n")
    );
}

// The cases the matrix leaves out.

/// A hak opened from `path` in `h`, `added` added to it: its tab.
fn changed_hak(h: &mut Harness<'_, Moonglow>, path: &Path, added: &Path) -> Tab {
    let id = mg_ui::hak_view::open_at(h.state_mut(), path).unwrap();
    settle(h);
    let d = h.state_mut().haks.iter_mut().find(|d| d.id == id).unwrap();
    d.hak.add_files(&[added.to_path_buf()]);
    assert!(d.hak.is_dirty());
    settle(h);
    Tab::Hak(id)
}

fn module_change(h: &mut Harness<'_, Moonglow>) {
    let key = ResKey::parse("mystore", ResType::TWODA).unwrap();
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new(
        "A table",
        vec![mg_edit::Edit::SetResource { key, data: Some(SMALL_TABLE.to_vec()) }],
    )));
    settle(h);
    assert!(h.state().ws.as_ref().is_some_and(|ws| ws.is_modified()));
}

/// Clicks the last button `label` (the question's, over the editors') and
/// settles.
fn answer(h: &mut Harness<'_, Moonglow>, label: &str) {
    h.get_all_by_label(label).last().unwrap().click();
    h.step();
    settle(h);
}

fn has_added(path: &Path) -> bool {
    let key = ResKey::parse("mg_new", ResType::TWODA).unwrap();
    mg_module::hak_edit::Hak::open(path).is_ok_and(|h| h.data(key).is_ok())
}

/// Closing the window with a changed module and two changed haks: the
/// module's question, then each hak's by its name, in turn, then the
/// window closes. Cancel at any of them closes nothing and keeps every
/// change; the module's Save saves the haks that have their files with it.
#[test]
fn quitting_asks_of_each_changed_document_in_turn() {
    for (way, answers) in [
        ("don't save the module, save both haks", ["Don't Save", "Save", "Save"]),
        ("cancel at the second hak", ["Don't Save", "Save", "Cancel"]),
        ("cancel at the first hak", ["Don't Save", "Cancel", ""]),
        ("save the module", ["Save", "", ""]),
    ] {
        let dir = mg_testkit::scratch_dir("ui-hak-closing-several");
        let src = dir.join("src");
        std::fs::create_dir_all(&src).unwrap();
        let added = src.join(ADDED);
        std::fs::write(&added, NEW_TABLE).unwrap();
        let (first, second) = (dir.join("first.hak"), dir.join("second.hak"));
        author_hak(&first, &src);
        author_hak(&second, &src);
        let mut app = Moonglow::new(None, Box::new(NoDialogs::default()));
        app.settings.no_last_area = true;
        app.open_module(&module(&dir));
        let mut h = harness(app, 1400.0, 900.0);
        settle(&mut h);
        let tabs = [changed_hak(&mut h, &first, &added), changed_hak(&mut h, &second, &added)];
        module_change(&mut h);
        h.state_mut().actions.push(mg_ui::Action::CloseWindow);
        settle(&mut h);
        let mut asked = Vec::new();
        for label in answers.iter().filter(|l| !l.is_empty()) {
            asked.extend(questions(&h));
            answer(&mut h, label);
        }
        asked.extend(questions(&h));
        let s = h.state();
        let open = |t: &Tab| s.dock.find_tab(t).is_some();
        match way {
            "don't save the module, save both haks" => {
                let want = [
                    "Save changes to sample.mod?",
                    "Save changes to first.hak?",
                    "Save changes to second.hak?",
                ];
                assert_eq!(asked, want, "{way}");
                assert!(s.quit_requested, "{way}: then the window closes");
                assert!(has_added(&first) && has_added(&second), "{way}");
                assert!(s.ws.as_ref().unwrap().is_modified(), "{way}: the module not saved");
            }
            "cancel at the second hak" | "cancel at the first hak" => {
                assert!(!s.quit_requested && s.side_then.is_none(), "{way}: nothing goes on");
                assert!(s.confirm_discard.is_none() && questions(&h).is_empty(), "{way}");
                let module = s.ws.as_ref().unwrap();
                assert!(module.is_modified(), "{way}: the module's change kept");
                assert!(open(&tabs[1]) && !has_added(&second), "{way}: the second as it was");
                assert!(s.haks.iter().all(|d| d.hak.is_dirty()), "{way}: changes kept");
                if way.contains("second") {
                    assert!(has_added(&first) && !open(&tabs[0]), "{way}: the first saved");
                } else {
                    assert!(open(&tabs[0]) && !has_added(&first), "{way}: the first as it was");
                    let want = ["Save changes to sample.mod?", "Save changes to first.hak?"];
                    assert_eq!(asked, want, "{way}");
                }
                assert_eq!(orphans(s), 0, "{way}");
            }
            _ => {
                assert_eq!(asked, ["Save changes to sample.mod?"], "{way}");
                assert!(s.quit_requested, "{way}");
                assert!(has_added(&first) && has_added(&second), "{way}: saved with the module");
            }
        }
    }
}

/// A save that fails says why in the question and under the editor, the
/// Exit asked for waits; with the cause put right (the added file put
/// back), Save saves the hak and the window closes. Don't Save closes it
/// too, with the hak as it was on disk.
#[test]
fn a_failed_save_says_why_and_the_exit_waits_for_it() {
    for fix in [true, false] {
        let dir = mg_testkit::scratch_dir("ui-hak-closing-failed");
        let src = dir.join("src");
        std::fs::create_dir_all(&src).unwrap();
        let added = src.join(ADDED);
        std::fs::write(&added, NEW_TABLE).unwrap();
        let path = dir.join("mine.hak");
        author_hak(&path, &src);
        let mut h = harness(Moonglow::new(None, Box::new(NoDialogs::default())), 1400.0, 900.0);
        settle(&mut h);
        let tab = changed_hak(&mut h, &path, &added);
        std::fs::remove_file(&added).unwrap();
        h.state_mut().actions.push(mg_ui::Action::Quit);
        settle(&mut h);
        assert_eq!(questions(&h), ["Save changes to mine.hak?"]);
        answer(&mut h, "Save");
        assert_eq!(questions(&h), ["Save changes to mine.hak?"], "asked again");
        let why: Vec<String> = h
            .query_all_by_label_contains("could not be saved")
            .filter_map(|n| text_of(&n))
            .collect();
        assert!(why.iter().any(|t| t.contains(ADDED)), "why, with the file's name: {why:?}");
        assert_eq!(reason_shown(&h), 2, "in the question and under the editor: {why:?}");
        assert!(!h.state().quit_requested && h.state().side_then.is_some(), "Exit waits");
        assert!(h.state().dock.find_tab(&tab).is_some());
        if fix {
            std::fs::write(&added, NEW_TABLE).unwrap();
            answer(&mut h, "Save");
            assert!(has_added(&path), "{:?}", h.state().log.entries);
        } else {
            answer(&mut h, "Don't Save");
            assert!(!has_added(&path));
        }
        assert!(questions(&h).is_empty() && reason_shown(&h) == 0);
        assert!(h.state().quit_requested, "then Exit");
    }
}

/// The editor's own Save, and its OK, say why a save failed under the
/// editor; OK then asks the tab's question (with Save again, Don't Save,
/// Cancel) rather than doing nothing to be seen.
#[test]
fn the_editor_says_why_its_save_failed() {
    let dir = mg_testkit::scratch_dir("ui-hak-closing-foot");
    let src = dir.join("src");
    std::fs::create_dir_all(&src).unwrap();
    let added = src.join(ADDED);
    std::fs::write(&added, NEW_TABLE).unwrap();
    let path = dir.join("mine.hak");
    author_hak(&path, &src);
    let mut h = harness(Moonglow::new(None, Box::new(NoDialogs::default())), 1400.0, 900.0);
    settle(&mut h);
    let tab = changed_hak(&mut h, &path, &added);
    std::fs::remove_file(&added).unwrap();
    assert!(click_in_tab(&mut h, &tab, "Save"));
    h.step();
    settle(&mut h);
    assert_eq!(reason_shown(&h), 1, "under the editor");
    assert!(questions(&h).is_empty(), "its own Save asks nothing");
    assert!(click_in_tab(&mut h, &tab, "OK"));
    h.step();
    settle(&mut h);
    assert_eq!(questions(&h), ["Save changes to mine.hak?"], "OK asks");
    assert_eq!(reason_shown(&h), 2);
    answer(&mut h, "Cancel");
    // (Cancel: the editor stays open and still says why, until the hak is
    // saved or let go.)
    assert!(questions(&h).is_empty(), "Cancel: nothing more asked");
    assert_eq!(reason_shown(&h), 1, "why, under the editor still");
    assert!(h.state().dock.find_tab(&tab).is_some());
    std::fs::write(&added, NEW_TABLE).unwrap();
    assert!(click_in_tab(&mut h, &tab, "OK"));
    h.step();
    settle(&mut h);
    assert!(has_added(&path) && h.state().dock.find_tab(&tab).is_none(), "put right: saved");
    assert_eq!(reason_shown(&h), 0, "saved: forgotten");
}

/// A side editor's document never outlives its tab: File › Close lets the
/// clean ones go with their tabs, a hak's tab kept with the module comes
/// back with its document when the module is opened again, a changed one
/// left by a way that does not ask (another module opened as the second
/// step of New Module) keeps its tab, and is asked of by name when the
/// module is closed.
#[test]
fn side_documents_follow_their_tabs() {
    let dir = mg_testkit::scratch_dir("ui-hak-closing-follow");
    let src = dir.join("src");
    std::fs::create_dir_all(&src).unwrap();
    let added = src.join(ADDED);
    std::fs::write(&added, NEW_TABLE).unwrap();
    let path = dir.join("mine.hak");
    author_hak(&path, &src);
    let module = module(&dir);
    let other_dir = dir.join("other");
    std::fs::create_dir_all(&other_dir).unwrap();
    let other = self::module(&other_dir);
    let mut app = Moonglow::new(None, Box::new(NoDialogs::default()));
    app.settings.no_last_area = true;
    app.open_tabs_dir = Some(dir.join("open-tabs"));
    app.open_module(&module);
    let mut h = harness(app, 1400.0, 900.0);
    settle(&mut h);
    // A clean hak, File › Close: let go with its tab.
    mg_ui::hak_view::open_at(h.state_mut(), &path).unwrap();
    settle(&mut h);
    h.state_mut().actions.push(mg_ui::Action::Close);
    settle(&mut h);
    assert!(h.state().ws.is_none() && h.state().haks.is_empty(), "let go");
    // Opened again: the hak's tab kept with the module, and its document.
    h.state_mut().actions.push(mg_ui::Action::OpenModule(module.clone()));
    settle(&mut h);
    h.run_steps(4);
    assert_eq!(h.state().haks.len(), 1, "the kept hak opened again");
    assert_eq!(orphans(h.state()), 0, "with its tab");
    // Changed, then another module opened by a way that does not ask:
    // kept, with its tab.
    let id = h.state().haks[0].id;
    let d = h.state_mut().haks.iter_mut().find(|d| d.id == id).unwrap();
    d.hak.add_files(std::slice::from_ref(&added));
    settle(&mut h);
    h.state_mut().open_module(&other);
    settle(&mut h);
    h.run_steps(4);
    assert!(h.state().haks.iter().any(|d| d.id == id && d.hak.is_dirty()), "kept");
    assert_eq!(orphans(h.state()), 0, "with its tab");
    // A clean one beside it, File › Close: the clean one let go, the
    // changed one asked of first.
    std::fs::copy(&path, dir.join("clean.hak")).unwrap();
    mg_ui::hak_view::open_at(h.state_mut(), &dir.join("clean.hak")).unwrap();
    settle(&mut h);
    h.state_mut().actions.push(mg_ui::Action::Close);
    settle(&mut h);
    assert_eq!(questions(&h), ["Save changes to mine.hak?"]);
    answer(&mut h, "Don't Save");
    assert!(h.state().ws.is_none(), "then closed");
    assert!(h.state().haks.is_empty() && orphans(h.state()) == 0);
}

/// The 2DA and tileset editors on the same path: the window's X and File ›
/// Exit ask of each by its name, then quit; a failed save says why in the
/// question, and Don't Save still quits.
#[test]
fn a_2da_and_a_tileset_ask_by_name_at_quit() {
    for (route, fail) in [
        (mg_ui::Action::CloseWindow, false),
        (mg_ui::Action::Quit, false),
        (mg_ui::Action::CloseWindow, true),
        (mg_ui::Action::Quit, true),
    ] {
        let dir = mg_testkit::scratch_dir("ui-hak-closing-siblings");
        std::fs::write(dir.join("a_file"), "x").unwrap();
        let table = dir.join("loose.2da");
        std::fs::write(&table, SMALL_TABLE).unwrap();
        let set = dir.join("zzz01.set");
        let dialogs = NoDialogs { save: vec![set.clone()], ..Default::default() };
        let mut h = harness(Moonglow::new(None, Box::new(dialogs)), 1400.0, 900.0);
        settle(&mut h);
        // A 2DA of its own file, a row added.
        let id = mg_ui::table_view::open_path(h.state_mut(), &table).unwrap();
        settle(&mut h);
        h.state_mut().tables.iter_mut().find(|d| d.id == id).unwrap().edit.add_rows(1);
        // A tileset, a terrain added.
        mg_ui::tileset_view::new_tileset(h.state_mut());
        settle(&mut h);
        let made = std::fs::read(&set).unwrap();
        h.get_by_label("Terrains and Crossers").click();
        h.run_steps(2);
        h.state_mut().tilesets[0].new_type = "Grass".into();
        h.run_steps(2);
        answer(&mut h, "Add Terrain");
        assert!(h.state().has_unsaved_work());
        if fail {
            let under = dir.join("a_file");
            let d = h.state_mut().tables.iter_mut().find(|d| d.id == id).unwrap();
            d.home = mg_ui::table_view::Home::File(under.join("loose.2da"));
            h.state_mut().tilesets[0].path = under.join("zzz01.set");
        }
        h.state_mut().actions.push(route.clone());
        settle(&mut h);
        let way = format!("{route:?}, failing {fail}");
        assert_eq!(questions(&h), ["Save changes to loose.2da?"], "{way}");
        answer(&mut h, "Save");
        if fail {
            assert_eq!(questions(&h), ["Save changes to loose.2da?"], "{way}: again");
            assert!(reason_shown(&h) >= 1, "{way}: why");
            answer(&mut h, "Don't Save");
        }
        assert_eq!(questions(&h), ["Save changes to zzz01.set?"], "{way}");
        assert!(!h.state().quit_requested, "{way}: not yet");
        answer(&mut h, "Save");
        if fail {
            assert_eq!(questions(&h), ["Save changes to zzz01.set?"], "{way}: again");
            assert!(reason_shown(&h) >= 1, "{way}: why");
            answer(&mut h, "Don't Save");
            assert_eq!(std::fs::read(&set).unwrap(), made, "{way}: as it was");
            assert_eq!(std::fs::read(&table).unwrap(), SMALL_TABLE, "{way}: as it was");
        } else {
            assert_ne!(std::fs::read(&set).unwrap(), made, "{way}: saved");
            assert_ne!(std::fs::read(&table).unwrap(), SMALL_TABLE, "{way}: saved");
        }
        assert!(questions(&h).is_empty(), "{way}");
        assert!(h.state().quit_requested, "{way}: then the window closes");
    }
}

/// A reason goes once the hak is clean another way: a failed save,
/// Cancel, then Undo (the remedy for an added file gone from disk): the
/// foot is clear, a new change after it shows no old reason, and the tab
/// closes unasked once it is undone again.
#[test]
fn a_reason_goes_once_the_hak_is_clean() {
    let dir = mg_testkit::scratch_dir("ui-hak-closing-undo");
    let src = dir.join("src");
    std::fs::create_dir_all(&src).unwrap();
    let added = src.join(ADDED);
    std::fs::write(&added, NEW_TABLE).unwrap();
    let path = dir.join("mine.hak");
    author_hak(&path, &src);
    let mut h = harness(Moonglow::new(None, Box::new(NoDialogs::default())), 1400.0, 900.0);
    settle(&mut h);
    let tab = changed_hak(&mut h, &path, &added);
    std::fs::remove_file(&added).unwrap();
    assert!(click_in_tab(&mut h, &tab, "OK"));
    h.step();
    settle(&mut h);
    assert_eq!(questions(&h), ["Save changes to mine.hak?"]);
    answer(&mut h, "Cancel");
    assert_eq!(reason_shown(&h), 1, "under the editor");
    let dirty = |h: &Harness<'_, Moonglow>| h.state().haks[0].hak.is_dirty();
    assert!(click_in_tab(&mut h, &tab, "Undo"));
    h.step();
    settle(&mut h);
    assert!(!dirty(&h), "undone: clean");
    assert_eq!(reason_shown(&h), 0, "clean: nothing to say");
    // A new change: no old reason.
    let other = src.join("mg_other.2da");
    std::fs::write(&other, NEW_TABLE).unwrap();
    h.state_mut().haks[0].hak.add_files(std::slice::from_ref(&other));
    settle(&mut h);
    assert!(dirty(&h));
    assert_eq!(reason_shown(&h), 0, "the reason was let go for good");
    assert!(click_in_tab(&mut h, &tab, "Undo"));
    h.step();
    settle(&mut h);
    assert!(!dirty(&h));
    h.state_mut().actions.push(mg_ui::Action::CloseTabs(vec![tab.clone()]));
    settle(&mut h);
    assert!(questions(&h).is_empty(), "nothing to ask");
    assert!(h.state().dock.find_tab(&tab).is_none() && h.state().haks.is_empty(), "closed");
}

/// The talk table shares the side editors' question and foot: its save
/// failing (a file that can't be written) says why in both, and Don't
/// Save closes it.
#[test]
fn the_talk_table_says_why_its_save_failed() {
    use mg_module::talk::{Source, Table};
    let dir = mg_testkit::scratch_dir("ui-hak-closing-talk");
    std::fs::write(dir.join("a_file"), "x").unwrap();
    let path = sample_module(&dir);
    let mut app = Moonglow::new(None, Box::new(NoDialogs::default()));
    app.settings.no_last_area = true;
    app.open_module(&path);
    let mut h = harness(app, 1400.0, 900.0);
    settle(&mut h);
    let file = dir.join("words.tlk");
    let mut table = Table::create("words", &dir, Language::ENGLISH, false);
    table.source = Source::File(file.clone());
    table.save().unwrap();
    let written = std::fs::read(&file).unwrap();
    h.state_mut().talk = Some(table);
    h.state_mut().talk_view.outside = true;
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::TalkTable));
    settle(&mut h);
    answer(&mut h, "Add Line");
    assert!(h.state().talk.as_ref().unwrap().is_dirty());
    h.state_mut().talk.as_mut().unwrap().source = Source::File(dir.join("a_file").join("w.tlk"));
    assert!(click_in_tab(&mut h, &Tab::TalkTable, "OK"));
    h.step();
    settle(&mut h);
    let asked = questions(&h);
    assert_eq!(asked, ["Save changes to the talk table words?"], "OK asks");
    assert_eq!(reason_shown(&h), 2, "in the question and under the editor");
    answer(&mut h, "Save");
    assert_eq!(questions(&h), asked, "asked again");
    assert_eq!(reason_shown(&h), 2, "saying why again");
    answer(&mut h, "Don't Save");
    assert!(questions(&h).is_empty() && reason_shown(&h) == 0);
    assert!(h.state().dock.find_tab(&Tab::TalkTable).is_none(), "closed");
    assert_eq!(std::fs::read(&file).unwrap(), written, "the file as it was");
}

/// The module's own question: its Save failing (a place that can't be
/// written) asks it again, saying why; Don't Save then quits; Cancel
/// quits nothing and lets the reason go.
#[test]
fn the_module_s_question_says_why_its_save_failed() {
    for last in ["Don't Save", "Cancel"] {
        let dir = mg_testkit::scratch_dir("ui-hak-closing-module");
        std::fs::write(dir.join("a_file"), "x").unwrap();
        let path = module(&dir);
        let mut app = Moonglow::new(None, Box::new(NoDialogs::default()));
        app.settings.no_last_area = true;
        app.open_module(&path);
        let mut h = harness(app, 1400.0, 900.0);
        settle(&mut h);
        module_change(&mut h);
        let under = ModuleLocation::Archive(dir.join("a_file").join("sample.mod"));
        h.state_mut().ws.as_mut().unwrap().module.location = Some(under);
        h.state_mut().actions.push(mg_ui::Action::Quit);
        settle(&mut h);
        assert_eq!(questions(&h), ["Save changes to sample.mod?"]);
        assert_eq!(reason_shown(&h), 0, "nothing failed yet");
        answer(&mut h, "Save");
        assert_eq!(questions(&h), ["Save changes to sample.mod?"], "{last}: asked again");
        assert_eq!(reason_shown(&h), 1, "{last}: saying why");
        assert!(!h.state().quit_requested);
        answer(&mut h, last);
        assert!(questions(&h).is_empty() && reason_shown(&h) == 0, "{last}");
        assert_eq!(h.state().quit_requested, last == "Don't Save", "{last}");
        if last == "Cancel" {
            // Asked anew: no old reason.
            h.state_mut().actions.push(mg_ui::Action::Quit);
            settle(&mut h);
            assert_eq!(questions(&h), ["Save changes to sample.mod?"]);
            assert_eq!(reason_shown(&h), 0, "asked anew");
        }
    }
}

/// A hak opened, then a module, in one frame (the dock made anew between
/// them, the hak's tab still to come): the hak's tab has its document,
/// whether or not the module has kept tabs of its own to put back.
#[test]
fn a_hak_opened_with_a_module_keeps_its_document() {
    for kept in [false, true] {
        let dir = mg_testkit::scratch_dir("ui-hak-closing-batch");
        let src = dir.join("src");
        std::fs::create_dir_all(&src).unwrap();
        let path = dir.join("mine.hak");
        author_hak(&path, &src);
        let module = module(&dir);
        let mut app = Moonglow::new(None, Box::new(NoDialogs::default()));
        app.settings.no_last_area = true;
        app.open_tabs_dir = Some(dir.join("open-tabs"));
        let mut h = harness(app, 1400.0, 900.0);
        settle(&mut h);
        if kept {
            // (A record of the module's tabs: Module Properties.)
            h.state_mut().actions.push(mg_ui::Action::OpenModule(module.clone()));
            settle(&mut h);
            h.state_mut().actions.push(mg_ui::Action::Close);
            settle(&mut h);
        }
        let id = mg_ui::hak_view::open_at(h.state_mut(), &path).unwrap();
        h.state_mut().actions.push(mg_ui::Action::OpenModule(module.clone()));
        settle(&mut h);
        h.run_steps(4);
        let way = if kept { "with kept tabs" } else { "without" };
        assert!(h.state().ws.is_some(), "{way}");
        assert!(h.state().dock.find_tab(&Tab::Hak(id)).is_some(), "{way}: its tab");
        assert!(h.state().haks.iter().any(|d| d.id == id), "{way}: its document");
        assert!(h.query_by_label("This hak was closed.").is_none(), "{way}");
        assert_eq!(orphans(h.state()), 0, "{way}");
    }
}
