//! UI flows driven through the accessibility tree (egui_kittest), on small
//! modules built by the tests; those that need the game's data skip
//! without it.
//!
//! One test binary: each subject is a module here, so that they are
//! linked once and their tests run side by side. A new test goes in the
//! module of its subject; what several share goes in `common`.

mod common;

mod area_view;
mod areas;
mod blueprints;
mod conversations;
mod drawing;
mod fields;
mod inventories;
mod looks;
mod modules;
mod options_and_keys;
mod palettes;
mod plugins;
mod projects;
mod renderer_check;
mod resources;
mod scripts;
mod search;
mod tables;
mod tiles;
mod tree;
mod windows;

/// A file of this folder not named above as a module is not compiled: its
/// tests would pass by not running.
#[test]
fn every_test_file_is_a_module() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/app");
    let left_out = mg_testkit::unlisted_modules(&dir);
    assert!(left_out.is_empty(), "not in tests/app/main.rs: {left_out:?}");
}
