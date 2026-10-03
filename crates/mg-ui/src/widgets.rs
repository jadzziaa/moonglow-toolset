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

impl VarRow {
    /// A new variable: 1 int, 2 float, 3 string.
    pub fn new(name: &str, kind: u32, value: &str) -> VarRow {
        VarRow { name: name.to_string(), kind, value: value.to_string(), original: Struct::new(0) }
    }
}

/// The Variables window: a VarTable list.
#[derive(Debug, Clone, PartialEq)]
pub struct VarTableEdit {
    pub target: FieldTarget,
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
        VarTableEdit { target, rows, set_name: None }
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
                            .on_hover_text("Keep these variables under a name, to add elsewhere")
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
                        || (ok && r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)));
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
            ui.horizontal(|ui| {
                if ui.add_enabled(problem.is_none(), egui::Button::new("OK")).clicked()
                    || (problem.is_none() && crate::widgets::enter(ui))
                {
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
/// field labels.
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
/// "Name"…): bold, in the strong text colour, to stand apart from the
/// values beside it.
pub(crate) fn field_label(ui: &mut egui::Ui, text: impl Into<String>) -> egui::Response {
    let bold = egui::FontFamily::Name(BOLD.into());
    let mut text = egui::RichText::new(text).strong();
    // (Until the fonts are in, the first frame, the strong colour alone.)
    if ui.fonts(|f| f.families().contains(&bold)) {
        text = text.family(bold);
    }
    ui.label(text)
}

/// A form's section heading ("Lighting Scheme", "Environment"): bold, in
/// the strong text colour and larger than the field labels under it.
pub(crate) fn section_heading(ui: &mut egui::Ui, text: impl Into<String>) -> egui::Response {
    let bold = egui::FontFamily::Name(BOLD.into());
    let size = egui::TextStyle::Body.resolve(ui.style()).size * HEADING_SCALE;
    let mut text = egui::RichText::new(text).strong().size(size);
    if ui.fonts(|f| f.families().contains(&bold)) {
        text = text.family(bold);
    }
    ui.label(text)
}

/// How much larger than the body text a section heading is.
const HEADING_SCALE: f32 = 1.25;

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
