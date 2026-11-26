//! The edits that make one GFF of another: what `mg merge-gff` writes for
//! `mg apply` where two sides of a merge conflict ([`gff_edits`]).
//!
//! A field changed is set, one gone is removed, one new is added (after
//! the struct's others: a field's place in its struct is not kept, which
//! no reader of a GFF minds). A list keeps the items both have in common
//! at their start, their end and in between (the longest run of items
//! alike in order), and the items between those are changed in place
//! where there are as many before as after, else removed and inserted.
//! A list's edits go from its end to its start, so each names its item by
//! the place it has when it is made. An item whose struct id changed, or a
//! struct with two fields of one label, is replaced whole.

use mg_gff::{Gff, GffPath, Step, Struct, Value, WriteError};
use mg_resman::ResKey;

use crate::Edit;

/// Above this many pairs of items, a list's changed items are not matched
/// up with each other (the items in common at its ends still are).
const MATCHED_PAIRS: usize = 1 << 20;

/// The edits that make `before` into `after` (resource `key`): none where
/// they are the same; the whole resource set where their file type,
/// version or root struct id differ.
pub fn gff_edits(key: ResKey, before: &Gff, after: &Gff) -> Result<Vec<Edit>, WriteError> {
    if before == after {
        return Ok(Vec::new());
    }
    let whole = || Ok(vec![Edit::SetResource { key, data: Some(after.to_bytes()?) }]);
    if (before.file_type, before.version) != (after.file_type, after.version) {
        return whole();
    }
    let mut out = Vec::new();
    match struct_edits(key, &GffPath::root(), &before.root, &after.root, &mut out) {
        Ok(()) => Ok(out),
        Err(Replace) => whole(),
    }
}

/// Where an edit is: the field it sets, or the list's item it inserts or
/// removes (the root for a whole resource).
pub fn place(edit: &Edit) -> GffPath {
    match edit {
        Edit::SetField { path, label, .. } => path.field(label),
        Edit::InsertItem { path, list, index, .. } | Edit::RemoveItem { path, list, index, .. } => {
            path.item(list, *index)
        }
        Edit::SetResource { .. } => GffPath::root(),
    }
}

/// A path's steps as values to order paths by: by label, a field before
/// a list's items, the items by place (and a path before those under it).
pub fn place_order(path: &GffPath) -> Vec<(String, u8, usize)> {
    (path.0.iter())
        .map(|s| match s {
            Step::Field(l) => (l.clone(), 0, 0),
            Step::Item(l, i) => (l.clone(), 1, *i),
        })
        .collect()
}

/// A struct that can't be edited into the other in place: replaced whole.
struct Replace;

fn has_duplicate_labels(s: &Struct) -> bool {
    let mut seen = std::collections::HashSet::new();
    !s.fields.iter().all(|f| seen.insert(f.label))
}

fn struct_edits(
    key: ResKey,
    path: &GffPath,
    a: &Struct,
    b: &Struct,
    out: &mut Vec<Edit>,
) -> Result<(), Replace> {
    if a == b {
        return Ok(());
    }
    if a.id != b.id || has_duplicate_labels(a) || has_duplicate_labels(b) {
        return Err(Replace);
    }
    let set = |label: &str, value: Option<Value>| Edit::SetField {
        key,
        path: path.clone(),
        label: label.to_string(),
        value,
    };
    for fa in &a.fields {
        let label = fa.label.to_string();
        match b.get(&label) {
            None => out.push(set(&label, None)),
            Some(vb) if *vb == fa.value => {}
            Some(vb) => match (&fa.value, vb) {
                (Value::List(la), Value::List(lb)) => list_edits(key, path, &label, la, lb, out),
                (Value::Struct(sa), Value::Struct(sb)) => {
                    let mut inner = Vec::new();
                    match struct_edits(key, &path.field(&label), sa, sb, &mut inner) {
                        Ok(()) => out.append(&mut inner),
                        Err(Replace) => out.push(set(&label, Some(vb.clone()))),
                    }
                }
                _ => out.push(set(&label, Some(vb.clone()))),
            },
        }
    }
    for fb in &b.fields {
        let label = fb.label.to_string();
        if a.get(&label).is_none() {
            out.push(set(&label, Some(fb.value.clone())));
        }
    }
    Ok(())
}

/// For two runs of items, the pairs of places of the longest run of items
/// alike in both, in order (none where there are too many to pair).
fn alike_in_order(a: &[Struct], b: &[Struct]) -> Vec<(usize, usize)> {
    let (n, m) = (a.len(), b.len());
    if n == 0 || m == 0 || n.saturating_mul(m) > MATCHED_PAIRS {
        return Vec::new();
    }
    // (The longest run from each pair of places on.)
    let mut run = vec![0u32; (n + 1) * (m + 1)];
    let at = |i: usize, j: usize| i * (m + 1) + j;
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            run[at(i, j)] = if a[i] == b[j] {
                run[at(i + 1, j + 1)] + 1
            } else {
                run[at(i + 1, j)].max(run[at(i, j + 1)])
            };
        }
    }
    let (mut i, mut j, mut out) = (0, 0, Vec::new());
    while i < n && j < m {
        if a[i] == b[j] {
            out.push((i, j));
            (i, j) = (i + 1, j + 1);
        } else if run[at(i + 1, j)] >= run[at(i, j + 1)] {
            i += 1;
        } else {
            j += 1;
        }
    }
    out
}

fn list_edits(
    key: ResKey,
    path: &GffPath,
    label: &str,
    a: &[Struct],
    b: &[Struct],
    out: &mut Vec<Edit>,
) {
    let start = a.iter().zip(b).take_while(|(x, y)| x == y).count();
    let end =
        (a[start..].iter().rev()).zip(b[start..].iter().rev()).take_while(|(x, y)| x == y).count();
    let (am, bm) = (&a[start..a.len() - end], &b[start..b.len() - end]);
    // The stretches between the items alike, in order: (before, after).
    let mut gaps = Vec::new();
    let (mut i, mut j) = (0, 0);
    for (ai, bj) in alike_in_order(am, bm) {
        gaps.push((i..ai, j..bj));
        (i, j) = (ai + 1, bj + 1);
    }
    gaps.push((i..am.len(), j..bm.len()));
    for (before, after) in gaps.into_iter().rev() {
        let at = start + before.start;
        if before.len() == after.len() {
            for (k, (x, y)) in am[before].iter().zip(&bm[after]).enumerate() {
                let mut inner = Vec::new();
                match struct_edits(key, &path.item(label, at + k), x, y, &mut inner) {
                    Ok(()) => out.append(&mut inner),
                    Err(Replace) => {
                        out.push(Edit::remove_item(key, path.clone(), label, at + k));
                        out.push(Edit::insert_item(key, path.clone(), label, at + k, y.clone()));
                    }
                }
            }
        } else {
            for k in (0..before.len()).rev() {
                out.push(Edit::remove_item(key, path.clone(), label, at + k));
            }
            for (k, y) in bm[after].iter().enumerate() {
                out.push(Edit::insert_item(key, path.clone(), label, at + k, y.clone()));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use mg_core::{ResRef, ResType};
    use mg_module::Module;

    use super::*;
    use crate::{Command, Workspace};

    fn key() -> ResKey {
        ResKey::new(ResRef::from_str("area").unwrap(), ResType::GIT)
    }

    /// `edits` made on `before`, as `mg apply` makes them.
    fn applied(before: &Gff, edits: Vec<Edit>) -> Gff {
        let mut m = Module::new();
        m.set_gff(key(), before).unwrap();
        let mut ws = Workspace::new(m);
        ws.apply(Command::new("t", edits)).unwrap();
        ws.doc(&key()).unwrap().clone()
    }

    fn item(name: &str, n: i32) -> Struct {
        let mut s = Struct::new(1);
        s.set("Tag", Value::String(name.into()));
        s.set("N", Value::Int(n));
        s
    }

    fn git(fields: &[(&str, Value)]) -> Gff {
        let mut g = Gff::new(*b"GIT ");
        for (l, v) in fields {
            g.root.set(l, v.clone());
        }
        g
    }

    fn list(items: &[Struct]) -> Value {
        Value::List(items.to_vec())
    }

    /// Each pair: the edits made on the first give the second.
    #[test]
    fn the_edits_make_one_gff_of_the_other() {
        let (a, b, c, d) = (item("a", 1), item("b", 2), item("c", 3), item("d", 4));
        let mut b2 = b.clone();
        b2.set("N", Value::Int(20));
        b2.set("New", Value::Byte(1));
        let mut c_id = c.clone();
        c_id.id = 7;
        let mut inner = Struct::new(3);
        inner.set("X", Value::Int(1));
        let mut inner2 = inner.clone();
        inner2.set("X", Value::Int(2));
        let base = git(&[
            ("Gone", Value::Int(1)),
            ("Kept", Value::Int(1)),
            ("Inner", Value::Struct(inner.clone())),
            ("List", list(&[a.clone(), b.clone(), c.clone(), d.clone()])),
        ]);
        let cases = [
            // Fields set, removed and added; a struct changed inside.
            git(&[
                ("Kept", Value::Int(2)),
                ("Inner", Value::Struct(inner2.clone())),
                ("List", list(&[a.clone(), b.clone(), c.clone(), d.clone()])),
                ("Added", Value::Int(5)),
            ]),
            // An item changed in place, one with a new struct id, items
            // inserted at both ends and in the middle, one removed.
            git(&[
                ("Gone", Value::Int(1)),
                ("Kept", Value::Int(1)),
                ("Inner", Value::Struct(inner.clone())),
                ("List", list(&[d.clone(), a.clone(), b2.clone(), c_id, item("e", 5)])),
            ]),
            git(&[
                ("Gone", Value::Int(1)),
                ("Kept", Value::Int(1)),
                ("Inner", Value::Struct(inner.clone())),
                ("List", list(&[item("x", 0), a.clone(), item("y", 0), b.clone(), d.clone()])),
            ]),
            // The list emptied; another made.
            git(&[
                ("Gone", Value::Int(1)),
                ("Kept", Value::Int(1)),
                ("Inner", Value::Struct(inner.clone())),
                ("List", list(&[])),
                ("Other", list(std::slice::from_ref(&a))),
            ]),
        ];
        for after in cases {
            let edits = gff_edits(key(), &base, &after).unwrap();
            assert!(!edits.iter().any(|e| matches!(e, Edit::SetResource { .. })), "{edits:?}");
            assert_eq!(applied(&base, edits), after);
        }
        assert!(gff_edits(key(), &base, &base).unwrap().is_empty());
        // Each edit at its place, an item changed in place by its field.
        let mut moved = base.clone();
        moved.root.list_mut("List").unwrap()[2].set("N", Value::Int(9));
        let edits = gff_edits(key(), &base, &moved).unwrap();
        assert_eq!(edits.len(), 1);
        assert_eq!(place(&edits[0]).to_string(), "/List[2]/N");
        // Another file type: the whole resource.
        let mut other = base.clone();
        other.file_type = *b"ARE ";
        let edits = gff_edits(key(), &base, &other).unwrap();
        assert!(matches!(edits[..], [Edit::SetResource { .. }]));
    }

    #[test]
    fn places_order_by_list_place() {
        let p = |s: &str| place_order(&s.parse::<GffPath>().unwrap());
        assert!(p("/L[10]") > p("/L[9]"));
        assert!(p("/L[3]/X") > p("/L[3]"));
        assert!(p("/L[3]") > p("/L[2]/X"));
    }
}
