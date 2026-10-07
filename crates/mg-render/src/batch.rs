//! Draws of one mesh put together, to be drawn as instances of it.
//!
//! An area's tiles are of few models, and what a draw costs past its own
//! values is the same however little it draws: a tile's mesh drawn once
//! for every tile it is on was most of what was left of drawing a large
//! area. Draws of the same mesh with the same material become one draw of
//! as many instances.
//!
//! The order of draws matters, though, where two of them meet: of coplanar
//! meshes (a tile's floor and what lies on it) the later drawn shows, and
//! see-through meshes blend in the order they are drawn. So a draw joins
//! the draws of its mesh only if that takes it past nothing it could meet:
//! every two draws whose bounds meet are drawn in the order they came in,
//! as are a model's own meshes in the solid pass.
//!
//! Bounds are boxes in the space where draws of the pass meet: in the
//! world for solid meshes (which meet where they are at the same depth:
//! the same place), on the screen for see-through ones (which meet where
//! one is seen through the other). Two boxes meet where they share a
//! patch of surface: they overlap, or a flat one lies on a face of the
//! other. Boxes that only touch do not meet: tiles lie edge to edge.

use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hasher};

use glam::Vec3;

/// What draws must share to be one draw: the mesh (its address), its
/// material's bind group, and the vertices that replace its own.
pub(crate) type Key = (usize, u32, Option<(u64, u64)>);

/// A draw's bounds: a box, in the space of its pass.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Bounds {
    pub(crate) min: Vec3,
    pub(crate) max: Vec3,
}

impl Bounds {
    /// Whether two draws could meet (share a patch of surface): their
    /// boxes overlap each way, or two ways with one of them flat the
    /// third way and on or in the other there (a decal on a floor or a
    /// wall). By more than twice `slack`: two tiles' floors, edge to edge,
    /// don't meet, nor a wall and the floor at its foot.
    pub(crate) fn meets(&self, other: &Bounds, slack: f32) -> bool {
        let over = (self.max.min(other.max) - self.min.max(other.min)).to_array();
        let (mine, its) = ((self.max - self.min).to_array(), (other.max - other.min).to_array());
        let mut on_a_face = 0;
        for axis in 0..3 {
            if over[axis] > 2.0 * slack {
                continue;
            }
            let flat = mine[axis] <= 2.0 * slack || its[axis] <= 2.0 * slack;
            if !(flat && over[axis] >= -slack) {
                return false;
            }
            on_a_face += 1;
        }
        on_a_face <= 1
    }

    fn somewhere(&self) -> bool {
        self.min.is_finite() && self.max.is_finite()
    }
}

/// A box over more squares than this is kept in a list of its own, and
/// tried for every draw.
const WIDE: i64 = 16;
/// How many of a square's draws are tried, the latest first, before the
/// draw is taken to meet one of them.
const TRIED: usize = 48;

/// A hash for the squares and the keys (small numbers, looked up for every
/// draw: the standard one costs more than the rest of placing a draw).
#[derive(Default)]
struct Mix(u64);

impl Hasher for Mix {
    fn write(&mut self, bytes: &[u8]) {
        for chunk in bytes.chunks(8) {
            let mut word = [0u8; 8];
            word[..chunk.len()].copy_from_slice(chunk);
            self.write_u64(u64::from_le_bytes(word));
        }
    }

    fn write_u64(&mut self, n: u64) {
        self.0 = (self.0.rotate_left(5) ^ n).wrapping_mul(0x517c_c1b7_2722_0a95);
    }

    fn write_u32(&mut self, n: u32) {
        self.write_u64(u64::from(n));
    }

    fn write_i32(&mut self, n: i32) {
        self.write_u64(n as u64);
    }

    fn write_usize(&mut self, n: usize) {
        self.write_u64(n as u64);
    }

    fn write_u8(&mut self, n: u8) {
        self.write_u64(u64::from(n));
    }

    fn finish(&self) -> u64 {
        // (The table takes its place from the low bits, which the
        // multiplication leaves the weakest.)
        self.0 ^ (self.0 >> 29)
    }
}

type Mixed<K, V> = HashMap<K, V, BuildHasherDefault<Mix>>;

/// A square's draws: the batch each is in and its box, and the latest
/// batch among them.
#[derive(Default)]
struct Square {
    draws: Vec<(u32, Bounds)>,
    latest: u32,
}

/// Puts a frame's draws into batches, pass by pass, each pass's draws in
/// the order they are to be drawn. Kept from frame to frame, for its room.
#[derive(Default)]
pub(crate) struct Batcher {
    /// How many batches there are.
    batches: u32,
    /// The pass's first batch: no draw joins one before it. Later, the
    /// batch of a draw without bounds, which nothing is taken past.
    floor: u32,
    /// Each key's latest batch, in this pass.
    latest: Mixed<Key, u32>,
    /// The squares' side and the slack of [`Bounds::meets`], in this pass.
    side: f32,
    slack: f32,
    /// The draws placed in this pass by the squares their boxes are on
    /// (each square's place in `squares`), and those on too many.
    places: Mixed<(i32, i32), u32>,
    squares: Vec<Square>,
    used: usize,
    wide: Vec<(u32, Bounds)>,
}

impl std::fmt::Debug for Batcher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Batcher").field("batches", &self.batches).finish_non_exhaustive()
    }
}

impl Batcher {
    /// A new frame.
    pub(crate) fn begin(&mut self) {
        self.batches = 0;
        self.pass(1.0, 0.0);
    }

    /// A new pass: its draws join no batch of the passes before. Its
    /// boxes are looked up by squares of side `side`, and meet by more
    /// than `slack` ([`Bounds::meets`]).
    pub(crate) fn pass(&mut self, side: f32, slack: f32) {
        self.floor = self.batches;
        self.latest.clear();
        self.places.clear();
        self.used = 0;
        self.wide.clear();
        (self.side, self.slack) = (side, slack);
    }

    /// How many batches the frame's draws are in.
    pub(crate) fn batches(&self) -> u32 {
        self.batches
    }

    /// The squares a box is on (first and last, each way), if it is on
    /// no more than [`WIDE`] of them.
    fn squares_of(&self, bounds: &Bounds) -> Option<(i32, i32, i32, i32)> {
        if self.side.is_nan() || self.side <= 0.0 {
            return None;
        }
        // (A float as an integer stops at the ends.)
        let at = |v: f32| (v / self.side).floor() as i32;
        // A box ends short of a square's edge it reaches (the next
        // square's boxes that begin there don't meet it); a flat one is
        // on both sides of the edge it lies on (it meets what ends
        // there, or within the slack of there).
        let along = |min: f32, max: f32| {
            let flat = max - min <= 2.0 * self.slack;
            let by = if flat { -2.0 * self.slack } else { self.slack };
            let first = at(min + by);
            (first, at(max - by).max(first))
        };
        let ((c0, c1), (r0, r1)) =
            (along(bounds.min.x, bounds.max.x), along(bounds.min.y, bounds.max.y));
        let count = (i64::from(c1) - i64::from(c0) + 1) * (i64::from(r1) - i64::from(r0) + 1);
        (count <= WIDE).then_some((c0, r0, c1, r1))
    }

    /// Whether a draw with `bounds` meets one placed in a batch after
    /// `batch` (which it would then be drawn before).
    fn meets_after(&self, batch: u32, bounds: &Bounds, on: Option<(i32, i32, i32, i32)>) -> bool {
        let meets =
            |(placed, other): &(u32, Bounds)| *placed > batch && bounds.meets(other, self.slack);
        if self.wide.iter().any(meets) {
            return true;
        }
        let Some((c0, r0, c1, r1)) = on else {
            // (On too many squares to look in: any draw placed since may
            // be one it meets.)
            return self.batches > batch + 1;
        };
        for r in r0..=r1 {
            for c in c0..=c1 {
                let Some(square) = self.places.get(&(c, r)) else { continue };
                let square = &self.squares[*square as usize];
                if square.latest <= batch {
                    continue;
                }
                // (More than are tried: one of the rest may be met.)
                if square.draws.len() > TRIED || square.draws.iter().rev().any(meets) {
                    return true;
                }
            }
        }
        false
    }

    /// Places the pass's next draw: the batch it is in. `bounds`: its box
    /// (`None`: it may be anywhere, a skinned mesh's, and is taken past
    /// nothing, nor anything past it). `after`: a batch it is not to be
    /// drawn before (the batch of its model's mesh before it).
    pub(crate) fn place(&mut self, key: Key, bounds: Option<Bounds>, after: Option<u32>) -> u32 {
        let bounds = bounds.filter(Bounds::somewhere);
        let on = bounds.as_ref().and_then(|b| self.squares_of(b));
        let floor = self.floor.max(after.unwrap_or(0));
        let joined = match (self.latest.get(&key), &bounds) {
            (Some(&batch), Some(b)) if batch >= floor && !self.meets_after(batch, b, on) => {
                Some(batch)
            }
            _ => None,
        };
        let batch = joined.unwrap_or_else(|| {
            self.batches += 1;
            self.latest.insert(key, self.batches - 1);
            self.batches - 1
        });
        match (bounds, on) {
            (Some(b), Some((c0, r0, c1, r1))) => {
                for r in r0..=r1 {
                    for c in c0..=c1 {
                        let square = *self.places.entry((c, r)).or_insert_with(|| {
                            if self.used == self.squares.len() {
                                self.squares.push(Square::default());
                            }
                            let square = &mut self.squares[self.used];
                            square.draws.clear();
                            square.latest = 0;
                            self.used += 1;
                            self.used as u32 - 1
                        });
                        let square = &mut self.squares[square as usize];
                        square.draws.push((batch, b));
                        square.latest = square.latest.max(batch);
                    }
                }
            }
            (Some(b), None) => self.wide.push((batch, b)),
            // (Nothing after it joins a batch before it.)
            (None, _) => self.floor = batch,
        }
        batch
    }
}

/// The draws in the order they are drawn in, batch by batch (each batch's
/// draws in the order they came in), from the batch each draw was placed
/// in: where each batch's draws begin (and, last, where they end), and the
/// draws.
pub(crate) fn in_order(batch_of: &[u32], batches: u32) -> (Vec<u32>, Vec<u32>) {
    let mut starts = vec![0u32; batches as usize + 1];
    for &b in batch_of {
        starts[b as usize + 1] += 1;
    }
    for b in 1..starts.len() {
        starts[b] += starts[b - 1];
    }
    let mut next = starts.clone();
    let mut order = vec![0u32; batch_of.len()];
    for (draw, &b) in batch_of.iter().enumerate() {
        order[next[b as usize] as usize] = draw as u32;
        next[b as usize] += 1;
    }
    (starts, order)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A sequence of numbers from 0 to 1 that looks random.
    struct Numbers(u64);

    impl Numbers {
        fn next(&mut self) -> f32 {
            self.0 = self.0.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
            (self.0 >> 40) as f32 / (1u64 << 24) as f32
        }
    }

    /// A box from a corner: `side` square on the ground, `tall` high.
    fn cube(x: f32, y: f32, z: f32, side: f32, tall: f32) -> Bounds {
        Bounds { min: Vec3::new(x, y, z), max: Vec3::new(x + side, y + side, z + tall) }
    }

    fn slab(min: [f32; 3], max: [f32; 3]) -> Bounds {
        Bounds { min: Vec3::from(min), max: Vec3::from(max) }
    }

    #[test]
    fn boxes_meet_where_they_share_a_patch_of_surface() {
        let s = 0.001;
        let floor = cube(0.0, 0.0, 0.0, 10.0, 0.0);
        // Floors edge to edge, and corner to corner: no.
        assert!(!floor.meets(&cube(10.0, 0.0, 0.0, 10.0, 0.0), s));
        assert!(!floor.meets(&cube(10.0, 10.0, 0.0, 10.0, 0.0), s));
        // One a little over the other, a decal on it, one under a block: yes.
        assert!(floor.meets(&cube(9.5, 0.0, 0.0, 10.0, 0.0), s));
        assert!(floor.meets(&cube(2.0, 2.0, 0.0, 3.0, 0.0), s));
        assert!(floor.meets(&cube(2.0, 2.0, 0.0, 3.0, 2.0), s));
        // The same higher up, and a branch over the floor: no.
        assert!(!floor.meets(&cube(2.0, 2.0, 0.5, 3.0, 0.0), s));
        assert!(!floor.meets(&cube(2.0, 2.0, 3.0, 3.0, 1.0), s));
        // A wall and a picture flat on its face: yes. The wall and the
        // floor at its foot, and the next tile's floor: no (an edge).
        let wall = slab([3.0, 0.0, 0.0], [3.2, 10.0, 3.0]);
        assert!(wall.meets(&slab([3.2, 4.0, 1.0], [3.2, 6.0, 2.0]), s));
        let thin = slab([10.0, 0.0, 0.0], [10.0, 10.0, 3.0]);
        assert!(!thin.meets(&floor, s));
        assert!(!thin.meets(&cube(10.0, 0.0, 0.0, 10.0, 0.0), s));
        // Blocks side by side: no; one in the other: yes.
        let block = cube(0.0, 0.0, 0.0, 10.0, 3.0);
        assert!(!block.meets(&cube(10.0, 0.0, 0.0, 10.0, 3.0), s));
        assert!(block.meets(&cube(8.0, 8.0, 1.0, 5.0, 5.0), s));
        // On the screen (flat boxes): by their rectangles.
        let on_screen = |x: f32, y: f32, w: f32| slab([x, y, 0.0], [x + w, y + w, 0.0]);
        assert!(on_screen(0.0, 0.0, 0.2).meets(&on_screen(0.1, 0.1, 0.2), 0.0001));
        assert!(!on_screen(0.0, 0.0, 0.2).meets(&on_screen(0.2, 0.0, 0.2), 0.0001));
    }

    /// Tiles of one model, edge to edge, each with a floor, a decal lying
    /// on it and a rock: three draws, however many tiles.
    #[test]
    fn tiles_of_one_model_are_three_draws() {
        let mut b = Batcher::default();
        b.begin();
        b.pass(10.0, 0.001);
        let mut placed = Vec::new();
        for tile in 0..64 {
            let (x, y) = ((tile % 8) as f32 * 10.0, (tile / 8) as f32 * 10.0);
            let floor = b.place((1, 0, None), Some(cube(x, y, 0.0, 10.0, 0.0)), None);
            let decal =
                b.place((2, 1, None), Some(cube(x + 2.0, y + 2.0, 0.0, 3.0, 0.0)), Some(floor));
            let rock =
                b.place((3, 0, None), Some(cube(x + 6.0, y + 6.0, 0.0, 2.0, 1.5)), Some(decal));
            placed.extend([floor, decal, rock]);
        }
        assert_eq!(b.batches(), 3);
        let (starts, order) = in_order(&placed, b.batches());
        assert_eq!(starts, [0, 64, 128, 192]);
        // Each batch's draws in the order they came in.
        assert_eq!(&order[..3], [0, 3, 6]);
        assert_eq!(&order[64..67], [1, 4, 7]);
    }

    /// What lies across two tiles (a rug, coplanar with both floors) is
    /// drawn after the floors it came after, and a floor that comes after
    /// it is not drawn before it.
    #[test]
    fn a_draw_is_not_taken_past_one_it_meets() {
        let mut b = Batcher::default();
        b.begin();
        b.pass(10.0, 0.001);
        let floor = (1, 0, None);
        let first = b.place(floor, Some(cube(0.0, 0.0, 0.0, 10.0, 0.0)), None);
        let rug = b.place((2, 1, None), Some(cube(8.0, 2.0, 0.0, 4.0, 0.0)), None);
        // The next tile's floor lies under the rug too: after it, as it came.
        let second = b.place(floor, Some(cube(10.0, 0.0, 0.0, 10.0, 0.0)), None);
        // A floor far from the rug joins the latest of its kind.
        let third = b.place(floor, Some(cube(50.0, 0.0, 0.0, 10.0, 0.0)), None);
        assert!(first < rug && rug < second, "{first} {rug} {second}");
        assert_eq!(third, second);
        // What is over the rug but well above it (a branch) meets nothing.
        let branch = (3, 0, None);
        let high = b.place(branch, Some(cube(8.0, 2.0, 3.0, 4.0, 1.0)), None);
        let other = b.place(branch, Some(cube(58.0, 2.0, 3.0, 4.0, 1.0)), None);
        assert_eq!(high, other);
        // A picture flat on a wall that ends on the squares' line: after
        // the wall, though they are on either side of the line.
        let (wall, picture) = ((5, 0, None), (6, 0, None));
        let _ = b.place(picture, Some(slab([70.0, 44.0, 1.0], [70.0, 46.0, 2.0])), None);
        let hung_on = b.place(wall, Some(slab([29.8, 40.0, 0.0], [30.0, 50.0, 3.0])), None);
        let hung = b.place(picture, Some(slab([30.0, 44.0, 1.0], [30.0, 46.0, 2.0])), None);
        assert!(hung > hung_on, "{hung} {hung_on}");
        // A draw without bounds: nothing joins a batch before it.
        let skinned = b.place((4, 0, None), None, None);
        let fourth = b.place(floor, Some(cube(90.0, 0.0, 0.0, 10.0, 0.0)), None);
        assert!(fourth > skinned);
        // And a new pass begins anew.
        b.pass(0.125, 0.0001);
        let later = b.place(floor, Some(cube(0.0, 0.0, 0.0, 0.1, 0.0)), None);
        assert!(later > fourth);
    }

    /// However draws lie: each is in one batch with draws of its key, and
    /// any two that meet (or of which one has no bounds, or which are one
    /// model's) are drawn in the order they came in.
    #[test]
    fn draws_that_meet_keep_their_order() {
        let mut n = Numbers(3);
        for scene in 0..80 {
            let mut b = Batcher::default();
            b.begin();
            let (side, slack) = if scene % 2 == 0 { (10.0, 0.001) } else { (0.125, 0.0001) };
            let span = side * [4.0, 12.0, 40.0][scene % 3];
            b.pass(side, slack);
            let count = 40 + scene * 7;
            let mut draws: Vec<(Key, Option<Bounds>, usize)> = Vec::new();
            let mut placed = Vec::new();
            let (mut model, mut last) = (0, None);
            for i in 0..count {
                // (Runs of draws are one model's.)
                if n.next() < 0.4 {
                    (model, last) = (i, None);
                }
                let key = ((n.next() * 9.0) as usize, (n.next() * 2.0) as u32, None);
                let bounds = match (n.next() * 20.0) as u32 {
                    0 => None,
                    1 => Some(cube(0.0, 0.0, 0.0, span * 3.0, 1.0)),
                    2 => Some(cube(f32::NAN, 0.0, 0.0, 1.0, 1.0)),
                    _ => {
                        // (On the squares' lines often, as tiles are, and
                        // flat one way or another.)
                        let lined = n.next() < 0.5;
                        let at = |n: &mut Numbers| {
                            let v = n.next() * span;
                            if lined { (v / side).round() * side } else { v }
                        };
                        let min = Vec3::new(at(&mut n), at(&mut n), (n.next() * 3.0).floor());
                        let mut size = Vec3::new(
                            if n.next() < 0.5 { side } else { n.next() * side * 2.5 },
                            if n.next() < 0.5 { side } else { n.next() * side * 2.5 },
                            (n.next() * 3.0).floor(),
                        );
                        match (n.next() * 6.0) as u32 {
                            0 => size.x = 0.0,
                            1 => size.y = 0.0,
                            2 => size.z = 0.0,
                            _ => {}
                        }
                        Some(Bounds { min, max: min + size })
                    }
                };
                let batch = b.place(key, bounds, last);
                last = Some(batch);
                draws.push((key, bounds, model));
                placed.push(batch);
            }
            let (starts, order) = in_order(&placed, b.batches());
            assert_eq!(order.len(), count);
            let mut drawn_at = vec![usize::MAX; count];
            for (at, &draw) in order.iter().enumerate() {
                assert_eq!(drawn_at[draw as usize], usize::MAX, "drawn once");
                drawn_at[draw as usize] = at;
            }
            for batch in starts.windows(2) {
                let of = &order[batch[0] as usize..batch[1] as usize];
                assert!(of.windows(2).all(|w| draws[w[0] as usize].0 == draws[w[1] as usize].0));
                assert!(of.windows(2).all(|w| w[0] < w[1]), "a batch's draws in their order");
            }
            for j in 0..count {
                for i in 0..j {
                    let (a, c) = (&draws[i], &draws[j]);
                    let meet = match (&a.1, &c.1) {
                        (Some(a), Some(c)) => {
                            !(a.somewhere() && c.somewhere()) || a.meets(c, slack)
                        }
                        _ => true,
                    };
                    if meet || a.2 == c.2 {
                        assert!(
                            drawn_at[i] < drawn_at[j],
                            "scene {scene}: draw {i} ({a:?}) is drawn after draw {j} ({c:?})"
                        );
                    }
                }
            }
        }
    }
}
