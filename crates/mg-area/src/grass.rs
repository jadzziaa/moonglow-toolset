//! An area's grass, for the editor: the edits that set it in a GIT (as
//! undoable workspace edits, each changing only what it must) and the
//! tufts the area view draws. What it is and how the engine keeps it:
//! [`mg_module::grass`], re-exported here.

use glam::{Vec2, Vec3};
use mg_edit::{Edit, GffPath};
use mg_gff::{Struct, Value};
use mg_mdl::{Face, Mesh, Model, Node, NodeKind};
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

/// The edits that turn the tileset's grass off or on in the GIT `git`.
pub fn set_disabled(git: ResKey, disabled: bool) -> Edit {
    Edit::SetField {
        key: git,
        path: properties_path(),
        label: DISABLED.into(),
        value: Some(Value::Byte(u8::from(disabled))),
    }
}

/// The edits that add `o` to the GIT `git` (whose root is `root`): into
/// its list, made if the GIT has none; in place of the override the list
/// has for its material.
pub fn add_override(git: ResKey, root: &Struct, o: &GrassOverride) -> Vec<Edit> {
    let props = root.child(PROPERTIES);
    let Some(list) = props.and_then(|p| p.child(OVERRIDES)).and_then(|s| s.list(LIST)) else {
        let mut overrides = Struct::new(OVERRIDES_ID);
        overrides.set(LIST, Value::List(vec![o.item()]));
        return vec![Edit::SetField {
            key: git,
            path: properties_path(),
            label: OVERRIDES.into(),
            value: Some(Value::Struct(overrides)),
        }];
    };
    let mut edits = Vec::new();
    // (Last first: the indices before stay as they are.)
    let same =
        list.iter().enumerate().rev().filter(|(_, s)| {
            GrassOverride::read(s).is_some_and(|there| there.material == o.material)
        });
    for (i, _) in same {
        edits.push(remove_item(git, i));
    }
    let left = list.len() - edits.len();
    edits.push(Edit::InsertItem {
        key: git,
        path: properties_path().field(OVERRIDES),
        list: LIST.into(),
        index: left,
        item: o.item(),
    });
    edits
}

/// The index of the item the GIT whose root is `root` has for `material`
/// (the last, should there be two).
fn item_for(root: &Struct, material: u32) -> Option<usize> {
    let list = root.child(PROPERTIES)?.child(OVERRIDES)?.list(LIST)?;
    list.iter().rposition(|s| GrassOverride::read(s).is_some_and(|o| o.material == material))
}

/// The edits that make the GIT whose root is `root` grow `o` on its
/// material: the item it has for the material, changed where it differs
/// (what else the item holds stays), else a new one; the others for the
/// material go, as `mg_module::grass::set_in` (`mg areas`) leaves it (they
/// stayed, the area's grass differing by how it was set).
pub fn set_override(git: ResKey, root: &Struct, o: &GrassOverride) -> Vec<Edit> {
    let Some(index) = item_for(root, o.material) else { return add_override(git, root, o) };
    let list = (root.child(PROPERTIES).and_then(|p| p.child(OVERRIDES)))
        .and_then(|s| s.list(LIST))
        .unwrap_or(&[]);
    let there = &list[index];
    let mut fields = o.fields();
    fields.push((TEXTURE, Value::String(o.look.texture.as_bytes().to_vec())));
    let mut edits: Vec<Edit> = fields
        .into_iter()
        .filter(|(label, v)| there.get(label) != Some(v))
        .map(|(label, value)| Edit::SetField {
            key: git,
            path: override_path(index),
            label: label.into(),
            value: Some(value),
        })
        .collect();
    // (The item is the last for its material: the others are before it,
    // taken out last first after its fields are set.)
    let same = |s: &Struct| GrassOverride::read(s).is_some_and(|t| t.material == o.material);
    let others = list[..index].iter().enumerate().rev().filter(|(_, s)| same(s));
    edits.extend(others.map(|(i, _)| remove_item(git, i)));
    edits
}

/// The edits that move the grass the GIT whose root is `root` grows on
/// material `from` to material `to` (in place of what grows there).
pub fn move_override(git: ResKey, root: &Struct, from: u32, to: u32) -> Vec<Edit> {
    let Some(index) = item_for(root, from).filter(|_| from != to) else { return Vec::new() };
    // (Those for `to` go first, after the item if they are: so that its
    // index stays, or before it, so that it moves down by as many.)
    let gone = remove_override(git, root, to);
    let before = gone
        .iter()
        .filter(|e| matches!(e, Edit::RemoveItem { index: i, .. } if *i < index))
        .count();
    let mut edits = gone;
    edits.push(Edit::SetField {
        key: git,
        path: override_path(index - before),
        label: MATERIAL.into(),
        value: Some(Value::Byte(to.min(255) as u8)),
    });
    edits
}

fn remove_item(git: ResKey, index: usize) -> Edit {
    Edit::RemoveItem {
        key: git,
        path: properties_path().field(OVERRIDES),
        list: LIST.into(),
        index,
    }
}

/// The edits that take the overrides for `material` out of the GIT whose
/// root is `root`.
pub fn remove_override(git: ResKey, root: &Struct, material: u32) -> Vec<Edit> {
    let list = (root.child(PROPERTIES).and_then(|p| p.child(OVERRIDES)))
        .and_then(|s| s.list(LIST))
        .unwrap_or(&[]);
    let same =
        list.iter().enumerate().rev().filter(|(_, s)| {
            GrassOverride::read(s).is_some_and(|there| there.material == material)
        });
    same.map(|(i, _)| remove_item(git, i)).collect()
}

/// The most tufts one tile grows (a denser tile's are thinned evenly): a
/// 10 by 10 meter field at density 20.
pub const MAX_TUFTS: usize = 2000;

/// The grass grown on a tile's walkmesh `faces` (its model's space): for
/// each material in `growing`, tufts scattered over its faces, as many to
/// a square meter as the density says. A tuft is the texture's quarter
/// picked at random (its four tufts, two by two), standing upright and
/// turned at random, as tall and as wide as the height (a little less, at
/// random), both of its sides drawn; lit as the ground beneath it is (the
/// game lights grass without its faces' directions), in the colours of
/// its kind. `None` where nothing grows. `name` is the tile model's: the
/// same model grows the same grass.
///
/// A mesh's tufts are in no order of place, so that any number of the
/// first of them are spread over all of its faces: [`thinned`] keeps the
/// first, for grass seen far off.
pub fn tufts(name: &str, faces: &[WalkFace], growing: &[(u32, GrassLook)]) -> Option<Model> {
    let mut nodes = vec![Node::new(&format!("{name}_grass"), NodeKind::Dummy)];
    let seed = name.bytes().fold(0xcbf2_9ce4_8422_2325u64, |h, b| {
        (h ^ u64::from(b)).wrapping_mul(0x0000_0100_0000_01b3)
    });
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
        for f in faces.iter().filter(|f| f.material == *material) {
            let count = area(f) * look.density * keep;
            let mut n = count.floor() as usize;
            if rng.f32() < count.fract() {
                n += 1;
            }
            for _ in 0..n {
                grown.push(tuft(f, look.height, &mut rng));
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
        let mut node =
            Node::new(&format!("{name}_grass{material}"), NodeKind::Mesh(Box::new(mesh)));
        node.parent = Some(0);
        nodes.push(node);
    }
    if nodes.len() == 1 {
        return None;
    }
    nodes[0].children = (1..nodes.len()).collect();
    Some(Model { name: format!("{name}_grass"), nodes, animation_scale: 1.0, ..Model::default() })
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

/// A tuft at a random point of face `f`.
fn tuft(f: &WalkFace, height: f32, rng: &mut fastrand::Rng) -> Tuft {
    let (mut s, mut t) = (rng.f32(), rng.f32());
    if s + t > 1.0 {
        (s, t) = (1.0 - s, 1.0 - t);
    }
    let [a, b, c] = f.corners;
    // (Its foot a little in the ground, so that it stands on a slope.)
    let foot = a + (b - a) * s + (c - a) * t - Vec3::Z * 0.05;
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

/// Which of [`LEVELS`] grass this far from the eye is drawn at.
pub fn level(distance: f32) -> usize {
    LEVELS.iter().rposition(|(from, _)| distance >= *from).unwrap_or(0)
}

/// The share of its tufts grass of `density` (tufts to a square meter)
/// is drawn with at `level`.
pub fn share(level: usize, density: f32) -> f32 {
    let most = LEVELS[level.min(LEVELS.len() - 1)].1;
    if density > most { most / density } else { 1.0 }
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
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use mg_core::{ResRef, ResType};

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
        let edits = add_override(key, &root, &dirt);
        apply(&mut root, &edits);
        let overrides = root.child(PROPERTIES).unwrap().child(OVERRIDES).unwrap();
        assert_eq!(overrides.id, OVERRIDES_ID);
        let wood = GrassOverride { material: 5, look: look("mgwood", 1.0) };
        let edits = add_override(key, &root, &wood);
        apply(&mut root, &edits);
        let dirt2 = GrassOverride { material: 1, look: look("mgdirt2", 4.0) };
        let edits = add_override(key, &root, &dirt2);
        apply(&mut root, &edits);
        assert_eq!(AreaGrass::read(&root).overrides, [wood.clone(), dirt2]);
        let edits = remove_override(key, &root, 1);
        apply(&mut root, &edits);
        assert_eq!(AreaGrass::read(&root).overrides, [wood]);
        apply(&mut root, &[set_disabled(key, true)]);
        assert!(AreaGrass::read(&root).default_disabled);
        // What else the GIT's properties hold stays.
        assert!(root.child(PROPERTIES).unwrap().integer("AmbientSndDay") == Some(3));
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
        let m = tufts("tst01_a01_01", &faces, &grass).unwrap();
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
        assert_eq!(tufts("tst01_a01_01", &faces, &grass), Some(m));
        assert!(tufts("tst01_a01_01", &faces, &[(5, look("grass", 5.0))]).is_none());
        // Ten times as dense: thinned to the most a tile grows.
        let dense = tufts("tst01_a01_01", &faces, &[(3, look("grass", 50.0))]).unwrap();
        assert!(tuft_count(&dense) <= MAX_TUFTS + 20);
        assert!(tuft_count(&dense) > MAX_TUFTS * 9 / 10);
    }

    /// Grass far off is drawn with the first of its tufts: they are spread
    /// over all of the ground (no corner bare), as many as the share says.
    #[test]
    fn thinned_grass_is_spread_over_all_of_its_ground() {
        let m = tufts("tst01_a01_01", &field(3), &[(3, look("grass", 5.0))]).unwrap();
        let all = tuft_count(&m);
        for share in [1.0, 0.5, 0.25, 0.125] {
            let thin = thinned(&m, share);
            let n = tuft_count(&thin);
            assert_eq!(n, kept(all, share));
            let mesh = thin.nodes.iter().find_map(Node::mesh).unwrap();
            // In each quarter of the field, about its share of them.
            for (qx, qy) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
                let feet = mesh.vertices.chunks(TUFT_VERTICES).filter(|t| {
                    let foot = (Vec3::from(t[0]) + Vec3::from(t[1])) / 2.0;
                    foot.x * qx > 0.0 && foot.y * qy > 0.0
                });
                let here = feet.count() as f32;
                assert!(here > n as f32 / 4.0 * 0.6, "{share}: {here} of {n} in ({qx}, {qy})");
            }
            // The first of all of them, as they are.
            let full = m.nodes.iter().find_map(Node::mesh).unwrap();
            assert_eq!(mesh.vertices[..], full.vertices[..mesh.vertices.len()]);
            assert_eq!(mesh.faces[..], full.faces[..mesh.faces.len()]);
        }
        assert_eq!(kept(7, 0.01), 1, "one at least");
        assert_eq!(kept(0, 0.5), 0);
    }

    /// Far off, grass is drawn no denser than its level's most to a square
    /// meter: the tilesets' (5) all of it out to the last level, dense
    /// grass thinned from nearer; near, all of any.
    #[test]
    fn grass_is_thinned_by_its_density_far_off() {
        assert_eq!(share(0, 20.0), 1.0);
        assert_eq!(share(0, 1000.0), 1.0);
        assert_eq!(share(1, 5.0), 1.0);
        assert_eq!(share(2, 5.0), 1.0);
        assert_eq!(share(3, 5.0), 0.5);
        assert_eq!(share(1, 20.0), 0.5);
        assert_eq!(share(3, 20.0), 0.125);
        // Each mesh's density: as grown, its kind's (thinned to the most a
        // tile grows).
        let mut faces = field(3);
        faces.extend(field(1));
        let growing = [(1, look("mgdirt", 1.0)), (3, look("grass", 50.0))];
        let m = tufts("tst", &faces, &growing).unwrap();
        let d = densities(&m, &faces, &growing);
        let keep = MAX_TUFTS as f32 / (100.0 * 51.0);
        assert_eq!(d, [1.0 * keep, 50.0 * keep]);
    }

    #[test]
    fn each_kind_of_grass_is_a_mesh_of_its_own() {
        let mut faces = field(3);
        faces.extend(field(1));
        let m = tufts("tst", &faces, &[(1, look("mgdirt", 1.0)), (3, look("grass", 1.0))]);
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
