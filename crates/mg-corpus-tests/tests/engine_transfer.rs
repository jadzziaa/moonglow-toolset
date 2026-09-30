//! Export and import end to end: an area exported (with its dependencies)
//! from one shipped module and imported into another presents the same
//! objects in the engine as it does in its original module.

use std::time::Duration;

use mg_core::{ResRef, ResType};
use mg_module::transfer::{export_erf, import_erf, plan_export, plan_import};
use mg_module::{Module, ModuleLocation};
use mg_resman::{GameInstall, LayerClass, ResKey, ResMan, priority};
use mg_schema::{StructExt, ifo};
use mg_testkit::engine::{run_server, server_binary};
use mg_testkit::{corpus, oracle_tool, scratch_dir};

mod common;
use common::{PROBE, compile, world};

fn with_probe(m: &mut Module, probe: &[u8]) {
    let mut info = m.info().unwrap();
    info.root.write(&ifo::MOD_ON_MOD_LOAD, ResRef::from_str("mg_probe").unwrap());
    m.set_info(&info).unwrap();
    m.set(ResKey::new(ResRef::from_str("mg_probe").unwrap(), ResType::NCS), probe.to_vec());
}

#[test]
fn exported_area_imports_into_another_module_intact() {
    let root = corpus!();
    if server_binary(&root).is_none() {
        eprintln!("skipped: no nwserver for this platform");
        return;
    }
    let _ = oracle_tool!("nwn_script_comp");
    let dir = scratch_dir("engine_transfer");
    let install = GameInstall::new(&root, None, "en");
    let probe = compile(&dir, "mg_probe", PROBE);

    let source = Module::open(&root.join("data/nwm/Prelude.nwm")).unwrap();
    let area = source.areas().unwrap()[0];
    let mut rm = ResMan::for_game(&install).unwrap();
    rm.add(priority::MODULE, "module", LayerClass::Erf, source.container());
    let plan = plan_export(&source, &[ResKey::new(area, ResType::ARE)], &rm);
    assert!(plan.resources.len() > 3, "an area and its dependencies: {:?}", plan.resources);
    let erf = export_erf(&source, &plan.resources, "exported by Moonglow", true).unwrap();

    let mut dest = Module::open(&root.join("data/mod/Neverwinter Chess.mod")).unwrap();
    let base = ResMan::for_game(&install).unwrap();
    let iplan = plan_import(&dest, &erf, &base).unwrap();
    let summary = import_erf(&mut dest, &erf, |_| false).unwrap();
    assert_eq!(summary.new_areas, [area]);
    eprintln!(
        "exported {} resources ({} missing deps), imported {} new, {} skipped of {} overwrite candidates",
        plan.resources.len(),
        plan.missing.len(),
        summary.added.len(),
        summary.skipped.len(),
        iplan.overwrites.len()
    );

    let mut runs = Vec::new();
    for (name, mut m) in [("src", source.clone()), ("dst", dest)] {
        with_probe(&mut m, &probe);
        let user = dir.join(format!("user-{name}"));
        m.save_as(&ModuleLocation::Archive(user.join(format!("modules/{name}.mod")))).unwrap();
        let run = run_server(&root, &user, name, "MG_DONE", Duration::from_secs(300)).unwrap();
        assert!(run.finished, "{name}: server did not finish");
        runs.push(run);
    }
    let in_area = |w: Vec<String>| -> Vec<String> {
        let tag = format!("MG_OBJ {}|", area.to_lowercase());
        w.into_iter().filter(|l| l.starts_with(&tag)).collect()
    };
    let (a, b) = (in_area(world(&runs[0])), in_area(world(&runs[1])));
    assert!(!a.is_empty(), "the source area has no objects");
    assert_eq!(a, b, "the imported area differs from the original");
    eprintln!("area {area}: {} objects identical after export and import", a.len());
}
