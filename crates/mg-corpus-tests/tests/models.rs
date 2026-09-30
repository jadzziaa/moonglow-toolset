//! Every model in the base game (binary and ASCII) reads, with consistent
//! meshes, skins and animations.

use mg_core::ResType;
use mg_mdl::{MeshExtra, Model, NodeKind};
use mg_resman::{GameInstall, ResMan};
use mg_testkit::corpus;
use rayon::prelude::*;

/// What is wrong with a model's data, if anything.
fn check(m: &Model) -> Option<String> {
    for (i, n) in m.nodes.iter().enumerate() {
        if let Some(p) = n.parent
            && (p >= i || !m.nodes[p].children.contains(&i))
        {
            return Some(format!("{}: bad parent", n.name));
        }
        let NodeKind::Mesh(mesh) = &n.kind else { continue };
        let count = mesh.vertices.len();
        if mesh.faces.iter().any(|f| f.vertices.iter().any(|&v| v as usize >= count)) {
            return Some(format!("{}: a face indexes past the vertices", n.name));
        }
        if !mesh.normals.is_empty() && mesh.normals.len() != count {
            return Some(format!(
                "{}: {} normals for {count} vertices",
                n.name,
                mesh.normals.len()
            ));
        }
        if mesh.uvs.iter().any(|uv| !uv.is_empty() && uv.len() != count) {
            return Some(format!("{}: UV count", n.name));
        }
        if mesh.source.len() != count || mesh.source_uv.len() != count {
            return Some(format!("{}: source count", n.name));
        }
        match &mesh.extra {
            MeshExtra::Skin(s) => {
                if !s.weights.is_empty() && s.weights.len() != count {
                    return Some(format!("{}: {} weights for {count}", n.name, s.weights.len()));
                }
                if s.bones.iter().any(|&b| b >= m.nodes.len()) {
                    return Some(format!("{}: bone past the nodes", n.name));
                }
                let used = s.weights.iter().flatten().filter(|(_, w)| *w > 0.0);
                if used.clone().any(|(b, _)| *b as usize >= s.bones.len()) {
                    return Some(format!("{}: weight on an unknown bone", n.name));
                }
            }
            MeshExtra::Dangly(d) if !d.constraints.is_empty() && d.constraints.len() != count => {
                return Some(format!("{}: constraints", n.name));
            }
            _ => {}
        }
    }
    for a in &m.animations {
        for (i, n) in a.nodes.iter().enumerate() {
            if n.parent.is_some_and(|p| p >= i) {
                return Some(format!("{}/{}: bad parent", a.name, n.name));
            }
            for c in &n.controllers {
                if c.values.len() != c.times.len() * c.columns {
                    return Some(format!("{}/{}: {} values", a.name, n.name, c.name));
                }
            }
            // Animated vertices and UVs cover the mesh's sources.
            let (Some(sets), Some(NodeKind::Mesh(mesh))) = (
                &n.anim_mesh,
                m.nodes.iter().find(|g| g.name.eq_ignore_ascii_case(&n.name)).map(|g| &g.kind),
            ) else {
                continue;
            };
            let need = |src: &[u32]| src.iter().max().map_or(0, |&x| x as usize + 1);
            if sets.vertex_sets.iter().any(|v| v.len() < need(&mesh.source))
                || sets.uv_sets.iter().any(|v| v.len() < need(&mesh.source_uv))
            {
                return Some(format!("{}/{}: animated mesh sets", a.name, n.name));
            }
        }
    }
    None
}

#[test]
fn every_model_reads() {
    let root = corpus!();
    let rm = ResMan::for_game(&GameInstall::new(&root, None, "en")).unwrap();
    let mut keys: Vec<_> =
        rm.entries().into_iter().map(|(k, _)| k).filter(|k| k.restype == ResType::MDL).collect();
    keys.sort();
    let results: Vec<(String, bool, Result<(), String>)> = keys
        .par_iter()
        .map(|k| {
            let data = rm.get(k).unwrap();
            let binary = mg_mdl::is_binary(&data);
            let r = match Model::read(&data) {
                Ok(m) => check(&m).map_or(Ok(()), Err),
                Err(e) => Err(e.to_string()),
            };
            (k.to_string(), binary, r)
        })
        .collect();
    let failed: Vec<String> = results
        .iter()
        .filter_map(|(k, b, r)| {
            r.as_ref().err().map(|e| format!("{k} ({}): {e}", if *b { "binary" } else { "ascii" }))
        })
        .collect();
    let binary = results.iter().filter(|r| r.1).count();
    eprintln!(
        "{} models ({binary} binary, {} ASCII), {} failed",
        results.len(),
        results.len() - binary,
        failed.len()
    );
    for f in failed.iter().take(40) {
        eprintln!("  {f}");
    }
    assert!(results.len() > 30_000);
    assert!(failed.is_empty());
}
