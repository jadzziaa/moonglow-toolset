//! The tags of a module's objects, as `GetObjectByTag` sees them: each
//! area's and each placed object's (a static placeable's too: the server
//! keeps it as an object, `engine_static_placeables`). A tag on two
//! objects gives a script one of them (the one placed later, of two in an
//! area), so a copy pasted with its original's tag takes its place.
//!
//! What a paste or a copy made twice ([`clashes`]), a tag of the module's
//! own made from another (`TAG_2`, [`retag`]), and a store's tag from the
//! project's pattern ([`store_tag`]).

use std::collections::{HashMap, HashSet};

use mg_core::{ResRef, ResType};
use mg_gff::Struct;
use mg_resman::ResKey;
use mg_rules::ObjectKind;

use crate::Module;
use crate::project_file::{PasteWarning, TagRules};

/// The longest tag Aurora takes.
pub const MAX_TAG: usize = 32;

/// An object with a tag: an area, or an object placed in one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Tagged {
    Area(ResRef),
    Object { area: ResRef, kind: ObjectKind, index: usize },
}

impl Tagged {
    /// The resource it is in: the area's ARE, or its GIT.
    pub fn key(self) -> ResKey {
        match self {
            Tagged::Area(area) => ResKey::new(area, ResType::ARE),
            Tagged::Object { area, .. } => ResKey::new(area, ResType::GIT),
        }
    }

    /// In a few words: `area inn`, `placeable 3 in inn`.
    pub fn describe(self) -> String {
        match self {
            Tagged::Area(area) => format!("area {area}"),
            Tagged::Object { area, kind, index } => {
                format!("{} {index} in {area}", format!("{kind:?}").to_lowercase())
            }
        }
    }
}

/// The module's objects by tag.
#[derive(Debug, Clone, Default)]
pub struct TagIndex {
    by_tag: HashMap<Vec<u8>, Vec<Tagged>>,
}

impl TagIndex {
    /// An area's own tag (from its ARE's root).
    pub fn add_area(&mut self, area: ResRef, are: &Struct) {
        if let Some(tag) = are.string("Tag").filter(|t| !t.is_empty()) {
            self.by_tag.entry(tag.to_vec()).or_default().push(Tagged::Area(area));
        }
    }

    /// The objects placed in an area (from its GIT's root).
    pub fn add_git(&mut self, area: ResRef, git: &Struct) {
        for kind in ObjectKind::ALL {
            for (index, o) in git.list(kind.list()).unwrap_or(&[]).iter().enumerate() {
                if let Some(tag) = o.string("Tag").filter(|t| !t.is_empty()) {
                    let at = Tagged::Object { area, kind, index };
                    self.by_tag.entry(tag.to_vec()).or_default().push(at);
                }
            }
        }
    }

    /// Every area's and placed object's tag in the module as saved.
    pub fn of_module(m: &Module) -> TagIndex {
        let mut index = TagIndex::default();
        let mut keys: Vec<ResKey> = m
            .keys()
            .filter(|k| matches!(k.restype, ResType::ARE | ResType::GIT))
            .copied()
            .collect();
        keys.sort();
        for key in keys {
            let Some(Ok(g)) = m.gff(&key) else { continue };
            match key.restype {
                ResType::ARE => index.add_area(key.resref, &g.root),
                _ => index.add_git(key.resref, &g.root),
            }
        }
        index
    }

    /// The objects with the tag (exactly so: the game's lookups compare
    /// tags as they are).
    pub fn places(&self, tag: &[u8]) -> &[Tagged] {
        self.by_tag.get(tag).map_or(&[], Vec::as_slice)
    }

    pub fn count(&self, tag: &[u8]) -> usize {
        self.places(tag).len()
    }

    /// The tags on more than one object, in order.
    pub fn duplicates(&self) -> Vec<(&[u8], &[Tagged])> {
        let mut out: Vec<(&[u8], &[Tagged])> = self
            .by_tag
            .iter()
            .filter(|(_, at)| at.len() > 1)
            .map(|(t, at)| (t.as_slice(), at.as_slice()))
            .collect();
        out.sort();
        out
    }
}

/// A tag a paste or a copy put on more than one object.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Clash {
    pub tag: String,
    /// How many objects of the module have it now.
    pub count: usize,
    /// The project's pattern it matches, of those that must be unique.
    pub unique: Option<String>,
}

impl Clash {
    /// "TAG is now on two objects", for people.
    pub fn sentence(&self) -> String {
        let n = match self.count {
            2 => "two".to_string(),
            3 => "three".to_string(),
            n => n.to_string(),
        };
        match &self.unique {
            Some(p) => format!("{} is now on {n} objects (the project wants {p} unique)", self.tag),
            None => format!("{} is now on {n} objects", self.tag),
        }
    }
}

/// The tags of `made` (those just pasted or copied) that are now on more
/// than one object of `index` (which has them), as the project's rules
/// say to warn of them.
pub fn clashes(index: &TagIndex, made: &[&[u8]], rules: &TagRules) -> Vec<Clash> {
    if rules.paste_warning == PasteWarning::Off {
        return Vec::new();
    }
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for &tag in made {
        if tag.is_empty() || !seen.insert(tag) || index.count(tag) < 2 {
            continue;
        }
        let text = String::from_utf8_lossy(tag).into_owned();
        let unique = rules.unique.matching(&text).map(str::to_string);
        if rules.paste_warning == PasteWarning::Unique && unique.is_none() {
            continue;
        }
        out.push(Clash { tag: text, count: index.count(tag), unique });
    }
    out
}

/// A tag of `tag`'s with a number, `TAG_2`, `TAG_3`… (one ending in
/// `_N` counts on from N), the first `taken` says no object has, within
/// [`MAX_TAG`] characters.
pub fn retag(tag: &str, taken: &dyn Fn(&str) -> bool) -> String {
    let (stem, from) = match tag.rsplit_once('_') {
        Some((stem, n))
            if !stem.is_empty() && !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()) =>
        {
            (stem, n.parse::<u32>().map_or(2, |n| n.saturating_add(1)))
        }
        _ => (tag, 2),
    };
    for n in from.. {
        let end = format!("_{n}");
        let room = MAX_TAG.saturating_sub(end.len());
        let mut cut = stem.len().min(room);
        while !stem.is_char_boundary(cut) {
            cut -= 1;
        }
        let candidate = format!("{}{end}", &stem[..cut]);
        if !taken(&candidate) {
            return candidate;
        }
    }
    unreachable!("a number is free")
}

/// A store's tag from the project's pattern (`{area}` the area's tag,
/// `{n}` the first number that makes it the only one), else, where the
/// tag is taken, [`retag`]ged.
pub fn store_tag(pattern: &str, area_tag: &str, taken: &dyn Fn(&str) -> bool) -> String {
    let base = pattern.replace("{area}", area_tag);
    if base.contains("{n}") {
        for n in 1u32..100_000 {
            let candidate = base.replace("{n}", &n.to_string());
            if candidate.len() > MAX_TAG {
                break; // (Too long for its number: as a pattern without one.)
            }
            if !taken(&candidate) {
                return candidate;
            }
        }
    }
    let mut tag = base.replace("{n}", "");
    let mut cut = tag.len().min(MAX_TAG);
    while !tag.is_char_boundary(cut) {
        cut -= 1;
    }
    tag.truncate(cut);
    if taken(&tag) { retag(&tag, taken) } else { tag }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project_file::Patterns;
    use mg_gff::Value;

    fn r(s: &str) -> ResRef {
        ResRef::from_str(s).unwrap()
    }

    fn tagged(tag: &str) -> Struct {
        let mut s = Struct::new(0);
        s.set("Tag", Value::String(tag.as_bytes().to_vec()));
        s
    }

    fn index() -> TagIndex {
        let mut git = Struct::new(0);
        git.set(
            "Placeable List",
            Value::List(vec![tagged("LAMP"), tagged("LAMP"), tagged(""), tagged("CHEST")]),
        );
        git.set("WaypointList", Value::List(vec![tagged("TDOD_START_KREZK"), tagged("INN")]));
        let mut i = TagIndex::default();
        i.add_git(r("inn"), &git);
        i.add_area(r("inn"), &tagged("INN"));
        i
    }

    #[test]
    fn objects_and_areas_are_found_by_their_tags() {
        let i = index();
        assert_eq!(i.count(b"LAMP"), 2);
        assert_eq!(i.count(b""), 0, "no tag is no tag");
        assert_eq!(i.count(b"lamp"), 0, "tags are compared as they are");
        assert_eq!(
            i.places(b"INN"),
            [
                Tagged::Object { area: r("inn"), kind: ObjectKind::Waypoint, index: 1 },
                Tagged::Area(r("inn"))
            ]
        );
        assert_eq!(i.places(b"INN")[0].describe(), "waypoint 1 in inn");
        let dups: Vec<&[u8]> = i.duplicates().into_iter().map(|d| d.0).collect();
        assert_eq!(dups, [&b"INN"[..], b"LAMP"]);
    }

    #[test]
    fn a_paste_says_what_it_made_twice_as_the_project_wants() {
        let i = index();
        let mut rules = TagRules::default();
        let made: [&[u8]; 4] = [b"LAMP", b"LAMP", b"CHEST", b"TDOD_START_KREZK"];
        let got = clashes(&i, &made, &rules);
        assert_eq!(got, [Clash { tag: "LAMP".into(), count: 2, unique: None }]);
        assert_eq!(got[0].sentence(), "LAMP is now on two objects");
        let mut more = i.clone();
        more.add_git(r("cave"), &{
            let mut g = Struct::new(0);
            g.set("WaypointList", Value::List(vec![tagged("TDOD_START_KREZK")]));
            g
        });
        rules.unique = Patterns::new(&["TDOD_START_*"]).unwrap();
        rules.paste_warning = PasteWarning::Unique;
        let got = clashes(&more, &made, &rules);
        assert_eq!(got.len(), 1);
        assert_eq!(
            got[0].sentence(),
            "TDOD_START_KREZK is now on two objects (the project wants TDOD_START_* unique)"
        );
        rules.paste_warning = PasteWarning::Off;
        assert!(clashes(&more, &made, &rules).is_empty());
    }

    #[test]
    fn a_retag_is_the_tag_with_the_first_free_number() {
        let taken = |t: &str| ["LAMP", "LAMP_2", "STORE_7"].contains(&t);
        assert_eq!(retag("LAMP", &taken), "LAMP_3");
        assert_eq!(retag("STORE_7", &taken), "STORE_8");
        assert_eq!(retag("CHEST", &taken), "CHEST_2");
        assert_eq!(retag("_9", &taken), "_9_2", "no stem: the whole tag is it");
        let long = "A".repeat(40);
        let got = retag(&long, &|_| false);
        assert_eq!(got.len(), MAX_TAG);
        assert!(got.ends_with("A_2"));
        assert_eq!(retag("ÉÉÉÉÉÉÉÉÉÉÉÉÉÉÉÉ", &|_| false).len(), 32, "cut at a letter's edge");
    }

    #[test]
    fn a_store_s_tag_follows_the_pattern_and_is_the_only_one() {
        let taken = |t: &str| ["KREZK_STORE_1", "SHOP"].contains(&t);
        assert_eq!(store_tag("{area}_STORE_{n}", "KREZK", &taken), "KREZK_STORE_2");
        assert_eq!(store_tag("{area}_STORE", "KREZK", &taken), "KREZK_STORE");
        assert_eq!(store_tag("SHOP", "KREZK", &taken), "SHOP_2");
        let long = "X".repeat(40);
        assert_eq!(store_tag("{area}{n}", &long, &|_| false).len(), MAX_TAG);
        let first = "X".repeat(32);
        assert_eq!(store_tag("{area}{n}", &long, &|t| t == first), format!("{}_2", "X".repeat(30)));
    }
}
