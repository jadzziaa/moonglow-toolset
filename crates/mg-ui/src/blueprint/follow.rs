//! A placed object's Properties following the object as the lists it is
//! in change: a held item's window is found by the item's place in its
//! inventory (an item's struct has no name of its own: its struct id is
//! its place), so an item added, removed or moved before it would leave
//! the window on another item. Worked out from the edits a command, an
//! Undo or a Redo is about to run, and applied once they have.

use std::collections::HashMap;

use mg_core::{ResRef, ResType};
use mg_edit::{Edit, GffPath, Step};
use mg_gff::{Struct, Value};
use mg_resman::ResKey;

use crate::{Moonglow, Tab};

/// The fields an item's move changes (beside its struct id): where it is
/// in an inventory's grid, and a store's Infinite (which a store gives the
/// items put in it). An item moved is the same item without them.
const MOVED_FIELDS: [&str; 3] = ["Repos_PosX", "Repos_Posy", "Infinite"];

/// Whether `a` and `b` are the same item in two places: alike in every
/// field but those a move changes ([`MOVED_FIELDS`], the struct id, a
/// blueprint entry's `EquippedRes` for `InventoryRes`), as an item taken
/// from the backpack is equipped, put in the backpack, or carried to
/// another object's inventory.
pub(crate) fn same_item(a: &Struct, b: &Struct) -> bool {
    fn fields(s: &Struct) -> impl Iterator<Item = (&[u8], &Value)> {
        s.fields
            .iter()
            .filter(|f| !MOVED_FIELDS.iter().any(|l| l.as_bytes() == f.label.as_bytes()))
            .map(|f| {
                let label = f.label.as_bytes();
                (if label == b"EquippedRes" { b"InventoryRes".as_slice() } else { label }, &f.value)
            })
    }
    fields(a).eq(fields(b))
}

/// Where `was` (an item taken out) is in `s`, put in: `s` itself, or an
/// item of a list in it (a store's page put in with the item in it). The
/// steps from `s` to it.
fn find(s: &Struct, was: &Struct) -> Option<Vec<Step>> {
    if same_item(s, was) {
        return Some(Vec::new());
    }
    for f in &s.fields {
        let Value::List(items) = &f.value else { continue };
        for (i, item) in items.iter().enumerate() {
            if let Some(rest) = find(item, was) {
                let mut steps = vec![Step::Item(f.label.to_string_lossy(), i)];
                steps.extend(rest);
                return Some(steps);
            }
        }
    }
    None
}

/// A window's object as the edits run through.
#[derive(Debug, Clone)]
enum Track {
    At(ResKey, GffPath),
    /// Taken out of its place: the struct taken (the window's, or what
    /// holds it), the steps from it on to the window's, and the window's
    /// place if its list was set whole and it is not found elsewhere.
    Out {
        was: Struct,
        rest: Vec<Step>,
        kept: Option<(ResKey, GffPath)>,
    },
}

/// Runs `e` on the copies of the documents in `docs` (an edit that does
/// not apply changes nothing: the command would fail as a whole).
fn run(docs: &mut HashMap<ResKey, Struct>, e: &Edit) {
    let Some(root) = docs.get_mut(e.key()) else { return };
    match e {
        Edit::SetField { path, label, value, .. } => {
            if let Some(s) = path.get_mut(root) {
                match value {
                    Some(v) if mg_gff::Label::new(label).is_ok() => s.set(label, v.clone()),
                    Some(_) => {}
                    None => {
                        s.remove(label);
                    }
                }
            }
        }
        Edit::InsertItem { path, list, index, item, .. } => {
            if let Some(s) = path.get_mut(root) {
                if s.get(list).is_none() {
                    s.set(list, Value::List(Vec::new()));
                }
                if let Some(items) = s.list_mut(list).filter(|l| *index <= l.len()) {
                    items.insert(*index, item.clone());
                }
            }
        }
        Edit::RemoveItem { path, list, index, .. } => {
            if let Some(items) = path.get_mut(root).and_then(|s| s.list_mut(list))
                && *index < items.len()
            {
                items.remove(*index);
            }
        }
        Edit::SetResource { .. } => {}
    }
}

/// The step of `path` into list `list` of the struct at `at`, if `path`
/// goes through it: the item's index there, to change.
fn through<'a>(path: &'a mut GffPath, at: &GffPath, list: &str) -> Option<&'a mut usize> {
    if !path.0.starts_with(&at.0) {
        return None;
    }
    match path.0.get_mut(at.0.len()) {
        Some(Step::Item(l, i)) if l == list => Some(i),
        _ => None,
    }
}

/// Where the item at `i` of `old` is in `new`, the list set in its place
/// whole (a window's Cancel setting back what it changed): the same item
/// at the same place, else the same struct elsewhere (the first as many
/// of its like came before it), else the same item moved (an item
/// equipped set back into the backpack).
fn found_in(old: &[Struct], i: usize, new: &[Struct]) -> Option<usize> {
    let was = old.get(i)?;
    if new.get(i) == Some(was) {
        return Some(i);
    }
    let before = old[..i].iter().filter(|s| *s == was).count();
    let equal: Vec<usize> = (0..new.len()).filter(|&j| new[j] == *was).collect();
    equal
        .get(before)
        .or(equal.first())
        .copied()
        .or_else(|| new.iter().position(|s| same_item(s, was)))
}

/// One edit's move of a window's object.
fn step(t: &mut Track, e: &Edit, docs: &HashMap<ResKey, Struct>) {
    let next = match (e, &mut *t) {
        (Edit::InsertItem { key, path: at, list, index, .. }, Track::At(k, p)) if k == key => {
            if let Some(i) = through(p, at, list)
                && *index <= *i
            {
                *i += 1;
            }
            None
        }
        // Taken out and put in again (a move, in its list, to another, or
        // into another area's): there.
        (Edit::InsertItem { key, path: at, list, index, item }, Track::Out { was, rest, .. })
            if key.restype == ResType::GIT =>
        {
            find(item, was).map(|within| {
                let mut p = at.item(list, *index);
                p.0.extend(within);
                p.0.extend(rest.iter().cloned());
                Track::At(*key, p)
            })
        }
        (Edit::RemoveItem { key, path: at, list, index }, Track::At(k, p)) if k == key => {
            let d = at.0.len();
            let whole = p.clone();
            match through(p, at, list) {
                Some(i) if *index < *i => {
                    *i -= 1;
                    None
                }
                Some(i) if *index == *i => {
                    let taken = docs.get(key).and_then(|root| at.item(list, *index).get(root));
                    let rest = whole.0[d + 1..].to_vec();
                    taken.map(|was| Track::Out { was: was.clone(), rest, kept: None })
                }
                _ => None,
            }
        }
        // Its list set whole: where it is in the new one; if not there,
        // taken out (to be found in another list set whole).
        (Edit::SetField { key, path: at, label, value }, Track::At(k, p)) if k == key => {
            let d = at.0.len();
            let whole = p.clone();
            let old = docs.get(key).and_then(|root| at.get(root)?.list(label));
            let new = match value {
                Some(Value::List(new)) => &new[..],
                _ => &[],
            };
            match (old, through(p, at, label)) {
                (Some(old), Some(i)) => match found_in(old, *i, new) {
                    Some(j) => {
                        *i = j;
                        None
                    }
                    None => old.get(*i).map(|was| Track::Out {
                        was: was.clone(),
                        rest: whole.0[d + 1..].to_vec(),
                        kept: Some((*key, whole.clone())),
                    }),
                },
                _ => None,
            }
        }
        _ => None,
    };
    if let Some(next) = next {
        *t = next;
    }
}

/// Where each of `places` (a struct in one of the documents `docs`: copies
/// of their roots before the edits, run on as they go) is after `edits`
/// run: moved by the items
/// inserted or removed before it in the lists along its path, and by a
/// move (taken out and the same item put in again, in its list, another
/// or another document's: Move item, Equip, To Backpack, a drag between
/// objects; set back by a list set whole). `None` if it was taken out and
/// not put back. A whole resource set can't be followed: the place is
/// kept.
pub(crate) fn follow(
    mut docs: HashMap<ResKey, Struct>,
    places: &[(ResKey, GffPath)],
    edits: &[Edit],
) -> Vec<Option<(ResKey, GffPath)>> {
    let mut tracks: Vec<Track> = places.iter().map(|(k, p)| Track::At(*k, p.clone())).collect();
    // The lists set whole, where an item taken out may be found.
    let mut set: Vec<(ResKey, GffPath, String)> = Vec::new();
    for e in edits {
        for t in &mut tracks {
            step(t, e, &docs);
        }
        if let Edit::SetField { key, path, label, value: Some(Value::List(_)) } = e
            && key.restype == ResType::GIT
        {
            set.push((*key, path.clone(), label.clone()));
        }
        run(&mut docs, e);
    }
    tracks
        .into_iter()
        .map(|t| match t {
            Track::At(k, p) => Some((k, p)),
            Track::Out { was, rest, kept } => {
                // In a list set whole: the same struct first, else the same
                // item moved.
                let lists = || {
                    set.iter().filter_map(|(k, at, label)| {
                        Some((*k, at, label, docs.get(k).and_then(|r| at.get(r)?.list(label))?))
                    })
                };
                let like = |exact: bool| {
                    lists().find_map(|(k, at, label, items)| {
                        let j = items
                            .iter()
                            .position(|s| if exact { *s == was } else { same_item(s, &was) })?;
                        let mut p = at.item(label, j);
                        p.0.extend(rest.iter().cloned());
                        Some((k, p))
                    })
                };
                // (Else, where its list was set whole, an item of its
                // blueprint: it, a field it had changed set back.)
                let template = |s: &Struct| s.resref("TemplateResRef");
                let kept = kept.filter(|(k, p)| {
                    let at = GffPath(p.0[..p.0.len() - rest.len()].to_vec());
                    docs.get(k)
                        .and_then(|r| at.get(r))
                        .is_some_and(|s| template(s).is_some() && template(s) == template(&was))
                });
                like(true).or_else(|| like(false)).or(kept)
            }
        })
        .collect()
}

/// Where a placed object (or an item one holds) is: the area, and its
/// path in the area's GIT.
pub(crate) type Place = (ResRef, GffPath);

/// The places of the open Properties windows of placed objects (and of
/// the items they hold) in the areas `edits` insert into, remove from or
/// set lists in, where each is after the edits (`None`: gone). Worked out
/// before they run; [`retarget`] once they have.
pub(crate) type Followed = HashMap<Place, Option<Place>>;

/// Whether `e` may move what is in a list: an item inserted or removed,
/// or a field set whole (a list set back) or removed.
fn moves_items(e: &Edit) -> bool {
    match e {
        Edit::InsertItem { .. } | Edit::RemoveItem { .. } => true,
        Edit::SetField { value, .. } => matches!(value, None | Some(Value::List(_))),
        Edit::SetResource { .. } => false,
    }
}

/// [`Followed`] for `edits`, those that moved.
pub(crate) fn followed(app: &mut Moonglow, edits: &[Edit]) -> Followed {
    let mut out = Followed::new();
    let mut areas: Vec<ResRef> = Vec::new();
    for e in edits.iter().filter(|e| moves_items(e)) {
        let k = e.key();
        if k.restype == ResType::GIT && !areas.contains(&k.resref) {
            areas.push(k.resref);
        }
    }
    if areas.is_empty() {
        return out;
    }
    let mut open: Vec<(ResRef, GffPath)> = Vec::new();
    for (_, tab) in app.dock.iter_all_tabs() {
        match tab {
            Tab::Instance { area, path } | Tab::InstanceModel { area, path } => {
                open.push((*area, path.clone()))
            }
            Tab::Instances { area, paths } => open.extend(paths.iter().map(|p| (*area, p.clone()))),
            _ => {}
        }
    }
    open.retain(|(area, _)| areas.contains(area));
    open.dedup();
    if open.is_empty() {
        return out;
    }
    let Some(ws) = app.ws.as_mut() else { return out };
    let mut docs = HashMap::new();
    for area in &areas {
        let key = ResKey::new(*area, ResType::GIT);
        if let Ok(doc) = ws.doc(&key) {
            docs.insert(key, doc.root.clone());
        }
    }
    let places: Vec<(ResKey, GffPath)> =
        open.iter().map(|(area, p)| (ResKey::new(*area, ResType::GIT), p.clone())).collect();
    for ((area, path), now) in open.into_iter().zip(follow(docs, &places, edits)) {
        let now = now.map(|(k, p)| (k.resref, p));
        if now.as_ref() != Some(&(area, path.clone())) {
            out.insert((area, path), now);
        }
    }
    out
}

/// The edits of `edits` that [`followed`] reads (an Undo's or a Redo's,
/// taken from the history without the rest).
pub(crate) fn list_edits(edits: &[Edit]) -> Vec<Edit> {
    edits.iter().filter(|e| moves_items(e)).cloned().collect()
}

/// The id of the Properties window of the object at `path` in `area`'s
/// GIT ([`Moonglow::window_ids`]): given on first asking, kept as the
/// window follows its object.
pub(crate) fn window_id(app: &mut Moonglow, area: ResRef, path: &GffPath) -> egui::Id {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    *app.window_ids.entry((area, path.clone())).or_insert_with(|| {
        egui::Id::new(("instance-window", NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)))
    })
}

/// `tab` pointed at where its objects are now; false if none is left.
fn moved_tab(tab: &mut Tab, moved: &Followed) -> bool {
    let now = |area: ResRef, path: &GffPath| match moved.get(&(area, path.clone())) {
        Some(now) => now.clone(),
        None => Some((area, path.clone())),
    };
    match tab {
        Tab::Instance { area, path } | Tab::InstanceModel { area, path } => {
            match now(*area, path) {
                Some((a, p)) => {
                    (*area, *path) = (a, p);
                    true
                }
                None => false,
            }
        }
        // (Several objects of one area edited together: those still in it.)
        Tab::Instances { area, paths } => {
            let here = *area;
            *paths = paths
                .iter()
                .filter_map(|p| now(here, p).filter(|(a, _)| *a == here).map(|(_, p)| p))
                .collect();
            !paths.is_empty()
        }
        _ => true,
    }
}

/// Points the Properties windows at where their objects went (with the
/// page each shows, its widgets' state and what each changed, for its
/// Cancel), and closes those whose objects are gone, as the area view does
/// those of the objects it deletes.
pub(crate) fn retarget(app: &mut Moonglow, moved: Followed) {
    if moved.is_empty() {
        return;
    }
    let mut closed = Vec::new();
    for (_, tab) in app.dock.iter_all_tabs_mut() {
        let was = tab.clone();
        if !moved_tab(tab, &moved) {
            closed.push(was);
        }
    }
    crate::tabs::close_tabs(&mut app.dock, |t| !closed.contains(t));
    // (Where an object of a window moved: `Some(None)` gone.)
    let now = |key: &ResKey, path: &GffPath| -> Option<Option<(ResKey, GffPath)>> {
        if key.restype != ResType::GIT {
            return None;
        }
        let to = moved.get(&(key.resref, path.clone()))?;
        Some(to.clone().map(|(area, p)| (ResKey::new(area, ResType::GIT), p)))
    };
    let ids: Vec<_> = app.window_ids.keys().cloned().collect();
    let mut taken = Vec::new();
    for (area, path) in ids {
        if let Some(to) = moved.get(&(area, path.clone()))
            && let Some(id) = app.window_ids.remove(&(area, path))
        {
            taken.extend(to.clone().map(|to| (to, id)));
        }
    }
    app.window_ids.extend(taken);
    // (Those of windows closed since, forgotten.)
    let mut open = std::collections::HashSet::new();
    for (_, tab) in app.dock.iter_all_tabs() {
        match tab {
            Tab::Instance { area, path } | Tab::InstanceModel { area, path } => {
                open.insert((*area, path.clone()));
            }
            Tab::Instances { area, paths } => open.extend(paths.iter().map(|p| (*area, p.clone()))),
            _ => {}
        }
    }
    app.window_ids.retain(|place, _| open.contains(place));
    let pages: Vec<_> = app.blueprint_pages.keys().cloned().collect();
    let mut taken = Vec::new();
    for (key, path) in pages {
        if let Some(to) = now(&key, &path)
            && let Some(page) = app.blueprint_pages.remove(&(key, path))
        {
            taken.extend(to.map(|to| (to, page)));
        }
    }
    app.blueprint_pages.extend(taken);
    let changed: Vec<_> = app.properties_changed.keys().cloned().collect();
    let mut taken = Vec::new();
    for (key, path) in changed {
        let Some(to) = now(&key, &path) else { continue };
        let Some(mut changes) = app.properties_changed.remove(&(key, path)) else { continue };
        let Some(to) = to else { continue };
        moved_tab(&mut changes.tab, &moved);
        for was in &mut changes.was {
            if let super::Was::Field(k, p, ..) = was
                && let Some(Some((k2, p2))) = now(k, p)
            {
                (*k, *p) = (k2, p2);
            }
        }
        taken.push((to, changes));
    }
    app.properties_changed.extend(taken);
}

#[cfg(test)]
mod tests {
    use mg_gff::Value;

    use super::*;

    fn item(tag: &str) -> Struct {
        let mut s = Struct::new(0);
        s.set("Tag", Value::String(tag.as_bytes().to_vec()));
        s
    }

    /// An area with a chest (the second placeable) holding three items.
    fn area() -> (ResKey, Struct) {
        let key = ResKey::parse("area", ResType::GIT).unwrap();
        let mut chest = Struct::new(9);
        chest.set("ItemList", Value::List(vec![item("a"), item("b"), item("c")]));
        let mut root = Struct::new(u32::MAX);
        root.set("Placeable List", Value::List(vec![Struct::new(9), chest]));
        (key, root)
    }

    fn chest() -> GffPath {
        GffPath::root().item("Placeable List", 1)
    }

    /// Where `path` of the document `key` is after `edits`, in it.
    fn one(root: &Struct, key: ResKey, path: &GffPath, edits: &[Edit]) -> Option<GffPath> {
        let docs = HashMap::from([(key, root.clone())]);
        let now = follow(docs, &[(key, path.clone())], edits).remove(0)?;
        assert_eq!(now.0, key);
        Some(now.1)
    }

    #[test]
    fn a_held_item_follows_what_is_added_and_removed_before_it() {
        let (key, root) = area();
        let b = chest().item("ItemList", 1);
        let insert = |i| Edit::insert_item(key, chest(), "ItemList", i, item("new"));
        let remove = |i| Edit::remove_item(key, chest(), "ItemList", i);
        assert_eq!(one(&root, key, &b, &[insert(0)]), Some(chest().item("ItemList", 2)));
        assert_eq!(one(&root, key, &b, &[insert(1)]), Some(chest().item("ItemList", 2)));
        assert_eq!(one(&root, key, &b, &[insert(2)]), Some(b.clone()), "after it");
        assert_eq!(one(&root, key, &b, &[remove(0)]), Some(chest().item("ItemList", 0)));
        assert_eq!(one(&root, key, &b, &[remove(2)]), Some(b.clone()));
        assert_eq!(one(&root, key, &b, &[remove(1)]), None, "removed");
        // Another object's list, another area's: not its.
        let other =
            Edit::remove_item(key, GffPath::root().item("Placeable List", 0), "ItemList", 0);
        assert_eq!(one(&root, key, &b, &[other]), Some(b.clone()));
        let elsewhere = ResKey::parse("other", ResType::GIT).unwrap();
        let there = Edit::remove_item(elsewhere, chest(), "ItemList", 0);
        assert_eq!(one(&root, key, &b, &[there]), Some(b.clone()));
    }

    /// Moved in its list (taken out and put back: Move item), it is
    /// followed there; an item like it put back is not taken for it.
    #[test]
    fn a_held_item_follows_a_move() {
        let (key, root) = area();
        let b = chest().item("ItemList", 1);
        let moved = [
            Edit::remove_item(key, chest(), "ItemList", 1),
            Edit::insert_item(key, chest(), "ItemList", 2, item("b")),
        ];
        assert_eq!(one(&root, key, &b, &moved), Some(chest().item("ItemList", 2)));
        // Another moved before it.
        let a_moved = [
            Edit::remove_item(key, chest(), "ItemList", 0),
            Edit::insert_item(key, chest(), "ItemList", 2, item("a")),
        ];
        assert_eq!(one(&root, key, &b, &a_moved), Some(chest().item("ItemList", 0)));
        // Taken out and another item put in: gone.
        let replaced = [
            Edit::remove_item(key, chest(), "ItemList", 1),
            Edit::insert_item(key, chest(), "ItemList", 1, item("x")),
        ];
        assert_eq!(one(&root, key, &b, &replaced), None);
    }

    /// What holds it moved: it moves with it; removed, it goes with it.
    #[test]
    fn a_held_item_goes_where_what_holds_it_goes() {
        let (key, root) = area();
        let b = chest().item("ItemList", 1);
        let before = Edit::remove_item(key, GffPath::root(), "Placeable List", 0);
        let held = GffPath::root().item("Placeable List", 0).item("ItemList", 1);
        assert_eq!(one(&root, key, &b, &[before]), Some(held));
        let it = Edit::remove_item(key, GffPath::root(), "Placeable List", 1);
        assert_eq!(one(&root, key, &b, &[it]), None);
    }

    /// A creature (the first in the list) with a cloak and a ring in its
    /// backpack and a helmet equipped, as the game's modules hold them.
    fn creature() -> (ResKey, Struct) {
        let key = ResKey::parse("area", ResType::GIT).unwrap();
        let held = |tag: &str, id: u32, x: Option<u16>| {
            let mut s = item(tag);
            s.id = id;
            s.set(
                "TemplateResRef",
                Value::resref(mg_core::ResRef::from_bytes(tag.as_bytes()).unwrap()),
            );
            if let Some(x) = x {
                s.set("Repos_PosX", Value::Word(x));
                s.set("Repos_Posy", Value::Word(0));
            }
            s
        };
        let mut c = Struct::new(4);
        c.set("ItemList", Value::List(vec![held("cloak", 0, Some(0)), held("ring", 1, Some(2))]));
        c.set("Equip_ItemList", Value::List(vec![held("helm", 1, None)]));
        let mut root = Struct::new(u32::MAX);
        root.set("Creature List", Value::List(vec![c]));
        (key, root)
    }

    fn at() -> GffPath {
        GffPath::root().item("Creature List", 0)
    }

    /// Equipped (taken from the backpack and put in a slot, as it is but
    /// for where it is held), put back in the backpack, or carried to
    /// another object of another area: followed there.
    #[test]
    fn a_held_item_follows_it_into_another_list() {
        let (key, root) = creature();
        let ring = at().item("ItemList", 1);
        let taken = ring.get(&root).unwrap().clone();
        let worn = mg_module::instances::equipped_item(&taken, 0x100);
        let equip = [
            Edit::remove_item(key, at(), "ItemList", 1),
            Edit::insert_item(key, at(), "Equip_ItemList", 1, worn.clone()),
        ];
        assert_eq!(one(&root, key, &ring, &equip), Some(at().item("Equip_ItemList", 1)));
        // Back to the backpack: at its end, in a place of the grid.
        let helm = at().item("Equip_ItemList", 0);
        let stowed = mg_module::instances::listed_item(helm.get(&root).unwrap(), 2, 3, 0);
        let stow = [
            Edit::remove_item(key, at(), "Equip_ItemList", 0),
            Edit::insert_item(key, at(), "ItemList", 2, stowed),
        ];
        assert_eq!(one(&root, key, &helm, &stow), Some(at().item("ItemList", 2)));
        // Changed on the way (another item): not followed.
        let mut other = worn.clone();
        other.set("Tag", Value::String(b"x".to_vec()));
        let swapped = [equip[0].clone(), Edit::insert_item(key, at(), "Equip_ItemList", 1, other)];
        assert_eq!(one(&root, key, &ring, &swapped), None);
        // Into a chest of another area: there.
        let (cave, mut cave_root) = area();
        let cave = ResKey::new(mg_core::ResRef::from_bytes(b"cave").unwrap(), cave.restype);
        cave_root.set("Creature List", Value::List(Vec::new()));
        let docs = HashMap::from([(key, root.clone()), (cave, cave_root)]);
        let carried = [
            Edit::remove_item(key, at(), "ItemList", 1),
            Edit::insert_item(
                cave,
                chest(),
                "ItemList",
                3,
                mg_module::instances::listed_item(&taken, 3, 0, 1),
            ),
        ];
        let now = follow(docs, &[(key, ring.clone())], &carried);
        assert_eq!(now, [Some((cave, chest().item("ItemList", 3)))]);
    }

    /// A list set whole (a window's Cancel): the item where it is in the
    /// new list (by what it is, as it was moved or not), and an item
    /// equipped found where the backpack set back has it, whichever list
    /// is set first. Not in either: gone.
    #[test]
    fn a_held_item_follows_its_list_set_whole() {
        let (key, root) = creature();
        let list = |p: &GffPath, l: &str| p.get(&root).unwrap().list(l).unwrap().to_vec();
        let (pack, worn) = (list(&at(), "ItemList"), list(&at(), "Equip_ItemList"));
        let set = |l: &str, items: Vec<Struct>| Edit::SetField {
            key,
            path: at(),
            label: l.into(),
            value: Some(Value::List(items)),
        };
        // The ring before the cloak now: followed to its new place.
        let ring = at().item("ItemList", 1);
        let turned = vec![pack[1].clone(), pack[0].clone()];
        assert_eq!(
            one(&root, key, &ring, &[set("ItemList", turned)]),
            Some(at().item("ItemList", 0))
        );
        // The ring was equipped; the lists set back: in the backpack again.
        let equipped = mg_module::instances::equipped_item(&pack[1], 0x100);
        let mut now = root.clone();
        let c = at().get_mut(&mut now).unwrap();
        c.set("ItemList", Value::List(vec![pack[0].clone()]));
        c.set("Equip_ItemList", Value::List(vec![worn[0].clone(), equipped]));
        let on = at().item("Equip_ItemList", 1);
        for order in [["ItemList", "Equip_ItemList"], ["Equip_ItemList", "ItemList"]] {
            let back: Vec<Edit> = order
                .iter()
                .map(|l| set(l, if *l == "ItemList" { pack.clone() } else { worn.clone() }))
                .collect();
            assert_eq!(one(&now, key, &on, &back), Some(ring.clone()), "{order:?}");
        }
        // A flag of it set back with the list: where it was.
        let mut flagged = pack.clone();
        flagged[1].set("Dropable", Value::Byte(1));
        let mut now = root.clone();
        at().get_mut(&mut now).unwrap().set("ItemList", Value::List(flagged));
        assert_eq!(one(&now, key, &ring, &[set("ItemList", pack.clone())]), Some(ring.clone()));
        // Gone from the list set back: gone.
        let without = vec![pack[0].clone()];
        assert_eq!(one(&root, key, &ring, &[set("ItemList", without)]), None);
    }

    /// The same item, in two places: all but where it is held alike.
    #[test]
    fn an_item_moved_is_the_same_item() {
        let (_, root) = creature();
        let ring = at().item("ItemList", 1).get(&root).unwrap().clone();
        let worn = mg_module::instances::equipped_item(&ring, 0x100);
        assert!(same_item(&ring, &worn));
        assert!(same_item(&mg_module::instances::listed_item(&worn, 5, 4, 2), &ring));
        let mut other = ring.clone();
        other.set("Dropable", Value::Byte(1));
        assert!(!same_item(&ring, &other));
    }
}
