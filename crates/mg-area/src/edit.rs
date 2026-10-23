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

/// Where a field that moving an object changes is: in the object itself,
/// in one of its spawn points or in a point of its outline.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Place {
    Object,
    Spawn(usize),
    Point(usize),
}

impl Place {
    fn path(self, object: &GffPath) -> GffPath {
        match self {
            Place::Object => object.clone(),
            Place::Spawn(i) => object.item("SpawnPointList", i),
            Place::Point(i) => object.item("Geometry", i),
        }
    }

    fn of(self, s: &mut Struct) -> Option<&mut Struct> {
        match self {
            Place::Object => Some(s),
            Place::Spawn(i) => s.list_mut("SpawnPointList")?.get_mut(i),
            Place::Point(i) => s.list_mut("Geometry")?.get_mut(i),
        }
    }
}

/// A field that moving an object changes.
type Change = (Place, &'static str, Value);

/// What moving object `o` (its GIT struct `s`) to `position`, turned by
/// `rotation`, changes; only fields whose value changes. A trigger or an
/// encounter turns by its outline: the points turn about its position (and
/// an encounter's spawn points with them).
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
        .map(|(label, v)| (Place::Object, label, Value::Float(v)))
        .collect();
    let turn = if o.kind.has_outline() { rotation - o.rotation } else { 0.0 };
    let turned = turn.abs() > 1e-6;
    let round = glam::Vec2::from_angle(turn);
    if turned {
        let (x, y) = if o.kind == ObjectKind::Trigger { ("PointX", "PointY") } else { ("X", "Y") };
        for (i, p) in s.list("Geometry").unwrap_or(&[]).iter().enumerate() {
            let was = glam::Vec2::new(p.float(x).unwrap_or(0.0), p.float(y).unwrap_or(0.0));
            let to = round.rotate(was);
            out.push((Place::Point(i), x, Value::Float(to.x)));
            out.push((Place::Point(i), y, Value::Float(to.y)));
        }
    }
    let delta = position - o.position;
    if o.kind == ObjectKind::Encounter && (delta != Vec3::ZERO || turned) {
        for (i, p) in s.list("SpawnPointList").unwrap_or(&[]).iter().enumerate() {
            let f = |label: &str| p.float(label).unwrap_or(0.0);
            let was = Vec3::new(f("X"), f("Y"), f("Z"));
            let about = round.rotate((was - o.position).truncate()).extend(was.z - o.position.z);
            let to = if turned { position + about } else { was + delta };
            for (label, was, to) in [("X", was.x, to.x), ("Y", was.y, to.y), ("Z", was.z, to.z)] {
                if to != was {
                    out.push((Place::Spawn(i), label, Value::Float(to)));
                }
            }
            if turned && p.contains("Orientation") {
                out.push((Place::Spawn(i), "Orientation", Value::Float(f("Orientation") + turn)));
            }
        }
    }
    out
}

/// The edits that move object `o` (its GIT struct `s`, in the GIT `git`)
/// to `position`, its model turned by `rotation` (radians; see
/// [`AreaObject::rotation`]). Triggers and encounters move with their
/// outlines (their points are relative to the position); an encounter's
/// spawn points, which are not, move along. Triggers and encounters turn
/// by their outlines; sounds do not turn.
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
        .map(|(place, label, value)| Edit::SetField {
            key: git,
            path: place.path(&path),
            label: label.into(),
            value: Some(value),
        })
        .collect()
}

/// A copy of object `o` (its GIT struct `s`) standing at `position`,
/// turned by `rotation` (for pasting).
pub fn moved(o: &AreaObject, s: &Struct, position: Vec3, rotation: f32) -> Struct {
    let mut copy = s.clone();
    for (place, label, value) in changes(o, s, position, rotation) {
        if let Some(at) = place.of(&mut copy) {
            at.set(label, value);
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

/// `VisTransformList`'s entry for scope 0 with `v`, as Aurora's Adjust
/// Location writes it.
fn visual_transform_entry(v: crate::VisualTransform) -> Struct {
    let mut entry = Struct::new(6);
    entry.set("Scope", Value::Int(0));
    entry.set("AnimationSpeed", axis(1.0));
    for (prefix, value) in [("Scale", v.scale), ("Rotate", v.rotate), ("Translate", v.translate)] {
        for (i, a) in ["X", "Y", "Z"].iter().enumerate() {
            entry.set(&format!("{prefix}{a}"), axis(value[i]));
        }
    }
    entry
}

/// A new object's `VisTransformList`, giving it the visual transform `v`
/// (as [`visual_transform_edits`] gives a placed one).
pub fn visual_transform_list(v: crate::VisualTransform) -> Value {
    Value::List(vec![visual_transform_entry(v)])
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
    let entry = visual_transform_entry(v);
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

/// An area's ARE and GIT, as workspace keys and their root structs.
#[derive(Debug)]
pub struct AreaFiles<'a> {
    pub are: ResKey,
    pub are_root: &'a Struct,
    pub git: ResKey,
    pub git_root: &'a Struct,
}

/// Tiles set to heights of their own (Tile Properties' Height): the edits
/// that write `heights` (each a `Tile_List` index and its new height, in
/// height steps of `step` metres) into the area's ARE, only their
/// `Tile_Height` where it changes (nothing around them is fitted to them:
/// a tile raised alone stands above its neighbours, with gaps between),
/// and move the objects of its GIT standing on those tiles (their
/// position over the tile's square) up or down with them, as far. The
/// objects are read from the GIT as it is (not from a view of the area,
/// which may not have caught up with the last change).
pub fn tile_height_edits(files: &AreaFiles, step: f32, heights: &[(usize, i32)]) -> Vec<Edit> {
    let width = files.are_root.integer("Width").unwrap_or(0).max(0) as usize;
    let list = files.are_root.list("Tile_List").unwrap_or(&[]);
    let mut edits = Vec::new();
    // (Each tile raised or lowered, in steps.)
    let mut moved: Vec<(usize, i32)> = Vec::new();
    for &(i, height) in heights {
        let Some(tile) = list.get(i) else { continue };
        let was = tile.integer("Tile_Height").unwrap_or(0) as i32;
        if was == height && tile.get("Tile_Height").is_some() {
            continue;
        }
        edits.push(Edit::SetField {
            key: files.are,
            path: GffPath::root().item("Tile_List", i),
            label: "Tile_Height".into(),
            value: Some(Value::Int(height)),
        });
        if was != height {
            moved.push((i, height - was));
        }
    }
    if width == 0 || moved.is_empty() {
        return edits;
    }
    for kind in ObjectKind::ALL {
        for (index, s) in files.git_root.list(kind.list()).unwrap_or(&[]).iter().enumerate() {
            let o = AreaObject::placed(kind, index, s);
            let (x, y) = (o.position.x / crate::TILE_SIZE, o.position.y / crate::TILE_SIZE);
            if x < 0.0 || y < 0.0 || x as usize >= width {
                continue;
            }
            let i = y as usize * width + x as usize;
            let Some(&(_, steps)) = moved.iter().find(|(t, _)| *t == i) else { continue };
            let to = o.position + Vec3::Z * (steps as f32 * step);
            edits.extend(move_edits(files.git, &o, s, to, o.rotation));
        }
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
            edits.push(Edit::remove_item(git, GffPath::root(), list, index));
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
            laid: 0,
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

    /// A static placeable is drawn upright whatever visual transform its
    /// file has, as the game draws it.
    #[test]
    fn a_static_placeable_takes_no_visual_transform() {
        let mut o = object(ObjectKind::Placeable, Vec3::new(1.0, 2.0, 0.0), 0.5);
        o.visual = Some(crate::VisualTransform {
            rotate: Vec3::new(30.0, 20.0, 45.0),
            ..Default::default()
        });
        assert_ne!(o.model_transform(), o.transform());
        o.is_static = true;
        assert_eq!(o.model_transform(), o.transform());
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

    /// A tile's height set alone: its `Tile_Height`, and nothing else of
    /// the area's tiles; what stands on it rises with it, the rest stays.
    #[test]
    fn a_tile_s_height_is_set_alone_and_what_stands_on_it_rises() {
        let tile = |h: i32| {
            let mut t = Struct::new(1);
            t.set("Tile_ID", Value::Int(3));
            t.set("Tile_Height", Value::Int(h));
            t
        };
        let mut are = Struct::new(0xffff_ffff);
        are.set("Width", Value::Int(2));
        are.set("Tile_List", Value::List(vec![tile(0), tile(0), tile(1), tile(0)]));
        let point = |x: f32, y: f32| {
            let mut s = Struct::new(5);
            for (label, v) in [("XPosition", x), ("YPosition", y), ("ZPosition", 0.0)] {
                s.set(label, Value::Float(v));
            }
            s
        };
        let mut git_root = Struct::new(0xffff_ffff);
        git_root.set("WaypointList", Value::List(vec![point(15.0, 5.0), point(5.0, 5.0)]));
        let files = AreaFiles {
            are: ResKey::new(ResRef::from_str("area").unwrap(), ResType::ARE),
            are_root: &are,
            git: git(),
            git_root: &git_root,
        };
        // Tile 1 (south-east) two steps of 5 m up; tile 2 kept at 1.
        let edits = tile_height_edits(&files, 5.0, &[(1, 2), (2, 1)]);
        let [tile_edit, raised] = edits.as_slice() else { panic!("{edits:?}") };
        let Edit::SetField { key, path, label, value } = tile_edit else { panic!() };
        assert_eq!(
            (key, path, label.as_str(), value),
            (
                &files.are,
                &GffPath::root().item("Tile_List", 1),
                "Tile_Height",
                &Some(Value::Int(2))
            )
        );
        let Edit::SetField { path, label, value, .. } = raised else { panic!() };
        assert_eq!(path, &GffPath::root().item("WaypointList", 0));
        assert_eq!((label.as_str(), value), ("ZPosition", &Some(Value::Float(10.0))));
        // Down below 0: as far.
        let edits = tile_height_edits(&files, 5.0, &[(0, -1)]);
        assert_eq!(labels(&edits), ["1Tile_Height", "1ZPosition"]);
        let Edit::SetField { value, .. } = &edits[1] else { panic!() };
        assert_eq!(value, &Some(Value::Float(-5.0)));
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
    fn a_trigger_turns_by_its_outline_and_an_encounter_takes_its_spawn_points_round() {
        let point = |labels: [&str; 2], x: f32, y: f32| {
            let mut p = Struct::new(3);
            p.set(labels[0], Value::Float(x));
            p.set(labels[1], Value::Float(y));
            p
        };
        let mut t = Struct::new(1);
        t.set("Geometry", Value::List(vec![point(["PointX", "PointY"], 2.0, 0.0)]));
        let o = object(ObjectKind::Trigger, Vec3::new(1.0, 1.0, 0.0), 0.0);
        let turned = moved(&o, &t, o.position, FRAC_PI_2);
        let p = &turned.list("Geometry").unwrap()[0];
        let (x, y) = (p.float("PointX").unwrap(), p.float("PointY").unwrap());
        assert!(x.abs() < 1e-5 && (y - 2.0).abs() < 1e-5, "{x}, {y}");
        // Not turned: nothing of the outline is written.
        let unturned = move_edits(git(), &o, &t, o.position, 0.0);
        assert!(labels(&unturned).iter().all(|l| l.starts_with('1')), "{unturned:?}");

        let mut e = Struct::new(7);
        e.set("Geometry", Value::List(vec![point(["X", "Y"], 0.0, 3.0)]));
        let mut spawn = point(["X", "Y"], 3.0, 1.0);
        spawn.set("Orientation", Value::Float(0.5));
        e.set("SpawnPointList", Value::List(vec![spawn]));
        let o = object(ObjectKind::Encounter, Vec3::new(1.0, 1.0, 0.0), 0.0);
        let turned = moved(&o, &e, o.position, FRAC_PI_2);
        let p = &turned.list("Geometry").unwrap()[0];
        assert!((p.float("X").unwrap() + 3.0).abs() < 1e-5 && p.float("Y").unwrap().abs() < 1e-5);
        let s = &turned.list("SpawnPointList").unwrap()[0];
        let (x, y) = (s.float("X").unwrap(), s.float("Y").unwrap());
        assert!((x - 1.0).abs() < 1e-5 && (y - 3.0).abs() < 1e-5, "{x}, {y}");
        assert!((s.float("Orientation").unwrap() - 0.5 - FRAC_PI_2).abs() < 1e-5);
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
