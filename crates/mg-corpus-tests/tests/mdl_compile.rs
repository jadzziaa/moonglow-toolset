//! The binary writer against the game's models: every compiled model, read
//! and written again, reads back the same; and what the writer derives
//! (planes, neighbours, bounds, part numbers) is what the game's files
//! hold.

#[path = "mdl_common/mod.rs"]
mod common;

use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use mg_core::{ResRef, ResType};
use mg_mdl::Model;
use mg_mdl::binary_write::{PartNumbers, part_numbers, write};
use mg_resman::{GameInstall, ResKey, ResMan};
use rayon::prelude::*;

/// A model's part numbers, with its supermodels' (read from the game).
fn numbered(
    rm: &ResMan,
    model: &Model,
    cache: &Mutex<HashMap<String, Option<(Model, PartNumbers)>>>,
    depth: usize,
) -> PartNumbers {
    let sup = model.supermodel.as_deref().filter(|_| depth < 16).and_then(|name| {
        if let Some(hit) = cache.lock().unwrap().get(name) {
            return hit.clone();
        }
        let loaded = ResRef::from_str(name)
            .ok()
            .and_then(|r| rm.get(&ResKey::new(r, ResType::MDL)).ok())
            .and_then(|d| Some((Model::read(&d).ok()?, d)))
            .map(|(m, d)| {
                // A compiled supermodel's own numbers; an ASCII one's as
                // a compiler would give them.
                let parts =
                    PartNumbers::read(&d).unwrap_or_else(|| numbered(rm, &m, cache, depth + 1));
                (m, parts)
            });
        cache.lock().unwrap().insert(name.to_string(), loaded.clone());
        loaded
    });
    part_numbers(model, sup.as_ref().map(|(m, p)| (m, p)))
}

fn u32_at(d: &[u8], at: usize) -> u32 {
    d.get(at..at + 4).map_or(0, |b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}
fn f32_at(d: &[u8], at: usize) -> f32 {
    f32::from_bits(u32_at(d, at))
}
fn i16_at(d: &[u8], at: usize) -> i16 {
    d.get(at..at + 2).map_or(0, |b| i16::from_le_bytes([b[0], b[1]]))
}

/// The offsets of a compiled model's nodes (from byte 12), in pre-order.
fn node_offsets(d: &[u8]) -> Vec<usize> {
    let m = &d[12..];
    let mut out = Vec::new();
    let mut stack = vec![u32_at(m, 0x48) as usize];
    while let Some(at) = stack.pop() {
        if out.len() > 100_000 {
            break;
        }
        out.push(at);
        let (list, n) = (u32_at(m, at + 0x48) as usize, u32_at(m, at + 0x4C) as usize);
        for i in (0..n).rev() {
            stack.push(u32_at(m, list + 4 * i) as usize);
        }
    }
    out
}

#[derive(Default)]
struct Tally {
    same: AtomicUsize,
    all: AtomicUsize,
}

impl Tally {
    fn add(&self, same: bool) {
        self.all.fetch_add(1, Ordering::Relaxed);
        if same {
            self.same.fetch_add(1, Ordering::Relaxed);
        }
    }
    fn share(&self) -> f64 {
        let all = self.all.load(Ordering::Relaxed);
        if all == 0 { 1.0 } else { self.same.load(Ordering::Relaxed) as f64 / all as f64 }
    }
    fn text(&self) -> String {
        format!(
            "{} of {} ({:.2}%)",
            self.same.load(Ordering::Relaxed),
            self.all.load(Ordering::Relaxed),
            self.share() * 100.0
        )
    }
}

fn near(a: f32, b: f32, eps: f32) -> bool {
    (a - b).abs() <= eps * (1.0 + a.abs().max(b.abs()))
}

#[test]
fn every_binary_model_writes_back_the_same() {
    let root = mg_testkit::corpus!();
    let rm = ResMan::for_game(&GameInstall::new(root, None, "en")).unwrap();
    let names = rm.list(ResType::MDL);
    let cache = Mutex::new(HashMap::new());
    let written = AtomicUsize::new(0);
    // What the writer derives, against the files BioWare's compiler and
    // nwnmdlcomp wrote (the game's own leaves garbage in places).
    let (parts, counts) = (Tally::default(), Tally::default());
    let (normals, distances, neighbours) = (Tally::default(), Tally::default(), Tally::default());
    let (boxes, radii, averages) = (Tally::default(), Tally::default(), Tally::default());
    let mut failures: Vec<(String, String)> = names
        .par_iter()
        .filter_map(|name| {
            let data = rm.get(&ResKey::new(*name, ResType::MDL)).ok()?;
            if !mg_mdl::is_binary(&data) {
                return None;
            }
            let model = Model::read(&data).ok()?;
            let numbers = numbered(&rm, &model, &cache, 0);
            let bytes = match write(&model, &numbers) {
                Ok(b) => b,
                Err(e) => return Some((name.to_string(), format!("not written: {e}"))),
            };
            written.fetch_add(1, Ordering::Relaxed);
            let back = match Model::read(&bytes) {
                Ok(m) => m,
                Err(e) => return Some((name.to_string(), format!("unreadable: {e}"))),
            };
            // (Without the part numbers the reader keeps: those written
            // are made anew here, and tallied against the file's below.)
            let bare = |m: &Model| {
                let mut m = m.clone();
                m.nodes.iter_mut().for_each(|n| n.part = None);
                m.animations.iter_mut().flat_map(|a| &mut a.nodes).for_each(|n| n.part = None);
                m
            };
            let (back, model) = (bare(&back), bare(&model));
            // (Compared as text: a NaN is not equal to itself.)
            if back != model && format!("{back:?}") != format!("{model:?}") {
                let node = model
                    .nodes
                    .iter()
                    .zip(&back.nodes)
                    .find(|(a, b)| format!("{a:?}") != format!("{b:?}"))
                    .map(|(a, _)| format!("node {} ({})", a.name, a.kind.type_name()));
                let anim = model
                    .animations
                    .iter()
                    .zip(&back.animations)
                    .find(|(a, b)| format!("{a:?}") != format!("{b:?}"))
                    .map(|(a, _)| format!("animation {}", a.name));
                let what = node.or(anim).unwrap_or_else(|| "the header".into());
                return Some((name.to_string(), format!("differs: {what}")));
            }
            // Derived values against the original's, where a compiler that
            // wrote them wrote the file.
            let routine = u32_at(&data, 12);
            if routine != 0x0040_BBC0 && routine != 0x0046_AB0C {
                return None;
            }
            let (m, mine) = (&data[12..], &bytes[12..]);
            let (theirs, ours) = (node_offsets(&data), node_offsets(&bytes));
            if theirs.len() != ours.len() {
                return None;
            }
            counts.add(u32_at(m, 0x4C) == u32_at(mine, 0x4C));
            for (i, (&a, &b)) in theirs.iter().zip(&ours).enumerate() {
                parts.add(u32_at(m, a + 0x1C) == u32_at(mine, b + 0x1C));
                let Some(mesh) = model.nodes[i].mesh() else { continue };
                if mesh.vertices.is_empty() {
                    continue;
                }
                let v3 =
                    |d: &[u8], at: usize| [f32_at(d, at), f32_at(d, at + 4), f32_at(d, at + 8)];
                let same3 =
                    |x: [f32; 3], y: [f32; 3], eps: f32| (0..3).all(|k| near(x[k], y[k], eps));
                boxes.add(
                    same3(v3(m, a + 0x84), v3(mine, b + 0x84), 1e-4)
                        && same3(v3(m, a + 0x90), v3(mine, b + 0x90), 1e-4),
                );
                radii.add(near(f32_at(m, a + 0x9C), f32_at(mine, b + 0x9C), 1e-3));
                averages.add(same3(v3(m, a + 0xA0), v3(mine, b + 0xA0), 1e-3));
                let (fa, fb) = (u32_at(m, a + 0x78) as usize, u32_at(mine, b + 0x78) as usize);
                for f in 0..mesh.faces.len() {
                    let (x, y) = (fa + f * 32, fb + f * 32);
                    normals.add(same3(v3(m, x), v3(mine, y), 1e-3));
                    distances.add(near(f32_at(m, x + 12), f32_at(mine, y + 12), 1e-3));
                    for e in 0..3 {
                        neighbours
                            .add(i16_at(m, x + 0x14 + 2 * e) == i16_at(mine, y + 0x14 + 2 * e));
                    }
                }
            }
            None
        })
        .collect();
    failures.sort();
    eprintln!(
        "{} models written; as the originals: part numbers {}, node counts {}, face normals {}, \
         plane distances {}, neighbours {}, boxes {}, radii {}, averages {}",
        written.load(Ordering::Relaxed),
        parts.text(),
        counts.text(),
        normals.text(),
        distances.text(),
        neighbours.text(),
        boxes.text(),
        radii.text(),
        averages.text()
    );
    for (name, what) in failures.iter().take(40) {
        eprintln!("  {name}: {what}");
    }
    assert!(failures.is_empty(), "{} models differ", failures.len());
    assert!(written.load(Ordering::Relaxed) > 25_000);
    // Part numbers and node counts fall short where the original's nodes
    // were numbered in their file's order (a compiled model's tree does
    // not keep it): then they are a permutation of these.
    for (what, tally, least) in [
        ("part numbers", &parts, 0.79),
        ("node counts", &counts, 0.98),
        ("face normals", &normals, 0.998),
        ("plane distances", &distances, 0.998),
        ("neighbours", &neighbours, 0.99),
        ("boxes", &boxes, 0.995),
        ("radii", &radii, 0.94),
        ("averages", &averages, 0.94),
    ] {
        assert!(tally.share() >= least, "{what}: {}", tally.text());
    }
}

/// A compiled supermodel and its part numbers, read from the game.
fn supermodel_of(rm: &ResMan, model: &Model) -> Option<(Model, PartNumbers)> {
    let name = model.supermodel.as_deref()?;
    let data = rm.get(&ResKey::new(ResRef::from_str(name).ok()?, ResType::MDL)).ok()?;
    Some((Model::read(&data).ok()?, PartNumbers::read(&data)?))
}

/// The native compiler on the game's models (the plan's Phase 9, stage B):
/// every compiled model, decompiled and compiled again, is the model it was, with the part numbers it had.
#[test]
fn decompiled_models_compile_back() {
    let root = mg_testkit::corpus!();
    let rm = ResMan::for_game(&GameInstall::new(root, None, "en")).unwrap();
    let names = rm.list(ResType::MDL);
    let (compiled, skins) = (AtomicUsize::new(0), AtomicUsize::new(0));
    // Numbers kept from the compiled model, and numbered afresh from the
    // text alone (where the model's numbers are its own).
    let (kept, fresh, binds) = (Tally::default(), Tally::default(), Tally::default());
    let odd_binds = Mutex::new(std::collections::BTreeSet::new());
    let mut failures: Vec<(String, Vec<String>)> = names
        .par_iter()
        .filter_map(|name| {
            let data = rm.get(&ResKey::new(*name, ResType::MDL)).ok()?;
            let model = Model::read(&data).ok().filter(|_| mg_mdl::is_binary(&data))?;
            if model.nodes.iter().any(|n| n.kind.type_name() == "skin") {
                skins.fetch_add(1, Ordering::Relaxed);
            }
            // Two nodes of one name cannot be told apart in text.
            if common::duplicate_names(&model) {
                return None;
            }
            let text = mg_mdl::decompile(&data).ok()?;
            let numbers = PartNumbers::read(&data)?;
            let sup = supermodel_of(&rm, &model);
            let sources = mg_mdl::compile::Sources {
                supermodel: sup.as_ref().map(|(m, p)| (m, p)),
                existing: Some((&model, &numbers)),
            };
            let out = match mg_mdl::compile::compile(text.as_bytes(), &sources) {
                Ok(c) => c,
                Err(e) => return Some((name.to_string(), vec![format!("not compiled: {e}")])),
            };
            compiled.fetch_add(1, Ordering::Relaxed);
            let back = match Model::read(&out.binary) {
                Ok(m) => m,
                Err(e) => return Some((name.to_string(), vec![format!("unreadable: {e}")])),
            };
            let mut problems = common::differences(&model, &back);
            let name_key = name;
            let name_of = |n: &ResRef| n.to_string();
            // A skin's inverse binds, made from the model at rest, against
            // those its file has: where each bone sees a point of the skin.
            for (i, node) in model.nodes.iter().enumerate() {
                let Some(mg_mdl::MeshExtra::Skin(theirs)) = node.mesh().map(|m| &m.extra) else {
                    continue;
                };
                let ours = back.node(&node.name).and_then(|j| match back.nodes[j].mesh()?.extra {
                    mg_mdl::MeshExtra::Skin(ref s) => Some(s),
                    _ => None,
                });
                let Some(ours) = ours else {
                    problems.push(format!("{}: no skin", node.name));
                    continue;
                };
                // (Compiled from the text, the model's own are not kept:
                // compare with a compile that had to make them.)
                let _ = i;
                for &bone in &theirs.bones {
                    let name = &model.nodes[bone].name;
                    let (Some(a), Some(b)) = (
                        theirs.inverse_bind.get(bone),
                        back.node(name).and_then(|j| ours.inverse_bind.get(j)),
                    ) else {
                        continue;
                    };
                    let seen = |(q, t): &([f32; 4], [f32; 3])| {
                        let len = q.iter().map(|x| x * x).sum::<f32>().sqrt().max(1e-12);
                        let [x, y, z, w] = q.map(|v| v / len);
                        let p = [0.3f32, -0.7, 1.1];
                        // q p q*
                        let u = [x, y, z];
                        let d = u[0] * p[0] + u[1] * p[1] + u[2] * p[2];
                        let c = [
                            u[1] * p[2] - u[2] * p[1],
                            u[2] * p[0] - u[0] * p[2],
                            u[0] * p[1] - u[1] * p[0],
                        ];
                        let uu = x * x + y * y + z * z;
                        std::array::from_fn::<f32, 3, _>(|k| {
                            2.0 * d * u[k] + (w * w - uu) * p[k] + 2.0 * w * c[k] + t[k]
                        })
                    };
                    let (pa, pb) = (seen(a), seen(b));
                    let off = (0..3).map(|k| (pa[k] - pb[k]).powi(2)).sum::<f32>().sqrt();
                    binds.add(off < 2e-3);
                    if off >= 2e-3 {
                        odd_binds.lock().unwrap().insert(name_of(name_key));
                    }
                }
            }
            let by_name = |m: &Model, p: &PartNumbers| {
                let mut v: Vec<(String, i32)> = m
                    .nodes
                    .iter()
                    .zip(&p.numbers)
                    .map(|(n, k)| (n.name.to_lowercase(), *k))
                    .collect();
                v.sort();
                v
            };
            let theirs = by_name(&model, &numbers);
            let ours = PartNumbers::read(&out.binary).unwrap();
            let same = by_name(&back, &ours) == theirs && ours.count == numbers.count;
            kept.add(same);
            if !same {
                problems.push("part numbers changed".into());
            }
            // From the text alone.
            let mut sorted = numbers.numbers.clone();
            sorted.sort_unstable();
            // (A skin is written after its bones, whatever its number.)
            let skinned = model.nodes.iter().any(|n| n.kind.type_name() == "skin");
            if model.supermodel.is_none()
                && !skinned
                && sorted.iter().enumerate().all(|(i, n)| *n == i as i32)
            {
                let alone = mg_mdl::compile::compile(text.as_bytes(), &Default::default()).ok()?;
                let parts = PartNumbers::read(&alone.binary).unwrap();
                fresh.add(by_name(&Model::read(&alone.binary).ok()?, &parts) == theirs);
            }
            (!problems.is_empty()).then(|| (name.to_string(), problems))
        })
        .collect();
    failures.sort();
    eprintln!(
        "{} models compiled ({} with skins), {} differ; part numbers kept from the compiled \
         model: {}; numbered again from the text alone: {}; skins' inverse binds as their \
         files': {}",
        compiled.load(Ordering::Relaxed),
        skins.load(Ordering::Relaxed),
        failures.len(),
        kept.text(),
        fresh.text(),
        binds.text()
    );
    for (name, d) in failures.iter().take(25) {
        eprintln!("  {name}: {}", d.iter().take(4).cloned().collect::<Vec<_>>().join("; "));
    }
    eprintln!(
        "skins bound in another pose than their model's rest: {:?}",
        odd_binds.lock().unwrap()
    );
    assert!(compiled.load(Ordering::Relaxed) > 25_000);
    assert!(skins.load(Ordering::Relaxed) > 300);
    assert!(failures.is_empty(), "{} models differ", failures.len());
    assert!(fresh.share() >= 1.0, "{}", fresh.text());
    // A few skins were bound in another pose than their model rests in,
    // which no text holds: every compiler binds those at rest.
    assert!(binds.share() >= 0.98, "{}", binds.text());
}

/// The game's own ASCII models (a fifth of its models are text): each
/// compiles natively to the model its text reads as.
#[test]
fn the_games_ascii_models_compile() {
    let root = mg_testkit::corpus!();
    let rm = ResMan::for_game(&GameInstall::new(root, None, "en")).unwrap();
    let names = rm.list(ResType::MDL);
    let compiled = AtomicUsize::new(0);
    let start = std::time::Instant::now();
    let lookup = |n: &str| {
        let key = ResKey::parse(n, ResType::MDL)?;
        rm.get(&key).ok().map(|d| d.into_owned())
    };
    let mut failures: Vec<(String, Vec<String>)> = names
        .par_iter()
        .filter_map(|name| {
            let data = rm.get(&ResKey::new(*name, ResType::MDL)).ok()?;
            if mg_mdl::is_binary(&data) {
                return None;
            }
            // (What the reader cannot read is not a model to compile.)
            let model = Model::read(&data).ok()?;
            let none = |_: &str| None;
            let out = match mg_mdl::compile::compile_named(&data, &name.to_string(), &lookup, &none)
            {
                Ok(c) => c,
                Err(e) => return Some((name.to_string(), vec![format!("not compiled: {e}")])),
            };
            compiled.fetch_add(1, Ordering::Relaxed);
            let back = match Model::read(&out.binary) {
                Ok(m) => m,
                Err(e) => return Some((name.to_string(), vec![format!("unreadable: {e}")])),
            };
            if common::duplicate_names(&model) {
                return None;
            }
            let mut problems = common::compare(&back, &model, true);
            // A shininess of 1 where the text has none is the compiler's
            // (the game's); animated sets are compared below.
            problems.retain(|p| {
                !p.ends_with(": shininess")
                    && !p.ends_with(": animvert")
                    && !p.ends_with(": animtvert")
            });
            // Animated sets, directly: the compiled model's are the text's,
            // vertex for vertex (the reader's render vertices are the
            // compiler's), where the text gave them per render vertex.
            for (a, b) in model.animations.iter().zip(&back.animations) {
                for (n, m) in a.nodes.iter().zip(&b.nodes) {
                    let count = model
                        .node(&n.name)
                        .and_then(|i| model.nodes[i].mesh())
                        .map_or(0, |mesh| mesh.vertices.len());
                    let (Some(x), Some(y)) = (&n.anim_mesh, &m.anim_mesh) else {
                        if n.anim_mesh.is_some() != m.anim_mesh.is_some() {
                            problems.push(format!("{}/{}: animated sets", a.name, n.name));
                        }
                        continue;
                    };
                    let whole = x.vertex_sets.iter().all(|v| v.len() == count)
                        && x.uv_sets.iter().all(|v| v.len() == count);
                    if whole && (x.vertex_sets != y.vertex_sets || x.uv_sets != y.uv_sets) {
                        problems.push(format!("{}/{}: animated sets", a.name, n.name));
                    }
                }
            }
            (!problems.is_empty()).then(|| (name.to_string(), problems))
        })
        .collect();
    failures.sort();
    eprintln!(
        "{} ASCII models compiled in {:.1} s, {} differ",
        compiled.load(Ordering::Relaxed),
        start.elapsed().as_secs_f32(),
        failures.len()
    );
    let mut kinds: HashMap<String, usize> = HashMap::new();
    for (_, d) in &failures {
        let kind = d[0].split(':').next_back().unwrap_or("").trim().to_string();
        *kinds.entry(kind).or_default() += 1;
    }
    eprintln!("{kinds:?}");
    for (name, d) in failures.iter().take(25) {
        eprintln!("  {name}: {}", d.iter().take(3).cloned().collect::<Vec<_>>().join("; "));
    }
    assert!(compiled.load(Ordering::Relaxed) > 7_000);
    assert!(failures.is_empty(), "{} models differ", failures.len());
}
