//! An area's grass, for the editor: the edits that set it in a GIT (as
//! undoable workspace edits, each changing only what it must: the steps
//! of [`mg_module::grass`]'s plans, which `mg areas` makes in place) and
//! the tufts the area view draws. What it is and how the engine keeps it:
//! [`mg_module::grass`], re-exported here.

use glam::{Vec2, Vec3};
use mg_edit::{Edit, GffPath};
use mg_gff::Struct;
use mg_mdl::{Face, Mesh, MeshExtra, Model, Node, NodeKind, Skin};
use mg_resman::ResKey;

pub use mg_module::grass::*;

use crate::walk::WalkFace;

/// The path of `AreaProperties` in a GIT.
pub fn properties_path() -> GffPath {
    GffPath::root().field(PROPERTIES)
}

/// The path of override `index`.
pub fn override_path(index: usize) -> GffPath {
    properties_path().field(OVERRIDES).item(LIST, index)
}

/// The undoable edits that make `steps` (a plan of [`mg_module::grass`],
/// as `mg areas` makes it in place) in the GIT `git`.
pub fn edits(git: ResKey, steps: Vec<Step>) -> Vec<Edit> {
    let list = || properties_path().field(OVERRIDES);
    steps
        .into_iter()
        .map(|step| match step {
            Step::Property { label, value } => Edit::SetField {
                key: git,
                path: properties_path(),
                label: label.into(),
                value: Some(value),
            },
            Step::Field { index, label, value } => Edit::SetField {
                key: git,
                path: override_path(index),
                label: label.into(),
                value: Some(value),
            },
            Step::Insert { index, item } => {
                Edit::InsertItem { key: git, path: list(), list: LIST.into(), index, item }
            }
            Step::Remove(index) => {
                Edit::RemoveItem { key: git, path: list(), list: LIST.into(), index }
            }
        })
        .collect()
}

/// The edits that turn the tileset's grass off or on in the GIT `git`
/// (whose root is `root`): [`disabled_plan`].
pub fn set_disabled(git: ResKey, root: &Struct, disabled: bool) -> Vec<Edit> {
    edits(git, disabled_plan(root.child(PROPERTIES), disabled))
}

/// The edits that make the GIT `git` (whose root is `root`) grow `o` on
/// its material: [`set_plan`], as `mg areas --grass` does.
pub fn set_override(git: ResKey, root: &Struct, o: &GrassOverride) -> Vec<Edit> {
    edits(git, set_plan(root.child(PROPERTIES), o))
}

/// The edits that move the grass the GIT whose root is `root` grows on
/// material `from` to material `to` (in place of what grows there):
/// [`move_plan`].
pub fn move_override(git: ResKey, root: &Struct, from: u32, to: u32) -> Vec<Edit> {
    edits(git, move_plan(root.child(PROPERTIES), from, to))
}

/// The edits that take the overrides for `material` out of the GIT whose
/// root is `root`: [`remove_plan`].
pub fn remove_override(git: ResKey, root: &Struct, material: u32) -> Vec<Edit> {
    edits(git, remove_plan(root.child(PROPERTIES), material))
}

/// How many kinds of grass the tiles of one model grow ([`tufts`]'s
/// `variant`), each tile the one its place picks ([`variant`]): with a
/// clump's ground the same for every tile of a model, a field of one
/// model showed the same clumps tile after tile, a grid seen from above
/// (`grass_over_a_field_of_one_tile_looks`). Each is made and uploaded
/// once, as a model's one grass was.
pub const VARIANTS: u32 = 4;

/// Which of a model's [`VARIANTS`] the tile at `column` and `row` grows:
/// the same every time, neighbours mostly apart.
pub fn variant(column: u32, row: u32) -> u32 {
    // (splitmix64 of the place.)
    let mut z = (u64::from(column) << 32 | u64::from(row)).wrapping_add(0x9e37_79b9_7f4a_7c15);
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    ((z ^ (z >> 31)) % u64::from(VARIANTS)) as u32
}

/// The most tufts one tile grows (a denser tile's are thinned evenly): a
/// 10 by 10 meter field at density 20.
pub const MAX_TUFTS: usize = 2000;

/// Grass grows in clumps, as the client's does: a clump to this many
/// square meters of its ground, at random, its tufts (the density's share
/// of that ground, so that grass is as dense on average) within
/// [`CLUMP_REACH`] of its middle, on the faces its material is on. The
/// two were fitted to the client's picture of a rural field
/// (`shadows_ttr01_grass_day_off`, Grass Detail High) beside the area
/// view's from its camera: the share of the near ground bare, at three
/// sizes of a neighborhood, within 0.02 of the client's; evenly
/// scattered, the view had a fifth as much bare ground, and none of the
/// client's dark clumps. Sparse grass has few tufts a clump, so that 0.3
/// a square meter looks scattered, as the client's white tufts did
/// (`reading_grass_tufts`). How the engine itself places them is not
/// read.
pub const CLUMP_GROUND: f32 = 5.0;
/// How far a clump's tufts stand from its middle, at most, meters.
pub const CLUMP_REACH: f32 = 1.4;

/// The grass grown on a tile's walkmesh `faces` (its model's space): for
/// each material in `growing`, tufts in clumps over its faces
/// ([`CLUMP_GROUND`]), as many to a square meter as the density says on
/// average. A tuft is the texture's quarter
/// picked at random (its four tufts, two by two), standing upright and
/// turned at random, as tall and as wide as the height (a little less, at
/// random), both of its sides drawn; lit as the ground beneath it is (the
/// game lights grass without its faces' directions), in the colours of
/// its kind. `None` where nothing grows. `name` is the tile model's and
/// `variant` one of its [`VARIANTS`] (by the tile's place, [`variant`]):
/// the same model and variant grow the same grass, every time.
///
/// Its meshes are skins of two bones: the model's root, which holds the
/// tufts' feet, and [`SWAY_NODE`], which carries their tops: posed by
/// [`sway`], the tops sway in the wind and the feet stay.
///
/// A mesh's tufts are in no order of place, so that any number of the
/// first of them are spread over all of its faces: [`thinned`] keeps the
/// first, for grass seen far off.
pub fn tufts(
    name: &str,
    variant: u32,
    faces: &[WalkFace],
    growing: &[(u32, GrassLook)],
) -> Option<Model> {
    let mut nodes = vec![
        Node::new(&format!("{name}_grass"), NodeKind::Dummy),
        Node { parent: Some(0), ..Node::new(&format!("{name}_grass_sway"), NodeKind::Dummy) },
    ];
    let seed = name.bytes().fold(0xcbf2_9ce4_8422_2325u64, |h, b| {
        (h ^ u64::from(b)).wrapping_mul(0x0000_0100_0000_01b3)
    }) ^ u64::from(variant).wrapping_mul(0x9e37_79b9_7f4a_7c15);
    let mut rng = fastrand::Rng::with_seed(seed);
    let keep = kept_on(faces, growing);
    for (material, look) in growing {
        let mut mesh = Mesh {
            diffuse: look.diffuse,
            ambient: look.ambient,
            render: true,
            textures: [Some(look.texture.clone()), None, None, None],
            ..Mesh::default()
        };
        let mut grown = Vec::new();
        let on: Vec<&WalkFace> = faces.iter().filter(|f| f.material == *material).collect();
        let boxes: Vec<(Vec2, Vec2)> = on.iter().map(|f| flat_box(f)).collect();
        // (A clump's tufts, as many on average as the density gives its
        // ground.)
        let a_clump = look.density * keep * CLUMP_GROUND;
        for (i, f) in on.iter().enumerate() {
            let clumps = chance(area(f) / CLUMP_GROUND, &mut rng);
            if clumps == 0 {
                continue;
            }
            // (Where its clumps' tufts may land: the face first, then
            // those of its material within reach of it.)
            let (lo, hi) = (boxes[i].0 - CLUMP_REACH, boxes[i].1 + CLUMP_REACH);
            let reached = |(a, b): &(Vec2, Vec2)| a.cmple(hi).all() && b.cmpge(lo).all();
            let nearby: Vec<&WalkFace> = std::iter::once(*f)
                .chain(
                    (on.iter().zip(&boxes).enumerate())
                        .filter_map(|(j, (g, b))| (j != i && reached(b)).then_some(*g)),
                )
                .collect();
            for _ in 0..clumps {
                let middle = point_on(f, &mut rng);
                for _ in 0..chance(a_clump, &mut rng) {
                    let foot = near(&nearby, middle, CLUMP_REACH, &mut rng);
                    grown.push(tuft(foot, look.height, &mut rng));
                }
            }
        }

        if grown.is_empty() {
            continue;
        }
        // (Put in no order by a sequence of their own: the tufts are those
        // they were before they were put so.)
        fastrand::Rng::with_seed(seed ^ u64::from(*material)).shuffle(&mut grown);
        for t in &grown {
            add_tuft(&mut mesh, t);
        }
        mesh.source = (0..mesh.vertices.len() as u32).collect();
        mesh.source_uv = mesh.source.clone();
        // (A tuft's feet held by the root, its top by the sway.)
        let weights = (0..mesh.vertices.len()).map(|v| {
            let bone = u16::from(v % TUFT_VERTICES >= 2);
            [(bone, 1.0), (0, 0.0), (0, 0.0), (0, 0.0)]
        });
        mesh.extra = MeshExtra::Skin(Skin {
            bones: vec![0, SWAY_NODE],
            weights: weights.collect(),
            inverse_bind: Vec::new(),
        });
        let mut node =
            Node::new(&format!("{name}_grass{material}"), NodeKind::Mesh(Box::new(mesh)));
        node.parent = Some(0);
        nodes.push(node);
    }
    if nodes.len() == 2 {
        return None;
    }
    nodes[0].children = (1..nodes.len()).collect();
    Some(Model { name: format!("{name}_grass"), nodes, animation_scale: 1.0, ..Model::default() })
}

/// The node of a tile's grass ([`tufts`]) that carries its tufts' tops.
pub const SWAY_NODE: usize = 1;

/// Where a grass tuft's top stands at `time` (seconds) from where it
/// stands still, in an area of `WindPower` `wind` (0 to 2), meters: all
/// the tufts together, north and south, as the client's sway
/// (`reading_grass_sway_by_clock` in `client_render.rs`, 2026-10-30): at
/// 2 between 0.055 m south and 0.098 m north of their feet (1.2 m tufts),
/// a swing about 4 s long; at 1 between 0.027 and 0.045 m, slower (a fall
/// from a peak longer than 3.4 s: a swing of 7 s taken). The swing is
/// taken to be a sine's, the same for tufts of any height; how it swings
/// between its ends, and at 0, were not read: there they stand still.
pub fn sway(wind: i64, time: f32) -> Vec3 {
    let (south, north, swing) = match wind {
        ..=0 => return Vec3::ZERO,
        1 => (0.027, 0.045, 7.0),
        _ => (0.055, 0.098, 4.0),
    };
    let (middle, reach) = ((north - south) / 2.0, (north + south) / 2.0);
    let turn = (time / swing).fract() * std::f32::consts::TAU;
    Vec3::Y * (middle + reach * turn.sin())
}

/// How much of the walkmesh of an area's tiles is of each surface
/// material (surfacemat.2da row: square meters), by the ARE's
/// `Tile_List` and the tileset's tiles: where grass of the area's own on
/// a material can grow. A material no tile has grows nothing.
pub fn materials_of(
    game: &mg_rules::GameData,
    tileset: &mg_set::Tileset,
    are: &Struct,
) -> std::collections::BTreeMap<u32, f32> {
    let mut out = std::collections::BTreeMap::new();
    // (A tile model's faces, read once: an area has few models.)
    let mut by_model: std::collections::HashMap<&str, Vec<(u32, f32)>> =
        std::collections::HashMap::new();
    for t in are.list("Tile_List").unwrap_or_default() {
        let id = t.integer("Tile_ID").and_then(|i| usize::try_from(i).ok());
        let Some(tile) = id.and_then(|i| tileset.tiles.get(i)) else { continue };
        if tile.model.is_empty() {
            continue;
        }
        let faces = by_model.entry(tile.model.as_str()).or_insert_with(|| {
            let walk = crate::walk::Walkmesh::of_tile(game, &tile.model);
            walk.map(|w| w.faces.iter().map(|f| (f.material, area(f))).collect())
                .unwrap_or_default()
        });
        for &(material, a) in faces.iter() {
            *out.entry(material).or_insert(0.0) += a;
        }
    }
    out
}

fn area(f: &WalkFace) -> f32 {
    let [a, b, c] = f.corners;
    (b - a).cross(c - a).length() / 2.0
}

/// The share of the tufts `growing` asks for that a tile of walkmesh
/// `faces` grows: all, or as many as [`MAX_TUFTS`] (thinned alike for
/// every kind, so that none crowds out another).
fn kept_on(faces: &[WalkFace], growing: &[(u32, GrassLook)]) -> f32 {
    let wanted: f32 = growing
        .iter()
        .map(|(m, l)| {
            let on: f32 = faces.iter().filter(|f| f.material == *m).map(area).sum();
            on * l.density
        })
        .sum();
    if wanted > MAX_TUFTS as f32 { MAX_TUFTS as f32 / wanted } else { 1.0 }
}

/// How many tufts to a square meter each mesh of [`tufts`]'s grass of
/// the same tile and kinds has, in order.
pub fn densities(grass: &Model, faces: &[WalkFace], growing: &[(u32, GrassLook)]) -> Vec<f32> {
    let keep = kept_on(faces, growing);
    let prefix = &grass.name;
    grass
        .nodes
        .iter()
        .filter(|n| n.mesh().is_some())
        .map(|n| {
            // (Its node is named for its material: `<tile>_grass<material>`.)
            let material = n.name.strip_prefix(prefix.as_str()).and_then(|m| m.parse::<u32>().ok());
            let look = growing.iter().find(|(m, _)| Some(*m) == material);
            look.map_or(f32::INFINITY, |(_, l)| l.density * keep)
        })
        .collect()
}

/// A tuft's corners and its corners' places on the texture.
struct Tuft {
    corners: [Vec3; 4],
    uvs: [[f32; 2]; 4],
}

/// `count` made a whole number by chance: its whole part, and one more as
/// often as its fraction says (on average, `count`).
fn chance(count: f32, rng: &mut fastrand::Rng) -> usize {
    count.floor() as usize + usize::from(rng.f32() < count.fract())
}

/// A random point of face `f`, evenly over it.
fn point_on(f: &WalkFace, rng: &mut fastrand::Rng) -> Vec3 {
    let (mut s, mut t) = (rng.f32(), rng.f32());
    if s + t > 1.0 {
        (s, t) = (1.0 - s, 1.0 - t);
    }
    let [a, b, c] = f.corners;
    a + (b - a) * s + (c - a) * t
}

/// Face `f`'s box seen from above: its least and its most X and Y.
fn flat_box(f: &WalkFace) -> (Vec2, Vec2) {
    let [a, b, c] = f.corners.map(Vec3::truncate);
    (a.min(b).min(c), a.max(b).max(c))
}

/// The point of face `f` above or below `p` (seen from above), if it is
/// on the face.
fn on_face(f: &WalkFace, p: Vec2) -> Option<Vec3> {
    let [a, b, c] = f.corners;
    let (ab, ac, ap) = ((b - a).truncate(), (c - a).truncate(), p - a.truncate());
    let d = ab.perp_dot(ac);
    if d.abs() < 1e-9 {
        return None;
    }
    let (s, t) = (ap.perp_dot(ac) / d, ab.perp_dot(ap) / d);
    (s >= 0.0 && t >= 0.0 && s + t <= 1.0).then(|| a + (b - a) * s + (c - a) * t)
}

/// How many times a tuft of a clump is put again, where it fell off its
/// faces, before it goes at the clump's middle.
const NEAR_TRIES: usize = 8;

/// A random point of `faces` within `reach` of `middle` (seen from
/// above), evenly over the circle, or over as much of it as is on them
/// (`middle` if every try falls off them).
fn near(faces: &[&WalkFace], middle: Vec3, reach: f32, rng: &mut fastrand::Rng) -> Vec3 {
    for _ in 0..NEAR_TRIES {
        let (r, a) = (reach * rng.f32().sqrt(), rng.f32() * std::f32::consts::TAU);
        let p = middle.truncate() + Vec2::from_angle(a) * r;
        if let Some(p) = faces.iter().find_map(|f| on_face(f, p)) {
            return p;
        }
    }
    middle
}

/// A tuft standing at `foot`.
fn tuft(foot: Vec3, height: f32, rng: &mut fastrand::Rng) -> Tuft {
    // (Its foot a little in the ground, so that it stands on a slope.)
    let foot = foot - Vec3::Z * 0.05;
    let size = height * (0.75 + 0.25 * rng.f32());
    let across =
        Vec3::from((Vec2::from_angle(rng.f32() * std::f32::consts::TAU) * size / 2.0, 0.0));
    let up = Vec3::Z * size;
    let corners = [foot - across, foot + across, foot + across + up, foot - across + up];
    let cell = rng.usize(0..4);
    let (u, v) = ((cell % 2) as f32 * 0.5, (cell / 2) as f32 * 0.5);
    let uvs = [[u, v], [u + 0.5, v], [u + 0.5, v + 0.5], [u, v + 0.5]];
    Tuft { corners, uvs }
}

/// The vertices and faces a tuft has in a mesh (four each, in a row).
pub const TUFT_VERTICES: usize = 4;
pub const TUFT_FACES: usize = 4;

fn add_tuft(mesh: &mut Mesh, t: &Tuft) {
    let first = mesh.vertices.len() as u32;
    mesh.vertices.extend(t.corners.map(|p| p.to_array()));
    mesh.normals.extend([[0.0, 0.0, 1.0]; 4]);
    mesh.uvs[0].extend(t.uvs);
    // Both sides.
    for v in [[0, 1, 2], [0, 2, 3], [0, 2, 1], [0, 3, 2]] {
        mesh.faces.push(Face { vertices: v.map(|i| first + i), material: 0 });
    }
}

/// Grass seen from this far away (meters, from the eye to the nearest of
/// it) is drawn with as many of its tufts as make this many to a square
/// meter at most, by [`level`] and [`share`]: all of them near, where the
/// view looks as the game's does; fewer further off, where a tuft is a
/// few pixels high, and only of dense grass (the tilesets' 5 a square
/// meter are all drawn out to 180 m, half of them past it); never none, so
/// that grass is drawn out to the end of what the view shows.
pub const LEVELS: [(f32, f32); 4] = [(0.0, f32::INFINITY), (45.0, 10.0), (90.0, 5.0), (180.0, 2.5)];

/// Grass thins to each of [`LEVELS`] past the first over a stretch from
/// its distance on, this share of it (45 m's to 56.25 m), in
/// [`FADE_STEPS`] steps: a level begun at once was a band where tufts were
/// gone at a tile's edge, which a careful eye saw as the camera moved.
/// (Nearer than a level's distance grass is as dense as before it.)
pub const FADE: f32 = 0.25;
pub const FADE_STEPS: usize = 8;

/// How many levels grass is drawn at ([`level`]): the first, and
/// [`FADE_STEPS`] towards each of the others, the last of them the level
/// itself.
pub const STEPS: usize = 1 + (LEVELS.len() - 1) * FADE_STEPS;

/// Which of the [`STEPS`] levels grass this far from the eye is drawn at:
/// 0 near, all of it; `k * FADE_STEPS` once past the stretch from
/// [`LEVELS`]`[k]` ([`FADE`]), and the steps between over it.
pub fn level(distance: f32) -> usize {
    for k in (1..LEVELS.len()).rev() {
        let from = LEVELS[k].0;
        let end = from * (1.0 + FADE);
        if distance >= end {
            return k * FADE_STEPS;
        }
        if distance >= from {
            let along = (distance - from) / (end - from);
            let step = (along * FADE_STEPS as f32) as usize + 1;
            return (k - 1) * FADE_STEPS + step.min(FADE_STEPS);
        }
    }
    0
}

/// The share of its tufts grass of `density` (tufts to a square meter)
/// is drawn with at `level` (of [`STEPS`]): at a level of [`LEVELS`], as
/// many as make its most to a square meter; between, a step of the way
/// from the one before.
pub fn share(level: usize, density: f32) -> f32 {
    let at = |k: usize| {
        let most = LEVELS[k.min(LEVELS.len() - 1)].1;
        if density > most { most / density } else { 1.0 }
    };
    let level = level.min(STEPS - 1);
    if level == 0 {
        return 1.0;
    }
    let (k, step) = ((level - 1) / FADE_STEPS + 1, (level - 1) % FADE_STEPS + 1);
    let t = step as f32 / FADE_STEPS as f32;
    at(k - 1) + (at(k) - at(k - 1)) * t
}

/// How many of a mesh's `all` tufts are kept of a `share` of them: one at
/// least (of a mesh that has any).
pub fn kept(all: usize, share: f32) -> usize {
    ((all as f32 * share).ceil() as usize).clamp(all.min(1), all)
}

/// `grass` ([`tufts`]) with the first `share` of each of its meshes'
/// tufts ([`kept`]): spread over its faces as all of them are.
pub fn thinned(grass: &Model, share: f32) -> Model {
    let mut out = grass.clone();
    for node in &mut out.nodes {
        let NodeKind::Mesh(mesh) = &mut node.kind else { continue };
        let kept = kept(mesh.vertices.len() / TUFT_VERTICES, share);
        mesh.vertices.truncate(kept * TUFT_VERTICES);
        mesh.normals.truncate(kept * TUFT_VERTICES);
        mesh.uvs[0].truncate(kept * TUFT_VERTICES);
        mesh.faces.truncate(kept * TUFT_FACES);
        mesh.source.truncate(kept * TUFT_VERTICES);
        mesh.source_uv.truncate(kept * TUFT_VERTICES);
        if let MeshExtra::Skin(skin) = &mut mesh.extra {
            skin.weights.truncate(kept * TUFT_VERTICES);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use mg_core::{ResRef, ResType};
    use mg_gff::Value;

    fn git_with(items: Vec<Struct>, disabled: Option<u8>) -> Struct {
        let mut props = Struct::new(100);
        props.set("AmbientSndDay", Value::Int(3));
        if let Some(d) = disabled {
            props.set(DISABLED, Value::Byte(d));
        }
        let mut overrides = Struct::new(OVERRIDES_ID);
        overrides.set(LIST, Value::List(items));
        props.set(OVERRIDES, Value::Struct(overrides));
        let mut root = Struct::new(u32::MAX);
        root.set(PROPERTIES, Value::Struct(props));
        root
    }

    fn look(texture: &str, density: f32) -> GrassLook {
        GrassLook {
            texture: texture.into(),
            density,
            height: 1.0,
            ambient: [0.1, 0.2, 0.3],
            diffuse: [0.4, 0.5, 0.6],
        }
    }

    /// Applies edits as the workspace would (enough of it for these).
    fn apply(root: &mut Struct, edits: &[Edit]) {
        for e in edits {
            match e {
                Edit::SetField { path, label, value, .. } => {
                    let s = path.get_mut(root).unwrap();
                    match value {
                        Some(v) => s.set(label, v.clone()),
                        None => {
                            s.remove(label);
                        }
                    }
                }
                Edit::InsertItem { path, list, index, item, .. } => {
                    let s = path.get_mut(root).unwrap();
                    s.list_mut(list).unwrap().insert(*index, item.clone());
                }
                Edit::RemoveItem { path, list, index, .. } => {
                    let s = path.get_mut(root).unwrap();
                    s.list_mut(list).unwrap().remove(*index);
                }
                Edit::SetResource { .. } => unreachable!(),
            }
        }
    }

    #[test]
    fn overrides_are_added_one_a_material_and_removed() {
        let key = ResKey::new(ResRef::from_str("field").unwrap(), ResType::GIT);
        let mut props = Struct::new(100);
        props.set("AmbientSndDay", Value::Int(3));
        let mut root = Struct::new(0);
        root.set(PROPERTIES, Value::Struct(props));
        let dirt = GrassOverride { material: 1, look: look("mgdirt", 2.0) };
        let edits = set_override(key, &root, &dirt);
        apply(&mut root, &edits);
        let overrides = root.child(PROPERTIES).unwrap().child(OVERRIDES).unwrap();
        assert_eq!(overrides.id, OVERRIDES_ID);
        let wood = GrassOverride { material: 5, look: look("mgwood", 1.0) };
        let edits = set_override(key, &root, &wood);
        apply(&mut root, &edits);
        // Dirt's again: in its place.
        let dirt2 = GrassOverride { material: 1, look: look("mgdirt2", 4.0) };
        let edits = set_override(key, &root, &dirt2);
        apply(&mut root, &edits);
        assert_eq!(AreaGrass::read(&root).overrides, [dirt2, wood.clone()]);
        let edits = remove_override(key, &root, 1);
        apply(&mut root, &edits);
        assert_eq!(AreaGrass::read(&root).overrides, [wood]);
        let edits = set_disabled(key, &root, true);
        apply(&mut root, &edits);
        assert!(AreaGrass::read(&root).default_disabled);
        assert!(set_disabled(key, &root, true).is_empty(), "off already");
        // What else the GIT's properties hold stays.
        assert!(root.child(PROPERTIES).unwrap().integer("AmbientSndDay") == Some(3));
    }

    /// The area view's edits and `mg areas`'s changes in place are one
    /// plan: from any GIT, the two leave the same GIT (the review of
    /// 2026-10-21 found them written twice, and they had drifted once).
    #[test]
    fn the_area_view_s_edits_leave_what_mg_areas_leaves() {
        let key = ResKey::new(ResRef::from_str("field").unwrap(), ResType::GIT);
        let grass = GrassOverride { material: 3, look: look("grass", 2.0) };
        let mut kept = grass.item();
        kept.set("MgUnknown", Value::Int(7));
        let wood = GrassOverride { material: 5, look: look("mgwood", 1.0) };
        let gits = [
            git_with(vec![], None),
            git_with(vec![kept.clone(), wood.item()], Some(0)),
            git_with(vec![grass.item(), wood.item(), kept.clone()], Some(1)),
            {
                // No list at all.
                let mut root = Struct::new(u32::MAX);
                root.set(PROPERTIES, Value::Struct(Struct::new(100)));
                root
            },
        ];
        let denser = GrassOverride { material: 3, look: look("grass", 6.0) };
        type InPlace = Box<dyn Fn(&mut Struct)>;
        type Edits = Box<dyn Fn(&Struct) -> Vec<Edit>>;
        let (d1, d2) = (denser.clone(), denser.clone());
        let cases: Vec<(&str, InPlace, Edits)> = vec![
            (
                "set",
                Box::new(move |p| _ = mg_module::grass::set_in(p, &d1)),
                Box::new(move |r| set_override(key, r, &d2)),
            ),
            (
                "remove",
                Box::new(|p| _ = mg_module::grass::remove_in(p, 3)),
                Box::new(move |r| remove_override(key, r, 3)),
            ),
            (
                "off",
                Box::new(|p| _ = mg_module::grass::set_disabled_in(p, true)),
                Box::new(move |r| set_disabled(key, r, true)),
            ),
            (
                "on",
                Box::new(|p| _ = mg_module::grass::set_disabled_in(p, false)),
                Box::new(move |r| set_disabled(key, r, false)),
            ),
            (
                "move",
                Box::new(|p| {
                    let steps = move_plan(Some(p), 3, 5);
                    mg_module::grass::apply(p, &steps);
                }),
                Box::new(move |r| move_override(key, r, 3, 5)),
            ),
        ];
        for (i, git) in gits.iter().enumerate() {
            for (what, in_place, edits) in &cases {
                let mut by_edits = git.clone();
                apply(&mut by_edits, &edits(git));
                let mut by_mg = git.clone();
                in_place(by_mg.child_mut(PROPERTIES).unwrap());
                assert_eq!(by_edits, by_mg, "{what} on GIT {i}");
            }
        }
    }

    #[test]
    fn an_override_is_changed_in_place_and_keeps_what_else_it_holds() {
        let key = ResKey::new(ResRef::from_str("field").unwrap(), ResType::GIT);
        let mut item = GrassOverride { material: 3, look: look("grass", 2.0) }.item();
        item.set("MgUnknown", Value::Int(7));
        let wood = GrassOverride { material: 5, look: look("mgwood", 1.0) };
        let mut root = git_with(vec![item, wood.item()], None);
        let mut denser = GrassOverride { material: 3, look: look("grass", 6.0) };
        denser.look.diffuse = [1.0, 0.0, 0.0];
        let edits = set_override(key, &root, &denser);
        // The density and the diffuse color's three.
        assert_eq!(edits.len(), 4, "{edits:?}");
        apply(&mut root, &edits);
        assert_eq!(AreaGrass::read(&root).overrides, [denser.clone(), wood.clone()]);
        let list = root.child(PROPERTIES).unwrap().child(OVERRIDES).unwrap().list(LIST).unwrap();
        assert_eq!(list[0].integer("MgUnknown"), Some(7));
        assert!(set_override(key, &root, &denser).is_empty());
        // Two for one material (a module's own, or an older Moonglow's):
        // the last is changed and the other goes, as `mg areas` leaves it.
        let mut twice = git_with(vec![denser.item(), wood.item(), denser.item()], None);
        let thinner = GrassOverride { material: 3, look: look("grass", 1.0) };
        let edits = set_override(key, &twice, &thinner);
        apply(&mut twice, &edits);
        assert_eq!(AreaGrass::read(&twice).overrides, [wood.clone(), thinner]);
        // Moved to wood's material: in place of wood's grass.
        let edits = move_override(key, &root, 3, 5);
        apply(&mut root, &edits);
        let moved = GrassOverride { material: 5, ..denser };
        assert_eq!(AreaGrass::read(&root).overrides, std::slice::from_ref(&moved));
        let list = root.child(PROPERTIES).unwrap().child(OVERRIDES).unwrap().list(LIST).unwrap();
        assert_eq!(list[0].integer("MgUnknown"), Some(7));
        // From a material it has none for: nothing.
        assert!(move_override(key, &root, 9, 1).is_empty());
        // The one moved to before the one moved: that one's index is one less.
        let mut dirt = moved.clone();
        dirt.material = 1;
        let mut root = git_with(vec![wood.item(), dirt.item()], None);
        let edits = move_override(key, &root, 1, 5);
        apply(&mut root, &edits);
        assert_eq!(AreaGrass::read(&root).overrides, [GrassOverride { material: 5, ..dirt }]);
    }

    fn field(material: u32) -> Vec<WalkFace> {
        // A 10 by 10 meter square, in two faces.
        let [a, b, c, d] =
            [[-5.0, -5.0], [5.0, -5.0], [5.0, 5.0], [-5.0, 5.0]].map(|[x, y]| Vec3::new(x, y, 0.0));
        vec![WalkFace { corners: [a, b, c], material }, WalkFace { corners: [a, c, d], material }]
    }

    fn tuft_count(m: &Model) -> usize {
        m.nodes.iter().filter_map(Node::mesh).map(|mesh| mesh.faces.len() / 4).sum()
    }

    #[test]
    fn tufts_grow_by_density_on_their_material_only() {
        let mut faces = field(3);
        faces.extend(field(4).into_iter().map(|mut f| {
            f.corners.iter_mut().for_each(|p| p.x += 10.0);
            f
        }));
        let grass = [(3, look("grass", 5.0))];
        let m = tufts("tst01_a01_01", 0, &faces, &grass).unwrap();
        let n = tuft_count(&m);
        assert!((450..=550).contains(&n), "about 5 a square meter on 100: {n}");
        let mesh = m.nodes.iter().find_map(Node::mesh).unwrap();
        assert_eq!(mesh.textures[0].as_deref(), Some("grass"));
        assert_eq!(mesh.diffuse, [0.4, 0.5, 0.6]);
        assert!(mesh.vertices.iter().all(|p| (-5.6..=5.6).contains(&p[0])), "on the grass only");
        // As tall as the height, at most.
        assert!(mesh.vertices.iter().all(|p| p[2] <= 1.0));
        assert!(mesh.vertices.iter().any(|p| p[2] > 0.7));
        // The same model grows the same grass.
        assert_eq!(tufts("tst01_a01_01", 0, &faces, &grass), Some(m));
        assert!(tufts("tst01_a01_01", 0, &faces, &[(5, look("grass", 5.0))]).is_none());
        // Ten times as dense: thinned to the most a tile grows.
        let dense = tufts("tst01_a01_01", 0, &faces, &[(3, look("grass", 50.0))]).unwrap();
        assert!(tuft_count(&dense) <= MAX_TUFTS + 20);
        assert!(tuft_count(&dense) > MAX_TUFTS * 9 / 10);
    }

    /// Grass far off is drawn with the first of its tufts: they are spread
    /// over all of the ground (no corner bare), as many as the share says.
    #[test]
    fn thinned_grass_is_spread_over_all_of_its_ground() {
        let m = tufts("tst01_a01_01", 0, &field(3), &[(3, look("grass", 5.0))]).unwrap();
        let all = tuft_count(&m);
        let quarters = [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)];
        let in_quarter = |mesh: &Mesh, (qx, qy): (f32, f32)| {
            let feet = mesh.vertices.chunks(TUFT_VERTICES).filter(|t| {
                let foot = (Vec3::from(t[0]) + Vec3::from(t[1])) / 2.0;
                foot.x * qx > 0.0 && foot.y * qy > 0.0
            });
            feet.count() as f32
        };
        let full = m.nodes.iter().find_map(Node::mesh).unwrap();
        for share in [1.0, 0.5, 0.25, 0.125] {
            let thin = thinned(&m, share);
            let n = tuft_count(&thin);
            assert_eq!(n, kept(all, share));
            let mesh = thin.nodes.iter().find_map(Node::mesh).unwrap();
            // In each quarter of the field, about its share of the tufts
            // there (the clumps make the quarters' own differ).
            for q in quarters {
                let (here, there) = (in_quarter(mesh, q), in_quarter(full, q) * share);
                assert!(
                    (here - there).abs() <= there * 0.4 + 3.0,
                    "{share}: {here}, {there} in {q:?}"
                );
            }
            // The first of all of them, as they are.
            assert_eq!(mesh.vertices[..], full.vertices[..mesh.vertices.len()]);
            assert_eq!(mesh.faces[..], full.faces[..mesh.faces.len()]);
        }
        assert_eq!(kept(7, 0.01), 1, "one at least");
        assert_eq!(kept(0, 0.5), 0);
    }

    /// Grass grows in clumps, as the client's does (Bee AI's ask): a field
    /// of the tilesets' density has bare ground between them, which
    /// evenly scattered tufts don't leave, as dense on average; sparse
    /// grass has a tuft or two a clump.
    #[test]
    fn grass_grows_in_clumps_as_dense_on_average() {
        let feet = |m: &Model| -> Vec<Vec2> {
            let mesh = m.nodes.iter().find_map(Node::mesh).unwrap();
            let foot = |t: &[[f32; 3]]| ((Vec3::from(t[0]) + Vec3::from(t[1])) / 2.0).truncate();
            mesh.vertices.chunks(TUFT_VERTICES).map(foot).collect()
        };
        // The share of the field's square meters with no tuft's foot.
        let bare = |feet: &[Vec2]| {
            let mut cells = [0usize; 100];
            for p in feet {
                let (x, y) = ((p.x + 5.0).clamp(0.0, 9.99), (p.y + 5.0).clamp(0.0, 9.99));
                cells[y as usize * 10 + x as usize] += 1;
            }
            cells.iter().filter(|&&c| c == 0).count() as f32 / 100.0
        };
        let mut bares = Vec::new();
        for name in ["tst01_a01_01", "tst01_b02_01", "tst01_c03_01", "tst01_d04_01"] {
            let m = tufts(name, 0, &field(3), &[(3, look("grass", 5.0))]).unwrap();
            let f = feet(&m);
            assert!((400..=600).contains(&f.len()), "about 5 a square meter: {}", f.len());
            assert!(f.iter().all(|p| p.x.abs() <= 5.0 && p.y.abs() <= 5.0), "on the field");
            bares.push(bare(&f));
        }
        // (Evenly, at 5 a square meter, one square meter in 150 is bare.)
        let mean = bares.iter().sum::<f32>() / bares.len() as f32;
        assert!((0.1..0.5).contains(&mean), "bare ground between the clumps: {bares:?}");
        // Sparse: as many as the density says, a clump of one or two.
        let mut sparse = 0;
        for k in 0..20 {
            let m = tufts(&format!("tst01_s{k:02}"), 0, &field(3), &[(3, look("grass", 0.3))]);
            sparse += m.map_or(0, |m| tuft_count(&m));
        }
        assert!((480..=720).contains(&sparse), "0.3 a square meter on 20 fields: {sparse}");
    }

    /// Far off, grass is drawn no denser than its level's most to a square
    /// meter: the tilesets' (5) all of it out to the last level, dense
    /// grass thinned from nearer; near, all of any.
    #[test]
    fn grass_is_thinned_by_its_density_far_off() {
        let at = |k: usize| k * FADE_STEPS;
        assert_eq!(share(0, 20.0), 1.0);
        assert_eq!(share(0, 1000.0), 1.0);
        assert_eq!(share(at(1), 5.0), 1.0);
        assert_eq!(share(at(2), 5.0), 1.0);
        assert_eq!(share(at(3), 5.0), 0.5);
        assert_eq!(share(at(1), 20.0), 0.5);
        assert_eq!(share(at(3), 20.0), 0.125);
        // Each mesh's density: as grown, its kind's (thinned to the most a
        // tile grows).
        let mut faces = field(3);
        faces.extend(field(1));
        let growing = [(1, look("mgdirt", 1.0)), (3, look("grass", 50.0))];
        let m = tufts("tst", 0, &faces, &growing).unwrap();
        let d = densities(&m, &faces, &growing);
        let keep = MAX_TUFTS as f32 / (100.0 * 51.0);
        assert_eq!(d, [1.0 * keep, 50.0 * keep]);
    }

    /// Grass thins over a stretch from each level on, a step at a time,
    /// never thicker further off: dense grass (30 a square meter) past
    /// 45 m lost two thirds of its tufts at once.
    #[test]
    fn grass_thins_over_a_stretch_from_each_level() {
        assert_eq!(level(44.9), 0, "near: all of it, as before");
        assert_eq!(level(45.0), 1, "the first step");
        assert_eq!(level(56.25), FADE_STEPS);
        assert_eq!(level(89.9), FADE_STEPS);
        assert_eq!(level(112.5), 2 * FADE_STEPS);
        assert_eq!(level(225.0), STEPS - 1);
        assert_eq!(level(1000.0), STEPS - 1);
        // Between 45 and 56.25 m, every step, in order.
        let steps: Vec<usize> = (0..=120).map(|d| level(44.5 + d as f32 / 10.0)).collect();
        assert!(steps.windows(2).all(|w| w[1] >= w[0] && w[1] - w[0] <= 1), "{steps:?}");
        assert_eq!((steps[0], *steps.last().unwrap()), (0, FADE_STEPS));
        for density in [1.0, 5.0, 12.0, 30.0, 500.0] {
            let mut last = 1.0;
            for (k, s) in (0..STEPS).map(|k| (k, share(k, density))) {
                assert!(s <= last && s > 0.0, "{density}: step {k} {s} after {last}");
                // (A step a fade's eighth of the way.)
                assert!(
                    last - s <= (1.0 - 10.0 / 30.0) / FADE_STEPS as f32 + 1e-6 || density > 30.0
                );
                last = s;
            }
        }
    }

    /// The tiles of one model grow a few kinds of grass by their places:
    /// each kind its own clumps, the same every time; a field of one model
    /// has them all, neighbours mostly apart (it showed the same clumps
    /// tile after tile, a grid from above).
    #[test]
    fn tiles_of_one_model_grow_their_grass_by_their_places() {
        let grow = |v: u32| tufts("tst01_a01_01", v, &field(3), &[(3, look("grass", 5.0))]);
        let kinds: Vec<Model> = (0..VARIANTS).map(|v| grow(v).unwrap()).collect();
        for (i, a) in kinds.iter().enumerate() {
            assert_eq!(grow(i as u32).as_ref(), Some(a), "the same every time");
            for b in &kinds[i + 1..] {
                assert_ne!(a, b, "each its own");
            }
            // As dense as the others.
            assert!((400..=600).contains(&tuft_count(a)), "{}", tuft_count(a));
        }
        let field: Vec<Vec<u32>> =
            (0..8).map(|row| (0..8).map(|column| variant(column, row)).collect()).collect();
        assert_eq!(
            field,
            (0..8)
                .map(|row| (0..8).map(|c| variant(c, row)).collect::<Vec<_>>())
                .collect::<Vec<_>>()
        );
        let mut used = [0; VARIANTS as usize];
        let mut alike = 0;
        for row in 0..8 {
            for column in 0..8 {
                used[field[row][column] as usize] += 1;
                if column > 0 && field[row][column] == field[row][column - 1] {
                    alike += 1;
                }
            }
        }
        assert!(used.iter().all(|&n| n >= 8), "every kind: {used:?}");
        assert!(alike < 56 / 2, "neighbours mostly apart: {alike} of 56 alike");
    }

    /// A tuft's feet are held by the grass's root and its top carried by
    /// the sway, in every mesh, thinned too.
    #[test]
    fn a_tuft_s_top_is_carried_by_the_sway_and_its_feet_held() {
        let m = tufts("tst", 0, &field(1), &[(1, look("grass", 2.0))]).unwrap();
        assert_eq!(m.nodes[SWAY_NODE].parent, Some(0));
        for m in [m.clone(), thinned(&m, 0.5)] {
            for mesh in m.nodes.iter().filter_map(Node::mesh) {
                let MeshExtra::Skin(skin) = &mesh.extra else { panic!("not a skin") };
                assert_eq!(skin.bones, [0, SWAY_NODE]);
                assert_eq!(skin.weights.len(), mesh.vertices.len());
                for (v, w) in skin.weights.iter().enumerate() {
                    // (Corners 2 and 3 the top: higher than 0 and 1.)
                    let top = v % TUFT_VERTICES >= 2;
                    assert_eq!(w[0], (u16::from(top), 1.0));
                    let foot = mesh.vertices[v - v % TUFT_VERTICES + v % 2];
                    assert_eq!(mesh.vertices[v][2] > foot[2], top);
                }
            }
        }
    }

    /// The sway reaches as far as the client's tufts' do, north and south,
    /// a swing of 4 s at wind 2, slower and shorter at 1, none at 0.
    #[test]
    fn grass_sways_as_far_and_as_fast_as_the_client_s() {
        let range = |wind: i64, swing: f32| {
            let ys: Vec<f32> =
                (0..400).map(|i| sway(wind, i as f32 * swing / 400.0)).map(|v| v.y).collect();
            let (lo, hi) = ys.iter().fold((f32::MAX, f32::MIN), |(l, h), &y| (l.min(y), h.max(y)));
            (lo, hi)
        };
        let (lo, hi) = range(2, 4.0);
        assert!((lo + 0.055).abs() < 1e-3 && (hi - 0.098).abs() < 1e-3, "{lo} {hi}");
        let (lo, hi) = range(1, 7.0);
        assert!((lo + 0.027).abs() < 1e-3 && (hi - 0.045).abs() < 1e-3, "{lo} {hi}");
        assert_eq!(range(0, 4.0), (0.0, 0.0));
        // (A swing on, where it was.)
        assert!((sway(2, 1.3) - sway(2, 5.3)).length() < 1e-4);
        assert!((sway(1, 1.3) - sway(1, 8.3)).length() < 1e-4);
        assert_eq!(sway(2, 0.7).x, 0.0);
    }

    #[test]
    fn each_kind_of_grass_is_a_mesh_of_its_own() {
        let mut faces = field(3);
        faces.extend(field(1));
        let m = tufts("tst", 0, &faces, &[(1, look("mgdirt", 1.0)), (3, look("grass", 1.0))]);
        let m = m.unwrap();
        let textures: Vec<&str> = m
            .nodes
            .iter()
            .filter_map(Node::mesh)
            .filter_map(|mesh| mesh.textures[0].as_deref())
            .collect();
        assert_eq!(textures, ["mgdirt", "grass"]);
    }
}
