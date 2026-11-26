//! The Advanced Search tab (Edit › Advanced Search…): the module's
//! blueprints, placed objects, areas and the rest found by what they hold
//! (`mg_module::search`), by one condition or several together (all of
//! them met, or any; and those ticked Not, not met), listed by
//! place as Find References lists them; a click opens the object. What is
//! found (all of it, or what is ticked) can be changed in one step (a
//! variable set or removed, an event's script, a field: `mg_edit::found`),
//! edited together, or its areas chosen in Edit Areas Together.

use mg_uikit::Controls as _;
use std::collections::BTreeMap;

use egui::Ui;
use mg_core::{Codepage, ResRef, ResType};
use mg_edit::found::{self, Change, Entry};
use mg_edit::{Command, GffPath};
use mg_module::search::{
    self, AppearanceOf, FieldValue, Hit, Object, PropertyParts, Query, Search, VarType,
};
use mg_resman::ResKey;
use mg_rules::{Choice, ChoiceColumns, GameData, ObjectKind};

use crate::row_copy;
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

    /// Its name as a session keeps it.
    fn id(self) -> &'static str {
        match self {
            Kind::Variable => "variable",
            Kind::ItemProperty => "item-property",
            Kind::Class => "class",
            Kind::Feat => "feat",
            Kind::Spell => "spell",
            Kind::Skill => "skill",
            Kind::Appearance => "appearance",
            Kind::Script => "script",
            Kind::Field => "field",
        }
    }

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
    /// A spell's class: the class it is known or memorized as (none: any
    /// class's, or a special ability).
    pub class: Option<usize>,
    pub of: AppearanceOf,
    /// The item property type's parts, for the type they are of.
    pub parts: Option<((usize, Option<usize>), PropertyParts)>,
    /// What is found must not meet it (one of the conditions with the
    /// first; the first's is not asked).
    pub not: bool,
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
            class: None,
            of: AppearanceOf::Creature,
            parts: None,
            not: false,
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
            Kind::Spell => {
                Search::Spell { spell: row("a spell")? as i64, class: self.class.map(|c| c as i64) }
            }
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
            Kind::Spell => Search::Spell { spell: 0, class: None },
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
    AddFeat,
    RemoveFeat,
    AddClass,
    RemoveClass,
    AddProperty,
    RemoveProperty,
    AddItem,
    RemoveItem,
}

impl Act {
    pub const ALL: [Act; 12] = [
        Act::SetVariable,
        Act::RemoveVariable,
        Act::SetScript,
        Act::SetField,
        Act::AddFeat,
        Act::RemoveFeat,
        Act::AddClass,
        Act::RemoveClass,
        Act::AddProperty,
        Act::RemoveProperty,
        Act::AddItem,
        Act::RemoveItem,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Act::SetVariable => "Set a variable",
            Act::RemoveVariable => "Remove a variable",
            Act::SetScript => "Set an event's script",
            Act::SetField => "Set a field",
            Act::AddFeat => "Add a feat",
            Act::RemoveFeat => "Remove a feat",
            Act::AddClass => "Add a class",
            Act::RemoveClass => "Remove a class",
            Act::AddProperty => "Add an item property",
            Act::RemoveProperty => "Remove an item property",
            Act::AddItem => "Add an item",
            Act::RemoveItem => "Remove an item",
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
    /// A field set is added where it is missing (Add where missing; off
    /// unless ticked: `found::Change::SetField`'s `add`).
    pub add: bool,
    /// The event (its field's label).
    pub event: Option<String>,
    /// The entry added or removed: a feat, a class (its level, as
    /// `levels.0`), an item property and its parts, picked as a
    /// condition's are.
    pub entry: Condition,
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
            add: false,
            event: None,
            entry: Condition { levels: (1, 1), ..Condition::default() },
            said: None,
        }
    }
}

impl ActDraft {
    /// The change asked, or what it still needs (the game's data names a
    /// feat, a class and an item property, and makes the property).
    pub fn change(&self, game: Option<&GameData>) -> Result<Change, String> {
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
            Act::SetField => Change::SetField {
                label: needs_name("field's label")?,
                value: self.value.clone(),
                add: self.add,
            },
            Act::AddFeat | Act::RemoveFeat | Act::AddClass | Act::RemoveClass => {
                let class = matches!(self.act, Act::AddClass | Act::RemoveClass);
                let row =
                    self.entry.row.ok_or(if class { "Choose a class" } else { "Choose a feat" })?;
                let mut c = self.entry.clone();
                c.kind = if class { Kind::Class } else { Kind::Feat };
                let named = c.table().map(|t| listed(game, &t)).unwrap_or_default();
                let name = mg_rules::choice_text(&named, row as i64);
                let entry = if class {
                    Entry::Class { class: row as i32, level: i16::from(self.entry.levels.0.max(1)) }
                } else {
                    Entry::Feat(row as u16)
                };
                let add = matches!(self.act, Act::AddFeat | Act::AddClass);
                if add {
                    Change::AddEntry { entry, name }
                } else {
                    Change::RemoveEntry { entry, name }
                }
            }
            Act::AddProperty | Act::RemoveProperty => {
                let game = game.ok_or("Item properties need the game's data")?;
                let row = self.entry.row.ok_or("Choose a property")? as u16;
                let t = game.property_type(row).ok_or("Not a property the game has")?;
                let (subtype, value, param) = (
                    self.entry.subtype.map(|s| s as u16),
                    self.entry.cost.map(|v| v as u16),
                    self.entry.param.map(|v| v as u8),
                );
                // Its first subtype, value and parameter, but those chosen.
                let mut p = game.new_property(&t, 0);
                if let Some(s) = subtype {
                    p.subtype = s;
                    p.param1 = game.property_param_table(&t, s).unwrap_or(255);
                    p.param1_value = match p.param1 {
                        255 => 0,
                        table => game.property_params(table).first().map_or(0, |c| c.row as u8),
                    };
                }
                if let Some(v) = value {
                    p.cost_value = v;
                }
                if let Some(v) = param {
                    p.param1_value = v;
                }
                let entry = Entry::Property { property: p, subtype, value, param };
                if self.act == Act::AddProperty {
                    Change::AddEntry { entry, name: game.property_text(&p) }
                } else {
                    Change::RemoveEntry { entry, name: t.name }
                }
            }
            Act::AddItem | Act::RemoveItem => {
                let name = needs_name("item blueprint's resref")?.to_ascii_lowercase();
                let r = ResRef::from_str(&name).map_err(|e| format!("{name:?}: {e}"))?;
                let entry = Entry::Item(r);
                if self.act == Act::AddItem {
                    Change::AddEntry { entry, name }
                } else {
                    Change::RemoveEntry { entry, name }
                }
            }
        })
    }
}

/// What an appearance is of, as a session keeps it.
const APPEARANCES: [(AppearanceOf, &str); 6] = [
    (AppearanceOf::Creature, "creature"),
    (AppearanceOf::Placeable, "placeable"),
    (AppearanceOf::Door, "door"),
    (AppearanceOf::GenericDoor, "generic-door"),
    (AppearanceOf::Waypoint, "waypoint"),
    (AppearanceOf::Item, "item"),
];

/// A condition as a session keeps it (Options › General's tabs opened
/// again), as text: the kind, what it names and its rows, parts and ranges.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub(crate) struct KeptCondition {
    kind: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    name: String,
    /// A variable's type, as the game numbers it.
    #[serde(skip_serializing_if = "Option::is_none")]
    var_type: Option<i64>,
    #[serde(skip_serializing_if = "String::is_empty")]
    value: String,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    number: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    row: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    subtype: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cost: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    param: Option<usize>,
    min_rank: u8,
    levels: [u8; 2],
    #[serde(skip_serializing_if = "Option::is_none")]
    class: Option<usize>,
    of: String,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    not: bool,
}

/// The Advanced Search window's question, as a session keeps it.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub(crate) struct KeptSearch {
    /// The first condition, then those the objects must also meet (or,
    /// with `any`, may meet instead; those `not`, must not).
    conditions: Vec<KeptCondition>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    any: bool,
    /// The kind of object looked at (its extension); none: all.
    #[serde(skip_serializing_if = "Option::is_none")]
    object: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    placed: Option<bool>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    regex: bool,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    haks_and_game: bool,
}

impl Condition {
    fn kept(&self) -> KeptCondition {
        KeptCondition {
            kind: self.kind.id().into(),
            name: self.name.clone(),
            var_type: self.var_type.map(|t| t as i64),
            value: self.value.clone(),
            number: self.number,
            row: self.row,
            subtype: self.subtype,
            cost: self.cost,
            param: self.param,
            min_rank: self.min_rank,
            levels: [self.levels.0, self.levels.1],
            class: self.class,
            of: APPEARANCES.iter().find(|(a, _)| *a == self.of).map_or("", |a| a.1).into(),
            not: self.not,
        }
    }

    /// The condition kept (`None`: a kind not known).
    fn from_kept(k: &KeptCondition) -> Option<Condition> {
        let d = Condition::default();
        Some(Condition {
            kind: Kind::ALL.into_iter().find(|kind| kind.id() == k.kind)?,
            name: k.name.clone(),
            var_type: k.var_type.and_then(VarType::of),
            value: k.value.clone(),
            number: k.number,
            row: k.row,
            subtype: k.subtype,
            cost: k.cost,
            param: k.param,
            min_rank: k.min_rank.max(1),
            levels: match k.levels {
                [0, 0] => d.levels,
                [a, b] => (a.clamp(1, TOP_LEVEL), b.clamp(1, TOP_LEVEL)),
            },
            class: k.class,
            of: APPEARANCES.iter().find(|(_, n)| *n == k.of).map_or(d.of, |a| a.0),
            parts: None,
            not: k.not,
        })
    }
}

/// The Advanced Search window's state.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AdvancedSearch {
    /// What is looked for.
    pub first: Condition,
    /// And what the same objects must also hold (or, with `any`, may hold
    /// instead; those ticked `not`, must not hold).
    pub and: Vec<Condition>,
    /// Objects meeting any of the conditions, not all of them.
    pub any: bool,
    /// The kind of object looked at; none: all.
    pub object: Option<ResType>,
    /// Only placed objects, only blueprints.
    pub placed: Option<bool>,
    /// Names and texts are regular expressions (not `*` patterns).
    pub regex: bool,
    /// The haks' and the game's blueprints are searched too (read-only).
    pub haks_and_game: bool,
    /// What was found, for the query in `searched`.
    pub hits: Vec<Hit>,
    pub searched: Option<Query>,
    /// Which of the hits are ticked (to change, to edit together).
    pub ticked: Vec<bool>,
    /// The change to make to them.
    pub act: ActDraft,
    /// The events the objects found have (script fields' labels).
    pub events: Vec<String>,
    /// The module's revision (`Workspace::revision`) the hits were found
    /// at. A hit is a place (`keep.git /Creature List[3]`): once the
    /// module has changed (an object deleted, an Undo, a file read again)
    /// another object may stand there, so nothing acts on the hits before
    /// they are found again.
    pub found_at: Option<u64>,
    /// Why a click did not do what it says (the hits were found again
    /// first), until the next Find or change.
    pub note: Option<String>,
}

impl AdvancedSearch {
    /// Its question, as a session keeps it (`None`: none asked, the
    /// window as it opens).
    pub(crate) fn kept(&self) -> Option<KeptSearch> {
        let fresh = AdvancedSearch::default();
        let asked = self.first != fresh.first
            || !self.and.is_empty()
            || self.object.is_some()
            || self.placed.is_some()
            || self.any
            || self.regex
            || self.haks_and_game;
        asked.then(|| KeptSearch {
            conditions: std::iter::once(&self.first)
                .chain(&self.and)
                .map(Condition::kept)
                .collect(),
            any: self.any,
            object: self.object.and_then(|t| t.extension()).map(str::to_string),
            placed: self.placed,
            regex: self.regex,
            haks_and_game: self.haks_and_game,
        })
    }

    /// Asks the question a session kept again (what it found is not: a
    /// search of a large module takes a while).
    pub(crate) fn ask_again(&mut self, k: &KeptSearch) {
        let mut conditions = k.conditions.iter().filter_map(Condition::from_kept);
        let Some(first) = conditions.next() else { return };
        self.first = first;
        self.and = conditions.collect();
        self.any = k.any;
        self.object = k.object.as_deref().and_then(ResType::from_extension);
        self.placed = k.placed;
        self.regex = k.regex;
        self.haks_and_game = k.haks_and_game;
    }

    /// The query the window asks, or what it still needs.
    pub fn query(&self) -> Result<Query, String> {
        let (not, and): (Vec<&Condition>, Vec<&Condition>) = self.and.iter().partition(|c| c.not);
        let searches = |cs: Vec<&Condition>| -> Result<Vec<Search>, String> {
            cs.into_iter().map(Condition::search).collect()
        };
        let q = Query {
            search: self.first.search()?,
            and: searches(and)?,
            any: self.any,
            not: searches(not)?,
            types: self.object.into_iter().collect(),
            placed: self.placed,
            regex: self.regex,
            haks_and_game: self.haks_and_game,
        };
        q.check().map_err(|e| format!("Not a regular expression: {e}"))?;
        Ok(q)
    }

    /// The hits ticked (a hak's or the game's blueprint never is: it is
    /// read-only).
    pub fn chosen(&self) -> Vec<&Hit> {
        (self.hits.iter().zip(&self.ticked))
            .filter(|(h, t)| **t && h.layer.is_none())
            .map(|(h, _)| h)
            .collect()
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
        draft.ticked = draft.hits.iter().map(|h| h.layer.is_none()).collect();
        draft.events = found::events(&ws.module, &draft.hits.iter().collect::<Vec<_>>());
        let asked_anew = draft.searched.as_ref() != Some(&q);
        draft.searched = Some(q);
        draft.found_at = Some(ws.revision());
        // (A field to set follows what is looked for, when another thing
        // is looked for. Every search wrote the field's label over the
        // name, which is a variable's and an item's too: a variable typed
        // as nBoss was Appearance_Type after the search a change ends
        // with, and the next change set a variable of that name.)
        if asked_anew
            && draft.act.act == Act::SetField
            && let Some(field) = draft.first.field()
        {
            draft.act.name = field;
        }
    }

    /// Whether the module changed since `draft`'s hits were found (its
    /// pending edits written first): they are found again then, all
    /// ticked, with a note of why, and the caller does nothing with them
    /// this time. The hits name places, and whatever stands there now
    /// would be changed, opened or edited together.
    pub fn found_again(&mut self, draft: &mut AdvancedSearch) -> bool {
        let Some(ws) = self.ws.as_mut() else { return false };
        if let Err(e) = ws.flush() {
            self.log.error(e.to_string());
            return true;
        }
        if draft.searched.is_none() || draft.found_at == Some(ws.revision()) {
            return false;
        }
        // (What was asked then, not what the fields say now.)
        let asked = draft.searched.clone();
        let game = self.game.as_deref();
        let cp = game.map_or(Codepage::WINDOWS_1252, GameData::codepage);
        if let Some(q) = &asked {
            draft.hits = self.search_kept.search(&ws.module, q, game, cp);
            draft.ticked = draft.hits.iter().map(|h| h.layer.is_none()).collect();
            draft.events = found::events(&ws.module, &draft.hits.iter().collect::<Vec<_>>());
            draft.found_at = Some(ws.revision());
        }
        draft.note = Some(
            "The module changed since this was found. Found again, everything checked: look the \
             list over first."
                .into(),
        );
        true
    }

    /// Makes the window's change to what it found and ticked, in one
    /// step, then finds again. Where the module changed since they were
    /// found, they are found again and nothing is changed
    /// ([`Moonglow::found_again`]).
    pub fn change_found(&mut self, draft: &mut AdvancedSearch) {
        let change = match draft.act.change(self.game.as_deref()) {
            Ok(c) => c,
            Err(e) => {
                draft.act.said = Some(e);
                return;
            }
        };
        if self.found_again(draft) {
            draft.act.said = draft.note.clone();
            return;
        }
        draft.note = None;
        let Some(ws) = self.ws.as_mut() else { return };
        if let Err(e) = ws.flush() {
            self.log.error(e.to_string());
            return;
        }
        let cp = self.game.as_deref().map_or(Codepage::WINDOWS_1252, GameData::codepage);
        let game = self.game.as_deref();
        let out = match found::edits(&ws.module, &draft.chosen(), &change, cp, game) {
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
        match self.apply_or_ask(Command::new(label.clone(), out.edits), None) {
            Ok(true) => {}
            // (Held: the Protected Area window names what it would change.)
            Ok(false) => {
                draft.act.said = Some("Asked first: it changes a protected area".into());
                return;
            }
            Err(e) => {
                draft.act.said = Some(e.to_string());
                return;
            }
        }
        draft.act.said = Some(format!("{label}{left}"));
        self.advanced_search(draft);
    }
}

/// A row of a table, chosen by name (a long list has a filter); `any`
/// names the choice of none. With no names (no game data), a number.
pub(crate) fn row_picker(
    ui: &mut Ui,
    id: &str,
    choices: &[Choice],
    row: &mut Option<usize>,
    any: Option<&str>,
) {
    row_picker_in(ui, id, choices, row, any, false);
}

/// [`row_picker`], in the order given where `ordered` (an item
/// property's values, as the item editor's Cost Parameter has them: +2
/// before +10).
fn row_picker_in(
    ui: &mut Ui,
    id: &str,
    choices: &[Choice],
    row: &mut Option<usize>,
    any: Option<&str>,
    ordered: bool,
) {
    if choices.is_empty() {
        let mut on = row.is_some() || any.is_none();
        if any.is_some() {
            ui.check_box(&mut on, "");
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
    let mut list = crate::widgets::row_choice(egui::Id::new(id), choices, current)
        .width(260.0)
        .about(&mut number);
    if ordered {
        list = list.ordered();
    }
    let list = match any {
        Some(any) => list.none(any),
        None => list.unset("Choose…"),
    };
    if let Some(picked) = list.show_optional(ui) {
        *row = picked.and_then(|r| usize::try_from(r).ok());
    }
}

/// The choices of a table as the game data keeps them (`Arc<Vec<Choice>>`),
/// shared rather than copying whole tables on every frame.
pub(crate) fn listed(
    game: Option<&GameData>,
    table: &mg_rules::rows::RowTable,
) -> std::sync::Arc<Vec<Choice>> {
    let Some(game) = game else { return Default::default() };
    let columns = ChoiceColumns { name: table.name, label: table.label };
    game.choice_list(table.table, columns).unwrap_or_default()
}

/// A condition's rows of the window's grid; `salt` keeps its widgets'
/// ids its own (empty for the first). True when Enter asks to find.
fn condition_ui(ui: &mut Ui, c: &mut Condition, game: Option<&GameData>, salt: &str) -> bool {
    let mut find = false;
    let text = |ui: &mut Ui, value: &mut String, hint: &str| -> bool {
        let r = ui.add(mg_uikit::text_field(value).hint_text(hint).desired_width(260.0));
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
            row_picker(ui, &id("property"), &listed(game, &table), &mut c.row, None);
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
                    // (Each in the item editor's order: the values as the
                    // cost table has them, the others by name.)
                    let ordered = label == "Value";
                    row_picker_in(ui, &id(label), choices, row, Some("Any"), ordered);
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
            row_picker(ui, &id("row"), &listed(game, &table), &mut c.row, None);
            ui.end_row();
            if c.kind == Kind::Spell {
                crate::widgets::field_label(ui, "As class")
                    .on_hover_text("Creatures that know or have memorized it as this class");
                let classes = Search::Class(0).table().expect("the classes' table");
                row_picker(
                    ui,
                    &id("spell-class"),
                    &listed(game, &classes),
                    &mut c.class,
                    Some("Any"),
                );
                ui.end_row();
            }
            if c.kind == Kind::Skill {
                crate::widgets::field_label(ui, "At least");
                let suffix = rank_suffix(c.min_rank);
                ui.add(egui::DragValue::new(&mut c.min_rank).range(1..=127).suffix(suffix));
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
                ui.check_box(&mut c.number, "Number").on_hover_text(
                    "Number fields holding this number (103, 1.5) or in a range (5..10)",
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
        (c.row, c.subtype, c.cost, c.param, c.class) = (None, None, None, None, None);
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
            let (label, hover) = match (c.not, d.any) {
                (true, _) => (
                    "Not",
                    "Objects that don't hold this (nor hold, nor are held by, one that does)",
                ),
                (false, true) => {
                    ("Or", "Objects that hold this instead (each found where it is found)")
                }
                (false, false) => {
                    ("And", "Objects that hold this too (or hold, or are held by, one that does)")
                }
            };
            crate::widgets::field_label(ui, label).on_hover_text(hover);
            ui.horizontal(|ui| {
                kind_choice(ui, c, &format!("advanced-search-kind-{i}"));
                ui.check_box(&mut c.not, "Not")
                    .on_hover_text("Leave out the objects that meet this condition");
                if ui.small_button("Remove").clicked() {
                    remove = Some(i);
                }
            });
            ui.end_row();
            find |= condition_ui(ui, c, game, &format!("-{i}"));
        }
        ui.label("");
        let (more, hover) = match d.any {
            false => ("And…", "Another condition the objects found must meet"),
            true => ("Or…", "Another condition the objects found may meet instead"),
        };
        if ui.button(more).on_hover_text(hover).clicked() {
            d.and.push(Condition { kind: Kind::Feat, ..Default::default() });
        }
        ui.end_row();
        if !d.and.is_empty() {
            crate::widgets::field_label(ui, "Find objects");
            let meeting = |any: &bool| match any {
                false => "meeting all of the conditions",
                true => "meeting any of the conditions",
            };
            crate::widgets::dropdown("advanced-search-any")
                .selected_text(meeting(&d.any))
                .width(260.0)
                .show(ui, &mut d.any, &[false, true], meeting)
                .on_hover_text(
                    "All: objects every condition finds. Any: objects any condition finds.",
                );
            ui.end_row();
        }
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
        ui.label("");
        ui.check_box(&mut d.regex, "Regular expressions").on_hover_text(
            "Names and texts are regular expressions, found anywhere unless anchored (^kz_)",
        );
        ui.end_row();
        ui.label("");
        ui.check_box(&mut d.haks_and_game, "The haks' and the game's blueprints too")
            .on_hover_text("Also the haks' and the game's blueprints, read-only");
        ui.end_row();
    });
    if let Some(i) = remove {
        d.and.remove(i);
    }
    find
}

/// What can be opened together from what is ticked: blueprints of one
/// type; objects of one kind placed in areas (one or several), each its
/// area and its path in the area's GIT; the items placed objects hold
/// (`kind` none), apart from those lying in an area and from the item
/// blueprints, which have pages of their own.
#[derive(Debug, Clone, PartialEq)]
enum Together {
    Blueprints(ResType, Vec<ResKey>),
    /// `areas`: how many areas they are in (worked out once, with the
    /// groups: not each frame the buttons are drawn).
    Placed {
        kind: Option<ObjectKind>,
        objects: Vec<(ResRef, GffPath)>,
        areas: usize,
    },
}

impl Together {
    fn tab(&self) -> Tab {
        match self {
            Together::Blueprints(_, keys) if keys.len() == 1 => Tab::Blueprint(keys[0]),
            Together::Blueprints(_, keys) => Tab::Blueprints(keys.clone()),
            Together::Placed { objects, .. } if objects.len() == 1 => {
                let (area, path) = objects[0].clone();
                Tab::Instance { area, path }
            }
            Together::Placed { objects, .. } => {
                Tab::Instances { objects: objects.as_slice().into() }
            }
        }
    }
}

/// The groups of what is ticked that open together, and the areas it is
/// in (or is).
fn together(hits: &[&Hit]) -> (Vec<Together>, Vec<ResRef>) {
    let mut blueprints: BTreeMap<ResType, Vec<ResKey>> = BTreeMap::new();
    let mut placed: BTreeMap<Option<ObjectKind>, Vec<(ResRef, GffPath)>> = BTreeMap::new();
    let mut areas: Vec<ResRef> = Vec::new();
    // (Each area looked for in a set, not the list: a persistent world's
    // 8,800 hits in 300 areas were a scan of the list a hit, most of a
    // frame of the window, every frame it showed.)
    let mut areas_seen = std::collections::HashSet::new();
    let mut seen = std::collections::HashSet::new();
    for h in hits {
        if matches!(h.key.restype, ResType::ARE | ResType::GIT) && areas_seen.insert(h.key.resref) {
            areas.push(h.key.resref);
        }
        let Some(o): Option<Object> = h.object() else { continue };
        if !seen.insert(o.clone()) {
            continue;
        }
        if o.path.is_root() && ObjectKind::from_restype(o.key.restype).is_some() {
            blueprints.entry(o.key.restype).or_default().push(o.key);
        } else if o.placed()
            && let [one] = &o.path.0[..]
            && let Some((list, i)) = one.item()
            && let Some(kind) = ObjectKind::from_list(list)
        {
            placed
                .entry(Some(kind))
                .or_default()
                .push((o.key.resref, GffPath::root().item(list, i)));
        } else if o.placed() && crate::blueprint::held_item(&o.path) {
            // (An item a placed object holds, or a store sells: not one in
            // a bag, edited with the bag.)
            placed.entry(None).or_default().push((o.key.resref, o.path));
        }
    }
    let mut out: Vec<Together> =
        blueprints.into_iter().map(|(t, keys)| Together::Blueprints(t, keys)).collect();
    // (The held items after the kinds of object placed.)
    let (held, kinds): (Vec<_>, Vec<_>) = placed.into_iter().partition(|(k, _)| k.is_none());
    out.extend(kinds.into_iter().chain(held).map(|(kind, objects)| {
        let areas = Tab::areas_of(&objects).len();
        Together::Placed { kind, objects, areas }
    }));
    (out, areas)
}

/// The change to make, and its button; true when it is asked.
fn act_ui(ui: &mut Ui, d: &mut AdvancedSearch, n: usize, game: Option<&GameData>) -> bool {
    let mut go = false;
    // (The field of what was looked for, offered as the one to set.)
    let offered = d.first.field();
    let a = &mut d.act;
    egui::Grid::new("advanced-search-act").num_columns(2).show(ui, |ui| {
        crate::widgets::field_label(ui, "Change");
        let was = a.act;
        crate::widgets::dropdown("advanced-search-act")
            .selected_text(a.act.label())
            .width(260.0)
            .show(ui, &mut a.act, &Act::ALL, |a| a.label());
        if a.act != was
            && a.act == Act::SetField
            && let Some(field) = offered
        {
            a.name = field;
        }
        ui.end_row();
        let edit = |ui: &mut Ui, v: &mut String, hint: &str| {
            ui.add(mg_uikit::text_field(v).hint_text(hint).desired_width(260.0));
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
            Act::AddFeat | Act::RemoveFeat | Act::AddClass | Act::RemoveClass => {
                let class = matches!(a.act, Act::AddClass | Act::RemoveClass);
                let kind = if class { Kind::Class } else { Kind::Feat };
                if a.entry.kind != kind {
                    (a.entry.kind, a.entry.row) = (kind, None);
                }
                crate::widgets::field_label(ui, if class { "Class" } else { "Feat" });
                let table = a.entry.table().expect("a table");
                row_picker(
                    ui,
                    "advanced-search-act-row",
                    &listed(game, &table),
                    &mut a.entry.row,
                    None,
                );
                ui.end_row();
                if a.act == Act::AddClass {
                    crate::widgets::field_label(ui, "Level");
                    ui.add(egui::DragValue::new(&mut a.entry.levels.0).range(1..=TOP_LEVEL))
                        .on_hover_text("The class's level on each creature it is added to");
                    ui.end_row();
                }
            }
            Act::AddProperty | Act::RemoveProperty => {
                if a.entry.kind != Kind::ItemProperty {
                    (a.entry.kind, a.entry.row) = (Kind::ItemProperty, None);
                }
                condition_ui(ui, &mut a.entry, game, "-act");
            }
            Act::AddItem | Act::RemoveItem => {
                crate::widgets::field_label(ui, "Item");
                edit(ui, &mut a.name, "its blueprint's resref, e.g. nw_it_torch001");
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
        if a.act == Act::SetField {
            ui.label("");
            ui.add(mg_uikit::Checkbox::new(&mut a.add, "Add where missing")).on_hover_text(ADD_TIP);
            ui.end_row();
        }
    });
    let ok = a.change(game);
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

/// A result as one line of text (its menu's Copy, a line of Copy All):
/// its place as listed, the layer it is read from where it is not the
/// module's, and what matched, "area_01 › Chest: Gold 50".
pub fn hit_line(app: &mut Moonglow, h: &mg_module::search::Hit) -> String {
    let place = place_shown(app, h);
    match &h.layer {
        Some(layer) => format!("{place} ({layer}): {}", h.matched),
        None => format!("{place}: {}", h.matched),
    }
}

/// A hit's place as its row shows it, with areas named as the rest of
/// the window names them (Options › General: Show areas by name).
fn place_shown(app: &mut Moonglow, h: &mg_module::search::Hit) -> String {
    let of_area = matches!(h.key.restype, ResType::ARE | ResType::GIT);
    if !of_area || !app.settings.area_names {
        return h.place.clone();
    }
    let name = crate::tabs::area_label(app, h.key.resref);
    h.place.replacen(&h.key.resref.to_lowercase().to_string(), &name, 1)
}

type TogetherGroups = (std::sync::Arc<Vec<Together>>, std::sync::Arc<Vec<ResRef>>);

/// Cached summary derived from the current hit list and tick vector so idle
/// frames do not scan 8,800+ hits or rebuild `together` when collapsed.
#[derive(Clone, Default)]
struct SearchUiCache {
    sig: (usize, usize, Option<u64>, Option<ResKey>, Option<ResKey>, u64),
    ticked_count: usize,
    outside_count: usize,
    together: Option<TogetherGroups>,
    /// Whether a plugin's command fits the ticked hits, for the commands
    /// it was worked out with (held: by their list's address and length,
    /// another list made at the same address took the old one's answer).
    ticked_fits: Option<(std::sync::Arc<[crate::plugins::PluginCommand]>, bool)>,
}

fn ticked_hash(ticked: &[bool]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325 ^ (ticked.len() as u64);
    for &b in ticked {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

fn hit_selected(h: &Hit) -> mg_plugin::Selected {
    crate::plugins::selected_at(h.key, &h.path)
}

/// The Advanced Search tab.
pub(crate) fn ui(app: &mut Moonglow, ui: &mut Ui) {
    if app.ws.is_none() {
        ui.label("Open a module first.");
        return;
    }
    let mut draft = std::mem::take(&mut app.advanced_search);
    let (mut find, mut go, mut change) = (false, None, false);
    let game = app.game.clone();
    ui.weak(
        "The module's blueprints, placed objects (and the items they hold), areas and \
         conversations, by what they hold; and, checked below, the haks' and the game's \
         blueprints.",
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
    if let Some(note) = &draft.note {
        ui.colored_label(ui.visuals().warn_fg_color, note);
    }
    let mut open = Vec::new();
    let mut choose_areas = None;
    // (A result's menu's Copy or Copy All: its text made after the rows.)
    let mut copy = None;
    if draft.searched.is_some() {
        ui.separator();
        draft.ticked.resize(draft.hits.len(), true);
        if draft.hits.is_empty() {
            ui.weak(match draft.searched.as_ref().is_some_and(|q| q.haks_and_game) {
                true => "Nothing in the module, its haks or the game holds it.",
                false => "Nothing in the module holds it.",
            });
        } else {
            let sig = (
                draft.hits.len(),
                draft.hits.as_ptr() as usize,
                draft.found_at,
                draft.hits.first().map(|h| h.key),
                draft.hits.last().map(|h| h.key),
                ticked_hash(&draft.ticked),
            );
            let cache_id = egui::Id::new("advanced-search-ui-cache");
            let mut cache: SearchUiCache = ui.data(|d| d.get_temp(cache_id)).unwrap_or_default();
            let mut cache_dirty = false;
            if cache.sig != sig {
                let ticked_count = draft
                    .hits
                    .iter()
                    .zip(&draft.ticked)
                    .filter(|(h, t)| **t && h.layer.is_none())
                    .count();
                let outside_count = draft.hits.iter().filter(|h| h.layer.is_some()).count();
                cache = SearchUiCache {
                    sig,
                    ticked_count,
                    outside_count,
                    together: None,
                    ticked_fits: None,
                };
                cache_dirty = true;
            }
            let n = cache.ticked_count;
            let outside = cache.outside_count;
            ui.horizontal(|ui| {
                ui.label(format!("{} found:", draft.hits.len()));
                if outside > 0 {
                    ui.weak(format!("({outside} of them the haks' or the game's, read-only)"));
                }
                if ui.small_button("Check All").clicked() {
                    for (t, h) in draft.ticked.iter_mut().zip(&draft.hits) {
                        *t = h.layer.is_none();
                    }
                }
                if ui.small_button("Check None").clicked() {
                    draft.ticked.fill(false);
                }
            });
            egui::CollapsingHeader::new(format!("Change the {n} checked"))
                .icon(mg_uikit::collapsing_icon)
                .id_salt("advanced-search-change")
                .show(ui, |ui| change = act_ui(ui, &mut draft, n, app.game.as_deref()));
            let area_names = app.settings.area_names;
            egui::CollapsingHeader::new(format!("Edit the {n} checked together"))
                .icon(mg_uikit::collapsing_icon)
                .id_salt("advanced-search-together")
                .show(ui, |ui| {
                    let (groups, areas) = cache
                        .together
                        .get_or_insert_with(|| {
                            cache_dirty = true;
                            let chosen = draft.chosen();
                            let (g, a) = together(&chosen);
                            (std::sync::Arc::new(g), std::sync::Arc::new(a))
                        })
                        .clone();
                    if groups.is_empty() && areas.is_empty() {
                        ui.weak(
                            "Blueprints of one type, objects of one kind placed in areas, \
                             the items they hold",
                        );
                    }
                    ui.horizontal_wrapped(|ui| {
                        for g in groups.iter() {
                            let text = match g {
                                Together::Blueprints(t, keys) => {
                                    let kind =
                                        ObjectKind::from_restype(*t).map_or("", ObjectKind::name);
                                    blueprints_together(kind, keys.len())
                                }
                                Together::Placed { kind, objects, areas } => {
                                    let area = objects[0].0;
                                    let place = match *areas {
                                        1 if area_names => crate::tabs::area_label(app, area),
                                        1 => area.to_lowercase().to_string(),
                                        n => format!("{n} areas"),
                                    };
                                    placed_together(*kind, objects.len(), &place)
                                }
                            };
                            if ui.button(text).clicked() {
                                open.push(g.tab());
                            }
                        }
                        if !areas.is_empty()
                            && ui
                                .button(format!(
                                    "Choose {} {}…",
                                    areas.len(),
                                    if areas.len() == 1 { "Area" } else { "Areas" }
                                ))
                                .on_hover_text(
                                    "Edit Areas Together with the areas of the checked results",
                                )
                                .clicked()
                        {
                            choose_areas = Some((*areas).clone());
                        }
                    });
                });
            let commands = app.plugin_menu_commands_cached();
            // What a plugin's command is run on from a ticked row: every
            // ticked hit's. Whether a command fits it is worked out once,
            // when a ticked row in sight first asks, and kept until the
            // hits, the ticks or the commands change.
            let for_these = |c: &std::sync::Arc<[crate::plugins::PluginCommand]>| {
                std::sync::Arc::ptr_eq(c, &commands)
            };
            let mut ticked_fits =
                cache.ticked_fits.as_ref().filter(|(c, _)| for_these(c)).map(|(_, f)| *f);
            let mut menu_actions = Vec::new();
            // (Only the rows in sight are named and laid out; the others
            // take a row's room, as Find References' do.)
            mg_uikit::show_rows(
                egui::ScrollArea::vertical().auto_shrink(false),
                ui,
                draft.hits.len(),
                |ui, range| {
                    for i in range {
                        let place = place_shown(app, &draft.hits[i]);
                        // (Each row exactly a row's room: rows of another
                        // height made the list's height change with what
                        // is in sight, and its scroll bar came and went.)
                        let size = egui::vec2(ui.available_width(), ui.spacing().interact_size.y);
                        let layout = egui::Layout::left_to_right(egui::Align::Center);
                        ui.allocate_ui_with_layout(size, layout, |ui| {
                            ui.set_min_height(size.y);
                            ui.set_max_height(size.y);
                            // (The row's own response first, under its tick
                            // and link: a right click on it opens its menu
                            // too. Not a Tab stop: its link is.)
                            let room = egui::Rect::from_min_size(ui.cursor().min, size);
                            let whole =
                                ui.interact(room, ui.id().with(("hit-row", i)), egui::Sense::CLICK);
                            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Truncate);
                            let h = &draft.hits[i];
                            let mine = h.layer.is_none();
                            let tick = mg_uikit::Checkbox::without_text(&mut draft.ticked[i]);
                            let tick = ui.add_enabled(mine, tick);
                            let r = ui.link(place).on_hover_ui(|ui| {
                                ui.label(match &h.layer {
                                    Some(layer) => {
                                        format!(
                                            "{} {}\nRead from {layer}: read-only",
                                            h.key, h.path
                                        )
                                    }
                                    None => format!("{} {}", h.key, h.path),
                                });
                            });
                            if h.layer.is_some() {
                                tick.on_disabled_hover_text(
                                    "Read-only: the module has no copy of it",
                                );
                            }
                            if r.clicked() {
                                go = Some(i);
                            }
                            if let Some(layer) = &h.layer {
                                ui.label(egui::RichText::new(format!("({layer})")).italics());
                            }
                            // (Cut at the row's end, whole in its tip; not
                            // selectable, a right click on it the row's.)
                            let matched = egui::RichText::new(&h.matched).weak();
                            ui.add(egui::Label::new(matched).selectable(false))
                                .on_hover_text(&h.matched);
                            // Its menu: the plugins' commands for it, or
                            // (one of those ticked) for each ticked; then
                            // Copy and Copy All (every result, ticked or
                            // not: the ticks say what a change takes, not
                            // what is chosen).
                            let is_ticked = draft.ticked[i];
                            let fits = if commands.is_empty() {
                                false
                            } else if is_ticked {
                                *ticked_fits.get_or_insert_with(|| {
                                    let mut kinds: Vec<mg_plugin::Target> =
                                        (draft.hits.iter().zip(&draft.ticked))
                                            .filter(|(_, t)| **t)
                                            .map(|(h, _)| hit_selected(h).target())
                                            .collect::<Option<Vec<_>>>()
                                            .unwrap_or_default();
                                    kinds.sort();
                                    kinds.dedup();
                                    commands.iter().any(|c| c.fits(&kinds))
                                })
                            } else {
                                let alone = [hit_selected(h)];
                                commands.iter().any(|c| c.fits(&crate::plugins::kinds_of(&alone)))
                            };
                            // (Shift+F10 and Ctrl+C where the keyboard is
                            // on its link, the row's menu's and its focus's
                            // id.)
                            let row = r | whole;
                            let (keyed, ctrl_c) = row_copy::focused_keys(ui, &row);
                            if ctrl_c {
                                copy = Some((row_copy::Asked::Row, i));
                            }
                            let actions = &mut menu_actions;
                            let asked = row_copy::row_menu(ui, &row, keyed, |ui| {
                                if !fits {
                                    return false;
                                }
                                let on: Vec<mg_plugin::Selected> = if is_ticked {
                                    let mut on: Vec<mg_plugin::Selected> =
                                        (draft.hits.iter().zip(&draft.ticked))
                                            .filter(|(_, t)| **t)
                                            .map(|(h, _)| hit_selected(h))
                                            .collect();
                                    on.dedup();
                                    on
                                } else {
                                    vec![hit_selected(h)]
                                };
                                crate::plugins::menu_on(ui, &commands, &on, actions);
                                true
                            });
                            if asked.is_some() {
                                copy = asked.map(|a| (a, i));
                            }
                        });
                    }
                },
            );
            app.actions.extend(menu_actions);
            if let Some(fits) = ticked_fits
                && !cache.ticked_fits.as_ref().is_some_and(|(c, f)| for_these(c) && *f == fits)
            {
                cache.ticked_fits = Some((commands.clone(), fits));
                cache_dirty = true;
            }
            if cache_dirty {
                ui.data_mut(|d| d.insert_temp(cache_id, cache));
            }
        }
    }
    match copy {
        Some((row_copy::Asked::Row, i)) => {
            let line = hit_line(app, &draft.hits[i]);
            ui.ctx().copy_text(line);
        }
        Some((row_copy::Asked::All, _)) => {
            let all: Vec<String> = draft.hits.iter().map(|h| hit_line(app, h)).collect();
            ui.ctx().copy_text(row_copy::lines(all));
        }
        None => {}
    }
    if find && draft.query().is_ok() {
        draft.act.said = None;
        draft.note = None;
        app.advanced_search(&mut draft);
    }
    if change {
        app.change_found(&mut draft);
    }
    // (A place opened or edited together is the hit's as it was found:
    // in a module changed since, they are found again instead.)
    let acts = go.is_some() || !open.is_empty() || choose_areas.is_some();
    if acts && app.found_again(&mut draft) {
        (go, choose_areas) = (None, None);
        open.clear();
    }
    if let Some(i) = go {
        let h = &draft.hits[i];
        match &h.layer {
            // (A hak's or the game's: its copy, to look at.)
            Some(layer) => {
                let tab = Tab::ResourceCopy { key: h.key, layer: layer.clone() };
                app.actions.push(Action::OpenTab(tab));
            }
            None => {
                let usage = h.usage();
                app.go_to_usage(&usage);
            }
        }
    }
    for tab in open {
        app.actions.push(Action::OpenTab(tab));
    }
    if let Some(areas) = choose_areas {
        app.area_chooser = Some(crate::area_props::AreaChooser::of(areas));
        crate::tool_sections::bring(app, crate::shell::Panel::AreasTogether);
    }
    app.advanced_search = draft;
}

/// Add where missing's tip.
pub const ADD_TIP: &str = "Also gives the field to those without it, where their kind of object \
     has it, instead of the game's default";

/// Skill's "At least" field's suffix: " rank" for one, " ranks" else.
fn rank_suffix(n: u8) -> &'static str {
    if n == 1 { " rank" } else { " ranks" }
}

/// Edit Together's button for `n` blueprints of `kind` (its name).
fn blueprints_together(kind: &str, n: usize) -> String {
    let word = if n == 1 { "blueprint" } else { "blueprints" };
    format!("Edit {n} {kind} {word} Together")
}

/// Edit Together's button for `n` objects of `kind` placed (`None`: items
/// placed objects hold) in `place`: an area, as the window names areas,
/// or how many.
fn placed_together(kind: Option<ObjectKind>, n: usize, place: &str) -> String {
    let noun = match (kind, n) {
        (Some(kind), 1) => kind.name().to_string(),
        (Some(kind), _) => kind.label().to_lowercase(),
        (None, 1) => "held item".to_string(),
        (None, _) => "held items".to_string(),
    };
    format!("Edit {n} {noun} in {place} Together")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Wave 3's counted words: one rank, one blueprint, one creature,
    /// and the plurals past one.
    #[test]
    fn counted_labels_say_one_and_more() {
        assert_eq!(rank_suffix(1), " rank");
        assert_eq!(rank_suffix(2), " ranks");
        assert_eq!(rank_suffix(127), " ranks");
        let creature = ObjectKind::Creature.name();
        assert_eq!(
            blueprints_together(creature, 1),
            format!("Edit 1 {creature} blueprint Together")
        );
        assert_eq!(
            blueprints_together(creature, 3),
            format!("Edit 3 {creature} blueprints Together")
        );
        let (creature, door) = (Some(ObjectKind::Creature), Some(ObjectKind::Door));
        assert_eq!(placed_together(creature, 1, "start"), "Edit 1 creature in start Together");
        assert_eq!(placed_together(creature, 2, "start"), "Edit 2 creatures in start Together");
        assert_eq!(placed_together(door, 1, "Inn"), "Edit 1 door in Inn Together");
        assert_eq!(placed_together(door, 4, "Inn"), "Edit 4 doors in Inn Together");
        assert_eq!(placed_together(door, 4, "3 areas"), "Edit 4 doors in 3 areas Together");
        assert_eq!(placed_together(None, 1, "Inn"), "Edit 1 held item in Inn Together");
        assert_eq!(placed_together(None, 5, "2 areas"), "Edit 5 held items in 2 areas Together");
    }

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

    /// The areas of the hits are each named once, in the order their
    /// first hit came, as a look through the list made them.
    #[test]
    fn the_hits_areas_are_named_once_in_their_order() {
        let hit = |area: &str, t: ResType| Hit {
            key: ResKey::new(ResRef::from_str(area).unwrap(), t),
            path: GffPath::root(),
            place: String::new(),
            matched: String::new(),
            object: t,
            carried: None,
            spawned: None,
            layer: None,
        };
        let names = ["b", "a", "b", "c", "a", "d", "c"];
        let mut hits: Vec<Hit> = names.iter().map(|n| hit(n, ResType::GIT)).collect();
        hits.push(hit("e", ResType::ARE));
        hits.push(hit("f", ResType::UTC));
        let chosen: Vec<&Hit> = hits.iter().collect();
        let mut looked: Vec<ResRef> = Vec::new();
        for h in &chosen {
            if matches!(h.key.restype, ResType::ARE | ResType::GIT)
                && !looked.contains(&h.key.resref)
            {
                looked.push(h.key.resref);
            }
        }
        let (_, areas) = together(&chosen);
        assert_eq!(areas, looked);
        assert_eq!(areas.len(), 5);
    }

    /// The conditions ticked Not are those the query leaves out, the
    /// others its `and` (any of them, with `any`); a session keeps both.
    #[test]
    fn conditions_not_met_and_any_of_them_are_asked_and_kept() {
        let feat =
            |row, not| Condition { kind: Kind::Feat, row: Some(row), not, ..Default::default() };
        let d = AdvancedSearch {
            first: Condition { kind: Kind::Class, row: Some(4), ..Default::default() },
            and: vec![feat(0, true), feat(1, false), feat(2, true)],
            any: true,
            ..Default::default()
        };
        let q = d.query().unwrap();
        assert_eq!((q.search, q.any), (Search::Class(4), true));
        assert_eq!(q.and, [Search::Feat(1)]);
        assert_eq!(q.not, [Search::Feat(0), Search::Feat(2)]);
        let mut again = AdvancedSearch::default();
        again.ask_again(&d.kept().unwrap());
        assert_eq!((again.first, again.and, again.any), (d.first, d.and, true));
        // (A session from before keeps none of them.)
        let old: KeptSearch =
            serde_json::from_str(r#"{"conditions":[{"kind":"feat","row":3}]}"#).unwrap();
        let mut again = AdvancedSearch::default();
        again.ask_again(&old);
        assert!(!again.any && !again.first.not);
        assert!(AdvancedSearch { any: true, ..Default::default() }.kept().is_some());
    }

    /// A spell's class and the haks' and the game's blueprints are asked,
    /// and a session keeps them.
    #[test]
    fn a_spell_s_class_and_the_haks_and_game_are_asked_and_kept() {
        let d = AdvancedSearch {
            first: Condition {
                kind: Kind::Spell,
                row: Some(107),
                class: Some(2),
                ..Default::default()
            },
            haks_and_game: true,
            ..Default::default()
        };
        let q = d.query().unwrap();
        assert_eq!(q.search, Search::Spell { spell: 107, class: Some(2) });
        assert!(q.haks_and_game);
        let mut again = AdvancedSearch::default();
        again.ask_again(&d.kept().unwrap());
        assert_eq!((again.first.class, again.haks_and_game), (Some(2), true));
        assert!(AdvancedSearch { haks_and_game: true, ..Default::default() }.kept().is_some());
    }
}
