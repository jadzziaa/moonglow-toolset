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

use crate::text::{decode, encode, to_editor, with_line_ends, with_loc_line_ends};
use crate::{Action, FileKind, Moonglow};

/// A field of a GFF resource that an editor window writes back to.
#[derive(Debug, Clone, PartialEq)]
pub struct FieldTarget {
    pub key: ResKey,
    pub path: GffPath,
    pub label: String,
    /// More structs that take the same value (several objects or
    /// blueprints edited at once): each document and path. Shared, not
    /// copied, by each field of a form so editing thousands of objects
    /// together does not clone them per field every frame.
    pub also: Also,
}

/// The other structs a field set goes to ([`FieldTarget::also`]).
pub type Also = std::sync::Arc<Vec<(ResKey, GffPath)>>;

impl FieldTarget {
    pub fn new(key: ResKey, path: GffPath, label: &str) -> FieldTarget {
        FieldTarget { key, path, label: label.to_string(), also: Also::default() }
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
    /// Language, gender, text (with plain line breaks) and the stored
    /// text, whose line ends the text is written back with (empty for a
    /// text added).
    pub entries: Vec<(Language, Gender, String, String)>,
    original: LocString,
    /// Each language's tokens, read when its Token… is first opened.
    tokens: Vec<(Language, Vec<String>)>,
    /// Move to Talk Table's line, added to the table at OK, and the
    /// String Ref it was shown as taking: not added where the String Ref
    /// was changed since, or the window canceled.
    moved: Option<(String, mg_module::talk::Line)>,
}

impl LocStringEdit {
    pub fn new(target: FieldTarget, title: &str, value: &LocString) -> LocStringEdit {
        let entries = value
            .strings
            .iter()
            .map(|(k, bytes)| {
                let text = crate::text::entry_codepage(k.language()).decode(bytes);
                (k.language(), k.gender(), to_editor(&text).0, text.into_owned())
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
            moved: None,
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
        for (language, gender, text, stored) in &self.entries {
            let text = with_loc_line_ends(text, stored);
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

/// What a variable is told apart by: its name and its type. (The game
/// keeps an int and a string of one name as two variables: `GetLocalInt`
/// and `GetLocalString` each read their own.)
fn var_key(s: &Struct) -> (&[u8], Option<&Value>) {
    (s.string("Name").unwrap_or_default(), s.get("Type"))
}

/// What an edit of one object's variables (`before` to `after`) makes of
/// another's (`theirs`): the variables added or changed are set on it (in
/// place where it has one of that name and type, else at the end), those
/// deleted are deleted from it, and it keeps its others as they are. (By
/// name alone, a change to an int replaced the string of its name too on
/// the others edited with it, and left them with two ints.)
pub(crate) fn merge_variables(
    before: &[Struct],
    after: &[Struct],
    theirs: &[Struct],
) -> Vec<Struct> {
    type Key<'a> = (&'a [u8], Option<&'a Value>);
    let had = |list: &[Struct], key: Key<'_>| list.iter().any(|s| var_key(s) == key);
    let changed: Vec<&Struct> = after.iter().filter(|a| !before.contains(a)).collect();
    let mut out: Vec<Struct> = theirs
        .iter()
        .filter(|s| !had(before, var_key(s)) || had(after, var_key(s)))
        .map(|s| {
            let set = changed.iter().find(|c| var_key(c) == var_key(s));
            set.map_or_else(|| s.clone(), |c| (*c).clone())
        })
        .collect();
    for c in changed {
        if !had(&out, var_key(c)) {
            out.push(c.clone());
        }
    }
    out
}

/// Whether a variable holds a color the Variables window offers a picker
/// for: a string `"#rrggbb"`, or one of the tint's locals however it is
/// set so far (The Domains of Dread's colorways, [`mg_rules::tint`]).
fn colored(r: &VarRow) -> bool {
    r.kind == 3
        && (mg_rules::tint::parse_bytes(&r.value).is_some()
            || mg_rules::tint::is_tint_name(&r.name))
}

/// A color's swatch and picker for a value of `"#rrggbb"` (white while it
/// is none), which a color chosen writes. While its picker is open it
/// says so ([`picking`]): the Variables page sets what was picked once
/// it closes, one command, and shows it meanwhile.
fn color_swatch(ui: &mut Ui, value: &mut String) {
    let mut rgb = mg_rules::tint::parse_bytes(value).unwrap_or([255; 3]);
    let popup = ui.auto_id_with("popup");
    let r = crate::color_button::edit_srgb(ui, &mut rgb)
        .on_hover_text("The color, written as #rrggbb (red, green and blue in hexadecimal)");
    if r.changed() {
        *value = mg_rules::tint::format(rgb);
    }
    if egui::Popup::is_id_open(ui.ctx(), popup) {
        ui.ctx().data_mut(|d| d.insert_temp(picking(), true));
    }
}

/// Where a Variables list's color picker says it is open this frame.
fn picking() -> egui::Id {
    egui::Id::new("variables-color-picking")
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
    /// The Variable Sets window (Add Set › Manage Sets…), while it's open.
    pub sets: Option<SetsWindow>,
    /// The project's described variables (`variables.toml`), as the
    /// window offers and checks them: read when it first shows.
    pub described: Option<VarHelp>,
}

/// The Variable Sets window (Add Set › Manage Sets… in the Variables
/// window): the sets saved in Moonglow's data folder, renamed and deleted.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SetsWindow {
    /// The set Rename… was chosen for, and the name typed so far.
    pub renaming: Option<(String, String)>,
    /// The set deleted last and its text, for Undo Delete (the file is
    /// Moonglow's, not the module's: Edit › Undo doesn't know it).
    pub deleted: Option<(String, String)>,
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
    /// Those values as their lists show them ([`Offered`]), made with
    /// them.
    offered: std::collections::HashMap<String, Offered>,
    /// The rows' names at the frame before, to see one change.
    names: Vec<String>,
    /// The row whose name is being typed: the names offered under it.
    pub completing: Option<usize>,
    /// The offer's row Up and Down move to, which Enter takes.
    pub highlighted: usize,
    /// How many were offered at the frame before (the keys are the
    /// offer's only while there is one).
    offered_before: usize,
    /// The row and name Escape closed the offer at: not offered again
    /// until the name changes.
    dismissed: Option<(usize, String)>,
    /// Each row's note ([`VarHelp::note`]) by its name, type and value,
    /// kept until a variable's values are read: a value was looked for in
    /// the whole of its variable's values (a table's 25,000 rows) for each
    /// row, every frame.
    notes: std::collections::HashMap<(String, u32, String), Option<String>>,
}

/// A described variable's values as its list shows them, made once when
/// they are read: a table's rows (`rows = "placeables"`, 25,000 with CEP)
/// were copied every frame for each such variable, and each row of the
/// open list was looked up in the whole list.
#[derive(Debug, Clone, PartialEq, Default)]
struct Offered {
    /// Each value's row, numbered by its place among the values: the
    /// value, and what it means after it.
    rows: Vec<mg_rules::Choice>,
    /// Where each value is among them (the first, of one written twice).
    at: std::collections::HashMap<String, usize>,
}

impl Offered {
    fn of(c: &Choices) -> Offered {
        let mut at = std::collections::HashMap::with_capacity(c.list.len());
        let rows = (c.list.iter().enumerate())
            .map(|(row, x)| {
                at.entry(x.value.clone()).or_insert(row);
                let text = match &x.note {
                    Some(n) => format!("{} — {n}", x.value),
                    None => x.value.clone(),
                };
                mg_rules::Choice { row, text }
            })
            .collect();
        Offered { rows, at }
    }
}

/// A name the Variables window offers as one is typed: a described
/// variable's, or the beginning of a pattern's names (`tdod_rental_` of
/// `tdod_rental_*`), typed on from there.
#[derive(Debug, Clone)]
struct Offer<'a> {
    /// What the name becomes when it is taken.
    name: String,
    described: &'a Described,
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
            self.offered.insert(d.name.clone(), Offered::of(&c));
            self.choices.insert(d.name.clone(), c);
            self.notes.clear();
        }
    }

    /// [`VarHelp::note`], kept for the row's name, type and value.
    fn kept_note(&mut self, r: &VarRow) -> Option<String> {
        let key = (r.name.clone(), r.kind, r.value.clone());
        if let Some(note) = self.notes.get(&key) {
            return note.clone();
        }
        let note = self.note(r);
        // (Names and values typed one letter at a time: not kept for ever.)
        if self.notes.len() > 4096 {
            self.notes.clear();
        }
        self.notes.insert(key, note.clone());
        note
    }

    /// What the rows lack that one of them needs beside it (the
    /// description's `requires`, its `[[rules]]`), a note each; not of
    /// the row whose name is being typed.
    fn needed(&self, rows: &[VarRow], typing: Option<usize>) -> Vec<String> {
        let file = self.variables.file();
        let here: Vec<(&str, &str)> = (rows.iter().enumerate())
            .filter(|(i, r)| Some(*i) != typing && !r.name.trim().is_empty())
            .map(|(_, r)| (r.name.trim(), r.value.as_str()))
            .collect();
        (self.variables.missing(&here).into_iter())
            .map(|n| match n.is {
                Some(is) => format!(
                    "{} is {is}: {} is needed beside it ({file}:{})",
                    n.by, n.require, n.line
                ),
                None => format!("{} needs {} beside it ({file}:{})", n.by, n.require, n.line),
            })
            .collect()
    }

    /// What is wrong with a row by the descriptions, for the note under
    /// the list (none where nothing is, or nothing is described).
    fn note(&self, r: &VarRow) -> Option<String> {
        let file = self.variables.file();
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
    /// row has; then the patterns' beginnings so (not where what is typed
    /// is already a name the pattern takes).
    fn offered(&self, typed: &str, rows: &[VarRow], row: usize) -> Vec<Offer<'_>> {
        let raw = typed.trim();
        let typed = raw.to_ascii_lowercase();
        let taken = |n: &str| rows.iter().enumerate().any(|(i, r)| i != row && r.name == n);
        // Those that begin with what is typed, then those that hold it.
        fn ranked<'a>(
            all: Vec<Offer<'a>>,
            typed: &str,
            taken: &dyn Fn(&str) -> bool,
        ) -> Vec<Offer<'a>> {
            let kept = all.into_iter().filter(|o| {
                let lower = o.name.to_ascii_lowercase();
                !taken(&o.name) && lower != typed && (typed.is_empty() || lower.contains(typed))
            });
            let (first, then): (Vec<_>, Vec<_>) =
                kept.partition(|o| o.name.to_ascii_lowercase().starts_with(typed));
            first.into_iter().chain(then).collect()
        }
        let names = self
            .variables
            .names_for(self.owner)
            .map(|d| Offer { name: d.name.clone(), described: d });
        let patterns = (self.variables.patterns_for(self.owner))
            .filter(|d| raw.is_empty() || !d.matches(raw))
            .flat_map(|d| d.beginnings().into_iter().map(move |name| Offer { name, described: d }));
        let mut out = ranked(names.collect(), &typed, &taken);
        out.extend(ranked(patterns.collect(), &typed, &taken));
        out.truncate(12);
        out
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
        VarTableEdit {
            target,
            original: list.to_vec(),
            rows,
            set_name: None,
            sets: None,
            described: None,
        }
    }

    /// For several objects edited together: the command that sets what
    /// was changed here on each, each keeping its other variables.
    fn merged(&self, app: &mut Moonglow) -> Action {
        let label = self.target.label.clone();
        let edits = (self.each_merged(app).into_iter())
            .map(|(key, path, merged)| Edit::SetField {
                key,
                path,
                label: label.clone(),
                value: Some(Value::List(merged)),
            })
            .collect();
        Action::Apply(Command::new("Edit variables", edits))
    }

    /// Each object's list as the table's changes make it, its own other
    /// variables kept: of those it changes.
    fn each_merged(&self, app: &mut Moonglow) -> Vec<(ResKey, GffPath, Vec<Struct>)> {
        let list = self.list();
        let t = &self.target;
        let targets = std::iter::once((t.key, &t.path)).chain(t.also.iter().map(|(k, p)| (*k, p)));
        let mut out = Vec::new();
        for (key, path) in targets {
            let Some(Ok(doc)) = app.ws.as_mut().map(|ws| ws.doc(&key)) else { continue };
            let Some(s) = path.get(&doc.root) else { continue };
            let theirs = s.list(&t.label).unwrap_or(&[]);
            let merged = merge_variables(&self.original, &list, theirs);
            if merged != theirs {
                out.push((key, path.clone(), merged));
            }
        }
        out
    }

    /// Why the table cannot be saved, if it cannot.
    pub fn problem(&self) -> Option<String> {
        let mut seen = (self.rows.len() > 16)
            .then(|| std::collections::HashSet::with_capacity(self.rows.len()));
        for (i, r) in self.rows.iter().enumerate() {
            if r.name.trim().is_empty() {
                return Some(format!("Variable {} has no name", i + 1));
            }
            if let Some(t @ (VarType::Int | VarType::Float)) = VarType::of(r.kind.into())
                && let Err(e) = t.value(&r.value, crate::text::game_codepage())
            {
                return Some(format!("{}: {e}", r.name));
            }
            let duplicate = match seen.as_mut() {
                Some(set) => !set.insert(r.name.as_str()),
                None => self.rows[..i].iter().any(|o| o.name == r.name),
            };
            if duplicate {
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

fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

/// A described variable's tip: what it is, and its help.
fn description_tip(d: &Described) -> String {
    let what = capitalize_first(&d.words());
    match &d.help {
        Some(h) => format!("{what}.\n{h}"),
        None => format!("{what}."),
    }
}

/// A row named (anew) as a described variable takes the type it is
/// described with, where the window sets that type; a value that type
/// can't hold is emptied. One with no value of its own yet (empty, or a
/// new row's 0) takes the description's default.
fn take_described_type(variables: &Variables, r: &mut VarRow) {
    // (Not one the scripts alone set: it is said, not taken.)
    let described = variables.describe(r.name.trim()).filter(|d| !d.runtime);
    if let Some(default) = described.and_then(|d| d.default.clone()) {
        take_type(described.and_then(|d| d.var_type), r);
        if matches!(r.value.trim(), "" | "0" | "0.0") {
            r.value = default;
        }
        return;
    }
    take_type(described.and_then(|d| d.var_type), r);
}

/// [`take_described_type`]'s type.
fn take_type(t: Option<VarType>, r: &mut VarRow) {
    let Some(t) = t else { return };
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

/// The described names offered under a name being typed, the one
/// `highlighted` (Up and Down move it) marked: the one clicked is
/// `picked` (its row and name). Whether the pointer is over the list. A
/// pattern's beginning is shown with the pattern.
fn completion(
    ui: &Ui,
    row: usize,
    field: &egui::Response,
    offered: &[Offer<'_>],
    highlighted: usize,
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
                mg_uikit::list_rows(ui);
                for (i, o) in offered.iter().enumerate() {
                    let d = o.described;
                    let item = ui
                        .selectable_label(i == highlighted, &o.name)
                        .on_hover_text(description_tip(d));
                    if d.is_pattern() {
                        ui.weak(format!("{}: {}", d.name, d.words()));
                    } else {
                        ui.weak(d.words());
                    }
                    if item.clicked() {
                        *picked = Some((row, o.name.clone()));
                    }
                }
            });
        });
    area.response.hovered() || area.response.contains_pointer()
}

/// The keys an offer of names takes from the name field under it while
/// it shows (`count` names): Up and Down move `highlighted`, Enter takes
/// that name (not the window's OK), Escape closes the offer (not the
/// window). Taken before the field is drawn, so the field and the window
/// don't see them. Whether Escape closed it, and the name Enter took.
fn offer_keys(ui: &Ui, count: usize, highlighted: &mut usize) -> (bool, bool) {
    if count == 0 {
        return (false, false);
    }
    let key = |k| ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, k));
    if key(egui::Key::ArrowDown) {
        *highlighted = (*highlighted + 1) % count;
    }
    if key(egui::Key::ArrowUp) {
        *highlighted = (*highlighted + count - 1) % count;
    }
    let taken = key(egui::Key::Enter);
    let closed = key(egui::Key::Escape);
    if closed {
        keep_escape(ui.ctx());
    }
    (closed, taken)
}

/// A described variable's value, chosen from its values (and typed, where
/// it takes `other`s too): the list every editor has for a table's rows
/// ([`RowChoice`]), in the order the description gives them, which lays
/// out the rows in sight and has a Filter field over a long one.
fn value_choice(
    ui: &mut Ui,
    row: usize,
    other: bool,
    c: &Choices,
    offered: &Offered,
    value: &mut String,
    width: f32,
) -> bool {
    let at = offered.at.get(value.as_str()).copied();
    let current = at.map(|a| a as i64);
    let chosen = |picked: Option<i64>, value: &mut String| {
        let picked = picked.and_then(|p| usize::try_from(p).ok());
        if let Some(x) = picked.and_then(|p| c.list.get(p)) {
            value.clone_from(&x.value);
        }
    };
    if other {
        let focused = ui
            .horizontal(|ui| {
                let r = ui.add(mg_uikit::text_field(value).desired_width(width - 28.0));
                // (Its values to choose from, the arrow keys stepping through
                // them as every dropdown's do.)
                let picked =
                    row_choice(egui::Id::new(("var-value-offer", row)), &offered.rows, current)
                        .shown("")
                        .width(24.0)
                        .ordered()
                        .hover_text("Its values (variables.toml)")
                        .show(ui);
                chosen(picked, value);
                r.has_focus()
            })
            .inner;
        return focused;
    }
    let text = match at {
        Some(a) => egui::RichText::new(&offered.rows[a].text),
        None if value.is_empty() => egui::RichText::new("(choose)").weak(),
        None => egui::RichText::new(format!("{value} (not one of its values)"))
            .color(ui.visuals().warn_fg_color),
    };
    let picked = row_choice(egui::Id::new(("var-value", row)), &offered.rows, current)
        .shown(text)
        .width(width)
        .ordered()
        .show(ui);
    chosen(picked, value);
    false
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
    pub filter: String,
    choices: Vec<ResRef>,
    /// The choices the Filter finds, and what it had typed (folded) when
    /// they were found: not looked for every frame (60,000 sounds with
    /// CEP's haks, a text made of each).
    found: Option<(String, Vec<ResRef>)>,
}

impl Picker {
    /// The choices the Filter finds, in their order.
    fn shown(&mut self) -> &[ResRef] {
        let filter = mg_core::text::Needle::new(&self.filter);
        if self.found.as_ref().is_none_or(|(typed, _)| typed != filter.as_str()) {
            let found =
                self.choices.iter().filter(|r| filter.finds(&r.to_string())).copied().collect();
            self.found = Some((filter.as_str().to_string(), found));
        }
        self.found.as_ref().map_or(&[], |(_, found)| found)
    }
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
        let (title, filter) = (title.to_string(), String::new());
        self.picker = Some(Picker { id, title, filter, choices, found: None });
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

/// A kind of set kept in Moonglow's data folder as its Add Set menu and
/// its window show it: variable sets (the Variables window) and
/// parameter sets (a conversation script's parameters).
pub(crate) struct SetKind<T: 'static> {
    pub shelf: crate::saved::Shelf,
    /// Its window's title (Add Set › Manage Sets…).
    pub title: &'static str,
    /// What the window says while none is saved.
    pub none_yet: &'static str,
    /// The tip of the menu's Add from File….
    pub from_file_tip: &'static str,
    /// A set read from its folder by its name.
    pub load: fn(&std::path::Path, &str) -> Result<T, String>,
    /// What a set holds, as its tip says it.
    pub describe: fn(&T) -> String,
}

pub(crate) const VARIABLE_SETS: SetKind<Vec<VarRow>> = SetKind {
    shelf: crate::var_sets::SHELF,
    title: "Variable Sets",
    none_yet: "No variable sets saved yet: Save Set… in the Variables window keeps one.",
    from_file_tip: "Add the variables of set files (.vars.json) anywhere",
    load: crate::var_sets::load,
    describe: |rows| crate::var_sets::describe(rows),
};

pub(crate) const PARAMETER_SETS: SetKind<crate::param_sets::ParamSet> = SetKind {
    shelf: crate::param_sets::SHELF,
    title: "Parameter Sets",
    none_yet: "No parameter sets saved yet: Save Set… beside a script's parameters keeps one.",
    from_file_tip: "Add the script and parameters of set files (.params.json) anywhere",
    load: crate::param_sets::load,
    describe: crate::param_sets::describe,
};

impl<T: Clone + Send + Sync + 'static> SetKind<T> {
    /// What one is called at the beginning of a message ("Variable set").
    fn what(&self) -> String {
        capitalize_first(self.shelf.what)
    }

    /// The sets saved in `dir`, each read ([`crate::saved::Shelf::read_all`],
    /// the files read again only when they change, kept in `ctx`'s memory).
    fn read(
        &self,
        ctx: &egui::Context,
        dir: Option<&std::path::Path>,
    ) -> Vec<(String, Result<T, String>)> {
        let id = egui::Id::new((self.shelf.folder, "read"));
        let mut kept: crate::saved::Read<T> =
            ctx.data_mut(|d| d.remove_temp(id)).unwrap_or_default();
        let read = self.shelf.read_all(dir, &mut kept, self.load);
        ctx.data_mut(|d| d.insert_temp(id, kept));
        read
    }
}

/// What was chosen in an Add Set menu ([`sets_menu`]).
pub(crate) enum SetChoice<T> {
    /// A saved set, to add.
    Add(T),
    /// Add from File….
    FromFile,
    /// Manage Sets….
    Manage,
}

/// An Add Set menu's items: the sets saved in `dir` (what each holds its
/// tip; one whose file isn't a set marked, and not to be chosen, why in
/// its tip), then Add from File…, Manage Sets… and Open Sets Folder
/// (which it opens itself).
pub(crate) fn sets_menu<T: Clone + Send + Sync + 'static>(
    ui: &mut Ui,
    log: &mut crate::Log,
    kind: &SetKind<T>,
    dir: Option<&std::path::Path>,
) -> Option<SetChoice<T>> {
    let mut chosen = None;
    // (Read as the menu is drawn: what each holds is its tip, and a file
    // that isn't a set is said so.)
    let read = kind.read(ui.ctx(), dir);
    if dir.is_none() {
        mg_uikit::menu_note(ui, "Moonglow has no data folder to keep sets in");
    } else if read.is_empty() {
        mg_uikit::menu_note(ui, format!("No {}s saved yet", kind.shelf.what));
    }
    for (n, set) in read {
        match set {
            Ok(set) => {
                let tip = (kind.describe)(&set);
                if ui.button(n).on_hover_text(tip).clicked() {
                    chosen = Some(SetChoice::Add(set));
                    ui.close();
                }
            }
            Err(e) => {
                let b = egui::Button::new(not_a_set(ui, &n));
                ui.add_enabled(false, b).on_disabled_hover_text(e);
            }
        }
    }
    mg_uikit::menu_separator(ui);
    if ui.button("Add from File…").on_hover_text(kind.from_file_tip).clicked() {
        chosen = Some(SetChoice::FromFile);
        ui.close();
    }
    if ui
        .add_enabled(dir.is_some(), egui::Button::new("Manage Sets…"))
        .on_hover_text("Rename or delete the saved sets")
        .clicked()
    {
        chosen = Some(SetChoice::Manage);
        ui.close();
    }
    if let Some(dir) = dir
        && ui
            .button("Open Sets Folder")
            .on_hover_text("Open the folder the sets are kept in, to copy or edit them")
            .clicked()
    {
        open_folder(ui.ctx(), dir, log);
        ui.close();
    }
    chosen
}

/// Under Save Set's name: that a set of the name on `shelf` (in `dir`)
/// is replaced, where there is one. (Said, as it isn't asked about: the
/// variable sets said it and the parameter sets did not.)
pub(crate) fn replaced_note(
    ui: &mut Ui,
    shelf: &crate::saved::Shelf,
    dir: Option<&std::path::Path>,
    name: &str,
) {
    let kept = dir.and_then(|dir| shelf.file(Some(dir), name).ok());
    if kept.is_some_and(|file| file.exists()) {
        ui.colored_label(ui.visuals().warn_fg_color, "A set of that name is replaced.");
    }
}

/// Save Set's name, typed in place: whether it is to be saved (Save, or
/// Enter in the name) and whether Cancel was pressed.
pub(crate) fn set_name_row(ui: &mut Ui, name: &mut String) -> (bool, bool) {
    let mut save = false;
    let mut cancel = false;
    // (Its buttons on a line of their own where a narrow pane has no room
    // for them beside the name: a conversation's panel, M4's U14.)
    ui.horizontal_wrapped(|ui| {
        let label = ui.label("Set name");
        let room = ui.available_width() - ui.spacing().item_spacing.x;
        let width = 180f32.min(room).max(label.rect.width());
        let r = ui.add(mg_uikit::text_field(name).desired_width(width));
        autofocus(ui, &r);
        let ok = crate::saved::valid_name(name);
        // (Enter in the name is the name's, not the window's OK.)
        let entered = r.lost_focus()
            && ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter));
        save = ui.add_enabled(ok, egui::Button::new("Save")).clicked() || (ok && entered);
        cancel = ui.button("Cancel").clicked();
    });
    (save, cancel)
}

/// A set's name as the menu and the sets' window list it: one whose file
/// isn't a set marked so, why in its tip.
fn not_a_set(ui: &Ui, name: &str) -> egui::RichText {
    egui::RichText::new(format!("{name} (not a set)")).color(ui.visuals().warn_fg_color)
}

/// A kind's sets' window (Add Set › Manage Sets…: the Variable Sets
/// window, the Parameter Sets window): the sets saved in `dir`, each
/// renamed or deleted (the one deleted last put back), and their folder
/// opened.
pub(crate) fn sets_window<T: Clone + Send + Sync + 'static>(
    app: &mut Moonglow,
    ctx: &egui::Context,
    sets: &mut Option<SetsWindow>,
    kind: &SetKind<T>,
    dir: Option<std::path::PathBuf>,
) {
    let (Some(w), Some(dir)) = (sets.as_mut(), dir) else {
        *sets = None;
        return;
    };
    let read = kind.read(ctx, Some(&dir));
    let names: Vec<String> = read.iter().map(|(n, _)| n.clone()).collect();
    let what = kind.what();
    let shelf = kind.shelf;
    let mut open = true;
    let mut close = false;
    let window = dialog(ctx, kind.title, &mut open).collapsible(false).resizable(false);
    // (As wide as a long folder's path needs, no wider: the path is cut.)
    let footed = window.max_width(460.0).show_foot(ctx, |ui| {
        ui.horizontal(|ui| {
            field_label(ui, "Folder");
            let path = dir.display().to_string();
            ui.add(egui::Label::new(&path).truncate()).on_hover_text(&path);
        });
        if ui
            .button("Open Folder")
            .on_hover_text(format!(
                "Open the folder in the file manager. Sets are JSON files, NAME{}.",
                shelf.suffix
            ))
            .clicked()
        {
            open_folder(ui.ctx(), &dir, &mut app.log);
        }
        ui.separator();
        if let Some((name, text)) = w.deleted.clone() {
            let mut keep = true;
            ui.horizontal_wrapped(|ui| {
                ui.label(format!("Set '{name}' deleted."));
                if ui.button("Undo Delete").on_hover_text("Put the set back").clicked() {
                    match shelf.restore(Some(&dir), &name, &text) {
                        Ok(()) => app.log.info(format!("{what} '{name}' put back")),
                        Err(e) => app.log.error(format!("Undo Delete: {e}")),
                    }
                    keep = false;
                }
                // (One word for one action: the note goes, the set stays
                // deleted.)
                let dismiss = clear_button().name("Dismiss");
                if ui
                    .add(dismiss)
                    .on_hover_text("Dismiss this note: the set stays deleted")
                    .clicked()
                {
                    keep = false;
                }
            });
            if !keep {
                w.deleted = None;
            }
            ui.separator();
        }
        // The name being typed for the set Rename… was chosen for.
        if let Some((name, to)) = w.renaming.as_mut().filter(|(n, _)| names.contains(n)) {
            let mut done = false;
            ui.horizontal_wrapped(|ui| {
                ui.label(format!("Rename '{name}' to"));
                let r = ui.add(mg_uikit::text_field(to).desired_width(160.0));
                autofocus(ui, &r);
                let taken = names.iter().any(|n| n != name && n.eq_ignore_ascii_case(to.trim()));
                let ok = crate::saved::valid_name(to) && !taken;
                // (Enter in the name is the name's, not the window's OK.)
                let entered = r.lost_focus()
                    && ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter));
                if ui.add_enabled(ok, egui::Button::new("Rename")).clicked() || (ok && entered) {
                    if to.trim() != name {
                        match shelf.rename(Some(&dir), name, to) {
                            Ok(()) => {
                                app.log.info(format!("{what} '{name}' renamed to '{}'", to.trim()))
                            }
                            Err(e) => app.log.error(format!("Rename set: {e}")),
                        }
                    }
                    done = true;
                } else if ui.button("Cancel").clicked() {
                    done = true;
                }
                if taken {
                    ui.colored_label(ui.visuals().warn_fg_color, "Another set has that name");
                } else if !ok && !to.trim().is_empty() {
                    ui.colored_label(
                        ui.visuals().warn_fg_color,
                        "Letters, digits, spaces, -, _ and . only (64 at most)",
                    );
                }
            });
            if done {
                w.renaming = None;
            }
            ui.separator();
        } else {
            w.renaming = None;
        }
        if names.is_empty() {
            ui.weak(kind.none_yet);
        }
        egui::ScrollArea::vertical().max_height(260.0).show(ui, |ui| {
            mg_uikit::list_rows(ui);
            egui::Grid::new(shelf.folder).num_columns(3).spacing([8.0, 0.0]).show(ui, |ui| {
                for (name, set) in &read {
                    // (What it holds in its tip; one that isn't a set, why.)
                    match set {
                        Ok(set) => ui.label(name).on_hover_text((kind.describe)(set)),
                        Err(e) => ui.label(not_a_set(ui, name)).on_hover_text(e),
                    };
                    if ui.small_button("Rename…").clicked() {
                        w.renaming = Some((name.clone(), name.clone()));
                    }
                    if ui
                        .small_button("Delete")
                        .on_hover_text("Delete the set's file (Undo Delete, above, puts it back)")
                        .clicked()
                    {
                        match shelf.delete(Some(&dir), name) {
                            Ok(text) => {
                                app.log.info(format!("{what} '{name}' deleted"));
                                w.deleted = Some((name.clone(), text));
                            }
                            Err(e) => app.log.error(format!("Delete set: {e}")),
                        }
                    }
                    ui.end_row();
                }
            });
        });
        ((), Foot::new().leave("Close"))
    });
    if let Some(f) = footed {
        close = f.pressed.cancelled();
    }
    if !open || close {
        *sets = None;
    }
}

/// Opens `dir` in the system's file manager, made first if it isn't there.
pub(crate) fn open_folder(ctx: &egui::Context, dir: &std::path::Path, log: &mut crate::Log) {
    if let Err(e) = std::fs::create_dir_all(dir) {
        log.error(format!("{}: {e}", dir.display()));
        return;
    }
    ctx.open_url(egui::OpenUrl::new_tab(crate::script_view::file_url(dir)));
}

const GENDERS: [(Gender, &str); 2] = [(Gender::Male, "Male"), (Gender::Female, "Female")];

pub(crate) fn ui(app: &mut Moonglow, ui: &mut Ui) {
    let ctx = ui.ctx().clone();
    let tlk_text = |app: &Moonglow, s: &str| -> Option<String> {
        let n: u32 = s.trim().parse().ok()?;
        app.game.as_deref()?.string(StrRef(n))
    };

    // (A conversation's line follows its renumberings with its editor
    // behind another tab.)
    crate::dialog_view::follow_unseen(app);
    if let Some(edit) = app.loc_edit.take() {
        let mut edit = edit;
        let mut close = false;
        let tlk = tlk_text(app, &edit.strref);
        let movable = crate::talk_view::can_add(app);
        let footed = modal(&ctx, format!("String Edit: {}", edit.title)).show_foot(&ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("String Ref");
                ui.add(
                    mg_uikit::text_field(&mut edit.strref).desired_width(120.0).hint_text("none"),
                );
            });
            // (A text on its way to the talk table: added with OK.)
            let moving = edit.moved.as_ref().filter(|(at, _)| at == edit.strref.trim());
            if let Some((_, line)) = moving {
                ui.weak(format!("Talk table: {} (added with OK)", line.text));
            } else if let Some(t) = &tlk {
                ui.weak(format!("Talk table: {t}"));
            }
            if let Some((language, feminine)) = movable {
                let at = |g: Gender| edit.entries.iter().position(|e| e.0 == language && e.1 == g);
                let (male, female) = (at(Gender::Male), at(Gender::Female));
                let name = language.name().unwrap_or("?");
                // (The game shows any text of a string's own before its
                // StrRef's line, in another language too: `engine_tlk`.)
                let others = edit.entries.iter().any(|e| e.0 != language);
                if ui
                    .add_enabled(male.is_some() && !others, egui::Button::new("Move to Talk Table"))
                    .on_hover_text(format!(
                        "Add the {name} text to the module's talk table and name it here by StrRef"
                    ))
                    .on_disabled_hover_text(match others {
                        true => format!(
                            "Its text in a language other than {name} would show before the line. \
                             Remove it first."
                        ),
                        false => format!("The string has no {name} text to move"),
                    })
                    .clicked()
                {
                    // (With plain line ends, as the game's own talk table
                    // has them all.)
                    let text = |i: usize| edit.entries[i].2.clone();
                    let male = male.expect("enabled");
                    let line = mg_module::talk::Line {
                        text: text(male),
                        feminine: feminine.then(|| text(female.unwrap_or(male))),
                        sound: String::new(),
                        sound_length: 0.0,
                    };
                    // (The line is added at OK: added here, Cancel left it
                    // in the table, named by nothing.)
                    match crate::talk_view::next_strref(app) {
                        Ok(strref) => {
                            edit.strref = strref.0.to_string();
                            edit.entries.retain(|e| {
                                e.0 != language || (e.1 == Gender::Female && !feminine)
                            });
                            edit.moved = Some((edit.strref.clone(), line));
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
                    dropdown(("loc-lang", i)).selected_text(language.name().unwrap_or("?")).show(
                        ui,
                        language,
                        &Language::EE,
                        |l| l.name().unwrap_or("?"),
                    );
                    let genders = GENDERS.map(|g| g.0);
                    dropdown(("loc-gender", i))
                        .selected_text(GENDERS.iter().find(|g| g.0 == *gender).map_or("?", |g| g.1))
                        .show(ui, gender, &genders, |g| {
                            GENDERS.iter().find(|n| n.0 == *g).map_or("?", |n| n.1)
                        });
                    // The language's own tokens (Polish has more than
                    // English), put where the caret was in its text.

                    let language = *language;
                    mg_uikit::menu_button(ui, "Token…", |ui| {
                        mg_uikit::menu_rows(ui);
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
                let field = egui::TextEdit::multiline(text).desired_rows(3).desired_width(420.0);
                text_area(ui, text_id, field);
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
                    edit.entries.push((l, g, String::new(), String::new()));
                }
            }
            ui.add_space(6.0);
            let value = edit.value();
            if value.is_none() {
                let problem = edit.problem().unwrap_or_else(|| "It cannot be saved.".into());
                ui.colored_label(ui.visuals().error_fg_color, problem);
            }
            let foot = Foot::new().main("OK", value.is_some()).cancel();
            (value, foot)
        });
        if let Some(f) = footed {
            if f.pressed.main()
                && let Some(value) = f.inner.flatten()
            {
                // The text moved to the talk table is added now, where
                // the string still names its line. (Not added: the
                // window stays, with why in the log.)
                let mut value = Some(value);
                let moved = edit.moved.take().filter(|(at, _)| at == edit.strref.trim());
                let added = moved.map(|(at, line)| {
                    let added = crate::talk_view::add_line(app, &line);
                    match &added {
                        Ok(strref) => {
                            edit.strref = strref.0.to_string();
                            value = edit.value();
                            app.log.info(format!(
                                "Added talk-table line {}; it's saved with the module",
                                strref.0
                            ));
                        }
                        Err(e) => {
                            app.log.error(format!("Move to Talk Table: {e}"));
                            edit.moved = Some((at, line));
                        }
                    }
                    added.is_ok()
                });
                if added != Some(false) {
                    if let Some(v) = value.filter(|v| *v != edit.original) {
                        app.actions.push(
                            edit.target
                                .command(&format!("Edit {}", edit.title), Value::LocString(v)),
                        );
                    }
                    close = true;
                }
            }
            if f.pressed.cancelled() {
                close = true;
            }
        }
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
        let footed = modal(&ctx, "Variables")
            .resizable(true)
            .default_size([
                (room.width() * 0.5).clamp(620.0, 900.0),
                (room.height() * 0.5).max(320.0),
            ])
            .min_size([520.0, 220.0])
            .show_foot(&ctx, |ui| {
                ui.ctx().data_mut(|d| d.remove::<bool>(picking()));
                let (problem, _) = var_body(app, ui, &mut edit, &mut help, (described, false));
                // (A color being picked: the area views draw it, as the
                // page has them; OK sets it, Cancel leaves it as it was.)
                let picked = ui.ctx().data(|d| d.get_temp::<bool>(picking())).unwrap_or(false);
                if picked && problem.is_none() {
                    let each = edit.each_merged(app);
                    app.var_preview_next.extend(each);
                }
                let foot = Foot::new().main("OK", problem.is_none()).cancel();
                ((), foot)
            });
        if let Some(f) = footed {
            if f.pressed.main() {
                let action = if edit.target.also.is_empty() {
                    edit.target.command("Edit variables", Value::List(edit.list()))
                } else {
                    edit.merged(app)
                };
                app.actions.push(action);
                close = true;
            }
            if f.pressed.cancelled() {
                close = true;
            }
        }
        let dir = app.var_set_dir.clone();
        sets_window(app, &ctx, &mut edit.sets, &VARIABLE_SETS, dir);
        edit.described = Some(help);
        app.var_edit = if close { None } else { Some(edit) };
    }

    if let Some(mut p) = app.picker.take() {
        let mut close = false;
        let footed = modal(&ctx, &p.title).show_foot(&ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Filter");
                let field = SearchField::new(
                    egui::Id::new(("resref-picker-find", &p.title)),
                    &mut p.filter,
                )
                .hint("ResRef")
                .width(200.0)
                .find_over()
                .show(ui)
                .response;
                crate::widgets::autofocus(ui, &field);
            });
            let (id, all) = (p.id, p.choices.len());
            let shown = p.shown();
            ui.weak(format!("{} of {all}", shown.len()));
            egui::Frame::group(ui.style()).show(ui, |ui| {
                let area = egui::ScrollArea::vertical().max_height(300.0);
                mg_uikit::show_rows(area, ui, shown.len(), |ui, range| {
                    ui.set_width(260.0);
                    for r in &shown[range] {
                        if ui.selectable_label(false, r.to_string()).clicked() {
                            app.picked.insert(id, *r);
                            close = true;
                        }
                    }
                });
            });
            ((), Foot::new().button("None", true).tip("Clear the field").cancel())
        });
        if let Some(f) = footed {
            if f.pressed.clicked("None") {
                app.picked.insert(p.id, ResRef::EMPTY);
                close = true;
            }
            if f.pressed.cancelled() {
                close = true;
            }
        }
        app.picker = if close { None } else { Some(p) };
    }
}

/// The text of a field a resref is typed in: only what
/// [`ResRef::typable`] takes goes in (letters, digits and `_`), in lower
/// case, as in Aurora's ResRef fields; the rest of what is typed or
/// pasted is left out. What was there before (a hak's `my-tex`, picked)
/// stays as it is.
pub(crate) struct ResRefText<'a>(pub &'a mut String);

impl egui::TextBuffer for ResRefText<'_> {
    fn is_mutable(&self) -> bool {
        true
    }

    fn as_str(&self) -> &str {
        self.0.as_str()
    }

    fn insert_text(&mut self, text: &str, char_index: egui::text::CharIndex) -> usize {
        // (Cut to the room left here, not by the field's `char_limit`:
        // that would count what is left out.)
        let room = ResRef::MAX_LEN.saturating_sub(self.0.chars().count());
        let kept: String = (text.chars().filter(|&c| ResRef::typable(c)))
            .map(|c| c.to_ascii_lowercase())
            .take(room)
            .collect();
        self.0.insert_text(&kept, char_index)
    }

    fn delete_char_range(&mut self, char_range: std::ops::Range<egui::text::CharIndex>) {
        self.0.delete_char_range(char_range);
    }

    fn type_id(&self) -> std::any::TypeId {
        std::any::TypeId::of::<ResRefText<'static>>()
    }
}

/// `add` drawn in a scope of its own (`id_salt`, as `Ui::push_id`), or,
/// where the room it took when last drawn is out of sight, that room alone
/// (as egui counts its ids, a scope is one space: those after it keep
/// theirs): a long form's parts out of sight (a bind's, an array's item)
/// are not laid out. The first frame draws every part, to learn its room,
/// and a part in sight is drawn and measured again each frame, as is one
/// holding the keyboard's focus (a field typed in and scrolled away keeps
/// it, and commits as it loses it).
pub(crate) fn in_sight<R>(
    ui: &mut Ui,
    id_salt: impl std::hash::Hash + std::fmt::Debug,
    add: impl FnOnce(&mut Ui) -> R,
) -> Option<R> {
    let key = ui.id().with(("room-in-sight", &id_salt));
    if let Some((size, focused)) = ui.data(|d| d.get_temp::<(egui::Vec2, bool)>(key)) {
        let room = egui::Rect::from_min_size(ui.cursor().min, size);
        if !focused && !ui.is_rect_visible(room) {
            ui.allocate_space(size);
            return None;
        }
    }
    let shown = ui.push_id(id_salt, add);
    let rect = shown.response.rect;
    let focused = ui
        .ctx()
        .memory(egui::Memory::focused)
        .is_some_and(|id| ui.ctx().read_response(id).is_some_and(|r| rect.contains_rect(r.rect)));
    ui.data_mut(|d| d.insert_temp(key, (rect.size(), focused)));
    Some(shown.inner)
}

/// A one-line field for typing a resref in `text` ([`ResRefText`]: at
/// most 16 characters).
pub(crate) fn resref_edit<'t>(text: &'t mut ResRefText<'_>) -> egui::TextEdit<'t> {
    mg_uikit::text_field(text)
}

/// A resref field with a picker button. Returns the new value when the user
/// types one (on focus loss) or picks one.
/// Where [`with_resref_width`] keeps the width [`resref_field`] takes.
const RESREF_WIDTH_ID: &str = "resref-field-width";

/// `add` with every [`resref_field`] in it `width` wide (a conversation's
/// panel's rows: as wide as the row leaves them, at least 16 characters).
pub(crate) fn with_resref_width<R>(ui: &mut Ui, width: f32, add: impl FnOnce(&mut Ui) -> R) -> R {
    let id = egui::Id::new(RESREF_WIDTH_ID);
    let was = ui.data(|d| d.get_temp::<f32>(id));
    ui.data_mut(|d| d.insert_temp(id, width));
    let r = add(ui);
    ui.data_mut(|d| match was {
        Some(was) => _ = d.insert_temp(id, was),
        None => d.remove::<f32>(id),
    });
    r
}

/// The width [`with_resref_width`] gives the resref fields drawn in it.
fn resref_width(ui: &Ui) -> Option<f32> {
    ui.data(|d| d.get_temp::<f32>(egui::Id::new(RESREF_WIDTH_ID)))
}

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
        // (Never past its room, its button kept beside it.)
        let button = ui.spacing().interact_size.y + 2.0 * ui.spacing().item_spacing.x + 8.0;
        let width = match (resref_width(ui), crate::blueprint::narrow(ui)) {
            (Some(width), _) => width,
            (None, true) => 160f32.min(ui.available_width() - button).max(60.0),
            (None, false) => 160.0,
        };
        let typed = commit_resref_in(&mut app.buffers, ui, id, &shown, width);
        if let Some(text) = typed {
            match ResRef::from_str(text.trim()) {
                Ok(v) => out = Some(v),
                Err(e) => app.log.error(e.to_string()),
            }
        }
        // (A square as high as the field: the board's `.field-group >
        // .btn-icon`, as a name's languages button.)
        let side = ui.spacing().interact_size.y;
        if ui
            .add(egui::Button::new("…").min_size(egui::vec2(side, side)))
            .on_hover_text(title)
            .clicked()
        {
            app.open_picker(id, title, types);
        }
    });
    out
}

/// The Variables window's list and its buttons (Add, the sets, the files),
/// what the project's descriptions say of it and why it can't be set as
/// it is (returned): drawn in the window, over its OK and Cancel, and as
/// a Properties window's Variables page ([`variables_page`]). `fit`: the
/// fields shrink to a narrow page (the window keeps its whole width,
/// which it is sized to as it opens).
fn var_body(
    app: &mut Moonglow,
    ui: &mut Ui,
    edit: &mut VarTableEdit,
    help: &mut VarHelp,
    (described, fit): (bool, bool),
) -> (Option<String>, bool) {
    // What the project's descriptions say is wrong (not of the
    // name being typed), a line each under the list.
    let notes: Vec<String> = if described {
        let typing = help.completing;
        let rows = (edit.rows.iter().enumerate()).filter(|(i, _)| Some(*i) != typing);
        let mut notes: Vec<String> = rows.filter_map(|(_, r)| help.kept_note(r)).collect();
        notes.extend(help.needed(&edit.rows, typing));
        notes
    } else {
        Vec::new()
    };
    let lines = notes.len().min(VAR_NOTES + 1) as f32;
    // (The note of several edited together as tall as it wraps: in a narrow
    // room, Edit Areas Together's section, M4's U29, it is several lines.)
    let together = together_note(edit);
    let together_tall = together.as_ref().map_or(0.0, |t| {
        let wrap = ui.available_width();
        let font = egui::TextStyle::Body.resolve(ui.style());
        let ppp = ui.ctx().pixels_per_point().to_bits();
        let id = ui.id().with("together-tall");
        let key = (edit.target.also.len(), wrap.to_bits(), font.size.to_bits(), ppp);
        ui.data(|d| d.get_temp::<((usize, u32, u32, u32), f32)>(id))
            .filter(|(k, _)| *k == key)
            .map(|(_, h)| h)
            .unwrap_or_else(|| {
                let color = ui.visuals().weak_text_color();
                let h = ui.fonts_mut(|f| f.layout(t.clone(), font, color, wrap)).size().y
                    + ui.spacing().item_spacing.y;
                ui.data_mut(|d| d.insert_temp(id, (key, h)));
                h
            })
    });
    // (The list scrolls, over the buttons kept in reach below it. In the
    // Properties panel's all-pages scroll, cap and shrink vertically so an
    // empty or short table uses only the height of its rows.)
    let in_panel_scroll = crate::blueprint::panel_scroll(ui);
    let tall = (ui.available_height() - VAR_FOOT - lines * 18.0 - together_tall).max(100.0);
    let tall = if in_panel_scroll { tall.min(260.0) } else { tall };
    // (Wider, the fields share what is spare; narrower, as a Properties
    // window's page often is, they shrink to it, down to two fifths: the
    // type, a color's swatch and Delete kept in sight.)
    let room = ui.available_width() - 210.0;
    let (name_width, value_width) = if room >= VAR_NAME_WIDTH + VAR_VALUE_WIDTH || !fit {
        let room = room.max(VAR_NAME_WIDTH + VAR_VALUE_WIDTH);
        let spare = room - VAR_NAME_WIDTH - VAR_VALUE_WIDTH;
        (VAR_NAME_WIDTH + spare * 0.45, VAR_VALUE_WIDTH + spare * 0.55)
    } else {
        let part = (room / (VAR_NAME_WIDTH + VAR_VALUE_WIDTH)).max(0.4);
        (VAR_NAME_WIDTH * part, VAR_VALUE_WIDTH * part)
    };
    let mut editing = false;
    egui::ScrollArea::vertical().max_height(tall).auto_shrink([false, in_panel_scroll]).show(
        ui,
        |ui| {
            // (A table of fields: rows 22 apart, a line between.)
            mg_uikit::table_rows(ui);
            let rule = mg_uikit::metrics::TABLE_RULE;
            egui::Grid::new("vars").num_columns(4).spacing([8.0, rule]).show(ui, |ui| {
                ui.strong("Name");
                ui.strong("Type");
                ui.strong("Value");
                ui.end_row();
                let mut remove = None;
                let mut picked = None;
                let mut picked_field: Option<egui::Id> = None;
                for i in 0..edit.rows.len() {
                    // (A minimum: a new window offers the fields no width yet,
                    // and they would keep to it.)
                    fn wide(text: &mut String, width: f32) -> egui::TextEdit<'_> {
                        mg_uikit::text_field(text)
                            .desired_width(width)
                            .min_size(egui::vec2(width, 0.0))
                    }
                    // The offer's keys, before the field takes them.
                    let offering = described && help.completing == Some(i);
                    let count = if offering { help.offered_before } else { 0 };
                    let (closed, taken) = offer_keys(ui, count, &mut help.highlighted);
                    let field = wide(&mut edit.rows[i].name, name_width);
                    let name = ui.add(field.hint_text("Name"));
                    editing |= name.has_focus();
                    let r = &edit.rows[i];
                    let desc = help.variables.describe(r.name.trim());
                    let name = match desc {
                        Some(d) => name.on_hover_ui(|ui| {
                            ui.label(description_tip(d));
                        }),
                        None => name,
                    };
                    // The described names, offered as one is typed.
                    if described {
                        // (Escape closes the offer, until the name
                        // changes; the name is still being typed.)
                        let dismissed = help
                            .dismissed
                            .as_ref()
                            .is_some_and(|(di, dn)| *di == i && dn == &r.name);
                        if closed {
                            help.completing = None;
                            help.dismissed = Some((i, r.name.clone()));
                            name.request_focus();
                        } else if (name.has_focus() || name.gained_focus()) && !dismissed {
                            help.completing = Some(i);
                            help.dismissed = None;
                        }
                        if help.completing == Some(i) {
                            let offered = help.offered(&r.name, &edit.rows, i);
                            // (Typing anew: the first again.)
                            let retyped = help.names.get(i).is_some_and(|n| *n != r.name);
                            let h = if retyped || help.highlighted >= offered.len() {
                                0
                            } else {
                                help.highlighted
                            };
                            if taken && let Some(o) = offered.get(h) {
                                picked = Some((i, o.name.clone()));
                            }
                            let over = completion(ui, i, &name, &offered, h, &mut picked);
                            let pressed = ui.input(|i| i.pointer.any_pressed());
                            let count = offered.len();
                            help.highlighted = h;
                            help.offered_before = count;
                            if picked.is_some() {
                                // (Typed on from there, from its end.)
                                picked_field = Some(name.id);
                            }
                            if count == 0
                                || picked.is_some()
                                || (!name.has_focus() && !over && pressed)
                                || (name.lost_focus() && !over)
                            {
                                help.completing = None;
                            }
                        }
                        if help.completing.is_none() {
                            help.offered_before = 0;
                        }
                    }
                    let r = &mut edit.rows[i];
                    // An object's or a location's is shown, not
                    // edited (it isn't set from text).
                    let edits =
                        VarType::of(r.kind.into()).is_some_and(|t| t.field_type().is_some());
                    ui.add_enabled_ui(edits, |ui| {
                        let kinds =
                            [VarType::Int, VarType::Float, VarType::String].map(VarType::code);
                        dropdown(("var-type", i)).selected_text(kind_name(r.kind)).show(
                            ui,
                            &mut r.kind,
                            &kinds,
                            |k| kind_name(*k),
                        );
                    });
                    let typed = desc.is_some_and(|d| d.var_type.is_none_or(|t| t.code() == r.kind));
                    let choices =
                        desc.and_then(|d| help.choices.get(&d.name).zip(help.offered.get(&d.name)));
                    match (desc, choices) {
                        (Some(d), Some((c, o))) if edits && typed && !c.list.is_empty() => {
                            editing |=
                                value_choice(ui, i, d.other, c, o, &mut r.value, value_width);
                        }
                        _ if edits && colored(r) => {
                            // A color's swatch beside its text.
                            ui.horizontal(|ui| {
                                let swatch =
                                    ui.spacing().interact_size.y + ui.spacing().item_spacing.x;
                                let vr = ui.add(wide(&mut r.value, value_width - swatch));
                                editing |= vr.has_focus();
                                color_swatch(ui, &mut r.value);
                            });
                        }
                        _ => {
                            let vr = ui.add_enabled(edits, wide(&mut r.value, value_width));
                            editing |= vr.has_focus();
                        }
                    }
                    if ui.small_button("Delete").clicked() {
                        remove = Some(i);
                    }
                    ui.end_row();
                }
                if let Some((i, name)) = picked {
                    // The field keeps the keyboard, its cursor at the
                    // end: a pattern's beginning is typed on from.
                    if let Some(id) = picked_field {
                        let end = egui::text::CCursor::new(name.chars().count());
                        let mut state =
                            egui::text_edit::TextEditState::load(ui.ctx(), id).unwrap_or_default();
                        state.cursor.set_char_range(Some(egui::text::CCursorRange::one(end)));
                        state.store(ui.ctx(), id);
                        ui.memory_mut(|m| m.request_focus(id));
                    }
                    edit.rows[i].name = name;
                }
                if let Some(i) = remove {
                    edit.rows.remove(i);
                    help.completing = None;
                }
                // A name changed to a described one: its type.
                if edit.rows.len() == help.names.len() {
                    for (r, was) in edit.rows.iter_mut().zip(&mut help.names) {
                        if r.name != *was {
                            *was = r.name.clone();
                            take_described_type(&help.variables, r);
                        }
                    }
                } else {
                    help.names = edit.rows.iter().map(|r| r.name.clone()).collect();
                }
            });
        },
    );
    let mut add_file = false;
    let mut save_file = false;
    // (Wrapping where the page is narrow: the Properties panel's, or the
    // window at the least window's size.)
    let narrow = fit || ui.available_width() < 520.0;
    let buttons = |ui: &mut Ui, add: &mut dyn FnMut(&mut Ui)| match narrow {
        true => ui.horizontal_wrapped(|ui| add(ui)),
        false => ui.horizontal(|ui| add(ui)),
    };
    buttons(ui, &mut |ui: &mut Ui| {
        if ui.button("Add").clicked() {
            edit.rows.push(VarRow::new("", 1, "0"));
        }
        ui.separator();
        // Variable sets, kept in Moonglow's data folder, and in
        // files anywhere.
        let dir = app.var_set_dir.clone();
        mg_uikit::menu_button(ui, "Add Set", |ui| {
            mg_uikit::menu_rows(ui);
            match sets_menu(ui, &mut app.log, &VARIABLE_SETS, dir.as_deref()) {
                Some(SetChoice::Add(rows)) => crate::var_sets::merge(&mut edit.rows, rows),
                Some(SetChoice::FromFile) => add_file = true,
                Some(SetChoice::Manage) => {
                    edit.sets.get_or_insert_default();
                }
                None => {}
            }
        })
        .response
        .on_hover_text("Add a saved set's variables (same names take its values)");
        if edit.set_name.is_none()
            && ui
                .add_enabled(dir.is_some() && !edit.rows.is_empty(), egui::Button::new("Save Set…"))
                .on_hover_text("Keep these variables under a name, to add elsewhere")
                .clicked()
        {
            edit.set_name = Some(String::new());
        }
        // Copied and pasted as a set file's text: to another
        // object or blueprint, in any module, or a message.
        if ui
            .add_enabled(!edit.rows.is_empty(), egui::Button::new("Copy Set"))
            .on_hover_text("Copy these variables, for Paste Set on another object or blueprint")
            .clicked()
        {
            match crate::var_sets::to_text(&edit.rows) {
                Ok(text) => ui.ctx().copy_text(text),
                Err(e) => app.log.error(format!("Copy Set: {e}")),
            }
        }
        // (This list's own: with one id for all, the paste went to whichever
        // Variables list was drawn first, another object's.)
        let paste_id = ui.id().with("var-set-paste");
        if ui
            .button("Paste Set")
            .on_hover_text(
                "Add the variables on the clipboard. Same names take their types and values.",
            )
            .clicked()
        {
            ask_paste(ui.ctx(), paste_id);
        }
        if let Some(text) = asked_paste(ui.ctx(), paste_id) {
            match crate::var_sets::from_text(&text) {
                Ok(rows) => crate::var_sets::merge(&mut edit.rows, rows),
                Err(e) => app.log.error(format!(
                    "Paste Set: the clipboard doesn't hold a variable set ({e}); \
                     nothing was changed"
                )),
            }
        }
        save_file = ui
            .add_enabled(!edit.rows.is_empty(), egui::Button::new("Save to File…"))
            .on_hover_text("Save these variables as a .vars.json file, to share")
            .clicked();
    });
    // (Several at once, in the order chosen: a later file's
    // variable of a name an earlier one has takes its value.)
    if add_file {
        for path in app.dialogs.open_files(FileKind::VariableSet, None) {
            match crate::var_sets::load_file(&path) {
                Ok(set) => crate::var_sets::merge(&mut edit.rows, set),
                Err(e) => app.log.error(format!("Add from File: {e}")),
            }
        }
    }
    if save_file
        && let Some(path) = app
            .dialogs
            .save_file(FileKind::VariableSet, Some(std::path::Path::new("variables.vars.json")))
    {
        match crate::var_sets::save_file(&path, &edit.rows) {
            Ok(p) => app.log.info(format!("Variable set saved: {}", p.display())),
            Err(e) => app.log.error(format!("Save to File: {e}")),
        }
    }
    if let Some(name) = edit.set_name.as_mut() {
        let (save, cancel) = set_name_row(ui, name);
        replaced_note(ui, &crate::var_sets::SHELF, app.var_set_dir.as_deref(), name);
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
    if let Some(t) = together {
        ui.weak(t);
    }
    (problem, editing)
}

/// The note under a Variables list edited for several at once.
fn together_note(edit: &VarTableEdit) -> Option<String> {
    (!edit.target.also.is_empty()).then(|| {
        format!(
            "{} edited together, shown as the first: the variables you add or change \
             are set on each, those you delete are deleted from each, and each keeps \
             its others.",
            edit.target.also.len() + 1
        )
    })
}

/// A Variables page's list as being edited ([`variables_page`]), and the
/// list it was read from.
#[derive(Debug, Clone)]
pub struct VarPage {
    edit: VarTableEdit,
    read: Vec<Struct>,
    as_read: Vec<Struct>,
    clean_rows: Vec<VarRow>,
    /// The frame it was last drawn in: one not drawn since is forgotten.
    drawn: u64,
    /// Whether one of this page's own variable fields currently holds keyboard focus.
    editing: bool,
}

/// A Properties window's Variables page (Carcerian's ask: variables are
/// edited all the time, and the Variables… button sits on the Advanced
/// page): the Variables window's list in place, for the VarTable `list`
/// of `target`. What is changed is set as the window's other fields are:
/// when the field typed in is let go, or at once for a row added or
/// deleted and a set added, as one command, as the window's OK sets it;
/// the list as it is anew where something else changed it (Undo, the
/// window). `locked`: why it can't be changed (another builder's area).
pub(crate) fn variables_page(
    app: &mut Moonglow,
    ui: &mut Ui,
    target: FieldTarget,
    list: &[Struct],
    locked: Option<String>,
) {
    let id = (target.key, target.path.clone());
    // (A page not drawn in the frame before is another showing of it: a
    // row added and not yet named, or a value that is no number, kept past
    // its window's Cancel, came back when the window was next opened, and
    // was set then. Pruned by frame rather than pass so multi-pass
    // layout discards keep the active page.)
    let frame = ui.ctx().cumulative_frame_nr();
    app.var_pages.retain(|_, p| p.drawn + 1 >= frame);
    let mut page = match app.var_pages.remove(&id) {
        Some(p) if p.edit.target == target && (p.read == list || p.editing) => p,
        _ => {
            let edit = VarTableEdit::new(target.clone(), list);
            let as_read = edit.list();
            let clean_rows = edit.rows.clone();
            VarPage { edit, read: list.to_vec(), as_read, clean_rows, drawn: frame, editing: false }
        }
    };
    page.drawn = frame;
    let edit = &mut page.edit;
    if edit.described.is_none() {
        edit.described = Some(VarHelp::of(app, &edit.target));
    }
    let mut help = edit.described.take().expect("just made");
    help.read_choices(app, &edit.rows);
    let described = !help.variables.is_empty();
    if let Some(why) = &locked {
        ui.colored_label(ui.visuals().warn_fg_color, format!("{why}: read-only here"));
    }
    ui.ctx().data_mut(|d| d.remove::<bool>(picking()));
    let (problem, editing) = ui
        .add_enabled_ui(locked.is_none(), |ui| {
            ui.push_id(("variables-page", target.key, &target.path), |ui| {
                var_body(app, ui, edit, &mut help, (described, true))
            })
            .inner
        })
        .inner;
    page.editing = editing;
    edit.described = Some(help);
    let dir = app.var_set_dir.clone();
    let ctx = ui.ctx().clone();
    sets_window(app, &ctx, &mut edit.sets, &VARIABLE_SETS, dir);
    // Set, once none of this page's fields is being typed in and it can be:
    // read anew after. While a color is being picked, not yet: the area views
    // show the object with it (`var_preview`), and it is set, one command, as
    // the picker closes.
    let modified = page.edit.rows != page.clean_rows || page.read != list;
    let dirty = if !modified {
        false
    } else if locked.is_none() && problem.is_none() {
        if page.read != list {
            page.as_read = VarTableEdit::new(target.clone(), list).list();
            page.read = list.to_vec();
        }
        page.edit.list() != page.as_read
    } else {
        false
    };
    let picking = ui.ctx().data(|d| d.get_temp::<bool>(picking())).unwrap_or(false);
    if picking && locked.is_none() && problem.is_none() && dirty {
        let each = page.edit.each_merged(app);
        app.var_preview_next.extend(each);
        app.var_pages.insert(id, page);
        return;
    }
    if locked.is_none() && !editing && problem.is_none() && dirty {
        let action = if page.edit.target.also.is_empty() {
            page.edit.target.command("Edit variables", Value::List(page.edit.list()))
        } else {
            page.edit.merged(app)
        };
        app.actions.push(action);
        return;
    }
    app.var_pages.insert(id, page);
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

/// The "Variables (N)…" button of an object's, an area's or the module's
/// VarTable of `count` variables: whether it was pressed.
pub(crate) fn variables_button(ui: &mut Ui, count: usize) -> bool {
    let label = format!("Variables ({count})…");
    ui.button(label).on_hover_text("Edit scripting variables").clicked()
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
            let why = "This text is in another language. Typing here gives it text in the one \
                       edited.";
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

/// A text field of a localized string as [`commit_loc_text`] makes one,
/// showing what [`loc_shown`] says and marked where that is not its own:
/// `rows` lines of it shown (one where `None`).
pub(crate) fn loc_text(
    app: &mut Moonglow,
    ui: &mut Ui,
    id: egui::Id,
    ls: &mg_core::LocString,
    rows: Option<usize>,
    width: f32,
) -> Option<String> {
    let (shown, borrowed) = loc_shown(app, ls);
    let mut commit =
        |ui: &mut Ui| commit_loc_text_in(&mut app.buffers, ui, id, &shown, rows, width, None);
    match borrowed {
        None => commit(ui),
        Some((from, why)) => borrowed_field(ui, &from, why, commit),
    }
}

/// The "…" beside a localized string's text, which opens String Edit for
/// its text in every language: whether it was clicked.
pub(crate) fn languages_button(ui: &mut Ui) -> bool {
    // (As high as the field it follows, a square: the board's
    // `.field-group > .btn-icon`.)
    let side = ui.spacing().interact_size.y;
    let button = egui::Button::new("…").min_size(egui::vec2(side, side));
    ui.add(button).on_hover_text("Edit text in multiple languages").clicked()
}

/// `current` (a text field's value, the field `id`) with `token` put
/// where the field's caret was last, in place of what was selected: at
/// the end if the caret was never in it. The caret is left after the
/// token.
pub(crate) fn with_token(ctx: &egui::Context, id: egui::Id, current: &str, token: &str) -> String {
    use egui::text::{CCursor, CCursorRange};
    let shown = to_editor(current).0;
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
    with_loc_line_ends(&out, current)
}

/// Whether a widget has the keyboard, the window having it or not. (A
/// response's own `has_focus` is false while another program has the
/// keyboard: a field that takes that for "left" throws away what was
/// typed in it when its window is left for another, by Alt + Tab.)
pub(crate) fn has_keys(r: &egui::Response) -> bool {
    r.ctx.memory(|m| m.has_focus(r.id))
}

/// A text field that commits when focus leaves it (one line, or several
/// with their line-end style kept). Returns the new text then. For a
/// file's text or a `CExoString` (a comment): a line typed into a text of
/// one line gets CRLF, as Aurora writes; a localized string's text:
/// [`commit_loc_text`].
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

/// [`commit_text`] for the text of a localized string (a description, a
/// conversation line's text): a line typed into a text of one line gets a
/// plain line end, as Aurora writes ([`with_loc_line_ends`]).
pub(crate) fn commit_loc_text(
    app: &mut Moonglow,
    ui: &mut Ui,
    id: egui::Id,
    current: &str,
    multiline: bool,
    width: f32,
) -> Option<String> {
    commit_loc_text_in(&mut app.buffers, ui, id, current, multiline.then_some(6), width, None)
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
    commit_text_hinted(buffers, ui, id, current, rows, width, limit, "")
}

/// [`commit_text_in`] with `hint` shown in the field while it is empty.
#[allow(clippy::too_many_arguments)]
pub(crate) fn commit_text_hinted(
    buffers: &mut std::collections::HashMap<egui::Id, String>,
    ui: &mut Ui,
    id: egui::Id,
    current: &str,
    rows: Option<usize>,
    width: f32,
    limit: Option<usize>,
    hint: &str,
) -> Option<String> {
    let shown = to_editor(current).0;
    let typed = commit_field(buffers, ui, id, &shown, rows, width, limit, hint, false);
    typed.map(|text| with_line_ends(&text, current))
}

/// [`commit_text_in`] for a resref: what is typed in it takes
/// [`ResRefText`]'s characters alone.
pub(crate) fn commit_resref_in(
    buffers: &mut std::collections::HashMap<egui::Id, String>,
    ui: &mut Ui,
    id: egui::Id,
    current: &str,
    width: f32,
) -> Option<String> {
    commit_field(buffers, ui, id, current, None, width, None, "", true)
}

/// [`commit_text_in`] for the text of a localized string
/// ([`commit_loc_text`]).
pub(crate) fn commit_loc_text_in(
    buffers: &mut std::collections::HashMap<egui::Id, String>,
    ui: &mut Ui,
    id: egui::Id,
    current: &str,
    rows: Option<usize>,
    width: f32,
    limit: Option<usize>,
) -> Option<String> {
    let shown = to_editor(current).0;
    let typed = commit_field(buffers, ui, id, &shown, rows, width, limit, "", false);
    typed.map(|text| with_loc_line_ends(&text, current))
}

/// The field of [`commit_text_in`], `shown` its value as the field shows
/// it ([`to_editor`]), `hint` shown while it is empty, `resref` whether
/// it takes a resref's characters alone ([`ResRefText`]): what was typed
/// (with plain line ends) when the keyboard leaves it changed.
#[allow(clippy::too_many_arguments)]
fn commit_field(
    buffers: &mut std::collections::HashMap<egui::Id, String>,
    ui: &mut Ui,
    id: egui::Id,
    shown: &str,
    rows: Option<usize>,
    width: f32,
    limit: Option<usize>,
    hint: &str,
    resref: bool,
) -> Option<String> {
    let buf = buffers.entry(id).or_insert_with(|| shown.to_owned());
    let mut name = ResRefText(&mut *buf);
    let edit = match rows {
        _ if resref => mg_uikit::text_field(&mut name),
        Some(rows) => egui::TextEdit::multiline(name.0).desired_rows(rows),
        None => mg_uikit::text_field(name.0),
    };
    let edit = if hint.is_empty() { edit } else { edit.hint_text(hint) };
    let edit = match limit {
        Some(n) => edit.char_limit(n),
        None => edit,
    };
    // (Room kept inside its right end for words drawn there: a localized
    // name's "StrRef 24650", `Form::locstring`.)
    let end = ui.data(|d| d.get_temp::<f32>(egui::Id::new(crate::blueprint::FIELD_END_ROOM)));
    let edit = match end.filter(|e| *e > 0.0) {
        Some(end) => {
            let mut margin = mg_uikit::controls::FIELD_MARGIN;
            margin.right = margin.right.saturating_add(end.round() as i8);
            edit.margin(margin)
        }
        None => edit,
    };
    let out = edit.id(id).desired_width(width).show(ui);
    if rows.is_some() {
        page_keys(ui, &out);
    }
    let r = out.response.response;
    leaves_at_a_press(ui.ctx(), &r);
    if !has_keys(&r) && !r.lost_focus() && *buf != shown {
        // Changed underneath (undo, another selection, the file read again
        // as another program changed it): show the value, in a frame asked
        // for. (Drawn already with the old: where no input followed, the
        // window rested on it until the next input.)
        *buf = shown.to_owned();
        ui.ctx().request_repaint();
    }
    (r.lost_focus() && *buf != shown).then(|| buf.clone())
}

/// A text field of several lines (`edit`, given the id `id`) drawn, with
/// Page Down and Page Up ([`page_keys`]).
pub(crate) fn text_area(
    ui: &mut Ui,
    id: egui::Id,
    edit: egui::TextEdit,
) -> egui::text_edit::TextEditOutput {
    let out = edit.id(id).show(ui);
    page_keys(ui, &out);
    out
}

/// Page Down and Page Up in a text field of several lines (`out`, as just
/// drawn), which egui's field lacks: the caret goes down or up by the lines
/// of the field in sight (less one, as a page in Windows' fields), and
/// the field scrolls with it; with Shift, the selection reaches to there.
/// Rows wrapped count as lines. Every text field of several lines but the
/// script editor's (which has its own) and a NUI window's in its preview
/// (as the game's, which has none) has them, through here.
pub(crate) fn page_keys(ui: &Ui, out: &egui::text_edit::TextEditOutput) {
    use egui::{Key, Modifiers};
    let id = out.response.id;
    if !ui.memory(|m| m.has_focus(id)) {
        return;
    }
    let galley = &out.galley;
    let row = galley.size().y / galley.rows.len().max(1) as f32;
    let seen = out.text_clip_rect.intersect(ui.clip_rect()).height();
    let page = ((seen / row.max(1.0)).floor() as usize).saturating_sub(1).max(1);
    for (key, down) in [(Key::PageDown, true), (Key::PageUp, false)] {
        // (Shift first: the key alone takes a press with Shift too.)
        let select = ui.input_mut(|i| i.consume_key(Modifiers::SHIFT, key));
        if !select && !ui.input_mut(|i| i.consume_key(Modifiers::NONE, key)) {
            continue;
        }
        let mut state = egui::TextEdit::load_state(ui.ctx(), id).unwrap_or_default();
        let Some(range) = state.cursor.char_range() else { continue };
        let (mut to, mut h_pos) = (range.primary, range.h_pos);
        for _ in 0..page {
            (to, h_pos) = match down {
                true => galley.cursor_down_one_row(&to, h_pos),
                false => galley.cursor_up_one_row(&to, h_pos),
            };
        }
        state.cursor.set_char_range(Some(egui::text::CCursorRange {
            primary: to,
            secondary: if select { range.secondary } else { to },
            h_pos,
        }));
        state.store(ui.ctx(), id);
        let at = galley.pos_from_cursor(to).translate(out.galley_pos.to_vec2());
        ui.scroll_to_rect(at.translate(egui::vec2(-galley.rect.left(), 0.0)), None);
        ui.ctx().request_repaint();
    }
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
    // (A press on a text field's menu is the field's own.)
    if text_menu_open(ctx) {
        return;
    }
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
/// number fields have: a click is one more or one less, within the range;
/// an arrow held down goes on stepping, as do Up and Down in the field.
/// Each click, key or held run is one change, made when it ends.
pub(crate) fn spin_number<T: egui::emath::Numeric>(
    ui: &mut Ui,
    current: T,
    range: std::ops::RangeInclusive<T>,
) -> Option<T> {
    spin_with(ui, current, range, 1.0, |d| d)
}

/// [`spin_with`] on a dialog's own copy of a number (a wizard's, Options',
/// Adjust Location's), set when it changes: the response is the field's
/// and its arrows', changed then.
pub(crate) fn spin_draft<T: egui::emath::Numeric>(
    ui: &mut Ui,
    value: &mut T,
    range: std::ops::RangeInclusive<T>,
    step: f64,
    make: impl for<'a> FnOnce(egui::DragValue<'a>) -> egui::DragValue<'a>,
) -> egui::Response {
    let mut r = ui.scope(|ui| spin_with(ui, *value, range, step, make));
    if let Some(v) = r.inner {
        *value = v;
        r.response.mark_changed();
    }
    r.response
}

/// How long an arrow is held before it repeats, and how often it does
/// then (seconds).
const SPIN_DELAY: f64 = 0.4;
const SPIN_RATE: f64 = 0.05;

/// A run of steps not yet made a change: the baseline value it started
/// from, the number reached, the arrow held (1 up, -1 down, 0 none: keys),
/// since when, how many steps it has made, and the last frame it was drawn.
#[derive(Clone, Copy)]
struct SpinRun {
    base: f64,
    value: f64,
    arrow: i8,
    since: f64,
    done: u32,
    frame: u64,
}

/// `v` one step `by` on, within `lo` to `hi`. A step from a number outside
/// the range comes back into it, but never goes the other way (one less
/// than 0 in 1 to 12 stays 0). A range given backwards (its start worked
/// out from a file's number, past its end) is taken the right way round:
/// `f64::clamp` panics on one.
fn spin_step(v: f64, by: f64, lo: f64, hi: f64) -> f64 {
    let (lo, hi) = (lo.min(hi), lo.max(hi));
    let to = (v + by).clamp(lo, hi);
    if (to - v) * by > 0.0 { to } else { v }
}

/// The width of a number field's arrows ([`spin_with`]).
pub(crate) const SPIN_ARROW_WIDTH: f32 = 14.0;

/// One of a number field's arrows ([`spin_with`]): [`SPIN_ARROW_WIDTH`]
/// wide, half the field's height, the upper or the lower, in the field's
/// `raised` group look (`.field-group > .btn-icon`), its chevron in
/// `text-2` (`text` hovered); pressed and held as a drag too.
struct SpinArrow<'a> {
    icon: mg_uikit::Icon,
    name: &'a str,
    height: f32,
    up: bool,
}

impl egui::Widget for SpinArrow<'_> {
    fn ui(self, ui: &mut Ui) -> egui::Response {
        let size = egui::vec2(SPIN_ARROW_WIDTH, self.height);
        let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click_and_drag());
        let enabled = ui.is_enabled();
        let name = self.name.to_string();
        response
            .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, &name));
        if ui.is_rect_visible(rect) {
            let t = mg_uikit::tokens(ui.ctx());
            let hot = enabled && (response.hovered() || response.is_pointer_button_down_on());
            let fill = if hot { t.raised_hover } else { t.raised };
            let color = match (enabled, hot) {
                (false, _) => t.text_off,
                (true, true) => t.text,
                (true, false) => t.text_2,
            };
            let r = mg_uikit::metrics::RADIUS;
            let corners = match self.up {
                true => egui::CornerRadius { ne: r, ..Default::default() },
                false => egui::CornerRadius { se: r, ..Default::default() },
            };
            ui.painter().rect_filled(rect, corners, fill);
            let side = (self.height - 1.0).clamp(6.0, mg_uikit::metrics::ICON_SMALL);
            let square = egui::Rect::from_center_size(rect.center(), egui::Vec2::splat(side));
            self.icon.paint(ui.painter(), square, color);
        }
        response
    }
}

/// [`spin_number`] by `step`, the field set up by `make` too (its speed,
/// decimals, suffix).
pub(crate) fn spin_with<T: egui::emath::Numeric>(
    ui: &mut Ui,
    current: T,
    range: std::ops::RangeInclusive<T>,
    step: f64,
    make: impl for<'a> FnOnce(egui::DragValue<'a>) -> egui::DragValue<'a>,
) -> Option<T> {
    let stepped = |v: f64, by: f64| spin_step(v, by, range.start().to_f64(), range.end().to_f64());
    // (Its label, before the field: the arrows are named with it.)
    let label = number_name(ui);
    ui.horizontal(|ui| {
        // (A field with its buttons glued on, the mock's `.field-group`:
        // 1 point between their frames, each a point past its widget.)
        ui.spacing_mut().item_spacing.x = 3.0;
        // (The field's own id: the next widget is the field.)
        let field = ui.next_auto_id();
        let run_id = field.with("spin-run");
        let frame = ui.ctx().cumulative_frame_nr();
        let base = current.to_f64();
        let mut run: Option<SpinRun> = ui
            .data(|d| d.get_temp::<SpinRun>(run_id))
            .filter(|r| r.base == base && r.frame + 1 >= frame);
        let now = ui.input(|i| i.time);
        let shown = run.map_or(base, |r| r.value);
        // Up and Down in the field (before it takes them as it would).
        let keys = [egui::Key::ArrowUp, egui::Key::ArrowDown];
        let focused = ui.memory(|m| m.has_focus(field));
        if focused {
            let n = ui.input_mut(|i| {
                i.count_and_consume_key(egui::Modifiers::NONE, keys[0]) as i64
                    - i.count_and_consume_key(egui::Modifiers::NONE, keys[1]) as i64
            });
            if n != 0 {
                let value = stepped(shown, n as f64 * step);
                run = Some(match run {
                    Some(r) => SpinRun { value, frame, ..r },
                    None => SpinRun { base, value, arrow: 0, since: now, done: 0, frame },
                });
            }
        }
        let shown = run.map_or(base, |r| r.value);
        let (typed, _, _) =
            drag_number_shown(ui, T::from_f64(shown), |d| make(d.range(range.clone())));
        // (A number typed is set as it is, the arrows still drawn.)
        if typed.is_some() {
            run = None;
        }
        let mut held = focused && ui.input(|i| keys.iter().any(|k| i.key_down(*k)));
        use mg_uikit::Icon;
        // (The arrows one over the other, glued to the field's right end
        // and as high as it together: sized with their field, M4's U10.)
        let field_h = ui.spacing().interact_size.y;
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            for (icon, words, arrow, tip) in [
                (Icon::ChevronUp, "One More", 1i8, "One more"),
                (Icon::ChevronDown, "One Less", -1i8, "One less"),
            ] {
                // (Named with the field's label, first: "Hit Points: One More".)
                let name = label.as_ref().map_or(words.to_string(), |l| format!("{l}: {words}"));
                let by = f64::from(arrow) * step;
                let can = stepped(shown, by) != shown;
                // (A drag too: egui takes a press held past a click's time
                // for no click, and the arrow for no longer pressed.)
                let half =
                    SpinArrow { icon, name: &name, height: (field_h / 2.0).floor(), up: arrow > 0 };
                let b = ui.add_enabled(can, half).on_hover_text(tip).on_disabled_hover_text(tip);
                if !b.is_pointer_button_down_on() {
                    continue;
                }
                held = true;
                match &mut run {
                    Some(r) if r.arrow == arrow => {
                        // Held: after a pause, a step each SPIN_RATE.
                        let held_for = now - r.since;
                        let due = if held_for < SPIN_DELAY {
                            1
                        } else {
                            2 + ((held_for - SPIN_DELAY) / SPIN_RATE) as u32
                        };
                        for _ in r.done..due {
                            r.value = stepped(r.value, by);
                        }
                        r.done = r.done.max(due);
                        r.frame = frame;
                    }
                    _ => {
                        run = Some(SpinRun {
                            base,
                            value: stepped(shown, by),
                            arrow,
                            since: now,
                            done: 1,
                            frame,
                        });
                    }
                }
                ui.ctx().request_repaint();
            }
        });
        if let Some(v) = typed {
            ui.data_mut(|d| d.remove::<SpinRun>(run_id));
            return (v.to_f64() != base).then_some(v);
        }
        match run {
            Some(r) if !held => {
                ui.data_mut(|d| d.remove::<SpinRun>(run_id));
                (r.value != base).then(|| T::from_f64(r.value))
            }
            Some(mut r) => {
                r.frame = frame;
                ui.data_mut(|d| d.insert_temp(run_id, r));
                None
            }
            None => {
                ui.data_mut(|d| d.remove::<SpinRun>(run_id));
                None
            }
        }
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

/// The keys of a search field (GitHub issue 12), read before the field
/// (whose id is `id`) is added: `find` (Find's keys where it takes them)
/// gives it the keyboard; in it, Escape clears what is typed, and Escape
/// again leaves it; Down leaves it for the list where `down` says so (else
/// it is the field's own).
fn field_keys(
    ui: &mut egui::Ui,
    id: egui::Id,
    text: &mut String,
    find: bool,
    down: bool,
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
    if down && has && ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown)) {
        ui.memory_mut(|m| m.surrender_focus(id));
        ui.data_mut(|d| d.insert_temp(had_id, false));
        return Some(FilterLeft::Down);
    }
    None
}

/// Whether Find's keys (Ctrl+F, or what Options › Keyboard gave it) were
/// pressed for a search field that takes them `here` (its pane has the
/// keys, the pointer is over its tab...): used up then, and not while a
/// text field has the keyboard.
fn find_here(ui: &egui::Ui, here: bool) -> bool {
    here && !ui.ctx().egui_wants_keyboard_input() && crate::keys::find_pressed(ui.ctx())
}

/// A field's clear button: a `close` icon, named Clear.
pub(crate) fn clear_button() -> mg_uikit::IconButton {
    mg_uikit::IconButton::new(mg_uikit::Icon::Close, "Clear").icon_only(true)
}

/// A small button of `text` and a small `icon` after it (a key and the
/// cross that takes it away), named `name`: what a click does ("Remove
/// Ctrl+S from Save"), which its words alone do not say.
pub(crate) fn text_then_icon(
    ui: &mut egui::Ui,
    text: &str,
    icon: mg_uikit::Icon,
    name: &str,
) -> egui::Response {
    let id = ui.next_auto_id().with("after-icon");
    let side = egui::Vec2::splat(mg_uikit::metrics::ICON_SMALL);
    let mut atoms = egui::Atoms::default();
    atoms.push_right(text.to_string());
    atoms.push_right(egui::Atom::custom(id, side));
    let drawn = egui::Button::new(atoms).small().atom_ui(ui);
    if let Some(rect) = drawn.rect(id) {
        let color = ui.style().interact(&drawn.response).text_color();
        icon.paint(ui.painter(), rect, color);
    }
    let r = drawn.response;
    r.ctx.accesskit_node_builder(r.id, |node| node.set_label(name));
    r
}

/// An icon button (22 px, frameless until hovered) named `name`, to add
/// (`ui.add_enabled`; its tip the caller's).
pub(crate) fn icon_only(icon: mg_uikit::Icon, name: &str) -> mg_uikit::IconButton {
    mg_uikit::IconButton::new(icon, name).icon_only(true)
}

/// An icon button (22 px, frameless until hovered) named `name`, with
/// `tip` (or its name) under the pointer, disabled too.
pub(crate) fn icon_button(
    ui: &mut egui::Ui,
    icon: mg_uikit::Icon,
    name: &str,
    tip: &str,
) -> egui::Response {
    let tip = if tip.is_empty() { name } else { tip };
    let r = ui.add(icon_only(icon, name));
    r.on_hover_text(tip).on_disabled_hover_text(tip)
}

/// A list's search field: what it looks in said in the field while it is
/// empty (`hint`), a button to clear it, and its keys (GitHub issue 12):
/// Find's (Ctrl+F) go to it where the caller says
/// ([`SearchField::find_when`], [`SearchField::find_over`]); in it, Escape
/// clears what is typed, and Escape again leaves it; Down leaves it for
/// the list where the list has keys of its own ([`SearchField::down`]).
/// What it finds is the caller's, by [`mg_core::text::Needle`].
pub(crate) struct SearchField<'a> {
    id: egui::Id,
    text: &'a mut String,
    hint: &'a str,
    clear_tip: &'a str,
    clear_name: &'a str,
    find: Option<bool>,
    find_over: bool,
    down: bool,
    width: Option<f32>,
    icon: mg_uikit::Icon,
    name: Option<&'a str>,
    clear_inside: bool,
}

/// What a [`SearchField`] did: the field's response, and how its keys
/// left it, if they did.
pub(crate) struct Searched {
    pub response: egui::Response,
    pub left: Option<FilterLeft>,
}

impl<'a> SearchField<'a> {
    pub(crate) fn new(id: egui::Id, text: &'a mut String) -> SearchField<'a> {
        SearchField {
            id,
            text,
            hint: "",
            clear_tip: "Clear the search",
            clear_name: "Clear",
            find: None,
            find_over: false,
            down: false,
            width: None,
            icon: mg_uikit::Icon::Search,
            name: None,
            clear_inside: false,
        }
    }

    /// Its clear button inside the field's box at its right end, as the
    /// boards draw it (`.field .end`, the Hak board's Find), the field
    /// `width` wide with it (M4's U21 review); else after the field.
    pub(crate) fn clear_inside(mut self) -> Self {
        self.clear_inside = true;
        self
    }

    /// Its name for a screen reader (and tests), apart from what it says
    /// while empty: the module tree's "Filter Module Tree", told from a
    /// dropdown's Filter that says the same.
    pub(crate) fn name(mut self, name: &'a str) -> Self {
        self.name = Some(name);
        self
    }

    /// The icon before what it looks in (the glass, where not given; the
    /// module tree's Filter has the filter's funnel, as the mock's).
    pub(crate) fn icon(mut self, icon: mg_uikit::Icon) -> Self {
        self.icon = icon;
        self
    }

    /// What the field says while it is empty.
    pub(crate) fn hint(mut self, hint: &'a str) -> Self {
        self.hint = hint;
        self
    }

    /// What the pointer over its clear button says.
    pub(crate) fn clear_tip(mut self, tip: &'a str) -> Self {
        self.clear_tip = tip;
        self
    }

    /// What its clear button is named, for a screen reader and tests
    /// ("Clear", where not given): a pane's own ("Clear Filter", the module
    /// tree's), apart from the palettes' beside it.
    pub(crate) fn clear_name(mut self, name: &'a str) -> Self {
        self.clear_name = name;
        self
    }

    /// Find's keys go to it when `here` (a pane's: while the pane has the
    /// keys).
    pub(crate) fn find_when(mut self, here: bool) -> Self {
        self.find = Some(here);
        self
    }

    /// Find's keys go to it while the pointer is over what the `ui` it is
    /// shown in shows (a tab's or a window's).
    pub(crate) fn find_over(mut self) -> Self {
        self.find_over = true;
        self
    }

    /// Down leaves it for the list ([`FilterLeft::Down`]).
    pub(crate) fn down(mut self) -> Self {
        self.down = true;
        self
    }

    /// Its width, clear button aside (else the room there is).
    pub(crate) fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    pub(crate) fn show(self, ui: &mut egui::Ui) -> Searched {
        let SearchField {
            id,
            text,
            hint,
            clear_tip,
            clear_name,
            find,
            find_over,
            down,
            width,
            icon,
            name,
            clear_inside,
        } = self;
        let here = find.unwrap_or(false) || (find_over && ui.rect_contains_pointer(ui.clip_rect()));
        let find = find_here(ui, here);
        let left = field_keys(ui, id, text, find, down);
        let response = ui
            .horizontal(|ui| {
                let clear = !text.is_empty();
                // (The field and its clear button fill the room together:
                // the button as wide as it was last drawn.)
                let button_id = id.with("clear-width");
                let button = ui.data(|d| d.get_temp::<f32>(button_id)).unwrap_or(26.0);
                let button = if clear && !clear_inside { button } else { 0.0 };
                let held = crate::blueprint::narrow(ui);
                let room = width.map_or(ui.available_width() - button, |w| match held {
                    true => w.min(ui.available_width() - button),
                    false => w,
                });
                let room = room.max(60.0);
                // (Every search field as the palette's and the module
                // tree's: the glass before what it looks in, an owner's
                // ask; drawn in the field, its words after it.)
                let field = mg_uikit::controls::FIELD_MARGIN;
                let glass = mg_uikit::metrics::ICON as i8 + 4;
                let mut margin = egui::Margin { left: field.left + glass, ..field };
                // (Room at its right for the clear button drawn in it.)
                const INSIDE: f32 = 22.0;
                if clear_inside {
                    margin.right += INSIDE as i8;
                }
                let r = ui.add(
                    mg_uikit::text_field(&mut *text)
                        .id(id)
                        .margin(margin)
                        .hint_text(hint)
                        .desired_width(room),
                );
                if let Some(name) = name {
                    r.ctx.accesskit_node_builder(r.id, |node| node.set_label(name));
                }
                if ui.is_rect_visible(r.rect) {
                    let side = egui::Vec2::splat(mg_uikit::metrics::ICON);
                    let at = egui::pos2(r.rect.left() + f32::from(field.left), r.rect.center().y);
                    let rect = egui::Rect::from_min_size(at - egui::vec2(0.0, side.y / 2.0), side);
                    let color = ui.visuals().weak_text_color();
                    icon.paint(ui.painter(), rect, color);
                }
                if clear && clear_inside {
                    let at = egui::pos2(r.rect.right() - INSIDE / 2.0, r.rect.center().y);
                    let rect = egui::Rect::from_center_size(at, egui::vec2(INSIDE, INSIDE));
                    let b = ui.put(rect, clear_button().name(clear_name)).on_hover_text(clear_tip);
                    if b.clicked() {
                        text.clear();
                    }
                } else if clear {
                    let b = ui.add(clear_button().name(clear_name)).on_hover_text(clear_tip);
                    let wide = b.rect.width() + ui.spacing().item_spacing.x;
                    if (wide - button).abs() > 0.1 {
                        ui.data_mut(|d| d.insert_temp(button_id, wide));
                        ui.ctx().request_repaint();
                    }
                    if b.clicked() {
                        text.clear();
                    }
                }
                r
            })
            .inner;
        // (After the field: typed in this frame counts for the next
        // frame's Escape.)
        search_field_shown(ui.ctx(), id, !text.is_empty());
        Searched { response, left }
    }
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
    /// The search fields shown this frame ([`search_field_shown`]), and
    /// whether something was typed in each.
    searches: Vec<(egui::Id, bool)>,
    /// Whether a search field had the keyboard as the last frame ended
    /// ([`dialogs_end`]), and so whether this frame's Escape is the
    /// field's (it clears what is typed, then leaves it); and whether
    /// something was typed in it.
    searching: bool,
    searching_text: bool,
    leaves_search: bool,
    /// This frame's Escape clears a search field's text: the dialog it is
    /// in stays open ([`escape_on`]).
    clears_search: bool,
    /// A popup (a menu, a combo box's list) or a kit popover (the area
    /// header's) was open as this frame began: its Escape, which closes
    /// it, is no one else's ([`escape_closes_a_popup`]).
    popup: bool,
    /// A text field (one line or several, a number being typed, a script's
    /// name) had the keyboard as the last frame ended ([`dialogs_end`]):
    /// this frame's Enter or Escape is the field's, which egui has let go
    /// of before anything is drawn ([`field_had_keyboard`]).
    field: bool,
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
    // (Nor a kit popover's, which egui's popups do not count: the
    // header's Lighting, its More. The Escape closes it, and nothing
    // else.)
    d.popup = egui::Popup::is_any_open(ctx) || mg_uikit::popover::Popover::any_open(ctx);
    d.escape = ctx.input(|i| i.key_pressed(egui::Key::Escape) && i.modifiers.is_none()) && !d.popup;
    d.leaves_search = d.escape && d.searching;
    d.clears_search = d.escape && d.searching_text;
    ctx.data_mut(|x| x.insert_temp(dialogs_id(), d));
}

/// This frame's Escape was something's in a dialog (it closed the names
/// a field offers): the dialog stays open.
pub(crate) fn keep_escape(ctx: &egui::Context) {
    ctx.data_mut(|x| {
        let d = x.get_temp_mut_or_default::<Dialogs>(dialogs_id());
        d.escape = false;
        d.leaves_search = false;
        d.clears_search = false;
    });
}

/// Once a frame, after everything is drawn: whether a search field has
/// the keyboard. (egui gives the keyboard up at an Escape before anything
/// of the frame is drawn, so a window can't tell then that it was typed
/// in.)
pub(crate) fn dialogs_end(ctx: &egui::Context) {
    let focused = ctx.memory(|m| m.focused());
    ctx.data_mut(|x| {
        let d = x.get_temp_mut_or_default::<Dialogs>(dialogs_id());
        let searches = std::mem::take(&mut d.searches);
        let typed = focused.and_then(|f| searches.iter().find(|(id, _)| *id == f));
        d.searching = typed.is_some();
        d.searching_text = typed.is_some_and(|(_, text)| *text);
    });
    let field = focused.is_some_and(|id| egui::text_edit::TextEditState::load(ctx, id).is_some());
    ctx.data_mut(|x| x.get_temp_mut_or_default::<Dialogs>(dialogs_id()).field = field);
}

/// Whether a text field had the keyboard as the last frame ended (M4's
/// U29 review): this frame's Enter commits it and its Escape gives it up,
/// and neither is a tool section's main action or Cancel, though egui has
/// let go of the field before the section reads the key.
pub(crate) fn field_had_keyboard(ctx: &egui::Context) -> bool {
    ctx.data(|x| x.get_temp::<Dialogs>(dialogs_id())).is_some_and(|d| d.field)
}

/// Whether a search field had the keyboard as the last frame ended: what
/// is typed now is the search's, before this frame's fields are drawn
/// ([`crate::blueprint::typing_kept_out`]).
pub(crate) fn search_has_keyboard(ctx: &egui::Context) -> bool {
    ctx.data(|x| x.get_temp::<Dialogs>(dialogs_id())).is_some_and(|d| d.searching)
}

/// A search field (`id`, its text field's) shown this frame, `typed` in or
/// not: Escape in it is its own ([`escape_left_search`]), and a dialog it
/// is in stays open while it clears what is typed ([`escape_on`]).
fn search_field_shown(ctx: &egui::Context, id: egui::Id, typed: bool) {
    ctx.data_mut(|x| {
        x.get_temp_mut_or_default::<Dialogs>(dialogs_id()).searches.push((id, typed));
    });
}

/// Whether this frame's Escape was in a search field (a list's filter, an
/// inventory's Find), whose Escape clears what is typed and then leaves
/// it: a Properties window stays open. In any other field Escape is the
/// window's Cancel at once, as Aurora's is.
pub(crate) fn escape_left_search(ctx: &egui::Context) -> bool {
    ctx.data(|x| x.get_temp::<Dialogs>(dialogs_id())).is_some_and(|d| d.leaves_search)
}

/// Escape for the dialog on `layer`: true when it was pressed and this is
/// the dialog opened last. Each dialog asks every frame it shows, which is
/// also how it is known to be open. Not while it clears what was typed
/// in a search field of it (Open Script's name, a wizard's Find): the next
/// Escape closes it.
fn escape_on(ctx: &egui::Context, layer: egui::LayerId) -> bool {
    ctx.data_mut(|x| {
        let d = x.get_temp_mut_or_default::<Dialogs>(dialogs_id());
        if !d.seen.contains(&layer) {
            d.seen.push(layer);
        }
        d.escape && !d.clears_search && d.open.last() == Some(&layer)
    }) && !modal_over(ctx, layer)
}

/// A scrolled dialog body or page (`ui`, its content) brought to the control that
/// has just taken the focus (Tab, Shift+Tab), where the body's view cuts
/// it: egui's Tab moves the focus but leaves the view where it was, and at
/// the least window a body's lower controls were focused out of sight
/// (U27's review). Only as the focus moves, so a builder's own scrolling
/// stays.
pub(crate) fn focus_in_sight(ui: &Ui) {
    // (Not in a frame with a press or a click: egui's `read_response`
    // gives up the focus of a widget a press landed outside, and it took
    // the keys from the field a click on its Hide box had given them back
    // to, a project command's input. The focus Tab moves comes with no
    // press.)
    let pressed =
        ui.input(|i| i.pointer.any_pressed() || i.pointer.any_click() || i.pointer.any_released());
    if pressed {
        return;
    }
    let Some(id) = ui.ctx().memory(|m| m.focused()) else { return };
    let Some(r) = ui.ctx().read_response(id) else { return };
    let mine = r.layer_id == ui.layer_id() && ui.min_rect().contains_rect(r.rect);
    if mine && r.gained_focus() && !ui.clip_rect().contains_rect(r.rect) {
        ui.scroll_to_rect(r.rect, None);
    }
}

/// Whether an `egui::Modal` other than the one on `layer` is up (a job's
/// window, a plugin's question): a window under it takes neither Escape nor
/// Enter. One key, one window, the topmost.
fn modal_over(ctx: &egui::Context, layer: egui::LayerId) -> bool {
    ctx.memory(|m| m.top_modal_layer()).is_some_and(|top| top != layer)
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

/// The paste a button asked for ([`ask_paste`]): whose, and in which pass.
#[derive(Clone, Copy)]
struct PasteAsked {
    id: egui::Id,
    pass: u64,
}

/// Where [`PasteAsked`] is kept, and the text that came for it.
fn paste_ids() -> (egui::Id, egui::Id) {
    (egui::Id::new("mg-paste-asked"), egui::Id::new("mg-paste-came"))
}

/// The passes an asked paste is waited for (the backend sends it the next
/// frame; an empty clipboard sends nothing).
const PASTE_WAIT: u64 = 10;

/// A Paste button's press (Paste Set, Paste Color): asks the backend for
/// the clipboard (`RequestPaste`), whose text comes as a later frame's
/// Paste. That paste is the button's (`id`): [`pastes_begin`] takes it
/// from the frame's events before anything else reads them (a
/// conversation's Paste of lines, a focused field), and
/// [`asked_paste`] gives it to the button.
pub(crate) fn ask_paste(ctx: &egui::Context, id: egui::Id) {
    ctx.send_viewport_cmd(egui::ViewportCommand::RequestPaste);
    ctx.request_repaint();
    let pass = ctx.cumulative_pass_nr();
    ctx.data_mut(|d| {
        d.insert_temp(paste_ids().0, PasteAsked { id, pass });
        d.remove::<(egui::Id, String, u64)>(paste_ids().1);
    });
}

/// Once a frame, first: the paste a button asked for ([`ask_paste`]),
/// taken from the frame's events and kept for it. One that doesn't come
/// (nothing on the clipboard) is waited for until a key or a mouse
/// button is pressed, or for [`PASTE_WAIT`] passes: a later Ctrl+V is
/// then the field's or the editor's, not the button's.
pub(crate) fn pastes_begin(ctx: &egui::Context) {
    let (asked_id, came_id) = paste_ids();
    let Some(asked) = ctx.data(|d| d.get_temp::<PasteAsked>(asked_id)) else { return };
    let now = ctx.cumulative_pass_nr();
    if now <= asked.pass {
        return;
    }
    let came = ctx.input_mut(|i| {
        let at = i.events.iter().position(|e| matches!(e, egui::Event::Paste(_)))?;
        match i.events.remove(at) {
            egui::Event::Paste(text) => Some(text),
            _ => None,
        }
    });
    let moved_on = ctx.input(|i| {
        i.events.iter().any(|e| {
            matches!(
                e,
                egui::Event::Key { pressed: true, .. }
                    | egui::Event::PointerButton { pressed: true, .. }
            )
        })
    });
    ctx.data_mut(|d| match came {
        Some(text) => {
            d.remove::<PasteAsked>(asked_id);
            d.insert_temp(came_id, (asked.id, text, now));
        }
        None if moved_on || now > asked.pass + PASTE_WAIT => d.remove::<PasteAsked>(asked_id),
        None => {}
    });
}

/// The clipboard's text a button asked for with [`ask_paste`] under `id`,
/// in the frame it came (a button not drawn then, its page left, has
/// none later).
pub(crate) fn asked_paste(ctx: &egui::Context, id: egui::Id) -> Option<String> {
    // (Where the frame didn't begin with them: a widget's own test.)
    pastes_begin(ctx);
    let came_id = paste_ids().1;
    let came: Option<(egui::Id, String, u64)> = ctx.data(|d| d.get_temp(came_id));
    let (by, text, pass) = came?;
    if by != id && pass + 1 >= ctx.cumulative_pass_nr() {
        return None;
    }
    ctx.data_mut(|d| d.remove::<(egui::Id, String, u64)>(came_id));
    (by == id && pass + 1 >= ctx.cumulative_pass_nr()).then_some(text)
}

/// What a text field's right-click menu ([`text_menu`]) does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TextAction {
    /// The field's own history's step back (the script editor's alone:
    /// [`TextUndo`]).
    Undo,
    Redo,
    Cut,
    Copy,
    Paste,
    SelectAll,
}

/// The script editor's Undo and Redo in its menu: whether each has a step
/// to take, and their keys (Options › Keyboard's).
pub(crate) struct TextUndo {
    pub can_undo: bool,
    pub can_redo: bool,
    pub undo_keys: String,
    pub redo_keys: String,
}

/// A text field's menu, open.
#[derive(Clone, Copy)]
struct TextMenuOpen {
    field: egui::Id,
    /// Where it stands: the pointer's place at the right click, under the
    /// caret for Shift+F10.
    at: egui::Pos2,
    /// The field's text can't be changed: Cut and Paste are off.
    read_only: bool,
    /// A password's field (its text shown as dots): Cut and Copy are off,
    /// egui's field copies nothing from it.
    password: bool,
    /// The field's layer: the menu's popup is counted on it, so that tips
    /// there wait while the menu is open.
    layer: egui::LayerId,
    /// Something is selected: Cut and Copy are on.
    selected: bool,
    /// The row the keys are at (Up and Down), counting every row.
    lit: Option<usize>,
    /// Enter (or Space) pressed: the lit row is chosen as it is drawn.
    choose: bool,
}

/// The text fields' menus' state, kept between frames.
#[derive(Clone, Default)]
struct TextMenus {
    open: Option<TextMenuOpen>,
    /// The rows of the menu last drawn, on or off: where Up and Down go.
    rows: Vec<bool>,
    /// The selection of the field that had the keys as a right press
    /// began (egui's field puts its caret where any button is pressed).
    pressed: Option<(egui::Id, egui::text::CCursorRange)>,
    /// What a row asked of its field, done as the next frame begins, the
    /// field still holding the keys: the event its keys send.
    pending: Option<(egui::Id, TextAction)>,
    /// The fields that drew their own menu this pass (the script
    /// editor's, with its own rows): left to it.
    own: (u64, Vec<egui::Id>),
    /// The fields whose text can't be changed this pass though their text
    /// could be (a form only looked at: `blueprint::typing_kept_out`).
    kept_out: (u64, Vec<egui::Id>),
    /// How egui gave the keys up at a click elsewhere before the menu
    /// opened: while the pointer is on the open menu, a click there keeps
    /// them in the field (a plugin's row, the menu's margin). A click
    /// elsewhere gives them up as it would without the menu.
    surrender: Option<egui::SurrenderFocusOn>,
    /// The menu as last drawn, and its rows (counting every row): a press
    /// on a row is the menu's alone ([`text_menu_input`]).
    menu_rect: Option<egui::Rect>,
    row_rects: Vec<egui::Rect>,
    /// The submenus open from it (a plugin's `menu`: egui's own, each its
    /// own area, which can lie outside the menu): a press there is on the
    /// menu too.
    sub_rects: Vec<egui::Rect>,
    /// A press taken by the menu, on the row it was on (an enabled one's):
    /// its release is the menu's too, and chooses that row if it comes
    /// over it.
    swallow: Option<(egui::PointerButton, Option<usize>)>,
    /// A key pressed closed the menu and its text did not come with it:
    /// that text, if it comes first in the next frame, is not the field's
    /// either.
    typed_away: bool,
}

impl TextMenus {
    /// Whether `pos` is on the open menu or one of its submenus.
    fn on_menu(&self, pos: egui::Pos2) -> bool {
        self.open.is_some()
            && (self.menu_rect.is_some_and(|r| r.contains(pos))
                || self.sub_rects.iter().any(|r| r.contains(pos)))
    }
}

fn text_menus_id() -> egui::Id {
    egui::Id::new("mg-text-menus")
}

/// The id of the text fields' menu (an egui popup, kept open by
/// [`TextMenus`] and not by egui's one open popup, so that a field inside
/// a popup, a dropdown's Filter, keeps its popup open under it).
fn text_menu_id() -> egui::Id {
    egui::Id::new("mg-text-menu")
}

/// The id a text menu's Paste asks the clipboard under ([`ask_paste`]).
fn text_paste_id() -> egui::Id {
    egui::Id::new("mg-text-menu-paste")
}

fn text_menus(ctx: &egui::Context) -> TextMenus {
    ctx.data(|d| d.get_temp::<TextMenus>(text_menus_id())).unwrap_or_default()
}

fn keep_text_menus(ctx: &egui::Context, m: TextMenus) {
    ctx.data_mut(|d| d.insert_temp(text_menus_id(), m));
}

/// Whether a text field's menu is open (a press on it is not a press
/// elsewhere: [`text_begin`]).
pub(crate) fn text_menu_open(ctx: &egui::Context) -> bool {
    ctx.data(|d| d.get_temp::<TextMenus>(text_menus_id())).is_some_and(|m| m.open.is_some())
}

/// Says that the field `id`'s text can't be changed this frame, though it
/// is drawn as if it could (a form only looked at): its menu has no Cut or
/// Paste.
pub(crate) fn text_kept_out(ctx: &egui::Context, id: egui::Id) {
    let pass = ctx.cumulative_pass_nr();
    let mut m = text_menus(ctx);
    if m.kept_out.0 != pass {
        m.kept_out = (pass, Vec::new());
    }
    m.kept_out.1.push(id);
    keep_text_menus(ctx, m);
}

/// First in a frame, before anything reads its events: what a text
/// menu's row asked last frame is sent to its field as its keys would send
/// it (Copy, Cut, the clipboard's text as a Paste once it came, Ctrl+A),
/// so the field does it as it does for them (its own undo, what its owner
/// does with a change); a right press's selection is noted
/// ([`keep_selection`]); and while the pointer is on an open menu, a click
/// there leaves the keys in the field. (Its keys and presses are taken
/// before egui reads them: [`text_menu_input`].)
pub(crate) fn text_menus_begin(ctx: &egui::Context) {
    use egui::{Event, Key, Modifiers};
    if ctx.plugin_opt::<TextMenuInput>().is_none() {
        ctx.add_plugin(TextMenuInput);
    }
    let mut m = text_menus(ctx);
    if let Some((field, action)) = m.pending {
        let keys = ctx.memory(|mm| mm.has_focus(field));
        let event = match action {
            TextAction::Copy => Some(Event::Copy),
            TextAction::Cut => Some(Event::Cut),
            TextAction::SelectAll => Some(Event::Key {
                key: Key::A,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Modifiers::COMMAND,
            }),
            TextAction::Paste => {
                let came = asked_paste(ctx, text_paste_id());
                let waiting = (ctx.data(|d| d.get_temp::<PasteAsked>(paste_ids().0)))
                    .is_some_and(|a| a.id == text_paste_id());
                if came.is_none() && waiting {
                    keep_text_menus(ctx, m.clone());
                }
                came.map(Event::Paste)
            }
            TextAction::Undo | TextAction::Redo => None,
        };
        // (A paste still on its way stays pending until the text comes.)
        let still_asked = action == TextAction::Paste
            && event.is_none()
            && ctx.data(|d| d.get_temp::<PasteAsked>(paste_ids().0)).is_some();
        if !still_asked {
            m.pending = None;
        }
        if let Some(e) = event.filter(|_| keys) {
            ctx.input_mut(|i| i.events.insert(0, e));
        }
    }
    m.pressed = None;
    if ctx.input(|i| i.pointer.button_pressed(egui::PointerButton::Secondary)) {
        m.pressed = ctx.memory(|mm| mm.focused()).and_then(|id| {
            let range = egui::text_edit::TextEditState::load(ctx, id)?.cursor.char_range()?;
            (range.primary != range.secondary).then_some((id, range))
        });
    }
    // (Only on the menu: elsewhere a click gives the keys up as it does
    // without it, and a field that sets its text then sets it.)
    let on_menu = ctx.input(|i| i.pointer.interact_pos()).is_some_and(|p| m.on_menu(p));
    let surrender = ctx.options(|o| o.input_options.surrender_focus_on);
    match (on_menu, m.surrender) {
        (true, None) => {
            m.surrender = Some(surrender);
            ctx.options_mut(|o| o.input_options.surrender_focus_on = egui::SurrenderFocusOn::Never);
        }
        (false, Some(was)) => {
            m.surrender = None;
            ctx.options_mut(|o| o.input_options.surrender_focus_on = was);
        }
        _ => {}
    }
    keep_text_menus(ctx, m);
}

/// The context's plugin that gives an open text menu its input before
/// egui reads it ([`text_menu_input`]).
struct TextMenuInput;

impl egui::Plugin for TextMenuInput {
    fn debug_name(&self) -> &'static str {
        "mg-ui text menu input"
    }

    fn input_hook(&mut self, ctx: &egui::Context, input: &mut egui::RawInput) {
        text_menu_input(ctx, input);
    }
}

/// While a text field's menu is open, before egui reads the frame's input:
/// - Up and Down move along its rows, Enter (and Space on a row) choose
///   the row there, Escape and Tab close it. (A plugin's submenu is the
///   pointer's alone.)
/// - Anything else typed (a letter, a key not the menu's, Ctrl+C, an input
///   method's text) closes it and is the menu's, not the field's, as a
///   desktop menu takes the keys while it is open: the field is left as it
///   was.
/// - A press on one of its rows, and its release, are the menu's alone: no
///   popup under it (a dropdown's list whose Filter it serves) takes them
///   for a click outside it. The release over the row pressed chooses it;
///   one dragged off it chooses nothing, the menu left open.
/// - A press elsewhere on it or its submenus (a plugin's row) is theirs; a
///   press outside them closes it and goes on to what it landed on, as it
///   would have without the menu.
/// - The window losing the keys closes it, and forgets a press taken
///   whose release never comes.
fn text_menu_input(ctx: &egui::Context, input: &mut egui::RawInput) {
    use egui::{Event, Key};
    let mut m = text_menus(ctx);
    if m.open.is_none() && m.swallow.is_none() && !m.typed_away {
        return;
    }
    let events = std::mem::take(&mut input.events);
    // (The text of the key that closed the menu last frame, if it comes
    // first now: one text, before any other key.)
    let mut its_text = std::mem::take(&mut m.typed_away);
    // (A key pressed that closed the menu this frame, its text not yet
    // come.)
    let mut awaiting_text = false;
    let mut typed_away = false;
    let mut space_taken = false;
    for e in events {
        // (A new press of a button whose taken press never had its release:
        // that one is gone.)
        if let Event::PointerButton { button, pressed: true, .. } = &e
            && m.swallow.is_some_and(|(b, _)| b == *button)
        {
            m.swallow = None;
        }
        if its_text {
            its_text = false;
            if matches!(e, Event::Text(_)) {
                continue;
            }
        }
        let keep = match (&e, &mut m.open) {
            (Event::WindowFocused(false) | Event::PointerGone, _) => {
                if matches!(e, Event::WindowFocused(false)) {
                    m.open = None;
                }
                m.swallow = None;
                true
            }
            (Event::PointerButton { pos, button, pressed: false, .. }, open)
                if m.swallow.is_some_and(|(b, _)| b == *button) =>
            {
                let pressed_on = m.swallow.take().and_then(|(_, row)| row);
                let over = m.row_rects.iter().position(|r| r.contains(*pos));
                if let (Some(open), Some(row)) = (open, pressed_on)
                    && over == Some(row)
                {
                    open.lit = Some(row);
                    open.choose = true;
                }
                false
            }
            (Event::PointerButton { pos, button, pressed: true, .. }, Some(_)) => {
                let row = m
                    .menu_rect
                    .is_some_and(|r| r.contains(*pos))
                    .then(|| m.row_rects.iter().position(|r| r.contains(*pos)));
                match row {
                    Some(Some(i)) => {
                        let on = *button == egui::PointerButton::Primary
                            && m.rows.get(i).copied().unwrap_or(false);
                        m.swallow = Some((*button, on.then_some(i)));
                        false
                    }
                    // (A plugin's row, a submenu's, the margin: theirs, the
                    // keys left in the field, `text_menus_begin`.)
                    _ if m.on_menu(*pos) => true,
                    _ => {
                        m.open = None;
                        true
                    }
                }
            }
            (Event::Key { key, pressed, repeat, .. }, Some(open)) => {
                let rows = &m.rows;
                match key {
                    Key::ArrowDown | Key::ArrowUp => {
                        if *pressed && !rows.is_empty() {
                            open.lit = next_row(rows, open.lit, *key == Key::ArrowDown);
                        }
                        false
                    }
                    Key::Enter => {
                        open.choose |= *pressed && open.lit.is_some();
                        false
                    }
                    Key::Space if open.lit.is_some() => {
                        open.choose |= *pressed;
                        space_taken = true;
                        false
                    }
                    Key::Escape | Key::Tab => {
                        if *pressed {
                            m.open = None;
                        }
                        false
                    }
                    // (Its own too, doing nothing: no submenu to go to.
                    // A key held down from before it opened, Shift+F10's,
                    // repeats into nothing.)
                    Key::ArrowLeft | Key::ArrowRight => false,
                    _ if *repeat => false,
                    _ if *pressed => {
                        m.open = None;
                        typed_away = true;
                        awaiting_text = true;
                        false
                    }
                    // (Let go of a key pressed before the menu opened.)
                    _ => true,
                }
            }
            (Event::Text(t), Some(_)) if space_taken && t == " " => false,
            (Event::Text(_) | Event::Copy | Event::Cut | Event::Paste(_), Some(_)) => {
                m.open = None;
                typed_away = true;
                false
            }
            // (An input method's text: it closes the menu and is the
            // menu's, but none of what it sends later.)
            (Event::Ime(_), Some(_)) => {
                m.open = None;
                false
            }
            // (The rest of a frame typed away in: not the field's either.)
            (Event::Key { pressed: true, .. }, None) if typed_away => false,
            (Event::Text(_), None) if typed_away => {
                awaiting_text = false;
                false
            }
            (Event::Copy | Event::Cut | Event::Paste(_), None) if typed_away => false,
            (Event::Ime(_), None) => {
                awaiting_text = false;
                true
            }
            _ => true,
        };
        if keep {
            input.events.push(e);
        }
    }
    m.typed_away = awaiting_text;
    keep_text_menus(ctx, m);
}

/// The next row on from `from` (down, or up), passing over rows that are
/// off, round from the last to the first.
fn next_row(rows: &[bool], from: Option<usize>, down: bool) -> Option<usize> {
    let n = rows.len();
    let mut at = from;
    for _ in 0..n {
        let i = match (at, down) {
            (None, true) => 0,
            (None, false) => n - 1,
            (Some(i), true) => (i + 1) % n,
            (Some(i), false) => (i + n - 1) % n,
        };
        at = Some(i);
        if rows[i] {
            return Some(i);
        }
    }
    from
}

/// A right click in the selection of the field `field` (just drawn)
/// keeps it, as in editors: egui's field puts the caret where any button
/// is pressed. Outside the selection, the caret stays where the click put
/// it. The selection kept, if it was.
pub(crate) fn keep_selection(
    ctx: &egui::Context,
    field: egui::Id,
) -> Option<egui::text::CCursorRange> {
    let pressed = ctx.input(|i| {
        i.pointer.button_pressed(egui::PointerButton::Secondary)
            || i.pointer.button_down(egui::PointerButton::Secondary)
    });
    if !pressed {
        return None;
    }
    let (id, range) = text_menus(ctx).pressed?;
    if id != field {
        return None;
    }
    let mut state = egui::text_edit::TextEditState::load(ctx, field)?;
    let caret = state.cursor.char_range()?.primary.index;
    let (a, b) = (range.primary.index, range.secondary.index);
    if caret < a.min(b) || caret > a.max(b) {
        return None;
    }
    state.cursor.set_char_range(Some(range));
    state.store(ctx, field);
    Some(range)
}

/// The text field `field`'s right-click menu (just drawn, in the frame
/// it was): Cut, Copy, Paste and Select All, each off where it does
/// nothing (nothing selected, a text that can't be changed), with the
/// keys that do the same; `undo` puts the field's own Undo and Redo
/// before them, and `extra` adds rows after them (the script editor's).
/// Opened by a right click on it, or by Shift+F10 while it has the keys
/// (under its caret, its first row lit: Up and Down, Enter, Escape). A row
/// chosen leaves the keys in the field and does what its keys would do
/// there ([`text_menus_begin`]); Undo and Redo are given back to do.
/// [`text_menus_end`] gives every field that draws no menu of its own this
/// one.
pub(crate) fn text_menu<R>(
    ctx: &egui::Context,
    field: egui::Id,
    undo: Option<TextUndo>,
    extra: impl FnOnce(&mut Ui, &mut TextMenuRows) -> R,
) -> (Option<TextAction>, Option<R>) {
    let mut m = text_menus(ctx);
    let has_keys = ctx.memory(|mm| mm.has_focus(field));
    let keyed =
        has_keys && ctx.input_mut(|i| i.consume_key(egui::Modifiers::SHIFT, egui::Key::F10));
    let clicked = secondary_clicked_on(ctx, field);
    if clicked || keyed {
        let selected = egui::text_edit::TextEditState::load(ctx, field)
            .and_then(|s| s.cursor.char_range())
            .is_some_and(|r| r.primary != r.secondary);
        // (A field whose text can change, with the keys, tells egui's
        // input method where its caret is; one that can't does not.)
        let ime = ctx.output(|o| o.ime.map(|i| (i.cursor_rect, i.purpose)));
        let password = ime.is_some_and(|(_, p)| p == egui::IMEPurpose::Password);
        let ime = ime.map(|(r, _)| r);
        // (Read here alone: a right click on the field, which keeps its
        // focus, or Shift+F10 after the field is drawn.)
        let response = ctx.read_response(field);
        let pass = ctx.cumulative_pass_nr();
        let kept_out = m.kept_out.0 == pass && m.kept_out.1.contains(&field);
        let at = match (keyed, ctx.pointer_interact_pos()) {
            (false, Some(p)) => p,
            _ => ime.map(|r| r.left_bottom()).unwrap_or_else(|| {
                response.as_ref().map_or(egui::Pos2::ZERO, |r| r.rect.left_bottom())
            }),
        };
        let layer = response.map_or(egui::LayerId::background(), |r| r.layer_id);
        m.open = Some(TextMenuOpen {
            field,
            at,
            read_only: ime.is_none() || kept_out,
            password,
            layer,
            selected,
            lit: None,
            choose: false,
        });
        m.rows.clear();
        m.menu_rect = None;
        m.row_rects.clear();
        ctx.request_repaint();
    }
    let Some(mut open) = m.open.filter(|o| o.field == field) else {
        keep_text_menus(ctx, m);
        return (None, None);
    };
    // (Gone with the keys: another field took them, or the field isn't
    // drawn.)
    // (`read_response` gives up the focus of a field a click landed
    // outside, before it is drawn; while the menu is open no click gives
    // up a focus, `text_menus_begin`.)
    if !has_keys || ctx.read_response(field).is_none() {
        m.open = None;
        keep_text_menus(ctx, m);
        return (None, None);
    }
    if keyed && open.lit.is_none() {
        open.lit = Some(usize::from(undo.as_ref().is_some_and(|u| !u.can_undo && !u.can_redo)));
    }
    // (Escape and the arrows stay the menu's: egui would give the keys up
    // at Escape, or move them at an arrow, before the frame's widgets.)
    ctx.memory_mut(|mm| {
        mm.set_focus_lock_filter(
            field,
            egui::EventFilter {
                tab: true,
                horizontal_arrows: true,
                vertical_arrows: true,
                escape: true,
            },
        );
    });
    let mut rows = TextMenuRows {
        lit: open.lit,
        choose: open.choose,
        at: 0,
        on: Vec::new(),
        rects: Vec::new(),
        picked: false,
    };
    let mut chosen = None;
    let shown = egui::Popup::new(text_menu_id(), ctx.clone(), open.at, open.layer)
        .kind(egui::PopupKind::Menu)
        .style(mg_uikit::menu_style)
        .open(true)
        .show(|ui| {
            mg_uikit::menu_rows(ui);
            // (As the menus show keys: Cmd on macOS.)
            let mac = ui.ctx().os() == egui::os::OperatingSystem::Mac;
            let shortcut = |key| {
                crate::keys::shown(&egui::KeyboardShortcut::new(egui::Modifiers::COMMAND, key), mac)
            };
            if let Some(u) = &undo {
                if rows.row(ui, "Undo", &u.undo_keys, u.can_undo) {
                    chosen = Some(TextAction::Undo);
                }
                if rows.row(ui, "Redo", &u.redo_keys, u.can_redo) {
                    chosen = Some(TextAction::Redo);
                }
                mg_uikit::menu_separator(ui);
            }
            let copy = open.selected && !open.password;
            if rows.row(ui, "Cut", &shortcut(egui::Key::X), copy && !open.read_only) {
                chosen = Some(TextAction::Cut);
            }
            if rows.row(ui, "Copy", &shortcut(egui::Key::C), copy) {
                chosen = Some(TextAction::Copy);
            }
            if rows.row(ui, "Paste", &shortcut(egui::Key::V), !open.read_only) {
                chosen = Some(TextAction::Paste);
            }
            mg_uikit::menu_separator(ui);
            if rows.row(ui, "Select All", &shortcut(egui::Key::A), true) {
                chosen = Some(TextAction::SelectAll);
            }
            extra(ui, &mut rows)
        });
    open.choose = false;
    let picked = chosen.is_some() || rows.picked;
    let closed = shown.as_ref().is_none_or(|s| s.response.should_close());
    m.menu_rect = shown.as_ref().map(|s| s.response.rect);
    // (Its submenus' areas, each as egui's menus keep which is open.)
    m.sub_rects.clear();
    let mut menu = text_menu_id();
    for _ in 0..8 {
        let open = egui::containers::menu::MenuState::from_id(ctx, menu, |s| s.open_item);
        let Some(sub) = open else { break };
        m.sub_rects.extend(ctx.memory(|mm| mm.area_rect(sub)));
        menu = sub;
    }
    let extra = shown.map(|s| s.inner);
    m.rows = rows.on;
    m.row_rects = rows.rects;
    m.open = (!picked && !closed).then_some(open);
    match chosen {
        Some(TextAction::Paste) => {
            ask_paste(ctx, text_paste_id());
            m.pending = Some((field, TextAction::Paste));
        }
        Some(a @ (TextAction::Cut | TextAction::Copy | TextAction::SelectAll)) => {
            m.pending = Some((field, a));
            ctx.request_repaint();
        }
        _ => {}
    }
    keep_text_menus(ctx, m);
    let undone = chosen.filter(|a| matches!(a, TextAction::Undo | TextAction::Redo));
    (undone, extra)
}

/// Whether the field `field`'s menu is open, or asked to open this frame
/// (a right click on it, Shift+F10 while it has the keys): when a field
/// that draws its own ([`own_text_menu`]) works out its rows.
pub(crate) fn text_menu_wanted(ctx: &egui::Context, field: egui::Id) -> bool {
    let open = text_menus(ctx).open.is_some_and(|o| o.field == field);
    let clicked = secondary_clicked_on(ctx, field);
    let keyed = ctx.memory(|mm| mm.has_focus(field))
        && ctx.input(|i| i.key_pressed(egui::Key::F10) && i.modifiers.shift_only());
    open || clicked || keyed
}

/// Whether the widget `id` was right-clicked (or long-touched) this frame,
/// as its `Response::secondary_clicked` says, without `read_response`:
/// that gives up the focus of a widget a click landed outside, so read
/// before a text field is drawn it took the keys from the field and its
/// selection with them (Replace then found no match selected).
fn secondary_clicked_on(ctx: &egui::Context, id: egui::Id) -> bool {
    let (clicked, long) =
        ctx.interaction_snapshot(|s| (s.clicked == Some(id), s.long_touched == Some(id)));
    long || (clicked && ctx.input(|i| i.pointer.button_clicked(egui::PointerButton::Secondary)))
}

/// Says that the field `field` draws its own menu ([`text_menu`] with its
/// rows) this frame: [`text_menus_end`] leaves it be.
pub(crate) fn own_text_menu(ctx: &egui::Context, field: egui::Id) {
    let pass = ctx.cumulative_pass_nr();
    let mut m = text_menus(ctx);
    if m.own.0 != pass {
        m.own = (pass, Vec::new());
    }
    m.own.1.push(field);
    keep_text_menus(ctx, m);
}

/// Last in a frame: the text field with the keys gets [`text_menu`]
/// (every egui text field of the window, through whatever helper or none
/// it was drawn), unless it drew its own ([`own_text_menu`]). Only where
/// a right click, Shift+F10 or an open menu asks: a frame at rest looks
/// at nothing. A text field is told by the state egui keeps for one.
pub(crate) fn text_menus_end(ctx: &egui::Context) {
    let m = text_menus(ctx);
    let asked = m.open.is_some()
        || ctx.input(|i| {
            i.pointer.button_pressed(egui::PointerButton::Secondary)
                || i.pointer.button_clicked(egui::PointerButton::Secondary)
                || i.events.iter().any(|e| {
                    matches!(
                        e,
                        egui::Event::Key { key: egui::Key::F10, pressed: true, modifiers, .. }
                            if modifiers.shift_only()
                    )
                })
        });
    if !asked {
        return;
    }
    let pass = ctx.cumulative_pass_nr();
    let Some(field) = ctx.memory(|mm| mm.focused()) else {
        if m.open.is_some() {
            close_text_menu(ctx);
        }
        return;
    };
    if m.own.0 == pass && m.own.1.contains(&field) {
        return;
    }
    if egui::text_edit::TextEditState::load(ctx, field).is_none() {
        if m.open.is_some() {
            close_text_menu(ctx);
        }
        return;
    }
    keep_selection(ctx, field);
    text_menu(ctx, field, None, |_, _| ());
}

/// Closes the text fields' menu.
fn close_text_menu(ctx: &egui::Context) {
    let mut m = text_menus(ctx);
    m.open = None;
    keep_text_menus(ctx, m);
}

/// The rows of a text field's menu ([`text_menu`]), for the keys: each
/// counted, lit where the keys are at it, chosen by a click or by Enter
/// there.
pub(crate) struct TextMenuRows {
    lit: Option<usize>,
    choose: bool,
    at: usize,
    on: Vec<bool>,
    rects: Vec<egui::Rect>,
    picked: bool,
}

impl TextMenuRows {
    /// A row named `name`, `keys` at its right, off where not `enabled`:
    /// whether it was chosen.
    pub(crate) fn row(&mut self, ui: &mut Ui, name: &str, keys: &str, enabled: bool) -> bool {
        let at = self.lit == Some(self.at);
        self.at += 1;
        self.on.push(enabled);
        let button = egui::Button::new(name).selected(at && enabled);
        let r = ui.add_enabled_ui(enabled, |ui| mg_uikit::menu_row_with_keys(ui, button, keys));
        self.rects.push(r.response.rect);
        let chosen = enabled && (r.inner.clicked() || (at && self.choose));
        self.picked |= chosen;
        chosen
    }
}

/// Once a frame, before anything reads the events: text pasted (or sent
/// as typed) with CRLF line ends, or a lone CR, comes with plain ones, as
/// every text field holds its text ([`to_editor`]). A field that kept the
/// `\r` would have a stop of the caret no one sees.
pub(crate) fn plain_pastes(ctx: &egui::Context) {
    ctx.input_mut(|i| {
        for e in &mut i.events {
            if let egui::Event::Paste(s) | egui::Event::Text(s) = e
                && s.contains('\r')
            {
                *s = s.replace("\r\n", "\n").replace('\r', "\n");
            }
        }
    });
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

/// What the name of a new resource of type `t` starts with, where it is
/// typed (New Script…, New Conversation…): the builder's code where the
/// team's pattern puts it (`ab_`; `_ab` with `{name}_{code}`), the
/// team's prefix without one, nothing in a module with neither (The
/// Domains of Dread's builder proposal, C1).
pub(crate) fn code_start(app: &crate::Moonglow, t: mg_core::ResType) -> String {
    app.ws.as_ref().map(|ws| ws.module.project_file.naming_for(t).fit("", "")).unwrap_or_default()
}

/// Puts a text field's cursor at `at` (characters), from the next frame.
pub(crate) fn place_cursor(ui: &egui::Ui, field: &egui::Response, at: usize) {
    let mut state = egui::TextEdit::load_state(ui.ctx(), field.id).unwrap_or_default();
    let at = egui::text::CCursor::new(at);
    state.cursor.set_char_range(Some(egui::text::CCursorRange::one(at)));
    state.store(ui.ctx(), field.id);
}

/// Whether a field is shown for the first time (since it was last away):
/// [`autofocus`] focuses it then.
fn first_shown(ui: &egui::Ui, field: &egui::Response) -> bool {
    let seen = field.id.with("autofocus");
    let last: Option<u64> = ui.data(|d| d.get_temp(seen));
    last.is_none_or(|l| l + 1 < ui.ctx().cumulative_pass_nr())
}

/// A window that asks for a new resource's name (New Script, New
/// Conversation, Save Script As).
pub(crate) struct NameWindow<'a> {
    /// It names a new resource, whose name starts with the builder's code
    /// ([`code_start`]): the cursor is put where the name goes.
    pub new: bool,
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
        let footed = modal(ctx, self.title).show_foot(ctx, |ui| {
            ui.label(self.label);
            let field = ui.add(resref_edit(&mut ResRefText(name)));
            // (Where the name goes: after `ab_`, before a code that goes after
            // it, `{name}_ab`.)
            if self.new
                && first_shown(ui, &field)
                && let Some(m) = module
            {
                let naming = m.project_file.naming_for(self.restype);
                if !naming.is_plain() && *name == naming.fit("", "") {
                    place_cursor(ui, &field, naming.before().chars().count());
                }
            }
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
            (key, Foot::new().main(self.button, may).cancel())
        });
        if let Some(f) = footed {
            if f.pressed.main()
                && let Some(Some(k)) = f.inner
            {
                out = Named::Chosen(k);
            }
            if f.pressed.cancelled() {
                out = Named::Cancelled;
            }
        }
        out
    }
}

/// Where a dialog opens: its middle at the middle of the application's
/// window (the Dialogs and Wizard boards: "it opens in the middle of the
/// main window"; the Area Wizard there stands at the window's middle, not
/// the editor area's).
pub(crate) fn dialog_center(ctx: &egui::Context) -> egui::Pos2 {
    ctx.content_rect().center()
}

/// A window of its own over the application's, in the middle of it as it
/// opens ([`dialog_center`]; by its middle: a window that grows, a
/// wizard's next page, grows about it), with the boards' title row
/// ([`Dialog`]) and moved by it. Every dialog is one, as [`modal`] or
/// [`dialog`] make it.
fn centered<'a>(ctx: &egui::Context, title: impl Into<egui::WidgetText>) -> Dialog<'a> {
    let title = title.into().text().to_string();
    let window = egui::Window::new(title.as_str())
        .pivot(egui::Align2::CENTER_CENTER)
        .default_pos(dialog_center(ctx));
    // (egui's windows resize unless told not to.)
    Dialog {
        id: egui::Id::new(Some(title.as_str())),
        window,
        title,
        open: None,
        resizes: true,
        close_cancels: false,
        folds: true,
        flush: false,
        sized: false,
    }
}

/// A dialog's window (an `egui::Window` with the methods its makers use)
/// with the boards' title row: egui's own title bar off, a row of the
/// `bar` fill across the window's top with the title at its left and,
/// on a dialog that Escape closes ([`dialog`]), a Close button at its
/// right (`mg_uikit::title_row`; the boards: "only on a dialog that Esc
/// closes"). The window is moved by the row alone: a drag on its body
/// that no control takes is caught behind it. The title stays the
/// window's name (`egui::Window::new`), which screen readers and tests
/// find it by, and its id, which [`escape_closes`] reads.
#[must_use = "show it"]
pub(crate) struct Dialog<'a> {
    window: egui::Window<'a>,
    title: String,
    id: egui::Id,
    /// Where it has a Close: its `open`, which the Close clears.
    open: Option<&'a mut bool>,
    /// Whether it is resized by its edges (it then shows the grip).
    resizes: bool,
    /// A Close that is its Cancel ([`Dialog::close_as_cancel`]).
    close_cancels: bool,
    /// Whether it folds to its title row by the arrow at the row's left
    /// (egui's windows fold unless told not to: [`modal`] says so).
    folds: bool,
    /// Its body laid out to the window's edges ([`Dialog::flush`]).
    flush: bool,
    /// Given a size to open at ([`Dialog::default_size`]): its footer
    /// stands at the window's foot. One not given a size fits its content
    /// (its footer under the body), resized by its edges or not.
    sized: bool,
}

/// The least width of a dialog's body inside its margin: egui's first
/// width for a window's (`egui::Resize`'s default), which every dialog
/// had at least while egui drew its title bar.
const DIALOG_LEAST_BODY: f32 = 320.0;

/// A dialog's body at most `width` wide, and narrower where its room is
/// (the least window): its words wrapped to what it has, never cut by the
/// window's edge (M4's U27 review).
pub(crate) fn fit_width(ui: &mut Ui, width: f32) {
    // (By the application's window, not the room the body has now: a
    // dialog that grows to its body had the width it opened at.)
    ui.set_max_width(width.min(body_room_width(ui.ctx())));
}

/// The widest a dialog's body can be: the application's window less the
/// dialog's margin and edges, and the room of a scroll bar beside it (a
/// body that must scroll at the least window, 386 points there).
pub(crate) fn body_room_width(ctx: &egui::Context) -> f32 {
    let style = ctx.global_style();
    let edges = 2.0 * style.visuals.window_stroke.width;
    let bar = style.spacing.scroll.allocated_width();
    ctx.content_rect().width() - 2.0 * mg_uikit::dialog::BODY_PADDING - edges - bar
}

/// A dialog's body at least `width` wide, where it has that room.
pub(crate) fn fit_min_width(ui: &mut Ui, width: f32) {
    ui.set_min_width(width.min(body_room_width(ui.ctx())));
}

/// The tip of a dialog's Close.
pub(crate) const DIALOG_CLOSE_TIP: &str = "Close the dialog (Esc)";

/// What [`Dialog::show_parts`] gives back from its window: the body's
/// value and, where it has a footer, what was pressed.
type Parts<R> = (R, Option<Pressed>);

impl<'a> Dialog<'a> {
    /// A Close in its title row that answers it as its Cancel button does
    /// ([`cancel`], [`cancel_discard`], which read it the next frame): a
    /// picker's, which the boards draw with one (Select Portrait, Select
    /// Texture), Escape closing it already.
    pub(crate) fn close_as_cancel(mut self) -> Self {
        self.close_cancels = true;
        self
    }

    /// Whether it folds to its title row (`true` unless said: egui's
    /// windows did): the arrow at the row's left, as a floating editor's
    /// bar has it.
    pub(crate) fn collapsible(mut self, collapsible: bool) -> Self {
        self.folds = collapsible;
        self
    }

    pub(crate) fn resizable(mut self, resizable: impl Into<egui::Vec2b>) -> Self {
        let resizable = resizable.into();
        self.resizes = resizable.any();
        self.window = self.window.resizable(resizable);
        self
    }

    pub(crate) fn default_width(mut self, width: f32) -> Self {
        self.window = self.window.default_width(width);
        self
    }

    pub(crate) fn default_size(mut self, size: impl Into<egui::Vec2>) -> Self {
        self.window = self.window.default_size(size);
        self.sized = true;
        self
    }

    pub(crate) fn default_pos(mut self, pos: impl Into<egui::Pos2>) -> Self {
        self.window = self.window.default_pos(pos);
        self
    }

    pub(crate) fn pivot(mut self, pivot: egui::Align2) -> Self {
        self.window = self.window.pivot(pivot);
        self
    }

    pub(crate) fn id(mut self, id: egui::Id) -> Self {
        self.window = self.window.id(id);
        self.id = id;
        self
    }

    pub(crate) fn min_width(mut self, width: f32) -> Self {
        self.window = self.window.min_width(width);
        self
    }

    pub(crate) fn min_height(mut self, height: f32) -> Self {
        self.window = self.window.min_height(height);
        self
    }

    pub(crate) fn min_size(mut self, size: impl Into<egui::Vec2>) -> Self {
        self.window = self.window.min_size(size);
        self
    }

    pub(crate) fn max_width(mut self, width: f32) -> Self {
        self.window = self.window.max_width(width);
        self
    }

    /// The body laid out to the window's edges, with no margin of its own
    /// (a wizard's step list and page, which keep their own room).
    pub(crate) fn flush(mut self) -> Self {
        self.flush = true;
        self
    }

    /// Shows it with no footer, the title row first and `add_contents`
    /// under it with the body's room (12 points, the Dialogs board's): a
    /// dialog with no buttons of its own (About, a report whose title
    /// row's Close and Escape close it). As `egui::Window::show` (nothing
    /// where it is closed; no body, `None` within, where it is folded to
    /// its row). Never taller or wider than the application's window: a
    /// body that would make it so (as it was last frame) scrolls in the
    /// room there is, so its title row and every button stay in reach.
    pub(crate) fn show_bare<R>(
        self,
        ctx: &egui::Context,
        add_contents: impl FnOnce(&mut Ui) -> R,
    ) -> Option<egui::InnerResponse<Option<R>>> {
        let shown = self.show_parts(ctx, false, |ui| (add_contents(ui), None))?;
        Some(egui::InnerResponse { inner: shown.inner.map(|(r, _)| r), response: shown.response })
    }

    /// [`Dialog::show_bare`] with the dialog pattern's footer (M4's U27):
    /// `add_contents` lays out the body and ends by saying its footer
    /// ([`Foot`]), which is laid out under it across the dialog's whole
    /// width, outside the body's room and its scroll, the rule from edge
    /// to edge, never cut (the dialog is at least as wide as its buttons
    /// need); the body scrolls in the room the footer leaves. What the
    /// buttons, Enter and Escape did is in the [`Footed`] given back, for
    /// the caller to act on once the window is drawn.
    pub(crate) fn show_foot<R>(
        self,
        ctx: &egui::Context,
        add_contents: impl FnOnce(&mut Ui) -> (R, Foot),
    ) -> Option<Footed<R>> {
        let shown = self.show_parts(ctx, true, |ui| {
            let (r, foot) = add_contents(ui);
            (r, Some(foot))
        })?;
        let (inner, pressed) = match shown.inner {
            Some((r, pressed)) => (Some(r), pressed.unwrap_or_default()),
            None => (None, Pressed::default()),
        };
        Some(Footed { inner, pressed, response: shown.response })
    }

    fn show_parts<R>(
        self,
        ctx: &egui::Context,
        footed: bool,
        add_contents: impl FnOnce(&mut Ui) -> (R, Option<Foot>),
    ) -> Option<egui::InnerResponse<Option<Parts<R>>>> {
        let Dialog { mut window, title, id, open, resizes, close_cancels, folds, flush, sized } =
            self;
        // (A footer's height and the spacing above it, which its place
        // under the body takes.)
        let style = ctx.global_style();
        let footer_high = match footed {
            true => mg_uikit::dialog::FOOTER_HEIGHT + style.spacing.item_spacing.y,
            false => 0.0,
        };
        if open.as_deref() == Some(&false) {
            return None;
        }
        let close = open.is_some() || close_cancels;
        // (Where its body was last frame: a drag there that no control
        // takes is caught, not the window's.)
        let body_id = id.with("mg-dialog-body");
        let body: Option<egui::Rect> = ctx.data(|d| d.get_temp(body_id));
        let margin = match flush {
            true => egui::Margin::ZERO,
            false => egui::Margin::same(mg_uikit::dialog::BODY_PADDING as i8),
        };
        let frame = egui::Frame::window(&style).inner_margin(0);
        // The room for its body: the application's window, less its title
        // row, margin and frame. What the body took last frame says
        // whether it scrolls in it.
        let screen = ctx.content_rect();
        let edges = 2.0 * style.visuals.window_stroke.width;
        let room = egui::vec2(
            screen.width() - margin.sum().x - edges,
            // (And the spacing under the row, which the body is put after.)
            screen.height()
                - mg_uikit::metrics::TITLE_ROW
                - style.spacing.item_spacing.y
                - footer_high
                - margin.sum().y
                - edges,
        )
        .max(egui::Vec2::splat(1.0));
        let natural_id = id.with("mg-dialog-natural");
        let natural: Option<egui::Vec2> = ctx.data(|d| d.get_temp(natural_id));
        let scroll = natural.map(|n| egui::Vec2b::new(n.x > room.x + 0.5, n.y > room.y + 0.5));
        let scroll = scroll.filter(|s| s.any());
        // Folded to its row (its height as it was kept to open it so).
        let fold_id = id.with("mg-dialog-folded");
        let fold: Option<Fold> = folds.then(|| ctx.data(|d| d.get_temp(fold_id))).flatten();
        match fold {
            Some(Fold::Folded(_)) => {
                // (Its row alone, as wide as it was: `titled`.)
                window = window.resizable(false);
            }
            None => {}
        }
        let folded = matches!(fold, Some(Fold::Folded(_)));
        let mut closed = false;
        let mut fold_clicked = false;
        let mut row_bottom = None;
        let mut took = None;
        let shown = window.title_bar(false).frame(frame).show(ctx, |ui| {
            if let Some(body) = body.filter(|_| !folded) {
                // (`Sense::DRAG` alone: `Sense::drag()` is focusable, and
                // the catcher was a stop of Tab with no name.)
                ui.interact(body, body_id, egui::Sense::DRAG);
            }
            let row = Row {
                title: &title,
                close: close.then_some(DIALOG_CLOSE_TIP),
                fold: folds.then_some(folded),
                folded_width: match fold {
                    Some(Fold::Folded(size)) => Some(size.x - edges),
                    _ => None,
                },
                margin,
                grows: !resizes,
                fills: resizes && sized,
            };
            let body = |ui: &mut Ui| match scroll {
                // (Its words wrapped to the room, `fit_width`: what scrolls
                // sideways is cut, which the tests of the dialog pattern
                // hold against, M4's U27 review.)
                Some(both) => {
                    let out = egui::ScrollArea::new(both)
                        .id_salt(natural_id)
                        .max_width(room.x)
                        .max_height(room.y)
                        .show(ui, |ui| {
                            let out = add_contents(ui);
                            focus_in_sight(ui);
                            out
                        });
                    let (inner, foot) = out.inner;
                    ((inner, Some(out.content_size)), foot)
                }
                None => {
                    let (inner, foot) = add_contents(ui);
                    ((inner, None), foot)
                }
            };
            let (inner, shown, natural) = titled_footed(ui, row, footed, body);
            closed = shown.closed();
            fold_clicked = shown.fold_clicked();
            row_bottom = Some(shown.rect.bottom());
            inner.map(|((inner, content), pressed)| {
                took = Some(content.unwrap_or(natural));
                (inner, pressed)
            })
        });
        let shown =
            shown.map(|s| egui::InnerResponse { inner: s.inner.flatten(), response: s.response });
        if let Some(took) = took {
            ctx.data_mut(|d| d.insert_temp(natural_id, took));
        }
        // (A dialog shown with no footer, noted as such for the tests.)
        if !footed && !folded && shown.is_some() && is_recording_footers(ctx) {
            let seen = FooterSeen {
                title: title.clone(),
                rect: None,
                window: egui::Rect::NOTHING,
                note: None,
                check: None,
                buttons: Vec::new(),
                enter_withheld: false,
                layer: shown.as_ref().map_or(egui::LayerId::background(), |s| s.response.layer_id),
                margin: 0.0,
            };
            record_footer(ctx, seen);
        }
        if let Some(s) = &shown {
            window_seen(ctx, &title, s.response.rect, margin.left as f32);
        }
        // The arrow: folded, with its size (its row as wide); shown again,
        // nothing kept (egui keeps the size a window's edges were left at).
        let next = match (fold, fold_clicked, &shown) {
            (None, true, Some(s)) => Some(Fold::Folded(s.response.rect.size())),
            (Some(Fold::Folded(h)), false, _) => Some(Fold::Folded(h)),
            _ => None,
        };
        match next {
            Some(f) => ctx.data_mut(|d| {
                d.insert_temp(fold_id, f);
            }),
            None => ctx.data_mut(|d| {
                d.remove::<Fold>(fold_id);
            }),
        }
        if fold_clicked {
            ctx.request_repaint();
        }
        if resizes
            && !folded
            && let Some(shown) = &shown
        {
            let window = shown.response.rect;
            // (Lit where egui resizes it by the corner, no further.)
            let hot = mg_uikit::window::resize_grip_hot(ctx, window, shown.response.layer_id);
            let painter = ctx.layer_painter(shown.response.layer_id);
            mg_uikit::paint_resize_grip(&painter, window, hot);
        }
        match (&shown, row_bottom) {
            (Some(shown), Some(bottom)) if !folded => {
                let mut body = shown.response.rect;
                body.min.y = bottom;
                ctx.data_mut(|d| d.insert_temp(body_id, body));
            }
            _ => ctx.data_mut(|d| d.remove::<egui::Rect>(body_id)),
        }
        if closed && let Some(open) = open {
            *open = false;
        }
        if closed
            && close_cancels
            && let Some(shown) = &shown
        {
            // (Its Cancel reads it as the window is drawn next.)
            let layer = shown.response.layer_id;
            ctx.data_mut(|d| d.insert_temp(closed_as_cancel_id(layer), true));
            ctx.request_repaint();
        }
        shown
    }
}

/// A dialog folded to its title row ([`Dialog::collapsible`]), with the
/// size it had.
#[derive(Clone, Copy, Debug)]
enum Fold {
    Folded(egui::Vec2),
}

/// What a title row ([`titled`]) holds and how its window takes room.
struct Row<'t> {
    title: &'t str,
    /// The Close's tip, where it has one.
    close: Option<&'t str>,
    /// Whether it is folded, where it folds.
    fold: Option<bool>,
    /// Folded: the width it keeps (its row alone, no body).
    folded_width: Option<f32>,
    /// The body's margin.
    margin: egui::Margin,
    /// Whether the window grows to what it holds (not resized by its
    /// edges).
    grows: bool,
    /// Resized by its edges and given a size: its footer at the window's
    /// foot, the body in the room above (M4's U27 review: one not given
    /// a size fits its content, with no dead room under it).
    fills: bool,
}

/// The boards' title row over `add_contents` (laid out with the row's
/// margin round it) in a window or modal with no margin of its own: its
/// room kept first, the row painted across the width the body made
/// (`mg_uikit::title_row_with`); and where the body says a footer
/// ([`Foot`]), the footer under it across the window's width, outside the
/// body's margin ([`Dialog::show_foot`]). What the body gave with the
/// footer's presses (none where it is folded, the row alone), the row
/// shown, and the size the body took inside its margin.
fn titled_footed<R>(
    ui: &mut Ui,
    row: Row<'_>,
    footed: bool,
    add_contents: impl FnOnce(&mut Ui) -> (R, Option<Foot>),
) -> (Option<(R, Option<Pressed>)>, mg_uikit::TitleRow, egui::Vec2) {
    let Row { title, close, fold, folded_width, margin, grows, fills } = row;
    // (Where the window grows to what it holds (`grows`, not resized by
    // its edges): as wide as its body was last frame, at least, and never
    // under egui's first width for a window's body (`egui::Resize`'s 320
    // and the margin), as egui's own title bar kept such a dialog: one
    // with a grid, whose columns are known only after its first frame,
    // keeps the width it opened at, not a frame narrower and moved about
    // its middle under a click. One resized by its edges is as wide as
    // it is made, and its row with it: as egui's title bar, it does not
    // hold it. Never wider than the application's window, though.)
    let mut least = mg_uikit::title_row_width_with(ui, title, close.is_some(), fold.is_some());
    if grows {
        let screen = ui.ctx().content_rect().width() - 2.0;
        let wide = ui.response().rect.width().max(DIALOG_LEAST_BODY + margin.sum().x);
        least = least.max(wide.min(screen));
    }
    if let Some(width) = folded_width {
        least = least.max(width);
    }
    let (kept, _) = ui
        .allocate_exact_size(egui::vec2(least, mg_uikit::metrics::TITLE_ROW), egui::Sense::hover());
    let (inner, took) = match folded_width {
        Some(_) => (None, egui::Vec2::ZERO),
        None if fills && footed => {
            // (Resized by its edges: the footer at the window's foot
            // whatever the body holds, the body in the room above it,
            // cut to it: a body that needs more scrolls itself.)
            let room = ui.available_rect_before_wrap();
            let high = mg_uikit::dialog::FOOTER_HEIGHT + ui.spacing().item_spacing.y;
            let body_room =
                egui::Rect::from_min_max(room.min, egui::pos2(room.right(), room.bottom() - high));
            let mut child = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(body_room)
                    .layout(egui::Layout::top_down(egui::Align::Min)),
            );
            child.set_clip_rect(body_room.intersect(ui.clip_rect()));
            let body = egui::Frame::NONE.inner_margin(margin).show(&mut child, |ui| {
                ui.set_min_height((body_room.height() - margin.sum().y).max(0.0));
                add_contents(ui)
            });
            let took = body.response.rect.size() - margin.sum();
            ui.advance_cursor_after_rect(body_room);
            let (inner, foot) = body.inner;
            let pressed = foot.map(|foot| place_foot(ui, foot, title));
            (Some((inner, pressed)), took)
        }
        None => {
            let body = egui::Frame::NONE.inner_margin(margin).show(ui, add_contents);
            let took = body.response.rect.size() - margin.sum();
            let (inner, foot) = body.inner;
            // (Across the window, as wide as its row or its body, or as
            // its buttons need.)
            let pressed = foot.map(|foot| place_foot(ui, foot, title));
            (Some((inner, pressed)), took)
        }
    };
    let rect = egui::Rect::from_x_y_ranges(ui.min_rect().x_range(), kept.y_range());
    let shown = mg_uikit::title_row_with(ui, rect, title, close, fold);
    (inner, shown, took)
}

/// An `egui::Modal` (what stops the work until it is answered: a job's
/// progress, a plugin's question) in the dialogs' frame, with no margin
/// of its own: [`modal_titled_foot`] draws in it.
pub(crate) fn modal_window(ctx: &egui::Context, id: egui::Id) -> egui::Modal {
    egui::Modal::new(id).frame(egui::Frame::window(&ctx.global_style()).inner_margin(0))
}

/// A modal's ([`modal_window`]) title row over `add_contents`, as a
/// dialog's ([`Dialog`]; no Close: what is in it answers it), with the
/// dialog pattern's footer, which `add_contents` says as it ends
/// ([`Foot`], as [`Dialog::show_foot`]'s). An `egui::Modal` has no name of
/// its own, so the title is named as a label: a screen reader and a test
/// find it by it. The body's value, the footer's presses, the title's
/// response (for a tip).
pub(crate) fn modal_titled_foot<R>(
    ui: &mut Ui,
    title: &str,
    add_contents: impl FnOnce(&mut Ui) -> (R, Foot),
) -> (R, Pressed, egui::Response) {
    let (inner, pressed, named) = modal_parts(ui, title, |ui| {
        let (r, foot) = add_contents(ui);
        (r, Some(foot))
    });
    (inner, pressed.unwrap_or_default(), named)
}

fn modal_parts<R>(
    ui: &mut Ui,
    title: &str,
    add_contents: impl FnOnce(&mut Ui) -> (R, Option<Foot>),
) -> (R, Option<Pressed>, egui::Response) {
    let margin = egui::Margin::same(mg_uikit::dialog::BODY_PADDING as i8);
    let row = Row {
        title,
        close: None,
        fold: None,
        folded_width: None,
        margin,
        grows: true,
        fills: false,
    };
    // Never larger than the application's window (as a `Dialog`, which
    // `show_parts` holds so): no wider than it, and the body scrolls in the
    // height its title row and footer leave.
    let screen = ui.ctx().content_rect();
    let edges = 2.0 * ui.visuals().window_stroke.width;
    let spacing = ui.spacing().item_spacing.y;
    ui.set_max_width(screen.width() - edges);
    let body_high = screen.height()
        - mg_uikit::metrics::TITLE_ROW
        - mg_uikit::dialog::FOOTER_HEIGHT
        - 2.0 * spacing
        - margin.sum().y
        - edges;
    let scrolled = |ui: &mut Ui| {
        egui::ScrollArea::vertical()
            .id_salt(("modal-body", title))
            .max_height(body_high.max(1.0))
            .auto_shrink([true, true])
            .show(ui, |ui| {
                let out = add_contents(ui);
                focus_in_sight(ui);
                out
            })
            .inner
    };
    let (inner, row, _) = titled_footed(ui, row, true, scrolled);
    let (inner, pressed) = inner.expect("a modal does not fold");
    if pressed.is_some() {
        window_seen(ui.ctx(), title, ui.min_rect(), margin.left as f32);
    }
    let named = ui.interact(row.rect, ui.id().with(("modal-title", title)), egui::Sense::hover());
    named.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Label, true, title));
    (inner, pressed, named)
}

/// A dialog that its own buttons answer (OK and Cancel, Yes and No, a
/// wizard's Finish), with no close button: [`centered`], not folded or
/// resized (a caller that wants it resized says so). Escape is its
/// footer's Cancel ([`Foot`]). (Aurora's message
/// boxes, pickers and wizards. An `egui::Modal`, the rest of the window
/// dimmed and out of reach, is kept for what stops the work until it is
/// answered: a job's progress, a plugin's question, a save a plugin
/// refuses. Opened over a Properties window, it
/// took its Escape too: Escape in Select Portrait closed the blueprint's
/// window, as its Cancel.)
pub(crate) fn modal<'a>(ctx: &egui::Context, title: impl Into<egui::WidgetText>) -> Dialog<'a> {
    centered(ctx, title).collapsible(false).resizable(false)
}

/// `window` resized by its edges and corners, opening `size` in the middle
/// of the application's window. It is held by its top left corner, so the
/// corner dragged stays under the pointer: one held by its middle, as
/// [`centered`] holds a dialog, moved half of each resize away from it
/// (Import Resources, Select Token, Conversation Test).
pub(crate) fn resizable<'a>(
    ctx: &egui::Context,
    window: Dialog<'a>,
    size: egui::Vec2,
) -> Dialog<'a> {
    window
        .resizable(true)
        .pivot(egui::Align2::LEFT_TOP)
        .default_pos(dialog_center(ctx) - size / 2.0)
        .default_size(size)
}

/// A dialog that its Close or Escape closes (`open`), as well as what it
/// has to answer it: [`centered`], with a Close at its title row's right.
pub(crate) fn dialog<'a>(ctx: &egui::Context, title: &str, open: &'a mut bool) -> Dialog<'a> {
    let was = *open;
    let open = open_unless_escape(ctx, title, open);
    if was && !*open {
        note_press(ctx, title, "Close", "Escape");
    }
    Dialog { open: Some(open), ..centered(ctx, title) }
}

/// Whether a wizard's step list stands at the left of `ui`'s room: only
/// where the page beside it keeps 320 points. In a window near its least
/// size the list would squeeze the page's controls to a letter's width;
/// the page bar still names the page there.
pub(crate) fn step_list_fits(ui: &Ui) -> bool {
    ui.available_width() >= mg_uikit::dialog::STEP_LIST_WIDTH + 320.0
}

/// Whether a dialog is open (one that Escape closes or a Cancel answers,
/// shown last frame or this one): the keys typed are its own, not those
/// of the panes under it (the module tree's).
pub(crate) fn dialog_open(ctx: &egui::Context) -> bool {
    ctx.data(|x| x.get_temp::<Dialogs>(dialogs_id())).is_some_and(|d| !d.open.is_empty())
}

/// Whether any dialog drew during the current frame, without the one-frame
/// `Dialogs.open` lag after a dialog closes.
pub(crate) fn dialog_drawn(ctx: &egui::Context) -> bool {
    ctx.data(|x| x.get_temp::<Dialogs>(dialogs_id())).is_some_and(|d| !d.seen.is_empty())
}

/// Whether the window on `layer` is a dialog (one that Escape closes or a
/// Cancel answers, shown last frame): what the Welcome page's update line
/// may stand under without the strip telling it (About over the line
/// moved the page).
pub(crate) fn is_dialog(ctx: &egui::Context, layer: egui::LayerId) -> bool {
    ctx.data(|x| x.get_temp::<Dialogs>(dialogs_id())).is_some_and(|d| d.open.contains(&layer))
}

/// Whether this frame's Escape closes a popup or a kit popover that was
/// open as the frame began (a menu, a combo box's list, a popover of the
/// area's header), and so is nothing else's: what the area view holds
/// stays in hand, though the menu has closed by the time the view reads
/// the key.
pub(crate) fn escape_closes_a_popup(ctx: &egui::Context) -> bool {
    ctx.data(|x| x.get_temp::<Dialogs>(dialogs_id())).is_some_and(|d| d.popup)
}

/// Whether a popup or a kit popover was open as this frame began (any
/// key this frame is its: Enter or Space that closed a combo box's list
/// earlier in the frame is not a list's under it too).
pub(crate) fn popup_was_open(ctx: &egui::Context) -> bool {
    escape_closes_a_popup(ctx)
}

/// Escape was pressed with no dialog open (for a window of the dock to
/// close on it).
pub(crate) fn escape_past_dialogs(ctx: &egui::Context) -> bool {
    ctx.data(|x| x.get_temp::<Dialogs>(dialogs_id())).is_some_and(|d| d.escape && d.open.is_empty())
}

/// What a "Files Already in…" window was answered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Already {
    /// Write (or add) every file, over those there.
    Replace,
    /// Write the others, and leave those there as they are.
    Skip,
    Cancel,
}

/// Asks what to do with files that are there already, before they are
/// written over: the hak editor's adding ("Files Already in the Hak") and
/// a model's Export Files ("…in the Folder"). `said` heads the list of
/// them (each with what it is on hover, where given); `replace` and
/// `skip` say what Replace and Skip Those do. `None` while unanswered.
/// Answered at once, nothing asked, where Options says what to do
/// (`there`), which "Always do this" sets.
pub(crate) fn files_already(
    ctx: &egui::Context,
    there: &mut crate::settings::FilesThere,
    place: &str,
    said: &str,
    files: &[(String, Option<String>)],
    [replace, skip]: [&str; 2],
) -> Option<Already> {
    use crate::settings::FilesThere;
    match there {
        FilesThere::Replace => return Some(Already::Replace),
        FilesThere::Skip => return Some(Already::Skip),
        FilesThere::Ask => {}
    }
    let mut answer = None;
    let always_id = egui::Id::new(("files-already-always", place));
    let (on, tip) = always_now(ctx, always_id, &format!("{QUESTIONS_AT} › Files there already"));
    let footed = modal(ctx, format!("Files Already in the {place}")).show_foot(ctx, |ui| {
        ui.label(said);
        egui::ScrollArea::vertical().max_height(180.0).show(ui, |ui| {
            for (name, about) in files {
                let r = ui.label(egui::RichText::new(name).monospace());
                if let Some(about) = about {
                    r.on_hover_text(about);
                }
            }
        });
        let foot = Foot::new()
            .check("Always do this", on, Some(tip))
            .main(format!("Replace {}", files.len()), true)
            .no_enter()
            .tip(replace)
            .button("Skip Those", true)
            .tip(skip)
            .cancel();
        ((), foot)
    });
    let mut always = false;
    if let Some(f) = footed {
        always = always_kept(ctx, always_id, f.pressed.checked);
        if f.pressed.main() {
            answer = Some(Already::Replace);
        }
        if f.pressed.clicked("Skip Those") {
            answer = Some(Already::Skip);
        }
        if f.pressed.cancelled() {
            answer = Some(Already::Cancel);
        }
    }
    // (Always so: Options' choice, from now on.)
    match answer {
        Some(Already::Replace) if always => *there = FilesThere::Replace,
        Some(Already::Skip) if always => *there = FilesThere::Skip,
        _ => {}
    }
    // (Answered: the tick is not the next asking's.)
    if answer.is_some() {
        ctx.data_mut(|d| d.remove_temp::<bool>(always_id));
    }
    answer
}

/// A question's "Always do this" box in its footer ([`Foot::check`];
/// `id` the question's, its state kept while it is asked): whether it is
/// ticked now, and its tip saying where Options changes it again
/// (`options_at`, "Tools › Options › General › Questions"). Asked anew
/// (not shown the pass before), it is not: kept, the box came ticked the
/// next time the question was asked, with Options set back to Ask, and
/// one answer became the setting again. [`always_kept`] keeps what the
/// footer left it at. Asked for by a builder (Carcerian): a question
/// always given the same answer is answered once.
pub(crate) fn always_now(ctx: &egui::Context, id: egui::Id, options_at: &str) -> (bool, String) {
    let (asked_id, now) = (id.with("asked"), ctx.cumulative_pass_nr());
    let anew = ctx.data(|d| d.get_temp::<u64>(asked_id)).is_none_or(|at| now > at + 1);
    let on = !anew && ctx.data(|d| d.get_temp::<bool>(id)).unwrap_or(false);
    (on, format!("Your answer is kept, and this isn't asked again: {options_at} changes it"))
}

/// Keeps what a footer's "Always do this" box was left at (`checked`,
/// [`Pressed::checked`]) for the question `id`:
/// whether it is ticked.
pub(crate) fn always_kept(ctx: &egui::Context, id: egui::Id, checked: Option<bool>) -> bool {
    let on = checked.unwrap_or(false);
    let now = ctx.cumulative_pass_nr();
    ctx.data_mut(|d| {
        d.insert_temp(id, on);
        d.insert_temp(id.with("asked"), now);
    });
    on
}

/// Where the questions' answers are kept, for [`always_now`]'s tip.
pub(crate) const QUESTIONS_AT: &str = "Tools › Options › General › Questions";

/// A Cancel button outside a dialog's footer (a row of a page's own):
/// clicked, or Escape pressed for the window it is in.
pub(crate) fn cancel(ui: &mut Ui) -> bool {
    let r = ui.button("Cancel");
    cancelled(ui, r.clicked())
}

/// A picker's footer (Select Portrait's, Select Texture's, M4's U26): `note`
/// at its left (a count), OK in the accent fill (enabled where `ok`, its
/// tip "Accept changes") and Cancel ("Discard changes") at its right.
pub(crate) fn picker_footer(note: Option<String>, ok: bool) -> Foot {
    let mut foot = Foot::new();
    if let Some(note) = note {
        foot = foot.note(note);
    }
    foot.main("OK", ok).tip("Accept changes").cancel_discard()
}

/// What a button of a dialog's footer is to it ([`Foot`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FootRole {
    /// The main action: first (after a wizard's Back), in the accent
    /// fill, and Enter's while it is enabled.
    Main,
    /// Another action, in its place.
    Other,
    /// What leaves the dialog without its action: Cancel (or a Done, a
    /// Later), last; Escape's, and a Close of the title row's that is its
    /// Cancel ([`Dialog::close_as_cancel`]).
    Cancel,
}

#[derive(Clone, Debug)]
struct FootButton {
    words: String,
    tip: Option<String>,
    enabled: bool,
    role: FootRole,
}

/// A dialog's footer (the dialog pattern, the Dialogs board's `.dlg-foot`,
/// M4's U27), said by its body as it ends ([`Dialog::show_foot`]): a note
/// or count at its left (or a setting's check box), its buttons at its
/// right in the order given, the code's order (the main action first in
/// the accent fill, a wizard's Back before it; Cancel, or a Done, last).
/// The one place Enter and Escape answer a dialog: Enter presses the main
/// action while it is enabled and the dialog is the one in front (with no
/// control holding the keys: a one-line text field gives them up at
/// Enter, a multi-line one and a focused button keep them), Escape its
/// Cancel. Laid out by the kit (`mg_uikit::dialog::Footer`), never cut:
/// the dialog is at least as wide as its buttons need.
#[derive(Clone, Debug, Default)]
#[must_use = "a dialog's body gives it to show_foot"]
pub(crate) struct Foot {
    note: Option<String>,
    /// A check box at the left: its words, whether it is on, its tip.
    check: Option<(String, bool, Option<String>)>,
    buttons: Vec<FootButton>,
    /// Enter withheld from the main action ([`Foot::no_enter`]).
    no_enter: bool,
    /// Escape takes the main action where there is no leave button
    /// ([`Foot::main_on_escape`]).
    main_on_escape: bool,
}

impl Foot {
    /// A footer with nothing in it yet.
    pub(crate) fn new() -> Foot {
        Foot::default()
    }

    /// With `note` at its left in `text-2`, shortened where the buttons
    /// need the room (its words its tooltip then): a count.
    pub(crate) fn note(mut self, note: impl Into<String>) -> Foot {
        self.note = Some(note.into());
        self
    }

    /// With a check box at its left (a setting: "Don't ask when nothing
    /// else names them"), `on` or not, tipped with `tip` where given;
    /// [`Pressed::checked`] says what it was left at.
    pub(crate) fn check(mut self, words: impl Into<String>, on: bool, tip: Option<String>) -> Foot {
        self.check = Some((words.into(), on, tip));
        self
    }

    fn push(mut self, words: impl Into<String>, enabled: bool, role: FootRole) -> Foot {
        self.buttons.push(FootButton { words: words.into(), tip: None, enabled, role });
        self
    }

    /// With the main action `words`, enabled where `enabled`: in the
    /// accent fill, and Enter's while enabled.
    pub(crate) fn main(self, words: impl Into<String>, enabled: bool) -> Foot {
        self.push(words, enabled, FootRole::Main)
    }

    /// With another button, `words`, enabled where `enabled`.
    pub(crate) fn button(self, words: impl Into<String>, enabled: bool) -> Foot {
        self.push(words, enabled, FootRole::Other)
    }

    /// With Cancel last: clicked, or Escape for the dialog, or its title
    /// row's Close where that is its Cancel.
    pub(crate) fn cancel(self) -> Foot {
        self.push("Cancel", true, FootRole::Cancel)
    }

    /// [`Foot::cancel`], saying on hover that changes are discarded.
    pub(crate) fn cancel_discard(self) -> Foot {
        self.cancel().tip("Discard changes")
    }

    /// With `words` last as what leaves the dialog (a Done, a Later, a
    /// Close): Escape's, as a Cancel.
    pub(crate) fn leave(self, words: impl Into<String>) -> Foot {
        self.push(words, true, FootRole::Cancel)
    }

    /// The tip of the button given last.
    pub(crate) fn tip(mut self, tip: impl Into<String>) -> Foot {
        if let Some(b) = self.buttons.last_mut() {
            b.tip = Some(tip.into());
        }
        self
    }

    /// Enter withheld from the main action (the orchestrator's ruling in
    /// M4's U27 review): a main action that grants trust, overrides a
    /// protection, overwrites or quits. It stays first and in the accent;
    /// Enter presses it only where it holds the focus, as at the tip.
    pub(crate) fn no_enter(mut self) -> Foot {
        self.no_enter = true;
        self
    }

    /// Escape takes the main action, where the footer has no leave button
    /// (a plugin's Message: its one OK).
    pub(crate) fn main_on_escape(mut self) -> Foot {
        self.main_on_escape = true;
        self
    }
}

/// How a dialog's footer was answered this frame ([`Foot`]).
#[derive(Clone, Debug, Default)]
pub(crate) struct Pressed {
    words: Vec<String>,
    clicked: Vec<bool>,
    /// The main action pressed: clicked, or Enter.
    main: bool,
    /// Left: its Cancel clicked, Escape, or the Close that is its Cancel.
    cancelled: bool,
    /// What the check box was left at, where it has one.
    pub(crate) checked: Option<bool>,
}

impl Pressed {
    /// The main action pressed (clicked, or Enter while it is enabled).
    pub(crate) fn main(&self) -> bool {
        self.main
    }

    /// Left without the action: Cancel (or the Done, Later or Close given
    /// last as leaving), Escape, or the title row's Close that is its
    /// Cancel.
    pub(crate) fn cancelled(&self) -> bool {
        self.cancelled
    }

    /// The button `words` clicked (an enabled one: a disabled button is
    /// never pressed).
    pub(crate) fn clicked(&self, words: &str) -> bool {
        self.words.iter().zip(&self.clicked).any(|(w, c)| *c && w == words)
    }

    /// The `i`th button clicked, in the order given.
    pub(crate) fn nth(&self, i: usize) -> bool {
        self.clicked.get(i).copied().unwrap_or(false)
    }
}

/// What a dialog showed of its footer in the frame last drawn (M4's U27):
/// for tests, which hold every dialog to the pattern (the main action
/// first in the accent, Cancel last, the footer whole).
#[doc(hidden)]
#[derive(Clone, Debug)]
pub struct FooterSeen {
    /// The dialog's title.
    pub title: String,
    /// The footer's rect (`None`: a dialog shown with no footer).
    pub rect: Option<egui::Rect>,
    /// The dialog's whole window (its frame), once drawn.
    pub window: egui::Rect,
    pub note: Option<String>,
    pub check: Option<String>,
    pub buttons: Vec<ButtonSeen>,
    /// Enter withheld from its main action ([`Foot::no_enter`]).
    pub enter_withheld: bool,
    /// The layer it is drawn on (its window's, or its modal's).
    pub layer: egui::LayerId,
    /// Its body's margin, inside its window's edges.
    pub margin: f32,
}

/// A footer's button as shown ([`FooterSeen`]).
#[doc(hidden)]
#[derive(Clone, Debug)]
pub struct ButtonSeen {
    pub words: String,
    /// The main action, drawn in the accent fill.
    pub main: bool,
    /// What leaves the dialog (Cancel, Done, Later): Escape's.
    pub leaves: bool,
    pub enabled: bool,
    pub rect: egui::Rect,
}

fn footers_id() -> egui::Id {
    egui::Id::new("mg-dialog-footers")
}

fn footers_record_id() -> egui::Id {
    egui::Id::new("mg-dialog-footers-record")
}

/// Starts recording [`FooterSeen`] snapshots on `ctx` (enabled by test
/// harnesses so the app does not build footer snapshots in normal runs).
#[doc(hidden)]
pub fn record_footers(ctx: &egui::Context) {
    ctx.data_mut(|d| d.insert_temp(footers_record_id(), true));
}

fn is_recording_footers(ctx: &egui::Context) -> bool {
    ctx.data(|d| d.get_temp::<bool>(footers_record_id()).unwrap_or(false))
}

fn record_footer(ctx: &egui::Context, seen: FooterSeen) {
    if !is_recording_footers(ctx) {
        return;
    }
    let pass = ctx.cumulative_pass_nr();
    ctx.data_mut(|d| {
        let kept = d.get_temp_mut_or_default::<(u64, Vec<FooterSeen>)>(footers_id());
        if kept.0 != pass {
            *kept = (pass, Vec::new());
        }
        kept.1.push(seen);
    });
}

/// Notes the window of the dialog `title` drawn this pass in its record.
fn window_seen(ctx: &egui::Context, title: &str, window: egui::Rect, margin: f32) {
    if !is_recording_footers(ctx) {
        return;
    }
    let pass = ctx.cumulative_pass_nr();
    ctx.data_mut(|d| {
        let kept = d.get_temp_mut_or_default::<(u64, Vec<FooterSeen>)>(footers_id());
        if kept.0 == pass
            && let Some(seen) = kept.1.iter_mut().rev().find(|f| f.title == title)
        {
            seen.window = window;
            seen.margin = margin;
        }
    });
}

/// The dialogs drawn in the frame last run, each with what its footer
/// showed (for tests: [`FooterSeen`]).
#[doc(hidden)]
pub fn footers_shown(ctx: &egui::Context) -> Vec<FooterSeen> {
    // (Those of the pass last run alone: a pass that drew none left the
    // record of the one before.)
    let last = ctx.cumulative_pass_nr().saturating_sub(1);
    let kept = ctx.data(|d| d.get_temp::<(u64, Vec<FooterSeen>)>(footers_id()));
    kept.filter(|k| k.0 == last).map(|k| k.1).unwrap_or_default()
}

fn presses_id() -> egui::Id {
    egui::Id::new("mg-dialog-presses")
}

/// The footer buttons pressed since this was last asked, each as its
/// dialog's title and its words, and how (`"click"`, `"Enter"`,
/// `"Escape"`; a dialog with a Close that Escape closed: `"Close"` by
/// `"Escape"`): for tests, which hold Enter and Escape to what a dialog's
/// buttons say. Kept only once asked (the first call starts the record),
/// so the application keeps none.
#[doc(hidden)]
pub fn dialog_presses(ctx: &egui::Context) -> Vec<(String, String, &'static str)> {
    ctx.data_mut(|d| {
        let kept = d.get_temp_mut_or_default::<Vec<(String, String, &'static str)>>(presses_id());
        std::mem::take(kept)
    })
}

fn note_press(ctx: &egui::Context, title: &str, words: &str, how: &'static str) {
    ctx.data_mut(|d| {
        type Kept = Vec<(String, String, &'static str)>;
        if let Some(mut kept) = d.get_temp::<Kept>(presses_id()) {
            kept.push((title.to_string(), words.to_string(), how));
            d.insert_temp(presses_id(), kept);
        }
    });
}

/// Whether the dialog on `layer` is the one in front: the `egui::Modal`
/// on top, or, with no modal up, the dialog opened last. (A dialog under a
/// job's window took its Enter and Escape too.)
fn in_front(ctx: &egui::Context, layer: egui::LayerId) -> bool {
    match layer.order {
        egui::Order::Middle => {
            !modal_over(ctx, layer)
                && ctx
                    .data(|x| x.get_temp::<Dialogs>(dialogs_id()))
                    .is_some_and(|d| d.open.last() == Some(&layer))
        }
        _ => ctx.memory(|m| m.top_modal_layer()) == Some(layer),
    }
}

/// The width a footer check box's words and box take.
fn check_width(ui: &Ui, words: &str) -> f32 {
    let font = egui::TextStyle::Body.resolve(ui.style());
    let galley = ui.painter().layout_no_wrap(words.to_string(), font, egui::Color32::WHITE);
    mg_uikit::controls::BOX + ui.spacing().icon_spacing + galley.size().x + 2.0
}

/// The least width `foot` needs (its ends' room, its buttons and check
/// box; the note is shortened), and the width its check box takes: its
/// words whole, or, where they would make the footer wider than the
/// application's window (the least window), cut to what is left, with
/// their whole words on hover.
fn foot_least(ui: &Ui, foot: &Foot) -> (f32, f32) {
    let least = |check: Option<f32>| {
        let mut kit = mg_uikit::dialog::Footer::new();
        for b in &foot.buttons {
            kit = kit.button(mg_uikit::KitButton::new(None, b.words.clone()));
        }
        if let Some(width) = check {
            kit = kit.left(width, |_| {});
        }
        kit.least_width(ui)
    };
    let Some((words, _, _)) = &foot.check else { return (least(None), 0.0) };
    let whole = check_width(ui, words);
    let full = least(Some(whole));
    let room = ui.ctx().content_rect().width() - 2.0 * ui.visuals().window_stroke.width - 2.0;
    let shortest = mg_uikit::controls::BOX + ui.spacing().icon_spacing + 24.0;
    let width = match full > room {
        true => (whole - (full - room)).max(shortest).min(whole),
        false => whole,
    };
    (least(Some(width)), width)
}

/// Shows `foot` (a dialog's, titled `title`) across `ui`'s width so far
/// (at least as wide as it needs), under what `ui` holds: the strip, its
/// presses (Enter and Escape among them), recorded for tests.
fn place_foot(ui: &mut Ui, foot: Foot, title: &str) -> Pressed {
    use mg_uikit::{ButtonLook, KitButton};
    let (least, check_room) = foot_least(ui, &foot);
    let wide = ui.min_rect().width().max(least);
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(wide, mg_uikit::dialog::FOOTER_HEIGHT),
        egui::Sense::hover(),
    );
    let left = ui.min_rect().left();
    let rect =
        egui::Rect::from_x_y_ranges(left..=left + wide.max(ui.min_rect().width()), rect.y_range());
    let mut slot = ui.new_child(egui::UiBuilder::new().max_rect(rect));
    let Foot { note, check, buttons, no_enter, main_on_escape } = foot;
    let mut kit = mg_uikit::dialog::Footer::new();
    if let Some(note) = &note {
        kit = kit.note(note.clone());
    }
    let checked = std::cell::Cell::new(None);
    if let Some((words, on, tip)) = &check {
        let width = check_room;
        let cut = width < check_width(&slot, words) - 0.5;
        let (words, tip, checked) = (words.clone(), tip.clone(), &checked);
        let mut on = *on;
        kit = kit.left(width, move |ui| {
            use mg_uikit::Controls as _;
            // (Cut to its room at the least window: its words on hover.)
            ui.set_max_width(width);
            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Truncate);
            let mut r = ui.check_box(&mut on, words.clone());
            let tip = match (cut, tip) {
                (true, Some(t)) => Some(format!("{words}. {t}")),
                (true, None) => Some(words),
                (false, t) => t,
            };
            if let Some(tip) = tip {
                r = r.on_hover_text(tip);
            }
            let _ = r;
            checked.set(Some(on));
        });
    }
    for b in &buttons {
        let look = if b.role == FootRole::Main { ButtonLook::Primary } else { ButtonLook::Raised };
        let mut k = KitButton::new(None, b.words.clone()).look(look).enabled(b.enabled);
        if let Some(tip) = &b.tip {
            k = k.tip(tip.clone());
        }
        kit = kit.button(k);
    }
    let shown = kit.show(&mut slot);
    let clicked: Vec<bool> =
        (shown.buttons.iter().zip(&buttons)).map(|(r, b)| b.enabled && r.clicked()).collect();
    let layer = ui.layer_id();
    let main_at = buttons.iter().position(|b| b.role == FootRole::Main);
    let main_enabled = main_at.is_some_and(|i| buttons[i].enabled);
    // (Enter: the main action's, enabled, in the dialog in front, with no
    // control holding the keys.)
    let enter = main_enabled && !no_enter && in_front(ui.ctx(), layer) && enter(ui);
    let leave_at = buttons.iter().rposition(|b| b.role == FootRole::Cancel);
    // (Escape: the main action's, where it is told so and nothing else
    // leaves.)
    let escaped = main_on_escape && leave_at.is_none() && main_enabled && cancelled(ui, false);
    let main = main_at.is_some_and(|i| clicked[i]) || enter || escaped;
    let cancelled = match leave_at {
        Some(i) => cancelled(ui, clicked[i]),
        None => false,
    };
    for (i, b) in buttons.iter().enumerate() {
        let how = match (clicked[i], b.role) {
            (true, _) => Some("click"),
            (false, FootRole::Main) if enter => Some("Enter"),
            (false, FootRole::Main) if escaped => Some("Escape"),
            (false, FootRole::Cancel) if cancelled && Some(i) == leave_at => Some("Escape"),
            _ => None,
        };
        if let Some(how) = how {
            note_press(ui.ctx(), title, &b.words, how);
        }
    }
    if is_recording_footers(ui.ctx()) {
        let seen = FooterSeen {
            title: title.to_string(),
            rect: Some(shown.rect),
            window: egui::Rect::NOTHING,
            note,
            check: check.map(|(w, _, _)| w),
            buttons: (buttons.iter().zip(&shown.buttons))
                .map(|(b, r)| ButtonSeen {
                    words: b.words.clone(),
                    main: b.role == FootRole::Main,
                    leaves: b.role == FootRole::Cancel,
                    enabled: b.enabled,
                    rect: r.rect,
                })
                .collect(),
            enter_withheld: no_enter,
            layer: ui.layer_id(),
            margin: 0.0,
        };
        record_footer(ui.ctx(), seen);
    }
    Pressed {
        words: buttons.into_iter().map(|b| b.words).collect(),
        clicked,
        main,
        cancelled,
        checked: checked.get(),
    }
}

/// What [`Dialog::show_foot`] gave: the body's value (`None` where the
/// dialog is folded to its title row), the footer's presses, the window.
pub(crate) struct Footed<R> {
    pub(crate) inner: Option<R>,
    pub(crate) pressed: Pressed,
    pub(crate) response: egui::Response,
}

/// A Cancel's answer: `clicked`, the title row's Close the frame before
/// ([`Dialog::close_as_cancel`]), or Escape on the window of `ui`.
fn cancelled(ui: &Ui, clicked: bool) -> bool {
    // (In a window of its own: not a Cancel within a docked tab. And the
    // Close of a picker's title row, the frame before:
    // `Dialog::close_as_cancel`.)
    let layer = ui.layer_id();
    let closed = ui.ctx().data_mut(|d| d.remove_temp::<bool>(closed_as_cancel_id(layer)));
    // (An `egui::Modal`'s Escape: the modal on top, as egui's own
    // `should_close` reads it.)
    let modal_escape = || {
        ui.ctx().memory(|m| m.top_modal_layer()) == Some(layer)
            && !egui::Popup::is_any_open(ui.ctx())
            && ui.ctx().input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape))
    };
    clicked
        || closed == Some(true)
        || (layer.order == egui::Order::Middle && escape_on(ui.ctx(), layer))
        || (layer.order != egui::Order::Middle && modal_escape())
}

/// Where a dialog's Close that is its Cancel ([`Dialog::close_as_cancel`])
/// is noted for the window on `layer`.
fn closed_as_cancel_id(layer: egui::LayerId) -> egui::Id {
    egui::Id::new(("mg-dialog-closed-as-cancel", layer))
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
    shown: Option<egui::WidgetText>,
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

/// The greatest height of a filtered list's rows, under its Filter field.
const ROWS_HEIGHT: f32 = 170.0;

/// How many of a long list's rows (the longest) its width is measured by.
const MEASURED_ROWS: usize = 64;

/// The rows of a 2DA as a list's choices, by its `name` column (StrRefs
/// read in the talk tables) else its `label` (`GameData::choice_list`:
/// the list made once and kept, not copied); none without a game.
pub(crate) fn table_choices(
    game: Option<&mg_rules::GameData>,
    table: &str,
    name: Option<&str>,
    label: Option<&str>,
) -> std::sync::Arc<Vec<mg_rules::Choice>> {
    let columns = mg_rules::ChoiceColumns { name, label };
    game.and_then(|g| g.choice_list(table, columns).ok()).unwrap_or_default()
}

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
    pub(crate) fn shown(mut self, text: impl Into<egui::WidgetText>) -> Self {
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
            Some(row) => mg_rules::choice_text(choices, row).into(),
            None => unset.into(),
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
            .icon(mg_uikit::combo_icon)
            .selected_text(shown)
            .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside);
        // (Never wider than its room: a narrow form's, the Properties panel.)
        if let Some(width) = width {
            let width = held_width(ui, width);
            wrap_for(ui, width);
            combo = combo.width(width);
        }
        if long {
            // (Room for the Filter field over the rows' own scroll area,
            // with no scroll bar of egui's around both.)
            let line = ui.text_style_height(&egui::TextStyle::Body) + ui.spacing().interact_size.y;
            combo = combo.height(ROWS_HEIGHT + 2.0 * line);
        }
        // Just opened: the list is whole and begins at the one chosen, not
        // at its top (a long list was searched for it every time).
        let opened = !ui.data(|d| d.get_temp::<bool>(open_id)).unwrap_or(false);
        let list = combo_looks(ui, |ui| {
            combo.show_ui(ui, |ui| {
                mg_uikit::list_rows(ui);
                let width_id = open_id.with("width");
                if opened {
                    ui.data_mut(|d| {
                        d.remove_temp::<String>(filter_id);
                        d.insert_temp(open_id.with("go"), true);
                    });
                }
                if opened && long {
                    // A long list's width, set once while open: the box's, or
                    // the widest row's and its scroll bar's if wider. egui's
                    // popup takes the width its contents had the frame before;
                    // with the rows filling it, and a scroll area that follows
                    // its rows, any part of a point more there grew the list
                    // every frame, until it reached the window's edge (a
                    // builder's report). (A short list is egui's own.)
                    let pad = 2.0 * ui.spacing().button_padding.x;
                    let bar = ui.spacing().scroll.allocated_width();
                    // (Measured: the longest rows by their characters, not
                    // every row of a table of thousands; order-independent.)
                    let mut texts: Vec<std::borrow::Cow<'_, str>> =
                        (none.map(std::borrow::Cow::from).into_iter())
                            .chain(choices.iter().map(|c| text(c, numbered)))
                            .collect();
                    let measured = texts.len().min(MEASURED_ROWS);
                    if measured < texts.len() {
                        texts.select_nth_unstable_by_key(measured, |t| {
                            std::cmp::Reverse(t.chars().count())
                        });
                    }
                    let widest = (texts.into_iter().take(measured))
                        .map(|t| {
                            egui::WidgetText::from(t.as_ref())
                                .into_galley(
                                    ui,
                                    Some(egui::TextWrapMode::Extend),
                                    f32::INFINITY,
                                    egui::TextStyle::Button,
                                )
                                .size()
                                .x
                        })
                        .fold(0.0, f32::max);
                    let most = ui.ctx().content_rect().width() - 40.0;
                    let width = ui.available_width().max((widest + pad + bar).min(most));
                    ui.data_mut(|d| d.insert_temp(width_id, width));
                }
                if let Some(width) = ui.data(|d| d.get_temp::<f32>(width_id)).filter(|_| long) {
                    ui.set_width(width);
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
                    // (Escape clears what is typed, then closes the list.)
                    let before = filter.clone();
                    let searched = SearchField::new(filter_id.with("field"), &mut filter)
                        .hint("Filter")
                        .show(ui);
                    narrowed = filter != before;
                    if searched.left == Some(FilterLeft::Escape) {
                        ui.close();
                    }
                    ui.data_mut(|d| d.insert_temp(filter_id, filter.clone()));
                }
                // (Case and accents aside: `arbol` finds `Árbol`.)
                let filter = mg_core::text::Needle::new(&filter);
                let matches = |c: &mg_rules::Choice| {
                    filter.finds(&text(c, numbered)) || c.row.to_string() == filter.as_str()
                };
                let chosen = |c: Option<&mg_rules::Choice>| match c {
                    Some(c) => current == Some(c.row as i64),
                    None => current.is_none(),
                };
                let mut row = |ui: &mut Ui, c: Option<&mg_rules::Choice>, scroll: bool| {
                    let label = match c {
                        Some(c) => text(c, numbered),
                        None => none.unwrap_or_default().into(),
                    };
                    let mut r = ui.selectable_label(chosen(c), label);
                    if scroll && go && chosen(c) {
                        r.scroll_to_me(Some(egui::Align::Center));
                    }
                    if let (Some(about), Some(c)) = (about.as_mut(), c) {
                        r = r.on_hover_ui_at_pointer(|ui| about(ui, c.row));
                    }
                    if r.clicked() {
                        pick = Some(c.map(|c| c.row as i64));
                        ui.close();
                    }
                };
                if long {
                    // (Of a long list, found once while it is open and kept until
                    // the filter changes: not sorted and searched every frame.)
                    type Kept = (ChoicesSig, bool, String, std::sync::Arc<Vec<usize>>);
                    let kept_id = open_id.with("found");
                    let sig = choices_sig(choices);
                    let kept = ui.data(|d| d.get_temp::<Kept>(kept_id));
                    let at = match kept {
                        Some((s, ordered_name, f, at))
                            if !opened
                                && s == sig
                                && ordered_name == by_name
                                && f.as_str() == filter.as_str() =>
                        {
                            at
                        }
                        _ => {
                            let at: Vec<usize> = if by_name {
                                let order = cached_name_order(ui, choices);
                                order.iter().copied().filter(|&i| matches(&choices[i])).collect()
                            } else {
                                (0..choices.len()).filter(|&i| matches(&choices[i])).collect()
                            };
                            let at = std::sync::Arc::new(at);
                            let kept: Kept = (sig, by_name, filter.as_str().to_owned(), at.clone());
                            ui.data_mut(|d| d.insert_temp(kept_id, kept));
                            at
                        }
                    };
                    let none_off = usize::from(none.is_some());
                    let shown_len = none_off + at.len();
                    let shown_at = |idx: usize| -> Option<&mg_rules::Choice> {
                        if none.is_some() {
                            (idx > 0).then(|| &choices[at[idx - 1]])
                        } else {
                            Some(&choices[at[idx]])
                        }
                    };
                    // (The rows scroll under the Filter field, which stays;
                    // only those in sight are laid out, of a table of tens of
                    // thousands too. Opened, the chosen one is in the middle.)
                    let height = ui.spacing().interact_size.y.max(
                        ui.text_style_height(&egui::TextStyle::Button)
                            + 2.0 * ui.spacing().button_padding.y,
                    );
                    let step = height + ui.spacing().item_spacing.y;
                    let mut area = egui::ScrollArea::vertical().max_height(ROWS_HEIGHT);
                    if narrowed {
                        area = area.vertical_scroll_offset(0.0);
                    } else if go {
                        let target = if let Some(cur) = current {
                            at.iter()
                                .position(|&i| choices[i].row as i64 == cur)
                                .map(|p| p + none_off)
                        } else if none.is_some() {
                            Some(0)
                        } else {
                            None
                        };
                        if let Some(pos) = target {
                            let offset = pos as f32 * step + height / 2.0 - ROWS_HEIGHT / 2.0;
                            area = area.vertical_scroll_offset(offset.max(0.0));
                        }
                    }
                    area.show_rows(ui, height, shown_len, |ui, range| {
                        for idx in range {
                            row(ui, shown_at(idx), false);
                        }
                    });
                } else {
                    if none.is_some() {
                        row(ui, None, true);
                    }
                    for c in in_order().iter().filter(|c| matches(c)) {
                        row(ui, Some(c), true);
                    }
                }
            })
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
            let nav_key = (r.clicked() || r.has_focus())
                && ui.input(|i| {
                    i.modifiers.is_none()
                        && [
                            egui::Key::ArrowDown,
                            egui::Key::ArrowUp,
                            egui::Key::PageDown,
                            egui::Key::PageUp,
                            egui::Key::Home,
                            egui::Key::End,
                        ]
                        .iter()
                        .any(|k| i.key_pressed(*k))
                });
            let wheel_moved = wheel
                && r.hovered()
                && ui.input(|i| {
                    i.events.iter().any(|e| matches!(e, egui::Event::MouseWheel { .. }))
                });
            if nav_key || wheel_moved {
                let rows: Vec<i64> = if by_name {
                    let order = cached_name_order(ui, choices);
                    (none.map(|_| NO_ROW).into_iter())
                        .chain(order.iter().map(|&i| choices[i].row as i64))
                        .collect()
                } else {
                    (none.map(|_| NO_ROW).into_iter())
                        .chain(choices.iter().map(|c| c.row as i64))
                        .collect()
                };
                let at = current.unwrap_or(NO_ROW);
                let stepped = arrow_pick(ui, r, &rows, at)
                    .or_else(|| wheel.then(|| wheel_pick(ui, r, &rows, at)).flatten());
                pick = stepped.map(|row| (row != NO_ROW).then_some(row));
            } else if r.clicked() || r.has_focus() {
                let _ = hold_arrows(ui, r);
            }
        }
        // Letters typed to a short list (a long one types into its Filter):
        // the first row whose name begins with them.
        if pick.is_none()
            && !long
            && (open || r.has_focus())
            && ui.input(|i| {
                i.events
                    .iter()
                    .any(|e| matches!(e, egui::Event::Text(t) if !t.chars().any(char::is_control)))
            })
        {
            let rows: Vec<Option<&mg_rules::Choice>> =
                none.map(|_| None).into_iter().chain(in_order().iter().map(Some)).collect();
            let names = |i: usize| match rows[i] {
                Some(c) => text(c, numbered).into_owned(),
                None => none.unwrap_or_default().to_string(),
            };
            let at = rows.iter().position(|c| c.map(|c| c.row as i64) == current);
            if let Some(i) = typed_to(ui, r, names, rows.len(), at) {
                pick = Some(rows[i].map(|c| c.row as i64));
            }
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

type ChoicesSig = (usize, u64);

fn choices_sig(choices: &[mg_rules::Choice]) -> ChoicesSig {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    let len = choices.len();
    len.hash(&mut h);
    // (Every one: a sample of them kept the order of names a renamed row
    // had before, and took one table's for another's of its length.
    // Hashing is still far less than sorting them by name.)
    for c in choices {
        c.row.hash(&mut h);
        c.text.hash(&mut h);
    }
    (len, h.finish())
}

fn cached_name_order(ui: &Ui, choices: &[mg_rules::Choice]) -> std::sync::Arc<Vec<usize>> {
    type Entry = (ChoicesSig, std::sync::Arc<Vec<usize>>);
    let sig = choices_sig(choices);
    let cache_id = egui::Id::new("mg-row-choice-name-order");
    if let Some(hit) = ui.data_mut(|d| {
        let lru = d.get_temp_mut_or_default::<Vec<Entry>>(cache_id);
        if let Some(pos) = lru.iter().position(|(s, _)| *s == sig) {
            let entry = lru.remove(pos);
            let order = entry.1.clone();
            lru.push(entry);
            Some(order)
        } else {
            None
        }
    }) {
        return hit;
    }
    let order = std::sync::Arc::new(mg_rules::name_order(choices));
    ui.data_mut(|d| {
        let lru = d.get_temp_mut_or_default::<Vec<Entry>>(cache_id);
        if lru.len() >= 8 {
            lru.remove(0);
        }
        lru.push((sig, order.clone()));
    });
    order
}

/// The arrow keys on a list of choices, as in Aurora: with the list's
/// button `r` in focus (clicked, or reached with Tab), Up and Down choose
/// the row before and after `current` among `rows` (in the order the list
/// shows them), Page Up and Page Down ten away, Home and End the first
/// and last. The row chosen so.
pub(crate) fn arrow_pick(ui: &Ui, r: &egui::Response, rows: &[i64], current: i64) -> Option<i64> {
    if !hold_arrows(ui, r) || rows.is_empty() {
        return None;
    }
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

/// A list of choices' box `r` takes the keyboard when clicked, and the
/// arrow keys while it has it (not egui's way from one widget to the
/// next): whether it has it.
fn hold_arrows(ui: &Ui, r: &egui::Response) -> bool {
    if r.clicked() {
        r.request_focus();
    }
    if !r.has_focus() {
        return false;
    }
    claim_arrows(ui, r);
    let arrows = egui::EventFilter { vertical_arrows: true, ..Default::default() };
    ui.memory_mut(|m| m.set_focus_lock_filter(r.id, arrows));
    true
}

/// (Where the list of choices with the keyboard is kept: [`choice_has_keys`].)
const CHOICE_WITH_KEYS: &str = "choice-with-keys";

/// Says that the list of choices `r` has the keyboard (and so the arrow
/// keys): a list beside it that takes them while it has no focus of its
/// own (the conversation tree) leaves them to it.
fn claim_arrows(ui: &Ui, r: &egui::Response) {
    ui.data_mut(|d| d.insert_temp(egui::Id::new(CHOICE_WITH_KEYS), r.id));
}

/// Whether a list of choices (a dropdown, a list of a table's rows, the
/// Lighting Scheme grid) has the keyboard now: the arrow keys are its, not
/// those of a list that has the keys otherwise (the conversation tree).
pub(crate) fn choice_has_keys(ctx: &egui::Context) -> bool {
    let id = ctx.data(|d| d.get_temp::<egui::Id>(egui::Id::new(CHOICE_WITH_KEYS)));
    id.is_some() && ctx.memory(|m| m.focused()) == id
}

/// The arrow keys in a grid of choices filled a column at a time, `down`
/// to a column, `count` in all: with the choice `r` (the one at `at`) in
/// focus (clicked, or reached with Tab), Up and Down choose the one
/// before and after it, Left and Right the one beside it in the next
/// column, Home and End the first and last. The index chosen so.
pub(crate) fn grid_arrow_pick(
    ui: &Ui,
    r: &egui::Response,
    at: usize,
    count: usize,
    down: usize,
) -> Option<usize> {
    if r.clicked() {
        r.request_focus();
    }
    if !r.has_focus() || count == 0 {
        return None;
    }
    claim_arrows(ui, r);
    // (Every arrow is the grid's while it has the focus.)
    let arrows =
        egui::EventFilter { vertical_arrows: true, horizontal_arrows: true, ..Default::default() };
    ui.memory_mut(|m| m.set_focus_lock_filter(r.id, arrows));
    let to = ui.input(|i| {
        let key = |k| i.key_pressed(k) && i.modifiers.is_none();
        if key(egui::Key::ArrowDown) {
            at.checked_add(1)
        } else if key(egui::Key::ArrowUp) {
            at.checked_sub(1)
        } else if key(egui::Key::ArrowRight) {
            at.checked_add(down.max(1))
        } else if key(egui::Key::ArrowLeft) {
            at.checked_sub(down.max(1))
        } else if key(egui::Key::Home) {
            Some(0)
        } else if key(egui::Key::End) {
            Some(count - 1)
        } else {
            None
        }
    })?;
    (to < count && to != at).then_some(to)
}

/// [`arrow_pick`] for an egui combo box of a few fixed `options` (not a
/// table's rows, which [`row_choice`] lists: the area's header, a
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

/// A dropdown: every one in the program is drawn by this (or, for a
/// table's rows, by [`row_choice`]), so that every list of choices takes
/// the arrow keys ([`arrow_step`]) once clicked or reached with Tab. A
/// builder over egui's combo box, `id_salt` its id.
pub(crate) struct Dropdown {
    combo: egui::ComboBox,
    /// Its id salt as an id: where [`Dropdown::show_menu`] keeps its keys.
    salt: egui::Id,
    /// [`Dropdown::show_menu`]'s list has a Filter field above its rows.
    filtered: bool,
    /// Keep the popup open after picking/toggling a row (for multi-select).
    stays_open: bool,
    /// The box's width as asked (held to its room as it is shown).
    width: Option<f32>,
}

/// A box's width as asked, `want`, held to `ui`'s room where it is wider
/// in a narrow form (the Properties panel's), never under 40 there; as
/// asked elsewhere (a grid sizes its columns by the frame before: held to
/// a column's first width, a box would keep it).
pub(crate) fn held_width(ui: &Ui, want: f32) -> f32 {
    let room = ui.available_width();
    if want > room && crate::blueprint::narrow(ui) { room.max(40.0) } else { want }
}

/// In a row that wraps in a narrow form (the Properties panel's), onto
/// the next line where what is left of this one is narrower than `width`:
/// a combo box is laid out whole where its row puts it, never wrapped by
/// the row itself.
pub(crate) fn wrap_for(ui: &mut Ui, width: f32) {
    let left = ui.available_size_before_wrap().x;
    let wraps = ui.layout().main_wrap() && ui.layout().is_horizontal();
    if wraps && crate::blueprint::narrow(ui) && left < width {
        ui.end_row();
    }
}

/// A [`Dropdown`] with the id `id_salt`.
pub(crate) fn dropdown(id_salt: impl egui::AsIdSalt) -> Dropdown {
    let salt = egui::Id::new(&id_salt);
    Dropdown {
        combo: egui::ComboBox::from_id_salt(id_salt).icon(mg_uikit::combo_icon),
        salt,
        filtered: false,
        stays_open: false,
        width: None,
    }
}

impl Dropdown {
    /// The box, its width as asked held to `ui`'s room (a narrow form's,
    /// the Properties panel: never past its edge).
    fn held(combo: egui::ComboBox, width: Option<f32>, ui: &mut Ui) -> egui::ComboBox {
        match width {
            Some(w) => {
                let w = held_width(ui, w);
                wrap_for(ui, w);
                combo.width(w)
            }
            None => combo,
        }
    }

    /// What the box shows.
    pub(crate) fn selected_text(mut self, text: impl Into<egui::WidgetText>) -> Self {
        self.combo = self.combo.selected_text(text);
        self
    }

    /// The box's width.
    pub(crate) fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    /// The open list's greatest height.
    pub(crate) fn height(mut self, height: f32) -> Self {
        self.combo = self.combo.height(height);
        self
    }

    /// A Filter field above [`show_menu`](Self::show_menu)'s rows when
    /// `long` (a builder's ask: Add hak… over a hak folder of hundreds),
    /// the rows scrolling under it. Opened by a click, the field has the
    /// keys: letters typed narrow the list ([`MenuKeys::shows`]), Down
    /// goes to its first row, Enter does what the row there does, Escape
    /// clears the field, then closes the list. Letters typed while the
    /// arrow keys are on the rows go to the field.
    pub(crate) fn filtered(mut self, long: bool) -> Self {
        if long {
            self.filtered = true;
            self.combo = self.combo.close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside);
        }
        self
    }

    /// Keeps [`Dropdown::show_menu`] open when rows inside the popup are
    /// clicked or toggled with the keyboard (only outside clicks or Escape
    /// close it).
    pub(crate) fn stays_open(mut self, keep: bool) -> Self {
        if keep {
            self.stays_open = true;
            self.combo = self.combo.close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside);
        }
        self
    }

    /// A choice of `options` (in the order shown), each named by `text`,
    /// `current` the one chosen: a click or the arrow keys change it. The
    /// box's response.
    pub(crate) fn show<T: PartialEq + Clone, S: Into<egui::WidgetText>>(
        self,
        ui: &mut Ui,
        current: &mut T,
        options: &[T],
        text: impl Fn(&T) -> S,
    ) -> egui::Response {
        let reveal_id = self.salt.with("reveal typed");
        let reveal = ui.data_mut(|d| d.remove_temp::<bool>(reveal_id)).unwrap_or(false);
        let r = self.show_with(
            ui,
            current,
            || options.to_vec(),
            |ui, current| {
                for o in options {
                    let row = ui.selectable_value(current, o.clone(), text(o));
                    if reveal && o == current {
                        row.scroll_to_me(Some(egui::Align::Center));
                    }
                }
            },
        );
        // Letters typed to it: the first whose name begins with them.
        let names = |i: usize| text(&options[i]).into().text().to_string();
        let at = options.iter().position(|o| o == current);
        if let Some(i) = typed_to(ui, &r, names, options.len(), at) {
            *current = options[i].clone();
            ui.data_mut(|d| d.insert_temp(reveal_id, true));
            ui.ctx().request_repaint();
        }
        r
    }

    /// [`show`](Self::show) with the list's own rows (`rows`: drawn while
    /// it is open, choosing into `current`), the arrow keys stepping
    /// `current` through `options` (made only while the box has the keys:
    /// a list read only while it is used).
    pub(crate) fn show_with<T: PartialEq + Clone>(
        self,
        ui: &mut Ui,
        current: &mut T,
        options: impl FnOnce() -> Vec<T>,
        rows: impl FnOnce(&mut Ui, &mut T),
    ) -> egui::Response {
        let r = combo_looks(ui, |ui| {
            (Self::held(self.combo, self.width, ui)).show_ui(ui, |ui| {
                mg_uikit::list_rows(ui);
                rows(ui, current)
            })
        })
        .response;
        // (The options read only when an arrow key is pressed: not each
        // frame the box has the keys.)
        let keyed = ui.input(|i| {
            use egui::Key;
            [Key::ArrowDown, Key::ArrowUp, Key::PageDown, Key::PageUp, Key::Home, Key::End]
                .iter()
                .any(|k| i.key_pressed(*k) && i.modifiers.is_none())
        });
        if keyed && (r.clicked() || r.has_focus()) {
            arrow_step(ui, &r, &options(), current);
        } else {
            hold_arrows(ui, &r);
        }
        r
    }

    /// A dropdown of commands (Add hak…) or of
    /// ticks (the 2DA editor's flags), not one choice: once its box is
    /// clicked or reached with Tab, Up and Down open it and move a cursor
    /// along its rows (Home and End: the first and last), and Enter or
    /// Space does what the row's click does ([`MenuKeys::row`]): a
    /// command closes the list, a tick leaves it open. The box's response.
    pub(crate) fn show_menu(
        self,
        ui: &mut Ui,
        contents: impl FnOnce(&mut Ui, &mut MenuKeys),
    ) -> egui::Response {
        let state_id = ui.make_persistent_id(("menu-keys", self.salt));
        let mut state: MenuState = ui.data(|d| d.get_temp(state_id)).unwrap_or_default();
        let focused = state.button.is_some_and(|b| ui.memory(|m| m.has_focus(b)));
        let popup = state.button.map(|b| b.with("popup"));
        let open = popup.is_some_and(|p| egui::Popup::is_id_open(ui.ctx(), p));
        let mut pressed = None;
        if focused && let Some(popup) = popup {
            let none = egui::Modifiers::NONE;
            let key = |k| ui.input_mut(|i| i.consume_key(none, k));
            let last = state.rows.saturating_sub(1);
            let at = state.cursor;
            let to = if key(egui::Key::ArrowDown) {
                Some(at.map_or(0, |a| (a + 1).min(last)))
            } else if key(egui::Key::ArrowUp) {
                Some(at.map_or(last, |a| a.saturating_sub(1)))
            } else if open && key(egui::Key::Home) {
                Some(0)
            } else if open && key(egui::Key::End) {
                Some(last)
            } else {
                None
            };
            if let Some(to) = to {
                if !open {
                    // (Opened at its first row: its rows are not known yet.)
                    egui::Popup::open_id(ui.ctx(), popup);
                    state.cursor = Some(0);
                } else {
                    state.cursor = Some(to);
                }
                state.reveal = true;
            }
            if open && state.cursor.is_some() && (key(egui::Key::Enter) || key(egui::Key::Space)) {
                pressed = state.cursor;
            }
        }
        let filter_id = state_id.with("filter");
        let field_id = filter_id.with("field");
        let mut filter: String = match open {
            true => ui.data(|d| d.get_temp(filter_id)).unwrap_or_default(),
            false => String::new(),
        };
        // (Letters typed while the rows or buttons have focus go to the filter field.)
        if self.filtered && open && !ui.memory(|m| m.has_focus(field_id)) {
            if !filter.is_empty()
                && ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape))
            {
                filter.clear();
                state.cursor = None;
                ui.memory_mut(|m| m.request_focus(field_id));
            }
            let typed: String = ui.input_mut(|i| {
                let mut typed = String::new();
                i.events.retain(|e| match e {
                    egui::Event::Text(t) if !t.chars().any(char::is_control) => {
                        typed.push_str(t);
                        false
                    }
                    _ => true,
                });
                typed
            });
            if !typed.is_empty() {
                filter.push_str(&typed);
                state.cursor = None;
                ui.memory_mut(|m| m.request_focus(field_id));
                // (Its cursor after them, where the next are typed.)
                let mut field =
                    egui::text_edit::TextEditState::load(ui.ctx(), field_id).unwrap_or_default();
                let end = egui::text::CCursor::new(filter.chars().count());
                field.cursor.set_char_range(Some(egui::text::CCursorRange::one(end)));
                field.store(ui.ctx(), field_id);
            }
        }
        let mut keys = MenuKeys {
            cursor: state.cursor,
            pressed,
            reveal: state.reveal,
            rows: 0,
            filter: mg_core::text::Needle::new(&filter),
            clicked: false,
        };
        let (filtered, button) = (self.filtered, state.button);
        let width_id = filter_id.with("width");
        let mut combo = Self::held(self.combo, self.width, ui);
        if filtered {
            // (Room for the Filter field over the rows' own scroll area.)
            let line = ui.text_style_height(&egui::TextStyle::Body) + ui.spacing().interact_size.y;
            combo = combo.height(ROWS_HEIGHT + 2.0 * line);
        }
        let r = combo_looks(ui, |ui| {
            combo.show_ui(ui, |ui| {
                mg_uikit::list_rows(ui);
                if !filtered {
                    return contents(ui, &mut keys);
                }
                // (As wide as when it opened, whatever the filter leaves.)
                if let Some(w) = ui.data(|d| d.get_temp::<f32>(width_id)).filter(|_| open) {
                    ui.set_min_width(w);
                }
                // Opened by a click (not by the keys): the field has the keys.
                if !open && keys.cursor.is_none() {
                    ui.memory_mut(|m| m.request_focus(field_id));
                }
                let before = filter.clone();
                let searched =
                    SearchField::new(field_id, &mut filter).hint("Filter").down().show(ui);
                let entered =
                    searched.response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                match searched.left {
                    Some(FilterLeft::Escape) => ui.close(),
                    Some(FilterLeft::Down) => {
                        if let Some(b) = button {
                            ui.memory_mut(|m| m.request_focus(b));
                        }
                        keys.cursor = Some(0);
                        keys.reveal = true;
                    }
                    None if entered => keys.pressed = Some(keys.cursor.unwrap_or(0)),
                    None => {}
                }
                if filter != before {
                    keys.cursor = None;
                    keys.filter = mg_core::text::Needle::new(&filter);
                }
                egui::ScrollArea::vertical().max_height(ROWS_HEIGHT).show(ui, |ui| {
                    contents(ui, &mut keys);
                    if keys.rows == 0 {
                        ui.weak("Nothing matches the filter");
                        // (Enter with nothing found chooses nothing: the
                        // list stays, its field with the keys, where it
                        // closed as if a row had been taken.)
                        if keys.pressed.take().is_some() {
                            ui.memory_mut(|m| m.request_focus(field_id));
                        }
                    } else if self.stays_open && entered {
                        ui.memory_mut(|m| m.request_focus(field_id));
                    }
                });
                if filter.is_empty() && !ui.is_sizing_pass() {
                    let w = ui.min_rect().width();
                    ui.data_mut(|d| d.insert_temp(width_id, w));
                }
            })
        })
        .response;
        if self.filtered {
            ui.data_mut(|d| d.insert_temp(filter_id, filter));
            // (Where the field's keys put the cursor: at the first row, or
            // none once the rows changed.)
            state.cursor = keys.cursor;
        }
        hold_arrows(ui, &r);
        let still_open = egui::ComboBox::is_open(ui.ctx(), r.id);
        // (A click on a row closes a filtered list too, which a click
        // inside it leaves open for its field.)
        let pressed = keys.pressed.or((keys.clicked && self.filtered).then_some(0));
        if !self.stays_open && pressed.is_some() {
            egui::Popup::close_id(ui.ctx(), r.id.with("popup"));
        }
        state = MenuState {
            button: Some(r.id),
            // (Once open, the rows are known.)
            rows: if still_open { keys.rows } else { state.rows.max(keys.rows) },
            cursor: if still_open || state.cursor.is_some() && !open {
                state.cursor.map(|c| c.min(keys.rows.max(1) - 1))
            } else {
                None
            },
            reveal: false,
        };
        if !self.stays_open && pressed.is_some() {
            state.cursor = None;
        }
        ui.data_mut(|d| d.insert_temp(state_id, state));
        r
    }

    /// A dropdown that is not a list of choices, whose arrow keys would
    /// have nothing to step through: a grid of colors.
    pub(crate) fn show_ui<R>(
        self,
        ui: &mut Ui,
        contents: impl FnOnce(&mut Ui) -> R,
    ) -> egui::InnerResponse<Option<R>> {
        let combo = Self::held(self.combo, self.width, ui);
        combo_looks(ui, |ui| combo.show_ui(ui, contents))
    }
}

/// `show`, a combo box laid out in `ui`, drawn open in the kit's look
/// ([`mg_uikit::combo_style`]: `raised-hover`, not the theme's `bar`), the
/// look `ui` had kept for what follows it.
fn combo_looks<R>(ui: &mut Ui, show: impl FnOnce(&mut Ui) -> R) -> R {
    // (In a narrow form, the Properties panel's, whose grid wraps its
    // text: the box's words cut short with an ellipsis, on one line.)
    if crate::blueprint::narrow(ui) && ui.style().wrap_mode != Some(egui::TextWrapMode::Truncate) {
        let was = ui.style().wrap_mode;
        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Truncate);
        let r = combo_looks(ui, show);
        ui.style_mut().wrap_mode = was;
        return r;
    }
    // (Only while a popup is open, which an open combo box's list is: a
    // `Ui`'s visuals changed are its style copied, kept from every frame
    // of every list.)
    if !egui::Popup::is_any_open(ui.ctx()) {
        return show(ui);
    }
    let kept = ui.visuals().widgets.open;
    mg_uikit::combo_style(ui);
    let r = show(ui);
    ui.visuals_mut().widgets.open = kept;
    r
}

/// How long letters typed to a list go together, seconds: after a rest,
/// what is typed begins anew.
const TYPED_REST: f64 = 1.0;

/// Letters typed to a list whose box `r` has the keys or whose list is
/// open: the option (of `count`, named by `names`, `at` the one chosen)
/// whose name begins with what was typed, case aside, as lists take
/// letters on every desktop (a builder's report: a long one could only
/// be scrolled through). The same letter again goes to the next that
/// begins with it; what is typed after a rest of [`TYPED_REST`] begins
/// anew. The letters are taken: nothing else reads them.
fn typed_to(
    ui: &mut Ui,
    r: &egui::Response,
    names: impl Fn(usize) -> String,
    count: usize,
    at: Option<usize>,
) -> Option<usize> {
    let open = egui::ComboBox::is_open(ui.ctx(), r.id);
    if count == 0 || !(open || r.has_focus()) {
        return None;
    }
    let typed: String = ui.input_mut(|i| {
        let mut typed = String::new();
        i.events.retain(|e| match e {
            egui::Event::Text(t) if !t.chars().any(char::is_control) => {
                typed.push_str(t);
                false
            }
            _ => true,
        });
        typed
    });
    if typed.is_empty() {
        return None;
    }
    let id = r.id.with("typed");
    let now = ui.input(|i| i.time);
    let (mut so_far, last): (String, f64) =
        ui.data(|d| d.get_temp(id)).unwrap_or((String::new(), f64::MIN));
    if now - last > TYPED_REST {
        so_far.clear();
    }
    so_far.push_str(&typed.to_lowercase());
    ui.data_mut(|d| d.insert_temp(id, (so_far.clone(), now)));
    let begins = |i: usize, with: &str| names(i).trim_start().to_lowercase().starts_with(with);
    // (One letter typed again and again: the next with it, round.)
    let mut letters = so_far.chars();
    let first = letters.next()?;
    let repeated = letters.all(|c| c == first);
    let (with, from) = if repeated {
        (first.to_string(), at.map_or(0, |a| a + 1))
    } else {
        (so_far.clone(), at.unwrap_or(0))
    };
    let order = (0..count).map(|k| (from + k) % count);
    let found = order.clone().find(|&i| begins(i, &with));
    // (Typed on past what begins with it: from the first again.)
    found.or_else(|| (0..count).find(|&i| begins(i, &so_far)))
}

/// Where [`Dropdown::show_menu`]'s keys are: the box (last drawn), its
/// rows (when last open) and the row the cursor is at.
#[derive(Clone, Copy, Default)]
struct MenuState {
    button: Option<egui::Id>,
    rows: usize,
    cursor: Option<usize>,
    reveal: bool,
}

/// The keys of a [`Dropdown::show_menu`]'s rows: each row's response goes
/// through [`MenuKeys::row`], in the order shown.
pub(crate) struct MenuKeys {
    cursor: Option<usize>,
    pressed: Option<usize>,
    reveal: bool,
    rows: usize,
    /// What a [`Dropdown::filtered`] list's Filter field holds.
    filter: mg_core::text::Needle,
    /// A row was clicked.
    clicked: bool,
}

impl MenuKeys {
    /// Whether the row named `text` is shown: found by what the Filter
    /// field holds ([`Dropdown::filtered`]; case and accents aside), or
    /// every row without one. A row not shown is not drawn.
    pub(crate) fn shows(&self, text: &str) -> bool {
        self.filter.finds(text)
    }

    /// A row of the list, `r` its response: shown as under the pointer
    /// where the cursor is; true when Enter or Space was pressed there (do
    /// what its click does).
    pub(crate) fn row(&mut self, r: &egui::Response) -> bool {
        let at = self.rows;
        self.rows += 1;
        self.clicked |= r.clicked();
        if self.cursor == Some(at) {
            r.clone().highlight();
            if self.reveal {
                r.scroll_to_me(None);
            }
        }
        self.pressed == Some(at)
    }

    /// A multi-select checkbox row whose label matches [`MenuKeys::shows`]:
    /// clicking the checkbox or pressing Enter/Space on the highlighted row
    /// toggles `checked` and returns whether it changed.
    pub(crate) fn checkbox(&mut self, ui: &mut Ui, checked: &mut bool, label: &str) -> bool {
        if !self.shows(label) {
            return false;
        }
        let was = *checked;
        let r = ui.add(mg_uikit::Checkbox::new(checked, label));
        if self.row(&r) {
            *checked = !*checked;
        }
        *checked != was
    }
}

/// Says on the system's clipboard what Moonglow has copied of its own
/// (objects, tiles, a conversation's lines: kept in Moonglow, not there).
/// Without text there, Windows gives Ctrl+V nothing to paste and the key
/// never reaches Moonglow: copied objects couldn't be pasted with it.
pub(crate) fn mark_clipboard(ctx: &egui::Context, what: &str) {
    ctx.copy_text(format!("Moonglow Toolset: {what}"));
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
// The fonts and the form helpers Moonglow Viewer's forms share with the
// toolset's (field labels, section headings, a palette's swatches) are
// mg-uikit's, under their old names here.
pub use mg_uikit::form::SECTION_GAP;
pub use mg_uikit::{install_fonts, section_heading, swatch, table_heading};

/// A field's label (`mg_uikit::field_label`), which also names the number
/// field after it on its row ([`name_next_number`]).
pub fn field_label(ui: &mut Ui, text: impl Into<String>) -> egui::Response {
    let text = text.into();
    let r = mg_uikit::field_label(ui, text.clone());
    name_number_by(ui, &text, r.rect);
    r
}

/// Names the number field after this on the row (its arrows: "Level: One
/// More"), where no field label names it ([`field_label`] does).
pub(crate) fn name_next_number(ui: &mut Ui, label: &str) {
    let at =
        egui::Rect::from_min_size(ui.cursor().min, egui::vec2(0.0, ui.spacing().interact_size.y));
    name_number_by(ui, label, at);
}

fn number_name_id() -> egui::Id {
    egui::Id::new("number-field-name")
}

/// The label `text` at `rect`, kept for a number field after it on its row.
fn name_number_by(ui: &Ui, text: &str, rect: egui::Rect) {
    let pass = ui.ctx().cumulative_pass_nr();
    let text = text.trim_end_matches(':').trim().to_string();
    ui.data_mut(|d| d.insert_temp(number_name_id(), (pass, text, rect)));
}

/// The label of the number field about to be drawn at `ui`'s cursor: the
/// last one kept this pass, on the same row and before it (taken: a field
/// on a later row isn't named by it).
fn number_name(ui: &Ui) -> Option<String> {
    let kept = ui.data_mut(|d| {
        let kept = d.get_temp::<(u64, String, egui::Rect)>(number_name_id());
        d.remove::<(u64, String, egui::Rect)>(number_name_id());
        kept
    })?;
    let (pass, text, rect) = kept;
    let at = ui.cursor();
    let row = rect.y_range().expand(4.0);
    let same_row = row.contains(at.top()) || row.contains(at.center().y);
    let before = rect.left() <= at.left() + 1.0;
    (pass == ui.ctx().cumulative_pass_nr() && same_row && before && !text.is_empty())
        .then_some(text)
}

/// The field label (`field_label`) of the field about to be drawn at
/// `ui`'s cursor, kept this pass: on its row before it, or right over it
/// where a narrow form stacks the label over its field. Not taken (a
/// number field after it is still named by it).
pub(crate) fn label_before(ui: &Ui) -> Option<egui::Rect> {
    let (pass, _, rect) = ui.data(|d| d.get_temp::<(u64, String, egui::Rect)>(number_name_id()))?;
    let at = ui.cursor();
    let row = rect.y_range().expand(4.0);
    let same_row =
        (row.contains(at.top()) || row.contains(at.center().y)) && rect.left() <= at.left() + 1.0;
    let over = (rect.bottom() - at.top()).abs() <= 10.0 && (rect.left() - at.left()).abs() <= 2.0;
    (pass == ui.ctx().cumulative_pass_nr() && (same_row || over)).then_some(rect)
}

/// How large a gallery's pictures are: the kit's slider of their side in
/// `range` (its value not shown, `text` beside it where given), a point a
/// step. The side moved to, rounded, when it is moved.
pub(crate) fn picture_size_slider(
    ui: &mut egui::Ui,
    mut side: f32,
    range: std::ops::RangeInclusive<f32>,
    text: Option<&str>,
) -> Option<u16> {
    let r = mg_uikit::gallery::size_slider(ui, &mut side, range, text);
    let moved = r.on_hover_text("How large the pictures are").changed();
    moved.then(|| side.round() as u16)
}

/// Opens a gallery's scroll (`shown`) at cell `at` of its grid, as `grid`
/// laid it out this frame: the scroll's own offset set (for the next
/// frame), so that no scroll around it moves (an item's page), the cell's
/// row at the top or the scroll at its end. Whether it is there already
/// (called each frame until it is: the first frames of a tab lay it out
/// at another width).
pub(crate) fn open_gallery_at<R>(
    ctx: &egui::Context,
    shown: &egui::scroll_area::ScrollAreaOutput<R>,
    grid: &mg_uikit::gallery::GridOutput,
    at: usize,
) -> bool {
    let Some(layout) = grid.layout.as_ref() else { return false };
    let cell = layout.rect(grid.rect.min, at);
    let top = shown.inner_rect.top() - shown.state.offset.y;
    let most = (shown.content_size.y - shown.inner_rect.height()).max(0.0);
    let wanted = (cell.top() - top).clamp(0.0, most);
    if (shown.state.offset.y - wanted).abs() < 1.0 {
        return true;
    }
    let mut state = shown.state;
    state.offset.y = wanted;
    state.store(ctx, shown.id);
    ctx.request_repaint();
    false
}

/// A dialog's grid of pictures in a well (the Pickers board's `.well`,
/// M4's U26): `size` with its border, the `inset` fill, a `line-soft`
/// border with corners of 3, and in it a scroll (`id`) whose drawn bar
/// stands at the well's right edge, the cells `pad` from its edges (the
/// bar's room left out of the width they are laid out in, so justified
/// rows reach it). The scroll's output, for [`open_gallery_at`].
pub(crate) fn picture_well<R>(
    ui: &mut Ui,
    id: egui::Id,
    size: egui::Vec2,
    pad: egui::Margin,
    add: impl FnOnce(&mut Ui) -> R,
) -> egui::scroll_area::ScrollAreaOutput<R> {
    let t = mg_uikit::tokens(ui.ctx());
    let layout = egui::Layout::top_down(egui::Align::Min);
    ui.allocate_ui_with_layout(size, layout, |ui| {
        egui::Frame::new()
            .fill(t.inset)
            .stroke(egui::Stroke::new(1.0, t.line_soft))
            .corner_radius(3)
            .inner_margin(1)
            .show(ui, |ui| {
                // (Less its margin and its border, both 1 a side: egui
                // counts a frame's stroke in its size, and a well as wide
                // as the room grew a window 2 points a frame.)
                let inner = size - egui::Vec2::splat(4.0);
                ui.set_min_size(inner);
                ui.set_max_width(inner.x);
                egui::ScrollArea::vertical()
                    .id_salt(id)
                    .max_height(inner.y)
                    .auto_shrink([false, false])
                    .show(ui, |ui| egui::Frame::new().inner_margin(pad).show(ui, add).inner)
            })
            .inner
    })
    .inner
}

/// A picture as [`Thumbnails::made`](crate::model_view::Thumbnails::made)
/// gives it (`Some(None)`: nothing to draw; `None`: not made yet), for
/// the kit's gallery cell.
pub(crate) fn cell_picture(made: Option<Option<egui::TextureId>>) -> mg_uikit::gallery::Picture {
    use mg_uikit::gallery::Picture;
    match made {
        Some(Some(id)) => Picture::Ready(id),
        Some(None) => Picture::Missing,
        None => Picture::Making,
    }
}

/// A PLT color (0 to 175) chosen among a palette's swatches (a TGA of
/// `pal_*`, its rows the colours): its swatch, then a list showing
/// `current` (or `text` in place of its number) that opens on all 176,
/// sixteen to a row, each named "`what` N". `None` for the palette (no
/// game data) leaves the list without swatches. The colour chosen.
pub fn palette_color(
    ui: &mut egui::Ui,
    id_salt: impl egui::AsIdSalt,
    palette: Option<&mg_image::Rgba>,
    what: &str,
    current: u8,
    text: Option<&str>,
) -> Option<u8> {
    let mut pick = None;
    ui.horizontal(|ui| {
        if let Some(p) = palette {
            let (rect, _) = ui.allocate_exact_size(egui::vec2(28.0, 16.0), egui::Sense::hover());
            swatch(ui, rect, &tones(p, current));
        }
        // (The arrow keys step through the colors in their order.)
        let mut chosen = current;
        dropdown(id_salt)
            .selected_text(text.map_or_else(|| current.to_string(), str::to_string))
            .width(56.0)
            .height(11.0 * 16.0 + 24.0)
            .show_with(
                ui,
                &mut chosen,
                || (0..176).collect(),
                |ui, chosen| {
                    let Some(p) = palette else {
                        ui.weak("No palette");
                        return;
                    };
                    let outline = text.is_none();
                    if let Some(i) =
                        mg_uikit::palette_swatches(ui, |i| tones(p, i), what, current, outline)
                    {
                        *chosen = i;
                        pick = Some(i);
                        ui.close();
                    }
                },
            );
        if chosen != current {
            pick = Some(chosen);
        }
    });
    pick
}

/// A palette colour's tones (its row, `index` counted from the top, at four
/// grey levels), as the colour chooser shows them.
pub fn tones(palette: &mg_image::Rgba, index: u8) -> [egui::Color32; 4] {
    let rows = palette.height.max(1);
    let y = rows - 1 - u32::from(index).min(rows - 1);
    [64u32, 128, 192, 250].map(|x| {
        let [r, g, b, _] = palette.pixel(x.min(palette.width.saturating_sub(1)), y);
        egui::Color32::from_rgb(r, g, b)
    })
}

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

/// Width required for a row of header trailing [`KitButton`](mg_uikit::KitButton)s
/// separated by [`HEADER_GAP`](mg_uikit::header::HEADER_GAP).
pub(crate) fn buttons_end_width(ui: &Ui, buttons: &[mg_uikit::KitButton]) -> f32 {
    let gap = mg_uikit::header::HEADER_GAP;
    buttons.iter().map(|b| b.width(ui)).sum::<f32>()
        + gap * (buttons.len().saturating_sub(1)) as f32
}

/// Shows `header` with trailing OK and Cancel (or Close when `close_tip` is
/// `Some`) buttons, returning any [`Pressed`](crate::blueprint::Pressed) click.
pub(crate) fn header_with_ok_cancel<'a>(
    ui: &mut Ui,
    header: mg_uikit::header::EditorHeader<'a>,
    close_tip: Option<&str>,
    tips: [&str; 2],
) -> mg_uikit::header::HeaderOutput<Option<crate::blueprint::Pressed>> {
    let buttons: Vec<mg_uikit::KitButton> = match close_tip {
        Some(tip) => vec![mg_uikit::KitButton::new(None, "Close").tip(tip)],
        None => vec![
            mg_uikit::KitButton::new(None, "OK").look(mg_uikit::ButtonLook::Primary).tip(tips[0]),
            mg_uikit::KitButton::new(None, "Cancel").tip(tips[1]),
        ],
    };
    let end_width = buttons_end_width(ui, &buttons);
    let gap = mg_uikit::header::HEADER_GAP;
    header.show_with_end(ui, end_width, |ui| {
        ui.spacing_mut().item_spacing.x = gap;
        let mut pressed = None;
        for b in buttons {
            let words = b.accessible_name().to_string();
            if ui.add(b).clicked() {
                pressed = Some(match words.as_str() {
                    "Cancel" => crate::blueprint::Pressed::Cancel,
                    _ => crate::blueprint::Pressed::Ok,
                });
            }
        }
        pressed
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A long column's parts out of sight are given the room they took and
    /// not laid out, from the second frame: the column is as long, and the
    /// ids after it are those it had with every part drawn. A part holding
    /// the keyboard's focus is drawn wherever it is.
    #[test]
    fn parts_out_of_sight_keep_their_room_and_ids() {
        let ctx = egui::Context::default();
        let field = std::cell::Cell::new(None);
        let frame = |cull: bool, sight: f32| {
            let mut drawn = 0;
            let mut held = false;
            let mut after = None;
            let mut length = 0.0;
            let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
                ui.set_clip_rect(egui::Rect::from_min_size(
                    ui.cursor().min,
                    egui::vec2(400.0, sight),
                ));
                let top = ui.cursor().min.y;
                for i in 0..500 {
                    let add = |ui: &mut Ui| {
                        drawn += 1;
                        if i == 400 {
                            let mut text = String::from("typed");
                            let edit = ui.text_edit_singleline(&mut text);
                            held |= edit.has_focus();
                            field.set(Some(edit.id));
                        } else {
                            ui.label(format!("Part {i}"));
                        }
                    };
                    if cull {
                        in_sight(ui, i, add);
                    } else {
                        ui.push_id(i, add);
                    }
                }
                length = ui.cursor().min.y - top;
                after = Some(ui.next_auto_id());
            });
            out.textures_delta.clear();
            (drawn, length, after, held)
        };
        let all = frame(false, 200.0);
        assert_eq!(all.0, 500);
        let first = frame(true, 200.0);
        assert_eq!(first, all, "the first frame draws every part");
        let (drawn, length, after, _) = frame(true, 200.0);
        assert!((1..30).contains(&drawn), "{drawn} parts drawn");
        assert_eq!((length, after), (all.1, all.2));
        // A field typed in, in sight, then scrolled away.
        ctx.memory_mut(|m| m.request_focus(field.get().unwrap()));
        assert!(frame(true, 1e5).3);
        let (focused, length, after, held) = frame(true, 200.0);
        assert!(held, "the field keeps its focus");
        assert_eq!(focused, drawn + 1, "the focused part drawn too");
        assert_eq!((length, after), (all.1, all.2));
    }

    /// A header's OK is its main action, filled `accent-fill` (the
    /// proposal's dialog pattern: the accent fill on the main button);
    /// Cancel is not, nor is a header's lone Close.
    #[test]
    fn a_header_s_ok_is_its_main_action() {
        for theme in mg_uikit::Theme::ALL {
            let t = theme.tokens();
            for close in [None, Some("Close the window")] {
                let ctx = egui::Context::default();
                mg_uikit::apply(&ctx, theme);
                ctx.run_ui(egui::RawInput::default(), |_| {}).textures_delta.clear();
                let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
                    let header = mg_uikit::header::EditorHeader::new("ok-cancel");
                    let _ = header_with_ok_cancel(ui, header, close, ["Keep it", "Drop it"]);
                });
                out.textures_delta.clear();
                let filled = out
                    .shapes
                    .iter()
                    .any(|c| matches!(&c.shape, egui::Shape::Rect(r) if r.fill == t.accent_fill));
                assert_eq!(filled, close.is_none(), "{theme:?} {close:?}");
            }
        }
    }

    /// The order of names is kept by every row's name (the review's C-6):
    /// a row renamed among a hundred, between those a sample took, and
    /// another table of the same length, are told apart.
    #[test]
    fn the_name_order_is_kept_by_every_row() {
        let rows = |renamed: Option<(usize, &str)>| -> Vec<mg_rules::Choice> {
            (0..100)
                .map(|row| {
                    let text = match renamed {
                        Some((r, t)) if r == row => t.to_string(),
                        _ => format!("Row {row:03}"),
                    };
                    mg_rules::Choice { row, text }
                })
                .collect()
        };
        let base = choices_sig(&rows(None));
        assert_ne!(base, choices_sig(&rows(Some((5, "Aardvark")))), "a row renamed");
        let other: Vec<mg_rules::Choice> =
            (0..100).map(|row| mg_rules::Choice { row, text: format!("Other {row:03}") }).collect();
        assert_ne!(base, choices_sig(&other), "another table of the same length");
        assert_eq!(base, choices_sig(&rows(None)), "the same rows");
    }

    /// A range that runs backwards (the Levelup Wizard's for a class past
    /// level 60, the review's B3) steps as the same range the right way
    /// round, and doesn't panic.
    #[test]
    fn a_spin_field_s_step_takes_a_range_given_backwards() {
        assert_eq!(spin_step(4.0, 1.0, 5.0, 3.0), 4.0 + 1.0);
        assert_eq!(spin_step(5.0, 1.0, 5.0, 3.0), 5.0, "at its end");
        assert_eq!(spin_step(70.0, 1.0, 61.0, 60.0), 70.0, "outside: never further out");
        assert_eq!(spin_step(70.0, -1.0, 61.0, 60.0), 61.0, "outside: back into it");
        assert_eq!(spin_step(0.0, -1.0, 1.0, 12.0), 0.0);
        assert_eq!(spin_step(0.0, 1.0, 1.0, 12.0), 1.0);
    }

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
            also: Also::default(),
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

    /// An int and a string of one name are two variables, as the game
    /// keeps them: a change to the one leaves the other as it is on the
    /// objects edited with it, a deletion takes the one deleted, and a
    /// variable given another type is that one changed (the review's S27:
    /// by name alone, the change replaced both).
    #[test]
    fn a_variable_edit_merges_by_name_and_type() {
        let int = |name: &str, value: i32| {
            let mut s = Struct::new(0);
            s.set("Name", Value::String(name.as_bytes().to_vec()));
            s.set("Type", Value::Dword(1));
            s.set("Value", Value::Int(value));
            s
        };
        let string = |name: &str, value: &str| {
            let mut s = Struct::new(0);
            s.set("Name", Value::String(name.as_bytes().to_vec()));
            s.set("Type", Value::Dword(3));
            s.set("Value", Value::String(value.as_bytes().to_vec()));
            s
        };
        // The int changed; the string of its name untouched.
        let before = [int("KIND", 1), string("KIND", "smith")];
        let after = [int("KIND", 2), string("KIND", "smith")];
        let theirs = [string("KIND", "tanner"), int("KIND", 1), int("OWN", 3)];
        assert_eq!(
            merge_variables(&before, &after, &theirs),
            [string("KIND", "tanner"), int("KIND", 2), int("OWN", 3)]
        );
        // One with the string alone gets the int beside it.
        assert_eq!(
            merge_variables(&before, &after, &[string("KIND", "tanner")]),
            [string("KIND", "tanner"), int("KIND", 2)]
        );
        // The string deleted: theirs goes, their int stays.
        let after = [int("KIND", 1)];
        assert_eq!(merge_variables(&before, &after, &theirs), [int("KIND", 1), int("OWN", 3)]);
        // An int made a string: the int goes, the string is set.
        let (before, after) = ([int("DC", 15)], [string("DC", "hard")]);
        assert_eq!(
            merge_variables(&before, &after, &[int("OWN", 3), int("DC", 20)]),
            [int("OWN", 3), string("DC", "hard")]
        );
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
        // Both kinds of line end, untouched: as they were; a line added
        // takes the text's own (CRLF, most of its lines').
        let mixed = b"a\r\nb\nc\r\nd".to_vec();
        let ls = LocString::from_text(Language::ENGLISH, Gender::Male, mixed);
        let mut edit = LocStringEdit::new(target(), "Description", &ls);
        assert_eq!(edit.entries[0].2, "a\nb\nc\nd");
        assert_eq!(edit.value(), Some(ls));
        edit.entries[0].2 = "a\nb\nc\nd\ne".into();
        let text =
            edit.value().unwrap().text(Language::ENGLISH, Gender::Male).unwrap().into_owned();
        assert_eq!(text, "a\r\nb\nc\r\nd\r\ne");
        let mut bad = edit;
        bad.strref = "x".into();
        assert_eq!(bad.value(), None);
    }

    #[test]
    fn variables_page_commits_when_unrelated_field_has_focus_and_survives_multi_pass() {
        use egui_kittest::Harness;
        use egui_kittest::kittest::Queryable;

        let key = ResKey::parse("inn", ResType::GIT).unwrap();
        let target = FieldTarget::new(key, GffPath::root(), "VarTable");
        let mut m = mg_module::Module::new();
        let mut git = mg_gff::Gff::new(*b"GIT ");
        git.root.set("VarTable", Value::List(Vec::new()));
        m.set(key, git.to_bytes().unwrap());
        let mut app = Moonglow::new(None, Box::new(crate::NoDialogs::default()));
        app.ws = Some(mg_edit::Workspace::new(m));

        struct State {
            app: Moonglow,
            target: FieldTarget,
            other: String,
            discard_once: bool,
        }

        let id = (key, GffPath::root());
        let mut h = Harness::builder().with_size(egui::vec2(800.0, 600.0)).build_ui_state(
            |ui, s: &mut State| {
                ui.text_edit_singleline(&mut s.other);
                let actions = std::mem::take(&mut s.app.actions);
                for action in actions {
                    if let crate::Action::Apply(cmd) = action {
                        let _ = s.app.apply(cmd);
                    }
                }
                let list = s
                    .app
                    .ws
                    .as_mut()
                    .and_then(|w| w.doc(&s.target.key).ok())
                    .and_then(|g| g.root.list("VarTable"))
                    .map(<[Struct]>::to_vec)
                    .unwrap_or_default();
                super::variables_page(&mut s.app, ui, s.target.clone(), &list, None);
                if s.discard_once {
                    s.discard_once = false;
                    ui.ctx().request_discard("multi-pass test");
                }
            },
            State { app, target, other: String::from("external"), discard_once: false },
        );
        h.run();

        // Add an incomplete draft row (empty name -> does not commit yet).
        h.get_by_label("Add").click();
        h.run();
        assert_eq!(h.state().app.var_pages.get(&id).map(|p| p.edit.rows.len()), Some(1));

        // Simulate a multi-pass layout discard within the same frame: page must remain.
        h.state_mut().discard_once = true;
        h.run();
        assert_eq!(
            h.state().app.var_pages.get(&id).map(|p| p.edit.rows.len()),
            Some(1),
            "multi-pass discard must not prune active VarPage"
        );

        // Give the row a valid name while focus lives in the unrelated TextEdit.
        h.state_mut().app.var_pages.get_mut(&id).unwrap().edit.rows[0].name = "nFlag".into();
        h.get_all_by_role(egui::accesskit::Role::TextInput).next().unwrap().focus();
        h.run();

        let committed = h
            .state_mut()
            .app
            .ws
            .as_mut()
            .unwrap()
            .doc(&key)
            .unwrap()
            .root
            .list("VarTable")
            .map(<[Struct]>::to_vec)
            .unwrap_or_default();
        assert_eq!(
            committed.len(),
            1,
            "expected VarTable commit while unrelated TextEdit had focus"
        );
        assert_eq!(committed[0].string("Name"), Some(&b"nFlag"[..]));
    }

    #[test]
    fn spin_with_cancels_stale_run_on_external_value_change_or_frame_gap() {
        use egui_kittest::Harness;
        use egui_kittest::kittest::Queryable;

        struct State {
            value: i32,
            show_spin: bool,
        }

        let mut h = Harness::builder().with_size(egui::vec2(400.0, 200.0)).build_ui_state(
            |ui, s: &mut State| {
                if s.show_spin
                    && let Some(next) = super::spin_with(ui, s.value, 0..=100, 1.0, |d| d)
                {
                    s.value = next;
                }
            },
            State { value: 10, show_spin: true },
        );
        h.run();

        let up_pos = h.get_by_label("One More").rect().center();
        let hold_up = |h: &mut Harness<'_, State>, start: f64| {
            h.hover_at(up_pos);
            h.input_mut().time = Some(start);
            h.event(egui::Event::PointerButton {
                pos: up_pos,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            });
            h.step();
            h.input_mut().time = Some(start + 0.5);
            h.step();
            h.input_mut().time = Some(start + 0.6);
            h.step();
        };
        let release_up = |h: &mut Harness<'_, State>, at: f64| {
            h.input_mut().time = Some(at);
            h.event(egui::Event::PointerButton {
                pos: up_pos,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            });
            h.step();
        };

        // Normal hold across SPIN_DELAY commits >= 12 upon release.
        hold_up(&mut h, 1.0);
        assert_eq!(h.state().value, 10, "value commits on release, not mid-hold");
        release_up(&mut h, 1.7);
        assert!(
            h.state().value >= 12,
            "expected normal spin hold to commit >= 12, got {}",
            h.state().value
        );

        // External update while held must cancel stale run rather than overwriting 50 on release.
        h.state_mut().value = 10;
        h.step();
        hold_up(&mut h, 2.0);
        h.state_mut().value = 50;
        release_up(&mut h, 2.7);
        assert_eq!(
            h.state().value,
            50,
            "external value change must cancel stale SpinRun rather than overwriting on release"
        );

        // Frame gap while spin is hidden must also cancel SpinRun so re-showing doesn't resume.
        hold_up(&mut h, 3.0);
        h.state_mut().value = 80;
        h.state_mut().show_spin = false;
        h.input_mut().time = Some(3.7);
        h.step();
        h.step();
        h.state_mut().show_spin = true;
        release_up(&mut h, 3.8);
        assert_eq!(h.state().value, 80, "frame gap must cancel stale SpinRun");

        h.input_mut().time = None;
        h.step();
    }
}

#[cfg(test)]
mod select_tests {
    use super::select_in_list;
    use std::collections::BTreeSet;

    /// The rule of every list of which several can be selected (the hak
    /// editor's, the module's haks, Open Script's, the tree's): a click
    /// alone, Ctrl toggling, Shift a range from the anchor, Ctrl+Shift the
    /// range added.
    #[test]
    fn a_click_with_ctrl_and_shift_selects_as_lists_do() {
        let listed = [10, 20, 30, 40, 50];
        let (mut selected, mut anchor) = (BTreeSet::new(), None);
        let mut click = |row, command, shift| {
            select_in_list(&mut selected, &mut anchor, &listed, row, command, shift);
            (selected.iter().copied().collect::<Vec<_>>(), anchor)
        };
        assert_eq!(click(20, false, false), (vec![20], Some(20)));
        assert_eq!(click(40, false, true), (vec![20, 30, 40], Some(20)), "anchor stays");
        assert_eq!(click(10, false, true), (vec![10, 20], Some(20)));
        assert_eq!(click(50, true, false), (vec![10, 20, 50], Some(50)));
        assert_eq!(click(20, true, false), (vec![10, 50], Some(20)), "Ctrl takes one away");
        assert_eq!(click(40, true, true), (vec![10, 20, 30, 40, 50], Some(20)));
        // Shift with no anchor listed is a click alone.
        let (mut selected, mut anchor) = (BTreeSet::new(), Some(99));
        select_in_list(&mut selected, &mut anchor, &listed, 30, false, true);
        assert_eq!((selected.into_iter().collect::<Vec<_>>(), anchor), (vec![30], Some(30)));
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

#[cfg(test)]
mod row_choice_tests {
    use egui_kittest::Harness;
    use egui_kittest::kittest::{NodeT, Queryable};

    /// The list of a dropdown of `count` rows (one of them long) open:
    /// where its box is, and where its list is on each of the first 60
    /// frames it is drawn.
    fn open_list(count: usize, long_row: bool) -> (egui::Rect, Vec<egui::Rect>) {
        struct State {
            choices: Vec<mg_rules::Choice>,
        }
        let mut choices: Vec<_> = (0..count)
            .map(|row| mg_rules::Choice { row, text: format!("Choice {row:03}") })
            .collect();
        if long_row {
            choices[count / 2].text = "A choice with a name far longer than its box".into();
        }
        let state = State { choices };
        let mut h = Harness::builder().with_size(egui::vec2(800.0, 600.0)).build_ui_state(
            |ui, s: &mut State| {
                // (The application's scroll bars: room of their own.)
                let bars = egui::style::ScrollStyle::solid();
                if ui.ctx().global_style().spacing.scroll != bars {
                    ui.ctx().all_styles_mut(|s| s.spacing.scroll = bars);
                }
                let id = egui::Id::new("list");
                ui.horizontal(|ui| {
                    ui.add_space(100.0);
                    super::row_choice(id, &s.choices, 3).width(160.0).show(ui);
                });
            },
            state,
        );
        h.run();
        let combo = egui_kittest::kittest::by().role(egui::accesskit::Role::ComboBox);
        let at = h.get(combo).rect();
        h.get(egui_kittest::kittest::by().role(egui::accesskit::Role::ComboBox)).click();
        // (Its first pass measures it, unseen.)
        h.step();
        let mut lists = Vec::new();
        for _ in 0..60 {
            h.step();
            let list = h.ctx.memory(|m| {
                let top = m.areas().top_layer_id(egui::Order::Foreground)?;
                m.area_rect(top.id)
            });
            lists.push(list.expect("the list, open"));
        }
        (at, lists)
    }

    /// An open list keeps one width from the first frame it is drawn (a
    /// long one grew to the window's edge under Windows: a builder's
    /// report): the box's, short or long, or its widest row's.
    #[test]
    fn an_open_list_keeps_its_width() {
        // A long list with a Filter field, and a short one taller than
        // egui's list (with egui's scroll bar).
        for count in [200, 20] {
            let (at, lists) = open_list(count, false);
            assert!(lists.iter().all(|l| *l == lists[0]), "{count} rows: {lists:?}");
            let list = lists[0];
            assert!(
                (list.left() - at.left()).abs() < 1.0 && (list.right() - at.right()).abs() < 1.0,
                "{count} rows: the list {list:?} under the box {at:?}"
            );
        }
        // A long list with a row wider than its box: as wide as that row.
        let (at, lists) = open_list(200, true);
        assert!(lists.iter().all(|l| *l == lists[0]), "{lists:?}");
        assert!(lists[0].width() > at.width() + 50.0, "{lists:?} beside {at:?}");
    }

    /// A question's "Always do this" box keeps its tick while the question
    /// is asked, and comes unticked when it is asked again later: kept, one
    /// answer became the setting again the next time (the review's S26).
    #[test]
    fn an_always_box_is_unticked_when_its_question_is_asked_again() {
        let id = egui::Id::new("a-question-always");
        let mut h = Harness::builder().with_size(egui::vec2(400.0, 200.0)).build_ui_state(
            move |ui, (asked, on): &mut (bool, bool)| {
                if *asked {
                    // (As a footer's check box shows it: `Foot::check`.)
                    let (mut now, tip) = super::always_now(ui.ctx(), id, "Options");
                    ui.checkbox(&mut now, "Always do this").on_hover_text(tip);
                    *on = super::always_kept(ui.ctx(), id, Some(now));
                } else {
                    ui.label("Nothing asked");
                }
            },
            (true, false),
        );
        h.run();
        h.get_by_label("Always do this").click();
        h.run();
        h.run();
        assert!(h.state().1, "ticked, and kept while it is asked");
        // Answered; asked again some frames on.
        h.state_mut().0 = false;
        h.run();
        h.run();
        h.state_mut().0 = true;
        h.run();
        assert!(!h.state().1, "asked anew: not ticked");
        h.get_by_label("Always do this").click();
        h.run();
        assert!(h.state().1);
    }

    /// A described variable's values from a table of 20,000 rows
    /// (`rows = "placeables"` with CEP): the list lays out the rows in
    /// sight, the chosen one among them, shows each value with what it
    /// means, and the one clicked becomes the variable's value. Closed, it
    /// costs nothing a row: the values were copied each frame, and the
    /// open list laid out whole, each row looked up among them all (the
    /// review's S20).
    #[test]
    fn a_row_s_kept_note_is_the_note_made_anew() {
        // (A row's note was made every frame, its value looked for in the
        // whole of its variable's values: kept now, for its name, type
        // and value, until a variable's values are read. The last pass's
        // item 17.)
        let mut variables = mg_module::var_desc::Variables::default();
        let source = "[KIND]\ntype = \"string\"\nvalues = [\"smith\", \"tanner\"]\n\
                      [DC]\ntype = \"int\"\nmin = 5\nmax = 40\n";
        variables.apply("variables.toml", source, &mut Vec::new()).unwrap();
        use super::{VarHelp, VarRow};
        let mut help = VarHelp { variables, ..Default::default() };
        let rows = [
            VarRow::new("KIND", 3, "smith"),
            VarRow::new("KIND", 3, "Smith"),
            VarRow::new("KIND", 1, "2"),
            VarRow::new("DC", 1, "50"),
            VarRow::new("DC", 1, "10"),
            VarRow::new("OTHER", 1, "1"),
        ];
        let fresh = |help: &VarHelp| rows.iter().map(|r| help.note(r)).collect::<Vec<_>>();
        let made = fresh(&help);
        for _ in 0..2 {
            assert_eq!(rows.iter().map(|r| help.kept_note(r)).collect::<Vec<_>>(), made);
        }
        // Its values read (here none come from elsewhere): made again.
        let d = help.variables.describe("KIND").unwrap().clone();
        let module = mg_module::Module::new();
        let c = d.choices(&mg_module::var_desc::ModuleLookup::new(&module, None));
        help.choices.insert(d.name.clone(), c);
        help.notes.clear();
        let made = fresh(&help);
        assert!(made[1].as_deref().is_some_and(|n| n.contains("case and all")), "{made:?}");
        assert_eq!(made[0], None, "a value it has");
        for _ in 0..2 {
            assert_eq!(rows.iter().map(|r| help.kept_note(r)).collect::<Vec<_>>(), made);
        }
    }

    #[test]
    fn a_described_variable_s_values_lay_out_only_the_rows_in_sight() {
        use mg_module::var_desc::{Choice, Choices};
        struct State {
            choices: Choices,
            offered: super::Offered,
            value: String,
        }
        let list: Vec<Choice> = (0..20_000)
            .map(|n| Choice { value: n.to_string(), note: Some(format!("Placeable {n:05}")) })
            .collect();
        let choices = Choices { list, rows: Some(20_000), problem: None };
        let offered = super::Offered::of(&choices);
        // (As the list each frame made them: every value, each with what
        // it means, found by its value.)
        for (n, x) in choices.list.iter().enumerate().step_by(997) {
            assert_eq!(offered.at[&x.value], n);
            assert_eq!(offered.rows[n].text, format!("{n} — Placeable {n:05}"));
            assert_eq!(offered.rows[n].row, n);
        }
        let state = State { choices, offered, value: "12345".into() };
        let mut h = Harness::builder().with_size(egui::vec2(800.0, 600.0)).build_ui_state(
            |ui, s: &mut State| {
                super::value_choice(ui, 0, false, &s.choices, &s.offered, &mut s.value, 220.0);
            },
            state,
        );
        h.run();
        let t = std::time::Instant::now();
        for _ in 0..20 {
            h.step();
        }
        // (Closed: 20,000 texts were made a frame.)
        assert!(t.elapsed() < std::time::Duration::from_millis(20 * 10), "{:?}", t.elapsed());
        let combo = egui_kittest::kittest::by().role(egui::accesskit::Role::ComboBox);
        assert_eq!(h.get(combo).value().as_deref(), Some("12345 — Placeable 12345"));
        h.get(egui_kittest::kittest::by().role(egui::accesskit::Role::ComboBox)).click();
        h.run();
        let rows = h.query_all_by_label_contains(" — Placeable ").count();
        assert!(rows < 30, "{rows} rows laid out");
        assert!(h.query_by_label("12345 — Placeable 12345").is_some(), "the chosen one in sight");
        let t = std::time::Instant::now();
        for _ in 0..20 {
            h.step();
        }
        assert!(t.elapsed() < std::time::Duration::from_millis(20 * 40), "{:?}", t.elapsed());
        // (In the description's order, not by name: 12346 follows 12345.)
        h.get_by_label("12346 — Placeable 12346").click();
        h.run();
        assert_eq!(h.state().value, "12346");
        // A value it doesn't list is said; one typed too, where it takes
        // others.
        h.state_mut().value = "none".into();
        h.run();
        let combo = egui_kittest::kittest::by().role(egui::accesskit::Role::ComboBox);
        assert_eq!(h.get(combo).value().as_deref(), Some("none (not one of its values)"));
    }

    /// Select Resource keeps what its Filter finds until what is typed
    /// changes: the names it finds, in their order, as a look through
    /// them all finds.
    #[test]
    fn select_resource_keeps_what_its_filter_finds() {
        let choices: Vec<mg_core::ResRef> = (0..500)
            .map(|n| mg_core::ResRef::from_str(&format!("as_sound_{n:03}")).unwrap())
            .collect();
        let mut p = super::Picker {
            id: egui::Id::new("picker"),
            title: "Select Resource".into(),
            filter: String::new(),
            choices: choices.clone(),
            found: None,
        };
        assert_eq!(p.shown(), choices);
        for typed in ["12", "SOUND 4", "sound_499", "none", "", "12"] {
            p.filter = typed.into();
            let filter = mg_core::text::Needle::new(typed);
            let look: Vec<mg_core::ResRef> =
                choices.iter().filter(|r| filter.finds(&r.to_string())).copied().collect();
            assert_eq!(p.shown(), look, "{typed}");
            assert_eq!(p.shown(), look, "{typed}, again");
            assert_eq!(p.found.as_ref().map(|f| f.0.as_str()), Some(filter.as_str()));
        }
    }

    /// A list of tens of thousands of rows (a 2DA's) lays out only those
    /// in sight while open, the chosen one among them in the middle,
    /// and isn't sorted or searched again each frame: 62 frames of one
    /// of 50,000 took 5.8 s before (93 ms a frame, in a release build),
    /// 20 ms after.
    #[test]
    fn a_long_list_lays_out_only_the_rows_in_sight() {
        struct State {
            choices: Vec<mg_rules::Choice>,
        }
        let choices = (0..50_000)
            .map(|row| mg_rules::Choice { row, text: format!("Choice {row:05}") })
            .collect();
        let mut h = Harness::builder().with_size(egui::vec2(800.0, 600.0)).build_ui_state(
            |ui, s: &mut State| {
                let id = egui::Id::new("list");
                super::row_choice(id, &s.choices, 31_234).width(160.0).show(ui);
            },
            State { choices },
        );
        h.run();
        h.get(egui_kittest::kittest::by().role(egui::accesskit::Role::ComboBox)).click();
        h.run();
        let rows = h.query_all_by_label_contains("Choice ").count();
        assert!(rows < 30, "{rows} rows laid out");
        assert!(h.query_by_label("Choice 31234").is_some(), "the chosen one in sight");
        // Narrowed by the filter: from its top.
        h.get(egui_kittest::kittest::by().role(egui::accesskit::Role::TextInput)).click();
        h.run();
        h.event(egui::Event::Text("4999".into()));
        h.run();
        h.run();
        let shown: Vec<String> = (h.query_all_by_label_contains("Choice "))
            .map(|n| n.accesskit_node().label().unwrap_or_default())
            .collect();
        assert_eq!(shown[0], "Choice 04999", "{shown:?}");
        let t = std::time::Instant::now();
        for _ in 0..20 {
            h.step();
        }
        // (A frame of it in a debug build, with a margin: it took 93 ms
        // in a release build when every row was laid out.)
        assert!(t.elapsed() < std::time::Duration::from_millis(20 * 40), "{:?}", t.elapsed());
    }

    #[test]
    fn capitalize_first_handles_empty_ascii_and_multibyte_chars() {
        assert_eq!(super::capitalize_first(""), "");
        assert_eq!(super::capitalize_first("variable set"), "Variable set");
        assert_eq!(super::capitalize_first("éclair"), "Éclair");
    }
}

#[cfg(test)]
mod text_menu_tests {
    use egui_kittest::Harness;
    use egui_kittest::kittest::{NodeT, Queryable};

    /// A text field whose text can't be changed (an immutable `&str`, as
    /// a text only looked at is shown) has Copy and Select All on its
    /// right-click menu, and no Cut or Paste; one whose text can, all four
    /// (with text selected).
    #[test]
    fn a_read_only_text_s_menu_has_no_cut_or_paste() {
        for read_only in [true, false] {
            let mut h = Harness::new_ui_state(
                move |ui, text: &mut String| {
                    super::text_menus_begin(ui.ctx());
                    let mut fixed = text.as_str();
                    let field = match read_only {
                        true => egui::TextEdit::singleline(&mut fixed),
                        false => egui::TextEdit::singleline(text),
                    };
                    ui.add(field.id(egui::Id::new("field")));
                    super::text_menus_end(ui.ctx());
                },
                "Looked at".to_string(),
            );
            h.run();
            h.get_by_role(egui::accesskit::Role::TextInput).click();
            h.run();
            h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
            h.run();
            let at = h.get_by_role(egui::accesskit::Role::TextInput).rect().center();
            h.hover_at(at);
            for pressed in [true, false] {
                h.event(egui::Event::PointerButton {
                    pos: at,
                    button: egui::PointerButton::Secondary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                });
            }
            h.run();
            let on = |h: &Harness<'_, String>, name: &str| {
                let row = h.query_all_by_role(egui::accesskit::Role::Button).find(|n| {
                    n.accesskit_node().label().is_some_and(|l| l.split(' ').next() == Some(name))
                });
                row.map(|n| !n.accesskit_node().is_disabled())
            };
            let rows = ["Cut", "Copy", "Paste", "Select"].map(|r| on(&h, r));
            let editable = Some(!read_only);
            assert_eq!(
                rows,
                [editable, Some(true), editable, Some(true)],
                "read only: {read_only}"
            );
        }
    }

    /// A click on a button beside a text field leaves the field's
    /// selection for the button to work on, in the frame it is clicked,
    /// though the field asks whether its menu is wanted before it is drawn
    /// (as the script editor does: its Replace works on the selected
    /// match). egui's `read_response` there gave up the field's focus
    /// before it was drawn, and the field dropped its selection.
    #[test]
    fn a_click_beside_a_text_field_keeps_its_selection() {
        let mut h = Harness::new_ui_state(
            |ui, (text, seen): &mut (String, Option<bool>)| {
                let id = egui::Id::new("field");
                super::text_menus_begin(ui.ctx());
                super::text_menu_wanted(ui.ctx(), id);
                ui.add(egui::TextEdit::singleline(text).id(id));
                if ui.button("Use Selection").clicked() {
                    let range = egui::text_edit::TextEditState::load(ui.ctx(), id)
                        .and_then(|s| s.cursor.char_range());
                    *seen = Some(range.is_some_and(|r| r.primary != r.secondary));
                }
                super::text_menus_end(ui.ctx());
            },
            ("Selected".to_string(), None),
        );
        h.run();
        h.get_by_role(egui::accesskit::Role::TextInput).click();
        h.run();
        h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
        h.run();
        h.get_by_label("Use Selection").click();
        h.run();
        assert_eq!(h.state().1, Some(true), "the selection there as the button is clicked");
    }

    use egui::accesskit::Role;

    /// A right click (press and release) at `at`.
    fn right_click<S>(h: &mut Harness<'_, S>, at: egui::Pos2) {
        h.hover_at(at);
        for pressed in [true, false] {
            h.event(egui::Event::PointerButton {
                pos: at,
                button: egui::PointerButton::Secondary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            });
        }
        h.run();
    }

    /// The open text menu's row whose name begins with `name`: on or off,
    /// and where.
    fn row<S>(h: &Harness<'_, S>, name: &str) -> Option<(bool, egui::Rect)> {
        let row = h.query_all_by_role(Role::Button).find(|n| {
            n.accesskit_node().label().is_some_and(|l| l.split(' ').next() == Some(name))
        })?;
        Some((!row.accesskit_node().is_disabled(), row.rect()))
    }

    /// The menu's Cut, Copy, Paste and Select All, on or off (`None`: not
    /// there).
    fn rows<S>(h: &Harness<'_, S>) -> [Option<bool>; 4] {
        ["Cut", "Copy", "Paste", "Select"].map(|r| row(h, r).map(|(on, _)| on))
    }

    /// A click on the menu's row `name`, its press and release in one
    /// frame (as a quick click comes).
    fn choose<S>(h: &mut Harness<'_, S>, name: &str) {
        let (on, rect) = row(h, name).unwrap_or_else(|| panic!("no row {name}"));
        assert!(on, "{name} is on");
        let at = rect.center();
        let button = |pressed| egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        h.input_mut().events.extend([egui::Event::PointerMoved(at), button(true), button(false)]);
        h.run();
    }

    /// The text field with the keys, its text all selected.
    fn select_all<S>(h: &mut Harness<'_, S>) {
        h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
        h.run();
    }

    /// A row is chosen by the release over it: pressed on Copy, dragged
    /// to Paste and let go, nothing is chosen and the menu stays open (as
    /// desktop menus do); pressed and let go on Select All, it is chosen.
    #[test]
    fn a_row_pressed_and_dragged_off_is_not_chosen() {
        let mut h = Harness::new_ui_state(
            |ui, text: &mut String| {
                super::text_menus_begin(ui.ctx());
                ui.add(egui::TextEdit::singleline(text).id(egui::Id::new("field")));
                super::text_menus_end(ui.ctx());
            },
            "Kept".to_string(),
        );
        h.run();
        h.get_by_role(Role::TextInput).click();
        h.run();
        select_all(&mut h);
        let at = h.get_by_role(Role::TextInput).rect().center();
        right_click(&mut h, at);
        let (_, copy) = row(&h, "Copy").expect("the menu");
        let (_, paste) = row(&h, "Paste").expect("the menu");
        let button = |pos, pressed| egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        h.hover_at(copy.center());
        h.event(button(copy.center(), true));
        h.run();
        assert!(row(&h, "Copy").is_some(), "open while pressed");
        h.hover_at(paste.center());
        h.event(button(paste.center(), false));
        h.run();
        assert_eq!(rows(&h), [Some(true); 4], "nothing chosen: still open");
        assert!(!h.ctx.input(|i| i.pointer.primary_down()), "no button left down");
        choose(&mut h, "Select");
        assert_eq!(rows(&h), [None; 4], "Select All chosen by its release");
    }

    /// A password's field: Cut and Copy off on its menu, with its text
    /// selected (egui's field copies nothing from it, and Cut would delete
    /// what it never copied); Paste and Select All on.
    #[test]
    fn a_password_s_menu_has_no_cut_or_copy() {
        let mut h = Harness::new_ui_state(
            |ui, text: &mut String| {
                super::text_menus_begin(ui.ctx());
                ui.add(egui::TextEdit::singleline(text).password(true).id(egui::Id::new("pw")));
                super::text_menus_end(ui.ctx());
            },
            "secret".to_string(),
        );
        h.run();
        h.get_by_role(Role::PasswordInput).click();
        h.run();
        select_all(&mut h);
        let at = h.get_by_role(Role::PasswordInput).rect().center();
        right_click(&mut h, at);
        assert_eq!(rows(&h), [Some(false), Some(false), Some(true), Some(true)]);
    }

    /// Typing while the menu is open (opened by the mouse, no row lit)
    /// closes it, and what was typed is the menu's: the field is left as it
    /// was, as a desktop menu takes the keys while it is open. Typed again,
    /// the field has it: it kept the keys.
    #[test]
    fn typing_while_the_menu_is_open_closes_it_and_types_nothing() {
        let mut h = Harness::new_ui_state(
            |ui, text: &mut String| {
                super::text_menus_begin(ui.ctx());
                ui.add(egui::TextEdit::singleline(text).id(egui::Id::new("field")));
                super::text_menus_end(ui.ctx());
            },
            "Kept".to_string(),
        );
        h.run();
        h.get_by_role(Role::TextInput).click();
        h.run();
        let at = h.get_by_role(Role::TextInput).rect().center();
        right_click(&mut h, at);
        assert!(row(&h, "Paste").is_some(), "the menu");
        let key = |key, pressed| egui::Event::Key {
            key,
            physical_key: None,
            pressed,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        };
        // (As a key comes: its press and its text in one frame.)
        let typed = [key(egui::Key::X, true), egui::Event::Text("x".into())];
        h.input_mut().events.extend(typed.clone());
        h.run();
        assert_eq!(rows(&h), [None; 4], "closed");
        assert_eq!(h.state(), "Kept", "typed into nothing");
        // (Its text a frame later, as kittest gives each queued event one.)
        h.event(key(egui::Key::Y, true));
        h.event(egui::Event::Text("y".into()));
        h.run();
        assert_eq!(h.state(), "Kepty", "the field had the keys");
        right_click(&mut h, at);
        h.event(key(egui::Key::Z, true));
        h.event(egui::Event::Text("z".into()));
        h.run();
        assert_eq!(h.state(), "Kepty", "a key's text in the next frame: the menu's too");
    }

    /// A dropdown's Filter field (a long list's: the Resources panel's
    /// type filter, Add hak…): a right click on it keeps the list open and
    /// shows the menu over it, and Select All and Paste work on the field
    /// (egui keeps one popup open: the menu, one of egui's, closed the list,
    /// and with it the field and the menu).
    #[test]
    fn a_dropdown_s_filter_field_keeps_its_list_open_under_the_menu() {
        let mut h = Harness::new_ui_state(
            |ui, chosen: &mut Option<&'static str>| {
                super::text_menus_begin(ui.ctx());
                super::pastes_begin(ui.ctx());
                super::dropdown("kinds").selected_text("Kinds").filtered(true).show_menu(
                    ui,
                    |ui, keys| {
                        for name in ["Alpha", "Beta", "Gamma"] {
                            if !keys.shows(name) {
                                continue;
                            }
                            let r = ui.selectable_label(false, name);
                            if keys.row(&r) || r.clicked() {
                                *chosen = Some(name);
                            }
                        }
                    },
                );
                super::text_menus_end(ui.ctx());
            },
            None,
        );
        h.run();
        h.get_by_role(Role::ComboBox).click();
        h.run();
        let filter = |h: &Harness<'_, Option<&'static str>>| {
            h.query_by_role(Role::TextInput).map(|n| (n.rect(), n.accesskit_node().value()))
        };
        let (rect, _) = filter(&h).expect("the list's Filter");
        h.get_by_role(Role::TextInput).click();
        h.run();
        h.event(egui::Event::Text("al".into()));
        h.run();
        assert!(h.query_by_label("Beta").is_none(), "filtered");
        right_click(&mut h, rect.center());
        assert!(filter(&h).is_some(), "the list still open");
        assert_eq!(rows(&h), [Some(false), Some(false), Some(true), Some(true)]);
        choose(&mut h, "Select");
        assert!(filter(&h).is_some(), "still open after the row");
        right_click(&mut h, rect.center());
        assert_eq!(rows(&h), [Some(true); 4], "all selected: Cut and Copy on");
        choose(&mut h, "Paste");
        h.event(egui::Event::Paste("bet".into()));
        h.run();
        assert_eq!(filter(&h).map(|(_, v)| v), Some(Some("bet".to_string())), "pasted over it");
        assert!(h.query_by_label("Beta").is_some() && h.query_by_label("Alpha").is_none());
        // Escape closes the menu alone, the list stays.
        right_click(&mut h, rect.center());
        h.key_press(egui::Key::Escape);
        h.run();
        assert_eq!(rows(&h), [None; 4]);
        assert!(filter(&h).is_some(), "the list kept at the menu's Escape");
        assert_eq!(*h.state(), None, "no row of the list chosen");
    }

    /// A field in a kit popover (the Brush popover's rows): its menu opens
    /// over the popover, which stays open, and Escape closes the menu
    /// alone.
    #[test]
    fn a_popover_s_field_keeps_its_popover_open_under_the_menu() {
        let mut h = Harness::new_ui_state(
            |ui, text: &mut String| {
                super::text_menus_begin(ui.ctx());
                let button = ui.button("Brush");
                mg_uikit::popover::Popover::new("Brush")
                    .title(None, "Brush")
                    .custom(|ui| {
                        ui.add(egui::TextEdit::singleline(text).id(egui::Id::new("size")));
                    })
                    .show(ui, &button);
                super::text_menus_end(ui.ctx());
            },
            "12".to_string(),
        );
        h.run();
        h.get_by_label("Brush").click();
        h.run();
        h.get_by_role(Role::TextInput).click();
        h.run();
        select_all(&mut h);
        let at = h.get_by_role(Role::TextInput).rect().center();
        right_click(&mut h, at);
        assert_eq!(rows(&h), [Some(true); 4]);
        choose(&mut h, "Copy");
        assert!(h.query_by_role(Role::TextInput).is_some(), "the popover open after the row");
        right_click(&mut h, at);
        h.key_press(egui::Key::Escape);
        h.run();
        assert_eq!(rows(&h), [None; 4]);
        assert!(h.query_by_role(Role::TextInput).is_some(), "the popover kept at the Escape");
    }

    /// A number typed into (a DragValue, clicked to type: a color's
    /// channel in its picker, inside a popup): its menu works on the text
    /// typed, and the number goes on being typed into after it.
    #[test]
    fn a_number_typed_into_has_the_menu_in_a_popup() {
        let mut h = Harness::new_ui_state(
            |ui, value: &mut f32| {
                super::text_menus_begin(ui.ctx());
                egui::ComboBox::from_id_salt("picker")
                    .selected_text("Color")
                    .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
                    .show_ui(ui, |ui| {
                        ui.add(egui::DragValue::new(value));
                    });
                super::text_menus_end(ui.ctx());
            },
            40.0,
        );
        h.run();
        h.get_by_role(Role::ComboBox).click();
        h.run();
        // (A click and no drag: the pointer's own press and release, as a
        // mouse sends them.)
        let at = h.get_by_role(Role::SpinButton).rect().center();
        h.hover_at(at);
        for pressed in [true, false] {
            h.event(egui::Event::PointerButton {
                pos: at,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            });
        }
        h.run();
        // (Typed into: its text field has the keys, though it is still
        // named a spin button.)
        let focused = h.ctx.memory(|m| m.focused()).expect("the keys");
        assert!(egui::text_edit::TextEditState::load(&h.ctx, focused).is_some(), "typed into");
        let at = h.get_by_role(Role::SpinButton).rect().center();
        select_all(&mut h);
        right_click(&mut h, at);
        assert_eq!(rows(&h), [Some(true); 4]);
        choose(&mut h, "Cut");
        h.event(egui::Event::Text("75".into()));
        h.key_press(egui::Key::Enter);
        h.run();
        assert_eq!(*h.state(), 75.0, "typed in after the menu's Cut");
    }

    /// The pointer resting on the open menu shows no tip of what is under
    /// it (a button with a tip, the menu over it), nor, resting on a button
    /// beside the field, that button's: the tips of the field's layer wait
    /// while the menu is open, as they do while egui's own menus are.
    #[test]
    fn no_tip_under_the_open_menu() {
        let mut h = Harness::new_ui_state(
            |ui, text: &mut String| {
                super::text_menus_begin(ui.ctx());
                ui.style_mut().interaction.tooltip_delay = 0.0;
                ui.horizontal(|ui| {
                    ui.add(egui::TextEdit::singleline(text).id(egui::Id::new("field")));
                    ui.button("Beside").on_hover_text("Beside the field");
                });
                for i in 0..6 {
                    ui.button(format!("Button {i}")).on_hover_text("Under the menu");
                }
                super::text_menus_end(ui.ctx());
            },
            "Text".to_string(),
        );
        h.run();
        h.get_by_role(Role::TextInput).click();
        h.run();
        let at = h.get_by_role(Role::TextInput).rect().left_bottom() + egui::vec2(4.0, -2.0);
        right_click(&mut h, at);
        let (_, paste) = row(&h, "Paste").expect("the menu");
        let under = h.get_by_label("Button 2").rect();
        assert!(paste.intersects(under) || under.contains(paste.left_center()), "over a button");
        let rest = egui::pos2(paste.left() + 4.0, paste.center().y);
        h.hover_at(rest);
        h.run_steps(4);
        assert!(row(&h, "Paste").is_some(), "the menu still open");
        assert!(h.query_by_label("Under the menu").is_none(), "no tip of what is under it");
        let beside = h.get_by_label("Beside").rect();
        assert!(!beside.intersects(paste), "beside the menu, not under it");
        h.hover_at(beside.center());
        h.run_steps(4);
        assert!(row(&h, "Paste").is_some(), "the menu still open");
        assert!(h.query_by_label("Beside the field").is_none(), "no tip beside it either");
    }
}

/// A field's menu with a plugin's rows and submenu (from the second review's tests).
#[cfg(test)]
mod text_menu_plugin_tests {
    use egui::accesskit::Role;
    use egui_kittest::Harness;
    use egui_kittest::kittest::Queryable;

    #[derive(Default)]
    struct S {
        text: String,
        plug: bool,
        deep: bool,
        other: u32,
    }

    fn app(ui: &mut egui::Ui, s: &mut S) {
        let id = egui::Id::new("field");
        super::text_menus_begin(ui.ctx());
        ui.add(egui::TextEdit::singleline(&mut s.text).id(id));
        if ui.button("Other").clicked() {
            s.other += 1;
        }
        super::own_text_menu(ui.ctx(), id);
        let (_, r) = super::text_menu(ui.ctx(), id, None, |ui, _rows| {
            let plug = ui.button("Plug").clicked();
            let deep = mg_uikit::menu_button(ui, "Sub", |ui| ui.button("Deep").clicked());
            (plug, deep.inner.unwrap_or(false))
        });
        if let Some((p, d)) = r {
            s.plug |= p;
            s.deep |= d;
        }
        super::text_menus_end(ui.ctx());
    }

    fn btn(at: egui::Pos2, b: egui::PointerButton, pressed: bool) -> egui::Event {
        egui::Event::PointerButton { pos: at, button: b, pressed, modifiers: egui::Modifiers::NONE }
    }

    fn open(h: &mut Harness<'_, S>) {
        h.run();
        h.get_by_role(Role::TextInput).click();
        h.run();
        let at = h.get_by_role(Role::TextInput).rect().center();
        h.hover_at(at);
        h.event(btn(at, egui::PointerButton::Secondary, true));
        h.event(btn(at, egui::PointerButton::Secondary, false));
        h.run();
        assert!(h.query_by_label_contains("Paste").is_some(), "menu open");
    }

    fn click_at(h: &mut Harness<'_, S>, at: egui::Pos2) {
        h.input_mut().events.extend([
            egui::Event::PointerMoved(at),
            btn(at, egui::PointerButton::Primary, true),
            btn(at, egui::PointerButton::Primary, false),
        ]);
        h.run();
        h.run();
    }

    /// A row a plugin adds to a field's menu (the script editor's) is clicked as any button is.
    #[test]
    fn a_plugin_row_of_the_menu_is_clicked() {
        let mut h = Harness::new_ui_state(app, S { text: "abc".into(), ..Default::default() });
        open(&mut h);
        let at = h.get_by_label("Plug").rect().center();
        h.hover_at(at);
        h.run();
        click_at(&mut h, at);
        assert!(h.state().plug, "top-level plugin row clicked");
    }

    /// A row in a submenu a plugin adds (an API 0.7 `menu`, egui's own submenu) is clicked.
    #[test]
    fn a_plugin_row_in_a_submenu_is_clicked() {
        let mut h = Harness::new_ui_state(app, S { text: "abc".into(), ..Default::default() });
        open(&mut h);
        let at = h.get_by_label_contains("Sub").rect().center();
        h.hover_at(at);
        for _ in 0..5 {
            h.run();
        }
        let deep = h.query_by_label("Deep").map(|n| n.rect().center());
        let Some(at) = deep else { panic!("submenu never shown") };
        h.hover_at(at);
        h.run();
        click_at(&mut h, at);
        assert!(h.state().deep, "submenu plugin row clicked");
    }

    /// A press on a menu row whose release never comes (the window lost the keys mid-press): the next click elsewhere is a click, no button left down.
    #[test]
    fn a_press_whose_release_never_comes_is_forgotten() {
        let mut h = Harness::new_ui_state(app, S { text: "abc".into(), ..Default::default() });
        open(&mut h);
        let at = h.get_by_label_contains("Select").rect().center();
        h.hover_at(at);
        h.event(btn(at, egui::PointerButton::Primary, true));
        h.run();
        // The window loses focus; no release ever comes for that press.
        h.event(egui::Event::WindowFocused(false));
        h.run();
        h.event(egui::Event::WindowFocused(true));
        h.run();
        let other = h.get_by_label("Other").rect().center();
        click_at(&mut h, other);
        let down = h.ctx.input(|i| i.pointer.primary_down());
        assert_eq!((h.state().other, down), (1, false), "next click is a click, button not stuck");
    }

    /// An input method's text closes the menu and is the menu's, but what it sends in later frames is the field's.
    #[test]
    fn an_input_method_s_text_after_the_menu_closed_is_typed() {
        let mut h = Harness::new_ui_state(app, S { text: "abc".into(), ..Default::default() });
        open(&mut h);
        h.event(egui::Event::Ime(egui::ImeEvent::Commit("x".into())));
        h.run();
        assert!(h.query_by_label_contains("Paste").is_none(), "closed");
        // The user's next IME-typed letter, a later frame, no Key event (an IME eats keys).
        h.event(egui::Event::Ime(egui::ImeEvent::Commit("y".into())));
        h.run();
        h.event(egui::Event::Ime(egui::ImeEvent::Commit("z".into())));
        h.run();
        assert_eq!(
            h.state().text,
            "abcyz",
            "the IME letters after the closing one reach the field"
        );
    }

    /// The window losing the keys closes the menu: the first key typed when they come back is the field's.
    #[test]
    fn the_window_losing_the_keys_closes_the_menu() {
        let mut h = Harness::new_ui_state(app, S { text: "abc".into(), ..Default::default() });
        open(&mut h);
        h.event(egui::Event::WindowFocused(false));
        h.run();
        let open_after = h.query_by_label_contains("Paste").is_some();
        h.event(egui::Event::WindowFocused(true));
        h.run();
        h.event(egui::Event::Key {
            key: egui::Key::Q,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        });
        h.event(egui::Event::Text("q".into()));
        h.run();
        assert_eq!((open_after, h.state().text.as_str()), (false, "abcq"));
    }
}

/// A field's menu whose plugin submenu lies outside it (from the second review's tests).
#[cfg(test)]
mod text_menu_submenu_tests {
    use egui::accesskit::Role;
    use egui_kittest::Harness;
    use egui_kittest::kittest::Queryable;

    #[derive(Default)]
    struct S {
        a: String,
        b: String,
        plug: bool,
        deep: bool,
        focus_b: bool,
    }

    fn app(ui: &mut egui::Ui, s: &mut S) {
        let id = egui::Id::new("field");
        let idb = egui::Id::new("fieldb");
        super::text_menus_begin(ui.ctx());
        let wanted = super::text_menu_wanted(ui.ctx(), id);
        ui.add(egui::TextEdit::multiline(&mut s.a).id(id).desired_width(600.0));
        ui.add(egui::TextEdit::singleline(&mut s.b).id(idb));
        if std::mem::take(&mut s.focus_b) {
            ui.memory_mut(|m| m.request_focus(idb));
        }
        super::own_text_menu(ui.ctx(), id);
        if wanted {
            let (_, r) = super::text_menu(ui.ctx(), id, None, |ui, _rows| {
                let plug = ui.button("Plug").clicked();
                let deep = mg_uikit::menu_button(ui, "Sub", |ui| {
                    let a = ui.button("Deep").clicked();
                    let b = ui.button("Deeper").clicked();
                    a || b
                });
                (plug, deep.inner.unwrap_or(false))
            });
            if let Some((p, d)) = r {
                s.plug |= p;
                s.deep |= d;
            }
        }
        super::text_menus_end(ui.ctx());
    }

    fn btn(at: egui::Pos2, b: egui::PointerButton, pressed: bool) -> egui::Event {
        egui::Event::PointerButton { pos: at, button: b, pressed, modifiers: egui::Modifiers::NONE }
    }

    fn harness() -> Harness<'static, S> {
        let mut h = Harness::builder()
            .with_size(egui::vec2(1400.0, 900.0))
            .build_ui_state(app, S { a: "abc\ndef".into(), b: "bee".into(), ..Default::default() });
        h.run();
        let at = h.get_all_by_role(Role::MultilineTextInput).next().unwrap().rect().center();
        h.event(egui::Event::PointerMoved(at));
        h.event(btn(at, egui::PointerButton::Primary, true));
        h.event(btn(at, egui::PointerButton::Primary, false));
        h.run();
        h.event(btn(at, egui::PointerButton::Secondary, true));
        h.event(btn(at, egui::PointerButton::Secondary, false));
        h.run();
        assert!(h.query_by_label_contains("Paste").is_some(), "menu open");
        h
    }

    /// A plugin's submenu row lying outside the menu's own rectangle (below it), pressed and released in frames of their own, is clicked: the press is on the menu, not outside it.
    #[test]
    fn a_submenu_row_outside_the_menu_is_clicked() {
        let mut h = harness();
        let paste = h.get_by_label_contains("Paste").rect();
        let sub = h.get_by_label_contains("Sub").rect().center();
        h.hover_at(sub);
        for _ in 0..5 {
            h.step();
        }
        let Some(at) = h.query_by_label("Deep").map(|n| n.rect().center()) else {
            panic!("submenu never shown")
        };
        let outside = at.x > paste.right();
        let at = h.get_by_label("Deeper").rect().center();
        let mr = super::text_menus(&h.ctx).menu_rect;
        let info = format!("deeper={at:?} menu={mr:?}");
        h.hover_at(at);
        h.step();
        h.event(btn(at, egui::PointerButton::Primary, true));
        h.run();
        h.event(btn(at, egui::PointerButton::Primary, false));
        h.run();
        assert!(h.state().deep, "second submenu row clicked: {info} {outside}");
    }

    /// A plugin's row of the menu, pressed and released in frames of their own, is clicked.
    #[test]
    fn a_plugin_row_pressed_and_released_apart_is_clicked() {
        let mut h = harness();
        let at = h.get_by_label("Plug").rect().center();
        h.hover_at(at);
        h.step();
        h.event(btn(at, egui::PointerButton::Primary, true));
        h.run();
        h.event(btn(at, egui::PointerButton::Primary, false));
        h.run();
        assert!(h.state().plug, "top plugin row clicked");
    }

    /// The keys moved to another field by the program close the menu: what is typed is that field's, and a click where a row was is no row's.
    #[test]
    fn the_keys_moved_to_another_field_close_the_menu() {
        let mut h = harness();
        let paste = h.get_by_label_contains("Paste").rect().center();
        h.state_mut().focus_b = true;
        h.run();
        let open = h.query_by_label_contains("Paste").is_some();
        h.event(egui::Event::Key {
            key: egui::Key::Q,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        });
        h.event(egui::Event::Text("q".into()));
        h.run();
        let b = h.state().b.clone();
        // a click where the old menu row was
        h.event(egui::Event::PointerMoved(paste));
        h.event(btn(paste, egui::PointerButton::Primary, true));
        h.event(btn(paste, egui::PointerButton::Primary, false));
        h.run();
        let m = super::text_menus(&h.ctx);
        assert_eq!(
            (open, b.as_str(), m.open.is_some(), m.swallow.is_some()),
            (false, "beeq", false, false)
        );
    }
}
