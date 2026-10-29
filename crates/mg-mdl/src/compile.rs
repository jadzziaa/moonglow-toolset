//! The native compiler: an ASCII model to a compiled one, in process (the
//! plan's Phase 9, stages B and C).
//!
//! `mg-mdl`'s reader already makes of the text what a compiled model holds
//! (a vertex per corner, normals from smoothing groups); [`crate::binary_write`]
//! writes it. Between them, here: tangents for a render hint that came
//! without, a walkmesh's tree where the text has none, a skin's inverse
//! binds from the model at rest, controllers the
//! node's type does not have (left out, as the game leaves them), and the
//! part numbers, by the order of the text and from the model's compiled
//! version where there is one.

use std::collections::HashMap;

use crate::ctrl::flags;
use crate::{AabbEntry, Mesh, MeshExtra, Model, NodeKind, Vec3, dot, sub};
use thiserror::Error;

use crate::binary_write::{self as binary, PartNumbers};

/// Why a text does not compile.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum CompileError {
    #[error("already a compiled model")]
    NotAscii,
    #[error(transparent)]
    Read(#[from] crate::MdlError),
    #[error("{node}: {bones} bones (a skin holds at most 64)")]
    Bones { node: String, bones: usize },
    #[error(transparent)]
    Write(#[from] binary::CompileError),
}

/// What a model is compiled against.
#[derive(Debug, Clone, Copy, Default)]
pub struct Sources<'a> {
    /// Its supermodel and that one's part numbers
    /// ([`PartNumbers::read`] of its compiled file, where it has one).
    pub supermodel: Option<(&'a Model, &'a PartNumbers)>,
    /// The compiled model of its own name, where it was compiled before:
    /// its nodes keep the numbers they have there.
    pub existing: Option<(&'a Model, &'a PartNumbers)>,
    /// Materials (`.mtr`) by name, where the game would find them: a mesh
    /// with no render hint takes its material's for its tangents.
    pub materials: Option<Materials<'a>>,
}

/// Finds a material (`.mtr`) by name: its file.
#[derive(Clone, Copy)]
pub struct Materials<'a>(pub &'a dyn Fn(&str) -> Option<Vec<u8>>);

impl std::fmt::Debug for Materials<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Materials")
    }
}

/// A material's render hint (`renderhint`), lower case.
fn material_hint(mtr: &[u8]) -> Option<String> {
    String::from_utf8_lossy(mtr).lines().find_map(|l| {
        let mut w = l.split_whitespace();
        w.next().filter(|k| k.eq_ignore_ascii_case("renderhint"))?;
        w.next().map(str::to_ascii_lowercase)
    })
}

/// A compiled model and what the compiler has to say about it.
#[derive(Debug, Clone, PartialEq)]
pub struct Compiled {
    pub binary: Vec<u8>,
    /// What was left out or made up, with the text's line where it has one
    /// (0: none).
    pub notes: Vec<(usize, String)>,
}

/// A mesh's tangents as the game's compiler makes them (nwn-mdl-spec
/// MDL-MESH-043), from its first UV set, triangle by triangle in the index
/// list: each adds its tangent to its corners' vertices, the first to
/// reach a vertex giving its handedness; one of the other handedness
/// takes a copy of the vertex instead (once per vertex and handedness;
/// an animated mesh's vertex is overwritten), its corner in the index list
/// rewritten, the faces left. Then each is made square to the normal and
/// unit. A triangle flat in UV adds its normals' mean, turned (y, z, x).
fn engine_tangents(mesh: &mut Mesh) {
    let count = mesh.vertices.len();
    if mesh.uvs[0].len() != count || mesh.normals.len() != count {
        return;
    }
    let animated = matches!(mesh.extra, MeshExtra::Anim(_));
    let mut tris: Vec<[u32; 3]> = mesh.triangles().collect();
    let mut tan: Vec<Vec3> = vec![[0.0; 3]; count];
    let mut hand: Vec<f32> = vec![0.0; count];
    let mut copies: HashMap<(u32, i8), u32> = HashMap::new();
    let mut cloned = false;
    for tri in &mut tris {
        let [a, b, c] = tri.map(|v| v as usize);
        if a >= mesh.vertices.len() || b >= mesh.vertices.len() || c >= mesh.vertices.len() {
            continue;
        }
        let (p, uv) = (&mesh.vertices, &mesh.uvs[0]);
        let (e1, e2) = (sub(p[b], p[a]), sub(p[c], p[a]));
        let (du1, dv1) = (uv[b][0] - uv[a][0], uv[b][1] - uv[a][1]);
        let (du2, dv2) = (uv[c][0] - uv[a][0], uv[c][1] - uv[a][1]);
        let det = du1 * dv2 - du2 * dv1;
        let (t, w): (Vec3, f32) = if det != 0.0 {
            (
                std::array::from_fn(|k| (dv2 * e1[k] - dv1 * e2[k]) / det),
                if det >= 0.0 { 1.0 } else { -1.0 },
            )
        } else {
            let n = &mesh.normals;
            let mean: Vec3 = std::array::from_fn(|k| (n[a][k] + n[b][k] + n[c][k]) / 3.0);
            ([mean[1], mean[2], mean[0]], 1.0)
        };
        for corner in tri.iter_mut() {
            let mut v = *corner as usize;
            if hand[v] == 0.0 {
                hand[v] = w;
            } else if hand[v] != w {
                if animated {
                    hand[v] = w;
                } else {
                    let key = (*corner, w as i8);
                    let copy = match copies.get(&key) {
                        Some(&i) => i as usize,
                        None => {
                            let i = clone_vertex(mesh, v);
                            tan.push([0.0; 3]);
                            hand.push(w);
                            copies.insert(key, i as u32);
                            i
                        }
                    };
                    *corner = copy as u32;
                    v = copy;
                    cloned = true;
                }
            }
            for k in 0..3 {
                tan[v][k] += t[k];
            }
        }
    }
    mesh.tangents = (0..mesh.vertices.len())
        .map(|v| {
            let n = mesh.normals[v];
            let d = dot(n, tan[v]);
            let t = crate::engine_normalize(std::array::from_fn(|k| tan[v][k] - n[k] * d));
            [t[0], t[1], t[2], hand[v]]
        })
        .collect();
    if cloned {
        mesh.drawn = tris;
    }
}

/// Appends a copy of vertex `v` of every per-vertex stream; its index.
fn clone_vertex(mesh: &mut Mesh, v: usize) -> usize {
    fn copy<T: Clone>(list: &mut Vec<T>, v: usize) {
        if let Some(x) = list.get(v).cloned() {
            list.push(x);
        }
    }
    copy(&mut mesh.vertices, v);
    copy(&mut mesh.normals, v);
    for set in &mut mesh.uvs {
        copy(set, v);
    }
    copy(&mut mesh.colors, v);
    copy(&mut mesh.source, v);
    copy(&mut mesh.source_uv, v);
    match &mut mesh.extra {
        MeshExtra::Skin(s) => copy(&mut s.weights, v),
        MeshExtra::Dangly(d) => copy(&mut d.constraints, v),
        _ => {}
    }
    mesh.vertices.len() - 1
}

/// A walkmesh's tree over its faces, as entries in pre-order: each box
/// split across its longest side at the middle face, down to one face.
pub fn aabb_tree(mesh: &Mesh) -> Vec<AabbEntry> {
    struct Leaf {
        min: Vec3,
        max: Vec3,
        centre: Vec3,
        face: i32,
    }
    fn build(leaves: &mut [Leaf], out: &mut Vec<AabbEntry>) {
        let (mut min, mut max) = (leaves[0].min, leaves[0].max);
        for l in leaves.iter() {
            for k in 0..3 {
                min[k] = min[k].min(l.min[k]);
                max[k] = max[k].max(l.max[k]);
            }
        }
        if let [leaf] = leaves {
            out.push(AabbEntry { min, max, face: leaf.face });
            return;
        }
        out.push(AabbEntry { min, max, face: -1 });
        let size = sub(max, min);
        let axis = (0..3).fold(0, |a, k| if size[k] > size[a] { k } else { a });
        leaves.sort_by(|a, b| a.centre[axis].total_cmp(&b.centre[axis]).then(a.face.cmp(&b.face)));
        let (left, right) = leaves.split_at_mut(leaves.len() / 2);
        build(left, out);
        build(right, out);
    }
    let corner = |v: u32| mesh.vertices.get(v as usize).copied().unwrap_or([0.0; 3]);
    let mut leaves: Vec<Leaf> = mesh
        .faces
        .iter()
        .enumerate()
        .map(|(i, f)| {
            let p = f.vertices.map(corner);
            let min = std::array::from_fn(|k| p[0][k].min(p[1][k]).min(p[2][k]));
            let max = std::array::from_fn(|k| p[0][k].max(p[1][k]).max(p[2][k]));
            let centre = std::array::from_fn(|k| (p[0][k] + p[1][k] + p[2][k]) / 3.0);
            Leaf { min, max, centre, face: i as i32 }
        })
        .collect();
    let mut out = Vec::with_capacity(leaves.len() * 2);
    if !leaves.is_empty() {
        build(&mut leaves, &mut out);
    }
    out
}

type Quat = [f32; 4];

fn quat_mul(a: Quat, b: Quat) -> Quat {
    let ([ax, ay, az, aw], [bx, by, bz, bw]) = (a.map(f64::from), b.map(f64::from));
    [
        aw * bx + ax * bw + ay * bz - az * by,
        aw * by - ax * bz + ay * bw + az * bx,
        aw * bz + ax * by - ay * bx + az * bw,
        aw * bw - ax * bx - ay * by - az * bz,
    ]
    .map(|x| x as f32)
}

fn quat_unit(q: Quat) -> Quat {
    let len = q.iter().map(|x| f64::from(*x).powi(2)).sum::<f64>().sqrt();
    if len < 1e-12 { crate::IDENTITY } else { q.map(|x| (f64::from(x) / len) as f32) }
}

fn quat_conjugate(q: Quat) -> Quat {
    [-q[0], -q[1], -q[2], q[3]]
}

fn rotate(q: Quat, v: Vec3) -> Vec3 {
    let p = quat_mul(quat_mul(q, [v[0], v[1], v[2], 0.0]), quat_conjugate(q));
    [p[0], p[1], p[2]]
}

/// Each node's place in the model at rest: its rotation and position from
/// the root's (nodes are in pre-order, a parent before its children).
fn rest_pose(model: &Model) -> Vec<(Quat, Vec3)> {
    let mut out: Vec<(Quat, Vec3)> = Vec::with_capacity(model.nodes.len());
    for node in &model.nodes {
        let local = quat_unit(node.orientation);
        let place = match node.parent.and_then(|p| out.get(p)) {
            Some(&(q, t)) => {
                let moved = rotate(q, node.position);
                (quat_unit(quat_mul(q, local)), [t[0] + moved[0], t[1] + moved[1], t[2] + moved[2]])
            }
            None => (local, node.position),
        };
        out.push(place);
    }
    out
}

/// A skin's inverse binds, one per node of the model: what takes a point
/// of the skin, at rest, into that node's own space (a rotation and a
/// translation).
pub(crate) fn inverse_binds(rest: &[(Quat, Vec3)], skin: usize) -> Vec<(Quat, Vec3)> {
    let Some(&(skin_q, skin_t)) = rest.get(skin) else { return Vec::new() };
    rest.iter()
        .map(|&(q, t)| {
            let back = quat_conjugate(q);
            (quat_unit(quat_mul(back, skin_q)), rotate(back, sub(skin_t, t)))
        })
        .collect()
}

/// Compiles an ASCII model.
pub fn compile(text: &[u8], sources: &Sources<'_>) -> Result<Compiled, CompileError> {
    if crate::is_binary(text) {
        return Err(CompileError::NotAscii);
    }
    let (mut model, map) = crate::ascii::read_mapped(text)?;
    let mut notes: Vec<(usize, String)> =
        map.notes.iter().map(|n| (n.line, n.message.clone())).collect();
    let line = |node: usize| map.nodes.get(node).map_or(0, |s| s.start);

    // Whether a node's block has a keyword (lines are numbered from 1).
    let text_lines: Vec<&str> = std::str::from_utf8(text).unwrap_or("").lines().collect();
    let says =
        |node: usize, keyword: &str| {
            let Some(span) = map.nodes.get(node).filter(|s| s.start > 0) else { return true };
            text_lines.iter().skip(span.start - 1).take(span.end + 1 - span.start).any(|l| {
                l.split_whitespace().next().is_some_and(|w| w.eq_ignore_ascii_case(keyword))
            })
        };

    // Each node's type, for an animation's nodes of the same name.
    let types: Vec<(String, u32)> = model
        .nodes
        .iter()
        .map(|n| (n.name.to_ascii_lowercase(), binary::node_flags(&n.kind)))
        .collect();
    let rest = rest_pose(&model);
    for (i, node) in model.nodes.iter_mut().enumerate() {
        let node_flags = binary::node_flags(&node.kind);
        let name = node.name.clone();
        node.controllers.retain(|c| {
            let known = binary::controller_id(node_flags, &c.name).is_some();
            if !known {
                notes.push((
                    line(i),
                    format!(
                        "{name}: {} is left out (not a controller of a {})",
                        c.name,
                        node.kind.type_name()
                    ),
                ));
            }
            known
        });
        let NodeKind::Mesh(mesh) = &mut node.kind else { continue };
        // Left unsaid, a mesh's shininess is 1 (the game's compiler's).
        if !says(i, "shininess") {
            mesh.shininess = 1.0;
        }
        if let MeshExtra::Skin(skin) = &mut mesh.extra {
            if skin.bones.len() > 64 {
                return Err(CompileError::Bones { node: name, bones: skin.bones.len() });
            }
            // Where each node is from the skin at rest, for the bones.
            if skin.inverse_bind.is_empty() {
                skin.inverse_bind = inverse_binds(&rest, i);
            }
        }
        // A normal-mapped render hint wants tangents: the text's, else
        // made here as the game makes them. Only `NormalAndSpecMapped`
        // and `NormalTangents` (any case); another word, none
        // (nwn-mdl-spec MDL-MESH-040, -042). A mesh with no hint takes
        // its material's: the one `materialname` names, else `bitmap`
        // (MDL-MESH-041); its own hint stays unsaid.
        let found = || {
            let find = sources.materials?;
            [mesh.material.as_deref(), mesh.textures[0].as_deref()]
                .into_iter()
                .flatten()
                .find_map(|n| (find.0)(n))
                .and_then(|m| material_hint(&m))
        };
        let hint = match mesh.renderhint.clone() {
            Some(h) => Some(h),
            None => found(),
        };
        let hinted = hint.as_deref().is_some_and(|h| {
            h.eq_ignore_ascii_case("normalandspecmapped")
                || h.eq_ignore_ascii_case("normaltangents")
        });
        if hinted && mesh.tangents.is_empty() && !mesh.vertices.is_empty() {
            engine_tangents(mesh);
        }
        if !hinted {
            mesh.tangents.clear();
        }
        if let MeshExtra::Aabb(entries) = &mesh.extra
            && entries.is_empty()
            && !mesh.faces.is_empty()
        {
            notes.push((line(i), format!("{name}: no aabb list, a tree was made of its faces")));
            mesh.extra = MeshExtra::Aabb(aabb_tree(mesh));
        }
    }
    for (a, anim) in model.animations.iter_mut().enumerate() {
        for (i, node) in anim.nodes.iter_mut().enumerate() {
            let own = types.iter().find(|(n, _)| n.eq_ignore_ascii_case(&node.name));
            let node_flags = match own {
                Some((_, f)) => f & !(flags::SKIN | flags::AABB),
                None => binary::flags_for_controllers(&node.controllers),
            };
            let at = map.animations.get(a).and_then(|s| s.nodes.get(i)).map_or(0, |s| s.start);
            let (anim_name, name) = (anim.name.clone(), node.name.clone());
            node.controllers.retain(|c| {
                let known = binary::controller_id(node_flags, &c.name).is_some();
                if !known {
                    notes.push((
                        at,
                        format!(
                            "{anim_name}, {name}: {} is left out (not a controller of this node)",
                            c.name
                        ),
                    ));
                }
                known
            });
        }
    }

    // Animated sets given per vertex of the text: a set per render vertex,
    // each taking its source vertex's (as the mesh's own lists do).
    let sources_of: Vec<(String, Vec<u32>, Vec<u32>)> = model
        .nodes
        .iter()
        .filter_map(|n| {
            let m = n.mesh()?;
            Some((n.name.to_ascii_lowercase(), m.source.clone(), m.source_uv.clone()))
        })
        .collect();
    for anim in &mut model.animations {
        for node in &mut anim.nodes {
            let Some(sets) = &mut node.anim_mesh else { continue };
            let Some((_, source, source_uv)) =
                sources_of.iter().find(|(n, ..)| n.eq_ignore_ascii_case(&node.name))
            else {
                continue;
            };
            // (A set read from text follows the source `verts` and
            // `tverts`; each merged vertex takes its source's entries,
            // nwn-mdl-spec MDL-MESH-062. A set as long as the merged list
            // is no exception: zspec_animtv's five tverts merge into five
            // vertices in another order.)
            for set in &mut sets.vertex_sets {
                if source.iter().all(|&v| (v as usize) < set.len()) {
                    *set = source.iter().map(|&v| set[v as usize]).collect();
                }
            }
            for set in &mut sets.uv_sets {
                if source_uv.iter().all(|&v| (v as usize) < set.len()) {
                    *set = source_uv.iter().map(|&v| set[v as usize]).collect();
                }
            }
        }
    }

    // A node with a see-through child mesh has its children sorted, as
    // the game's compiler sorts them (after numbering, which keeps the
    // text's order): every child that is not a mesh first, then the
    // meshes by transparency hint, by libstdc++'s `std::sort`
    // (nwn-mdl-spec MDL-NODE-012). Not an animation's tree.
    let hint = |m: &Model, i: usize| m.nodes[i].mesh().map(|x| x.transparency_hint as i32);
    for i in 0..model.nodes.len() {
        let children = model.nodes[i].children.clone();
        if !children.iter().any(|&c| hint(&model, c).is_some_and(|h| h > 0)) {
            continue;
        }
        let mut sorted = children;
        let m = &model;
        crate::stdsort::sort(&mut sorted, &|&a, &b| match (hint(m, a), hint(m, b)) {
            (None, Some(_)) => true,
            (Some(x), Some(y)) => x < y,
            _ => false,
        });
        model.nodes[i].children = sorted;
    }
    // The nodes in the order of the tree as it is written: what a skin's
    // bones and bind poses are numbered by (nwn-mdl-spec MDL-SKIN-011),
    // and what the sort changed. (`was`: each node's index before.)
    let was = in_tree_order(&mut model);

    // Numbered in the order of the text.
    // (Those of its nodes left out of the tree among them: each takes a
    // number, past the model's nodes in `order`.)
    let count = model.nodes.len();
    let mut order: Vec<usize> = (0..count + map.left_out.len()).collect();
    let line_of = |i: usize| match i.checked_sub(count) {
        Some(k) => map.left_out[k].start,
        None => line(was[i]),
    };
    order.sort_by_key(|&i| (line_of(i) == 0, line_of(i), i));
    let parts = binary::number_nodes(&model, &order, sources.supermodel, sources.existing);
    notes.sort();
    // (Each mesh's bounds and face records from its text's vertices and
    // faces, as the game's compiler computes them.)
    let meshes: Vec<_> = was.iter().map(|&w| map.meshes.get(w).and_then(Option::as_ref)).collect();
    Ok(Compiled { binary: binary::write_from(&model, &parts, &meshes)?, notes })
}

/// Puts a model's nodes in the order of a depth-first walk of its tree,
/// children in their order, nodes outside it after; every reference to a
/// node follows (parents, children, a skin's bones and bind poses). Returns
/// each node's index before.
fn in_tree_order(model: &mut Model) -> Vec<usize> {
    let n = model.nodes.len();
    let mut order = Vec::with_capacity(n);
    let mut seen = vec![false; n];
    let mut stack: Vec<usize> =
        model.nodes.iter().position(|x| x.parent.is_none()).into_iter().collect();
    while let Some(i) = stack.pop() {
        if i >= n || std::mem::replace(&mut seen[i], true) {
            continue;
        }
        order.push(i);
        stack.extend(model.nodes[i].children.iter().rev());
    }
    order.extend((0..n).filter(|&i| !seen[i]));
    if order.iter().enumerate().all(|(new, &old)| new == old) {
        return order;
    }
    let mut new_of = vec![0; n];
    for (new, &old) in order.iter().enumerate() {
        new_of[old] = new;
    }
    let mut nodes: Vec<crate::Node> = order.iter().map(|&o| model.nodes[o].clone()).collect();
    for node in &mut nodes {
        node.parent = node.parent.map(|p| new_of.get(p).copied().unwrap_or(p));
        for c in &mut node.children {
            *c = new_of.get(*c).copied().unwrap_or(*c);
        }
        if let NodeKind::Mesh(m) = &mut node.kind
            && let MeshExtra::Skin(skin) = &mut m.extra
        {
            for b in &mut skin.bones {
                *b = new_of.get(*b).copied().unwrap_or(*b);
            }
            if skin.inverse_bind.len() == n {
                skin.inverse_bind = order.iter().map(|&o| skin.inverse_bind[o]).collect();
            }
        }
    }
    model.nodes = nodes;
    order
}

/// A model by name as a compiler needs it: the model and its part numbers.
/// A compiled one's are read from it; a text is compiled first (or, where
/// that fails, numbered in its tree's order).
fn numbered(
    name: &str,
    lookup: &dyn Fn(&str) -> Option<Vec<u8>>,
    depth: usize,
) -> Option<(Model, PartNumbers)> {
    let data = lookup(name)?;
    if !crate::is_binary(&data) && depth < 16 {
        if let Ok(c) = compile_named_at(&data, name, lookup, &|_| None, None, depth + 1) {
            return Some((Model::read(&c.binary).ok()?, PartNumbers::read(&c.binary)?));
        }
        let model = Model::read(&data).ok()?;
        let parts = binary::part_numbers(&model, None);
        return Some((model, parts));
    }
    Some((Model::read(&data).ok()?, PartNumbers::read(&data)?))
}

fn compile_named_at(
    text: &[u8],
    name: &str,
    lookup: &dyn Fn(&str) -> Option<Vec<u8>>,
    compiled_before: &dyn Fn(&str) -> Option<Vec<u8>>,
    materials: Option<Materials<'_>>,
    depth: usize,
) -> Result<Compiled, CompileError> {
    if crate::is_binary(text) {
        return Err(CompileError::NotAscii);
    }
    let supermodel = Model::read(text)?
        .supermodel
        .filter(|s| !s.eq_ignore_ascii_case(name))
        .and_then(|s| numbered(&s, lookup, depth));
    let existing = compiled_before(name)
        .filter(|d| crate::is_binary(d))
        .and_then(|d| Some((Model::read(&d).ok()?, PartNumbers::read(&d)?)));
    let sources = Sources {
        supermodel: supermodel.as_ref().map(|(m, p)| (m, p)),
        existing: existing.as_ref().map(|(m, p)| (m, p)),
        materials,
    };
    compile(text, &sources)
}

/// Compiles the ASCII model called `name` against what there is:
/// `lookup` gives a model's file by name (its supermodels, compiled or
/// text), `compiled_before` the compiled model of a name where one exists
/// under the text being compiled (the game's, a hak's), whose part numbers
/// it keeps.
pub fn compile_named(
    text: &[u8],
    name: &str,
    lookup: &dyn Fn(&str) -> Option<Vec<u8>>,
    compiled_before: &dyn Fn(&str) -> Option<Vec<u8>>,
) -> Result<Compiled, CompileError> {
    compile_named_at(text, name, lookup, compiled_before, None, 0)
}

/// [`compile_named`] with `materials` (`.mtr` files by name, where the game
/// would find them), for meshes that take their render hint from one.
pub fn compile_named_with(
    text: &[u8],
    name: &str,
    lookup: &dyn Fn(&str) -> Option<Vec<u8>>,
    compiled_before: &dyn Fn(&str) -> Option<Vec<u8>>,
    materials: &dyn Fn(&str) -> Option<Vec<u8>>,
) -> Result<Compiled, CompileError> {
    compile_named_at(text, name, lookup, compiled_before, Some(Materials(materials)), 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    const MODEL: &str = "newmodel m\nsetsupermodel m NULL\nclassification character\n\
setanimationscale 1\nbeginmodelgeom m\n\
node dummy m\n  parent NULL\nendnode\n\
node trimesh b\n  parent m\n  bitmap wood\n  renderhint NormalAndSpecMapped\n  alpha 0.5\n  birthrate 3\n  \
verts 4\n    0 0 0\n    1 0 0\n    1 1 0\n    0 1 0\n  \
tverts 4\n    0 0 0\n    1 0 0\n    1 1 0\n    0 1 0\n  \
faces 2\n    0 1 2 1 0 1 2 0\n    0 2 3 1 0 2 3 0\nendnode\n\
node dummy a\n  parent m\nendnode\n\
node aabb floor\n  parent m\n  \
verts 4\n    0 0 0\n    2 0 0\n    2 2 0\n    0 2 0\n  \
faces 2\n    0 1 2 1 0 0 0 3\n    0 2 3 1 0 0 0 4\nendnode\n\
endmodelgeom m\ndonemodel m\n";

    /// Children sorted as the game sorts them change the tree's order,
    /// which a skin's bones are numbered by: they still name their nodes
    /// (a corpus run caught a placeable's skin bound to the wrong bones).
    #[test]
    fn a_skin_keeps_its_bones_when_the_tree_is_sorted() {
        let text = "newmodel m\nsetsupermodel m NULL\nbeginmodelgeom m\n\
            node dummy m\n  parent NULL\nendnode\n\
            node trimesh glass\n  parent m\n  transparencyhint 1\n  verts 3\n    0 0 0\n    1 0 0\n    0 1 0\n\
            faces 1\n    0 1 2 1 0 0 0 0\nendnode\n\
            node dummy bone\n  parent m\n  position 0 0 2\nendnode\n\
            node skin s\n  parent m\n  verts 3\n    0 0 0\n    1 0 0\n    0 1 0\n\
            faces 1\n    0 1 2 1 0 0 0 0\n  weights 3\n    bone 1\n    bone 1\n    bone 1\nendnode\n\
            endmodelgeom m\ndonemodel m\n";
        let back =
            Model::read(&compile(text.as_bytes(), &Sources::default()).unwrap().binary).unwrap();
        let root = &back.nodes[back.node("m").unwrap()];
        let names: Vec<&str> = root.children.iter().map(|&c| back.nodes[c].name.as_str()).collect();
        assert_eq!(names, ["bone", "s", "glass"], "dummies, then meshes by hint");
        let s = back.nodes[back.node("s").unwrap()].mesh().unwrap();
        let MeshExtra::Skin(skin) = &s.extra else { panic!("a skin") };
        let bone =
            |v: usize| back.nodes[skin.bones[usize::from(skin.weights[v][0].0)]].name.as_str();
        assert_eq!((bone(0), bone(1), bone(2)), ("bone", "bone", "bone"));
    }

    #[test]
    fn a_text_compiles_to_what_it_says() {
        let compiled = compile(MODEL.as_bytes(), &Sources::default()).unwrap();
        let back = Model::read(&compiled.binary).unwrap();
        let mesh = back.nodes[back.node("b").unwrap()].mesh().unwrap();
        assert_eq!(mesh.vertices.len(), 4);
        // Tangents were made for the render hint: along u, which is x here.
        assert_eq!(mesh.tangents.len(), 4);
        assert!(mesh.tangents.iter().all(|t| (t[0] - 1.0).abs() < 1e-6 && t[3] == 1.0));
        // What is not a mesh's controller is left out.
        assert!(back.nodes[back.node("b").unwrap()].value("alpha").is_some());
        assert!(back.nodes[back.node("b").unwrap()].value("birthrate").is_none());
        // The walkmesh got a tree: a box over both faces, then each face.
        let floor = back.nodes[back.node("floor").unwrap()].mesh().unwrap();
        let MeshExtra::Aabb(tree) = &floor.extra else { panic!("no tree") };
        assert_eq!(tree.iter().map(|e| e.face).collect::<Vec<_>>().len(), 3);
        assert_eq!(tree[0].face, -1);
        assert_eq!((tree[0].min, tree[0].max), ([0.0; 3], [2.0, 2.0, 0.0]));
        let mut leaves: Vec<i32> = tree[1..].iter().map(|e| e.face).collect();
        leaves.sort_unstable();
        assert_eq!(leaves, [0, 1]);
        // Numbered in the text's order.
        let parts = PartNumbers::read(&compiled.binary).unwrap();
        let of = |name: &str| parts.numbers[back.node(name).unwrap()];
        assert_eq!((of("m"), of("b"), of("a"), of("floor")), (0, 1, 2, 3));
    }

    #[test]
    fn a_model_compiled_before_keeps_its_numbers() {
        let first = compile(MODEL.as_bytes(), &Sources::default()).unwrap();
        let old = Model::read(&first.binary).unwrap();
        let old_parts = PartNumbers::read(&first.binary).unwrap();
        // The same model with `a` first and a new node.
        let text = MODEL
            .replace("node dummy a\n  parent m\nendnode\n", "")
            .replace("node trimesh b", "node dummy a\n  parent m\nendnode\nnode dummy new\n  parent a\nendnode\nnode trimesh b");
        let sources = Sources { existing: Some((&old, &old_parts)), ..Default::default() };
        let again = compile(text.as_bytes(), &sources).unwrap();
        let back = Model::read(&again.binary).unwrap();
        let parts = PartNumbers::read(&again.binary).unwrap();
        let of = |name: &str| parts.numbers[back.node(name).unwrap()];
        assert_eq!((of("m"), of("b"), of("a"), of("floor")), (0, 1, 2, 3));
        assert_eq!(of("new"), 4);
        // Without the old one, the text's order numbers them.
        let fresh = compile(text.as_bytes(), &Sources::default()).unwrap();
        let fresh_parts = PartNumbers::read(&fresh.binary).unwrap();
        let fresh_model = Model::read(&fresh.binary).unwrap();
        assert_eq!(fresh_parts.numbers[fresh_model.node("a").unwrap()], 1);
    }

    #[test]
    fn a_model_finds_its_supermodel_and_its_old_self() {
        // The supermodel, as text: compiled first for its numbers.
        let sup = MODEL.replace(" m\n", " sup\n").replace("parent m", "parent sup");
        let text = MODEL.replace("setsupermodel m NULL", "setsupermodel m sup");
        let lookup = |n: &str| (n == "sup").then(|| sup.clone().into_bytes());
        let none = |_: &str| None;
        let out = compile_named(text.as_bytes(), "m", &lookup, &none).unwrap();
        let parts = PartNumbers::read(&out.binary).unwrap();
        // Its nodes are the supermodel's, by name: its numbers.
        assert_eq!(parts.numbers, [0, 1, 2, 3]);
        assert!(parts.count > 4, "past the supermodel's: {parts:?}");
        // Compiled before under other numbers: those stay.
        let old = {
            let m = Model::read(text.as_bytes()).unwrap();
            let numbers = PartNumbers { numbers: vec![0, 7, 5, 9], count: 12 };
            binary::write(&Model::read(&binary::write(&m, &numbers).unwrap()).unwrap(), &numbers)
                .unwrap()
        };
        let before = |n: &str| (n == "m").then(|| old.clone());
        let out = compile_named(text.as_bytes(), "m", &lookup, &before).unwrap();
        let back = Model::read(&out.binary).unwrap();
        let parts = PartNumbers::read(&out.binary).unwrap();
        let of = |name: &str| parts.numbers[back.node(name).unwrap()];
        let old_model = Model::read(&old).unwrap();
        let old_of = |name: &str| [0, 7, 5, 9][old_model.node(name).unwrap()];
        for name in ["m", "b", "a", "floor"] {
            assert_eq!(of(name), old_of(name), "{name}");
        }
        assert_eq!(parts.count, 12);
    }

    /// A node with no `parent` line is in no tree: the game's compiler
    /// leaves it out, and its number goes unused (nwn-mdl-spec
    /// MDL-ASCII-031, MDL-NODE-010; zspec_numbers' `n_caps`).
    #[test]
    fn a_node_left_out_keeps_its_number() {
        let text = MODEL.replace(
            "node dummy a\n  parent m\nendnode\n",
            "node dummy loose\n  PARENT m\nendnode\nnode dummy a\n  parent m\nendnode\n",
        );
        let out = compile(text.as_bytes(), &Sources::default()).unwrap();
        let back = Model::read(&out.binary).unwrap();
        assert_eq!(back.node("loose"), None);
        let parts = PartNumbers::read(&out.binary).unwrap();
        let alone = compile(MODEL.as_bytes(), &Sources::default()).unwrap();
        let before = PartNumbers::read(&alone.binary).unwrap();
        assert_eq!(parts.count, before.count + 1);
        let numbered = |m: &Model, p: &PartNumbers, name: &str| p.numbers[m.node(name).unwrap()];
        let first = Model::read(&alone.binary).unwrap();
        // (`a` came after it in the text: one past where it was.)
        assert_eq!(numbered(&back, &parts, "a"), numbered(&first, &before, "a") + 1);
        assert!(out.notes.iter().any(|(_, n)| n.contains("loose: has no parent line")));
    }

    #[test]
    fn what_is_not_text_is_refused() {
        let compiled = compile(MODEL.as_bytes(), &Sources::default()).unwrap();
        assert_eq!(compile(&compiled.binary, &Sources::default()), Err(CompileError::NotAscii));
    }

    /// A skin gets the place of each node from itself at rest: a bone a
    /// metre up and a quarter turn round sees the skin's origin a metre
    /// down, turned back.
    #[test]
    fn a_skin_gets_its_inverse_binds() {
        let skin = MODEL.replace(
            "node dummy a\n  parent m\nendnode\n",
            "node dummy a\n  parent m\n  position 0 0 1\n  orientation 0 0 1 1.5707964\nendnode\n\
             node skin s\n  parent m\n  position 1 0 0\n  verts 3\n    0 0 0\n    1 0 0\n    0 1 0\n  \
             faces 1\n    0 1 2 1 0 0 0 0\n  weights 3\n    a 1\n    a 0.5 m 0.5\n    m 1\nendnode\n",
        );
        let out = compile(skin.as_bytes(), &Sources::default()).unwrap();
        let back = Model::read(&out.binary).unwrap();
        let (s, a) = (back.node("s").unwrap(), back.node("a").unwrap());
        let MeshExtra::Skin(skin) = &back.nodes[s].mesh().unwrap().extra else { panic!() };
        assert_eq!(skin.inverse_bind.len(), back.nodes.len());
        // The skin's origin (1, 0, 0) seen from `a` at (0, 0, 1), turned a
        // quarter round z: (0, -1, -1).
        let (q, t) = skin.inverse_bind[a];
        let seen = rotate(q, [0.0; 3]).map(|x| x + 0.0);
        let at = [seen[0] + t[0], seen[1] + t[1], seen[2] + t[2]];
        for (x, want) in at.iter().zip([0.0, -1.0, -1.0]) {
            assert!((x - want).abs() < 1e-5, "{at:?}");
        }
        // From itself: nothing.
        let (q, t) = skin.inverse_bind[s];
        assert!(q[3].abs() > 0.99999 && t.iter().all(|x| x.abs() < 1e-6));
        // Its weights name its bones.
        let names: Vec<&str> = skin.bones.iter().map(|&b| back.nodes[b].name.as_str()).collect();
        assert_eq!(names, ["a", "m"]);
        assert_eq!(skin.weights.len(), 3);
    }
}
