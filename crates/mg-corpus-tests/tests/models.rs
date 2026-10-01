//! Every model in the base game (binary and ASCII) reads, with consistent
//! meshes, skins and animations.

use mg_core::ResType;
use mg_mdl::walkmesh::{DoorState, PointKind, Walkmesh, WalkmeshKind};
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
        if !mesh.tangents.is_empty() && mesh.tangents.len() != count {
            return Some(format!("{}: {} tangents for {count}", n.name, mesh.tangents.len()));
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
                if c.values.len() != c.times.len() * c.columns
                    || c.is_bezier() && c.handles.len() != c.values.len() * 2
                {
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

/// Every walkmesh reads with its nodes under the object's root, and the
/// points sit where the object puts them.
#[test]
fn every_walkmesh_reads() {
    let root = corpus!();
    let rm = ResMan::for_game(&GameInstall::new(&root, None, "en")).unwrap();
    let mut failed = Vec::new();
    let mut counts = [0usize; 3];
    let (mut empty, mut use_points, mut door_points) = (0, 0, 0);
    let mut door_states = [0usize; 3];
    for (k, _) in rm.entries() {
        let Some(kind) = k.restype.extension().and_then(WalkmeshKind::from_extension) else {
            continue;
        };
        let data = rm.get(&k).unwrap();
        let w = match Walkmesh::read(&data, kind) {
            Ok(w) => w,
            Err(e) => {
                failed.push(format!("{k}: {e}"));
                continue;
            }
        };
        if let Some(e) = check(&w.model) {
            failed.push(format!("{k}: {e}"));
        }
        if w.model.nodes.iter().skip(1).any(|n| n.parent.is_none()) {
            failed.push(format!("{k}: a second root"));
        }
        let i = match kind {
            WalkmeshKind::Tile => 0,
            WalkmeshKind::Placeable => 1,
            WalkmeshKind::Door => 2,
        };
        counts[i] += 1;
        empty += usize::from(w.surfaces.is_empty());
        use_points += w.use_points().len();
        door_points += w.points.iter().filter(|p| matches!(p.kind, PointKind::Door(..))).count();
        for (j, state) in [DoorState::Closed, DoorState::Open1, DoorState::Open2].iter().enumerate()
        {
            door_states[j] += usize::from(w.surfaces.iter().any(|s| s.door == Some(*state)));
        }
    }
    eprintln!(
        "{} tile, {} placeable and {} door walkmeshes ({empty} without surfaces); \
         {use_points} use points, {door_points} door points; door meshes closed, open 1, \
         open 2: {door_states:?}; {} failed",
        counts[0],
        counts[1],
        counts[2],
        failed.len()
    );
    for f in failed.iter().take(40) {
        eprintln!("  {f}");
    }
    assert!(counts.iter().all(|&c| c > 50), "{counts:?}");
    assert!(failed.is_empty());
}

/// Every ASCII model's source map: a span per node, in the text and in file
/// order, and what the reader left out of the game's models.
#[test]
fn ascii_source_maps() {
    let root = corpus!();
    let rm = ResMan::for_game(&GameInstall::new(&root, None, "en")).unwrap();
    let keys: Vec<_> =
        rm.entries().into_iter().map(|(k, _)| k).filter(|k| k.restype == ResType::MDL).collect();
    let results: Vec<(String, Option<String>, usize)> = keys
        .par_iter()
        .filter_map(|k| {
            let data = rm.get(k).unwrap();
            if mg_mdl::is_binary(&data) {
                return None;
            }
            let lines = data.split(|&b| b == b'\n').count();
            let (m, map) = mg_mdl::ascii::read_mapped(&data).unwrap();
            let bad =
                if map.nodes.len() != m.nodes.len() || map.animations.len() != m.animations.len() {
                    Some("counts".to_string())
                } else {
                    map.nodes
                        .iter()
                        .chain(map.animations.iter().flat_map(|a| &a.nodes))
                        .find(|s| s.start == 0 || s.end < s.start || s.end > lines)
                        .map(|s| format!("span {s:?}"))
                };
            Some((k.to_string(), bad, map.notes.len()))
        })
        .collect();
    let bad: Vec<String> =
        results.iter().filter_map(|(k, b, _)| b.as_ref().map(|b| format!("{k}: {b}"))).collect();
    let noted = results.iter().filter(|r| r.2 > 0).count();
    eprintln!(
        "{} ASCII models; {noted} with notes ({} notes)",
        results.len(),
        results.iter().map(|r| r.2).sum::<usize>()
    );
    for b in bad.iter().take(20) {
        eprintln!("  {b}");
    }
    assert!(results.len() > 5_000);
    assert!(bad.is_empty());
}
