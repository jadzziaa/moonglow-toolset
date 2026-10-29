//! Questions about a module for scripts and build pipelines (`mg find`,
//! `mg info`): its blueprints and the objects placed in its areas, by type,
//! tag, name, resref, area and field values; and what the module is.

use mg_core::{Gender, Language, ResRef, ResType};
use mg_gff::{Struct, Value};
use mg_resman::ResKey;
use mg_rules::ObjectKind;

use crate::Module;

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
    /// Tag, ignoring case and accents; `*` matches any run of characters.
    pub tag: Option<String>,
    /// Words the name contains, ignoring case and accents.
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
    /// Fields whose value depends on the object's type (a row given by
    /// its name: `Appearance` is a placeables.2da row for a placeable, a
    /// doortypes.2da row for a door): the label, and the value it must
    /// have for each type. An object of a type not listed doesn't match.
    pub typed_fields: Vec<(String, Vec<(ResType, String)>)>,
}

/// Whether `text` matches `pattern`, ignoring case and accents, `*`
/// matching any run.
pub fn glob(pattern: &str, text: &str) -> bool {
    let (p, t) = (mg_core::text::fold(pattern), mg_core::text::fold(text));
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
    if let Some(number) = v.number_text() {
        return Some(number);
    }
    Some(match v {
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
    let template = ObjectKind::resref_field_of(restype);
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
            let name = mg_core::text::fold(&f.name);
            n.split_whitespace().all(|w| name.contains(&mg_core::text::fold(w)))
        })
        && q.resref.as_ref().is_none_or(|r| f.resref.is_some_and(|x| glob(r, &x.to_string())))
        && q.fields.iter().all(|(label, want)| match (s.get(label), want) {
            (None, _) => false,
            (Some(_), None) => true,
            (Some(v), Some(w)) => field_text(v).is_some_and(|t| glob(w, &t)),
        })
        && q.typed_fields.iter().all(|(label, wants)| {
            let want = wants.iter().find(|(t, _)| *t == f.restype).map(|(_, w)| w);
            match (s.get(label), want) {
                (Some(v), Some(w)) => field_text(v).is_some_and(|t| glob(w, &t)),
                _ => false,
            }
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
            .filter(|k| ObjectKind::from_restype(k.restype).is_some() && wanted(k.restype))
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
            for (list, restype) in ObjectKind::ALL.map(|k| (k.list(), k.restype())) {
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

/// [`info_in`] the module's text read as Windows-1252.
pub fn info(m: &Module) -> Result<Info, crate::ModuleError> {
    info_in(m, mg_core::Codepage::WINDOWS_1252)
}

/// What a module is, its text (name, tag, description) read in `codepage`:
/// the module's (`mg_rules::text_codepage`; Windows-1252, or the table of
/// an `encoding.2da` in its haks).
pub fn info_in(m: &Module, codepage: mg_core::Codepage) -> Result<Info, crate::ModuleError> {
    let ifo = m.info()?;
    let r = &ifo.root;
    let text = |l: &str| r.string(l).map(|b| codepage.decode(b).into_owned()).unwrap_or_default();
    let localized = |l: &str| match r.get(l) {
        Some(Value::LocString(ls)) => ls
            .text_in(Language::ENGLISH, Gender::Male, codepage)
            .map(|t| t.into_owned())
            .unwrap_or_default(),
        _ => String::new(),
    };
    let mut counts: std::collections::BTreeMap<String, usize> = Default::default();
    for k in m.keys() {
        *counts.entry(k.restype.extension().unwrap_or("?").to_string()).or_default() += 1;
    }
    Ok(Info {
        name: localized("Mod_Name"),
        tag: text("Mod_Tag"),
        description: localized("Mod_Description"),
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
        assert!(glob("*arbol*", "Viejo_ÁRBOL_01"));
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
        // A value for each type: a creature's here; a type not listed
        // doesn't match.
        let typed = |values: Vec<(ResType, String)>| Query {
            typed_fields: vec![("Appearance_Type".into(), values)],
            placed: Some(false),
            ..Default::default()
        };
        assert_eq!(find(&m, &typed(vec![(ResType::UTC, "6".into())])).len(), 1);
        assert!(find(&m, &typed(vec![(ResType::UTC, "7".into())])).is_empty());
        assert!(find(&m, &typed(vec![(ResType::UTP, "6".into())])).is_empty());
        let q = Query { types: vec![ResType::UTP], ..Default::default() };
        assert!(find(&m, &q).is_empty());
        let i = info(&m).unwrap();
        assert_eq!((i.tag.as_str(), i.areas.len()), ("MOD", 1));
        assert!(i.resources.contains(&("utc".to_string(), 1)));
    }
}
