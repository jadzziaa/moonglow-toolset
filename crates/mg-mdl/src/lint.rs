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

/// Every line's words, as [`words`] splits them, split once: the checks
/// look at a line several times, and a tile model has hundreds of
/// thousands of lines.
struct Lines<'a> {
    words: Vec<&'a str>,
    /// Where each line's words end in `words`.
    ends: Vec<usize>,
}

impl<'a> Lines<'a> {
    fn new(text: &'a str) -> Self {
        let mut words = Vec::with_capacity(text.len() / 8);
        let mut ends = Vec::with_capacity(text.len() / 32);
        // (Text all ASCII, as models are, is split by its bytes: the
        // whitespace `split_whitespace` knows there is these six.)
        let ascii = text.is_ascii();
        let space = |c: u8| matches!(c, b'\t' | b'\n' | 0x0B | 0x0C | b'\r' | b' ');
        for line in text.lines() {
            let line = line.split('#').next().unwrap_or_default();
            if ascii {
                let b = line.as_bytes();
                let mut i = 0;
                while i < b.len() {
                    while i < b.len() && space(b[i]) {
                        i += 1;
                    }
                    let start = i;
                    while i < b.len() && !space(b[i]) {
                        i += 1;
                    }
                    if start < i {
                        words.push(&line[start..i]);
                    }
                }
            } else {
                words.extend(line.split_whitespace());
            }
            ends.push(words.len());
        }
        Lines { words, ends }
    }

    fn len(&self) -> usize {
        self.ends.len()
    }

    /// Line `i`'s words.
    fn line(&self, i: usize) -> &[&'a str] {
        let start = if i == 0 { 0 } else { self.ends[i - 1] };
        &self.words[start..self.ends[i]]
    }
}

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
/// A mesh's faces: a binary face names its neighbours in 16 bits, signed.
/// Not 65535 / 3 (nwn.wiki's Models page): the game draws a mesh's index
/// list, whose count is 32 bits, and a mesh of 29,768 faces was drawn
/// whole in the client (`placeables_look`, dafena's report after 3.1.2).
pub const MAX_FACES: usize = 32_767;
/// A mesh's vertices: their indices are 16 bits.
pub const MAX_VERTICES: usize = 65_535;
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
    /// In an animation: its `animverts` (true) and `animtverts` lines.
    frames: Vec<(usize, bool)>,
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
    let lines = Lines::new(text);
    let mut node: Option<Node> = None;
    let mut in_anim = false;
    let mut geometry_names: HashMap<String, usize> = HashMap::new();
    let mut nodes: Vec<Node> = Vec::new();
    let mut anim_names: HashSet<String> = HashSet::new();
    let mut duplicates: Vec<(usize, usize, String)> = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let w = lines.line(i);
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
                    // The game divides an animated mesh's frames by the
                    // animation node's own `verts` and `tverts`: without
                    // them it crashes (nwn-mdl-spec E54).
                    for &(line, verts) in &n.frames {
                        let (given, list) =
                            if verts { (n.verts, "verts") } else { (n.tverts, "tverts") };
                        if given.is_none_or(|c| c == 0) {
                            let frames = if verts { "animverts" } else { "animtverts" };
                            l.push(
                                line,
                                Severity::Error,
                                format!(
                                    "{frames} with no {list} in the animation's node: the game \
                                     divides by its {list} and crashes"
                                ),
                            );
                        }
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
                i = node_line(&mut l, n, &lines, at, w, &key, in_anim);
            }
        }
    }
    if let Some(n) = node.take() {
        l.push(n.line, Severity::Warning, format!("node {} has no endnode", n.name));
        nodes.push(n);
    }

    // Parents: geometry parents must be earlier nodes. (And the last line
    // naming each parent, animations' too, for the duplicates below.)
    let mut seen: HashSet<String> = HashSet::new();
    let mut last_parent: HashMap<String, usize> = HashMap::new();
    let mut in_anim = false;
    for at in 0..lines.len() {
        let w = lines.line(at);
        let Some(first) = w.first() else { continue };
        if first.eq_ignore_ascii_case("newanim") {
            in_anim = true;
        } else if first.eq_ignore_ascii_case("doneanim") {
            in_anim = false;
        } else if first.eq_ignore_ascii_case("node") {
            if !in_anim && let Some(n) = w.get(2) {
                seen.insert(n.to_ascii_lowercase());
            }
        } else if first.eq_ignore_ascii_case("parent")
            && let Some(p) = w.get(1)
        {
            if !duplicates.is_empty() {
                last_parent.insert(p.to_ascii_lowercase(), at);
            }
            if !in_anim
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
    }

    // Duplicate names: harmless unless something hangs from the second.
    for (at, prev, name) in duplicates {
        let parented = last_parent.get(&name.to_ascii_lowercase()).is_some_and(|&p| p > at);
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
    lines: &Lines<'_>,
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
        let mut rows: Vec<(usize, &[&str])> = Vec::new();
        while next < lines.len() {
            let rw = lines.line(next);
            let Some(first) = rw.first() else {
                next += 1;
                continue;
            };
            let full = declared.is_some_and(|c| rows.len() >= c);
            if full
                || first.eq_ignore_ascii_case("endlist")
                || first.eq_ignore_ascii_case("endnode")
                || (!named && !is_number(first))
            {
                break;
            }
            rows.push((next, rw));
            next += 1;
        }
        // An `endlist` after the rows belongs to the list.
        while next < lines.len() && lines.line(next).is_empty() {
            next += 1;
        }
        if next < lines.len()
            && lines.line(next).first().is_some_and(|w| w.eq_ignore_ascii_case("endlist"))
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
            // (An animated mesh's frames, and what the game divides
            // their counts by.)
            match key {
                "verts" => n.verts = Some(rows.len()),
                "tverts" | "tverts0" => n.tverts = Some(rows.len()),
                "animverts" | "animtverts" => n.frames.push((at, key == "animverts")),
                _ => {}
            }
            return next;
        }
        match key {
            "verts" => {
                n.verts = Some(rows.len());
                if rows.len() > MAX_VERTICES {
                    l.push(
                        at,
                        Severity::Error,
                        format!("{} vertices; a mesh has at most {MAX_VERTICES}", rows.len()),
                    );
                }
            }
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
            while next < lines.len() && lines.line(next).first().is_some_and(|w| is_number(w)) {
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
                // A texture's name is cut to 16 characters as the game
                // looks it up (seen in the client: a `bitmap` of 20 drawn
                // with the TGA of its first 16; dafena's report after
                // 3.1.2, Blender's `.014` names); a model's is not known.
                let texture = !matches!(key, "refmodel" | "chunkname");
                let cut: String = name.chars().take(MAX_RESREF).collect();
                l.push(
                    at,
                    if texture { Severity::Warning } else { Severity::Error },
                    if texture {
                        format!(
                            "{name}: longer than a resource name ({MAX_RESREF} characters): \
                             the game looks for {cut}"
                        )
                    } else {
                        format!("{name}: resource names are at most {MAX_RESREF} characters")
                    },
                );
            }
        }
        "p2p" => n.p2p = value().is_some_and(|v| v != 0.0),
        "update" => n.update = w.get(1).map(|s| s.to_ascii_lowercase()),
        "birthrate" => n.birthrate = value().map(|v| (at, v)),
        "period" => n.period = value().map(|v| (at, v)),
        "parent" if w[0] != "parent" => l.push(
            at,
            Severity::Warning,
            format!("{}: the game reads parent in lower case only; this node is in no tree", w[0]),
        ),
        _ => {}
    }
    unknown(l, at, &n.ty, w[0], key);
    next
}

/// Reports a keyword the node's type does not use: a warning if no node
/// uses it (a typo, most likely), a note if another type does (the game's
/// own models carry dangly settings on skins, for one).
fn unknown(l: &mut Lint, at: usize, ty: &str, word: &str, key: &str) {
    if let Some((_, instead)) = keywords::NOT_READ.iter().find(|(k, _)| *k == key) {
        let message = if instead.is_empty() {
            format!("{word}: the game does not read it")
        } else {
            format!("{word}: the game does not read it; it reads {instead}")
        };
        l.push(at, Severity::Info, message);
        return;
    }
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
    // (Most models have no `p2p` at all: then there is nothing to outline.)
    let b = text.as_bytes();
    let p2p_written = text.match_indices('2').any(|(i, _)| {
        i > 0
            && b[i - 1].eq_ignore_ascii_case(&b'p')
            && b.get(i + 1).is_some_and(|c| c.eq_ignore_ascii_case(&b'p'))
    });
    if !p2p_written {
        return Vec::new();
    }
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
        // A texture's long name: what the game looks for (its first 16
        // characters), not an error; a model's is one.
        let long = GOOD.replace("bitmap tex", "bitmap a_very_long_texture_name");
        assert!(messages(&long).iter().any(|(_, s, m)| *s == Severity::Warning
            && m.contains("the game looks for a_very_long_text")));
        assert!(!messages(&long).iter().any(|(_, s, _)| *s == Severity::Error));
        let model = GOOD.replace("bitmap tex", "refmodel a_very_long_model_name");
        assert!(
            messages(&model)
                .iter()
                .any(|(_, s, m)| *s == Severity::Error && m.contains("at most 16"))
        );
        // More faces than 65535 / 3 (dafena's 24,005): drawn by the game;
        // more than the format's 32,767 not.
        let faces = |n: usize| {
            let rows: String = (0..n).map(|_| "    0 1 2 1 0 1 2 0\n").collect();
            GOOD.replacen("  faces 1\n", &format!("  faces {}\n{rows}", n + 1), 1)
        };
        let too_many = |n| {
            messages(&faces(n))
                .iter()
                .any(|(_, s, m)| *s == Severity::Error && m.contains("faces;"))
        };
        assert!(!too_many(22_000), "more than 21845 faces are drawn");
        assert!(too_many(32_800), "past the binary's 32767");
        let ee = GOOD.replace("  bitmap tex", "  bitmap tex\n  materialname mat");
        assert!(
            messages(&ee).iter().any(|(_, s, m)| *s == Severity::Info && m.contains("nwnmdlcomp"))
        );
    }

    /// What tools write and the game does not read is pointed out
    /// (nwn-mdl-spec MDL-ASCII-070, MDL-ASCII-010).
    #[test]
    fn keywords_the_game_does_not_read() {
        let text = "newmodel t\nbeginmodelgeom t\nnode dummy t\n  parent NULL\nendnode\n\
                    node trimesh m\n  parent t\n  setfillumcolor 1 1 1\nendnode\n\
                    node light l\n  PARENT t\n  ambient_only 1\nendnode\n\
                    endmodelgeom t\ndonemodel t\n";
        let found = messages(text);
        let said =
            |s: &str| found.iter().find(|(_, _, m)| m.contains(s)).map(|(l, sev, _)| (*l, *sev));
        assert_eq!(said("it reads selfillumcolor"), Some((7, Severity::Info)), "{found:?}");
        assert_eq!(said("it reads ambientonly"), Some((11, Severity::Info)), "{found:?}");
        assert_eq!(said("lower case only"), Some((10, Severity::Warning)), "{found:?}");
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
        // (Keywords in any case: the text is searched for `p2p` first.)
        let upper = p2p.replace("  p2p 1", "  P2P 1");
        assert!(messages(&upper).iter().any(|(_, _, m)| m.contains("reference")));
        assert!(check_p2p(GOOD).is_empty());
    }

    /// A second node of a name is a warning when a `parent` line after it
    /// names it, an animation's too; else a note.
    #[test]
    fn a_duplicate_is_parented_by_any_later_line() {
        let dup =
            GOOD.replace("node trimesh plane\n  parent box", "node trimesh box\n  parent NULL");
        let severity = |text: &str| {
            messages(text)
                .into_iter()
                .find(|(_, _, m)| m.contains("second node named box"))
                .map(|f| f.1)
        };
        // The animation's `parent NULL` names nothing.
        assert_eq!(severity(&dup), Some(Severity::Info));
        let in_anim = dup.replace(
            "node dummy box\n  parent NULL\norientationkey",
            "node dummy box\n  parent box\norientationkey",
        );
        assert_ne!(in_anim, dup);
        assert_eq!(severity(&in_anim), Some(Severity::Warning));
        // A parent line before the second node is not after it.
        let before = dup.replace(
            "node dummy box\n  parent NULL\nendnode",
            "node dummy box\n  parent box\nendnode",
        );
        assert_ne!(before, dup);
        assert_eq!(severity(&before), Some(Severity::Info), "{:?}", messages(&before));
    }

    /// An animated mesh's frames with no `verts` (or `tverts`) in the
    /// animation's node crash the game (nwn-mdl-spec E54); with them, or
    /// with no frames, nothing is said.
    #[test]
    fn frames_need_the_animation_node_s_verts() {
        let anim = |lists: &str| {
            format!(
                "newmodel w\nbeginmodelgeom w\nnode dummy w\n  parent NULL\nendnode\n\
                 endmodelgeom w\nnewanim a w\n  length 1\n  node animmesh m\n    parent w\n\
                 {lists}  endnode\ndoneanim a w\ndonemodel w\n"
            )
        };
        let crashes = |text: &str| {
            let found = messages(text);
            found.iter().filter(|(_, s, m)| *s == Severity::Error && m.contains("crashes")).count()
        };
        assert_eq!(crashes(&anim("animverts 1\n  0 0 0\nanimtverts 1\n  0 0 0\n")), 2);
        let given = "verts 1\n  0 0 0\ntverts 1\n  0 0 0\nanimverts 1\n  0 0 0\n\
                     animtverts 1\n  0 0 0\n";
        assert_eq!(crashes(&anim(given)), 0);
        assert_eq!(crashes(&anim("sampleperiod 1\n")), 0);
    }

    /// Words are split as `split_whitespace` splits them: a vertical tab
    /// and a form feed too, and text that is not all ASCII by its
    /// characters (a no-break space).
    #[test]
    fn words_split_at_any_whitespace() {
        let long = |space: &str| {
            let text =
                GOOD.replace("  bitmap tex", &format!("{space}bitmap{space}a234567890123456789"));
            messages(&text).into_iter().any(|(l, s, m)| {
                l == 10 && s == Severity::Warning && m.contains("longer than a resource name")
            })
        };
        assert!(long("\x0b"), "a vertical tab");
        assert!(long("\x0c"), "a form feed");
        assert!(long("\t"));
        assert!(long("\u{a0}"), "a no-break space");
        assert!(long("\u{3000}"), "an ideographic space");
    }
}
