//! ASCII models, as the game reads them: keywords case-insensitive, `#`
//! comments, `true`/`false` for booleans, unknown keywords skipped,
//! non-standard node types (`patch`, `pwk`, …) as dummies, list counts that
//! may be wrong in old files (lists end early at a keyword), key lists
//! ended by `endlist` or by the next keyword.
//!
//! Meshes are de-indexed: a render vertex per distinct (vertex, texture
//! vertex, normal), with normals from the file (EE) or from smoothing
//! groups; per-vertex lists (weights, constraints, colours, tangents,
//! animated vertices) follow each render vertex's source vertex.
//!
//! Integers are read as C reads them: a number's whole part, wrapped to 32
//! bits (`spawntype -1` is 0xFFFFFFFF, as the game's compiler stores it).
//! [`read_mapped`] also tells where each node and animation is in the text
//! and what the reader left out or guessed ([`SourceMap`]).

use std::collections::{HashMap, HashSet};

use crate::ctrl::{self, flags};
use crate::{
    AabbEntry, AnimMesh, AnimMeshSets, AnimNode, Animation, Classification, Controller, Dangly,
    EMITTER_FLAGS, Emitter, Face, Light, MdlError, Mesh, MeshExtra, Model, Node, NodeKind,
    Reference, Skin, Vec2, Vec3, axis_angle, face_normal, normalize,
};

/// Where things are in an ASCII model's text: line numbers from 1 (0: not
/// in the text, such as a walkmesh's root).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SourceMap {
    /// Per node of [`Model::nodes`] (same order): its `node` … `endnode`
    /// lines.
    pub nodes: Vec<Span>,
    /// Per animation of [`Model::animations`]: its lines and its nodes'.
    pub animations: Vec<AnimationSpan>,
    /// What the reader left out or guessed, in line order.
    pub notes: Vec<Note>,
}

impl SourceMap {
    /// The node of [`Model::nodes`] whose block holds a line.
    pub fn node_at(&self, line: usize) -> Option<usize> {
        self.nodes.iter().position(|s| s.contains(line))
    }
}

/// First and last line of a block (inclusive).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn contains(&self, line: usize) -> bool {
        self.start > 0 && (self.start..=self.end).contains(&line)
    }
}

/// An animation's lines (`newanim` … `doneanim`) and, per node of
/// [`Animation::nodes`], its block's.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AnimationSpan {
    pub span: Span,
    pub nodes: Vec<Span>,
}

/// Something the reader left out or guessed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Note {
    pub line: usize,
    pub message: String,
}

/// A line's words, without comments, and its number in the text.
struct Line<'a> {
    number: usize,
    words: Vec<&'a str>,
    /// The blank lines just before it (in a list of skin weights, each is
    /// a vertex no bone moves).
    blank_before: usize,
}

fn lines(text: &str) -> Vec<Line<'_>> {
    let mut blank = 0;
    text.lines()
        .enumerate()
        .filter_map(|(i, l)| {
            if l.trim().is_empty() {
                blank += 1;
                return None;
            }
            let blank_before = std::mem::take(&mut blank);
            let l = l.split('#').next().unwrap_or_default();
            let words: Vec<&str> = l.split_whitespace().collect();
            (!words.is_empty()).then_some(Line { number: i + 1, words, blank_before })
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

/// An integer as C reads one: the number's whole part, wrapped to 32 bits.
fn int(w: Option<&&str>) -> u32 {
    let Some(w) = w else { return 0 };
    match w.parse::<i64>() {
        Ok(i) => i as u32,
        Err(_) => w.parse::<f64>().ok().filter(|f| f.is_finite()).map_or(0, |f| f as i64 as u32),
    }
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

/// A list as written: its keyword's line, the count it gives and its rows.
#[derive(Default)]
struct List<'a> {
    line: usize,
    rows: Vec<Vec<&'a str>>,
}

/// A key list: its controller, whether its keys are Bézier keys, and its
/// rows (time first).
struct RawKeys {
    name: String,
    line: usize,
    bezier: bool,
    rows: Vec<Vec<f32>>,
}

/// A node block as written: its type, name, single-line properties, lists
/// and key lists, and its lines.
#[derive(Default)]
struct RawNode<'a> {
    ty: String,
    name: String,
    parent: Option<String>,
    props: Vec<(String, Vec<&'a str>)>,
    lists: HashMap<String, List<'a>>,
    keys: Vec<RawKeys>,
    aabb: Vec<Vec<f32>>,
    span: Span,
}

impl<'a> RawNode<'a> {
    fn prop(&self, key: &str) -> Option<&Vec<&'a str>> {
        self.props.iter().rev().find(|(k, _)| k == key).map(|(_, v)| v)
    }

    fn list(&self, key: &str) -> &[Vec<&'a str>] {
        self.lists.get(key).map_or(&[], |l| l.rows.as_slice())
    }

    fn list_line(&self, key: &str) -> usize {
        self.lists.get(key).map_or(self.span.start, |l| l.line)
    }
}

fn note(notes: &mut Vec<Note>, line: usize, message: String) {
    notes.push(Note { line, message });
}

/// Parses a node block starting after its `node` line; returns the node and
/// the index of the line after `endnode`.
fn node_block<'a>(
    ls: &'a [Line<'a>],
    mut i: usize,
    header: &Line<'a>,
    notes: &mut Vec<Note>,
) -> (RawNode<'a>, usize) {
    let mut n = RawNode {
        ty: header.words.get(1).map(|w| w.to_ascii_lowercase()).unwrap_or_default(),
        name: header.words.get(2).map(|w| w.to_string()).unwrap_or_default(),
        span: Span { start: header.number, end: header.number },
        ..Default::default()
    };
    let mut closed = false;
    while i < ls.len() {
        let l = &ls[i];
        let key = l.words[0].to_ascii_lowercase();
        i += 1;
        n.span.end = l.number;
        match key.as_str() {
            "endnode" => {
                closed = true;
                break;
            }
            "node" | "newanim" | "doneanim" | "donemodel" | "endmodelgeom" => {
                i -= 1;
                n.span.end = ls[i - 1].number;
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
                    n.span.end = ls[i].number;
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
                    // A blank line among skin weights is a vertex without
                    // any (it stays where the mesh has it): the rows after
                    // it are still the vertices after it.
                    if k == "weights" && !rows.is_empty() {
                        let room = count - rows.len();
                        rows.extend(std::iter::repeat_n(Vec::new(), ls[i].blank_before.min(room)));
                        if rows.len() == count {
                            break;
                        }
                    }
                    let first = ls[i].words[0];
                    if first.eq_ignore_ascii_case("endnode")
                        || first.eq_ignore_ascii_case("endlist")
                        || !named && !is_number(first)
                    {
                        break;
                    }
                    rows.push(ls[i].words.clone());
                    n.span.end = ls[i].number;
                    i += 1;
                }
                if rows.len() < count {
                    note(notes, l.number, format!("{k}: {} of {count} rows", rows.len()));
                }
                if i < ls.len() && ls[i].words[0].eq_ignore_ascii_case("endlist") {
                    n.span.end = ls[i].number;
                    i += 1;
                }
                n.lists.insert(k.to_string(), List { line: l.number, rows });
            }
            k if k.ends_with("key") && k.len() > 3 => {
                let bezier = k.ends_with("bezierkey");
                let name = k.trim_end_matches("key").trim_end_matches("bezier").to_string();
                let count = l.words.get(1).and_then(|w| w.parse::<usize>().ok());
                let mut rows = Vec::new();
                while i < ls.len() && is_number(ls[i].words[0]) {
                    if count.is_some_and(|c| rows.len() >= c) {
                        break;
                    }
                    rows.push(ls[i].words.iter().filter_map(|w| w.parse().ok()).collect());
                    n.span.end = ls[i].number;
                    i += 1;
                }
                if let Some(c) = count.filter(|&c| rows.len() < c) {
                    note(notes, l.number, format!("{k}: {} of {c} keys", rows.len()));
                }
                if i < ls.len() && ls[i].words[0].eq_ignore_ascii_case("endlist") {
                    n.span.end = ls[i].number;
                    i += 1;
                }
                n.keys.push(RawKeys { name, line: l.number, bezier, rows });
            }
            _ => n.props.push((key, l.words[1..].to_vec())),
        }
    }
    if !closed {
        note(notes, header.number, format!("node {} has no endnode", n.name));
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
/// quaternions. Bézier rows hold a value and its two handles, each as wide
/// as the value; rows that cannot be split so are read as linear keys.
fn key_controller(name: &str, rows: &[Vec<f32>], bezier: bool) -> Controller {
    let orientation = name == "orientation";
    let width = rows.iter().map(|r| r.len().saturating_sub(1)).max().unwrap_or(0);
    let bezier = bezier && width >= 3 && width % 3 == 0;
    let columns = if bezier { width / 3 } else { width };
    let mut c = Controller {
        name: if name == "setfillumcolor" { "selfillumcolor".into() } else { name.into() },
        columns: if orientation { 4 } else { columns },
        ..Default::default()
    };
    for r in rows {
        c.times.push(r.first().copied().unwrap_or(0.0));
        let v = |i: usize| r.get(i + 1).copied().unwrap_or(0.0);
        let part = |start: usize, out: &mut Vec<f32>| {
            if orientation {
                out.extend(axis_angle([v(start), v(start + 1), v(start + 2)], v(start + 3)));
            } else {
                out.extend((start..start + columns).map(v));
            }
        };
        part(0, &mut c.values);
        if bezier {
            part(columns, &mut c.handles);
            part(2 * columns, &mut c.handles);
        }
    }
    c
}

/// De-indexes a mesh's faces; returns the mesh streams.
fn build_mesh(raw: &RawNode<'_>, m: &mut Mesh, notes: &mut Vec<Note>) {
    let verts = vec3s(raw.list("verts"));
    let tangents: Vec<[f32; 4]> = raw
        .list("tangents")
        .iter()
        .map(|r| {
            [num(r.first()), num(r.get(1)), num(r.get(2)), r.get(3).map_or(1.0, |w| num(Some(w)))]
        })
        .collect();
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
    let mut dropped = 0;
    let faces: Vec<F> = raw
        .list("faces")
        .iter()
        .filter_map(|r| {
            let n = |i: usize| r.get(i).and_then(|w| w.parse::<f32>().ok()).map(|v| v as i64);
            let v = [n(0)?, n(1)?, n(2)?];
            if v.iter().any(|&x| x < 0 || x as usize >= verts.len()) {
                dropped += 1;
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
    if dropped > 0 {
        note(
            notes,
            raw.list_line("faces"),
            format!(
                "{dropped} faces refer to vertices past the {} given and are left out",
                verts.len()
            ),
        );
    }
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
                if tangents.len() >= verts.len() {
                    m.tangents.push(tangents[v]);
                }
                m.source.push(v as u32);
                m.source_uv.push(t as u32);
                i
            });
        }
        m.faces.push(Face { vertices: out, material: f.material });
    }
}

/// A geometry node.
fn geometry_node(raw: &RawNode<'_>, notes: &mut Vec<Note>) -> Node {
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
                priority: int(p("lightpriority").and_then(|v| v.first())),
                ambient_only: flag(p("ambientonly").or(p("ambient_only")).and_then(|v| v.first())),
                dynamic_type: int(p("ndynamictype")
                    .or(p("n_dynamic_type"))
                    .or(p("isdynamic"))
                    .and_then(|v| v.first())),
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
                xgrid: int(p("xgrid").and_then(|v| v.first())),
                ygrid: int(p("ygrid").and_then(|v| v.first())),
                spawntype: int(p("spawntype").and_then(|v| v.first())),
                update: s("update"),
                render: s("render"),
                blend: s("blend"),
                texture: resname(p("texture").and_then(|v| v.first())),
                chunk: resname(p("chunkname").and_then(|v| v.first())),
                two_sided: flag(p("twosidedtex").and_then(|v| v.first())),
                looping: flag(p("loop").and_then(|v| v.first())),
                // Compiled models keep 16 bits.
                render_order: int(p("renderorder").and_then(|v| v.first())) & 0xFFFF,
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
                tilefade: int(p("tilefade").and_then(|v| v.first())),
                transparency_hint: int(p("transparencyhint").and_then(|v| v.first())),
                material: resname(p("materialname").and_then(|v| v.first())),
                renderhint: p("renderhint").and_then(|v| v.first()).map(|w| w.to_ascii_lowercase()),
                ..Default::default()
            };
            m.textures[0] = resname(p("texture0").or(p("bitmap")).and_then(|v| v.first()));
            for i in 1..4 {
                m.textures[i] = resname(p(&format!("texture{i}")).and_then(|v| v.first()));
            }
            build_mesh(raw, &mut m, notes);
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
/// unknown and `NULL` parents make roots, and later roots hang under the
/// first).
fn parents(raws: &[RawNode<'_>], notes: Option<&mut Vec<Note>>) -> Vec<Option<usize>> {
    let mut by_name: HashMap<String, usize> = HashMap::new();
    let mut out = Vec::with_capacity(raws.len());
    let mut unknown = Vec::new();
    for (i, r) in raws.iter().enumerate() {
        let named = r.parent.as_ref().filter(|p| !p.eq_ignore_ascii_case("null"));
        let p = named.and_then(|p| by_name.get(&p.to_ascii_lowercase()).copied());
        if let (Some(name), None) = (named, p)
            && i > 0
        {
            unknown
                .push((r.span.start, format!("{}: parent {name} is not a node before it", r.name)));
        }
        out.push(p);
        by_name.entry(r.name.to_ascii_lowercase()).or_insert(i);
    }
    if let Some(first) = out.iter().position(Option::is_none) {
        for (i, p) in out.iter_mut().enumerate() {
            if p.is_none() && i != first {
                *p = Some(first);
            }
        }
    }
    if let Some(notes) = notes {
        for (line, message) in unknown {
            note(notes, line, format!("{message}; it hangs from the root"));
        }
    }
    out
}

fn anim_node(raw: &RawNode<'_>, source_counts: &HashMap<String, (usize, usize)>) -> AnimNode {
    let node_flags = type_flags(&raw.ty);
    let mut n = AnimNode { name: raw.name.clone(), ..Default::default() };
    for k in &raw.keys {
        n.controllers.push(key_controller(&k.name, &k.rows, k.bezier));
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
        n.controllers.push(key_controller(k, &[row], false));
    }
    let verts = vec3s(raw.list("animverts"));
    let tverts = vec2s(raw.list("animtverts"));
    if !verts.is_empty() || !tverts.is_empty() {
        let (count, uv_count) =
            source_counts.get(&raw.name.to_ascii_lowercase()).copied().unwrap_or((0, 0));
        let split3 = |v: &[Vec3]| -> Vec<Vec<Vec3>> {
            if count == 0 { Vec::new() } else { v.chunks(count).map(<[Vec3]>::to_vec).collect() }
        };
        let split2 = |v: &[Vec2]| -> Vec<Vec<Vec2>> {
            if uv_count == 0 {
                Vec::new()
            } else {
                v.chunks(uv_count).map(<[Vec2]>::to_vec).collect()
            }
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
    read_mapped(data).map(|(m, _)| m)
}

/// Reads an ASCII model, and where its nodes and animations are in the
/// text.
pub fn read_mapped(data: &[u8]) -> Result<(Model, SourceMap), MdlError> {
    read_with(data, false)
}

/// Reads an ASCII model; with `implicit_root`, nodes whose parent is not in
/// the file (a walkmesh's, which hang from the object's root) hang from a
/// dummy of that name at the origin.
pub(crate) fn read_with(data: &[u8], implicit_root: bool) -> Result<(Model, SourceMap), MdlError> {
    let text = String::from_utf8_lossy(data);
    let ls = lines(&text);
    let mut notes = Vec::new();
    let mut model = Model { animation_scale: 1.0, ..Default::default() };
    let mut geometry: Vec<RawNode<'_>> = Vec::new();
    let mut anims: Vec<(Animation, Span, Vec<RawNode<'_>>)> = Vec::new();
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
                let span = Span { start: l.number, end: l.number };
                anims.push((Animation { name, ..Default::default() }, span, Vec::new()));
            }
            "doneanim" => {
                if let Some((_, span, _)) = anims.last_mut().filter(|_| in_anim) {
                    span.end = l.number;
                }
                in_anim = false;
            }
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
                let (raw, next) = node_block(&ls, i, l, &mut notes);
                i = next;
                if in_anim {
                    let (_, span, nodes) = anims.last_mut().expect("in anim");
                    span.end = raw.span.end;
                    nodes.push(raw);
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
    if implicit_root
        && let Some(parent) = geometry[0].parent.clone()
        && !parent.eq_ignore_ascii_case("null")
        && !geometry.iter().any(|r| r.name.eq_ignore_ascii_case(&parent))
    {
        let root = RawNode { ty: "dummy".into(), name: parent, ..Default::default() };
        geometry.insert(0, root);
    }
    if model.name.is_empty() {
        model.name = geometry[0].name.clone();
    }

    // Geometry in pre-order.
    let parent_of = parents(&geometry, Some(&mut notes));
    let (order, new_index) = preorder(&parent_of);
    let mut nodes: Vec<Node> =
        order.iter().map(|&old| geometry_node(&geometry[old], &mut notes)).collect();
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
        let mut unknown: HashSet<String> = HashSet::new();
        let per_source: Vec<[(u16, f32); 4]> = raw
            .list("weights")
            .iter()
            .map(|r| {
                let mut out = [(0u16, 0.0f32); 4];
                for (j, pair) in r.chunks(2).take(4).enumerate() {
                    let (Some(name), Some(w)) = (pair.first(), pair.get(1)) else { continue };
                    let Some(&node) = by_name.get(&name.to_ascii_lowercase()) else {
                        unknown.insert(name.to_ascii_lowercase());
                        continue;
                    };
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
        let mut unknown: Vec<String> = unknown.into_iter().collect();
        unknown.sort();
        for name in unknown {
            note(&mut notes, raw.list_line("weights"), format!("weights: no node {name}"));
        }
        skin.weights = m
            .source
            .iter()
            .map(|&s| per_source.get(s as usize).copied().unwrap_or([(0, 0.0); 4]))
            .collect();
    }
    model.nodes = nodes;
    let mut map = SourceMap {
        nodes: order.iter().map(|&old| geometry[old].span).collect(),
        ..Default::default()
    };

    // Animations: nodes in pre-order, animated vertices and UVs over the
    // mesh's source vertices and texture vertices.
    let source_counts: HashMap<String, (usize, usize)> = geometry
        .iter()
        .map(|r| {
            let uvs =
                if r.list("tverts").is_empty() { r.list("tverts0") } else { r.list("tverts") };
            (r.name.to_ascii_lowercase(), (r.list("verts").len(), uvs.len()))
        })
        .collect();
    for (mut anim, span, raws) in anims {
        let parent_of = parents(&raws, None);
        let (order, new_index) = preorder(&parent_of);
        anim.nodes = order
            .iter()
            .map(|&old| {
                let mut n = anim_node(&raws[old], &source_counts);
                n.parent = parent_of[old].map(|p| new_index[p]);
                n
            })
            .collect();
        for r in &raws {
            for k in r.keys.iter().filter(|k| k.bezier) {
                let width = k.rows.iter().map(|r| r.len().saturating_sub(1)).max().unwrap_or(0);
                if width < 3 || width % 3 != 0 {
                    note(
                        &mut notes,
                        k.line,
                        format!("{}bezierkey: rows need a value and two handles", k.name),
                    );
                }
            }
        }
        map.animations
            .push(AnimationSpan { span, nodes: order.iter().map(|&old| raws[old].span).collect() });
        model.animations.push(anim);
    }
    notes.sort_by_key(|n| n.line);
    map.notes = notes;
    Ok((model, map))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A blank line among a skin's weights is a vertex without any: the
    /// rows after it stay with their vertices (a Vault model, Mindwitness).
    #[test]
    fn a_blank_line_among_skin_weights_is_a_vertex_without_any() {
        let text = "\
newmodel s
setsupermodel s NULL
beginmodelgeom s
node dummy s
  parent NULL
endnode
node dummy bone_a
  parent s
endnode
node dummy bone_b
  parent s
endnode
node skin hide
  parent s
  bitmap x
  verts 3
    0 0 0
    1 0 0
    0 1 0
  tverts 3
    0 0 0
    1 0 0
    0 1 0
  faces 1
    0 1 2 1 0 1 2 0
  weights 3
    bone_a 1.0
    
    bone_b 1.0
endnode
endmodelgeom s
donemodel s
";
        let m = read(text.as_bytes()).unwrap();
        let hide = m.nodes.iter().find(|n| n.name == "hide").unwrap();
        let NodeKind::Mesh(mesh) = &hide.kind else { panic!("a mesh") };
        let MeshExtra::Skin(skin) = &mesh.extra else { panic!("a skin") };
        let of = |source: u32| {
            let v = mesh.source.iter().position(|&s| s == source).unwrap();
            skin.weights[v]
                .iter()
                .filter(|w| w.1 > 0.0)
                .map(|w| m.nodes[skin.bones[w.0 as usize]].name.as_str())
                .collect::<Vec<_>>()
        };
        assert_eq!(of(0), ["bone_a"]);
        assert!(of(1).is_empty(), "the blank line's vertex has no bone");
        assert_eq!(of(2), ["bone_b"], "the last keeps its own");
    }

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
    fn integers_wrap_as_in_c() {
        let mdl = "newmodel t\nbeginmodelgeom t\nnode dummy t\nendnode\nnode emitter e\nparent t\n\
                   spawntype -1\nxgrid 2.7\nrenderorder -2\nendnode\nendmodelgeom t\n";
        let m = read(mdl.as_bytes()).unwrap();
        let NodeKind::Emitter(e) = &m.nodes[1].kind else { panic!("emitter") };
        assert_eq!(e.spawntype, 0xFFFF_FFFF, "as the game's compiler stores it");
        assert_eq!(e.xgrid, 2);
        assert_eq!(e.render_order, 0xFFFE, "16 bits, as compiled");
    }

    #[test]
    fn bezier_keys_keep_their_handles() {
        let mdl = "newmodel t\nbeginmodelgeom t\nnode dummy t\nendnode\nendmodelgeom t\n\
                   newanim a t\nlength 2\nnode dummy t\nparent NULL\n\
                   positionbezierkey 2\n0 0 0 0 0.1 0.2 0.3 0.4 0.5 0.6\n2 1 1 1 0.7 0.8 0.9 1.1 1.2 1.3\n\
                   endlist\nscalebezierkey 1\n0 1 2\nendnode\ndoneanim a t\n";
        let (m, map) = read_mapped(mdl.as_bytes()).unwrap();
        let c = &m.animations[0].nodes[0].controllers;
        assert_eq!(c[0].name, "position");
        assert_eq!((c[0].columns, c[0].row(1)), (3, &[1.0, 1.0, 1.0][..]));
        assert!(c[0].is_bezier());
        assert_eq!(c[0].handles[..6], [0.1, 0.2, 0.3, 0.4, 0.5, 0.6]);
        // Two values a key cannot be a value and two handles: linear.
        assert!(!c[1].is_bezier() && c[1].columns == 2);
        assert_eq!(map.notes.len(), 1, "{:?}", map.notes);
        assert!(map.notes[0].message.starts_with("scalebezierkey"));
    }

    #[test]
    fn tangents_follow_their_vertex() {
        let mdl = "newmodel t\nbeginmodelgeom t\nnode dummy t\nendnode\nnode trimesh m\nparent t\n\
                   renderhint NormalAndSpecMapped\nverts 3\n0 0 0\n1 0 0\n0 1 0\n\
                   tangents 3\n1 0 0 1\n0 1 0 -1\n0 0 1 1\nfaces 1\n2 1 0 1 0 0 0 0\n\
                   endnode\nendmodelgeom t\n";
        let m = read(mdl.as_bytes()).unwrap();
        let mesh = m.nodes[1].mesh().unwrap();
        assert_eq!(mesh.renderhint.as_deref(), Some("normalandspecmapped"));
        let at = |v: u32| mesh.tangents[mesh.source.iter().position(|&s| s == v).unwrap()];
        assert_eq!(at(1), [0.0, 1.0, 0.0, -1.0]);
        assert_eq!(at(2), [0.0, 0.0, 1.0, 1.0]);
    }

    #[test]
    fn source_map_and_notes() {
        let text = "\
newmodel t

beginmodelgeom t
node dummy t
  parent NULL
endnode
node trimesh b
  parent a
  verts 3
    0 0 0
    1 0 0
  faces 2
    0 1 2 1 0 0 0 0
    0 1 1 1 0 0 0 0
endnode
node dummy a
  parent t
endmodelgeom t
newanim go t
  node dummy t
    parent NULL
  endnode
doneanim go t
";
        let (m, map) = read_mapped(text.as_bytes()).unwrap();
        let names: Vec<&str> = m.nodes.iter().map(|n| n.name.as_str()).collect();
        assert_eq!(names, ["t", "b", "a"]);
        assert_eq!(map.nodes[0], Span { start: 4, end: 6 });
        assert_eq!(map.nodes[1], Span { start: 7, end: 15 });
        assert_eq!(map.nodes[2], Span { start: 16, end: 17 }, "no endnode: to its last line");
        assert_eq!(map.node_at(10), Some(1));
        assert_eq!(map.node_at(2), None);
        assert_eq!(map.animations[0].span, Span { start: 19, end: 23 });
        assert_eq!(map.animations[0].nodes, [Span { start: 20, end: 22 }]);
        let notes: Vec<(usize, &str)> =
            map.notes.iter().map(|n| (n.line, n.message.as_str())).collect();
        assert_eq!(
            notes,
            [
                (7, "b: parent a is not a node before it; it hangs from the root"),
                (9, "verts: 2 of 3 rows"),
                (12, "1 faces refer to vertices past the 2 given and are left out"),
                (16, "node a has no endnode"),
            ]
        );
    }

    #[test]
    fn preorder_follows_file_order_of_children() {
        let (order, _) = preorder(&[None, Some(0), Some(0), Some(1)]);
        assert_eq!(order, [0, 1, 3, 2]);
    }
}
