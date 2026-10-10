//! Where a resource or a tag is used in a module, and renaming a resource
//! everywhere: the resource (with its companions: a script's compiled code,
//! an area's instances and comments), every reference to it, its own name
//! fields, `#include` lines and, if asked, the strings in scripts that spell
//! its name.

use mg_core::{Gender, Language, ResRef, ResType};
use mg_gff::{Gff, Struct, Value};
use mg_resman::ResKey;
use mg_script::lex::{TokenKind, tokenize};

use crate::Module;
use crate::refs::{RefKind, Reference, references, rewrite_references};

/// The reference kinds that point at a resource of this type; empty if
/// nothing refers to such resources by name (they can't be renamed here).
pub fn kinds(restype: ResType) -> &'static [RefKind] {
    match restype {
        ResType::NSS | ResType::NCS | ResType::NDB => &[RefKind::Script, RefKind::Include],
        ResType::DLG => &[RefKind::Conversation],
        ResType::ARE | ResType::GIT | ResType::GIC => &[RefKind::Area],
        ResType::UTC => &[RefKind::Blueprint(ResType::UTC)],
        ResType::UTD => &[RefKind::Blueprint(ResType::UTD)],
        ResType::UTE => &[RefKind::Blueprint(ResType::UTE)],
        ResType::UTI => &[RefKind::Blueprint(ResType::UTI)],
        ResType::UTM => &[RefKind::Blueprint(ResType::UTM)],
        ResType::UTP => &[RefKind::Blueprint(ResType::UTP)],
        ResType::UTS => &[RefKind::Blueprint(ResType::UTS)],
        ResType::UTT => &[RefKind::Blueprint(ResType::UTT)],
        ResType::UTW => &[RefKind::Blueprint(ResType::UTW)],
        _ => &[],
    }
}

/// The resources renamed together with one of this type.
fn family(restype: ResType) -> &'static [ResType] {
    match restype {
        ResType::NSS | ResType::NCS | ResType::NDB => &[ResType::NSS, ResType::NCS, ResType::NDB],
        ResType::ARE | ResType::GIT | ResType::GIC => &[ResType::ARE, ResType::GIT, ResType::GIC],
        ResType::DLG => &[ResType::DLG],
        ResType::UTC => &[ResType::UTC],
        ResType::UTD => &[ResType::UTD],
        ResType::UTE => &[ResType::UTE],
        ResType::UTI => &[ResType::UTI],
        ResType::UTM => &[ResType::UTM],
        ResType::UTP => &[ResType::UTP],
        ResType::UTS => &[ResType::UTS],
        ResType::UTT => &[ResType::UTT],
        ResType::UTW => &[ResType::UTW],
        _ => &[],
    }
}

/// Whether resources of this type can be renamed everywhere.
pub fn renamable(restype: ResType) -> bool {
    !kinds(restype).is_empty()
}

/// One place that names a resource or a tag.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Usage {
    /// The resource that names it.
    pub from: ResKey,
    /// Where in it: a GFF path (`/Creature List[3]/ScriptSpawn`) or, in a
    /// script, `line N`.
    pub path: String,
    /// Where, for people: `area001 › creature GUARD › OnSpawn`.
    pub place: String,
}

impl Usage {
    /// The line, for a usage in a script.
    pub fn line(&self) -> Option<usize> {
        self.path.strip_prefix("line ")?.parse().ok()
    }

    /// For a placed object: the GIT list it is in and its index there.
    pub fn instance(&self) -> Option<(&str, usize)> {
        if self.from.restype != ResType::GIT {
            return None;
        }
        let first = self.path.strip_prefix('/')?.split('/').next()?;
        let (list, rest) = first.split_once('[')?;
        Some((list, rest.strip_suffix(']')?.parse().ok()?))
    }
}

/// A string in a script that spells a name (`"guard_spawn"`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mention {
    pub script: ResKey,
    pub line: usize,
    /// The line, trimmed.
    pub text: String,
}

/// Where the resource `target` is used in the module: every reference of
/// the kinds that point at its type (in GFF resources and `#include`
/// lines), each with a readable place, in resource order.
pub fn usages(module: &Module, target: ResKey) -> Vec<Usage> {
    let kinds = kinds(target.restype);
    let mut keys: Vec<ResKey> = module.keys().copied().collect();
    keys.sort();
    let mut out = Vec::new();
    for key in keys {
        let Some(data) = module.get(&key) else { continue };
        let refs: Vec<_> = references(key, data)
            .into_iter()
            .filter(|r| kinds.contains(&r.kind) && r.target == target.resref)
            .collect();
        if refs.is_empty() {
            continue;
        }
        let gff = if key.restype.is_gff() { Gff::read(data).ok() } else { None };
        for r in refs {
            let place = describe(key, gff.as_ref(), &r.path);
            out.push(Usage { from: key, path: r.path, place });
        }
    }
    out
}

/// The module's references, kept from one question to the next
/// ([`usages`] reads every resource each time: 0.8 s in a persistent
/// world). A resource is read again only when the module holds other
/// bytes for it, so the first question costs what [`usages`] does and the
/// next ones what changed since.
#[derive(Debug, Clone, Default)]
pub struct UsageIndex {
    /// Each resource as read.
    seen: std::collections::HashMap<ResKey, Read>,
    /// How many resources the last [`refresh`](Self::refresh) read.
    pub read: usize,
}

/// What one resource holds that a question may ask for.
#[derive(Debug, Clone)]
struct Read {
    /// Its bytes as read.
    data: std::sync::Arc<[u8]>,
    refs: Vec<Reference>,
    /// A GFF's fields that name a tag ([`TAG_FIELDS`]): where, and the tag.
    tags: Vec<(String, Vec<u8>)>,
    /// A script's string literals: where each begins (inside the quotes).
    strings: Vec<(usize, usize)>,
    /// The talk-table lines it names: a GFF's localized strings that have
    /// a StrRef, and a 2DA's cells in the columns known to hold one
    /// (`doctor::STRREF_COLUMNS`); where, and the StrRef.
    strrefs: Vec<(String, u32)>,
}

impl Read {
    fn of(key: ResKey, data: &std::sync::Arc<[u8]>) -> Read {
        let mut read = Read {
            data: data.clone(),
            refs: Vec::new(),
            tags: Vec::new(),
            strings: Vec::new(),
            strrefs: Vec::new(),
        };
        if key.restype == ResType::NSS {
            read.refs = crate::refs::script_includes(key, data);
            read.strings = tokenize(data)
                .into_iter()
                .filter(|t| matches!(t.kind, TokenKind::String { terminated: true }))
                .map(|t| (t.span.start + 1, t.span.end - 1))
                .collect();
        } else if key.restype.is_gff()
            && let Ok(gff) = Gff::read(data)
        {
            read.refs = crate::refs::gff_references(key, &gff);
            tag_fields(&gff.root, "", &mut read.tags);
            strref_fields(&gff.root, "", &mut read.strrefs);
        } else if key.restype == ResType::TWODA
            && let Ok(table) = mg_2da::TwoDa::parse(data, mg_core::Codepage::default())
        {
            let name = key.resref.to_lowercase().to_string();
            for (_, column) in crate::doctor::STRREF_COLUMNS.iter().filter(|(t, _)| *t == name) {
                for row in 0..table.len() {
                    let cell = table.get(row, column).and_then(mg_2da::parse_int);
                    if let Some(n) = cell.and_then(|n| u32::try_from(n).ok()) {
                        read.strrefs.push((format!("row {row}, {column}"), n));
                    }
                }
            }
        }
        read
    }
}

/// The localized strings of `s` that name a talk-table line, with their
/// paths.
fn strref_fields(s: &Struct, path: &str, out: &mut Vec<(String, u32)>) {
    for f in &s.fields {
        let label = f.label.to_string_lossy();
        let p = format!("{path}/{label}");
        match &f.value {
            Value::LocString(v) if !v.strref.is_none() => out.push((p, v.strref.0)),
            Value::Struct(c) => strref_fields(c, &p, out),
            Value::List(items) => {
                for (i, c) in items.iter().enumerate() {
                    strref_fields(c, &format!("{p}[{i}]"), out);
                }
            }
            _ => {}
        }
    }
}

/// The fields of `s` that name a tag, with their paths.
fn tag_fields(s: &Struct, path: &str, out: &mut Vec<(String, Vec<u8>)>) {
    for f in &s.fields {
        let label = f.label.to_string_lossy();
        let p = format!("{path}/{label}");
        match &f.value {
            Value::String(v) if TAG_FIELDS.contains(&label.as_str()) => out.push((p, v.clone())),
            Value::Struct(c) => tag_fields(c, &p, out),
            Value::List(items) => {
                for (i, c) in items.iter().enumerate() {
                    tag_fields(c, &format!("{p}[{i}]"), out);
                }
            }
            _ => {}
        }
    }
}

impl UsageIndex {
    /// Reads the resources that are new or changed since the last time,
    /// and forgets those that are gone.
    pub fn refresh(&mut self, module: &Module) {
        self.read = 0;
        self.seen.retain(|k, _| module.contains(k));
        for key in module.keys() {
            let Some(data) = module.shared(key) else { continue };
            match self.seen.get_mut(key) {
                Some(was) if std::sync::Arc::ptr_eq(&was.data, data) => continue,
                // (Set again with the same bytes, as a flush of the
                // documents open does: nothing to read.)
                Some(was) if *was.data == **data => {
                    was.data = data.clone();
                    continue;
                }
                _ => {}
            }
            self.read += 1;
            self.seen.insert(*key, Read::of(*key, data));
        }
    }

    /// [`usages`] of `target`, from what [`refresh`](Self::refresh) read.
    pub fn usages(&self, target: ResKey) -> Vec<Usage> {
        let kinds = kinds(target.restype);
        let hit = |r: &&Reference| kinds.contains(&r.kind) && r.target == target.resref;
        let mut keys: Vec<&ResKey> = self
            .seen
            .iter()
            .filter(|(_, read)| read.refs.iter().any(|r| hit(&r)))
            .map(|(k, _)| k)
            .collect();
        keys.sort();
        let mut out = Vec::new();
        for key in keys {
            let read = &self.seen[key];
            let gff = if key.restype.is_gff() { Gff::read(&read.data).ok() } else { None };
            for r in read.refs.iter().filter(hit) {
                let place = describe(*key, gff.as_ref(), &r.path);
                out.push(Usage { from: *key, path: r.path.clone(), place });
            }
        }
        out
    }

    /// [`tag_usages`] of `tag`, from what [`refresh`](Self::refresh) read.
    pub fn tag_usages(&self, tag: &str) -> Vec<Usage> {
        if tag.is_empty() {
            return Vec::new();
        }
        let hit = |t: &&(String, Vec<u8>)| t.1 == tag.as_bytes();
        let mut keys: Vec<&ResKey> = self
            .seen
            .iter()
            .filter(|(_, r)| r.tags.iter().any(|t| hit(&t)))
            .map(|(k, _)| k)
            .collect();
        keys.sort();
        let mut out = Vec::new();
        for key in keys {
            let read = &self.seen[key];
            let gff = Gff::read(&read.data).ok();
            for (path, _) in read.tags.iter().filter(hit) {
                let place = describe(*key, gff.as_ref(), path);
                out.push(Usage { from: *key, path: path.clone(), place });
            }
        }
        out
    }

    /// Where the module names the talk-table line `strref` (the number as
    /// the files have it: a custom table's lines from 16777216): in a
    /// localized string of a GFF resource, or in a 2DA of the module in a
    /// column known to hold StrRefs. From what
    /// [`refresh`](Self::refresh) read; the haks' 2DAs are not the
    /// module's, and not looked in.
    pub fn strref_usages(&self, strref: u32) -> Vec<Usage> {
        let hit = |t: &&(String, u32)| t.1 == strref;
        let mut keys: Vec<&ResKey> = (self.seen.iter())
            .filter(|(_, r)| r.strrefs.iter().any(|t| hit(&t)))
            .map(|(k, _)| k)
            .collect();
        keys.sort();
        let mut out = Vec::new();
        for key in keys {
            let read = &self.seen[key];
            let gff = if key.restype.is_gff() { Gff::read(&read.data).ok() } else { None };
            for (path, _) in read.strrefs.iter().filter(hit) {
                let place = match &gff {
                    Some(_) => describe(*key, gff.as_ref(), path),
                    None => format!("{key} › {path}"),
                };
                out.push(Usage { from: *key, path: path.clone(), place });
            }
        }
        out
    }

    /// [`mentions`] of `name`, from what [`refresh`](Self::refresh) read.
    pub fn mentions(&self, name: &str, ignore_case: bool) -> Vec<Mention> {
        let mut keys: Vec<&ResKey> =
            self.seen.iter().filter(|(_, r)| !r.strings.is_empty()).map(|(k, _)| k).collect();
        keys.sort();
        let mut out = Vec::new();
        for key in keys {
            let read = &self.seen[key];
            let src = &read.data[..];
            for &(a, b) in &read.strings {
                let s = &src[a..b];
                let same = match ignore_case {
                    true => s.eq_ignore_ascii_case(name.as_bytes()),
                    false => s == name.as_bytes(),
                };
                if same {
                    out.push(mention_at(*key, src, a));
                }
            }
        }
        out
    }
}

/// The mention of a string that begins at `start` of a script's source.
fn mention_at(script: ResKey, src: &[u8], start: usize) -> Mention {
    let line = src[..start].iter().filter(|b| **b == b'\n').count() + 1;
    let text = String::from_utf8_lossy(src).lines().nth(line - 1).unwrap_or("").trim().to_string();
    Mention { script, line, text }
}

/// GFF string fields that name a tag: an object's own, a transition's
/// destination, a lock's key, a conversation line's journal category.
const TAG_FIELDS: [&str; 4] = ["Tag", "LinkedTo", "KeyName", "Quest"];

/// Where a tag is used: objects, blueprints and areas with that tag, and
/// the transitions, locks and conversation lines that name it (compared
/// exactly, as the game's tag lookups do).
pub fn tag_usages(module: &Module, tag: &str) -> Vec<Usage> {
    fn walk(s: &Struct, path: &str, tag: &[u8], out: &mut Vec<String>) {
        for f in &s.fields {
            let label = f.label.to_string_lossy();
            let p = format!("{path}/{label}");
            match &f.value {
                Value::String(v) if TAG_FIELDS.contains(&label.as_str()) && v == tag => out.push(p),
                Value::Struct(c) => walk(c, &p, tag, out),
                Value::List(items) => {
                    for (i, c) in items.iter().enumerate() {
                        walk(c, &format!("{p}[{i}]"), tag, out);
                    }
                }
                _ => {}
            }
        }
    }
    if tag.is_empty() {
        return Vec::new();
    }
    let mut keys: Vec<ResKey> = module.keys().filter(|k| k.restype.is_gff()).copied().collect();
    keys.sort();
    let mut out = Vec::new();
    for key in keys {
        let Some(Ok(gff)) = module.gff(&key) else { continue };
        let mut paths = Vec::new();
        walk(&gff.root, "", tag.as_bytes(), &mut paths);
        for path in paths {
            let place = describe(key, Some(&gff), &path);
            out.push(Usage { from: key, path, place });
        }
    }
    out
}

/// The strings in the module's scripts that spell `name`: ignoring case for
/// a resource name (the game looks resources up so), exactly for a tag.
pub fn mentions(module: &Module, name: &str, ignore_case: bool) -> Vec<Mention> {
    let mut keys: Vec<ResKey> = module.keys_of(ResType::NSS).copied().collect();
    keys.sort();
    let mut out = Vec::new();
    for key in keys {
        let Some(src) = module.get(&key) else { continue };
        for (start, _) in string_literals(src, name, ignore_case) {
            out.push(mention_at(key, src, start));
        }
    }
    out
}

/// The spans (inside the quotes) of a source's plain string literals equal
/// to `name`.
fn string_literals(src: &[u8], name: &str, ignore_case: bool) -> Vec<(usize, usize)> {
    tokenize(src)
        .into_iter()
        .filter(|t| matches!(t.kind, TokenKind::String { terminated: true }))
        .map(|t| (t.span.start + 1, t.span.end - 1))
        .filter(|&(a, b)| {
            let s = &src[a..b];
            if ignore_case { s.eq_ignore_ascii_case(name.as_bytes()) } else { s == name.as_bytes() }
        })
        .collect()
}

/// A path's place, for people.
pub(crate) fn describe(key: ResKey, gff: Option<&Gff>, path: &str) -> String {
    let name = key.resref.to_string();
    let field = |label: &str| -> String {
        match label.strip_prefix("Script") {
            Some(rest) if !rest.is_empty() => format!("On{rest}"),
            _ => label.to_string(),
        }
    };
    let steps: Vec<&str> = path.trim_start_matches('/').split('/').collect();
    let last = steps.last().copied().unwrap_or_default();
    let last = last.split('[').next().unwrap_or(last);
    let item = |list: &str| -> Option<(String, usize)> {
        let (l, rest) = list.split_once('[')?;
        Some((l.to_string(), rest.strip_suffix(']')?.parse().ok()?))
    };
    let string = |s: &Struct, label: &str| -> Option<String> {
        match s.get(label)? {
            Value::String(v) => Some(String::from_utf8_lossy(v).into_owned()),
            _ => None,
        }
    };
    match key.restype {
        ResType::NSS => format!("{key} › {path}"),
        ResType::IFO => {
            let what = match steps.first().copied().unwrap_or_default() {
                s if s.starts_with("Mod_Area_list") => "area list".to_string(),
                "Mod_Entry_Area" => "start area".to_string(),
                s if s.starts_with("Mod_CacheNSSList") => "cached scripts".to_string(),
                s => s.trim_start_matches("Mod_").to_string(),
            };
            format!("Module › {what}")
        }
        ResType::GIT => {
            let Some((list, index)) = steps.first().and_then(|s| item(s)) else {
                return format!("{name} › {}", field(last));
            };
            let kind = match list.as_str() {
                "Creature List" => "creature",
                "Door List" => "door",
                "Encounter List" => "encounter",
                "List" => "item",
                "Placeable List" => "placeable",
                "SoundList" => "sound",
                "StoreList" => "store",
                "TriggerList" => "trigger",
                "WaypointList" => "waypoint",
                _ => "object",
            };
            let tag = gff
                .and_then(|g| match g.root.get(&list) {
                    Some(Value::List(items)) => items.get(index),
                    _ => None,
                })
                .and_then(|s| string(s, "Tag"))
                .filter(|t| !t.is_empty())
                .unwrap_or_else(|| format!("#{}", index + 1));
            let inner = if steps.len() > 2 {
                let what = steps[1].split('[').next().unwrap_or_default();
                format!(
                    "{} {}",
                    match what {
                        "ItemList" | "Equip_ItemList" => "inventory",
                        "CreatureList" => "creatures",
                        other => other,
                    },
                    field(last)
                )
            } else {
                field(last)
            };
            format!("{name} › {kind} {tag} › {inner}")
        }
        ResType::ARE => format!("{name} (area) › {}", field(last)),
        ResType::DLG => {
            let Some((list, index)) = steps.first().and_then(|s| item(s)) else {
                return format!("{name} › {}", field(last));
            };
            let who = match list.as_str() {
                "EntryList" => "NPC line",
                "ReplyList" => "PC line",
                "StartingList" => "start",
                _ => list.as_str(),
            };
            let text = gff
                .and_then(|g| match g.root.get(&list) {
                    Some(Value::List(items)) => items.get(index),
                    _ => None,
                })
                .and_then(|s| s.locstring("Text"))
                .and_then(|t| t.text(Language::ENGLISH, Gender::Male).map(|c| c.into_owned()))
                .map(|t| {
                    let t: String = t.chars().take(40).collect();
                    format!(" “{t}”")
                })
                .unwrap_or_default();
            let what = match last {
                "Active" => "condition".to_string(),
                "Script" => "action".to_string(),
                other => field(other),
            };
            let link = if steps.len() > 2 { " (link)" } else { "" };
            format!("{name} › {who} {}{text}{link} › {what}", index + 1)
        }
        _ => format!("{key} › {}", field(last)),
    }
}

/// Why a rename can't be done.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RenameError {
    #[error("{0} can't be renamed")]
    NotRenamable(ResKey),
    #[error("{0} is not in the module")]
    Missing(ResKey),
    #[error("{0} already exists")]
    Exists(ResKey),
    #[error("the new name is the old one")]
    SameName,
    #[error("{0}: {1}")]
    Resource(ResKey, String),
}

/// What a rename changed.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RenameReport {
    /// The resources written (renamed ones under their new names).
    pub changed: Vec<ResKey>,
    /// References rewritten.
    pub references: usize,
    /// `#include` lines and script strings rewritten.
    pub in_scripts: usize,
    /// Scripts whose source changed; their compiled code was removed, to be
    /// compiled again.
    pub recompile: Vec<ResKey>,
}

/// Renames `from` (with its family: a script's `.ncs` and `.ndb`, an area's
/// `.git` and `.gic`) to `to` and points everything in the module that names
/// it at the new name: references, its own name fields (an area's `ResRef`,
/// a blueprint's `TemplateResRef`, a store's `ResRef`), `#include` lines,
/// and, with `literals`, script strings spelling the old name.
pub fn rename(
    module: &mut Module,
    from: ResKey,
    to: ResRef,
    literals: bool,
) -> Result<RenameReport, RenameError> {
    let kinds = kinds(from.restype);
    if kinds.is_empty() {
        return Err(RenameError::NotRenamable(from));
    }
    if to == from.resref {
        return Err(RenameError::SameName);
    }
    let family = family(from.restype);
    let moving: Vec<ResType> =
        family.iter().copied().filter(|t| module.contains(&ResKey::new(from.resref, *t))).collect();
    if moving.is_empty() {
        return Err(RenameError::Missing(from));
    }
    if let Some(t) = family.iter().find(|t| module.contains(&ResKey::new(to, **t))) {
        return Err(RenameError::Exists(ResKey::new(to, *t)));
    }
    let mut report = RenameReport::default();
    let bad = |k: ResKey| move |e: String| RenameError::Resource(k, e);

    // The resources themselves, under the new name, with their own name.
    for t in moving {
        let (old, new) = (ResKey::new(from.resref, t), ResKey::new(to, t));
        let mut data = module.remove(&old).expect("present").to_vec();
        let own = match t {
            ResType::ARE => Some("ResRef"),
            ResType::UTM => Some("ResRef"),
            t if t.is_gff() && kinds.iter().any(|k| matches!(k, RefKind::Blueprint(_))) => {
                Some("TemplateResRef")
            }
            _ => None,
        };
        if let Some(label) = own {
            let mut g = Gff::read(&data).map_err(|e| bad(old)(e.to_string()))?;
            if g.root.get(label).is_some() {
                g.root.set(label, Value::resref(to));
                data = g.to_bytes().map_err(|e| bad(old)(e.to_string()))?;
            }
        }
        module.set(new, data);
        report.changed.push(new);
    }

    // References in GFF resources.
    let keys: Vec<ResKey> = module.keys().copied().collect();
    for key in keys {
        if key.restype.is_gff() {
            let Some(Ok(mut g)) = module.gff(&key) else { continue };
            let n = rewrite_references(&mut g, kinds, from.resref, to);
            if n > 0 {
                module.set_gff(key, &g).map_err(|e| bad(key)(e.to_string()))?;
                report.references += n;
                if !report.changed.contains(&key) {
                    report.changed.push(key);
                }
            }
        }
    }

    // Script sources: #include lines, and strings if asked.
    let scripts: Vec<ResKey> = module.keys_of(ResType::NSS).copied().collect();
    for key in scripts {
        let src = module.get(&key).expect("present").to_vec();
        let mut spans = Vec::new();
        if kinds.contains(&RefKind::Include) {
            spans.extend(include_spans(&src, &from.resref.to_string()));
        }
        if literals {
            for s in string_literals(&src, &from.resref.to_string(), true) {
                if !spans.contains(&s) {
                    spans.push(s);
                }
            }
        }
        if spans.is_empty() {
            continue;
        }
        spans.sort();
        let new_name = to.to_string();
        let mut out = Vec::with_capacity(src.len());
        let mut at = 0;
        for (a, b) in &spans {
            out.extend_from_slice(&src[at..*a]);
            out.extend_from_slice(new_name.as_bytes());
            at = *b;
        }
        out.extend_from_slice(&src[at..]);
        module.set(key, out);
        report.in_scripts += spans.len();
        if !report.changed.contains(&key) {
            report.changed.push(key);
        }
        report.recompile.push(key);
    }
    // Scripts that include a changed one, however indirectly, compile
    // differently too.
    let more = crate::refs::includers(module, &report.recompile);
    report.recompile.extend(more);
    for key in &report.recompile {
        for t in [ResType::NCS, ResType::NDB] {
            module.remove(&ResKey::new(key.resref, t));
        }
    }
    report.recompile.sort();
    Ok(report)
}

/// The spans (inside the quotes) of `#include "name"` lines naming `name`.
fn include_spans(src: &[u8], name: &str) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut offset = 0;
    for line in src.split(|b| *b == b'\n') {
        let start = line.iter().position(|b| !b.is_ascii_whitespace()).unwrap_or(line.len());
        if line[start..].starts_with(b"#include")
            && let Some(q1) = line[start..].iter().position(|b| *b == b'"').map(|q| start + q + 1)
            && let Some(q2) = line[q1..].iter().position(|b| *b == b'"').map(|q| q1 + q)
            && line[q1..q2].eq_ignore_ascii_case(name.as_bytes())
        {
            out.push((offset + q1, offset + q2));
        }
        offset += line.len() + 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use mg_core::LocString;
    use mg_schema::{StructExt, git, ifo};

    use super::*;

    fn rr(s: &str) -> ResRef {
        ResRef::from_str(s).unwrap()
    }
    fn key(n: &str, t: ResType) -> ResKey {
        ResKey::new(rr(n), t)
    }

    /// A module with an area whose creature spawns with `guard_spawn`, which
    /// includes `inc_guard`; a conversation runs `guard_spawn` too.
    fn module() -> Module {
        let mut m = Module::new();
        let mut info = Gff::new(*b"IFO ");
        let mut a = ifo::MOD_AREA_LIST.new_item();
        a.write(&ifo::mod_area_list::AREA_NAME, rr("keep"));
        info.root.items_mut(&ifo::MOD_AREA_LIST).push(a);
        info.root.write(&ifo::MOD_ENTRY_AREA, rr("keep"));
        m.set_info(&info).unwrap();
        let mut are = Gff::new(*b"ARE ");
        are.root.set("ResRef", Value::resref(rr("keep")));
        are.root.set("Tag", Value::String(b"KEEP".to_vec()));
        m.set_gff(key("keep", ResType::ARE), &are).unwrap();
        let mut g = Gff::new(*b"GIT ");
        let mut c = git::CREATURE_LIST.new_item();
        c.write(&git::creature_list::TEMPLATE_RES_REF, rr("guard"));
        c.write(&git::creature_list::SCRIPT_SPAWN, rr("guard_spawn"));
        c.set("Tag", Value::String(b"GUARD".to_vec()));
        g.root.items_mut(&git::CREATURE_LIST).push(c);
        let mut door = Struct::new(8);
        door.set("Tag", Value::String(b"GATE".to_vec()));
        door.set("LinkedTo", Value::String(b"GUARD".to_vec()));
        g.root.set("Door List", Value::List(vec![door]));
        m.set_gff(key("keep", ResType::GIT), &g).unwrap();
        m.set_gff(key("keep", ResType::GIC), &Gff::new(*b"GIC ")).unwrap();
        let mut utc = Gff::new(*b"UTC ");
        utc.root.set("TemplateResRef", Value::resref(rr("guard")));
        utc.root.set("ScriptSpawn", Value::resref(rr("guard_spawn")));
        m.set_gff(key("guard", ResType::UTC), &utc).unwrap();
        let mut dlg = Gff::new(*b"DLG ");
        let mut entry = Struct::new(0);
        entry.set(
            "Text",
            Value::LocString(LocString::from_text(Language::ENGLISH, Gender::Male, "Halt!")),
        );
        entry.set("Script", Value::resref(rr("guard_spawn")));
        dlg.root.set("EntryList", Value::List(vec![entry]));
        m.set_gff(key("guard_talk", ResType::DLG), &dlg).unwrap();
        m.set(
            key("guard_spawn", ResType::NSS),
            b"#include \"inc_guard\"\nvoid main() { Guard(); }\n".to_vec(),
        );
        m.set(key("guard_spawn", ResType::NCS), b"NCS V1.0".to_vec());
        m.set(key("inc_guard", ResType::NSS), b"void Guard() {}\n".to_vec());
        m.set(
            key("on_load", ResType::NSS),
            b"void main() {\n  // \"guard_spawn\" in a comment\n  ExecuteScript(\"GUARD_SPAWN\", OBJECT_SELF);\n  object o = GetObjectByTag(\"GUARD\");\n}\n"
                .to_vec(),
        );
        m
    }

    #[test]
    fn usages_have_readable_places() {
        let m = module();
        let places: Vec<String> =
            usages(&m, key("guard_spawn", ResType::NSS)).into_iter().map(|u| u.place).collect();
        assert_eq!(
            places,
            [
                "guard.utc › OnSpawn",
                "guard_talk › NPC line 1 “Halt!” › action",
                "keep › creature GUARD › OnSpawn",
            ]
        );
        let inc = usages(&m, key("inc_guard", ResType::NSS));
        assert_eq!(inc.len(), 1);
        assert_eq!((inc[0].from, inc[0].line()), (key("guard_spawn", ResType::NSS), Some(1)));
        let area = usages(&m, key("keep", ResType::ARE));
        let places: Vec<&str> = area.iter().map(|u| u.place.as_str()).collect();
        assert_eq!(places, ["Module › area list", "Module › start area"]);
        let creature = usages(&m, key("guard", ResType::UTC));
        assert_eq!(creature[0].instance(), Some(("Creature List", 0)));

        let tags: Vec<String> = tag_usages(&m, "GUARD").into_iter().map(|u| u.place).collect();
        assert_eq!(tags, ["keep › creature GUARD › Tag", "keep › door GATE › LinkedTo"]);
        assert_eq!(mentions(&m, "GUARD", false).len(), 1);
        let m2 = mentions(&m, "guard_spawn", true);
        assert_eq!(m2.len(), 1, "comments don't count");
        assert_eq!(m2[0].line, 3);
    }

    #[test]
    fn renaming_a_script_rewrites_references_and_asks_for_recompiling() {
        let mut m = module();
        let r = rename(&mut m, key("guard_spawn", ResType::NSS), rr("guard_wake"), true).unwrap();
        assert!(!m.contains(&key("guard_spawn", ResType::NSS)));
        assert!(m.contains(&key("guard_wake", ResType::NSS)));
        assert!(m.contains(&key("guard_wake", ResType::NCS)), "compiled code moves with it");
        assert_eq!(r.references, 3);
        assert_eq!(r.in_scripts, 1);
        assert_eq!(r.recompile, [key("on_load", ResType::NSS)]);
        assert!(!m.contains(&key("on_load", ResType::NCS)));
        let src =
            String::from_utf8(m.get(&key("on_load", ResType::NSS)).unwrap().to_vec()).unwrap();
        assert!(src.contains("ExecuteScript(\"guard_wake\"") && src.contains("// \"guard_spawn\""));
        assert!(usages(&m, key("guard_spawn", ResType::NSS)).is_empty());
        assert_eq!(usages(&m, key("guard_wake", ResType::NSS)).len(), 3);
        // The same again is refused; the old name is gone.
        assert_eq!(
            rename(&mut m, key("guard_spawn", ResType::NSS), rr("x"), false),
            Err(RenameError::Missing(key("guard_spawn", ResType::NSS)))
        );
    }

    /// The index answers as the scan does, and reads again only what
    /// changed: a resource set anew, one added, one removed.
    #[test]
    fn the_index_answers_as_the_scan_and_reads_only_what_changed() {
        let mut m = module();
        let targets = [
            key("guard_spawn", ResType::NSS),
            key("inc_guard", ResType::NSS),
            key("keep", ResType::ARE),
            key("guard", ResType::UTC),
        ];
        let mut index = UsageIndex::default();
        let same = |index: &UsageIndex, m: &Module| {
            for t in targets {
                assert_eq!(index.usages(t), usages(m, t), "{t}");
                let name = t.resref.to_string();
                assert_eq!(index.mentions(&name, true), mentions(m, &name, true), "{name}");
            }
            for tag in ["GUARD", "guard", "KEEP", ""] {
                assert_eq!(index.tag_usages(tag), tag_usages(m, tag), "{tag}");
                assert_eq!(index.mentions(tag, false), mentions(m, tag, false), "{tag}");
            }
        };
        index.refresh(&m);
        assert_eq!(index.read, m.len());
        assert!(!index.usages(targets[0]).is_empty());
        assert!(!index.tag_usages("GUARD").is_empty());
        assert!(!index.mentions("guard_spawn", true).is_empty());
        same(&index, &m);
        index.refresh(&m);
        assert_eq!(index.read, 0, "nothing changed");
        // A script that names the include no more; one that newly does.
        m.set(key("guard_spawn", ResType::NSS), b"void main() {}\n".to_vec());
        m.set(key("fresh", ResType::NSS), b"#include \"inc_guard\"\nvoid main() {}\n".to_vec());
        index.refresh(&m);
        assert_eq!(index.read, 2);
        same(&index, &m);
        m.remove(&key("fresh", ResType::NSS));
        index.refresh(&m);
        assert_eq!(index.read, 0);
        // Talk-table lines: a blueprint's name by StrRef, and a 2DA's
        // cell in a column that holds one; a text of its own is no line.
        let mut item = Gff::new(*b"UTI ");
        let named = mg_core::LocString { strref: mg_core::StrRef(16_777_220), strings: Vec::new() };
        item.root.set("LocalizedName", Value::LocString(named));
        let own = mg_core::LocString { strref: mg_core::StrRef::NONE, strings: Vec::new() };
        item.root.set("Description", Value::LocString(own));
        m.set_gff(key("sword", ResType::UTI), &item).unwrap();
        let table = b"2DA V2.0\n\n   Label StrRef ModelName\n0  Chair 16777220 plc_chair\n1  Stool 5 plc_stool\n";
        m.set(key("placeables", ResType::TWODA), table.to_vec());
        index.refresh(&m);
        let places: Vec<String> =
            index.strref_usages(16_777_220).into_iter().map(|u| u.place).collect();
        assert_eq!(places.len(), 2, "{places:?}");
        assert!(places[0].contains("placeables.2da") && places[0].contains("row 0, StrRef"));
        assert!(places[1].starts_with("sword"), "{places:?}");
        assert_eq!(index.strref_usages(5).len(), 1);
        assert!(index.strref_usages(u32::MAX).is_empty());
        // Set again with the bytes it had: not read again.
        let again = m.get(&key("inc_guard", ResType::NSS)).unwrap().to_vec();
        m.set(key("inc_guard", ResType::NSS), again);
        index.refresh(&m);
        assert_eq!(index.read, 0);
        same(&index, &m);
    }

    #[test]
    fn changed_includes_recompile_their_includers() {
        let mut m = module();
        m.set(
            key("inc_guard", ResType::NSS),
            b"void Guard() { ExecuteScript(\"guard_spawn\", OBJECT_SELF); }\n".to_vec(),
        );
        m.set(key("top", ResType::NSS), b"#include \"guard_spawn\"\n".to_vec());
        m.set(key("top", ResType::NCS), b"NCS V1.0".to_vec());
        let r = rename(&mut m, key("guard_spawn", ResType::NSS), rr("guard_wake"), true).unwrap();
        // inc_guard's string changed; guard_wake includes it; top includes guard_wake.
        let names: Vec<String> = r.recompile.iter().map(|k| k.resref.to_string()).collect();
        assert_eq!(names, ["guard_wake", "inc_guard", "on_load", "top"]);
        assert!(
            !m.contains(&key("guard_wake", ResType::NCS)) && !m.contains(&key("top", ResType::NCS))
        );
    }

    #[test]
    fn renaming_an_include_rewrites_include_lines() {
        let mut m = module();
        let r = rename(&mut m, key("inc_guard", ResType::NSS), rr("inc_watch"), false).unwrap();
        assert_eq!(r.in_scripts, 1);
        let src = m.get(&key("guard_spawn", ResType::NSS)).unwrap();
        assert!(src.starts_with(b"#include \"inc_watch\"\n"));
        assert!(!m.contains(&key("guard_spawn", ResType::NCS)), "to be compiled again");
    }

    #[test]
    fn renaming_an_area_moves_its_files_and_the_module_follows() {
        let mut m = module();
        rename(&mut m, key("keep", ResType::GIT), rr("castle_keep"), false).unwrap();
        for t in [ResType::ARE, ResType::GIT, ResType::GIC] {
            assert!(m.contains(&key("castle_keep", t)) && !m.contains(&key("keep", t)));
        }
        assert_eq!(m.areas().unwrap(), [rr("castle_keep")]);
        let info = m.info().unwrap();
        assert_eq!(info.root.read(&ifo::MOD_ENTRY_AREA), rr("castle_keep"));
        let are = m.gff(&key("castle_keep", ResType::ARE)).unwrap().unwrap();
        assert_eq!(are.root.get("ResRef").and_then(Value::as_resref), Some(rr("castle_keep")));
    }

    #[test]
    fn renaming_a_blueprint_updates_instances_and_its_own_name() {
        let mut m = module();
        let r = rename(&mut m, key("guard", ResType::UTC), rr("gate_guard"), false).unwrap();
        assert_eq!(r.references, 1);
        let utc = m.gff(&key("gate_guard", ResType::UTC)).unwrap().unwrap();
        assert_eq!(
            utc.root.get("TemplateResRef").and_then(Value::as_resref),
            Some(rr("gate_guard"))
        );
        let g = m.gff(&key("keep", ResType::GIT)).unwrap().unwrap();
        assert_eq!(
            g.root.items(&git::CREATURE_LIST)[0].read(&git::creature_list::TEMPLATE_RES_REF),
            rr("gate_guard")
        );
        // Taken names and types without references are refused.
        assert_eq!(
            rename(&mut m, key("gate_guard", ResType::UTC), rr("gate_guard"), false),
            Err(RenameError::SameName)
        );
        m.set_gff(key("other", ResType::UTC), &Gff::new(*b"UTC ")).unwrap();
        assert!(matches!(
            rename(&mut m, key("other", ResType::UTC), rr("gate_guard"), false),
            Err(RenameError::Exists(_))
        ));
        assert!(matches!(
            rename(&mut m, key("module", ResType::IFO), rr("x"), false),
            Err(RenameError::NotRenamable(_))
        ));
    }
}
