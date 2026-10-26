//! Several areas' properties at once (`mg areas`, and what the window's
//! Edit Areas Together does): the module's areas, narrowed by name, tileset
//! and kind, and fields, flags and variables set on each.
//!
//! An area's properties are its ARE's root fields, and, for its ambient
//! sounds and music, the `AreaProperties` struct of its GIT. A field keeps
//! the type it has; one an area lacks takes the type the game's areas have
//! for it.

use mg_core::{Codepage, Language, ResRef, ResType};
use mg_gff::{FieldType, Struct, Value};
use mg_resman::ResKey;

use crate::Module;
use crate::project_file::StaticRules;
use crate::vars::VarType;

/// ARE `Flags` bits.
pub const INTERIOR: u32 = 0x1;
pub const UNDERGROUND: u32 = 0x2;
pub const NATURAL: u32 = 0x4;

/// The flags as fields to set (`Interior=yes`).
const FLAG_FIELDS: [(&str, u32); 3] =
    [("Interior", INTERIOR), ("Underground", UNDERGROUND), ("Natural", NATURAL)];

/// The GIT's `AreaProperties` fields (all ints): ambient sounds and music.
const AUDIO_FIELDS: [&str; 9] = [
    "AmbientSndDay",
    "AmbientSndDayVol",
    "AmbientSndNight",
    "AmbientSndNitVol",
    "EnvAudio",
    "MusicBattle",
    "MusicDay",
    "MusicDelay",
    "MusicNight",
];

/// What lists of areas show and filter by.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AreaInfo {
    pub resref: ResRef,
    /// Its name's English text, else its first text; empty when it has only
    /// a talk-table string.
    pub name: String,
    pub tag: String,
    pub tileset: ResRef,
    /// `Flags`: [`INTERIOR`], [`UNDERGROUND`], [`NATURAL`], and shader flags.
    pub flags: u32,
}

/// Every area the module has (each ARE), by ResRef.
pub fn list(m: &Module) -> Vec<AreaInfo> {
    list_in(m, mg_core::Codepage::WINDOWS_1252)
}

/// [`list`] with the game's codepage, where a module has its own table
/// (`encoding.2da`).
pub fn list_in(m: &Module, game: mg_core::Codepage) -> Vec<AreaInfo> {
    let mut out: Vec<AreaInfo> = m
        .keys_of(ResType::ARE)
        .filter_map(|k| {
            let are = m.gff(k)?.ok()?;
            let name = are.root.locstring("Name").map_or(String::new(), |n| {
                let english = n.text_in(Language::ENGLISH, mg_core::Gender::Male, game);
                let any = || {
                    (n.strings.first())
                        .map(|(k, t)| game.for_language(k.language()).decode(t).into_owned())
                };
                english.map(|t| t.into_owned()).or_else(any).unwrap_or_default()
            });
            Some(AreaInfo {
                resref: k.resref,
                name,
                tag: String::from_utf8_lossy(are.root.string("Tag").unwrap_or_default()).into(),
                tileset: are.root.resref("Tileset").unwrap_or(ResRef::EMPTY),
                flags: are.root.integer("Flags").unwrap_or(0) as u32,
            })
        })
        .collect();
    out.sort_by_key(|a| a.resref);
    out
}

/// Which areas: all the conditions given must hold.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Filter {
    /// These areas only (none: any).
    pub only: Vec<ResRef>,
    /// Text in the name, the tag or the ResRef, whatever its case and
    /// accents.
    pub text: Option<String>,
    pub tileset: Option<ResRef>,
    pub interior: Option<bool>,
    pub underground: Option<bool>,
    pub natural: Option<bool>,
}

impl Filter {
    /// Whether it narrows the areas at all.
    pub fn is_empty(&self) -> bool {
        *self == Filter::default()
    }

    pub fn matches(&self, a: &AreaInfo) -> bool {
        let text = self.text.as_deref().map(mg_core::text::Needle::new);
        let has = |s: &str| text.as_ref().is_none_or(|t| t.finds(s));
        (self.only.is_empty() || self.only.contains(&a.resref))
            && (has(&a.name) || has(&a.tag) || has(&a.resref.to_string()))
            && self.tileset.is_none_or(|t| t == a.tileset)
            && [(self.interior, INTERIOR), (self.underground, UNDERGROUND), (self.natural, NATURAL)]
                .iter()
                .all(|(want, bit)| want.is_none_or(|w| (a.flags & bit != 0) == w))
    }
}

/// A change to make to each area.
#[derive(Debug, Clone, PartialEq)]
pub enum Change {
    /// A field (an ARE root field, an `AreaProperties` field, or a flag:
    /// `Interior`, `Underground`, `Natural`) and its value as text.
    Set { field: String, value: String },
    /// A scripting variable: its type (1 int, 2 float, 3 string; `None`: the
    /// type the area's variable of that name has, else what the value
    /// reads as) and its value.
    Var { name: String, kind: Option<u32>, value: String },
    /// A scripting variable to delete.
    RemoveVar(String),
    /// Makes the area's placeables static where nothing is lost by it
    /// ([`static_plan`]).
    StaticPlaceables,
    /// Makes every static placeable of the area dynamic.
    DynamicPlaceables,
    /// Grows grass of the area's own on a surface material (in place of
    /// what it grows there): `SetAreaGrassOverride` from the start.
    Grass(crate::grass::GrassOverride),
    /// Takes the area's own grass off a surface material.
    RemoveGrass(u32),
    /// The tileset's grass grows (`true`) or not: `GrassDefDisabled`.
    TilesetGrass(bool),
    /// A lighting scheme (an environment.2da row), as Area Properties'
    /// Lighting Scheme gives it: the area's lighting, fog, shadows, day
    /// and night and weather, and each tile's lights picked at random from
    /// the scheme's colors.
    LightingScheme(crate::new::Scheme),
}

impl Change {
    /// `FIELD=VALUE`.
    pub fn parse_set(text: &str) -> Result<Change, String> {
        let (field, value) =
            text.split_once('=').ok_or_else(|| format!("{text}: expected FIELD=VALUE"))?;
        if field.trim().is_empty() {
            return Err(format!("{text}: no field"));
        }
        Ok(Change::Set { field: field.trim().into(), value: value.into() })
    }

    /// `NAME=VALUE`, or `NAME:int=VALUE` (`int`, `float` or `string`).
    pub fn parse_var(text: &str) -> Result<Change, String> {
        let (name, value) =
            text.split_once('=').ok_or_else(|| format!("{text}: expected NAME=VALUE"))?;
        let (name, kind) = match name.rsplit_once(':') {
            Some((n, "int")) => (n, Some(1)),
            Some((n, "float")) => (n, Some(2)),
            Some((n, "string")) => (n, Some(3)),
            Some((_, other)) => return Err(format!("{other}: not int, float or string")),
            None => (name, None),
        };
        if name.trim().is_empty() {
            return Err(format!("{text}: no variable name"));
        }
        Ok(Change::Var { name: name.trim().into(), kind, value: value.into() })
    }
}

/// Which of an area's placeables can be made static, and why the others
/// are left: the game merges static placeables into the area's tiles when
/// it loads, so they cost less to load and draw, but they can't be used,
/// tilted or scaled, play no animations, are Plot and can't be destroyed
/// from sight. (The server keeps one as an object, which scripts find by
/// its tag and whose variables they read, `engine_static_placeables`; but
/// what they do to it, the players don't see.)
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StaticPlan {
    /// To make static: their places in `Placeable List`.
    pub convert: Vec<usize>,
    /// Static already.
    pub already: usize,
    /// Left dynamic: Useable.
    pub useable: usize,
    /// Left dynamic: tilted, scaled or moved by a visual transform.
    pub transformed: usize,
    /// Left dynamic: with a script, a conversation, a trap or an
    /// inventory, or switched on (its animation state).
    pub active: usize,
    /// Left dynamic: with variables (those the project names, else any),
    /// which scripts set or read on it.
    pub variables: usize,
    /// Left dynamic: the project's rules name its tag or its blueprint.
    pub project: usize,
    /// Each left dynamic, and why.
    pub kept: Vec<(usize, Kept)>,
}

/// Why Make Placeables Static leaves a placeable dynamic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Kept {
    Useable,
    Transformed,
    /// A script (`OnUsed`), `Conversation`, `TrapFlag`, `HasInventory` or
    /// `AnimationState`: the field.
    Active(String),
    /// A variable, by name.
    Variable(String),
    /// The project's `keep_tags` pattern its tag matches.
    Tag(String),
    /// The project's `keep_resrefs` pattern its blueprint matches.
    ResRef(String),
}

impl Kept {
    /// Why, in a few words: `it has the variable tdod_light`.
    pub fn describe(&self) -> String {
        match self {
            Kept::Useable => "it is Useable".into(),
            Kept::Transformed => "a visual transform tilts, scales or moves it".into(),
            Kept::Active(field) => format!("it has {field}"),
            Kept::Variable(name) => format!("it has the variable {name}"),
            Kept::Tag(p) => format!("the project keeps tags {p} dynamic"),
            Kept::ResRef(p) => format!("the project keeps blueprints {p} dynamic"),
        }
    }
}

impl StaticPlan {
    /// What was left dynamic, by reason, for a line of text (`2 Useable,
    /// 1 with variables`); empty for none.
    pub fn left(&self) -> String {
        let parts = [
            (self.useable, "Useable"),
            (self.transformed, "with a visual transform"),
            (self.active, "with a script, conversation, trap, inventory or animation"),
            (self.variables, "with variables"),
            (self.project, "kept by the project's rules"),
        ];
        let said: Vec<String> =
            parts.iter().filter(|(n, _)| *n > 0).map(|(n, what)| format!("{n} {what}")).collect();
        said.join(", ")
    }

    /// Adds another area's plan to this one's counts (not its places).
    pub fn add(&mut self, other: &StaticPlan) {
        self.already += other.already;
        self.useable += other.useable;
        self.transformed += other.transformed;
        self.active += other.active;
        self.variables += other.variables;
        self.project += other.project;
    }
}

/// Whether a placed object's visual transform (EE's `VisTransformList`,
/// or the older `VisualTransform`) changes how its model shows.
pub(crate) fn transforms(s: &Struct) -> bool {
    const AXES: [(&str, f32); 3] = [("Scale", 1.0), ("Rotate", 0.0), ("Translate", 0.0)];
    let changed = |get: &dyn Fn(&str) -> Option<f32>| {
        AXES.iter().any(|(prefix, rest)| {
            ["X", "Y", "Z"].iter().any(|a| get(&format!("{prefix}{a}")).unwrap_or(*rest) != *rest)
        })
    };
    let listed =
        s.list("VisTransformList").unwrap_or(&[]).iter().any(|entry| {
            changed(&|label: &str| entry.child(label).and_then(|c| c.float("ValueTo")))
        });
    listed || s.child("VisualTransform").is_some_and(|v| changed(&|label: &str| v.float(label)))
}

/// Why a placeable that isn't static is to stay dynamic, if it is.
fn kept(p: &Struct, rules: &StaticRules) -> Option<Kept> {
    let on = |label: &str| p.integer(label).unwrap_or(0) != 0;
    if on("Useable") {
        return Some(Kept::Useable);
    }
    if transforms(p) {
        return Some(Kept::Transformed);
    }
    let script = p.fields.iter().find(|f| {
        let empty = matches!(&f.value, Value::ResRef(r) if r.is_empty());
        f.label.as_bytes().starts_with(b"On") && matches!(f.value, Value::ResRef(_)) && !empty
    });
    if let Some(f) = script {
        return Some(Kept::Active(f.label.to_string_lossy().to_string()));
    }
    if p.resref("Conversation").is_some_and(|r| !r.is_empty()) {
        return Some(Kept::Active("Conversation".into()));
    }
    for label in ["TrapFlag", "HasInventory", "AnimationState"] {
        if on(label) {
            return Some(Kept::Active(label.into()));
        }
    }
    let text = |b: &[u8]| String::from_utf8_lossy(b).into_owned();
    for v in p.list("VarTable").unwrap_or(&[]) {
        let name = text(v.string("Name").unwrap_or_default());
        if rules.keep_variables.matches(&name) {
            return Some(Kept::Variable(name));
        }
    }
    let tag = text(p.string("Tag").unwrap_or_default());
    if let Some(glob) = rules.keep_tags.matching(&tag) {
        return Some(Kept::Tag(glob.into()));
    }
    let resref = p.resref("TemplateResRef").map(|r| r.to_string()).unwrap_or_default();
    if let Some(glob) = rules.keep_resrefs.matching(&resref).filter(|_| !resref.is_empty()) {
        return Some(Kept::ResRef(glob.into()));
    }
    None
}

/// What making an area's placeables static would do (`git`: the area's
/// GIT root), by the project's rules (`StaticRules::default()` without a
/// project file: a placeable with any variable stays dynamic).
pub fn static_plan(git: &Struct, rules: &StaticRules) -> StaticPlan {
    let mut plan = StaticPlan::default();
    for (i, p) in git.list("Placeable List").unwrap_or(&[]).iter().enumerate() {
        if p.integer("Static").unwrap_or(0) != 0 {
            plan.already += 1;
            continue;
        }
        let Some(why) = kept(p, rules) else {
            plan.convert.push(i);
            continue;
        };
        match &why {
            Kept::Useable => plan.useable += 1,
            Kept::Transformed => plan.transformed += 1,
            Kept::Active(_) => plan.active += 1,
            Kept::Variable(_) => plan.variables += 1,
            Kept::Tag(_) | Kept::ResRef(_) => plan.project += 1,
        }
        plan.kept.push((i, why));
    }
    plan
}

/// A placed object's visual transform: EE's list, and the older struct.
pub const VISUAL_TRANSFORM_FIELDS: [&str; 2] = ["VisTransformList", "VisualTransform"];

/// The fields to change to make placeable `p` static: `Static` set, and
/// its visual transform taken off (a static placeable has none; it would
/// change nothing). Each with its new value (`None`: removed).
pub fn static_fields(p: &Struct) -> Vec<(&'static str, Option<Value>)> {
    let mut out = vec![("Static", Some(Value::Byte(1)))];
    for label in VISUAL_TRANSFORM_FIELDS {
        if p.contains(label) {
            out.push((label, None));
        }
    }
    out
}

/// What a change did to one area.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Changed {
    pub area: ResRef,
    /// The field, or `var NAME`.
    pub what: String,
    /// As text; `None`: it wasn't there.
    pub from: Option<String>,
    /// `None`: removed.
    pub to: Option<String>,
}

/// A value as `mg areas` prints it (text in `codepage`).
fn shown(v: &Value, codepage: Codepage) -> String {
    match v {
        Value::String(b) => codepage.decode(b).into_owned(),
        Value::ResRef(b) => String::from_utf8_lossy(b).into_owned(),
        Value::Byte(i) => i.to_string(),
        Value::Char(i) => i.to_string(),
        Value::Word(i) => i.to_string(),
        Value::Short(i) => i.to_string(),
        Value::Dword(i) => i.to_string(),
        Value::Int(i) => i.to_string(),
        Value::Dword64(i) => i.to_string(),
        Value::Int64(i) => i.to_string(),
        Value::Float(f) => f.to_string(),
        Value::Double(f) => f.to_string(),
        other => format!("{other:?}"),
    }
}

/// `yes` or `no` (`1`, `true`, `on`; `0`, `false`, `off`).
pub fn yes_no(text: &str) -> Option<bool> {
    match text.trim().to_ascii_lowercase().as_str() {
        "1" | "yes" | "true" | "on" => Some(true),
        "0" | "no" | "false" | "off" => Some(false),
        _ => None,
    }
}

/// Sets `field` of `s`, keeping its type (else `default`); what changed.
fn set_field(
    s: &mut Struct,
    field: &str,
    default: Option<FieldType>,
    text: &str,
    codepage: Codepage,
) -> Result<Option<(Option<String>, String)>, String> {
    let t = s
        .get(field)
        .map(Value::field_type)
        .or(default)
        .ok_or_else(|| format!("{field}: not a field of an area"))?;
    // (An area's name has every language: not one set as text here.)
    if t == FieldType::LocString {
        return Err(format!("{field} is a {}: not a field this sets", t.json_name()));
    }
    let value =
        Value::from_text(t, text, s.get(field), codepage).map_err(|e| format!("{field}: {e}"))?;
    if s.get(field) == Some(&value) {
        return Ok(None);
    }
    let shown = |v: &Value| shown(v, codepage);
    let from = s.get(field).map(shown);
    let to = shown(&value);
    s.set(field, value);
    Ok(Some((from, to)))
}

/// Sets a variable of `vars` (in place, else at the end); what changed.
fn set_var(
    vars: &mut Vec<Struct>,
    name: &str,
    kind: Option<u32>,
    text: &str,
    codepage: Codepage,
) -> Result<Option<(Option<String>, String)>, String> {
    let named = codepage.encode_lossy(name);
    let at = vars.iter().position(|v| v.string("Name") == Some(&named[..]));
    let kind = kind.or_else(|| at.and_then(|i| vars[i].dword("Type")));
    let kind = match kind {
        Some(n) => VarType::of(n.into()),
        None => Some(VarType::of_text(text)),
    };
    let value = match kind {
        Some(t) => t.value(text, codepage).map_err(|e| format!("{name}: {e}"))?,
        None => return Err(format!("{name} is a variable of a type this doesn't set")),
    };
    let kind = kind.expect("valued").code();
    let mut var = at.map_or_else(|| Struct::new(0), |i| vars[i].clone());
    let shown = |v: &Value| shown(v, codepage);
    let from = var.get("Value").map(shown);
    var.set("Name", Value::String(named));
    var.set("Type", Value::Dword(kind));
    var.set("Value", value.clone());
    match at {
        Some(i) if vars[i] == var => return Ok(None),
        Some(i) => vars[i] = var,
        None => vars.push(var),
    }
    Ok(Some((from, shown(&value))))
}

/// Makes `changes` to each of `areas`, each area keeping what they don't
/// name (its other flags and variables). Nothing is changed unless every
/// change can be made to every area. Returns what changed (an area that
/// already had a value isn't listed for it). The module isn't saved. Text
/// is written in `codepage`, as the game reads a module's strings.
pub fn apply(
    m: &mut Module,
    areas: &[ResRef],
    changes: &[Change],
    codepage: Codepage,
) -> Result<Vec<Changed>, String> {
    let mut done = Vec::new();
    let mut write = Vec::new();
    let rules = m.project_file.make_static.clone();
    // (A lighting scheme's colors for each tile.)
    let mut rng = fastrand::Rng::new();
    for &area in areas {
        let are_key = ResKey::new(area, ResType::ARE);
        let git_key = ResKey::new(area, ResType::GIT);
        let read = |key: &ResKey| match m.gff(key) {
            Some(Ok(g)) => Ok(Some(g)),
            Some(Err(e)) => Err(format!("{key}: {e}")),
            None => Ok(None),
        };
        let mut are =
            read(&are_key)?.ok_or_else(|| format!("{area} is not an area of the module"))?;
        let mut git = read(&git_key)?;
        let (mut are_changed, mut git_changed) = (false, false);
        let mut note = |what: String, from: Option<String>, to: Option<String>| {
            done.push(Changed { area, what, from, to });
        };
        // An area's variables are its GIT's: the game reads none from its
        // ARE (`engine_area_variables`), where Moonglow wrote them before
        // 1.34.3. Those are moved to the GIT first (one of the same name
        // there keeps its value).
        let vars_changed =
            changes.iter().any(|c| matches!(c, Change::Var { .. } | Change::RemoveVar(_)));
        if vars_changed {
            let git = git.as_mut().ok_or_else(|| format!("{area} has no GIT for its variables"))?;
            let in_are = are.root.list("VarTable").unwrap_or(&[]).to_vec();
            if !in_are.is_empty() {
                let mut vars = git.root.list("VarTable").unwrap_or(&[]).to_vec();
                for v in in_are {
                    if !vars.iter().any(|g| g.string("Name") == v.string("Name")) {
                        vars.push(v);
                    }
                }
                git.root.set("VarTable", Value::List(vars));
                are.root.remove("VarTable");
                (are_changed, git_changed) = (true, true);
                note("variables moved from the ARE to the GIT".into(), None, None);
            }
        }
        for change in changes {
            match change {
                Change::Set { field, value } => {
                    if let Some((name, bit)) =
                        FLAG_FIELDS.iter().find(|(n, _)| n.eq_ignore_ascii_case(field))
                    {
                        let on = yes_no(value)
                            .ok_or_else(|| format!("{name}: {value:?} is not yes or no"))?;
                        let flags = are.root.integer("Flags").unwrap_or(0) as u32;
                        if (flags & bit != 0) != on {
                            let new = if on { flags | bit } else { flags & !bit };
                            are.root.set("Flags", Value::Dword(new));
                            are_changed = true;
                            let text = |b: bool| Some(if b { "yes" } else { "no" }.to_string());
                            note((*name).into(), text(!on), text(on));
                        }
                    } else if AUDIO_FIELDS.contains(&field.as_str()) {
                        let props = git
                            .as_mut()
                            .and_then(|g| g.root.child_mut("AreaProperties"))
                            .ok_or_else(|| {
                            format!("{area} has no audio settings (its GIT lacks AreaProperties)")
                        })?;
                        if let Some((from, to)) =
                            set_field(props, field, Some(FieldType::Int), value, codepage)?
                        {
                            git_changed = true;
                            note(field.clone(), from, Some(to));
                        }
                    } else {
                        let default = mg_schema::root_field_type(ResType::ARE, field);
                        if let Some((from, to)) =
                            set_field(&mut are.root, field, default, value, codepage)?
                        {
                            are_changed = true;
                            note(field.clone(), from, Some(to));
                        }
                    }
                }
                Change::Var { name, kind, value } => {
                    let Some(git) = git.as_mut() else { continue };
                    let mut vars = git.root.list("VarTable").unwrap_or(&[]).to_vec();
                    if let Some((from, to)) = set_var(&mut vars, name, *kind, value, codepage)? {
                        git.root.set("VarTable", Value::List(vars));
                        git_changed = true;
                        note(format!("var {name}"), from, Some(to));
                    }
                }
                Change::StaticPlaceables => {
                    let Some(git) = git.as_mut() else { continue };
                    let plan = static_plan(&git.root, &rules);
                    let Some(list) = git.root.list_mut("Placeable List") else { continue };
                    for &i in &plan.convert {
                        let p = &mut list[i];
                        for (label, value) in static_fields(p) {
                            match value {
                                Some(v) => p.set(label, v),
                                None => _ = p.remove(label),
                            }
                        }
                        let tag = String::from_utf8_lossy(p.string("Tag").unwrap_or_default());
                        note(
                            format!("placeable {i} {tag} Static"),
                            Some("0".into()),
                            Some("1".into()),
                        );
                        git_changed = true;
                    }
                }
                Change::DynamicPlaceables => {
                    let Some(list) = git.as_mut().and_then(|g| g.root.list_mut("Placeable List"))
                    else {
                        continue;
                    };
                    for (i, p) in list.iter_mut().enumerate() {
                        if p.integer("Static").unwrap_or(0) != 0 {
                            p.set("Static", Value::Byte(0));
                            let tag = String::from_utf8_lossy(p.string("Tag").unwrap_or_default());
                            let what = format!("placeable {i} {tag} Static");
                            note(what, Some("1".into()), Some("0".into()));
                            git_changed = true;
                        }
                    }
                }
                Change::Grass(_) | Change::RemoveGrass(_) | Change::TilesetGrass(_) => {
                    use crate::grass;
                    let props = git
                        .as_mut()
                        .and_then(|g| g.root.child_mut(grass::PROPERTIES))
                        .ok_or_else(|| {
                            format!("{area} has no grass settings (its GIT lacks AreaProperties)")
                        })?;
                    let shown = |o: &grass::GrassOverride| o.look.describe();
                    match change {
                        Change::Grass(o) => {
                            if let Some(was) = grass::set_in(props, o) {
                                let what = format!("grass on material {}", o.material);
                                note(what, was.as_ref().map(shown), Some(shown(o)));
                                git_changed = true;
                            }
                        }
                        Change::RemoveGrass(material) => {
                            if let Some(was) = grass::remove_in(props, *material) {
                                note(
                                    format!("grass on material {material}"),
                                    Some(shown(&was)),
                                    None,
                                );
                                git_changed = true;
                            }
                        }
                        Change::TilesetGrass(on) => {
                            if grass::set_disabled_in(props, !on) {
                                let text = |b: bool| Some(if b { "yes" } else { "no" }.to_string());
                                note("tileset grass".into(), text(!on), text(*on));
                                git_changed = true;
                            }
                        }
                        _ => unreachable!(),
                    }
                }
                Change::LightingScheme(scheme) => {
                    let was = are.root.get("LightingScheme").map(|v| shown(v, codepage));
                    let mut fields_changed = false;
                    for (label, value) in scheme.applied_fields() {
                        if are.root.get(label) != Some(&value) {
                            are.root.set(label, value);
                            fields_changed = true;
                        }
                    }
                    if fields_changed {
                        are_changed = true;
                        note("LightingScheme".into(), was, Some(scheme.row.to_string()));
                    }
                    let (mut n, mut lit) = (0, 0);
                    for tile in are.root.list_mut("Tile_List").into_iter().flatten() {
                        let mut changed = false;
                        for (label, value) in scheme.tile_fields(&mut rng) {
                            changed |= tile.get(label) != Some(&value);
                            tile.set(label, value);
                        }
                        (n, lit) = (n + 1, lit + usize::from(changed));
                    }
                    if lit > 0 {
                        are_changed = true;
                        let what = format!("lights of {n} tile{}", if n == 1 { "" } else { "s" });
                        note(
                            what,
                            Some("their own".into()),
                            Some(format!("scheme {}'s", scheme.row)),
                        );
                    }
                }
                Change::RemoveVar(name) => {
                    let Some(git) = git.as_mut() else { continue };
                    let mut vars = git.root.list("VarTable").unwrap_or(&[]).to_vec();
                    let named = codepage.encode_lossy(name);
                    let at = vars.iter().position(|v| v.string("Name") == Some(&named[..]));
                    if let Some(i) = at {
                        let from = vars.remove(i).get("Value").map(|v| shown(v, codepage));
                        git.root.set("VarTable", Value::List(vars));
                        git_changed = true;
                        note(format!("var {name}"), from, None);
                    }
                }
            }
        }
        if are_changed {
            write.push((are_key, are));
        }
        if let (true, Some(git)) = (git_changed, git) {
            write.push((git_key, git));
        }
    }
    for (key, gff) in write {
        m.set_gff(key, &gff).map_err(|e| format!("{key}: {e}"))?;
    }
    Ok(done)
}

#[cfg(test)]
mod tests {
    use super::*;
    use mg_core::{Gender, LocString};
    use mg_gff::Gff;

    const CP: Codepage = Codepage::WINDOWS_1252;

    fn r(s: &str) -> ResRef {
        ResRef::from_str(s).unwrap()
    }

    fn var(name: &str, kind: u32, value: Value) -> Struct {
        let mut s = Struct::new(0);
        s.set("Name", Value::String(name.as_bytes().to_vec()));
        s.set("Type", Value::Dword(kind));
        s.set("Value", value);
        s
    }

    /// Two caves and an inn.
    fn module() -> Module {
        let mut m = Module::new();
        for (name, title, tileset, flags, vars) in [
            ("cave1", "Wolf Cave", "tdc01", 0x7u32, vec![var("nDepth", 1, Value::Int(1))]),
            ("cave2", "Bear Cave", "tdc01", 0x3 | 0x100, Vec::new()),
            (
                "inn",
                "Apple Inn",
                "tin01",
                0x1,
                vec![var("sHost", 3, Value::String(b"Ann".to_vec()))],
            ),
        ] {
            let mut are = Gff::new(*b"ARE ");
            let title = LocString::from_text(Language::ENGLISH, Gender::Male, title);
            are.root.set("Name", Value::LocString(title));
            are.root.set("Tag", Value::String(name.to_uppercase().into_bytes()));
            are.root.set("Tileset", Value::resref(r(tileset)));
            are.root.set("Flags", Value::Dword(flags));
            are.root.set("SunFogAmount", Value::Byte(2));
            are.root.set("VarTable", Value::List(vars));
            m.set_gff(ResKey::new(r(name), ResType::ARE), &are).unwrap();
            let mut git = Gff::new(*b"GIT ");
            let mut props = Struct::new(100);
            props.set("MusicDay", Value::Int(3));
            git.root.set("AreaProperties", Value::Struct(props));
            m.set_gff(ResKey::new(r(name), ResType::GIT), &git).unwrap();
        }
        m
    }

    fn are(m: &Module, name: &str) -> Struct {
        m.gff(&ResKey::new(r(name), ResType::ARE)).unwrap().unwrap().root
    }

    fn git(m: &Module, name: &str) -> Struct {
        m.gff(&ResKey::new(r(name), ResType::GIT)).unwrap().unwrap().root
    }

    #[test]
    fn areas_are_listed_and_filtered() {
        let m = module();
        let all = list(&m);
        let names = |f: &Filter| -> Vec<String> {
            all.iter().filter(|a| f.matches(a)).map(|a| a.resref.to_string()).collect()
        };
        assert_eq!(names(&Filter::default()), ["cave1", "cave2", "inn"]);
        assert_eq!((all[0].name.as_str(), all[0].tag.as_str()), ("Wolf Cave", "CAVE1"));
        let under = Filter { underground: Some(true), ..Default::default() };
        assert_eq!(names(&under), ["cave1", "cave2"]);
        assert!(!under.is_empty() && Filter::default().is_empty());
        let natural_caves = Filter { natural: Some(true), tileset: Some(r("tdc01")), ..under };
        assert_eq!(names(&natural_caves), ["cave1"]);
        let text = Filter { text: Some("APPLE".into()), ..Default::default() };
        assert_eq!(names(&text), ["inn"]);
        let only =
            Filter { only: vec![r("cave2"), r("inn")], interior: Some(true), ..Default::default() };
        assert_eq!(names(&only), ["cave2", "inn"]);
    }

    #[test]
    fn fields_flags_and_variables_are_set_on_each_area() {
        let mut m = module();
        let changes = [
            Change::parse_set("SunFogAmount=9").unwrap(),
            // A field the areas lack: the game's type for it (a dword).
            Change::parse_set("SunFogColor=0x102030").unwrap(),
            Change::parse_set("MusicDay=57").unwrap(),
            Change::parse_set("underground=no").unwrap(),
            Change::parse_var("nMusic=57").unwrap(),
            Change::parse_var("nDepth=4").unwrap(),
            Change::parse_var("sGreeting=café").unwrap(),
            Change::RemoveVar("sHost".into()),
        ];
        let caves = [r("cave1"), r("cave2")];
        let done = apply(&mut m, &caves, &changes, CP).unwrap();
        for name in ["cave1", "cave2"] {
            let a = are(&m, name);
            assert_eq!(a.get("SunFogAmount"), Some(&Value::Byte(9)));
            assert_eq!(a.get("SunFogColor"), Some(&Value::Dword(0x10_2030)));
            let git = m.gff(&ResKey::new(r(name), ResType::GIT)).unwrap().unwrap();
            assert_eq!(git.root.child("AreaProperties").unwrap().integer("MusicDay"), Some(57));
        }
        // Each keeps its other flags and variables. The variables are the
        // GIT's, where the game reads them: those an earlier Moonglow left
        // in the ARE (cave1's nDepth) are moved there first.
        assert_eq!(are(&m, "cave1").integer("Flags"), Some(0x5));
        assert_eq!(are(&m, "cave2").integer("Flags"), Some(0x1 | 0x100));
        assert_eq!(
            git(&m, "cave1").list("VarTable").unwrap(),
            [
                var("nDepth", 1, Value::Int(4)),
                var("nMusic", 1, Value::Int(57)),
                var("sGreeting", 3, Value::String(b"caf\xe9".to_vec())),
            ],
            "text in the codepage, not UTF-8"
        );
        assert_eq!(
            git(&m, "cave2").list("VarTable").unwrap(),
            [
                var("nMusic", 1, Value::Int(57)),
                var("nDepth", 1, Value::Int(4)),
                var("sGreeting", 3, Value::String(b"caf\xe9".to_vec())),
            ]
        );
        assert_eq!(are(&m, "cave1").list("VarTable"), None, "moved from the ARE");
        // The inn wasn't named: its variable stays where it is.
        assert_eq!(are(&m, "inn").integer("SunFogAmount"), Some(2));
        assert_eq!(are(&m, "inn").list("VarTable").unwrap().len(), 1);
        // What changed, by area.
        let fog: Vec<&Changed> = done.iter().filter(|c| c.what == "SunFogAmount").collect();
        assert_eq!(fog.len(), 2);
        assert_eq!((fog[0].from.as_deref(), fog[0].to.as_deref()), (Some("2"), Some("9")));
        // Again: nothing to change.
        assert!(apply(&mut m, &caves, &changes, CP).unwrap().is_empty());
    }

    /// A placed placeable with these fields.
    fn placeable(tag: &str, fields: &[(&str, Value)]) -> Struct {
        let mut p = Struct::new(9);
        p.set("Tag", Value::String(tag.as_bytes().to_vec()));
        p.set("Static", Value::Byte(0));
        p.set("Useable", Value::Byte(0));
        p.set("OnHeartbeat", Value::resref(ResRef::EMPTY));
        for (label, value) in fields {
            p.set(label, value.clone());
        }
        p
    }

    /// A VarTable of one string variable.
    fn vars_named(name: &str) -> Value {
        Value::List(vec![var(name, 3, Value::String(b"x".to_vec()))])
    }

    /// A placeable with variables stays dynamic (scripts set or read
    /// them, and what they do to it a static one wouldn't show), and so
    /// do those the project's rules name; each with why.
    #[test]
    fn variables_and_the_project_s_rules_keep_placeables_dynamic() {
        let mut m = module();
        let key = ResKey::new(r("inn"), ResType::GIT);
        let mut git = m.gff(&key).unwrap().unwrap();
        let blueprint = |name: &str| ("TemplateResRef", Value::resref(r(name)));
        git.root.set(
            "Placeable List",
            Value::List(vec![
                placeable("ROCK", &[]),
                placeable("STONE", &[("VarTable", vars_named("tdod_lore"))]),
                placeable("CRATE", &[("VarTable", vars_named("x_note"))]),
                placeable("LAMP_3", &[]),
                placeable("BENCH", &[blueprint("tdod_bench")]),
                placeable("TABLE", &[blueprint("plc_table")]),
            ]),
        );
        m.set_gff(key, &git).unwrap();
        // Without a project file: any variable.
        let plan = static_plan(&git.root, &StaticRules::default());
        assert_eq!(plan.convert, [0, 3, 4, 5]);
        assert_eq!(plan.variables, 2);
        assert_eq!(plan.kept[0], (1, Kept::Variable("tdod_lore".into())));
        assert_eq!(plan.left(), "2 with variables");
        // The project's: its variables, tags and blueprints.
        let p = |globs: &[&str]| crate::project_file::Patterns::new(globs).unwrap();
        let rules = StaticRules {
            keep_variables: p(&["tdod_*"]),
            keep_tags: p(&["LAMP_*"]),
            keep_resrefs: p(&["TDOD_*"]),
        };
        let plan = static_plan(&git.root, &rules);
        assert_eq!(plan.convert, [0, 2, 5], "x_note is not the project's");
        assert_eq!((plan.variables, plan.project), (1, 2));
        let why: Vec<String> = plan.kept.iter().map(|(_, k)| k.describe()).collect();
        assert_eq!(
            why,
            [
                "it has the variable tdod_lore",
                "the project keeps tags LAMP_* dynamic",
                "the project keeps blueprints TDOD_* dynamic"
            ]
        );
        assert_eq!(plan.left(), "1 with variables, 2 kept by the project's rules");
        // The command follows the module's project file.
        m.project_file.make_static = rules;
        let done = apply(&mut m, &[r("inn")], &[Change::StaticPlaceables], CP).unwrap();
        let what: Vec<&str> = done.iter().map(|c| c.what.as_str()).collect();
        assert_eq!(
            what,
            ["placeable 0 ROCK Static", "placeable 2 CRATE Static", "placeable 5 TABLE Static"]
        );
    }

    /// A `VisTransformList` whose X rotation is `degrees`.
    fn tilt(degrees: f32) -> Value {
        let mut entry = Struct::new(6);
        for (label, rest) in [("ScaleX", 1.0), ("RotateX", degrees), ("TranslateZ", 0.0)] {
            let mut axis = Struct::new(0);
            axis.set("ValueTo", Value::Float(rest));
            entry.set(label, Value::Struct(axis));
        }
        Value::List(vec![entry])
    }

    #[test]
    fn placeables_are_made_static_where_nothing_is_lost() {
        let mut m = module();
        let key = ResKey::new(r("inn"), ResType::GIT);
        let mut git = m.gff(&key).unwrap().unwrap();
        let script = Value::resref(r("pulse"));
        git.root.set(
            "Placeable List",
            Value::List(vec![
                placeable("ROCK", &[]),
                placeable("WALL", &[("Static", Value::Byte(1))]),
                placeable("CHEST", &[("Useable", Value::Byte(1))]),
                placeable("LEANING", &[("VisTransformList", tilt(30.0))]),
                placeable("UPRIGHT", &[("VisTransformList", tilt(0.0))]),
                placeable("PULSING", &[("OnHeartbeat", script)]),
                placeable("TALKER", &[("Conversation", Value::resref(r("hello")))]),
                placeable("TORCH", &[("AnimationState", Value::Byte(1))]),
            ]),
        );
        m.set_gff(key, &git).unwrap();
        let plan = static_plan(&git.root, &StaticRules::default());
        assert_eq!(plan.convert, [0, 4]);
        assert_eq!((plan.already, plan.useable, plan.transformed, plan.active), (1, 1, 1, 3));
        assert_eq!(plan.kept[2], (5, Kept::Active("OnHeartbeat".into())));
        assert_eq!(plan.kept[2].1.describe(), "it has OnHeartbeat");

        let areas = [r("cave1"), r("inn")];
        let done = apply(&mut m, &areas, &[Change::StaticPlaceables], CP).unwrap();
        let what: Vec<&str> = done.iter().map(|c| c.what.as_str()).collect();
        assert_eq!(what, ["placeable 0 ROCK Static", "placeable 4 UPRIGHT Static"]);
        assert!(done.iter().all(|c| c.area == r("inn")), "the cave has no placeables");
        let git = m.gff(&key).unwrap().unwrap();
        let list = git.root.list("Placeable List").unwrap();
        let fixed: Vec<i64> = list.iter().map(|p| p.integer("Static").unwrap()).collect();
        assert_eq!(fixed, [1, 1, 0, 0, 1, 0, 0, 0]);
        assert!(!list[4].contains("VisTransformList"), "a static placeable has none");
        assert!(list[3].contains("VisTransformList"), "the leaning one keeps its tilt");
        // Again: nothing left to do.
        assert_eq!(apply(&mut m, &areas, &[Change::StaticPlaceables], CP).unwrap(), []);
        // And back: every static one, the wall that was static before too.
        let done = apply(&mut m, &areas, &[Change::DynamicPlaceables], CP).unwrap();
        assert_eq!(done.len(), 3);
        let git = m.gff(&key).unwrap().unwrap();
        let list = git.root.list("Placeable List").unwrap();
        assert!(list.iter().all(|p| p.integer("Static") == Some(0)));
    }

    #[test]
    fn a_change_that_cannot_be_made_changes_nothing() {
        let mut m = module();
        let before: Vec<Struct> = ["cave1", "cave2", "inn"].map(|n| are(&m, n)).to_vec();
        let all = [r("cave1"), r("cave2"), r("inn")];
        for bad in [
            Change::parse_set("SunFogAmount=300").unwrap(),
            Change::parse_set("NoSuchField=1").unwrap(),
            Change::parse_set("Name=Cave").unwrap(),
            Change::parse_set("Interior=maybe").unwrap(),
            Change::parse_var("nDepth:int=deep").unwrap(),
        ] {
            let ok = Change::parse_set("SunFogAmount=5").unwrap();
            let e = apply(&mut m, &all, &[ok, bad.clone()], CP);
            assert!(e.is_err(), "{bad:?}");
            assert_eq!(["cave1", "cave2", "inn"].map(|n| are(&m, n)).to_vec(), before);
        }
        assert!(apply(&mut m, &[r("nowhere")], &[], CP).is_err());
        assert!(Change::parse_set("SunFogAmount").is_err());
        assert!(Change::parse_var("x:bool=1").is_err());
    }

    /// A lighting scheme, found by its label or its row, sets the area's
    /// lighting and each tile's lights from its colors, as Area
    /// Properties' Lighting Scheme does; a row past the table is refused.
    #[test]
    fn a_lighting_scheme_lights_the_area_and_its_tiles() {
        use mg_resman::{LayerClass, MemContainer, ResMan, priority};
        let mut table = MemContainer::new();
        let colors = |prefix: &str| (1..=4).map(|n| format!("{prefix}{n}")).collect::<Vec<_>>();
        let columns =
            [colors("MAIN1_COLOR"), colors("MAIN2_COLOR"), colors("SECONDARY_COLOR")].concat();
        let text = format!(
            "2DA V2.0\n\n LABEL STRREF DAYNIGHT LIGHT_FOG {}\n0 Day **** cycle 2 1 1 1 1 2 2 2 2 \
             3 3 3 3\n1 Crypt **** night 9 4 5 4 5 6 7 6 7 8 9 8 9\n",
            columns.join(" ")
        );
        table.insert(ResKey::new(r("environment"), ResType::TWODA), text.into_bytes());
        let mut rm = ResMan::new();
        rm.add(priority::KEY, "game", LayerClass::Key, table);
        let game = mg_rules::GameData::new(rm, mg_tlk::Tlk::new(Language::ENGLISH));
        assert_eq!(crate::new::Scheme::find(&game, "crypt"), Ok(1));
        assert_eq!(crate::new::Scheme::find(&game, "0"), Ok(0));
        assert!(crate::new::Scheme::find(&game, "7").is_err(), "past the table");
        assert!(crate::new::Scheme::read(&game, 7).is_err());

        let mut m = module();
        let key = ResKey::new(r("cave1"), ResType::ARE);
        let mut cave = m.gff(&key).unwrap().unwrap();
        let tile = |main1: u8| {
            let mut t = Struct::new(1);
            for label in crate::new::TILE_LIGHT_FIELDS {
                t.set(label, Value::Byte(main1));
            }
            t.set("Tile_ID", Value::Int(3));
            t
        };
        cave.root.set("Tile_List", Value::List(vec![tile(1), tile(2), tile(3)]));
        m.set_gff(key, &cave).unwrap();
        let scheme = crate::new::Scheme::read(&game, 1).unwrap();
        let done = apply(&mut m, &[r("cave1")], &[Change::LightingScheme(scheme)], CP).unwrap();
        let said: Vec<(&str, Option<&str>, Option<&str>)> =
            done.iter().map(|c| (c.what.as_str(), c.from.as_deref(), c.to.as_deref())).collect();
        assert_eq!(
            said,
            [
                ("LightingScheme", None, Some("1")),
                ("lights of 3 tiles", Some("their own"), Some("scheme 1's"))
            ]
        );
        let cave = are(&m, "cave1");
        assert_eq!(cave.integer("LightingScheme"), Some(1));
        assert_eq!(cave.integer("IsNight"), Some(1));
        assert_eq!(cave.integer("SunFogAmount"), Some(9), "the scheme's, not the area's 2");
        for t in cave.list("Tile_List").unwrap() {
            let light = |l: &str| t.integer(l).unwrap();
            assert!([4, 5].contains(&light("Tile_MainLight1")), "{t:?}");
            assert!([6, 7].contains(&light("Tile_MainLight2")), "{t:?}");
            assert!([8, 9].contains(&light("Tile_SrcLight1")), "{t:?}");
            assert_eq!(light("Tile_SrcLight2"), light("Tile_SrcLight1"));
            assert_eq!(t.integer("Tile_ID"), Some(3), "the tile itself stays");
        }
    }
}
