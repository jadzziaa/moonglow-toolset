//! The binary and ASCII readers agree with each other through nwnmdlcomp
//! (Torlack's compiler/decompiler, the tool Neverblender uses): a sample of
//! the game's compiled models, decompiled to ASCII by nwnmdlcomp and read by
//! the ASCII reader, gives the same model as the binary reader: nodes,
//! transforms, triangles with texture coordinates and materials, skin
//! weights, lights, emitters and animation keys.

use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;

use mg_core::ResType;
use mg_mdl::{Controller, MeshExtra, Model, NodeKind, Quat, Vec3};
use mg_resman::{GameInstall, ResMan};
use mg_testkit::{corpus, oracle_tool, scratch_dir};
use rayon::prelude::*;

const TOL: f32 = 2e-3;

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() <= TOL * a.abs().max(b.abs()).max(1.0)
}

fn close3(a: Vec3, b: Vec3) -> bool {
    (0..3).all(|i| close(a[i], b[i]))
}

fn same_rotation(a: Quat, b: Quat) -> bool {
    let dot: f32 = (0..4).map(|i| a[i] * b[i]).sum();
    dot.abs() > 0.999
}

/// A mesh's triangles as sorted corner lists (position, UV), rounded.
fn triangles(m: &mg_mdl::Mesh) -> BTreeMap<Vec<[i64; 5]>, usize> {
    let q = |v: f32| (v * 500.0).round() as i64;
    let mut out = BTreeMap::new();
    for f in &m.faces {
        let mut corners: Vec<[i64; 5]> = f
            .vertices
            .iter()
            .map(|&i| {
                let p = m.vertices[i as usize];
                let uv = m.uvs[0].get(i as usize).copied().unwrap_or([0.0, 0.0]);
                [q(p[0]), q(p[1]), q(p[2]), q(uv[0]), q(uv[1])]
            })
            .collect();
        corners.sort();
        corners.push([f.material as i64, 0, 0, 0, 0]);
        *out.entry(corners).or_insert(0) += 1;
    }
    out
}

/// Per corner position, the sorted (bone name, weight) pairs.
fn skin_weights(model: &Model, m: &mg_mdl::Mesh) -> BTreeMap<[i64; 3], Vec<(String, i64)>> {
    let MeshExtra::Skin(s) = &m.extra else { return BTreeMap::new() };
    let q = |v: f32| (v * 500.0).round() as i64;
    let mut out = BTreeMap::new();
    for (i, w) in s.weights.iter().enumerate() {
        let p = m.vertices[i];
        let mut pairs: Vec<(String, i64)> = w
            .iter()
            .filter(|(_, w)| *w > 0.0005)
            .map(|(b, w)| (model.nodes[s.bones[*b as usize]].name.to_ascii_lowercase(), q(*w)))
            .collect();
        pairs.sort();
        out.insert([q(p[0]), q(p[1]), q(p[2])], pairs);
    }
    out
}

fn compare_controllers(what: &str, a: &[Controller], b: &[Controller], diffs: &mut Vec<String>) {
    for ca in a {
        let Some(cb) = b.iter().find(|c| c.name == ca.name) else {
            diffs.push(format!("{what}: {} missing from the ASCII", ca.name));
            continue;
        };
        if ca.times.len() != cb.times.len() || ca.columns != cb.columns {
            diffs.push(format!(
                "{what}: {} has {}×{} vs {}×{}",
                ca.name,
                ca.times.len(),
                ca.columns,
                cb.times.len(),
                cb.columns
            ));
            continue;
        }
        let ok = ca.times.iter().zip(&cb.times).all(|(x, y)| close(*x, *y))
            && (0..ca.times.len()).all(|r| {
                let (x, y) = (ca.row(r), cb.row(r));
                if ca.name == "orientation" {
                    same_rotation([x[0], x[1], x[2], x[3]], [y[0], y[1], y[2], y[3]])
                } else {
                    x.iter().zip(y).all(|(x, y)| close(*x, *y))
                }
            });
        if !ok {
            diffs.push(format!("{what}: {} values differ", ca.name));
        }
    }
}

/// How the binary model and the decompiled one differ.
fn compare(bin: &Model, asc: &Model) -> Vec<String> {
    let mut d = Vec::new();
    if !bin.name.eq_ignore_ascii_case(&asc.name) {
        d.push(format!("name {} vs {}", bin.name, asc.name));
    }
    if bin.supermodel != asc.supermodel {
        d.push(format!("supermodel {:?} vs {:?}", bin.supermodel, asc.supermodel));
    }
    if bin.classification != asc.classification {
        d.push(format!("classification {:?} vs {:?}", bin.classification, asc.classification));
    }
    if bin.nodes.len() != asc.nodes.len() {
        d.push(format!("{} nodes vs {}", bin.nodes.len(), asc.nodes.len()));
        return d;
    }
    // nwnmdlcomp writes siblings in its own order: match nodes by name.
    let parent_name =
        |m: &Model, i: usize| m.nodes[i].parent.map(|p| m.nodes[p].name.to_ascii_lowercase());
    for (ia, a) in bin.nodes.iter().enumerate() {
        let what = &a.name;
        // Several nodes may share a name: the one with the same parent,
        // nearest in position.
        let dist = |p: Vec3| (0..3).map(|i| (p[i] - a.position[i]).powi(2)).sum::<f32>();
        let Some(ib) = asc
            .nodes
            .iter()
            .enumerate()
            .filter(|(_, n)| n.name.eq_ignore_ascii_case(&a.name))
            .min_by(|(i, x), (j, y)| {
                let same = |k: usize| parent_name(asc, k) == parent_name(bin, ia);
                same(*j).cmp(&same(*i)).then(dist(x.position).total_cmp(&dist(y.position)))
            })
            .map(|(i, _)| i)
        else {
            d.push(format!("{what}: missing from the ASCII"));
            continue;
        };
        let b = &asc.nodes[ib];
        if parent_name(bin, ia) != parent_name(asc, ib) {
            d.push(format!(
                "{what}: parent {:?} vs {:?}",
                parent_name(bin, ia),
                parent_name(asc, ib)
            ));
            continue;
        }
        if a.kind.type_name() != b.kind.type_name() {
            d.push(format!("{what}: {} vs {}", a.kind.type_name(), b.kind.type_name()));
            continue;
        }
        if !close3(a.position, b.position) || !same_rotation(a.orientation, b.orientation) {
            d.push(format!(
                "{what}: transform {:?} {:?} vs {:?} {:?}",
                a.position, a.orientation, b.position, b.orientation
            ));
        }
        compare_controllers(what, &a.controllers, &b.controllers, &mut d);
        match (&a.kind, &b.kind) {
            (NodeKind::Mesh(ma), NodeKind::Mesh(mb)) => {
                if ma.textures[0] != mb.textures[0] {
                    d.push(format!("{what}: bitmap {:?} vs {:?}", ma.textures[0], mb.textures[0]));
                }
                let flags = |m: &mg_mdl::Mesh| {
                    (m.render, m.shadow, m.transparency_hint, m.tilefade, m.rotate_texture)
                };
                if flags(ma) != flags(mb) {
                    d.push(format!("{what}: mesh flags {:?} vs {:?}", flags(ma), flags(mb)));
                }
                if !close3(ma.diffuse, mb.diffuse) || !close3(ma.ambient, mb.ambient) {
                    d.push(format!("{what}: colours {:?} vs {:?}", ma.diffuse, mb.diffuse));
                }
                if triangles(ma) != triangles(mb) {
                    d.push(format!(
                        "{what}: triangles ({} vs {} faces)",
                        ma.faces.len(),
                        mb.faces.len()
                    ));
                }
                if skin_weights(bin, ma) != skin_weights(asc, mb) {
                    d.push(format!("{what}: skin weights"));
                }
                match (&ma.extra, &mb.extra) {
                    (MeshExtra::Dangly(x), MeshExtra::Dangly(y)) => {
                        if !close(x.displacement, y.displacement)
                            || !close(x.tightness, y.tightness)
                            || !close(x.period, y.period)
                        {
                            d.push(format!("{what}: dangly settings"));
                        }
                    }
                    (MeshExtra::Aabb(x), MeshExtra::Aabb(y)) if x.len() != y.len() => {
                        d.push(format!("{what}: {} AABB entries vs {}", x.len(), y.len()));
                    }
                    _ => {}
                }
            }
            (NodeKind::Light(x), NodeKind::Light(y)) => {
                if (
                    x.priority,
                    x.ambient_only,
                    x.dynamic_type,
                    x.affect_dynamic,
                    x.shadow,
                    x.fading,
                ) != (
                    y.priority,
                    y.ambient_only,
                    y.dynamic_type,
                    y.affect_dynamic,
                    y.shadow,
                    y.fading,
                ) || x.flare_textures != y.flare_textures
                {
                    d.push(format!("{what}: light {x:?} vs {y:?}"));
                }
            }
            (NodeKind::Emitter(x), NodeKind::Emitter(y)) => {
                let key = |e: &mg_mdl::Emitter| {
                    (
                        e.update.to_ascii_lowercase(),
                        e.render.to_ascii_lowercase(),
                        e.blend.to_ascii_lowercase(),
                        e.texture.clone(),
                        e.xgrid,
                        e.ygrid,
                        e.flags,
                        e.looping,
                        e.render_order,
                    )
                };
                if key(x) != key(y) {
                    d.push(format!("{what}: emitter {:?} vs {:?}", key(x), key(y)));
                }
            }
            (NodeKind::Reference(x), NodeKind::Reference(y)) if x != y => {
                d.push(format!("{what}: reference {x:?} vs {y:?}"));
            }
            _ => {}
        }
    }
    if bin.animations.len() != asc.animations.len() {
        d.push(format!("{} animations vs {}", bin.animations.len(), asc.animations.len()));
        return d;
    }
    for (a, b) in bin.animations.iter().zip(&asc.animations) {
        let what = format!("anim {}", a.name);
        if !a.name.eq_ignore_ascii_case(&b.name)
            || !close(a.length, b.length)
            || !close(a.transtime, b.transtime)
            || !a.animroot.eq_ignore_ascii_case(&b.animroot)
            || a.events.len() != b.events.len()
        {
            d.push(format!("{what}: header"));
            continue;
        }
        for na in &a.nodes {
            let Some(nb) = b.nodes.iter().find(|n| n.name.eq_ignore_ascii_case(&na.name)) else {
                d.push(format!("{what}: node {} missing", na.name));
                continue;
            };
            compare_controllers(
                &format!("{what}/{}", na.name),
                &na.controllers,
                &nb.controllers,
                &mut d,
            );
            let sets = |n: &mg_mdl::AnimNode| n.anim_mesh.as_ref().map(|s| s.vertex_sets.len());
            if sets(na) != sets(nb) {
                d.push(format!("{what}/{}: vertex sets {:?} vs {:?}", na.name, sets(na), sets(nb)));
            }
        }
    }
    d
}

fn decompile(
    tool: &Path,
    root: &Path,
    dir: &Path,
    name: &str,
    data: &[u8],
) -> Result<Vec<u8>, String> {
    let work = dir.join(name);
    std::fs::create_dir_all(&work).unwrap();
    std::fs::write(work.join("model.mdl"), data).unwrap();
    let out = Command::new(tool)
        .args(["-d", "model.mdl"])
        .current_dir(&work)
        .env("NWNDIR", root)
        .output()
        .map_err(|e| e.to_string())?;
    let log = String::from_utf8_lossy(&out.stdout);
    std::fs::read(work.join("model.mdl.ascii"))
        .map_err(|_| format!("no output: {}", log.lines().last().unwrap_or_default()))
}

#[test]
fn models_match_nwnmdlcomp() {
    let root = corpus!();
    let tool = oracle_tool!("nwnmdlcomp");
    let rm = ResMan::for_game(&GameInstall::new(&root, None, "en")).unwrap();
    let dir = scratch_dir("models_nwnmdlcomp");
    let mut keys: Vec<_> =
        rm.entries().into_iter().map(|(k, _)| k).filter(|k| k.restype == ResType::MDL).collect();
    keys.sort();
    let binaries: Vec<_> =
        keys.into_iter().filter(|k| rm.get(k).is_ok_and(|d| mg_mdl::is_binary(&d))).collect();
    // Every 150th, plus models known to exercise skins (EE and BioWare),
    // dangly meshes, emitters, lights, animated meshes and walkmeshes.
    let named = [
        "a_ba",
        "c_golemerald",
        "plc_a01",
        "tcn01_a01_01",
        "t_door09",
        "ashlw_011",
        "pfe0_belt004",
        "c_kocrachn",
        "c_halaster",
        "fx_flame01",
        "tcn01_a15_01",
    ];
    let sample: Vec<_> = binaries
        .iter()
        .enumerate()
        .filter(|(i, k)| i % 150 == 0 || named.contains(&k.resref.to_string().as_str()))
        .map(|(_, k)| *k)
        .collect();
    let results: Vec<(String, Result<Vec<String>, String>)> = sample
        .par_iter()
        .map(|k| {
            let data = rm.get(k).unwrap();
            let bin = Model::read(&data).unwrap();
            let r = decompile(&tool, &root, &dir, &k.resref.to_string(), &data).and_then(|ascii| {
                let asc = Model::read(&ascii).map_err(|e| format!("ASCII: {e}"))?;
                Ok(compare(&bin, &asc))
            });
            (k.to_string(), r)
        })
        .collect();
    let mut bad = 0;
    let mut skipped = 0;
    for (name, r) in &results {
        match r {
            Ok(d) if d.is_empty() => {}
            Ok(d) => {
                bad += 1;
                eprintln!("{name}: {} differences, e.g.", d.len());
                for x in d.iter().take(4) {
                    eprintln!("    {x}");
                }
            }
            Err(e) => {
                skipped += 1;
                eprintln!("{name}: nwnmdlcomp failed: {e}");
            }
        }
    }
    eprintln!("{} models compared, {bad} differ, {skipped} not decompiled", results.len());
    assert!(results.len() > 150);
    assert!(skipped * 10 < results.len(), "nwnmdlcomp failed too often");
    assert_eq!(bad, 0);
}
