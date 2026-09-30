//! Resource resolution compared with the engine's own (`ResManGetAliasFor`).

use std::collections::HashMap;
use std::time::Duration;

use mg_core::{ResRef, ResType};
use mg_erf::ErfWriter;
use mg_resman::{ErfContainer, GameInstall, LayerClass, ResKey, ResMan, priority};
use mg_testkit::engine::{run_server, server_binary};
use mg_testkit::{corpus, oracle_tool, scratch_dir};

mod common;
use common::probe_module;

/// The engine's name for a layer.
fn alias_for(label: &str, module: &str) -> Option<String> {
    Some(match label {
        "development" => "DEVELOPMENT:".to_string(),
        "override" => "OVERRIDE:".to_string(),
        l if l.starts_with("key:") => format!("HD0INSTALL:data/{}", &l[4..]),
        l if l == format!("module:{module}") => format!("CURRENTGAME:{module}"),
        _ => return None,
    })
}

fn key(name: &str, t: ResType) -> ResKey {
    ResKey::parse(name, t).unwrap()
}

/// Moonglow's resman picks the same source as the engine's
/// `ResManGetAliasFor`, for resources placed so that every precedence
/// boundary is crossed: `development/` over a hak, a user-directory hak over
/// an install hak listed before it, haks over the module, the module over
/// `override/`, `override/` over the keys, retail over base keys; plus a
/// sample of everything in each layer.
#[test]
fn resolution_matches_engine() {
    let root = corpus!();
    if server_binary(&root).is_none() {
        eprintln!("skipped: no nwserver for this platform");
        return;
    }
    let _ = oracle_tool!("nwn_script_comp");
    let dir = scratch_dir("engine_resman");
    let module = "mg_resman";
    let install_hak = "id_resources";
    let user_hak = "mg_userhak";
    let marker = |s: &str| format!("2DA V2.0\n\n LABEL\n0 {s}\n").into_bytes();

    // development/ over the install hak.
    std::fs::create_dir_all(dir.join("development")).unwrap();
    std::fs::write(dir.join("development/random_hostile.2da"), marker("dev")).unwrap();
    // A user hak (listed second) over the install hak (listed first).
    std::fs::create_dir_all(dir.join("hak")).unwrap();
    let mut hak = ErfWriter::new(*b"HAK ");
    hak.add(ResRef::from_str("random_neutral").unwrap(), ResType::TWODA, marker("userhak"))
        .unwrap();
    hak.add(ResRef::from_str("mg_only_userhak").unwrap(), ResType::TWODA, marker("x")).unwrap();
    std::fs::write(dir.join(format!("hak/{user_hak}.hak")), hak.to_bytes().unwrap()).unwrap();
    // override/ under the module, over the keys.
    std::fs::create_dir_all(dir.join("override")).unwrap();
    for f in ["repute.2da", "classes.2da", "nw_chicken.utc"] {
        std::fs::write(dir.join("override").join(f), marker("override")).unwrap();
    }
    // The module: under the hak, over override.
    let extra = [
        (key("random_lines", ResType::TWODA), marker("module")),
        (key("repute", ResType::TWODA), marker("module")),
    ];
    let haks = [install_hak, user_hak];
    probe_module(&root, &dir, module, "void main() {}", &haks, &extra);

    let install = GameInstall::new(&root, Some(dir.clone()), "en");
    let build = || {
        let mut rm = ResMan::for_game(&install).unwrap();
        // The headless server loads no portraits and no ambient or music folders.
        for l in ["portraits", "user portraits", "ambient", "music", "user ambient", "user music"] {
            rm.remove(l);
        }
        assert!(rm.add_haks(&install, &haks).unwrap().is_empty());
        let modfile = dir.join(format!("modules/{module}.mod"));
        let module_layer = ErfContainer::open(&modfile).unwrap();
        rm.add(priority::MODULE, format!("module:{module}"), LayerClass::Erf, module_layer);
        rm
    };
    let rm = build();

    let origin = |n: &str, t: ResType| rm.origin(&key(n, t)).unwrap().to_string();
    assert_eq!(origin("random_hostile", ResType::TWODA), "development");
    assert_eq!(origin("random_neutral", ResType::TWODA), format!("hak:{user_hak}"));
    assert_eq!(origin("random_lines", ResType::TWODA), format!("hak:{install_hak}"));
    assert_eq!(origin("repute", ResType::TWODA), format!("module:{module}"));
    assert_eq!(origin("classes", ResType::TWODA), "override");
    assert_eq!(origin("nw_chicken", ResType::UTC), "override");

    let mut sample: Vec<ResKey> = [
        "random_hostile",
        "random_neutral",
        "random_lines",
        "repute",
        "classes",
        "mg_only_userhak",
    ]
    .iter()
    .map(|n| key(n, ResType::TWODA))
    .chain([key("nw_chicken", ResType::UTC)])
    .collect();
    for l in rm.layers() {
        let mut keys: Vec<ResKey> = l.container.keys().collect();
        keys.sort();
        let step = if keys.len() > 300 { keys.len() / 150 } else { 1 };
        sample.extend(keys.into_iter().step_by(step));
    }
    sample.sort();
    sample.dedup();
    sample.retain(|k| !(k.restype == ResType::NCS && k.resref.as_str() == Some(module)));

    let mut script = String::from("void main()\n{\n");
    for k in &sample {
        script += &format!(
            "    WriteTimestampedLogEntry(\"MG_ALIAS {k}=\" + ResManGetAliasFor(\"{}\", {}));\n",
            k.resref.to_lowercase(),
            k.restype.0
        );
    }
    script += "    WriteTimestampedLogEntry(\"MG_DONE\");\n}\n";
    // Unmap the module before rewriting it with the probe script, then look
    // again at what the engine will see.
    drop(rm);
    probe_module(&root, &dir, module, &script, &haks, &extra);
    let rm = build();

    let run = run_server(&root, &dir, module, "MG_DONE", Duration::from_secs(120)).unwrap();
    assert!(
        run.finished,
        "server did not finish; log tail:\n{}",
        &run.log[run.log.len().saturating_sub(2000)..]
    );
    let engine: HashMap<String, String> = run
        .values("MG_ALIAS")
        .into_iter()
        .filter_map(|v| v.split_once('=').map(|(a, b)| (a.to_string(), b.to_string())))
        .collect();

    // Haks: the engine names them by where it found them.
    let hak_alias: HashMap<String, String> = run
        .values("MG_ALIAS")
        .iter()
        .filter_map(|v| v.split_once('='))
        .filter(|(_, a)| a.contains(':'))
        .map(|(_, a)| a.to_string())
        .filter(|a| a.ends_with(install_hak) || a.ends_with(user_hak))
        .map(|a| (a.rsplit(':').next().unwrap().to_string(), a))
        .collect();

    let mut failures = Vec::new();
    for k in &sample {
        let ours = rm.origin(k).unwrap().to_string();
        let theirs = engine.get(&k.to_string()).cloned().unwrap_or_else(|| "<not logged>".into());
        let expected = match ours.strip_prefix("hak:") {
            Some(h) => hak_alias.get(h).cloned(),
            None => alias_for(&ours, module),
        };
        match expected {
            Some(e) if e == theirs => {}
            Some(e) => failures.push(format!("{k}: we pick {ours} ({e}), engine {theirs:?}")),
            None => {
                failures.push(format!("{k}: we pick {ours}, engine {theirs:?} (no alias mapping)"))
            }
        }
    }
    eprintln!("compared {} resources; hak aliases {hak_alias:?}", sample.len());
    assert!(failures.is_empty(), "{} mismatches:\n{}", failures.len(), failures.join("\n"));
}
