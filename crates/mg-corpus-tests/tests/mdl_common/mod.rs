//! Comparing two models as the game would see them: node by node, face
//! corner by face corner (the ASCII reader numbers vertices its own way).

#![allow(dead_code, unreachable_pub)]

use mg_mdl::{Controller, Mesh, MeshExtra, Model, NodeKind, Quat};

/// Pairs the n-th item of each name in `a` with the n-th of that name in
/// `b` (case-insensitive); unmatched items are left out (the counts are
/// compared separately).
pub fn match_by_name<'x>(
    a: impl Iterator<Item = &'x str>,
    b: impl Iterator<Item = &'x str>,
) -> Vec<(usize, usize)> {
    let mut slots: std::collections::HashMap<String, std::collections::VecDeque<usize>> =
        std::collections::HashMap::new();
    for (i, n) in b.enumerate() {
        slots.entry(n.to_ascii_lowercase()).or_default().push_back(i);
    }
    a.enumerate()
        .filter_map(|(i, n)| Some((i, slots.get_mut(&n.to_ascii_lowercase())?.pop_front()?)))
        .collect()
}

/// Whether two geometry nodes share a name (case-insensitive).
pub fn duplicate_names(m: &Model) -> bool {
    let mut seen = std::collections::HashSet::new();
    m.nodes.iter().any(|n| !seen.insert(n.name.to_ascii_lowercase()))
}

pub fn close(a: f32, b: f32, eps: f32) -> bool {
    (a - b).abs() <= eps * (1.0 + a.abs().max(b.abs()))
}

fn close_all(a: &[f32], b: &[f32], eps: f32) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(x, y)| close(*x, *y, eps))
}

/// Same rotation (q and −q are), within `eps`.
pub fn same_rotation(a: Quat, b: Quat, eps: f32) -> bool {
    let norm = |q: Quat| {
        let l = q.iter().map(|x| x * x).sum::<f32>().sqrt();
        if l < 1e-12 { mg_mdl::IDENTITY } else { q.map(|x| x / l) }
    };
    let (a, b) = (norm(a), norm(b));
    let dot: f32 = (0..4).map(|i| a[i] * b[i]).sum();
    dot.abs() >= 1.0 - eps
}

/// Header integers written signed (−1 in EE-compiled models): `mg-mdl`'s
/// ASCII reader reads −1 as 0 (a saturating conversion; see the plan's
/// proposed toolset changes).
/// A header integer: as stored, or 0 for a negative one (nwnmdlcomp takes
/// only 0 and 1 for `spawntype`, so it gets 0 for −1).
fn same_int(bin: u32, asc: u32) -> bool {
    bin == asc || (bin as i32) < 0 && asc == 0
}

fn same_word(a: &str, b: &str) -> bool {
    let n = |s: &str| if s.trim().is_empty() { "null".to_string() } else { s.to_ascii_lowercase() };
    n(a) == n(b)
}

/// The differences between a model and its ASCII reading, as messages.
pub fn differences(bin: &Model, asc: &Model) -> Vec<String> {
    compare(bin, asc, true)
}

/// [`differences`], optionally without normals (for tools that recompute
/// them).
pub fn compare(bin: &Model, asc: &Model, normals: bool) -> Vec<String> {
    let mut d = Vec::new();
    macro_rules! check {
        ($cond:expr, $($fmt:tt)*) => {
            if !$cond {
                d.push(format!($($fmt)*));
            }
        };
    }
    check!(bin.name.eq_ignore_ascii_case(&asc.name), "name {} vs {}", bin.name, asc.name);
    check!(
        bin.supermodel.as_deref().map(str::to_ascii_lowercase)
            == asc.supermodel.as_deref().map(str::to_ascii_lowercase),
        "supermodel {:?} vs {:?}",
        bin.supermodel,
        asc.supermodel
    );
    check!(bin.classification == asc.classification, "classification");
    check!(bin.ignore_fog == asc.ignore_fog, "ignorefog");
    check!(close(bin.animation_scale, asc.animation_scale, 1e-6), "animation scale");
    check!(bin.nodes.len() == asc.nodes.len(), "{} nodes vs {}", bin.nodes.len(), asc.nodes.len());
    if !d.is_empty() {
        return d;
    }
    // Nodes by name (the n-th node of a name with the n-th of that name):
    // tools order siblings differently (nwnmdlcomp by part number).
    let pairs = match_by_name(
        bin.nodes.iter().map(|n| n.name.as_str()),
        asc.nodes.iter().map(|n| n.name.as_str()),
    );
    for (bi, ai) in pairs {
        let (b, a) = (&bin.nodes[bi], &asc.nodes[ai]);
        let at = &b.name;
        check!(b.name.eq_ignore_ascii_case(&a.name), "node {} vs {}", b.name, a.name);
        check!(b.kind.type_name() == a.kind.type_name(), "{at}: type");
        let pname = |m: &Model, p: Option<usize>| p.map(|p| m.nodes[p].name.to_ascii_lowercase());
        check!(pname(bin, b.parent) == pname(asc, a.parent), "{at}: parent");
        check!(close_all(&b.position, &a.position, 1e-6), "{at}: position");
        check!(same_rotation(b.orientation, a.orientation, 1e-5), "{at}: orientation");
        check!(close(b.scale, a.scale, 1e-6), "{at}: scale");
        check!(b.inherit_color == a.inherit_color, "{at}: inheritcolor");
        // `detonate` at rest means nothing (the writer leaves it out).
        let rest =
            |c: &&Controller| !c.name.starts_with("ctrl") && c.columns > 0 && c.name != "detonate";
        for c in b.controllers.iter().filter(rest) {
            let other = a.controllers.iter().find(|x| x.name == c.name);
            check!(
                other.is_some_and(|o| close_all(c.row(0), o.row(0), 1e-6)),
                "{at}: rest {} {:?} vs {:?}",
                c.name,
                c.row(0),
                other.map(|o| o.row(0))
            );
        }
        match (&b.kind, &a.kind) {
            (NodeKind::Light(x), NodeKind::Light(y)) => {
                check!(x == y, "{at}: light {x:?} vs {y:?}")
            }
            (NodeKind::Emitter(x), NodeKind::Emitter(y)) => {
                let same = same_word(&x.update, &y.update)
                    && same_word(&x.render, &y.render)
                    && same_word(&x.blend, &y.blend)
                    && x.texture == y.texture
                    && x.chunk == y.chunk
                    && x.flags == y.flags
                    && x.xgrid == y.xgrid
                    && x.ygrid == y.ygrid
                    && same_int(x.spawntype, y.spawntype)
                    && x.two_sided == y.two_sided
                    && x.looping == y.looping
                    && x.render_order == y.render_order
                    && close(x.deadspace, y.deadspace, 1e-6)
                    && close(x.blast_radius, y.blast_radius, 1e-6)
                    && close(x.blast_length, y.blast_length, 1e-6);
                check!(same, "{at}: emitter {x:?} vs {y:?}");
            }
            (NodeKind::Reference(x), NodeKind::Reference(y)) => check!(x == y, "{at}: reference"),
            (NodeKind::Mesh(x), NodeKind::Mesh(y)) => mesh(bin, asc, at, x, y, normals, &mut d),
            _ => {}
        }
    }
    check!(
        bin.animations.len() == asc.animations.len(),
        "{} animations vs {}",
        bin.animations.len(),
        asc.animations.len()
    );
    for (ba, aa) in bin.animations.iter().zip(&asc.animations) {
        let at = &ba.name;
        check!(ba.name == aa.name, "animation {} vs {}", ba.name, aa.name);
        check!(close(ba.length, aa.length, 1e-6), "{at}: length");
        check!(close(ba.transtime, aa.transtime, 1e-6), "{at}: transtime");
        check!(ba.animroot.eq_ignore_ascii_case(&aa.animroot), "{at}: animroot");
        check!(
            ba.events.len() == aa.events.len()
                && ba
                    .events
                    .iter()
                    .zip(&aa.events)
                    .all(|(x, y)| close(x.0, y.0, 1e-6) && x.1 == y.1),
            "{at}: events"
        );
        check!(
            ba.nodes.len() == aa.nodes.len(),
            "{at}: {} nodes vs {}",
            ba.nodes.len(),
            aa.nodes.len()
        );
        let pairs = match_by_name(
            ba.nodes.iter().map(|n| n.name.as_str()),
            aa.nodes.iter().map(|n| n.name.as_str()),
        );
        for (bn, an) in pairs.into_iter().map(|(x, y)| (&ba.nodes[x], &aa.nodes[y])) {
            let at = format!("{at}/{}", bn.name);
            check!(bn.name.eq_ignore_ascii_case(&an.name), "{at}: name {}", an.name);
            for c in bn.controllers.iter().filter(|c| !c.name.starts_with("ctrl")) {
                let o = an.controllers.iter().find(|x| x.name == c.name);
                check!(o.is_some_and(|o| same_keys(c, o)), "{at}: keys {}", c.name);
            }
            // Animated vertices, corner by corner through the geometry.
            if let (Some(bm), Some(am)) = (&bn.anim_mesh, &an.anim_mesh) {
                let (Some(gb), Some(ga)) = (
                    bin.node(&bn.name).and_then(|i| bin.nodes[i].mesh()),
                    asc.node(&an.name).and_then(|i| asc.nodes[i].mesh()),
                ) else {
                    continue;
                };
                check!(bm.vertex_sets.len() == am.vertex_sets.len(), "{at}: vertex sets");
                check!(bm.uv_sets.len() == am.uv_sets.len(), "{at}: uv sets");
                for (fb, fa) in gb.faces.iter().zip(&ga.faces) {
                    for c in 0..3 {
                        let (rb, ra) = (fb.vertices[c] as usize, fa.vertices[c] as usize);
                        let (sv, st) = (ga.source[ra] as usize, ga.source_uv[ra] as usize);
                        for (x, y) in bm.vertex_sets.iter().zip(&am.vertex_sets) {
                            if let (Some(p), Some(q)) = (x.get(rb), y.get(sv)) {
                                check!(close_all(p, q, 1e-6), "{at}: animvert");
                            }
                        }
                        for (x, y) in bm.uv_sets.iter().zip(&am.uv_sets) {
                            if let (Some(p), Some(q)) = (x.get(rb), y.get(st)) {
                                check!(close_all(p, q, 1e-6), "{at}: animtvert");
                            }
                        }
                    }
                }
            }
        }
    }
    d.sort();
    d.dedup();
    d
}

fn same_keys(a: &Controller, b: &Controller) -> bool {
    if a.times.len() != b.times.len() || !close_all(&a.times, &b.times, 1e-6) {
        return false;
    }
    (0..a.times.len()).all(|i| {
        if a.name == "orientation" && a.columns == 4 && b.columns == 4 {
            let (x, y) = (a.row(i), b.row(i));
            same_rotation([x[0], x[1], x[2], x[3]], [y[0], y[1], y[2], y[3]], 1e-5)
        } else {
            close_all(a.row(i), b.row(i), 1e-6)
        }
    })
}

fn mesh(
    bin: &Model,
    asc: &Model,
    at: &str,
    b: &Mesh,
    a: &Mesh,
    normals: bool,
    d: &mut Vec<String>,
) {
    macro_rules! check {
        ($cond:expr, $($fmt:tt)*) => {
            if !$cond {
                d.push(format!($($fmt)*));
            }
        };
    }
    check!(close_all(&b.diffuse, &a.diffuse, 1e-6), "{at}: diffuse");
    check!(close_all(&b.ambient, &a.ambient, 1e-6), "{at}: ambient");
    check!(close_all(&b.specular, &a.specular, 1e-6), "{at}: specular");
    check!(close(b.shininess, a.shininess, 1e-6), "{at}: shininess");
    check!(b.render == a.render, "{at}: render");
    check!(b.shadow == a.shadow, "{at}: shadow");
    check!(b.beaming == a.beaming, "{at}: beaming");
    check!(b.rotate_texture == a.rotate_texture, "{at}: rotatetexture");
    check!(b.tilefade == a.tilefade, "{at}: tilefade");
    check!(b.transparency_hint == a.transparency_hint, "{at}: transparencyhint");
    check!(b.textures == a.textures, "{at}: textures {:?} vs {:?}", b.textures, a.textures);
    check!(b.material == a.material && b.renderhint == a.renderhint, "{at}: material");
    // Faces whose corners the binary model has (others are dropped by
    // readers, and the ASCII reader drops them too).
    let n = b.vertices.len();
    let faces: Vec<_> =
        b.faces.iter().filter(|f| f.vertices.iter().all(|&v| (v as usize) < n)).collect();
    check!(faces.len() == a.faces.len(), "{at}: {} faces vs {}", faces.len(), a.faces.len());
    if faces.len() != a.faces.len() {
        return;
    }
    let weights = |m: &Model, mesh: &Mesh, v: usize| -> Vec<(String, u32)> {
        let MeshExtra::Skin(s) = &mesh.extra else { return Vec::new() };
        let mut w: Vec<(String, u32)> = s
            .weights
            .get(v)
            .map(|ws| {
                ws.iter()
                    .filter(|(_, w)| *w != 0.0)
                    .map(|(bone, w)| {
                        let name = s
                            .bones
                            .get(*bone as usize)
                            .and_then(|&n| m.nodes.get(n))
                            .map_or(String::new(), |n| n.name.to_ascii_lowercase());
                        (name, w.to_bits())
                    })
                    .collect()
            })
            .unwrap_or_default();
        w.sort();
        w
    };
    for (fb, fa) in faces.iter().zip(&a.faces) {
        check!(fb.material == fa.material, "{at}: face material");
        for c in 0..3 {
            let (rb, ra) = (fb.vertices[c] as usize, fa.vertices[c] as usize);
            check!(close_all(&b.vertices[rb], &a.vertices[ra], 1e-6), "{at}: vertex");
            if normals && b.normals.len() >= n {
                check!(close_all(&b.normals[rb], &a.normals[ra], 1e-6), "{at}: normal");
            }
            for set in 0..4 {
                if b.uvs[set].len() >= n {
                    check!(
                        a.uvs[set].get(ra).is_some_and(|uv| close_all(&b.uvs[set][rb], uv, 1e-6)),
                        "{at}: uv{set}"
                    );
                }
            }
            if b.colors.len() >= n && b.colors.iter().any(|c| c[..3] != [255, 255, 255]) {
                check!(
                    a.colors.get(ra).is_some_and(|x| x[..3] == b.colors[rb][..3]),
                    "{at}: colour"
                );
            }
            check!(weights(bin, b, rb) == weights(asc, a, ra), "{at}: weights");
            if let (MeshExtra::Dangly(x), MeshExtra::Dangly(y)) = (&b.extra, &a.extra)
                && x.constraints.len() >= n
            {
                check!(
                    y.constraints.get(ra).is_some_and(|v| close(x.constraints[rb], *v, 1e-6)),
                    "{at}: constraint"
                );
            }
        }
    }
    match (&b.extra, &a.extra) {
        (MeshExtra::Dangly(x), MeshExtra::Dangly(y)) => {
            check!(
                close(x.displacement, y.displacement, 1e-6)
                    && close(x.tightness, y.tightness, 1e-6)
                    && close(x.period, y.period, 1e-6),
                "{at}: dangly"
            );
        }
        (MeshExtra::Anim(x), MeshExtra::Anim(y)) => {
            check!(close(x.sample_period, y.sample_period, 1e-6), "{at}: sampleperiod");
        }
        (MeshExtra::Aabb(x), MeshExtra::Aabb(y)) => check!(x == y, "{at}: aabb tree"),
        _ => {}
    }
}
