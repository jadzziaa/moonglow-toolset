//! Blueprints placed in areas: the instance (a GIT list entry) of a
//! blueprint is its fields, less the palette's, where it stands.
//!
//! Doors, triggers, encounters, sounds and waypoints so far; creatures,
//! placeables, stores and items also carry their inventories inline, which
//! the area editor will expand (Phase 9).

use mg_core::ResType;
use mg_gff::{Struct, Value};

/// Where an instance stands: a position in metres and a facing in radians
/// (counter-clockwise from east).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Placement {
    pub position: [f32; 3],
    pub facing: f32,
}

/// The GIT list a blueprint type's instances go in, and their struct id.
pub fn git_list(restype: ResType) -> Option<(&'static str, u32)> {
    Some(match restype {
        ResType::UTC => ("Creature List", 4),
        ResType::UTD => ("Door List", 8),
        ResType::UTE => ("Encounter List", 7),
        ResType::UTI => ("List", 0),
        ResType::UTP => ("Placeable List", 9),
        ResType::UTS => ("SoundList", 6),
        ResType::UTM => ("StoreList", 11),
        ResType::UTT => ("TriggerList", 1),
        ResType::UTW => ("WaypointList", 5),
        _ => return None,
    })
}

/// Fields only blueprints have (the palette category).
const BLUEPRINT_ONLY: [&str; 2] = ["PaletteID", "ID"];

/// An instance of a blueprint of `restype` at `at`: for triggers and
/// encounters, `polygon` gives their outline (relative to the position);
/// encounters spawn at their position.
pub fn instance(
    restype: ResType,
    blueprint: &Struct,
    at: Placement,
    polygon: &[[f32; 2]],
) -> Option<Struct> {
    let (_, id) = git_list(restype)?;
    let mut s = blueprint.clone();
    s.id = id;
    for label in BLUEPRINT_ONLY {
        s.remove(label);
    }
    let [x, y, z] = at.position;
    let f = Value::Float;
    match restype {
        ResType::UTD => {
            for (label, v) in [("X", x), ("Y", y), ("Z", z), ("Bearing", at.facing)] {
                s.set(label, f(v));
            }
        }
        ResType::UTS => {
            for (label, v) in [("XPosition", x), ("YPosition", y), ("ZPosition", z)] {
                s.set(label, f(v));
            }
            s.set("GeneratedType", Value::Dword(0));
        }
        ResType::UTW => {
            for (label, v) in [
                ("XPosition", x),
                ("YPosition", y),
                ("ZPosition", z),
                ("XOrientation", at.facing.cos()),
                ("YOrientation", at.facing.sin()),
            ] {
                s.set(label, f(v));
            }
        }
        ResType::UTT => {
            for (label, v) in [
                ("XPosition", x),
                ("YPosition", y),
                ("ZPosition", z),
                ("XOrientation", 0.0),
                ("YOrientation", 0.0),
                ("ZOrientation", 0.0),
            ] {
                s.set(label, f(v));
            }
            let points = polygon
                .iter()
                .map(|[px, py]| {
                    let mut p = Struct::new(3);
                    p.set("PointX", f(*px));
                    p.set("PointY", f(*py));
                    p.set("PointZ", f(0.0));
                    p
                })
                .collect();
            s.set("Geometry", Value::List(points));
        }
        ResType::UTE => {
            for (label, v) in [("XPosition", x), ("YPosition", y), ("ZPosition", z)] {
                s.set(label, f(v));
            }
            let points = polygon
                .iter()
                .map(|[px, py]| {
                    let mut p = Struct::new(1);
                    p.set("X", f(*px));
                    p.set("Y", f(*py));
                    p.set("Z", f(0.0));
                    p
                })
                .collect();
            s.set("Geometry", Value::List(points));
            let mut spawn = Struct::new(2);
            for (label, v) in [("X", x), ("Y", y), ("Z", z), ("Orientation", at.facing)] {
                spawn.set(label, f(v));
            }
            s.set("SpawnPointList", Value::List(vec![spawn]));
        }
        _ => return None,
    }
    Some(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instances_stand_where_placed() {
        let mut bp = Struct::new(0);
        bp.set("Tag", Value::String(b"DOOR".to_vec()));
        bp.set("PaletteID", Value::Byte(3));
        let at = Placement { position: [1.0, 2.0, 0.5], facing: 1.5 };
        let d = instance(ResType::UTD, &bp, at, &[]).unwrap();
        assert_eq!(d.id, 8);
        assert_eq!((d.float("X"), d.float("Bearing")), (Some(1.0), Some(1.5)));
        assert!(!d.contains("PaletteID"));
        assert_eq!(d.string("Tag"), Some(&b"DOOR"[..]));
        let t = instance(ResType::UTT, &bp, at, &[[0.0, 0.0], [2.0, 0.0], [2.0, 2.0]]).unwrap();
        let g = t.list("Geometry").unwrap();
        assert_eq!((g.len(), g[1].id, g[1].float("PointX")), (3, 3, Some(2.0)));
        let e = instance(ResType::UTE, &bp, at, &[[0.0, 0.0], [1.0, 1.0]]).unwrap();
        assert_eq!(e.list("SpawnPointList").unwrap()[0].float("Orientation"), Some(1.5));
        let w = instance(ResType::UTW, &bp, Placement::default(), &[]).unwrap();
        assert_eq!(w.float("XOrientation"), Some(1.0));
        assert!(instance(ResType::UTC, &bp, at, &[]).is_none(), "creatures: not yet");
    }
}
