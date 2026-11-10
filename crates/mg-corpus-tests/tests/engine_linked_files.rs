//! A file linked into the user folder's `development/` (and `override/`)
//! rather than copied there: whether the game reads it, and that Moonglow
//! lists it as the game does (the review's Z3).

use std::time::Duration;

use mg_core::{ResRef, ResType};
use mg_resman::{GameInstall, ResKey, ResMan};
use mg_testkit::engine::{run_server, server_binary};
use mg_testkit::{corpus, oracle_tool, scratch_dir};

use crate::common::probe_module;

/// A table linked into `development/` and one into `override/`, each from
/// a folder outside the user directory: the engine finds both there
/// (`ResManGetAliasFor`) and reads them (`Get2DAString`), and Moonglow's
/// load order has them where the engine does.
#[cfg(unix)]
#[test]
fn the_engine_reads_a_file_linked_into_development_and_override() {
    let root = corpus!();
    if server_binary(&root).is_none() {
        eprintln!("skipped: no nwserver for this platform");
        return;
    }
    let _ = oracle_tool!("nwn_script_comp");
    let dir = scratch_dir("engine_linked_files");
    let outside = dir.join("project-build");
    std::fs::create_dir_all(&outside).unwrap();
    let table = |label: &str| format!("2DA V2.0\n\n LABEL\n0 {label}\n").into_bytes();
    std::fs::write(outside.join("mg_linked_dev.2da"), table("from_dev_link")).unwrap();
    std::fs::write(outside.join("mg_linked_ovr.2da"), table("from_override_link")).unwrap();
    for (folder, name) in [("development", "mg_linked_dev"), ("override", "mg_linked_ovr")] {
        std::fs::create_dir_all(dir.join(folder)).unwrap();
        let link = dir.join(folder).join(format!("{name}.2da"));
        std::os::unix::fs::symlink(outside.join(format!("{name}.2da")), link).unwrap();
    }
    let module = "mg_linked";
    let mut script = String::from("void main()\n{\n");
    for name in ["mg_linked_dev", "mg_linked_ovr"] {
        script += &format!(
            "    WriteTimestampedLogEntry(\"MG_ALIAS {name}=\" + ResManGetAliasFor(\"{name}\", \
             2017));\n    WriteTimestampedLogEntry(\"MG_READ {name}=\" + \
             Get2DAString(\"{name}\", \"LABEL\", 0));\n"
        );
    }
    script += "    WriteTimestampedLogEntry(\"MG_DONE\");\n}\n";
    probe_module(&root, &dir, module, &script, &[], &[]);
    let run = run_server(&root, &dir, module, "MG_DONE", Duration::from_secs(120)).unwrap();
    let tail = &run.log[run.log.len().saturating_sub(2000)..];
    assert!(run.finished, "server did not finish; log tail:\n{tail}");
    let value = |tag: &str, name: &str| {
        run.values(tag)
            .into_iter()
            .find_map(|v| v.split_once('=').filter(|(n, _)| *n == name).map(|(_, v)| v.to_string()))
    };
    // The engine: found where linked, and read.
    assert_eq!(value("MG_ALIAS", "mg_linked_dev").as_deref(), Some("DEVELOPMENT:"));
    assert_eq!(value("MG_READ", "mg_linked_dev").as_deref(), Some("from_dev_link"));
    assert_eq!(value("MG_ALIAS", "mg_linked_ovr").as_deref(), Some("OVERRIDE:"));
    assert_eq!(value("MG_READ", "mg_linked_ovr").as_deref(), Some("from_override_link"));
    // Moonglow: the same.
    let install = GameInstall::new(&root, Some(dir.clone()), "en");
    let rm = ResMan::for_game(&install).unwrap();
    let origin = |name: &str| {
        let key = ResKey::new(ResRef::from_str(name).unwrap(), ResType::TWODA);
        rm.origin(&key).map(|o| o.to_string())
    };
    assert_eq!(origin("mg_linked_dev").as_deref(), Some("development"));
    assert_eq!(origin("mg_linked_ovr").as_deref(), Some("override"));
}
