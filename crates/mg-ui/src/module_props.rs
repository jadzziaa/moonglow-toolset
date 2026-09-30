//! Module Properties (module.ifo). Every change is one undoable command.

use egui::Ui;
use mg_core::{Gender, Language, LocString, ResRef, ResType};
use mg_edit::{Command, Edit, GffPath};
use mg_gff::{Struct, Value};
use mg_resman::ResKey;
use mg_schema::{ExoString, Field, GffValue, StructExt, ifo};

use crate::text::{decode, encode, from_editor, to_editor, with_english};
use crate::{Action, Moonglow};

fn info_key() -> ResKey {
    ResKey::parse("module", ResType::IFO).expect("valid")
}

/// A command setting one module-info field.
fn set<T: GffValue>(label: &str, f: &Field<T>, v: T) -> Action {
    Action::Apply(Command::new(
        label,
        vec![Edit::SetField {
            key: info_key(),
            path: GffPath::root(),
            label: f.label.to_string(),
            value: Some(v.into_value()),
        }],
    ))
}

/// A one-line text field that commits when focus leaves it.
fn text_field(app: &mut Moonglow, ui: &mut Ui, id: &str, current: &str) -> Option<String> {
    edit_field(app, ui, id, current, false)
}

/// A text field (one line or several) that commits when focus leaves it.
/// Multi-line text keeps the line-end style it had (CRLF or LF).
fn edit_field(
    app: &mut Moonglow,
    ui: &mut Ui,
    id: &str,
    current: &str,
    multiline: bool,
) -> Option<String> {
    let id = egui::Id::new(("ifo", id));
    let (shown, crlf) = to_editor(current);
    let buf = app.buffers.entry(id).or_insert_with(|| shown.clone());
    let edit = if multiline {
        egui::TextEdit::multiline(buf).desired_rows(4)
    } else {
        egui::TextEdit::singleline(buf)
    };
    let r = ui.add(edit.id(id).desired_width(420.0));
    if !r.has_focus() && !r.lost_focus() && *buf != shown {
        // Changed underneath (undo): show the document's value.
        *buf = shown.clone();
    }
    (r.lost_focus() && *buf != shown).then(|| from_editor(buf, crlf))
}

pub(crate) fn ui(app: &mut Moonglow, ui: &mut Ui) {
    let Some(ws) = &mut app.ws else {
        ui.label("No module is open.");
        return;
    };
    let root: Struct = match ws.doc(&info_key()) {
        Ok(g) => g.root.clone(),
        Err(e) => {
            ui.colored_label(ui.visuals().error_fg_color, e.to_string());
            return;
        }
    };
    let areas: Vec<ResRef> = root
        .items(&ifo::MOD_AREA_LIST)
        .iter()
        .filter_map(|a| a.try_read(&ifo::mod_area_list::AREA_NAME))
        .collect();
    egui::ScrollArea::vertical().show(ui, |ui| {
        egui::CollapsingHeader::new("Basic").default_open(true).show(ui, |ui| {
            egui::Grid::new("ifo-basic").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
                ui.label("Name");
                let name: LocString = root.read(&ifo::MOD_NAME);
                let text = name
                    .text(Language::ENGLISH, Gender::Male)
                    .map(|t| t.into_owned())
                    .unwrap_or_default();
                if let Some(v) = text_field(app, ui, "name", &text) {
                    app.actions.push(set("Module name", &ifo::MOD_NAME, with_english(name, &v)));
                }
                ui.end_row();

                ui.label("Tag");
                let tag = decode(root.read(&ifo::MOD_TAG).as_bytes());
                if let Some(v) = text_field(app, ui, "tag", &tag) {
                    app.actions.push(set("Module tag", &ifo::MOD_TAG, ExoString(encode(&v))));
                }
                ui.end_row();

                ui.label("Description");
                let desc: LocString = root.read(&ifo::MOD_DESCRIPTION);
                let text = desc
                    .text(Language::ENGLISH, Gender::Male)
                    .map(|t| t.into_owned())
                    .unwrap_or_default();
                if let Some(v) = edit_field(app, ui, "desc", &text, true) {
                    app.actions.push(set(
                        "Module description",
                        &ifo::MOD_DESCRIPTION,
                        with_english(desc, &v),
                    ));
                }
                ui.end_row();

                ui.label("Starting area");
                let entry = root.read(&ifo::MOD_ENTRY_AREA);
                egui::ComboBox::from_id_salt("entry-area")
                    .selected_text(entry.to_string())
                    .show_ui(ui, |ui| {
                        for a in &areas {
                            if ui.selectable_label(*a == entry, a.to_string()).clicked()
                                && *a != entry
                            {
                                app.actions.push(set("Starting area", &ifo::MOD_ENTRY_AREA, *a));
                            }
                        }
                    });
                ui.end_row();

                ui.label("Experience scale (%)");
                let mut xp = root.read(&ifo::MOD_XP_SCALE);
                let r = ui.add(egui::Slider::new(&mut xp, 0..=200));
                if r.drag_stopped() || (r.changed() && !r.dragged()) {
                    app.actions.push(set("Experience scale", &ifo::MOD_XP_SCALE, xp));
                }
                ui.end_row();

                ui.label("Custom TLK");
                let tlk = decode(root.read(&ifo::MOD_CUSTOM_TLK).as_bytes());
                if let Some(v) = text_field(app, ui, "tlk", &tlk) {
                    app.actions.push(set(
                        "Custom TLK",
                        &ifo::MOD_CUSTOM_TLK,
                        ExoString(encode(&v)),
                    ));
                }
                ui.end_row();
            });
        });

        egui::CollapsingHeader::new("Time").show(ui, |ui| {
            egui::Grid::new("ifo-time").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
                for (label, f, max) in [
                    ("Dawn hour", &ifo::MOD_DAWN_HOUR, 23u8),
                    ("Dusk hour", &ifo::MOD_DUSK_HOUR, 23),
                    ("Minutes per game hour", &ifo::MOD_MIN_PER_HOUR, 60),
                    ("Start hour", &ifo::MOD_START_HOUR, 23),
                    ("Start day", &ifo::MOD_START_DAY, 31),
                    ("Start month", &ifo::MOD_START_MONTH, 12),
                ] {
                    ui.label(label);
                    let mut v = root.read(f);
                    if ui.add(egui::DragValue::new(&mut v).range(0..=max)).changed() {
                        app.actions.push(set(label, f, v));
                    }
                    ui.end_row();
                }
                ui.label("Start year");
                let mut y = root.read(&ifo::MOD_START_YEAR);
                if ui.add(egui::DragValue::new(&mut y)).changed() {
                    app.actions.push(set("Start year", &ifo::MOD_START_YEAR, y));
                }
                ui.end_row();
            });
        });

        egui::CollapsingHeader::new("Events").show(ui, |ui| {
            egui::Grid::new("ifo-events").num_columns(2).spacing([12.0, 4.0]).show(ui, |ui| {
                for f in
                    root.fields.iter().filter(|f| f.label.to_string_lossy().starts_with("Mod_On"))
                {
                    let label = f.label.to_string_lossy();
                    ui.label(label.trim_start_matches("Mod_On"));
                    let current = match &f.value {
                        Value::ResRef(b) => decode(b),
                        _ => String::new(),
                    };
                    if let Some(v) = text_field(app, ui, &label, &current) {
                        match ResRef::from_str(&v) {
                            Ok(r) => app.actions.push(Action::Apply(Command::new(
                                format!("{label} script"),
                                vec![Edit::SetField {
                                    key: info_key(),
                                    path: GffPath::root(),
                                    label: label.clone(),
                                    value: Some(Value::resref(r)),
                                }],
                            ))),
                            Err(e) => app.log.error(e.to_string()),
                        }
                    }
                    ui.end_row();
                }
            });
        });

        egui::CollapsingHeader::new("Hak paks").show(ui, |ui| {
            let haks: Vec<String> = root
                .items(&ifo::MOD_HAK_LIST)
                .iter()
                .map(|h| decode(h.read(&ifo::mod_hak_list::MOD_HAK).as_bytes()))
                .collect();
            ui.label("Highest priority first. Hak changes apply when the module is reopened.");
            for (i, h) in haks.iter().enumerate() {
                ui.horizontal(|ui| {
                    ui.monospace(h);
                    let edit =
                        |edits: Vec<Edit>, label: &str| Action::Apply(Command::new(label, edits));
                    let remove = Edit::RemoveItem {
                        key: info_key(),
                        path: GffPath::root(),
                        list: ifo::MOD_HAK_LIST.label.into(),
                        index: i,
                    };
                    if i > 0 && ui.small_button("Up").clicked() {
                        let item = root.items(&ifo::MOD_HAK_LIST)[i].clone();
                        let insert = Edit::InsertItem {
                            key: info_key(),
                            path: GffPath::root(),
                            list: ifo::MOD_HAK_LIST.label.into(),
                            index: i - 1,
                            item,
                        };
                        app.actions.push(edit(vec![remove.clone(), insert], "Move hak up"));
                    }
                    if ui.small_button("Remove").clicked() {
                        app.actions.push(edit(vec![remove], "Remove hak"));
                    }
                });
            }
            let new_id = egui::Id::new("new-hak");
            let mut name = app.buffers.get(&new_id).cloned().unwrap_or_default();
            ui.horizontal(|ui| {
                ui.text_edit_singleline(&mut name);
                if ui.button("Add hak").clicked() && !name.trim().is_empty() {
                    let mut item = ifo::MOD_HAK_LIST.new_item();
                    item.write(&ifo::mod_hak_list::MOD_HAK, ExoString(encode(name.trim())));
                    app.actions.push(Action::Apply(Command::new(
                        "Add hak",
                        vec![Edit::InsertItem {
                            key: info_key(),
                            path: GffPath::root(),
                            list: ifo::MOD_HAK_LIST.label.into(),
                            index: haks.len(),
                            item,
                        }],
                    )));
                    name.clear();
                }
            });
            app.buffers.insert(new_id, name);
        });

        egui::CollapsingHeader::new("Areas").show(ui, |ui| {
            for a in &areas {
                ui.monospace(a.to_string());
            }
        });
    });
}
