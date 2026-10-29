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
/// tag it had is not. Read before the edits are applied (`ws` as it is
/// then).
pub fn made_by(ws: &mut Workspace, edits: &[Edit]) -> Vec<Made> {
    let placed = |key: &ResKey, list: &str| {
        key.restype == ResType::GIT && ObjectKind::from_list(list).is_some()
    };
    let removes = |key: &ResKey, list: &str| {
        edits
            .iter()
            .any(|e| matches!(e, Edit::RemoveItem { key: k, list: l, .. } if k == key && l == list))
    };
    let mut out = Vec::new();
    for e in edits {
        match e {
            Edit::SetField { key, path, label, value: Some(Value::String(new)) }
                if label == "Tag" =>
            {
                let object = match path.0.as_slice() {
                    [] => key.restype == ResType::ARE,
                    [Step::Item(list, _)] => placed(key, list),
                    _ => false,
                };
                if !object {
                    continue;
                }
                let was = ws.doc(key).ok().and_then(|d| path.get(&d.root)?.string("Tag"));
                if was != Some(new.as_slice()) {
                    out.push(Made { key: *key, path: path.clone() });
                }
            }
            Edit::InsertItem { key, path, list, index, .. }
                if path.0.is_empty() && placed(key, list) && !removes(key, list) =>
            {
                out.push(Made { key: *key, path: path.item(list, *index) });
            }
            _ => {}
        }
    }
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
