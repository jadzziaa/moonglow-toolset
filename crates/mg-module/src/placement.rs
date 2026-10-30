//! Where a placed object stands, read and written one way: its position
//! and facing in the fields its kind keeps them in (doors and placeables
//! `X`, `Y`, `Z` and `Bearing`; the others `XPosition`… and, those that
//! face a way, `XOrientation` and `YOrientation`), a trigger's or an
//! encounter's outline (`Geometry`: `PointX`… or `X`…), and its visual
//! transform (EE's `VisTransformList`, or the older `VisualTransform`).
//! The area view, Update Instances, Resize Area, `mg find`, Add to
//! Palette, the locks, Setup Store and the area's sounds read them here.

use std::f32::consts::FRAC_PI_2;

use mg_gff::Struct;
use mg_rules::ObjectKind;

/// Where an instance stands: a position in metres and its model's turn
/// around Z in radians (its facing less 90°: 0 faces north, as Aurora
/// places everything).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Placement {
    pub position: [f32; 3],
    pub rotation: f32,
}

/// The fields that hold where an object of `kind` stands.
pub fn position_fields(kind: ObjectKind) -> [&'static str; 3] {
    if kind.has_bearing() { ["X", "Y", "Z"] } else { ["XPosition", "YPosition", "ZPosition"] }
}

/// Whether an object of `kind` faces a way of its own: doors and
/// placeables by `Bearing`, creatures, items, stores and waypoints by
/// their orientation. Sounds, triggers and encounters don't (a trigger's
/// and an encounter's outline turns instead).
pub fn faces(kind: ObjectKind) -> bool {
    !(kind.has_outline() || kind == ObjectKind::Sound)
}

/// The fields of an outline's points (`Geometry`) of `kind`, for the
/// kinds that have one.
pub fn point_fields(kind: ObjectKind) -> Option<[&'static str; 3]> {
    match kind {
        ObjectKind::Trigger => Some(["PointX", "PointY", "PointZ"]),
        ObjectKind::Encounter => Some(["X", "Y", "Z"]),
        _ => None,
    }
}

/// Every field that says where an object of `kind` stands, as Moonglow
/// and Aurora write a placed one: its position, its facing (`Bearing`, or
/// the orientation's; a trigger's three, which it keeps at 0) and its
/// outline.
pub fn fields_of(kind: ObjectKind) -> Vec<&'static str> {
    let mut out = position_fields(kind).to_vec();
    match kind {
        _ if kind.has_bearing() => out.push("Bearing"),
        ObjectKind::Trigger => out.extend(["XOrientation", "YOrientation", "ZOrientation"]),
        _ if faces(kind) => out.extend(["XOrientation", "YOrientation"]),
        _ => {}
    }
    if kind.has_outline() {
        out.push("Geometry");
    }
    out
}

impl Placement {
    /// Where the placed object `s` of `kind` stands. A kind that faces
    /// no way, or an orientation of none (0, 0), turns by 0.
    pub fn read(kind: ObjectKind, s: &Struct) -> Placement {
        let f = |l: &str| s.float(l).unwrap_or(0.0);
        let position = position_fields(kind).map(f);
        let rotation = if kind.has_bearing() {
            f("Bearing")
        } else {
            let (x, y) = (f("XOrientation"), f("YOrientation"));
            if !faces(kind) || (x == 0.0 && y == 0.0) { 0.0 } else { y.atan2(x) - FRAC_PI_2 }
        };
        Placement { position, rotation }
    }

    /// The fields it is written as for an object of `kind` (as
    /// [`Placement::read`] reads them back): the position, and the
    /// facing of a kind that faces a way.
    pub fn fields(&self, kind: ObjectKind) -> Vec<(&'static str, f32)> {
        let mut out: Vec<(&'static str, f32)> =
            position_fields(kind).into_iter().zip(self.position).collect();
        if kind.has_bearing() {
            out.push(("Bearing", self.rotation));
        } else if faces(kind) {
            let facing = self.rotation + FRAC_PI_2;
            out.extend([("XOrientation", facing.cos()), ("YOrientation", facing.sin())]);
        }
        out
    }
}

/// The outline's points of the placed object `s` of `kind`, as stored
/// (relative to its position); none for a kind without one.
pub fn outline(kind: ObjectKind, s: &Struct) -> Vec<[f32; 3]> {
    let Some(labels) = point_fields(kind) else { return Vec::new() };
    let points = s.list("Geometry").unwrap_or(&[]);
    points.iter().map(|p| labels.map(|l| p.float(l).unwrap_or(0.0))).collect()
}

/// A visual transform's fields as the file has them (`ScaleX`…
/// `TranslateZ`; rotations in degrees). What they do to the model is
/// `mg_area::VisualTransform::matrix`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transform {
    pub scale: [f32; 3],
    pub rotate: [f32; 3],
    pub translate: [f32; 3],
}

impl Default for Transform {
    fn default() -> Transform {
        Transform { scale: [1.0; 3], rotate: [0.0; 3], translate: [0.0; 3] }
    }
}

/// EE's list of a placed object's visual transforms, and the older
/// struct.
pub const VISUAL_TRANSFORM_FIELDS: [&str; 2] = ["VisTransformList", "VisualTransform"];

impl Transform {
    /// From its fields, `get` reading each (`None`: the field's rest).
    fn from(get: &dyn Fn(&str) -> Option<f32>) -> Transform {
        let axes = |prefix: &str, rest: f32| {
            ["X", "Y", "Z"].map(|a| get(&format!("{prefix}{a}")).unwrap_or(rest))
        };
        Transform {
            scale: axes("Scale", 1.0),
            rotate: axes("Rotate", 0.0),
            translate: axes("Translate", 0.0),
        }
    }

    /// An entry of `VisTransformList`: each field a struct whose
    /// `ValueTo` is where it ends (the engine's lerp's goal).
    fn of_entry(entry: &Struct) -> Transform {
        Transform::from(&|label| entry.child(label).and_then(|c| c.float("ValueTo")))
    }

    /// The older `VisualTransform` struct: its fields plain floats.
    fn of_struct(legacy: &Struct) -> Transform {
        Transform::from(&|label| legacy.float(label))
    }

    pub fn is_identity(&self) -> bool {
        *self == Transform::default()
    }
}

/// The visual transform the placed object `s`'s model is drawn with:
/// `VisTransformList`'s entry for scope 0 (an entry with no `Scope` is
/// of 0); without the list, the older `VisualTransform`. `None` without
/// either, or with a list of other scopes only: the client draws such a
/// placeable untransformed (`reading_visual_transform_without_scope_0` in
/// `client_render.rs`, 2026-10-29: a scope 1 or 4 entry alone changes
/// nothing), and the engine reads a creature's scope 0 so when its head
/// alone is scaled (`the_engine_keeps_part_scopes_animation_speed_and_lerps`).
pub fn visual_transform(s: &Struct) -> Option<Transform> {
    if let Some(list) = s.list("VisTransformList") {
        let entry = list.iter().find(|e| e.integer("Scope").unwrap_or(0) == 0)?;
        return Some(Transform::of_entry(entry));
    }
    s.child("VisualTransform").map(Transform::of_struct)
}

/// A creature's parts a visual transform can be given apart from the rest
/// (`OBJECT_VISUAL_TRANSFORM_DATA_SCOPE_CREATURE_*`): its head, tail,
/// wings and cloak, by scope.
pub const CREATURE_SCOPES: [(i64, &str); 4] =
    [(254, "head"), (253, "tail"), (252, "wings"), (243, "cloak")];

/// What a scope is (`kind` the object's: the same numbers name an item's
/// parts, `OBJECT_VISUAL_TRANSFORM_DATA_SCOPE_ITEM_PART1` 255 to 5 251).
pub fn scope_name(kind: ObjectKind, scope: i64) -> String {
    match (kind, scope) {
        (_, 0) => "the whole object".to_string(),
        (ObjectKind::Creature, s) => match CREATURE_SCOPES.iter().find(|(n, _)| *n == s) {
            Some((_, name)) => format!("its {name}"),
            None => format!("scope {s}"),
        },
        (ObjectKind::Item, s @ 251..=255) => format!("its part {}", 256 - s),
        (_, s) => format!("scope {s}"),
    }
}

/// The axes of a `VisTransformList` entry, in the order Aurora writes
/// them.
pub const AXES: [&str; 10] = [
    "AnimationSpeed",
    "ScaleX",
    "ScaleY",
    "ScaleZ",
    "RotateX",
    "RotateY",
    "RotateZ",
    "TranslateX",
    "TranslateY",
    "TranslateZ",
];

/// An axis's lerp in progress (`SetObjectVisualTransform` with a
/// `LerpType` and a duration, kept in a saved game or a module made from
/// one): from where, to where, over how long and how far along.
#[derive(Debug, Clone, PartialEq)]
pub struct Lerp {
    /// The axis ([`AXES`]).
    pub axis: &'static str,
    pub from: f32,
    pub to: f32,
    /// Seconds, as the script gave it.
    pub duration: f32,
    /// How far along it is, as the engine counts it (`LerpProgress`).
    pub progress: f32,
    /// `OBJECT_VISUAL_TRANSFORM_LERP_*` (1 linear, 2 smoothstep…).
    pub kind: i64,
    /// `OBJECT_VISUAL_TRANSFORM_BEHAVIOR_*` (1: bounce between the two).
    pub behavior: i64,
    /// Times it is to play again (`RepeatsRemaining`; -1: for ever).
    pub repeats: i64,
}

/// One entry of `VisTransformList`: its scope, its transform (each axis's
/// goal), its animation speed and the lerps it has in progress.
#[derive(Debug, Clone, PartialEq)]
pub struct Scoped {
    pub scope: i64,
    pub transform: Transform,
    /// `AnimationSpeed`'s goal: how fast the model's animations play.
    pub speed: f32,
    pub lerps: Vec<Lerp>,
}

/// A number field of whatever type the file has it as.
fn number(s: &Struct, label: &str) -> Option<f32> {
    s.float(label).or_else(|| s.integer(label).map(|n| n as f32))
}

/// Every entry of the placed object `s`'s `VisTransformList`, in its
/// order: each scope's transform, speed and lerps in progress (an axis
/// with a `ValueFrom` or a `LerpDuration`).
pub fn visual_scopes(s: &Struct) -> Vec<Scoped> {
    let list = s.list("VisTransformList").unwrap_or(&[]);
    list.iter()
        .map(|e| {
            let lerps = AXES
                .iter()
                .filter_map(|axis| {
                    let a = e.child(axis)?;
                    if !a.contains("ValueFrom") && !a.contains("LerpDuration") {
                        return None;
                    }
                    let n = |label: &str| number(a, label);
                    Some(Lerp {
                        axis,
                        from: n("ValueFrom").unwrap_or(0.0),
                        to: n("ValueTo").unwrap_or(0.0),
                        duration: n("LerpDuration").unwrap_or(0.0),
                        progress: n("LerpProgress").unwrap_or(0.0),
                        kind: a.integer("LerpType").unwrap_or(0),
                        behavior: a.integer("BehaviorFlags").unwrap_or(0),
                        repeats: a.integer("RepeatsRemaining").unwrap_or(0),
                    })
                })
                .collect();
            Scoped {
                scope: e.integer("Scope").unwrap_or(0),
                transform: Transform::of_entry(e),
                speed: e.child("AnimationSpeed").and_then(|a| number(a, "ValueTo")).unwrap_or(1.0),
                lerps,
            }
        })
        .collect()
}

/// The transform of scope `scope` of the placed object `s` (a creature's
/// head, 254…), where its list has one.
pub fn scope_transform(s: &Struct, scope: i64) -> Option<Transform> {
    let list = s.list("VisTransformList")?;
    let entry = list.iter().find(|e| e.integer("Scope").unwrap_or(0) == scope)?;
    Some(Transform::of_entry(entry))
}

/// Whether any of the placed object `s`'s visual transforms changes how
/// its model shows: an entry of any scope, or the older struct (what
/// Make Placeables Static and Verify's `static-transform` ask: a change
/// the game would lose, whichever part it is of).
pub fn is_transformed(s: &Struct) -> bool {
    let listed = s.list("VisTransformList").unwrap_or(&[]).iter().map(Transform::of_entry);
    let legacy = s.child("VisualTransform").map(Transform::of_struct);
    listed.chain(legacy).any(|t| !t.is_identity())
}

#[cfg(test)]
mod tests {
    use super::*;
    use mg_gff::Value;

    fn object(fields: &[(&str, f32)]) -> Struct {
        let mut s = Struct::new(0);
        for (label, v) in fields {
            s.set(label, Value::Float(*v));
        }
        s
    }

    /// What is written reads back as it was, for every kind: its position,
    /// and the facing of the kinds that face a way (the others turn by 0).
    #[test]
    fn a_placement_reads_back_as_written_for_every_kind() {
        let at = Placement { position: [1.5, -2.0, 0.25], rotation: 0.7 };
        for kind in ObjectKind::ALL {
            let fields = at.fields(kind);
            let s = object(&fields);
            let back = Placement::read(kind, &s);
            assert_eq!(back.position, at.position, "{kind:?}");
            let turn = if faces(kind) { at.rotation } else { 0.0 };
            assert!((back.rotation - turn).abs() < 1e-5, "{kind:?}: {}", back.rotation);
            // Each field it writes is one of those that say where it stands.
            let all = fields_of(kind);
            assert!(fields.iter().all(|(l, _)| all.contains(l)), "{kind:?}");
        }
        assert_eq!(position_fields(ObjectKind::Door), ["X", "Y", "Z"]);
        assert_eq!(position_fields(ObjectKind::Sound)[0], "XPosition");
        // A waypoint facing north (orientation 0, 1) turns by 0; one of no
        // orientation at all, too.
        let w = object(&[("XOrientation", 0.0), ("YOrientation", 1.0)]);
        assert!(Placement::read(ObjectKind::Waypoint, &w).rotation.abs() < 1e-6);
        assert_eq!(Placement::read(ObjectKind::Waypoint, &Struct::new(0)).rotation, 0.0);
        // A trigger's orientation, whatever it holds, doesn't turn it.
        let t = object(&[("XOrientation", 1.0), ("YOrientation", 0.0)]);
        assert_eq!(Placement::read(ObjectKind::Trigger, &t).rotation, 0.0);
    }

    #[test]
    fn an_outline_is_read_by_its_kind_s_point_fields() {
        let point = |labels: [&str; 3], v: [f32; 3]| {
            let mut p = Struct::new(3);
            for (l, v) in labels.into_iter().zip(v) {
                p.set(l, Value::Float(v));
            }
            p
        };
        let mut t = Struct::new(1);
        t.set(
            "Geometry",
            Value::List(vec![point(["PointX", "PointY", "PointZ"], [1.0, 2.0, 0.0])]),
        );
        assert_eq!(outline(ObjectKind::Trigger, &t), [[1.0, 2.0, 0.0]]);
        let mut e = Struct::new(7);
        e.set("Geometry", Value::List(vec![point(["X", "Y", "Z"], [3.0, 4.0, 0.0])]));
        assert_eq!(outline(ObjectKind::Encounter, &e), [[3.0, 4.0, 0.0]]);
        assert!(outline(ObjectKind::Placeable, &e).is_empty());
    }

    fn entry(scope: Option<i32>, scale: f32) -> Struct {
        let mut e = Struct::new(0);
        if let Some(s) = scope {
            e.set("Scope", Value::Int(s));
        }
        let mut v = Struct::new(0);
        v.set("ValueTo", Value::Float(scale));
        e.set("ScaleX", Value::Struct(v));
        e
    }

    /// Every scope of the list is read: a creature's head with its scale,
    /// the base with its animation speed and a lerp in progress (`ValueFrom`
    /// and `LerpDuration` beside the goal), as the engine saves one.
    #[test]
    fn every_scope_speed_and_lerp_is_read() {
        let mut s = Struct::new(0);
        assert!(visual_scopes(&s).is_empty());
        let mut base = entry(None, 1.0);
        let mut speed = Struct::new(0);
        speed.set("ValueTo", Value::Float(2.5));
        base.set("AnimationSpeed", Value::Struct(speed));
        let mut lerp = Struct::new(0);
        for (label, v) in [("ValueFrom", 1.0), ("ValueTo", 3.0), ("LerpDuration", 600.0)] {
            lerp.set(label, Value::Float(v));
        }
        lerp.set("LerpProgress", Value::Float(0.25));
        lerp.set("LerpType", Value::Int(1));
        lerp.set("RepeatsRemaining", Value::Int(-1));
        base.set("RotateX", Value::Struct(lerp));
        s.set("VisTransformList", Value::List(vec![base, entry(Some(254), 2.0)]));
        let scopes = visual_scopes(&s);
        assert_eq!(scopes.len(), 2);
        assert_eq!((scopes[0].scope, scopes[0].speed), (0, 2.5));
        assert_eq!(scopes[0].transform.rotate[0], 3.0, "the goal");
        let l = &scopes[0].lerps;
        assert_eq!(l.len(), 1);
        assert_eq!((l[0].axis, l[0].from, l[0].to, l[0].duration), ("RotateX", 1.0, 3.0, 600.0));
        assert_eq!((l[0].progress, l[0].kind, l[0].repeats), (0.25, 1, -1));
        assert_eq!(
            (scopes[1].scope, scopes[1].speed, scopes[1].transform.scale[0]),
            (254, 1.0, 2.0)
        );
        assert_eq!(scope_name(ObjectKind::Creature, 254), "its head");
        assert_eq!(scope_name(ObjectKind::Item, 254), "its part 2");
        assert_eq!(scope_name(ObjectKind::Placeable, 7), "scope 7");
    }

    /// One reader, two questions: the transform the model is drawn with
    /// (scope 0's, else none where there is a list, else the older
    /// struct), and
    /// whether any changes how it shows (any scope's, or the struct).
    #[test]
    fn visual_transforms_are_read_one_way() {
        let mut s = Struct::new(0);
        assert_eq!(visual_transform(&s), None);
        assert!(!is_transformed(&s));
        // A head's scale (scope 1) and the base's (no scope: 0).
        s.set("VisTransformList", Value::List(vec![entry(Some(1), 2.0), entry(None, 1.5)]));
        assert_eq!(visual_transform(&s).unwrap().scale[0], 1.5, "scope 0's");
        assert!(is_transformed(&s));
        // Only another scope's: the whole object untransformed, as the
        // client draws a placeable so and the engine reads it (a part of
        // it is transformed by its own scope).
        s.set("VisTransformList", Value::List(vec![entry(Some(4), 2.0)]));
        assert_eq!(visual_transform(&s), None, "no scope 0");
        s.set("VisTransformList", Value::List(vec![entry(Some(1), 2.0)]));
        assert_eq!(visual_transform(&s), None, "a head's alone");
        s.set("VisTransformList", Value::List(vec![entry(Some(254), 2.0)]));
        assert_eq!(visual_transform(&s), None, "scope 0 has none");
        assert_eq!(scope_transform(&s, 254).unwrap().scale[0], 2.0);
        assert_eq!(scope_transform(&s, 253), None);
        assert!(is_transformed(&s));
        // An identity list beside an older struct that scales: drawn by the
        // list (as before), but transformed.
        let mut legacy = Struct::new(6);
        legacy.set("ScaleX", Value::Float(3.0));
        s.set("VisTransformList", Value::List(vec![entry(None, 1.0)]));
        s.set("VisualTransform", Value::Struct(legacy.clone()));
        assert!(visual_transform(&s).unwrap().is_identity());
        assert!(is_transformed(&s));
        // The older struct alone.
        let mut old = Struct::new(0);
        old.set("VisualTransform", Value::Struct(legacy));
        assert_eq!(visual_transform(&old).unwrap().scale, [3.0, 1.0, 1.0]);
    }
}
