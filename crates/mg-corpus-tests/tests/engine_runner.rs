//! The engine runner itself: what a run that does not finish says.

use std::time::Duration;

use mg_testkit::engine::{run_server, server_binary};
use mg_testkit::{corpus, oracle_tool, scratch_dir};

use crate::common;
use common::probe_module;

#[test]
fn a_run_that_does_not_finish_says_why() {
    let root = corpus!();
    if server_binary(&root).is_none() {
        eprintln!("skipped: no nwserver for this platform");
        return;
    }
    let _ = oracle_tool!("nwn_script_comp");
    let dir = scratch_dir("engine_runner_unfinished");
    // A module that never logs the done marker.
    let script = "void main()\n{\n    WriteTimestampedLogEntry(\"MG_STARTED\");\n}\n";
    probe_module(&root, &dir, "mg_runner_probe", script, &[], &[]);
    let run =
        run_server(&root, &dir, "mg_runner_probe", "MG_NEVER", Duration::from_secs(5)).unwrap();
    assert!(!run.finished);
    let why = run.why.expect("an unfinished run says why");
    assert!(why.starts_with("the server did not finish after "), "{why}");
    assert!(why.contains("was still running"), "the server was stopped at the timeout: {why}");
    assert!(
        why.contains("nwengineLog.txt ends:\n") && why.contains("nwserverError1.txt ends:"),
        "{why}"
    );
    assert!(
        !why.contains("(not read:") || why.contains("nwserverError1.txt ends:\n(not read:"),
        "the engine's log is there to read: {why}"
    );
}
