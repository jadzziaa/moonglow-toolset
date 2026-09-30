//! Phase 3 exit check: every shipped module, rewritten by Moonglow (every GFF
//! re-serialized, the archive rebuilt), loads in the engine and presents the
//! same world as the original: the same areas and, in each, the same objects
//! with the same types, tags, blueprints and positions.

use std::time::Duration;

use mg_core::{ResRef, ResType};
use mg_module::{Module, ModuleLocation};
use mg_resman::ResKey;
use mg_schema::{StructExt, ifo};
use mg_testkit::engine::{ServerRun, run_server, server_binary};
use mg_testkit::{bundled_modules, corpus, oracle_tool, scratch_dir};
use rayon::prelude::*;

mod common;
use common::{compile, rewrite_all_gffs};

/// Walks every area and object during module load, before any heartbeat or
/// AI has run, yielding with `DelayCommand` only when the instruction budget
/// runs low (then resuming at the same area and object). Ends with the done
/// marker and padding that makes the server flush its buffered log.
const PROBE: &str = r#"
void Log(string s) { WriteTimestampedLogEntry(s); }

void Finish()
{
    Log("MG_DONE");
    int i;
    for (i = 0; i < 2000; i++)
        Log("MG_PAD ................................................................");
}

void ProbeFrom(int nArea, int nObj)
{
    int a = 0;
    object oArea = GetFirstArea();
    while (GetIsObjectValid(oArea) && a < nArea) { oArea = GetNextArea(); a++; }
    while (GetIsObjectValid(oArea))
    {
        string sArea = GetResRef(oArea);
        if (nObj == 0)
            Log("MG_AREA " + sArea + "|" + GetTag(oArea) + "|" + GetName(oArea));
        int i = 0;
        object o = GetFirstObjectInArea(oArea);
        while (GetIsObjectValid(o))
        {
            if (i >= nObj)
            {
                if (GetScriptInstructionsRemaining() < 20000)
                {
                    DelayCommand(0.0, ProbeFrom(a, i));
                    return;
                }
                vector v = GetPosition(o);
                Log("MG_OBJ " + sArea + "|" + IntToString(GetObjectType(o)) + "|" + GetTag(o)
                    + "|" + GetResRef(o) + "|" + FloatToString(v.x, 0, 2) + "," + FloatToString(v.y, 0, 2)
                    + "," + FloatToString(v.z, 0, 2) + "|" + FloatToString(GetFacing(o), 0, 1));
            }
            i++;
            o = GetNextObjectInArea(oArea);
        }
        nObj = 0;
        a++;
        oArea = GetNextArea();
    }
    DelayCommand(0.0, Finish());
}

void main()
{
    Log("MG_MODULE " + GetName(GetModule()) + "|" + GetTag(GetModule()));
    ProbeFrom(0, 0);
}
"#;

/// The probe's view of the world, sorted. Creatures keep only their area,
/// type, tag and blueprint: their AI may move them between probe chunks.
fn world(run: &ServerRun) -> Vec<String> {
    let mut lines: Vec<String> = ["MG_MODULE", "MG_AREA", "MG_OBJ"]
        .iter()
        .flat_map(|tag| run.values(tag).into_iter().map(move |v| format!("{tag} {v}")))
        .map(|l| match l.strip_prefix("MG_OBJ ") {
            Some(obj) if obj.split('|').nth(1) == Some("1") => {
                let identity: Vec<&str> = obj.split('|').take(4).collect();
                format!("MG_OBJ {} (creature)", identity.join("|"))
            }
            _ => l,
        })
        .collect();
    lines.sort();
    lines
}

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
