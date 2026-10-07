//! What a mesh needs of the scene before it is drawn: the lights that reach
//! it, and whether the camera sees it at all.
//!
//! Each mesh takes the 32 most important lights that reach it. Trying every
//! light for every mesh was the larger part of drawing a large area (750
//! lights by 3,700 meshes, every frame): the lights are sorted once into
//! the squares of a grid on the ground that they reach, and a mesh tries
//! only those of the squares it stands on. What a mesh takes is what it
//! took when every light was tried.

use glam::{Mat4, Vec3, Vec4};

use crate::renderer::MAX_LIGHTS;
use crate::scene::PointLight;

/// A square's side, meters (a tile's), unless the lights lie wider than
/// [`ACROSS`] squares.
const SQUARE: f32 = 10.0;
/// The squares along a side, at most.
const ACROSS: usize = 64;
/// A light that reaches more squares than this, or a mesh that stands on
/// more, is not looked up in the grid: the light is tried for every mesh,
/// the mesh tries every light.
const WIDE: usize = 64;
/// With fewer lights than this there is no grid: every light is tried.
const FEW: usize = 16;

/// A light that reaches a mesh: its priority, how far it is from the
/// mesh's surface, and its place among the scene's lights.
pub(crate) type Reaching = (u32, f32, u32);

/// The scene's lights, by the ground they reach.
pub(crate) struct LightGrid<'a> {
    lights: &'a [PointLight],
    /// The grid's corner, its squares' side and how many there are each
    /// way; none across with no grid.
    min: (f32, f32),
    square: f32,
    across: usize,
    down: usize,
    /// Each square's lights: `items[starts[s]..starts[s + 1]]`.
    starts: Vec<u32>,
    items: Vec<u32>,
    /// The lights in no square.
    wide: Vec<u32>,
    /// The search each light was last tried in (a light in several of a
    /// mesh's squares is tried once).
    tried: Vec<u32>,
    search: u32,
}

impl<'a> LightGrid<'a> {
    pub(crate) fn new(lights: &'a [PointLight]) -> LightGrid<'a> {
        let mut grid = LightGrid {
            lights,
            min: (0.0, 0.0),
            square: SQUARE,
            across: 0,
            down: 0,
            starts: Vec::new(),
            items: Vec::new(),
            wide: Vec::new(),
            tried: Vec::new(),
            search: 0,
        };
        if lights.len() < FEW {
            return grid;
        }
        // The ground each light reaches; one that has no place on it (or
        // reaches nothing, as the test of a mesh still decides) is tried
        // for every mesh.
        let placed = |l: &PointLight| {
            let (x, y, r) = (l.position.x, l.position.y, l.cutoff);
            (x.is_finite() && y.is_finite() && r.is_finite() && r > 0.0).then_some((
                x - r,
                y - r,
                x + r,
                y + r,
            ))
        };
        let mut bounds: Option<(f32, f32, f32, f32)> = None;
        for (x0, y0, x1, y1) in lights.iter().filter_map(placed) {
            bounds = Some(match bounds {
                Some(b) => (b.0.min(x0), b.1.min(y0), b.2.max(x1), b.3.max(y1)),
                None => (x0, y0, x1, y1),
            });
        }
        let Some((x0, y0, x1, y1)) = bounds else {
            grid.wide = (0..lights.len() as u32).collect();
            return grid;
        };
        let (wide, deep) = (x1 - x0, y1 - y0);
        grid.min = (x0, y0);
        grid.square = SQUARE.max(wide / ACROSS as f32).max(deep / ACROSS as f32);
        if !grid.square.is_finite() {
            grid.wide = (0..lights.len() as u32).collect();
            return grid;
        }
        grid.across = ((wide / grid.square).ceil() as usize).clamp(1, ACROSS);
        grid.down = ((deep / grid.square).ceil() as usize).clamp(1, ACROSS);
        // Counted, then filled: each square's lights in the scene's order.
        let mut squares: Vec<Option<(usize, usize, usize, usize)>> =
            Vec::with_capacity(lights.len());
        let mut counts = vec![0u32; grid.across * grid.down + 1];
        for (i, l) in lights.iter().enumerate() {
            let on = placed(l)
                .map(|(x0, y0, x1, y1)| grid.squares(x0, y0, x1, y1))
                .filter(|&(c0, r0, c1, r1)| (c1 - c0 + 1) * (r1 - r0 + 1) <= WIDE);
            match on {
                Some((c0, r0, c1, r1)) => {
                    for r in r0..=r1 {
                        for c in c0..=c1 {
                            counts[r * grid.across + c + 1] += 1;
                        }
                    }
                }
                None => grid.wide.push(i as u32),
            }
            squares.push(on);
        }
        for s in 1..counts.len() {
            counts[s] += counts[s - 1];
        }
        grid.items = vec![0; counts[counts.len() - 1] as usize];
        let mut next = counts.clone();
        for (i, on) in squares.iter().enumerate() {
            let Some((c0, r0, c1, r1)) = *on else { continue };
            for r in r0..=r1 {
                for c in c0..=c1 {
                    let at = &mut next[r * grid.across + c];
                    grid.items[*at as usize] = i as u32;
                    *at += 1;
                }
            }
        }
        grid.starts = counts;
        grid.tried = vec![0; lights.len()];
        grid
    }

    /// The squares a box on the ground lies on (columns and rows, first
    /// and last), those at the grid's edge for what lies beyond it.
    fn squares(&self, x0: f32, y0: f32, x1: f32, y1: f32) -> (usize, usize, usize, usize) {
        let along = |v: f32, min: f32, count: usize| {
            // (A float as an index stops at the ends, and takes not a
            // number for 0.)
            (((v - min) / self.square).floor() as usize).min(count - 1)
        };
        (
            along(x0, self.min.0, self.across),
            along(y0, self.min.1, self.down),
            along(x1, self.min.0, self.across),
            along(y1, self.min.1, self.down),
        )
    }

    /// The lights that reach a mesh, a ball of `radius` around `centre`,
    /// into `out`: the [`MAX_LIGHTS`] most important (by priority, then
    /// the nearest, then as the scene lists them).
    pub(crate) fn reaching(&mut self, centre: Vec3, radius: f32, out: &mut Vec<Reaching>) {
        out.clear();
        let lights = self.lights;
        let mut try_light = |i: u32| {
            let l = &lights[i as usize];
            let d = l.position.distance(centre) - radius;
            if d <= l.cutoff {
                out.push((l.priority, d, i));
            }
        };
        // The squares the mesh stands on, if it can be said where it is.
        let on = (self.across > 0 && centre.x.is_finite() && centre.y.is_finite())
            .then_some((centre.x - radius, centre.y - radius, centre.x + radius, centre.y + radius))
            .filter(|b| b.0.is_finite() && b.1.is_finite() && b.2.is_finite() && b.3.is_finite())
            .filter(|b| b.0 <= b.2 && b.1 <= b.3)
            .map(|(x0, y0, x1, y1)| self.squares(x0, y0, x1, y1))
            .filter(|&(c0, r0, c1, r1)| (c1 - c0 + 1) * (r1 - r0 + 1) <= WIDE);
        match on {
            Some((c0, r0, c1, r1)) => {
                self.search = self.search.wrapping_add(1);
                if self.search == 0 {
                    self.tried.fill(0);
                    self.search = 1;
                }
                for r in r0..=r1 {
                    for c in c0..=c1 {
                        let s = r * self.across + c;
                        let here = self.starts[s] as usize..self.starts[s + 1] as usize;
                        for &i in &self.items[here] {
                            if self.tried[i as usize] != self.search {
                                self.tried[i as usize] = self.search;
                                try_light(i);
                            }
                        }
                    }
                }
                for &i in &self.wide {
                    try_light(i);
                }
            }
            None => (0..lights.len() as u32).for_each(&mut try_light),
        }
        // (No two the same, by their place in the scene: whatever order
        // they were found in, the same lights in the same order.)
        let order = |a: &Reaching, b: &Reaching| {
            a.0.cmp(&b.0).then(a.1.total_cmp(&b.1)).then(a.2.cmp(&b.2))
        };
        if out.len() > MAX_LIGHTS {
            out.select_nth_unstable_by(MAX_LIGHTS - 1, order);
            out.truncate(MAX_LIGHTS);
        }
        out.sort_unstable_by(order);
    }
}

/// What the camera sees, as the six planes around it.
pub(crate) struct Frustum([Vec4; 6]);

impl Frustum {
    /// Of a camera's projection times its view.
    pub(crate) fn new(view_proj: Mat4) -> Frustum {
        let (x, y, z, w) = (view_proj.row(0), view_proj.row(1), view_proj.row(2), view_proj.row(3));
        // Left, right, bottom, top, near (depths from 0), far; each with
        // its normal a unit long, pointing in.
        Frustum([w + x, w - x, w + y, w - y, z, w - z].map(|p| {
            let long = p.truncate().length();
            if long.is_finite() && long > 0.0 { p / long } else { Vec4::ZERO }
        }))
    }

    /// Whether a ball is wholly out of sight.
    pub(crate) fn hides(&self, centre: Vec3, radius: f32) -> bool {
        self.0.iter().any(|p| p.truncate().dot(centre) + p.w < -radius)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::Camera;

    /// A sequence of numbers from 0 to 1 that looks random.
    struct Numbers(u64);

    impl Numbers {
        fn next(&mut self) -> f32 {
            self.0 = self.0.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
            (self.0 >> 40) as f32 / (1u64 << 24) as f32
        }
    }

    /// Every light tried, as the renderer chose them before the grid.
    fn every(lights: &[PointLight], centre: Vec3, radius: f32) -> Vec<Reaching> {
        let mut chosen: Vec<Reaching> = lights
            .iter()
            .enumerate()
            .filter_map(|(i, l)| {
                let d = l.position.distance(centre) - radius;
                (d <= l.cutoff).then_some((l.priority, d, i as u32))
            })
            .collect();
        chosen.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.total_cmp(&b.1)));
        chosen.truncate(MAX_LIGHTS);
        chosen
    }

    fn light(position: Vec3, cutoff: f32, priority: u32) -> PointLight {
        PointLight { position, color: Vec3::ONE, cutoff, ambient_only: false, priority }
    }

    #[test]
    fn a_mesh_takes_the_lights_it_took_when_every_light_was_tried() {
        let mut n = Numbers(7);
        for scene in 0..40 {
            // Areas of some size, crowded and not; lights that reach a
            // little and a lot, some with no place or nothing to reach.
            let side = [40.0, 160.0, 320.0, 2000.0][scene % 4];
            let count = [3, 20, 150, 800][scene / 4 % 4];
            let mut lights: Vec<PointLight> = (0..count)
                .map(|_| {
                    let at = Vec3::new(n.next() * side, n.next() * side, n.next() * 6.0);
                    let reach = if n.next() < 0.1 { n.next() * 400.0 } else { n.next() * 30.0 };
                    light(at, reach, 1 + (n.next() * 5.0) as u32)
                })
                .collect();
            if scene % 3 == 0 {
                lights.push(light(Vec3::new(f32::NAN, 5.0, 0.0), 10.0, 1));
                lights.push(light(Vec3::new(5.0, f32::INFINITY, 0.0), f32::INFINITY, 2));
                lights.push(light(Vec3::new(20.0, 20.0, 0.0), 0.0, 1));
                lights.push(light(Vec3::new(20.0, 20.0, 0.0), -3.0, 1));
                // (Two the same: the scene's order tells them apart.)
                lights.push(light(Vec3::new(30.0, 30.0, 1.0), 25.0, 3));
                lights.push(light(Vec3::new(30.0, 30.0, 1.0), 25.0, 3));
            }
            let mut grid = LightGrid::new(&lights);
            let mut out = Vec::new();
            for mesh in 0..300 {
                let centre = Vec3::new(
                    (n.next() * 1.4 - 0.2) * side,
                    (n.next() * 1.4 - 0.2) * side,
                    n.next() * 8.0 - 2.0,
                );
                let radius = match mesh % 7 {
                    0 => 0.0,
                    1 => n.next() * 2000.0,
                    2 => f32::INFINITY,
                    _ => n.next() * 12.0,
                };
                let centre = if mesh % 31 == 5 { centre.with_x(f32::NAN) } else { centre };
                grid.reaching(centre, radius, &mut out);
                let expected = every(&lights, centre, radius);
                // (Distances that are not numbers are no distances to tell
                // apart: the lights are the same, and in the same order.)
                let same = |a: &Reaching, b: &Reaching| {
                    a.0 == b.0 && a.2 == b.2 && a.1.to_bits() == b.1.to_bits()
                };
                assert!(
                    out.len() == expected.len()
                        && out.iter().zip(&expected).all(|(a, b)| same(a, b)),
                    "scene {scene} ({count} lights over {side} m), a mesh at {centre} of radius \
                     {radius}: {out:?}, not {expected:?}"
                );
            }
        }
    }

    #[test]
    fn the_camera_hides_only_what_is_out_of_its_sight() {
        let camera = Camera::orbit(Vec3::new(50.0, 50.0, 0.0), 30.0, -1.2, 0.6);
        let view_proj = camera.projection(1.6) * camera.view();
        let frustum = Frustum::new(view_proj);
        let mut n = Numbers(11);
        let (mut hidden, mut seen) = (0, 0);
        for _ in 0..4000 {
            let centre =
                Vec3::new(n.next() * 200.0 - 50.0, n.next() * 200.0 - 50.0, n.next() * 30.0 - 10.0);
            let radius = n.next() * 8.0;
            // Points of the ball: none of a hidden ball's is in sight.
            let in_sight = (0..60).any(|_| {
                let dir = Vec3::new(n.next() - 0.5, n.next() - 0.5, n.next() - 0.5);
                let p = centre + dir.normalize_or_zero() * radius * n.next();
                let clip = view_proj * p.extend(1.0);
                clip.w > 0.0
                    && clip.x.abs() <= clip.w
                    && clip.y.abs() <= clip.w
                    && (0.0..=clip.w).contains(&clip.z)
            });
            if frustum.hides(centre, radius) {
                hidden += 1;
                assert!(!in_sight, "a ball at {centre} of radius {radius} is in sight");
            } else {
                seen += 1;
            }
        }
        // (Most of what is around a camera that looks at a spot is hidden.)
        assert!(hidden > 2000 && seen > 100, "{hidden} hidden, {seen} seen");
        // What it looks at is seen; what is behind it is hidden; and a ball
        // around the camera itself is seen.
        assert!(!frustum.hides(camera.target, 0.1));
        assert!(frustum.hides(camera.eye + (camera.eye - camera.target), 1.0));
        assert!(!frustum.hides(camera.eye, 5.0));
        assert!(!frustum.hides(Vec3::splat(f32::NAN), 1.0), "not a place: not hidden");
    }
}
