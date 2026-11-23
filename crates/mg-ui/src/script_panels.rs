//! A script's own panels of the Bottom Dock (M4's U13, the Script board):
//! **Compiler**, what its latest compile said, a click on a message with
//! a place going to it; **Bookmarks**, its bookmarked lines and numbered
//! bookmarks, a click going to the line. A script's tab brings them, with
//! their switches in the Status Bar after the Log's (`shell::brings_of`);
//! floating, its window's bottom pane shows them, Search Results after
//! them (Floating.png). Each body is drawn before the script's editor, as
//! the docks are: what it does is the editor's to do once drawn
//! (`ScriptTools::jump`), and where the editor changes what it shows (a
//! compile, a bookmark) in the same pass, the frame is drawn again
//! (`ScriptTools::made`, `shell::follow_bottom`).

use egui::Ui;
use mg_resman::ResKey;

use crate::palette_view::Keys;
use crate::row_copy;
use crate::script_tools as tools;
use crate::{Moonglow, Tab};

/// The accessible name of Compiler's body (its messages): apart from the
/// panel's tab and switch, which share its name.
pub const COMPILER_MESSAGES: &str = "Compiler messages";
/// The accessible name of Bookmarks' body (its rows).
pub const BOOKMARKED_LINES: &str = "Bookmarked lines";

/// The script `editor` is, if it is one.
fn script_of(editor: Option<&Tab>) -> Option<ResKey> {
    match editor? {
        Tab::Script(key) => Some(*key),
        _ => None,
    }
}

/// What every Bottom Dock panel of the script's (and Search Results)
/// does first: names its body `name` for a screen reader (and the tests),
/// draws the outline of the pane the arrow keys are in where they are its
/// (`keys`), takes the keyboard's focus where F6 brought them to it (from
/// the script's text: what is typed then goes nowhere), and notes what it
/// showed in this pass ([`tools::ScriptTools::bottom_drawn`]).
pub(crate) fn begin(app: &mut Moonglow, ui: &mut Ui, name: &str, keys: Keys) {
    mg_uikit::name_group(ui, name);
    crate::palette_view::keys_outline(ui, ui.max_rect(), keys);
    // (A stop of F6, not of Tab: the panel is made focusable only while
    // F6 brings the keys to it, or it has them.)
    let id = ui.id().with("bottom-panel-focus");
    let asked = app.script_tools.list_focus.take_if(|k| *k == keys).is_some();
    if asked || ui.memory(|m| m.has_focus(id)) {
        let focus = ui.interact(ui.max_rect(), id, egui::Sense::FOCUSABLE);
        let said = format!("{name}, the panel");
        focus.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Other, true, &said));
        if asked {
            focus.request_focus();
        }
    }
    let pass = ui.ctx().cumulative_pass_nr();
    let made = app.script_tools.made;
    let t = &mut app.script_tools;
    t.bottom_drawn = Some(match t.bottom_drawn {
        Some((p, least)) if p == pass => (pass, least.min(made)),
        _ => (pass, made),
    });
}

/// Compiler's body: first, where checking as you type found an error in
/// the script, a "Found as you type" row with the compiler's words whole
/// (wrapped, 5.2.0: the Status Bar's line cut it short, Jasperre's report),
/// a click going to its line (in the include, where it is in one); then
/// the script's messages from its latest compile (a save not compiled, To
/// Scratch), each after the time it was said, on the board's inset ground
/// (`.compiler-out`), an error in the theme's `error`; a click on one
/// that says where goes there (the script, or the include, at its line).
/// Each row's menu (a right click) copies it or every row, without their
/// times (no row is chosen, so no Ctrl+C).
pub(crate) fn compiler(app: &mut Moonglow, ui: &mut Ui, editor: Option<&Tab>) {
    let Some(key) = script_of(editor) else { return };
    let t = mg_uikit::tokens(ui.ctx());
    ui.painter().rect_filled(ui.max_rect(), 0.0, t.inset);
    begin(app, ui, COMPILER_MESSAGES, Keys::Compiler);
    let mut open = None;
    let mut copy = None;
    let margin = egui::Margin { left: 10, right: 10, top: 6, bottom: 6 };
    egui::Frame::new().inner_margin(margin).show(ui, |ui| {
        let area = egui::ScrollArea::vertical().id_salt("compiler-messages");
        let mut any = false;
        area.auto_shrink([false, false]).show(ui, |ui| {
            let font = egui::TextStyle::Monospace.resolve(ui.style());
            if let Some((said, to)) = live_row(app, key) {
                any = true;
                let format = |color| egui::TextFormat::simple(font.clone(), color);
                let mut job = egui::text::LayoutJob::default();
                job.append(FOUND_AS_YOU_TYPE, 0.0, format(t.text_3));
                job.append(said, 8.0, format(t.error));
                job.wrap.max_width = ui.available_width();
                let row =
                    egui::Label::new(job).wrap().selectable(false).sense(egui::Sense::click());
                let r = ui.add(row);
                r.widget_info(|| {
                    let name = format!("{FOUND_AS_YOU_TYPE}: {said}");
                    egui::WidgetInfo::labeled(egui::WidgetType::Button, true, name)
                });
                match row_copy::row_menu(ui, &r, false, |_| false) {
                    Some(row_copy::Asked::Row) => copy = Some(said.to_string()),
                    Some(row_copy::Asked::All) => copy = Some(messages_text(app, key)),
                    None => {}
                }
                let r = match to {
                    Some(_) => r.on_hover_text("Click to go to the line"),
                    None => r,
                };
                if r.clicked()
                    && let Some((file, line)) = to
                {
                    open =
                        Some((file.map_or_else(|| key.resref.to_string(), str::to_string), line));
                }
                ui.add_space(4.0);
            }
            for m in app.script_tools.messages.iter().filter(|m| m.script == key) {
                any = true;
                let color = if m.error { t.error } else { t.text };
                // (When it was said, before it, quieter: two compiles in a
                // row read as two.)
                let mut job = egui::text::LayoutJob::default();
                let format = |color| egui::TextFormat::simple(font.clone(), color);
                job.append(&m.at, 0.0, format(t.text_3));
                job.append(&format!(" {}", m.text), 0.0, format(color));
                let r = ui.add(egui::Label::new(job).sense(egui::Sense::click()));
                // (Shift+F10 and Ctrl+C on the message the keyboard is on.)
                let (keyed, ctrl_c) = row_copy::focused_keys(ui, &r);
                if ctrl_c {
                    copy = Some(m.text.clone());
                }
                match row_copy::row_menu(ui, &r, keyed, |_| false) {
                    Some(row_copy::Asked::Row) => copy = Some(m.text.clone()),
                    Some(row_copy::Asked::All) => copy = Some(messages_text(app, key)),
                    None => {}
                }
                let r = match m.location {
                    Some(_) => r.on_hover_text("Click to go to the line"),
                    None => r,
                };
                if r.clicked()
                    && let Some((file, line)) = &m.location
                {
                    open = Some((file.clone(), *line));
                }
            }
            if !any {
                ui.weak("Compile saves the script and says here what the compiler found.");
            }
        });
    });
    if let Some(text) = copy {
        ui.ctx().copy_text(text);
    }
    if let Some((file, line)) = open {
        crate::script_view::open_line(app, &file, line);
    }
}

/// Every row Compiler lists for `key`, one a line (its menu's Copy All):
/// the error found as you type first, then the latest compile's messages.
fn messages_text(app: &Moonglow, key: ResKey) -> String {
    let live = live_row(app, key).map(|(said, _)| said.to_string());
    let of = app.script_tools.messages.iter().filter(|m| m.script == key);
    row_copy::lines(live.into_iter().chain(of.map(|m| m.text.clone())))
}

/// What Compiler's first row says before the error checking as you type
/// found (5.2.0).
pub const FOUND_AS_YOU_TYPE: &str = "Found as you type";

/// The error checking as you type found in `key` (none while it is off,
/// Options › Script Editor), in the compiler's words, and where a click
/// on it goes: the include it is in (none: the script itself) and the
/// line, from 1.
#[allow(clippy::type_complexity)]
fn live_row(app: &Moonglow, key: ResKey) -> Option<(&str, Option<(Option<&str>, usize)>)> {
    let (line, said) = app.live_error(key)?;
    let to = match app.live_include(key) {
        Some((include, at)) => Some((Some(include.as_str()), *at)),
        None => line.map(|l| (None, l + 1)),
    };
    Some((said.trim_end(), to))
}

/// Bookmarks' body: the script's numbered bookmarks (`[1]`, Ctrl and the
/// digit going there) and its bookmarked lines, each with its line's
/// number and text (whole in its tip); a click goes to the line, its menu
/// (a right click) copies it or every row (no row is chosen, so no
/// Ctrl+C). Bookmark (its key, the
/// header's button) sets and takes away a bookmark on the cursor's line,
/// Ctrl+Shift with a digit a numbered one.
pub(crate) fn bookmarks(app: &mut Moonglow, ui: &mut Ui, editor: Option<&Tab>) {
    let Some(key) = script_of(editor) else { return };
    begin(app, ui, BOOKMARKED_LINES, Keys::Bookmarks);
    // (Its editor not drawn yet, a tab's first frame: nothing to list.)
    let Some(buf) = app.scripts.get(&key) else { return };
    let mut jump = None;
    let mut copy = None;
    let area = egui::ScrollArea::vertical().id_salt("bookmarked-lines");
    area.auto_shrink([false, false]).show(ui, |ui| {
        mg_uikit::list_rows(ui);
        // (The marked lines' texts in one pass over the script, not one
        // from its top for each.)
        let marked = buf.numbered.iter().flatten().chain(&buf.bookmarks).copied();
        let texts = tools::lines_text(&buf.text, marked);
        let mut any = false;
        // (Each row as its menu's Copy copies it: its number and text.)
        let numbered = buf.numbered.iter().enumerate().filter_map(|(n, l)| Some((Some(n), (*l)?)));
        let rows = numbered.chain(buf.bookmarks.iter().map(|l| (None, *l)));
        let words = |n: Option<usize>, line: usize| {
            let text = texts.get(&line).copied().unwrap_or_default();
            match n {
                Some(n) => format!("[{n}] {}: {text}", line + 1),
                None => format!("{}: {text}", line + 1),
            }
        };
        for (n, line) in rows.clone() {
            any = true;
            let text = texts.get(&line).copied().unwrap_or_default();
            let shown = match n {
                Some(n) => format!("[{n}] {:>5}: {text}", line + 1),
                None => format!("{:>5}: {text}", line + 1),
            };
            let r = ui.selectable_label(false, shown);
            // (Shift+F10 and Ctrl+C on the row the keyboard is on.)
            let (keyed, ctrl_c) = row_copy::focused_keys(ui, &r);
            if ctrl_c {
                copy = Some(words(n, line));
            }
            match row_copy::row_menu(ui, &r, keyed, |_| false) {
                Some(row_copy::Asked::Row) => copy = Some(words(n, line)),
                Some(row_copy::Asked::All) => {
                    copy = Some(row_copy::lines(rows.clone().map(|(n, l)| words(n, l))));
                }
                None => {}
            }
            let tip = match n {
                Some(n) => format!("{}\nCtrl+{n}", words(None, line)),
                None => words(None, line),
            };
            if r.on_hover_text(tip).clicked() {
                jump = Some(tools::line_start(&buf.text, line));
            }
        }
        if !any {
            let key_of = app.keymap.label(crate::keys::Cmd::Bookmark, ui.ctx());
            let key_of = if key_of.is_empty() { String::new() } else { format!(" ({key_of})") };
            ui.weak(format!(
                "No bookmarks. Bookmark{key_of} marks the cursor's line, Ctrl+Shift and a digit \
                 a numbered one."
            ));
        }
    });
    if let Some(text) = copy {
        ui.ctx().copy_text(text);
    }
    if let Some(at) = jump {
        app.script_tools.jump = Some((key, at));
    }
}
