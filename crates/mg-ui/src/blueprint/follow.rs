//! A placed object's Properties following the object as the lists it is
//! in change: a held item's window is found by the item's place in its
//! inventory (an item's struct has no name of its own: its struct id is
//! its place), so an item added, removed or moved before it would leave
//! the window on another item. Worked out from the edits a command, an
//! Undo or a Redo is about to run, and applied once they have.

use std::collections::HashMap;

use mg_core::{ResRef, ResType};
use mg_edit::{Edit, GffPath, Step};
use mg_gff::Struct;
use mg_resman::ResKey;

use crate::{Moonglow, Tab};

/// Where the struct at `path` in document `key` is after `edits` run
/// (`start`: the document's root before them): moved by the items
/// inserted or removed before it in the lists along its path, and by a
/// move (taken out and the same struct put back into its list, as Move
/// item does). `None` if it was taken out and not put back. A whole
/// resource or list set (not item by item) can't be followed: the place
/// is kept.
pub(crate) fn follow(
    start: &Struct,
    key: ResKey,
    path: &GffPath,
    edits: &[Edit],
) -> Option<GffPath> {
    let mut now = path.clone();
    // Taken out: the step of the path it was (its depth) and the struct
    // there as it was, to know it if it is put back.
    let mut out: Option<(usize, Option<Struct>)> = None;
    for e in edits {
        let (at, list) = match e {
            Edit::InsertItem { key: k, path, list, .. }
            | Edit::RemoveItem { key: k, path, list, .. }
                if *k == key =>
            {
                (path, list)
            }
            _ => continue,
        };
        // (An edit of the list at this step of the path.)
        let d = at.0.len();
        if !now.0.starts_with(&at.0) {
            continue;
        }
        let Some(Step::Item(l, i)) = now.0.get_mut(d) else { continue };
        if l != list {
            continue;
        }
        let gone_at = out.as_ref().map(|(depth, _)| *depth);
        match e {
            Edit::InsertItem { index, item, .. } => {
                if let Some((depth, was)) = &out
                    && *depth == d
                    && was.as_ref() == Some(item)
                {
                    *i = *index;
                    out = None;
                } else if gone_at.is_none_or(|g| d < g) && *index <= *i {
                    *i += 1;
                }
            }
            Edit::RemoveItem { index, .. } if gone_at.is_none_or(|g| d < g) => {
                if *index < *i {
                    *i -= 1;
                } else if *index == *i {
                    // (As the document had it before the edits: the path
                    // as it was then.)
                    let was = GffPath(path.0[..=d].to_vec()).get(start).cloned();
                    out = Some((d, was));
                }
            }
            _ => {}
        }
    }
    out.is_none().then_some(now)
}

/// The places of the open Properties windows of placed objects (and of
/// the items they hold) in the areas `edits` insert into or remove from,
/// where each is after the edits (`None`: gone). Worked out before they
/// run; [`retarget`] once they have.
pub(crate) type Followed = HashMap<(ResRef, GffPath), Option<GffPath>>;

/// [`Followed`] for `edits`, those that moved.
pub(crate) fn followed(app: &mut Moonglow, edits: &[Edit]) -> Followed {
    let mut out = Followed::new();
    let lists = |e: &&Edit| matches!(e, Edit::InsertItem { .. } | Edit::RemoveItem { .. });
    let areas: Vec<ResRef> = edits
        .iter()
        .filter(lists)
        .map(|e| e.key())
        .filter(|k| k.restype == ResType::GIT)
        .map(|k| k.resref)
        .collect();
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
    let Some(ws) = app.ws.as_mut() else { return out };
    for (area, path) in open {
        let key = ResKey::new(area, ResType::GIT);
        let Ok(doc) = ws.doc(&key) else { continue };
        let now = follow(&doc.root, key, &path, edits);
        if now.as_ref() != Some(&path) {
            out.insert((area, path), now);
        }
    }
    out
}

/// The edits of `edits` that [`followed`] reads (an Undo's or a Redo's,
/// taken from the history without the rest).
pub(crate) fn list_edits(edits: &[Edit]) -> Vec<Edit> {
    edits
        .iter()
        .filter(|e| matches!(e, Edit::InsertItem { .. } | Edit::RemoveItem { .. }))
        .cloned()
        .collect()
}

/// `tab` pointed at where its objects are now; false if none is left.
fn moved_tab(tab: &mut Tab, moved: &Followed) -> bool {
    let now = |area: ResRef, path: &GffPath| match moved.get(&(area, path.clone())) {
        Some(now) => now.clone(),
        None => Some(path.clone()),
    };
    match tab {
        Tab::Instance { area, path } | Tab::InstanceModel { area, path } => {
            match now(*area, path) {
                Some(p) => {
                    *path = p;
                    true
                }
                None => false,
            }
        }
        Tab::Instances { area, paths } => {
            *paths = paths.iter().filter_map(|p| now(*area, p)).collect();
            !paths.is_empty()
        }
        _ => true,
    }
}

/// Points the Properties windows at where their objects went (with the
/// page each shows and what each changed, for its Cancel), and closes
/// those whose objects are gone, as the area view does those of the
/// objects it deletes.
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
    let now = |key: &ResKey, path: &GffPath| -> Option<Option<GffPath>> {
        if key.restype != ResType::GIT {
            return None;
        }
        moved.get(&(key.resref, path.clone())).cloned()
    };
    let pages: Vec<_> = app.blueprint_pages.keys().cloned().collect();
    let mut taken = Vec::new();
    for (key, path) in pages {
        if let Some(to) = now(&key, &path)
            && let Some(page) = app.blueprint_pages.remove(&(key, path))
        {
            taken.extend(to.map(|to| ((key, to), page)));
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
                && let Some(Some(to)) = now(k, p)
            {
                *p = to;
            }
        }
        taken.push(((key, to), changes));
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

    #[test]
    fn a_held_item_follows_what_is_added_and_removed_before_it() {
        let (key, root) = area();
        let b = chest().item("ItemList", 1);
        let insert = |i| Edit::insert_item(key, chest(), "ItemList", i, item("new"));
        let remove = |i| Edit::remove_item(key, chest(), "ItemList", i);
        assert_eq!(follow(&root, key, &b, &[insert(0)]), Some(chest().item("ItemList", 2)));
        assert_eq!(follow(&root, key, &b, &[insert(1)]), Some(chest().item("ItemList", 2)));
        assert_eq!(follow(&root, key, &b, &[insert(2)]), Some(b.clone()), "after it");
        assert_eq!(follow(&root, key, &b, &[remove(0)]), Some(chest().item("ItemList", 0)));
        assert_eq!(follow(&root, key, &b, &[remove(2)]), Some(b.clone()));
        assert_eq!(follow(&root, key, &b, &[remove(1)]), None, "removed");
        // Another object's list, another area's: not its.
        let other =
            Edit::remove_item(key, GffPath::root().item("Placeable List", 0), "ItemList", 0);
        assert_eq!(follow(&root, key, &b, &[other]), Some(b.clone()));
        let elsewhere = ResKey::parse("other", ResType::GIT).unwrap();
        let there = Edit::remove_item(elsewhere, chest(), "ItemList", 0);
        assert_eq!(follow(&root, key, &b, &[there]), Some(b.clone()));
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
        assert_eq!(follow(&root, key, &b, &moved), Some(chest().item("ItemList", 2)));
        // Another moved before it.
        let a_moved = [
            Edit::remove_item(key, chest(), "ItemList", 0),
            Edit::insert_item(key, chest(), "ItemList", 2, item("a")),
        ];
        assert_eq!(follow(&root, key, &b, &a_moved), Some(chest().item("ItemList", 0)));
        // Taken out and another item put in: gone.
        let replaced = [
            Edit::remove_item(key, chest(), "ItemList", 1),
            Edit::insert_item(key, chest(), "ItemList", 1, item("x")),
        ];
        assert_eq!(follow(&root, key, &b, &replaced), None);
    }

    /// What holds it moved: it moves with it; removed, it goes with it.
    #[test]
    fn a_held_item_goes_where_what_holds_it_goes() {
        let (key, root) = area();
        let b = chest().item("ItemList", 1);
        let before = Edit::remove_item(key, GffPath::root(), "Placeable List", 0);
        let held = GffPath::root().item("Placeable List", 0).item("ItemList", 1);
        assert_eq!(follow(&root, key, &b, &[before]), Some(held));
        let it = Edit::remove_item(key, GffPath::root(), "Placeable List", 1);
        assert_eq!(follow(&root, key, &b, &[it]), None);
    }
}
