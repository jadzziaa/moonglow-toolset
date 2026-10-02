//! Rename everywhere, checked in the engine: in each shipped module, the
//! start area, a placed object's blueprint and the OnModuleLoad script (the
//! probe itself) are renamed; the module must load, run the renamed script,
//! and present the same world under the new names.

use std::time::Duration;

use mg_core::{ResRef, ResType};
use mg_module::rename::rename;
use mg_module::{Module, ModuleLocation};
use mg_resman::ResKey;
use mg_schema::{StructExt, ifo};
use mg_testkit::engine::{ServerRun, run_server, server_binary};
use mg_testkit::{bundled_modules, corpus, oracle_tool, scratch_dir};
use rayon::prelude::*;

mod common;
use common::{PROBE, compile, world};

/// The first placed object (not a creature: they may wander) whose blueprint
/// is in the module.
fn placed_blueprint(m: &Module, area: ResRef) -> Option<ResKey> {
    let git = m.gff(&ResKey::new(area, ResType::GIT))?.ok()?;
    for (list, t) in [("Placeable List", ResType::UTP), ("WaypointList", ResType::UTW)] {
        if let Some(mg_gff::Value::List(items)) = git.root.get(list) {
            for item in items {
                if let Some(r) = item.get("TemplateResRef").and_then(mg_gff::Value::as_resref) {
                    let k = ResKey::new(r, t);
                    if m.contains(&k) {
                        return Some(k);
                    }
                }
            }
        }
    }
    None
}

#[test]
fn renamed_resources_present_the_same_world_in_the_engine() {
    let root = corpus!();
    if server_binary(&root).is_none() {
        eprintln!("skipped: no nwserver for this platform");
        return;
    }
    let _ = oracle_tool!("nwn_script_comp");
    let dir = scratch_dir("engine_rename");
    let probe = compile(&dir, "mg_probe", PROBE);
    let probe_key = ResKey::new(ResRef::from_str("mg_probe").unwrap(), ResType::NCS);
    let modules: Vec<_> = bundled_modules(&root)
        .into_iter()
        .filter(|m| m.extension().is_some_and(|e| e == "mod"))
        .collect();

    // (label, renames as (old, new) names, the two module names).
    let mut cases = Vec::new();
    let mut jobs = Vec::new();
    for (i, path) in modules.iter().enumerate() {
        let mut a = Module::open(path).unwrap();
        let mut info = a.info().unwrap();
        info.root.write(&ifo::MOD_ON_MOD_LOAD, ResRef::from_str("mg_probe").unwrap());
        a.set_info(&info).unwrap();
        a.set(probe_key, probe.clone());
        let entry = a.info().unwrap().root.read(&ifo::MOD_ENTRY_AREA);
        let mut b = a.clone();
        let mut renames = Vec::new();
        let new = |s: &str| ResRef::from_str(s).unwrap();
        if b.contains(&ResKey::new(entry, ResType::ARE)) {
            rename(&mut b, ResKey::new(entry, ResType::ARE), new("mg_area_r"), false).unwrap();
            renames.push((entry.to_string(), "mg_area_r".to_string()));
            if let Some(bp) = placed_blueprint(&b, new("mg_area_r")) {
                rename(&mut b, bp, new("mg_bp_r"), false).unwrap();
                renames.push((bp.resref.to_string(), "mg_bp_r".to_string()));
            }
        }
        rename(&mut b, probe_key, new("mg_probe_r"), false).unwrap();
        assert_eq!(
            b.info().unwrap().root.read(&ifo::MOD_ON_MOD_LOAD),
            new("mg_probe_r"),
            "the module runs the renamed script"
        );
        for (m, variant) in [(a, "a"), (b, "b")] {
            let name = format!("r{i}{variant}");
            let user = dir.join(format!("user-{name}"));
            let mut m = m;
            m.save_as(&ModuleLocation::Archive(user.join(format!("modules/{name}.mod")))).unwrap();
            jobs.push((name, user));
        }
        cases.push((path.file_name().unwrap().to_string_lossy().into_owned(), renames));
    }

    let pool = rayon::ThreadPoolBuilder::new().num_threads(4).build().unwrap();
    let runs: Vec<ServerRun> = pool.install(|| {
        jobs.par_iter()
            .map(|(name, user)| {
                run_server(&root, user, name, "MG_DONE", Duration::from_secs(300)).unwrap()
            })
            .collect()
    });

    let mut failures = Vec::new();
    let mut renamed = 0;
    for ((label, renames), pair) in cases.iter().zip(runs.chunks(2)) {
        let [a, b] = pair else { unreachable!() };
        if !a.finished || !b.finished {
            failures.push(format!(
                "{label}: server did not finish (original {}, renamed {})",
                a.finished, b.finished
            ));
            continue;
        }
        // The original's world under the new names: area resrefs (field 1
        // of MG_AREA, field 1 of MG_OBJ) and blueprints (field 4 of MG_OBJ).
        let rename_field = |v: &str| {
            renames
                .iter()
                .find(|(old, _)| old.eq_ignore_ascii_case(v))
                .map_or_else(|| v.to_string(), |(_, new)| new.clone())
        };
        let mut want: Vec<String> = world(a)
            .into_iter()
            .map(|l| {
                let (tag, rest) = l.split_once(' ').unwrap();
                let mut fields: Vec<String> = rest.split('|').map(str::to_string).collect();
                match tag {
                    "MG_AREA" => fields[0] = rename_field(&fields[0]),
                    "MG_OBJ" => {
                        fields[0] = rename_field(&fields[0]);
                        if fields.len() > 3 {
                            fields[3] = rename_field(&fields[3]);
                        }
                    }
                    _ => {}
                }
                format!("{tag} {}", fields.join("|"))
            })
            .collect();
        want.sort();
        let got = world(b);
        renamed += renames.len() + 1;
        if want != got {
            let only_a: Vec<_> = want.iter().filter(|l| !got.contains(l)).take(3).collect();
            let only_b: Vec<_> = got.iter().filter(|l| !want.contains(l)).take(3).collect();
            failures.push(format!("{label}: expected only {only_a:?}; renamed only {only_b:?}"));
        }
    }
    eprintln!("{renamed} renames in {} modules checked in the engine", cases.len());
    assert!(failures.is_empty(), "{} failures:\n{}", failures.len(), failures.join("\n"));
}
