//! Where things are in an ASCII model's text: the model's header, each node
//! block and each animation, by line. The editor uses it to follow the
//! cursor (the node under it is selected in the view) and to jump to a
//! node picked in the view.

use std::ops::Range;

/// A node block.
#[derive(Debug, Clone, PartialEq)]
pub struct NodeSpan {
    /// The node type as written (lower case).
    pub ty: String,
    pub name: String,
    pub parent: Option<String>,
    /// Lines (0-based) from `node` to its `endnode` (inclusive).
    pub lines: Range<usize>,
    /// The animation it belongs to (index into [`Outline::animations`]);
    /// `None`: the geometry.
    pub animation: Option<usize>,
}

/// An animation block.
#[derive(Debug, Clone, PartialEq)]
pub struct AnimSpan {
    pub name: String,
    /// Lines from `newanim` to `doneanim` (inclusive).
    pub lines: Range<usize>,
    pub length: Option<f32>,
}

/// The structure of an ASCII model.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Outline {
    pub model: Option<String>,
    pub supermodel: Option<String>,
    pub classification: Option<String>,
    pub nodes: Vec<NodeSpan>,
    pub animations: Vec<AnimSpan>,
}

impl Outline {
    /// The node block a line is in.
    pub fn node_at(&self, line: usize) -> Option<&NodeSpan> {
        self.nodes.iter().find(|n| n.lines.contains(&line))
    }

    /// The animation a line is in.
    pub fn animation_at(&self, line: usize) -> Option<&AnimSpan> {
        self.animations.iter().find(|a| a.lines.contains(&line))
    }

    /// A node by name (case-insensitive) in the geometry or in an
    /// animation.
    pub fn find_node(&self, name: &str, animation: Option<&str>) -> Option<&NodeSpan> {
        let anim = match animation {
            Some(a) => Some(self.animations.iter().position(|x| x.name.eq_ignore_ascii_case(a))?),
            None => None,
        };
        self.nodes.iter().find(|n| n.animation == anim && n.name.eq_ignore_ascii_case(name))
    }

    /// The geometry's node blocks.
    pub fn geometry(&self) -> impl Iterator<Item = &NodeSpan> {
        self.nodes.iter().filter(|n| n.animation.is_none())
    }
}

/// The first word of a line (comments removed), lower case, and the rest.
pub(crate) fn words(line: &str) -> Vec<&str> {
    line.split('#').next().unwrap_or_default().split_whitespace().collect()
}

/// Scans a model's text.
pub fn outline(text: &str) -> Outline {
    let mut o = Outline::default();
    let mut open_node: Option<NodeSpan> = None;
    let mut open_anim: Option<AnimSpan> = None;
    let mut line_count = 0;
    for (i, line) in text.lines().enumerate() {
        line_count = i + 1;
        let w = words(line);
        let Some(first) = w.first() else { continue };
        let key = first.to_ascii_lowercase();
        let close_node = |o: &mut Outline, n: Option<NodeSpan>, end: usize| {
            if let Some(mut n) = n {
                n.lines.end = end;
                o.nodes.push(n);
            }
        };
        match key.as_str() {
            "newmodel" => o.model = w.get(1).map(|s| s.to_string()),
            "setsupermodel" => {
                o.supermodel =
                    w.get(2).filter(|s| !s.eq_ignore_ascii_case("null")).map(|s| s.to_string());
            }
            "classification" => o.classification = w.get(1).map(|s| s.to_ascii_lowercase()),
            "node" => {
                // A missing endnode ends the previous block here.
                close_node(&mut o, open_node.take(), i);
                open_node = Some(NodeSpan {
                    ty: w.get(1).map(|s| s.to_ascii_lowercase()).unwrap_or_default(),
                    name: w.get(2).map(|s| s.to_string()).unwrap_or_default(),
                    parent: None,
                    lines: i..i + 1,
                    animation: open_anim.as_ref().map(|_| o.animations.len()),
                });
            }
            "parent" => {
                if let Some(n) = &mut open_node {
                    n.parent =
                        w.get(1).filter(|s| !s.eq_ignore_ascii_case("null")).map(|s| s.to_string());
                }
            }
            "endnode" => close_node(&mut o, open_node.take(), i + 1),
            "newanim" => {
                close_node(&mut o, open_node.take(), i);
                if let Some(mut a) = open_anim.take() {
                    a.lines.end = i;
                    o.animations.push(a);
                }
                open_anim = Some(AnimSpan {
                    name: w.get(1).map(|s| s.to_string()).unwrap_or_default(),
                    lines: i..i + 1,
                    length: None,
                });
            }
            "length" => {
                if let Some(a) = &mut open_anim {
                    a.length = w.get(1).and_then(|v| v.parse().ok());
                }
            }
            "doneanim" => {
                close_node(&mut o, open_node.take(), i);
                if let Some(mut a) = open_anim.take() {
                    a.lines.end = i + 1;
                    o.animations.push(a);
                }
            }
            "endmodelgeom" | "endwalkmeshgeom" | "donemodel" => {
                close_node(&mut o, open_node.take(), i);
            }
            _ => {}
        }
    }
    if let Some(mut n) = open_node.take() {
        n.lines.end = line_count;
        o.nodes.push(n);
    }
    if let Some(mut a) = open_anim.take() {
        a.lines.end = line_count;
        o.animations.push(a);
    }
    o
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEXT: &str = "newmodel box\nsetsupermodel box NULL\nclassification Character\n\
beginmodelgeom box\nnode dummy box\n  parent NULL\nendnode\n\
node trimesh plane # a comment\n  parent box\n  verts 0\nendnode\nendmodelgeom box\n\
newanim spin box\n  length 2\nnode dummy box\n  parent NULL\nendnode\ndoneanim spin box\n\
donemodel box\n";

    #[test]
    fn spans() {
        let o = outline(TEXT);
        assert_eq!(o.model.as_deref(), Some("box"));
        assert_eq!(o.supermodel, None);
        assert_eq!(o.classification.as_deref(), Some("character"));
        assert_eq!(o.nodes.len(), 3);
        let plane = o.find_node("PLANE", None).unwrap();
        assert_eq!(plane.lines, 7..11);
        assert_eq!(plane.parent.as_deref(), Some("box"));
        assert_eq!(o.node_at(9).unwrap().name, "plane");
        assert_eq!(o.animations.len(), 1);
        assert_eq!(o.animations[0].lines, 12..18);
        assert_eq!(o.animations[0].length, Some(2.0));
        let anim_box = o.find_node("box", Some("spin")).unwrap();
        assert_eq!(anim_box.lines, 14..17);
        assert_eq!(o.animation_at(15).unwrap().name, "spin");
        assert!(o.find_node("box", Some("walk")).is_none());
    }

    #[test]
    fn missing_endnode_ends_at_the_next_node() {
        let o = outline("node dummy a\n  parent NULL\nnode dummy b\n  parent a\n");
        assert_eq!(o.nodes[0].lines, 0..2);
        assert_eq!(o.nodes[1].lines, 2..4);
    }
}
