//! The command line's tests, one binary: each file of `tests/` is a module
//! here, so that they are linked once and their tests run side by side. A
//! new test file is named here (`every_test_file_is_in_the_group` says so).

#[path = "../areas.rs"]
mod areas;
#[path = "../attach.rs"]
mod attach;
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
#[path = "../json.rs"]
mod json;
#[path = "../lsp.rs"]
mod lsp;
#[path = "../minimap.rs"]
mod minimap;
#[path = "../models.rs"]
mod models;
#[path = "../nasher.rs"]
mod nasher;
#[path = "../nui.rs"]
mod nui;
#[path = "../plugins.rs"]
mod plugins;
#[path = "../replace.rs"]
mod replace;
#[path = "../roundtrip.rs"]
mod roundtrip;
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
#[path = "../verify.rs"]
mod verify;
#[path = "../watch.rs"]
mod watch;

/// `mg`, as every test here starts it: with Moonglow's data and cache
/// folders (what it remembers of the scripts it compiled, the language
/// server's cache) under `target/test-output`, not the developer's own.
/// (On macOS the data folder follows `HOME`, which also finds the game: it
/// is left alone.)
fn mg() -> std::process::Command {
    let home = mg_testkit::output_dir("mg-home");
    let mut mg = std::process::Command::new(env!("CARGO_BIN_EXE_mg"));
    mg.env("XDG_DATA_HOME", home.join("data"))
        .env("XDG_CACHE_HOME", home.join("cache"))
        .env("APPDATA", home.join("data"));
    mg
}

/// This crate's tests are not found by Cargo (`autotests = false`): a file
/// not named above would not be compiled.
#[test]
fn every_test_file_is_in_the_group() {
    let left_out = mg_testkit::ungrouped_tests(env!("CARGO_MANIFEST_DIR"));
    assert!(left_out.is_empty(), "not in tests/groups/cli.rs: {left_out:?}");
}
