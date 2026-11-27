//! The game's server query as Options › Test Module's command waits on it
//! (GitHub issue 52): `mg_serve::answers` finds the game's own headless
//! server once its module runs, as neverwinter.nim's `nwn_net bnes` does,
//! and none at a port nothing serves.

use std::time::{Duration, Instant};

use mg_testkit::engine::server_binary;
use mg_testkit::{corpus, oracle_tool, scratch_dir};

use crate::common;

#[test]
fn a_running_server_answers_the_games_query() {
    let root = corpus!();
    let Some(server) = server_binary(&root) else {
        eprintln!("skipped: no nwserver for this platform");
        return;
    };
    let _ = oracle_tool!("nwn_script_comp");
    let nwn_net = oracle_tool!("nwn_net");
    let dir = scratch_dir("engine_server_query");
    let script = "void main()\n{\n    WriteTimestampedLogEntry(\"MG_LOADED\");\n}\n";
    common::probe_module(&root, &dir, "mg_query_probe", script, &[], &[]);
    // (Private, with passwords: mg_serve's defaults.)
    let mut running = mg_serve::Server::start(&mg_serve::Options {
        server,
        user_dir: dir.clone(),
        module: "mg_query_probe".into(),
        name: "moonglow-test".into(),
        ..Default::default()
    })
    .unwrap();
    let address = format!("127.0.0.1:{}", running.port);
    // Asked from the start: when it first answers, against when its module
    // has loaded (a game that joins before that quits).
    let began = Instant::now();
    let (mut answered, mut loaded) = (None, None);
    while answered.is_none() || loaded.is_none() {
        assert!(began.elapsed() < Duration::from_secs(120), "{}", running.log());
        assert!(running.exited().is_none(), "the server ended: {}", running.log());
        if loaded.is_none() && running.log().contains("MG_LOADED") {
            loaded = Some(began.elapsed());
        }
        if answered.is_none() && mg_serve::answers(&address, Duration::from_millis(200)) {
            answered = Some(began.elapsed());
        }
    }
    let (answered, loaded) = (answered.unwrap(), loaded.unwrap());
    eprintln!("first answer after {answered:?}, module loaded after {loaded:?}");
    assert!(
        answered + Duration::from_millis(300) >= loaded,
        "answered at {answered:?}, before the module loaded at {loaded:?}"
    );
    // The oracle asks the same and hears the server's name back.
    let oracle = std::process::Command::new(&nwn_net).args([&address, "bnes"]).output().unwrap();
    let said = String::from_utf8_lossy(&oracle.stdout);
    assert!(oracle.status.success() && said.contains("moonglow-test"), "nwn_net: {said}");
    running.stop(Duration::from_secs(10));
    assert!(!mg_serve::answers(&address, Duration::from_secs(2)), "stopped, yet {address}");
}
