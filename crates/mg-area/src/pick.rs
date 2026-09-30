//! Finding what is under the pointer: rays from the camera through the
//! view, against objects' boxes and triggers' and encounters' outlines.

use glam::{Mat4, Vec2, Vec3};
use mg_render::Camera;

use crate::{AreaModel, ObjectKind};

/// A ray from `origin` along `dir` (unit length).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ray {
    pub origin: Vec3,
    pub dir: Vec3,
}

impl Ray {
    /// The ray through a point of the view: `at` from (0, 0) at the
    /// top-left to (1, 1) at the bottom-right; `aspect` is width / height.
    pub fn from_screen(camera: &Camera, aspect: f32, at: Vec2) -> Ray {
        let inverse = (camera.projection(aspect) * camera.view()).inverse();
        let ndc = Vec2::new(at.x * 2.0 - 1.0, 1.0 - at.y * 2.0);
        let near = inverse.project_point3(ndc.extend(0.0));
        let far = inverse.project_point3(ndc.extend(1.0));
        Ray { origin: near, dir: (far - near).normalize_or(Vec3::NEG_Z) }
    }

    pub fn at(&self, t: f32) -> Vec3 {
        self.origin + self.dir * t
    }

    /// Where the ray crosses the horizontal plane at height `z` (ahead of
    /// its origin).
    pub fn at_height(&self, z: f32) -> Option<Vec3> {
        if self.dir.z.abs() < 1e-6 {
            return None;
        }
        let t = (z - self.origin.z) / self.dir.z;
        (t >= 0.0).then(|| self.at(t))
    }

    /// The distance to a box (`min`, `max` in the space `transform` places),
    /// if the ray meets it.
    pub fn hits_box(&self, min: Vec3, max: Vec3, transform: Mat4) -> Option<f32> {
        let inverse = transform.inverse();
        let origin = inverse.transform_point3(self.origin);
        let dir = inverse.transform_vector3(self.dir);
        let (mut near, mut far) = (f32::MIN, f32::MAX);
        for axis in 0..3 {
            if dir[axis].abs() < 1e-9 {
                if origin[axis] < min[axis] || origin[axis] > max[axis] {
                    return None;
                }
                continue;
            }
            let a = (min[axis] - origin[axis]) / dir[axis];
            let b = (max[axis] - origin[axis]) / dir[axis];
            near = near.max(a.min(b));
            far = far.min(a.max(b));
        }
        // The transform may scale: distances are the same along the ray
        // only when it does not, which placements never do.
        (near <= far && far >= 0.0).then_some(near.max(0.0))
    }

    /// The distance to a flat outline (at its points' mean height), if the
    /// ray meets it inside.
    pub fn hits_outline(&self, points: &[Vec3]) -> Option<f32> {
        if points.len() < 3 {
            return None;
        }
        let z = points.iter().map(|p| p.z).sum::<f32>() / points.len() as f32;
        let hit = self.at_height(z)?;
        inside(points, hit.truncate()).then(|| (hit - self.origin).length())
    }
}

/// Whether `p` is inside the polygon `points` (in the XY plane; even-odd).
pub fn inside(points: &[Vec3], p: Vec2) -> bool {
    let mut inside = false;
    let mut j = points.len() - 1;
    for i in 0..points.len() {
        let (a, b) = (points[i], points[j]);
        if (a.y > p.y) != (b.y > p.y) && p.x < (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x {
            inside = !inside;
        }
        j = i;
    }
    inside
}

/// Where a point shows in the view (0..1 from the top-left), if it is in
/// front of the camera.
pub fn project(camera: &Camera, aspect: f32, p: Vec3) -> Option<Vec2> {
    let clip = camera.projection(aspect) * camera.view() * p.extend(1.0);
    if clip.w <= 1e-6 {
        return None;
    }
    let ndc = clip.truncate() / clip.w;
    Some(Vec2::new((ndc.x + 1.0) * 0.5, (1.0 - ndc.y) * 0.5))
}

/// The box drawn and picked for an object without a model: waypoints,
/// sounds and stores, and objects whose appearance cannot be shown.
pub fn marker_bounds(kind: ObjectKind) -> (Vec3, Vec3) {
    match kind {
        ObjectKind::Waypoint => (Vec3::new(-0.3, -0.3, 0.0), Vec3::new(0.3, 0.3, 1.8)),
        ObjectKind::Sound => (Vec3::new(-0.4, -0.4, 0.0), Vec3::new(0.4, 0.4, 0.8)),
        _ => (Vec3::new(-0.5, -0.5, 0.0), Vec3::new(0.5, 0.5, 1.0)),
    }
}

/// The nearest object `ray` meets: objects by their boxes (`bounds` gives
/// object `i`'s, in its own space), triggers and encounters by their
/// outlines. Only objects `shown` count.
pub fn pick(
    area: &AreaModel,
    ray: &Ray,
    bounds: &dyn Fn(usize) -> (Vec3, Vec3),
    shown: &dyn Fn(ObjectKind) -> bool,
) -> Option<usize> {
    let mut best: Option<(f32, usize)> = None;
    for (i, o) in area.objects.iter().enumerate() {
        if !shown(o.kind) {
            continue;
        }
        let t = if o.kind.has_outline() {
            ray.hits_outline(&o.outline)
        } else {
            let (min, max) = bounds(i);
            ray.hits_box(min, max, o.transform())
        };
        if let Some(t) = t {
            // Outlines lie on the ground: whatever stands on them is nearer.
            if best.is_none_or(|(b, _)| t < b) {
                best = Some((t, i));
            }
        }
    }
    best.map(|(_, i)| i)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: Vec3, b: Vec3) -> bool {
        (a - b).abs().max_element() < 1e-3
    }

    #[test]
    fn rays_through_the_view_centre_meet_the_target() {
        let camera = Camera::orbit(Vec3::new(10.0, 20.0, 0.0), 30.0, 0.7, 0.9);
        let ray = Ray::from_screen(&camera, 1.5, Vec2::splat(0.5));
        let ground = ray.at_height(0.0).unwrap();
        assert!(close(ground, Vec3::new(10.0, 20.0, 0.0)), "{ground}");
        let back = project(&camera, 1.5, Vec3::new(10.0, 20.0, 0.0)).unwrap();
        assert!((back - Vec2::splat(0.5)).length() < 1e-4);
        // A point projects where its ray starts.
        let p = Vec3::new(14.0, 18.0, 1.0);
        let at = project(&camera, 1.5, p).unwrap();
        let r = Ray::from_screen(&camera, 1.5, at);
        let t = (p - r.origin).dot(r.dir);
        assert!(close(r.at(t), p));
    }

    #[test]
    fn boxes_are_met_in_their_own_space() {
        let ray = Ray { origin: Vec3::new(0.0, 0.0, 10.0), dir: Vec3::NEG_Z };
        let turned = Mat4::from_rotation_z(0.8) * Mat4::from_translation(Vec3::ZERO);
        let t = ray.hits_box(Vec3::new(-1.0, -1.0, 0.0), Vec3::ONE, turned).unwrap();
        assert!((t - 9.0).abs() < 1e-4);
        let aside = Mat4::from_translation(Vec3::new(5.0, 0.0, 0.0));
        assert!(ray.hits_box(-Vec3::ONE, Vec3::ONE, aside).is_none());
        let square = [
            Vec3::new(-1.0, -1.0, 0.0),
            Vec3::new(1.0, -1.0, 0.0),
            Vec3::new(1.0, 1.0, 0.0),
            Vec3::new(-1.0, 1.0, 0.0),
        ];
        assert_eq!(ray.hits_outline(&square), Some(10.0));
        let away = Ray { origin: Vec3::new(3.0, 0.0, 10.0), dir: Vec3::NEG_Z };
        assert!(away.hits_outline(&square).is_none());
    }
}
