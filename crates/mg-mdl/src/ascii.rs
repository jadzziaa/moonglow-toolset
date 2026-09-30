//! ASCII models, as the game reads them: keywords case-insensitive, `#`
//! comments, `true`/`false` for booleans, unknown keywords skipped,
//! non-standard node types (`patch`, `pwk`, …) as dummies, list counts that
//! may be wrong in old files (lists end early at a keyword), key lists
//! ended by `endlist` or by the next keyword.
//!
//! Meshes are de-indexed: a render vertex per distinct (vertex, texture
//! vertex, normal), with normals from the file (EE) or from smoothing
//! groups; per-vertex lists (weights, constraints, colours, animated
//! vertices) follow each render vertex's source vertex.

use std::collections::HashMap;

use crate::ctrl::{self, flags};
use crate::{
    AabbEntry, AnimMesh, AnimMeshSets, AnimNode, Animation, Classification, Controller, Dangly,
    EMITTER_FLAGS, Emitter, Face, Light, MdlError, Mesh, MeshExtra, Model, Node, NodeKind,
    Reference, Skin, Vec2, Vec3, axis_angle, face_normal, normalize,
};

/// A line's words, without comments.
struct Line<'a> {
    words: Vec<&'a str>,
}

fn lines(text: &str) -> Vec<Line<'_>> {
    text.lines()
        .filter_map(|l| {
            let l = l.split('#').next().unwrap_or_default();
            let words: Vec<&str> = l.split_whitespace().collect();
            (!words.is_empty()).then_some(Line { words })
        })
        .collect()
}

fn is_number(w: &str) -> bool {
    let b = w.as_bytes();
    matches!(b.first(), Some(b'0'..=b'9' | b'-' | b'+' | b'.')) && w.parse::<f32>().is_ok()
}

fn num(w: Option<&&str>) -> f32 {
    w.and_then(|w| w.parse::<f32>().ok()).unwrap_or(0.0)
}

fn flag(w: Option<&&str>) -> bool {
    match w {
        Some(w) if w.eq_ignore_ascii_case("true") => true,
        Some(w) => w.parse::<f32>().is_ok_and(|v| v != 0.0),
        None => false,
    }
}

fn resname(w: Option<&&str>) -> Option<String> {
    let w = w?.to_ascii_lowercase();
    (!w.is_empty() && w != "null").then_some(w)
}

/// Keywords followed by a count and that many lines.
const LISTS: &[&str] = &[
    "verts",
    "tverts",
    "tverts0",
    "tverts1",
    "tverts2",
    "tverts3",
    "faces",
    "normals",
    "tangents",
    "colors",
    "weights",
    "constraints",
    "animverts",
    "animtverts",
    "flarepositions",
    "flaresizes",
    "flarecolorshifts",
    "texturenames",
    "multimaterial",
    "qbone_ref_inv",
    "tbone_ref_inv",
    "boneconstantindices",
    "texindices1",
    "texindices2",
    "texindices3",
];

/// Lists whose lines start with a name, not a number (counts are trusted).
const NAMED_LISTS: &[&str] = &["weights", "texturenames", "multimaterial"];

/// A node block as written: its type, name, single-line properties, lists
/// and key lists.
#[derive(Default)]
struct RawNode<'a> {
    ty: String,
    name: String,
    parent: Option<String>,
    props: Vec<(String, Vec<&'a str>)>,
    lists: HashMap<String, Vec<Vec<&'a str>>>,
    keys: Vec<(String, Vec<Vec<f32>>)>,
    aabb: Vec<Vec<f32>>,
}

impl<'a> RawNode<'a> {
    fn prop(&self, key: &str) -> Option<&Vec<&'a str>> {
        self.props.iter().rev().find(|(k, _)| k == key).map(|(_, v)| v)
    }

    fn list(&self, key: &str) -> &[Vec<&'a str>] {
        self.lists.get(key).map_or(&[], Vec::as_slice)
    }
}

/// Parses a node block starting after its `node` line; returns the node and
/// the index of the line after `endnode`.
fn node_block<'a>(ls: &'a [Line<'a>], mut i: usize, header: &Line<'a>) -> (RawNode<'a>, usize) {
    let mut n = RawNode {
        ty: header.words.get(1).map(|w| w.to_ascii_lowercase()).unwrap_or_default(),
        name: header.words.get(2).map(|w| w.to_string()).unwrap_or_default(),
        ..Default::default()
    };
    while i < ls.len() {
        let l = &ls[i];
        let key = l.words[0].to_ascii_lowercase();
        i += 1;
        match key.as_str() {
            "endnode" => break,
            "node" | "newanim" | "doneanim" | "donemodel" | "endmodelgeom" => {
                // A missing endnode.
                i -= 1;
                break;
            }
            "parent" => n.parent = l.words.get(1).map(|w| w.to_string()),
            "aabb" => {
                let mut row: Vec<f32> =
                    l.words[1..].iter().filter_map(|w| w.parse().ok()).collect();
                if !row.is_empty() {
                    n.aabb.push(std::mem::take(&mut row));
                }
                while i < ls.len() && is_number(ls[i].words[0]) {
                    n.aabb.push(ls[i].words.iter().filter_map(|w| w.parse().ok()).collect());
                    i += 1;
                }
            }
            k if LISTS.contains(&k) => {
                let count = l.words.get(1).and_then(|w| w.parse::<usize>().ok()).unwrap_or(0);
                let named = NAMED_LISTS.contains(&k);
                let mut rows: Vec<Vec<&str>> = Vec::new();
                // Values on the keyword's own line (e.g. `flaresizes 3 1 2 3`).
                if l.words.len() > 2 && !named {
                    for w in &l.words[2..] {
                        rows.push(vec![*w]);
                    }
                }
                while rows.len() < count && i < ls.len() {
                    let first = ls[i].words[0];
                    if first.eq_ignore_ascii_case("endnode")
                        || first.eq_ignore_ascii_case("endlist")
                        || !named && !is_number(first)
                    {
                        break;
                    }
                    rows.push(ls[i].words.clone());
                    i += 1;
                }
                if i < ls.len() && ls[i].words[0].eq_ignore_ascii_case("endlist") {
                    i += 1;
                }
                n.lists.insert(k.to_string(), rows);
            }
            k if k.ends_with("key") && k.len() > 3 => {
                let name = k.trim_end_matches("key").trim_end_matches("bezier").to_string();
                let count = l.words.get(1).and_then(|w| w.parse::<usize>().ok());
                let mut rows = Vec::new();
                while i < ls.len() && is_number(ls[i].words[0]) {
                    if count.is_some_and(|c| rows.len() >= c) {
                        break;
                    }
                    rows.push(ls[i].words.iter().filter_map(|w| w.parse().ok()).collect());
                    i += 1;
                }
                if i < ls.len() && ls[i].words[0].eq_ignore_ascii_case("endlist") {
                    i += 1;
                }
                if !k.ends_with("bezierkey") {
                    n.keys.push((name, rows));
                }
            }
            _ => n.props.push((key, l.words[1..].to_vec())),
        }
    }
    (n, i)
}

/// The flags of a node type, for controller lookups.
fn type_flags(ty: &str) -> u32 {
    match ty {
        "light" => 0x3,
        "emitter" => 0x5,
        "reference" => 0x11,
        "camera" => 0x9,
        "trimesh" => 0x21,
        "skin" => 0x61,
        "animmesh" => 0xA1,
        "danglymesh" => 0x121,
        "aabb" => 0x221,
        _ => flags::HEADER,
    }
}

fn vec3(words: &[&str]) -> Vec3 {
    [num(words.first()), num(words.get(1)), num(words.get(2))]
}

/// Rows of numbers as vectors (missing values 0).
fn vec3s(rows: &[Vec<&str>]) -> Vec<Vec3> {
    rows.iter().map(|r| vec3(r)).collect()
}

fn vec2s(rows: &[Vec<&str>]) -> Vec<Vec2> {
    rows.iter().map(|r| [num(r.first()), num(r.get(1))]).collect()
}

/// A controller from a key list; orientation rows (axis-angle) become
/// quaternions.
fn key_controller(name: &str, rows: &[Vec<f32>]) -> Controller {
    let orientation = name == "orientation";
    let columns = rows.iter().map(|r| r.len().saturating_sub(1)).max().unwrap_or(0);
    let mut c = Controller {
        name: if name == "setfillumcolor" { "selfillumcolor".into() } else { name.into() },
        columns: if orientation { 4 } else { columns },
        ..Default::default()
    };
    for r in rows {
        c.times.push(r.first().copied().unwrap_or(0.0));
        if orientation {
            let v = |i: usize| r.get(i).copied().unwrap_or(0.0);
            c.values.extend(axis_angle([v(1), v(2), v(3)], v(4)));
        } else {
            for j in 0..columns {
                c.values.push(r.get(j + 1).copied().unwrap_or(0.0));
            }
        }
    }
    c
}

/// De-indexes a mesh's faces; returns the mesh streams.
fn build_mesh(raw: &RawNode<'_>, m: &mut Mesh) {
    let verts = vec3s(raw.list("verts"));
    let tvert_list = |k: &str| vec2s(raw.list(k));
    let mut tverts =
        [tvert_list("tverts"), tvert_list("tverts1"), tvert_list("tverts2"), tvert_list("tverts3")];
    if tverts[0].is_empty() {
        tverts[0] = tvert_list("tverts0");
    }
    let file_normals = vec3s(raw.list("normals"));
    let colors: Vec<[u8; 4]> = raw
        .list("colors")
        .iter()
        .map(|r| {
            let c = vec3(r);
            let b = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
            [b(c[0]), b(c[1]), b(c[2]), 255]
        })
        .collect();
    // Faces: v1 v2 v3 smoothing t1 t2 t3 material.
    struct F {
        v: [usize; 3],
        t: [usize; 3],
        smooth: u32,
        material: u32,
    }
    let faces: Vec<F> = raw
        .list("faces")
        .iter()
        .filter_map(|r| {
            let n = |i: usize| r.get(i).and_then(|w| w.parse::<f32>().ok()).map(|v| v as i64);
            let v = [n(0)?, n(1)?, n(2)?];
            if v.iter().any(|&x| x < 0 || x as usize >= verts.len()) {
                return None;
            }
            let t = [n(4).unwrap_or(0), n(5).unwrap_or(0), n(6).unwrap_or(0)];
            Some(F {
                v: v.map(|x| x as usize),
                t: t.map(|x| x.max(0) as usize),
                smooth: n(3).unwrap_or(0) as u32,
                material: n(7).unwrap_or(0).max(0) as u32,
            })
        })
        .collect();
    // Normals per corner: from the file, else from smoothing groups
    // (area-weighted face normals of the faces sharing a group).
    let face_normals: Vec<Vec3> =
        faces.iter().map(|f| face_normal(verts[f.v[0]], verts[f.v[1]], verts[f.v[2]])).collect();
    let mut at_vertex: Vec<Vec<usize>> = vec![Vec::new(); verts.len()];
    if file_normals.len() < verts.len() {
        for (fi, f) in faces.iter().enumerate() {
            for &v in &f.v {
                at_vertex[v].push(fi);
            }
        }
    }
    let corner_normal = |fi: usize, v: usize| -> Vec3 {
        if let Some(n) = file_normals.get(v).filter(|_| file_normals.len() >= verts.len()) {
            return *n;
        }
        let f = &faces[fi];
        if f.smooth == 0 {
            return normalize(face_normals[fi]);
        }
        let mut sum = [0.0f32; 3];
        for &g in &at_vertex[v] {
            if g == fi || faces[g].smooth & f.smooth != 0 {
                for k in 0..3 {
                    sum[k] += face_normals[g][k];
                }
            }
        }
        normalize(sum)
    };
    let mut index: HashMap<(usize, usize, [u32; 3]), u32> = HashMap::new();
    for (fi, f) in faces.iter().enumerate() {
        let mut out = [0u32; 3];
        for (c, slot) in out.iter_mut().enumerate() {
            let (v, t) = (f.v[c], f.t[c]);
            let n = corner_normal(fi, v);
            let key = (v, t, n.map(f32::to_bits));
            *slot = *index.entry(key).or_insert_with(|| {
                let i = m.vertices.len() as u32;
                m.vertices.push(verts[v]);
                m.normals.push(n);
                for (set, list) in tverts.iter().enumerate() {
                    if !list.is_empty() {
                        m.uvs[set].push(list.get(t).copied().unwrap_or([0.0, 0.0]));
                    }
                }
                if !colors.is_empty() {
                    m.colors.push(colors.get(v).copied().unwrap_or([255; 4]));
                }
                m.source.push(v as u32);
                i
            });
        }
        m.faces.push(Face { vertices: out, material: f.material });
    }
}

/// A geometry node.
fn geometry_node(raw: &RawNode<'_>) -> Node {
    let node_flags = type_flags(&raw.ty);
    let p = |k: &str| raw.prop(k);
    let kind = match raw.ty.as_str() {
        "light" => {
            let floats = |k: &str| {
                raw.list(k).iter().filter_map(|r| r.first().and_then(|w| w.parse().ok())).collect()
            };
            NodeKind::Light(Light {
                flare_radius: num(p("flareradius").and_then(|v| v.first())),
                flare_sizes: floats("flaresizes"),
                flare_positions: floats("flarepositions"),
                flare_color_shifts: vec3s(raw.list("flarecolorshifts")),
                flare_textures: raw
                    .list("texturenames")
                    .iter()
                    .filter_map(|r| r.first().map(|w| w.to_ascii_lowercase()))
                    .collect(),
                priority: num(p("lightpriority").and_then(|v| v.first())) as u32,
                ambient_only: flag(p("ambientonly").or(p("ambient_only")).and_then(|v| v.first())),
                dynamic_type: num(p("ndynamictype")
                    .or(p("n_dynamic_type"))
                    .or(p("isdynamic"))
                    .and_then(|v| v.first())) as u32,
                affect_dynamic: p("affectdynamic")
                    .or(p("affect_dynamic"))
                    .is_none_or(|v| flag(v.first())),
                shadow: p("shadow").is_none_or(|v| flag(v.first())),
                generate_flare: flag(p("generateflare").and_then(|v| v.first())),
                fading: flag(p("fadinglight").or(p("fading_light")).and_then(|v| v.first())),
            })
        }
        "emitter" => {
            let s =
                |k: &str| p(k).and_then(|v| v.first()).map(|w| w.to_string()).unwrap_or_default();
            let mut e = Emitter {
                deadspace: num(p("deadspace").and_then(|v| v.first())),
                blast_radius: num(p("blastradius").and_then(|v| v.first())),
                blast_length: num(p("blastlength").and_then(|v| v.first())),
                xgrid: num(p("xgrid").and_then(|v| v.first())) as u32,
                ygrid: num(p("ygrid").and_then(|v| v.first())) as u32,
                spawntype: num(p("spawntype").and_then(|v| v.first())) as u32,
                update: s("update"),
                render: s("render"),
                blend: s("blend"),
                texture: resname(p("texture").and_then(|v| v.first())),
                chunk: resname(p("chunkname").and_then(|v| v.first())),
                two_sided: flag(p("twosidedtex").and_then(|v| v.first())),
                looping: flag(p("loop").and_then(|v| v.first())),
                render_order: num(p("renderorder").and_then(|v| v.first())) as u32,
                flags: 0,
            };
            for (name, bit) in EMITTER_FLAGS {
                let v = p(name).or(if name == "m_istinted" { p("m_istnited") } else { None });
                if flag(v.and_then(|v| v.first())) {
                    e.flags |= bit;
                }
            }
            NodeKind::Emitter(e)
        }
        "reference" => NodeKind::Reference(Reference {
            model: resname(p("refmodel").and_then(|v| v.first())),
            reattachable: flag(p("reattachable").and_then(|v| v.first())),
        }),
        "camera" => NodeKind::Camera,
        "trimesh" | "skin" | "animmesh" | "danglymesh" | "aabb" => {
            let bool_or = |k: &str, d: bool| p(k).map_or(d, |v| flag(v.first()));
            // Walkmeshes are never drawn (and nwnmdlcomp omits the flags).
            let walkmesh = raw.ty == "aabb";
            let mut m = Mesh {
                diffuse: p("diffuse").map_or([1.0; 3], |v| vec3(v)),
                ambient: p("ambient").map_or([1.0; 3], |v| vec3(v)),
                specular: p("specular").map_or([0.0; 3], |v| vec3(v)),
                shininess: num(p("shininess").and_then(|v| v.first())),
                render: bool_or("render", !walkmesh),
                shadow: bool_or("shadow", !walkmesh),
                beaming: bool_or("beaming", false),
                rotate_texture: bool_or("rotatetexture", false),
                tilefade: num(p("tilefade").and_then(|v| v.first())) as u32,
                transparency_hint: num(p("transparencyhint").and_then(|v| v.first())) as u32,
                material: resname(p("materialname").and_then(|v| v.first())),
                renderhint: p("renderhint").and_then(|v| v.first()).map(|w| w.to_ascii_lowercase()),
                ..Default::default()
            };
            m.textures[0] = resname(p("texture0").or(p("bitmap")).and_then(|v| v.first()));
            for i in 1..4 {
                m.textures[i] = resname(p(&format!("texture{i}")).and_then(|v| v.first()));
            }
            build_mesh(raw, &mut m);
            m.extra = match raw.ty.as_str() {
                "danglymesh" => {
                    let c: Vec<f32> =
                        raw.list("constraints").iter().map(|r| num(r.first())).collect();
                    MeshExtra::Dangly(Dangly {
                        constraints: m
                            .source
                            .iter()
                            .map(|&s| c.get(s as usize).copied().unwrap_or(0.0))
                            .collect(),
                        displacement: num(p("displacement").and_then(|v| v.first())),
                        tightness: num(p("tightness").and_then(|v| v.first())),
                        period: num(p("period").and_then(|v| v.first())),
                    })
                }
                "animmesh" => MeshExtra::Anim(AnimMesh {
                    sample_period: num(p("sampleperiod").and_then(|v| v.first())),
                }),
                "aabb" => MeshExtra::Aabb(
                    raw.aabb
                        .iter()
                        .map(|r| {
                            let v = |i: usize| r.get(i).copied().unwrap_or(0.0);
                            AabbEntry {
                                min: [v(0), v(1), v(2)],
                                max: [v(3), v(4), v(5)],
                                face: v(6) as i32,
                            }
                        })
                        .collect(),
                ),
                // Skin weights are resolved once all node names are known.
                "skin" => MeshExtra::Skin(Skin::default()),
                _ => MeshExtra::None,
            };
            NodeKind::Mesh(Box::new(m))
        }
        _ => NodeKind::Dummy,
    };
    let mut node = Node::new(&raw.name, kind);
    if let Some(v) = p("position") {
        node.position = vec3(v);
    }
    if let Some(v) = p("orientation") {
        node.orientation = axis_angle(vec3(v), num(v.get(3)));
    }
    if let Some(v) = p("scale") {
        node.scale = num(v.first());
    }
    node.inherit_color = flag(p("inheritcolor").and_then(|v| v.first()));
    // Rest-pose controllers: single-line values of controller keywords.
    for (k, v) in &raw.props {
        let Some(cols) = ctrl::columns(node_flags, k) else { continue };
        if matches!(k.as_str(), "position" | "orientation" | "scale") {
            continue;
        }
        let values: Vec<f32> = v.iter().take(cols.max(1)).filter_map(|w| w.parse().ok()).collect();
        if values.is_empty() && cols > 0 {
            continue;
        }
        let name = if k == "setfillumcolor" { "selfillumcolor" } else { k.as_str() };
        node.controllers.retain(|c| c.name != name);
        node.controllers.push(Controller::constant(name, &values[..values.len().min(cols)]));
    }
    node
}

/// Parent links to pre-order: the order in which nodes appear when each
/// node's children are visited in file order, from the first root. Returns
/// the new order (old indices) and each old index's new index.
fn preorder(parents: &[Option<usize>]) -> (Vec<usize>, Vec<usize>) {
    let n = parents.len();
    let mut children: Vec<Vec<usize>> = vec![Vec::new(); n];
    let mut roots = Vec::new();
    for (i, p) in parents.iter().enumerate() {
        match p {
            Some(p) if *p != i => children[*p].push(i),
            _ => roots.push(i),
        }
    }
    let mut order = Vec::with_capacity(n);
    let mut seen = vec![false; n];
    let mut stack: Vec<usize> = roots.into_iter().rev().collect();
    while let Some(i) = stack.pop() {
        if std::mem::replace(&mut seen[i], true) {
            continue;
        }
        order.push(i);
        for &c in children[i].iter().rev() {
            stack.push(c);
        }
    }
    // Nodes in a parent loop: append.
    for (i, s) in seen.iter().enumerate() {
        if !s {
            order.push(i);
        }
    }
    let mut new_index = vec![0; n];
    for (new, &old) in order.iter().enumerate() {
        new_index[old] = new;
    }
    (order, new_index)
}

/// Resolves `parent` names to indices of earlier nodes (case-insensitive;
/// unknown and `NULL` parents make roots).
fn parents(raws: &[RawNode<'_>]) -> Vec<Option<usize>> {
    let mut by_name: HashMap<String, usize> = HashMap::new();
    let mut out = Vec::with_capacity(raws.len());
    for (i, r) in raws.iter().enumerate() {
        let p = r
            .parent
            .as_ref()
            .filter(|p| !p.eq_ignore_ascii_case("null"))
            .and_then(|p| by_name.get(&p.to_ascii_lowercase()).copied());
        out.push(p);
        by_name.entry(r.name.to_ascii_lowercase()).or_insert(i);
    }
    // Later roots hang under the first (the model's root node).
    if let Some(first) = out.iter().position(Option::is_none) {
        for (i, p) in out.iter_mut().enumerate() {
            if p.is_none() && i != first {
                *p = Some(first);
            }
        }
    }
    out
}

fn anim_node(raw: &RawNode<'_>, source_counts: &HashMap<String, usize>) -> AnimNode {
    let node_flags = type_flags(&raw.ty);
    let mut n = AnimNode { name: raw.name.clone(), ..Default::default() };
    for (k, rows) in &raw.keys {
        n.controllers.push(key_controller(k, rows));
    }
    // Single values hold for the whole animation.
    for (k, v) in &raw.props {
        let is_ctrl = matches!(k.as_str(), "position" | "orientation" | "scale")
            || ctrl::columns(node_flags, k).is_some();
        if !is_ctrl || n.controllers.iter().any(|c| &c.name == k) {
            continue;
        }
        let mut row: Vec<f32> = vec![0.0];
        row.extend(v.iter().filter_map(|w| w.parse::<f32>().ok()));
        n.controllers.push(key_controller(k, &[row]));
    }
    let verts = vec3s(raw.list("animverts"));
    let tverts = vec2s(raw.list("animtverts"));
    if !verts.is_empty() || !tverts.is_empty() {
        let count = source_counts.get(&raw.name.to_ascii_lowercase()).copied().unwrap_or(0);
        let split3 = |v: &[Vec3]| -> Vec<Vec<Vec3>> {
            if count == 0 { Vec::new() } else { v.chunks(count).map(<[Vec3]>::to_vec).collect() }
        };
        let split2 = |v: &[Vec2]| -> Vec<Vec<Vec2>> {
            if count == 0 { Vec::new() } else { v.chunks(count).map(<[Vec2]>::to_vec).collect() }
        };
        n.anim_mesh = Some(AnimMeshSets {
            sample_period: num(raw.prop("sampleperiod").and_then(|v| v.first())),
            vertex_sets: split3(&verts),
            uv_sets: split2(&tverts),
        });
    }
    n
}

/// Reads an ASCII model.
pub fn read(data: &[u8]) -> Result<Model, MdlError> {
    let text = String::from_utf8_lossy(data);
    let ls = lines(&text);
    let mut model = Model { animation_scale: 1.0, ..Default::default() };
    let mut geometry: Vec<RawNode<'_>> = Vec::new();
    let mut anims: Vec<(Animation, Vec<RawNode<'_>>)> = Vec::new();
    let mut in_anim = false;
    let mut i = 0;
    while i < ls.len() {
        let l = &ls[i];
        i += 1;
        match l.words[0].to_ascii_lowercase().as_str() {
            "newmodel" => model.name = l.words.get(1).map(|w| w.to_string()).unwrap_or_default(),
            "setsupermodel" => model.supermodel = resname(l.words.get(2)),
            "classification" => {
                model.classification =
                    Classification::from_name(l.words.get(1).copied().unwrap_or_default());
            }
            "ignorefog" => model.ignore_fog = flag(l.words.get(1)),
            "setanimationscale" => model.animation_scale = num(l.words.get(1)),
            "newanim" => {
                in_anim = true;
                let name = l.words.get(1).map(|w| w.to_string()).unwrap_or_default();
                anims.push((Animation { name, ..Default::default() }, Vec::new()));
            }
            "doneanim" => in_anim = false,
            "length" if in_anim => {
                anims.last_mut().expect("in anim").0.length = num(l.words.get(1))
            }
            "transtime" if in_anim => {
                anims.last_mut().expect("in anim").0.transtime = num(l.words.get(1));
            }
            "animroot" if in_anim => {
                anims.last_mut().expect("in anim").0.animroot =
                    l.words.get(1).map(|w| w.to_string()).unwrap_or_default();
            }
            "event" if in_anim => {
                let name = l.words.get(2).map(|w| w.to_string()).unwrap_or_default();
                anims.last_mut().expect("in anim").0.events.push((num(l.words.get(1)), name));
            }
            "node" => {
                let (raw, next) = node_block(&ls, i, l);
                i = next;
                if in_anim {
                    anims.last_mut().expect("in anim").1.push(raw);
                } else {
                    geometry.push(raw);
                }
            }
            _ => {}
        }
    }
    if geometry.is_empty() {
        return Err(MdlError::Empty);
    }
    if model.name.is_empty() {
        model.name = geometry[0].name.clone();
    }

    // Geometry in pre-order.
    let parent_of = parents(&geometry);
    let (order, new_index) = preorder(&parent_of);
    let mut nodes: Vec<Node> = order.iter().map(|&old| geometry_node(&geometry[old])).collect();
    for (new, &old) in order.iter().enumerate() {
        nodes[new].parent = parent_of[old].map(|p| new_index[p]);
    }
    for i in 0..nodes.len() {
        if let Some(p) = nodes[i].parent {
            nodes[p].children.push(i);
        }
    }
    // Skin weights: `bone weight` pairs per source vertex, by node name.
    let by_name: HashMap<String, usize> =
        nodes.iter().enumerate().rev().map(|(i, n)| (n.name.to_ascii_lowercase(), i)).collect();
    for (new, &old) in order.iter().enumerate() {
        let raw = &geometry[old];
        let NodeKind::Mesh(m) = &mut nodes[new].kind else { continue };
        let MeshExtra::Skin(skin) = &mut m.extra else { continue };
        let mut bone_of: HashMap<usize, u16> = HashMap::new();
        let per_source: Vec<[(u16, f32); 4]> = raw
            .list("weights")
            .iter()
            .map(|r| {
                let mut out = [(0u16, 0.0f32); 4];
                for (j, pair) in r.chunks(2).take(4).enumerate() {
                    let (Some(name), Some(w)) = (pair.first(), pair.get(1)) else { continue };
                    let Some(&node) = by_name.get(&name.to_ascii_lowercase()) else { continue };
                    let next = skin.bones.len() as u16;
                    let bone = *bone_of.entry(node).or_insert_with(|| {
                        skin.bones.push(node);
                        next
                    });
                    out[j] = (bone, w.parse().unwrap_or(0.0));
                }
                out
            })
            .collect();
        skin.weights = m
            .source
            .iter()
            .map(|&s| per_source.get(s as usize).copied().unwrap_or([(0, 0.0); 4]))
            .collect();
    }
    model.nodes = nodes;

    // Animations: nodes in pre-order, animated vertices over the mesh's
    // source vertices.
    let source_counts: HashMap<String, usize> =
        geometry.iter().map(|r| (r.name.to_ascii_lowercase(), r.list("verts").len())).collect();
    for (mut anim, raws) in anims {
        let parent_of = parents(&raws);
        let (order, new_index) = preorder(&parent_of);
        anim.nodes = order
            .iter()
            .map(|&old| {
                let mut n = anim_node(&raws[old], &source_counts);
                n.parent = parent_of[old].map(|p| new_index[p]);
                n
            })
            .collect();
        model.animations.push(anim);
    }
    Ok(model)
}

#[cfg(test)]
mod tests {
    use super::*;

    const CUBE_ISH: &str = "\
# a test model
filedependancy test.max
newmodel test
setsupermodel test NULL
classification Character
setanimationscale 1.5
beginmodelgeom test
node dummy test
  parent NULL
endnode
node trimesh Plane01
  parent test
  position 1 2 3
  orientation 0 0 1 3.14159265
  bitmap tex01
  Diffuse 0.5 0.5 0.5 1
  setfillumcolor 0.1 0.2 0.3
  verts 4
    0 0 0
    1 0 0
    1 1 0
    0 1 0
  tverts 4
    0 0 0
    1 0 0
    1 1 0
    0 1 0
  faces 2
    0 1 2 1 0 1 2 3
    0 2 3 1 0 2 3 3
endnode
node danglymesh Cape
  parent Plane01
  verts 3
    0 0 0
    1 0 0
    0 1 0
  faces 1
    0 1 2 0 0 0 0 0
  constraints 3
    0
    128
    255
  displacement 0.5
endnode
node light Light01
  parent test
  color 1 0.5 0
  radius 5
  ndynamictype 1
  flaresizes 2 0.5 1
endnode
endmodelgeom test
newanim walk test
  length 1.0
  transtime 0.25
  animroot test
  event 0.5 snd_footstep
  node dummy test
    parent NULL
  endnode
  node trimesh Plane01
    parent test
    positionkey
      0 0 0 0
      1 0 0 1
    endlist
    orientationkey 1
      0 0 0 1 0
    alpha 0.5
  endnode
doneanim walk test
donemodel test
";

    #[test]
    fn a_small_model() {
        let m = read(CUBE_ISH.as_bytes()).unwrap();
        assert_eq!(m.name, "test");
        assert_eq!(m.supermodel, None);
        assert_eq!(m.classification, Classification::Character);
        assert_eq!(m.animation_scale, 1.5);
        let names: Vec<&str> = m.nodes.iter().map(|n| n.name.as_str()).collect();
        assert_eq!(names, ["test", "Plane01", "Cape", "Light01"]);
        assert_eq!(m.nodes[1].parent, Some(0));
        assert_eq!(m.nodes[2].parent, Some(1));
        assert_eq!(m.nodes[0].children, [1, 3]);
        let plane = &m.nodes[1];
        assert_eq!(plane.position, [1.0, 2.0, 3.0]);
        assert!((plane.orientation[2] - 1.0).abs() < 1e-6);
        assert_eq!(plane.value("selfillumcolor"), Some(&[0.1, 0.2, 0.3][..]));
        let mesh = plane.mesh().unwrap();
        assert_eq!(mesh.textures[0].as_deref(), Some("tex01"));
        assert_eq!(mesh.diffuse, [0.5, 0.5, 0.5]);
        // Two triangles sharing an edge, one smoothing group, flat: 4 vertices.
        assert_eq!(mesh.vertices.len(), 4);
        assert_eq!(mesh.faces.len(), 2);
        assert_eq!(mesh.faces[0].material, 3);
        assert_eq!(mesh.normals[0], [0.0, 0.0, 1.0]);
        assert_eq!(mesh.uvs[0][2], [1.0, 1.0]);
        let MeshExtra::Dangly(d) = &m.nodes[2].mesh().unwrap().extra else { panic!("dangly") };
        assert_eq!(d.constraints, [0.0, 128.0, 255.0]);
        assert_eq!(d.displacement, 0.5);
        let NodeKind::Light(l) = &m.nodes[3].kind else { panic!("light") };
        assert_eq!(l.dynamic_type, 1);
        assert_eq!(l.flare_sizes, [0.5, 1.0]);
        assert_eq!(m.nodes[3].value("color"), Some(&[1.0, 0.5, 0.0][..]));
        assert_eq!(m.nodes[3].value("radius"), Some(&[5.0][..]));
        let walk = m.animation("WALK").unwrap();
        assert_eq!(walk.length, 1.0);
        assert_eq!(walk.events, [(0.5, "snd_footstep".to_string())]);
        let a = &walk.nodes[1];
        assert_eq!(a.name, "Plane01");
        let pos = &a.controllers[0];
        assert_eq!(pos.name, "position");
        assert_eq!(pos.times, [0.0, 1.0]);
        assert_eq!(pos.row(1), [0.0, 0.0, 1.0]);
        let o = &a.controllers[1];
        assert_eq!(o.row(0), [0.0, 0.0, 0.0, 1.0]);
        assert_eq!(a.controllers[2].name, "alpha");
    }

    #[test]
    fn smoothing_groups_split_vertices() {
        // Two faces at a right angle sharing an edge: same group smooths
        // (4 vertices), different groups split the edge (6).
        let mdl = |g2: u32| {
            format!(
                "newmodel t\nbeginmodelgeom t\nnode dummy t\nendnode\nnode trimesh m\nparent t\n\
                 verts 4\n0 0 0\n1 0 0\n0 1 0\n0 0 1\nfaces 2\n0 1 2 1 0 0 0 0\n0 3 1 {g2} 0 0 0 0\n\
                 endnode\nendmodelgeom t\ndonemodel t\n"
            )
        };
        let count = |g2| {
            let m = read(mdl(g2).as_bytes()).unwrap();
            m.nodes[1].mesh().unwrap().vertices.len()
        };
        assert_eq!(count(1), 4);
        assert_eq!(count(2), 6);
    }

    #[test]
    fn preorder_follows_file_order_of_children() {
        let (order, _) = preorder(&[None, Some(0), Some(0), Some(1)]);
        assert_eq!(order, [0, 1, 3, 2]);
    }
}
