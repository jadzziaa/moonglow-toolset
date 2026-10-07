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

/// The game draws a mesh's index list, which in many tiles its compiler
/// wrote has more triangles than the face list: a tree's roots in the
/// Forest - Facelift tileset (57 faces, 159 triangles), where drawing the
/// faces alone leaves holes. With the list, every vertex is drawn.
#[test]
fn index_lists_longer_than_the_faces_are_drawn() {
    let root = corpus!();
    let rm = ResMan::for_game(&GameInstall::new(&root, None, "en")).unwrap();
    let model = Model::read(&rm.get_named("ttf02_p02_01", ResType::MDL).unwrap()).unwrap();
    let roots = model.nodes[model.node("bark02b_16").unwrap()].mesh().unwrap();
    assert_eq!((roots.faces.len(), roots.triangles().count()), (57, 159));
    let mut drawn: Vec<u32> = roots.triangles().flatten().collect();
    drawn.sort_unstable();
    drawn.dedup();
    assert_eq!(drawn.len(), roots.vertices.len());
    // A mesh whose list repeats its faces draws its faces.
    let ground = model.nodes[model.node("floor01_123").unwrap()].mesh().unwrap();
    assert!(ground.drawn.is_empty());
    assert_eq!(ground.triangles().count(), ground.faces.len());
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

/// Exploration: where binding a supermodel's animations to a model's
/// nodes by part number (as the game does) and by name (as text is
/// written) would move different nodes, among the compiled game models.
#[test]
#[ignore]
fn binding_by_part_number_and_by_name() {
    let root = corpus!();
    let rm = ResMan::for_game(&GameInstall::new(&root, None, "en")).unwrap();
    let mut keys: Vec<_> =
        rm.entries().into_iter().map(|(k, _)| k).filter(|k| k.restype == ResType::MDL).collect();
    keys.sort();
    let load = |name: &str| {
        rm.get_named(&name.to_lowercase(), ResType::MDL).ok().and_then(|d| Model::read(&d).ok())
    };
    let (mut models, mut differing, mut nodes) = (0, 0, 0);
    let mut shown = 0;
    for key in keys {
        let Ok(data) = rm.get(&key) else { continue };
        let Ok(model) = Model::read(&data) else { continue };
        if model.nodes.first().is_none_or(|n| n.part.is_none()) {
            continue;
        }
        let Some(sup) = model.supermodel.as_deref().and_then(load) else { continue };
        if sup.nodes.first().is_none_or(|n| n.part.is_none()) {
            continue;
        }
        models += 1;
        // Each node of the supermodel that its animations move: the
        // model's node of that name, and its node of that number.
        let mut here: Vec<(String, i32, Option<usize>, Option<usize>)> = Vec::new();
        for a in &sup.animations {
            for n in &a.nodes {
                // (The root, numbered 0 and named after its model, apart.)
                let Some(part) = n.part.filter(|p| *p > 0) else { continue };
                let by_name = model.node(&n.name);
                let by_part = model.nodes.iter().position(|m| m.part == Some(part));
                if by_name != by_part && !here.iter().any(|(name, ..)| *name == n.name) {
                    here.push((n.name.clone(), part, by_name, by_part));
                }
            }
        }
        if !here.is_empty() {
            differing += 1;
            nodes += here.len();
            if shown < 40 {
                shown += 1;
                let said: Vec<String> = here
                    .iter()
                    .take(4)
                    .map(|(name, part, by_name, by_part)| {
                        let at = |i: &Option<usize>| {
                            i.map_or("none".to_string(), |i| model.nodes[i].name.clone())
                        };
                        let own = by_name.and_then(|i| model.nodes[i].part);
                        format!(
                            "{name}#{part}: name->{}#{own:?} part->{}",
                            at(by_name),
                            at(by_part)
                        )
                    })
                    .collect();
                eprintln!("{} (super {}): {}", key, sup.name, said.join("; "));
            }
        }
    }
    eprintln!("{models} models with a compiled supermodel; {differing} differ, in {nodes} nodes");
    // Animation nodes numbered −1 (none of the model's) that have keys
    // and a node of the model of their name: a model's own, and its
    // supermodel's.
    let (mut own_unbound, mut super_unbound, mut eg) = (0, 0, Vec::new());
    let mut keys: Vec<_> =
        rm.entries().into_iter().map(|(k, _)| k).filter(|k| k.restype == ResType::MDL).collect();
    keys.sort();
    for key in keys {
        let Ok(data) = rm.get(&key) else { continue };
        let Ok(model) = Model::read(&data) else { continue };
        if model.nodes.first().is_none_or(|n| n.part.is_none()) {
            continue;
        }
        let loose = |m: &Model, of: &Model| {
            m.animations
                .iter()
                .flat_map(|a| &a.nodes)
                .filter(|n| n.part.is_some_and(|p| p < 0) && !n.controllers.is_empty())
                .filter(|n| of.node(&n.name).is_some())
                .map(|n| n.name.clone())
                .collect::<std::collections::BTreeSet<_>>()
        };
        let own = loose(&model, &model);
        own_unbound += own.len();
        if !own.is_empty() && eg.len() < 8 {
            eg.push(format!("{key}: {:?}", own.iter().take(3).collect::<Vec<_>>()));
        }
        if let Some(sup) = model.supermodel.as_deref().and_then(load) {
            super_unbound += loose(&sup, &model).len();
        }
    }
    eprintln!(
        "numbered -1 with keys and a namesake: {own_unbound} own, {super_unbound} of supermodels; {eg:?}"
    );
    // How far the supermodel's pause turns a node the two ways disagree on
    // (degrees between its first key and the one farthest from it): what
    // would show in the game, for a look there.
    for (model, node) in [
        ("c_sharkgb", "FinR"),
        ("c_cat_lion", "head"),
        ("c_giantliz", "head"),
        ("c_cat_jag", "head"),
        ("c_blade_m", "head_g"),
    ] {
        let Some(m) = load(model) else { continue };
        let Some(sup) = m.supermodel.as_deref().and_then(load) else { continue };
        for a in sup
            .animations
            .iter()
            .filter(|a| a.name.starts_with("cpause1") || a.name.starts_with("cwalk"))
        {
            let Some(n) = a.nodes.iter().find(|n| n.name.eq_ignore_ascii_case(node)) else {
                continue;
            };
            let Some(c) = n.controllers.iter().find(|c| c.name == "orientation") else { continue };
            let q = |i: usize| {
                let r = c.row(i);
                glam::Quat::from_xyzw(r[0], r[1], r[2], r[3])
            };
            let rows = c.times.len();
            let turn =
                (1..rows).map(|i| q(0).angle_between(q(i)).to_degrees()).fold(0.0f32, f32::max);
            eprintln!(
                "{model} {node} in {} of {}: {rows} keys, turns {turn:.1}°",
                a.name, sup.name
            );
        }
    }
}
