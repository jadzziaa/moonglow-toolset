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

/// The scene's lights, by the ground they reach: sorted into their
/// squares the first time a mesh asks ([`LightGrid::reaching`]). A frame
/// whose meshes all find their lights as they took them the frame before
/// ([`ReachCache`]: the lights unchanged, the camera still or turning)
/// asks nothing, and the grid, filled and dropped every frame, was made
/// for nothing.
pub(crate) struct LightGrid<'a> {
    lights: &'a [PointLight],
    /// The lights are sorted into the squares.
    built: bool,
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
    /// The grid of `lights`, not sorted yet.
    pub(crate) fn new(lights: &'a [PointLight]) -> LightGrid<'a> {
        LightGrid {
            lights,
            built: false,
            min: (0.0, 0.0),
            square: SQUARE,
            across: 0,
            down: 0,
            starts: Vec::new(),
            items: Vec::new(),
            wide: Vec::new(),
            tried: Vec::new(),
            search: 0,
        }
    }

    /// The grid of `lights`, sorted into its squares.
    fn sorted(lights: &'a [PointLight]) -> LightGrid<'a> {
        let mut grid = LightGrid {
            lights,
            built: true,
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
        if !self.built {
            *self = LightGrid::sorted(self.lights);
        }
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

/// The lights that reached each mesh in the frames before, kept while
/// the scene's lights stay as they were (their places, how far they reach
/// and their priorities: all a mesh's choice depends on): a large area's
/// thousands of meshes, each trying the lights of the squares it stands
/// on, were the larger part of a frame of the camera turning. A mesh is
/// known by its ball, to the bit: what one takes is what
/// [`LightGrid::reaching`] gives it.
///
/// A mesh asks by its place among the scene's (mostly what it was the
/// frame before): its choice is first looked for where the mesh in its
/// place took one last frame, then by its ball (looking every mesh's up
/// by its ball, and copying its lights from where they had been put, was
/// a twelfth of a large area's frame). Choices no mesh took lately are let
/// go once they are many.
///
/// Each choice's set of lights is numbered once, as it is made: the game's
/// draws share the values of the lights a set comes to, and finding a
/// draw's set by its lights in a map made anew every frame (each set's
/// lights copied into it, and let go of after) was a tenth of a large
/// area's frame.
#[derive(Debug, Default)]
pub(crate) struct ReachCache {
    /// The lights the kept choices were made among: each one's place,
    /// reach and priority, as bits.
    lights: Vec<[u32; 5]>,
    /// The choices made, and their lights one after another.
    choices: Vec<Choice>,
    chosen: Vec<Reaching>,
    /// The choices by their balls.
    by_ball: crate::game::Fast<[u32; 4], u32>,
    /// The choice each mesh took, by its place among the scene's, in the
    /// frame before and in this one (`u32::MAX`: none).
    before: Vec<u32>,
    now: Vec<u32>,
    /// The choices this frame took.
    taken: usize,
    /// The sets of lights chosen, numbered (while the lights stay).
    sets: crate::game::Fast<Box<[u32]>, u32>,
    /// By a set's number, the frame that last asked for it and its number
    /// in that frame ([`ReachCache::in_frame`]); and how many the frame
    /// asked for.
    numbered: Vec<(u32, u32)>,
    asked: u32,
    /// This frame's number.
    frame: u32,
}

/// A mesh's choice of lights: its ball (as bits), the frame that last took
/// it, where its lights are in [`ReachCache::chosen`], and its set's
/// number.
#[derive(Debug, Clone, Copy)]
struct Choice {
    ball: [u32; 4],
    taken: u32,
    from: u32,
    to: u32,
    set: u32,
}

/// How many choices and sets of lights are kept past twice as many as a
/// frame takes, before those it didn't are let go.
const KEPT: usize = 4096;

impl ReachCache {
    /// Begins a frame of `lights`: what was kept is let go if any of them
    /// differs from those it was chosen among.
    pub(crate) fn begin(&mut self, lights: &[PointLight]) {
        let key = |l: &PointLight| {
            let p = l.position;
            [p.x.to_bits(), p.y.to_bits(), p.z.to_bits(), l.cutoff.to_bits(), l.priority]
        };
        let same = self.lights.len() == lights.len()
            && self.lights.iter().zip(lights).all(|(k, l)| *k == key(l));
        // (The sets numbered anew with the lights, or where many more are
        // kept than a frame asks for: a camera moving about. Their numbers
        // are the choices', which go with them.)
        if !same || self.sets.len() > 2 * self.taken + KEPT {
            self.choices.clear();
            self.chosen.clear();
            self.by_ball.clear();
            self.before.clear();
            self.now.clear();
            self.sets.clear();
            self.numbered.clear();
            self.lights.clear();
            self.lights.extend(lights.iter().map(key));
        } else if self.choices.len() > 2 * self.taken + KEPT {
            self.keep_taken();
        }
        std::mem::swap(&mut self.before, &mut self.now);
        self.now.clear();
        self.taken = 0;
        self.frame = self.frame.wrapping_add(1);
        self.asked = 0;
    }

    /// Lets go of the choices the last frame didn't take.
    fn keep_taken(&mut self) {
        let mut moved = vec![u32::MAX; self.choices.len()];
        let (choices, chosen) =
            (std::mem::take(&mut self.choices), std::mem::take(&mut self.chosen));
        for (i, c) in choices.iter().enumerate().filter(|(_, c)| c.taken == self.frame) {
            moved[i] = self.choices.len() as u32;
            let from = self.chosen.len() as u32;
            self.chosen.extend_from_slice(&chosen[c.from as usize..c.to as usize]);
            let to = self.chosen.len() as u32;
            self.choices.push(Choice { from, to, ..*c });
        }
        self.by_ball.retain(|_, c| {
            *c = moved[*c as usize];
            *c != u32::MAX
        });
        // (The last frame's meshes took only what is kept.)
        for c in self.now.iter_mut().filter(|c| **c != u32::MAX) {
            *c = moved[*c as usize];
        }
    }

    /// [`LightGrid::reaching`], as it was for the same ball since the
    /// lights were last as they are (`grid` made of those lights), for the
    /// mesh at `place` among the scene's; and the number of the set of
    /// lights it comes to (the same for every choice of the same lights,
    /// while they stay).
    pub(crate) fn reaching(
        &mut self,
        grid: &mut LightGrid<'_>,
        (centre, radius): (Vec3, f32),
        place: usize,
        out: &mut Vec<Reaching>,
    ) -> u32 {
        let ball = [centre.x, centre.y, centre.z, radius].map(f32::to_bits);
        // (Where the mesh in its place last frame took it, else by its ball,
        // else chosen.)
        let before = self.before.get(place).copied().filter(|&c| c != u32::MAX);
        let at = match before.filter(|&c| self.choices[c as usize].ball == ball) {
            Some(c) => c,
            None => match self.by_ball.entry(ball) {
                std::collections::hash_map::Entry::Occupied(e) => *e.get(),
                std::collections::hash_map::Entry::Vacant(e) => {
                    grid.reaching(centre, radius, out);
                    let lights: Box<[u32]> = out.iter().map(|&(_, _, i)| i).collect();
                    let next = self.sets.len() as u32;
                    let set = *self.sets.entry(lights).or_insert(next);
                    let from = self.chosen.len() as u32;
                    self.chosen.extend_from_slice(out);
                    let to = self.chosen.len() as u32;
                    let taken = self.frame.wrapping_sub(1);
                    self.choices.push(Choice { ball, taken, from, to, set });
                    *e.insert(self.choices.len() as u32 - 1)
                }
            },
        };
        if self.now.len() <= place {
            self.now.resize(place + 1, u32::MAX);
        }
        self.now[place] = at;
        let c = &mut self.choices[at as usize];
        if c.taken != self.frame {
            c.taken = self.frame;
            self.taken += 1;
        }
        out.clear();
        out.extend_from_slice(&self.chosen[c.from as usize..c.to as usize]);
        c.set
    }

    /// A set's number among those this frame asks for: from 0, in the
    /// order it first asks for them.
    pub(crate) fn in_frame(&mut self, set: u32) -> u32 {
        let at = set as usize;
        if self.numbered.len() <= at {
            self.numbered.resize(at + 1, (self.frame.wrapping_sub(1), 0));
        }
        let (asked, number) = &mut self.numbered[at];
        if *asked != self.frame {
            (*asked, *number) = (self.frame, self.asked);
            self.asked += 1;
        }
        *number
    }
}

/// The eight corners of the box `min`–`max`: corner `i` takes `max`'s x
/// where bit 0 of `i` is set, its y where bit 1 is, its z where bit 2
/// is (so 0 is `min` and 7 `max`, and corners one bit apart share an
/// edge). The one place a box's corners are worked out: the shadows'
/// reach, the screen's boxes, the area view's outlines, the previews'
/// and the model viewer's framing.
pub fn box_corners(min: Vec3, max: Vec3) -> [Vec3; 8] {
    std::array::from_fn(|i| {
        Vec3::new(
            if i & 1 == 0 { min.x } else { max.x },
            if i & 2 == 0 { min.y } else { max.y },
            if i & 4 == 0 { min.z } else { max.z },
        )
    })
}

/// The box around `points` (its least and greatest corners): `None` where
/// none of them is anywhere. A point that is not finite is left out: the
/// running least and greatest took a NaN and then the next point, so a
/// NaN in the middle of a list left the box of the points after it alone
/// (a mesh culled while in view), and one in the last place left none (an
/// area without the sun's shadows). The one place a box is put around
/// points: a mesh's, the shadows' receivers and casters, the previews'
/// and the model viewer's framing.
pub fn box_of(points: impl IntoIterator<Item = Vec3>) -> Option<(Vec3, Vec3)> {
    let mut somewhere = points.into_iter().filter(|p| p.is_finite());
    let first = somewhere.next()?;
    Some(somewhere.fold((first, first), |(min, max), p| (min.min(p), max.max(p))))
}

/// What stands for the box around no points, where one is needed all the
/// same: its least corner past its greatest, so that it holds nothing.
pub(crate) const NO_BOX: (Vec3, Vec3) = (Vec3::MAX, Vec3::MIN);

/// What the camera sees, as the six planes around it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Frustum([Vec4; 6]);

impl Frustum {
    /// Of a camera's projection times its view.
    pub fn new(view_proj: Mat4) -> Frustum {
        let (x, y, z, w) = (view_proj.row(0), view_proj.row(1), view_proj.row(2), view_proj.row(3));
        // Left, right, bottom, top, near (depths from 0), far; each with
        // its normal a unit long, pointing in.
        Frustum([w + x, w - x, w + y, w - y, z, w - z].map(|p| {
            let long = p.truncate().length();
            if long.is_finite() && long > 0.0 { p / long } else { Vec4::ZERO }
        }))
    }

    /// Whether a ball is wholly out of sight.
    pub fn hides(&self, centre: Vec3, radius: f32) -> bool {
        self.0.iter().any(|p| p.truncate().dot(centre) + p.w < -radius)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::Camera;

    /// The box around points leaves out those that are nowhere, wherever
    /// they are in the list: a NaN in the middle changes nothing (it left
    /// the box of the points after it alone), nor one at the end (which
    /// left a box of NaN); no points anywhere, no box.
    #[test]
    fn the_box_around_points_leaves_out_what_is_nowhere() {
        let (a, b, c) =
            (Vec3::new(1.0, 5.0, -2.0), Vec3::new(-3.0, 2.0, 0.0), Vec3::new(0.0, 9.0, 4.0));
        let whole = Some((Vec3::new(-3.0, 2.0, -2.0), Vec3::new(1.0, 9.0, 4.0)));
        assert_eq!(box_of([a, b, c]), whole);
        let nowhere = Vec3::new(0.0, f32::NAN, 0.0);
        for list in [
            vec![a, nowhere, b, c],
            vec![a, b, nowhere, c],
            vec![a, b, c, nowhere],
            vec![nowhere, a, b, c],
            vec![a, b, c, Vec3::new(f32::INFINITY, 0.0, 0.0), Vec3::NEG_INFINITY],
        ] {
            assert_eq!(box_of(list.clone()), whole, "{list:?}");
        }
        assert_eq!(box_of([a]), Some((a, a)));
        assert_eq!(box_of([nowhere, Vec3::NAN]), None);
        assert_eq!(box_of(Vec::<Vec3>::new()), None);
        assert!(NO_BOX.0.cmpgt(NO_BOX.1).all());
    }

    #[test]
    fn a_box_s_corners_in_their_order() {
        let (min, max) = (Vec3::new(1.0, 2.0, 3.0), Vec3::new(4.0, 5.0, 6.0));
        let c = box_corners(min, max);
        assert_eq!(c[0], min);
        assert_eq!(c[7], max);
        assert_eq!(c[1], Vec3::new(4.0, 2.0, 3.0));
        assert_eq!(c[2], Vec3::new(1.0, 5.0, 3.0));
        assert_eq!(c[4], Vec3::new(1.0, 2.0, 6.0));
        assert_eq!(c[6], Vec3::new(1.0, 5.0, 6.0));
        // Corners one bit apart differ along one axis: an edge.
        for i in 0..8 {
            for bit in [1, 2, 4] {
                let d = c[i] - c[i ^ bit];
                assert_eq!(d.abs().cmpgt(Vec3::ZERO).bitmask().count_ones(), 1, "{i} {bit}");
            }
        }
    }

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
        PointLight {
            position,
            color: Vec3::ONE,
            cutoff,
            ambient_only: false,
            priority,
            casts_shadows: false,
            shadow_lift: 0.0,
        }
    }

    /// What a mesh took in the frames before is what it takes again
    /// while the lights stay where they are, and what every light tried
    /// gives once one moves, reaches further, changes priority or another
    /// comes; its colour changes nothing of it. Whether it asks from the
    /// place it asked from before or another (some meshes of a frame not
    /// drawn in the next, some twice). The same lights are the same set,
    /// numbered in a frame from 0 in the order the frame asks for them.
    #[test]
    fn a_mesh_keeps_its_lights_until_they_change() {
        let mut n = Numbers(11);
        let mut lights: Vec<PointLight> = (0..300)
            .map(|_| {
                let at = Vec3::new(n.next() * 200.0, n.next() * 200.0, n.next() * 6.0);
                light(at, n.next() * 30.0, 1 + (n.next() * 3.0) as u32)
            })
            .collect();
        let meshes: Vec<(Vec3, f32)> = (0..200)
            .map(|_| (Vec3::new(n.next() * 200.0, n.next() * 200.0, 0.0), n.next() * 8.0))
            .collect();
        let mut cache = ReachCache::default();
        let mut out = Vec::new();
        for frame in 0..8 {
            match frame {
                3 => lights[17].position.x += 1.0,
                4 => lights[40].cutoff *= 2.0,
                5 => lights[90].priority += 1,
                6 => lights.push(light(Vec3::new(100.0, 100.0, 1.0), 50.0, 1)),
                7 => lights[3].color = Vec3::new(0.2, 0.4, 0.8),
                _ => {}
            }
            cache.begin(&lights);
            let mut grid = LightGrid::new(&lights);
            // (The grid is sorted only where a mesh asks it: not in a
            // frame whose meshes are those of the frame before, under the
            // same lights.)
            let known = (1..=2).contains(&frame) || frame == 7;
            let mut sets: std::collections::HashMap<Vec<u32>, u32> = Default::default();
            let mut asked = Vec::new();
            // (Some meshes of a frame not drawn in the next, some twice.)
            let these = meshes.iter().skip(frame % 3).chain(&meshes[..5]);
            for (place, (centre, radius)) in these.enumerate() {
                let set = cache.reaching(&mut grid, (*centre, *radius), place, &mut out);
                assert_eq!(out, every(&lights, *centre, *radius), "frame {frame}");
                let taken: Vec<u32> = out.iter().map(|&(_, _, i)| i).collect();
                assert_eq!(*sets.entry(taken).or_insert(set), set, "frame {frame}");
                let number = cache.in_frame(set) as usize;
                match asked.iter().position(|&s| s == set) {
                    Some(k) => assert_eq!(number, k, "frame {frame}"),
                    None => {
                        assert_eq!(number, asked.len(), "frame {frame}");
                        asked.push(set);
                    }
                }
            }
            assert_eq!(grid.built, !known, "frame {frame}");
        }
    }

    /// The choices no mesh took lately are let go once they are many more
    /// than a frame takes; those the last frame took are kept, as they
    /// were, with their sets' numbers.
    #[test]
    fn the_choices_not_taken_lately_are_let_go() {
        let mut n = Numbers(5);
        let lights: Vec<PointLight> = (0..40)
            .map(|_| {
                let at = Vec3::new(n.next() * 400.0, n.next() * 400.0, 0.0);
                light(at, 20.0 + n.next() * 30.0, 1)
            })
            .collect();
        let meshes: Vec<(Vec3, f32)> = (0..KEPT + 3000)
            .map(|_| (Vec3::new(n.next() * 400.0, n.next() * 400.0, 0.0), n.next() * 4.0))
            .collect();
        let mut cache = ReachCache::default();
        let mut out = Vec::new();
        // A frame of all of them, then one of ten (in other places).
        cache.begin(&lights);
        let mut grid = LightGrid::new(&lights);
        let mut sets = Vec::new();
        for (place, (centre, radius)) in meshes.iter().enumerate() {
            sets.push(cache.reaching(&mut grid, (*centre, *radius), place, &mut out));
        }
        assert_eq!(cache.choices.len(), meshes.len());
        cache.begin(&lights);
        let mut grid = LightGrid::new(&lights);
        for (place, (centre, radius)) in meshes[100..110].iter().enumerate() {
            cache.reaching(&mut grid, (*centre, *radius), place + 3, &mut out);
        }
        // Those ten are kept, the rest let go.
        cache.begin(&lights);
        assert_eq!(cache.choices.len(), 10);
        assert_eq!(cache.by_ball.len(), 10);
        let mut grid = LightGrid::new(&lights);
        for (place, (centre, radius)) in meshes[100..110].iter().enumerate() {
            let set = cache.reaching(&mut grid, (*centre, *radius), place + 3, &mut out);
            assert_eq!(out, every(&lights, *centre, *radius));
            assert_eq!(set, sets[100 + place]);
        }
        assert_eq!(cache.choices.len(), 10, "none chosen anew");
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
