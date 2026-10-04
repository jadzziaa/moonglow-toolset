//! Edits to an area's objects, as workspace edits of its GIT: moving and
//! turning them, and deleting them. Only the fields that change are
//! written, so an object that is moved keeps its orientation bit for bit.

use std::collections::BTreeMap;
use std::f32::consts::FRAC_PI_2;

use glam::Vec3;
use mg_edit::{Edit, GffPath};
use mg_gff::{Struct, Value};
use mg_resman::ResKey;

use crate::{AreaObject, ObjectKind};

/// A field that moving an object changes: in the object itself (`None`) or
/// in one of its spawn points.
type Change = (Option<usize>, &'static str, Value);

/// What moving object `o` (its GIT struct `s`) to `position`, turned by
/// `rotation`, changes; only fields whose value changes.
fn changes(o: &AreaObject, s: &Struct, position: Vec3, rotation: f32) -> Vec<Change> {
    let mut fields: Vec<(&'static str, f32)> = Vec::new();
    if o.kind.has_bearing() {
        fields.extend([("X", position.x), ("Y", position.y), ("Z", position.z)]);
        fields.push(("Bearing", rotation));
    } else {
        fields.extend([
            ("XPosition", position.x),
            ("YPosition", position.y),
            ("ZPosition", position.z),
        ]);
        let turns = matches!(
            o.kind,
            ObjectKind::Creature | ObjectKind::Item | ObjectKind::Store | ObjectKind::Waypoint
        );
        if turns && (rotation - o.rotation).abs() > 1e-6 {
            let facing = rotation + FRAC_PI_2;
            fields.extend([("XOrientation", facing.cos()), ("YOrientation", facing.sin())]);
        }
    }
    let mut out: Vec<Change> = fields
        .into_iter()
        .filter(|(label, v)| s.float(label) != Some(*v))
        .map(|(label, v)| (None, label, Value::Float(v)))
        .collect();
    let delta = position - o.position;
    if o.kind == ObjectKind::Encounter && delta != Vec3::ZERO {
        for (i, p) in s.list("SpawnPointList").unwrap_or(&[]).iter().enumerate() {
            for (label, d) in [("X", delta.x), ("Y", delta.y), ("Z", delta.z)] {
                if d != 0.0 {
                    out.push((Some(i), label, Value::Float(p.float(label).unwrap_or(0.0) + d)));
                }
            }
        }
    }
    out
}

/// The edits that move object `o` (its GIT struct `s`, in the GIT `git`)
/// to `position`, its model turned by `rotation` (radians; see
/// [`AreaObject::rotation`]). Triggers and encounters move with their
/// outlines (their points are relative to the position); an encounter's
/// spawn points, which are not, move along. Sounds, triggers and
/// encounters do not turn.
pub fn move_edits(
    git: ResKey,
    o: &AreaObject,
    s: &Struct,
    position: Vec3,
    rotation: f32,
) -> Vec<Edit> {
    let path = GffPath::root().item(o.kind.list(), o.index);
    changes(o, s, position, rotation)
        .into_iter()
        .map(|(spawn, label, value)| Edit::SetField {
            key: git,
            path: match spawn {
                Some(i) => path.item("SpawnPointList", i),
                None => path.clone(),
            },
            label: label.into(),
            value: Some(value),
        })
        .collect()
}

/// A copy of object `o` (its GIT struct `s`) standing at `position`,
/// turned by `rotation` (for pasting).
pub fn moved(o: &AreaObject, s: &Struct, position: Vec3, rotation: f32) -> Struct {
    let mut copy = s.clone();
    for (spawn, label, value) in changes(o, s, position, rotation) {
        match spawn {
            None => copy.set(label, value),
            Some(i) => {
                if let Some(p) = copy.list_mut("SpawnPointList").and_then(|l| l.get_mut(i)) {
                    p.set(label, value);
                }
            }
        }
    }
    copy
}

/// One axis of a visual transform entry, as Aurora writes it.
fn axis(value: f32) -> Value {
    let mut s = Struct::new(0);
    s.set("TimerType", Value::Int(0));
    s.set("ValueTo", Value::Float(value));
    s.set("LerpType", Value::Int(0));
    Value::Struct(s)
}

/// The edits that give object `o` (its GIT struct `s`) the visual
/// transform `v`, as Aurora's Adjust Location writes it: `VisTransformList`
/// with one entry for scope 0 (an older `VisualTransform` goes). Nothing
/// when it already has it, or has none and `v` changes nothing.
pub fn visual_transform_edits(
    git: ResKey,
    o: &AreaObject,
    s: &Struct,
    v: crate::VisualTransform,
) -> Vec<Edit> {
    let current = crate::VisualTransform::read(s);
    if current.unwrap_or_default() == v && (current.is_some() || v.is_identity()) {
        return Vec::new();
    }
    let mut entry = Struct::new(6);
    entry.set("Scope", Value::Int(0));
    entry.set("AnimationSpeed", axis(1.0));
    for (prefix, value) in [("Scale", v.scale), ("Rotate", v.rotate), ("Translate", v.translate)] {
        for (i, a) in ["X", "Y", "Z"].iter().enumerate() {
            entry.set(&format!("{prefix}{a}"), axis(value[i]));
        }
    }
    // Other scopes' entries stay.
    let mut list: Vec<Struct> = s
        .list("VisTransformList")
        .unwrap_or(&[])
        .iter()
        .filter(|e| e.integer("Scope").unwrap_or(0) != 0)
        .cloned()
        .collect();
    list.insert(0, entry);
    let path = GffPath::root().item(o.kind.list(), o.index);
    let mut edits = vec![Edit::SetField {
        key: git,
        path: path.clone(),
        label: "VisTransformList".into(),
        value: Some(Value::List(list)),
    }];
    if s.contains("VisualTransform") {
        edits.push(Edit::SetField { key: git, path, label: "VisualTransform".into(), value: None });
    }
    edits
}

/// The edits that delete `objects` (kind and index in its list) from the
/// GIT `git`: from the end of each list, so that the indices stay valid.
pub fn delete_edits(git: ResKey, objects: &[(ObjectKind, usize)]) -> Vec<Edit> {
    let mut by_list: BTreeMap<&str, Vec<usize>> = BTreeMap::new();
    for (kind, index) in objects {
        by_list.entry(kind.list()).or_default().push(*index);
    }
    let mut edits = Vec::new();
    for (list, mut indices) in by_list {
        indices.sort_unstable();
        indices.dedup();
        for index in indices.into_iter().rev() {
            edits.push(Edit::RemoveItem {
                key: git,
                path: GffPath::root(),
                list: list.into(),
                index,
            });
        }
    }
    edits
}

#[cfg(test)]
mod tests {
    use super::*;
    use mg_core::{ResRef, ResType};

    fn git() -> ResKey {
        ResKey::new(ResRef::from_str("area").unwrap(), ResType::GIT)
    }

    fn object(kind: ObjectKind, position: Vec3, rotation: f32) -> AreaObject {
        AreaObject {
            kind,
            index: 2,
            position,
            rotation,
            tag: String::new(),
            template: None,
            preview: None,
            problem: None,
            outline: Vec::new(),
            visual: None,
            is_static: false,
            trigger_type: 0,
            conversation: None,
            spawn_points: Vec::new(),
            spawn_facings: Vec::new(),
            locked: false,
            state: 0,
        }
    }

    fn labels(edits: &[Edit]) -> Vec<String> {
        edits
            .iter()
            .map(|e| match e {
                Edit::SetField { label, path, .. } => format!("{}{label}", path.0.len()),
                _ => String::new(),
            })
            .collect()
    }

    #[test]
    fn moving_writes_only_what_changes() {
        let mut s = Struct::new(4);
        for (label, v) in [
            ("XPosition", 1.0),
            ("YPosition", 2.0),
            ("ZPosition", 0.0),
            ("XOrientation", 0.3),
            ("YOrientation", 0.954),
        ] {
            s.set(label, Value::Float(v));
        }
        let o = object(ObjectKind::Creature, Vec3::new(1.0, 2.0, 0.0), 0.3);
        let edits = move_edits(git(), &o, &s, Vec3::new(4.0, 2.0, 0.0), 0.3);
        assert_eq!(labels(&edits), ["1XPosition"]);
        let Edit::SetField { path, value, .. } = &edits[0] else { unreachable!() };
        assert_eq!(path, &GffPath::root().item("Creature List", 2));
        assert_eq!(value, &Some(Value::Float(4.0)));
        // Turning writes the orientation vector along the new facing.
        let edits = move_edits(git(), &o, &s, o.position, 0.0);
        assert_eq!(labels(&edits), ["1XOrientation", "1YOrientation"]);
        let Edit::SetField { value: Some(Value::Float(y)), .. } = &edits[1] else { panic!() };
        assert!((y - 1.0).abs() < 1e-6, "facing north");
    }

    #[test]
    fn placeables_turn_by_bearing_and_encounters_take_their_spawn_points() {
        let mut p = Struct::new(9);
        p.set("Bearing", Value::Float(0.0));
        let o = object(ObjectKind::Placeable, Vec3::ZERO, 0.0);
        let edits = move_edits(git(), &o, &p, Vec3::new(0.0, 0.0, 1.0), 1.0);
        assert_eq!(labels(&edits), ["1X", "1Y", "1Z", "1Bearing"]);
        let mut e = Struct::new(7);
        let mut spawn = Struct::new(2);
        spawn.set("X", Value::Float(5.0));
        spawn.set("Y", Value::Float(5.0));
        e.set("SpawnPointList", Value::List(vec![spawn]));
        let o = object(ObjectKind::Encounter, Vec3::ZERO, 0.0);
        let edits = move_edits(git(), &o, &e, Vec3::new(2.0, 0.0, 0.0), 0.0);
        assert_eq!(labels(&edits), ["1XPosition", "1YPosition", "1ZPosition", "2X"]);
        let Edit::SetField { value, .. } = &edits[3] else { unreachable!() };
        assert_eq!(value, &Some(Value::Float(7.0)));
    }

    #[test]
    fn copies_stand_where_pasted() {
        let mut e = Struct::new(7);
        e.set("XPosition", Value::Float(1.0));
        let mut spawn = Struct::new(2);
        spawn.set("X", Value::Float(5.0));
        e.set("SpawnPointList", Value::List(vec![spawn]));
        let o = object(ObjectKind::Encounter, Vec3::new(1.0, 0.0, 0.0), 0.0);
        let copy = moved(&o, &e, Vec3::new(4.0, 2.0, 0.0), 0.0);
        assert_eq!((copy.float("XPosition"), copy.float("YPosition")), (Some(4.0), Some(2.0)));
        assert_eq!(copy.list("SpawnPointList").unwrap()[0].float("X"), Some(8.0));
        assert_eq!(e.float("XPosition"), Some(1.0), "the original is untouched");
    }

    #[test]
    fn deletes_go_from_the_end() {
        let edits = delete_edits(
            git(),
            &[(ObjectKind::Placeable, 1), (ObjectKind::Placeable, 4), (ObjectKind::Door, 0)],
        );
        let order: Vec<(String, usize)> = edits
            .iter()
            .map(|e| match e {
                Edit::RemoveItem { list, index, .. } => (list.clone(), *index),
                _ => unreachable!(),
            })
            .collect();
        assert_eq!(
            order,
            [("Door List".into(), 0), ("Placeable List".into(), 4), ("Placeable List".into(), 1)]
        );
    }
}
