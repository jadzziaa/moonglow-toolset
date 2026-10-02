//! Questions about a module for scripts and build pipelines (`mg find`,
//! `mg info`): its blueprints and the objects placed in its areas, by type,
//! tag, name, resref, area and field values; and what the module is.

use mg_core::{Gender, Language, ResRef, ResType};
use mg_gff::{Struct, Value};
use mg_resman::ResKey;

use crate::Module;
use crate::instances::GIT_LISTS;

/// Something found: a blueprint, or an object placed in an area.
#[derive(Debug, Clone, PartialEq)]
pub struct Found {
    /// The blueprint type (`utc`, `utp`…), also for placed objects.
    pub restype: ResType,
    /// The blueprint's resref; a placed object's blueprint
    /// (`TemplateResRef`, a store's `ResRef`), if it names one.
    pub resref: Option<ResRef>,
    pub tag: String,
    /// The name in English (a creature's first and last names).
    pub name: String,
    /// The area and the index in its GIT list, for a placed object.
    pub placed: Option<(ResRef, usize)>,
    pub position: Option<[f32; 3]>,
}

/// What to look for; every given part must match.
#[derive(Debug, Clone, Default)]
pub struct Query {
    /// Blueprint types; empty: all.
    pub types: Vec<ResType>,
    /// Tag, ignoring case; `*` matches any run of characters.
    pub tag: Option<String>,
    /// Words the name contains, ignoring case.
    pub name: Option<String>,
    /// Resref (a placed object's blueprint), ignoring case; `*` as in tags.
    pub resref: Option<String>,
    /// Only objects placed in this area.
    pub area: Option<ResRef>,
    /// Only placed objects (`Some(true)`) or only blueprints (`Some(false)`).
    pub placed: Option<bool>,
    /// Fields that must have these values (`*` as in tags), or exist
    /// (`None`).
    pub fields: Vec<(String, Option<String>)>,
}

/// Whether `text` matches `pattern`, ignoring case, `*` matching any run.
pub fn glob(pattern: &str, text: &str) -> bool {
    let (p, t) = (pattern.to_lowercase(), text.to_lowercase());
    let parts: Vec<&str> = p.split('*').collect();
    if parts.len() == 1 {
        return p == t;
    }
    let mut at = 0;
    for (i, part) in parts.iter().enumerate() {
        if i == 0 {
            if !t.starts_with(part) {
                return false;
            }
            at = part.len();
        } else if i == parts.len() - 1 {
            return t.len() >= at + part.len() && t[at..].ends_with(part);
        } else {
            match t[at..].find(part) {
                Some(k) => at += k + part.len(),
                None => return false,
            }
        }
    }
    true
}

fn english(v: Option<&Value>) -> String {
    match v {
        Some(Value::LocString(ls)) => {
            ls.text(Language::ENGLISH, Gender::Male).map(|t| t.into_owned()).unwrap_or_default()
        }
        Some(Value::String(b)) => String::from_utf8_lossy(b).into_owned(),
        _ => String::new(),
    }
}

/// A field's value as text, for comparing: numbers as written, strings and
/// resrefs as they are, a localized string's English.
pub fn field_text(v: &Value) -> Option<String> {
    Some(match v {
        Value::Byte(x) => x.to_string(),
        Value::Char(x) => x.to_string(),
        Value::Word(x) => x.to_string(),
        Value::Short(x) => x.to_string(),
        Value::Dword(x) => x.to_string(),
        Value::Int(x) => x.to_string(),
        Value::Dword64(x) => x.to_string(),
        Value::Int64(x) => x.to_string(),
        Value::Float(x) => x.to_string(),
        Value::Double(x) => x.to_string(),
        Value::String(b) | Value::ResRef(b) => String::from_utf8_lossy(b).into_owned(),
        Value::LocString(_) => english(Some(v)),
        _ => return None,
    })
}

fn describe(s: &Struct, restype: ResType, placed: Option<(ResRef, usize)>) -> Found {
    let name = if restype == ResType::UTC {
        let parts = [english(s.get("FirstName")), english(s.get("LastName"))];
        parts.iter().filter(|p| !p.is_empty()).cloned().collect::<Vec<_>>().join(" ")
    } else {
        let n = english(s.get("LocalizedName"));
        if n.is_empty() { english(s.get("LocName")) } else { n }
    };
    let template = if restype == ResType::UTM { "ResRef" } else { "TemplateResRef" };
    let resref = s.resref(template).filter(|r| !r.is_empty());
    let float = |l: &str| s.float(l);
    let position = match (float("X"), float("Y"), float("Z")) {
        (Some(x), Some(y), Some(z)) => Some([x, y, z]),
        _ => match (float("XPosition"), float("YPosition"), float("ZPosition")) {
            (Some(x), Some(y), Some(z)) => Some([x, y, z]),
            _ => None,
        },
    };
    Found {
        restype,
        resref,
        tag: s.string("Tag").map(|t| String::from_utf8_lossy(t).into_owned()).unwrap_or_default(),
        name,
        placed,
        position: placed.and(position),
    }
}

fn matches(q: &Query, s: &Struct, f: &Found) -> bool {
    q.tag.as_ref().is_none_or(|t| glob(t, &f.tag))
        && q.name.as_ref().is_none_or(|n| {
            let name = f.name.to_lowercase();
            n.split_whitespace().all(|w| name.contains(&w.to_lowercase()))
        })
        && q.resref.as_ref().is_none_or(|r| f.resref.is_some_and(|x| glob(r, &x.to_string())))
        && q.fields.iter().all(|(label, want)| match (s.get(label), want) {
            (None, _) => false,
            (Some(_), None) => true,
            (Some(v), Some(w)) => field_text(v).is_some_and(|t| glob(w, &t)),
        })
}

/// Blueprints, then placed objects (area by area, list by list), that the
/// query matches.
pub fn find(m: &Module, q: &Query) -> Vec<Found> {
    let wanted = |t: ResType| q.types.is_empty() || q.types.contains(&t);
    let mut out = Vec::new();
    if q.placed != Some(true) && q.area.is_none() {
        let mut keys: Vec<ResKey> = m
            .keys()
            .copied()
            .filter(|k| GIT_LISTS.iter().any(|(_, t)| *t == k.restype) && wanted(k.restype))
            .collect();
        keys.sort();
        for k in keys {
            let Some(Ok(g)) = m.gff(&k) else { continue };
            let mut f = describe(&g.root, k.restype, None);
            f.resref = Some(k.resref);
            if matches(q, &g.root, &f) {
                out.push(f);
            }
        }
    }
    if q.placed != Some(false) {
        for area in m.areas().unwrap_or_default() {
            if q.area.is_some_and(|a| a != area) {
                continue;
            }
            let Some(Ok(git)) = m.gff(&ResKey::new(area, ResType::GIT)) else { continue };
            for (list, restype) in GIT_LISTS {
                if !wanted(restype) {
                    continue;
                }
                let Some(Value::List(items)) = git.root.get(list) else { continue };
                for (i, s) in items.iter().enumerate() {
                    let f = describe(s, restype, Some((area, i)));
                    if matches(q, s, &f) {
                        out.push(f);
                    }
                }
            }
        }
    }
    out
}

/// What a module is: its name, tag, areas, haks, talk table and resources
/// by type.
#[derive(Debug, Clone, PartialEq)]
pub struct Info {
    pub name: String,
    pub tag: String,
    pub description: String,
    pub entry_area: Option<ResRef>,
    pub areas: Vec<ResRef>,
    pub haks: Vec<String>,
    pub custom_tlk: Option<String>,
    pub min_game_version: String,
    /// Resource types (extensions) and how many of each, by extension.
    pub resources: Vec<(String, usize)>,
}

pub fn info(m: &Module) -> Result<Info, crate::ModuleError> {
    let ifo = m.info()?;
    let r = &ifo.root;
    let text =
        |l: &str| r.string(l).map(|b| String::from_utf8_lossy(b).into_owned()).unwrap_or_default();
    let mut counts: std::collections::BTreeMap<String, usize> = Default::default();
    for k in m.keys() {
        *counts.entry(k.restype.extension().unwrap_or("?").to_string()).or_default() += 1;
    }
    Ok(Info {
        name: english(r.get("Mod_Name")),
        tag: text("Mod_Tag"),
        description: english(r.get("Mod_Description")),
        entry_area: r.resref("Mod_Entry_Area").filter(|a| !a.is_empty()),
        areas: m.areas()?,
        haks: m.haks()?,
        custom_tlk: m.custom_tlk()?,
        min_game_version: text("Mod_MinGameVer"),
        resources: counts.into_iter().collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use mg_gff::Gff;

    #[test]
    fn globs() {
        assert!(glob("GUARD", "guard"));
        assert!(glob("guard_*", "GUARD_01"));
        assert!(glob("*_01", "guard_01"));
        assert!(glob("g*d*1", "guard_01"));
        assert!(!glob("guard", "guard_01"));
        assert!(!glob("*x*", "guard"));
    }

    fn creature(tag: &str, first: &str) -> Struct {
        let mut s = Struct::new(4);
        s.set("Tag", Value::String(tag.as_bytes().to_vec()));
        s.set(
            "FirstName",
            Value::LocString(mg_core::LocString::from_text(Language::ENGLISH, Gender::Male, first)),
        );
        s.set("TemplateResRef", Value::ResRef(b"guard".to_vec()));
        s.set("XPosition", Value::Float(1.0));
        s.set("YPosition", Value::Float(2.0));
        s.set("ZPosition", Value::Float(0.0));
        s.set("Appearance_Type", Value::Word(6));
        s
    }

    #[test]
    fn blueprints_and_placed_objects() {
        let mut m = Module::new();
        let mut ifo = Gff::new(*b"IFO ");
        let mut a = Struct::new(6);
        a.set("Area_Name", Value::ResRef(b"keep".to_vec()));
        ifo.root.set("Mod_Area_list", Value::List(vec![a]));
        ifo.root.set("Mod_Tag", Value::String(b"MOD".to_vec()));
        m.set_info(&ifo).unwrap();
        let mut utc = Gff::new(*b"UTC ");
        utc.root = creature("GUARD", "Gate Guard");
        m.set_gff(ResKey::parse("guard", ResType::UTC).unwrap(), &utc).unwrap();
        let mut git = Gff::new(*b"GIT ");
        let c2 = creature("GUARD_02", "Wall Guard");
        git.root.set("Creature List", Value::List(vec![creature("GUARD_01", "Gate Guard"), c2]));
        m.set_gff(ResKey::parse("keep", ResType::GIT).unwrap(), &git).unwrap();

        let all = find(&m, &Query { tag: Some("guard*".into()), ..Default::default() });
        assert_eq!(all.len(), 3);
        assert_eq!(all[0].placed, None);
        assert_eq!(all[1].placed, Some((ResRef::from_str("keep").unwrap(), 0)));
        assert_eq!(all[1].position, Some([1.0, 2.0, 0.0]));
        let q = Query { name: Some("wall".into()), ..Default::default() };
        assert_eq!(find(&m, &q).iter().map(|f| f.tag.as_str()).collect::<Vec<_>>(), ["GUARD_02"]);
        let q = Query { placed: Some(true), resref: Some("guard".into()), ..Default::default() };
        assert_eq!(find(&m, &q).len(), 2);
        let q = Query {
            fields: vec![("Appearance_Type".into(), Some("6".into()))],
            placed: Some(false),
            ..Default::default()
        };
        assert_eq!(find(&m, &q).len(), 1);
        let q = Query { types: vec![ResType::UTP], ..Default::default() };
        assert!(find(&m, &q).is_empty());
        let i = info(&m).unwrap();
        assert_eq!((i.tag.as_str(), i.areas.len()), ("MOD", 1));
        assert!(i.resources.contains(&("utc".to_string(), 1)));
    }
}
