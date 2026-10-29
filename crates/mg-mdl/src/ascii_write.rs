//! Models as ASCII: the native decompiler.
//!
//! The layout follows nwnmdlcomp's decompiler (Torlack's `NwnMdlDecomp.cpp`),
//! which the game, nwnmdlcomp and Neverblender all read: a header, the
//! geometry in pre-order, then the animations, controllers last in each node,
//! key lists with their counts. What differs, and why:
//!
//! - **Vertices are welded by everything a vertex carries** (position,
//!   normal, colour, dangly constraint, skin weights, animated positions),
//!   not just position, colour and constraint: binary models store a vertex
//!   per face corner, and welding only by position would merge vertices
//!   whose weights or animation differ.
//! - **Normals are written** (`normals`, an EE keyword), so the game and the
//!   engine compiler rebuild exactly the stored normals; smoothing groups
//!   are still derived from them (faces meeting at a position with the same
//!   normal share a group), for nwnmdlcomp, which ignores `normals`.
//! - **Texture vertices are welded across all four UV sets together**, and
//!   faces' texture indices serve every set, as the game's ASCII reader and
//!   Neverblender apply them; `texindices1..3` repeat them for nwnmdlcomp.
//! - **An animated mesh's frames** (`animverts`, `animtverts`) are written
//!   over its `verts` and `tverts` (nwn-mdl-spec MDL-MESH-062), its
//!   vertices split by their UVs kept apart, so that each of the compiled
//!   model's vertices has its frames; and its animation node repeats the
//!   mesh's `verts`, `tverts` and `faces`, which the game divides the
//!   frames' counts by (without them it crashes, the spec's E54).
//! - Numbers are the shortest text that reads back as the same `f32`.
//! - Self-illumination is written twice: `setfillumcolor`, the spelling
//!   nwnmdlcomp compiles (and 2,387 of the game's own ASCII models use),
//!   then `selfillumcolor`, the only one the game's compiler keeps (checked
//!   by compiling in the game).

use std::collections::HashMap;
use std::fmt::Write as _;

use crate::{
    AabbEntry, AnimNode, Animation, Classification, Controller, EMITTER_FLAGS, Mesh, MeshExtra,
    Model, Node, NodeKind, Quat,
};

/// What to write.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Options {
    /// Write `normals` lists (EE). Without them, readers compute normals
    /// from the smoothing groups.
    pub normals: bool,
    /// A comment line naming the writer.
    pub banner: bool,
}

impl Default for Options {
    fn default() -> Options {
        Options { normals: true, banner: true }
    }
}

/// A model as ASCII text, its nodes in the tree's order.
pub fn to_ascii(model: &Model, opts: &Options) -> String {
    let order: Vec<usize> = (0..model.nodes.len()).collect();
    to_ascii_in_order(model, opts, &order)
}

/// A model as ASCII text, its nodes written in `order` (indices of
/// `model.nodes`, each once, a parent before its children: [`file_order`]).
/// Compilers number a model's nodes in the order of its text, and the game
/// finds a supermodel's animations by those numbers.
pub fn to_ascii_in_order(model: &Model, opts: &Options, order: &[usize]) -> String {
    to_ascii_on(model, opts, order, None)
}

/// [`to_ascii_in_order`] of a body part, whose skins may be bound to the
/// creature's skeleton it is worn on (`skeleton`; nwn-mdl-spec
/// MDL-SKIN-012): their bones are the skeleton's nodes, named from it
/// (MDL-SKIN-040). Without the skeleton such bones are left unnamed
/// (`invalidnodeindex`, as nwnmdlcomp writes a bone it cannot name), and
/// a comment says so.
pub fn to_ascii_on(
    model: &Model,
    opts: &Options,
    order: &[usize],
    skeleton: Option<&Model>,
) -> String {
    let skeleton =
        skeleton.map(|s| s.tree_order().into_iter().map(|i| s.nodes[i].name.clone()).collect());
    let mut w = Writer { out: String::new(), opts: *opts, skeleton };
    w.model(model, order);
    w.out
}

/// Whether a skin of `model` is bound to a skeleton it is worn on, not to
/// its own tree: a bone lies past the model's last node (nwn-mdl-spec
/// MDL-SKIN-040; Beamdog's HD `pmh0_neck001` has three nodes and bones on
/// the skeleton's positions 2, 4 and 5).
pub fn bound_elsewhere(model: &Model, skin: &crate::Skin) -> bool {
    skin.bones.iter().any(|&b| b >= model.nodes.len())
}

/// The skeleton a body part is worn on, by the part's name: `pmh0` for
/// `pmh0_neck001` (`p<gender><race><phenotype>_<part><number>`).
pub fn skeleton_of(part: &str) -> Option<&str> {
    let (skeleton, rest) = part.split_once('_')?;
    let is = skeleton.len() == 4
        && skeleton.starts_with(['p', 'P'])
        && skeleton[3..].bytes().all(|b| b.is_ascii_digit())
        && !rest.is_empty();
    is.then_some(skeleton)
}

/// The order to write a compiled model's nodes in so that a compiler
/// numbers them as they are numbered (`numbers`: each node's part number,
/// [`crate::binary_write::PartNumbers`]): by number, a parent still before its
/// children. A node without a number (−1: its supermodel lacks it) goes
/// where the tree has it, after the node before it. So a model decompiled
/// and compiled again keeps its numbers, where they are its own (with a
/// supermodel, BioWare's numbers can have gaps no text reproduces). A
/// model with two nodes of one name stays in its tree's order, and so does
/// one whose skin would be written before a bone of its own.
pub fn file_order(model: &Model, numbers: &[i32]) -> Vec<usize> {
    let count = model.nodes.len();
    // Two nodes of one name: text tells them apart only by a child
    // following its parent, which the tree's order keeps.
    let mut names = std::collections::HashSet::new();
    if model.nodes.iter().any(|n| !names.insert(n.name.to_ascii_lowercase())) {
        return (0..count).collect();
    }
    // Each node's place: its number, or the place of the node before it.
    let mut place = vec![0i64; count];
    for i in 0..count {
        place[i] = match numbers.get(i) {
            Some(&n) if n >= 0 => i64::from(n),
            _ if i > 0 => place[i - 1],
            _ => -1,
        };
    }
    let mut written = vec![false; count];
    let mut order = Vec::with_capacity(count);
    let mut ready: std::collections::BTreeSet<(i64, usize)> = model
        .nodes
        .iter()
        .enumerate()
        .filter(|(_, n)| n.parent.is_none_or(|p| p >= count))
        .map(|(i, _)| (place[i], i))
        .collect();
    while let Some((_, i)) = ready.pop_first() {
        if std::mem::replace(&mut written[i], true) {
            continue;
        }
        order.push(i);
        for &c in model.nodes[i].children.iter().filter(|&&c| c < count) {
            ready.insert((place[c], c));
        }
    }
    // (A node no parent leads to, in a broken tree: last, as it is.)
    order.extend((0..count).filter(|&i| !written[i]));
    // A skin's weights name its bones, and nwnmdlcomp knows only the nodes
    // it has read by then: where a skin would come before one of its
    // bones, the tree's order, which has them first.
    let mut at = vec![0usize; count];
    for (position, &i) in order.iter().enumerate() {
        at[i] = position;
    }
    let early = model.nodes.iter().enumerate().any(|(i, n)| match n.mesh().map(|m| &m.extra) {
        Some(crate::MeshExtra::Skin(skin)) => {
            skin.bones.iter().any(|&b| b < count && at[b] > at[i])
        }
        _ => false,
    });
    if early {
        return (0..count).collect();
    }
    order
}

/// A number as the shortest text that reads back as the same `f32`.
pub fn num(v: f32) -> String {
    if v == 0.0 || !v.is_finite() {
        return "0".into();
    }
    let a = v.abs();
    if !(1e-4..1e7).contains(&a) { format!("{v:e}") } else { format!("{v}") }
}

/// Spellings of controller names (lower case in `mg-mdl`) as nwnmdlcomp
/// writes them; others are written as they are.
fn controller_keyword(name: &str) -> &str {
    match name {
        "selfillumcolor" => "setfillumcolor",
        "colorstart" => "colorStart",
        "colormid" => "colorMid",
        "colorend" => "colorEnd",
        "alphastart" => "alphaStart",
        "alphamid" => "alphaMid",
        "alphaend" => "alphaEnd",
        "sizestart" => "sizeStart",
        "sizemid" => "sizeMid",
        "sizeend" => "sizeEnd",
        "sizestart_y" => "sizeStart_y",
        "sizemid_y" => "sizeMid_y",
        "sizeend_y" => "sizeEnd_y",
        "framestart" => "frameStart",
        "frameend" => "frameEnd",
        "lifeexp" => "lifeExp",
        "particlerot" => "particleRot",
        "blurlength" => "blurLength",
        "lightningdelay" => "lightningDelay",
        "lightningradius" => "lightningRadius",
        "lightningscale" => "lightningScale",
        "percentstart" => "percentStart",
        "percentmid" => "percentMid",
        "percentend" => "percentEnd",
        n => n,
    }
}

/// Axis and angle (radians) of a rotation; identity is `0 0 0 0`.
pub fn axis_angle(q: Quat) -> [f32; 4] {
    let [x, y, z, w] = q.map(f64::from);
    let len = (x * x + y * y + z * z + w * w).sqrt();
    if len < 1e-12 {
        return [0.0; 4];
    }
    let (x, y, z, w) = (x / len, y / len, z / len, w / len);
    let s = (x * x + y * y + z * z).sqrt();
    if s < 1e-12 {
        return [0.0; 4];
    }
    let angle = 2.0 * s.atan2(w);
    [(x / s) as f32, (y / s) as f32, (z / s) as f32, angle as f32]
}

struct Writer {
    out: String,
    opts: Options,
    /// The names of the skeleton's nodes by tree position, for skins bound
    /// to it ([`to_ascii_on`]).
    skeleton: Option<Vec<String>>,
}

/// How a mesh's corners map to written vertices and texture vertices.
struct Welded {
    /// Per written vertex: the mesh vertex it stands for.
    verts: Vec<usize>,
    /// Per mesh vertex: its written vertex.
    vert_of: Vec<usize>,
    /// Per written texture vertex: the mesh vertex it stands for.
    tverts: Vec<usize>,
    /// Per mesh vertex: its written texture vertex.
    tvert_of: Vec<usize>,
    /// Smoothing-group bit mask per face.
    smoothing: Vec<u32>,
}

/// Animated vertex and UV sets of a mesh node, from every animation.
#[derive(Default)]
struct AnimSets<'a> {
    verts: Vec<&'a [crate::Vec3]>,
    uvs: Vec<&'a [crate::Vec2]>,
}

fn bits3(v: [f32; 3]) -> [u32; 3] {
    v.map(f32::to_bits)
}

fn weld(m: &Mesh, anim: &AnimSets<'_>, use_normals: bool) -> Welded {
    let n = m.vertices.len();
    // Vertices: everything per vertex must agree.
    let mut vkey_index: HashMap<Vec<u32>, usize> = HashMap::new();
    let mut verts = Vec::new();
    let mut vert_of = vec![0; n];
    let mut tkey_index: HashMap<Vec<u32>, usize> = HashMap::new();
    let mut tverts = Vec::new();
    let mut tvert_of = vec![0; n];
    let weights = match &m.extra {
        MeshExtra::Skin(s) => Some(&s.weights),
        _ => None,
    };
    let constraints = match &m.extra {
        MeshExtra::Dangly(d) => Some(&d.constraints),
        _ => None,
    };
    // In face order, so vertices are numbered as faces first use them.
    let mut order: Vec<usize> = Vec::with_capacity(n);
    let mut seen = vec![false; n];
    for f in &m.faces {
        for &v in &f.vertices {
            let v = v as usize;
            if v < n && !std::mem::replace(&mut seen[v], true) {
                order.push(v);
            }
        }
    }
    order.extend((0..n).filter(|&v| !seen[v]));
    // An animated mesh's sets are read over its `verts` (nwn-mdl-spec
    // MDL-MESH-062), and a vertex the game split by its UVs carries its
    // sets twice: such vertices stay apart, so that the sets are written
    // once for each of the compiled model's vertices, as it holds them.
    let animated = !anim.verts.is_empty() || !anim.uvs.is_empty();
    for v in order {
        let mut key: Vec<u32> = bits3(m.vertices[v]).to_vec();
        if animated {
            for uv in &m.uvs {
                if let Some(t) = uv.get(v) {
                    key.extend(t.map(f32::to_bits));
                }
            }
        }
        if use_normals && let Some(nv) = m.normals.get(v) {
            key.extend(bits3(*nv));
        }
        if use_normals && let Some(t) = m.tangents.get(v) {
            key.extend(t.map(f32::to_bits));
        }
        if let Some(c) = m.colors.get(v) {
            key.push(u32::from_le_bytes(*c));
        }
        if let Some(c) = constraints.and_then(|c| c.get(v)) {
            key.push(c.to_bits());
        }
        if let Some(w) = weights.and_then(|w| w.get(v)) {
            for (bone, weight) in w.iter().filter(|(_, w)| *w != 0.0) {
                key.push(u32::from(*bone));
                key.push(weight.to_bits());
            }
            key.push(u32::MAX);
        }
        for set in &anim.verts {
            if let Some(p) = set.get(v) {
                key.extend(bits3(*p));
            }
        }
        let next = verts.len();
        let i = *vkey_index.entry(key).or_insert(next);
        if i == next {
            verts.push(v);
        }
        vert_of[v] = i;

        let mut tkey: Vec<u32> = Vec::new();
        for uv in &m.uvs {
            if let Some(t) = uv.get(v) {
                tkey.extend(t.map(f32::to_bits));
            }
        }
        for set in &anim.uvs {
            if let Some(t) = set.get(v) {
                tkey.extend(t.map(f32::to_bits));
            }
        }
        let next = tverts.len();
        let i = *tkey_index.entry(tkey).or_insert(next);
        if i == next {
            tverts.push(v);
        }
        tvert_of[v] = i;
    }
    let smoothing = smoothing_groups(m);
    Welded { verts, vert_of, tverts, tvert_of, smoothing }
}

/// Smoothing groups from the stored normals, as nwnmdlcomp's decompiler
/// derives them: faces that meet at a position with (almost) the same
/// normal there share a group; a face meeting another at a position with a
/// different normal gets a group that one does not have.
fn smoothing_groups(m: &Mesh) -> Vec<u32> {
    let nf = m.faces.len();
    if m.normals.len() < m.vertices.len() || nf == 0 {
        return vec![1; nf];
    }
    // Faces at each position.
    let mut at: HashMap<[u32; 3], Vec<(usize, usize)>> = HashMap::new();
    for (fi, f) in m.faces.iter().enumerate() {
        for &v in &f.vertices {
            if let Some(p) = m.vertices.get(v as usize) {
                at.entry(bits3(*p)).or_default().push((fi, v as usize));
            }
        }
    }
    let close = |a: [f32; 3], b: [f32; 3]| (0..3).all(|i| (a[i] - b[i]).abs() <= 1e-5);
    let mut group = vec![0usize; nf];
    let mut next_group = 1;
    for start in 0..nf {
        if group[start] != 0 {
            continue;
        }
        // The faces smoothly connected to this one, and the groups of
        // faces they meet at a crease.
        let mut component = vec![start];
        let mut conflicts = 0u64;
        let mut stack = vec![start];
        let mut visited = std::collections::HashSet::from([start]);
        while let Some(fi) = stack.pop() {
            for &v in &m.faces[fi].vertices {
                let Some(p) = m.vertices.get(v as usize) else { continue };
                let n = m.normals[v as usize];
                for &(other, ov) in &at[&bits3(*p)] {
                    if other == fi {
                        continue;
                    }
                    if close(m.normals[ov], n) {
                        if visited.insert(other) {
                            component.push(other);
                            stack.push(other);
                        }
                    } else if group[other] != 0 {
                        conflicts |= 1 << (group[other] - 1).min(63);
                    }
                }
            }
        }
        let g = (1..next_group).find(|g| conflicts & (1 << (g - 1)) == 0).unwrap_or(next_group);
        if g == next_group {
            next_group += 1;
        }
        for f in component {
            group[f] = g;
        }
    }
    group.iter().map(|&g| 1u32 << (g.clamp(1, 32) - 1)).collect()
}

impl Writer {
    fn line(&mut self, s: &str) {
        self.out.push_str(s);
        self.out.push('\n');
    }

    fn model(&mut self, m: &Model, order: &[usize]) {
        self.line("#MAXMODEL ASCII");
        if self.opts.banner {
            self.line(&format!("# model: {} (decompiled by Moonglow)", m.name));
        }
        self.line(&format!("filedependancy {}.max", m.name));
        self.line(&format!("newmodel {}", m.name));
        self.line(&format!(
            "setsupermodel {} {}",
            m.name,
            m.supermodel.as_deref().unwrap_or("NULL")
        ));
        let class = match m.classification {
            Classification::Effect => Some("effect"),
            Classification::Tile => Some("tile"),
            Classification::Character => Some("character"),
            Classification::Door => Some("door"),
            Classification::Other => None,
        };
        if let Some(c) = class {
            self.line(&format!("classification {c}"));
        }
        self.line(&format!("setanimationscale {}", num(m.animation_scale)));
        if m.ignore_fog {
            self.line("ignorefog 1");
        }

        // Animated vertex sets per mesh node, gathered from every
        // animation, so welding keeps vertices whose animation differs.
        let mut sets: HashMap<String, AnimSets<'_>> = HashMap::new();
        for a in &m.animations {
            for n in &a.nodes {
                if let Some(am) = &n.anim_mesh {
                    let e = sets.entry(n.name.to_ascii_lowercase()).or_default();
                    e.verts.extend(am.vertex_sets.iter().map(Vec::as_slice));
                    e.uvs.extend(am.uv_sets.iter().map(Vec::as_slice));
                }
            }
        }
        let empty = AnimSets::default();
        let welded: Vec<Option<Welded>> = m
            .nodes
            .iter()
            .map(|n| {
                let s = sets.get(&n.name.to_ascii_lowercase()).unwrap_or(&empty);
                n.mesh().map(|mesh| weld(mesh, s, self.opts.normals))
            })
            .collect();

        self.line("#MAXGEOM  ASCII");
        self.line(&format!("beginmodelgeom {}", m.name));
        for &i in order.iter().filter(|&&i| i < m.nodes.len()) {
            self.node(m, &m.nodes[i], welded[i].as_ref());
        }
        self.line(&format!("endmodelgeom {}", m.name));

        let by_name: HashMap<String, usize> = m
            .nodes
            .iter()
            .enumerate()
            .rev()
            .map(|(i, n)| (n.name.to_ascii_lowercase(), i))
            .collect();
        for a in &m.animations {
            self.animation(m, a, &by_name, &welded);
        }
        self.line("");
        self.line(&format!("donemodel {}", m.name));
    }

    fn node(&mut self, m: &Model, n: &Node, welded: Option<&Welded>) {
        self.line(&format!("node {} {}", n.kind.type_name(), n.name));
        let parent = n.parent.map_or("NULL", |p| m.nodes[p].name.as_str());
        self.line(&format!("  parent {parent}"));
        if n.inherit_color {
            self.line("  inheritcolor 1");
        }
        match &n.kind {
            NodeKind::Light(l) => {
                self.line(&format!("  ambientonly {}", u8::from(l.ambient_only)));
                self.line(&format!("  shadow {}", u8::from(l.shadow)));
                self.line(&format!("  isdynamic {}", int(l.dynamic_type)));
                self.line(&format!("  affectdynamic {}", u8::from(l.affect_dynamic)));
                self.line(&format!("  lightpriority {}", int(l.priority)));
                if l.generate_flare {
                    self.line("  generateflare 1");
                }
                self.line(&format!("  fadingLight {}", u8::from(l.fading)));
                self.line(&format!("  flareradius {}", num(l.flare_radius)));
                if !l.flare_textures.is_empty() {
                    self.line(&format!("  texturenames {}", l.flare_textures.len()));
                    for t in &l.flare_textures {
                        self.line(&format!("    {t}"));
                    }
                }
                if !l.flare_sizes.is_empty() {
                    self.line(&format!("  flaresizes {}", l.flare_sizes.len()));
                    for s in &l.flare_sizes {
                        self.line(&format!("    {}", num(*s)));
                    }
                }
                if !l.flare_positions.is_empty() {
                    self.line(&format!("  flarepositions {}", l.flare_positions.len()));
                    for s in &l.flare_positions {
                        self.line(&format!("    {}", num(*s)));
                    }
                }
                if !l.flare_color_shifts.is_empty() {
                    self.line(&format!("  flarecolorshifts {}", l.flare_color_shifts.len()));
                    for c in &l.flare_color_shifts {
                        self.line(&format!("    {}", vec(c)));
                    }
                }
            }
            NodeKind::Emitter(e) => {
                for (name, bit) in EMITTER_FLAGS {
                    let name = match name {
                        "affectedbywind" => "affectedByWind",
                        "m_istinted" => "m_isTinted",
                        n => n,
                    };
                    self.line(&format!("  {name} {}", u8::from(e.flags & bit != 0)));
                }
                self.line(&format!("  renderorder {}", int(e.render_order)));
                // 0 (per second) or 1 (per metre moved, a trail); some
                // EE-compiled models hold −1 (which the nwnmdlcomp back end
                // gives that compiler as 0, all it accepts besides 1).
                self.line(&format!("  spawntype {}", int(e.spawntype)));
                self.line(&format!("  update {}", word(&e.update)));
                self.line(&format!("  render {}", word(&e.render)));
                self.line(&format!("  blend {}", word(&e.blend)));
                self.line(&format!("  texture {}", e.texture.as_deref().unwrap_or("NULL")));
                if let Some(c) = &e.chunk {
                    self.line(&format!("  chunkName {c}"));
                }
                self.line(&format!("  xgrid {}", int(e.xgrid)));
                self.line(&format!("  ygrid {}", int(e.ygrid)));
                self.line(&format!("  loop {}", u8::from(e.looping)));
                self.line(&format!("  deadspace {}", num(e.deadspace)));
                self.line(&format!("  twosidedtex {}", u8::from(e.two_sided)));
                self.line(&format!("  blastRadius {}", num(e.blast_radius)));
                self.line(&format!("  blastLength {}", num(e.blast_length)));
            }
            NodeKind::Reference(r) => {
                self.line(&format!("  refModel {}", r.model.as_deref().unwrap_or("NULL")));
                self.line(&format!("  reattachable {}", u8::from(r.reattachable)));
            }
            NodeKind::Mesh(mesh) => {
                if let Some(w) = welded {
                    self.mesh(m, mesh, w);
                }
            }
            NodeKind::Dummy | NodeKind::Camera => {}
        }
        // The rest pose.
        self.line(&format!("  position {}", vec(&n.position)));
        self.line(&format!("  orientation {}", quat(n.orientation)));
        if n.scale != 1.0 {
            self.line(&format!("  scale {}", num(n.scale)));
        }
        for c in &n.controllers {
            self.rest_controller(c);
        }
        self.line("endnode");
    }

    /// A rest-pose controller: its first row (readers keep only that).
    /// An emitter's `detonate` has no values: a bare line, which the
    /// game's compiler makes the same controller of (nwn-mdl-spec
    /// MDL-CTRL-012; the stock `vwp_flash_blured` has one).
    fn rest_controller(&mut self, c: &Controller) {
        if c.name.starts_with("ctrl") {
            self.line(&format!("  # {} not written: no ASCII keyword", c.name));
            return;
        }
        if c.name == "detonate" {
            if !c.times.is_empty() {
                self.line("  detonate");
            }
            return;
        }
        if c.columns == 0 || c.times.is_empty() {
            return;
        }
        let values: Vec<String> = c.row(0).iter().map(|v| num(*v)).collect();
        self.line(&format!("  {} {}", controller_keyword(&c.name), values.join(" ")));
        // The game's compiler knows only this spelling, nwnmdlcomp only
        // the other: each reads its own and skips the other's.
        if c.name == "selfillumcolor" {
            self.line(&format!("  selfillumcolor {}", values.join(" ")));
        }
    }

    /// A mesh's `verts`, as welded.
    fn verts(&mut self, m: &Mesh, w: &Welded) {
        self.line(&format!("  verts {}", w.verts.len()));
        for &v in &w.verts {
            self.line(&format!("    {}", vec(&m.vertices[v])));
        }
    }

    /// A mesh's texture vertices of a UV set (`tverts`, `tverts1`…).
    fn tverts(&mut self, m: &Mesh, w: &Welded, set: usize) {
        let key = if set == 0 { "tverts".to_string() } else { format!("tverts{set}") };
        self.line(&format!("  {key} {}", w.tverts.len()));
        for &v in &w.tverts {
            let [u, t] = m.uvs[set][v];
            self.line(&format!("    {} {} 0", num(u), num(t)));
        }
    }

    /// A mesh's `faces`: corners, smoothing group, texture vertices (where
    /// `uv0`: it has UVs) and material.
    fn faces(&mut self, m: &Mesh, w: &Welded, uv0: bool) {
        if m.faces.is_empty() {
            return;
        }
        self.line(&format!("  faces {}", m.faces.len()));
        for (fi, f) in m.faces.iter().enumerate() {
            let v = f.vertices.map(|v| w.vert_of.get(v as usize).copied().unwrap_or(0));
            let t = if uv0 {
                f.vertices.map(|v| w.tvert_of.get(v as usize).copied().unwrap_or(0))
            } else {
                [0; 3]
            };
            let _ = writeln!(
                self.out,
                "    {} {} {} {} {} {} {} {}",
                v[0], v[1], v[2], w.smoothing[fi], t[0], t[1], t[2], f.material
            );
        }
    }

    fn mesh(&mut self, model: &Model, m: &Mesh, w: &Welded) {
        let walkmesh = matches!(m.extra, MeshExtra::Aabb(_));
        self.line(&format!("  ambient {}", vec(&m.ambient)));
        self.line(&format!("  diffuse {}", vec(&m.diffuse)));
        self.line(&format!("  specular {}", vec(&m.specular)));
        self.line(&format!("  shininess {}", num(m.shininess)));
        // Flags where they differ from what readers assume (walkmeshes are
        // neither drawn nor shadowed unless they say so).
        if m.shadow == walkmesh {
            self.line(&format!("  shadow {}", u8::from(m.shadow)));
        }
        if m.transparency_hint != 0 {
            self.line(&format!("  transparencyhint {}", int(m.transparency_hint)));
        }
        if m.beaming {
            self.line("  beaming 1");
        }
        if m.render == walkmesh {
            self.line(&format!("  render {}", u8::from(m.render)));
        }
        if m.rotate_texture {
            self.line("  rotatetexture 1");
        }
        if m.tilefade != 0 {
            self.line(&format!("  tilefade {}", int(m.tilefade)));
        }
        for (i, t) in m.textures.iter().enumerate() {
            if let Some(t) = t {
                let key = if i == 0 { "bitmap".to_string() } else { format!("texture{i}") };
                self.line(&format!("  {key} {t}"));
            }
        }
        if let Some(mat) = &m.material {
            self.line(&format!("  materialname {mat}"));
        }
        if let Some(h) = &m.renderhint {
            self.line(&format!("  renderhint {}", renderhint(h)));
        }
        if !w.verts.is_empty() {
            self.verts(m, w);
            if self.opts.normals && m.normals.len() >= m.vertices.len() {
                self.line(&format!("  normals {}", w.verts.len()));
                for &v in &w.verts {
                    self.line(&format!("    {}", vec(&m.normals[v])));
                }
            }
            if self.opts.normals && m.tangents.len() >= m.vertices.len() {
                self.line(&format!("  tangents {}", w.verts.len()));
                for &v in &w.verts {
                    let t = m.tangents[v];
                    self.line(&format!("    {} {}", vec(&[t[0], t[1], t[2]]), num(t[3])));
                }
            }
        }
        let has_uvs = uv_sets(m);
        for &set in &has_uvs {
            self.tverts(m, w, set);
        }
        if m.colors.len() >= m.vertices.len()
            && m.colors.iter().any(|c| c[..3] != [255, 255, 255])
            && !w.verts.is_empty()
        {
            self.line(&format!("  colors {}", w.verts.len()));
            for &v in &w.verts {
                let c = m.colors[v];
                let f = |b: u8| num(f32::from(b) / 255.0);
                self.line(&format!("    {} {} {}", f(c[0]), f(c[1]), f(c[2])));
            }
        }
        let uv0 = has_uvs.contains(&0);
        for &set in has_uvs.iter().filter(|&&s| s > 0) {
            self.line(&format!("  texindices{set} {}", m.faces.len()));
            for f in &m.faces {
                let t = f.vertices.map(|v| w.tvert_of.get(v as usize).copied().unwrap_or(0));
                self.line(&format!("    {} {} {}", t[0], t[1], t[2]));
            }
        }
        self.faces(m, w, uv0);
        match &m.extra {
            MeshExtra::Skin(s) => {
                // (Bound to the skeleton it is worn on: its bones are the
                // skeleton's tree positions, named from it where it is
                // given.)
                let skeleton = bound_elsewhere(model, s).then_some(self.skeleton.as_ref());
                let names: Vec<Option<String>> = (s.bones.iter())
                    .map(|&n| match skeleton {
                        Some(on) => on.and_then(|on| on.get(n).cloned()),
                        None => model.nodes.get(n).map(|n| n.name.clone()),
                    })
                    .collect();
                if skeleton == Some(None) {
                    self.line(
                        "  # a body part's skin bound to the skeleton it is worn on, which was \
                         not found: its bones are not named",
                    );
                }
                self.line(&format!("  weights {}", w.verts.len()));
                for &v in &w.verts {
                    let mut line = String::from("   ");
                    if let Some(ws) = s.weights.get(v) {
                        for &(bone, weight) in ws.iter().filter(|(_, w)| *w != 0.0) {
                            let name = (names.get(bone as usize))
                                .and_then(Option::as_deref)
                                .unwrap_or("invalidnodeindex");
                            let _ = write!(line, " {name} {}", num(weight));
                        }
                    }
                    // A vertex weighted only to bones that map to no node
                    // (c_wings2): readers skip blank lines, which would take
                    // the next line as this vertex's weights. nwnmdlcomp
                    // names such bones `invalidnodeindex`.
                    if line.trim().is_empty() {
                        line.push_str(" invalidnodeindex 0");
                    }
                    self.line(&line);
                }
            }
            MeshExtra::Dangly(d) => {
                self.line(&format!("  displacement {}", num(d.displacement)));
                self.line(&format!("  period {}", num(d.period)));
                self.line(&format!("  tightness {}", num(d.tightness)));
                if d.constraints.len() >= m.vertices.len() && !w.verts.is_empty() {
                    self.line(&format!("  constraints {}", w.verts.len()));
                    for &v in &w.verts {
                        self.line(&format!("    {}", num(d.constraints[v])));
                    }
                }
            }
            MeshExtra::Anim(a) => {
                self.line(&format!("  sampleperiod {}", num(a.sample_period)));
            }
            MeshExtra::Aabb(tree) => self.aabb(tree),
            MeshExtra::None => {}
        }
    }

    /// The walkmesh's bounding-box tree in pre-order, children indented.
    fn aabb(&mut self, tree: &[AabbEntry]) {
        if tree.is_empty() {
            return;
        }
        // Depths: an inner entry (face −1) has two children after it.
        let mut depth = Vec::with_capacity(tree.len());
        let mut open: Vec<(usize, u8)> = Vec::new(); // (depth, children still to come)
        for e in tree {
            let d = open.last().map_or(0, |(d, _)| d + 1);
            depth.push(d);
            if let Some(top) = open.last_mut() {
                top.1 -= 1;
            }
            while open.last().is_some_and(|(_, left)| *left == 0) {
                open.pop();
            }
            if e.face < 0 {
                open.push((d, 2));
            }
        }
        for (e, d) in tree.iter().zip(depth) {
            let lead = if d == 0 { "  aabb".to_string() } else { " ".repeat(d + 6) };
            self.line(&format!("{lead} {} {} {}", vec(&e.min), vec(&e.max), e.face));
        }
    }

    fn animation(
        &mut self,
        m: &Model,
        a: &Animation,
        by_name: &HashMap<String, usize>,
        welded: &[Option<Welded>],
    ) {
        self.line("");
        self.line("#MAXANIM ASCII");
        self.line(&format!("newanim {} {}", a.name, m.name));
        self.line(&format!("  length {}", num(a.length)));
        self.line(&format!("  transtime {}", num(a.transtime)));
        if !a.animroot.is_empty() {
            self.line(&format!("  animroot {}", a.animroot));
        }
        for (t, name) in &a.events {
            self.line(&format!("  event {} {name}", num(*t)));
        }
        for n in &a.nodes {
            let geometry = by_name.get(&n.name.to_ascii_lowercase()).copied();
            self.anim_node(m, a, n, geometry, geometry.and_then(|g| welded[g].as_ref()));
        }
        self.line(&format!("doneanim {} {}", a.name, m.name));
    }

    fn anim_node(
        &mut self,
        m: &Model,
        a: &Animation,
        n: &AnimNode,
        geometry: Option<usize>,
        welded: Option<&Welded>,
    ) {
        let ty = geometry.map_or("dummy", |g| m.nodes[g].kind.type_name());
        self.line(&format!("node {ty} {}", n.name));
        let parent = n.parent.and_then(|p| a.nodes.get(p)).map_or("NULL", |p| p.name.as_str());
        self.line(&format!("  parent {parent}"));
        for c in &n.controllers {
            if c.name.starts_with("ctrl") {
                self.line(&format!("  # {} not written: no ASCII keyword", c.name));
                continue;
            }
            // Self-illumination under both spellings, as at rest.
            let keywords: &[&str] = if c.name == "selfillumcolor" {
                &["setfillumcolor", "selfillumcolor"]
            } else {
                &[controller_keyword(&c.name)]
            };
            // Bézier keys: each key's value, then its two handles.
            let bezier = c.is_bezier() && c.handles.len() == c.values.len() * 2;
            let kind = if bezier { "bezierkey" } else { "key" };
            for keyword in keywords {
                self.line(&format!("  {keyword}{kind} {}", c.times.len()));
                for (i, t) in c.times.iter().enumerate() {
                    let mut line = format!("    {}", num(*t));
                    let mut parts = vec![c.row(i)];
                    if bezier {
                        let h = 2 * c.columns;
                        parts.extend(c.handles[i * h..(i + 1) * h].chunks(c.columns));
                    }
                    for part in parts {
                        if c.name == "orientation" && part.len() == 4 {
                            let _ = write!(line, " {}", quat([part[0], part[1], part[2], part[3]]));
                        } else {
                            for v in part {
                                let _ = write!(line, " {}", num(*v));
                            }
                        }
                    }
                    self.line(&line);
                }
            }
        }
        if let (Some(am), Some(w)) = (&n.anim_mesh, welded) {
            self.line(&format!("  sampleperiod {}", num(am.sample_period)));
            // The mesh's vertices and faces again: the game divides the
            // sets' counts by the animation node's own `verts` and
            // `tverts`, and without them it crashes (nwn-mdl-spec E54).
            if let Some(mesh) = geometry.and_then(|g| m.nodes[g].mesh()) {
                self.verts(mesh, w);
                let uv0 = uv_sets(mesh).contains(&0);
                if uv0 {
                    self.tverts(mesh, w, 0);
                }
                self.faces(mesh, w, uv0);
            }
            let vsets: Vec<&Vec<crate::Vec3>> =
                am.vertex_sets.iter().filter(|s| !s.is_empty()).collect();
            if !vsets.is_empty() {
                self.line(&format!("  animverts {}", vsets.len() * w.verts.len()));
                for s in vsets {
                    for &v in &w.verts {
                        let p = s.get(v).copied().unwrap_or_default();
                        self.line(&format!("    {}", vec(&p)));
                    }
                }
            }
            let tsets: Vec<&Vec<crate::Vec2>> =
                am.uv_sets.iter().filter(|s| !s.is_empty()).collect();
            if !tsets.is_empty() {
                self.line(&format!("  animtverts {}", tsets.len() * w.tverts.len()));
                for s in tsets {
                    for &v in &w.tverts {
                        let [u, t] = s.get(v).copied().unwrap_or_default();
                        self.line(&format!("    {} {} 0", num(u), num(t)));
                    }
                }
            }
        }
        self.line("endnode");
    }
}

/// The UV sets a mesh has for every vertex.
fn uv_sets(m: &Mesh) -> Vec<usize> {
    (0..4).filter(|&i| m.uvs[i].len() >= m.vertices.len()).collect()
}

/// A header integer as nwnmdlcomp writes it (signed: EE-compiled models
/// hold −1 in some, which nwnmdlcomp refuses written as 4294967295).
fn int(v: u32) -> i32 {
    v as i32
}

fn vec(v: &[f32; 3]) -> String {
    format!("{} {} {}", num(v[0]), num(v[1]), num(v[2]))
}

fn quat(q: Quat) -> String {
    let a = axis_angle(q);
    format!("{} {} {} {}", num(a[0]), num(a[1]), num(a[2]), num(a[3]))
}

/// A one-word value (emitter modes are names; never write an empty word,
/// which would shift what follows).
fn word(s: &str) -> &str {
    if s.trim().is_empty() { "NULL" } else { s }
}

/// `renderhint` values in the game's spelling.
fn renderhint(h: &str) -> &str {
    match h {
        "none" => "None",
        "normalandspecmapped" => "NormalAndSpecMapped",
        "normaltangents" => "NormalTangents",
        h => h,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Nodes are written by part number, a parent before its children, and
    /// one without a number where the tree has it.
    #[test]
    fn nodes_are_written_in_the_order_of_their_numbers() {
        let text = "newmodel m\nsetsupermodel m NULL\nbeginmodelgeom m\n\
node dummy m\n  parent NULL\nendnode\n\
node dummy a\n  parent m\nendnode\n\
node dummy a1\n  parent a\nendnode\n\
node dummy b\n  parent m\nendnode\n\
node dummy b1\n  parent b\nendnode\n\
endmodelgeom m\ndonemodel m\n";
        let model = Model::read(text.as_bytes()).unwrap();
        let names = |order: &[usize]| -> Vec<&str> {
            order.iter().map(|&i| model.nodes[i].name.as_str()).collect()
        };
        // The tree's order, when that is the numbers'.
        assert_eq!(file_order(&model, &[0, 1, 2, 3, 4]), [0, 1, 2, 3, 4]);
        // The file had b before a, and a1 last.
        assert_eq!(names(&file_order(&model, &[0, 2, 4, 1, 3])), ["m", "b", "a", "b1", "a1"]);
        // A child numbered before its parent still follows it.
        assert_eq!(names(&file_order(&model, &[0, 4, 1, 2, 3])), ["m", "b", "b1", "a", "a1"]);
        // No number: after the node before it in the tree.
        assert_eq!(names(&file_order(&model, &[0, 3, -1, 1, 2])), ["m", "b", "b1", "a", "a1"]);
        assert_eq!(file_order(&model, &[]).len(), 5);
        // The text follows the order, and reads back as the same nodes.
        let order = file_order(&model, &[0, 2, 4, 1, 3]);
        let out = to_ascii_in_order(&model, &Options::default(), &order);
        let written: Vec<&str> =
            out.lines().filter_map(|l| l.strip_prefix("node dummy ")).collect();
        assert_eq!(written, ["m", "b", "a", "b1", "a1"]);
        assert_eq!(Model::read(out.as_bytes()).unwrap().nodes.len(), 5);
    }

    /// An animated mesh with a vertex split by its UVs (nwn-mdl-spec's
    /// zspec_animtv): decompiled, each of the compiled vertices has its
    /// frames, and the animation node gives the `verts`, `tverts` and
    /// `faces` the game divides them by (E54); compiled again, the same.
    #[test]
    fn an_animated_mesh_keeps_its_frames_per_vertex() {
        let text = "newmodel w\nsetsupermodel w NULL\nclassification tile\nbeginmodelgeom w\n\
            node dummy w\n  parent NULL\nendnode\n\
            node animmesh water\n  parent w\n  verts 4\n    -1 -1 0\n    1 -1 0\n    1 1 0\n    -1 1 0\n\
            tverts 5\n    0 0 0\n    1 0 0\n    1 1 0\n    0 1 0\n    0.5 0.5 0\n\
            faces 2\n    0 1 2 1 0 1 4 0\n    0 2 3 1 0 2 3 0\nendnode\nendmodelgeom w\n\
            newanim loop w\n  length 1\n  animroot w\n  node dummy w\n    parent NULL\n  endnode\n\
            node animmesh water\n  parent w\n  sampleperiod 0.5\n\
            verts 4\n    -1 -1 0\n    1 -1 0\n    1 1 0\n    -1 1 0\n\
            tverts 5\n    0 0 0\n    1 0 0\n    1 1 0\n    0 1 0\n    0.5 0.5 0\n\
            faces 2\n    0 1 2 1 0 1 4 0\n    0 2 3 1 0 2 3 0\n\
            animverts 8\n    -1 -1 0\n    1 -1 0.1\n    1 1 0.2\n    -1 1 0.3\n\
            -1 -1 1\n    1 -1 1.1\n    1 1 1.2\n    -1 1 1.3\n  endnode\ndoneanim loop w\ndonemodel w\n";
        let compiled = crate::compile::compile(text.as_bytes(), &Default::default()).unwrap();
        let out = crate::decompile(&compiled.binary).unwrap();
        let anim = &out[out.find("newanim").unwrap()..];
        // (Five vertices: the one split by its UVs twice.)
        assert!(anim.contains("  verts 5\n") && anim.contains("  faces 2\n"), "{anim}");
        assert!(anim.contains("  animverts 10\n"), "{anim}");
        let frames = |data: &[u8]| {
            let m = Model::read(data).unwrap();
            let am = m.animations[0].nodes[1].anim_mesh.clone().unwrap();
            let mesh = m.nodes[1].mesh().unwrap().clone();
            // Each corner's place in each frame.
            (am.vertex_sets.iter())
                .map(|s| {
                    let at = |v: u32| s.get(v as usize).copied().unwrap_or_default();
                    mesh.faces.iter().flat_map(|f| f.vertices.map(at)).collect::<Vec<_>>()
                })
                .collect::<Vec<_>>()
        };
        let again = crate::compile::compile(out.as_bytes(), &Default::default()).unwrap();
        assert_eq!(frames(&again.binary), frames(&compiled.binary));
    }

    /// A body part's skin bound to the skeleton it is worn on (as Beamdog's
    /// HD `pmh0_neck001`): its bones are the skeleton's tree positions,
    /// named from the skeleton, not from the part's own nodes; without the
    /// skeleton they are left unnamed and the text says so (nwn-mdl-spec
    /// MDL-SKIN-040).
    #[test]
    fn a_bound_body_part_s_bones_are_named_from_its_skeleton() {
        let skeleton = "newmodel pmh0\nbeginmodelgeom pmh0\nnode dummy pmh0\n  parent NULL\nendnode\n\
            node dummy pelvis_g\n  parent pmh0\nendnode\nnode dummy torso_g\n  parent pelvis_g\n\
            endnode\nendmodelgeom pmh0\ndonemodel pmh0\n";
        let skeleton = Model::read(skeleton.as_bytes()).unwrap();
        let part = "newmodel pmh0_neck001\nbeginmodelgeom pmh0_neck001\n\
            node dummy pmh0_neck001\n  parent NULL\nendnode\n\
            node skin neck\n  parent pmh0_neck001\n  verts 3\n    0 0 0\n    1 0 0\n    0 1 0\n\
            faces 1\n    0 1 2 1 0 0 0 0\n  weights 3\n    neck 1\n    neck 1\n    neck 1\n\
            endnode\nendmodelgeom pmh0_neck001\ndonemodel pmh0_neck001\n";
        let mut part = Model::read(part.as_bytes()).unwrap();
        // (As compiled against the skeleton: its bone is tree position 2.)
        let NodeKind::Mesh(m) = &mut part.nodes[1].kind else { panic!("a mesh") };
        let MeshExtra::Skin(s) = &mut m.extra else { panic!("a skin") };
        s.bones = vec![2];
        let s = s.clone();
        assert!(bound_elsewhere(&part, &s));
        let order = [0, 1];
        let opts = Options::default();
        let named = to_ascii_on(&part, &opts, &order, Some(&skeleton));
        assert!(named.contains("    torso_g 1\n"), "{named}");
        let unnamed = to_ascii_on(&part, &opts, &order, None);
        assert!(unnamed.contains("    invalidnodeindex 1\n"), "{unnamed}");
        assert!(unnamed.contains("not found: its bones are not named"));
        assert_eq!(skeleton_of("pmh0_neck001"), Some("pmh0"));
        assert_eq!(skeleton_of("PFE2_robe003"), Some("PFE2"));
        assert_eq!(skeleton_of("plc_a01"), None);
        assert_eq!(skeleton_of("pmh0"), None);
    }

    #[test]
    fn numbers_read_back_exactly() {
        for v in [0.0f32, 1.0, -1.5, 0.1, 1.0 / 3.0, 1e-5, 3.5e-30, 123456.79, 2e9, -0.0] {
            let s = num(v);
            let back: f32 = s.parse().unwrap();
            assert!(back == v || (v == 0.0 && back == 0.0), "{v} -> {s} -> {back}");
        }
        assert_eq!(num(0.1), "0.1");
        assert_eq!(num(-0.0), "0");
        assert_eq!(num(f32::NAN), "0");
    }

    #[test]
    fn axis_angles() {
        assert_eq!(axis_angle(crate::IDENTITY), [0.0; 4]);
        let q = crate::axis_angle([0.0, 0.0, 1.0], 1.0);
        let a = axis_angle(q);
        assert!((a[2] - 1.0).abs() < 1e-6 && (a[3] - 1.0).abs() < 1e-6, "{a:?}");
        // q and −q are the same rotation.
        let neg = axis_angle(q.map(|x| -x));
        let back = crate::axis_angle([neg[0], neg[1], neg[2]], neg[3]);
        let dot: f32 = (0..4).map(|i| back[i] * q[i]).sum();
        assert!(dot.abs() > 0.99999, "{back:?} vs {q:?}");
    }

    #[test]
    fn aabb_depths() {
        let e = |face| AabbEntry { min: [0.0; 3], max: [1.0; 3], face };
        let tree = [e(-1), e(-1), e(0), e(1), e(2)];
        let mut w = Writer { out: String::new(), opts: Options::default(), skeleton: None };
        w.aabb(&tree);
        let indents: Vec<usize> = w.out.lines().map(|l| l.len() - l.trim_start().len()).collect();
        assert!(w.out.starts_with("  aabb 0 0 0 1 1 1 -1"));
        // nwnmdlcomp: depth + 6 spaces, then a space before the numbers.
        assert_eq!(indents, vec![2, 8, 9, 9, 8]);
    }
}
