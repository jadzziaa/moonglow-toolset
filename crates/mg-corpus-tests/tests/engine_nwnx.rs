//! NWNX in the server Moonglow runs: the project's `[nwnx]` resolved
//! (`mg_module::nwnx`) into the server's environment, with the local NWNX
//! build (`MOONGLOW_NWNX_HOME`, else `~/.local/opt/nwnx/current`; skipped
//! without one unless `MOONGLOW_REQUIRE_CORPUS`), and an NWNX_Util call
//! answered by the server. Linux only: NWNX loads into `nwserver` there
//! alone.
#![cfg(target_os = "linux")]

use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::time::{Duration, Instant};

use mg_module::nwnx::{self, Level};
use mg_module::project_file::ProjectFile;
use mg_testkit::engine::server_binary;
use mg_testkit::{corpus, scratch_dir};

use crate::common::probe_module;

/// The local NWNX build, or none (the test skips; it fails where the
/// corpus is required).
fn nwnx_home() -> Option<PathBuf> {
    let home = std::env::var_os("MOONGLOW_NWNX_HOME").map(PathBuf::from).or_else(|| {
        std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/opt/nwnx/current"))
    });
    let home = home.filter(|h| h.join("Binaries/NWNX_Core.so").is_file());
    if home.is_none() {
        assert!(!mg_testkit::corpus_required(), "MOONGLOW_REQUIRE_CORPUS: no NWNX build");
        eprintln!("skipped: no NWNX build (MOONGLOW_NWNX_HOME)");
    }
    home
}

/// A project folder whose `[nwnx]` uses `home` with these settings.
fn project(dir: &Path, home: &Path, env: &str) -> ProjectFile {
    std::fs::create_dir_all(dir).unwrap();
    let team = format!(
        "[nwnx]\nuse = true\nhome = \"{}\"\nenv = \"nwnx.env\"\n",
        home.display().to_string().replace('\\', "/")
    );
    std::fs::write(dir.join("moonglow.toml"), team).unwrap();
    std::fs::write(dir.join("nwnx.env"), env).unwrap();
    ProjectFile::read(dir)
}

/// The server, with the project's NWNX, loads the module, and a script's
/// NWNX_Util call is answered (the name of the script running); NWNX's
/// build is the server's, as its first lines say.
#[test]
fn the_server_runs_nwnx_from_the_project_s_settings() {
    let root = corpus!();
    let Some(server) = server_binary(&root) else {
        eprintln!("skipped: no nwserver for this platform");
        return;
    };
    let Some(home) = nwnx_home() else { return };
    let dir = scratch_dir("engine_nwnx");
    let user = dir.join("server");
    std::fs::create_dir_all(&user).unwrap();
    // (The include beside the script, as the compiler finds it.)
    let include = std::fs::read(home.join("NWScript/nwnx_util.nss")).unwrap();
    std::fs::write(user.join("nwnx_util.nss"), include).unwrap();
    let module = "mg_nwnx";
    let script = "#include \"nwnx_util\"\nvoid main()\n{\n    \
                  WriteTimestampedLogEntry(\"MG_NWNX \" + NWNX_Util_GetCurrentScriptName());\n    \
                  WriteTimestampedLogEntry(\"MG_DONE\");\n}\n";
    probe_module(&root, &user, module, script, &[], &[]);
    let env = "NWNX_CORE_SKIP_ALL=y\nNWNX_UTIL_SKIP=n\nNWNX_CORE_LOG_LEVEL=6\n";
    let p = project(&dir.join("project"), &home, env);
    let resolved = nwnx::resolve(&p, &Default::default());
    assert!(!resolved.failed(), "{:?}", resolved.notes);
    let n = resolved.nwnx.expect("NWNX on");
    assert!(n.loads("Util") && !n.loads("Store"), "{:?}", n.plugins);
    let mut s = mg_serve::Server::start(&mg_serve::Options {
        server,
        user_dir: user.clone(),
        module: module.into(),
        name: "moonglow-test".into(),
        console: true,
        env: n.server_env(),
        clean_env: true,
        ..Default::default()
    })
    .unwrap();
    let ready = s.wait_ready(Duration::from_secs(120), &AtomicBool::new(false));
    let lines: Vec<String> = s.lines().into_iter().map(|l| l.text).collect();
    assert!(ready.is_ok(), "{ready:?}: {lines:#?}");
    let mut builds = nwnx::Builds::default();
    let said: Vec<_> = lines.iter().filter_map(|l| builds.line(l)).collect();
    assert_eq!(said.len(), 1, "both builds told: {lines:#?}");
    assert_eq!(said[0].level, Level::Info, "{}", said[0].text);
    assert!(lines.iter().all(|l| nwnx::console_note(l, true).is_none()), "{lines:#?}");
    let start = Instant::now();
    while !s.log().contains("MG_DONE") && start.elapsed() < Duration::from_secs(60) {
        std::thread::sleep(Duration::from_millis(100));
    }
    let log = s.log();
    s.stop(Duration::from_secs(10));
    assert!(log.contains(&format!("MG_NWNX {module}")), "NWNX_Util answered: {log}");
}

/// A plugin the settings switch on that the build has no file of stops
/// the start, named; so does a home without NWNX's core. (A build made
/// for another game can't be had on one machine: the mismatch is read
/// from the server's lines, `mg_module::nwnx`'s unit tests.)
#[test]
fn a_missing_plugin_or_core_stops_the_start() {
    let Some(home) = nwnx_home() else { return };
    let dir = scratch_dir("engine_nwnx_missing");
    let env = "NWNX_CORE_SKIP_ALL=y\nNWNX_UTIL_SKIP=n\nNWNX_MOONGLOWNOSUCH_SKIP=n\n";
    let r = nwnx::resolve(&project(&dir.join("project"), &home, env), &Default::default());
    assert!(r.failed() && r.nwnx.is_none());
    assert!(r.errors()[0].contains("NWNX_MOONGLOWNOSUCH"), "{:?}", r.notes);
    let empty = dir.join("empty-home");
    std::fs::create_dir_all(empty.join("Binaries")).unwrap();
    let r = nwnx::resolve(&project(&dir.join("project"), &empty, ""), &Default::default());
    assert!(r.errors()[0].contains("NWNX_Core.so is not there"), "{:?}", r.notes);
}
