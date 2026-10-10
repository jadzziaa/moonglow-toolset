//! Diagnostics for ASCII models, by line.
//!
//! The game's ASCII reader (and `mg-mdl`'s, which follows it) is lenient: it
//! skips keywords it does not know and reads lists as far as they go, so a
//! typo gives a model that loads but looks wrong. This pass reports what the
//! reader would silently skip or misread, what crashes the game, and what
//! nwnmdlcomp would drop. Limits are the engine's (nwn.wiki: MDL ASCII,
//! Models; patch notes).

use std::collections::{HashMap, HashSet};

use crate::keywords;
use crate::outline::words;

/// The most bones nwnmdlcomp compiles into one skin.
pub const NWNMDLCOMP_BONES: usize = 17;

/// How bad a diagnostic is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Severity {
    /// Worth knowing (a keyword nwnmdlcomp drops).
    Info,
    /// Probably not what was meant (an unknown keyword, a count that does
    /// not match).
    Warning,
    /// The model is wrong or crashes the game.
    Error,
}

/// One diagnostic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    /// 0-based line.
    pub line: usize,
    pub severity: Severity,
    pub message: String,
}

/// Engine limits.
pub const MAX_BONES: usize = 64;
pub const MAX_FACES: usize = 21_845;
pub const MAX_RESREF: usize = 16;
pub const MAX_NODE_NAME: usize = 32;
pub const MAX_LIGHTNING_SEGMENTS: f32 = 9.0;

#[derive(Default)]
struct Node {
    ty: String,
    name: String,
    line: usize,
    verts: Option<usize>,
    tverts: Option<usize>,
    /// Faces' vertex and texture-vertex indices, with their lines.
    faces: Vec<(usize, [i64; 3], [i64; 3])>,
    constraints: Option<(usize, usize)>,
    weights: Vec<(usize, Vec<String>)>,
    p2p: bool,
    update: Option<String>,
    birthrate: Option<(usize, f32)>,
    period: Option<(usize, f32)>,
}

struct Lint {
    out: Vec<Diagnostic>,
}

impl Lint {
    fn push(&mut self, line: usize, severity: Severity, message: impl Into<String>) {
        self.out.push(Diagnostic { line, severity, message: message.into() });
    }
}

fn is_number(w: &str) -> bool {
    w.parse::<f32>().is_ok()
}

/// Checks a model's text.
pub fn lint(text: &str) -> Vec<Diagnostic> {
    let mut l = Lint { out: Vec::new() };
    let lines: Vec<&str> = text.lines().collect();
    let mut node: Option<Node> = None;
    let mut in_anim = false;
    let mut geometry_names: HashMap<String, usize> = HashMap::new();
    let mut nodes: Vec<Node> = Vec::new();
    let mut anim_names: HashSet<String> = HashSet::new();
    let mut duplicates: Vec<(usize, usize, String)> = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let w = words(lines[i]);
        let at = i;
        i += 1;
        let Some(first) = w.first() else { continue };
        let key = first.to_ascii_lowercase();
        match key.as_str() {
            "node" => {
                if let Some(n) = node.take() {
                    l.push(n.line, Severity::Warning, format!("node {} has no endnode", n.name));
                    nodes.push(n);
                }
                let ty = w.get(1).map(|s| s.to_ascii_lowercase()).unwrap_or_default();
                let name = w.get(2).map(|s| s.to_string()).unwrap_or_default();
                if name.is_empty() {
                    l.push(at, Severity::Error, "a node needs a type and a name");
                }
                if !keywords::NODE_TYPES.contains(&ty.as_str()) {
                    l.push(
                        at,
                        Severity::Warning,
                        format!("unknown node type {ty}: read as a dummy"),
                    );
                }
                if name.len() > MAX_NODE_NAME {
                    l.push(
                        at,
                        Severity::Warning,
                        format!("node names are at most {MAX_NODE_NAME} characters"),
                    );
                }
                if !in_anim {
                    match geometry_names.get(&name.to_ascii_lowercase()) {
                        Some(&prev) => duplicates.push((at, prev, name.clone())),
                        None => {
                            geometry_names.insert(name.to_ascii_lowercase(), at);
                        }
                    }
                }
                node = Some(Node { ty, name, line: at, ..Default::default() });
            }
            "endnode" => match node.take() {
                Some(n) => {
                    if !in_anim {
                        check_node(&mut l, &n);
                    }
                    nodes.push(n);
                }
                None => l.push(at, Severity::Warning, "endnode without a node"),
            },
            "newanim" => {
                in_anim = true;
                if let Some(name) = w.get(1)
                    && !anim_names.insert(name.to_ascii_lowercase())
                {
                    l.push(at, Severity::Warning, format!("a second animation named {name}"));
                }
            }
            "doneanim" => in_anim = false,
            "newmodel" => {
                // The game finds models by file name; the name inside may be
                // longer (the game's own vwp_flash_pnkor).
                if let Some(n) = w.get(1)
                    && n.len() > MAX_RESREF
                {
                    l.push(
                        at,
                        Severity::Info,
                        format!("{n} is longer than a resource name ({MAX_RESREF} characters)"),
                    );
                }
            }
            "setsupermodel" => {
                if let Some(n) = w.get(2)
                    && n.len() > MAX_RESREF
                {
                    l.push(
                        at,
                        Severity::Error,
                        format!("{n}: resource names are at most {MAX_RESREF} characters"),
                    );
                }
            }
            _ if node.is_none() => {
                if !keywords::TOP_LEVEL.contains(&key.as_str()) {
                    l.push(
                        at,
                        Severity::Warning,
                        format!("unknown keyword {first}: the game skips it"),
                    );
                }
            }
            _ => {
                let n = node.as_mut().expect("in a node");
                i = node_line(&mut l, n, &lines, at, &w, &key, in_anim);
            }
        }
    }
    if let Some(n) = node.take() {
        l.push(n.line, Severity::Warning, format!("node {} has no endnode", n.name));
        nodes.push(n);
    }

    // Parents: geometry parents must be earlier nodes.
    let mut seen: HashSet<String> = HashSet::new();
    let mut in_anim = false;
    for (at, line) in lines.iter().enumerate() {
        let w = words(line);
        match w.first().map(|s| s.to_ascii_lowercase()).as_deref() {
            Some("newanim") => in_anim = true,
            Some("doneanim") => in_anim = false,
            Some("node") if !in_anim => {
                if let Some(n) = w.get(2) {
                    seen.insert(n.to_ascii_lowercase());
                }
            }
            Some("parent") if !in_anim => {
                if let Some(p) = w.get(1)
                    && !p.eq_ignore_ascii_case("null")
                    && !seen.contains(&p.to_ascii_lowercase())
                {
                    l.push(
                        at,
                        Severity::Warning,
                        format!("parent {p} is not a node above: the node hangs from the root"),
                    );
                }
            }
            _ => {}
        }
    }

    // Duplicate names: harmless unless something hangs from the second.
    for (at, prev, name) in duplicates {
        let lower = name.to_ascii_lowercase();
        let parented = lines.iter().skip(at + 1).any(|line| {
            let w = words(line);
            w.first().is_some_and(|k| k.eq_ignore_ascii_case("parent"))
                && w.get(1).is_some_and(|p| p.eq_ignore_ascii_case(&lower))
        });
        let (severity, consequence) = if parented {
            (Severity::Warning, "its children hang from the first")
        } else {
            (Severity::Info, "animations find the first")
        };
        l.push(
            at,
            severity,
            format!("a second node named {name} (line {}): {consequence}", prev + 1),
        );
    }

    // Skins: bones by name.
    for n in nodes.iter().filter(|n| !n.weights.is_empty()) {
        let mut bones: HashSet<String> = HashSet::new();
        for (at, names) in &n.weights {
            for b in names {
                let lower = b.to_ascii_lowercase();
                if !geometry_names.contains_key(&lower) && lower != "invalidnodeindex" {
                    l.push(
                        *at,
                        Severity::Warning,
                        format!("bone {b} is not a node: its weight is ignored"),
                    );
                }
                bones.insert(lower);
            }
        }
        if bones.len() > MAX_BONES {
            l.push(
                n.line,
                Severity::Error,
                format!("{} bones; a skin has at most {MAX_BONES}", bones.len()),
            );
        } else if bones.len() > NWNMDLCOMP_BONES {
            l.push(
                n.line,
                Severity::Info,
                format!(
                    "{} bones: nwnmdlcomp compiles at most {}; the game's compiler takes {MAX_BONES}",
                    bones.len(),
                    NWNMDLCOMP_BONES
                ),
            );
        }
    }
    l.out.sort_by_key(|d| (d.line, std::cmp::Reverse(d.severity)));
    l.out.dedup();
    l.out
}

/// One line inside a node; returns the next line to read (after a list).
fn node_line(
    l: &mut Lint,
    n: &mut Node,
    lines: &[&str],
    at: usize,
    w: &[&str],
    key: &str,
    in_anim: bool,
) -> usize {
    let mut next = at + 1;
    let value = || w.get(1).and_then(|v| v.parse::<f32>().ok());
    // Lists: a count, then that many rows.
    if keywords::LISTS.contains(&key) || (key.ends_with("key") && key.len() > 3) {
        let keyed = !keywords::LISTS.contains(&key);
        let declared = w.get(1).and_then(|c| c.parse::<usize>().ok());
        let named = keywords::NAMED_LISTS.contains(&key);
        let mut rows: Vec<(usize, Vec<&str>)> = Vec::new();
        while next < lines.len() {
            let rw = words(lines[next]);
            let Some(first) = rw.first() else {
                next += 1;
                continue;
            };
            let full = declared.is_some_and(|c| rows.len() >= c);
            if first.eq_ignore_ascii_case("endlist")
                || first.eq_ignore_ascii_case("endnode")
                || (!named && !is_number(first))
                || full
            {
                break;
            }
            rows.push((next, rw));
            next += 1;
        }
        // An `endlist` after the rows belongs to the list.
        while next < lines.len() && words(lines[next]).is_empty() {
            next += 1;
        }
        if next < lines.len()
            && words(lines[next]).first().is_some_and(|w| w.eq_ignore_ascii_case("endlist"))
        {
            next += 1;
        }
        if let Some(c) = declared
            && c != rows.len()
        {
            l.push(at, Severity::Warning, format!("{key} says {c}, {} follow", rows.len()));
        }
        if !keyed && declared.is_none() && key != "aabb" {
            l.push(at, Severity::Warning, format!("{key} needs a count"));
        }
        if in_anim {
            return next;
        }
        match key {
            "verts" => n.verts = Some(rows.len()),
            "tverts" | "tverts0" => n.tverts = Some(rows.len()),
            "faces" => {
                if rows.len() > MAX_FACES {
                    l.push(
                        at,
                        Severity::Error,
                        format!("{} faces; a mesh has at most {MAX_FACES}", rows.len()),
                    );
                }
                for (line, r) in &rows {
                    let int =
                        |k: usize| r.get(k).and_then(|v| v.parse::<f32>().ok()).map(|v| v as i64);
                    if r.len() < 8 {
                        l.push(
                            *line,
                            Severity::Warning,
                            "a face is: 3 vertices, smoothing group, 3 texture vertices, material",
                        );
                    }
                    let v = [int(0).unwrap_or(-1), int(1).unwrap_or(-1), int(2).unwrap_or(-1)];
                    let t = [int(4).unwrap_or(0), int(5).unwrap_or(0), int(6).unwrap_or(0)];
                    n.faces.push((*line, v, t));
                }
            }
            "constraints" => n.constraints = Some((at, rows.len())),
            "weights" => {
                for (line, r) in rows {
                    let names: Vec<String> =
                        r.chunks(2).filter_map(|p| p.first().map(|s| s.to_string())).collect();
                    if r.len() % 2 != 0 {
                        l.push(line, Severity::Warning, "weights are pairs: bone name, weight");
                    }
                    if r.len() > 8 {
                        l.push(line, Severity::Warning, "at most 4 bones weight a vertex");
                    }
                    n.weights.push((line, names));
                }
            }
            _ => {}
        }
        return next;
    }
    if in_anim {
        if !keywords::ANIM_NODE.contains(&key) {
            unknown(l, at, &n.ty, w[0], key);
        }
        return next;
    }
    match key {
        "aabb" => {
            // The tree: rows of 7 numbers, the first on this line.
            while next < lines.len() && words(lines[next]).first().is_some_and(|w| is_number(w)) {
                next += 1;
            }
            return next;
        }
        "renderhint" => l.push(
            at,
            Severity::Info,
            format!("{key} is EE: nwnmdlcomp drops it (the game's compiler keeps it)"),
        ),
        "bitmap" | "texture0" | "texture1" | "texture2" | "texture3" | "materialname"
        | "refmodel" | "chunkname" | "texture" => {
            if key == "materialname" {
                l.push(
                    at,
                    Severity::Info,
                    "materialname is EE: nwnmdlcomp drops it (the game's compiler keeps it)",
                );
            }
            if let Some(name) = w.get(1)
                && name.len() > MAX_RESREF
            {
                l.push(
                    at,
                    Severity::Error,
                    format!("{name}: resource names are at most {MAX_RESREF} characters"),
                );
            }
        }
        "p2p" => n.p2p = value().is_some_and(|v| v != 0.0),
        "update" => n.update = w.get(1).map(|s| s.to_ascii_lowercase()),
        "birthrate" => n.birthrate = value().map(|v| (at, v)),
        "period" => n.period = value().map(|v| (at, v)),
        "parent" => {}
        _ => {}
    }
    unknown(l, at, &n.ty, w[0], key);
    next
}

/// Reports a keyword the node's type does not use: a warning if no node
/// uses it (a typo, most likely), a note if another type does (the game's
/// own models carry dangly settings on skins, for one).
fn unknown(l: &mut Lint, at: usize, ty: &str, word: &str, key: &str) {
    if keywords::known_in_node(ty, key) {
        return;
    }
    if keywords::known_anywhere(key) {
        l.push(at, Severity::Info, format!("{word} does nothing on a {ty} node"));
    } else {
        l.push(at, Severity::Warning, format!("unknown keyword {word}: the game skips it"));
    }
}

/// Checks a finished geometry node.
fn check_node(l: &mut Lint, n: &Node) {
    let verts = n.verts.unwrap_or(0);
    for (line, v, t) in &n.faces {
        if v.iter().any(|&x| x < 0 || x as usize >= verts) {
            l.push(
                *line,
                Severity::Error,
                format!("vertex index out of range (verts {verts}): the face is dropped"),
            );
        }
        if let Some(tv) = n.tverts
            && t.iter().any(|&x| x < 0 || x as usize >= tv)
        {
            l.push(
                *line,
                Severity::Warning,
                format!("texture vertex index out of range (tverts {tv})"),
            );
        }
    }
    if n.ty == "danglymesh" {
        match n.constraints {
            Some((line, c)) if c != verts => l.push(
                line,
                Severity::Error,
                format!("{c} constraints for {verts} vertices: the game needs one per vertex"),
            ),
            None if verts > 0 => {
                l.push(
                    n.line,
                    Severity::Error,
                    "a danglymesh needs constraints for every vertex (the game crashes)",
                );
            }
            _ => {}
        }
        if let Some((line, p)) = n.period
            && p >= 60.0
        {
            l.push(line, Severity::Warning, "period 60 or more: the mesh does not move");
        }
    }
    if n.ty == "emitter"
        && n.update.as_deref() == Some("lightning")
        && let Some((line, b)) = n.birthrate
        && b > MAX_LIGHTNING_SEGMENTS
    {
        // nwn.wiki says over 9 crashes the game, but its own vim_raycold
        // and vdr_magearmor use more.
        l.push(
            line,
            Severity::Info,
            "nwn.wiki warns that a lightning birthrate over 9 can crash the game",
        );
    }
}

/// Emitters with `p2p 1` need a reference node among their children, or the
/// game crashes; checked over the whole model.
pub fn check_p2p(text: &str) -> Vec<Diagnostic> {
    let o = crate::outline::outline(text);
    let mut out = Vec::new();
    for n in o.geometry().filter(|n| n.ty == "emitter") {
        let lines: Vec<&str> = text.lines().skip(n.lines.start).take(n.lines.len()).collect();
        let p2p = lines.iter().any(|l| {
            let w = words(l);
            w.first().is_some_and(|k| k.eq_ignore_ascii_case("p2p"))
                && w.get(1).and_then(|v| v.parse::<f32>().ok()).is_some_and(|v| v != 0.0)
        });
        let has_reference = o.geometry().any(|c| {
            c.ty == "reference"
                && c.parent.as_deref().is_some_and(|p| p.eq_ignore_ascii_case(&n.name))
        });
        if p2p && !has_reference {
            out.push(Diagnostic {
                line: n.lines.start,
                severity: Severity::Error,
                message: "a point-to-point emitter needs a reference node as its child (the game \
                          crashes)"
                    .into(),
            });
        }
    }
    out
}

/// Every diagnostic for a model's text.
pub fn check(text: &str) -> Vec<Diagnostic> {
    let mut d = lint(text);
    d.extend(check_p2p(text));
    d.sort_by_key(|d| (d.line, std::cmp::Reverse(d.severity)));
    d
}

#[cfg(test)]
mod tests {
    use super::*;

    fn messages(text: &str) -> Vec<(usize, Severity, String)> {
        check(text).into_iter().map(|d| (d.line, d.severity, d.message)).collect()
    }

    const GOOD: &str = "newmodel box\nsetsupermodel box NULL\nclassification character\n\
setanimationscale 1\nbeginmodelgeom box\nnode dummy box\n  parent NULL\nendnode\n\
node trimesh plane\n  parent box\n  bitmap tex\n  verts 3\n    0 0 0\n    1 0 0\n    0 1 0\n\
  tverts 3\n    0 0 0\n    1 0 0\n    0 1 0\n  faces 1\n    0 1 2 1 0 1 2 0\nendnode\n\
endmodelgeom box\nnewanim spin box\n  length 1\n  transtime 0\nnode dummy box\n  parent NULL\n\
  orientationkey 2\n    0 0 0 1 0\n    1 0 0 1 3.14\n  endlist\nendnode\ndoneanim spin box\n\
donemodel box\n";

    #[test]
    fn a_good_model_is_quiet() {
        assert_eq!(messages(GOOD), vec![]);
    }

    #[test]
    fn typos_counts_and_indices() {
        let bad = GOOD
            .replace("  bitmap tex", "  bitmapp tex")
            .replace("  verts 3\n", "  verts 4\n")
            .replace("0 1 2 1 0 1 2 0", "0 1 7 1 0 1 2 0");
        let m = messages(&bad);
        assert!(
            m.iter()
                .any(|(l, s, msg)| *l == 10 && *s == Severity::Warning && msg.contains("bitmapp")),
            "{m:?}"
        );
        assert!(
            m.iter().any(|(l, _, msg)| *l == 11 && msg.contains("verts says 4, 3 follow")),
            "{m:?}"
        );
        assert!(
            m.iter().any(|(l, s, msg)| *l == 20
                && *s == Severity::Error
                && msg.contains("out of range")),
            "{m:?}"
        );
    }

    #[test]
    fn engine_limits() {
        let dangly = GOOD.replace("node trimesh plane", "node danglymesh plane");
        assert!(
            messages(&dangly)
                .iter()
                .any(|(_, s, m)| *s == Severity::Error && m.contains("constraints"))
        );
        let long = GOOD.replace("bitmap tex", "bitmap a_very_long_texture_name");
        assert!(
            messages(&long)
                .iter()
                .any(|(_, s, m)| *s == Severity::Error && m.contains("at most 16"))
        );
        let ee = GOOD.replace("  bitmap tex", "  bitmap tex\n  materialname mat");
        assert!(
            messages(&ee).iter().any(|(_, s, m)| *s == Severity::Info && m.contains("nwnmdlcomp"))
        );
    }

    #[test]
    fn parents_and_duplicates() {
        let orphan = GOOD.replace("  parent box\n  bitmap", "  parent nobody\n  bitmap");
        assert!(messages(&orphan).iter().any(|(_, _, m)| m.contains("nobody")));
        let dup = GOOD.replace("node trimesh plane", "node trimesh box");
        assert!(messages(&dup).iter().any(|(_, _, m)| m.contains("second node named box")));
    }

    #[test]
    fn point_to_point_needs_a_reference() {
        let p2p = GOOD.replace(
            "endmodelgeom box",
            "node emitter fx\n  parent box\n  p2p 1\n  update Fountain\nendnode\nendmodelgeom box",
        );
        assert!(
            messages(&p2p).iter().any(|(_, s, m)| *s == Severity::Error && m.contains("reference"))
        );
        let with_ref = p2p.replace(
            "endmodelgeom box",
            "node reference target\n  parent fx\nendnode\nendmodelgeom box",
        );
        assert!(!messages(&with_ref).iter().any(|(_, _, m)| m.contains("reference node")));
    }
}
