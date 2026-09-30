//! Verification compared with the engine: for every shipped module, each
//! reference Moonglow reports missing is unknown to the engine
//! (`ResManGetAliasFor` finds none of its candidate types), and a sample of
//! references it considers satisfied resolve in the engine.

use std::collections::{BTreeSet, HashMap};
use std::time::Duration;

use mg_core::{ResRef, ResType};
use mg_module::refs::RefKind;
use mg_module::verify::{missing, module_references};
use mg_module::{Module, ModuleLocation};
use mg_resman::{GameInstall, LayerClass, ResKey, ResMan, priority};
use mg_schema::{StructExt, ifo};
use mg_testkit::engine::{run_server, server_binary};
use mg_testkit::{bundled_modules, corpus, oracle_tool, scratch_dir};
use rayon::prelude::*;

mod common;
use common::compile;

/// Types the engine is asked about for a reference: scripts must be
/// compiled; portraits are not loaded by the headless server.
fn engine_types(kind: RefKind) -> Vec<ResType> {
    match kind {
        RefKind::Script => vec![ResType::NCS],
        RefKind::Portrait | RefKind::Movie => vec![],
        k => k.types().to_vec(),
    }
}

#[test]
fn missing_references_agree_with_engine() {
    let root = corpus!();
    if server_binary(&root).is_none() {
        eprintln!("skipped: no nwserver for this platform");
        return;
    }
    let _ = oracle_tool!("nwn_script_comp");
    let dir = scratch_dir("engine_verify");
    let install = GameInstall::new(&root, None, "en");
    let modules = bundled_modules(&root);

    struct Job {
        label: String,
        name: String,
        // (kind, target, expected to resolve)
        checks: Vec<(RefKind, ResRef, bool)>,
    }
    let mut jobs = Vec::new();
    for (i, path) in modules.iter().enumerate() {
        let m = Module::open(path).unwrap();
        let mut rm = ResMan::for_game(&install).unwrap();
        let haks = m.haks().unwrap();
        rm.add_haks(&install, &haks.iter().map(String::as_str).collect::<Vec<_>>()).unwrap();
        rm.add(priority::MODULE, "module", LayerClass::Erf, m.container());

        let missing_set: BTreeSet<(RefKind, ResRef)> = missing(&m, &rm)
            .into_iter()
            .map(|x| (x.reference.kind, x.reference.target))
            .filter(|(k, _)| !engine_types(*k).is_empty())
            .collect();
        let mut satisfied: Vec<(RefKind, ResRef)> = module_references(&m)
            .into_iter()
            .map(|r| (r.kind, r.target))
            .filter(|x| !engine_types(x.0).is_empty() && !missing_set.contains(x))
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        let step = satisfied.len().div_ceil(150).max(1);
        satisfied = satisfied.into_iter().step_by(step).collect();
        let checks = missing_set
            .into_iter()
            .map(|(k, t)| (k, t, false))
            .chain(satisfied.into_iter().map(|(k, t)| (k, t, true)))
            .collect();

        // The module with a probe script as OnModuleLoad.
        let name = format!("v{i}");
        let mut probe_mod = m.clone();
        let mut info = probe_mod.info().unwrap();
        info.root.write(&ifo::MOD_ON_MOD_LOAD, ResRef::from_str("mg_probe").unwrap());
        probe_mod.set_info(&info).unwrap();
        let user = dir.join(format!("user-{name}"));
        jobs.push(Job {
            label: path.file_name().unwrap().to_string_lossy().into_owned(),
            name,
            checks,
        });
        std::fs::create_dir_all(user.join("modules")).unwrap();
        // The probe is compiled per module below; store the module now.
        let job = jobs.last().unwrap();
        let mut script = String::from("void main()\n{\n");
        for (k, t, _) in &job.checks {
            for ty in engine_types(*k) {
                let t = t.to_lowercase();
                script += &format!(
                    "    WriteTimestampedLogEntry(\"MG_R <{t}.{ty}>=\" + ResManGetAliasFor(\"{t}\", {}));\n",
                    ty.0
                );
            }
        }
        script += "    WriteTimestampedLogEntry(\"MG_DONE\");\n    int i; for (i = 0; i < 2000; i++) WriteTimestampedLogEntry(\"MG_PAD ................................................\");\n}\n";
        let ncs = compile(&dir, &format!("probe{i}"), &script);
        probe_mod.set(ResKey::new(ResRef::from_str("mg_probe").unwrap(), ResType::NCS), ncs);
        probe_mod
            .save_as(&ModuleLocation::Archive(user.join(format!("modules/{}.mod", job.name))))
            .unwrap();
    }

    let pool = rayon::ThreadPoolBuilder::new().num_threads(4).build().unwrap();
    let failures: Vec<String> = pool.install(|| {
        jobs.par_iter()
            .flat_map_iter(|job| {
                let user = dir.join(format!("user-{}", job.name));
                let run = run_server(&root, &user, &job.name, "MG_DONE", Duration::from_secs(300))
                    .unwrap();
                if !run.finished {
                    return vec![format!("{}: server did not finish", job.label)];
                }
                let engine: HashMap<String, String> = run
                    .values("MG_R")
                    .into_iter()
                    .filter_map(|v| {
                        // `<name.type>=alias`; names may have spaces at either end.
                        let (key, alias) = v.strip_prefix('<')?.split_once(">=")?;
                        Some((key.to_string(), alias.to_string()))
                    })
                    .collect();
                let mut fails = Vec::new();
                for (k, t, expect_found) in &job.checks {
                    let aliases: Vec<&str> = engine_types(*k)
                        .iter()
                        .map(|ty| {
                            engine
                                .get(&format!("{}.{ty}", t.to_lowercase()))
                                .map_or("<not logged>", String::as_str)
                        })
                        .collect();
                    let found = aliases.iter().any(|a| !a.is_empty() && *a != "<not logged>");
                    if found != *expect_found {
                        fails.push(format!(
                            "{}: {k:?} {t}: we say {}, engine aliases {aliases:?}",
                            job.label,
                            if *expect_found { "found" } else { "missing" }
                        ));
                    }
                }
                fails
            })
            .collect()
    });
    let total: usize = jobs.iter().map(|j| j.checks.len()).sum();
    let missing_total: usize = jobs.iter().map(|j| j.checks.iter().filter(|c| !c.2).count()).sum();
    eprintln!("checked {total} references ({missing_total} missing) in {} modules", jobs.len());
    assert!(failures.is_empty(), "{} failures:\n{}", failures.len(), failures.join("\n"));
}
