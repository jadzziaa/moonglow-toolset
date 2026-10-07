//! The Conversation Editor (`.dlg`), laid out like Aurora's: the tree of
//! NPC lines (red, `[OWNER]` or the speaker's tag) and PC lines (blue), links
//! in grey; Add, Copy, Cut, Paste, Paste As Link, Delete, Expand/Collapse
//! All; the selected line's speaker and text; and the tabs Text Appears
//! When…, Actions Taken, Other Actions, Comments and Current File. Each
//! change is one undoable command.

use std::collections::HashSet;

use egui::{Color32, RichText, Ui};
use mg_core::{ResRef, ResType};
use mg_edit::{Command, Edit, GffPath};
use mg_gff::{Gff, Struct, Value};
use mg_module::dialog::{
    ANIMATIONS, Branch, Kind, Parent, add_node, copy_branch, is_link, link_index, link_lines,
    links, move_link, new_dialog, node, paste_branch, remove, shift_link, word_count,
};
use mg_resman::ResKey;
use mg_schema::{GffValue, StructExt, jrl};

use crate::text::{decode, encode, with_english};
use crate::widgets::{FieldTarget, LocStringEdit, commit_text, resref_field};
use crate::{Action, Moonglow, Tab};

/// A selected row: a link under a parent (or nothing: Root).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Row {
    pub parent: Parent,
    pub pos: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DataTab {
    #[default]
    Condition,
    Action,
    Other,
    Comments,
    File,
}

/// The editor's state for one conversation.
#[derive(Debug, Clone, Default)]
pub struct DialogView {
    pub selected: Option<Row>,
    pub collapsed: HashSet<(Kind, u32)>,
    pub tab: DataTab,
    /// Highlight lines with a comment, an action, a quest, an animation or a
    /// sound (Aurora's filter buttons).
    pub highlight: [bool; 5],
    pub token_picker: bool,
    /// Aurora's bottom tabs: Data, Bookmarks, Search.
    pub bottom: Bottom,
    /// Bookmarked lines.
    pub bookmarks: Vec<(Kind, u32)>,
    pub search: DialogSearch,
    /// Test mode: the lines spoken, an NPC's and the player's reply in
    /// turn, the last reply last.
    pub test: Option<Vec<(Kind, u32)>>,
    /// Test mode: what each condition script is taken to return (TRUE
    /// unless set), since the editor can't run it.
    pub assume: std::collections::BTreeMap<String, bool>,
    /// The Input Text popup's new line, while it is open.
    pub input: Option<NewLine>,
}

/// A line the Input Text popup is asking for (Options › Conversation
/// Editor, "Show popup when creating a new text entry").
#[derive(Debug, Clone, PartialEq)]
pub struct NewLine {
    pub parent: Parent,
    pub text: String,
    /// Whether its text has been selected for typing over.
    pub shown: bool,
}

/// A row dropped on a line (or the root): moved, or linked with Ctrl.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Drop {
    from: Row,
    onto: Parent,
    link: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Bottom {
    #[default]
    Data,
    Bookmarks,
    Search,
}

/// Find / Replace in conversations.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DialogSearch {
    pub find: String,
    pub replace: String,
    pub match_case: bool,
    pub whole_word: bool,
    /// All conversations in the module, not only this one.
    pub all_files: bool,
    /// Conversation, line, and its text.
    pub results: Vec<(ResKey, Kind, u32, String)>,
    /// Asked for by a key: the Find What field takes the keyboard.
    focus: bool,
    /// Asked for by a key: find again.
    again: bool,
}

/// What Copy and Cut put aside (shared by all conversations, like Aurora's
/// Scrap).
#[derive(Debug, Clone, PartialEq)]
pub struct DialogClip {
    pub from: ResKey,
    pub branch: Branch,
    /// The copied line, for Paste As Link.
    pub target: (Kind, u32),
}

/// The GFF path of a link.
/// A line's own text: in the language edited (Options › Language), else,
/// where it has none in it, in another (English first).
fn text(n: &Struct) -> String {
    let Some(ls) = n.locstring("Text") else { return String::new() };
    crate::text::shown_text(ls)
}

/// A line's text as shown: its own (in the language edited), else its talk-table string
/// (the original campaign's lines, and many modules', are all in the game's
/// talk table).
fn line_text(game: Option<&mg_rules::GameData>, n: &Struct) -> String {
    let own = text(n);
    if !own.is_empty() {
        return own;
    }
    n.locstring("Text")
        .filter(|ls| !ls.strref.is_none())
        .and_then(|ls| game?.string(ls.strref))
        .unwrap_or_default()
}

fn link_path(row: Row) -> GffPath {
    match row.parent {
        Parent::Root => GffPath::root().item("StartingList", row.pos),
        Parent::Node(k, i) => GffPath::root().item(k.list(), i as usize).item(k.links(), row.pos),
    }
}

fn node_path(kind: Kind, index: u32) -> GffPath {
    GffPath::root().item(kind.list(), index as usize)
}

fn set(key: ResKey, what: &str, path: GffPath, label: &str, value: Value) -> Action {
    Action::Apply(Command::new(
        what,
        vec![Edit::SetField { key, path, label: label.to_string(), value: Some(value) }],
    ))
}

fn replace(key: ResKey, what: &str, g: &Gff) -> Action {
    Action::Apply(Command::new(what, vec![Edit::SetResource { key, data: g.to_bytes().ok() }]))
}

/// The key/value pairs of a parameter list.
fn params(s: &Struct, label: &str) -> Vec<(String, String)> {
    s.list(label)
        .unwrap_or(&[])
        .iter()
        .map(|p| {
            (
                decode(p.string("Key").unwrap_or_default()),
                decode(p.string("Value").unwrap_or_default()),
            )
        })
        .collect()
}

pub(crate) fn ui(app: &mut Moonglow, ui: &mut Ui, key: ResKey) {
    let Some(ws) = &mut app.ws else { return };
    let g = match ws.doc(&key) {
        Ok(g) => g.clone(),
        Err(e) => {
            ui.colored_label(ui.visuals().error_fg_color, e.to_string());
            return;
        }
    };
    let mut view = app.dialog_views.get(&key).cloned().unwrap_or_default();
    // A selection the conversation no longer has (undo) is dropped.
    view.selected = view.selected.filter(|r| r.pos < links(&g, r.parent).len());
    let mut actions: Vec<Action> = Vec::new();
    let target_of = |r: Row| (r.parent.child_kind(), link_index(&links(&g, r.parent)[r.pos]));

    // Options › Conversation Editor.
    let popup = !app.settings.dialog_no_text_popup;
    let paste_source_first = app.settings.dialog_paste_source_to_dest;
    let drag_source_first = !app.settings.dialog_drag_dest_to_source;
    let tlk = |strref: u32, english: &str| {
        app.game
            .as_ref()
            .and_then(|g| g.string(mg_core::StrRef(strref)))
            .unwrap_or_else(|| english.to_string())
    };
    let placeholder = tlk(10336, "<< Enter text here >>");
    let prompts = [
        tlk(67057, "Enter what the NPC says next:"),
        tlk(67056, "Enter what the player says next:"),
    ];

    // Keys, with the pointer over the editor and no text being typed:
    // Options › Keyboard's, and the platform's Copy, Cut and Paste.
    let here = ui.ui_contains_pointer() && ui.memory(|m| m.focused().is_none());
    let keymap = app.keymap.clone();
    let pressed = |c: crate::keys::Cmd| here && ui.input(|i| keymap.pressed(i, c));
    let (key_add, key_delete) =
        (pressed(crate::keys::Cmd::AddLine), pressed(crate::keys::Cmd::DeleteLine));
    let (key_find, key_find_next) =
        (pressed(crate::keys::Cmd::DialogFind), pressed(crate::keys::Cmd::DialogFindNext));
    let (key_copy, key_cut, key_paste) = if here {
        ui.input(|i| {
            let event = |f: fn(&egui::Event) -> bool| i.events.iter().any(f);
            let ctrl = |k: egui::Key| i.modifiers.command && i.key_pressed(k);
            (
                event(|e| matches!(e, egui::Event::Copy)) || ctrl(egui::Key::C),
                event(|e| matches!(e, egui::Event::Cut)) || ctrl(egui::Key::X),
                event(|e| matches!(e, egui::Event::Paste(_))) || ctrl(egui::Key::V),
            )
        })
    } else {
        (false, false, false)
    };
    let add_hint =
        keymap.titled("Add a line under the selection", crate::keys::Cmd::AddLine, ui.ctx());

    // Toolbar.
    ui.horizontal_wrapped(|ui| {
        let sel = view.selected;
        let add_parent = match sel {
            None => Some(Parent::Root),
            Some(r) if !is_link(&links(&g, r.parent)[r.pos]) => {
                let (k, i) = target_of(r);
                Some(Parent::Node(k, i))
            }
            Some(_) => None,
        };
        if (ui
            .add_enabled(add_parent.is_some(), egui::Button::new("Add"))
            .on_hover_text(add_hint)
            .clicked()
            || key_add)
            && let Some(p) = add_parent
        {
            if popup {
                view.input = Some(NewLine { parent: p, text: placeholder.clone(), shown: false });
            } else {
                // The new line is selected, its text edited in place.
                let mut ng = g.clone();
                add_node(&mut ng, p, "");
                view.selected = Some(Row { parent: p, pos: links(&ng, p).len() - 1 });
                if let Parent::Node(k, i) = p {
                    view.collapsed.remove(&(k, i));
                }
                actions.push(replace(key, "Add line", &ng));
            }
        }
        let copy = |r: Row| {
            copy_branch(&g, r.parent, r.pos).map(|b| DialogClip {
                from: key,
                branch: b,
                target: target_of(r),
            })
        };
        if ui.add_enabled(sel.is_some(), egui::Button::new("Copy")).clicked()
            || (key_copy && sel.is_some())
        {
            app.dialog_clip = sel.and_then(copy);
            if app.dialog_clip.is_some() {
                crate::widgets::mark_clipboard(ui.ctx(), "conversation lines");
            }
        }
        let mut delete = false;
        if (ui.add_enabled(sel.is_some(), egui::Button::new("Cut")).clicked() || key_cut)
            && sel.is_some()
        {
            app.dialog_clip = sel.and_then(copy);
            if app.dialog_clip.is_some() {
                crate::widgets::mark_clipboard(ui.ctx(), "conversation lines");
            }
            delete = true;
        }
        let paste_parent = add_parent;
        let clip = app.dialog_clip.clone();
        let fits = |p: Parent| clip.as_ref().is_some_and(|c| c.branch.kind == p.child_kind());
        let can_paste = paste_parent.is_some_and(fits);
        if (ui.add_enabled(can_paste, egui::Button::new("Paste")).clicked() || key_paste)
            && can_paste
            && let (Some(p), Some(c)) = (paste_parent, &clip)
        {
            let mut ng = g.clone();
            if paste_branch(&mut ng, p, &c.branch, c.from == key) {
                view.selected = Some(Row { parent: p, pos: links(&ng, p).len() - 1 });
                actions.push(replace(key, "Paste lines", &ng));
            }
        }
        // Paste As Link: by default the selected line gets a link to the
        // copied one (Link Destination To Source); the other way round,
        // the copied line gets a link to the selected one.
        let selected_line = sel.filter(|r| !is_link(&links(&g, r.parent)[r.pos])).map(target_of);
        let pair = match (selected_line, clip.as_ref().filter(|c| c.from == key)) {
            (Some(dest), Some(c)) => {
                let (from, to) =
                    if paste_source_first { (c.target, dest) } else { (dest, c.target) };
                (from.0.child() == to.0).then_some((from, to))
            }
            _ => None,
        };
        if ui.add_enabled(pair.is_some(), egui::Button::new("Paste As Link")).clicked()
            && let Some((from, to)) = pair
        {
            let mut ng = g.clone();
            if link_lines(&mut ng, from, to) {
                actions.push(replace(key, "Paste as link", &ng));
            }
        }
        if ui.add_enabled(sel.is_some(), egui::Button::new("Delete")).clicked() || key_delete {
            delete = true;
        }
        if delete && let Some(r) = sel {
            let mut ng = g.clone();
            remove(&mut ng, r.parent, r.pos);
            view.selected = None;
            actions.push(replace(key, "Delete line", &ng));
        }
        // Up and down among the lines of its parent: the order the game
        // tries them in (Alt + the arrow keys, too).
        let siblings = sel.map_or(0, |r| links(&g, r.parent).len());
        for (label, up, arrow, hint) in [
            ("⏶", true, egui::Key::ArrowUp, "Move the line up among its parent's (Alt+Up)"),
            ("⏷", false, egui::Key::ArrowDown, "Move the line down among its parent's (Alt+Down)"),
        ] {
            let can = sel.is_some_and(|r| if up { r.pos > 0 } else { r.pos + 1 < siblings });
            let key_move = here && ui.input_mut(|i| i.consume_key(egui::Modifiers::ALT, arrow));
            let clicked =
                ui.add_enabled(can, egui::Button::new(label)).on_hover_text(hint).clicked();
            if (clicked || key_move)
                && can
                && let Some(r) = sel
            {
                let mut ng = g.clone();
                if let Some(pos) = shift_link(&mut ng, r.parent, r.pos, up) {
                    view.selected = Some(Row { parent: r.parent, pos });
                    let what = if up { "Move line up" } else { "Move line down" };
                    actions.push(replace(key, what, &ng));
                }
            }
        }
        ui.separator();
        if ui.button("Expand All").clicked() {
            view.collapsed.clear();
        }
        if ui.button("Collapse All").clicked() {
            for k in [Kind::Entry, Kind::Reply] {
                for i in 0..mg_module::dialog::nodes(&g, k).len() as u32 {
                    view.collapsed.insert((k, i));
                }
            }
        }
        ui.separator();
        for (i, label) in
            ["Comments", "Actions", "Quests", "Animations", "Sounds"].iter().enumerate()
        {
            ui.toggle_value(&mut view.highlight[i], *label)
                .on_hover_text(format!("Highlight lines with {}", label.to_lowercase()));
        }
        ui.separator();
        // Export and import (dialog_io): the lines back by CSV.
        let mut export = None;
        ui.menu_button("Export", |ui| {
            for f in mg_module::dialog_io::Format::ALL {
                if ui.button(format!("{}…", f.name())).clicked() {
                    export = Some(f);
                    ui.close();
                }
            }
        });
        if let Some(f) = export {
            let suggested =
                std::path::PathBuf::from(format!("{}.{}", key.resref, f.extensions()[0]));
            if let Some(path) =
                app.dialogs.save_file(crate::dialogs::FileKind::Conversation(f), Some(&suggested))
            {
                match std::fs::write(&path, f.write(&g, &key.resref.to_string())) {
                    Ok(()) => app.log.info(format!("Exported {key} to {}", path.display())),
                    Err(e) => app.log.error(format!("{}: {e}", path.display())),
                }
            }
        }
        if ui
            .button("Import Lines…")
            .on_hover_text("Read back the text, speakers and comments of a CSV export")
            .clicked()
            && let Some(path) = app.dialogs.open_file(
                crate::dialogs::FileKind::Conversation(mg_module::dialog_io::Format::Csv),
                None,
            )
        {
            let mut ng = g.clone();
            let read = std::fs::read_to_string(&path).map_err(|e| e.to_string());
            match read.and_then(|csv| mg_module::dialog_io::update_from_csv(&mut ng, &csv)) {
                Ok(0) => app.log.info(format!("{}: no lines changed", path.display())),
                Ok(n) => {
                    app.log.info(format!("{}: {n} line(s) changed", path.display()));
                    actions.push(replace(key, "Import lines", &ng));
                }
                Err(e) => app.log.error(format!("{}: {e}", path.display())),
            }
        }
        ui.separator();
        let mut scripts = !app.settings.dialog_hide_scripts;
        if ui
            .toggle_value(&mut scripts, "Scripts")
            .on_hover_text("Show each line's condition, action, journal update and sound")
            .changed()
        {
            app.settings.dialog_hide_scripts = !scripts;
        }
    });
    ui.separator();

    // The fields of the selection.
    // (At most seven tenths of the editor: the lines keep their room.)
    let most = (ui.available_height() * 0.7).max(80.0);
    let fields = egui::Panel::bottom(egui::Id::new(("dlg-data", key)))
        .resizable(true)
        .default_size(260.0f32.min(most))
        .max_size(most);
    fields.show(ui, |ui| {
        let words = g.root.dword("NumWords").unwrap_or(0);
        // Find and Replace, and Find Next (Options › Keyboard): the
        // Search pane.
        if key_find || key_find_next {
            view.bottom = Bottom::Search;
            view.search.focus = key_find;
            view.search.again = key_find_next;
        }
        ui.horizontal(|ui| {
            ui.selectable_value(&mut view.bottom, Bottom::Data, "Data");
            ui.selectable_value(&mut view.bottom, Bottom::Bookmarks, "Bookmarks");
            ui.selectable_value(&mut view.bottom, Bottom::Search, "Search");
            ui.separator();
            let sel = view.selected.filter(|r| r.pos < links(&g, r.parent).len());
            if ui.add_enabled(sel.is_some(), egui::Button::new("Bookmark").small()).clicked()
                && let Some(r) = sel
            {
                let t = target_of(r);
                if let Some(i) = view.bookmarks.iter().position(|b| *b == t) {
                    view.bookmarks.remove(i);
                } else {
                    view.bookmarks.push(t);
                }
            }
            if ui.button("Test").on_hover_text("Click through the conversation").clicked() {
                view.test = Some(Vec::new());
            }
        });
        ui.separator();
        if view.bottom == Bottom::Bookmarks {
            for (kind, index) in view.bookmarks.clone() {
                let Some(n) = node(&g, kind, index) else { continue };
                if ui
                    .selectable_label(
                        false,
                        format!("{kind:?} {index}: {}", line_text(app.game.as_deref(), n)),
                    )
                    .clicked()
                    && let Some((parent, pos)) = mg_module::dialog::owner(&g, kind, index)
                {
                    view.selected = Some(Row { parent, pos });
                }
            }
            return;
        }
        if view.bottom == Bottom::Search {
            search_pane(app, ui, key, &g, &mut view, &mut actions);
            return;
        }
        // A row added this frame appears once the command has run.
        match view.selected.filter(|r| r.pos < links(&g, r.parent).len()) {
            None => {
                ui.weak(format!("Root. {words} words in the file. Add adds an NPC greeting."));
                file_tab(app, ui, key, &g, &mut actions);
            }
            Some(r) => {
                let (kind, index) = target_of(r);
                let link = links(&g, r.parent)[r.pos].clone();
                let Some(n) = node(&g, kind, index).cloned() else { return };
                let line = text(&n);
                ui.weak(format!(
                    "Line: {} letters, {} words. File: {words} words.{}",
                    line.chars().count(),
                    line.split_whitespace().count(),
                    if is_link(&link) {
                        " This row is a link: the line is edited where it is owned."
                    } else {
                        ""
                    }
                ));
                ui.columns(2, |cols| {
                    text_panel(app, &mut cols[0], key, kind, index, &n, &mut view, &mut actions);
                    let ui = &mut cols[1];
                    ui.horizontal(|ui| {
                        for (t, label) in [
                            (DataTab::Condition, "Text Appears When…"),
                            (DataTab::Action, "Actions Taken"),
                            (DataTab::Other, "Other Actions"),
                            (DataTab::Comments, "Comments"),
                            (DataTab::File, "Current File"),
                        ] {
                            ui.selectable_value(&mut view.tab, t, label);
                        }
                    });
                    ui.separator();
                    egui::ScrollArea::vertical().id_salt(("dlg-tab", key)).show(
                        ui,
                        |ui| match view.tab {
                            DataTab::Condition => script_with_params(
                                app,
                                ui,
                                key,
                                link_path(r),
                                "Active",
                                "ConditionParams",
                                &link,
                                "cond",
                                &mut actions,
                            ),
                            DataTab::Action => script_with_params(
                                app,
                                ui,
                                key,
                                node_path(kind, index),
                                "Script",
                                "ActionParams",
                                &n,
                                "act",
                                &mut actions,
                            ),
                            DataTab::Other => {
                                other_tab(app, ui, key, kind, index, &n, &mut actions)
                            }
                            DataTab::Comments => {
                                let (path, label, value) = if is_link(&link) {
                                    (
                                        link_path(r),
                                        "LinkComment",
                                        decode(link.string("LinkComment").unwrap_or_default()),
                                    )
                                } else {
                                    (
                                        node_path(kind, index),
                                        "Comment",
                                        decode(n.string("Comment").unwrap_or_default()),
                                    )
                                };
                                let id = egui::Id::new(("dlg-comment", key, kind, index, label));
                                if let Some(v) =
                                    commit_text(app, ui, id, &value, true, f32::INFINITY)
                                {
                                    actions.push(set(
                                        key,
                                        "Comment",
                                        path,
                                        label,
                                        Value::String(encode(&v)),
                                    ));
                                }
                            }
                            DataTab::File => file_tab(app, ui, key, &g, &mut actions),
                        },
                    );
                });
            }
        }
    });

    // The tree, in the Options' colours.
    let hl = view.highlight;
    let mut dropped: Option<Drop> = None;
    let look = Look::new(&app.settings);
    egui::ScrollArea::both().id_salt(("dlg-tree", key)).auto_shrink([false, false]).show(
        ui,
        |ui| {
            let root = ui.selectable_label(view.selected.is_none(), RichText::new("Root").strong());
            if root.clicked() {
                view.selected = None;
            }
            if let Some(from) = root.dnd_release_payload::<Row>() {
                dropped = Some(Drop { from: *from, onto: Parent::Root, link: link_key(ui) });
            }
            let mut path = HashSet::new();
            tree(
                ui,
                app.game.as_deref(),
                &g,
                Parent::Root,
                1,
                &mut view,
                &mut path,
                hl,
                &look,
                &mut dropped,
            );
        },
    );
    // A drag moves the line (with its branch) under the line it is dropped
    // on; with Ctrl it links them instead, by default the dragged line to
    // the other (Link Source To Destination).
    if let Some(d) = dropped {
        let mut ng = g.clone();
        let source = target_of(d.from);
        let done = match d.onto {
            Parent::Node(k, i) if d.link => {
                let (from, to) =
                    if drag_source_first { (source, (k, i)) } else { ((k, i), source) };
                link_lines(&mut ng, from, to)
            }
            _ if d.link => false,
            onto => {
                let moved = move_link(&mut ng, d.from.parent, d.from.pos, onto);
                if moved {
                    view.selected = Some(Row { parent: onto, pos: links(&ng, onto).len() - 1 });
                }
                moved
            }
        };
        if done {
            actions.push(replace(key, if d.link { "Link lines" } else { "Move line" }, &ng));
        }
    }
    // The Input Text popup: OK adds the line (the parent stays selected, as
    // in Aurora), Cancel adds nothing.
    if let Some(mut input) = view.input.take() {
        let prompt = &prompts[usize::from(input.parent.child_kind() == Kind::Reply)];
        let (mut ok, mut cancel) = (false, false);
        let modal = egui::Modal::new(egui::Id::new(("dlg-input", key))).show(ui.ctx(), |ui| {
            ui.set_width(340.0);
            ui.label(prompt.as_str());
            let id = egui::Id::new(("dlg-input-text", key));
            let out = egui::TextEdit::multiline(&mut input.text).id(id).desired_rows(6).show(ui);
            if !input.shown {
                // The placeholder is selected, so typing replaces it.
                let mut state = out.state.clone();
                let all = input.text.chars().count();
                state.cursor.set_char_range(Some(egui::text::CCursorRange::two(
                    egui::text::CCursor::new(0),
                    egui::text::CCursor::new(all),
                )));
                state.store(ui.ctx(), id);
                out.response.request_focus();
                input.shown = true;
            }
            ui.horizontal(|ui| {
                ok = ui.button("OK").on_hover_text("Accept changes").clicked()
                    || crate::widgets::enter(ui);
                cancel = crate::widgets::cancel_discard(ui);
            });
        });
        if ok {
            let mut ng = g.clone();
            let kind = input.parent.child_kind();
            let i = add_node(&mut ng, input.parent, "");
            if let Some(n) = ng.root.list_mut(kind.list()).and_then(|l| l.get_mut(i as usize)) {
                let t = with_english(Default::default(), input.text.trim_end());
                n.set("Text", Value::LocString(t));
            }
            let words = word_count(&ng);
            ng.root.set("NumWords", Value::Dword(words));
            if let Parent::Node(k, i) = input.parent {
                view.collapsed.remove(&(k, i));
            }
            actions.push(replace(key, "Add line", &ng));
        } else if !cancel && !modal.should_close() {
            view.input = Some(input);
        }
    }

    app.dialog_views.insert(key, view);
    app.actions.extend(actions);
}

fn highlighted(n: &Struct, hl: [bool; 5]) -> bool {
    let has = |label: &str| match n.get(label) {
        Some(Value::String(b)) => !b.is_empty(),
        Some(Value::ResRef(b)) => !b.is_empty(),
        Some(Value::Dword(v)) => *v != 0,
        _ => false,
    };
    (hl[0] && has("Comment"))
        || (hl[1] && has("Script"))
        || (hl[2] && has("Quest"))
        || (hl[3] && has("Animation"))
        || (hl[4] && has("Sound"))
}

/// What a line does besides its text, for the tree: its link's condition,
/// its action, journal update and sound, by name (`if c_has_key`, `do
/// a_give_gold`, `journal q_rats 20`, `sound vs_hello`).
pub(crate) fn markers(link: &Struct, n: &Struct) -> Vec<String> {
    let resref = |s: &Struct, l: &str| s.resref(l).filter(|r| !r.is_empty()).map(|r| r.to_string());
    let mut out = Vec::new();
    if let Some(c) = resref(link, "Active") {
        out.push(format!("if {c}"));
    }
    if let Some(a) = resref(n, "Script") {
        out.push(format!("do {a}"));
    }
    let quest = decode(n.string("Quest").unwrap_or_default());
    if !quest.is_empty() {
        let entry = n.integer("QuestEntry").unwrap_or(0);
        out.push(format!("journal {quest} {entry}"));
    }
    if let Some(s) = resref(n, "Sound") {
        out.push(format!("sound {s}"));
    }
    out
}

/// How lines look (Options › Conversation Editor).
struct Look {
    names: bool,
    /// Each line's condition, action, journal update and sound, named.
    scripts: bool,
    npc: Color32,
    pc: Color32,
}

impl Look {
    fn new(s: &crate::Settings) -> Look {
        let rgb =
            |c: Option<[u8; 3]>, d: Color32| c.map_or(d, |[r, g, b]| Color32::from_rgb(r, g, b));
        Look {
            names: !s.dialog_hide_names,
            scripts: !s.dialog_hide_scripts,
            npc: rgb(s.dialog_npc_color, Color32::from_rgb(210, 70, 70)),
            pc: rgb(s.dialog_pc_color, Color32::from_rgb(90, 140, 230)),
        }
    }
}

/// Whether a drop links (Ctrl, or Cmd on macOS) rather than moves.
fn link_key(ui: &Ui) -> bool {
    ui.input(|i| i.modifiers.ctrl || i.modifiers.mac_cmd)
}

/// Draws the rows under a parent; `path` stops cycles through owning links.
#[allow(clippy::too_many_arguments)]
fn tree(
    ui: &mut Ui,
    game: Option<&mg_rules::GameData>,
    g: &Gff,
    parent: Parent,
    depth: usize,
    view: &mut DialogView,
    path: &mut HashSet<(Kind, u32)>,
    hl: [bool; 5],
    look: &Look,
    dropped: &mut Option<Drop>,
) {
    let kind = parent.child_kind();
    for (pos, l) in links(g, parent).iter().enumerate() {
        let index = link_index(l);
        let Some(n) = node(g, kind, index) else { continue };
        let row = Row { parent, pos };
        let linked = is_link(l);
        let children = !linked && !links(g, Parent::Node(kind, index)).is_empty();
        let open = !view.collapsed.contains(&(kind, index));
        let speaker = decode(n.string("Speaker").unwrap_or_default());
        let first = line_text(game, n).lines().next().unwrap_or_default().to_string();
        let mut label = match kind {
            Kind::Entry if !look.names => first,
            Kind::Entry if speaker.is_empty() => format!("[OWNER] - {first}"),
            Kind::Entry => format!("[{speaker}] - {first}"),
            Kind::Reply if first.is_empty() && !linked => "[CONTINUE]".to_string(),
            Kind::Reply => first,
        };
        if kind == Kind::Reply && !linked && links(g, Parent::Node(kind, index)).is_empty() {
            label.push_str(" [END DIALOGUE]");
        }
        let cond = l.resref("Active").filter(|r| !r.is_empty());
        let color = if linked {
            Color32::GRAY
        } else if kind == Kind::Entry {
            look.npc
        } else {
            look.pc
        };
        let mut rich = RichText::new(label).color(color);
        if linked {
            rich = rich.italics();
        }
        if highlighted(n, hl) {
            rich = rich.background_color(Color32::from_rgba_unmultiplied(230, 200, 60, 70));
        }
        ui.horizontal(|ui| {
            ui.add_space(depth as f32 * 16.0);
            if children {
                if ui.small_button(if open { "−" } else { "+" }).clicked() {
                    if open {
                        view.collapsed.insert((kind, index));
                    } else {
                        view.collapsed.remove(&(kind, index));
                    }
                }
            } else {
                ui.add_space(18.0);
            }
            let r =
                ui.selectable_label(view.selected == Some(row), rich).interact(egui::Sense::drag());
            r.dnd_set_drag_payload(row);
            if let Some(from) = r.dnd_release_payload::<Row>()
                && *from != row
            {
                *dropped =
                    Some(Drop { from: *from, onto: Parent::Node(kind, index), link: link_key(ui) });
            }
            let r = match cond {
                Some(c) => r.on_hover_text(format!("Appears when {c} returns TRUE")),
                None => r,
            };
            if r.clicked() {
                view.selected = Some(row);
            }
            if look.scripts {
                let marks = markers(l, n);
                if !marks.is_empty() {
                    ui.label(RichText::new(marks.join("  ")).weak().small());
                }
            } else if cond.is_some() {
                ui.weak("?");
            }
        });
        if children && open && path.insert((kind, index)) {
            tree(ui, game, g, Parent::Node(kind, index), depth + 1, view, path, hl, look, dropped);
            path.remove(&(kind, index));
        }
    }
}

/// The tags of the creatures placed in the module's areas, sorted, each
/// once (the Speaker Tag list).
fn creature_tags(app: &mut Moonglow) -> Vec<String> {
    let Some(ws) = app.ws.as_mut() else { return Vec::new() };
    let areas = ws.module.areas().unwrap_or_default();
    let mut tags = std::collections::BTreeSet::new();
    for a in areas {
        let Ok(g) = ws.doc(&ResKey::new(a, mg_core::ResType::GIT)) else { continue };
        for c in g.root.list("Creature List").unwrap_or(&[]) {
            if let Some(t) = c.string("Tag").filter(|t| !t.is_empty()) {
                tags.insert(decode(t));
            }
        }
    }
    tags.into_iter().collect()
}

#[allow(clippy::too_many_arguments)]
fn text_panel(
    app: &mut Moonglow,
    ui: &mut Ui,
    key: ResKey,
    kind: Kind,
    index: u32,
    n: &Struct,
    view: &mut DialogView,
    actions: &mut Vec<Action>,
) {
    let path = node_path(kind, index);
    // Within its column, however narrow the window: a row too wide for it
    // wraps, rather than push the text under the script's side.
    let width = ui.available_width();
    ui.set_max_width(width);
    if kind == Kind::Entry {
        let tags = creature_tags(app);
        ui.horizontal_wrapped(|ui| {
            ui.label("Speaker Tag");
            let speaker = decode(n.string("Speaker").unwrap_or_default());
            let id = egui::Id::new(("dlg-speaker", key, index));
            let mut chosen = commit_text(app, ui, id, &speaker, false, 160.0);
            // The tags of the creatures placed in the module's areas.
            egui::ComboBox::from_id_salt(("dlg-speaker-tags", key, index))
                .selected_text("")
                .width(24.0)
                .show_ui(ui, |ui| {
                    for t in &tags {
                        if ui.selectable_label(*t == speaker, t).clicked() {
                            chosen = Some(t.clone());
                        }
                    }
                })
                .response
                .on_hover_text("The tag of a creature placed in the module");
            if let Some(v) = chosen {
                actions.push(set(
                    key,
                    "Speaker",
                    path.clone(),
                    "Speaker",
                    Value::String(encode(&v)),
                ));
            }
            ui.weak("(empty: the conversation's owner)");
        });
    }
    let ls = n.locstring("Text").cloned().unwrap_or_default();
    ui.horizontal(|ui| {
        ui.label("Text");
        if ui.small_button("…").on_hover_text("Edit text in multiple languages").clicked() {
            app.loc_edit = Some(LocStringEdit::new(
                FieldTarget::new(key, path.clone(), "Text"),
                "Line text",
                &ls,
            ));
        }
        if ui.small_button("Token…").on_hover_text("Insert a token such as <FirstName>").clicked()
        {
            view.token_picker = true;
        }
    });
    let english = ls
        .text(crate::text::edit_language(), mg_core::Gender::Male)
        .map(|t| t.into_owned())
        .unwrap_or_default();
    let id = egui::Id::new(("dlg-text", key, kind, index));
    // Text from the talk table shows under the field; text typed in the
    // field is the line's own, said instead.
    let from_tlk = (english.is_empty() && !ls.strref.is_none())
        .then(|| app.game.as_deref().and_then(|g| g.string(ls.strref)))
        .flatten();
    // Without text in the language edited or a talk-table string: its
    // text in another language (English first), marked as not its own.
    let other = (english.is_empty() && ls.strref.is_none())
        .then(|| ls.elsewhere(crate::text::edit_language()))
        .flatten();
    let shown = other.as_ref().map_or(english.clone(), |(_, text)| text.clone());
    let typed = match &other {
        Some((language, _)) => crate::widgets::borrowed_field(
            ui,
            language.name().unwrap_or("another language"),
            "This line's text is in another language: it has none in the language edited \
             (Options › Language). Typing here gives it text in that language",
            |ui| commit_text(app, ui, id, &shown, true, width),
        ),
        None => commit_text(app, ui, id, &shown, true, width),
    };
    if let Some(v) = typed {
        actions.push(set(
            key,
            "Line text",
            path.clone(),
            "Text",
            with_english(ls.clone(), &v).into_value(),
        ));
    }
    if let Some(t) = from_tlk {
        ui.weak(format!("From the talk table (string {}): {t}", ls.strref.0));
    }
    if view.token_picker {
        // (Those of the language edited: Options › General.)
        let tokens = crate::widgets::language_tokens(app, crate::text::edit_language());
        let mut close = false;
        egui::Window::new("Select Token")
            .pivot(egui::Align2::CENTER_CENTER)
            .default_pos(ui.ctx().content_rect().center())
            .collapsible(false)
            .show(ui.ctx(), |ui| {
                egui::ScrollArea::vertical().max_height(260.0).show(ui, |ui| {
                    for t in &tokens {
                        if ui.selectable_label(false, t).clicked() {
                            let v = crate::widgets::with_token(ui.ctx(), id, &shown, t);
                            actions.push(set(
                                key,
                                "Insert token",
                                path.clone(),
                                "Text",
                                with_english(ls.clone(), &v).into_value(),
                            ));
                            close = true;
                        }
                    }
                });
                if crate::widgets::cancel(ui) {
                    close = true;
                }
            });
        if close {
            view.token_picker = false;
        }
    }
}

/// A script field with its parameters and a preview of its source.
#[allow(clippy::too_many_arguments)]
fn script_with_params(
    app: &mut Moonglow,
    ui: &mut Ui,
    key: ResKey,
    path: GffPath,
    label: &str,
    list: &str,
    s: &Struct,
    salt: &str,
    actions: &mut Vec<Action>,
) {
    let current = s.resref(label).unwrap_or(ResRef::EMPTY);
    ui.horizontal(|ui| {
        ui.label("Script");
        let id = egui::Id::new(("dlg-script", key, salt, path.to_string()));
        if let Some(v) =
            resref_field(app, ui, id, current, "Select a script", &[ResType::NSS, ResType::NCS])
        {
            actions.push(set(key, "Script", path.clone(), label, Value::resref(v)));
        }
        if let Some(name) = crate::widgets::edit_script_button(app, ui, id, current) {
            actions.push(Action::EditScript { name, condition: label == "Active" });
        }
    });
    if ui.button("Script Wizard…").on_hover_text("Write a new script from a few choices").clicked()
    {
        app.open_script_wizard(key, path.clone(), label, salt == "cond");
    }
    // The pairs being edited live in a buffer until a field loses focus or a
    // row is added or removed; with no field focused the file's pairs show.
    let stored = params(s, list);
    let buf_id = egui::Id::new(("dlg-params-buf", key, salt, path.to_string()));
    let mut pairs: Vec<(String, String)> =
        ui.ctx().data_mut(|d| d.get_temp(buf_id)).unwrap_or_else(|| stored.clone());
    ui.label("Parameters");
    let mut commit = false;
    let mut focused = false;
    let mut remove_at = None;
    egui::Grid::new(("dlg-params", key, salt, path.to_string())).num_columns(3).show(ui, |ui| {
        for (i, (k, v)) in pairs.iter_mut().enumerate() {
            let rk = ui.add(egui::TextEdit::singleline(k).desired_width(110.0).hint_text("name"));
            let rv = ui.add(egui::TextEdit::singleline(v).desired_width(140.0).hint_text("value"));
            commit |= rk.lost_focus() || rv.lost_focus();
            focused |= rk.has_focus() || rv.has_focus();
            if ui.small_button("−").clicked() {
                remove_at = Some(i);
            }
            ui.end_row();
        }
    });
    if let Some(i) = remove_at {
        pairs.remove(i);
        commit = true;
    }
    if ui
        .small_button("+")
        .on_hover_text("Add a parameter (EE: read with GetScriptParam)")
        .clicked()
    {
        pairs.push((String::new(), String::new()));
        commit = true;
    }
    if commit {
        ui.ctx().data_mut(|d| d.remove_temp::<Vec<(String, String)>>(buf_id));
        if pairs != stored {
            actions.push(set(
                key,
                "Script parameters",
                path.clone(),
                list,
                mg_module::dialog::params_value(&pairs),
            ));
        }
    } else if focused {
        ui.ctx().data_mut(|d| d.insert_temp(buf_id, pairs));
    } else {
        ui.ctx().data_mut(|d| d.remove_temp::<Vec<(String, String)>>(buf_id));
    }
    if !current.is_empty() {
        let source = app
            .ws
            .as_ref()
            .and_then(|w| w.module.get(&ResKey::new(current, ResType::NSS)).map(decode))
            .or_else(|| {
                app.game
                    .as_ref()?
                    .resman
                    .get(&ResKey::new(current, ResType::NSS))
                    .ok()
                    .map(|d| decode(&d))
            });
        ui.separator();
        ui.label("Script Preview");
        match source {
            Some(src) => {
                egui::ScrollArea::both().id_salt(("dlg-preview", salt)).max_height(140.0).show(
                    ui,
                    |ui| {
                        ui.monospace(src);
                    },
                );
            }
            None => {
                ui.weak("No source for this script.");
            }
        }
    }
}

fn other_tab(
    app: &mut Moonglow,
    ui: &mut Ui,
    key: ResKey,
    kind: Kind,
    index: u32,
    n: &Struct,
    actions: &mut Vec<Action>,
) {
    let path = node_path(kind, index);
    egui::Grid::new(("dlg-other", key, kind, index)).num_columns(2).spacing([10.0, 6.0]).show(
        ui,
        |ui| {
            crate::widgets::field_label(ui, "Play Animation");
            let anim = n.dword("Animation").unwrap_or(0);
            let shown = ANIMATIONS
                .iter()
                .find(|a| a.1 == anim)
                .map_or_else(|| anim.to_string(), |a| a.0.to_string());
            egui::ComboBox::from_id_salt(("dlg-anim", key, kind, index))
                .selected_text(shown)
                .show_ui(ui, |ui| {
                    for (label, value) in ANIMATIONS {
                        if ui.selectable_label(value == anim, label).clicked() && value != anim {
                            actions.push(set(
                                key,
                                "Animation",
                                path.clone(),
                                "Animation",
                                Value::Dword(value),
                            ));
                        }
                    }
                });
            ui.end_row();

            crate::widgets::field_label(ui, "Play Sound");
            let sound = n.resref("Sound").unwrap_or(ResRef::EMPTY);
            let id = egui::Id::new(("dlg-sound", key, kind, index));
            ui.horizontal(|ui| {
                if let Some(v) = resref_field(app, ui, id, sound, "Select a sound", &[ResType::WAV])
                {
                    actions.push(set(key, "Sound", path.clone(), "Sound", Value::resref(v)));
                }
                if ui.add_enabled(!sound.is_empty(), egui::Button::new("Play")).clicked() {
                    app.play_sound(crate::audio::Channel::Preview, sound, 1.0, false);
                }
            });
            ui.end_row();

            // Journal categories and entries of the module.
            let journal: Vec<Struct> = app
                .ws
                .as_mut()
                .and_then(|w| w.doc(&ResKey::parse("module", ResType::JRL)?).ok())
                .map(|g| g.root.items(&jrl::CATEGORIES).to_vec())
                .unwrap_or_default();
            let quest = decode(n.string("Quest").unwrap_or_default());
            ui.label("Journal");
            egui::ComboBox::from_id_salt(("dlg-quest", key, kind, index))
                .selected_text(if quest.is_empty() { "(none)".into() } else { quest.clone() })
                .show_ui(ui, |ui| {
                    if ui.selectable_label(quest.is_empty(), "(none)").clicked()
                        && !quest.is_empty()
                    {
                        actions.push(set(
                            key,
                            "Journal",
                            path.clone(),
                            "Quest",
                            Value::String(Vec::new()),
                        ));
                    }
                    for c in &journal {
                        let tag = decode(c.read(&jrl::categories::TAG).as_bytes());
                        if ui.selectable_label(tag == quest, &tag).clicked() && tag != quest {
                            let mut edits = vec![Edit::SetField {
                                key,
                                path: path.clone(),
                                label: "Quest".into(),
                                value: Some(Value::String(encode(&tag))),
                            }];
                            let first = mg_module::journal::entries(c)
                                .first()
                                .map(|e| e.read(&jrl::categories::entry_list::ID));
                            edits.push(Edit::SetField {
                                key,
                                path: path.clone(),
                                label: "QuestEntry".into(),
                                value: Some(Value::Dword(first.unwrap_or(0))),
                            });
                            actions.push(Action::Apply(Command::new("Journal", edits)));
                        }
                    }
                });
            ui.end_row();
            if !quest.is_empty() {
                ui.label("Journal entry");
                let entry = n.dword("QuestEntry").unwrap_or(0);
                let category = journal
                    .iter()
                    .find(|c| decode(c.read(&jrl::categories::TAG).as_bytes()) == quest);
                egui::ComboBox::from_id_salt(("dlg-quest-entry", key, kind, index))
                    .selected_text(entry.to_string())
                    .show_ui(ui, |ui| {
                        for e in category.map(mg_module::journal::entries).unwrap_or(&[]) {
                            let id = e.read(&jrl::categories::entry_list::ID);
                            let t = e.read(&jrl::categories::entry_list::TEXT);
                            let t = crate::text::shown_text(&t);
                            if ui.selectable_label(id == entry, format!("{id}: {t}")).clicked()
                                && id != entry
                            {
                                actions.push(set(
                                    key,
                                    "Journal entry",
                                    path.clone(),
                                    "QuestEntry",
                                    Value::Dword(id),
                                ));
                            }
                        }
                    });
                ui.end_row();
            }
        },
    );
}

fn file_tab(app: &mut Moonglow, ui: &mut Ui, key: ResKey, g: &Gff, actions: &mut Vec<Action>) {
    egui::Grid::new(("dlg-file", key)).num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
        for (label, field) in [("Normal end", "EndConversation"), ("Aborted", "EndConverAbort")] {
            crate::widgets::field_label(ui, label);
            let current = g.root.resref(field).unwrap_or(ResRef::EMPTY);
            let id = egui::Id::new(("dlg-end", key, field));
            if let Some(v) =
                resref_field(app, ui, id, current, "Select a script", &[ResType::NSS, ResType::NCS])
            {
                actions.push(set(key, label, GffPath::root(), field, Value::resref(v)));
            }
            ui.end_row();
        }
        let mut zoom = g.root.byte("PreventZoomIn").unwrap_or(0) != 0;
        ui.label("");
        if ui.checkbox(&mut zoom, "Stop camera zoom in").changed() {
            actions.push(set(
                key,
                "Camera zoom",
                GffPath::root(),
                "PreventZoomIn",
                Value::Byte(u8::from(zoom)),
            ));
        }
        ui.end_row();
    });
}

/// A new, empty conversation's bytes.
pub(crate) fn new_file() -> Vec<u8> {
    new_dialog().to_bytes().unwrap_or_default()
}

/// The New Conversation window.
pub(crate) fn windows(app: &mut Moonglow, ui: &mut Ui) {
    let Some(mut name) = app.new_dialog.clone() else { return };
    let mut close = false;
    egui::Window::new("New Conversation")
        .pivot(egui::Align2::CENTER_CENTER)
        .default_pos(ui.ctx().content_rect().center())
        .collapsible(false)
        .resizable(false)
        .show(ui.ctx(), |ui| {
            ui.label("Name (up to 16 characters)");
            let field = ui.add(egui::TextEdit::singleline(&mut name).char_limit(16));
            crate::widgets::autofocus(ui, &field);
            let key = ResRef::from_str(name.trim())
                .ok()
                .filter(|r| !r.is_empty())
                .map(|r| ResKey::new(r, ResType::DLG));
            let exists =
                key.is_some_and(|k| app.ws.as_ref().is_some_and(|w| w.module.contains(&k)));
            if exists {
                ui.colored_label(
                    ui.visuals().error_fg_color,
                    "The module already has a conversation of this name.",
                );
            }
            ui.horizontal(|ui| {
                if (ui.add_enabled(key.is_some() && !exists, egui::Button::new("Create")).clicked()
                    || (key.is_some() && !exists && crate::widgets::enter(ui)))
                    && let Some(k) = key
                {
                    app.actions.push(Action::Apply(Command::new(
                        format!("New conversation {k}"),
                        vec![Edit::SetResource { key: k, data: Some(new_file()) }],
                    )));
                    app.actions.push(Action::OpenTab(Tab::Dialog(k)));
                    close = true;
                }
                if crate::widgets::cancel(ui) {
                    close = true;
                }
            });
        });
    app.new_dialog = if close { None } else { Some(name) };
}

fn matches(text: &str, find: &str, s: &DialogSearch) -> bool {
    let o = crate::script_tools::SearchOptions {
        match_case: s.match_case,
        whole_word: s.whole_word,
        backwards: false,
    };
    crate::script_tools::find(text, find, 0, &o).is_some()
}

/// The Search pane: find in this conversation or all of them, replace in
/// this one.
fn search_pane(
    app: &mut Moonglow,
    ui: &mut Ui,
    key: ResKey,
    g: &Gff,
    view: &mut DialogView,
    actions: &mut Vec<Action>,
) {
    let s = &mut view.search;
    egui::Grid::new(("dlg-search", key)).num_columns(2).show(ui, |ui| {
        crate::widgets::field_label(ui, "Find What");
        let field = ui.text_edit_singleline(&mut s.find);
        crate::widgets::autofocus(ui, &field);
        if std::mem::take(&mut s.focus) {
            field.request_focus();
        }
        ui.end_row();
        crate::widgets::field_label(ui, "Replace With");
        ui.text_edit_singleline(&mut s.replace);
        ui.end_row();
    });
    ui.horizontal(|ui| {
        ui.checkbox(&mut s.match_case, "Match Case");
        ui.checkbox(&mut s.whole_word, "Match Whole Word Only");
        ui.checkbox(&mut s.all_files, "All Files in Module");
    });
    let mut find = false;
    let mut replace_all = false;
    ui.horizontal(|ui| {
        find = ui.add_enabled(!s.find.is_empty(), egui::Button::new("Find")).clicked();
        find |= std::mem::take(&mut s.again) && !s.find.is_empty();
        replace_all = ui
            .add_enabled(!s.find.is_empty() && !s.all_files, egui::Button::new("Replace All"))
            .clicked();
    });
    if find {
        let keys: Vec<ResKey> = if s.all_files {
            let mut k: Vec<ResKey> = app
                .ws
                .as_ref()
                .map(|w| w.module.keys_of(ResType::DLG).copied().collect())
                .unwrap_or_default();
            k.sort();
            k
        } else {
            vec![key]
        };
        let mut results = Vec::new();
        for k in keys {
            let doc = if k == key {
                Some(g.clone())
            } else {
                app.ws.as_mut().and_then(|w| w.doc(&k).ok().cloned())
            };
            let Some(d) = doc else { continue };
            for kind in [Kind::Entry, Kind::Reply] {
                for (i, n) in mg_module::dialog::nodes(&d, kind).iter().enumerate() {
                    let t = line_text(app.game.as_deref(), n);
                    if matches(&t, &s.find, s) {
                        results.push((k, kind, i as u32, t));
                    }
                }
            }
        }
        s.results = results;
    }
    if replace_all {
        let mut ng = g.clone();
        let mut n = 0;
        for kind in [Kind::Entry, Kind::Reply] {
            if let Some(list) = ng.root.list_mut(kind.list()) {
                for line in list {
                    let t = text(line);
                    let o = crate::script_tools::SearchOptions {
                        match_case: s.match_case,
                        whole_word: s.whole_word,
                        backwards: false,
                    };
                    let (new, count) =
                        crate::script_tools::replace_all(&t, &s.find, &s.replace, &o);
                    if count > 0 {
                        let ls = line.locstring("Text").cloned().unwrap_or_default();
                        line.set("Text", with_english(ls, &new).into_value());
                        n += count;
                    }
                }
            }
        }
        if n > 0 {
            actions.push(replace(key, "Replace text", &ng));
        }
        app.log.info(format!("Replaced {n} in {key}"));
    }
    ui.separator();
    for (k, kind, index, t) in s.results.clone() {
        let label = if k == key {
            format!("{kind:?} {index}: {t}")
        } else {
            format!("{k}: {kind:?} {index}: {t}")
        };
        if ui.selectable_label(false, label).clicked() {
            if k == key {
                if let Some((parent, pos)) = mg_module::dialog::owner(g, kind, index) {
                    view.selected = Some(Row { parent, pos });
                }
            } else {
                actions.push(Action::OpenTab(Tab::Dialog(k)));
            }
        }
    }
}

/// Test mode: click through the conversation from a greeting, as a player
/// would, without evaluating conditions.
impl Moonglow {
    /// Reads a Twine or Ink story as a new conversation of the module,
    /// named after the file (made unique), and opens it; its name.
    pub fn import_conversation(&mut self, path: &std::path::Path) -> Option<ResKey> {
        use mg_module::dialog_io::Format;
        let source = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(e) => {
                self.log.error(format!("{}: {e}", path.display()));
                return None;
            }
        };
        let read = Format::of(path)
            .and_then(|f| f.read(&source))
            .unwrap_or_else(|| Err("not a Twine (.twee) or Ink (.ink) story".to_string()));
        let g = match read {
            Ok(g) => g,
            Err(e) => {
                self.log.error(format!("{}: {e}", path.display()));
                return None;
            }
        };
        let ws = self.ws.as_mut()?;
        let stem: String = path
            .file_stem()
            .map(|s| s.to_string_lossy().to_ascii_lowercase())
            .unwrap_or_default()
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
            .take(16)
            .collect();
        let stem = if stem.is_empty() { "conversation".to_string() } else { stem };
        let free =
            |name: &str| ResKey::parse(name, ResType::DLG).filter(|k| !ws.module.contains(k));
        let key = free(&stem).or_else(|| {
            (1..1000).find_map(|n| {
                let suffix = format!("{n:03}");
                let base: String = stem.chars().take(16 - suffix.len()).collect();
                free(&format!("{base}{suffix}"))
            })
        })?;
        let lines = mg_module::dialog::nodes(&g, Kind::Entry).len()
            + mg_module::dialog::nodes(&g, Kind::Reply).len();
        self.actions.push(Action::Apply(mg_edit::Command::new(
            format!("Import conversation {}", key.resref),
            vec![mg_edit::Edit::SetResource { key, data: g.to_bytes().ok() }],
        )));
        self.actions.push(Action::OpenTab(Tab::Dialog(key)));
        self.log.info(format!("Imported {} as {key}: {lines} lines", path.display()));
        Some(key)
    }
}

/// A link's condition script, if it has one.
fn condition(link: &Struct) -> Option<String> {
    link.resref("Active").filter(|r| !r.is_empty()).map(|r| r.to_string())
}

/// Whether a link's condition is taken to pass.
fn passes(link: &Struct, assume: &std::collections::BTreeMap<String, bool>) -> bool {
    condition(link).is_none_or(|c| assume.get(&c).copied().unwrap_or(true))
}

/// The NPC line the game says among `parent`'s: the first whose condition
/// passes.
pub(crate) fn npc_line(
    g: &Gff,
    parent: Parent,
    assume: &std::collections::BTreeMap<String, bool>,
) -> Option<u32> {
    links(g, parent).iter().find(|l| passes(l, assume)).map(link_index)
}

/// A condition's name as a toggle: TRUE or FALSE, clicked to switch.
fn condition_toggle(ui: &mut Ui, c: &str, assume: &mut std::collections::BTreeMap<String, bool>) {
    let on = assume.get(c).copied().unwrap_or(true);
    let text = format!("if {c}: {}", if on { "TRUE" } else { "FALSE" });
    if ui
        .small_button(text)
        .on_hover_text("What this condition is taken to return; click to switch")
        .clicked()
    {
        assume.insert(c.to_string(), !on);
    }
}

/// A line's actions and journal update (its markers but the condition).
fn effects(link: &Struct, n: &Struct) -> String {
    markers(link, n).into_iter().filter(|m| !m.starts_with("if ")).collect::<Vec<_>>().join("  ")
}

/// Test mode: the conversation played as the game plays it. The NPC says
/// the first of its lines whose condition passes; the player is offered
/// the replies whose conditions pass. Conditions are taken to return TRUE
/// until switched, and the actions and journal updates of each line are
/// shown.
pub(crate) fn test_window(app: &mut Moonglow, ui: &mut Ui) {
    let open: Vec<ResKey> =
        app.dialog_views.iter().filter(|(_, v)| v.test.is_some()).map(|(k, _)| *k).collect();
    for key in open {
        let Some(g) = app.ws.as_mut().and_then(|w| w.doc(&key).ok().cloned()) else { continue };
        let view = app.dialog_views.get_mut(&key).expect("listed");
        let game = app.game.as_deref();
        let mut path = view.test.clone().unwrap_or_default();
        let mut assume = view.assume.clone();
        let title = format!("Conversation Test: {key}");
        let mut close = crate::widgets::escape_closes(ui.ctx(), &title);
        egui::Window::new(title)
            .pivot(egui::Align2::CENTER_CENTER)
            .default_pos(ui.ctx().content_rect().center())
            .collapsible(false)
            .show(ui.ctx(), |ui| {
                // What was said so far.
                egui::ScrollArea::vertical().max_height(180.0).show(ui, |ui| {
                    for (k, i) in &path {
                        if let Some(n) = node(&g, *k, *i) {
                            let who = if *k == Kind::Entry { "NPC" } else { "You" };
                            ui.weak(format!("{who}: {}", line_text(game, n)));
                        }
                    }
                });
                ui.separator();
                // The NPC's turn: from the root, or after the last reply.
                let parent = match path.last() {
                    None => Parent::Root,
                    Some(&(k, i)) => Parent::Node(k, i),
                };
                let candidates = links(&g, parent).to_vec();
                let spoken = npc_line(&g, parent, &assume);
                // The NPC lines before the one said were passed over.
                for l in &candidates {
                    let i = link_index(l);
                    let Some(n) = node(&g, Kind::Entry, i) else { continue };
                    ui.horizontal(|ui| {
                        if Some(i) == spoken {
                            ui.label(
                                RichText::new(format!("NPC: {}", line_text(game, n))).strong(),
                            );
                        } else {
                            ui.weak(format!("(not said) {}", line_text(game, n)));
                        }
                        if let Some(c) = condition(l) {
                            condition_toggle(ui, &c, &mut assume);
                        }
                    });
                    if Some(i) == spoken {
                        let e = effects(l, n);
                        if !e.is_empty() {
                            ui.weak(e);
                        }
                        break;
                    }
                }
                let Some(entry) = spoken else {
                    ui.weak(if candidates.is_empty() {
                        "[END DIALOGUE]"
                    } else {
                        "[END DIALOGUE: no NPC line's condition passes]"
                    });
                    footer(ui, &mut path, &mut close);
                    return;
                };
                ui.separator();
                // The player's replies.
                let replies = links(&g, Parent::Node(Kind::Entry, entry)).to_vec();
                if replies.is_empty() {
                    ui.weak("[END DIALOGUE]");
                }
                // Numbered as the game numbers the ones it offers.
                let mut number = 0;
                for l in &replies {
                    let i = link_index(l);
                    let Some(n) = node(&g, Kind::Reply, i) else { continue };
                    let t = line_text(game, n);
                    let t = if t.is_empty() { "[CONTINUE]".to_string() } else { t };
                    let shown = passes(l, &assume);
                    number += usize::from(shown);
                    let label = if shown { format!("{number}. {t}") } else { t };
                    ui.horizontal(|ui| {
                        if shown {
                            if ui.button(&label).clicked() {
                                path.push((Kind::Entry, entry));
                                path.push((Kind::Reply, i));
                            }
                        } else {
                            ui.weak(format!("(hidden) {label}"));
                        }
                        if let Some(c) = condition(l) {
                            condition_toggle(ui, &c, &mut assume);
                        }
                        let e = effects(l, n);
                        if !e.is_empty() {
                            ui.weak(e);
                        }
                    });
                }
                footer(ui, &mut path, &mut close);
            });
        view.test = if close { None } else { Some(path) };
        view.assume = assume;
    }
}

/// Back (a turn: the reply and the NPC line before it) and Done.
fn footer(ui: &mut Ui, path: &mut Vec<(Kind, u32)>, close: &mut bool) {
    ui.separator();
    ui.horizontal(|ui| {
        if ui.add_enabled(!path.is_empty(), egui::Button::new("<-- Back")).clicked() {
            path.pop();
            path.pop();
        }
        if ui.button("Done").clicked() {
            *close = true;
        }
    });
}
