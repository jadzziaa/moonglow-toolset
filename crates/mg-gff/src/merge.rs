//! Merging two GFF trees that were both made from a third (a three-way
//! merge, as git's merge driver needs one).
//!
//! What one side changed is taken; where both changed a value, the same
//! change is one and different changes are a conflict (the preferred
//! side's value is taken, and the conflict is named by its path).
//!
//! Structs merge field by field: a field is its label (the second field
//! of one label, in a file that has two, its label and that it is the
//! second). The merged struct has ours' fields in ours' order, then those
//! theirs added. Lists merge by item, and what an item *is* the caller's
//! [`ListRule`] says, list by list: its place in the list, the value of a
//! field or two (a variable's name), its struct id, or, for objects placed
//! in an area, a match made in steps ([`ListRule::Placed`]). A list only
//! one side changed is that side's, whole (its order too).

use std::collections::HashMap;

use crate::Gff;
use crate::value::{Label, Struct, Value};
use mg_core::{LocString, LocStringKey};

/// Whose value a conflict takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Prefer {
    Ours,
    Theirs,
}

/// What tells a list's items apart.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListRule {
    /// An item is its place in the list (a tile, a skill, a conversation
    /// node that others name by number). Two items added at one place are
    /// a conflict.
    ByIndex,
    /// An item is the values of these fields (a variable's name, an
    /// area's resref). Kept items stay in the base's order, then come
    /// ours' new ones, then theirs'; the same key added by both with
    /// different contents is a conflict. Items of one key (malformed, but
    /// seen) pair in order.
    ByKey(&'static [&'static str]),
    /// An item is its struct id (the slot of an equipped item).
    ByStructId,
    /// Objects placed in an area, which carry no id of their own. An item
    /// of the base is the item of a side that is, in this order: the same
    /// in every field; of the same `ObjectId`, where both have one; of the
    /// same `TemplateResRef`, `Tag` and position (`XPosition`/`YPosition`/
    /// `ZPosition`, or `X`/`Y`/`Z`, to 4 decimal places); the only one of
    /// its `TemplateResRef` and `Tag` left (an object moved); the only one
    /// of its `TemplateResRef` and position left (an object retagged).
    /// Anything else is a removed object and an added one. Objects both
    /// sides added are all kept (one, where both added the same object).
    Placed,
}

/// A value both sides changed, each to something else, or one changed
/// and the other removed (`None`: not there).
#[derive(Debug, Clone, PartialEq)]
pub struct Conflict {
    /// Where, as `/Placeable List[plc_chest "CHEST" at 10, 4.5]/Locked`.
    pub path: String,
    pub base: Option<Value>,
    pub ours: Option<Value>,
    pub theirs: Option<Value>,
}

/// Two trees merged.
#[derive(Debug, Clone, PartialEq)]
pub struct Merged {
    /// The merged tree (with the preferred side's value at each conflict).
    pub gff: Gff,
    pub conflicts: Vec<Conflict>,
}

/// Merges `ours` and `theirs`, both made from `base`. `rules` says what an
/// item of a list is, by the file's type (`"GIT"`, without the trailing
/// space) and the list's label.
pub fn merge(
    base: &Gff,
    ours: &Gff,
    theirs: &Gff,
    prefer: Prefer,
    rules: &dyn Fn(&str, &str) -> ListRule,
) -> Merged {
    let mut m = Merger { file_type: ours.file_type_str(), rules, prefer, conflicts: Vec::new() };
    let header = |g: &Gff| Value::String([g.file_type, g.version].concat());
    let (hb, ho, ht) = (header(base), header(ours), header(theirs));
    let top = match m.scalar("(file type)", Some(&hb), Some(&ho), Some(&ht)) {
        Some(Value::String(h)) if h.len() == 8 => h,
        _ => [ours.file_type, ours.version].concat(),
    };
    let root = m.strukt("", &base.root, &ours.root, &theirs.root);
    let gff = Gff {
        file_type: top[..4].try_into().expect("four bytes"),
        version: top[4..].try_into().expect("four bytes"),
        root,
    };
    Merged { gff, conflicts: m.conflicts }
}

struct Merger<'a> {
    file_type: String,
    rules: &'a dyn Fn(&str, &str) -> ListRule,
    prefer: Prefer,
    conflicts: Vec<Conflict>,
}

/// The three-way choice: a side that left the base as it was takes the
/// other's; two that agree take either. `None` where they disagree.
fn settled<T: PartialEq + Clone>(b: &T, o: &T, t: &T) -> Option<T> {
    if o == b {
        Some(t.clone())
    } else if t == b || o == t {
        Some(o.clone())
    } else {
        None
    }
}

impl Merger<'_> {
    /// A value both changed that cannot be merged inside: a conflict.
    fn scalar(
        &mut self,
        path: &str,
        b: Option<&Value>,
        o: Option<&Value>,
        t: Option<&Value>,
    ) -> Option<Value> {
        if let Some(v) = settled(&b, &o, &t) {
            return v.cloned();
        }
        self.conflict(path, b, o, t)
    }

    fn conflict(
        &mut self,
        path: &str,
        b: Option<&Value>,
        o: Option<&Value>,
        t: Option<&Value>,
    ) -> Option<Value> {
        self.conflicts.push(Conflict {
            path: path.to_string(),
            base: b.cloned(),
            ours: o.cloned(),
            theirs: t.cloned(),
        });
        match self.prefer {
            Prefer::Ours => o.cloned(),
            Prefer::Theirs => t.cloned(),
        }
    }

    fn value(&mut self, path: &str, label: &str, b: &Value, o: &Value, t: &Value) -> Option<Value> {
        if let Some(v) = settled(b, o, t) {
            return Some(v);
        }
        match (b, o, t) {
            (Value::Struct(b), Value::Struct(o), Value::Struct(t)) => {
                Some(Value::Struct(self.strukt(path, b, o, t)))
            }
            (Value::List(b), Value::List(o), Value::List(t)) => {
                Some(Value::List(self.list(path, label, b, o, t)))
            }
            (Value::LocString(b), Value::LocString(o), Value::LocString(t)) => {
                Some(Value::LocString(self.locstring(path, b, o, t)))
            }
            _ => self.conflict(path, Some(b), Some(o), Some(t)),
        }
    }

    /// A localized string both changed, merged language by language (and
    /// its talk-table line as a value): one side's English and the other's
    /// French are both kept; a conflict is the one language's.
    fn locstring(&mut self, path: &str, b: &LocString, o: &LocString, t: &LocString) -> LocString {
        let strref = |s: &LocString| Value::Dword(s.strref.0);
        let strref = match self.scalar(
            &format!("{path}/(strref)"),
            Some(&strref(b)),
            Some(&strref(o)),
            Some(&strref(t)),
        ) {
            Some(Value::Dword(v)) => mg_core::StrRef(v),
            _ => o.strref,
        };
        let text = |s: &LocString, k: LocStringKey| {
            s.strings.iter().find(|(key, _)| *key == k).map(|(_, t)| Value::String(t.clone()))
        };
        let mut keys: Vec<LocStringKey> = Vec::new();
        for (k, _) in o.strings.iter().chain(&t.strings).chain(&b.strings) {
            if !keys.contains(k) {
                keys.push(*k);
            }
        }
        let mut out = LocString { strref, strings: Vec::new() };
        for k in keys {
            let (bv, ov, tv) = (text(b, k), text(o, k), text(t, k));
            let p = format!("{path}/[{}]", k.0);
            if let Some(Value::String(s)) = self.scalar(&p, bv.as_ref(), ov.as_ref(), tv.as_ref()) {
                out.strings.push((k, s));
            }
        }
        out
    }

    fn strukt(&mut self, path: &str, b: &Struct, o: &Struct, t: &Struct) -> Struct {
        if let Some(s) = settled(b, o, t) {
            return s;
        }
        let id = |s: &Struct| Value::Dword(s.id);
        let id = match self.scalar(
            &format!("{path}/(struct id)"),
            Some(&id(b)),
            Some(&id(o)),
            Some(&id(t)),
        ) {
            Some(Value::Dword(id)) => id,
            _ => o.id,
        };
        let (bk, ok, tk) = (keyed(b), keyed(o), keyed(t));
        let mut out = Struct::new(id);
        let push = |out: &mut Struct, label: Label, v: Option<Value>| {
            if let Some(value) = v {
                out.fields.push(crate::Field { label, value });
            }
        };
        for &(k, ov) in &ok {
            let p = field_path(path, &k);
            let label = k.0.to_string_lossy();
            let v = match (find(&bk, &k), find(&tk, &k)) {
                (Some(bv), Some(tv)) => self.value(&p, &label, bv, ov, tv),
                // Theirs took it away.
                (Some(bv), None) => match ov == bv {
                    true => None,
                    false => self.conflict(&p, Some(bv), Some(ov), None),
                },
                // Both added it: two lists added are merged as if from
                // an empty one.
                (None, Some(tv)) => match (ov, tv) {
                    _ if ov == tv => Some(ov.clone()),
                    (Value::List(ol), Value::List(tl)) => {
                        Some(Value::List(self.list(&p, &label, &[], ol, tl)))
                    }
                    _ => self.conflict(&p, None, Some(ov), Some(tv)),
                },
                (None, None) => Some(ov.clone()),
            };
            push(&mut out, k.0, v);
        }
        for &(k, tv) in &tk {
            if find(&ok, &k).is_some() {
                continue;
            }
            let v = match find(&bk, &k) {
                // Ours took it away.
                Some(bv) => match tv == bv {
                    true => None,
                    false => self.conflict(&field_path(path, &k), Some(bv), None, Some(tv)),
                },
                None => Some(tv.clone()),
            };
            push(&mut out, k.0, v);
        }
        out
    }

    fn list(
        &mut self,
        path: &str,
        label: &str,
        b: &[Struct],
        o: &[Struct],
        t: &[Struct],
    ) -> Vec<Struct> {
        if let Some(l) = settled(&b, &o, &t) {
            return l.to_vec();
        }
        match (self.rules)(&self.file_type, label) {
            ListRule::ByIndex => self.by_index(path, b, o, t),
            rule => self.by_identity(path, rule, b, o, t),
        }
    }

    fn by_index(&mut self, path: &str, b: &[Struct], o: &[Struct], t: &[Struct]) -> Vec<Struct> {
        let len = b.len().max(o.len()).max(t.len());
        let mut out = Vec::with_capacity(len);
        for i in 0..len {
            let p = format!("{path}[{i}]");
            let (bi, oi, ti) = (b.get(i), o.get(i), t.get(i));
            let item = match (settled(&bi, &oi, &ti), bi, oi, ti) {
                (Some(s), ..) => s.cloned(),
                (None, Some(bi), Some(oi), Some(ti)) => Some(self.strukt(&p, bi, oi, ti)),
                _ => {
                    let v = |s: Option<&Struct>| s.map(|s| Value::Struct(s.clone()));
                    match self.conflict(&p, v(bi).as_ref(), v(oi).as_ref(), v(ti).as_ref()) {
                        Some(Value::Struct(s)) => Some(s),
                        _ => None,
                    }
                }
            };
            out.extend(item);
        }
        out
    }

    /// A list whose items are told by what they are, merged item by
    /// item, in sequence: the base's items kept in the order a side that
    /// reordered them gave (the base's where both did, each its own way),
    /// and each side's new items after the item they follow there (at the
    /// top where they lead), ours before theirs at one place.
    fn by_identity(
        &mut self,
        path: &str,
        rule: ListRule,
        b: &[Struct],
        o: &[Struct],
        t: &[Struct],
    ) -> Vec<Struct> {
        let (om, tm) = (matching(rule, b, o), matching(rule, b, t));
        let item_path = |s: &Struct| format!("{path}[{}]", describe(rule, s));
        let as_value = |s: &Struct| Value::Struct(s.clone());
        // The base's items, merged; `None` where gone.
        let mut kept: Vec<Option<Struct>> = Vec::with_capacity(b.len());
        for (i, bi) in b.iter().enumerate() {
            let p = item_path(bi);
            let item = match (om[i].map(|j| &o[j]), tm[i].map(|j| &t[j])) {
                (Some(oi), Some(ti)) => Some(self.strukt(&p, bi, oi, ti)),
                (Some(oi), None) if oi == bi => None,
                (None, Some(ti)) if ti == bi => None,
                (Some(oi), None) => self.removed_and_changed(&p, bi, Some(oi), None),
                (None, Some(ti)) => self.removed_and_changed(&p, bi, None, Some(ti)),
                (None, None) => None,
            };
            kept.push(item);
        }
        // Their order: the base's, unless one side moved them.
        let order_on = |m: &[Option<usize>]| -> Vec<usize> {
            let mut on: Vec<usize> = (0..b.len()).filter(|&i| kept[i].is_some()).collect();
            on.sort_by_key(|&i| m[i].unwrap_or(usize::MAX));
            on
        };
        let base_order: Vec<usize> = (0..b.len()).filter(|&i| kept[i].is_some()).collect();
        let (o_order, t_order) = (order_on(&om), order_on(&tm));
        let order = match (o_order != base_order, t_order != base_order) {
            (true, false) => o_order,
            (false, true) => t_order,
            (true, true) if o_order == t_order => o_order,
            _ => base_order,
        };
        // Each side's new items, with the base item they follow on it.
        let added = |side: &[Struct], m: &[Option<usize>]| -> Vec<(Option<usize>, usize)> {
            let mut of_side: Vec<Option<usize>> = vec![None; side.len()];
            for (i, j) in m.iter().enumerate() {
                if let Some(j) = j
                    && kept[i].is_some()
                {
                    of_side[*j] = Some(i);
                }
            }
            let used: Vec<bool> = (0..side.len()).map(|j| m.contains(&Some(j))).collect();
            let mut after = None;
            let mut out = Vec::new();
            for j in 0..side.len() {
                if let Some(i) = of_side[j] {
                    after = Some(i);
                } else if !used[j] {
                    out.push((after, j));
                }
            }
            out
        };
        let mut ours_added: Vec<(Option<usize>, Struct)> =
            added(o, &om).into_iter().map(|(a, j)| (a, o[j].clone())).collect();
        let mut theirs_added: Vec<(Option<usize>, Struct)> = Vec::new();
        for (after, j) in added(t, &tm) {
            let ti = &t[j];
            // The same item added by both is one.
            if ours_added.iter().any(|(_, s)| s == ti) {
                continue;
            }
            let same_key = match rule {
                ListRule::Placed => None,
                _ => {
                    let k = key(rule, ti);
                    ours_added.iter().position(|(_, s)| k.is_some() && key(rule, s) == k)
                }
            };
            match same_key {
                Some(n) => {
                    let p = item_path(ti);
                    let ours = as_value(&ours_added[n].1);
                    if let Some(Value::Struct(s)) =
                        self.conflict(&p, None, Some(&ours), Some(&as_value(ti)))
                    {
                        ours_added[n].1 = s;
                    }
                }
                None => theirs_added.push((after, ti.clone())),
            }
        }
        let mut out = Vec::with_capacity(order.len() + ours_added.len() + theirs_added.len());
        let mut place = |out: &mut Vec<Struct>, at: Option<usize>| {
            for list in [&mut ours_added, &mut theirs_added] {
                let (here, rest): (Vec<_>, Vec<_>) = list.drain(..).partition(|(a, _)| *a == at);
                out.extend(here.into_iter().map(|(_, s)| s));
                *list = rest;
            }
        };
        place(&mut out, None);
        for i in order {
            out.extend(kept[i].take());
            place(&mut out, Some(i));
        }
        out
    }

    fn removed_and_changed(
        &mut self,
        path: &str,
        b: &Struct,
        o: Option<&Struct>,
        t: Option<&Struct>,
    ) -> Option<Struct> {
        let v = |s: Option<&Struct>| s.map(|s| Value::Struct(s.clone()));
        match self.conflict(path, v(Some(b)).as_ref(), v(o).as_ref(), v(t).as_ref()) {
            Some(Value::Struct(s)) => Some(s),
            _ => None,
        }
    }
}

/// A field as one of its struct's: its label, and which of the fields of
/// that label it is.
type FieldKey = (Label, usize);

fn keyed(s: &Struct) -> Vec<(FieldKey, &Value)> {
    let mut seen: HashMap<Label, usize> = HashMap::new();
    (s.fields.iter())
        .map(|f| {
            let n = seen.entry(f.label).or_default();
            *n += 1;
            ((f.label, *n - 1), &f.value)
        })
        .collect()
}

fn find<'a>(fields: &[(FieldKey, &'a Value)], k: &FieldKey) -> Option<&'a Value> {
    fields.iter().find(|(key, _)| key == k).map(|(_, v)| *v)
}

fn field_path(path: &str, (label, n): &FieldKey) -> String {
    match n {
        0 => format!("{path}/{label}"),
        n => format!("{path}/{label}#{}", n + 1),
    }
}

/// A value as bytes to compare by: resrefs whatever their case, floats to
/// 4 decimal places.
fn key_bytes(v: &Value, out: &mut Vec<u8>) {
    out.push(v.field_type() as u8);
    let mut bytes = |b: &[u8]| {
        out.extend_from_slice(&(b.len() as u32).to_le_bytes());
        out.extend_from_slice(b);
    };
    match v {
        Value::Float(f) => bytes(&((*f as f64 * 1e4).round() as i64).to_le_bytes()),
        Value::Double(f) => bytes(&((f * 1e4).round() as i64).to_le_bytes()),
        Value::String(s) => bytes(s),
        Value::ResRef(s) => bytes(&s.to_ascii_lowercase()),
        other => match integer(other) {
            Some(i) => bytes(&i.to_le_bytes()),
            None => bytes(format!("{other:?}").as_bytes()),
        },
    }
}

fn integer(v: &Value) -> Option<i128> {
    Some(match v {
        Value::Byte(x) => (*x).into(),
        Value::Char(x) => (*x).into(),
        Value::Word(x) => (*x).into(),
        Value::Short(x) => (*x).into(),
        Value::Dword(x) => (*x).into(),
        Value::Int(x) => (*x).into(),
        Value::Dword64(x) => (*x).into(),
        Value::Int64(x) => (*x).into(),
        _ => return None,
    })
}

fn fields_key(s: &Struct, labels: &[&str]) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    for l in labels {
        key_bytes(s.get(l)?, &mut out);
    }
    Some(out)
}

const POSITION: [&str; 3] = ["XPosition", "YPosition", "ZPosition"];
const POINT: [&str; 3] = ["X", "Y", "Z"];

fn position_key(s: &Struct) -> Option<Vec<u8>> {
    fields_key(s, &POSITION).or_else(|| fields_key(s, &POINT))
}

/// An item's identity in one step of a match: `None` where it has none.
type KeyOf = fn(&Struct) -> Option<Vec<u8>>;

/// The steps of [`ListRule::Placed`] after the first (the same in every
/// field), each a key and whether it pairs only where it is the only one
/// of its key left on both sides. The last, the blueprint alone, pairs an
/// object one side both moved and retagged.
fn placed_steps() -> [(KeyOf, bool); 5] {
    fn join(a: Option<Vec<u8>>, b: Option<Vec<u8>>) -> Option<Vec<u8>> {
        Some([a?, b?].concat())
    }
    [
        (|s| fields_key(s, &["ObjectId"]), false),
        (|s| join(fields_key(s, &["TemplateResRef", "Tag"]), position_key(s)), false),
        (|s| fields_key(s, &["TemplateResRef", "Tag"]), true),
        (|s| join(fields_key(s, &["TemplateResRef"]), position_key(s)), true),
        (|s| fields_key(s, &["TemplateResRef"]), true),
    ]
}

/// An item's identity under `rule` (`None`: it has none, and pairs only
/// with an item the same in every field).
fn key(rule: ListRule, s: &Struct) -> Option<Vec<u8>> {
    match rule {
        ListRule::ByIndex | ListRule::Placed => None,
        ListRule::ByKey(labels) => fields_key(s, labels),
        ListRule::ByStructId => Some(s.id.to_le_bytes().to_vec()),
    }
}

/// For each item of `base`, the item of `side` that is it.
fn matching(rule: ListRule, base: &[Struct], side: &[Struct]) -> Vec<Option<usize>> {
    let mut of_base: Vec<Option<usize>> = vec![None; base.len()];
    let mut used = vec![false; side.len()];
    // The same in every field, in order (mostly at the same place, so
    // looked for there first).
    let mut next = 0;
    for (i, b) in base.iter().enumerate() {
        let at =
            (next..side.len()).chain(0..next.min(side.len())).find(|&j| !used[j] && side[j] == *b);
        if let Some(j) = at {
            (of_base[i], used[j], next) = (Some(j), true, j + 1);
        }
    }
    let mut pair = |key: &dyn Fn(&Struct) -> Option<Vec<u8>>, only: bool| {
        let mut left: HashMap<Vec<u8>, (Vec<usize>, Vec<usize>)> = HashMap::new();
        for (i, b) in base.iter().enumerate().filter(|(i, _)| of_base[*i].is_none()) {
            if let Some(k) = key(b) {
                left.entry(k).or_default().0.push(i);
            }
        }
        for (j, s) in side.iter().enumerate().filter(|(j, _)| !used[*j]) {
            if let Some(k) = key(s)
                && let Some(e) = left.get_mut(&k)
            {
                e.1.push(j);
            }
        }
        for (bs, ss) in left.values() {
            if only && (bs.len() != 1 || ss.len() != 1) {
                continue;
            }
            for (&i, &j) in bs.iter().zip(ss) {
                (of_base[i], used[j]) = (Some(j), true);
            }
        }
    };
    match rule {
        ListRule::ByIndex => {}
        ListRule::Placed => {
            for (key, only) in placed_steps() {
                pair(&key, only);
            }
        }
        rule => pair(&|s| key(rule, s), false),
    }
    of_base
}

fn text(v: Option<&Value>) -> String {
    match v {
        Some(Value::String(b) | Value::ResRef(b)) => String::from_utf8_lossy(b).into_owned(),
        Some(Value::Float(f)) => format!("{f}"),
        Some(Value::Double(f)) => format!("{f}"),
        Some(other) => integer(other).map_or_else(|| "?".into(), |i| i.to_string()),
        None => "?".into(),
    }
}

/// An item as a path names it: by what it is.
fn describe(rule: ListRule, s: &Struct) -> String {
    match rule {
        ListRule::ByIndex => "?".into(),
        ListRule::ByKey(labels) => {
            labels.iter().map(|l| text(s.get(l))).collect::<Vec<_>>().join(", ")
        }
        ListRule::ByStructId => format!("#{}", s.id),
        ListRule::Placed => {
            let at = if s.contains("XPosition") { POSITION } else { POINT };
            let (x, y) = (text(s.get(at[0])), text(s.get(at[1])));
            format!("{} \"{}\" at {x}, {y}", text(s.get("TemplateResRef")), text(s.get("Tag")))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn var(name: &str, value: i32) -> Struct {
        let mut s = Struct::new(0);
        s.set("Name", Value::String(name.into()));
        s.set("Type", Value::Dword(1));
        s.set("Value", Value::Int(value));
        s
    }

    fn placed(template: &str, tag: &str, x: f32, y: f32) -> Struct {
        let mut s = Struct::new(9);
        s.set("Appearance", Value::Dword(1));
        s.set("Tag", Value::String(tag.into()));
        s.set("TemplateResRef", Value::ResRef(template.into()));
        s.set("X", Value::Float(x));
        s.set("Y", Value::Float(y));
        s.set("Z", Value::Float(0.0));
        s
    }

    fn rules(_: &str, label: &str) -> ListRule {
        match label {
            "VarTable" => ListRule::ByKey(&["Name", "Type"]),
            "Mod_Area_list" => ListRule::ByKey(&["Area_Name"]),
            "Placeable List" => ListRule::Placed,
            "Equip_ItemList" => ListRule::ByStructId,
            _ => ListRule::ByIndex,
        }
    }

    fn gff(root: Struct) -> Gff {
        Gff { file_type: *b"GIT ", version: Gff::VERSION, root }
    }

    fn root(fields: &[(&str, Value)]) -> Struct {
        let mut s = Struct::new(crate::ROOT_STRUCT_ID);
        for (l, v) in fields {
            s.set(l, v.clone());
        }
        s
    }

    fn run(b: &Struct, o: &Struct, t: &Struct, prefer: Prefer) -> Merged {
        merge(&gff(b.clone()), &gff(o.clone()), &gff(t.clone()), prefer, &rules)
    }

    fn labels(s: &Struct) -> Vec<String> {
        s.fields.iter().map(|f| f.label.to_string()).collect()
    }

    #[test]
    fn a_field_one_side_changed_is_taken() {
        let b = root(&[("A", Value::Int(1)), ("B", Value::Int(2)), ("C", Value::Int(3))]);
        let o = root(&[("A", Value::Int(10)), ("B", Value::Int(2)), ("C", Value::Int(3))]);
        let t = root(&[("A", Value::Int(1)), ("B", Value::Int(20)), ("C", Value::Int(3))]);
        let m = run(&b, &o, &t, Prefer::Ours);
        assert!(m.conflicts.is_empty());
        assert_eq!(
            m.gff.root,
            root(&[("A", Value::Int(10)), ("B", Value::Int(20)), ("C", Value::Int(3))])
        );
        // The same change on both sides is one.
        let m = run(&b, &o, &o, Prefer::Ours);
        assert!(m.conflicts.is_empty());
        assert_eq!(m.gff.root, o);
        // Nothing changed: the tree as it was.
        assert_eq!(run(&b, &b, &b, Prefer::Ours).gff.root, b);
    }

    #[test]
    fn a_field_both_changed_differently_is_a_conflict() {
        let b = root(&[("A", Value::Int(1))]);
        let o = root(&[("A", Value::Int(2))]);
        let t = root(&[("A", Value::Int(3))]);
        let m = run(&b, &o, &t, Prefer::Ours);
        assert_eq!(m.conflicts.len(), 1);
        let c = &m.conflicts[0];
        assert_eq!(c.path, "/A");
        assert_eq!(
            (c.base.clone(), c.ours.clone(), c.theirs.clone()),
            (Some(Value::Int(1)), Some(Value::Int(2)), Some(Value::Int(3)))
        );
        assert_eq!(m.gff.root.get("A"), Some(&Value::Int(2)));
        assert_eq!(run(&b, &o, &t, Prefer::Theirs).gff.root.get("A"), Some(&Value::Int(3)));
        // A type changed is a value changed.
        let t = root(&[("A", Value::Dword(1))]);
        assert_eq!(run(&b, &o, &t, Prefer::Ours).conflicts.len(), 1);
    }

    #[test]
    fn fields_added_and_removed_keep_ours_order() {
        let b = root(&[("A", Value::Int(1)), ("B", Value::Int(2)), ("C", Value::Int(3))]);
        // Ours put C first, took B away and added D; theirs added E.
        let o = root(&[("C", Value::Int(3)), ("A", Value::Int(1)), ("D", Value::Int(4))]);
        let t = root(&[
            ("A", Value::Int(1)),
            ("B", Value::Int(2)),
            ("C", Value::Int(3)),
            ("E", Value::Int(5)),
        ]);
        let m = run(&b, &o, &t, Prefer::Ours);
        assert!(m.conflicts.is_empty());
        assert_eq!(labels(&m.gff.root), ["C", "A", "D", "E"]);
        // One took a field away the other changed: a conflict.
        let t = root(&[("A", Value::Int(1)), ("B", Value::Int(22)), ("C", Value::Int(3))]);
        let m = run(&b, &o, &t, Prefer::Ours);
        assert_eq!(m.conflicts.len(), 1);
        assert_eq!((m.conflicts[0].path.as_str(), m.conflicts[0].ours.is_none()), ("/B", true));
        assert!(!m.gff.root.contains("B"), "ours took it away");
        assert_eq!(run(&b, &o, &t, Prefer::Theirs).gff.root.get("B"), Some(&Value::Int(22)));
    }

    #[test]
    fn duplicate_labels_and_unknown_fields_are_kept() {
        let mut b = root(&[("Odd", Value::Void(vec![1, 2]))]);
        b.fields.push(crate::Field { label: Label::new("Dup").unwrap(), value: Value::Int(1) });
        b.fields.push(crate::Field { label: Label::new("Dup").unwrap(), value: Value::Int(2) });
        let mut o = b.clone();
        o.fields[2].value = Value::Int(20); // the second Dup
        let mut t = b.clone();
        t.fields[1].value = Value::Int(10); // the first
        t.set("New", Value::Byte(1));
        let m = run(&b, &o, &t, Prefer::Ours);
        assert!(m.conflicts.is_empty(), "{:?}", m.conflicts);
        let values: Vec<&Value> = m.gff.root.fields.iter().map(|f| &f.value).collect();
        assert_eq!(
            values,
            [&Value::Void(vec![1, 2]), &Value::Int(10), &Value::Int(20), &Value::Byte(1)]
        );
        // A conflict on the second names it so.
        let mut t2 = b.clone();
        t2.fields[2].value = Value::Int(30);
        assert_eq!(run(&b, &o, &t2, Prefer::Ours).conflicts[0].path, "/Dup#2");
    }

    #[test]
    fn struct_ids_merge_as_values() {
        let mut b = root(&[("A", Value::Int(1))]);
        let s = |id| {
            let mut s = Struct::new(id);
            s.set("X", Value::Int(1));
            Value::Struct(s)
        };
        b.set("S", s(1));
        let mut o = b.clone();
        o.set("S", s(2));
        let mut t = b.clone();
        t.set("A", Value::Int(5));
        let m = run(&b, &o, &t, Prefer::Ours);
        assert!(m.conflicts.is_empty());
        assert_eq!(m.gff.root.get("S"), Some(&s(2)));
        let mut t = b.clone();
        t.set("S", s(3));
        assert_eq!(run(&b, &o, &t, Prefer::Ours).conflicts[0].path, "/S/(struct id)");
    }

    #[test]
    fn variables_merge_by_name() {
        let list = |vars: &[Struct]| root(&[("VarTable", Value::List(vars.to_vec()))]);
        let b = list(&[var("a", 1), var("b", 2), var("c", 3)]);
        // Ours: b changed, d added; theirs: a removed, e added, c changed.
        let o = list(&[var("a", 1), var("b", 20), var("c", 3), var("d", 4)]);
        let t = list(&[var("b", 2), var("c", 30), var("e", 5)]);
        let m = run(&b, &o, &t, Prefer::Ours);
        assert!(m.conflicts.is_empty(), "{:?}", m.conflicts);
        assert_eq!(m.gff.root, list(&[var("b", 20), var("c", 30), var("d", 4), var("e", 5)]));
        // Both set one variable differently; both added one under one name.
        let o = list(&[var("a", 1), var("b", 20), var("c", 3), var("x", 1)]);
        let t = list(&[var("a", 1), var("b", 21), var("c", 3), var("x", 2)]);
        let m = run(&b, &o, &t, Prefer::Theirs);
        let paths: Vec<&str> = m.conflicts.iter().map(|c| c.path.as_str()).collect();
        assert_eq!(paths, ["/VarTable[b, 1]/Value", "/VarTable[x, 1]"]);
        assert_eq!(m.gff.root, list(&[var("a", 1), var("b", 21), var("c", 3), var("x", 2)]));
        // An int and a string of one name are two variables, as the game
        // keeps them: one added by each side, both kept.
        let mut text = Struct::new(0);
        text.set("Name", Value::String(b"x".to_vec()));
        text.set("Type", Value::Dword(3));
        text.set("Value", Value::String(b"hello".to_vec()));
        let o = list(&[var("a", 1), var("b", 2), var("c", 3), var("x", 1)]);
        let t = list(&[var("a", 1), var("b", 2), var("c", 3), text.clone()]);
        let m = run(&b, &o, &t, Prefer::Ours);
        assert!(m.conflicts.is_empty(), "{:?}", m.conflicts);
        assert_eq!(m.gff.root, list(&[var("a", 1), var("b", 2), var("c", 3), var("x", 1), text]));
    }

    #[test]
    fn areas_are_a_union() {
        let area = |name: &str| {
            let mut s = Struct::new(6);
            s.set("Area_Name", Value::ResRef(name.into()));
            s
        };
        let list = |names: &[&str]| {
            root(&[("Mod_Area_list", Value::List(names.iter().map(|n| area(n)).collect()))])
        };
        let b = list(&["start", "town"]);
        let m = run(
            &b,
            &list(&["start", "town", "cave"]),
            &list(&["start", "town", "crypt", "keep"]),
            Prefer::Ours,
        );
        assert!(m.conflicts.is_empty());
        assert_eq!(m.gff.root, list(&["start", "town", "cave", "crypt", "keep"]));
        // Both added the same area: it is there once. One removed one.
        let m = run(&b, &list(&["start", "cave"]), &list(&["start", "town", "cave"]), Prefer::Ours);
        assert!(m.conflicts.is_empty());
        assert_eq!(m.gff.root, list(&["start", "cave"]));
        // A resref is the same whatever its case.
        let m = run(&b, &list(&["START", "town"]), &list(&["start", "town", "x"]), Prefer::Ours);
        assert_eq!(m.gff.root, list(&["START", "town", "x"]));
    }

    /// Where both changed a list, one side's reordering is kept and each
    /// side's new items stay after the item they follow there.
    #[test]
    fn a_list_s_order_merges_in_sequence() {
        let area = |name: &str| {
            let mut s = Struct::new(6);
            s.set("Area_Name", Value::ResRef(name.into()));
            s
        };
        let list = |names: &[&str]| {
            root(&[("Mod_Area_list", Value::List(names.iter().map(|n| area(n)).collect()))])
        };
        let b = list(&["start", "town", "cave"]);
        // Ours moved the cave up; theirs added a keep after the town.
        let m = run(
            &b,
            &list(&["cave", "start", "town"]),
            &list(&["start", "town", "keep", "cave"]),
            Prefer::Ours,
        );
        assert!(m.conflicts.is_empty());
        assert_eq!(m.gff.root, list(&["cave", "start", "town", "keep"]));
        // Theirs put a new area first and moved the cave up; ours added
        // one after the cave, which it follows wherever the cave is.
        let m = run(
            &b,
            &list(&["start", "town", "cave", "crypt"]),
            &list(&["intro", "cave", "start", "town"]),
            Prefer::Ours,
        );
        assert_eq!(m.gff.root, list(&["intro", "cave", "crypt", "start", "town"]));
        // Both reordered, each its own way: the base's order.
        let m = run(
            &b,
            &list(&["cave", "start", "town"]),
            &list(&["town", "start", "cave", "x"]),
            Prefer::Ours,
        );
        assert_eq!(m.gff.root, list(&["start", "town", "cave", "x"]));
        // New items both put after one: ours first.
        let m = run(
            &b,
            &list(&["start", "a", "town", "cave"]),
            &list(&["start", "z", "town", "cave"]),
            Prefer::Ours,
        );
        assert_eq!(m.gff.root, list(&["start", "a", "z", "town", "cave"]));
    }

    #[test]
    fn placed_objects_merge_by_object() {
        let list = |objs: &[Struct]| root(&[("Placeable List", Value::List(objs.to_vec()))]);
        let (chest, table, barrel) = (
            placed("plc_chest", "CHEST", 1.0, 2.0),
            placed("plc_table", "TABLE", 5.0, 5.0),
            placed("plc_barrel", "BARREL", 9.0, 1.0),
        );
        let b = list(&[chest.clone(), table.clone(), barrel.clone()]);
        // Ours: the chest moved, a lamp added. Theirs: the table removed,
        // the barrel retagged, a rug added.
        let mut moved = chest.clone();
        moved.set("X", Value::Float(1.5));
        let lamp = placed("plc_lamp", "LAMP", 3.0, 3.0);
        let o = list(&[moved.clone(), table.clone(), barrel.clone(), lamp.clone()]);
        let mut retagged = barrel.clone();
        retagged.set("Tag", Value::String(b"KEG".to_vec()));
        let rug = placed("plc_rug", "RUG", 4.0, 4.0);
        let t = list(&[chest.clone(), retagged.clone(), rug.clone()]);
        let m = run(&b, &o, &t, Prefer::Ours);
        assert!(m.conflicts.is_empty(), "{:?}", m.conflicts);
        assert_eq!(m.gff.root, list(&[moved.clone(), retagged.clone(), lamp.clone(), rug.clone()]));
        // Both edited the chest: field by field, even with ours moving it.
        let mut locked = chest.clone();
        locked.set("Locked", Value::Byte(1));
        let t = list(&[locked, table.clone(), barrel.clone()]);
        let m = run(&b, &o, &t, Prefer::Ours);
        let mut both = moved.clone();
        both.set("Locked", Value::Byte(1));
        assert_eq!(m.gff.root, list(&[both, table.clone(), barrel.clone(), lamp.clone()]));
        // Two objects both added, though alike, are both kept.
        let lamp2 = placed("plc_lamp", "LAMP", 7.0, 7.0);
        let m = run(
            &b,
            &list(&[chest.clone(), table.clone(), barrel.clone(), lamp.clone()]),
            &list(&[chest.clone(), table.clone(), barrel.clone(), lamp2.clone()]),
            Prefer::Ours,
        );
        assert!(m.conflicts.is_empty());
        assert_eq!(m.gff.root.list("Placeable List").unwrap().len(), 5);
        // One removed what the other changed: a conflict, named by the object.
        let t = list(&[chest.clone(), barrel.clone()]);
        let mut wider = table.clone();
        wider.set("Appearance", Value::Dword(7));
        let o = list(&[chest.clone(), wider, barrel.clone()]);
        let m = run(&b, &o, &t, Prefer::Theirs);
        assert_eq!(m.conflicts.len(), 1);
        assert_eq!(m.conflicts[0].path, "/Placeable List[plc_table \"TABLE\" at 5, 5]");
        assert_eq!(m.gff.root, list(&[chest.clone(), barrel.clone()]));
    }

    #[test]
    fn placed_objects_with_an_object_id_match_by_it() {
        let with_id = |s: &Struct, id: u32| {
            let mut s = s.clone();
            s.set("ObjectId", Value::Dword(id));
            s
        };
        let list = |objs: &[Struct]| root(&[("Placeable List", Value::List(objs.to_vec()))]);
        let (a, c) = (
            with_id(&placed("plc_a", "A", 1.0, 1.0), 1),
            with_id(&placed("plc_a", "A", 2.0, 2.0), 2),
        );
        let b = list(&[a.clone(), c.clone()]);
        // Both moved and retagged by ours: still the same objects.
        let mut a2 = a.clone();
        a2.set("Tag", Value::String(b"B".to_vec()));
        a2.set("X", Value::Float(8.0));
        let mut c2 = c.clone();
        c2.set("Locked", Value::Byte(1));
        let m =
            run(&b, &list(&[a2.clone(), c.clone()]), &list(&[a.clone(), c2.clone()]), Prefer::Ours);
        assert!(m.conflicts.is_empty());
        assert_eq!(m.gff.root, list(&[a2, c2]));
    }

    #[test]
    fn positional_lists_merge_by_place() {
        let item = |n: i32| {
            let mut s = Struct::new(0);
            s.set("Rank", Value::Int(n));
            s
        };
        let list =
            |ns: &[i32]| root(&[("SkillList", Value::List(ns.iter().map(|n| item(*n)).collect()))]);
        let b = list(&[0, 0, 0]);
        let m = run(&b, &list(&[1, 0, 0]), &list(&[0, 0, 2]), Prefer::Ours);
        assert!(m.conflicts.is_empty());
        assert_eq!(m.gff.root, list(&[1, 0, 2]));
        // Two items added at one place: a conflict.
        let m = run(&b, &list(&[0, 0, 0, 4]), &list(&[0, 0, 0, 5]), Prefer::Ours);
        assert_eq!(m.conflicts.len(), 1);
        assert_eq!(m.conflicts[0].path, "/SkillList[3]");
        assert_eq!(m.gff.root, list(&[0, 0, 0, 4]));
        // A list one side alone changed is that side's, order and all.
        let m = run(&b, &list(&[0, 0, 0]), &list(&[3, 2, 1, 0]), Prefer::Ours);
        assert_eq!(m.gff.root, list(&[3, 2, 1, 0]));
    }

    #[test]
    fn struct_ids_tell_slots_apart() {
        let slot = |id: u32, r: &str| {
            let mut s = Struct::new(id);
            s.set("EquippedRes", Value::ResRef(r.into()));
            s
        };
        let list = |s: &[Struct]| root(&[("Equip_ItemList", Value::List(s.to_vec()))]);
        let b = list(&[slot(2, "helm"), slot(16, "sword")]);
        let m = run(
            &b,
            &list(&[slot(16, "sword")]),
            &list(&[slot(2, "helm"), slot(16, "axe"), slot(32, "shield")]),
            Prefer::Ours,
        );
        assert!(m.conflicts.is_empty());
        assert_eq!(m.gff.root, list(&[slot(16, "axe"), slot(32, "shield")]));
    }

    /// One side's English and the other's French of one name are both
    /// kept; both changing the English is a conflict of the English alone.
    #[test]
    fn localized_strings_merge_by_language() {
        let name = |pairs: &[(u32, &str)]| {
            let mut ls = LocString::from_strref(mg_core::StrRef::NONE);
            for (k, t) in pairs {
                ls.strings.push((LocStringKey(*k), t.as_bytes().to_vec()));
            }
            Value::LocString(ls)
        };
        let b = root(&[("Name", name(&[(0, "Chest"), (2, "Coffre")]))]);
        let o = root(&[("Name", name(&[(0, "Old Chest"), (2, "Coffre")]))]);
        let t = root(&[("Name", name(&[(0, "Chest"), (2, "Vieux coffre"), (4, "Truhe")]))]);
        let m = run(&b, &o, &t, Prefer::Ours);
        assert!(m.conflicts.is_empty(), "{:?}", m.conflicts);
        assert_eq!(
            m.gff.root,
            root(&[("Name", name(&[(0, "Old Chest"), (2, "Vieux coffre"), (4, "Truhe")]))])
        );
        let t = root(&[("Name", name(&[(0, "Big Chest"), (2, "Coffre")]))]);
        let m = run(&b, &o, &t, Prefer::Theirs);
        assert_eq!(m.conflicts.len(), 1);
        assert_eq!(m.conflicts[0].path, "/Name/[0]");
        assert_eq!(m.gff.root, root(&[("Name", name(&[(0, "Big Chest"), (2, "Coffre")]))]));
        // A language one took away and the other left: gone.
        let o = root(&[("Name", name(&[(0, "Chest")]))]);
        let t = root(&[("Name", name(&[(0, "Chest"), (2, "Coffre")])), ("X", Value::Int(1))]);
        let m = run(&b, &o, &t, Prefer::Ours);
        assert!(m.conflicts.is_empty());
        assert_eq!(m.gff.root.get("Name"), Some(&name(&[(0, "Chest")])));
    }

    /// An object one side both moved and retagged is still that object
    /// where it is the only one of its blueprint left: the other side's
    /// change to it merges in, rather than a conflict.
    #[test]
    fn an_object_moved_and_retagged_is_still_itself() {
        let list = |items: Vec<Struct>| root(&[("Placeable List", Value::List(items))]);
        let chest = placed("plc_chest", "CHEST", 1.0, 1.0);
        let lamp = placed("plc_lamp", "LAMP", 5.0, 5.0);
        let mut moved = placed("plc_chest", "RED_CHEST", 3.0, 2.0);
        moved.set("Appearance", Value::Dword(1));
        let mut lit = chest.clone();
        lit.set("Appearance", Value::Dword(7));
        let b = list(vec![chest.clone(), lamp.clone()]);
        let o = list(vec![moved.clone(), lamp.clone()]);
        let t = list(vec![lit, lamp.clone()]);
        let m = run(&b, &o, &t, Prefer::Ours);
        assert!(m.conflicts.is_empty(), "{:?}", m.conflicts);
        let mut want = moved.clone();
        want.set("Appearance", Value::Dword(7));
        assert_eq!(m.gff.root, list(vec![want, lamp.clone()]));
        // Two of the blueprint left: no telling which, so removed and added.
        let other = placed("plc_chest", "CHEST2", 9.0, 9.0);
        let b = list(vec![chest.clone(), other.clone()]);
        let o = list(vec![moved.clone(), placed("plc_chest", "BLUE", 8.0, 8.0)]);
        let m = run(&b, &o, &b, Prefer::Ours);
        assert_eq!(m.gff.root, o);
    }

    #[test]
    fn a_list_both_added_merges_from_nothing() {
        let o = root(&[("VarTable", Value::List(vec![var("a", 1)]))]);
        let t = root(&[("VarTable", Value::List(vec![var("b", 2)]))]);
        let m = run(&root(&[]), &o, &t, Prefer::Ours);
        assert!(m.conflicts.is_empty());
        assert_eq!(m.gff.root, root(&[("VarTable", Value::List(vec![var("a", 1), var("b", 2)]))]));
    }
}
