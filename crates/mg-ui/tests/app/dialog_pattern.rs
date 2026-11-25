//! Every dialog by the dialog pattern (the redesign's M4, task U27, the
//! Dialogs board): each dialog of the inventory (`target/u27/inventory.txt`)
//! opened as a builder opens it, and held at 1280 by 720, at the least
//! window and at the zooms 1.25 and 1.5, in each of the states its opener
//! lists (a wizard's later pages), to the pattern: its footer at its foot,
//! its buttons whole and in the order [`table::ORDERS`] gives (the main
//! action first in the accent fill unless the table names why not, what
//! leaves it last); every widget the accessibility tree names inside the
//! window, the body's inside its margin, none over another (a scrolled
//! body's rows out of sight excepted: [`layout_problems`]); the dialog
//! inside the window. Escape presses what leaves it and it closes; Enter
//! takes its main action, seen by its effect (it closed, or what its
//! opener says), with nothing holding the keys and with a one-line field
//! holding them, and nothing while the main action is off; for the
//! dialogs [`table::ENTER_WITHHELD`] lists, Enter does nothing unless the
//! main action holds the focus. The drawn fill of each main action is the
//! accent's ([`the_main_actions_are_drawn_in_the_accent`]).
//!
//! The list is whole: [`every_dialog_is_listed`] reads the sources for
//! every call of a dialog's constructor (`modal`, `dialog`,
//! `modal_window`, however imported, `egui::Window::new` and
//! `egui::Modal::new` but where allowed by name) and of the shared makers
//! (their callers, [`SHARED_MAKERS`]), and fails on one no table names.
//! The openers are in `dialog_pattern/`, a file a group of sources.

use crate::common::*;

mod openers_core;
mod openers_editors;
mod openers_module;
pub(crate) mod openers_shared;
mod openers_wizards;
pub(crate) mod openers_world;
pub(crate) mod table;

/// A state of a dialog beyond the one it opens in (a wizard's later page):
/// its name and how it is reached from where the opener leaves it.
pub(crate) type Step = (&'static str, fn(&mut Harness<'static, Moonglow>));

/// What Enter's main action does where it does not close the dialog: said,
/// and seen in the window.
pub(crate) type Effect = (&'static str, fn(&Harness<'static, Moonglow>) -> bool);

/// A dialog of the inventory: where it is made, how a test opens it, and
/// what Enter is tried with.
pub(crate) struct Opener {
    /// Where it is made: the source file under `crates/mg-ui/src`, the
    /// function, and which of the dialogs made in that function it is
    /// (from 1, in the order they stand), as [`made_in_sources`] names
    /// them: `"wizards.rs:ui#2"`.
    pub(crate) key: &'static str,
    /// Its title (its window's accessible name).
    pub(crate) title: &'static str,
    /// A window to open it in; the harness `None` where the game is not
    /// installed.
    pub(crate) setup: fn() -> Option<Harness<'static, Moonglow>>,
    /// Opens it in that window.
    pub(crate) open: fn(&mut Harness<'static, Moonglow>),
    /// Which of its one-line text fields (from 0, in the order the
    /// accessibility tree lists them) Enter is tried in.
    pub(crate) field: Option<usize>,
    /// Puts its main action off, where a builder can.
    pub(crate) off: Option<fn(&mut Harness<'static, Moonglow>)>,
    /// Drawn with no footer: it has no buttons of its own.
    pub(crate) bare: bool,
    /// Its further states, each held as it opens is.
    pub(crate) states: Vec<Step>,
    /// What Enter does, where it does not close it.
    pub(crate) effect: Option<Effect>,
    /// What Enter does while the main action is off, where it does
    /// something then (as at the tip: Open Script's opens the first listed).
    pub(crate) enter_when_off: Option<Effect>,
    /// Shown while a job runs (its window asks for frames): waited on by
    /// [`rested`], not [`settle`].
    pub(crate) job: bool,
}

impl Opener {
    /// A dialog opened by `open` in the window `setup` makes.
    pub(crate) const fn new(
        key: &'static str,
        title: &'static str,
        setup: fn() -> Option<Harness<'static, Moonglow>>,
        open: fn(&mut Harness<'static, Moonglow>),
    ) -> Opener {
        Opener {
            key,
            title,
            setup,
            open,
            field: None,
            off: None,
            bare: false,
            states: Vec::new(),
            effect: None,
            enter_when_off: None,
            job: false,
        }
    }

    pub(crate) fn field(mut self, nth: usize) -> Opener {
        self.field = Some(nth);
        self
    }

    pub(crate) fn off(mut self, off: fn(&mut Harness<'static, Moonglow>)) -> Opener {
        self.off = Some(off);
        self
    }

    pub(crate) fn bare(mut self) -> Opener {
        self.bare = true;
        self
    }

    pub(crate) fn states(mut self, states: Vec<Step>) -> Opener {
        self.states = states;
        self
    }

    pub(crate) fn enter_does(
        mut self,
        what: &'static str,
        seen: fn(&Harness<'static, Moonglow>) -> bool,
    ) -> Opener {
        self.effect = Some((what, seen));
        self
    }

    pub(crate) fn enter_when_off(
        mut self,
        what: &'static str,
        seen: fn(&Harness<'static, Moonglow>) -> bool,
    ) -> Opener {
        self.enter_when_off = Some((what, seen));
        self
    }

    pub(crate) fn job(mut self) -> Opener {
        self.job = true;
        self
    }
}

/// Every table of openers.
pub(crate) fn all() -> Vec<Opener> {
    let mut all = Vec::new();
    all.extend(openers_core::openers());
    all.extend(openers_world::openers());
    all.extend(openers_module::openers());
    all.extend(openers_editors::openers());
    all.extend(openers_wizards::openers());
    all.extend(openers_shared::openers());
    all
}

/// The constructors of a dialog, as [`made_in_sources`] finds their calls:
/// `widgets::modal`, `widgets::dialog` and `widgets::modal_window`, by a
/// path ending in `widgets::`, or bare where the file imports the name
/// from `widgets` (or is `widgets.rs`).
const CONSTRUCTORS: [&str; 3] = ["modal", "dialog", "modal_window"];

/// Where egui's own `Window::new` and `Modal::new` may be called: the
/// constructors themselves (every other dialog is made through them).
const DIRECT_ALLOWED: [&str; 2] = ["widgets.rs:centered", "widgets.rs:modal_window"];

/// The shared makers of `widgets.rs`: a dialog made in one place for many
/// callers (its opener in the table is the maker's), with how a caller
/// opens it and each caller as `file:function#n`. A new caller fails
/// [`every_dialog_is_listed`] until it is named here.
pub(crate) const SHARED_MAKERS: &[(&str, &str, &[&str])] = &[
    (
        "Files Already in the …",
        "files_already(",
        &[
            "browser.rs:exporting_window#1",
            "dialog_export.rs:ask_window#1",
            "hak_view.rs:adding_window#1",
            "hak_view.rs:extracting_window#1",
            "model_view.rs:export_window#1",
            "nui_view.rs:windows#1",
            "tileset_view.rs:windows#1",
        ],
    ),
    (
        "the name window (NameWindow::show)",
        "NameWindow {",
        &[
            "dialog_view.rs:windows#1",
            "nui_view.rs:windows#1",
            "script_view.rs:windows#1",
            "script_view.rs:windows#2",
        ],
    ),
    (
        "Select Resource (widgets.rs:ui#3)",
        "open_picker(",
        &[
            "blueprint/sound.rs:sound_list#1",
            "blueprint_wizard.rs:ui#1",
            "script_wizard.rs:action_page#1",
            "widgets.rs:resref_field#1",
        ],
    ),
    (
        "String Edit (widgets.rs:ui#1)",
        "loc_edit = Some(",
        &[
            "blueprint/mod.rs:locstring_memo#1",
            "blueprint/mod.rs:locstring_memo_to#1",
            "blueprint/mod.rs:open_loc_edit_on_click#1",
            "dialog_view.rs:text_panel#1",
        ],
    ),
    (
        "Variables (widgets.rs:ui#2)",
        "var_edit = Some(",
        &[
            "area_props.rs:area_variables#1",
            "blueprint/mod.rs:variables#1",
            "widgets.rs:open_variables#1",
        ],
    ),
];

/// The `.rs` files under `crates/mg-ui/src`, as (their name from there,
/// their text up to their tests' module).
fn sources() -> Vec<(String, String)> {
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    let mut dirs = vec![src.clone()];
    while let Some(dir) = dirs.pop() {
        for e in std::fs::read_dir(&dir).unwrap() {
            let p = e.unwrap().path();
            if p.is_dir() {
                dirs.push(p);
            } else if p.extension().is_some_and(|x| x == "rs") {
                files.push(p);
            }
        }
    }
    files.sort();
    files
        .into_iter()
        .map(|path| {
            let name = path.strip_prefix(&src).unwrap().to_string_lossy().replace('\\', "/");
            let text = std::fs::read_to_string(&path).unwrap();
            // (Less its tests' modules: `#[cfg(test)]` then `mod name;`, or
            // `mod name {` to the `}` that closes it at the line's start.)
            let lines: Vec<&str> = text.lines().collect();
            let mut kept = Vec::new();
            let mut i = 0;
            while i < lines.len() {
                let next = lines.get(i + 1).map(|l| l.trim_start()).unwrap_or_default();
                if lines[i].trim() == "#[cfg(test)]" && next.starts_with("mod ") {
                    i += 2;
                    if next.ends_with('{') {
                        while i < lines.len() && lines[i] != "}" {
                            i += 1;
                        }
                        i += 1;
                    }
                    continue;
                }
                kept.push(lines[i]);
                i += 1;
            }
            (name, kept.join("\n"))
        })
        .collect()
}

/// The names `file` imports from `widgets` (`*` for a glob), joined from
/// its `use` statements (one may span lines); a constructor imported under
/// another name fails here, for the scan would not see its calls.
fn imported_from_widgets(name: &str, text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut statement = String::new();
    for line in text.lines() {
        let t = line.trim();
        if statement.is_empty() && !(t.starts_with("use ") || t.starts_with("pub(crate) use ")) {
            continue;
        }
        statement.push_str(t);
        statement.push(' ');
        if !t.ends_with(';') {
            continue;
        }
        let s = std::mem::take(&mut statement);
        let Some(at) = s.find("widgets::") else { continue };
        let tail = &s[at + "widgets::".len()..];
        for word in tail.split(|c: char| !(c.is_alphanumeric() || c == '_' || c == '*')) {
            if !word.is_empty() {
                out.push(word.to_string());
            }
        }
        for c in CONSTRUCTORS {
            assert!(
                !tail.contains(&format!("{c} as ")),
                "{name}: widgets::{c} imported under another name: {s}"
            );
        }
    }
    out
}

/// Whether `t[..i]` ends a call's path that reaches `widgets` or nothing
/// (a bare name): `Some(true)` for `widgets::`, `Some(false)` for bare,
/// `None` for another path, a method or a definition.
fn called_at(t: &str, i: usize) -> Option<bool> {
    let before = &t[..i];
    if before.ends_with("widgets::") {
        return Some(true);
    }
    let last = before.chars().last();
    let bare = last.is_none_or(|c| !c.is_alphanumeric() && c != '_' && c != ':' && c != '.');
    (bare && !before.ends_with("fn ")).then_some(false)
}

/// Walks `text` by line with the function each line is in, counting the
/// matches `count` finds on each; gives each as `file:function#n` (the
/// `n`th in that function, counted through every function of the name in
/// the file: two `fn ui` of two types number on).
fn keyed(name: &str, text: &str, count: impl Fn(&str) -> usize) -> Vec<String> {
    let mut func = String::new();
    let mut made: std::collections::HashMap<String, usize> = Default::default();
    let mut out = Vec::new();
    for line in text.lines() {
        let t = line.trim_start();
        if t.starts_with("//") {
            continue;
        }
        // (A function's start: `fn name(`, `pub(crate) fn name<`.)
        if let Some(at) = t.find("fn ")
            && (at == 0 || t[..at].ends_with(' '))
            && !t.starts_with("let ")
            && t[..at].split_whitespace().all(|w| {
                matches!(w, "pub" | "pub(crate)" | "pub(super)" | "async" | "const" | "unsafe")
            })
        {
            let rest = &t[at + 3..];
            let end = rest.find(|c: char| !c.is_alphanumeric() && c != '_').unwrap_or(0);
            if end > 0 {
                func = rest[..end].to_string();
            }
        }
        for _ in 0..count(t) {
            let n = made.entry(func.clone()).or_default();
            *n += 1;
            out.push(format!("{name}:{func}#{n}"));
        }
    }
    out
}

/// The places in `crates/mg-ui/src` a dialog is made, each as
/// `file:function#n`: the calls of [`CONSTRUCTORS`]; and the functions
/// calling egui's `Window::new` or `Modal::new` (`file:function`).
fn made_in_sources() -> (std::collections::BTreeSet<String>, Vec<String>) {
    let mut out = std::collections::BTreeSet::new();
    let mut direct = Vec::new();
    for (name, text) in sources() {
        let imported = imported_from_widgets(&name, &text);
        let bare_ok = |c: &str| name == "widgets.rs" || imported.iter().any(|w| w == c || w == "*");
        let count = |t: &str| {
            CONSTRUCTORS
                .iter()
                .map(|c| {
                    let call = format!("{c}(");
                    t.match_indices(&call)
                        .filter(|(i, _)| match called_at(t, *i) {
                            Some(true) => true,
                            Some(false) => bare_ok(c),
                            None => false,
                        })
                        .count()
                })
                .sum()
        };
        out.extend(keyed(&name, &text, count));
        let egui_own = |t: &str| {
            ["Window::new(", "Modal::new("]
                .iter()
                .map(|c| {
                    t.match_indices(c)
                        .filter(|(i, _)| {
                            let before = &t[..*i];
                            before.ends_with("egui::")
                                || before.chars().last().is_none_or(|c| !c.is_alphanumeric())
                        })
                        .count()
                })
                .sum()
        };
        for k in keyed(&name, &text, egui_own) {
            direct.push(k.rsplit_once('#').unwrap().0.to_string());
        }
    }
    (out, direct)
}

/// The callers of each shared maker found in the sources, by its pattern.
fn shared_callers(pattern: &str) -> Vec<String> {
    let mut out = Vec::new();
    for (name, text) in sources() {
        let count = |t: &str| {
            t.match_indices(pattern)
                .filter(|(i, _)| {
                    let before = &t[..*i];
                    !before.ends_with("fn ")
                        && !before.ends_with("struct ")
                        && !before.ends_with("impl ")
                        && before.chars().last().is_none_or(|c| !c.is_alphanumeric() && c != '_')
                })
                .count()
        };
        out.extend(keyed(&name, &text, count));
    }
    out.sort();
    out
}

/// The list is whole: every place a dialog is made is in a table, and
/// every table's key is such a place (a dialog removed is taken out); egui's
/// own constructors called only where allowed; each shared maker's callers
/// as [`SHARED_MAKERS`] lists them.
#[test]
fn every_dialog_is_listed() {
    let (made, direct) = made_in_sources();
    // (A key's variant after a space: the kinds of a plugin's question, all
    // drawn by the job's window.)
    let listed: std::collections::BTreeSet<String> =
        all().iter().map(|o| o.key.split(' ').next().unwrap().to_string()).collect();
    let missing: Vec<&String> = made.difference(&listed).collect();
    let gone: Vec<&String> = listed.difference(&made).collect();
    assert!(
        missing.is_empty() && gone.is_empty(),
        "dialogs made but not listed (give each an opener in tests/app/dialog_pattern/): \
         {missing:#?}\nlisted but not made: {gone:#?}"
    );
    assert!(made.len() > 60, "the sources read: {}", made.len());
    let stray: Vec<&String> =
        direct.iter().filter(|d| !DIRECT_ALLOWED.contains(&d.as_str())).collect();
    assert!(
        stray.is_empty(),
        "egui's Window::new or Modal::new called outside the constructors: {stray:?}"
    );
    for (maker, pattern, callers) in SHARED_MAKERS {
        let found = shared_callers(pattern);
        let found: Vec<&str> = found.iter().map(String::as_str).collect();
        assert_eq!(found, *callers, "{maker}: its callers ({pattern})");
    }
}

/// The scan finds a constructor however it is reached: by its path, by a
/// name imported (in a `use` of one line or several, or a glob), and not a
/// method or another function of the name; a renamed import fails.
#[test]
fn the_scan_finds_a_dialog_however_imported() {
    let text = "use crate::widgets::{\n    Foot,\n    modal,\n};\nfn a() {\n    let x = modal(ctx, \"A\");\n    let y = crate::widgets::dialog(ctx, \"B\", &mut o);\n    let z = thing.modal(ctx);\n}\nfn dialog(x: u8) {}\nfn b() {\n    dialog(3);\n}\n";
    let imported = imported_from_widgets("probe.rs", text);
    assert!(imported.iter().any(|w| w == "modal") && !imported.iter().any(|w| w == "dialog"));
    let count = |t: &str| {
        CONSTRUCTORS
            .iter()
            .map(|c| {
                t.match_indices(&format!("{c}("))
                    .filter(|(i, _)| match called_at(t, *i) {
                        Some(true) => true,
                        Some(false) => imported.iter().any(|w| w == c),
                        None => false,
                    })
                    .count()
            })
            .sum()
    };
    assert_eq!(keyed("probe.rs", text, count), ["probe.rs:a#1", "probe.rs:a#2"]);
    let glob = imported_from_widgets("glob.rs", "use crate::widgets::*;\n");
    assert_eq!(glob, ["*"]);
    let renamed = std::panic::catch_unwind(|| {
        imported_from_widgets("r.rs", "use crate::widgets::{modal as m};\n")
    });
    assert!(renamed.is_err(), "a renamed constructor fails the scan");
}

/// What the dialog `title` showed of its footer in the frame last run.
fn seen(h: &Harness<'_, Moonglow>, title: &str) -> mg_ui::widgets::FooterSeen {
    let all = mg_ui::widgets::footers_shown(&h.ctx);
    let mine: Vec<_> = all.iter().filter(|f| f.title == title).collect();
    assert_eq!(mine.len(), 1, "{title}: shown once, as {all:#?}");
    mine[0].clone()
}

/// Whether the dialog `title` was drawn in the frame last run.
pub(crate) fn is_shown(h: &Harness<'_, Moonglow>, title: &str) -> bool {
    mg_ui::widgets::footers_shown(&h.ctx).iter().any(|f| f.title == title)
}

/// Where the dialog `title`'s window was drawn in the frame last run (as
/// the dialog notes it: a control elsewhere may bear its title).
pub(crate) fn window_now(h: &Harness<'_, Moonglow>, title: &str) -> egui::Rect {
    let all = mg_ui::widgets::footers_shown(&h.ctx);
    all.iter().find(|f| f.title == title).map(|f| f.window).unwrap_or(egui::Rect::NOTHING)
}

/// The one-line and other text fields inside the dialog `title`'s window,
/// in the order the accessibility tree lists them: (their rect).
pub(crate) fn fields_in<'h>(
    h: &'h Harness<'_, Moonglow>,
    title: &str,
) -> Vec<egui_kittest::Node<'h>> {
    let window = window_now(h, title);
    h.query_all(egui_kittest::kittest::by().role(egui::accesskit::Role::TextInput))
        .filter(|n| window.contains_rect(n.rect().shrink(1.0)))
        .collect()
}

/// Lets `h` rest: [`settle`]; for a dialog shown while a job runs (whose
/// window asks for a frame every 50 ms and so never rests), frames until
/// its window is still for four in a row, which is past its fade in too
/// (egui's fade takes the style's `animation_time`, under one of the
/// harness's quarter-second steps).
pub(crate) fn rested(h: &mut Harness<'static, Moonglow>, title: &str, job: bool) {
    if !job {
        return settle(h);
    }
    // (Still for four frames in a row, a little time between: a job
    // called off or answered ends on its thread a moment later, and its
    // window goes with it.)
    let started = std::time::Instant::now();
    let (mut last, mut still) = (None, 0);
    while still < 4 {
        h.step();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let now = (window_now(h, title), h.state().busy());
        still = if last == Some(now) { still + 1 } else { 0 };
        assert!(
            started.elapsed() < std::time::Duration::from_secs(20),
            "{title} never came to rest"
        );
        last = Some(now);
    }
}

/// Where the dialog `title`'s window is, once it rests.
fn window(h: &mut Harness<'static, Moonglow>, title: &str, at: &str) -> egui::Rect {
    let rect = |h: &Harness<'_, Moonglow>| window_now(h, title);
    at_rest(h, title, &rect);
    let r = rect(h);
    assert!(r.is_positive(), "{title} open {at}");
    r
}

/// What is wrong with how the dialog `seen` is laid out, in the frame last
/// run: each widget of its layer that the accessibility tree names (its
/// role and name or value) whole inside its window; those in its body (by
/// their middle: under its title row, over its footer) inside the body's
/// margin too; and no two of its named widgets over each other (more than
/// a quarter of the smaller, as [`overlapping`] has it). Excepted by rule:
/// a scrolled body's rows out of sight, that is a widget cut only beyond
/// the ends of a scroll bar the dialog shows (its visible part within that
/// bar's span, the widget in the bar's column or row), on the bar's axis
/// alone; across, only a list's or a well's inset from the body's edge (a
/// body scrolled sideways has its words cut).
fn layout_problems(h: &Harness<'_, Moonglow>, seen: &mg_ui::widgets::FooterSeen) -> Vec<String> {
    use egui::accesskit::Role;
    use egui_kittest::kittest::{AccessKitNode, NodeT};
    use std::collections::HashMap;
    fn names(n: AccessKitNode<'_>, out: &mut HashMap<u64, (String, bool)>, bars: &mut Vec<u64>) {
        let role = n.role();
        if role == Role::ScrollBar {
            bars.push(n.locate().0.0);
        }
        let name = n.label().or_else(|| n.value()).unwrap_or_default();
        let part = matches!(
            role,
            Role::Unknown
                | Role::GenericContainer
                | Role::List
                | Role::TextRun
                | Role::Window
                | Role::ScrollBar
        );
        if !name.trim().is_empty() && !part {
            let leaf = n.children().all(|k| k.role() == Role::TextRun);
            out.insert(n.locate().0.0, (format!("{role:?} {name:?}"), leaf));
        }
        for k in n.children() {
            names(k, out, bars);
        }
    }
    let (mut named, mut bar_ids) = (HashMap::new(), Vec::new());
    names(h.root().accesskit_node(), &mut named, &mut bar_ids);
    let w = seen.window;
    let title_bottom = w.top() + mg_uikit::metrics::TITLE_ROW;
    let foot_top = seen.rect.map_or(w.bottom(), |f| f.top());
    let body = egui::Rect::from_min_max(
        egui::pos2(w.left() + seen.margin, title_bottom),
        egui::pos2(w.right() - seen.margin, foot_top),
    );
    let mut out = Vec::new();
    h.ctx.viewport(|v| {
        let rects = &v.prev_pass.widgets;
        let widgets: Vec<&egui::WidgetRect> = rects.get_layer(seen.layer).collect();
        // (By the accessibility tree: egui keeps a widget's kind only in
        // a debug build with its debug view on.)
        let bars: Vec<egui::Rect> =
            (widgets.iter()).filter(|x| bar_ids.contains(&x.id.value())).map(|x| x.rect).collect();
        let scrolled_out = |r: egui::Rect, visible: egui::Rect, room: egui::Rect| {
            bars.iter().any(|b| match b.height() > b.width() {
                true => {
                    r.left() >= room.left() - 1.0
                        && r.right() <= b.right() + 1.0
                        && (r.top() < b.top() - 0.5 || r.bottom() > b.bottom() + 0.5)
                        && (!visible.is_positive()
                            || (visible.top() >= b.top() - 1.0
                                && visible.bottom() <= b.bottom() + 1.0))
                }
                // (A list's or a well's, inset from the body's edge: the
                // body's own bar across is no exception, its words cut.)
                false if b.left() > body.left() + 2.0 => {
                    r.top() >= room.top() - 1.0
                        && r.bottom() <= b.bottom() + 1.0
                        && (r.left() < b.left() - 0.5 || r.right() > b.right() + 0.5)
                        && (!visible.is_positive()
                            || (visible.left() >= b.left() - 1.0
                                && visible.right() <= b.right() + 1.0))
                }
                false => false,
            })
        };
        for x in &widgets {
            let Some((name, _)) = named.get(&x.id.value()) else { continue };
            let r = x.rect;
            if !r.is_positive() {
                continue;
            }
            let middle = r.center().y;
            let room = if middle < title_bottom || middle > foot_top { w } else { body };
            if room.expand(1.0).contains_rect(r) || scrolled_out(r, x.interact_rect, room) {
                continue;
            }
            out.push(format!("{name} cut: {r:?} outside {room:?}"));
        }
        let shown: Vec<(egui::Rect, &String)> = (widgets.iter())
            .filter(|x| x.interact_rect.area() > 1.0)
            .filter_map(|x| {
                let (n, leaf) = named.get(&x.id.value())?;
                leaf.then_some((x.interact_rect, n))
            })
            .collect();
        for (i, (a, name_a)) in shown.iter().enumerate() {
            for (b, name_b) in &shown[i + 1..] {
                let both = a.intersect(*b);
                if both.is_positive() && both.area() > 0.25 * a.area().min(b.area()) {
                    out.push(format!("{name_a} over {name_b}"));
                }
            }
        }
    });
    out
}

/// The pattern, held at the window's size `at` (module comment). The
/// footer as seen.
fn holds_the_pattern(
    h: &mut Harness<'static, Moonglow>,
    op: &Opener,
    at: &str,
) -> mg_ui::widgets::FooterSeen {
    let title = op.title;
    let window = window(h, title, at);
    let screen = h.ctx.content_rect();
    assert!(screen.contains_rect(window.shrink(0.5)), "{title} {at}: {window:?} in {screen:?}");
    let seen = seen(h, title);
    let problems = layout_problems(h, &seen);
    assert!(problems.is_empty(), "{title} {at}: laid out wrong: {problems:#?}");
    if op.bare {
        assert!(seen.rect.is_none(), "{title}: listed as with no footer, but it has one");
        return seen;
    }
    let foot = seen.rect.unwrap_or_else(|| panic!("{title} {at}: shown with no footer"));
    assert!(!seen.buttons.is_empty(), "{title}: a footer with no buttons");
    assert!(
        window.shrink(0.5).contains_rect(foot.shrink(1.0)),
        "{title} {at}: {foot:?} {window:?}"
    );
    assert!(window.bottom() - foot.bottom() < 3.0, "{title} {at}: at its foot {foot:?} {window:?}");
    assert_eq!(foot.height(), mg_uikit::dialog::FOOTER_HEIGHT, "{title} {at}");
    for (i, b) in seen.buttons.iter().enumerate() {
        assert!(foot.contains_rect(b.rect), "{title} {at}: {} cut: {:?} {foot:?}", b.words, b.rect);
        if i > 0 {
            let before = &seen.buttons[i - 1];
            assert!(
                before.rect.right() <= b.rect.left(),
                "{title} {at}: {} over {}",
                before.words,
                b.words
            );
        }
    }
    // Its whole order, as the table has it.
    let words: Vec<&str> = seen.buttons.iter().map(|b| b.words.as_str()).collect();
    let (order, exception) = table::order(op.key).unwrap_or_else(|| {
        panic!("{title}: not in table::ORDERS: (\"{}\", &{words:?}, None),", op.key)
    });
    assert_eq!(words, order, "{title} {at}: its buttons in the table's order");
    let mains: Vec<usize> = (0..seen.buttons.len()).filter(|i| seen.buttons[*i].main).collect();
    assert!(mains.len() <= 1, "{title}: one main action, not {mains:?}");
    if let Some(&m) = mains.first() {
        assert!(
            m == 0 || exception.is_some(),
            "{title}: {:?} before the main action {} and no exception named in the table",
            &words[..m],
            words[m]
        );
    }
    if let Some(l) = seen.buttons.iter().rposition(|b| b.leaves) {
        assert_eq!(l, seen.buttons.len() - 1, "{title}: {} last", seen.buttons[l].words);
    }
    let withheld = table::ENTER_WITHHELD.contains(&op.key);
    assert_eq!(seen.enter_withheld, withheld, "{title}: Enter withheld as the table has it");
    seen
}

/// Gives up the keys held by a control.
fn unfocus(h: &mut Harness<'static, Moonglow>) {
    h.ctx.memory_mut(|m| {
        if let Some(f) = m.focused() {
            m.surrender_focus(f);
        }
    });
}

/// One frame: run to its end, or for a dialog shown while a job runs
/// (which asks for frames), stepped once.
fn frame(h: &mut Harness<'static, Moonglow>, job: bool) {
    match job {
        true => h.step(),
        false => _ = h.run(),
    }
}

thread_local! {
    /// The text copied by the frame after a key ([`pressed`]).
    static COPIED: std::cell::RefCell<Vec<String>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// `key` pressed in the dialog `op` and the frame that takes it stepped,
/// what it copied noted ([`copied`]), then let rest.
fn pressed(h: &mut Harness<'static, Moonglow>, op: &Opener, key: egui::Key) {
    // (Down and up a frame each, so the frame the key went down in, which
    // copies, is read too: `key_press` steps both and keeps the last.)
    let copies = |h: &Harness<'static, Moonglow>| -> Vec<String> {
        (h.output().platform_output.commands.iter())
            .filter_map(|c| match c {
                egui::OutputCommand::CopyText(t) => Some(t.clone()),
                _ => None,
            })
            .collect()
    };
    h.key_down(key);
    h.step();
    let mut copied = copies(h);
    h.key_up(key);
    h.step();
    copied.extend(copies(h));
    COPIED.with(|c| *c.borrow_mut() = copied);
    rested(h, op.title, op.job);
}

/// Whether `done` holds for `op`, open in `h`, after a key [`pressed`]:
/// for a dialog of the window's, once it has rested; for a job's, by a
/// deadline, frames stepped meanwhile. (A job's window goes, and its
/// answer is seen, when its thread has taken the press and ended; the
/// window is still until then, and on a busy machine longer than
/// [`rested`]'s four frames: Windows CI saw Plugin Question still shown
/// after its Escape.)
fn comes_to(
    h: &mut Harness<'static, Moonglow>,
    op: &Opener,
    done: impl Fn(&Harness<'static, Moonglow>) -> bool,
) -> bool {
    let started = std::time::Instant::now();
    while op.job && !done(h) && started.elapsed() < SETTLE_TIME {
        h.step();
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    done(h)
}

thread_local! {
    /// The words the dialog showed before the last Enter ([`enter`]).
    static BEFORE: std::cell::RefCell<Vec<String>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// The words (names and values) the accessibility tree gives inside the
/// dialog `title`'s window, its footer's buttons left out.
fn words_in(h: &Harness<'_, Moonglow>, title: &str) -> Vec<String> {
    let seen = mg_ui::widgets::footers_shown(&h.ctx).into_iter().find(|f| f.title == title);
    let Some(seen) = seen else { return Vec::new() };
    let body = seen.window.with_max_y(seen.rect.map_or(seen.window.bottom(), |f| f.top()));
    h.query_all(egui_kittest::kittest::by())
        .filter(|n| n.accesskit_node().bounding_box().is_some() && body.contains_rect(n.rect()))
        .filter_map(|n| {
            let n = n.accesskit_node();
            n.label().or_else(|| n.value()).filter(|w| !w.trim().is_empty())
        })
        .collect()
}

/// The dialog `title` still open, showing other words than before Enter
/// (its main action's result in it: a search's results, the next page).
pub(crate) fn shows_other_words(h: &Harness<'_, Moonglow>, title: &str) -> bool {
    is_shown(h, title) && BEFORE.with(|b| *b.borrow() != words_in(h, title))
}

/// What the last key [`pressed`] copied.
pub(crate) fn copied() -> Vec<String> {
    COPIED.with(|c| c.borrow().clone())
}

/// Escape for `op`, open in `h`: its Cancel (or what leaves it) pressed by
/// Escape and the dialog closed.
fn escape(h: &mut Harness<'static, Moonglow>, op: &Opener, seen: &mg_ui::widgets::FooterSeen) {
    let title = op.title;
    // (Nothing in it holding the keys: a field's Escape is the field's.)
    unfocus(h);
    frame(h, op.job);
    mg_ui::widgets::dialog_presses(&h.ctx);
    pressed(h, op, egui::Key::Escape);
    let presses = mg_ui::widgets::dialog_presses(&h.ctx);
    let leave = seen.buttons.iter().rev().find(|b| b.leaves);
    if let Some(leave) = leave {
        // (Its Cancel, or the Close of a dialog that has one, which is
        // its Cancel: both leave it unanswered.)
        let pressed = presses.iter().any(|(t, w, how)| {
            t == title && (*w == leave.words || w == "Close") && *how == "Escape"
        });
        assert!(pressed, "{title}: Escape is its {}: {presses:?}", leave.words);
    }
    assert!(comes_to(h, op, |h| !is_shown(h, title)), "{title}: closed by Escape");
}

/// Enter for `op`, open in `h`, the keys held by its `field`th text field
/// or by nothing: its main action's effect seen (it closed, or what its
/// opener says) where the main action is enabled and Enter not withheld
/// from it; else nothing pressed and the dialog still open.
fn enter(h: &mut Harness<'static, Moonglow>, op: &Opener, field: Option<usize>, how: &str) {
    let title = op.title;
    let before = seen(h, title);
    match field {
        Some(nth) => {
            let fields = fields_in(h, title);
            fields.get(nth).unwrap_or_else(|| panic!("{title}: its field {nth}")).focus();
        }
        None => unfocus(h),
    }
    frame(h, op.job);
    BEFORE.with(|b| *b.borrow_mut() = words_in(h, title));
    mg_ui::widgets::dialog_presses(&h.ctx);
    pressed(h, op, egui::Key::Enter);
    let presses = mg_ui::widgets::dialog_presses(&h.ctx);
    let mine: Vec<_> = presses.iter().filter(|(t, _, _)| t == title).collect();
    let main = before.buttons.iter().find(|b| b.main && b.enabled);
    // (For the message: what the log said last.)
    let log = &h.state().log.entries;
    let said = log[log.len().saturating_sub(3)..].to_vec();
    match main.filter(|_| !before.enter_withheld) {
        Some(main) => match op.effect {
            Some((what, done)) => assert!(
                comes_to(h, op, done),
                "{title} ({how}): Enter is its {}: {what}: {presses:?}, the log {said:?}, \
                 the words before {:?} and after {:?}",
                main.words,
                BEFORE.with(|b| b.borrow().clone()),
                words_in(h, title)
            ),
            None => assert!(
                comes_to(h, op, |h| !is_shown(h, title)),
                "{title} ({how}): Enter is its {}, which closes it: {presses:?}, the log {said:?}",
                main.words
            ),
        },
        None => {
            assert!(
                mine.iter().all(|(_, _, h)| *h != "Enter"),
                "{title} ({how}): no main action Enter takes, Enter pressed {mine:?}"
            );
            match op.enter_when_off.filter(|_| !before.enter_withheld) {
                Some((what, done)) => {
                    let done = comes_to(h, op, done);
                    assert!(done, "{title} ({how}): its main action off, Enter {what}")
                }
                None => assert!(is_shown(h, title), "{title} ({how}): Enter leaves it open"),
            }
        }
    }
}

/// Enter withheld from `op`'s main action, the other way: with the main
/// action holding the focus, Enter presses it (as at the tip), and its
/// effect is seen.
fn enter_on_the_focused_main(h: &mut Harness<'static, Moonglow>, op: &Opener) {
    let title = op.title;
    let before = seen(h, title);
    let main = before.buttons.iter().find(|b| b.main).expect("a main action");
    let node = h
        .query_all_by_label(&main.words)
        .find(|n| main.rect.contains(n.rect().center()))
        .unwrap_or_else(|| panic!("{title}: its {}", main.words));
    node.focus();
    frame(h, op.job);
    mg_ui::widgets::dialog_presses(&h.ctx);
    pressed(h, op, egui::Key::Enter);
    let presses = mg_ui::widgets::dialog_presses(&h.ctx);
    assert!(
        presses.iter().any(|(t, w, _)| t == title && *w == main.words),
        "{title}: Enter on its focused {} presses it: {presses:?}",
        main.words
    );
    match op.effect {
        Some((what, done)) => assert!(comes_to(h, op, done), "{title}: {}: {what}", main.words),
        None => assert!(
            comes_to(h, op, |h| !is_shown(h, title)),
            "{title}: its {} closes it",
            main.words
        ),
    }
}

/// Opens `op` in a new window of `size` at `zoom` and lets it rest.
pub(crate) fn opened(
    op: &Opener,
    size: egui::Vec2,
    zoom: f32,
) -> Option<Box<Harness<'static, Moonglow>>> {
    // Boxed: a test holds several of these at once, and a debug build keeps
    // each on the stack whole (Windows' test threads overflowed theirs).
    let mut h = Box::new((op.setup)()?);
    h.state_mut().settings.ui_scale = Some((zoom * 100.0).round() as u16);
    h.set_size(size);
    settle(&mut h);
    (op.open)(&mut h);
    rested(&mut h, op.title, op.job);
    Some(h)
}

/// What a test's panic said.
fn said(e: Box<dyn std::any::Any + Send>) -> String {
    e.downcast_ref::<String>()
        .cloned()
        .or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string()))
        .unwrap_or_default()
}

/// One of `ops` held to the pattern (module comment).
fn hold_one(op: &Opener) {
    let screen = egui::vec2(1280.0, 720.0);
    let title = op.title;
    let Some(mut h) = opened(op, screen, 1.0) else { return };
    let shown = holds_the_pattern(&mut h, op, "at 1280 by 720");
    escape(&mut h, op, &shown);
    if !op.bare {
        // Enter with nothing holding the keys.
        let mut h = opened(op, screen, 1.0).expect("opened before");
        enter(&mut h, op, None, "nothing holding the keys");
        // Withheld: Enter on the main action holding the focus presses it.
        if shown.enter_withheld {
            let mut h = opened(op, screen, 1.0).expect("opened before");
            enter_on_the_focused_main(&mut h, op);
        }
        // Enter in a one-line field.
        if op.field.is_some() {
            let mut h = opened(op, screen, 1.0).expect("opened before");
            enter(&mut h, op, op.field, "a field holding the keys");
        }
        // The main action off: Enter presses nothing.
        if let Some(off) = op.off {
            let mut h = opened(op, screen, 1.0).expect("opened before");
            off(&mut h);
            rested(&mut h, title, op.job);
            let now = seen(&h, title);
            let main = (now.buttons.iter().find(|b| b.main))
                .unwrap_or_else(|| panic!("{title}: a main action"));
            assert!(!main.enabled, "{title}: its {} off", main.words);
            enter(&mut h, op, None, "its main action off");
        }
    }
    // Whole at each size and zoom, in each of its states. ((screen, 1.0) is
    // already checked above when op.states is empty.)
    if op.states.is_empty() {
        let mut h = opened(op, mg_ui::LEAST_WINDOW_SIZE, 1.0).expect("opened before");
        for (size, zoom) in
            [(mg_ui::LEAST_WINDOW_SIZE, 1.0_f32), (screen, 1.25_f32), (screen, 1.5_f32)]
        {
            let at = format!("at {size:?}, zoom {zoom}");
            h.state_mut().settings.ui_scale = Some((zoom * 100.0).round() as u16);
            h.set_size(size);
            rested(&mut h, title, op.job);
            holds_the_pattern(&mut h, op, &at);
        }
    } else {
        for (size, zoom) in
            [(screen, 1.0), (mg_ui::LEAST_WINDOW_SIZE, 1.0), (screen, 1.25), (screen, 1.5)]
        {
            let at = format!("at {size:?}, zoom {zoom}");
            let mut h = opened(op, size, zoom).expect("opened before");
            holds_the_pattern(&mut h, op, &at);
            for (state, step) in &op.states {
                step(&mut h);
                rested(&mut h, title, op.job);
                holds_the_pattern(&mut h, op, &format!("{state}, {at}"));
            }
        }
    }
}

/// Each of `ops` held to the pattern; every dialog that fails is named
/// with why, not the first alone.
pub(crate) fn hold(ops: &[Opener]) {
    let mut failed = Vec::new();
    for op in ops {
        if let Err(e) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| hold_one(op))) {
            failed.push(format!("{} ({}): {}", op.title, op.key, said(e)));
        }
    }
    assert!(
        failed.is_empty(),
        "{} dialogs off the pattern:\n{}",
        failed.len(),
        failed.join("\n\n")
    );
}

#[test]
fn the_core_dialogs_hold_the_pattern() {
    hold(&openers_core::openers());
}

#[test]
fn the_world_dialogs_hold_the_pattern() {
    hold(&openers_world::openers());
}

#[test]
fn the_module_dialogs_hold_the_pattern() {
    hold(&openers_module::openers());
}

#[test]
fn the_editor_dialogs_hold_the_pattern() {
    hold(&openers_editors::openers());
}

#[test]
fn the_wizard_dialogs_hold_the_pattern() {
    hold(&openers_wizards::openers());
}

#[test]
fn the_shared_dialogs_hold_the_pattern() {
    hold(&openers_shared::openers());
}

thread_local! {
    /// What the test running on this thread adds to its scratch folders'
    /// names ([`scratch`]): the tables' tests none, the others their own, so
    /// that two tests opening the same dialog at once keep apart.
    static RUN: std::cell::Cell<&'static str> = const { std::cell::Cell::new("") };
}

/// The name of an opener's scratch folder (`name`) for the test running on
/// this thread.
pub(crate) fn scratch(name: &str) -> String {
    RUN.with(|r| format!("{name}{}", r.get()))
}

/// The test on this thread opens its dialogs in scratch folders of its own,
/// `tag` added to their names.
pub(crate) fn running(tag: &'static str) {
    RUN.with(|r| r.set(tag));
}

/// The opener of the dialog `key`.
fn op(key: &str) -> Opener {
    all().into_iter().find(|o| o.key == key).unwrap_or_else(|| panic!("an opener for {key}"))
}

/// `key` pressed, down and up, and the application at rest (`job`: a job's
/// window up, waited on as [`rested`] does).
fn press_key(h: &mut Harness<'static, Moonglow>, key: egui::Key, title: &str, job: bool) {
    h.key_press(key);
    rested(h, title, job);
}

/// One key, one window, the topmost (U27's review, fault 1): with the job
/// window (an `egui::Modal`) over New Module, Enter does not create the
/// module under it, and Escape calls the job off and leaves New Module
/// open, its name kept.
#[test]
fn keys_reach_the_job_window_not_the_dialog_under_it() {
    running("-keys-job");
    let new_module = op("wizards.rs:ui#1");
    for key in [egui::Key::Enter, egui::Key::Escape] {
        let mut h = opened(&new_module, egui::vec2(1280.0, 720.0), 1.0).expect("opened");
        h.event(egui::Event::Text("Keep".into()));
        settle(&mut h);
        unfocus(&mut h);
        settle(&mut h);
        h.state_mut().start_job_for_tests("Held", |progress| {
            progress.step(1, 3);
            let started = std::time::Instant::now();
            while !progress.cancelled() && started.elapsed().as_secs() < 20 {
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
        });
        rested(&mut h, "Held", true);
        assert!(is_shown(&h, "New Module") && is_shown(&h, "Held"), "both up");
        mg_ui::widgets::dialog_presses(&h.ctx);
        press_key(&mut h, key, "Held", true);
        let presses = mg_ui::widgets::dialog_presses(&h.ctx);
        assert!(
            presses.iter().all(|(t, _, _)| t != "New Module"),
            "{key:?}: nothing pressed in New Module under the job: {presses:?}"
        );
        assert!(h.state().ws.is_none(), "{key:?}: no module made under the job");
        assert!(is_shown(&h, "New Module"), "{key:?}: New Module stays open");
        match key {
            egui::Key::Escape => {
                // (The window goes once the job's thread has seen Cancel.)
                settle(&mut h);
                assert!(!is_shown(&h, "Held") && !h.state().busy(), "Escape calls the job off")
            }
            _ => assert!(is_shown(&h, "Held"), "Enter leaves the job running (no main action)"),
        }
        if is_shown(&h, "Held") {
            h.get_all_by_label("Cancel").last().unwrap().click();
            rested(&mut h, "Held", true);
        }
        // (Its name kept: nothing under the job took the keys.)
        let name = match &h.state().wizard {
            Some(mg_ui::wizards::Wizard::NewModule { name }) => name.clone(),
            _ => String::new(),
        };
        assert!(name.contains("Keep"), "{key:?}: the name kept, not {name:?}");
    }
}

/// A picker over a wizard: Escape closes the picker, not the wizard, and
/// Enter is the picker's, not the wizard's Next.
#[test]
fn keys_reach_a_picker_not_the_wizard_under_it() {
    running("-keys-picker");
    let wizard = op("script_wizard.rs:window#1");
    for key in [egui::Key::Escape, egui::Key::Enter] {
        let mut h = opened(&wizard, egui::vec2(1280.0, 720.0), 1.0).expect("opened");
        // (An action script on its Give rewards page: its item list has a
        // picker.)
        let mut w = mg_ui::script_wizard::ScriptWizard::new(
            ResKey::parse("u27dlg", ResType::DLG).unwrap(),
            mg_gff::GffPath::root(),
            "Action Taken",
            false,
            Default::default(),
            "u27script".into(),
        );
        w.pages[0] = true;
        w.step = mg_ui::script_wizard::Step::Page(0);
        h.state_mut().script_wizard = Some(w);
        settle(&mut h);
        h.get_by_label("…").click();
        settle(&mut h);
        let picker = "Select an item blueprint";
        assert!(is_shown(&h, picker) && is_shown(&h, "Script Wizard"), "both up");
        unfocus(&mut h);
        settle(&mut h);
        mg_ui::widgets::dialog_presses(&h.ctx);
        press_key(&mut h, key, picker, false);
        let presses = mg_ui::widgets::dialog_presses(&h.ctx);
        assert!(
            presses.iter().all(|(t, _, _)| t != "Script Wizard"),
            "{key:?}: nothing pressed in the wizard under the picker: {presses:?}"
        );
        let step = h.state().script_wizard.as_ref().map(|w| w.step);
        assert_eq!(step, Some(mg_ui::script_wizard::Step::Page(0)), "{key:?}: the wizard stays");
        if key == egui::Key::Escape {
            assert!(!is_shown(&h, picker), "Escape closes the picker");
            assert!(presses.iter().any(|(t, _, k)| t == picker && *k == "Escape"), "{presses:?}");
        }
    }
}

/// A confirmation over a dialog (a plugin's Confirm, an `egui::Modal`,
/// over New Module): Escape answers the question only (No) and leaves New
/// Module open; Enter, withheld from Confirm's Yes, presses nothing, and
/// above all not New Module's Create.
#[test]
fn keys_reach_a_confirmation_not_the_dialog_under_it() {
    running("-keys-confirm");
    let new_module = op("wizards.rs:ui#1");
    let question = "Plugin Question";
    for key in [egui::Key::Escape, egui::Key::Enter] {
        let mut h = opened(&new_module, egui::vec2(1280.0, 720.0), 1.0).expect("opened");
        openers_shared::ask(&mut h, openers_shared::confirm());
        rested(&mut h, question, true);
        assert!(is_shown(&h, "New Module") && is_shown(&h, question), "both up");
        unfocus(&mut h);
        rested(&mut h, question, true);
        mg_ui::widgets::dialog_presses(&h.ctx);
        press_key(&mut h, key, question, true);
        let presses = mg_ui::widgets::dialog_presses(&h.ctx);
        assert!(
            presses.iter().all(|(t, _, _)| t != "New Module"),
            "{key:?}: nothing pressed in New Module under the question: {presses:?}"
        );
        assert!(h.state().ws.is_none(), "{key:?}: no module made under the question");
        assert!(is_shown(&h, "New Module"), "{key:?}: New Module stays open");
        match key {
            egui::Key::Escape => {
                assert!(!is_shown(&h, question), "Escape answers the question");
                let answer = openers_shared::answered();
                assert!(answer.is_some(), "Escape answers the question: {answer:?}");
            }
            _ => {
                assert!(is_shown(&h, question), "Enter, withheld from Yes, leaves it asking");
                assert_eq!(openers_shared::answered(), None, "not answered by Enter");
                h.get_by_label("No").click();
                rested(&mut h, question, true);
            }
        }
    }
}

/// A plugin's Message (one button, OK, its main action): Escape closes it
/// as Enter does, answered OK either way (U27's review: Escape left it up).
#[test]
fn a_plugin_message_closes_by_escape_and_by_enter() {
    running("-message");
    let message = op("jobs.rs:window#1 Message");
    for key in [egui::Key::Escape, egui::Key::Enter] {
        let mut h = opened(&message, egui::vec2(1280.0, 720.0), 1.0).expect("opened");
        assert!(is_shown(&h, message.title), "asked");
        unfocus(&mut h);
        rested(&mut h, message.title, true);
        press_key(&mut h, key, message.title, true);
        let closed = comes_to(&mut h, &message, |h| {
            !is_shown(h, message.title) && openers_shared::answered().is_some()
        });
        assert!(closed, "{key:?} closes the message");
        let answer = openers_shared::answered();
        assert_eq!(answer, Some(Some(mg_plugin::Answer::Yes)), "{key:?}: answered OK");
    }
}

/// The lowest named widget of the dialog `seen`'s body (above its footer).
fn lowest_in_body(h: &Harness<'_, Moonglow>, seen: &mg_ui::widgets::FooterSeen) -> egui::Rect {
    let foot = seen.rect.expect("a footer");
    let body = seen.window.with_max_y(foot.top());
    h.query_all(egui_kittest::kittest::by())
        .filter(|n| n.accesskit_node().bounding_box().is_some())
        .map(|n| n.rect())
        .filter(|r| r.is_positive() && body.contains_rect(*r) && r.top() > body.top() + 30.0)
        .fold(egui::Rect::NOTHING, |a, r| if r.bottom() > a.bottom() { r } else { a })
}

/// Recover Unsaved Work, Update Instances and Change Tileset fit their
/// content (U27's review, fault 2): no empty band between the body and the
/// footer, which stood at the foot of a window egui had made taller.
/// Recover is the board's 548 by 173 (to a few points: its words are the
/// test's), its times in one column.
#[test]
fn small_dialogs_fit_their_content() {
    running("-fit");
    for key in
        ["recovery.rs:window#1", "bulk.rs:update_window#1", "area_reshape.rs:tileset_window#1"]
    {
        let o = op(key);
        let Some(h) = opened(&o, egui::vec2(1280.0, 720.0), 1.0) else { continue };
        let seen = seen(&h, o.title);
        let foot = seen.rect.expect("a footer");
        let lowest = lowest_in_body(&h, &seen);
        let gap = foot.top() - lowest.bottom();
        eprintln!("FIT {} window {:?} gap {gap}", o.title, seen.window.size());
        assert!(gap <= 2.0 * 12.0 + 4.0, "{}: {gap} empty above its footer", o.title);
        if o.title == "Recover Unsaved Work" {
            let size = seen.window.size();
            assert!((size.x - 548.0).abs() <= 1.0, "Recover {size:?}: the board's 548 wide");
            assert!((size.y - 173.0).abs() <= 6.0, "Recover {size:?}: about the board's 173 high");
            // (The times in one column: their left edges level.)
            let times: Vec<f32> =
                h.query_all_by_label_contains(" ago").map(|n| n.rect().left()).collect();
            assert_eq!(times.len(), 2, "two copies' times: {times:?}");
            assert!((times[0] - times[1]).abs() < 0.5, "the times aligned: {times:?}");
        }
    }
}

/// Rename is the Dialogs board's 440 wide (U30's #9: it was about 345),
/// and never wider than the window: at the least window and at 150%.
#[test]
fn rename_is_the_board_s_440_wide_within_the_window() {
    running("-rename-width");
    let o = op("references.rs:rename_window#1");
    for (size, zoom) in [
        (egui::vec2(1280.0, 720.0), 1.0),
        (mg_ui::LEAST_WINDOW_SIZE, 1.0),
        (egui::vec2(640.0, 720.0), 1.5),
    ] {
        let Some(h) = opened(&o, size, zoom) else { return };
        let window = window_now(&h, o.title);
        let screen = size.x / zoom;
        if screen > 500.0 {
            let w = window.width();
            assert!((w - 440.0).abs() <= 1.0, "at {size:?} by {zoom}: {w:.0} wide, not 440");
        }
        let inside = window.left() >= -0.5 && window.right() <= screen + 0.5;
        assert!(inside, "at {size:?} by {zoom}: {window:?} past the window");
    }
}

/// A body that must scroll at the least window brings the control Tab
/// moves to into sight (U27's review, fault 3): each focused control, Tab
/// after Tab, stands inside the body as drawn, not under the footer or
/// past the window's edge.
#[test]
fn tab_brings_a_scrolled_body_s_control_into_sight() {
    running("-tab");
    let mut scrolled = Vec::new();
    for key in ["script_wizard.rs:window#1"] {
        let o = op(key);
        let Some(mut h) = opened(&o, mg_ui::LEAST_WINDOW_SIZE, 1.0) else { continue };
        let first = seen(&h, o.title);
        let bars = h.query_all(egui_kittest::kittest::by().role(egui::accesskit::Role::ScrollBar));
        let in_body = bars.filter(|b| first.window.contains_rect(b.rect())).count();
        if in_body == 0 {
            continue;
        }
        scrolled.push(o.title);
        // (From the body's first control: Tab goes through the windows under
        // the dialog first, as egui's Tab goes in the order drawn.)
        let foot = first.rect.expect("a footer");
        let body = first.window.with_max_y(foot.top()).with_min_y(first.window.top() + 30.0);
        let start = h
            .query_all(egui_kittest::kittest::by())
            .filter(|n| n.accesskit_node().bounding_box().is_some())
            .filter(|n| {
                use egui::accesskit::Role;
                let role = n.accesskit_node().role();
                let focusable = matches!(
                    role,
                    Role::Button
                        | Role::TextInput
                        | Role::SpinButton
                        | Role::CheckBox
                        | Role::ComboBox
                        | Role::Slider
                        | Role::RadioButton
                );
                focusable && body.contains_rect(n.rect())
            })
            .min_by(|a, b| a.rect().top().total_cmp(&b.rect().top()))
            .unwrap_or_else(|| panic!("{}: a control in its body", o.title));
        start.focus();
        settle(&mut h);
        let mut focused = Vec::new();
        for n in 0..60 {
            h.key_press(egui::Key::Tab);
            settle(&mut h);
            let Some(id) = h.ctx.memory(|m| m.focused()) else { continue };
            let Some(r) = h.ctx.read_response(id) else { continue };
            let now = seen(&h, o.title);
            let foot = now.rect.expect("a footer");
            // (The dialog's own controls: Tab goes on to the window's under
            // it too, as egui's Tab does.)
            if r.layer_id != now.layer || foot.contains_rect(r.rect) {
                continue;
            }
            // (Whole in sight: the part of it its clip shows, which for a
            // control of the body is the scrolled body's view, is all of it;
            // and above the footer.)
            let shown = r.interact_rect;
            assert!(
                shown.expand(0.5).contains_rect(r.rect) && r.rect.bottom() <= foot.top() + 0.5,
                "{} Tab {n}: the focused control {:?} out of sight (shown {shown:?}, the \
                 footer {foot:?})",
                o.title,
                r.rect
            );
            focused.push(r.rect.top());
        }
        let lowest = focused.iter().copied().fold(f32::MIN, f32::max);
        let highest = focused.iter().copied().fold(f32::MAX, f32::min);
        assert!(focused.len() > 3, "{}: Tab moved through its body: {focused:?}", o.title);
        eprintln!("{}: focused tops from {highest} to {lowest}", o.title);
    }
    assert!(!scrolled.is_empty(), "a dialog that scrolls at the least window");
}

/// What the window says names a menu, a tab or a section as they are drawn:
/// a menu path with "›" as the menus show it, the Plugins tab (Plugins ›
/// Manage Plugins opens it), the NUI Creator's Row Data section. (The
/// user manual's chapters were read against the program in M5's V2; these
/// were the strings it found naming them otherwise.)
#[test]
fn messages_name_menus_tabs_and_sections_as_drawn() {
    let wrong = [
        "Tools > Options",
        "Plugins › Manage Plugins lists",
        "enable it in Manage Plugins",
        "list's Row data",
    ];
    let mut found = Vec::new();
    for (name, text) in sources() {
        for (n, line) in text.lines().enumerate() {
            let code = line.trim_start();
            if code.starts_with("//") {
                continue;
            }
            for w in wrong.iter().filter(|w| code.contains(**w)) {
                found.push(format!("{name}:{} says {w:?}", n + 1));
            }
        }
    }
    assert!(found.is_empty(), "interface text naming a place otherwise: {found:#?}");
}
