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
mod bake;
mod blueprints;
mod conversations;
mod drawing;
mod fields;
mod gpu_memory;
mod hak_closing;
mod icon_tips;
mod inventories;
mod lists_together;
mod looks;
mod menus;
mod module_panel;
mod modules;
mod options_and_keys;
mod palettes;
mod part_emitters;
mod plugin_kinds;
mod plugins;
mod preview_panel;
mod projects;
mod quick_open;
mod renderer_check;
mod resources;
mod resources_panel;
mod right_dock;
mod scripts;
mod search;
mod shell;
mod sounds;
mod status_bar;
mod tables;
mod tile_edges;
mod tiles;
mod tree;
mod typed_text;
mod updates;
mod windows;

/// A file of this folder not named above as a module is not compiled: its
/// tests would pass by not running.
#[test]
fn every_test_file_is_a_module() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/app");
    let left_out = mg_testkit::unlisted_modules(&dir);
    assert!(left_out.is_empty(), "not in tests/app/main.rs: {left_out:?}");
}
