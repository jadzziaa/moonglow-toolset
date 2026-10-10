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
        // An affine map keeps the ray's parameter: `near` is a world
        // distance even when the transform scales.
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
    // (Nearer than the near plane is not in the picture either; and a
    // point a hair in front of the eye would land far off the screen: a
    // walkmesh's face with such a corner was a streak across the view.)
    if clip.w < camera.near.max(1e-6) {
        return None;
    }
    let ndc = clip.truncate() / clip.w;
    Some(Vec2::new((ndc.x + 1.0) * 0.5, (1.0 - ndc.y) * 0.5))
}

/// How far `p` is from the eye along the view.
pub fn depth(camera: &Camera, p: Vec3) -> f32 {
    let to_target = camera.target - camera.eye;
    (p - camera.eye).dot(to_target / to_target.length().max(1e-6))
}

/// The part of a flat polygon that is `least` or further from the eye
/// along the view (its corners in order; fewer than three: nothing of it
/// is). Projected whole, a polygon with a corner behind the eye can't be
/// drawn, and one with a corner beside it is a streak.
pub fn clip_polygon(camera: &Camera, corners: &[Vec3], least: f32) -> Vec<Vec3> {
    let mut out = Vec::with_capacity(corners.len() + 1);
    for (i, a) in corners.iter().enumerate() {
        let b = corners[(i + 1) % corners.len()];
        let (da, db) = (depth(camera, *a) - least, depth(camera, b) - least);
        if da >= 0.0 {
            out.push(*a);
        }
        if (da >= 0.0) != (db >= 0.0) {
            out.push(*a + (b - *a) * (da / (da - db)));
        }
    }
    if out.len() < 3 { Vec::new() } else { out }
}

/// The part of the line from `a` to `b` that is `least` or further from
/// the eye along the view.
pub fn clip_segment(camera: &Camera, a: Vec3, b: Vec3, least: f32) -> Option<(Vec3, Vec3)> {
    let (da, db) = (depth(camera, a) - least, depth(camera, b) - least);
    let cut = || a + (b - a) * (da / (da - db));
    match (da >= 0.0, db >= 0.0) {
        (true, true) => Some((a, b)),
        (true, false) => Some((a, cut())),
        (false, true) => Some((cut(), b)),
        (false, false) => None,
    }
}

/// The box drawn and picked for an object without a model: sounds,
/// waypoints without a flag, and objects whose appearance cannot be shown.
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
///
/// An object with a model is picked by the model itself where `model`
/// tells (how far along the ray it meets object `i`'s triangles;
/// `Some(None)`: the ray passes them; `None`: by its box): a click through
/// the empty part of a wide box reaches what is behind it, a trigger on
/// the ground under an arch. Where the ray meets nothing so, the nearest
/// box is taken after all, so that a thin thing can still be clicked
/// beside.
pub fn pick_precisely(
    area: &AreaModel,
    ray: &Ray,
    bounds: &dyn Fn(usize) -> (Vec3, Vec3),
    model: &dyn Fn(usize) -> Option<Option<f32>>,
    shown: &dyn Fn(ObjectKind) -> bool,
) -> Option<usize> {
    let hits =
        area.objects.iter().enumerate().filter(|(_, o)| shown(o.kind)).filter_map(|(i, o)| {
            if o.kind.has_outline() {
                // Outlines lie on the ground: whatever stands on them is nearer.
                return ray.hits_outline(&o.outline).map(|t| (i, Hit::Met(t)));
            }
            let (min, max) = bounds(i);
            let boxed = ray.hits_box(min, max, o.model_transform())?;
            Some((
                i,
                match model(i) {
                    None => Hit::Met(boxed),
                    Some(Some(t)) => Hit::Met(t),
                    Some(None) => Hit::Beside(boxed),
                },
            ))
        });
    nearest(hits)
}

/// How a ray meets an object, and how far along.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Hit {
    /// It meets the object itself: its model, its outline, a marker's box.
    Met(f32),
    /// It passes through the box of an object whose model it misses.
    Beside(f32),
}

/// The object picked among those a ray meets: the nearest it meets itself,
/// else the nearest whose box it passes through.
fn nearest(hits: impl Iterator<Item = (usize, Hit)>) -> Option<usize> {
    let mut met: Option<(f32, usize)> = None;
    let mut beside: Option<(f32, usize)> = None;
    for (i, hit) in hits {
        let (best, t) = match hit {
            Hit::Met(t) => (&mut met, t),
            Hit::Beside(t) => (&mut beside, t),
        };
        if best.is_none_or(|(b, _)| t < b) {
            *best = Some((t, i));
        }
    }
    met.or(beside).map(|(_, i)| i)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: Vec3, b: Vec3) -> bool {
        (a - b).abs().max_element() < 1e-3
    }

    /// A point beside the eye, a hair in front of its plane, is not in
    /// the view: projected, it lay thousands of screens away, and an
    /// overlay's face with such a corner was a streak across the picture.
    #[test]
    fn a_point_nearer_than_the_near_plane_is_not_projected() {
        let camera = Camera::orbit(Vec3::new(10.0, 20.0, 0.0), 30.0, 0.7, 0.9);
        let forward = (camera.target - camera.eye).normalize();
        let side = forward.cross(Vec3::Z).normalize();
        let beside = |ahead: f32| camera.eye + forward * ahead + side * 2.0;
        assert_eq!(project(&camera, 1.5, beside(camera.near * 0.01)), None);
        assert_eq!(project(&camera, 1.5, beside(-1.0)), None);
        let seen = project(&camera, 1.5, beside(camera.near * 4.0 + 4.0)).unwrap();
        assert!(seen.abs().max_element() < 4.0, "{seen}");
    }

    /// A face is cut where it passes behind the camera's near plane (it
    /// stays in its plane), and so is a line; one wholly behind is gone,
    /// one wholly in front as it was.
    #[test]
    fn faces_and_lines_are_cut_at_a_depth() {
        let camera = Camera::orbit(Vec3::new(10.0, 20.0, 0.0), 11.4, 0.7, 0.14);
        let forward = (camera.target - camera.eye).normalize();
        let side = forward.cross(Vec3::Z).normalize();
        let at = |ahead: f32, aside: f32| camera.eye + forward * ahead + side * aside;
        let least = 1.0;
        let small = [at(0.9, 2.0), at(1.3, 1.9), at(1.4, 2.2)];
        let cut = clip_polygon(&camera, &small, least);
        assert_eq!(cut.len(), 4, "a corner cut off: a quadrilateral");
        assert!(cut.iter().all(|p| depth(&camera, *p) >= least - 1e-4));
        let normal = (small[1] - small[0]).cross(small[2] - small[0]).normalize();
        assert!(cut.iter().all(|p| (*p - small[0]).dot(normal).abs() < 1e-3));
        let out_there = [at(6.0, 1.0), at(9.0, -2.0), at(11.4, 0.0)];
        assert!(clip_polygon(&camera, &small, 5.0).is_empty());
        assert_eq!(clip_polygon(&camera, &out_there, least), out_there);
        // A face with a corner behind the eye shows what is in front.
        let behind = [at(-2.0, 0.0), at(3.0, 1.0), at(3.0, -1.0)];
        let shown = clip_polygon(&camera, &behind, camera.near);
        assert_eq!(shown.len(), 4);
        assert!(shown.iter().all(|p| project(&camera, 2.0, *p).is_some()));
        let (a, b) = clip_segment(&camera, at(0.5, 0.0), at(6.0, 0.0), least).unwrap();
        assert!((depth(&camera, a) - least).abs() < 1e-4 && b == at(6.0, 0.0));
        assert_eq!(clip_segment(&camera, at(0.2, 0.0), at(0.5, 1.0), least), None);
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

    /// A click through the empty part of a wide box reaches what is behind
    /// it; with nothing behind, the box's object is taken after all.
    #[test]
    fn what_is_met_itself_comes_before_a_box_passed_through() {
        // An arch's box, nearer, missed by its model; a trigger behind it.
        let through = [(0, Hit::Beside(4.0)), (1, Hit::Met(9.0))];
        assert_eq!(nearest(through.into_iter()), Some(1));
        // The arch's own stone, nearer than the trigger.
        let on_it = [(0, Hit::Met(4.5)), (1, Hit::Met(9.0))];
        assert_eq!(nearest(on_it.into_iter()), Some(0));
        // Beside a thin post with nothing behind: the post.
        let beside = [(0, Hit::Beside(4.0)), (2, Hit::Beside(3.0))];
        assert_eq!(nearest(beside.into_iter()), Some(2));
        assert_eq!(nearest(std::iter::empty()), None);
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
