//! The Journal Editor (`module.jrl`): a tree of categories (quests) and
//! their entries, Add/Copy/Cut/Paste/Delete as in Aurora, and the selected
//! node's fields. Each change is one undoable command.

use egui::Ui;
use mg_core::{ResRef, ResType};
use mg_edit::{Command, Edit, GffPath};
use mg_gff::{Struct, Value};
use mg_module::journal::{PRIORITIES, entries, list_value, new_category, new_entry, new_journal};
use mg_resman::ResKey;
use mg_schema::{StructExt, jrl};

use crate::blueprint::Form;
use crate::text::decode;
use crate::{Action, Moonglow};

/// What is selected: a category, or an entry of one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Node {
    Category(usize),
    Entry(usize, usize),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Clip {
    Category(Struct),
    Entry(Struct),
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct JournalView {
    pub selected: Option<Node>,
    pub clipboard: Option<Clip>,
    /// The categories opened out to their entries (closed to begin with,
    /// as in Aurora).
    pub open: std::collections::BTreeSet<usize>,
}

fn key() -> ResKey {
    ResKey::new(ResRef::from_str("module").expect("valid"), ResType::JRL)
}

/// A command replacing the category list (creating the journal if the
/// module has none).
fn set_categories(what: &str, exists: bool, cats: Vec<Struct>) -> Action {
    let mut edits = Vec::new();
    if !exists {
        edits.push(Edit::SetResource { key: key(), data: new_journal().to_bytes().ok() });
    }
    edits.push(Edit::SetField {
        key: key(),
        path: GffPath::root(),
        label: jrl::CATEGORIES.label.to_string(),
        value: Some(list_value(cats)),
    });
    Action::Apply(Command::new(what, edits))
}

fn set_entries(what: &str, category: usize, list: Vec<Struct>) -> Action {
    Action::Apply(Command::new(
        what,
        vec![Edit::SetField {
            key: key(),
            path: GffPath::root().item(jrl::CATEGORIES.label, category),
            label: jrl::categories::ENTRY_LIST.label.to_string(),
            value: Some(list_value(list)),
        }],
    ))
}

/// The journal editor, over its OK and Cancel (`blueprint::ok_cancel`).
pub(crate) fn ui(app: &mut Moonglow, ui: &mut Ui) {
    let targets = [(key(), mg_edit::GffPath::root())];
    let pressed = app.ws.is_some().then(|| crate::blueprint::ok_cancel(ui, &targets[0], false));
    let pending = app.actions.len();
    editor(app, ui);
    let tab = crate::Tab::Journal;
    crate::blueprint::window_drawn(app, pending, &tab, &targets, pressed.flatten());
}

fn editor(app: &mut Moonglow, ui: &mut Ui) {
    let Some(ws) = &mut app.ws else {
        ui.label("No module is open.");
        return;
    };
    let exists = ws.module.contains(&key());
    let cats: Vec<Struct> = match ws.doc(&key()) {
        Ok(g) => g.root.items(&jrl::CATEGORIES).to_vec(),
        Err(_) if !exists => Vec::new(),
        Err(e) => {
            ui.colored_label(ui.visuals().error_fg_color, e.to_string());
            return;
        }
    };
    let mut view = std::mem::take(&mut app.journal_view);
    // A selection the journal no longer has (undo) is dropped.
    let valid = |n: &Node| match *n {
        Node::Category(c) => c < cats.len(),
        Node::Entry(c, e) => c < cats.len() && e < entries(&cats[c]).len(),
    };
    view.selected = view.selected.filter(valid);
    let mut actions = Vec::new();

    ui.horizontal(|ui| {
        let can_add = !matches!(view.selected, Some(Node::Entry(..)));
        if ui
            .add_enabled(can_add, egui::Button::new("Add"))
            .on_hover_text("A category, or an entry in the selected category")
            .clicked()
        {
            match view.selected {
                Some(Node::Category(c)) => {
                    let mut list = entries(&cats[c]).to_vec();
                    list.push(new_entry(&list));
                    view.selected = Some(Node::Entry(c, list.len() - 1));
                    actions.push(set_entries("Add journal entry", c, list));
                }
                _ => {
                    let mut list = cats.clone();
                    list.push(new_category(&list));
                    view.selected = Some(Node::Category(list.len() - 1));
                    actions.push(set_categories("Add journal category", exists, list));
                }
            }
        }
        let selected = view.selected.is_some();
        let copied = |n: Node| match n {
            Node::Category(c) => Clip::Category(cats[c].clone()),
            Node::Entry(c, e) => Clip::Entry(entries(&cats[c])[e].clone()),
        };
        if ui.add_enabled(selected, egui::Button::new("Copy")).clicked() {
            view.clipboard = view.selected.map(copied);
        }
        let mut delete = false;
        if ui.add_enabled(selected, egui::Button::new("Cut")).clicked() {
            view.clipboard = view.selected.map(copied);
            delete = true;
        }
        let paste_ok = matches!(
            (&view.clipboard, view.selected),
            (Some(Clip::Category(_)), _) | (Some(Clip::Entry(_)), Some(_))
        );
        if ui.add_enabled(paste_ok, egui::Button::new("Paste")).clicked() {
            match (view.clipboard.clone(), view.selected) {
                (Some(Clip::Category(s)), _) => {
                    let mut list = cats.clone();
                    list.push(s);
                    view.selected = Some(Node::Category(list.len() - 1));
                    actions.push(set_categories("Paste journal category", exists, list));
                }
                (Some(Clip::Entry(s)), Some(Node::Category(c) | Node::Entry(c, _))) => {
                    let mut list = entries(&cats[c]).to_vec();
                    list.push(s);
                    view.selected = Some(Node::Entry(c, list.len() - 1));
                    actions.push(set_entries("Paste journal entry", c, list));
                }
                _ => {}
            }
        }
        if ui.add_enabled(selected, egui::Button::new("Delete")).clicked() {
            delete = true;
        }
        if delete {
            match view.selected {
                Some(Node::Category(c)) => {
                    let mut list = cats.clone();
                    list.remove(c);
                    actions.push(set_categories("Delete journal category", exists, list));
                }
                Some(Node::Entry(c, e)) => {
                    let mut list = entries(&cats[c]).to_vec();
                    list.remove(e);
                    actions.push(set_entries("Delete journal entry", c, list));
                }
                None => {}
            }
            view.selected = None;
        }
    });
    ui.separator();

    // (At most seven tenths of the editor: the list keeps its room.)
    let most = (ui.available_height() * 0.7).max(80.0);
    let fields = egui::Panel::bottom("journal-fields")
        .resizable(true)
        .default_size(230.0f32.min(most))
        .max_size(most);
    fields.show(ui, |ui| {
        // As tall as it was dragged to, whatever is in it: a panel is as
        // tall as its content, so the pane couldn't be dragged taller
        // than the fields chosen and went back to one line for Root.
        ui.set_min_height(ui.max_rect().height());
        // A node added this frame appears once the command has run.
        match view.selected.filter(valid) {
            Some(Node::Category(c)) => category_fields(app, ui, c, &cats[c]),
            Some(Node::Entry(c, e)) => entry_fields(app, ui, c, e, &entries(&cats[c])[e]),
            None => {
                ui.weak(
                    "Select a category or an entry. Add with nothing selected adds a category.",
                );
            }
        }
    });
    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        // As in Aurora: select Root to add categories.
        if ui.selectable_label(view.selected.is_none(), "Root").clicked() {
            view.selected = None;
        }
        if cats.is_empty() {
            ui.weak("The module has no journal categories.");
        } else {
            match crate::widgets::fold_buttons(ui, "category") {
                Some(true) => view.open = (0..cats.len()).collect(),
                Some(false) => view.open.clear(),
                None => {}
            }
        }
        // The category of the entry selected shows it (a new entry, too).
        if let Some(Node::Entry(c, _)) = view.selected {
            view.open.insert(c);
        }
        for (c, cat) in cats.iter().enumerate() {
            let name = crate::text::shown_text(&cat.read(&jrl::categories::NAME));
            let tag = decode(cat.read(&jrl::categories::TAG).as_bytes());
            let count = entries(cat).len();
            let open = view.open.contains(&c);
            ui.horizontal(|ui| {
                let arrow = if open { "⏷" } else { "⏵" };
                let hint = if open { "Close the category" } else { "Show its entries" };
                if ui.add(egui::Button::new(arrow).frame(false)).on_hover_text(hint).clicked() {
                    if open {
                        view.open.remove(&c);
                        // (Its entry, now out of sight, isn't left selected.)
                        if matches!(view.selected, Some(Node::Entry(x, _)) if x == c) {
                            view.selected = Some(Node::Category(c));
                        }
                    } else {
                        view.open.insert(c);
                    }
                }
                let head = ui.selectable_label(
                    view.selected == Some(Node::Category(c)),
                    format!("{name}  [{tag}]"),
                );
                if head.clicked() {
                    view.selected = Some(Node::Category(c));
                }
                if head.double_clicked() && !view.open.remove(&c) {
                    view.open.insert(c);
                }
                if !open {
                    ui.weak(format!("{count} {}", if count == 1 { "entry" } else { "entries" }));
                }
            });
            if !view.open.contains(&c) {
                continue;
            }
            ui.indent(("journal-cat", c), |ui| {
                for (e, entry) in entries(cat).iter().enumerate() {
                    let id = entry.read(&jrl::categories::entry_list::ID);
                    let text =
                        crate::text::shown_text(&entry.read(&jrl::categories::entry_list::TEXT));
                    let end = entry.read(&jrl::categories::entry_list::END) != 0;
                    let first = text.lines().next().unwrap_or_default();
                    let label =
                        format!("[{id:04}] {first}{}", if end { "  (finishes)" } else { "" });
                    if ui
                        .selectable_label(view.selected == Some(Node::Entry(c, e)), label)
                        .clicked()
                    {
                        view.selected = Some(Node::Entry(c, e));
                    }
                }
            });
        }
    });
    app.journal_view = view;
    app.actions.extend(actions);
}

fn category_fields(app: &mut Moonglow, ui: &mut Ui, c: usize, cat: &Struct) {
    use jrl::categories as f;
    let path = GffPath::root().item(jrl::CATEGORIES.label, c);
    // (A category's fields as a blueprint's are: `blueprint::Form`'s, their
    // typing kept, its tag of 32 characters.)
    let mut form = Form { app, key: key(), path, also: Vec::new(), root: cat.clone() };
    crate::widgets::two_columns(ui, 300.0, |ui, col| {
        if col == 0 {
            egui::Grid::new(("jrl-cat-grid", c)).num_columns(2).spacing([10.0, 6.0]).show(
                ui,
                |ui| {
                    crate::widgets::field_label(ui, "Name");
                    form.locstring(ui, "Category name", f::NAME.label);
                    ui.end_row();
                    crate::widgets::field_label(ui, "Tag");
                    form.tag(ui, "Category tag", f::TAG.label);
                    ui.end_row();
                    crate::widgets::field_label(ui, "Priority");
                    let prio = cat.read(&f::PRIORITY);
                    let all: Vec<u32> = (0..PRIORITIES.len() as u32).collect();
                    let mut pick = prio;
                    crate::widgets::dropdown(("jrl-prio", c))
                        .selected_text(PRIORITIES.get(prio as usize).copied().unwrap_or("?"))
                        .show(ui, &mut pick, &all, |p| PRIORITIES[*p as usize]);
                    if pick != prio {
                        form.set("Category priority", f::PRIORITY.label, Value::Dword(pick));
                    }
                    ui.end_row();
                    crate::widgets::field_label(ui, "XP");
                    form.number(ui, "Category XP", f::XP.label, 0..=i64::from(u32::MAX));
                    ui.end_row();
                },
            );
        } else {
            crate::widgets::field_label(ui, "Comments");
            form.memo(ui, "Category comments", f::COMMENT.label);
        }
    });
}

fn entry_fields(app: &mut Moonglow, ui: &mut Ui, c: usize, e: usize, entry: &Struct) {
    use jrl::categories::entry_list as f;
    let path =
        GffPath::root().item(jrl::CATEGORIES.label, c).item(jrl::categories::ENTRY_LIST.label, e);
    let mut form = Form { app, key: key(), path, also: Vec::new(), root: entry.clone() };
    crate::widgets::two_columns(ui, 300.0, |ui, col| {
        if col == 0 {
            egui::Grid::new(("jrl-entry-grid", c, e)).num_columns(2).spacing([10.0, 6.0]).show(
                ui,
                |ui| {
                    crate::widgets::field_label(ui, "ID");
                    form.number(ui, "Entry ID", f::ID.label, 0..=i64::from(u32::MAX));
                    ui.end_row();
                    form.check(ui, "Finish Category", f::END.label);
                    ui.end_row();
                },
            );
        } else {
            form.locstring_memo(ui, "Text", f::TEXT.label);
        }
    });
}
