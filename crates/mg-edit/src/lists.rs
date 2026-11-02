//! Lists of an object's own (feats, classes, item properties, an
//! inventory...) edited on several objects at once: the list shown merged
//! (an entry some have, and how many), and the edits that add an entry to
//! each that lacks it, take it from each that has it, or set its values on
//! each (`docs/lists-together-proposal.md`). Edit Together's list pages
//! and Advanced Search's changes to what it found build their commands
//! here, so the rules' limits (room for a class, a creature's last class)
//! hold for both.
//!
//! Every insert and removal is built from that object's own list, not the
//! first's; the removals from one list go from its last entry.

use mg_gff::{Struct, Value};
use mg_resman::ResKey;
use mg_rules::ItemProperty;

use crate::{Edit, GffPath};

/// One of the objects edited: its document, the path of the struct the
/// list is in (the object, or a class of a creature's for its spells), the
/// struct as it is, and its name for what is said of it.
#[derive(Debug, Clone, PartialEq)]
pub struct Target {
    pub key: ResKey,
    pub path: GffPath,
    pub at: Struct,
    pub name: String,
}

impl Target {
    /// A target named by its document (a blueprint's resref) or, placed
    /// in an area, by its tag.
    pub fn new(key: ResKey, path: GffPath, at: Struct) -> Target {
        let name = if path.0.is_empty() {
            key.resref.to_string()
        } else {
            let tag = at.string("Tag").map(|t| String::from_utf8_lossy(t).into_owned());
            tag.filter(|t| !t.is_empty()).unwrap_or_else(|| path.to_string())
        };
        Target { key, path, at, name }
    }

    /// The entries of list `list` it has (none without the list).
    pub fn list(&self, list: &str) -> &[Struct] {
        self.at.list(list).unwrap_or(&[])
    }
}

/// An entry of the merged list: its key, the entry as the first object
/// having it has it, and which of the objects have it (their places in
/// the targets given, in order).
#[derive(Debug, Clone, PartialEq)]
pub struct Merged<K> {
    pub key: K,
    pub entry: Struct,
    pub having: Vec<usize>,
}

impl<K> Merged<K> {
    /// Whether all `total` of the objects have it.
    pub fn all(&self, total: usize) -> bool {
        self.having.len() >= total
    }

    /// "≠ 2 of 3" where some have it, nothing where all do.
    pub fn mark(&self, total: usize) -> Option<String> {
        (!self.all(total)).then(|| format!("≠ {} of {total}", self.having.len()))
    }
}

/// The entries of `lists` (each object's list), merged by `key`: the
/// first one's in its order, then the others' new ones in theirs. An
/// entry without a key is left out.
pub fn merge<K: PartialEq + Clone>(
    lists: &[&[Struct]],
    key: impl Fn(&Struct) -> Option<K>,
) -> Vec<Merged<K>> {
    let mut out: Vec<Merged<K>> = Vec::new();
    for (n, list) in lists.iter().enumerate() {
        for entry in *list {
            let Some(k) = key(entry) else { continue };
            match out.iter_mut().find(|m| m.key == k) {
                Some(m) => {
                    if m.having.last() != Some(&n) {
                        m.having.push(n);
                    }
                }
                None => out.push(Merged { key: k, entry: entry.clone(), having: vec![n] }),
            }
        }
    }
    out
}

/// [`merge`] of list `list` of each target.
pub fn merged<K: PartialEq + Clone>(
    targets: &[Target],
    list: &str,
    key: impl Fn(&Struct) -> Option<K>,
) -> Vec<Merged<K>> {
    let lists: Vec<&[Struct]> = targets.iter().map(|t| t.list(list)).collect();
    merge(&lists, key)
}

/// Whether the objects having an entry differ in what `value` reads of
/// it (the first entry of it each has).
pub fn differs<T: PartialEq>(
    targets: &[Target],
    list: &str,
    has: impl Fn(&Struct) -> bool,
    value: impl Fn(&Struct) -> T,
) -> bool {
    let mut seen: Option<T> = None;
    for t in targets {
        let Some(e) = t.list(list).iter().find(|e| has(e)) else { continue };
        let v = value(e);
        match &seen {
            Some(s) if *s != v => return true,
            Some(_) => {}
            None => seen = Some(v),
        }
    }
    false
}

/// The edits a change makes to each object, how many it changed, and
/// those it left, each with why (`mg_b: has it already`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Outcome {
    pub edits: Vec<Edit>,
    pub changed: usize,
    pub left: Vec<String>,
}

impl Outcome {
    fn leave(&mut self, t: &Target, why: impl std::fmt::Display) {
        self.left.push(format!("{}: {why}", t.name));
    }

    /// What it did, for the window's note: "Added to 2; left: mg_b: has
    /// it already". `done` says what was done ("Added to").
    pub fn said(&self, done: &str) -> String {
        let mut s = format!("{done} {}", self.changed);
        if !self.left.is_empty() {
            s += &format!("; left: {}", self.left.join("; "));
        }
        s
    }
}

/// Adds an entry to each object whose list `list` has none that `has`
/// finds, at its end: `make` makes it for that object (an `Err` leaves
/// the object, saying why: no room).
pub fn add(
    targets: &[Target],
    list: &str,
    has: impl Fn(&Struct) -> bool,
    mut make: impl FnMut(&Target) -> Result<Struct, String>,
) -> Outcome {
    let mut out = Outcome::default();
    for t in targets {
        let entries = t.list(list);
        if entries.iter().any(&has) {
            out.leave(t, "has it already");
            continue;
        }
        match make(t) {
            Ok(entry) => {
                out.edits.push(Edit::insert_item(
                    t.key,
                    t.path.clone(),
                    list,
                    entries.len(),
                    entry,
                ));
                out.changed += 1;
            }
            Err(why) => out.leave(t, why),
        }
    }
    out
}

/// Takes every entry of list `list` that `has` finds from each object
/// that has one, but from those `keep` refuses (saying why: a creature's
/// only class).
pub fn remove(
    targets: &[Target],
    list: &str,
    has: impl Fn(&Struct) -> bool,
    keep: impl Fn(&Target) -> Option<String>,
) -> Outcome {
    let mut out = Outcome::default();
    for t in targets {
        let at: Vec<usize> =
            t.list(list).iter().enumerate().filter(|(_, e)| has(e)).map(|(i, _)| i).collect();
        if at.is_empty() {
            continue;
        }
        if let Some(why) = keep(t) {
            out.leave(t, why);
            continue;
        }
        for i in at.into_iter().rev() {
            out.edits.push(Edit::remove_item(t.key, t.path.clone(), list, i));
        }
        out.changed += 1;
    }
    out
}

/// Sets fields on every entry of list `list` that `has` finds, on each
/// object that has one: `value` gives each field's value for the entry as
/// it is (its stored type kept), `None` to leave it.
pub fn set(
    targets: &[Target],
    list: &str,
    has: impl Fn(&Struct) -> bool,
    fields: &[&str],
    value: impl Fn(&Struct, &str) -> Option<Value>,
) -> Outcome {
    let mut out = Outcome::default();
    for t in targets {
        let mut any = false;
        for (i, e) in t.list(list).iter().enumerate().filter(|(_, e)| has(e)) {
            for label in fields {
                let Some(v) = value(e, label) else { continue };
                if e.get(label) == Some(&v) {
                    continue;
                }
                out.edits.push(Edit::SetField {
                    key: t.key,
                    path: t.path.clone().item(list, i),
                    label: (*label).to_string(),
                    value: Some(v),
                });
                any = true;
            }
        }
        out.changed += usize::from(any);
    }
    out
}

/// The edits that make the entries of `list` that `has` finds number
/// `count` on each object: more are copies of the first of them (or of
/// `new` where it has none) after its last, fewer drop its last ones.
pub fn set_count(
    targets: &[Target],
    list: &str,
    has: impl Fn(&Struct) -> bool,
    count: usize,
    new: impl Fn(&Target) -> Struct,
) -> Outcome {
    let mut out = Outcome::default();
    for t in targets {
        let entries = t.list(list);
        let at: Vec<usize> =
            entries.iter().enumerate().filter(|(_, e)| has(e)).map(|(i, _)| i).collect();
        if at.len() == count {
            continue;
        }
        if count > at.len() {
            let (after, first) = match at.first() {
                Some(&f) => (at[at.len() - 1] + 1, entries[f].clone()),
                None => (entries.len(), new(t)),
            };
            for k in 0..count - at.len() {
                out.edits.push(Edit::insert_item(
                    t.key,
                    t.path.clone(),
                    list,
                    after + k,
                    first.clone(),
                ));
            }
        } else {
            for &i in at[count..].iter().rev() {
                out.edits.push(Edit::remove_item(t.key, t.path.clone(), list, i));
            }
        }
        out.changed += 1;
    }
    out
}

/// A value of the type `existing` has (else `default`'s type), as a field
/// set on an entry keeps its type.
pub fn like(existing: Option<&Value>, v: i64, default: Value) -> Value {
    let template = existing.unwrap_or(&default);
    match template {
        Value::Byte(_) => Value::Byte(v.clamp(0, 255) as u8),
        Value::Char(_) => Value::Char(v.clamp(-128, 127) as i8),
        Value::Word(_) => Value::Word(v.clamp(0, 65535) as u16),
        Value::Short(_) => Value::Short(v.clamp(-32768, 32767) as i16),
        Value::Dword(_) => Value::Dword(v.clamp(0, u32::MAX as i64) as u32),
        Value::Int(_) => Value::Int(v.clamp(i32::MIN as i64, i32::MAX as i64) as i32),
        _ => default,
    }
}

// --- The pages' rules -------------------------------------------------

/// The struct id of a creature's list entries as the toolset writes them.
pub const FEAT_ID: u32 = 1;

/// A creature's skill `row` set to `rank` on each: its `SkillList` is
/// indexed by skill, so entries up to the row are added where it is
/// short.
pub fn set_skill_rank(targets: &[Target], row: usize, rank: u8) -> Outcome {
    let mut out = Outcome::default();
    for t in targets {
        let list = t.list("SkillList");
        if list.get(row).and_then(|e| e.integer("Rank")) == Some(i64::from(rank)) {
            continue;
        }
        for i in list.len()..=row {
            let mut s = Struct::new(0);
            s.set("Rank", Value::Byte(0));
            out.edits.push(Edit::insert_item(t.key, t.path.clone(), "SkillList", i, s));
        }
        let old = list.get(row).and_then(|e| e.get("Rank"));
        out.edits.push(Edit::SetField {
            key: t.key,
            path: t.path.clone().item("SkillList", row),
            label: "Rank".into(),
            value: Some(like(old, i64::from(rank), Value::Byte(0))),
        });
        out.changed += 1;
    }
    out
}

/// A creature's skill `row`'s rank (0 where its list is short).
pub fn skill_rank(t: &Target, row: usize) -> i64 {
    t.list("SkillList").get(row).and_then(|e| e.integer("Rank")).unwrap_or(0)
}

/// The feat an entry of `FeatList` is.
pub fn feat(e: &Struct) -> Option<i64> {
    e.integer("Feat")
}

/// Feat `row` given to each creature that lacks it.
pub fn add_feat(targets: &[Target], row: u16) -> Outcome {
    add(
        targets,
        "FeatList",
        |e| feat(e) == Some(i64::from(row)),
        |_| {
            let mut s = Struct::new(FEAT_ID);
            s.set("Feat", Value::Word(row));
            Ok(s)
        },
    )
}

/// Feat `row` taken from each creature that has it.
pub fn remove_feat(targets: &[Target], row: u16) -> Outcome {
    remove(targets, "FeatList", |e| feat(e) == Some(i64::from(row)), |_| None)
}

/// The most classes a creature has (EE).
pub const MAX_CLASSES: usize = 8;

/// The class an entry of `ClassList` is.
pub fn class(e: &Struct) -> Option<i64> {
    e.integer("Class")
}

/// Class `row` added at `level` to each creature that lacks it and has
/// room for it (fewer than [`MAX_CLASSES`]).
pub fn add_class(targets: &[Target], row: i32, level: i16) -> Outcome {
    add(
        targets,
        "ClassList",
        |e| class(e) == Some(i64::from(row)),
        |t| {
            if t.list("ClassList").len() >= MAX_CLASSES {
                return Err(format!("{MAX_CLASSES} classes already"));
            }
            let mut s = Struct::new(2);
            s.set("Class", Value::Int(row));
            s.set("ClassLevel", Value::Short(level));
            Ok(s)
        },
    )
}

/// Class `row` taken from each creature that has it, but from one that
/// has no other class.
pub fn remove_class(targets: &[Target], row: i32) -> Outcome {
    let is = |e: &Struct| class(e) == Some(i64::from(row));
    remove(targets, "ClassList", is, |t| {
        t.list("ClassList").iter().all(is).then(|| "its only class".to_string())
    })
}

/// Class `from` made class `to` on each creature that has it, but on one
/// that has `to` already.
pub fn change_class(targets: &[Target], from: i64, to: i32) -> Outcome {
    let mut out = Outcome::default();
    for t in targets {
        let list = t.list("ClassList");
        let Some(i) = list.iter().position(|e| class(e) == Some(from)) else { continue };
        if list.iter().any(|e| class(e) == Some(i64::from(to))) {
            out.leave(t, "has that class already");
            continue;
        }
        let old = list[i].get("Class");
        out.edits.push(Edit::SetField {
            key: t.key,
            path: t.path.clone().item("ClassList", i),
            label: "Class".into(),
            value: Some(like(old, i64::from(to), Value::Int(0))),
        });
        out.changed += 1;
    }
    out
}

/// The creatures of `targets` that have class `row`, each as the target
/// of that class's own lists (its spells): its entry of `ClassList`.
pub fn with_class(targets: &[Target], row: i64) -> Vec<Target> {
    targets
        .iter()
        .filter_map(|t| {
            let (i, c) =
                t.list("ClassList").iter().enumerate().find(|(_, c)| class(c) == Some(row))?;
            Some(Target {
                key: t.key,
                path: t.path.clone().item("ClassList", i),
                at: c.clone(),
                name: t.name.clone(),
            })
        })
        .collect()
}

/// What an item property is told apart by: its type, subtype, cost value
/// and parameter value.
pub type PropertyKey = (u16, u16, u16, u8);

/// An item property's [`PropertyKey`].
pub fn property_key(p: &ItemProperty) -> PropertyKey {
    (p.property, p.subtype, p.cost_value, p.param1_value)
}

/// Whether an entry of `PropertiesList` is the property of `key`.
pub fn is_property(e: &Struct, key: PropertyKey) -> bool {
    property_key(&ItemProperty::from_gff(e)) == key
}

/// Property `p` given to each item that lacks it so (its type, subtype
/// and value), where `takes` says its base item takes the type (else it
/// is left, said so).
pub fn add_property(
    targets: &[Target],
    p: &ItemProperty,
    takes: impl Fn(&Target) -> bool,
) -> Outcome {
    let key = property_key(p);
    add(
        targets,
        "PropertiesList",
        |e| is_property(e, key),
        |t| {
            if takes(t) { Ok(p.to_gff()) } else { Err("its base item doesn't take it".into()) }
        },
    )
}

/// The property of `key` taken from each item that has it.
pub fn remove_property(targets: &[Target], key: PropertyKey) -> Outcome {
    remove(targets, "PropertiesList", |e| is_property(e, key), |_| None)
}

/// The page (its `StoreList` struct id) of each baseitems.2da
/// `StorePanel`, as Aurora writes a store's pages.
pub const STORE_PANEL_PAGE: [u32; 5] = [0, 4, 2, 3, 1];

/// The first free place for a `w`×`h` item in an inventory grid
/// `width` wide, row by row, among `taken` rectangles (x, y, w, h).
pub fn place(taken: &[(u32, u32, u32, u32)], width: u32, w: u32, h: u32) -> (u32, u32) {
    let w = w.clamp(1, width);
    let free = |x: u32, y: u32| {
        taken
            .iter()
            .all(|&(tx, ty, tw, th)| x + w <= tx || tx + tw <= x || y + h <= ty || ty + th <= y)
    };
    (0..)
        .find_map(|y| (0..=width - w).find(|&x| free(x, y)).map(|x| (x, y)))
        .expect("an empty row fits any item")
}

/// The item blueprint an inventory's entry is of (`InventoryRes`,
/// `EquippedRes`, a whole item's `TemplateResRef`), lowercase.
pub fn item_of(e: &Struct) -> Option<mg_core::ResRef> {
    ["InventoryRes", "EquippedRes", "TemplateResRef"]
        .iter()
        .find_map(|l| e.resref(l))
        .filter(|r| !r.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use mg_core::ResType;

    fn feats(rows: &[u16]) -> Struct {
        let mut s = Struct::new(0);
        let list = rows
            .iter()
            .map(|r| {
                let mut e = Struct::new(FEAT_ID);
                e.set("Feat", Value::Word(*r));
                e
            })
            .collect();
        s.set("FeatList", Value::List(list));
        s
    }

    fn targets(of: Vec<Struct>) -> Vec<Target> {
        of.into_iter()
            .enumerate()
            .map(|(i, s)| {
                let key = ResKey::parse(&format!("mg_{i}"), ResType::UTC).unwrap();
                Target::new(key, GffPath::root(), s)
            })
            .collect()
    }

    #[test]
    fn a_list_is_merged_in_the_first_one_s_order_then_the_others() {
        let t = targets(vec![feats(&[5, 3]), feats(&[3, 9, 3]), feats(&[])]);
        let m = merged(&t, "FeatList", feat);
        let keys: Vec<i64> = m.iter().map(|m| m.key).collect();
        assert_eq!(keys, [5, 3, 9]);
        assert_eq!(m[1].having, [0, 1], "each once, a feat listed twice too");
        assert_eq!(m[0].mark(3).as_deref(), Some("≠ 1 of 3"));
        let all = merged(&t[..1], "FeatList", feat);
        assert_eq!(all[0].mark(1), None);
    }

    #[test]
    fn an_entry_is_added_where_it_lacks_and_taken_where_it_is() {
        let t = targets(vec![feats(&[5, 3]), feats(&[3, 9, 3]), feats(&[])]);
        let added = add_feat(&t, 9);
        assert_eq!(added.changed, 2);
        assert_eq!(added.left, ["mg_1: has it already"]);
        // At the end of each one's own list (the third has none yet).
        let at: Vec<(String, usize)> = added
            .edits
            .iter()
            .map(|e| match e {
                Edit::InsertItem { key, index, .. } => (key.resref.to_string(), *index),
                _ => panic!("{e:?}"),
            })
            .collect();
        assert_eq!(at, [("mg_0".into(), 2), ("mg_2".into(), 0)]);
        // Every copy, from the last.
        let taken = remove_feat(&t, 3);
        assert_eq!(taken.changed, 2);
        let at: Vec<(String, usize)> = taken
            .edits
            .iter()
            .map(|e| match e {
                Edit::RemoveItem { key, index, .. } => (key.resref.to_string(), *index),
                _ => panic!("{e:?}"),
            })
            .collect();
        assert_eq!(at, [("mg_0".into(), 1), ("mg_1".into(), 2), ("mg_1".into(), 0)]);
    }

    #[test]
    fn a_skill_rank_is_set_at_its_index_on_each() {
        let mut short = Struct::new(0);
        let mut e = Struct::new(0);
        e.set("Rank", Value::Byte(4));
        short.set("SkillList", Value::List(vec![e.clone(), e.clone()]));
        let mut long = Struct::new(0);
        long.set("SkillList", Value::List(vec![e.clone(); 6]));
        let t = targets(vec![short, long]);
        let out = set_skill_rank(&t, 4, 7);
        // The short list gains three entries (2, 3, 4), then the rank.
        let inserts = out.edits.iter().filter(|e| matches!(e, Edit::InsertItem { .. })).count();
        assert_eq!(inserts, 3);
        assert_eq!(out.changed, 2);
        // Already so: nothing.
        let same = set_skill_rank(&t[1..], 0, 4);
        assert!(same.edits.is_empty());
    }

    fn classes(rows: &[(i32, i16)]) -> Struct {
        let mut s = Struct::new(0);
        let list = rows
            .iter()
            .map(|(c, l)| {
                let mut e = Struct::new(2);
                e.set("Class", Value::Int(*c));
                e.set("ClassLevel", Value::Short(*l));
                e
            })
            .collect();
        s.set("ClassList", Value::List(list));
        s
    }

    #[test]
    fn a_class_is_added_where_there_is_room_and_never_takes_the_last() {
        let full: Vec<(i32, i16)> = (0..8).map(|c| (c + 10, 1)).collect();
        let t = targets(vec![classes(&[(4, 3)]), classes(&full), classes(&[(4, 1), (1, 2)])]);
        let added = add_class(&t, 1, 1);
        assert_eq!(added.changed, 1);
        assert_eq!(added.left, ["mg_1: 8 classes already", "mg_2: has it already"]);
        let taken = remove_class(&t, 4);
        assert_eq!(taken.changed, 1, "the third, which has another");
        assert_eq!(taken.left, ["mg_0: its only class"]);
        let changed = change_class(&t, 4, 1);
        assert_eq!(changed.changed, 1);
        assert_eq!(changed.left, ["mg_2: has that class already"]);
        // A class's own lists: by its row, wherever it is in the list.
        let fighters = with_class(&t, 1);
        assert_eq!(fighters.len(), 1);
        assert_eq!(fighters[0].path, GffPath::root().item("ClassList", 1));
        // Its level, set on each that has it, its type kept.
        let level = set(
            &t,
            "ClassList",
            |e| class(e) == Some(4),
            &["ClassLevel"],
            |e, l| Some(like(e.get(l), 5, Value::Short(0))),
        );
        assert_eq!(level.changed, 2);
        assert!(matches!(&level.edits[0], Edit::SetField { value: Some(Value::Short(5)), .. }));
        assert!(differs(&t, "ClassList", |e| class(e) == Some(4), |e| e.integer("ClassLevel")));
    }

    #[test]
    fn item_properties_are_matched_by_type_subtype_and_value() {
        let p = |cost: u16| ItemProperty {
            property: 6,
            subtype: 0,
            cost_table: 2,
            cost_value: cost,
            param1: 255,
            param1_value: 0,
            chance: 100,
        };
        let item = |props: &[ItemProperty]| {
            let mut s = Struct::new(0);
            s.set("PropertiesList", Value::List(props.iter().map(ItemProperty::to_gff).collect()));
            s
        };
        let t = targets(vec![item(&[p(1)]), item(&[p(2)]), item(&[])]);
        let m = merged(&t, "PropertiesList", |e| Some(property_key(&ItemProperty::from_gff(e))));
        assert_eq!(m.len(), 2, "+1 and +2 are two entries");
        let added = add_property(&t, &p(1), |t| t.name != "mg_2");
        assert_eq!(added.changed, 1, "the +2 item gains a +1");
        assert_eq!(added.left, ["mg_0: has it already", "mg_2: its base item doesn't take it"]);
        let taken = remove_property(&t, property_key(&p(2)));
        assert_eq!(taken.changed, 1);
    }

    #[test]
    fn a_count_is_set_on_each_from_its_own_entries() {
        let t = targets(vec![feats(&[5, 5, 3]), feats(&[3])]);
        let is = |e: &Struct| feat(e) == Some(5);
        let new = |_: &Target| {
            let mut e = Struct::new(FEAT_ID);
            e.set("Feat", Value::Word(5));
            e
        };
        let out = set_count(&t, "FeatList", is, 1, new);
        assert_eq!(
            out.edits,
            [
                Edit::remove_item(t[0].key, GffPath::root(), "FeatList", 1),
                Edit::insert_item(t[1].key, GffPath::root(), "FeatList", 1, new(&t[1]))
            ]
        );
    }

    #[test]
    fn items_take_the_first_free_place() {
        assert_eq!(place(&[], 10, 2, 3), (0, 0));
        assert_eq!(place(&[(0, 0, 2, 3)], 10, 2, 3), (2, 0));
        let row: Vec<_> = (0..5).map(|i| (i * 2, 0, 2, 1)).collect();
        assert_eq!(place(&row, 10, 1, 1), (0, 1));
    }
}
