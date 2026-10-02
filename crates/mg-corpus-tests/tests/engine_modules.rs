//! Phase 3 exit check: every shipped module, rewritten by Moonglow (every GFF
//! re-serialized, the archive rebuilt), loads in the engine and presents the
//! same world as the original: the same areas and, in each, the same objects
//! with the same types, tags, blueprints and positions.

use std::collections::BTreeMap;
use std::time::Duration;

use mg_core::{ResRef, ResType};
use mg_module::{Module, ModuleLocation};
use mg_resman::ResKey;
use mg_schema::{StructExt, ifo};
use mg_testkit::engine::{ServerRun, run_server, server_binary};

use mg_testkit::{bundled_modules, corpus, oracle_tool, scratch_dir};
use rayon::prelude::*;

mod common;
use common::{PROBE, compile, rewrite_all_gffs, world};

#[test]
fn rewritten_modules_present_the_same_world_in_the_engine() {
    let root = corpus!();
    if server_binary(&root).is_none() {
        eprintln!("skipped: no nwserver for this platform");
        return;
    }
    let _ = oracle_tool!("nwn_script_comp");
    let dir = scratch_dir("engine_modules");
    std::fs::create_dir_all(dir.join("modules")).unwrap();
    let probe = compile(&dir, "mg_probe", PROBE);
    let probe_key = ResKey::new(ResRef::from_str("mg_probe").unwrap(), ResType::NCS);

    let modules = bundled_modules(&root);
    // Build both copies of every module: the original's resources and
    // Moonglow's rewrite, each with the probe as OnModuleLoad.
    let mut jobs = Vec::new();
    for (i, path) in modules.iter().enumerate() {
        let original = Module::open(path).unwrap();
        for (variant, rewrite) in [("a", false), ("b", true)] {
            let mut m = original.clone();
            if rewrite {
                rewrite_all_gffs(&mut m);
            }
            let mut info = m.info().unwrap();
            info.root.write(&ifo::MOD_ON_MOD_LOAD, ResRef::from_str("mg_probe").unwrap());
            m.set_info(&info).unwrap();
            m.set(probe_key, probe.clone());
            let name = format!("m{i}{variant}");
            m.save_as(&ModuleLocation::Archive(dir.join(format!("modules/{name}.mod")))).unwrap();
            jobs.push((path.file_name().unwrap().to_string_lossy().into_owned(), name));
        }
    }

    // Each server gets its own user directory (logs are per directory),
    // sharing the modules folder.
    let pool = rayon::ThreadPoolBuilder::new().num_threads(4).build().unwrap();
    let runs: Vec<(String, String, ServerRun)> = pool.install(|| {
        jobs.par_iter()
            .map(|(label, name)| {
                let user = dir.join(format!("user-{name}"));
                std::fs::create_dir_all(user.join("modules")).unwrap();
                let src = dir.join(format!("modules/{name}.mod"));
                std::fs::rename(&src, user.join(format!("modules/{name}.mod"))).unwrap();
                let run =
                    run_server(&root, &user, name, "MG_DONE", Duration::from_secs(300)).unwrap();
                (label.clone(), name.clone(), run)
            })
            .collect()
    });

    let mut failures = Vec::new();
    let mut objects = 0;
    for pair in runs.chunks(2) {
        let [(label, _, a), (_, _, b)] = pair else { unreachable!() };
        if !a.finished || !b.finished {
            failures.push(format!(
                "{label}: server did not finish (original {}, rewrite {})",
                a.finished, b.finished
            ));
            continue;
        }
        let (wa, wb) = (world(a), world(b));
        objects += wa.iter().filter(|l| l.starts_with("MG_OBJ")).count();
        if wa.iter().filter(|l| l.starts_with("MG_AREA")).count() == 0 {
            failures.push(format!("{label}: the probe saw no areas"));
        }
        if wa != wb {
            let only_a: Vec<_> = wa.iter().filter(|l| !wb.contains(l)).take(3).collect();
            let only_b: Vec<_> = wb.iter().filter(|l| !wa.contains(l)).take(3).collect();
            failures.push(format!(
                "{label}: worlds differ; original only {only_a:?}; rewrite only {only_b:?}"
            ));
        }
    }
    eprintln!("compared {} modules ({objects} objects) in the engine", modules.len());
    assert!(failures.is_empty(), "{} failures:\n{}", failures.len(), failures.join("\n"));
}

/// A module put into a nasher project (GFFs as nasher's JSON, floats rounded
/// to 4 places, `Mod_ID` and area versions dropped) and packed from it
/// presents the same world as the original, but for nasher's rounding: an
/// object may move by a hundredth and turn by a degree (the engine's facing
/// is in whole degrees, and a bearing of -0 reads as 360). The project keeps
/// no compiled scripts, so the original's are added back before packing.
#[test]
fn modules_packed_from_nasher_projects_present_the_same_world() {
    let root = corpus!();
    if server_binary(&root).is_none() {
        eprintln!("skipped: no nwserver for this platform");
        return;
    }
    let _ = oracle_tool!("nwn_script_comp");
    let dir = scratch_dir("engine_nasher");
    let probe = compile(&dir, "mg_probe", PROBE);
    let probe_key = ResKey::new(ResRef::from_str("mg_probe").unwrap(), ResType::NCS);
    let with_probe = |m: &mut Module| {
        let mut info = m.info().unwrap();
        info.root.write(&ifo::MOD_ON_MOD_LOAD, ResRef::from_str("mg_probe").unwrap());
        m.set_info(&info).unwrap();
        m.set(probe_key, probe.clone());
    };
    let modules: Vec<_> = bundled_modules(&root)
        .into_iter()
        .filter(|m| m.extension().is_some_and(|e| e == "mod"))
        .collect();
    let mut jobs = Vec::new();
    for (i, path) in modules.iter().enumerate() {
        let original = Module::open(path).unwrap();
        let label = path.file_name().unwrap().to_string_lossy().into_owned();
        // The original, as it is.
        let mut a = original.clone();
        with_probe(&mut a);
        let a_name = format!("n{i}a");
        let user = dir.join(format!("user-{a_name}"));
        a.save_as(&ModuleLocation::Archive(user.join(format!("modules/{a_name}.mod")))).unwrap();
        jobs.push((label.clone(), a_name, user));
        // Into a project, reopened from it, and packed.
        let project = dir.join(format!("project-{i}"));
        let mut copy = original.clone();
        copy.save_as(&ModuleLocation::Project { root: project.clone(), target: "default".into() })
            .unwrap();
        let mut b = Module::open(&project).unwrap();
        for k in original.keys_of(ResType::NCS) {
            b.set(*k, original.get(k).unwrap().to_vec());
        }
        with_probe(&mut b);
        let (_, bytes) = b.target_archive().unwrap().unwrap();
        let b_name = format!("n{i}b");
        let user = dir.join(format!("user-{b_name}"));
        std::fs::create_dir_all(user.join("modules")).unwrap();
        std::fs::write(user.join(format!("modules/{b_name}.mod")), bytes).unwrap();
        jobs.push((label, b_name, user));
    }
    let pool = rayon::ThreadPoolBuilder::new().num_threads(4).build().unwrap();
    let runs: Vec<(String, ServerRun)> = pool.install(|| {
        jobs.par_iter()
            .map(|(label, name, user)| {
                let run =
                    run_server(&root, user, name, "MG_DONE", Duration::from_secs(300)).unwrap();
                (label.clone(), run)
            })
            .collect()
    });
    let mut failures = Vec::new();
    let mut objects = 0;
    for pair in runs.chunks(2) {
        let [(label, a), (_, b)] = pair else { unreachable!() };
        if !a.finished || !b.finished {
            failures.push(format!(
                "{label}: server did not finish (original {}, project {})",
                a.finished, b.finished
            ));
            continue;
        }
        let (wa, wb) = (world(a), world(b));
        objects += wa.iter().filter(|l| l.starts_with("MG_OBJ")).count();
        let (only_a, only_b) = unmatched(&wa, &wb);
        if !only_a.is_empty() || !only_b.is_empty() {
            failures.push(format!(
                "{label}: worlds differ; original only {:?}; project only {:?}",
                &only_a[..only_a.len().min(3)],
                &only_b[..only_b.len().min(3)]
            ));
        }
    }
    if !failures.is_empty() {
        eprintln!("server logs: {}/user-*/logs.0/nwserverLog1.txt", dir.display());
    }
    eprintln!("compared {} modules ({objects} objects) in the engine", modules.len());
    assert!(failures.is_empty(), "{} failures:\n{}", failures.len(), failures.join("\n"));
}

/// The lines of each world with no partner in the other: objects pair up
/// with an object of the same area, type, tag and blueprint at most a
/// hundredth away and turned at most a degree, other lines with an equal
/// line. Objects are paired by a maximum matching rather than in sorted
/// order, which a hundredth's rounding can reshuffle among alike objects
/// standing close together.
fn unmatched<'a>(a: &'a [String], b: &'a [String]) -> (Vec<&'a str>, Vec<&'a str>) {
    // Per identity (or whole line, for lines that aren't objects): the
    // lines on each side with their numbers.
    type Side<'a> = Vec<(&'a str, [f64; 4])>;
    let mut groups: BTreeMap<String, (Side<'a>, Side<'a>)> = BTreeMap::new();
    for (lines, right) in [(a, false), (b, true)] {
        for l in lines {
            let (key, p) = split_object(l).unwrap_or_else(|| (l.clone(), [0.0; 4]));
            let group = groups.entry(key).or_default();
            if right { &mut group.1 } else { &mut group.0 }.push((l, p));
        }
    }
    let near = |p: &[f64; 4], q: &[f64; 4]| {
        let turn = (p[3] - q[3]).rem_euclid(360.0);
        p[..3].iter().zip(&q[..3]).all(|(u, v)| (u - v).abs() <= 0.011)
            && (turn <= 1.0 || turn >= 359.0)
    };
    let (mut only_a, mut only_b) = (Vec::new(), Vec::new());
    for (left, right) in groups.values() {
        let edges: Vec<Vec<usize>> = left
            .iter()
            .map(|(_, p)| (0..right.len()).filter(|&j| near(p, &right[j].1)).collect())
            .collect();
        // Kuhn's augmenting paths: partner[j] is the left line paired with
        // right line j.
        let mut partner: Vec<Option<usize>> = vec![None; right.len()];
        fn augment(
            i: usize,
            edges: &[Vec<usize>],
            seen: &mut [bool],
            partner: &mut [Option<usize>],
        ) -> bool {
            for &j in &edges[i] {
                if !std::mem::replace(&mut seen[j], true)
                    && partner[j].is_none_or(|k| augment(k, edges, seen, partner))
                {
                    partner[j] = Some(i);
                    return true;
                }
            }
            false
        }
        let mut paired = vec![false; left.len()];
        for (i, done) in paired.iter_mut().enumerate() {
            *done = augment(i, &edges, &mut vec![false; right.len()], &mut partner);
        }
        only_a.extend(left.iter().zip(&paired).filter(|(_, p)| !**p).map(|(l, _)| l.0));
        only_b.extend(right.iter().zip(&partner).filter(|(_, p)| p.is_none()).map(|(l, _)| l.0));
    }
    (only_a, only_b)
}

/// An `MG_OBJ area|type|tag|resref|x,y,z|facing` line: everything but the
/// numbers, and x, y, z and the facing.
fn split_object(line: &str) -> Option<(String, [f64; 4])> {
    let rest = line.strip_prefix("MG_OBJ ")?;
    let parts: Vec<&str> = rest.split('|').collect();
    let [area, kind, tag, resref, pos, facing] = parts[..] else { return None };
    let xyz: Vec<f64> = pos.split(',').filter_map(|v| v.parse().ok()).collect();
    let [x, y, z] = xyz[..] else { return None };
    Some((format!("{area}|{kind}|{tag}|{resref}"), [x, y, z, facing.parse().ok()?]))
}

#[test]
fn worlds_pair_alike_objects_whatever_their_sorted_order() {
    let lines = |v: &[&str]| -> Vec<String> {
        let mut v: Vec<String> = v.iter().map(|l| l.to_string()).collect();
        v.sort();
        v
    };
    // Rounding puts the second urn first in the project's sorted order.
    let a = lines(&[
        "MG_AREA hall|Hall|Hall",
        "MG_OBJ hall|64|Urn|plc_urn|10.00,6.00,0.00|90.0",
        "MG_OBJ hall|64|Urn|plc_urn|10.01,5.00,0.00|0.0",
        "MG_OBJ hall|1|Guard|guard (creature)",
    ]);
    let b = lines(&[
        "MG_AREA hall|Hall|Hall",
        "MG_OBJ hall|64|Urn|plc_urn|10.01,6.00,0.00|89.0",
        "MG_OBJ hall|64|Urn|plc_urn|10.01,5.00,0.00|360.0",
        "MG_OBJ hall|1|Guard|guard (creature)",
    ]);
    assert_eq!(unmatched(&a, &b), (vec![], vec![]));

    // An urn moved, a guard missing, an area renamed.
    let c = lines(&[
        "MG_AREA hall|Hall|Great Hall",
        "MG_OBJ hall|64|Urn|plc_urn|10.00,6.00,0.00|90.0",
        "MG_OBJ hall|64|Urn|plc_urn|10.03,5.00,0.00|0.0",
    ]);
    let (only_a, only_c) = unmatched(&a, &c);
    assert_eq!(
        only_a,
        [
            "MG_AREA hall|Hall|Hall",
            "MG_OBJ hall|1|Guard|guard (creature)",
            "MG_OBJ hall|64|Urn|plc_urn|10.01,5.00,0.00|0.0"
        ]
    );
    assert_eq!(
        only_c,
        ["MG_AREA hall|Hall|Great Hall", "MG_OBJ hall|64|Urn|plc_urn|10.03,5.00,0.00|0.0"]
    );
}

#[test]
#[should_panic(expected = "the probe logged [\"2\"] objects, the server log holds 1")]
fn worlds_refuse_a_log_missing_objects() {
    let log = "[Fri Oct  2 07:20:23] MG_MODULE Hall|HALL\n\
               [Fri Oct  2 07:20:23] MG_AREA hall|Hall|Hall\n\
               [Fri Oct  2 07:20:23] MG_OBJ hall|64|Urn|plc_urn|10.00,6.00,0.00|90.0\n\
               [Fri Oct  2 07:20:23] MG_COUNT 2\n\
               [Fri Oct  2 07:20:23] MG_DONE\n";
    world(&ServerRun { log: log.into(), finished: true });
}
