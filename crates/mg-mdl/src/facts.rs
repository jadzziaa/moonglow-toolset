//! What a model is, in numbers: its nodes by kind, its faces (all, and
//! those of meshes the game draws), the box its drawn geometry fills at
//! rest, its skins' bones, the textures and materials it names, its
//! supermodel and its animations. The same for a model read from text or
//! compiled: `mg model info` prints it.

use std::collections::{BTreeMap, BTreeSet};

use crate::compile::{quat_mul, quat_unit, rotate};
use crate::{Classification, MeshExtra, Model, NodeKind, Quat, Vec3};

/// A model's facts ([`Facts::of`]).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Facts {
    pub name: String,
    pub supermodel: Option<String>,
    pub classification: Classification,
    pub animation_scale: f32,
    /// Every node.
    pub nodes: usize,
    /// Nodes by their ASCII type (`dummy`, `trimesh`, `skin`,
    /// `danglymesh`, `animmesh`, `aabb`, `emitter`, `light`, `reference`,
    /// `camera`).
    pub node_types: BTreeMap<&'static str, usize>,
    /// The triangles of every mesh (a walkmesh's `aabb` and those not
    /// drawn too), as [`crate::Mesh::triangles`] counts them.
    pub faces: usize,
    /// The triangles of the meshes the game draws: `render` on, not an
    /// `aabb` walkmesh.
    pub faces_rendered: usize,
    /// The vertices of every mesh.
    pub vertices: usize,
    /// The box around the vertices of the meshes the game draws, in the
    /// model's space at rest (each node placed by its parents' positions,
    /// orientations and scales); a skin's at its bind pose, where its
    /// vertices lie in its own node. `None`: it draws no mesh. Emitters'
    /// particles, lights and walkmeshes are not in it.
    pub bounds: Option<(Vec3, Vec3)>,
    /// Each skin mesh: its node's name, the bones its weights move (the
    /// distinct nodes given a weight above zero) and its triangles.
    pub skins: Vec<(String, usize, usize)>,
    /// The bones of all its skins together (each node once).
    pub bones: usize,
    /// The textures it names (lower case): its meshes' (`bitmap`,
    /// `texture0` to `texture3`), its emitters' and its lights' flares'.
    pub textures: BTreeSet<String>,
    /// The materials (`.mtr`) its meshes name, lower case.
    pub materials: BTreeSet<String>,
    /// Its own animations: name, length and transition time (seconds),
    /// and how many events each has.
    pub animations: Vec<(String, f32, f32, usize)>,
}

impl Facts {
    pub fn of(model: &Model) -> Facts {
        let mut f = Facts {
            name: model.name.clone(),
            supermodel: model.supermodel.clone().filter(|s| !s.eq_ignore_ascii_case("null")),
            classification: model.classification,
            animation_scale: model.animation_scale,
            nodes: model.nodes.len(),
            ..Default::default()
        };
        let rest = rest_places(model);
        let mut bones_of_all = BTreeSet::new();
        for (i, node) in model.nodes.iter().enumerate() {
            *f.node_types.entry(node.kind.type_name()).or_default() += 1;
            match &node.kind {
                NodeKind::Mesh(mesh) => {
                    let triangles = mesh.triangles().count();
                    f.faces += triangles;
                    f.vertices += mesh.vertices.len();
                    f.textures.extend(mesh.textures.iter().filter_map(named).map(lower));
                    f.materials.extend(named(&mesh.material).map(lower));
                    let drawn = mesh.render && !matches!(mesh.extra, MeshExtra::Aabb(_));
                    if drawn {
                        f.faces_rendered += triangles;
                        let (q, t, s) =
                            rest.get(i).copied().unwrap_or((crate::IDENTITY, [0.0; 3], 1.0));
                        for v in &mesh.vertices {
                            let r = rotate(q, *v);
                            let p = [t[0] + s * r[0], t[1] + s * r[1], t[2] + s * r[2]];
                            let (lo, hi) = f.bounds.get_or_insert((p, p));
                            for k in 0..3 {
                                lo[k] = lo[k].min(p[k]);
                                hi[k] = hi[k].max(p[k]);
                            }
                        }
                    }
                    if let MeshExtra::Skin(skin) = &mesh.extra {
                        let bones: BTreeSet<usize> = (skin.weights.iter().flatten())
                            .filter(|(_, w)| *w > 0.0)
                            .filter_map(|(b, _)| skin.bones.get(usize::from(*b)).copied())
                            .collect();
                        f.skins.push((node.name.clone(), bones.len(), triangles));
                        bones_of_all.extend(bones);
                    }
                }
                NodeKind::Emitter(e) => f.textures.extend(named(&e.texture).map(lower)),
                NodeKind::Light(l) => {
                    let flares =
                        l.flare_textures.iter().filter(|t| named(&Some(t.to_string())).is_some());
                    f.textures.extend(flares.map(|t| lower(t)));
                }
                _ => {}
            }
        }
        f.bones = bones_of_all.len();
        f.animations = (model.animations.iter())
            .map(|a| (a.name.clone(), a.length, a.transtime, a.events.len()))
            .collect();
        f
    }
}

/// A name given (not empty, not `NULL`).
fn named(t: &Option<String>) -> Option<&str> {
    t.as_deref().filter(|t| !t.is_empty() && !t.eq_ignore_ascii_case("null"))
}

fn lower(s: &str) -> String {
    s.to_ascii_lowercase()
}

/// Each node's place in the model at rest, as the renderer composes it:
/// its rotation, position and scale from the root's (a parent before its
/// children; a node whose parent comes after it stands as if it had none).
fn rest_places(model: &Model) -> Vec<(Quat, Vec3, f32)> {
    let mut out: Vec<(Quat, Vec3, f32)> = Vec::with_capacity(model.nodes.len());
    for (i, node) in model.nodes.iter().enumerate() {
        let local = quat_unit(node.orientation);
        let place = match node.parent.filter(|&p| p < i).and_then(|p| out.get(p)) {
            Some(&(q, t, s)) => {
                let moved = rotate(q, node.position);
                let at = [t[0] + s * moved[0], t[1] + s * moved[1], t[2] + s * moved[2]];
                (quat_unit(quat_mul(q, local)), at, s * node.scale)
            }
            None => (local, node.position, node.scale),
        };
        out.push(place);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A placeable of a root, a scaled and turned dummy, a mesh under it
    /// (drawn), a mesh not drawn, a skin weighted to two of three bones,
    /// an emitter and an animation: its facts read from its text and from
    /// it compiled are the same (the box within float rounding). The
    /// bones are counted by the weights, not the skin's list of bones,
    /// which a compiled skin may make of every node.
    const MODEL: &str = "newmodel mg_facts\nsetsupermodel mg_facts NULL\n\
classification placeable\nsetanimationscale 1\nbeginmodelgeom mg_facts\n\
node dummy mg_facts\n  parent NULL\nendnode\n\
node dummy turn\n  parent mg_facts\n  position 10 0 0\n  orientation 0 0 1 1.5707964\n  scale 2\nendnode\n\
node trimesh plane\n  parent turn\n  position 1 0 0\n  bitmap Mg_Tex\n  materialname mg_mat\n\
  verts 3\n    0 0 0\n    1 0 0\n    0 1 0\n\
  faces 1\n    0 1 2 1 0 0 0 0\n\
  tverts 3\n    0 0 0\n    1 0 0\n    0 1 0\nendnode\n\
node trimesh hidden\n  parent mg_facts\n  render 0\n  bitmap mg_hidden\n\
  verts 3\n    0 0 -50\n    1 0 -50\n    0 1 -50\n\
  faces 1\n    0 1 2 1 0 0 0 0\nendnode\n\
node dummy bone_a\n  parent mg_facts\nendnode\n\
node dummy bone_b\n  parent mg_facts\nendnode\n\
node skin body\n  parent mg_facts\n  bitmap mg_skin\n\
  verts 3\n    0 0 0\n    0 0 1\n    0 1 1\n\
  faces 1\n    0 1 2 1 0 0 0 0\n\
  weights 3\n    bone_a 1\n    bone_a 0.5 bone_b 0.5\n    bone_b 1\nendnode\n\
node emitter smoke\n  parent mg_facts\n  update Fountain\n  render Normal\n  blend Normal\n\
  texture fxpa_smoke\nendnode\n\
endmodelgeom mg_facts\n\
newanim open mg_facts\n  length 1.5\n  transtime 0.25\n  animroot mg_facts\n  event 0.5 hit\n\
  node dummy mg_facts\n    parent NULL\n  endnode\ndoneanim open mg_facts\n\
donemodel mg_facts\n";

    fn close(a: Vec3, b: Vec3) -> bool {
        a.iter().zip(b).all(|(a, b)| (a - b).abs() < 1e-4)
    }

    #[test]
    fn a_model_s_facts_are_the_same_from_its_text_and_compiled() {
        let text = Model::read(MODEL.as_bytes()).unwrap();
        let facts = Facts::of(&text);
        assert_eq!(facts.name, "mg_facts");
        assert_eq!(facts.supermodel, None);
        assert_eq!(facts.nodes, 8);
        let types: Vec<(&str, usize)> = facts.node_types.iter().map(|(k, v)| (*k, *v)).collect();
        assert_eq!(types, [("dummy", 4), ("emitter", 1), ("skin", 1), ("trimesh", 2)]);
        assert_eq!((facts.faces, facts.faces_rendered), (3, 2));
        // The plane: 1 m along `turn`'s X, which stands at (10, 0, 0) turned
        // a quarter about Z and doubled, so at (10, 2, 0), its corners
        // there and at (10, 4, 0) and (8, 2, 0); the skin at its bind pose,
        // (0, 0, 0) to (0, 1, 1). The hidden mesh
        // (at z -50) is not in it.
        let (lo, hi) = facts.bounds.unwrap();
        assert!(close(lo, [0.0, 0.0, 0.0]) && close(hi, [10.0, 4.0, 1.0]), "{lo:?} {hi:?}");
        assert_eq!(facts.skins, [("body".to_string(), 2, 1)]);
        assert_eq!(facts.bones, 2);
        let textures: Vec<&str> = facts.textures.iter().map(String::as_str).collect();
        assert_eq!(textures, ["fxpa_smoke", "mg_hidden", "mg_skin", "mg_tex"]);
        assert_eq!(facts.materials.iter().collect::<Vec<_>>(), ["mg_mat"]);
        assert_eq!(facts.animations, [("open".to_string(), 1.5, 0.25, 1)]);

        let compiled = crate::compile::compile_named_with(
            MODEL.as_bytes(),
            "mg_facts",
            &|_| None,
            &|_| None,
            &|_| None,
        )
        .unwrap();
        let binary = Model::read(&compiled.binary).unwrap();
        let from_binary = Facts::of(&binary);
        let (lo, hi) = from_binary.bounds.unwrap();
        assert!(close(lo, [0.0, 0.0, 0.0]) && close(hi, [10.0, 4.0, 1.0]), "{lo:?} {hi:?}");
        assert_eq!(Facts { bounds: None, ..from_binary }, Facts { bounds: None, ..facts });
    }

    #[test]
    fn a_model_with_no_drawn_mesh_has_no_bounds() {
        let model = Model::read(
            b"newmodel e\nsetsupermodel e base\nclassification effect\nbeginmodelgeom e\n\
node dummy e\n  parent NULL\nendnode\nendmodelgeom e\ndonemodel e\n",
        )
        .unwrap();
        let facts = Facts::of(&model);
        assert_eq!(
            (facts.bounds, facts.faces, facts.supermodel.as_deref()),
            (None, 0, Some("base"))
        );
    }
}
