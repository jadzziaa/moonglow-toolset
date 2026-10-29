//! The Advanced Search tab (Edit › Advanced Search…): the module's
//! blueprints, placed objects, areas and the rest found by what they hold
//! (`mg_module::search`), by one condition or several together, listed by
//! place as Find References lists them; a click opens the object. What is
//! found (all of it, or what is ticked) can be changed in one step (a
//! variable set or removed, an event's script, a field: `mg_edit::found`),
//! edited together, or its areas chosen in Edit Areas Together.

use std::collections::BTreeMap;

use egui::Ui;
use mg_core::{Codepage, ResRef, ResType};
use mg_edit::found::{self, Change};
use mg_edit::{Command, GffPath};
use mg_module::search::{
    self, AppearanceOf, FieldValue, Hit, Object, PropertyParts, Query, Search, VarType,
};
use mg_resman::ResKey;
use mg_rules::{Choice, ChoiceColumns, GameData, ObjectKind};

use crate::{Action, Moonglow, Tab};

/// What the window looks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Variable,
    ItemProperty,
    Class,
    Feat,
    Spell,
    Skill,
    Appearance,
    Script,
    Field,
}

impl Kind {
    pub const ALL: [Kind; 9] = [
        Kind::Variable,
        Kind::ItemProperty,
        Kind::Class,
        Kind::Feat,
        Kind::Spell,
        Kind::Skill,
        Kind::Appearance,
        Kind::Script,
        Kind::Field,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Kind::Variable => "Variables",
            Kind::ItemProperty => "Item properties",
            Kind::Class => "Creatures by class",
            Kind::Feat => "Creatures by feat",
            Kind::Spell => "Creatures by spell",
            Kind::Skill => "Creatures by skill",
            Kind::Appearance => "Appearances",
            Kind::Script => "Scripts attached",
            Kind::Field => "Any field",
        }
    }
}

/// What else a search may be kept to, beside the placed objects' kinds.
const OTHERS: [(ResType, &str); 3] =
    [(ResType::ARE, "Areas"), (ResType::DLG, "Conversations"), (ResType::IFO, "The module")];

/// The kinds of object a search may be kept to: the blueprints' types,
/// then areas, conversations and the module.
fn objects() -> impl Iterator<Item = (ResType, &'static str)> {
    ObjectKind::ALL.into_iter().map(|k| (k.restype(), k.label())).chain(OTHERS)
}

/// What a search kept to `object` is called (`None`: everything).
fn object_label(object: Option<ResType>) -> &'static str {
    objects().find(|(t, _)| Some(*t) == object).map_or("Everything", |o| o.1)
}

/// The highest class level a condition's range reaches.
const TOP_LEVEL: u8 = 60;

/// One condition: what is looked for.
#[derive(Debug, Clone, PartialEq)]
pub struct Condition {
    pub kind: Kind,
    /// A variable's or script's name, a field's label.
    pub name: String,
    pub var_type: Option<VarType>,
    /// A variable's value, a field's value.
    pub value: String,
    /// A field's value is a number or a range of them (else text).
    pub number: bool,
    /// The row looked for: a class, feat, spell, skill, appearance, or an
    /// item property's type.
    pub row: Option<usize>,
    pub subtype: Option<usize>,
    pub cost: Option<usize>,
    pub param: Option<usize>,
    pub min_rank: u8,
    /// A class's levels, from and to.
    pub levels: (u8, u8),
    pub of: AppearanceOf,
    /// The item property type's parts, for the type they are of.
    pub parts: Option<((usize, Option<usize>), PropertyParts)>,
}

impl Default for Condition {
    fn default() -> Condition {
        Condition {
            kind: Kind::Variable,
            name: String::new(),
            var_type: None,
            value: String::new(),
            number: false,
            row: None,
            subtype: None,
            cost: None,
            param: None,
            min_rank: 1,
            levels: (1, TOP_LEVEL),
            of: AppearanceOf::Creature,
            parts: None,
        }
    }
}

impl Condition {
    /// The search it asks, or what it still needs.
    pub fn search(&self) -> Result<Search, String> {
        let name = self.name.trim();
        let value = self.value.trim();
        let row = |what: &str| self.row.ok_or_else(|| format!("Choose {what}"));
        Ok(match self.kind {
            Kind::Variable => {
                if name.is_empty() {
                    return Err("Type a variable's name (* for any characters)".into());
                }
                Search::Variable {
                    name: name.to_string(),
                    kind: self.var_type,
                    value: (!value.is_empty()).then(|| value.to_string()),
                }
            }
            Kind::ItemProperty => Search::ItemProperty {
                property: row("a property")? as u16,
                subtype: self.subtype.map(|s| s as u16),
                value: self.cost.map(|s| s as u16),
                param: self.param.map(|s| s as u8),
            },
            Kind::Class if self.levels == (1, TOP_LEVEL) => Search::Class(row("a class")? as i64),
            Kind::Class => Search::ClassLevel {
                class: row("a class")? as i64,
                min: self.levels.0.into(),
                max: self.levels.1.into(),
            },
            Kind::Feat => Search::Feat(row("a feat")? as i64),
            Kind::Spell => Search::Spell(row("a spell")? as i64),
            Kind::Skill => Search::Skill { skill: row("a skill")?, min_rank: self.min_rank },
            Kind::Appearance => {
                Search::Appearance { of: self.of, row: row("an appearance")? as i64 }
            }
            Kind::Script => {
                if name.is_empty() {
                    return Err("Type a script's name (* for any characters)".into());
                }
                Search::Script(name.to_string())
            }
            Kind::Field => {
                let value = match (value.is_empty(), self.number) {
                    (true, _) => None,
                    (false, true) => Some(FieldValue::number(value)?),
                    (false, false) => Some(FieldValue::Text(value.to_string())),
                };
                if name.is_empty() && value.is_none() {
                    return Err("Type a field's label, a value, or both".into());
                }
                Search::Field { label: (!name.is_empty()).then(|| name.to_string()), value }
            }
        })
    }

    /// The table the row is chosen from.
    fn table(&self) -> Option<mg_rules::rows::RowTable> {
        let search = match self.kind {
            Kind::ItemProperty => {
                Search::ItemProperty { property: 0, subtype: None, value: None, param: None }
            }
            Kind::Class => Search::Class(0),
            Kind::Feat => Search::Feat(0),
            Kind::Spell => Search::Spell(0),
            Kind::Skill => Search::Skill { skill: 0, min_rank: 1 },
            Kind::Appearance => Search::Appearance { of: self.of, row: 0 },
            _ => return None,
        };
        search.table()
    }

    /// The field a change of what it finds sets: the label looked for,
    /// an appearance's field.
    fn field(&self) -> Option<String> {
        match self.kind {
            Kind::Field if !self.name.contains('*') && !self.name.trim().is_empty() => {
                Some(self.name.trim().to_string())
            }
            Kind::Appearance => Some(self.of.field().1.to_string()),
            _ => None,
        }
    }
}

/// What is done to what is found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Act {
    SetVariable,
    RemoveVariable,
    SetScript,
    SetField,
}

impl Act {
    pub const ALL: [Act; 4] =
        [Act::SetVariable, Act::RemoveVariable, Act::SetScript, Act::SetField];

    pub fn label(self) -> &'static str {
        match self {
            Act::SetVariable => "Set a variable",
            Act::RemoveVariable => "Remove a variable",
            Act::SetScript => "Set an event's script",
            Act::SetField => "Set a field",
        }
    }
}

/// The change to make to what is found.
#[derive(Debug, Clone, PartialEq)]
pub struct ActDraft {
    pub act: Act,
    /// A variable's name, a field's label.
    pub name: String,
    pub var_type: VarType,
    /// A variable's or field's value, a script's name.
    pub value: String,
    /// The event (its field's label).
    pub event: Option<String>,
    /// What the last change did.
    pub said: Option<String>,
}

impl Default for ActDraft {
    fn default() -> ActDraft {
        ActDraft {
            act: Act::SetVariable,
            name: String::new(),
            var_type: VarType::Int,
            value: String::new(),
            event: None,
            said: None,
        }
    }
}

impl ActDraft {
    /// The change asked, or what it still needs.
    pub fn change(&self) -> Result<Change, String> {
        let name = self.name.trim().to_string();
        let needs_name = |what: &str| {
            if name.is_empty() { Err(format!("Type the {what}")) } else { Ok(name.clone()) }
        };
        Ok(match self.act {
            Act::SetVariable => Change::SetVariable {
                name: needs_name("variable's name")?,
                kind: self.var_type,
                value: self.value.clone(),
            },
            Act::RemoveVariable => Change::RemoveVariable { name: needs_name("variable's name")? },
            Act::SetScript => Change::SetScript {
                event: self.event.clone().ok_or("Choose an event")?,
                script: self.value.trim().to_string(),
            },
            Act::SetField => {
                Change::SetField { label: needs_name("field's label")?, value: self.value.clone() }
            }
        })
    }
}

/// The Advanced Search window's state.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AdvancedSearch {
    /// What is looked for.
    pub first: Condition,
    /// And what the same objects must also hold.
    pub and: Vec<Condition>,
    /// The kind of object looked at; none: all.
    pub object: Option<ResType>,
    /// Only placed objects, only blueprints.
    pub placed: Option<bool>,
    /// What was found, for the query in `searched`.
    pub hits: Vec<Hit>,
    pub searched: Option<Query>,
    /// Which of the hits are ticked (to change, to edit together).
    pub ticked: Vec<bool>,
    /// The change to make to them.
    pub act: ActDraft,
    /// The events the objects found have (script fields' labels).
    pub events: Vec<String>,
}

impl AdvancedSearch {
    /// The query the window asks, or what it still needs.
    pub fn query(&self) -> Result<Query, String> {
        let and = self.and.iter().map(Condition::search).collect::<Result<_, _>>()?;
        Ok(Query {
            search: self.first.search()?,
            and,
            types: self.object.into_iter().collect(),
            placed: self.placed,
        })
    }

    /// The hits ticked.
    pub fn chosen(&self) -> Vec<&Hit> {
        self.hits.iter().zip(&self.ticked).filter(|(_, t)| **t).map(|(h, _)| h).collect()
    }
}

impl Moonglow {
    /// Finds what the window asks in the module (its unsaved edits
    /// included); everything found is ticked.
    pub fn advanced_search(&mut self, draft: &mut AdvancedSearch) {
        let Ok(q) = draft.query() else { return };
        let Some(ws) = self.ws.as_mut() else { return };
        if let Err(e) = ws.flush() {
            self.log.error(e.to_string());
            return;
        }
        let game = self.game.as_deref();
        let cp = game.map_or(Codepage::WINDOWS_1252, GameData::codepage);
        // (Asked again, only what changed since is read.)
        draft.hits = self.search_kept.search(&ws.module, &q, game, cp);
        draft.ticked = vec![true; draft.hits.len()];
        draft.events = found::events(&ws.module, &draft.hits.iter().collect::<Vec<_>>());
        draft.searched = Some(q);
        // (A change's field follows what is looked for.)
        if let Some(field) = draft.first.field() {
            draft.act.name = field;
        }
    }

    /// Makes the window's change to what it found and ticked, in one
    /// step, then finds again.
    pub fn change_found(&mut self, draft: &mut AdvancedSearch) {
        let change = match draft.act.change() {
            Ok(c) => c,
            Err(e) => {
                draft.act.said = Some(e);
                return;
            }
        };
        let Some(ws) = self.ws.as_mut() else { return };
        if let Err(e) = ws.flush() {
            self.log.error(e.to_string());
            return;
        }
        let cp = self.game.as_deref().map_or(Codepage::WINDOWS_1252, GameData::codepage);
        let out = match found::edits(&ws.module, &draft.chosen(), &change, cp) {
            Ok(out) => out,
            Err(e) => {
                draft.act.said = Some(e);
                return;
            }
        };
        let left = match out.skipped.len() {
            0 => String::new(),
            n => format!("; {n} left as they were: {}", out.skipped.join("; ")),
        };
        if out.edits.is_empty() {
            draft.act.said = Some(format!("Nothing to change{left}"));
            return;
        }
        let label = format!("{} on {} found", change.describe(), out.changed);
        if let Err(e) = self.apply(Command::new(label.clone(), out.edits)) {
            draft.act.said = Some(e.to_string());
            return;
        }
        draft.act.said = Some(format!("{label}{left}"));
        self.advanced_search(draft);
    }
}

/// A row of a table, chosen by name (a long list has a filter); `any`
/// names the choice of none. With no names (no game data), a number.
fn row_picker(
    ui: &mut Ui,
    id: &str,
    choices: &[Choice],
    row: &mut Option<usize>,
    any: Option<&str>,
) {
    if choices.is_empty() {
        let mut on = row.is_some() || any.is_none();
        if any.is_some() {
            ui.checkbox(&mut on, "");
        }
        let mut n = row.unwrap_or(0);
        ui.add_enabled(on, egui::DragValue::new(&mut n).range(0..=65535))
            .on_hover_text("The row's number (the game's data isn't loaded to name it)");
        *row = on.then_some(n);
        return;
    }
    let mut number = |ui: &mut Ui, row: usize| {
        ui.label(format!("Row {row}"));
    };
    let current = row.map(|r| r as i64);
    let list = crate::widgets::row_choice(egui::Id::new(id), choices, current)
        .width(260.0)
        .about(&mut number);
    let list = match any {
        Some(any) => list.none(any),
        None => list.unset("Choose…"),
    };
    if let Some(picked) = list.show_optional(ui) {
        *row = picked.and_then(|r| usize::try_from(r).ok());
    }
}

/// The choices of a table, as the game names them.
fn choices(game: Option<&GameData>, table: &mg_rules::rows::RowTable) -> Vec<Choice> {
    let Some(game) = game else { return Vec::new() };
    let columns = ChoiceColumns { name: table.name, label: table.label };
    game.choice_list(table.table, columns).map(|c| c.to_vec()).unwrap_or_default()
}

/// A condition's rows of the window's grid; `salt` keeps its widgets'
/// ids its own (empty for the first). True when Enter asks to find.
fn condition_ui(ui: &mut Ui, c: &mut Condition, game: Option<&GameData>, salt: &str) -> bool {
    let mut find = false;
    let text = |ui: &mut Ui, value: &mut String, hint: &str| -> bool {
        let r = ui.add(egui::TextEdit::singleline(value).hint_text(hint).desired_width(260.0));
        r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))
    };
    let id = |name: &str| format!("advanced-search-{name}{salt}");
    match c.kind {
        Kind::Variable => {
            crate::widgets::field_label(ui, "Name");
            find |= text(ui, &mut c.name, "e.g. *sDeidad* (* for any characters)");
            ui.end_row();
            crate::widgets::field_label(ui, "Type");
            let types: Vec<Option<VarType>> =
                std::iter::once(None).chain(VarType::ALL.map(Some)).collect();
            crate::widgets::dropdown(id("var-type"))
                .selected_text(c.var_type.map_or("Any", VarType::name))
                .show(ui, &mut c.var_type, &types, |t| t.map_or("Any", VarType::name));
            ui.end_row();
            crate::widgets::field_label(ui, "Value");
            find |= text(ui, &mut c.value, "any (* for any characters)");
            ui.end_row();
        }
        Kind::ItemProperty => {
            let table = c.table().expect("a table");
            crate::widgets::field_label(ui, "Property");
            let was = c.row;
            row_picker(ui, &id("property"), &choices(game, &table), &mut c.row, None);
            if c.row != was {
                (c.subtype, c.cost, c.param) = (None, None, None);
            }
            ui.end_row();
            let parts = match (game, c.row) {
                (Some(g), Some(p)) => {
                    let key = (p, c.subtype);
                    if c.parts.as_ref().is_none_or(|(k, _)| *k != key) {
                        let parts =
                            search::property_parts(g, p as u16, c.subtype.map(|s| s as u16));
                        c.parts = Some((key, parts));
                    }
                    c.parts.as_ref().map(|(_, p)| p.clone()).unwrap_or_default()
                }
                _ => PropertyParts::default(),
            };
            // (Without the game's data, every part by number.)
            let all = game.is_none();
            for (label, choices, row) in [
                ("Subtype", &parts.subtypes, &mut c.subtype),
                ("Value", &parts.values, &mut c.cost),
                ("Parameter", &parts.params, &mut c.param),
            ] {
                if all || !choices.is_empty() {
                    crate::widgets::field_label(ui, label);
                    row_picker(ui, &id(label), choices, row, Some("Any"));
                    ui.end_row();
                }
            }
        }
        Kind::Class | Kind::Feat | Kind::Spell | Kind::Skill | Kind::Appearance => {
            if c.kind == Kind::Appearance {
                crate::widgets::field_label(ui, "Of");
                let was = c.of;
                crate::widgets::dropdown(id("of")).selected_text(c.of.label()).width(260.0).show(
                    ui,
                    &mut c.of,
                    &AppearanceOf::ALL,
                    |a| a.label(),
                );
                if c.of != was {
                    c.row = None;
                }
                ui.end_row();
            }
            let label = match c.kind {
                Kind::Class => "Class",
                Kind::Feat => "Feat",
                Kind::Spell => "Spell",
                Kind::Skill => "Skill",
                _ => "Appearance",
            };
            crate::widgets::field_label(ui, label);
            let table = c.table().expect("a table");
            row_picker(ui, &id("row"), &choices(game, &table), &mut c.row, None);
            ui.end_row();
            if c.kind == Kind::Skill {
                crate::widgets::field_label(ui, "At least");
                ui.add(egui::DragValue::new(&mut c.min_rank).range(1..=127).suffix(" rank(s)"));
                ui.end_row();
            }
            if c.kind == Kind::Class {
                crate::widgets::field_label(ui, "Levels");
                ui.horizontal(|ui| {
                    let (from, to) = &mut c.levels;
                    ui.add(egui::DragValue::new(from).range(1..=TOP_LEVEL))
                        .on_hover_text("The fewest levels in the class");
                    ui.label("to");
                    ui.add(egui::DragValue::new(to).range(1..=TOP_LEVEL))
                        .on_hover_text("The most levels in the class");
                    *to = (*to).max(*from);
                });
                ui.end_row();
            }
        }
        Kind::Script => {
            crate::widgets::field_label(ui, "Script");
            find |= text(ui, &mut c.name, "e.g. x2_def_* (* for any characters)");
            ui.end_row();
        }
        Kind::Field => {
            crate::widgets::field_label(ui, "Label");
            find |= text(ui, &mut c.name, "any (* for any characters)");
            ui.end_row();
            crate::widgets::field_label(ui, "Value");
            ui.horizontal(|ui| {
                let hint = if c.number { "e.g. 103, 1.5 or 5..10" } else { "text it contains" };
                find |= text(ui, &mut c.value, hint);
                ui.checkbox(&mut c.number, "Number").on_hover_text(
                    "Number fields holding this number (103, 1.5) or one in a range (5..10, \
                     5.., ..10)",
                );
            });
            ui.end_row();
        }
    }
    find
}

/// A condition's kind chosen; the rows chosen for the last one forgotten.
fn kind_choice(ui: &mut Ui, c: &mut Condition, id: &str) {
    let was = c.kind;
    crate::widgets::dropdown(id).selected_text(c.kind.label()).width(260.0).show(
        ui,
        &mut c.kind,
        &Kind::ALL,
        |k| k.label(),
    );
    if c.kind != was {
        (c.row, c.subtype, c.cost, c.param) = (None, None, None, None);
    }
}

/// What to look for, by kind, and the conditions with it.
fn search_ui(ui: &mut Ui, d: &mut AdvancedSearch, game: Option<&GameData>) -> bool {
    let mut find = false;
    let mut remove = None;
    egui::Grid::new("advanced-search").num_columns(2).show(ui, |ui| {
        crate::widgets::field_label(ui, "Look for");
        kind_choice(ui, &mut d.first, "advanced-search-kind");
        ui.end_row();
        find |= condition_ui(ui, &mut d.first, game, "");
        for (i, c) in d.and.iter_mut().enumerate() {
            crate::widgets::field_label(ui, "And").on_hover_text(
                "Objects that hold this too (or hold, or are held by, one that does)",
            );
            ui.horizontal(|ui| {
                kind_choice(ui, c, &format!("advanced-search-kind-{i}"));
                if ui.small_button("Remove").clicked() {
                    remove = Some(i);
                }
            });
            ui.end_row();
            find |= condition_ui(ui, c, game, &format!("-{i}"));
        }
        ui.label("");
        if ui
            .button("And…")
            .on_hover_text("Another condition the objects found must meet")
            .clicked()
        {
            d.and.push(Condition { kind: Kind::Feat, ..Default::default() });
        }
        ui.end_row();
        crate::widgets::field_label(ui, "Look at");
        ui.horizontal(|ui| {
            let objects: Vec<_> =
                std::iter::once(None).chain(objects().map(|o| Some(o.0))).collect();
            crate::widgets::dropdown("advanced-search-object")
                .selected_text(object_label(d.object))
                .show(ui, &mut d.object, &objects, |t| object_label(*t))
                .on_hover_text("Items include those in inventories, equipment and stores");
            let placed = match d.placed {
                None => "blueprints and placed objects",
                Some(true) => "placed objects only",
                Some(false) => "blueprints only",
            };
            crate::widgets::dropdown("advanced-search-placed").selected_text(placed).show(
                ui,
                &mut d.placed,
                &[None, Some(false), Some(true)],
                |p| match p {
                    None => "blueprints and placed objects",
                    Some(true) => "placed objects only",
                    Some(false) => "blueprints only",
                },
            );
        });
        ui.end_row();
    });
    if let Some(i) = remove {
        d.and.remove(i);
    }
    find
}

/// What can be opened together from what is ticked: blueprints of one
/// type, objects of one kind placed in one area.
#[derive(Debug, Clone, PartialEq)]
enum Together {
    Blueprints(ResType, Vec<ResKey>),
    Placed { area: ResRef, kind: ObjectKind, paths: Vec<GffPath> },
}

impl Together {
    fn tab(&self) -> Tab {
        match self {
            Together::Blueprints(_, keys) if keys.len() == 1 => Tab::Blueprint(keys[0]),
            Together::Blueprints(_, keys) => Tab::Blueprints(keys.clone()),
            Together::Placed { area, paths, .. } if paths.len() == 1 => {
                Tab::Instance { area: *area, path: paths[0].clone() }
            }
            Together::Placed { area, paths, .. } => {
                Tab::Instances { area: *area, paths: paths.clone() }
            }
        }
    }
}

/// The groups of what is ticked that open together, and the areas it is
/// in (or is).
fn together(hits: &[&Hit]) -> (Vec<Together>, Vec<ResRef>) {
    let mut blueprints: BTreeMap<ResType, Vec<ResKey>> = BTreeMap::new();
    let mut placed: BTreeMap<(ResRef, &'static str), Vec<GffPath>> = BTreeMap::new();
    let mut areas: Vec<ResRef> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for h in hits {
        if matches!(h.key.restype, ResType::ARE | ResType::GIT) && !areas.contains(&h.key.resref) {
            areas.push(h.key.resref);
        }
        let Some(o): Option<Object> = h.object() else { continue };
        if !seen.insert(o.clone()) {
            continue;
        }
        let steps: Vec<&str> = o.path.trim_start_matches('/').split('/').collect();
        if o.path == "/" && ObjectKind::from_restype(o.key.restype).is_some() {
            blueprints.entry(o.key.restype).or_default().push(o.key);
        } else if o.placed()
            && let [one] = steps[..]
            && let Some((list, i)) = one.strip_suffix(']').and_then(|s| s.rsplit_once('['))
            && let (Some(kind), Ok(i)) = (ObjectKind::from_list(list), i.parse::<usize>())
        {
            placed
                .entry((o.key.resref, kind.list()))
                .or_default()
                .push(GffPath::root().item(list, i));
        }
    }
    let mut out: Vec<Together> =
        blueprints.into_iter().map(|(t, keys)| Together::Blueprints(t, keys)).collect();
    out.extend(placed.into_iter().map(|((area, list), paths)| Together::Placed {
        area,
        kind: ObjectKind::from_list(list).expect("a placed object's list"),
        paths,
    }));
    (out, areas)
}

/// The change to make, and its button; true when it is asked.
fn act_ui(ui: &mut Ui, d: &mut AdvancedSearch, n: usize) -> bool {
    let mut go = false;
    let a = &mut d.act;
    egui::Grid::new("advanced-search-act").num_columns(2).show(ui, |ui| {
        crate::widgets::field_label(ui, "Change");
        crate::widgets::dropdown("advanced-search-act")
            .selected_text(a.act.label())
            .width(260.0)
            .show(ui, &mut a.act, &Act::ALL, |a| a.label());
        ui.end_row();
        let edit = |ui: &mut Ui, v: &mut String, hint: &str| {
            ui.add(egui::TextEdit::singleline(v).hint_text(hint).desired_width(260.0));
        };
        match a.act {
            Act::SetVariable | Act::RemoveVariable => {
                crate::widgets::field_label(ui, "Variable");
                edit(ui, &mut a.name, "its name, e.g. nQuestState");
                ui.end_row();
            }
            Act::SetScript => {
                crate::widgets::field_label(ui, "Event");
                let events: Vec<Option<String>> = d.events.iter().cloned().map(Some).collect();
                let shown =
                    |e: &Option<String>| e.as_deref().map_or("Choose…".into(), search::event);
                crate::widgets::dropdown("advanced-search-event")
                    .selected_text(shown(&a.event))
                    .width(260.0)
                    .show(ui, &mut a.event, &events, |e| shown(e));
                ui.end_row();
                crate::widgets::field_label(ui, "Script");
                edit(ui, &mut a.value, "its name (empty: none)");
                ui.end_row();
            }
            Act::SetField => {
                crate::widgets::field_label(ui, "Field");
                edit(ui, &mut a.name, "its label, e.g. Appearance_Type");
                ui.end_row();
            }
        }
        if a.act == Act::SetVariable {
            crate::widgets::field_label(ui, "Type");
            let types = [VarType::Int, VarType::Float, VarType::String];
            crate::widgets::dropdown("advanced-search-act-type")
                .selected_text(a.var_type.name())
                .show(ui, &mut a.var_type, &types, |t| t.name());
            ui.end_row();
        }
        if matches!(a.act, Act::SetVariable | Act::SetField) {
            crate::widgets::field_label(ui, "Value");
            edit(ui, &mut a.value, "the value");
            ui.end_row();
        }
    });
    let ok = a.change();
    ui.horizontal(|ui| {
        let button = egui::Button::new(format!("Change {n} Found"));
        go = ui
            .add_enabled(ok.is_ok() && n > 0, button)
            .on_hover_text("In one step, undone as one")
            .clicked();
        if let Err(why) = &ok {
            ui.weak(why);
        }
    });
    if let Some(said) = &a.said {
        ui.weak(said);
    }
    go
}

/// The Advanced Search tab.
pub(crate) fn ui(app: &mut Moonglow, ui: &mut Ui) {
    if app.ws.is_none() {
        ui.label("Open a module first.");
        return;
    }
    let mut draft = std::mem::take(&mut app.advanced_search);
    let (mut find, mut go, mut change) = (false, None, false);
    // The places, with areas named as the rest of the window names them
    // (Options › General: Show areas by name).
    let places: Vec<String> = draft
        .hits
        .iter()
        .map(|h| {
            let of_area = matches!(h.key.restype, ResType::ARE | ResType::GIT);
            if !of_area || !app.settings.area_names {
                return h.place.clone();
            }
            let name = crate::tabs::area_label(app, h.key.resref);
            h.place.replacen(&h.key.resref.to_lowercase().to_string(), &name, 1)
        })
        .collect();
    let game = app.game.clone();
    ui.weak(
        "The module's blueprints, placed objects (and the items they hold), areas and \
         conversations, by what they hold.",
    );
    if game.is_none() {
        ui.weak("The game's data isn't loaded: choose rows by number.");
    }
    find |= search_ui(ui, &mut draft, game.as_deref());
    let query = draft.query();
    ui.horizontal(|ui| {
        find |= ui.add_enabled(query.is_ok(), egui::Button::new("Find")).clicked();
        if let Err(why) = &query {
            ui.weak(why);
        }
    });
    let current = query.as_ref().ok() == draft.searched.as_ref();
    if draft.searched.is_some() && !current {
        ui.weak("Find again for what's asked now.");
    }
    let mut open = Vec::new();
    let mut choose_areas = None;
    if draft.searched.is_some() {
        ui.separator();
        draft.ticked.resize(draft.hits.len(), true);
        if draft.hits.is_empty() {
            ui.weak("Nothing in the module holds it.");
        } else {
            let chosen = draft.chosen();
            let n = chosen.len();
            let (groups, areas) = together(&chosen);
            // (Areas named as the list names them.)
            let mut area_name = |area: ResRef| match app.settings.area_names {
                true => crate::tabs::area_label(app, area),
                false => area.to_lowercase().to_string(),
            };
            let labels: Vec<String> = groups
                .iter()
                .map(|g| match g {
                    Together::Blueprints(t, keys) => {
                        let kind = ObjectKind::from_restype(*t).map_or("", ObjectKind::name);
                        format!("Edit {} {kind} blueprint(s) Together", keys.len())
                    }
                    Together::Placed { area, kind, paths } => format!(
                        "Edit {} {}(s) in {} Together",
                        paths.len(),
                        kind.name(),
                        area_name(*area)
                    ),
                })
                .collect();
            ui.horizontal(|ui| {
                ui.label(format!("{} found:", draft.hits.len()));
                if ui.small_button("Tick All").clicked() {
                    draft.ticked.fill(true);
                }
                if ui.small_button("Tick None").clicked() {
                    draft.ticked.fill(false);
                }
            });
            egui::CollapsingHeader::new(format!("Change the {n} ticked"))
                .id_salt("advanced-search-change")
                .show(ui, |ui| change = act_ui(ui, &mut draft, n));
            egui::CollapsingHeader::new(format!("Edit the {n} ticked together"))
                .id_salt("advanced-search-together")
                .show(ui, |ui| {
                    if groups.is_empty() && areas.is_empty() {
                        ui.weak("Blueprints of one type, objects of one kind placed in one area");
                    }
                    ui.horizontal_wrapped(|ui| {
                        for (g, text) in groups.iter().zip(labels) {
                            if ui.button(text).clicked() {
                                open.push(g.tab());
                            }
                        }
                        if !areas.is_empty()
                            && ui
                                .button(format!("Choose {} Area(s)…", areas.len()))
                                .on_hover_text(
                                    "Edit Areas Together, with the areas of what is ticked \
                                     ticked",
                                )
                                .clicked()
                        {
                            choose_areas = Some(areas.clone());
                        }
                    });
                });
        }
        egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
            for (i, h) in draft.hits.iter().enumerate() {
                ui.horizontal(|ui| {
                    ui.checkbox(&mut draft.ticked[i], "");
                    let hover = format!("{} {}", h.key, h.path);
                    if ui.link(&places[i]).on_hover_text(hover).clicked() {
                        go = Some(i);
                    }
                    ui.weak(&h.matched);
                });
            }
        });
    }
    if find && draft.query().is_ok() {
        draft.act.said = None;
        app.advanced_search(&mut draft);
    }
    if change {
        app.change_found(&mut draft);
    }
    if let Some(i) = go {
        let usage = draft.hits[i].usage();
        app.go_to_usage(&usage);
    }
    for tab in open {
        app.actions.push(Action::OpenTab(tab));
    }
    if let Some(areas) = choose_areas {
        app.area_chooser = Some(crate::area_props::AreaChooser {
            chosen: areas.into_iter().collect(),
            ..Default::default()
        });
    }
    app.advanced_search = draft;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A search is kept to each placed object kind by its type and the
    /// name the toolset calls it by, then to areas, conversations and the
    /// module.
    #[test]
    fn a_search_is_kept_to_each_kind_of_object() {
        let listed: Vec<_> = objects().collect();
        assert_eq!(listed.len(), ObjectKind::ALL.len() + 3);
        for k in ObjectKind::ALL {
            assert_eq!(object_label(Some(k.restype())), k.label());
        }
        assert_eq!(object_label(Some(ResType::UTM)), "Stores");
        assert_eq!(object_label(Some(ResType::IFO)), "The module");
        assert_eq!(object_label(None), "Everything");
    }
}
