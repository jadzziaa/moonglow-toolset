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
use crate::project_file::{PasteWarning, Scope, TagRules};

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

    /// The area it is (an area's own tag) or is placed in.
    pub fn area(self) -> ResRef {
        match self {
            Tagged::Area(area) | Tagged::Object { area, .. } => area,
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

    /// The objects with the tag in one area (the area itself among them).
    pub fn count_in(&self, tag: &[u8], area: ResRef) -> usize {
        self.places(tag).iter().filter(|p| p.area() == area).count()
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

    /// The tags on more than one object of one area, by area, in order:
    /// each tag's places in that area.
    pub fn duplicates_in_areas(&self) -> Vec<(&[u8], Vec<Tagged>)> {
        let mut out = Vec::new();
        for (tag, places) in self.duplicates() {
            let mut areas: Vec<ResRef> = places.iter().map(|p| p.area()).collect();
            areas.sort();
            areas.dedup();
            for area in areas {
                let here: Vec<Tagged> =
                    places.iter().copied().filter(|p| p.area() == area).collect();
                if here.len() > 1 {
                    out.push((tag, here));
                }
            }
        }
        out
    }

    /// The tags the project wants unique that are not (`[tags] unique`:
    /// on more than one object of the module; `unique_in_area`: of one
    /// area, each area's said apart), in order.
    pub fn breaches<'a>(&'a self, rules: &'a TagRules) -> Vec<Breach<'a>> {
        if !rules.any_unique() {
            return Vec::new();
        }
        let mut out = Vec::new();
        for (tag, places) in self.duplicates() {
            let text = String::from_utf8_lossy(tag);
            if let Some((pattern, Scope::Module)) = rules.unique_for(&text) {
                out.push(Breach { tag, pattern, scope: Scope::Module, places: places.to_vec() });
            }
        }
        for (tag, places) in self.duplicates_in_areas() {
            let text = String::from_utf8_lossy(tag);
            if let Some((pattern, Scope::Area)) = rules.unique_for(&text) {
                out.push(Breach { tag, pattern, scope: Scope::Area, places });
            }
        }
        out.sort_by(|a, b| (a.tag, &a.places).cmp(&(b.tag, &b.places)));
        out
    }
}

/// A tag the project wants unique on more than one object (of the
/// module, or of one area): [`TagIndex::breaches`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Breach<'a> {
    pub tag: &'a [u8],
    /// The project's pattern that wants it unique.
    pub pattern: &'a str,
    pub scope: Scope,
    /// Its places (of the area, for [`Scope::Area`]), in order.
    pub places: Vec<Tagged>,
}

/// A tag a paste or a copy put on more than one object.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Clash {
    pub tag: String,
    /// How many objects of the module have it now (of `area`, where the
    /// project wants it unique in each area).
    pub count: usize,
    /// The project's pattern it matches, of those that must be unique.
    pub unique: Option<String>,
    /// The area it is twice in, where the project wants it unique in each
    /// area (`unique_in_area`) and counts it there.
    pub area: Option<ResRef>,
}

impl Clash {
    /// "TAG is now on two objects", for people.
    pub fn sentence(&self) -> String {
        let n = match self.count {
            2 => "two".to_string(),
            3 => "three".to_string(),
            n => n.to_string(),
        };
        match (&self.unique, self.area) {
            (Some(p), Some(area)) => format!(
                "{} is now on {n} objects in {area} (the project wants {p} unique in each area)",
                self.tag
            ),
            (Some(p), None) => {
                format!("{} is now on {n} objects (the project wants {p} unique)", self.tag)
            }
            (None, _) => format!("{} is now on {n} objects", self.tag),
        }
    }
}

/// An object just pasted or copied, as [`clashes`] weighs it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MadeTag<'a> {
    pub tag: &'a [u8],
    /// The area it is (an area copied) or was made in.
    pub area: ResRef,
    /// A builder set the tag ([`is_custom_tag`]).
    pub custom: bool,
    /// A static placeable: scenery, never warned of.
    pub is_static: bool,
}

/// Whether a builder set an object's tag: it isn't its blueprint's own
/// (`blueprint`: the one its `TemplateResRef` names, as found in the
/// module, its haks or the game), or the blueprint isn't found (an area,
/// which has none, too).
pub fn is_custom_tag(object: &Struct, blueprint: Option<&Struct>) -> bool {
    blueprint.is_none_or(|b| {
        b.string("Tag").unwrap_or_default() != object.string("Tag").unwrap_or_default()
    })
}

/// The tags of `made` (those just pasted or copied) that are now on more
/// than one object of `index` (which has them), as `warning` says to warn
/// of them (the project's rule or the builder's option,
/// [`TagRules::paste_warning_or`]; `custom`: tags a builder set and those
/// that must be unique). A static placeable's never. A tag the project
/// wants unique in each area (`unique_in_area`) is counted in the area it
/// was made in, whatever the rule: other areas' are as meant.
pub fn clashes(
    index: &TagIndex,
    made: &[MadeTag],
    rules: &TagRules,
    warning: PasteWarning,
) -> Vec<Clash> {
    if warning == PasteWarning::Off {
        return Vec::new();
    }
    let mut seen: HashSet<(&[u8], Option<ResRef>)> = HashSet::new();
    let mut out = Vec::new();
    for m in made {
        let tag = m.tag;
        if tag.is_empty() || m.is_static || index.count(tag) < 2 {
            continue;
        }
        let text = String::from_utf8_lossy(tag).into_owned();
        let rule = rules.unique_for(&text);
        let (count, area) = match rule {
            Some((_, Scope::Area)) => (index.count_in(tag, m.area), Some(m.area)),
            _ => (index.count(tag), None),
        };
        if count < 2 || seen.contains(&(tag, area)) {
            continue;
        }
        let unique = rule.map(|(p, _)| p.to_string());
        let warned = match warning {
            PasteWarning::Any => true,
            PasteWarning::Custom => m.custom || unique.is_some(),
            PasteWarning::Unique => unique.is_some(),
            PasteWarning::Off => false,
        };
        if warned {
            seen.insert((tag, area));
            out.push(Clash { tag: text, count, unique, area });
        }
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

    fn made(tag: &[u8], custom: bool, is_static: bool) -> MadeTag<'_> {
        MadeTag { tag, area: r("inn"), custom, is_static }
    }

    #[test]
    fn a_paste_says_what_it_made_twice_as_the_project_wants() {
        let i = index();
        let mut rules = TagRules::default();
        // The default: tags a builder set; a blueprint's own is quiet.
        let quiet = [made(b"LAMP", false, false), made(b"CHEST", true, false)];
        assert!(
            clashes(&i, &quiet, &rules, rules.paste_warning_or(true)).is_empty(),
            "LAMP is its blueprint's, CHEST once"
        );
        let made_custom = [made(b"LAMP", false, false), made(b"LAMP", true, false)];
        let got = clashes(&i, &made_custom, &rules, rules.paste_warning_or(true));
        assert_eq!(got, [Clash { tag: "LAMP".into(), count: 2, unique: None, area: None }]);
        assert_eq!(got[0].sentence(), "LAMP is now on two objects");
        // The builder's option off (Options › General), the project saying
        // nothing and wanting no tag unique: none.
        assert!(clashes(&i, &made_custom, &rules, rules.paste_warning_or(false)).is_empty());
        // A static placeable never.
        assert!(
            clashes(&i, &[made(b"LAMP", true, true)], &rules, rules.paste_warning_or(true))
                .is_empty()
        );
        rules.paste_warning = Some(PasteWarning::Any);
        assert_eq!(
            clashes(&i, &[made(b"LAMP", false, false)], &rules, rules.paste_warning_or(true)).len(),
            1
        );
        assert!(
            clashes(&i, &[made(b"LAMP", false, true)], &rules, rules.paste_warning_or(true))
                .is_empty()
        );
        let made: [MadeTag; 4] = [
            made(b"LAMP", false, false),
            made(b"LAMP", false, false),
            made(b"CHEST", false, false),
            made(b"TDOD_START_KREZK", false, false),
        ];
        let mut more = i.clone();
        more.add_git(r("cave"), &{
            let mut g = Struct::new(0);
            g.set("WaypointList", Value::List(vec![tagged("TDOD_START_KREZK")]));
            g
        });
        rules.unique = Patterns::new(&["TDOD_START_*"]).unwrap();
        // The unique ones always, the blueprint's own tag or not.
        rules.paste_warning = Some(PasteWarning::Custom);
        assert_eq!(clashes(&more, &made, &rules, rules.paste_warning_or(true)).len(), 1);
        rules.paste_warning = Some(PasteWarning::Unique);
        let got = clashes(&more, &made, &rules, rules.paste_warning_or(true));
        assert_eq!(got.len(), 1);
        assert_eq!(
            got[0].sentence(),
            "TDOD_START_KREZK is now on two objects (the project wants TDOD_START_* unique)"
        );
        rules.paste_warning = Some(PasteWarning::Off);
        assert!(clashes(&more, &made, &rules, rules.paste_warning_or(true)).is_empty());
    }

    /// The Domains of Dread's notes on their setup, 5: a store's tag is
    /// looked for in its own area, so another area may have one too.
    #[test]
    fn a_tag_unique_in_each_area_is_counted_in_its_own() {
        let store = |tag: &str| {
            let mut g = Struct::new(0);
            g.set("StoreList", Value::List(vec![tagged(tag)]));
            g
        };
        let mut i = index();
        i.add_git(r("cave"), &store("KZ_STORE_1"));
        i.add_git(r("mill"), &store("KZ_STORE_1"));
        let mut rules = TagRules {
            unique: Patterns::new(&["KZ_*"]).unwrap(),
            unique_in_area: Patterns::new(&["*_STORE_*"]).unwrap(),
            paste_warning: Some(PasteWarning::Unique),
        };
        assert_eq!(i.count_in(b"KZ_STORE_1", r("cave")), 1);
        // Two areas, one each: as meant, though KZ_* takes the tag in too.
        assert_eq!(rules.unique_for("KZ_STORE_1"), Some(("*_STORE_*", Scope::Area)));
        assert!(i.breaches(&rules).is_empty());
        let in_mill =
            MadeTag { tag: b"KZ_STORE_1", area: r("mill"), custom: true, is_static: false };
        assert!(clashes(&i, &[in_mill], &rules, rules.paste_warning_or(true)).is_empty());
        rules.paste_warning = Some(PasteWarning::Any);
        assert!(
            clashes(&i, &[in_mill], &rules, rules.paste_warning_or(true)).is_empty(),
            "other areas' are as meant"
        );
        // A second in the mill: said, with the area.
        i.add_git(r("mill"), &store("KZ_STORE_1"));
        let got = clashes(&i, &[in_mill, in_mill], &rules, rules.paste_warning_or(true));
        assert_eq!(
            got,
            [Clash {
                tag: "KZ_STORE_1".into(),
                count: 2,
                unique: Some("*_STORE_*".into()),
                area: Some(r("mill"))
            }]
        );
        assert_eq!(
            got[0].sentence(),
            "KZ_STORE_1 is now on two objects in mill (the project wants *_STORE_* unique in each \
             area)"
        );
        let b = i.breaches(&rules);
        assert_eq!(b.len(), 1, "{b:?}");
        assert_eq!((b[0].pattern, b[0].scope), ("*_STORE_*", Scope::Area));
        let at: Vec<String> = b[0].places.iter().map(|p| p.describe()).collect();
        assert_eq!(at, ["store 0 in mill", "store 0 in mill"]);
        // Module-wide, a pattern of `unique` alone: every area's together.
        rules.unique_in_area = Patterns::default();
        let b = i.breaches(&rules);
        assert_eq!(b.len(), 1);
        assert_eq!((b[0].scope, b[0].places.len()), (Scope::Module, 3));
        let got = clashes(&i, &[in_mill], &rules, rules.paste_warning_or(true));
        assert_eq!((got[0].count, got[0].area), (3, None));
        assert!(TagIndex::default().breaches(&TagRules::default()).is_empty());
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
