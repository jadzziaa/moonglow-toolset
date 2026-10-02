//! Arranging placed objects: snapping to a grid and to angles, lining up,
//! spacing out, matching facings and mirroring. Positions are on the ground
//! plane (x, y); rotations are the model's turn around Z (its facing less
//! 90°), as in [`AreaObject`](crate::AreaObject).

use std::f32::consts::PI;

use glam::Vec2;

/// `v` to the nearest multiple of `step` (unchanged without a step).
pub fn snap(v: f32, step: Option<f32>) -> f32 {
    match step.filter(|s| *s > 0.0) {
        Some(s) => (v / s).round() * s,
        None => v,
    }
}

/// A point to the nearest grid crossing.
pub fn snap_point(p: Vec2, step: Option<f32>) -> Vec2 {
    Vec2::new(snap(p.x, step), snap(p.y, step))
}

/// A rotation (radians) to the nearest multiple of `step` degrees of
/// facing.
pub fn snap_rotation(rotation: f32, step_degrees: Option<f32>) -> f32 {
    snap(rotation, step_degrees.map(f32::to_radians))
}

/// How to arrange a selection; the first object chosen is the one the
/// others follow.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arrange {
    /// On a west–east line through the first (its y).
    LineUpWestEast,
    /// On a south–north line through the first (its x).
    LineUpSouthNorth,
    /// Evenly spaced between the two farthest apart, in their order along
    /// that line.
    Distribute,
    /// Facing as the first does.
    SameFacing,
    /// Mirrored west to east about the selection's middle.
    MirrorWestEast,
    /// Mirrored south to north about the selection's middle.
    MirrorSouthNorth,
}

/// The objects' new places and rotations.
pub fn arrange(how: Arrange, objects: &[(Vec2, f32)]) -> Vec<(Vec2, f32)> {
    let Some(&(first, first_rot)) = objects.first() else { return Vec::new() };
    let (min, max) =
        objects.iter().fold((first, first), |(lo, hi), (p, _)| (lo.min(*p), hi.max(*p)));
    let middle = (min + max) / 2.0;
    match how {
        Arrange::LineUpWestEast => {
            objects.iter().map(|&(p, r)| (Vec2::new(p.x, first.y), r)).collect()
        }
        Arrange::LineUpSouthNorth => {
            objects.iter().map(|&(p, r)| (Vec2::new(first.x, p.y), r)).collect()
        }
        Arrange::SameFacing => objects.iter().map(|&(p, _)| (p, first_rot)).collect(),
        Arrange::MirrorWestEast => {
            objects.iter().map(|&(p, r)| (Vec2::new(2.0 * middle.x - p.x, p.y), wrap(-r))).collect()
        }
        Arrange::MirrorSouthNorth => objects
            .iter()
            .map(|&(p, r)| (Vec2::new(p.x, 2.0 * middle.y - p.y), wrap(-r - PI)))
            .collect(),
        Arrange::Distribute => {
            if objects.len() < 3 {
                return objects.to_vec();
            }
            // The two farthest apart are the ends; the others keep their
            // order along the line between them.
            let mut ends = (0, 1);
            for i in 0..objects.len() {
                for j in i + 1..objects.len() {
                    let d = objects[i].0.distance_squared(objects[j].0);
                    if d > objects[ends.0].0.distance_squared(objects[ends.1].0) {
                        ends = (i, j);
                    }
                }
            }
            let (a, b) = (objects[ends.0].0, objects[ends.1].0);
            let along = (b - a).normalize_or_zero();
            let mut order: Vec<usize> = (0..objects.len()).collect();
            order.sort_by(|&i, &j| {
                (objects[i].0 - a).dot(along).total_cmp(&(objects[j].0 - a).dot(along))
            });
            let mut out = objects.to_vec();
            let n = (objects.len() - 1) as f32;
            for (k, &i) in order.iter().enumerate() {
                out[i].0 = a + (b - a) * (k as f32 / n);
            }
            out
        }
    }
}

/// An angle into (-π, π].
fn wrap(a: f32) -> f32 {
    let w = (a + PI).rem_euclid(2.0 * PI) - PI;
    if w <= -PI { w + 2.0 * PI } else { w }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: &[(Vec2, f32)], b: &[(Vec2, f32)]) -> bool {
        a.len() == b.len()
            && a.iter()
                .zip(b)
                .all(|((p, r), (q, s))| p.distance(*q) < 1e-4 && (wrap(*r - *s)).abs() < 1e-4)
    }

    #[test]
    fn snapping() {
        assert_eq!(snap(1.37, Some(0.5)), 1.5);
        assert_eq!(snap(1.37, None), 1.37);
        assert_eq!(snap_point(Vec2::new(2.4, -2.6), Some(1.0)), Vec2::new(2.0, -3.0));
        assert!((snap_rotation(0.3, Some(15.0)) - 15f32.to_radians()).abs() < 1e-6);
    }

    #[test]
    fn lining_up_and_spacing_out() {
        let objs =
            [(Vec2::new(0.0, 1.0), 0.0), (Vec2::new(4.0, 3.0), 1.0), (Vec2::new(10.0, -2.0), 2.0)];
        assert!(close(
            &arrange(Arrange::LineUpWestEast, &objs),
            &[(Vec2::new(0.0, 1.0), 0.0), (Vec2::new(4.0, 1.0), 1.0), (Vec2::new(10.0, 1.0), 2.0)]
        ));
        assert!(close(
            &arrange(Arrange::SameFacing, &objs),
            &[(objs[0].0, 0.0), (objs[1].0, 0.0), (objs[2].0, 0.0)]
        ));
        let line = [
            (Vec2::new(0.0, 0.0), 0.0),
            (Vec2::new(9.0, 0.0), 0.0),
            (Vec2::new(1.0, 0.0), 0.0),
            (Vec2::new(4.0, 0.0), 0.0),
        ];
        let spaced: Vec<f32> =
            arrange(Arrange::Distribute, &line).iter().map(|(p, _)| p.x).collect();
        assert_eq!(spaced, [0.0, 9.0, 3.0, 6.0]);
    }

    #[test]
    fn mirroring_turns_facings() {
        // An object facing east (rotation -90°) on the west side faces west
        // on the east side after a west–east mirror.
        let objs = [(Vec2::new(0.0, 0.0), -PI / 2.0), (Vec2::new(10.0, 0.0), 0.0)];
        let m = arrange(Arrange::MirrorWestEast, &objs);
        assert!(close(&m, &[(Vec2::new(10.0, 0.0), PI / 2.0), (Vec2::new(0.0, 0.0), 0.0)]));
        // Facing north (rotation 0) mirrored south to north faces south.
        let m = arrange(
            Arrange::MirrorSouthNorth,
            &[(Vec2::new(0.0, 0.0), 0.0), (Vec2::new(0.0, 4.0), 0.0)],
        );
        assert!(close(&m, &[(Vec2::new(0.0, 4.0), PI), (Vec2::new(0.0, 0.0), PI)]));
    }
}
