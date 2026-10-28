//! Described variables: what a project's local variables are, said in a
//! file of text the team keeps beside its `moonglow.toml`
//! (`variables.toml`, in the manner of a table's `name.2da.toml`): each
//! variable's type, the values it takes, the objects it goes on and a
//! line of help. The Variables window offers the names and their values;
//! Verify holds the module's variables to it.
//!
//! A variable read as another type than it was set is that type's empty
//! value (an Int read as a String is ""), and a name is matched in its
//! case only (`engine_local_variables`): both fail without a word, which
//! is what the description is there to catch.
//!
//! ```toml
//! undescribed = ["tdod_*"]        # a variable so named and not described here is said
//!
//! [tdod_workbench]
//! type = "string"
//! on = ["placeable"]
//! values = { forge = "Smithing", tanner = "Leatherworking" }
//! help = "The craft the bench serves, within 5 m"
//!
//! [tdod_lore_dc]
//! type = "int"
//! min = 1
//! max = 60
//!
//! [tdod_kind]
//! type = "string"
//! constants = "tdod_kind_inc"     # the values: this script's constants...
//! prefix = "TDOD_KIND_"           # ...whose names begin so
//!
//! [tdod_class]
//! type = "int"
//! table = "classes"               # a row of classes.2da (`column`: a cell of it)
//!
//! ["tdod_rental_*"]               # every variable so named
//! type = "string"
//! on = ["creature", "placeable"]
//! ```

use std::path::{Path, PathBuf};

use mg_2da::TwoDa;
use mg_core::{Codepage, ResRef, ResType};
use mg_gff::{Struct, Value};
use mg_resman::{ResKey, ResMan};
use mg_rules::ObjectKind;
use regex_lite::Regex;
use toml::de::{DeTable, DeValue};

use crate::Module;
use crate::project_file::{Reader, in_order, line_of};
use crate::table_desc::whole;
use crate::vars::VarType;

/// The team's file of described variables, in the project's root.
pub const FILE_NAME: &str = "variables.toml";

/// The longest name a variable is described by: what Aurora's Variables
/// window takes (its Name field's MaxLength). The game reads longer ones
/// (`engine_local_variables`: 4000 characters, from a file and set by a
/// script); a pattern is not held to it, as its `*` takes any length.
pub const NAME_MAX: usize = 1024;

/// What a variable is on: the module, an area, or an object (placed, or a
/// blueprint, or an item held).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Owner {
    Module,
    Area,
    Creature,
    Door,
    Encounter,
    Item,
    Placeable,
    Sound,
    Store,
    Trigger,
    Waypoint,
}

impl Owner {
    pub const ALL: [Owner; 11] = [
        Owner::Module,
        Owner::Area,
        Owner::Creature,
        Owner::Door,
        Owner::Encounter,
        Owner::Item,
        Owner::Placeable,
        Owner::Sound,
        Owner::Store,
        Owner::Trigger,
        Owner::Waypoint,
    ];

    /// As the file names it: `module`, `placeable`.
    pub fn name(self) -> &'static str {
        match self {
            Owner::Module => "module",
            Owner::Area => "area",
            Owner::Creature => "creature",
            Owner::Door => "door",
            Owner::Encounter => "encounter",
            Owner::Item => "item",
            Owner::Placeable => "placeable",
            Owner::Sound => "sound",
            Owner::Store => "store",
            Owner::Trigger => "trigger",
            Owner::Waypoint => "waypoint",
        }
    }

    pub fn from_name(name: &str) -> Option<Owner> {
        Owner::ALL.into_iter().find(|o| o.name().eq_ignore_ascii_case(name.trim()))
    }

    pub fn of_kind(kind: ObjectKind) -> Owner {
        match kind {
            ObjectKind::Creature => Owner::Creature,
            ObjectKind::Door => Owner::Door,
            ObjectKind::Encounter => Owner::Encounter,
            ObjectKind::Item => Owner::Item,
            ObjectKind::Placeable => Owner::Placeable,
            ObjectKind::Sound => Owner::Sound,
            ObjectKind::Store => Owner::Store,
            ObjectKind::Trigger => Owner::Trigger,
            ObjectKind::Waypoint => Owner::Waypoint,
        }
    }

    /// What the variables at a resource's root are on: the module's
    /// (`module.ifo`), an area's (its GIT, where the game reads them), a
    /// blueprint's.
    pub fn of_restype(t: ResType) -> Option<Owner> {
        match t {
            ResType::IFO => Some(Owner::Module),
            ResType::GIT => Some(Owner::Area),
            t => ObjectKind::from_restype(t).map(Owner::of_kind),
        }
    }

    /// What the struct a path in a resource of type `t` leads to is,
    /// by the lists the path goes through (`Placeable List`, `ItemList`),
    /// as [`variables_of`] walks them.
    pub fn of_place(t: ResType, lists: &[&str]) -> Option<Owner> {
        let mut owner = Owner::of_restype(t)?;
        for list in lists {
            owner = match *list {
                l if owner == Owner::Area => Owner::of_kind(ObjectKind::from_list(l)?),
                "ItemList" | "Equip_ItemList" => Owner::Item,
                // (A store's page.)
                "StoreList" if owner == Owner::Store => Owner::Store,
                _ => return None,
            };
        }
        Some(owner)
    }
}

/// A value a variable takes, and what it means (shown with it).
#[derive(Debug, Clone, PartialEq)]
pub struct Choice {
    pub value: String,
    pub note: Option<String>,
}

/// Where a variable's values are listed beyond those written.
#[derive(Debug, Clone, PartialEq)]
pub enum Source {
    /// A script's constants whose names begin with `prefix` (the
    /// project's script of that name, else the game's).
    Constants { script: String, prefix: String },
    /// A table's rows: an Int's value is a row's number; with `column`, a
    /// String's is a cell of that column.
    Rows { table: String, column: Option<String> },
}

/// A described variable (or every variable a pattern names).
#[derive(Debug, Clone)]
pub struct Described {
    /// Its name, or the glob that names them (`tdod_rental_*`).
    pub name: String,
    matcher: Option<Regex>,
    /// Its line in the file.
    pub line: usize,
    pub var_type: Option<VarType>,
    /// What it goes on (none: anything).
    pub on: Vec<Owner>,
    pub help: Option<String>,
    /// The values written, in order.
    pub values: Vec<Choice>,
    pub source: Option<Source>,
    /// A value of the builder's own is right too (the list is a help).
    pub other: bool,
    pub min: Option<f64>,
    pub max: Option<f64>,
    /// `runtime = true`: the scripts set it while the server runs (state,
    /// a test's hook), and it is never kept in a file: not offered, and
    /// one found in the module is said (`var-runtime`).
    pub runtime: bool,
}

impl PartialEq for Described {
    fn eq(&self, o: &Self) -> bool {
        (&self.name, self.line, self.var_type, &self.on, &self.help, &self.values)
            == (&o.name, o.line, o.var_type, &o.on, &o.help, &o.values)
            && (&self.source, self.other, self.min, self.max, self.runtime)
                == (&o.source, o.other, o.min, o.max, o.runtime)
    }
}

/// What finds the scripts and tables a description names.
pub trait Lookup {
    fn script(&self, name: &str) -> Option<Vec<u8>>;
    fn table(&self, name: &str) -> Option<TwoDa>;
}

/// A description's values, as [`Described::choices`] reads them.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Choices {
    pub list: Vec<Choice>,
    /// An Int's values are a table's rows: how many it has.
    pub rows: Option<usize>,
    /// What could not be read (the script or table is not there).
    pub problem: Option<String>,
}

impl Described {
    /// Whether it names many variables (a glob).
    pub fn is_pattern(&self) -> bool {
        self.matcher.is_some()
    }

    /// What the names a pattern takes begin with: each alternative's text
    /// up to its first wildcard (`tdod_rental_` of `tdod_rental_*`;
    /// `tdod_shop_` and `tdod_inn_` of `tdod_{shop,inn}_*`), each once,
    /// none empty. A name's, itself.
    pub fn beginnings(&self) -> Vec<String> {
        if !self.is_pattern() {
            return vec![self.name.clone()];
        }
        let mut all = alternatives(&self.name, 64);
        if all.is_empty() {
            all.push(self.name.clone());
        }
        let mut out: Vec<String> = Vec::new();
        for a in all {
            let cut = a.find(['*', '?', '[', '\\', '{']).unwrap_or(a.len());
            let b = a[..cut].to_string();
            if !b.is_empty() && !out.contains(&b) {
                out.push(b);
            }
        }
        out
    }

    /// Its name to quote beside a variable it takes: of a pattern with
    /// alternatives, the one alternative that takes it
    /// (`tdod_*_{d6,d20}` for `tdod_skill_d20`: `tdod_*_d20`); and no
    /// longer than a line, as a pattern may be hundreds of characters.
    pub fn quoted_for(&self, name: &str) -> String {
        const LONGEST: usize = 80;
        let taken = alternatives(&self.name, 4096)
            .into_iter()
            .find(|a| glob(a).is_ok_and(|rx| rx.is_match(name)));
        let shown = taken.unwrap_or_else(|| self.name.clone());
        match shown.char_indices().nth(LONGEST - 1) {
            Some((at, _)) => format!("{}…", &shown[..at]),
            None => shown,
        }
    }
}

/// A pattern's alternatives, each `{a,b}` group taken apart (groups don't
/// nest; inside `[...]` a brace is a character): none when there are more
/// than `most`, or no group.
fn alternatives(pattern: &str, most: usize) -> Vec<String> {
    let mut out = vec![String::new()];
    let mut grouped = false;
    let mut chars = pattern.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '[' => {
                let mut set = String::from('[');
                for c in chars.by_ref() {
                    set.push(c);
                    if c == ']' && set.len() > 2 {
                        break;
                    }
                }
                out.iter_mut().for_each(|o| o.push_str(&set));
            }
            '{' => {
                let mut group = String::new();
                for c in chars.by_ref() {
                    if c == '}' {
                        break;
                    }
                    group.push(c);
                }
                let options: Vec<&str> = group.split(',').collect();
                if out.len() * options.len() > most {
                    return Vec::new();
                }
                out = out
                    .iter()
                    .flat_map(|o| options.iter().map(move |a| format!("{o}{a}")))
                    .collect();
                grouped = true;
            }
            '\\' => {
                let next = chars.next();
                out.iter_mut().for_each(|o| {
                    o.push('\\');
                    o.extend(next);
                });
            }
            c => out.iter_mut().for_each(|o| o.push(c)),
        }
    }
    if grouped { out } else { Vec::new() }
}

impl Described {
    /// Whether it describes a variable so named (in that case only).
    pub fn matches(&self, name: &str) -> bool {
        match &self.matcher {
            Some(m) => m.is_match(name),
            None => self.name == name,
        }
    }

    /// Whether it goes on what `owner` is.
    pub fn belongs_on(&self, owner: Owner) -> bool {
        self.on.is_empty() || self.on.contains(&owner)
    }

    /// Whether it lists its values (to choose from).
    pub fn has_list(&self) -> bool {
        !self.values.is_empty() || self.source.is_some()
    }

    /// Its values: those written, then its script's constants or its
    /// table's rows.
    pub fn choices(&self, lookup: &dyn Lookup) -> Choices {
        let mut c = Choices { list: self.values.clone(), ..Default::default() };
        match &self.source {
            None => {}
            Some(Source::Constants { script, prefix }) => match lookup.script(script) {
                None => c.problem = Some(format!("the script {script} was not found")),
                Some(source) => {
                    let text = self.var_type == Some(VarType::String);
                    let (found, _) = crate::table_desc::constants(&source, prefix, text);
                    for e in found {
                        let crate::table_desc::Entry { name, value } = e;
                        if !c.list.iter().any(|x| x.value == value) {
                            c.list.push(Choice { value, note: Some(name) });
                        }
                    }
                }
            },
            Some(Source::Rows { table, column }) => match lookup.table(table) {
                None => c.problem = Some(format!("the table {table}.2da was not found")),
                Some(t) => {
                    let label = column.clone().or_else(|| {
                        let has = |c: &str| t.columns().iter().any(|x| x.eq_ignore_ascii_case(c));
                        ["Label", "LABEL", "Name"].into_iter().find(|c| has(c)).map(String::from)
                    });
                    // (A String's value is a cell: of its column, else
                    // the row's own label.)
                    let string = self.var_type == Some(VarType::String);
                    let column = match (column, &label) {
                        (None, Some(l)) if string => Some(l),
                        _ => column.as_ref(),
                    };
                    match column {
                        None if string => {
                            c.problem = Some(format!(
                                "{table}.2da has no Label column: a String's values are a \
                                 column's cells (column = \"…\")"
                            ));
                        }
                        Some(col) if !t.columns().iter().any(|x| x.eq_ignore_ascii_case(col)) => {
                            c.problem = Some(format!("{table}.2da has no column {col}"));
                        }
                        Some(col) => {
                            for row in 0..t.len() {
                                if let Some(v) = t.get(row, col)
                                    && !c.list.iter().any(|x| x.value == v)
                                {
                                    let note = Some(format!("row {row}"));
                                    c.list.push(Choice { value: v.to_string(), note });
                                }
                            }
                        }
                        None => {
                            c.rows = Some(t.len());
                            for row in 0..t.len() {
                                let note = label.as_deref().and_then(|l| t.get(row, l));
                                if let Some(note) = note {
                                    c.list.push(Choice {
                                        value: row.to_string(),
                                        note: Some(note.to_string()),
                                    });
                                }
                            }
                        }
                    }
                }
            },
        }
        c
    }

    /// What is wrong with a value of it (as text, of its type), by its
    /// range and its values; none where it is right, or where its values
    /// could not be read.
    pub fn value_problem(&self, value: &str, choices: &Choices) -> Option<String> {
        let number = match self.var_type {
            Some(VarType::Int) => whole(value).map(|v| v as f64),
            Some(VarType::Float) => value.trim().parse::<f64>().ok(),
            _ => None,
        };
        if let Some(n) = number {
            let words = |v: f64| {
                if self.var_type == Some(VarType::Int) {
                    (v as i64).to_string()
                } else {
                    v.to_string()
                }
            };
            if let Some(min) = self.min.filter(|m| n < *m) {
                return Some(format!("{value} is less than {}, the least it takes", words(min)));
            }
            if let Some(max) = self.max.filter(|m| n > *m) {
                return Some(format!("{value} is more than {}, the most it takes", words(max)));
            }
        }
        if self.other || choices.problem.is_some() || !self.has_list() {
            return None;
        }
        // (A table's rows: any row it has, named or not.)
        if let (Some(rows), Some(n)) = (choices.rows, number) {
            if n >= 0.0 && (n as usize) < rows {
                return None;
            }
            let Some(Source::Rows { table, .. }) = &self.source else { return None };
            return Some(format!("{value} is no row of {table}.2da ({rows} rows)"));
        }
        let same = |c: &Choice| match number {
            Some(n) => match self.var_type {
                Some(VarType::Int) => whole(&c.value).is_some_and(|v| v as f64 == n),
                _ => c.value.trim().parse::<f64>().is_ok_and(|v| v == n),
            },
            None => c.value == value,
        };
        if choices.list.iter().any(same) {
            return None;
        }
        let near = choices.list.iter().find(|c| c.value.eq_ignore_ascii_case(value.trim()));
        let listed: Vec<&str> = choices.list.iter().take(12).map(|c| c.value.as_str()).collect();
        let more = if choices.list.len() > listed.len() { ", …" } else { "" };
        Some(match near {
            Some(c) => format!(
                "\"{value}\" is none of its values: \"{}\" (the scripts compare it as it is \
                 written, case and all)",
                c.value
            ),
            None => format!(
                "\"{value}\" is none of its values ({}{more})",
                listed.iter().map(|v| format!("\"{v}\"")).collect::<Vec<_>>().join(", ")
            ),
        })
    }

    /// What it is, for people: "a String, on a placeable".
    pub fn words(&self) -> String {
        let mut s = match self.var_type {
            Some(t) => format!("{} {}", article(t), type_label(t)),
            None => "a variable of any type".to_string(),
        };
        match (self.min, self.max) {
            (Some(a), Some(b)) => s.push_str(&format!(", {a} to {b}")),
            (Some(a), None) => s.push_str(&format!(", at least {a}")),
            (None, Some(b)) => s.push_str(&format!(", at most {b}")),
            (None, None) => {}
        }
        if !self.on.is_empty() {
            let on: Vec<&str> = self.on.iter().map(|o| o.name()).collect();
            s.push_str(&format!(", on {} {}", article_of(on[0]), or_list(&on)));
        }
        if self.runtime {
            s.push_str(", set by the scripts while the server runs, never in a file");
        }
        s
    }
}

/// A type as the Variables window names it: `Int`, `String`.
pub fn type_label(t: VarType) -> &'static str {
    match t {
        VarType::Int => "Int",
        VarType::Float => "Float",
        VarType::String => "String",
        VarType::Object => "Object",
        VarType::Location => "Location",
    }
}

fn article(t: VarType) -> &'static str {
    if t == VarType::Int { "an" } else { "a" }
}

fn article_of(word: &str) -> &'static str {
    if word.starts_with(['a', 'e', 'i', 'o', 'u']) { "an" } else { "a" }
}

fn or_list(words: &[&str]) -> String {
    match words {
        [] => String::new(),
        [one] => one.to_string(),
        [rest @ .., last] => format!("{} or {last}", rest.join(", ")),
    }
}

/// A project's described variables (`variables.toml`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Variables {
    /// In the file's order.
    pub described: Vec<Described>,
    /// `undescribed`: globs of names that are to be described, so that a
    /// variable named so and described nowhere is said (a typo of a
    /// described one, most likely).
    pub undescribed: Vec<String>,
    undescribed_rx: Vec<RegexEq>,
    /// The file's name, as messages say it (none: [`FILE_NAME`]).
    file: Option<String>,
}

/// A regular expression compared by its text.
#[derive(Debug, Clone)]
struct RegexEq(Regex);

impl PartialEq for RegexEq {
    fn eq(&self, o: &Self) -> bool {
        self.0.as_str() == o.0.as_str()
    }
}

/// A glob, matched in its case only (variables' names are).
fn glob(g: &str) -> Result<Regex, String> {
    let rx = crate::nasher::glob::to_regex(g, false)?;
    Regex::new(&rx).map_err(|e| format!("{g}: {e}"))
}

/// What may be wrong with a variable's name, against the descriptions.
#[derive(Debug, Clone, PartialEq)]
pub enum Unknown<'a> {
    /// Described, but in another case: the game tells them apart.
    OtherCase(&'a Described),
    /// Named as the project's are (`undescribed`) and described nowhere:
    /// perhaps a typo of the nearest described name.
    Undescribed { pattern: &'a str, nearest: Option<&'a str> },
}

impl Variables {
    pub fn is_empty(&self) -> bool {
        self.described.is_empty() && self.undescribed.is_empty()
    }

    /// The file's name, as messages say it: `variables.toml`, or beside a
    /// `.mod` the module's (`krezk.variables.toml`).
    pub fn file(&self) -> &str {
        self.file.as_deref().unwrap_or(FILE_NAME)
    }

    /// The description of a variable so named: of that very name, else
    /// the first pattern that matches it.
    pub fn describe(&self, name: &str) -> Option<&Described> {
        let exact = self.described.iter().find(|d| !d.is_pattern() && d.name == name);
        exact.or_else(|| self.described.iter().find(|d| d.is_pattern() && d.matches(name)))
    }

    /// The names (not patterns) that may go on `owner`, in the file's
    /// order: those the Variables window offers (never one only the
    /// scripts set, `runtime`).
    pub fn names_for(&self, owner: Option<Owner>) -> impl Iterator<Item = &Described> {
        self.described
            .iter()
            .filter(move |d| !d.is_pattern() && !d.runtime && owner.is_none_or(|o| d.belongs_on(o)))
    }

    /// The patterns that may go on `owner`, in the file's order: those the
    /// Variables window offers after the names, by what the names they
    /// take begin with ([`Described::beginnings`]; never one only the
    /// scripts set).
    pub fn patterns_for(&self, owner: Option<Owner>) -> impl Iterator<Item = &Described> {
        self.described
            .iter()
            .filter(move |d| d.is_pattern() && !d.runtime && owner.is_none_or(|o| d.belongs_on(o)))
    }

    /// What is wrong with an undescribed variable's name, if anything.
    pub fn unknown(&self, name: &str) -> Option<Unknown<'_>> {
        if self.describe(name).is_some() {
            return None;
        }
        let other = self.described.iter().find(|d| {
            !d.is_pattern() && d.name.eq_ignore_ascii_case(name)
                || d.is_pattern() && d.matches(&name.to_ascii_lowercase())
        });
        if let Some(d) = other {
            return Some(Unknown::OtherCase(d));
        }
        let at = self.undescribed_rx.iter().position(|r| r.0.is_match(name))?;
        let nearest = self.nearest(name, |_| true).map(|d| d.name.as_str());
        Some(Unknown::Undescribed { pattern: &self.undescribed[at], nearest })
    }

    /// The described name nearest to `name`, of those `keep` takes, where
    /// it is near enough to be a typo of it (a letter or two added,
    /// dropped, changed or swapped), and not `name` itself: a name, or a
    /// builder's glob (not one of the scripts', `runtime`) that a name so
    /// near would match (`tdod_rentl_term`, for `tdod_rental_*`). Of two
    /// as near, a name.
    pub fn nearest(&self, name: &str, keep: impl Fn(&Described) -> bool) -> Option<&Described> {
        let most = 2.max(name.len() / 5);
        self.described
            .iter()
            .filter(|d| keep(d))
            .filter_map(|d| match &d.matcher {
                None => (d.name != name).then(|| (distance(&d.name, name), d)),
                // (A glob held to little, `*_x`, is near too much.)
                Some(_) if d.runtime => None,
                Some(_) => {
                    let (n, fixed) = glob_distance(&d.name, name)?;
                    (n > 0 && n * 3 <= fixed).then_some((n, d))
                }
            })
            .filter(|(n, _)| *n <= most)
            .min_by_key(|(n, d)| (*n, d.is_pattern(), d.name.as_str()))
            .map(|(_, d)| d)
    }

    /// For a variable a `runtime = true` pattern takes (one the scripts
    /// set), a name of its own entry, or a builder's glob, that it is
    /// near: a typo of that one, most likely a builder's (`tdod_lore_txt`
    /// for `tdod_lore_text`, which `tdod_lore_*` takes; `tdod_rentl_term`
    /// for `tdod_rental_*`).
    pub fn runtime_typo(&self, name: &str) -> Option<&Described> {
        let desc = self.describe(name)?;
        if !desc.runtime || !desc.is_pattern() {
            return None;
        }
        self.nearest(name, |_| true)
    }

    /// Reads a file's text into these (`name`: as messages give it):
    /// warnings and errors by line.
    pub fn apply(
        &mut self,
        name: &str,
        source: &str,
        warnings: &mut Vec<String>,
    ) -> Result<(), String> {
        let table = DeTable::parse(source).map_err(|e| {
            let line = e.span().map_or(1, |s| line_of(source, s.start));
            format!("{name}:{line}: {}", e.message().trim())
        })?;
        let mut r = Reader { name, source, warnings: Vec::new() };
        self.file = (name != FILE_NAME).then(|| name.to_string());
        for entry in in_order(table.get_ref()) {
            let (key, value) = entry;
            let at = key.span().start;
            match value.get_ref() {
                DeValue::Table(t) => {
                    // (A name twice is no TOML: an error before this.)
                    if let Some(d) = described(&mut r, key.get_ref(), at, t) {
                        self.described.push(d);
                    }
                }
                DeValue::Array(items) if key.get_ref() == "undescribed" => {
                    for item in items.iter() {
                        match item.get_ref() {
                            DeValue::String(s) => match glob(s) {
                                Ok(rx) => {
                                    self.undescribed.push(s.to_string());
                                    self.undescribed_rx.push(RegexEq(rx));
                                }
                                Err(e) => r.warn(item.span().start, e),
                            },
                            _ => r.warn(item.span().start, "undescribed: not text"),
                        }
                    }
                }
                _ if key.get_ref() == "undescribed" => {
                    r.warn(value.span().start, "undescribed is to be a list of names: [\"tdod_*\"]")
                }
                _ => r.warn(
                    at,
                    format!(
                        "\"{}\" is neither a variable ([{}]) nor undescribed = [...]",
                        key.get_ref(),
                        key.get_ref()
                    ),
                ),
            }
        }
        warnings.append(&mut r.warnings);
        Ok(())
    }
}

/// The edit distance of two names (a typo's: letters added, dropped,
/// changed or swapped).
fn distance(a: &str, b: &str) -> usize {
    let (a, b): (Vec<char>, Vec<char>) = (a.chars().collect(), b.chars().collect());
    let mut d = vec![vec![0usize; b.len() + 1]; a.len() + 1];
    for (i, row) in d.iter_mut().enumerate() {
        row[0] = i;
    }
    for (j, cell) in d[0].iter_mut().enumerate() {
        *cell = j;
    }
    for i in 1..=a.len() {
        for j in 1..=b.len() {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            d[i][j] = (d[i - 1][j] + 1).min(d[i][j - 1] + 1).min(d[i - 1][j - 1] + cost);
            if i > 1 && j > 1 && a[i - 1] == b[j - 2] && a[i - 2] == b[j - 1] {
                d[i][j] = d[i][j].min(d[i - 2][j - 2] + 1);
            }
        }
    }
    d[a.len()][b.len()]
}

/// One place of a glob, for [`glob_distance`].
#[derive(Debug, Clone, PartialEq)]
enum Place {
    /// A character, `?` (none: any) or a set (`[abc]`, `[!abc]`).
    One(Option<char>),
    Set(Vec<char>, bool),
    /// `*`: any characters, none too.
    Any,
}

impl Place {
    fn takes(&self, c: char) -> bool {
        match self {
            Place::One(None) => true,
            Place::One(Some(x)) => *x == c,
            Place::Set(set, not) => set.contains(&c) != *not,
            Place::Any => true,
        }
    }
}

/// A glob's places (one alternative of it: no `{a,b}` group).
fn places(glob: &str) -> Vec<Place> {
    let mut out = Vec::new();
    let mut chars = glob.chars().peekable();
    while let Some(c) = chars.next() {
        out.push(match c {
            '\\' => Place::One(chars.next()),
            '*' => {
                while chars.peek() == Some(&'*') {
                    chars.next();
                }
                Place::Any
            }
            '?' => Place::One(None),
            '[' => {
                // (`[!a]` is the negation; `[^a]` takes `^`, as the
                // glob's regex has it.)
                let not = chars.next_if(|c| *c == '!').is_some();
                let mut set: Vec<char> = Vec::new();
                while let Some(c) = chars.next() {
                    if c == ']' && !set.is_empty() {
                        break;
                    }
                    if c == '-'
                        && let (Some(&from), Some(&to)) = (set.last(), chars.peek())
                        && to != ']'
                    {
                        chars.next();
                        set.extend((from..=to).skip(1));
                        continue;
                    }
                    set.push(c);
                }
                Place::Set(set, not)
            }
            c => Place::One(Some(c)),
        });
    }
    out
}

/// How few typo's edits (a letter added, dropped, changed or two
/// swapped) make `name` one that `glob` matches, and how many characters
/// the glob holds a name to (its places but `*`): of the alternative
/// nearest, none where it has too many alternatives to try.
fn glob_distance(glob: &str, name: &str) -> Option<(usize, usize)> {
    let mut alts = alternatives(glob, 64);
    if alts.is_empty() {
        if glob.contains('{') {
            return None;
        }
        alts.push(glob.to_string());
    }
    let b: Vec<char> = name.chars().collect();
    alts.iter()
        .map(|alt| {
            let a = places(alt);
            let fixed = a.iter().filter(|p| **p != Place::Any).count();
            let mut d = vec![vec![0usize; b.len() + 1]; a.len() + 1];
            d[0] = (0..=b.len()).collect();
            for i in 1..=a.len() {
                let any = a[i - 1] == Place::Any;
                d[i][0] = d[i - 1][0] + usize::from(!any);
                for j in 1..=b.len() {
                    d[i][j] = if any {
                        d[i - 1][j].min(d[i][j - 1])
                    } else {
                        let cost = usize::from(!a[i - 1].takes(b[j - 1]));
                        let mut n =
                            (d[i - 1][j] + 1).min(d[i][j - 1] + 1).min(d[i - 1][j - 1] + cost);
                        if i > 1
                            && j > 1
                            && a[i - 2] != Place::Any
                            && a[i - 1].takes(b[j - 2])
                            && a[i - 2].takes(b[j - 1])
                        {
                            n = n.min(d[i - 2][j - 2] + 1);
                        }
                        n
                    };
                }
            }
            (d[a.len()][b.len()], fixed)
        })
        .min()
}

/// A value written in the file, as a variable's value is written (a
/// number as a number, text as it is).
fn value_text(v: &DeValue) -> Option<String> {
    match v {
        DeValue::String(s) => Some(s.to_string()),
        DeValue::Integer(i) => Some(i.to_string()),
        DeValue::Float(f) => Some(f.to_string()),
        _ => None,
    }
}

fn number(v: &DeValue) -> Option<f64> {
    match v {
        DeValue::Integer(i) => i.to_string().parse().ok(),
        DeValue::Float(f) => f.to_string().parse().ok(),
        _ => None,
    }
}

/// One variable's description, from its table (`[name]`).
fn described(r: &mut Reader, name: &str, at: usize, t: &DeTable) -> Option<Described> {
    let matcher = if crate::nasher::glob::has_magic(name) {
        match glob(name) {
            Ok(rx) => Some(rx),
            Err(e) => {
                r.warn(at, e);
                return None;
            }
        }
    } else {
        None
    };
    // (A pattern's own length says nothing of the names it takes: `*`
    // takes any. A name is held to what a builder can type.)
    if name.is_empty() {
        r.warn(at, "[\"\"]: a variable's name is never empty");
        return None;
    }
    if matcher.is_none() && name.chars().count() > NAME_MAX {
        r.warn(
            at,
            format!(
                "\"{name}\": a variable's name is at most {NAME_MAX} characters (Aurora's \
                 Variables window takes no more)"
            ),
        );
        return None;
    }
    let mut d = Described {
        name: name.to_string(),
        matcher,
        line: line_of(r.source, at),
        var_type: None,
        on: Vec::new(),
        help: None,
        values: Vec::new(),
        source: None,
        other: false,
        min: None,
        max: None,
        runtime: false,
    };
    let (mut script, mut prefix, mut table, mut column) = (None, None, None, None);
    for e in in_order(t) {
        let (k, v) = e;
        let at = v.span().start;
        match k.get_ref().as_ref() {
            "type" => {
                if let Some(t) = r.text(e) {
                    match VarType::from_name(&t) {
                        Some(t) => d.var_type = Some(t),
                        None => r.warn(
                            at,
                            format!(
                                "type \"{t}\": \"int\", \"float\", \"string\", \"object\" or \
                                 \"location\""
                            ),
                        ),
                    }
                }
            }
            "on" => match v.get_ref() {
                DeValue::Array(items) => {
                    for item in items.iter() {
                        let owner = match item.get_ref() {
                            DeValue::String(s) => Owner::from_name(s),
                            _ => None,
                        };
                        match owner {
                            Some(o) if !d.on.contains(&o) => d.on.push(o),
                            Some(_) => {}
                            None => r.warn(
                                item.span().start,
                                format!(
                                    "on: {}, not {}",
                                    or_list(&Owner::ALL.map(Owner::name)),
                                    value_text(item.get_ref()).unwrap_or_else(|| "that".into())
                                ),
                            ),
                        }
                    }
                }
                _ => r.warn(at, "on is to be a list: [\"placeable\", \"creature\"]"),
            },
            "help" => d.help = r.text(e).filter(|h| !h.trim().is_empty()),
            "values" => match v.get_ref() {
                DeValue::Array(items) => {
                    for item in items.iter() {
                        match value_text(item.get_ref()) {
                            Some(value) => d.values.push(Choice { value, note: None }),
                            None => r.warn(item.span().start, "values: a number or text"),
                        }
                    }
                }
                DeValue::Table(values) => {
                    for (value, note) in in_order(values) {
                        let note = match note.get_ref() {
                            DeValue::String(s) => Some(s.to_string()),
                            _ => {
                                r.warn(
                                    note.span().start,
                                    format!("values: {} = \"what it means\"", value.get_ref()),
                                );
                                None
                            }
                        };
                        d.values.push(Choice { value: value.get_ref().to_string(), note });
                    }
                }
                _ => r.warn(
                    at,
                    "values are written [\"forge\", \"tanner\"] or { forge = \"Smithing\" }",
                ),
            },
            "other" => match v.get_ref() {
                DeValue::Boolean(b) => d.other = *b,
                _ => r.warn(at, "other is true or false"),
            },
            "runtime" => match v.get_ref() {
                DeValue::Boolean(b) => d.runtime = *b,
                _ => r.warn(at, "runtime is true or false"),
            },
            "min" | "max" => match number(v.get_ref()) {
                Some(n) if k.get_ref() == "min" => d.min = Some(n),
                Some(n) => d.max = Some(n),
                None => r.warn(at, format!("{} is a number", k.get_ref())),
            },
            "constants" => script = r.text(e).map(|s| s.trim_end_matches(".nss").to_string()),
            "prefix" => prefix = r.text(e),
            "table" => table = r.text(e).map(|s| s.trim_end_matches(".2da").to_ascii_lowercase()),
            "column" => column = r.text(e),
            other => r.warn(
                k.span().start,
                format!("\"{other}\" is not a key of a variable's ([{name}])"),
            ),
        }
    }
    let line_at = at;
    d.source = match (script, table) {
        (Some(script), None) => {
            Some(Source::Constants { script, prefix: prefix.take().unwrap_or_default() })
        }
        (None, Some(table)) => Some(Source::Rows { table, column: column.take() }),
        (Some(_), Some(_)) => {
            r.warn(
                line_at,
                format!("{name}: its values are a script's constants or a table's rows, not both"),
            );
            None
        }
        (None, None) => None,
    };
    if prefix.is_some() && !matches!(d.source, Some(Source::Constants { .. })) {
        r.warn(line_at, format!("{name}: prefix is for constants = \"script\""));
    }
    if column.is_some() && !matches!(d.source, Some(Source::Rows { .. })) {
        r.warn(line_at, format!("{name}: column is for table = \"…\""));
    }
    let listed = d.has_list() || d.min.is_some() || d.max.is_some();
    match d.var_type {
        Some(VarType::Object | VarType::Location) if listed => {
            r.warn(line_at, format!("{name}: an object's or a location's value isn't listed (values, min, max, constants, table)"));
            d.values.clear();
            d.source = None;
            (d.min, d.max) = (None, None);
        }
        Some(VarType::String) if d.min.is_some() || d.max.is_some() => {
            r.warn(line_at, format!("{name}: min and max are for an int or a float"));
            (d.min, d.max) = (None, None);
        }
        Some(t @ (VarType::Int | VarType::Float)) => {
            let bad = d.values.iter().find(|c| match t {
                VarType::Int => whole(&c.value).is_none(),
                _ => c.value.trim().parse::<f64>().is_err(),
            });
            if let Some(c) = bad {
                r.warn(
                    line_at,
                    format!("{name}: the value \"{}\" is no {}", c.value, type_label(t)),
                );
            }
            if let (Some(Source::Rows { column: Some(_), .. }), VarType::Int) = (&d.source, t) {
                // (A row's cell is text; an Int's value is the row's number.)
                r.warn(
                    line_at,
                    format!("{name}: an Int's value is a row's number: column is for a String"),
                );
            }
        }
        _ => {}
    }
    // (One the scripts alone set is described for people: no type is
    // checked, as none is found in a file.)
    if d.var_type.is_none() && !d.runtime {
        r.warn(
            line_at,
            format!("{name} has no type: its type isn't checked (type = \"int\", \"string\"...)"),
        );
    }
    Some(d)
}

/// The scripts and tables a module's description names: the module's
/// own, the project's files, then the game's (and its haks, as `resman`
/// layers them).
pub struct ModuleLookup<'a> {
    pub module: &'a Module,
    pub resman: Option<&'a ResMan>,
    files: Option<crate::table_desc::Beside>,
}

impl std::fmt::Debug for ModuleLookup<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ModuleLookup").field("resman", &self.resman.is_some()).finish()
    }
}

impl<'a> ModuleLookup<'a> {
    pub fn new(module: &'a Module, resman: Option<&'a ResMan>) -> ModuleLookup<'a> {
        let root = module.location.as_ref().and_then(crate::project_file::ProjectFile::root_of);
        let files = root.map(crate::table_desc::Beside::of);
        ModuleLookup { module, resman, files }
    }

    fn get(&self, name: &str, t: ResType) -> Option<Vec<u8>> {
        let key = ResKey::new(ResRef::from_str(name).ok()?, t);
        if let Some(b) = self.module.get(&key) {
            return Some(b.to_vec());
        }
        self.resman?.get(&key).ok().map(|b| b.into_owned())
    }
}

impl Lookup for ModuleLookup<'_> {
    fn script(&self, name: &str) -> Option<Vec<u8>> {
        let own = self.files.as_ref().and_then(|f| f.script(name));
        own.or_else(|| self.get(name, ResType::NSS))
    }

    fn table(&self, name: &str) -> Option<TwoDa> {
        let own = self.files.as_ref().and_then(|f| f.table(name, Codepage::default()));
        own.or_else(|| TwoDa::parse(&self.get(name, ResType::TWODA)?, Codepage::default()).ok())
    }
}

/// One local variable found in the module: what it is on and where.
#[derive(Debug, Clone, PartialEq)]
pub struct Found {
    pub key: ResKey,
    /// The path of the object (or the resource's root) whose `VarTable`
    /// it is in: `/Placeable List[3]`.
    pub path: String,
    /// Its place in the `VarTable`.
    pub index: usize,
    pub owner: Owner,
    pub name: String,
    pub var_type: Option<VarType>,
    /// Its value as text (an Int's number, a String's text).
    pub value: String,
}

/// Every local variable of the module: the module's, the areas' (in
/// their GITs), placed objects', blueprints' and the items they hold,
/// stores' pages too.
pub fn module_variables(module: &Module) -> Vec<Found> {
    let mut keys: Vec<ResKey> =
        module.keys().filter(|k| Owner::of_restype(k.restype).is_some()).copied().collect();
    keys.sort();
    let mut out = Vec::new();
    for key in keys {
        let Some(Ok(gff)) = module.gff(&key) else { continue };
        out.extend(variables_of(key, &gff));
    }
    out
}

/// The local variables of one resource (`module.ifo`, a GIT, a
/// blueprint) and of what it holds.
pub fn variables_of(key: ResKey, gff: &mg_gff::Gff) -> Vec<Found> {
    let mut out = Vec::new();
    if let Some(owner) = Owner::of_restype(key.restype) {
        walk(key, &gff.root, owner, String::new(), &mut out);
    }
    out
}

/// The variables of a struct that is `owner`, and of those it holds.
fn walk(key: ResKey, s: &Struct, owner: Owner, path: String, out: &mut Vec<Found>) {
    if let Some(vars) = s.list("VarTable") {
        for (index, v) in vars.iter().enumerate() {
            let name = String::from_utf8_lossy(v.string("Name").unwrap_or_default()).into_owned();
            let var_type = v.integer("Type").and_then(VarType::of);
            let value = match v.get("Value") {
                Some(Value::Int(i)) => i.to_string(),
                Some(Value::Float(f)) => f.to_string(),
                Some(Value::String(b)) => String::from_utf8_lossy(b).into_owned(),
                Some(Value::Dword(d)) => d.to_string(),
                _ => String::new(),
            };
            out.push(Found { key, path: path.clone(), index, owner, name, var_type, value });
        }
    }
    for f in &s.fields {
        let label = f.label.to_string_lossy();
        let Value::List(items) = &f.value else { continue };
        let held = match label.as_ref() {
            // (An area's GIT: its objects' lists, its stores' among them.)
            l if owner == Owner::Area => match ObjectKind::from_list(l) {
                Some(kind) => Some(Owner::of_kind(kind)),
                None => continue,
            },
            "ItemList" | "Equip_ItemList" => Some(Owner::Item),
            // (A store's pages, which hold its items.)
            "StoreList" if owner == Owner::Store => None,
            _ => continue,
        };
        for (i, item) in items.iter().enumerate() {
            let at = format!("{path}/{label}[{i}]");
            match held {
                Some(o) => walk(key, item, o, at, out),
                None => {
                    if let Some(Value::List(goods)) = item.get("ItemList") {
                        for (j, g) in goods.iter().enumerate() {
                            walk(key, g, Owner::Item, format!("{at}/ItemList[{j}]"), out);
                        }
                    }
                }
            }
        }
    }
}

/// The file of described variables in a project's root.
pub fn path_in(root: &Path) -> PathBuf {
    root.join(FILE_NAME)
}

impl Variables {
    /// The module's variables (`found`, as [`module_variables`] lists
    /// them) that no description names, not even a pattern: each name
    /// once, in the order first found, with every place it is.
    pub fn undescribed_of<'a>(&self, found: &'a [Found]) -> Vec<(&'a str, Vec<&'a Found>)> {
        let mut out: Vec<(&str, Vec<&Found>)> = Vec::new();
        for f in found.iter().filter(|f| self.describe(&f.name).is_none()) {
            match out.iter_mut().find(|(n, _)| *n == f.name) {
                Some((_, places)) => places.push(f),
                None => out.push((&f.name, vec![f])),
            }
        }
        out
    }
}

/// Whether a glob (`tdod_*`, `tdod_{shop,inn}_*`) takes a variable's name,
/// in its case only, as a description's does; a glob that can't be read
/// takes its own text alone.
pub fn glob_takes(pattern: &str, name: &str) -> bool {
    glob(pattern).map_or(pattern == name, |rx| rx.is_match(name))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The module documentation's example.
    const EXAMPLE: &str = r#"
undescribed = ["tdod_*"]

[tdod_workbench]
type = "string"
on = ["placeable"]
values = { forge = "Smithing", tanner = "Leatherworking" }
help = "The craft the bench serves, within 5 m"

[tdod_lore_dc]
type = "int"
min = 1
max = 60

[tdod_kind]
type = "string"
constants = "tdod_kind_inc"
prefix = "TDOD_KIND_"

[tdod_class]
type = "int"
table = "classes"

["tdod_rental_*"]
type = "string"
on = ["creature", "placeable"]
"#;

    fn read(source: &str) -> (Variables, Vec<String>) {
        let mut v = Variables::default();
        let mut warnings = Vec::new();
        v.apply(FILE_NAME, source, &mut warnings).unwrap();
        (v, warnings)
    }

    struct Fake;

    impl Lookup for Fake {
        fn script(&self, name: &str) -> Option<Vec<u8>> {
            (name == "tdod_kind_inc").then(|| {
                b"const string TDOD_KIND_BEAST = \"beast\";\n\
                  const string TDOD_KIND_UNDEAD = \"undead\";\n\
                  const string OTHER = \"no\";\n"
                    .to_vec()
            })
        }

        fn table(&self, name: &str) -> Option<TwoDa> {
            let text: &[u8] = match name {
                "classes" => {
                    b"2DA V2.0\n\n   Label Key\n0  Barbarian barb\n1  ****  ****\n2  Bard bard\n"
                }
                "keys" => b"2DA V2.0\n\n   Key\n0  barb\n",
                _ => return None,
            };
            TwoDa::parse(text, Codepage::default()).ok()
        }
    }

    #[test]
    fn the_example_reads_whole() {
        let (v, warnings) = read(EXAMPLE);
        assert_eq!(warnings, Vec::<String>::new());
        assert_eq!(v.undescribed, ["tdod_*"]);
        let names: Vec<&str> = v.described.iter().map(|d| d.name.as_str()).collect();
        assert_eq!(
            names,
            ["tdod_workbench", "tdod_lore_dc", "tdod_kind", "tdod_class", "tdod_rental_*"]
        );
        let bench = v.describe("tdod_workbench").unwrap();
        assert_eq!(bench.line, 4);
        assert_eq!(bench.var_type, Some(VarType::String));
        assert_eq!(bench.on, [Owner::Placeable]);
        assert_eq!(bench.help.as_deref(), Some("The craft the bench serves, within 5 m"));
        assert_eq!(
            bench.values,
            [
                Choice { value: "forge".into(), note: Some("Smithing".into()) },
                Choice { value: "tanner".into(), note: Some("Leatherworking".into()) },
            ]
        );
        assert_eq!(bench.words(), "a String, on a placeable");
        assert_eq!(v.describe("tdod_lore_dc").unwrap().words(), "an Int, 1 to 60");
        let rental = v.describe("tdod_rental_door").unwrap();
        assert!(rental.is_pattern() && rental.belongs_on(Owner::Creature));
        assert!(!rental.belongs_on(Owner::Door));
        assert!(v.describe("TDOD_RENTAL_DOOR").is_none(), "names are matched in their case");
        let offered: Vec<&str> =
            v.names_for(Some(Owner::Creature)).map(|d| d.name.as_str()).collect();
        assert_eq!(offered, ["tdod_lore_dc", "tdod_kind", "tdod_class"], "no patterns");
        let patterns: Vec<&str> =
            v.patterns_for(Some(Owner::Creature)).map(|d| d.name.as_str()).collect();
        assert_eq!(patterns, ["tdod_rental_*"]);
        assert!(v.patterns_for(Some(Owner::Door)).next().is_none());
        assert_eq!(rental.beginnings(), ["tdod_rental_"]);
        assert_eq!(bench.beginnings(), ["tdod_workbench"]);
    }

    /// A pattern's names begin with each alternative's text up to its
    /// first wildcard.
    #[test]
    fn a_pattern_s_names_begin_with_its_text_before_a_wildcard() {
        let mut v = Variables::default();
        let text = "[\"tdod_{shop,inn}_*\"]\ntype = \"int\"\n\n[\"tdod_*_{d6,d20}\"]\n\
                    type = \"int\"\n\n[\"*_count\"]\ntype = \"int\"\n";
        v.apply("variables.toml", text, &mut Vec::new()).unwrap();
        let begins: Vec<Vec<String>> = v.described.iter().map(Described::beginnings).collect();
        assert_eq!(begins[0], ["tdod_shop_", "tdod_inn_"]);
        assert_eq!(begins[1], ["tdod_"]);
        assert!(begins[2].is_empty(), "nothing to begin with");
    }

    /// The manual's example (Modules, the project file) reads whole.
    #[test]
    fn the_manual_s_example_reads_whole() {
        let manual = concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/manual/03-modules.md");
        // (A Windows checkout may give the manual CRLF line ends.)
        let text = std::fs::read_to_string(manual).unwrap().replace("\r\n", "\n");
        let section = &text[text.find("### Described variables").expect("the section")..];
        let start = section.find("```toml\n").expect("its example") + 8;
        let example = &section[start..start + section[start..].find("```").unwrap()];
        let (v, warnings) = read(example);
        assert_eq!(warnings, Vec::<String>::new());
        assert!(v.described.len() >= 8, "{}", v.described.len());
        for d in &v.described {
            let c = d.choices(&Fake);
            if d.name != "tdod_kind" {
                assert!(c.problem.is_none() || d.source.is_some(), "{}: {:?}", d.name, c.problem);
            }
        }
    }

    #[test]
    fn values_come_from_the_file_a_script_s_constants_and_a_table_s_rows() {
        let (v, _) = read(EXAMPLE);
        let kind = v.describe("tdod_kind").unwrap();
        let c = kind.choices(&Fake);
        let values: Vec<(&str, Option<&str>)> =
            c.list.iter().map(|c| (c.value.as_str(), c.note.as_deref())).collect();
        assert_eq!(
            values,
            [("beast", Some("TDOD_KIND_BEAST")), ("undead", Some("TDOD_KIND_UNDEAD"))]
        );
        assert_eq!(kind.value_problem("beast", &c), None);
        assert_eq!(
            kind.value_problem("Beast", &c).unwrap(),
            "\"Beast\" is none of its values: \"beast\" (the scripts compare it as it is \
             written, case and all)"
        );
        let class = v.describe("tdod_class").unwrap();
        let c = class.choices(&Fake);
        assert_eq!(c.rows, Some(3));
        let values: Vec<(&str, Option<&str>)> =
            c.list.iter().map(|c| (c.value.as_str(), c.note.as_deref())).collect();
        assert_eq!(values, [("0", Some("Barbarian")), ("2", Some("Bard"))], "rows with a label");
        assert_eq!(class.value_problem("2", &c), None);
        assert_eq!(class.value_problem("1", &c), None, "a row, though unnamed");
        assert_eq!(class.value_problem("3", &c).unwrap(), "3 is no row of classes.2da (3 rows)");
        // A String of a column's cells.
        let (v, w) = read("[k]\ntype = \"string\"\ntable = \"classes\"\ncolumn = \"Key\"\n");
        assert_eq!(w, Vec::<String>::new());
        let k = v.describe("k").unwrap();
        let c = k.choices(&Fake);
        let values: Vec<&str> = c.list.iter().map(|c| c.value.as_str()).collect();
        assert_eq!(values, ["barb", "bard"]);
        assert!(k.value_problem("bard", &c).is_none() && k.value_problem("monk", &c).is_some());
        // Without one, the row's own label.
        let (v, w) = read("[k]\ntype = \"string\"\ntable = \"classes\"\n");
        assert_eq!(w, Vec::<String>::new());
        let k = v.describe("k").unwrap();
        let c = k.choices(&Fake);
        let values: Vec<(&str, Option<&str>)> =
            c.list.iter().map(|c| (c.value.as_str(), c.note.as_deref())).collect();
        assert_eq!(values, [("Barbarian", Some("row 0")), ("Bard", Some("row 2"))]);
        assert_eq!(c.rows, None);
        assert_eq!(k.value_problem("Bard", &c), None);
        assert_eq!(
            k.value_problem("2", &c).unwrap(),
            "\"2\" is none of its values (\"Barbarian\", \"Bard\")"
        );
        // (A table with no label: said, and nothing is wrong by it.)
        let (v, _) = read("[k]\ntype = \"string\"\ntable = \"keys\"\n");
        let k = v.describe("k").unwrap();
        let c = k.choices(&Fake);
        assert_eq!(
            c.problem.as_deref(),
            Some(
                "keys.2da has no Label column: a String's values are a column's cells (column = \
                 \"…\")"
            )
        );
        assert_eq!(k.value_problem("anything", &c), None);
        // What isn't there is said, and nothing is wrong by it.
        let (v, _) = read("[k]\ntype = \"int\"\ntable = \"nothing\"\n");
        let k = v.describe("k").unwrap();
        let c = k.choices(&Fake);
        assert_eq!(c.problem.as_deref(), Some("the table nothing.2da was not found"));
        assert_eq!(k.value_problem("12", &c), None);
        // Floats, and `other`.
        let (v, _) = read("[f]\ntype = \"float\"\nmin = 0.5\nvalues = [1.0, 2.5]\nother = true\n");
        let f = v.describe("f").unwrap();
        let c = f.choices(&Fake);
        assert_eq!(f.value_problem("2.50", &c), None);
        assert_eq!(f.value_problem("7", &c), None, "other: any");
        assert_eq!(
            f.value_problem("0.25", &c).unwrap(),
            "0.25 is less than 0.5, the least it takes"
        );
    }

    #[test]
    fn a_name_in_another_case_or_like_the_project_s_is_said() {
        let (v, _) = read(EXAMPLE);
        assert_eq!(v.unknown("tdod_workbench"), None);
        let Some(Unknown::OtherCase(d)) = v.unknown("TDOD_Workbench") else { panic!() };
        assert_eq!(d.name, "tdod_workbench");
        let Some(Unknown::OtherCase(d)) = v.unknown("TDOD_RENTAL_DOOR") else { panic!() };
        assert_eq!(d.name, "tdod_rental_*");
        assert_eq!(
            v.unknown("tdod_wrokbench"),
            Some(Unknown::Undescribed { pattern: "tdod_*", nearest: Some("tdod_workbench") })
        );
        assert_eq!(
            v.unknown("tdod_something"),
            Some(Unknown::Undescribed { pattern: "tdod_*", nearest: None })
        );
        assert_eq!(v.unknown("nw_other"), None, "not the project's");
        assert_eq!(distance("tdod_lit", "tdod_lti"), 1);
    }

    /// `mg vars --undescribed`: the module's variables no description
    /// names, each name once with its places.
    #[test]
    fn the_module_s_undescribed_variables_are_listed_by_name() {
        let (v, _) = read(EXAMPLE);
        let at = |name: &str, path: &str| Found {
            key: ResKey::parse("tavern", ResType::GIT).unwrap(),
            path: path.into(),
            index: 0,
            owner: Owner::Placeable,
            name: name.into(),
            var_type: Some(VarType::Int),
            value: "1".into(),
        };
        let found = [
            at("tdod_workbench", "/Placeable List[0]"),
            at("nw_other", "/Placeable List[0]"),
            at("tdod_rental_door", "/Placeable List[1]"),
            at("nw_other", "/Placeable List[2]"),
            at("TDOD_Workbench", "/Placeable List[2]"),
        ];
        let listed: Vec<(&str, usize)> =
            v.undescribed_of(&found).into_iter().map(|(n, at)| (n, at.len())).collect();
        assert_eq!(listed, [("nw_other", 2), ("TDOD_Workbench", 1)]);
        assert!(glob_takes("nw_*", "nw_other") && !glob_takes("nw_*", "NW_other"));
        assert!(glob_takes("tdod_{a,b}", "tdod_b") && glob_takes("[", "["));
    }

    /// A pattern quoted beside a variable it takes is the one alternative
    /// that takes it, and a line at most (the Domains of Dread's note on
    /// 2.2.0's long patterns: the whole, of 400 characters, was quoted).
    #[test]
    fn a_pattern_is_quoted_by_the_alternative_that_takes_the_name() {
        let long: Vec<String> = (0..40).map(|i| format!("part{i}")).collect();
        let group = format!("{{{}}}", long.join(","));
        let source = format!(
            "[\"tdod_*_{{d6,d10,d20,d100}}{{,_again}}\"]\nruntime = true\n\
             [\"tdod_[ab]{{x,y}}\"]\nruntime = true\n\
             [\"tdod_{}_*\"]\nruntime = true\n\
             [\"tdod_long_*_{}\"]\nruntime = true\n",
            group,
            "z".repeat(90)
        );
        let (v, w) = read(&source);
        assert!(w.is_empty(), "{w:?}");
        let quoted = |name: &str| v.describe(name).unwrap().quoted_for(name);
        assert_eq!(quoted("tdod_skill_d20_again"), "tdod_*_d20_again");
        assert_eq!(quoted("tdod_skill_d6"), "tdod_*_d6");
        assert_eq!(quoted("tdod_by"), "tdod_[ab]y");
        assert_eq!(quoted("tdod_part31_state"), "tdod_part31_*");
        let long = quoted(&format!("tdod_long_a_{}", "z".repeat(90)));
        assert_eq!((long.chars().count(), long.ends_with('…')), (80, true));
        assert_eq!(alternatives("tdod_*", 10), Vec::<String>::new(), "no group");
        assert_eq!(alternatives("{a,b}{c,d}", 3), Vec::<String>::new(), "too many");
    }

    /// The Domains of Dread's notes on their setup, 1: the scripts' own
    /// variables (state, a test's hook) are described as such, said by
    /// no `undescribed`, and offered nowhere.
    #[test]
    fn a_variable_the_scripts_alone_set_is_described_and_never_offered() {
        let (v, w) = read(
            "undescribed = [\"tdod_*\"]\n\
             [tdod_trodden]\nruntime = true\nhelp = \"Visited\"\n\
             [\"tdod_*_d20\"]\ntype = \"int\"\non = [\"module\"]\nruntime = true\n\
             [tdod_lit]\ntype = \"int\"\nruntime = false\n\
             [tdod_x]\ntype = \"int\"\nruntime = \"yes\"\n",
        );
        assert_eq!(w, ["variables.toml:14: runtime is true or false"], "no type needed");
        let trodden = v.describe("tdod_trodden").unwrap();
        assert!(trodden.runtime && trodden.var_type.is_none());
        assert_eq!(
            trodden.words(),
            "a variable of any type, set by the scripts while the server runs, never in a file"
        );
        assert!(v.describe("tdod_save_d20").unwrap().runtime);
        assert!(!v.describe("tdod_lit").unwrap().runtime && !v.describe("tdod_x").unwrap().runtime);
        assert_eq!(v.unknown("tdod_trodden"), None, "described: not said as a typo");
        assert_eq!(v.unknown("tdod_save_d20"), None);
        let offered: Vec<&str> = v.names_for(None).map(|d| d.name.as_str()).collect();
        assert_eq!(offered, ["tdod_lit", "tdod_x"]);
        let on_module: Vec<&str> =
            v.names_for(Some(Owner::Module)).map(|d| d.name.as_str()).collect();
        assert_eq!(on_module, ["tdod_lit", "tdod_x"]);
    }

    /// The Domains of Dread's notes on 2.2.0, 1: a pattern is not held to
    /// a name's length (their brace globs of the scripts' own were split
    /// into 58 entries of 64 characters or fewer); a name is, to what
    /// Aurora takes; and a typo of a builder's name that one of the
    /// scripts' patterns takes is said near it.
    #[test]
    fn a_pattern_is_as_long_as_it_needs_and_a_name_is_held_to_aurora_s() {
        let family = "tdod_{aberrant,ablative_stored,afflict_onset,amalgam,aoo_made,armor,\
                      ascetic_state,ascetic_unable,attacks_miss,attitude_ours,auric,\
                      backfire_chance,bag_on,balanced,bane,barter_gold,bastions,beat,\
                      bg_sees_all,bodied,bridge_why,bulk_size,butcher,charge,circle,\
                      claw_blows,cloaked,cm,companion,companion_of,conc_aoo,\
                      confused_turned_on,controlled_hd,conviction_less,countered,\
                      cutbeyond_left,cvc_with,lore_*}";
        assert!(family.len() > 300, "{}", family.len());
        let long = format!("tdod_{}", "x".repeat(NAME_MAX - 5));
        let too_long = format!("{long}x");
        let (v, w) = read(&format!(
            "[tdod_lore_text]\ntype = \"string\"\n\n\
             [\"{family}\"]\nruntime = true\n\n\
             [{long}]\ntype = \"int\"\n\n\
             [{too_long}]\ntype = \"int\"\n\n\
             [\"\"]\ntype = \"int\"\n"
        ));
        assert_eq!(
            w,
            [
                format!(
                    "variables.toml:10: \"{too_long}\": a variable's name is at most 1024 \
                     characters (Aurora's Variables window takes no more)"
                ),
                "variables.toml:13: [\"\"]: a variable's name is never empty".to_string(),
            ]
        );
        for name in ["tdod_aberrant", "tdod_confused_turned_on", "tdod_cvc_with", "tdod_lore_seen"]
        {
            let d = v.describe(name).unwrap_or_else(|| panic!("{name}"));
            assert!(d.runtime && d.is_pattern(), "{name}");
        }
        assert!(!v.describe("tdod_lore_text").unwrap().runtime, "its own entry wins");
        assert_eq!(v.describe(&long).map(|d| d.name.len()), Some(NAME_MAX));
        assert!(v.describe(&too_long).is_none());
        assert_eq!(
            v.runtime_typo("tdod_lore_txt").map(|d| d.name.as_str()),
            Some("tdod_lore_text")
        );
        assert_eq!(v.runtime_typo("tdod_lore_seen"), None, "near no name");
        assert_eq!(v.runtime_typo("tdod_lore_text"), None, "its own entry");
    }

    /// A typo of a name a builder's glob describes, that a glob of the
    /// scripts' takes, is said with the builder's glob (the Domains of
    /// Dread's notes on 2.2.0); a glob holding a name to little, or one of
    /// the scripts' own, is near nothing.
    #[test]
    fn a_typo_of_a_name_of_a_builder_s_glob_is_said_with_the_glob() {
        let (v, _) = read(
            "[tdod_lore_text]\ntype = \"string\"\n\n\
             [\"tdod_rental_*\"]\ntype = \"string\"\n\n\
             [\"tdod_{d6,d20}_[abc]?\"]\ntype = \"int\"\n\n\
             [\"*_x\"]\ntype = \"int\"\n\n\
             [\"tdod_*\"]\nruntime = true\n",
        );
        let near = |name: &str| v.runtime_typo(name).map(|d| d.name.as_str());
        assert_eq!(v.describe("tdod_rentl_term").map(|d| d.runtime), Some(true));
        assert_eq!(near("tdod_rentl_term"), Some("tdod_rental_*"), "a letter dropped");
        assert_eq!(near("tdod_rnetal_term"), Some("tdod_rental_*"), "two swapped");
        assert_eq!(near("tdod_rental"), Some("tdod_rental_*"), "the _ dropped");
        assert_eq!(near("tdod_d20_z1"), Some("tdod_{d6,d20}_[abc]?"), "an alternative");
        assert_eq!(near("tdod_lore_txt"), Some("tdod_lore_text"), "a name first");
        assert_eq!(near("tdod_lore_seen"), None, "near no name");
        assert_eq!(near("tdod_x_y"), None, "*_x holds a name to too little");
        assert_eq!(glob_distance("tdod_rental_*", "tdod_rental_term"), Some((0, 12)));
        assert_eq!(glob_distance("[a-c]x", "bx"), Some((0, 2)));
        assert_eq!(glob_distance("[!a-c]x", "bx"), Some((1, 2)));
        // var-name's nearest takes the builders' globs too.
        let (v, _) = read(
            "undescribed = [\"tdod_*\"]\n\n[\"tdod_rental_*\"]\ntype = \"string\"\n\n\
             [\"tdod_*_d20\"]\nruntime = true\n",
        );
        assert_eq!(
            v.unknown("tdod_retnal_term"),
            Some(Unknown::Undescribed { pattern: "tdod_*", nearest: Some("tdod_rental_*") })
        );
        assert_eq!(
            v.unknown("tdod_skill_d02"),
            Some(Unknown::Undescribed { pattern: "tdod_*", nearest: None }),
            "never one of the scripts'"
        );
    }

    #[test]
    fn what_is_not_understood_is_said_by_line_and_the_rest_read() {
        let (v, w) = read(
            "undescribed = \"tdod_*\"\ncolour = 3\n[a]\ntype = \"integer\"\non = [\"house\", \"door\"]\n\
             size = 3\n[b]\ntype = \"object\"\nvalues = [1]\n[c]\ntype = \"int\"\nvalues = [\"x\"]\n\
             [d]\nconstants = \"s\"\ntable = \"t\"\n[e]\ntype = \"string\"\nmin = 1\nprefix = \"X\"\n\
             [\"[\"]\n",
        );
        assert_eq!(
            w,
            [
                "variables.toml:1: undescribed is to be a list of names: [\"tdod_*\"]",
                "variables.toml:2: \"colour\" is neither a variable ([colour]) nor undescribed = [...]",
                "variables.toml:4: type \"integer\": \"int\", \"float\", \"string\", \"object\" or \"location\"",
                "variables.toml:5: on: module, area, creature, door, encounter, item, placeable, sound, \
                 store, trigger or waypoint, not house",
                "variables.toml:6: \"size\" is not a key of a variable's ([a])",
                "variables.toml:3: a has no type: its type isn't checked (type = \"int\", \"string\"...)",
                "variables.toml:7: b: an object's or a location's value isn't listed (values, min, max, \
                 constants, table)",
                "variables.toml:10: c: the value \"x\" is no Int",
                "variables.toml:13: d: its values are a script's constants or a table's rows, not both",
                "variables.toml:13: d has no type: its type isn't checked (type = \"int\", \"string\"...)",
                "variables.toml:16: e: prefix is for constants = \"script\"",
                "variables.toml:16: e: min and max are for an int or a float",
                "variables.toml:20: [: missing ']'",
            ]
        );
        assert_eq!(v.describe("a").unwrap().on, [Owner::Door]);
        assert!(v.describe("b").unwrap().values.is_empty());
        let mut v = Variables::default();
        let e = v.apply(FILE_NAME, "[a]\ntype = \"int\n", &mut Vec::new()).unwrap_err();
        assert!(e.starts_with("variables.toml:2: "), "{e}");
    }

    #[test]
    fn every_cut_of_the_file_reads_without_a_panic() {
        for end in (0..=EXAMPLE.len()).filter(|&i| EXAMPLE.is_char_boundary(i)) {
            let mut v = Variables::default();
            let _ = v.apply(FILE_NAME, &EXAMPLE[..end], &mut Vec::new());
            for d in &v.described {
                let c = d.choices(&Fake);
                let _ = d.value_problem("1", &c);
                let _ = d.words();
            }
            let _ = v.unknown("tdod_x");
        }
        let mut v = Variables::default();
        let bytes = String::from_utf8_lossy(&[0xFF, b'[', 0, b'"', 0xC3]).into_owned();
        assert!(v.apply(FILE_NAME, &bytes, &mut Vec::new()).is_err());
    }

    #[test]
    fn every_object_s_variables_are_found_with_what_they_are_on() {
        use mg_gff::Gff;
        let var = |name: &str| {
            let mut v = Struct::new(0);
            v.set("Name", Value::String(name.as_bytes().to_vec()));
            v.set("Type", Value::Dword(1));
            v.set("Value", Value::Int(1));
            v
        };
        let holding = |id: u32, name: &str, list: &str, held: Vec<Struct>| {
            let mut s = Struct::new(id);
            s.set("VarTable", Value::List(vec![var(name)]));
            s.set(list, Value::List(held));
            s
        };
        let item = holding(0, "on_item", "ItemList", Vec::new());
        let page = holding(0, "page", "ItemList", vec![item.clone()]);
        let store = holding(11, "on_store", "StoreList", vec![page]);
        let keeper = holding(4, "on_creature", "Equip_ItemList", vec![item.clone()]);
        let mut git = Gff::new(*b"GIT ");
        git.root.set("VarTable", Value::List(vec![var("on_area")]));
        git.root.set("StoreList", Value::List(vec![store.clone()]));
        git.root.set("Creature List", Value::List(vec![keeper]));
        git.root.set("List", Value::List(vec![item]));
        let key = ResKey::new(ResRef::from_str("krezk").unwrap(), ResType::GIT);
        let found: Vec<(Owner, String, String)> =
            variables_of(key, &git).into_iter().map(|f| (f.owner, f.path, f.name)).collect();
        let o = |owner, path: &str, name: &str| (owner, path.to_string(), name.to_string());
        assert_eq!(
            found,
            [
                o(Owner::Area, "", "on_area"),
                o(Owner::Store, "/StoreList[0]", "on_store"),
                o(Owner::Item, "/StoreList[0]/StoreList[0]/ItemList[0]", "on_item"),
                o(Owner::Creature, "/Creature List[0]", "on_creature"),
                o(Owner::Item, "/Creature List[0]/Equip_ItemList[0]", "on_item"),
                o(Owner::Item, "/List[0]", "on_item"),
            ]
        );
        // A store's blueprint, its pages' goods.
        let mut utm = Gff::new(*b"UTM ");
        utm.root = store;
        let key = ResKey::new(ResRef::from_str("shop").unwrap(), ResType::UTM);
        let owners: Vec<Owner> = variables_of(key, &utm).into_iter().map(|f| f.owner).collect();
        assert_eq!(owners, [Owner::Store, Owner::Item]);
    }
}
