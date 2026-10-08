//! Several areas' properties at once (`mg areas`, and what the window's
//! Edit Areas Together does): the module's areas, narrowed by name, tileset
//! and kind, and fields, flags and variables set on each.
//!
//! An area's properties are its ARE's root fields, and, for its ambient
//! sounds and music, the `AreaProperties` struct of its GIT. A field keeps
//! the type it has; one an area lacks takes the type the game's areas have
//! for it.

use mg_core::{Language, ResRef, ResType};
use mg_gff::{FieldType, Struct, Value};
use mg_resman::ResKey;

use crate::Module;

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
    /// Text in the name, the tag or the ResRef, whatever its case.
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
        let text = self.text.as_deref().map(|t| t.trim().to_lowercase());
        let has = |s: &str| text.as_deref().is_none_or(|t| s.to_lowercase().contains(t));
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
/// tilted or scaled, play no animations and can't be destroyed.
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
}

/// Whether a placed object's visual transform (EE's `VisTransformList`,
/// or the older `VisualTransform`) changes how its model shows.
fn transforms(s: &Struct) -> bool {
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

/// What making an area's placeables static would do (`git`: the area's
/// GIT root).
pub fn static_plan(git: &Struct) -> StaticPlan {
    let mut plan = StaticPlan::default();
    let on = |s: &Struct, label: &str| s.integer(label).unwrap_or(0) != 0;
    for (i, p) in git.list("Placeable List").unwrap_or(&[]).iter().enumerate() {
        let named = |label: &str| p.resref(label).is_some_and(|r| !r.is_empty());
        let scripted = p.fields.iter().any(|f| {
            let empty = matches!(&f.value, Value::ResRef(r) if r.is_empty());
            f.label.as_bytes().starts_with(b"On") && matches!(f.value, Value::ResRef(_)) && !empty
        });
        if on(p, "Static") {
            plan.already += 1;
        } else if on(p, "Useable") {
            plan.useable += 1;
        } else if transforms(p) {
            plan.transformed += 1;
        } else if scripted
            || named("Conversation")
            || on(p, "TrapFlag")
            || on(p, "HasInventory")
            || on(p, "AnimationState")
        {
            plan.active += 1;
        } else {
            plan.convert.push(i);
        }
    }
    plan
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

/// A value as `mg areas` prints it.
fn shown(v: &Value) -> String {
    match v {
        Value::String(b) | Value::ResRef(b) => String::from_utf8_lossy(b).into_owned(),
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

/// A whole number: decimal, or `0x` and hexadecimal (colours are
/// `0xBBGGRR`).
fn integer(text: &str) -> Option<i64> {
    let t = text.trim();
    match t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")) {
        Some(hex) => i64::from_str_radix(hex, 16).ok(),
        None => t.parse().ok(),
    }
}

fn yes_no(text: &str) -> Option<bool> {
    match text.trim().to_ascii_lowercase().as_str() {
        "1" | "yes" | "true" | "on" => Some(true),
        "0" | "no" | "false" | "off" => Some(false),
        _ => None,
    }
}

/// `text` as a value of type `t`.
fn typed(field: &str, t: FieldType, text: &str) -> Result<Value, String> {
    let int = |min: i64, max: i64| {
        integer(text)
            .filter(|v| (min..=max).contains(v))
            .ok_or_else(|| format!("{field}: {text:?} is not a whole number from {min} to {max}"))
    };
    Ok(match t {
        FieldType::Byte => Value::Byte(int(0, 255)? as u8),
        FieldType::Char => Value::Char(int(-128, 127)? as i8),
        FieldType::Word => Value::Word(int(0, 65535)? as u16),
        FieldType::Short => Value::Short(int(-32768, 32767)? as i16),
        FieldType::Dword => Value::Dword(int(0, i64::from(u32::MAX))? as u32),
        FieldType::Int => Value::Int(int(i64::from(i32::MIN), i64::from(i32::MAX))? as i32),
        FieldType::Dword64 => Value::Dword64(int(0, i64::MAX)? as u64),
        FieldType::Int64 => Value::Int64(int(i64::MIN, i64::MAX)?),
        FieldType::Float => Value::Float(
            text.trim().parse().map_err(|_| format!("{field}: {text:?} is not a number"))?,
        ),
        FieldType::Double => Value::Double(
            text.trim().parse().map_err(|_| format!("{field}: {text:?} is not a number"))?,
        ),
        FieldType::String => Value::String(text.as_bytes().to_vec()),
        FieldType::ResRef => Value::resref(
            ResRef::from_str(text.trim()).map_err(|e| format!("{field}: {text:?}: {e}"))?,
        ),
        other => return Err(format!("{field} is a {other:?}: not a field this sets")),
    })
}

/// Sets `field` of `s`, keeping its type (else `default`); what changed.
fn set_field(
    s: &mut Struct,
    field: &str,
    default: Option<FieldType>,
    text: &str,
) -> Result<Option<(Option<String>, String)>, String> {
    let t = s
        .get(field)
        .map(Value::field_type)
        .or(default)
        .ok_or_else(|| format!("{field}: not a field of an area"))?;
    let value = typed(field, t, text)?;
    if s.get(field) == Some(&value) {
        return Ok(None);
    }
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
) -> Result<Option<(Option<String>, String)>, String> {
    let at = vars.iter().position(|v| v.string("Name") == Some(name.as_bytes()));
    let kind = kind.or_else(|| at.and_then(|i| vars[i].dword("Type"))).unwrap_or_else(|| {
        let t = text.trim();
        if t.parse::<i32>().is_ok() {
            1
        } else if t.parse::<f32>().is_ok() {
            2
        } else {
            3
        }
    });
    let value = match kind {
        1 => {
            Value::Int(text.trim().parse().map_err(|_| format!("{name}: {text:?} is not an int"))?)
        }
        2 => Value::Float(
            text.trim().parse().map_err(|_| format!("{name}: {text:?} is not a float"))?,
        ),
        3 => Value::String(text.as_bytes().to_vec()),
        other => return Err(format!("{name} is a variable of type {other}: not one this sets")),
    };
    let mut var = at.map_or_else(|| Struct::new(0), |i| vars[i].clone());
    let from = var.get("Value").map(shown);
    var.set("Name", Value::String(name.as_bytes().to_vec()));
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
/// already had a value isn't listed for it). The module isn't saved.
pub fn apply(m: &mut Module, areas: &[ResRef], changes: &[Change]) -> Result<Vec<Changed>, String> {
    let mut done = Vec::new();
    let mut write = Vec::new();
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
                            set_field(props, field, Some(FieldType::Int), value)?
                        {
                            git_changed = true;
                            note(field.clone(), from, Some(to));
                        }
                    } else {
                        let default = mg_schema::root_field_type(ResType::ARE, field);
                        if let Some((from, to)) = set_field(&mut are.root, field, default, value)? {
                            are_changed = true;
                            note(field.clone(), from, Some(to));
                        }
                    }
                }
                Change::Var { name, kind, value } => {
                    let mut vars = are.root.list("VarTable").unwrap_or(&[]).to_vec();
                    if let Some((from, to)) = set_var(&mut vars, name, *kind, value)? {
                        are.root.set("VarTable", Value::List(vars));
                        are_changed = true;
                        note(format!("var {name}"), from, Some(to));
                    }
                }
                Change::StaticPlaceables => {
                    let Some(git) = git.as_mut() else { continue };
                    let plan = static_plan(&git.root);
                    let Some(list) = git.root.list_mut("Placeable List") else { continue };
                    for &i in &plan.convert {
                        let p = &mut list[i];
                        p.set("Static", Value::Byte(1));
                        // (One that changes nothing: static placeables
                        // have none.)
                        p.remove("VisTransformList");
                        p.remove("VisualTransform");
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
                Change::RemoveVar(name) => {
                    let mut vars = are.root.list("VarTable").unwrap_or(&[]).to_vec();
                    let at = vars.iter().position(|v| v.string("Name") == Some(name.as_bytes()));
                    if let Some(i) = at {
                        let from = vars.remove(i).get("Value").map(shown);
                        are.root.set("VarTable", Value::List(vars));
                        are_changed = true;
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
            Change::RemoveVar("sHost".into()),
        ];
        let caves = [r("cave1"), r("cave2")];
        let done = apply(&mut m, &caves, &changes).unwrap();
        for name in ["cave1", "cave2"] {
            let a = are(&m, name);
            assert_eq!(a.get("SunFogAmount"), Some(&Value::Byte(9)));
            assert_eq!(a.get("SunFogColor"), Some(&Value::Dword(0x10_2030)));
            let git = m.gff(&ResKey::new(r(name), ResType::GIT)).unwrap().unwrap();
            assert_eq!(git.root.child("AreaProperties").unwrap().integer("MusicDay"), Some(57));
        }
        // Each keeps its other flags and variables.
        assert_eq!(are(&m, "cave1").integer("Flags"), Some(0x5));
        assert_eq!(are(&m, "cave2").integer("Flags"), Some(0x1 | 0x100));
        assert_eq!(
            are(&m, "cave1").list("VarTable").unwrap(),
            [var("nDepth", 1, Value::Int(4)), var("nMusic", 1, Value::Int(57))]
        );
        assert_eq!(
            are(&m, "cave2").list("VarTable").unwrap(),
            [var("nMusic", 1, Value::Int(57)), var("nDepth", 1, Value::Int(4))]
        );
        // The inn wasn't named.
        assert_eq!(are(&m, "inn").integer("SunFogAmount"), Some(2));
        assert_eq!(are(&m, "inn").list("VarTable").unwrap().len(), 1);
        // What changed, by area.
        let fog: Vec<&Changed> = done.iter().filter(|c| c.what == "SunFogAmount").collect();
        assert_eq!(fog.len(), 2);
        assert_eq!((fog[0].from.as_deref(), fog[0].to.as_deref()), (Some("2"), Some("9")));
        // Again: nothing to change.
        assert!(apply(&mut m, &caves, &changes).unwrap().is_empty());
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
        let plan = static_plan(&git.root);
        assert_eq!(plan.convert, [0, 4]);
        assert_eq!((plan.already, plan.useable, plan.transformed, plan.active), (1, 1, 1, 3));

        let areas = [r("cave1"), r("inn")];
        let done = apply(&mut m, &areas, &[Change::StaticPlaceables]).unwrap();
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
        assert_eq!(apply(&mut m, &areas, &[Change::StaticPlaceables]).unwrap(), []);
        // And back: every static one, the wall that was static before too.
        let done = apply(&mut m, &areas, &[Change::DynamicPlaceables]).unwrap();
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
            let e = apply(&mut m, &all, &[ok, bad.clone()]);
            assert!(e.is_err(), "{bad:?}");
            assert_eq!(["cave1", "cave2", "inn"].map(|n| are(&m, n)).to_vec(), before);
        }
        assert!(apply(&mut m, &[r("nowhere")], &[]).is_err());
        assert!(Change::parse_set("SunFogAmount").is_err());
        assert!(Change::parse_var("x:bool=1").is_err());
    }
}
