//! Editors shared by every object editor, as in Aurora: localized strings
//! (String Edit), scripting variables (Variables) and resource pickers
//! (Select Resource). Each opens a window; OK writes the field back through
//! one undoable command.

use egui::Ui;
use mg_core::{Gender, Language, LocString, LocStringKey, ResRef, ResType, StrRef};
use mg_edit::{Command, Edit, GffPath};
use mg_gff::{Struct, Value};
use mg_module::var_desc::{
    Choices, Described, ModuleLookup, Owner, Unknown, Variables, type_label,
};
use mg_module::vars::VarType;
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
                let text = crate::text::entry_codepage(k.language()).decode(bytes);
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

    /// Why [`value`](Self::value) is `None`, for the window to say.
    pub fn problem(&self) -> Option<String> {
        if !self.strref.trim().is_empty() && self.strref.trim().parse::<u32>().is_err() {
            return Some("The String Ref must be a number, or empty.".into());
        }
        self.entries.iter().find_map(|(language, _, text, _)| {
            let codepage = crate::text::entry_codepage(*language);
            let lacking: String = (text.chars())
                .filter(|c| codepage.encode(&c.to_string()).is_none())
                .take(6)
                .collect();
            (!lacking.is_empty()).then(|| {
                format!(
                    "The {} text has letters its language's text cannot hold: {lacking}",
                    language.name().unwrap_or("other")
                )
            })
        })
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
            let bytes = crate::text::entry_codepage(*language).encode(&text)?;
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
    /// Its place in the stored list (`None`: new), where it is written
    /// back: the window lists by name, the file keeps its order.
    stored_at: Option<usize>,
}

impl VarRow {
    /// A new variable: 1 int, 2 float, 3 string.
    pub fn new(name: &str, kind: u32, value: &str) -> VarRow {
        VarRow {
            name: name.to_string(),
            kind,
            value: value.to_string(),
            original: Struct::new(0),
            stored_at: None,
        }
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
    /// The project's described variables (`variables.toml`), as the
    /// window offers and checks them: read when it first shows.
    pub described: Option<VarHelp>,
}

/// What the Variables window knows of the project's described variables
/// (`variables.toml`): the names it offers as one is typed, their types
/// set as one is chosen, their values to choose from, their help.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct VarHelp {
    /// What the variables are on (the module, an area, a placeable…).
    pub owner: Option<Owner>,
    pub variables: Variables,
    /// Each described variable's values, by its description's name, read
    /// when first needed (a table's rows, a script's constants).
    choices: std::collections::HashMap<String, Choices>,
    /// The rows' names at the frame before, to see one change.
    names: Vec<String>,
    /// The row whose name is being typed: the names offered under it.
    pub completing: Option<usize>,
}

impl VarHelp {
    /// The descriptions of the open module's project, for the variables
    /// of `target`.
    fn of(app: &Moonglow, target: &FieldTarget) -> VarHelp {
        let variables = (app.ws.as_ref())
            .map(|ws| ws.module.project_file.variables.clone())
            .unwrap_or_default();
        let lists: Vec<&str> = (target.path.0.iter())
            .filter_map(|s| match s {
                mg_edit::Step::Item(list, _) => Some(list.as_str()),
                mg_edit::Step::Field(_) => None,
            })
            .collect();
        let owner = Owner::of_place(target.key.restype, &lists);
        VarHelp { owner, variables, ..Default::default() }
    }

    /// The values of the variables the rows name, read where not yet.
    fn read_choices(&mut self, app: &Moonglow, rows: &[VarRow]) {
        let Some(ws) = app.ws.as_ref() else { return };
        let mut lookup = None;
        for r in rows {
            let Some(d) = self.variables.describe(&r.name) else { continue };
            if !d.has_list() || self.choices.contains_key(&d.name) {
                continue;
            }
            let rm = app.game.as_deref().map(|g| &g.resman);
            let lookup = lookup.get_or_insert_with(|| ModuleLookup::new(&ws.module, rm));
            let c = d.choices(lookup);
            self.choices.insert(d.name.clone(), c);
        }
    }

    /// What is wrong with a row by the descriptions, for the note under
    /// the list (none where nothing is, or nothing is described).
    fn note(&self, r: &VarRow) -> Option<String> {
        let file = mg_module::var_desc::FILE_NAME;
        let name = r.name.trim();
        if name.is_empty() {
            return None;
        }
        let Some(d) = self.variables.describe(name) else {
            return match self.variables.unknown(name)? {
                Unknown::OtherCase(d) => Some(format!(
                    "{name}: described as {} ({file}:{}); the game tells names apart by their case",
                    d.name, d.line
                )),
                Unknown::Undescribed { pattern, nearest } => Some(format!(
                    "{name} is named as the project's variables are ({pattern}) and described \
                     nowhere{}",
                    nearest.map(|n| format!(": {n}?")).unwrap_or_default()
                )),
            };
        };
        if d.runtime {
            return Some(format!(
                "{name} is set by the scripts while the server runs ({file}:{}): it is never to \
                 be kept in a file",
                d.line
            ));
        }
        let have = VarType::of(r.kind.into());
        if let (Some(want), Some(have)) = (d.var_type, have)
            && want != have
        {
            return Some(format!(
                "{name} is to be {} ({file}:{}): as {} the scripts read nothing",
                d.words(),
                d.line,
                type_label(have)
            ));
        }
        if let Some(o) = self.owner.filter(|o| !d.belongs_on(*o)) {
            return Some(format!(
                "{name} goes on {} ({file}:{}), not on {} {}",
                d.on.iter().map(|o| o.name()).collect::<Vec<_>>().join(", "),
                d.line,
                if o.name().starts_with(['a', 'e', 'i', 'o', 'u']) { "an" } else { "a" },
                o.name()
            ));
        }
        // (A value not yet chosen: Verify says it, the window waits.)
        if r.value.trim().is_empty() {
            return None;
        }
        let empty = Choices::default();
        let c = self.choices.get(&d.name).unwrap_or(&empty);
        d.value_problem(r.value.trim(), c).map(|p| format!("{name}: {p} ({file}:{})", d.line))
    }

    /// The described names to offer for a name being typed: those that
    /// go on this object, begin with (or hold) what is typed, and no other
    /// row has.
    fn offered(&self, typed: &str, rows: &[VarRow], row: usize) -> Vec<&Described> {
        let typed = typed.trim().to_ascii_lowercase();
        let taken = |n: &str| rows.iter().enumerate().any(|(i, r)| i != row && r.name == n);
        let mut first: Vec<&Described> = Vec::new();
        let mut then: Vec<&Described> = Vec::new();
        for d in self.variables.names_for(self.owner) {
            let lower = d.name.to_ascii_lowercase();
            if taken(&d.name) || d.name == typed {
                continue;
            }
            if lower.starts_with(&typed) {
                first.push(d);
            } else if !typed.is_empty() && lower.contains(&typed) {
                then.push(d);
            }
        }
        first.extend(then);
        first.truncate(12);
        first
    }
}

impl VarTableEdit {
    /// The window lists the variables by name (a builder's ask: they were
    /// hard to find in the order they were made), each case alike.
    pub fn new(target: FieldTarget, list: &[Struct]) -> VarTableEdit {
        let mut rows: Vec<VarRow> = list
            .iter()
            .enumerate()
            .map(|(at, s)| {
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
                    stored_at: Some(at),
                }
            })
            .collect();
        rows.sort_by_cached_key(|r| r.name.to_lowercase());
        VarTableEdit { target, original: list.to_vec(), rows, set_name: None, described: None }
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
            if let Some(t @ (VarType::Int | VarType::Float)) = VarType::of(r.kind.into())
                && let Err(e) = t.value(&r.value, crate::text::game_codepage())
            {
                return Some(format!("{}: {e}", r.name));
            }
            if self.rows[..i].iter().any(|o| o.name == r.name) {
                return Some(format!("{} is defined twice", r.name));
            }
        }
        None
    }

    /// The VarTable list: the variables in the order they were stored,
    /// those added after them (an untouched list is written as it was).
    pub fn list(&self) -> Vec<Struct> {
        let mut rows: Vec<&VarRow> = self.rows.iter().collect();
        rows.sort_by_key(|r| r.stored_at.unwrap_or(usize::MAX));
        rows.into_iter()
            .map(|r| {
                let mut s = r.original.clone();
                if s.fields.is_empty() {
                    s.id = 0;
                }
                s.set("Name", Value::String(encode(&r.name)));
                s.set("Type", Value::Dword(r.kind));
                let cp = crate::text::game_codepage();
                let value = match VarType::of(r.kind.into()) {
                    // (Text the codepage lacks as `?`, as every field takes it.)
                    Some(VarType::String) => Some(Value::String(encode(&r.value))),
                    Some(t @ (VarType::Int | VarType::Float)) => t.value(&r.value, cp).ok(),
                    // An object's or a location's, which the window doesn't
                    // edit, as it was: it wrote them as strings.
                    _ => None,
                };
                if let Some(value) = value {
                    s.set("Value", value);
                }
                s
            })
            .collect()
    }
}

/// A described variable's tip: what it is, and its help.
fn description_tip(d: &Described) -> String {
    let what = d.words();
    let what = format!("{}{}", what[..1].to_uppercase(), &what[1..]);
    match &d.help {
        Some(h) => format!("{what}.\n{h}"),
        None => format!("{what}."),
    }
}

/// A row named (anew) as a described variable takes the type it is
/// described with, where the window sets that type; a value that type
/// can't hold is emptied.
fn take_described_type(variables: &Variables, r: &mut VarRow) {
    // (Not one the scripts alone set: it is said, not taken.)
    let described = variables.describe(r.name.trim()).filter(|d| !d.runtime);
    let Some(t) = described.and_then(|d| d.var_type) else { return };
    let editable = |k: u32| VarType::of(k.into()).is_some_and(|t| t.field_type().is_some());
    if t.code() == r.kind || t.field_type().is_none() || !editable(r.kind) {
        return;
    }
    let was = r.kind;
    r.kind = t.code();
    if t != VarType::String && t.value(&r.value, crate::text::game_codepage()).is_err() {
        r.value = if t == VarType::Int { "0".into() } else { "0.0".into() };
    }
    // (A new row's 0 is an Int's, not a String's value.)
    if t == VarType::String && was == VarType::Int.code() && r.value.trim() == "0" {
        r.value.clear();
    }
}

/// The described names offered under a name being typed: the one
/// clicked is `picked` (its row and name). Whether the pointer is over
/// the list.
fn completion(
    ui: &Ui,
    row: usize,
    field: &egui::Response,
    offered: &[&Described],
    picked: &mut Option<(usize, String)>,
) -> bool {
    if offered.is_empty() {
        return false;
    }
    let area = egui::Area::new(field.id.with("described"))
        .order(egui::Order::Foreground)
        .fixed_pos(field.rect.left_bottom())
        .show(ui.ctx(), |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                ui.set_min_width(field.rect.width());
                for d in offered {
                    let what = d.words();
                    let item =
                        ui.selectable_label(false, &d.name).on_hover_text(description_tip(d));
                    ui.weak(what);
                    if item.clicked() {
                        *picked = Some((row, d.name.clone()));
                    }
                }
            });
        });
    area.response.hovered() || area.response.contains_pointer()
}

/// A described variable's value, chosen from its values (and typed, where
/// it takes others too).
fn value_choice(
    ui: &mut Ui,
    row: usize,
    d: &Described,
    c: &Choices,
    value: &mut String,
    width: f32,
) {
    let listed = c.list.iter().find(|x| x.value == *value);
    let shown = |x: &mg_module::var_desc::Choice| match &x.note {
        Some(n) => format!("{} — {n}", x.value),
        None => x.value.clone(),
    };
    if d.other {
        ui.horizontal(|ui| {
            ui.add(egui::TextEdit::singleline(value).desired_width(width - 28.0));
            ui.menu_button("▾", |ui| {
                for x in &c.list {
                    if ui.button(shown(x)).clicked() {
                        *value = x.value.clone();
                        ui.close();
                    }
                }
            })
            .response
            .on_hover_text("Its values (variables.toml)");
        });
        return;
    }
    let text = match listed {
        Some(x) => egui::RichText::new(shown(x)),
        None if value.is_empty() => egui::RichText::new("(choose)").weak(),
        None => egui::RichText::new(format!("{value} (not one of its values)"))
            .color(ui.visuals().warn_fg_color),
    };
    egui::ComboBox::from_id_salt(("var-value", row))
        .selected_text(text)
        .width(width)
        .height(320.0)
        .show_ui(ui, |ui| {
            for x in &c.list {
                ui.selectable_value(value, x.value.clone(), shown(x));
            }
        });
}

/// A variable's type as the window names it.
fn kind_name(kind: u32) -> &'static str {
    VarType::of(kind.into()).map_or("value", VarType::name)
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
/// The most notes of the project's descriptions shown under the list.
const VAR_NOTES: usize = 4;

/// A window in the middle of the app's, of a size its content gives it.
pub(crate) fn window(title: &str) -> egui::Window<'_> {
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

    if let Some(edit) = app.loc_edit.take() {
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
                let problem = edit.problem().unwrap_or_else(|| "It cannot be saved.".into());
                ui.colored_label(ui.visuals().error_fg_color, problem);
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

    if let Some(mut edit) = app.var_edit.take() {
        let mut close = false;
        // The project's described variables, and the values of those the
        // rows name.
        if edit.described.is_none() {
            edit.described = Some(VarHelp::of(app, &edit.target));
        }
        let mut help = edit.described.take().expect("just made");
        help.read_choices(app, &edit.rows);
        let described = !help.variables.is_empty();
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
                // What the project's descriptions say is wrong (not of the
                // name being typed), a line each under the list.
                let notes: Vec<String> = if described {
                    let typing = help.completing;
                    (edit.rows.iter().enumerate())
                        .filter(|(i, _)| Some(*i) != typing)
                        .filter_map(|(_, r)| help.note(r))
                        .collect()
                } else {
                    Vec::new()
                };
                let lines = notes.len().min(VAR_NOTES + 1) as f32;
                // (The list scrolls, over the buttons kept in reach below it.)
                let tall = (ui.available_height() - VAR_FOOT - lines * 18.0).max(100.0);
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
                            let mut picked = None;
                            for i in 0..edit.rows.len() {
                                // (A minimum: a new window offers the fields no width yet,
                                // and they would keep to it.)
                                fn wide(text: &mut String, width: f32) -> egui::TextEdit<'_> {
                                    egui::TextEdit::singleline(text)
                                        .desired_width(width)
                                        .min_size(egui::vec2(width, 0.0))
                                }
                                let field = wide(&mut edit.rows[i].name, name_width);
                                let name = ui.add(field.hint_text("Name"));
                                let r = &edit.rows[i];
                                let desc = help.variables.describe(r.name.trim());
                                let name = match desc {
                                    Some(d) => name.on_hover_text(description_tip(d)),
                                    None => name,
                                };
                                // The described names, offered as one is typed.
                                if described {
                                    if name.has_focus() || name.gained_focus() {
                                        help.completing = Some(i);
                                    }
                                    if help.completing == Some(i) {
                                        let offered = help.offered(&r.name, &edit.rows, i);
                                        let over = completion(ui, i, &name, &offered, &mut picked);
                                        let pressed = ui.input(|i| i.pointer.any_pressed());
                                        if offered.is_empty()
                                            || picked.is_some()
                                            || (!name.has_focus() && !over && pressed)
                                            || (name.lost_focus() && !over)
                                        {
                                            help.completing = None;
                                        }
                                    }
                                }
                                let r = &mut edit.rows[i];
                                // An object's or a location's is shown, not
                                // edited (it isn't set from text).
                                let edits = VarType::of(r.kind.into())
                                    .is_some_and(|t| t.field_type().is_some());
                                ui.add_enabled_ui(edits, |ui| {
                                    egui::ComboBox::from_id_salt(("var-type", i))
                                        .selected_text(kind_name(r.kind))
                                        .show_ui(ui, |ui| {
                                            for t in [VarType::Int, VarType::Float, VarType::String]
                                            {
                                                let (k, name) = (t.code(), t.name());
                                                ui.selectable_value(&mut r.kind, k, name);
                                            }
                                        });
                                });
                                let typed = desc
                                    .is_some_and(|d| d.var_type.is_none_or(|t| t.code() == r.kind));
                                let choices = desc.and_then(|d| help.choices.get(&d.name));
                                match (desc, choices) {
                                    (Some(d), Some(c)) if edits && typed && !c.list.is_empty() => {
                                        value_choice(ui, i, d, c, &mut r.value, value_width);
                                    }
                                    _ => {
                                        ui.add_enabled(edits, wide(&mut r.value, value_width));
                                    }
                                }
                                if ui.small_button("Delete").clicked() {
                                    remove = Some(i);
                                }
                                ui.end_row();
                            }
                            if let Some((i, name)) = picked {
                                edit.rows[i].name = name;
                            }
                            if let Some(i) = remove {
                                edit.rows.remove(i);
                                help.completing = None;
                            }
                            // A name changed to a described one: its type.
                            let names: Vec<String> =
                                edit.rows.iter().map(|r| r.name.clone()).collect();
                            if names.len() == help.names.len() {
                                for (r, was) in edit.rows.iter_mut().zip(&help.names) {
                                    if r.name != *was {
                                        take_described_type(&help.variables, r);
                                    }
                                }
                            }
                            help.names = names;
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
                        // (Enter in the name is the name's, not the window's OK.)
                        let entered = r.lost_focus()
                            && ui.input_mut(|i| {
                                i.consume_key(egui::Modifiers::NONE, egui::Key::Enter)
                            });
                        save = ui.add_enabled(ok, egui::Button::new("Save")).clicked()
                            || (ok && entered);
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
                // What the project's descriptions say is wrong (OK still
                // writes it: Verify Module says it too).
                for n in notes.iter().take(VAR_NOTES) {
                    let text = egui::RichText::new(n).color(ui.visuals().warn_fg_color);
                    ui.add(egui::Label::new(text).truncate()).on_hover_text(n);
                }
                if notes.len() > VAR_NOTES {
                    ui.weak(format!("and {} more", notes.len() - VAR_NOTES));
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
        edit.described = Some(help);
        app.var_edit = if close { None } else { Some(edit) };
    }

    if let Some(mut p) = app.picker.take() {
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
        // (A name typed is set when a click takes the field away, another
        // page's tab or its window's close button: it was lost.)
        let shown = current.to_string();
        let typed = commit_text_in(&mut app.buffers, ui, id, &shown, None, 160.0, Some(16));
        if let Some(text) = typed {
            match ResRef::from_str(text.trim()) {
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

/// Opens the Variables window on the VarTable of the struct at `path` in
/// `key` (a placed object in its area's GIT, a blueprint at its root).
pub(crate) fn open_variables(app: &mut Moonglow, key: ResKey, path: GffPath) {
    let list = (app.ws.as_mut())
        .and_then(|ws| ws.doc(&key).ok())
        .and_then(|d| path.get(&d.root))
        .and_then(|s| s.list("VarTable"))
        .unwrap_or(&[])
        .to_vec();
    app.var_edit = Some(VarTableEdit::new(FieldTarget::new(key, path, "VarTable"), &list));
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

/// Whether the text kept under `id` (a list's filter) is another than at
/// the frame before: what a filter opens, it opens when it changes, and
/// leaves to be closed again after.
pub(crate) fn text_changed(ui: &Ui, id: egui::Id, text: &str) -> bool {
    let id = id.with("was");
    let was: Option<String> = ui.data(|d| d.get_temp(id));
    if was.as_deref() == Some(text) {
        return false;
    }
    ui.data_mut(|d| d.insert_temp(id, text.to_string()));
    // (At the first look at it, if there is any: a filter that is there
    // when its list first shows opens what it finds.)
    was.is_some() || !text.is_empty()
}

/// Home and End for a list that scrolls: with the pointer over it and no
/// text being typed, they go to its top and its bottom.
pub(crate) fn home_and_end<R>(ui: &Ui, list: &mut egui::scroll_area::ScrollAreaOutput<R>) {
    if !ui.rect_contains_pointer(list.inner_rect) || ui.ctx().egui_wants_keyboard_input() {
        return;
    }
    let (home, end) = ui.input(|i| (i.key_pressed(egui::Key::Home), i.key_pressed(egui::Key::End)));
    if home || end {
        let bottom = (list.content_size.y - list.inner_rect.height()).max(0.0);
        list.state.offset.y = if home { 0.0 } else { bottom };
        list.state.store(ui.ctx(), list.id);
        ui.ctx().request_repaint();
    }
}

/// What a field shows of a localized string, and where that is from if
/// it is not the string's own text in the language edited: the talk
/// table (its StrRef, as Aurora shows the game's blueprints), else
/// another language (English first, as Aurora shows a name written in
/// English alone). With the source comes why the text looks as it does.
pub(crate) fn loc_shown(
    app: &Moonglow,
    ls: &mg_core::LocString,
) -> (String, Option<(String, &'static str)>) {
    let own = crate::text::edited_text(ls);
    if !own.is_empty() {
        return (own, None);
    }
    let table = (!ls.strref.is_none())
        .then(|| app.game.as_deref().and_then(|g| g.string(ls.strref)))
        .flatten()
        .filter(|t| !t.is_empty());
    if let Some(text) = table {
        let why = "This text is the talk table's, not its own: typing here gives it text of \
                   its own in its place";
        return (text, Some((format!("StrRef {}", ls.strref.0), why)));
    }
    match crate::text::elsewhere(ls) {
        Some((language, text)) => {
            let why = "This text is in another language: there is none in the language edited \
                       (Options › Language). Typing here gives it text in that language; … has \
                       every language's";
            (text, Some((language.name().unwrap_or("another language").to_string(), why)))
        }
        // (A StrRef the talk table lacks is still named.)
        None if !ls.strref.is_none() => (
            String::new(),
            Some((format!("StrRef {}", ls.strref.0), "The talk table has no such string")),
        ),
        None => (String::new(), None),
    }
}

/// A text field (`field` draws it) whose text is not the string's own
/// ([`loc_shown`]): in the talk table's color, with where it is from
/// beside it.
pub(crate) fn borrowed_field<R>(
    ui: &mut Ui,
    from: &str,
    why: &str,
    field: impl FnOnce(&mut Ui) -> R,
) -> R {
    let color = crate::blueprint::talk_table_color(ui);
    let out = ui
        .scope(|ui| {
            ui.visuals_mut().override_text_color = Some(color);
            field(ui)
        })
        .inner;
    ui.colored_label(color, from).on_hover_text(why);
    out
}

/// A text field of a localized string as [`commit_text`] makes one,
/// showing what [`loc_shown`] says and marked where that is not its own.
pub(crate) fn loc_text(
    app: &mut Moonglow,
    ui: &mut Ui,
    id: egui::Id,
    ls: &mg_core::LocString,
    multiline: bool,
    width: f32,
) -> Option<String> {
    match loc_shown(app, ls) {
        (shown, None) => commit_text(app, ui, id, &shown, multiline, width),
        (shown, Some((from, why))) => {
            borrowed_field(ui, &from, why, |ui| commit_text(app, ui, id, &shown, multiline, width))
        }
    }
}

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

/// Whether a widget has the keyboard, the window having it or not. (A
/// response's own `has_focus` is false while another program has the
/// keyboard: a field that takes that for "left" throws away what was
/// typed in it when its window is left for another, by Alt + Tab.)
pub(crate) fn has_keys(r: &egui::Response) -> bool {
    r.ctx.memory(|m| m.has_focus(r.id))
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
    commit_text_in(&mut app.buffers, ui, id, current, multiline.then_some(6), width, None)
}

/// [`commit_text`] with the buffers what is typed is kept in until then:
/// several lines (`rows` of them shown) or one, `width` wide, of at most
/// `limit` characters. Every text field that sets its text when the
/// keyboard leaves it goes through here, so that what was typed is set when
/// a click elsewhere takes the field away ([`leaves_at_a_press`]).
pub(crate) fn commit_text_in(
    buffers: &mut std::collections::HashMap<egui::Id, String>,
    ui: &mut Ui,
    id: egui::Id,
    current: &str,
    rows: Option<usize>,
    width: f32,
    limit: Option<usize>,
) -> Option<String> {
    let (shown, crlf) = to_editor(current);
    let buf = buffers.entry(id).or_insert_with(|| shown.clone());
    let edit = match rows {
        Some(rows) => egui::TextEdit::multiline(buf).desired_rows(rows),
        None => egui::TextEdit::singleline(buf),
    };
    let edit = match limit {
        Some(n) => edit.char_limit(n),
        None => edit,
    };
    let r = ui.add(edit.id(id).desired_width(width));
    leaves_at_a_press(ui.ctx(), &r);
    if !has_keys(&r) && !r.lost_focus() && *buf != shown {
        // Changed underneath (undo, another selection): show the value.
        *buf = shown.clone();
    }
    (r.lost_focus() && *buf != shown).then(|| from_editor(buf, crlf))
}

fn press_leaves_id() -> egui::Id {
    egui::Id::new("moonglow-text-left-at-a-press")
}

/// Says of a text field that sets its text when the keyboard leaves it
/// (`r`, as drawn) that a press of the pointer anywhere else is to take
/// the keyboard from it at once ([`text_begin`]).
pub(crate) fn leaves_at_a_press(ctx: &egui::Context, r: &egui::Response) {
    if has_keys(r) {
        ctx.data_mut(|d| d.insert_temp(press_leaves_id(), (r.id, r.rect)));
    }
}

/// Takes the keyboard from the text field that sets its text when the
/// keyboard leaves it, if one has it: whether one had. It sets its text in
/// the frame now drawn (a command that would take it away is run after:
/// `commands::keys_pressed`).
pub(crate) fn leave_text(ctx: &egui::Context) -> bool {
    let held = ctx.data(|d| d.get_temp::<(egui::Id, egui::Rect)>(press_leaves_id()));
    let Some((id, _)) = held.filter(|(id, _)| ctx.memory(|m| m.has_focus(*id))) else {
        return false;
    };
    ctx.memory_mut(|m| m.surrender_focus(id));
    true
}

/// Once a frame, before anything is drawn: a press of the pointer outside
/// the text field that has the keyboard takes the keyboard from it now,
/// while it is still drawn, so that it sets what was typed. The field
/// alone would give the keyboard up at the click's end, by when what was
/// clicked (another page's tab, its window's close button) may have taken
/// the field away, and what was typed with it.
pub(crate) fn text_begin(ctx: &egui::Context) {
    let Some((id, rect)) = ctx.data(|d| d.get_temp::<(egui::Id, egui::Rect)>(press_leaves_id()))
    else {
        return;
    };
    if !ctx.memory(|m| m.has_focus(id)) {
        return;
    }
    let pressed_elsewhere = ctx.input(|i| {
        i.pointer.any_pressed() && i.pointer.interact_pos().is_some_and(|p| !rect.contains(p))
    });
    if pressed_elsewhere {
        ctx.memory_mut(|m| m.surrender_focus(id));
    }
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

/// [`commit_number`] with a pair of arrows after the field, as Aurora's
/// number fields have: a click is one more or one less, within the range
/// (one change each).
pub(crate) fn spin_number(
    ui: &mut Ui,
    current: i64,
    range: std::ops::RangeInclusive<i64>,
) -> Option<i64> {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        let mut set = commit_number(ui, current, range.clone());
        for (glyph, by, tip) in [("⏶", 1, "One more"), ("⏷", -1, "One less")] {
            let to = (current + by).clamp(*range.start(), *range.end());
            let arrow = egui::Button::new(glyph).small();
            if ui.add_enabled(to != current, arrow).on_hover_text(tip).clicked() {
                set = Some(to);
            }
        }
        set
    })
    .inner
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

/// How a pane's filter field was left by the keys, for its list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FilterLeft {
    /// Down: into the list, at its first row.
    Down,
    /// Escape with nothing to clear: back to the list as it was.
    Escape,
}

/// The keys of a pane's filter field (GitHub issue 12), to be called
/// before the field (whose id is `id`) is added: `find` (Ctrl+F in the
/// pane) goes to it; in it, Escape clears what is typed, and Escape
/// again or Down leaves it for the pane's list.
pub(crate) fn filter_keys(
    ui: &mut egui::Ui,
    id: egui::Id,
    text: &mut String,
    find: bool,
) -> Option<FilterLeft> {
    if find {
        ui.memory_mut(|m| m.request_focus(id));
    }
    // (egui takes the focus from a field at Escape before anything is
    // drawn: the field had it if it had it when last drawn.)
    let has = ui.memory(|m| m.has_focus(id));
    let had_id = id.with("had-focus");
    let had = ui.data(|d| d.get_temp::<bool>(had_id)).unwrap_or(false);
    ui.data_mut(|d| d.insert_temp(had_id, has || find));
    let escape = ui.input(|i| i.key_pressed(egui::Key::Escape) && i.modifiers.is_none());
    if had && escape {
        ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape));
        if text.is_empty() {
            ui.memory_mut(|m| m.surrender_focus(id));
            ui.data_mut(|d| d.insert_temp(had_id, false));
            return Some(FilterLeft::Escape);
        }
        text.clear();
        ui.memory_mut(|m| m.request_focus(id));
        ui.data_mut(|d| d.insert_temp(had_id, true));
        return None;
    }
    if has && ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown)) {
        ui.memory_mut(|m| m.surrender_focus(id));
        ui.data_mut(|d| d.insert_temp(had_id, false));
        return Some(FilterLeft::Down);
    }
    None
}

/// [`filter_keys`] for the filter field of a tab or a window that has no
/// keys of its own for its list: Ctrl+F goes to the field while the
/// pointer is over the tab (what of it shows: `ui`'s clip), and Escape
/// clears it, then leaves it.
pub(crate) fn pane_filter_keys(
    ui: &mut egui::Ui,
    id: egui::Id,
    text: &mut String,
) -> Option<FilterLeft> {
    let find = ui.rect_contains_pointer(ui.clip_rect())
        && !ui.ctx().egui_wants_keyboard_input()
        && ui.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::F));
    filter_keys(ui, id, text, find)
}

/// Two columns side by side where each gets at least `min` points, else one
/// above the other (in a narrow tab or window, half the width would squeeze
/// fixed-width fields into the other column): `add(ui, 0)` fills the first,
/// `add(ui, 1)` the second. Nor side by side where what a column holds was
/// wider than half the width when last drawn (a name's field with its
/// talk-table reference and its button ran under the second column).
pub(crate) fn two_columns(ui: &mut egui::Ui, min: f32, mut add: impl FnMut(&mut egui::Ui, usize)) {
    let gap = ui.spacing().item_spacing.x;
    let id = ui.id().with("two-columns-widths");
    let drawn: Option<[f32; 2]> = ui.data(|d| d.get_temp(id));
    let half = (ui.available_width() - gap) / 2.0;
    let fits = drawn.is_none_or(|w| w[0].max(w[1]) <= half + 0.5);
    let mut widths = [0.0_f32; 2];
    if ui.available_width() >= 2.0 * min + gap && fits {
        ui.columns(2, |cols| {
            for (i, col) in cols.iter_mut().enumerate() {
                widths[i] = col.scope(|ui| add(ui, i)).response.rect.width();
            }
        });
    } else {
        ui.vertical(|ui| {
            widths[0] = ui.scope(|ui| add(ui, 0)).response.rect.width();
            ui.add_space(8.0);
            widths[1] = ui.scope(|ui| add(ui, 1)).response.rect.width();
        });
    }
    if drawn != Some(widths) {
        ui.data_mut(|d| d.insert_temp(id, widths));
        // (What was just drawn may be the layout that does not fit.)
        if drawn.is_none_or(|w| (w[0].max(w[1]) <= half) != (widths[0].max(widths[1]) <= half)) {
            ui.ctx().request_repaint();
        }
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

/// Once a frame, before anything is drawn: the dialog opened last stays
/// above the docked windows (a click on Properties used to bury the
/// Variables window opened from it); and Escape is
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
    // (The one opened last alone: egui keeps the order among those raised
    // in one frame, and a docked window with a Cancel of its own counts as
    // a dialog. Raised with the colour picker opened from it, Tile
    // Properties stayed in front of it once it had been clicked.)
    if let Some(l) = d.open.last() {
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

/// Escape for the window `ui` is in, when it is one of its own (not a
/// docked tab) and the dialog opened last: as a Cancel button reads it,
/// for a window that asks before it closes.
pub(crate) fn escape_here(ui: &Ui) -> bool {
    let layer = ui.layer_id();
    layer.order == egui::Order::Middle && escape_on(ui.ctx(), layer)
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

/// The classic (CUA) clipboard keys, Aurora's and every Windows
/// program's, as egui's Copy, Cut and Paste, before anything reads this
/// frame's keys: Ctrl+Insert copies, Shift+Delete cuts and Shift+Insert
/// pastes, wherever Ctrl+C, Ctrl+X and Ctrl+V do (text fields, the script
/// editor, and Moonglow's own: the area view, conversations, inventories,
/// tables, NUI windows). The key press is taken: Shift+Delete never also
/// deletes. egui-winit does this on Windows alone (there the keys come as
/// the events already); elsewhere they come as keys. A paste needs the
/// clipboard, which the backend reads: Shift+Insert asks it to
/// (`RequestPaste`), and the text comes as a Paste the next frame.
pub(crate) fn clipboard_keys(ctx: &egui::Context) {
    use egui::{Event, Key, Modifiers};
    let mut paste = false;
    ctx.input_mut(|i| {
        let events = std::mem::take(&mut i.events);
        for e in events {
            if let Event::Key { key, pressed: true, modifiers: m, .. } = e {
                // (Ctrl is Cmd on macOS, which has no Insert key; its Ctrl too.)
                let ctrl = m.matches_exact(Modifiers::COMMAND) || m.matches_exact(Modifiers::CTRL);
                let shift = m.matches_exact(Modifiers::SHIFT);
                match key {
                    Key::Insert if ctrl => i.events.push(Event::Copy),
                    Key::Delete if shift => i.events.push(Event::Cut),
                    Key::Insert if shift => paste = true,
                    _ => i.events.push(e),
                }
            } else {
                i.events.push(e);
            }
        }
    });
    if paste {
        ctx.send_viewport_cmd(egui::ViewportCommand::RequestPaste);
    }
}

/// Whether copy, cut and paste were asked for this frame: egui's events,
/// or Ctrl (Cmd) with C, X and V where it sends none.
pub(crate) fn copy_cut_paste(ui: &egui::Ui) -> (bool, bool, bool) {
    ui.input(|i| {
        let key = |k: egui::Key| i.modifiers.command && i.key_pressed(k);
        let event = |f: fn(&egui::Event) -> bool| i.events.iter().any(f);
        (
            event(|e| matches!(e, egui::Event::Copy)) || key(egui::Key::C),
            event(|e| matches!(e, egui::Event::Cut)) || key(egui::Key::X),
            event(|e| matches!(e, egui::Event::Paste(_))) || key(egui::Key::V),
        )
    })
}

/// A window that asks for a new resource's name (New Script, New
/// Conversation, Save Script As).
pub(crate) struct NameWindow<'a> {
    pub title: &'a str,
    pub label: &'a str,
    pub restype: mg_core::ResType,
    /// What to say where the module has a resource of the name already.
    pub taken: &'a str,
    /// One of the name is replaced (said as a warning), not refused.
    pub replaces: bool,
    /// The button that takes the name ("Create", "Save").
    pub button: &'a str,
}

/// What a [`NameWindow`] came to this frame.
pub(crate) enum Named {
    Open,
    Cancelled,
    Chosen(ResKey),
}

impl NameWindow<'_> {
    /// Shows the window with `name` to type in: the name chosen (the
    /// button, or Enter), or that it was cancelled.
    pub(crate) fn show(
        &self,
        ctx: &egui::Context,
        module: Option<&mg_module::Module>,
        name: &mut String,
    ) -> Named {
        let mut out = Named::Open;
        egui::Window::new(self.title)
            .pivot(egui::Align2::CENTER_CENTER)
            .default_pos(ctx.content_rect().center())
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label(self.label);
                let field = ui.add(egui::TextEdit::singleline(name).char_limit(16));
                autofocus(ui, &field);
                let key = mg_core::ResRef::from_str(name.trim())
                    .ok()
                    .filter(|r| !r.is_empty())
                    .map(|r| ResKey::new(r, self.restype));
                let exists = key.is_some_and(|k| module.is_some_and(|m| m.contains(&k)));
                if exists {
                    let color = match self.replaces {
                        true => ui.visuals().warn_fg_color,
                        false => ui.visuals().error_fg_color,
                    };
                    ui.colored_label(color, self.taken);
                }
                let may = key.is_some() && (self.replaces || !exists);
                ui.horizontal(|ui| {
                    if (ui.add_enabled(may, egui::Button::new(self.button)).clicked()
                        || (may && enter(ui)))
                        && let Some(k) = key
                    {
                        out = Named::Chosen(k);
                    }
                    if cancel(ui) {
                        out = Named::Cancelled;
                    }
                });
            });
        out
    }
}

/// A dialog: a window in the middle of the application's, which its
/// close button or Escape closes (`open`).
pub(crate) fn dialog<'a>(ctx: &egui::Context, title: &str, open: &'a mut bool) -> egui::Window<'a> {
    egui::Window::new(title)
        .pivot(egui::Align2::CENTER_CENTER)
        .default_pos(ctx.content_rect().center())
        .open(open_unless_escape(ctx, title, open))
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

/// The mouse wheel over a closed choice list `r` (a body or armor part's):
/// down chooses the row after `current` among `rows`, up the one before,
/// without opening it; the wheel is the list's then, not the page's (which
/// does not scroll under it). The row chosen so.
pub(crate) fn wheel_pick(ui: &Ui, r: &egui::Response, rows: &[i64], current: i64) -> Option<i64> {
    if !r.hovered() || rows.is_empty() || egui::Popup::is_id_open(ui.ctx(), r.id.with("popup")) {
        return None;
    }
    // (The wheel's turn this frame, as its events have it: a notch a step.)
    let scroll: f32 = ui.input(|i| {
        (i.events.iter())
            .filter_map(|e| match e {
                egui::Event::MouseWheel { delta, .. } => Some(delta.y),
                _ => None,
            })
            .sum()
    });
    if scroll == 0.0 {
        return None;
    }
    // (Taken: the page under the list doesn't scroll.)
    ui.input_mut(|i| i.smooth_scroll_delta = egui::Vec2::ZERO);
    let last = rows.len() as i64 - 1;
    let at = rows.iter().position(|&row| row == current).map(|a| a as i64);
    let by = if scroll < 0.0 { 1 } else { -1 };
    let to = at.map_or(0, |a| (a + by).clamp(0, last));
    Some(rows[to as usize])
}

/// A row of a table to choose among `choices` (a 2DA's rows by name, or
/// any numbered list), the one way every editor does it:
///
/// - the box shows the chosen row's name, else its number in parentheses
///   ([`mg_rules::choice_text`]);
/// - a list of more than [`mg_rules::ORDERED_CHOICES`] is shown by name
///   (unless [`ordered`](RowChoice::ordered)), a shorter one as given;
/// - past [`FILTERED_CHOICES`] rows, a Filter field above them narrows
///   them by name (or by a row's number), the rows scrolling under it;
/// - opened, the list is whole and begins at the chosen row;
/// - with the box in focus, the arrow keys step through it, as in Aurora
///   ([`arrow_pick`]), in the order the list shows.
///
/// Made with [`row_choice`]; [`show`](RowChoice::show) draws it and says
/// the row chosen.
#[must_use]
pub(crate) struct RowChoice<'a> {
    id: egui::Id,
    choices: &'a [mg_rules::Choice],
    current: Option<i64>,
    width: Option<f32>,
    shown: Option<String>,
    unset: &'a str,
    none: Option<&'a str>,
    ordered: bool,
    wheel: bool,
    numbered: bool,
    hover: Option<String>,
    about: Option<About<'a>>,
}

/// What a row list shows of a row beside it (its picture): [`RowChoice::about`].
type About<'a> = &'a mut dyn FnMut(&mut Ui, usize);

/// Lists of more than this many rows have a Filter field.
pub(crate) const FILTERED_CHOICES: usize = 30;

/// A list of `choices` with `current` chosen (`None`: no row), its box
/// `id`: see [`RowChoice`].
pub(crate) fn row_choice<'a>(
    id: egui::Id,
    choices: &'a [mg_rules::Choice],
    current: impl Into<Option<i64>>,
) -> RowChoice<'a> {
    RowChoice {
        id,
        choices,
        current: current.into(),
        width: None,
        shown: None,
        unset: "",
        none: None,
        ordered: false,
        wheel: false,
        numbered: false,
        hover: None,
        about: None,
    }
}

/// (The arrow keys' and the wheel's stand-in for the row of none.)
const NO_ROW: i64 = i64::MIN;

impl<'a> RowChoice<'a> {
    /// Whether the list `id` (in `ui`) is open or has the keys, or the
    /// pointer is on its box: when it reads its rows beyond the one chosen
    /// (a caller may make them only then, when they are many). The rows
    /// are there in the pass a click opens it in, which measures the list
    /// to place it above or below the box.
    pub(crate) fn in_use(ui: &Ui, id: egui::Id) -> bool {
        let key = ui.make_persistent_id(id).with("in use");
        let Some((used, rect)) = ui.data(|d| d.get_temp::<(bool, egui::Rect)>(key)) else {
            return false;
        };
        used || ui.input(|i| i.pointer.latest_pos().is_some_and(|p| rect.contains(p)))
    }

    /// The box's width (else egui's for a combo box).
    pub(crate) fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    /// What the box shows, in place of the chosen row's name.
    pub(crate) fn shown(mut self, text: impl Into<String>) -> Self {
        self.shown = Some(text.into());
        self
    }

    /// What the box shows while no row is chosen, with no row of none to
    /// choose ("Not set").
    pub(crate) fn unset(mut self, text: &'a str) -> Self {
        self.unset = text;
        self
    }

    /// A row for none, first, named `text` (the box shows it while none
    /// is chosen).
    pub(crate) fn none(mut self, text: &'a str) -> Self {
        self.none = Some(text);
        self.unset = text;
        self
    }

    /// In the order given, however long (an order that means something:
    /// part numbers, bonuses from +1 up, the palette's categories).
    pub(crate) fn ordered(mut self) -> Self {
        self.ordered = true;
        self
    }

    /// The mouse wheel over the closed box steps through it too
    /// ([`wheel_pick`]).
    pub(crate) fn wheel(mut self) -> Self {
        self.wheel = true;
        self
    }

    /// Each row with its number after its name.
    pub(crate) fn numbered(mut self) -> Self {
        self.numbered = true;
        self
    }

    /// Said with the pointer on the box.
    pub(crate) fn hover_text(mut self, text: impl Into<String>) -> Self {
        self.hover = Some(text.into());
        self
    }

    /// What `about` shows of a row (its picture) beside the row the pointer
    /// rests on, and of the chosen one with the pointer on the box.
    pub(crate) fn about(mut self, about: About<'a>) -> Self {
        self.about = Some(about);
        self
    }

    /// Draws it: the row chosen, if another than the one chosen before.
    pub(crate) fn show(self, ui: &mut Ui) -> Option<i64> {
        self.show_optional(ui).flatten()
    }

    /// [`show`](Self::show), where the row of none may be chosen
    /// (`Some(None)`).
    pub(crate) fn show_optional(self, ui: &mut Ui) -> Option<Option<i64>> {
        let RowChoice {
            id,
            choices,
            current,
            width,
            shown,
            unset,
            none,
            ordered,
            wheel,
            numbered,
            hover,
            mut about,
        } = self;
        let shown = shown.unwrap_or_else(|| match current {
            Some(row) => mg_rules::choice_text(choices, row),
            None => unset.to_string(),
        });
        // (The widget's own state, by `id`, where `in_use` finds it.)
        let key = ui.make_persistent_id(id);
        let filter_id = key.with("filter");
        let open_id = key.with("open");
        let long = choices.len() > FILTERED_CHOICES;
        // A long list by name; a short one as given. (Put in order only
        // when it is used: open, or in focus for the arrow keys.)
        let sorted = std::cell::OnceCell::new();
        let by_name = !ordered && choices.len() > mg_rules::ORDERED_CHOICES;
        let in_order = || -> &[mg_rules::Choice] {
            if by_name {
                sorted.get_or_init(|| mg_rules::by_name(choices.to_vec()))
            } else {
                choices
            }
        };
        fn text(c: &mg_rules::Choice, numbered: bool) -> std::borrow::Cow<'_, str> {
            if numbered {
                format!("{}  ({})", c.text, c.row).into()
            } else {
                c.text.as_str().into()
            }
        }
        let mut pick: Option<Option<i64>> = None;
        // (Open until a choice is made or a click lands outside: a click in
        // its Filter field must not close it.)
        let mut combo = egui::ComboBox::from_id_salt(id)
            .selected_text(shown)
            .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside);
        if let Some(width) = width {
            combo = combo.width(width);
        }
        // Just opened: the list is whole and begins at the one chosen, not
        // at its top (a long list was searched for it every time).
        let opened = !ui.data(|d| d.get_temp::<bool>(open_id)).unwrap_or(false);
        let list = combo.show_ui(ui, |ui| {
            if opened {
                ui.data_mut(|d| {
                    d.remove_temp::<String>(filter_id);
                    d.insert_temp(open_id.with("go"), true);
                });
            }
            // (A popup's first pass only measures it: the list is brought
            // to the chosen row in the first pass that draws it.)
            let go = !ui.is_sizing_pass()
                && ui.data_mut(|d| d.remove_temp::<bool>(open_id.with("go"))).is_some();
            let mut filter: String = ui.data(|d| d.get_temp(filter_id)).unwrap_or_default();
            // (What is typed narrows the list from its top, wherever it
            // had been scrolled to.)
            let mut narrowed = false;
            if long {
                narrowed =
                    ui.add(egui::TextEdit::singleline(&mut filter).hint_text("Filter")).changed();
                ui.data_mut(|d| d.insert_temp(filter_id, filter.clone()));
            }
            let filter = filter.trim().to_lowercase();
            let matches = |c: &mg_rules::Choice| {
                filter.is_empty()
                    || text(c, numbered).to_lowercase().contains(&filter)
                    || c.row.to_string() == filter
            };
            // (The rows scroll under the Filter field, which stays.)
            let mut rows = |ui: &mut Ui| {
                if let Some(none) = none {
                    let r = ui.selectable_label(current.is_none(), none);
                    if go && current.is_none() {
                        r.scroll_to_me(Some(egui::Align::Center));
                    }
                    if r.clicked() {
                        pick = Some(None);
                        ui.close();
                    }
                }
                for c in in_order().iter().filter(|c| matches(c)) {
                    let chosen = current == Some(c.row as i64);
                    let mut r = ui.selectable_label(chosen, text(c, numbered));
                    if go && chosen {
                        r.scroll_to_me(Some(egui::Align::Center));
                    }
                    if let Some(about) = about.as_mut() {
                        r = r.on_hover_ui_at_pointer(|ui| about(ui, c.row));
                    }
                    if r.clicked() {
                        pick = Some(Some(c.row as i64));
                        ui.close();
                    }
                }
            };
            if long {
                let mut area = egui::ScrollArea::vertical().max_height(170.0);
                if narrowed {
                    area = area.vertical_scroll_offset(0.0);
                }
                area.show(ui, rows);
            } else {
                rows(ui);
            }
        });
        let r = &list.response;
        // (In use once a click has opened it, before it is drawn open, or
        // with the keys; and where its box was, for the pointer: see
        // `in_use`.)
        let open = list.inner.is_some() || egui::ComboBox::is_open(ui.ctx(), r.id);
        let in_use = (open || r.has_focus(), r.rect);
        ui.data_mut(|d| {
            d.insert_temp(open_id, list.inner.is_some());
            d.insert_temp(key.with("in use"), in_use);
        });
        // The arrow keys step through it, as in Aurora (in the order
        // shown), and the wheel where asked.
        let stepping = r.clicked() || r.has_focus() || (wheel && r.hovered());
        if pick.is_none() && stepping {
            let rows: Vec<i64> = (none.map(|_| NO_ROW).into_iter())
                .chain(in_order().iter().map(|c| c.row as i64))
                .collect();
            let at = current.unwrap_or(NO_ROW);
            let stepped = arrow_pick(ui, r, &rows, at)
                .or_else(|| wheel.then(|| wheel_pick(ui, r, &rows, at)).flatten());
            pick = stepped.map(|row| (row != NO_ROW).then_some(row));
        }
        let mut response = list.response;
        if let Some(hover) = hover {
            response = response.on_hover_text(hover);
        }
        // The chosen one's picture with the pointer on the box: it follows
        // the arrow keys.
        if let (Some(about), Some(Ok(row))) = (about, current.map(usize::try_from)) {
            response.on_hover_ui(|ui| about(ui, row));
        }
        pick.filter(|&p| p != current)
    }
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

/// [`arrow_pick`] for an egui combo box of a few fixed `options` (not a
/// table's rows, which [`row_choice`] lists: the area's toolbar, a
/// dialog's filters), its box `r`: once clicked or reached with Tab, the
/// arrow keys step `current` through them, in the order shown. Whether
/// it changed.
pub(crate) fn arrow_step<T: PartialEq + Clone>(
    ui: &Ui,
    r: &egui::Response,
    options: &[T],
    current: &mut T,
) -> bool {
    let rows: Vec<i64> = (0..options.len() as i64).collect();
    // (A value none of them is, as a turn typed under Other: the first.)
    let at = options.iter().position(|o| o == current).map_or(-1, |i| i as i64);
    let Some(to) = arrow_pick(ui, r, &rows, at).and_then(|i| options.get(i as usize)) else {
        return false;
    };
    *current = to.clone();
    true
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
/// presses) has it. The press is taken, so one Enter does one thing: a
/// wizard's Next and its Finish both asked, and an Enter on the page
/// before the last went on and finished at once.
pub(crate) fn enter(ui: &egui::Ui) -> bool {
    ui.memory(|m| m.focused().is_none())
        && ui.input_mut(|i| {
            i.modifiers.is_none() && i.consume_key(egui::Modifiers::NONE, egui::Key::Enter)
        })
}

/// A click on a row of a list of which several can be selected (or the
/// keys moved to it): alone, that row alone; with Ctrl, that row added or
/// taken away; with Shift, the rows from the anchor (the one clicked last)
/// to it, as `listed`, and with Ctrl+Shift those added. The hak editor's
/// and the module's hak list's rule.
pub(crate) fn select_in_list<T: Ord + Copy>(
    selected: &mut std::collections::BTreeSet<T>,
    anchor: &mut Option<T>,
    listed: &[T],
    row: T,
    command: bool,
    shift: bool,
) {
    let place = |t: T| listed.iter().position(|l| *l == t);
    match (anchor.filter(|_| shift).and_then(place), place(row)) {
        // From the anchor to this one, as listed (with Ctrl, added to
        // what is selected); the anchor stays.
        (Some(a), Some(b)) => {
            if !command {
                selected.clear();
            }
            selected.extend(listed[a.min(b)..=a.max(b)].iter().copied());
        }
        _ => {
            if command {
                if !selected.remove(&row) {
                    selected.insert(row);
                }
            } else {
                *selected = [row].into();
            }
            *anchor = Some(row);
        }
    }
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

/// How thick the strip along a pane's inner edge is, the edge it faces the
/// middle by (the module tree's right, the palettes' left, the log's top):
/// the pane's fold button is in it, and it is all a folded pane leaves.
pub(crate) const PANE_EDGE: f32 = 16.0;

/// Which way a pane's fold button points: the way its pane goes when it
/// is clicked.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Toward {
    Left,
    Right,
    Up,
    Down,
}

/// Where a pane's fold button is in the strip along its edge
/// ([`PANE_EDGE`] thick): across all of it, and centered along it (an
/// up-and-down strip when `upright`).
pub(crate) fn pane_fold_rect(strip: egui::Rect, upright: bool) -> egui::Rect {
    const LONG: f32 = 44.0;
    let size = if upright {
        egui::vec2(strip.width(), LONG.min(strip.height()))
    } else {
        egui::vec2(LONG.min(strip.width()), strip.height())
    };
    egui::Rect::from_center_size(strip.center(), size)
}

/// A pane's own button to fold it away and to bring it back (GitHub issue
/// 10): a chevron in the strip along the pane's inner edge, at `rect`
/// ([`pane_fold_rect`]), the same place shown and folded. Unlike the
/// buttons beside it (the palettes' types, a pane's tabs): narrow and
/// long, with no text, and lit under the pointer. Named `label` for
/// screen readers. True on the frame it is clicked.
pub(crate) fn pane_fold_button(
    ui: &mut Ui,
    rect: egui::Rect,
    id: egui::Id,
    toward: Toward,
    label: &str,
    tip: &str,
) -> bool {
    let response = ui.interact(rect, id, egui::Sense::click());
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, label));
    let response = response.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text(tip);
    if ui.is_rect_visible(rect) {
        let visuals = ui.style().interact(&response);
        let lit =
            response.hovered() || response.has_focus() || response.is_pointer_button_down_on();
        let fill = if lit { visuals.bg_fill } else { ui.visuals().widgets.inactive.weak_bg_fill };
        let stroke = if lit { visuals.bg_stroke } else { egui::Stroke::NONE };
        let painter = ui.painter();
        let body = rect.shrink(1.0);
        painter.rect(body, 3.0, fill, stroke, egui::StrokeKind::Inside);
        // The chevron, pointing the way the pane goes.
        let c = body.center();
        let (a, b) = (2.5, 4.5);
        let points = match toward {
            Toward::Left => [c + egui::vec2(a, -b), c + egui::vec2(-a, 0.0), c + egui::vec2(a, b)],
            Toward::Right => {
                [c + egui::vec2(-a, -b), c + egui::vec2(a, 0.0), c + egui::vec2(-a, b)]
            }
            Toward::Up => [c + egui::vec2(-b, a), c + egui::vec2(0.0, -a), c + egui::vec2(b, a)],
            Toward::Down => [c + egui::vec2(-b, -a), c + egui::vec2(0.0, a), c + egui::vec2(b, -a)],
        };
        let color = if lit { visuals.fg_stroke.color } else { ui.visuals().text_color() };
        painter.add(egui::Shape::line(points.to_vec(), egui::Stroke::new(1.5, color)));
    }
    response.clicked()
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

    /// An object's or a location's variable, which the Variables window
    /// doesn't edit, is written back as it was: OK wrote every variable
    /// not an int or a float as a string (a code review's finding).
    #[test]
    fn variables_the_window_does_not_edit_keep_their_values() {
        let var = |name: &[u8], kind: u32, value: Value| {
            let mut s = Struct::new(0);
            s.set("Name", Value::String(name.to_vec()));
            s.set("Type", Value::Dword(kind));
            s.set("Value", value);
            s
        };
        let mut place = Struct::new(1);
        place.set("Area", Value::Dword(7));
        let list = vec![
            var(b"oGuard", 4, Value::Dword(0x7f00_0001)),
            var(b"lHome", 5, Value::Struct(place)),
            var(b"nCount", 1, Value::Int(3)),
        ];
        let target = FieldTarget {
            key: ResKey::parse("inn", ResType::GIT).unwrap(),
            path: GffPath::root(),
            label: "VarTable".into(),
            also: Vec::new(),
        };
        let mut edit = VarTableEdit::new(target, &list);
        assert_eq!(edit.list(), list, "untouched, written as it was");
        let count = edit.rows.iter_mut().find(|r| r.name == "nCount").unwrap();
        count.value = "4".into();
        assert_eq!(edit.problem(), None);
        let written = edit.list();
        assert_eq!(written[..2], list[..2], "the object and the location kept");
        assert_eq!(written[2].get("Value"), Some(&Value::Int(4)));
        let count = edit.rows.iter_mut().find(|r| r.name == "nCount").unwrap();
        count.value = "four".into();
        assert!(edit.problem().is_some_and(|p| p.starts_with("nCount: ")));
    }

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

    /// GitHub issue 13: with Polish the editing language, String Edit reads
    /// a string's English text as a Polish game shows it (Windows-1250:
    /// Polish modules keep Polish names there) and takes Polish letters
    /// typed into it; with English the editing language it is
    /// Windows-1252, and a letter that has none says why it can't be saved.
    #[test]
    fn string_edit_reads_english_text_as_the_polish_game_does() {
        use mg_core::{Gender, Language, LocStringKey, StrRef};
        let polish = "Żółw";
        let bytes = mg_core::Codepage::WINDOWS_1250.encode(polish).unwrap().into_owned();
        let english = LocStringKey::new(Language::ENGLISH, Gender::Male);
        let value = LocString { strref: StrRef::NONE, strings: vec![(english, bytes.clone())] };
        crate::text::set_edit_language(Language::POLISH);
        let mut edit = LocStringEdit::new(target(), "Name", &value);
        assert_eq!(edit.entries[0].2, polish);
        assert_eq!(edit.value(), Some(value.clone()), "written back as it was");
        edit.entries[0].2 = "Żółwie".into();
        let typed = edit.value().expect("Polish letters are taken");
        assert_eq!(mg_core::Codepage::WINDOWS_1250.decode(&typed.strings[0].1), "Żółwie");
        assert_eq!(edit.problem(), None);
        // English the editing language: Windows-1252, where "ł" has no
        // place, and the window says so (it named the String Ref).
        crate::text::set_edit_language(Language::ENGLISH);
        let mut edit = LocStringEdit::new(target(), "Name", &value);
        assert_ne!(edit.entries[0].2, polish);
        edit.entries[0].2 = "Żółw".into();
        assert_eq!(edit.value(), None);
        let problem = edit.problem().unwrap();
        assert!(problem.contains("English") && problem.contains('ł'), "{problem}");
        edit.strref = "x".into();
        assert!(edit.problem().unwrap().contains("String Ref"));
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
        let mut bad = edit;
        bad.strref = "x".into();
        assert_eq!(bad.value(), None);
    }
}

#[cfg(test)]
mod token_tests {
    use super::with_token;
    use egui::text::{CCursor, CCursorRange};

    /// Two columns are side by side only where what the first holds fits
    /// in its half; and again once there is room.
    #[test]
    fn two_columns_stack_where_the_first_does_not_fit() {
        let ctx = egui::Context::default();
        // Where the second column's content begins, in a window so wide.
        let second_at = |width: f32| -> egui::Pos2 {
            let mut at = egui::Pos2::ZERO;
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(width, 400.0),
                )),
                ..Default::default()
            };
            let mut out = ctx.run_ui(input, |ui| {
                egui::CentralPanel::default().show(ui, |ui| {
                    super::two_columns(ui, 100.0, |ui, col| {
                        let size = if col == 0 { [300.0, 20.0] } else { [80.0, 20.0] };
                        let r = ui.allocate_exact_size(size.into(), egui::Sense::hover()).0;
                        if col == 1 {
                            at = r.min;
                        }
                    });
                });
            });
            out.textures_delta.clear();
            at
        };
        let top = second_at(900.0).y;
        // (Drawn twice: the first pass measures.)
        let wide = second_at(900.0);
        assert!(wide.x > 300.0 && wide.y == top, "beside: {wide:?}");
        second_at(500.0);
        let narrow = second_at(500.0);
        assert!(narrow.x < 100.0 && narrow.y > top, "beneath, not under the first: {narrow:?}");
        second_at(900.0);
        let again = second_at(900.0);
        assert!(again.x > 300.0 && again.y == top, "beside again: {again:?}");
    }

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
