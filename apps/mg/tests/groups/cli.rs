//! The command line's tests, one binary: each file of `tests/` is a module
//! here, so that they are linked once and their tests run side by side. A
//! new test file is named here (`every_test_file_is_in_the_group` says so).

#[path = "../areas.rs"]
mod areas;
#[path = "../attach.rs"]
mod attach;
#[path = "../claims.rs"]
mod claims;
#[path = "../compile.rs"]
mod compile;
#[path = "../dialog_io.rs"]
mod dialog_io;
#[path = "../diff.rs"]
mod diff;
#[path = "../edits.rs"]
mod edits;
#[path = "../encoding.rs"]
mod encoding;
#[path = "../export_files.rs"]
mod export_files;
#[path = "../haks.rs"]
mod haks;
#[path = "../json.rs"]
mod json;
#[path = "../lsp.rs"]
mod lsp;
#[path = "../merge_gff.rs"]
mod merge_gff;
#[path = "../minimap.rs"]
mod minimap;
#[path = "../models.rs"]
mod models;
#[path = "../nasher.rs"]
mod nasher;
#[path = "../nui.rs"]
mod nui;
#[path = "../nwsync_upload.rs"]
mod nwsync_upload;
#[path = "../places.rs"]
mod places;
#[path = "../plugins.rs"]
mod plugins;
#[path = "../replace.rs"]
mod replace;
#[path = "../roundtrip.rs"]
mod roundtrip;
#[path = "../row_names.rs"]
mod row_names;
#[path = "../search.rs"]
mod search;
#[path = "../serve.rs"]
mod serve;
#[path = "../test_cmd.rs"]
mod test_cmd;
#[path = "../tlk_alloc.rs"]
mod tlk_alloc;
#[path = "../tlk_io.rs"]
mod tlk_io;
#[path = "../twoda_check.rs"]
mod twoda_check;
#[path = "../update_instances.rs"]
mod update_instances;
#[path = "../vars.rs"]
mod vars;
#[path = "../verify.rs"]
mod verify;
#[path = "../watch.rs"]
mod watch;

/// `mg`, as every test here starts it: with Moonglow's data and cache
/// folders (what it remembers of the scripts it compiled, the language
/// server's cache) under `target/test-output`, not the developer's own,
/// on every system (`MOONGLOW_DATA_DIR`, `MOONGLOW_CACHE_DIR`: on macOS
/// the folders otherwise follow `HOME`, which also finds the game).
fn mg() -> std::process::Command {
    let home = mg_testkit::output_dir("mg-home");
    let mut mg = std::process::Command::new(env!("CARGO_BIN_EXE_mg"));
    mg.env("MOONGLOW_DATA_DIR", home.join("data/moonglow"))
        .env("MOONGLOW_CACHE_DIR", home.join("cache/moonglow"));
    mg
}

/// [`mg`] on the game installed at `root`, with `user` as the game's
/// user folder or none (`--no-user-dir`): what a test of the game's data
/// adds.
fn mg_on(
    root: impl AsRef<std::path::Path>,
    user: Option<&std::path::Path>,
) -> std::process::Command {
    let mut mg = mg();
    mg.arg("--root").arg(root.as_ref());
    match user {
        Some(user) => mg.arg("--user-dir").arg(user),
        None => mg.arg("--no-user-dir"),
    };
    mg
}

/// [`mg`] with no user folder of the game's (`--no-user-dir`), on the
/// game wherever it finds one: for what reads no game, or only its own.
fn mg_alone() -> std::process::Command {
    let mut mg = mg();
    mg.arg("--no-user-dir");
    mg
}

/// `mg --json ARGS`: whether it succeeded, and the JSON it printed.
fn mg_json(args: &[&str]) -> (bool, serde_json::Value) {
    json_out(mg().arg("--json").args(args))
}

/// `mg ARGS`, run to its end.
fn mg_output(args: &[&str]) -> std::process::Output {
    mg().args(args).output().unwrap()
}

/// What `mg` printed as JSON (`--json` among its arguments): whether it
/// succeeded, and the JSON.
fn json_out(mg: &mut std::process::Command) -> (bool, serde_json::Value) {
    read_json(mg, false)
}

/// [`json_out`], and nothing printed to standard error.
fn quiet_json_out(mg: &mut std::process::Command) -> (bool, serde_json::Value) {
    read_json(mg, true)
}

fn read_json(mg: &mut std::process::Command, quiet: bool) -> (bool, serde_json::Value) {
    let out = mg.output().unwrap();
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!quiet || stderr.trim().is_empty(), "{mg:?}: stderr {stderr}");
    let json = serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|e| panic!("{mg:?}: {e}: {}", String::from_utf8_lossy(&out.stdout)));
    (out.status.success(), json)
}

/// This crate's tests are not found by Cargo (`autotests = false`): a file
/// not named above would not be compiled.
#[test]
fn every_test_file_is_in_the_group() {
    let left_out = mg_testkit::ungrouped_tests(env!("CARGO_MANIFEST_DIR"));
    assert!(left_out.is_empty(), "not in tests/groups/cli.rs: {left_out:?}");
}
