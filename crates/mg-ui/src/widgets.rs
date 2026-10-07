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
    /// More structs that take the same value (several objects or
    /// blueprints edited at once): each document and path.
    pub also: Vec<(ResKey, GffPath)>,
}

impl FieldTarget {
    pub fn new(key: ResKey, path: GffPath, label: &str) -> FieldTarget {
        FieldTarget { key, path, label: label.to_string(), also: Vec::new() }
    }

    fn command(&self, what: &str, value: Value) -> Action {
        let edits = std::iter::once((self.key, &self.path))
            .chain(self.also.iter().map(|(k, p)| (*k, p)))
            .map(|(key, path)| Edit::SetField {
                key,
                path: path.clone(),
                label: self.label.clone(),
                value: Some(value.clone()),
            })
            .collect();
        Action::Apply(Command::new(what, edits))
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
    /// Each language's tokens, read when its Token… is first opened.
    tokens: Vec<(Language, Vec<String>)>,
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
        LocStringEdit {
            target,
            title: title.to_string(),
            strref,
            entries,
            original: value.clone(),
            tokens: Vec::new(),
        }
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

impl VarRow {
    /// A new variable: 1 int, 2 float, 3 string.
    pub fn new(name: &str, kind: u32, value: &str) -> VarRow {
        VarRow { name: name.to_string(), kind, value: value.to_string(), original: Struct::new(0) }
    }
}

/// A variable's name.
fn var_name(s: &Struct) -> &[u8] {
    s.string("Name").unwrap_or_default()
}

/// What an edit of one object's variables (`before` to `after`) makes of
/// another's (`theirs`): the variables added or changed are set on it (in
/// place where it has one of that name, else at the end), those deleted
/// are deleted from it, and it keeps its others as they are.
pub(crate) fn merge_variables(
    before: &[Struct],
    after: &[Struct],
    theirs: &[Struct],
) -> Vec<Struct> {
    let had = |list: &[Struct], name: &[u8]| list.iter().any(|s| var_name(s) == name);
    let changed: Vec<&Struct> = after.iter().filter(|a| !before.contains(a)).collect();
    let mut out: Vec<Struct> = theirs
        .iter()
        .filter(|s| !had(before, var_name(s)) || had(after, var_name(s)))
        .map(|s| {
            let set = changed.iter().find(|c| var_name(c) == var_name(s));
            set.map_or_else(|| s.clone(), |c| (*c).clone())
        })
        .collect();
    for c in changed {
        if !had(&out, var_name(c)) {
            out.push(c.clone());
        }
    }
    out
}

/// The Variables window: a VarTable list.
#[derive(Debug, Clone, PartialEq)]
pub struct VarTableEdit {
    pub target: FieldTarget,
    /// The list as it was when the window opened.
    original: Vec<Struct>,
    pub rows: Vec<VarRow>,
    /// Save Set's name, while it's being typed.
    pub set_name: Option<String>,
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
        VarTableEdit { target, original: list.to_vec(), rows, set_name: None }
    }

    /// For several objects edited together: the command that sets what
    /// was changed here on each, each keeping its other variables.
    fn merged(&self, app: &mut Moonglow) -> Action {
        let list = self.list();
        let t = &self.target;
        let targets = std::iter::once((t.key, &t.path)).chain(t.also.iter().map(|(k, p)| (*k, p)));
        let mut edits = Vec::new();
        for (key, path) in targets {
            let Some(Ok(doc)) = app.ws.as_mut().map(|ws| ws.doc(&key)) else { continue };
            let Some(s) = path.get(&doc.root) else { continue };
            let theirs = s.list(&t.label).unwrap_or(&[]);
            let merged = merge_variables(&self.original, &list, theirs);
            if merged != theirs {
                edits.push(Edit::SetField {
                    key,
                    path: path.clone(),
                    label: t.label.clone(),
                    value: Some(Value::List(merged)),
                });
            }
        }
        Action::Apply(Command::new("Edit variables", edits))
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

/// The Variables window's name and value fields' widths: a name of 32
/// characters shows whole.
const VAR_NAME_WIDTH: f32 = 260.0;
const VAR_VALUE_WIDTH: f32 = 220.0;
/// The room under the Variables list for its buttons and notes.
const VAR_FOOT: f32 = 96.0;

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
        app.game.as_deref()?.string(StrRef(n))
    };

    if let Some(edit) = app.loc_edit.clone() {
        let mut edit = edit;
        let mut close = false;
        let tlk = tlk_text(app, &edit.strref);
        let movable = crate::talk_view::can_add(app);
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
            if let Some((language, feminine)) = movable {
                let at = |g: Gender| edit.entries.iter().position(|e| e.0 == language && e.1 == g);
                let (male, female) = (at(Gender::Male), at(Gender::Female));
                let name = language.name().unwrap_or("?");
                if ui
                    .add_enabled(male.is_some(), egui::Button::new("Move to Talk Table"))
                    .on_hover_text(format!(
                        "Adds the {name} text as a new line of the module's talk table, and \
                         names it here by its StrRef"
                    ))
                    .clicked()
                {
                    let text = |i: usize| from_editor(&edit.entries[i].2, edit.entries[i].3);
                    let male = male.expect("enabled");
                    let line = mg_module::talk::Line {
                        text: text(male),
                        feminine: feminine.then(|| text(female.unwrap_or(male))),
                        sound: String::new(),
                        sound_length: 0.0,
                    };
                    match crate::talk_view::add_line(app, &line) {
                        Ok(strref) => {
                            edit.strref = strref.0.to_string();
                            edit.entries.retain(|e| {
                                e.0 != language || (e.1 == Gender::Female && !feminine)
                            });
                            app.log.info(format!(
                                "Added talk-table line {}; it's saved with the module",
                                strref.0
                            ));
                        }
                        Err(e) => app.log.error(format!("Move to Talk Table: {e}")),
                    }
                }
            }
            ui.add_space(6.0);
            let mut remove = None;
            let edit_tokens = &mut edit.tokens;
            for (i, (language, gender, text, _)) in edit.entries.iter_mut().enumerate() {
                let text_id = egui::Id::new(("loc-text", i));
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
                    // The language's own tokens (Polish has more than
                    // English), put where the caret was in its text.

                    let language = *language;
                    ui.menu_button("Token…", |ui| {
                        if !edit_tokens.iter().any(|(l, _)| *l == language) {
                            edit_tokens.push((language, language_tokens(app, language)));
                        }
                        let tokens =
                            &edit_tokens.iter().find(|(l, _)| *l == language).expect("read").1;
                        egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
                            for t in tokens {
                                if ui.button(t).clicked() {
                                    *text = with_token(ui.ctx(), text_id, text, t);
                                    ui.close();
                                }
                            }
                        });
                    })
                    .response
                    .on_hover_text(format!(
                        "Insert a token such as <FirstName>: those of {}",
                        language.name().unwrap_or("the language")
                    ));
                    if ui.small_button("Remove").clicked() {
                        remove = Some(i);
                    }
                });
                let field = egui::TextEdit::multiline(text).id(text_id);
                ui.add(field.desired_rows(3).desired_width(420.0));
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
                if ui.add_enabled(value.is_some(), egui::Button::new("OK")).clicked()
                    || (value.is_some() && crate::widgets::enter(ui))
                {
                    if let Some(v) = value.filter(|v| *v != edit.original) {
                        app.actions.push(
                            edit.target
                                .command(&format!("Edit {}", edit.title), Value::LocString(v)),
                        );
                    }
                    close = true;
                }
                if cancel(ui) {
                    close = true;
                }
            });
        });
        app.loc_edit = if close { None } else { Some(edit) };
    }

    if let Some(mut edit) = app.var_edit.clone() {
        let mut close = false;
        // A window to size as the list needs: its edges are dragged, and the
        // name and value fields take the width there is.
        let room = ctx.content_rect();
        egui::Window::new("Variables")
            .collapsible(false)
            .resizable(true)
            .pivot(egui::Align2::CENTER_CENTER)
            .default_pos(room.center())
            .default_size([
                (room.width() * 0.5).clamp(620.0, 900.0),
                (room.height() * 0.5).max(320.0),
            ])
            .min_size([520.0, 220.0])
            .show(&ctx, |ui| {
                // (The list scrolls, over the buttons kept in reach below it.)
                let tall = (ui.available_height() - VAR_FOOT).max(100.0);
                let spare =
                    (ui.available_width() - VAR_NAME_WIDTH - VAR_VALUE_WIDTH - 210.0).max(0.0);
                let (name_width, value_width) =
                    (VAR_NAME_WIDTH + spare * 0.45, VAR_VALUE_WIDTH + spare * 0.55);
                egui::ScrollArea::vertical().max_height(tall).auto_shrink([false, false]).show(
                    ui,
                    |ui| {
                        egui::Grid::new("vars").num_columns(4).spacing([8.0, 4.0]).show(ui, |ui| {
                            ui.strong("Name");
                            ui.strong("Type");
                            ui.strong("Value");
                            ui.end_row();
                            let mut remove = None;
                            for (i, r) in edit.rows.iter_mut().enumerate() {
                                // (A minimum: a new window offers the fields no width yet,
                                // and they would keep to it.)
                                let wide = |text, width| {
                                    egui::TextEdit::singleline(text)
                                        .desired_width(width)
                                        .min_size(egui::vec2(width, 0.0))
                                };
                                ui.add(wide(&mut r.name, name_width));
                                egui::ComboBox::from_id_salt(("var-type", i))
                                    .selected_text(kind_name(r.kind))
                                    .show_ui(ui, |ui| {
                                        for k in 1..=3 {
                                            ui.selectable_value(&mut r.kind, k, kind_name(k));
                                        }
                                    });
                                ui.add(wide(&mut r.value, value_width));
                                if ui.small_button("Delete").clicked() {
                                    remove = Some(i);
                                }
                                ui.end_row();
                            }
                            if let Some(i) = remove {
                                edit.rows.remove(i);
                            }
                        });
                    },
                );
                ui.horizontal(|ui| {
                    if ui.button("Add").clicked() {
                        edit.rows.push(VarRow::new("", 1, "0"));
                    }
                    ui.separator();
                    // Variable sets, kept in Moonglow's data folder.
                    let dir = app.var_set_dir.clone();
                    let names = crate::var_sets::list(dir.as_deref());
                    ui.add_enabled_ui(dir.is_some(), |ui| {
                        ui.menu_button("Add Set", |ui| {
                            if names.is_empty() {
                                ui.weak("No variable sets saved yet");
                            }
                            for n in &names {
                                if ui.button(n).clicked() {
                                    let dir = dir.as_deref().expect("enabled");
                                    match crate::var_sets::load(dir, n) {
                                        Ok(set) => crate::var_sets::merge(&mut edit.rows, set),
                                        Err(e) => app.log.error(format!("Add Set: {e}")),
                                    }
                                    ui.close();
                                }
                            }
                        })
                        .response
                        .on_hover_text("Add a saved set's variables (same names take its values)");
                        if edit.set_name.is_none()
                            && ui
                                .add_enabled(!edit.rows.is_empty(), egui::Button::new("Save Set…"))
                                .on_hover_text(
                                    "Keep these variables under a name, to add elsewhere",
                                )
                                .clicked()
                        {
                            edit.set_name = Some(String::new());
                        }
                    });
                });
                if let Some(name) = edit.set_name.as_mut() {
                    let mut save = false;
                    let mut cancel = false;
                    ui.horizontal(|ui| {
                        ui.label("Set name");
                        let r = ui.add(egui::TextEdit::singleline(name).desired_width(180.0));
                        crate::widgets::autofocus(ui, &r);
                        let ok = crate::prefabs::valid_name(name);
                        save = ui.add_enabled(ok, egui::Button::new("Save")).clicked()
                            || (ok
                                && r.lost_focus()
                                && ui.input(|i| i.key_pressed(egui::Key::Enter)));
                        cancel = ui.button("Cancel").clicked();
                    });
                    if save && let Some(dir) = app.var_set_dir.clone() {
                        match crate::var_sets::save(&dir, name, &edit.rows) {
                            Ok(p) => app.log.info(format!("Variable set saved: {}", p.display())),
                            Err(e) => app.log.error(format!("Save Set: {e}")),
                        }
                    }
                    if save || cancel {
                        edit.set_name = None;
                    }
                }
                let problem = edit.problem();
                if let Some(p) = &problem {
                    ui.colored_label(ui.visuals().error_fg_color, p);
                }
                if !edit.target.also.is_empty() {
                    ui.weak(format!(
                        "{} edited together, shown as the first: the variables you add or change \
                     are set on each, those you delete are deleted from each, and each keeps \
                     its others.",
                        edit.target.also.len() + 1
                    ));
                }
                ui.horizontal(|ui| {
                    if ui.add_enabled(problem.is_none(), egui::Button::new("OK")).clicked()
                        || (problem.is_none() && crate::widgets::enter(ui))
                    {
                        let action = if edit.target.also.is_empty() {
                            edit.target.command("Edit variables", Value::List(edit.list()))
                        } else {
                            edit.merged(app)
                        };
                        app.actions.push(action);
                        close = true;
                    }
                    if cancel(ui) {
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
                let field = ui.text_edit_singleline(&mut p.filter);
                crate::widgets::autofocus(ui, &field);
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
                if cancel(ui) {
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
/// `current` (a text field's value, the field `id`) with `token` put
/// where the field's caret was last, in place of what was selected: at
/// the end if the caret was never in it. The caret is left after the
/// token.
pub(crate) fn with_token(ctx: &egui::Context, id: egui::Id, current: &str, token: &str) -> String {
    use egui::text::{CCursor, CCursorRange};
    let (shown, crlf) = to_editor(current);
    let chars = shown.chars().count();
    let mut state = egui::TextEdit::load_state(ctx, id);
    let range = state
        .as_ref()
        .and_then(|s| s.cursor.char_range())
        .map(|r| {
            (r.primary.index.0.min(r.secondary.index.0), r.primary.index.0.max(r.secondary.index.0))
        })
        .filter(|&(_, to)| to <= chars)
        .unwrap_or((chars, chars));
    let byte = |at: usize| shown.char_indices().nth(at).map_or(shown.len(), |(b, _)| b);
    let out = format!("{}{token}{}", &shown[..byte(range.0)], &shown[byte(range.1)..]);
    if let Some(state) = &mut state {
        let after = CCursor::new(range.0 + token.chars().count());
        state.cursor.set_char_range(Some(CCursorRange::one(after)));
        state.clone().store(ctx, id);
    }
    from_editor(&out, crlf)
}

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
    drag_number(ui, current, |d| d.range(range))
}

/// A number field showing `current`, set up by `make` (its range, speed,
/// suffix): the number it is changed to, once: when typed, or when a drag
/// across it is let go (the whole drag is one change). The number a drag
/// has reached is kept here meanwhile, since `current` is the stored one
/// until then.
pub(crate) fn drag_number<T: egui::emath::Numeric>(
    ui: &mut Ui,
    current: T,
    make: impl for<'a> FnOnce(egui::DragValue<'a>) -> egui::DragValue<'a>,
) -> Option<T> {
    drag_number_shown(ui, current, make).0
}

/// [`drag_number`], with the number the field shows (a drag's, while it
/// goes on) and its response.
pub(crate) fn drag_number_shown<T: egui::emath::Numeric>(
    ui: &mut Ui,
    current: T,
    make: impl for<'a> FnOnce(egui::DragValue<'a>) -> egui::DragValue<'a>,
) -> (Option<T>, T, egui::Response) {
    let id = ui.next_auto_id().with("dragged-to");
    let held: Option<f64> = ui.data(|d| d.get_temp(id));
    let mut v = held.map_or(current, T::from_f64);
    let r = ui.add(make(egui::DragValue::new(&mut v).clamp_existing_to_range(false)));
    if r.dragged() {
        ui.data_mut(|d| d.insert_temp(id, v.to_f64()));
    } else if held.is_some() {
        ui.data_mut(|d| d.remove::<f64>(id));
    }
    let set = (r.drag_stopped() || (r.changed() && !r.dragged())) && v != current;
    (set.then_some(v), v, r)
}

/// Two columns side by side where each gets at least `min` points, else one
/// above the other (in a narrow tab or window, half the width would squeeze
/// fixed-width fields into the other column): `add(ui, 0)` fills the first,
/// `add(ui, 1)` the second.
pub(crate) fn two_columns(ui: &mut egui::Ui, min: f32, mut add: impl FnMut(&mut egui::Ui, usize)) {
    if ui.available_width() >= 2.0 * min + ui.spacing().item_spacing.x {
        ui.columns(2, |cols| {
            add(&mut cols[0], 0);
            add(&mut cols[1], 1);
        });
    } else {
        ui.vertical(|ui| {
            add(ui, 0);
            ui.add_space(8.0);
            add(ui, 1);
        });
    }
}

/// The dialogs open (windows over the toolset: wizards, prompts, tool
/// windows), in the order they were opened, and whether Escape was pressed
/// for the last of them this frame.
#[derive(Clone, Default)]
struct Dialogs {
    open: Vec<egui::LayerId>,
    seen: Vec<egui::LayerId>,
    escape: bool,
}

fn dialogs_id() -> egui::Id {
    egui::Id::new("moonglow-dialogs")
}

/// Once a frame, before anything is drawn: the dialogs shown last frame
/// stay above the docked windows (a click on Properties used to bury the
/// Variables window opened from it), the last opened on top; and Escape is
/// read for [`escape_closes`]. Not while a list or menu is open, which
/// Escape closes first.
pub(crate) fn dialogs_begin(ctx: &egui::Context) {
    let mut d: Dialogs = ctx.data(|x| x.get_temp(dialogs_id())).unwrap_or_default();
    let seen = std::mem::take(&mut d.seen);
    d.open.retain(|l| seen.contains(l));
    for l in seen {
        if !d.open.contains(&l) {
            d.open.push(l);
        }
    }
    for l in &d.open {
        ctx.move_to_top(*l);
    }
    d.escape = ctx.input(|i| i.key_pressed(egui::Key::Escape) && i.modifiers.is_none())
        && !egui::Popup::is_any_open(ctx);
    ctx.data_mut(|x| x.insert_temp(dialogs_id(), d));
}

/// Escape for the dialog on `layer`: true when it was pressed and this is
/// the dialog opened last. Each dialog asks every frame it shows, which is
/// also how it is known to be open.
fn escape_on(ctx: &egui::Context, layer: egui::LayerId) -> bool {
    ctx.data_mut(|x| {
        let d = x.get_temp_mut_or_default::<Dialogs>(dialogs_id());
        if !d.seen.contains(&layer) {
            d.seen.push(layer);
        }
        d.escape && d.open.last() == Some(&layer)
    })
}

/// Escape closes the window titled `title` (as its close button does):
/// true when to close it.
pub(crate) fn escape_closes(ctx: &egui::Context, title: &str) -> bool {
    // (A window's id is made from its title so: `Window::new`.)
    escape_on(ctx, egui::LayerId::new(egui::Order::Middle, egui::Id::new(Some(title))))
}

/// A window's `open` flag, cleared when Escape closes it
/// ([`escape_closes`]): for `Window::open`.
pub(crate) fn open_unless_escape<'a>(
    ctx: &egui::Context,
    title: &str,
    open: &'a mut bool,
) -> &'a mut bool {
    if escape_closes(ctx, title) {
        *open = false;
    }
    open
}

/// Escape was pressed with no dialog open (for a window of the dock to
/// close on it).
pub(crate) fn escape_past_dialogs(ctx: &egui::Context) -> bool {
    ctx.data(|x| x.get_temp::<Dialogs>(dialogs_id())).is_some_and(|d| d.escape && d.open.is_empty())
}

/// A dialog's Cancel button: clicked, or Escape pressed for its window.
pub(crate) fn cancel(ui: &mut Ui) -> bool {
    cancel_button(ui, None)
}

/// [`cancel`], saying on hover that changes are discarded.
pub(crate) fn cancel_discard(ui: &mut Ui) -> bool {
    cancel_button(ui, Some("Discard changes"))
}

fn cancel_button(ui: &mut Ui, hover: Option<&str>) -> bool {
    let mut r = ui.button("Cancel");
    if let Some(h) = hover {
        r = r.on_hover_text(h);
    }
    // (In a window of its own: not a Cancel within a docked tab.)
    let layer = ui.layer_id();
    r.clicked() || (layer.order == egui::Order::Middle && escape_on(ui.ctx(), layer))
}

/// The arrow keys on a list of choices, as in Aurora: with the list's
/// button `r` in focus (clicked, or reached with Tab), Up and Down choose
/// the row before and after `current` among `rows` (in the order the list
/// shows them), Page Up and Page Down ten away, Home and End the first
/// and last. The row chosen so.
pub(crate) fn arrow_pick(ui: &Ui, r: &egui::Response, rows: &[i64], current: i64) -> Option<i64> {
    if r.clicked() {
        r.request_focus();
    }
    if !r.has_focus() || rows.is_empty() {
        return None;
    }
    // (The arrows are the list's while it has the focus, not egui's way
    // from one widget to the next.)
    let arrows = egui::EventFilter { vertical_arrows: true, ..Default::default() };
    ui.memory_mut(|m| m.set_focus_lock_filter(r.id, arrows));
    let last = rows.len() as i64 - 1;
    let at = rows.iter().position(|&row| row == current).map(|a| a as i64);
    let to = ui.input(|i| {
        let key = |k| i.key_pressed(k) && i.modifiers.is_none();
        let step = |by: i64| Some(at.map_or(0, |a| (a + by).clamp(0, last)));
        if key(egui::Key::ArrowDown) {
            step(1)
        } else if key(egui::Key::ArrowUp) {
            step(-1)
        } else if key(egui::Key::PageDown) {
            step(10)
        } else if key(egui::Key::PageUp) {
            step(-10)
        } else if key(egui::Key::Home) {
            Some(0)
        } else if key(egui::Key::End) {
            Some(last)
        } else {
            None
        }
    })?;
    rows.get(to as usize).copied().filter(|&row| row != current)
}

/// Says on the system's clipboard what Moonglow has copied of its own
/// (objects, tiles, a conversation's lines: kept in Moonglow, not there).
/// Without text there, Windows gives Ctrl+V nothing to paste and the key
/// never reaches Moonglow: copied objects couldn't be pasted with it.
pub(crate) fn mark_clipboard(ctx: &egui::Context, what: &str) {
    ctx.copy_text(format!("Moonglow Toolset: {what}"));
}

/// The room under a gallery's picture for its name: two lines of text.
pub(crate) fn tile_label_height(ui: &Ui) -> f32 {
    2.0 * ui.text_style_height(&egui::TextStyle::Body) + 4.0
}

/// A picture over its name, in a gallery: `side` points square and its
/// name under it, on up to two lines. `made`: the picture (`Some(None)`: there
/// is nothing to draw; `None`: not made yet). Its response, and whether
/// it is in sight (to have its picture made).
pub(crate) fn picture_tile(
    ui: &mut Ui,
    side: f32,
    label: &str,
    chosen: bool,
    made: Option<Option<egui::TextureId>>,
) -> (egui::Response, bool) {
    let size = egui::vec2(side, side + tile_label_height(ui));
    let (rect, r) = ui.allocate_exact_size(size, egui::Sense::click_and_drag());
    if !ui.is_rect_visible(rect) {
        return (r, false);
    }
    let visuals = ui.style().interact_selectable(&r, chosen);
    if chosen || r.hovered() {
        ui.painter().rect_filled(rect, 3.0, visuals.weak_bg_fill);
    }
    if chosen {
        let stroke = egui::Stroke::new(1.5, ui.visuals().selection.stroke.color);
        ui.painter().rect_stroke(rect, 3.0, stroke, egui::StrokeKind::Inside);
    }
    let picture = egui::Rect::from_min_size(rect.min, egui::vec2(side, side)).shrink(2.0);
    // (The name in the text's own size, on up to two lines: in small type
    // on one, long names were hard to read and cut short.)
    let font = egui::TextStyle::Body.resolve(ui.style());
    let text = ui.visuals().text_color();
    match made {
        Some(Some(id)) => {
            let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
            ui.painter().image(id, picture, uv, egui::Color32::WHITE);
        }
        _ => {
            let mark = if made.is_none() { "…" } else { "(no picture)" };
            ui.painter().text(
                picture.center(),
                egui::Align2::CENTER_CENTER,
                mark,
                font.clone(),
                ui.visuals().weak_text_color(),
            );
        }
    }
    let name = egui::Rect::from_min_max(
        egui::pos2(rect.left() + 2.0, picture.bottom()),
        rect.max - egui::vec2(2.0, 0.0),
    );
    let mut job = egui::text::LayoutJob::simple(label.to_string(), font, text, name.width());
    job.wrap.max_rows = 2;
    job.wrap.break_anywhere = false;
    job.halign = egui::Align::Center;
    let galley = ui.painter().layout_job(job);
    ui.painter().galley(egui::pos2(name.center().x, name.top() + 1.0), galley, text);
    r.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Button, true, chosen, label));
    (r, true)
}

/// Enter in a dialog: what its main button (Create, OK, Next, Finish) does,
/// when nothing else has the keyboard: after typing in a one-line field
/// (which lets go of it on Enter), or with nothing focused. Not while a
/// multi-line field (where Enter starts a line) or a button (which Enter
/// presses) has it.
pub(crate) fn enter(ui: &egui::Ui) -> bool {
    ui.input(|i| i.key_pressed(egui::Key::Enter) && i.modifiers.is_none())
        && ui.memory(|m| m.focused().is_none())
}

/// Gives a dialog's main field the keyboard when it appears (the dialog
/// opening, a wizard reaching its page): a field shown this frame but not
/// the one before. Later clicks elsewhere keep their focus.
pub(crate) fn autofocus(ui: &egui::Ui, field: &egui::Response) {
    let seen = field.id.with("autofocus");
    let pass = ui.ctx().cumulative_pass_nr();
    let last: Option<u64> = ui.data(|d| d.get_temp(seen));
    ui.data_mut(|d| d.insert_temp(seen, pass));
    if last.is_none_or(|l| l + 1 < pass) {
        field.request_focus();
    }
}
/// The bold font family (Ubuntu Bold, the bold of egui's own Ubuntu), for
/// section headings.
const BOLD: &str = "bold";

/// Adds the bold font family to `ctx`'s fonts (once a context; it is there
/// from the next frame): Ubuntu Bold, then egui's emoji fonts for the
/// glyphs it lacks.
pub(crate) fn install_fonts(ctx: &egui::Context) {
    let done = egui::Id::new("moonglow-fonts");
    if ctx.data(|d| d.get_temp::<bool>(done)).is_some() {
        return;
    }
    ctx.data_mut(|d| d.insert_temp(done, true));
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "Ubuntu-Bold".into(),
        std::sync::Arc::new(egui::FontData::from_static(include_bytes!(
            "../fonts/Ubuntu-Bold.ttf"
        ))),
    );
    let fallbacks = fonts.families.get(&egui::FontFamily::Proportional).cloned();
    let mut bold = vec!["Ubuntu-Bold".to_string()];
    bold.extend(fallbacks.unwrap_or_default().into_iter().skip(1));
    fonts.families.insert(egui::FontFamily::Name(BOLD.into()), bold);
    ctx.set_fonts(fonts);
}

/// A form's field label (the first column of an editor's grid: "Tag",
/// "Name"…): in the strong text colour (white in the dark theme), to stand
/// apart from the values beside it; not bold, which is the headings'.
pub(crate) fn field_label(ui: &mut egui::Ui, text: impl Into<String>) -> egui::Response {
    ui.label(egui::RichText::new(text).strong())
}

/// A heading in a table's cell (a group of rows: Options › Keyboard's):
/// as a section's, without its rule and room, which a cell has no place
/// for.
pub(crate) fn table_heading(ui: &mut egui::Ui, text: impl Into<String>) -> egui::Response {
    let bold = egui::FontFamily::Name(BOLD.into());
    let size = egui::TextStyle::Body.resolve(ui.style()).size * HEADING_SCALE;
    let mut text = egui::RichText::new(text).strong().size(size);
    if ui.fonts(|f| f.families().contains(&bold)) {
        text = text.family(bold);
    }
    ui.label(text)
}

/// A form's section heading ("Lighting Scheme", "Environment"): in Ubuntu
/// Bold (headings alone are bold), the strong text colour, and larger than
/// the field labels under it; under it a rule a pixel thick, three quarters
/// as wide as the pane it is in, then a little room before what follows.
pub(crate) fn section_heading(ui: &mut egui::Ui, text: impl Into<String>) -> egui::Response {
    let heading = table_heading(ui, text);
    // (A pane with no width of its own yet: the heading's.)
    let pane = ui.available_width();
    let width = if pane.is_finite() { pane * HEADING_RULE } else { heading.rect.width() };
    let (rule, _) = ui.allocate_exact_size(egui::vec2(width, 1.0), egui::Sense::hover());
    let stroke = egui::Stroke::new(1.0, ui.visuals().widgets.noninteractive.bg_stroke.color);
    ui.painter().hline(rule.x_range(), rule.center().y, stroke);
    ui.add_space(HEADING_ROOM);
    heading
}

/// The room above a section heading that follows other content (its rule
/// is under it; no separator above).
pub(crate) const SECTION_GAP: f32 = 10.0;
/// How much larger than the body text a section heading is.
const HEADING_SCALE: f32 = 1.25;
/// How much of its pane's width a section heading's rule spans.
const HEADING_RULE: f32 = 0.75;
/// The room between a section heading's rule and what follows.
const HEADING_ROOM: f32 = 6.0;

/// The tokens text in `language` can use, each as written (`<FirstName>`):
/// the language's own table where the game has one (its
/// `lang/<code>/data/ovr/stringtokens.2da`: Polish has tokens English
/// doesn't), else the game's, then the custom token and the highlights.
pub(crate) fn language_tokens(app: &Moonglow, language: Language) -> Vec<String> {
    let column = |t: &mg_2da::TwoDa| -> Vec<String> {
        (0..t.len()).filter_map(|r| t.get(r, "Token").map(|t| format!("<{t}>"))).collect()
    };
    let own = language.short_code().zip(app.install.as_ref()).and_then(|(code, install)| {
        let path = install.root.join("lang").join(code).join("data/ovr/stringtokens.2da");
        let data = std::fs::read(path).ok()?;
        mg_2da::TwoDa::parse(&data, language.codepage()).ok()
    });
    let mut tokens = match own {
        Some(t) => column(&t),
        None => app
            .game
            .as_ref()
            .and_then(|g| g.table("stringtokens").ok())
            .map(|t| column(&t))
            .unwrap_or_default(),
    };
    tokens.extend(
        [
            "<CUSTOM100>",
            "<StartAction></Start>",
            "<StartCheck></Start>",
            "<StartHighlight></Start>",
        ]
        .map(String::from),
    );
    tokens
}

/// Expand All and Collapse All for a list of `what`s (groups, categories)
/// that open and close: `Some(true)` or `Some(false)` on the frame one is
/// clicked, to give to each header's `open`.
pub(crate) fn fold_buttons(ui: &mut Ui, what: &str) -> Option<bool> {
    let mut fold = None;
    ui.horizontal_wrapped(|ui| {
        if ui.small_button("Expand All").on_hover_text(format!("Open every {what}")).clicked() {
            fold = Some(true);
        }
        if ui.small_button("Collapse All").on_hover_text(format!("Close every {what}")).clicked() {
            fold = Some(false);
        }
    });
    fold
}

/// Edit beside a script's name, the field `field` ([`resref_field`]):
/// the name to give [`Action::EditScript`](crate::Action::EditScript) when
/// it is clicked. There for any name, as soon as one is typed (the field
/// takes it when the click leaves it), saying what it will do with one
/// that is nowhere.
pub(crate) fn edit_script_button(
    app: &Moonglow,
    ui: &mut Ui,
    field: egui::Id,
    current: ResRef,
) -> Option<ResRef> {
    let typed = app.buffers.get(&field).and_then(|b| ResRef::from_str(b.trim()).ok());
    let name = typed.unwrap_or(current);
    let key = ResKey::new(name, ResType::NSS);
    let found = app.ws.as_ref().is_some_and(|w| w.module.contains(&key))
        || app.game.as_deref().is_some_and(|g| g.resman.contains(&key));
    let r = ui.add_enabled(!name.is_empty(), egui::Button::new("Edit").small());
    let r = if found || name.is_empty() {
        r
    } else {
        r.on_hover_text(format!("There is no script '{name}': Edit creates it in the module"))
    };
    r.clicked().then_some(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Polish has tokens of its own (the game's `lang/pl` table); a
    /// language without a table has the game's.
    #[test]
    fn a_language_s_tokens_are_its_own() {
        let Some(root) = mg_testkit::nwn_root() else {
            eprintln!("skipped: no game install");
            return;
        };
        if !root.join("lang/pl/data/ovr/stringtokens.2da").is_file() {
            eprintln!("skipped: the install has no Polish");
            return;
        }
        let install = mg_resman::GameInstall::new(&root, None, "en");
        let app = Moonglow::new(Some(install), Box::new(crate::NoDialogs::default()));
        let english = language_tokens(&app, Language(0));
        let polish = language_tokens(&app, Language(5));
        let has = |list: &[String], t: &str| list.iter().any(|x| x == t);
        assert!(has(&english, "<FirstName>") && has(&polish, "<FirstName>"));
        assert!(has(&polish, "<bracie/siostro>") && !has(&english, "<bracie/siostro>"));
        assert!(has(&english, "<CUSTOM100>") && has(&polish, "<StartAction></Start>"));
        // French has no table of its own: the game's.
        assert_eq!(language_tokens(&app, Language(1)), english);
    }

    #[test]
    fn a_variable_edit_merges_into_the_others_edited_with_it() {
        let var = |name: &str, value: i32| {
            let mut s = Struct::new(0);
            s.set("Name", Value::String(name.as_bytes().to_vec()));
            s.set("Type", Value::Dword(1));
            s.set("Value", Value::Int(value));
            s
        };
        // The first object's variables, before and after the edit: MUSIC
        // changed, OLD deleted, NEW added, KEPT untouched.
        let before = [var("MUSIC", 1), var("OLD", 5), var("KEPT", 7)];
        let after = [var("MUSIC", 2), var("KEPT", 7), var("NEW", 9)];
        // Another object: its own KEPT and OWN stay as they are.
        let theirs = [var("OWN", 3), var("OLD", 6), var("MUSIC", 1), var("KEPT", 100)];
        assert_eq!(
            merge_variables(&before, &after, &theirs),
            [var("OWN", 3), var("MUSIC", 2), var("KEPT", 100), var("NEW", 9)]
        );
        // One without variables gets what was added or changed.
        assert_eq!(merge_variables(&before, &after, &[]), [var("MUSIC", 2), var("NEW", 9)]);
        // Nothing changed: nothing changes.
        assert_eq!(merge_variables(&before, &before, &theirs), theirs);
    }

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

#[cfg(test)]
mod token_tests {
    use super::with_token;
    use egui::text::{CCursor, CCursorRange};

    #[test]
    fn a_token_goes_where_the_caret_was() {
        let ctx = egui::Context::default();
        let id = egui::Id::new("field");
        // The caret never in the field: at the end.
        assert_eq!(with_token(&ctx, id, "Hello there", "<FirstName>"), "Hello there<FirstName>");
        // After "Hello ": there, and the caret after the token.
        let mut state = egui::text_edit::TextEditState::default();
        state.cursor.set_char_range(Some(CCursorRange::one(CCursor::new(6))));
        state.store(&ctx, id);
        assert_eq!(with_token(&ctx, id, "Hello there", "<FirstName>"), "Hello <FirstName>there");
        let after = egui::TextEdit::load_state(&ctx, id).unwrap().cursor.char_range().unwrap();
        assert_eq!(after.primary.index.0, 17);
        // A selection is replaced; letters of more than a byte count as one.
        let mut state = egui::text_edit::TextEditState::default();
        state.cursor.set_char_range(Some(CCursorRange::two(CCursor::new(1), CCursor::new(3))));
        state.store(&ctx, id);
        assert_eq!(with_token(&ctx, id, "żółw", "<x>"), "ż<x>w");
        // A caret past the text (the text changed underneath): the end.
        let mut state = egui::text_edit::TextEditState::default();
        state.cursor.set_char_range(Some(CCursorRange::one(CCursor::new(40))));
        state.store(&ctx, id);
        assert_eq!(with_token(&ctx, id, "ab", "<x>"), "ab<x>");
    }
}
