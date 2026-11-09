//! Palettes grouped more than one way (`docs/palette-groupings-proposal.md`,
//! design A): groupings of Moonglow's own, from rules in the project file,
//! that show a palette's blueprints under groups other than their
//! categories (TDoD's placeables by what each one is: Tables, Seats,
//! Food) without changing a palette file or copying a blueprint.
//!
//! ```toml
//! [palette.kind]                 # a grouping, its key the id; any number of them
//! title = "Kind"                 # shown in Group by (left out: the id, "Kind")
//! types = ["utp", "uti"]         # the palettes it is offered in; left out, every one
//! words_of = '^.*?:\s*(.*?)\s*(\(|$)'   # the part of a name its words come from
//! from = "palette_kinds.toml"    # lists a tool writes, beside moonglow.toml
//!
//! [[palette.kind.group]]         # in the order shown
//! name = "Tables"
//! types = ["utp"]                # only in these of the grouping's palettes
//! names = ["table", "desk", "dining table"]   # whole words, case and accents ignored
//! resrefs = ["dd_*_tbl*"]        # ResRef patterns, `*` any run, case ignored
//! ```
//!
//! The `from` file holds groups by name, each a list of ResRefs, alone (in
//! every palette the group is offered in) or with their type (that one
//! only): `"Tables" = ["dd_n2e_table2", "dd_din_fruitpapa.utp"]`. A group
//! it names that the project file doesn't is a group too, after those the
//! project file names, in the file's order.
//!
//! A blueprint goes under every group it falls in, by a rule or by a list;
//! those no group takes are Other's, shown last. What isn't understood (a
//! bad key, a pattern that isn't one, a `from` file missing or no TOML) is
//! a warning, and the grouping works without it.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use mg_core::{ResRef, ResType};
use regex_lite::Regex;
use toml::de::{DeTable, DeValue};

use crate::palette::{BlueprintKind, Palette, PaletteBlueprint, PaletteName, PaletteNode};
use crate::project_file::{Entry, Patterns, Reader, in_order};

/// What the blueprints no group takes are shown under, last.
pub const OTHER: &str = "Other";

/// The project's groupings (`[palette.*]`), in the file's order.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Groupings {
    pub list: Vec<Grouping>,
    /// What was not understood of them, or of their `from` files: also
    /// among the project file's warnings, and said by Verify.
    pub problems: Vec<String>,
    /// A hash of the groupings as read (their rules and their lists): a
    /// view made from them is made again when it changes.
    pub fingerprint: u64,
}

/// A way of grouping palettes' blueprints (`[palette.<id>]`).
#[derive(Debug, Clone, PartialEq)]
pub struct Grouping {
    /// Its key in the project file (`kind`).
    pub id: String,
    /// What Group by shows (`title`; left out, the id as a title).
    pub title: String,
    /// The palettes it is offered in (none: every one).
    pub types: Vec<BlueprintKind>,
    /// The part of a name its words come from (`words_of`).
    pub words_of: Option<WordsOf>,
    /// The file of lists a tool writes (`from`), beside the project file.
    pub from: Option<String>,
    /// Its groups, in the order shown: the project file's, then those
    /// only its `from` file names; with `order = "name"`, by name.
    pub groups: Vec<Group>,
    /// `order = "name"`: its groups sorted by name, declared and listed
    /// alike, as palettes sort names (case and accents aside); else
    /// (`"file"`) as the project file and its list give them.
    pub by_name: bool,
}

impl Grouping {
    /// Puts the groups in the order shown ([`Grouping::by_name`]): a
    /// stable sort, so groups of one name keep the order they came in.
    fn put_in_order(&mut self) {
        if self.by_name {
            self.groups.sort_by_cached_key(|g| crate::new::word_sort_key(&g.name));
        }
    }
}

/// `words_of`: an expression whose first capture group is the part of a
/// name its words are taken from. A name it doesn't match, or matches
/// without the group, gives its words whole.
#[derive(Debug, Clone)]
pub struct WordsOf {
    pub pattern: String,
    regex: Regex,
}

impl PartialEq for WordsOf {
    fn eq(&self, other: &WordsOf) -> bool {
        self.pattern == other.pattern
    }
}

impl WordsOf {
    pub fn new(pattern: &str) -> Result<WordsOf, String> {
        let regex = Regex::new(pattern).map_err(|e| e.to_string())?;
        Ok(WordsOf { pattern: pattern.to_string(), regex })
    }

    /// The part of `name` its words come from.
    pub fn of<'n>(&self, name: &'n str) -> &'n str {
        self.regex.captures(name).and_then(|c| c.get(1)).map_or(name, |m| m.as_str())
    }
}

/// A group of a grouping (`[[palette.<id>.group]]`, or a list of its
/// `from` file).
#[derive(Debug, Clone, PartialEq)]
pub struct Group {
    pub name: String,
    /// The grouping's palettes it is offered in (none: all of them).
    pub types: Vec<BlueprintKind>,
    /// `names`, as written: whole words of a blueprint's name.
    pub names: Vec<String>,
    /// Each of `names` as words, case and accents folded.
    phrases: Vec<Vec<String>>,
    /// `resrefs`: ResRef patterns.
    pub resrefs: Patterns,
    /// The blueprints its `from` file lists: a ResRef, and the type where
    /// one is given.
    pub listed: Vec<(ResRef, Option<BlueprintKind>)>,
    /// Named only by the `from` file.
    pub from_file: bool,
}

impl Group {
    fn named(name: String, from_file: bool) -> Group {
        Group {
            name,
            types: Vec::new(),
            names: Vec::new(),
            phrases: Vec::new(),
            resrefs: Patterns::default(),
            listed: Vec::new(),
            from_file,
        }
    }
}

/// A name's words, as groups match them: runs of letters and digits,
/// case and accents folded (`Dining-Table, Óak` is `dining table oak`).
pub fn words(text: &str) -> Vec<String> {
    mg_core::text::fold(text)
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(str::to_string)
        .collect()
}

fn kind_of_extension(ext: &str) -> Option<BlueprintKind> {
    let t = ResType::from_extension(ext.trim().trim_start_matches('.'))?;
    BlueprintKind::from_restype(t)
}

fn extensions() -> String {
    let all: Vec<&str> =
        BlueprintKind::ALL.iter().filter_map(|k| k.restype().extension()).collect();
    all.join(", ")
}

impl Grouping {
    /// Whether it is offered in `kind`'s palettes.
    pub fn offered(&self, kind: BlueprintKind) -> bool {
        self.types.is_empty() || self.types.contains(&kind)
    }

    /// Whether its group `g` is offered in `kind`'s palettes.
    pub fn group_offered(&self, g: &Group, kind: BlueprintKind) -> bool {
        self.offered(kind) && (g.types.is_empty() || g.types.contains(&kind))
    }

    /// The kinds of palette its group `g` is offered in.
    pub fn kinds_of(&self, g: &Group) -> Vec<BlueprintKind> {
        BlueprintKind::ALL.into_iter().filter(|k| self.group_offered(g, *k)).collect()
    }

    /// Which groups each of `entries` (a ResRef and the name as the palette
    /// shows it) falls in, in `kind`'s palette: for each group, by its
    /// place in [`Grouping::groups`], the entries in it, in their order;
    /// then those no group takes (Other). Groups not offered in the
    /// palette have none.
    pub fn sort(&self, kind: BlueprintKind, entries: &[(ResRef, &str)]) -> Sorted {
        let n = self.groups.len();
        let mut groups: Vec<Vec<usize>> = vec![Vec::new(); n];
        if !self.offered(kind) {
            return Sorted { groups, other: (0..entries.len()).collect() };
        }
        let offered: Vec<bool> = self.groups.iter().map(|g| self.group_offered(g, kind)).collect();
        // The phrases by their first word: a name's words looked up once
        // each, not each against every rule.
        let mut starts: HashMap<&str, Vec<(usize, &[String])>> = HashMap::new();
        for (i, g) in self.groups.iter().enumerate().filter(|(i, _)| offered[*i]) {
            for p in &g.phrases {
                if let Some(first) = p.first() {
                    starts.entry(first.as_str()).or_default().push((i, p.as_slice()));
                }
            }
        }
        let listed: Vec<HashSet<ResRef>> = self
            .groups
            .iter()
            .map(|g| {
                (g.listed.iter())
                    .filter(|(_, t)| t.is_none_or(|t| t == kind))
                    .map(|(r, _)| *r)
                    .collect()
            })
            .collect();
        let patterned: Vec<usize> =
            (0..n).filter(|i| offered[*i] && !self.groups[*i].resrefs.is_empty()).collect();
        let mut other = Vec::new();
        let mut in_group = vec![false; n];
        for (at, (resref, name)) in entries.iter().enumerate() {
            in_group.iter_mut().for_each(|g| *g = false);
            let part = self.words_of.as_ref().map_or(*name, |w| w.of(name));
            let words = words(part);
            for (w, word) in words.iter().enumerate() {
                for (g, phrase) in starts.get(word.as_str()).into_iter().flatten() {
                    if !in_group[*g] && words[w..].starts_with(phrase) {
                        in_group[*g] = true;
                    }
                }
            }
            let text = resref.to_string_lossy();
            for &g in &patterned {
                if !in_group[g] && self.groups[g].resrefs.matches(&text) {
                    in_group[g] = true;
                }
            }
            for (g, set) in listed.iter().enumerate() {
                if offered[g] && !in_group[g] && set.contains(resref) {
                    in_group[g] = true;
                }
            }
            let mut any = false;
            for (g, inside) in in_group.iter().enumerate() {
                if *inside {
                    groups[g].push(at);
                    any = true;
                }
            }
            if !any {
                other.push(at);
            }
        }
        Sorted { groups, other }
    }

    /// The group named `name` (case ignored), by its place; [`OTHER`] is
    /// none of them.
    pub fn group(&self, name: &str) -> Option<usize> {
        let name = name.trim();
        self.groups.iter().position(|g| g.name.eq_ignore_ascii_case(name))
    }
}

/// What [`Grouping::sort`] found: each group's entries, by the group's
/// place, and Other's.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Sorted {
    pub groups: Vec<Vec<usize>>,
    pub other: Vec<usize>,
}

impl Groupings {
    /// The grouping of an id or a title (case ignored).
    pub fn find(&self, name: &str) -> Option<&Grouping> {
        let name = name.trim();
        (self.list.iter().find(|g| g.id == name))
            .or_else(|| self.list.iter().find(|g| g.id.eq_ignore_ascii_case(name)))
            .or_else(|| self.list.iter().find(|g| g.title.eq_ignore_ascii_case(name)))
    }

    /// The groupings offered in `kind`'s palettes, in the file's order.
    pub fn offered(&self, kind: BlueprintKind) -> impl Iterator<Item = &Grouping> {
        self.list.iter().filter(move |g| g.offered(kind))
    }

    /// The `from` files, where they are (beside the project file, in
    /// `dir`), in the groupings' order.
    pub fn from_paths(&self, dir: &Path) -> Vec<PathBuf> {
        self.list.iter().filter_map(|g| g.from.as_deref()).map(|f| dir.join(f)).collect()
    }

    fn warn(&mut self, warnings: &mut Vec<String>, message: String) {
        self.problems.push(message.clone());
        warnings.push(message);
    }

    /// Reads `[palette]` of the project file: a grouping for each of its
    /// tables.
    pub(crate) fn read(&mut self, r: &mut Reader, entry: Entry) {
        let start = r.warnings.len();
        for e in r.section(entry) {
            let id = e.0.get_ref().to_string();
            let DeValue::Table(t) = e.1.get_ref() else {
                r.warn(
                    e.0.span().start,
                    format!(
                        "palette.{id} is to be a grouping, a table of its own: [palette.{id}] \
                         with title, types and its [[palette.{id}.group]]"
                    ),
                );
                continue;
            };
            if self.list.iter().any(|g| g.id == id) {
                r.warn(e.0.span().start, format!("[palette.{id}] is there twice"));
                continue;
            }
            let mut g = Grouping {
                title: crate::project_file::label_of(&id),
                id: id.clone(),
                types: Vec::new(),
                words_of: None,
                from: None,
                groups: Vec::new(),
                by_name: false,
            };
            let section = format!("palette.{id}");
            for k in in_order(t) {
                match k.0.get_ref().as_ref() {
                    "title" => {
                        if let Some(t) = r.text(k).filter(|t| !t.trim().is_empty()) {
                            g.title = t.trim().to_string();
                        }
                    }
                    "types" => {
                        if let Some(types) = read_types(r, k, &section) {
                            g.types = types;
                        }
                    }
                    "words_of" => {
                        if let Some(p) = r.text(k) {
                            match WordsOf::new(&p) {
                                Ok(w) if w.regex.captures_len() > 1 => g.words_of = Some(w),
                                Ok(_) => r.warn(
                                    k.1.span().start,
                                    format!(
                                        "{section} words_of '{p}' has no group in brackets, \
                                         ( ): the words are the whole name's"
                                    ),
                                ),
                                Err(e) => r.warn(
                                    k.1.span().start,
                                    format!(
                                        "{section} words_of '{p}' is no expression ({e}): the \
                                         words are the whole name's"
                                    ),
                                ),
                            }
                        }
                    }
                    "from" => g.from = r.file_inside(k).filter(|f| !f.is_empty()),
                    "group" => read_groups(r, k, &mut g),
                    "order" => match r.text(k).as_deref().map(str::trim) {
                        Some("name") => g.by_name = true,
                        Some("file") => g.by_name = false,
                        Some(other) => r.warn(
                            k.1.span().start,
                            format!(
                                "{section} order \"{other}\" is neither \"file\" (the groups as \
                                 the project file and its list give them) nor \"name\" (by \
                                 name): they are as the file gives them"
                            ),
                        ),
                        None => {}
                    },
                    _ => r.unknown(&section, k),
                }
            }
            g.put_in_order();
            self.list.push(g);
        }
        self.problems.extend(r.warnings[start..].iter().cloned());
    }

    /// Reads each grouping's `from` file, in `dir` (where the project
    /// file is): its lists added to the groups of their names, a group
    /// of a name the project file doesn't give added after them. A file
    /// missing, unreadable or no TOML is a warning (into `warnings` too).
    pub(crate) fn read_lists(&mut self, dir: &Path, warnings: &mut Vec<String>) {
        for i in 0..self.list.len() {
            let Some(from) = self.list[i].from.clone() else { continue };
            let path = dir.join(&from);
            let id = self.list[i].id.clone();
            match std::fs::read(&path) {
                Ok(bytes) => {
                    let source = String::from_utf8_lossy(&bytes);
                    let mut found = Vec::new();
                    read_list(&from, &source, &mut self.list[i], &mut found);
                    for w in found {
                        self.warn(warnings, w);
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => self.warn(
                    warnings,
                    format!(
                        "{from}: not there ([palette.{id}] from): the grouping has only its \
                         rules"
                    ),
                ),
                Err(e) => self.warn(
                    warnings,
                    format!("{from}: {e} ([palette.{id}] from): the grouping has only its rules"),
                ),
            }
        }
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        format!("{:?}", self.list).hash(&mut h);
        self.fingerprint = h.finish();
    }
}

/// A list of blueprint types (`["utp", "uti"]`), by their extensions.
fn read_types(r: &mut Reader, (key, value): Entry, section: &str) -> Option<Vec<BlueprintKind>> {
    let DeValue::Array(items) = value.get_ref() else {
        let message = format!(
            "{section} {} is to be a list of blueprint types: [\"utp\", \"uti\"]",
            key.get_ref()
        );
        r.warn(value.span().start, message);
        return None;
    };
    let mut out = Vec::new();
    for item in items.iter() {
        match item.get_ref() {
            DeValue::String(s) => match kind_of_extension(s) {
                Some(k) if !out.contains(&k) => out.push(k),
                Some(_) => {}
                None => r.warn(
                    item.span().start,
                    format!("{section} types: \"{s}\" is none of {}", extensions()),
                ),
            },
            _ => r.warn(item.span().start, format!("{section} types: not text")),
        }
    }
    Some(out)
}

/// A list of texts (`names`).
fn read_texts(r: &mut Reader, (key, value): Entry, section: &str) -> Option<Vec<String>> {
    let DeValue::Array(items) = value.get_ref() else {
        let message = format!(
            "{section} {} is to be a list of words in quotes: [\"table\", \"desk\"]",
            key.get_ref()
        );
        r.warn(value.span().start, message);
        return None;
    };
    let mut out = Vec::new();
    for item in items.iter() {
        match item.get_ref() {
            DeValue::String(s) if !words(s).is_empty() => out.push(s.to_string()),
            DeValue::String(s) => r.warn(
                item.span().start,
                format!("{section} {}: \"{s}\" has no word to match", key.get_ref()),
            ),
            _ => r.warn(item.span().start, format!("{section} {}: not text", key.get_ref())),
        }
    }
    Some(out)
}

/// `[[palette.<id>.group]]`: the grouping's groups, in order.
fn read_groups(r: &mut Reader, (key, value): Entry, g: &mut Grouping) {
    let section = format!("palette.{}.group", g.id);
    let DeValue::Array(items) = value.get_ref() else {
        let message =
            format!("{} is to be its groups, each [[{section}]] with a name", key.get_ref());
        r.warn(value.span().start, message);
        return;
    };
    for item in items.iter() {
        let DeValue::Table(t) = item.get_ref() else {
            r.warn(item.span().start, format!("[[{section}]]: each group is a table"));
            continue;
        };
        let entries = in_order(t);
        let Some(name) = entries
            .iter()
            .find(|e| e.0.get_ref() == "name")
            .and_then(|e| r.text(*e))
            .map(|n| n.trim().to_string())
            .filter(|n| !n.is_empty())
        else {
            r.warn(item.span().start, format!("[[{section}]] has no name: it is left out"));
            continue;
        };
        if name.eq_ignore_ascii_case(OTHER) {
            r.warn(
                item.span().start,
                format!(
                    "[[{section}]] \"{name}\": the blueprints no group takes are shown as \
                     {OTHER} already; give the group another name (it is left out)"
                ),
            );
            continue;
        }
        let at = match g.groups.iter().position(|o| o.name.eq_ignore_ascii_case(&name)) {
            Some(at) => {
                let message = format!(
                    "[[{section}]] \"{name}\" is there twice: the second's rules are added to \
                     the first's"
                );
                r.warn(item.span().start, message);
                at
            }
            None => {
                g.groups.push(Group::named(name.clone(), false));
                g.groups.len() - 1
            }
        };
        for e in entries {
            let group = &mut g.groups[at];
            match e.0.get_ref().as_ref() {
                "name" => {}
                "types" => {
                    if let Some(types) = read_types(r, e, &section) {
                        let outside: Vec<&str> = (types.iter())
                            .filter(|k| !g.types.is_empty() && !g.types.contains(k))
                            .filter_map(|k| k.restype().extension())
                            .collect();
                        if !outside.is_empty() {
                            r.warn(
                                e.1.span().start,
                                format!(
                                    "[[{section}]] \"{name}\" types: {} not among \
                                     [palette.{}]'s types: it isn't offered there",
                                    outside.join(", "),
                                    g.id
                                ),
                            );
                        }
                        let group = &mut g.groups[at];
                        for k in types {
                            if !group.types.contains(&k) {
                                group.types.push(k);
                            }
                        }
                    }
                }
                "names" => {
                    if let Some(names) = read_texts(r, e, &section) {
                        group.phrases.extend(names.iter().map(|n| words(n)));
                        group.names.extend(names);
                    }
                }
                "resrefs" => {
                    if let Some(p) = r.patterns(e) {
                        let mut all = group.resrefs.globs.clone();
                        all.extend(p.globs);
                        let all: Vec<&str> = all.iter().map(String::as_str).collect();
                        group.resrefs = Patterns::new(&all).expect("each read");
                    }
                }
                _ => r.unknown(&section, e),
            }
        }
    }
}

/// One `from` file's text (`name`, as messages give it) into a grouping's
/// groups. What isn't understood goes in `warnings`.
pub fn read_list(name: &str, source: &str, g: &mut Grouping, warnings: &mut Vec<String>) {
    let table = match DeTable::parse(source) {
        Ok(t) => t,
        Err(e) => {
            let line = e.span().map_or(1, |s| crate::project_file::line_of(source, s.start));
            warnings.push(format!(
                "{name}:{line}: {} ([palette.{}] from): the grouping has only its rules",
                e.message().trim(),
                g.id
            ));
            return;
        }
    };
    let mut r = Reader { name, source, warnings: Vec::new() };
    for (key, value) in in_order(table.get_ref()) {
        let group_name = key.get_ref().trim().to_string();
        if group_name.is_empty() || group_name.eq_ignore_ascii_case(OTHER) {
            let message = if group_name.is_empty() {
                "a group's name is empty: its list is left out".to_string()
            } else {
                format!(
                    "\"{group_name}\": the blueprints no group takes are shown as {OTHER} \
                     already; give the group another name (its list is left out)"
                )
            };
            r.warn(key.span().start, message);
            continue;
        }
        let DeValue::Array(items) = value.get_ref() else {
            let message = format!(
                "\"{group_name}\" is to be a list of ResRefs: [\"dd_table01\", \
                 \"dd_stool.utp\"]"
            );
            r.warn(value.span().start, message);
            continue;
        };
        let at = match g.group(&group_name) {
            Some(at) => at,
            None => {
                g.groups.push(Group::named(group_name.clone(), true));
                g.groups.len() - 1
            }
        };
        for item in items.iter() {
            let DeValue::String(text) = item.get_ref() else {
                r.warn(item.span().start, format!("\"{group_name}\": not text"));
                continue;
            };
            let text = text.trim();
            let (resref, kind) = match text.rsplit_once('.') {
                Some((stem, ext)) => match kind_of_extension(ext) {
                    Some(k) => (stem, Some(k)),
                    None => {
                        let message = format!(
                            "\"{group_name}\": \"{text}\": .{ext} is none of {}",
                            extensions()
                        );
                        r.warn(item.span().start, message);
                        continue;
                    }
                },
                None => (text, None),
            };
            match ResRef::from_str(resref) {
                Ok(r_) if !r_.is_empty() => {
                    if let Some(k) = kind
                        && !g.group_offered(&g.groups[at], k)
                    {
                        r.warn(
                            item.span().start,
                            format!(
                                "\"{group_name}\": \"{text}\": the group isn't offered in the \
                                 {} palette",
                                k.name()
                            ),
                        );
                    }
                    g.groups[at].listed.push((r_, kind));
                }
                Ok(_) => r.warn(item.span().start, format!("\"{group_name}\": an empty ResRef")),
                Err(e) => r.warn(item.span().start, format!("\"{group_name}\": \"{text}\": {e}")),
            }
        }
    }
    g.put_in_order();
    warnings.append(&mut r.warnings);
}

/// A palette grouped by a grouping: a node for each group with blueprints
/// in it, Other last, as the palette pane shows it (Group by).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Grouped {
    /// The groups: with the palette's categories in each where they are
    /// nested (Then by Category), each pruned to the group's blueprints;
    /// else the group's blueprints in a list, in the palette's order.
    pub palette: Palette,
    /// Each group node's group, by its place among them (`None`: Other).
    pub groups: Vec<Option<usize>>,
    /// Each category's place in the palette it came from, by its place
    /// here (group nodes have none).
    pub origin: HashMap<Vec<usize>, Vec<usize>>,
}

impl Grouped {
    /// The name of a group node, by its place among them.
    pub fn group_name<'g>(&self, grouping: &'g Grouping, at: usize) -> &'g str {
        match self.groups.get(at).copied().flatten() {
            Some(g) => &grouping.groups[g].name,
            None => OTHER,
        }
    }
}

/// Every blueprint of a palette with its node's place, in the tree's order.
pub fn entries(palette: &Palette) -> Vec<(Vec<usize>, &PaletteBlueprint)> {
    fn walk<'p>(
        nodes: &'p [PaletteNode],
        path: &mut Vec<usize>,
        out: &mut Vec<(Vec<usize>, &'p PaletteBlueprint)>,
    ) {
        for (i, n) in nodes.iter().enumerate() {
            path.push(i);
            out.extend(n.blueprints.iter().map(|b| (path.clone(), b)));
            walk(&n.children, path, out);
            path.pop();
        }
    }
    let mut out = Vec::new();
    walk(&palette.nodes, &mut Vec::new(), &mut out);
    out
}

/// `palette` (of `kind`) grouped by `grouping`: the names as the palette
/// shows them (`name`: a talk-table string looked up), each group with
/// the palette's categories in it (`nested`), or its blueprints in a list.
/// Groups no blueprint falls in are left out; Other is last.
pub fn group_palette(
    palette: &Palette,
    kind: BlueprintKind,
    grouping: &Grouping,
    name: &dyn Fn(&PaletteName) -> String,
    nested: bool,
) -> Grouped {
    let all = entries(palette);
    let names: Vec<String> = all.iter().map(|(_, b)| name(&b.name)).collect();
    let keyed: Vec<(ResRef, &str)> =
        all.iter().zip(&names).map(|((_, b), n)| (b.resref, n.as_str())).collect();
    let sorted = grouping.sort(kind, &keyed);
    let mut out = Grouped::default();
    let shown = sorted
        .groups
        .iter()
        .enumerate()
        .map(|(g, members)| (Some(g), members))
        .chain(std::iter::once((None, &sorted.other)))
        .filter(|(_, members)| !members.is_empty());
    for (group, members) in shown {
        let title = match group {
            Some(g) => grouping.groups[g].name.as_str(),
            None => OTHER,
        };
        let at = out.palette.nodes.len();
        let mut node = PaletteNode {
            name: PaletteName::Text(title.as_bytes().to_vec()),
            id: None,
            children: Vec::new(),
            blueprints: Vec::new(),
        };
        if nested {
            // (The entries by their node, to prune the tree to them.)
            let mut by_node: HashMap<&[usize], Vec<usize>> = HashMap::new();
            for &m in members {
                by_node.entry(all[m].0.as_slice()).or_default().push(m);
            }
            let mut from = Vec::new();
            let mut here = vec![at];
            node.children =
                pruned(&palette.nodes, &by_node, &all, &mut from, &mut here, &mut out.origin);
        } else {
            node.blueprints = members.iter().map(|&m| all[m].1.clone()).collect();
        }
        out.palette.nodes.push(node);
        out.groups.push(group);
    }
    out
}

/// A blueprint of a group, as `mg find --palette-group` lists it.
#[derive(Debug, Clone, PartialEq)]
pub struct Member {
    pub resref: ResRef,
    /// Its name as the palette shows it.
    pub name: String,
    /// Its category in the palette ("Domains of Dread / Tavern").
    pub category: String,
}

/// The blueprints of `palette` (of `kind`) in a group of `grouping` (by
/// its place; `None`: Other), in the palette's order, each with its
/// category.
pub fn members(
    palette: &Palette,
    kind: BlueprintKind,
    grouping: &Grouping,
    group: Option<usize>,
    name: &dyn Fn(&PaletteName) -> String,
) -> Vec<Member> {
    fn walk(
        nodes: &[PaletteNode],
        path: &str,
        name: &dyn Fn(&PaletteName) -> String,
        out: &mut Vec<Member>,
    ) {
        for n in nodes {
            let here = match path {
                "" => name(&n.name),
                _ => format!("{path} / {}", name(&n.name)),
            };
            out.extend(n.blueprints.iter().map(|b| Member {
                resref: b.resref,
                name: name(&b.name),
                category: here.clone(),
            }));
            walk(&n.children, &here, name, out);
        }
    }
    let grouped = group_palette(palette, kind, grouping, name, true);
    let mut out = Vec::new();
    if let Some(at) = grouped.groups.iter().position(|g| *g == group) {
        walk(&grouped.palette.nodes[at].children, "", name, &mut out);
    }
    out
}

/// The categories of `nodes` with the blueprints of `by_node` in them (or
/// under them), each keeping only those; their places noted in `origin`.
fn pruned(
    nodes: &[PaletteNode],
    by_node: &HashMap<&[usize], Vec<usize>>,
    all: &[(Vec<usize>, &PaletteBlueprint)],
    from: &mut Vec<usize>,
    here: &mut Vec<usize>,
    origin: &mut HashMap<Vec<usize>, Vec<usize>>,
) -> Vec<PaletteNode> {
    let mut out = Vec::new();
    for (i, n) in nodes.iter().enumerate() {
        from.push(i);
        here.push(out.len());
        let children = pruned(&n.children, by_node, all, from, here, origin);
        let blueprints: Vec<PaletteBlueprint> = by_node
            .get(from.as_slice())
            .map(|m| m.iter().map(|&m| all[m].1.clone()).collect())
            .unwrap_or_default();
        if !children.is_empty() || !blueprints.is_empty() {
            origin.insert(here.clone(), from.clone());
            out.push(PaletteNode { name: n.name.clone(), id: n.id, children, blueprints });
        }
        here.pop();
        from.pop();
    }
    out
}

#[cfg(test)]
mod tests;
