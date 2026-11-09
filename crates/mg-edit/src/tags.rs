//! The tags a command gives the module's objects, to weigh once it is
//! applied (`mg_module::tags::clashes`): the window's Duplicate Tags and
//! `mg set`/`mg apply`'s warnings read them here.

use mg_core::{ResRef, ResType};
use mg_gff::{Struct, Value};
use mg_resman::ResKey;
use mg_rules::ObjectKind;

use crate::{Edit, GffPath, Step, Workspace};

/// An object given a tag: where its tag is.
#[derive(Debug, Clone, PartialEq)]
pub struct Made {
    /// Its GIT (a placed object) or ARE (an area's own tag).
    pub key: ResKey,
    pub path: GffPath,
}

/// The objects `edits` give a tag: each placed object a `SetField` gives
/// another `Tag`, each area's own, and each object they place (an
/// `InsertItem` into an area's list of objects). An object only moved
/// (taken out of a list and put back in the same command) or given the
/// tag it had is not: an object placed is a moved one only where the
/// command takes one of its tag and blueprint out of that list, one for
/// one (a command that takes one object out and places another gives the
/// other's tag). Read before the edits are applied (`ws` as it is then);
/// each object is given where it is once they are (an edit's places are
/// those of its moment in the command).
pub fn made_by(ws: &mut Workspace, edits: &[Edit]) -> Vec<Made> {
    /// An item of a list of placed objects while the edits go by.
    #[derive(Clone, Copy, PartialEq)]
    enum Slot {
        /// The item at this place before the command.
        Was(usize),
        /// The item this edit (by its number) places.
        New(usize),
        /// One of a list set whole: nothing is known of it.
        Other,
    }
    type ListAt = (ResKey, String);
    let placed = |key: &ResKey, list: &str| {
        key.restype == ResType::GIT && ObjectKind::from_list(list).is_some()
    };
    // The lists the edits touch, each as the command leaves it.
    let mut lists: Vec<(ListAt, Vec<Slot>)> = Vec::new();
    fn slots_of(
        lists: &mut Vec<(ListAt, Vec<Slot>)>,
        ws: &mut Workspace,
        key: &ResKey,
        list: &str,
    ) -> usize {
        if let Some(i) = lists.iter().position(|((k, l), _)| k == key && l == list) {
            return i;
        }
        let len = (ws.doc(key).ok()).and_then(|d| d.root.list(list)).map_or(0, <[Struct]>::len);
        lists.push(((*key, list.to_string()), (0..len).map(Slot::Was).collect()));
        lists.len() - 1
    }
    // What the edits take out, tags they set (the edit's number, its
    // list and the slot), and the area tags set.
    let mut taken: Vec<(usize, Slot)> = Vec::new();
    let mut retagged: Vec<(usize, usize, Slot, &[u8])> = Vec::new();
    let mut areas: Vec<(usize, Made)> = Vec::new();
    for (n, e) in edits.iter().enumerate() {
        match e {
            Edit::SetField { key, path, label, value: Some(Value::String(new)) }
                if label == "Tag" =>
            {
                match path.0.as_slice() {
                    [] if key.restype == ResType::ARE => {
                        let was = ws.doc(key).ok().and_then(|d| d.root.string("Tag"));
                        if was != Some(new.as_slice()) {
                            areas.push((n, Made { key: *key, path: path.clone() }));
                        }
                    }
                    [Step::Item(list, i)] if placed(key, list) => {
                        let at = slots_of(&mut lists, ws, key, list);
                        if let Some(slot) = lists[at].1.get(*i) {
                            retagged.push((n, at, *slot, new.as_slice()));
                        }
                    }
                    _ => {}
                }
            }
            Edit::SetField { key, path, label, value }
                if path.0.is_empty() && placed(key, label) =>
            {
                let at = slots_of(&mut lists, ws, key, label);
                let len = match value {
                    Some(Value::List(items)) => items.len(),
                    _ => 0,
                };
                lists[at].1 = vec![Slot::Other; len];
            }
            Edit::InsertItem { key, path, list, index, .. }
                if path.0.is_empty() && placed(key, list) =>
            {
                let at = slots_of(&mut lists, ws, key, list);
                let slots = &mut lists[at].1;
                slots.insert((*index).min(slots.len()), Slot::New(n));
            }
            Edit::RemoveItem { key, path, list, index }
                if path.0.is_empty() && placed(key, list) =>
            {
                let at = slots_of(&mut lists, ws, key, list);
                if *index < lists[at].1.len() {
                    taken.push((at, lists[at].1.remove(*index)));
                }
            }
            _ => {}
        }
    }
    // What an object is told by, for a move: its tag and its blueprint.
    let told = |ws: &mut Workspace, at: usize, slot: Slot| -> Option<(Vec<u8>, Option<ResRef>)> {
        let (key, list) = &lists[at].0;
        let field = ObjectKind::from_list(list)?.resref_field();
        let of = |s: &Struct| (s.string("Tag").unwrap_or(b"").to_vec(), s.resref(field));
        match slot {
            Slot::Was(i) => ws.doc(key).ok()?.root.list(list)?.get(i).map(of),
            Slot::New(n) => match edits.get(n)? {
                Edit::InsertItem { item, .. } => Some(of(item)),
                _ => None,
            },
            Slot::Other => None,
        }
    };
    // The objects that were there and are taken out: one of them stands
    // for one placed that is the same.
    let mut gone: Vec<(usize, (Vec<u8>, Option<ResRef>))> = Vec::new();
    for &(at, slot) in &taken {
        if matches!(slot, Slot::Was(_))
            && let Some(t) = told(ws, at, slot)
        {
            gone.push((at, t));
        }
    }
    let mut made: Vec<(usize, Made)> = areas;
    for at in 0..lists.len() {
        for (place, slot) in lists[at].1.clone().into_iter().enumerate() {
            let here = || {
                let (key, list) = &lists[at].0;
                Made { key: *key, path: GffPath::root().item(list, place) }
            };
            // The first edit that set its tag to another than it had.
            let was = told(ws, at, slot).map(|(tag, _)| tag);
            let set = (retagged.iter())
                .find(|(_, l, s, new)| *l == at && *s == slot && was.as_deref() != Some(*new));
            let Slot::New(n) = slot else {
                made.extend(set.map(|(n, ..)| (*n, here())));
                continue;
            };
            let moved =
                told(ws, at, slot).and_then(|t| gone.iter().position(|(l, g)| *l == at && *g == t));
            match moved {
                Some(i) => {
                    gone.remove(i);
                    made.extend(set.map(|(n, ..)| (*n, here())));
                }
                None => made.push((n, here())),
            }
        }
    }
    made.sort_by_key(|(n, _)| *n);
    let mut out: Vec<Made> = made.into_iter().map(|(_, m)| m).collect();
    out.dedup();
    out
}

/// A made object's tag, as `mg_module::tags::clashes` weighs it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Weighed {
    pub tag: Vec<u8>,
    /// The area it is (an area's own tag) or is placed in.
    pub area: ResRef,
    /// A builder set it: not its blueprint's own (`blueprint` finds the
    /// blueprint an object names), an area's always.
    pub custom: bool,
    /// A static placeable: scenery, never warned of.
    pub is_static: bool,
}

/// What a made object's tag is, as the workspace has it now (after the
/// command): `blueprint` gives the blueprint a placed object names (the
/// module's, its haks', the game's), where it is found.
pub fn weigh(
    ws: &mut Workspace,
    made: &Made,
    blueprint: &dyn Fn(&Workspace, ResKey) -> Option<Struct>,
) -> Option<Weighed> {
    let doc = ws.doc(&made.key).ok()?;
    let s = made.path.get(&doc.root)?.clone();
    let kind = match made.path.0.last() {
        Some(Step::Item(list, _)) => ObjectKind::from_list(list),
        _ => None,
    };
    // (A store names its blueprint in ResRef, the others in TemplateResRef.)
    let template =
        kind.and_then(|kind| Some(ResKey::new(s.resref(kind.resref_field())?, kind.restype())));
    let bp = template.and_then(|key| blueprint(ws, key));
    let w = mg_module::tags::weigh(&s, kind, bp.as_ref(), made.key.resref)?;
    Some(Weighed { tag: w.tag.to_vec(), area: w.area, custom: w.custom, is_static: w.is_static })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Command;
    use mg_gff::Gff;
    use mg_module::Module;

    fn tagged(tag: &str, template: &str) -> Struct {
        let mut s = Struct::new(9);
        s.set("Tag", Value::String(tag.as_bytes().to_vec()));
        s.set("TemplateResRef", Value::resref(ResRef::from_str(template).unwrap()));
        s
    }

    /// A tag set to another, an area's own, an object placed: made; a tag
    /// set to what it was, a field not a tag, an object moved: not.
    #[test]
    fn a_command_s_tags_are_those_it_changes_or_places() {
        let area = ResRef::from_str("inn").unwrap();
        let (are, git) = (ResKey::new(area, ResType::ARE), ResKey::new(area, ResType::GIT));
        let mut m = Module::default();
        let mut a = Gff::new(*b"ARE ");
        a.root.set("Tag", Value::String(b"INN".to_vec()));
        m.set_gff(are, &a).unwrap();
        let mut g = Gff::new(*b"GIT ");
        g.root.set("WaypointList", Value::List(vec![tagged("WP_A", "nw_waypoint001")]));
        g.root.set("Placeable List", Value::List(vec![tagged("BARREL", "plc_barrel")]));
        m.set_gff(git, &g).unwrap();
        let mut ws = Workspace::new(m);
        let set = |path: GffPath, label: &str, tag: &str| Edit::SetField {
            key: if path.0.is_empty() { are } else { git },
            path,
            label: label.into(),
            value: Some(Value::String(tag.as_bytes().to_vec())),
        };
        let wp = GffPath::root().item("WaypointList", 0);
        let edits = vec![
            set(wp.clone(), "Tag", "WP_B"),
            set(GffPath::root(), "Tag", "INN"),
            set(GffPath::root().item("Placeable List", 0), "LocName", "x"),
            Edit::insert_item(
                git,
                GffPath::root(),
                "WaypointList",
                1,
                tagged("WP_A", "nw_waypoint001"),
            ),
        ];
        let made = made_by(&mut ws, &edits);
        let at = GffPath::root().item("WaypointList", 1);
        assert_eq!(
            made,
            [Made { key: git, path: wp.clone() }, Made { key: git, path: at.clone() }]
        );
        // A barrel moved to the end of its list: not made.
        let moved = vec![
            Edit::RemoveItem {
                key: git,
                path: GffPath::root(),
                list: "Placeable List".into(),
                index: 0,
            },
            Edit::insert_item(
                git,
                GffPath::root(),
                "Placeable List",
                0,
                tagged("BARREL", "plc_barrel"),
            ),
        ];
        assert!(made_by(&mut ws, &moved).is_empty());
        // The barrel taken out and another placeable placed, with the
        // tag the waypoint has: that one is made (any removal of the list
        // made every object placed into it a moved one).
        let remove_barrel = moved[0].clone();
        let other = |index| {
            let item = tagged("WP_A", "plc_chest");
            Edit::insert_item(git, GffPath::root(), "Placeable List", index, item)
        };
        let barrel = GffPath::root().item("Placeable List", 0);
        let replaced = vec![remove_barrel.clone(), other(0)];
        assert_eq!(made_by(&mut ws, &replaced), [Made { key: git, path: barrel.clone() }]);
        // One for one: the barrel moved and a second barrel placed.
        let twice = vec![remove_barrel.clone(), moved[1].clone(), moved[1].clone()];
        assert_eq!(made_by(&mut ws, &twice).len(), 1);
        // Placed before the barrel, which is then taken out: where it is
        // after the command, and a tag set after the removal is read
        // from the object it is set on.
        let chest = vec![other(1), remove_barrel.clone(), set(barrel.clone(), "Tag", "WP_A")];
        assert_eq!(made_by(&mut ws, &chest), [Made { key: git, path: barrel.clone() }]);
        let chest = vec![other(1), remove_barrel, set(barrel.clone(), "Tag", "CHEST")];
        assert_eq!(made_by(&mut ws, &chest), [Made { key: git, path: barrel }]);
        // Weighed after: the placed waypoint's tag is its blueprint's.
        ws.apply(Command::new("t", edits)).unwrap();
        let blueprint = |_: &Workspace, k: ResKey| {
            (k.resref.to_string() == "nw_waypoint001").then(|| tagged("WP_A", "nw_waypoint001"))
        };
        let w = weigh(&mut ws, &Made { key: git, path: at }, &blueprint).unwrap();
        assert_eq!((w.tag.as_slice(), w.custom, w.area), (&b"WP_A"[..], false, area));
        let w = weigh(&mut ws, &Made { key: git, path: wp }, &blueprint).unwrap();
        assert_eq!((w.tag.as_slice(), w.custom), (&b"WP_B"[..], true));
    }
}
