//! Editors shared by every object editor, as in Aurora: localized strings
//! (String Edit), scripting variables (Variables) and resource pickers
//! (Select Resource). Each opens a window; OK writes the field back through
//! one undoable command.

use egui::Ui;
use mg_core::{Gender, Language, LocString, LocStringKey, ResRef, ResType, StrRef};
use mg_edit::{Command, Edit, GffPath};
use mg_gff::{Struct, Value};
use mg_resman::ResKey;

use crate::text::{decode, encode, from_editor, to_editor};
use crate::{Action, Moonglow};

/// A field of a GFF resource that an editor window writes back to.
#[derive(Debug, Clone, PartialEq)]
pub struct FieldTarget {
    pub key: ResKey,
    pub path: GffPath,
    pub label: String,
}

impl FieldTarget {
    pub fn new(key: ResKey, path: GffPath, label: &str) -> FieldTarget {
        FieldTarget { key, path, label: label.to_string() }
    }

    fn command(&self, what: &str, value: Value) -> Action {
        Action::Apply(Command::new(
            what,
            vec![Edit::SetField {
                key: self.key,
                path: self.path.clone(),
                label: self.label.clone(),
                value: Some(value),
            }],
        ))
    }
}

/// The String Edit window: a localized string's StrRef and its texts by
/// language and gender.
#[derive(Debug, Clone, PartialEq)]
pub struct LocStringEdit {
    pub target: FieldTarget,
    pub title: String,
    pub strref: String,
    /// Language, gender, text (with plain line breaks) and whether the
    /// stored text used CRLF.
    pub entries: Vec<(Language, Gender, String, bool)>,
    original: LocString,
}

impl LocStringEdit {
    pub fn new(target: FieldTarget, title: &str, value: &LocString) -> LocStringEdit {
        let entries = value
            .strings
            .iter()
            .map(|(k, bytes)| {
                let text = k.language().codepage().decode(bytes);
                let (shown, crlf) = to_editor(&text);
                (k.language(), k.gender(), shown, crlf)
            })
            .collect();
        let strref =
            if value.strref.is_none() { String::new() } else { value.strref.0.to_string() };
        LocStringEdit { target, title: title.to_string(), strref, entries, original: value.clone() }
    }

    /// The edited string; `None` if the StrRef is not a number or a text
    /// cannot be written in its language's codepage.
    pub fn value(&self) -> Option<LocString> {
        let strref = match self.strref.trim() {
            "" => StrRef::NONE,
            s => StrRef(s.parse().ok()?),
        };
        let mut out = LocString { strref, strings: Vec::new() };
        for (language, gender, text, crlf) in &self.entries {
            let text = from_editor(text, *crlf);
            let bytes = language.codepage().encode(&text)?;
            out.strings.push((LocStringKey::new(*language, *gender), bytes.into_owned()));
        }
        Some(out)
    }
}

/// One scripting variable being edited.
#[derive(Debug, Clone, PartialEq)]
pub struct VarRow {
    pub name: String,
    /// 1 int, 2 float, 3 string.
    pub kind: u32,
    pub value: String,
    /// The stored struct, so fields the editor does not show survive.
    original: Struct,
}

/// The Variables window: a VarTable list.
#[derive(Debug, Clone, PartialEq)]
pub struct VarTableEdit {
    pub target: FieldTarget,
    pub rows: Vec<VarRow>,
}

impl VarTableEdit {
    pub fn new(target: FieldTarget, list: &[Struct]) -> VarTableEdit {
        let rows = list
            .iter()
            .map(|s| {
                let value = match s.get("Value") {
                    Some(Value::Int(v)) => v.to_string(),
                    Some(Value::Float(v)) => v.to_string(),
                    Some(Value::String(b)) => decode(b),
                    Some(Value::Dword(v)) => v.to_string(),
                    _ => String::new(),
                };
                VarRow {
                    name: decode(s.string("Name").unwrap_or_default()),
                    kind: s.dword("Type").unwrap_or(1),
                    value,
                    original: s.clone(),
                }
            })
            .collect();
        VarTableEdit { target, rows }
    }

    /// Why the table cannot be saved, if it cannot.
    pub fn problem(&self) -> Option<String> {
        for (i, r) in self.rows.iter().enumerate() {
            if r.name.trim().is_empty() {
                return Some(format!("Variable {} has no name", i + 1));
            }
            let ok = match r.kind {
                1 => r.value.trim().parse::<i32>().is_ok(),
                2 => r.value.trim().parse::<f32>().is_ok(),
                _ => true,
            };
            if !ok {
                return Some(format!("{}: {:?} is not a {}", r.name, r.value, kind_name(r.kind)));
            }
            if self.rows[..i].iter().any(|o| o.name == r.name) {
                return Some(format!("{} is defined twice", r.name));
            }
        }
        None
    }

    /// The VarTable list.
    pub fn list(&self) -> Vec<Struct> {
        self.rows
            .iter()
            .map(|r| {
                let mut s = r.original.clone();
                if s.fields.is_empty() {
                    s.id = 0;
                }
                s.set("Name", Value::String(encode(&r.name)));
                s.set("Type", Value::Dword(r.kind));
                let value = match r.kind {
                    1 => Value::Int(r.value.trim().parse().unwrap_or(0)),
                    2 => Value::Float(r.value.trim().parse().unwrap_or(0.0)),
                    _ => Value::String(encode(&r.value)),
                };
                s.set("Value", value);
                s
            })
            .collect()
    }
}

fn kind_name(kind: u32) -> &'static str {
    match kind {
        1 => "int",
        2 => "float",
        3 => "string",
        _ => "value",
    }
}

/// The Select Resource window, for a field identified by `id`.
#[derive(Debug, Clone, PartialEq)]
pub struct Picker {
    pub id: egui::Id,
    pub title: String,
    pub types: Vec<ResType>,
    pub filter: String,
    choices: Vec<ResRef>,
}

impl Moonglow {
    /// Opens the resource picker for a field; its choice arrives through
    /// [`take_pick`](Moonglow::take_pick).
    pub(crate) fn open_picker(&mut self, id: egui::Id, title: &str, types: &[ResType]) {
        let mut choices: Vec<ResRef> = Vec::new();
        if let Some(g) = &self.game {
            for t in types {
                choices.extend(g.resman.list(*t));
            }
        }
        if let Some(ws) = &self.ws {
            choices
                .extend(ws.module.keys().filter(|k| types.contains(&k.restype)).map(|k| k.resref));
        }
        choices.sort();
        choices.dedup();
        self.picker = Some(Picker {
            id,
            title: title.to_string(),
            types: types.to_vec(),
            filter: String::new(),
            choices,
        });
    }

    /// The resource picked for a field, once.
    pub(crate) fn take_pick(&mut self, id: egui::Id) -> Option<ResRef> {
        self.picked.remove(&id)
    }
}

fn window(title: &str) -> egui::Window<'_> {
    egui::Window::new(title)
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
}

const GENDERS: [(Gender, &str); 2] = [(Gender::Male, "Male"), (Gender::Female, "Female")];

pub(crate) fn ui(app: &mut Moonglow, ui: &mut Ui) {
    let ctx = ui.ctx().clone();
    let tlk_text = |app: &Moonglow, s: &str| -> Option<String> {
        let n: u32 = s.trim().parse().ok()?;
        app.game.as_ref()?.string(StrRef(n))
    };

    if let Some(edit) = app.loc_edit.clone() {
        let mut edit = edit;
        let mut close = false;
        let tlk = tlk_text(app, &edit.strref);
        window(&format!("String Edit: {}", edit.title)).show(&ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("String Ref");
                ui.add(
                    egui::TextEdit::singleline(&mut edit.strref)
                        .desired_width(120.0)
                        .hint_text("none"),
                );
            });
            if let Some(t) = &tlk {
                ui.weak(format!("Talk table: {t}"));
            }
            ui.add_space(6.0);
            let mut remove = None;
            for (i, (language, gender, text, _)) in edit.entries.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    egui::ComboBox::from_id_salt(("loc-lang", i))
                        .selected_text(language.name().unwrap_or("?"))
                        .show_ui(ui, |ui| {
                            for l in Language::EE {
                                ui.selectable_value(language, l, l.name().unwrap_or("?"));
                            }
                        });
                    egui::ComboBox::from_id_salt(("loc-gender", i))
                        .selected_text(GENDERS.iter().find(|g| g.0 == *gender).map_or("?", |g| g.1))
                        .show_ui(ui, |ui| {
                            for (g, name) in GENDERS {
                                ui.selectable_value(gender, g, name);
                            }
                        });
                    if ui.small_button("Remove").clicked() {
                        remove = Some(i);
                    }
                });
                ui.add(egui::TextEdit::multiline(text).desired_rows(3).desired_width(420.0));
            }
            if let Some(i) = remove {
                edit.entries.remove(i);
            }
            if ui.button("Add Text").clicked() {
                let free = Language::EE
                    .iter()
                    .flat_map(|l| GENDERS.iter().map(move |g| (*l, g.0)))
                    .find(|(l, g)| !edit.entries.iter().any(|e| e.0 == *l && e.1 == *g));
                if let Some((l, g)) = free {
                    edit.entries.push((l, g, String::new(), false));
                }
            }
            ui.add_space(6.0);
            let value = edit.value();
            if value.is_none() {
                ui.colored_label(
                    ui.visuals().error_fg_color,
                    "The String Ref must be a number, or empty.",
                );
            }
            ui.horizontal(|ui| {
                if ui.add_enabled(value.is_some(), egui::Button::new("OK")).clicked() {
                    if let Some(v) = value.filter(|v| *v != edit.original) {
                        app.actions.push(
                            edit.target
                                .command(&format!("Edit {}", edit.title), Value::LocString(v)),
                        );
                    }
                    close = true;
                }
                if ui.button("Cancel").clicked() {
                    close = true;
                }
            });
        });
        app.loc_edit = if close { None } else { Some(edit) };
    }

    if let Some(mut edit) = app.var_edit.clone() {
        let mut close = false;
        window("Variables").show(&ctx, |ui| {
            egui::Grid::new("vars").num_columns(4).spacing([8.0, 4.0]).show(ui, |ui| {
                ui.strong("Name");
                ui.strong("Type");
                ui.strong("Value");
                ui.end_row();
                let mut remove = None;
                for (i, r) in edit.rows.iter_mut().enumerate() {
                    ui.add(egui::TextEdit::singleline(&mut r.name).desired_width(160.0));
                    egui::ComboBox::from_id_salt(("var-type", i))
                        .selected_text(kind_name(r.kind))
                        .show_ui(ui, |ui| {
                            for k in 1..=3 {
                                ui.selectable_value(&mut r.kind, k, kind_name(k));
                            }
                        });
                    ui.add(egui::TextEdit::singleline(&mut r.value).desired_width(200.0));
                    if ui.small_button("Delete").clicked() {
                        remove = Some(i);
                    }
                    ui.end_row();
                }
                if let Some(i) = remove {
                    edit.rows.remove(i);
                }
            });
            if ui.button("Add").clicked() {
                edit.rows.push(VarRow {
                    name: String::new(),
                    kind: 1,
                    value: "0".into(),
                    original: Struct::new(0),
                });
            }
            let problem = edit.problem();
            if let Some(p) = &problem {
                ui.colored_label(ui.visuals().error_fg_color, p);
            }
            ui.horizontal(|ui| {
                if ui.add_enabled(problem.is_none(), egui::Button::new("OK")).clicked() {
                    app.actions
                        .push(edit.target.command("Edit variables", Value::List(edit.list())));
                    close = true;
                }
                if ui.button("Cancel").clicked() {
                    close = true;
                }
            });
        });
        app.var_edit = if close { None } else { Some(edit) };
    }

    if let Some(mut p) = app.picker.clone() {
        let mut close = false;
        window(&p.title).show(&ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Filter");
                ui.text_edit_singleline(&mut p.filter);
            });
            let filter = p.filter.to_ascii_lowercase();
            let shown: Vec<ResRef> = p
                .choices
                .iter()
                .filter(|r| filter.is_empty() || r.to_string().contains(&filter))
                .copied()
                .collect();
            ui.weak(format!("{} of {}", shown.len(), p.choices.len()));
            egui::Frame::group(ui.style()).show(ui, |ui| {
                let row = ui.text_style_height(&egui::TextStyle::Body) + 4.0;
                egui::ScrollArea::vertical().max_height(300.0).show_rows(
                    ui,
                    row,
                    shown.len(),
                    |ui, range| {
                        ui.set_width(260.0);
                        for r in &shown[range] {
                            if ui.selectable_label(false, r.to_string()).clicked() {
                                app.picked.insert(p.id, *r);
                                close = true;
                            }
                        }
                    },
                );
            });
            ui.horizontal(|ui| {
                if ui.button("None").on_hover_text("Clear the field").clicked() {
                    app.picked.insert(p.id, ResRef::EMPTY);
                    close = true;
                }
                if ui.button("Cancel").clicked() {
                    close = true;
                }
            });
        });
        app.picker = if close { None } else { Some(p) };
    }
}

/// A resref field with a picker button. Returns the new value when the user
/// types one (on focus loss) or picks one.
pub(crate) fn resref_field(
    app: &mut Moonglow,
    ui: &mut Ui,
    id: egui::Id,
    current: ResRef,
    title: &str,
    types: &[ResType],
) -> Option<ResRef> {
    let mut out = app.take_pick(id).filter(|r| *r != current);
    ui.horizontal(|ui| {
        let buf = app.buffers.entry(id).or_insert_with(|| current.to_string());
        let r = ui.add(egui::TextEdit::singleline(buf).id(id).desired_width(160.0).char_limit(16));
        if !r.has_focus() && !r.lost_focus() && *buf != current.to_string() {
            *buf = current.to_string();
        }
        if r.lost_focus() && *buf != current.to_string() {
            match ResRef::from_str(buf.trim()) {
                Ok(v) => out = Some(v),
                Err(e) => app.log.error(e.to_string()),
            }
        }
        if ui.small_button("…").on_hover_text(title).clicked() {
            app.open_picker(id, title, types);
        }
    });
    out
}

/// A "Variables…" button for an object's VarTable.
pub(crate) fn variables_button(
    app: &mut Moonglow,
    ui: &mut Ui,
    target: FieldTarget,
    list: &[Struct],
) {
    let label = format!("Variables ({})…", list.len());
    if ui.button(label).clicked() {
        app.var_edit = Some(VarTableEdit::new(target, list));
    }
}

/// A text field that commits when focus leaves it (one line, or several
/// with their line-end style kept). Returns the new text then.
pub(crate) fn commit_text(
    app: &mut Moonglow,
    ui: &mut Ui,
    id: egui::Id,
    current: &str,
    multiline: bool,
    width: f32,
) -> Option<String> {
    let (shown, crlf) = to_editor(current);
    let buf = app.buffers.entry(id).or_insert_with(|| shown.clone());
    let edit = if multiline {
        egui::TextEdit::multiline(buf).desired_rows(6)
    } else {
        egui::TextEdit::singleline(buf)
    };
    let r = ui.add(edit.id(id).desired_width(width));
    if !r.has_focus() && !r.lost_focus() && *buf != shown {
        // Changed underneath (undo, another selection): show the value.
        *buf = shown.clone();
    }
    (r.lost_focus() && *buf != shown).then(|| from_editor(buf, crlf))
}

/// A number field committed when it changes (a drag: when released). A
/// stored value outside `range` is shown as it is: only edits are clamped.
pub(crate) fn commit_number<T: egui::emath::Numeric>(
    ui: &mut Ui,
    current: T,
    range: std::ops::RangeInclusive<T>,
) -> Option<T> {
    let mut v = current;
    let r = ui.add(egui::DragValue::new(&mut v).range(range).clamp_existing_to_range(false));
    ((r.drag_stopped() || (r.changed() && !r.dragged())) && v != current).then_some(v)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn target() -> FieldTarget {
        FieldTarget::new(
            ResKey::parse("module", ResType::IFO).unwrap(),
            GffPath::root(),
            "VarTable",
        )
    }

    #[test]
    fn variables_round_trip_and_keep_unknown_fields() {
        let mut s = Struct::new(0);
        s.set("Name", Value::String(b"nCount".to_vec()));
        s.set("Type", Value::Dword(1));
        s.set("Value", Value::Int(3));
        s.set("Comment", Value::String(b"kept".to_vec()));
        let mut edit = VarTableEdit::new(target(), std::slice::from_ref(&s));
        assert_eq!(edit.list(), [s.clone()]);
        edit.rows[0].kind = 2;
        edit.rows[0].value = "1.5".into();
        let out = edit.list();
        assert_eq!(out[0].get("Value"), Some(&Value::Float(1.5)));
        assert_eq!(out[0].string("Comment"), Some(&b"kept"[..]));
        edit.rows[0].value = "abc".into();
        assert!(edit.problem().is_some());
    }

    #[test]
    fn locstring_edit_keeps_line_ends_and_strref() {
        let mut ls = LocString::from_text(Language::ENGLISH, Gender::Male, b"a\r\nb".to_vec());
        ls.strref = StrRef(12);
        let edit = LocStringEdit::new(target(), "Name", &ls);
        assert_eq!(edit.entries[0].2, "a\nb");
        assert_eq!(edit.value(), Some(ls));
        let mut bad = edit.clone();
        bad.strref = "x".into();
        assert_eq!(bad.value(), None);
    }
}
