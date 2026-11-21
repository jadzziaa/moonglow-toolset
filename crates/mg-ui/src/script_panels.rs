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

/// Compiler's body: the script's messages from its latest compile (a save
/// not compiled, To Scratch), each after the time it was said, on the
/// board's inset ground (`.compiler-out`), an error in the theme's
/// `error`; a click on one that says where goes there (the script, or the
/// include, at its line).
pub(crate) fn compiler(app: &mut Moonglow, ui: &mut Ui, editor: Option<&Tab>) {
    let Some(key) = script_of(editor) else { return };
    let t = mg_uikit::tokens(ui.ctx());
    ui.painter().rect_filled(ui.max_rect(), 0.0, t.inset);
    begin(app, ui, COMPILER_MESSAGES, Keys::Compiler);
    let mut open = None;
    let margin = egui::Margin { left: 10, right: 10, top: 6, bottom: 6 };
    egui::Frame::new().inner_margin(margin).show(ui, |ui| {
        let area = egui::ScrollArea::vertical().id_salt("compiler-messages");
        let mut any = false;
        area.auto_shrink([false, false]).show(ui, |ui| {
            let font = egui::TextStyle::Monospace.resolve(ui.style());
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
    if let Some((file, line)) = open {
        crate::script_view::open_line(app, &file, line);
    }
}

/// Bookmarks' body: the script's numbered bookmarks (`[1]`, Ctrl and the
/// digit going there) and its bookmarked lines, each with its line's
/// number and text; a click goes to the line. Bookmark (its key, the
/// header's button) sets and takes away a bookmark on the cursor's line,
/// Ctrl+Shift with a digit a numbered one.
pub(crate) fn bookmarks(app: &mut Moonglow, ui: &mut Ui, editor: Option<&Tab>) {
    let Some(key) = script_of(editor) else { return };
    begin(app, ui, BOOKMARKED_LINES, Keys::Bookmarks);
    // (Its editor not drawn yet, a tab's first frame: nothing to list.)
    let Some(buf) = app.scripts.get(&key) else { return };
    let mut jump = None;
    let area = egui::ScrollArea::vertical().id_salt("bookmarked-lines");
    area.auto_shrink([false, false]).show(ui, |ui| {
        mg_uikit::list_rows(ui);
        // (The marked lines' texts in one pass over the script, not one
        // from its top for each.)
        let marked = buf.numbered.iter().flatten().chain(&buf.bookmarks).copied();
        let texts = tools::lines_text(&buf.text, marked);
        let mut any = false;
        for (n, line) in buf.numbered.iter().enumerate() {
            let Some(line) = line else { continue };
            any = true;
            let text = texts.get(line).copied().unwrap_or_default();
            if ui
                .selectable_label(false, format!("[{n}] {:>5}: {text}", line + 1))
                .on_hover_text(format!("Ctrl+{n}"))
                .clicked()
            {
                jump = Some(tools::line_start(&buf.text, *line));
            }
        }
        for line in &buf.bookmarks {
            any = true;
            let text = texts.get(line).copied().unwrap_or_default();
            if ui.selectable_label(false, format!("{:>5}: {text}", line + 1)).clicked() {
                jump = Some(tools::line_start(&buf.text, *line));
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
    if let Some(at) = jump {
        app.script_tools.jump = Some((key, at));
    }
}
