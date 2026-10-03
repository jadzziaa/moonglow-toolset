//! Moonglow's own marker: the yellow arrow along an object's facing that
//! stands for a merchant, and for a waypoint or merchant whose marker model
//! the game lacks. A model in the scene, so what stands in front hides it.

use glam::Vec3;
use mg_mdl::{Face, Mesh, Model, Node, NodeKind};

/// The arrow's corners on the ground, anticlockwise from its tip: it
/// points along +Y, as an object not turned faces.
const CORNERS: [[f32; 2]; 3] = [[0.0, 1.0], [-0.6, -0.6], [0.6, -0.6]];
/// Its underside and top, metres above the object (clear of the ground).
const HEIGHTS: (f32, f32) = (0.03, 0.15);
/// Its faces and its rim (gamma space).
const YELLOW: [f32; 3] = [240.0 / 255.0, 210.0 / 255.0, 40.0 / 255.0];
const RIM: [f32; 3] = [120.0 / 255.0, 100.0 / 255.0, 0.0];

/// The arrow's box in its own space.
pub fn arrow_bounds() -> (Vec3, Vec3) {
    (Vec3::new(-0.6, -0.6, 0.0), Vec3::new(0.6, 1.0, HEIGHTS.1))
}

fn mesh(color: [f32; 3], triangles: &[[[f32; 3]; 3]]) -> Mesh {
    let mut m = Mesh { diffuse: color, ambient: color, render: true, ..Mesh::default() };
    for t in triangles {
        let [a, b, c] = t.map(Vec3::from);
        let normal = (b - a).cross(c - a).normalize_or_zero().to_array();
        let first = m.vertices.len() as u32;
        m.vertices.extend(t);
        m.normals.extend([normal; 3]);
        m.faces.push(Face { vertices: [first, first + 1, first + 2], material: 0 });
    }
    m.source = (0..m.vertices.len() as u32).collect();
    m.source_uv = m.source.clone();
    m
}

/// The arrow: a flat yellow triangle with a darker rim.
pub fn arrow() -> Model {
    let at = |i: usize, z: f32| [CORNERS[i][0], CORNERS[i][1], z];
    let (low, high) = HEIGHTS;
    // Seen from above and from below.
    let faces = [[at(0, high), at(1, high), at(2, high)], [at(0, low), at(2, low), at(1, low)]];
    // Each edge's side, facing out.
    let rim: Vec<[[f32; 3]; 3]> = (0..3)
        .flat_map(|i| {
            let j = (i + 1) % 3;
            [[at(i, low), at(j, low), at(j, high)], [at(i, low), at(j, high), at(i, high)]]
        })
        .collect();
    let mut root = Node::new("mg_arrow", NodeKind::Dummy);
    root.children = vec![1, 2];
    let child = |name: &str, m: Mesh| {
        let mut n = Node::new(name, NodeKind::Mesh(Box::new(m)));
        n.parent = Some(0);
        n
    };
    Model {
        name: "mg_arrow".into(),
        nodes: vec![root, child("faces", mesh(YELLOW, &faces)), child("rim", mesh(RIM, &rim))],
        animation_scale: 1.0,
        ..Model::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_arrow_points_ahead_and_faces_outwards() {
        let model = arrow();
        let (min, max) = arrow_bounds();
        let centre = Vec3::new(0.0, -0.07, (HEIGHTS.0 + HEIGHTS.1) / 2.0);
        let mut triangles = 0;
        for mesh in model.nodes.iter().filter_map(Node::mesh) {
            assert!(mesh.render && mesh.textures[0].is_none());
            for (f, n) in mesh.faces.iter().zip(mesh.normals.chunks(3)) {
                let [a, b, c] = f.vertices.map(|v| Vec3::from(mesh.vertices[v as usize]));
                for p in [a, b, c] {
                    assert!(p.cmpge(min).all() && p.cmple(max).all(), "{p} is in its box");
                }
                // Anticlockwise seen from outside (the renderer culls the
                // other side).
                let out = (a + b + c) / 3.0 - centre;
                assert!((b - a).cross(c - a).dot(out) > 0.0, "{a} {b} {c}");
                assert!(Vec3::from(n[0]).dot(out) > 0.0);
                triangles += 1;
            }
        }
        assert_eq!(triangles, 8, "top, underside and three sides");
        // The tip is ahead (+Y), on the axis.
        let tip = Vec3::new(CORNERS[0][0], CORNERS[0][1], 0.0);
        assert_eq!((tip.x, tip.y), (0.0, max.y));
    }
}
