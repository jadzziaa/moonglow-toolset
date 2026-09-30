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
    ANIMATIONS, Branch, Kind, Parent, add_link, add_node, copy_branch, is_link, link_index, links,
    new_dialog, node, paste_branch, remove, text,
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
        if ui
            .add_enabled(add_parent.is_some(), egui::Button::new("Add"))
            .on_hover_text("Add a line under the selection (Ctrl+A)")
            .clicked()
            && let Some(p) = add_parent
        {
            let mut ng = g.clone();
            add_node(&mut ng, p, "");
            view.selected = Some(Row { parent: p, pos: links(&ng, p).len() - 1 });
            if let Parent::Node(k, i) = p {
                view.collapsed.remove(&(k, i));
            }
            actions.push(replace(key, "Add line", &ng));
        }
        let copy = |r: Row| {
            copy_branch(&g, r.parent, r.pos).map(|b| DialogClip {
                from: key,
                branch: b,
                target: target_of(r),
            })
        };
        if ui.add_enabled(sel.is_some(), egui::Button::new("Copy")).clicked() {
            app.dialog_clip = sel.and_then(copy);
        }
        let mut delete = false;
        if ui.add_enabled(sel.is_some(), egui::Button::new("Cut")).clicked() {
            app.dialog_clip = sel.and_then(copy);
            delete = true;
        }
        let paste_parent = add_parent;
        let clip = app.dialog_clip.clone();
        let fits = |p: Parent| clip.as_ref().is_some_and(|c| c.branch.kind == p.child_kind());
        if ui.add_enabled(paste_parent.is_some_and(fits), egui::Button::new("Paste")).clicked()
            && let (Some(p), Some(c)) = (paste_parent, &clip)
        {
            let mut ng = g.clone();
            if paste_branch(&mut ng, p, &c.branch, c.from == key) {
                view.selected = Some(Row { parent: p, pos: links(&ng, p).len() - 1 });
                actions.push(replace(key, "Paste lines", &ng));
            }
        }
        let can_link = paste_parent.is_some_and(|p| p != Parent::Root && fits(p))
            && clip.as_ref().is_some_and(|c| c.from == key);
        if ui.add_enabled(can_link, egui::Button::new("Paste As Link")).clicked()
            && let (Some(p), Some(c)) = (paste_parent, &clip)
        {
            let mut ng = g.clone();
            if add_link(&mut ng, p, c.target.1) {
                actions.push(replace(key, "Paste as link", &ng));
            }
        }
        if ui.add_enabled(sel.is_some(), egui::Button::new("Delete")).clicked() {
            delete = true;
        }
        if delete && let Some(r) = sel {
            let mut ng = g.clone();
            remove(&mut ng, r.parent, r.pos);
            view.selected = None;
            actions.push(replace(key, "Delete line", &ng));
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
    });
    ui.separator();

    // The fields of the selection.
    egui::Panel::bottom(egui::Id::new(("dlg-data", key))).resizable(true).default_size(260.0).show(
        ui,
        |ui| {
            let words = g.root.dword("NumWords").unwrap_or(0);
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
                        text_panel(
                            app,
                            &mut cols[0],
                            key,
                            kind,
                            index,
                            &n,
                            &mut view,
                            &mut actions,
                        );
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
                        egui::ScrollArea::vertical().id_salt(("dlg-tab", key)).show(ui, |ui| {
                            match view.tab {
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
                                    let id =
                                        egui::Id::new(("dlg-comment", key, kind, index, label));
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
                            }
                        });
                    });
                }
            }
        },
    );

    // The tree.
    let hl = view.highlight;
    egui::ScrollArea::both().id_salt(("dlg-tree", key)).auto_shrink([false, false]).show(
        ui,
        |ui| {
            if ui
                .selectable_label(view.selected.is_none(), RichText::new("Root").strong())
                .clicked()
            {
                view.selected = None;
            }
            let mut path = HashSet::new();
            tree(ui, &g, Parent::Root, 1, &mut view, &mut path, hl);
        },
    );

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

/// Draws the rows under a parent; `path` stops cycles through owning links.
fn tree(
    ui: &mut Ui,
    g: &Gff,
    parent: Parent,
    depth: usize,
    view: &mut DialogView,
    path: &mut HashSet<(Kind, u32)>,
    hl: [bool; 5],
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
        let first = text(n).lines().next().unwrap_or_default().to_string();
        let mut label = match kind {
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
            Color32::from_rgb(210, 70, 70)
        } else {
            Color32::from_rgb(90, 140, 230)
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
            let r = ui.selectable_label(view.selected == Some(row), rich);
            let r = match cond {
                Some(c) => r.on_hover_text(format!("Appears when {c} returns TRUE")),
                None => r,
            };
            if r.clicked() {
                view.selected = Some(row);
            }
            if cond.is_some() {
                ui.weak("?");
            }
        });
        if children && open && path.insert((kind, index)) {
            tree(ui, g, Parent::Node(kind, index), depth + 1, view, path, hl);
            path.remove(&(kind, index));
        }
    }
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
    if kind == Kind::Entry {
        ui.horizontal(|ui| {
            ui.label("Speaker Tag");
            let speaker = decode(n.string("Speaker").unwrap_or_default());
            let id = egui::Id::new(("dlg-speaker", key, index));
            if let Some(v) = commit_text(app, ui, id, &speaker, false, 160.0) {
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
        .text(mg_core::Language::ENGLISH, mg_core::Gender::Male)
        .map(|t| t.into_owned())
        .unwrap_or_default();
    let id = egui::Id::new(("dlg-text", key, kind, index));
    if let Some(v) = commit_text(app, ui, id, &english, true, f32::INFINITY) {
        actions.push(set(
            key,
            "Line text",
            path.clone(),
            "Text",
            with_english(ls.clone(), &v).into_value(),
        ));
    }
    if view.token_picker {
        let tokens: Vec<String> = app
            .game
            .as_ref()
            .and_then(|g| g.table("stringtokens").ok())
            .map(|t| (0..t.len()).filter_map(|r| t.get(r, "Token").map(str::to_string)).collect())
            .unwrap_or_default();
        let mut close = false;
        egui::Window::new("Select Token").collapsible(false).show(ui.ctx(), |ui| {
            egui::ScrollArea::vertical().max_height(260.0).show(ui, |ui| {
                for t in &tokens {
                    if ui.selectable_label(false, format!("<{t}>")).clicked() {
                        let v = format!("{english}<{t}>");
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
                for (label, token) in [
                    ("Custom token", "<CUSTOM100>"),
                    ("Action highlight", "<StartAction></Start>"),
                    ("Skill check", "<StartCheck></Start>"),
                    ("Highlight", "<StartHighlight></Start>"),
                ] {
                    if ui.selectable_label(false, format!("{token}  ({label})")).clicked() {
                        let v = format!("{english}{token}");
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
            if ui.button("Cancel").clicked() {
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
        let nss = ResKey::new(current, ResType::NSS);
        let in_module = app.ws.as_ref().is_some_and(|w| w.module.contains(&nss));
        if ui.add_enabled(!current.is_empty(), egui::Button::new("Edit").small()).clicked() {
            actions.push(Action::OpenTab(if in_module {
                Tab::Script(nss)
            } else {
                Tab::Resource(nss)
            }));
        }
    });
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
            ui.label("Play Animation");
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

            ui.label("Play Sound");
            let sound = n.resref("Sound").unwrap_or(ResRef::EMPTY);
            let id = egui::Id::new(("dlg-sound", key, kind, index));
            if let Some(v) = resref_field(app, ui, id, sound, "Select a sound", &[ResType::WAV]) {
                actions.push(set(key, "Sound", path.clone(), "Sound", Value::resref(v)));
            }
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
                            let t = t
                                .text(mg_core::Language::ENGLISH, mg_core::Gender::Male)
                                .unwrap_or_default();
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
            ui.label(label);
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
    egui::Window::new("New Conversation").collapsible(false).resizable(false).show(
        ui.ctx(),
        |ui| {
            ui.label("Name (up to 16 characters)");
            ui.add(egui::TextEdit::singleline(&mut name).char_limit(16));
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
                if ui.add_enabled(key.is_some() && !exists, egui::Button::new("Create")).clicked()
                    && let Some(k) = key
                {
                    app.actions.push(Action::Apply(Command::new(
                        format!("New conversation {k}"),
                        vec![Edit::SetResource { key: k, data: Some(new_file()) }],
                    )));
                    app.actions.push(Action::OpenTab(Tab::Dialog(k)));
                    close = true;
                }
                if ui.button("Cancel").clicked() {
                    close = true;
                }
            });
        },
    );
    app.new_dialog = if close { None } else { Some(name) };
}
