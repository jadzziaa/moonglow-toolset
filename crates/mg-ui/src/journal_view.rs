//! The Journal Editor (`module.jrl`): a tree of categories (quests) and
//! their entries, Add/Copy/Cut/Paste/Delete as in Aurora, and the selected
//! node's fields. Each change is one undoable command.

use egui::Ui;
use mg_core::{LocString, ResRef, ResType};
use mg_edit::{Command, Edit, GffPath};
use mg_gff::{Struct, Value};
use mg_module::journal::{PRIORITIES, entries, list_value, new_category, new_entry, new_journal};
use mg_resman::ResKey;
use mg_schema::{ExoString, GffValue, StructExt, jrl};

use crate::text::{decode, encode, with_english};
use crate::widgets::{FieldTarget, LocStringEdit, commit_number, commit_text};
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
}

fn key() -> ResKey {
    ResKey::new(ResRef::from_str("module").expect("valid"), ResType::JRL)
}

/// The text in the editing language (Options › Language).
fn english(ls: &LocString) -> String {
    crate::text::edited_text(ls)
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

fn set_field(what: &str, path: GffPath, label: &str, value: Value) -> Action {
    Action::Apply(Command::new(
        what,
        vec![Edit::SetField { key: key(), path, label: label.to_string(), value: Some(value) }],
    ))
}

pub(crate) fn ui(app: &mut Moonglow, ui: &mut Ui) {
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
    let mut view = app.journal_view.clone();
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

    egui::Panel::bottom("journal-fields").resizable(true).default_size(230.0).show(ui, |ui| {
        // A node added this frame appears once the command has run.
        match view.selected.filter(valid) {
            Some(Node::Category(c)) => category_fields(app, ui, c, &cats[c], &mut actions),
            Some(Node::Entry(c, e)) => {
                entry_fields(app, ui, c, e, &entries(&cats[c])[e], &mut actions)
            }
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
        }
        for (c, cat) in cats.iter().enumerate() {
            let name = english(&cat.read(&jrl::categories::NAME));
            let tag = decode(cat.read(&jrl::categories::TAG).as_bytes());
            let head = ui.selectable_label(
                view.selected == Some(Node::Category(c)),
                format!("{name}  [{tag}]"),
            );
            if head.clicked() {
                view.selected = Some(Node::Category(c));
            }
            ui.indent(("journal-cat", c), |ui| {
                for (e, entry) in entries(cat).iter().enumerate() {
                    let id = entry.read(&jrl::categories::entry_list::ID);
                    let text = english(&entry.read(&jrl::categories::entry_list::TEXT));
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

fn category_fields(
    app: &mut Moonglow,
    ui: &mut Ui,
    c: usize,
    cat: &Struct,
    actions: &mut Vec<Action>,
) {
    use jrl::categories as f;
    let path = GffPath::root().item(jrl::CATEGORIES.label, c);
    let id = |name: &str| egui::Id::new(("jrl-cat", c, name.to_string()));
    crate::widgets::two_columns(ui, 300.0, |ui, col| {
        if col == 0 {
            egui::Grid::new(("jrl-cat-grid", c)).num_columns(2).spacing([10.0, 6.0]).show(
                ui,
                |ui| {
                    crate::widgets::field_label(ui, "Name");
                    let name = cat.read(&f::NAME);
                    ui.horizontal(|ui| {
                        if let Some(v) =
                            commit_text(app, ui, id("name"), &english(&name), false, 200.0)
                        {
                            actions.push(set_field(
                                "Category name",
                                path.clone(),
                                f::NAME.label,
                                with_english(name.clone(), &v).into_value(),
                            ));
                        }
                        if ui
                            .small_button("…")
                            .on_hover_text("Edit text in multiple languages")
                            .clicked()
                        {
                            app.loc_edit = Some(LocStringEdit::new(
                                FieldTarget::new(key(), path.clone(), f::NAME.label),
                                "Category name",
                                &name,
                            ));
                        }
                    });
                    ui.end_row();
                    crate::widgets::field_label(ui, "Tag");
                    let tag = decode(cat.read(&f::TAG).as_bytes());
                    if let Some(v) = commit_text(app, ui, id("tag"), &tag, false, 200.0) {
                        let v: String = v.chars().take(32).collect();
                        actions.push(set_field(
                            "Category tag",
                            path.clone(),
                            f::TAG.label,
                            ExoString(encode(&v)).into_value(),
                        ));
                    }
                    ui.end_row();
                    crate::widgets::field_label(ui, "Priority");
                    let prio = cat.read(&f::PRIORITY);
                    egui::ComboBox::from_id_salt(("jrl-prio", c))
                        .selected_text(PRIORITIES.get(prio as usize).copied().unwrap_or("?"))
                        .show_ui(ui, |ui| {
                            for (i, p) in PRIORITIES.iter().enumerate() {
                                if ui.selectable_label(prio == i as u32, *p).clicked()
                                    && prio != i as u32
                                {
                                    actions.push(set_field(
                                        "Category priority",
                                        path.clone(),
                                        f::PRIORITY.label,
                                        Value::Dword(i as u32),
                                    ));
                                }
                            }
                        });
                    ui.end_row();
                    crate::widgets::field_label(ui, "XP");
                    if let Some(v) = commit_number(ui, cat.read(&f::XP), 0..=u32::MAX) {
                        actions.push(set_field(
                            "Category XP",
                            path.clone(),
                            f::XP.label,
                            Value::Dword(v),
                        ));
                    }
                    ui.end_row();
                },
            );
        } else {
            crate::widgets::field_label(ui, "Comments");
            let comment = decode(cat.read(&f::COMMENT).as_bytes());
            if let Some(v) = commit_text(app, ui, id("comment"), &comment, true, f32::INFINITY) {
                actions.push(set_field(
                    "Category comments",
                    path.clone(),
                    f::COMMENT.label,
                    ExoString(encode(&v)).into_value(),
                ));
            }
        }
    });
}

fn entry_fields(
    app: &mut Moonglow,
    ui: &mut Ui,
    c: usize,
    e: usize,
    entry: &Struct,
    actions: &mut Vec<Action>,
) {
    use jrl::categories::entry_list as f;
    let path =
        GffPath::root().item(jrl::CATEGORIES.label, c).item(jrl::categories::ENTRY_LIST.label, e);
    crate::widgets::two_columns(ui, 300.0, |ui, col| {
        if col == 0 {
            egui::Grid::new(("jrl-entry-grid", c, e)).num_columns(2).spacing([10.0, 6.0]).show(
                ui,
                |ui| {
                    crate::widgets::field_label(ui, "ID");
                    if let Some(v) = commit_number(ui, entry.read(&f::ID), 0..=u32::MAX) {
                        actions.push(set_field(
                            "Entry ID",
                            path.clone(),
                            f::ID.label,
                            Value::Dword(v),
                        ));
                    }
                    ui.end_row();
                    let mut end = entry.read(&f::END) != 0;
                    if ui.checkbox(&mut end, "Finish Category").changed() {
                        actions.push(set_field(
                            "Entry finishes the category",
                            path.clone(),
                            f::END.label,
                            Value::Word(u16::from(end)),
                        ));
                    }
                    ui.end_row();
                },
            );
        } else {
            let text = entry.read(&f::TEXT);
            ui.horizontal(|ui| {
                crate::widgets::field_label(ui, "Text");
                if ui.small_button("…").on_hover_text("Edit text in multiple languages").clicked()
                {
                    app.loc_edit = Some(LocStringEdit::new(
                        FieldTarget::new(key(), path.clone(), f::TEXT.label),
                        "Entry text",
                        &text,
                    ));
                }
            });
            let id = egui::Id::new(("jrl-entry-text", c, e));
            if let Some(v) = commit_text(app, ui, id, &english(&text), true, f32::INFINITY) {
                actions.push(set_field(
                    "Entry text",
                    path.clone(),
                    f::TEXT.label,
                    with_english(text.clone(), &v).into_value(),
                ));
            }
        }
    });
}
