//! File formats, archives and modules: the game's own files read, written back
//! and compared with the oracle tools.
//!
//! One test binary for these files: each is a module here, so that they are
//! linked once and their tests run side by side. A new test file is named
//! in a group (`every_test_file_is_in_a_group` in `formats` says so).

#[path = "../common/mod.rs"]
mod common;

#[path = "../archives.rs"]
mod archives;
#[path = "../audio.rs"]
mod audio;
#[path = "../base_game.rs"]
mod base_game;
#[path = "../creature_names.rs"]
mod creature_names;
#[path = "../creature_stats.rs"]
mod creature_stats;
#[path = "../dialog_formats.rs"]
mod dialog_formats;
#[path = "../doctor.rs"]
mod doctor;
#[path = "../hak_edit.rs"]
mod hak_edit;
#[path = "../item_properties.rs"]
mod item_properties;
#[path = "../item_wizard.rs"]
mod item_wizard;
#[path = "../module_search.rs"]
mod module_search;
#[path = "../modules.rs"]
mod modules;
#[path = "../nasher.rs"]
mod nasher;
#[path = "../nwn_gff_diff.rs"]
mod nwn_gff_diff;
#[path = "../nwnt_diff.rs"]
mod nwnt_diff;
#[path = "../nwsync.rs"]
mod nwsync;
#[path = "../palettes.rs"]
mod palettes;
#[path = "../shaders.rs"]
mod shaders;
#[path = "../small_formats.rs"]
mod small_formats;
#[path = "../text_replace.rs"]
mod text_replace;
#[path = "../textures.rs"]
mod textures;
#[path = "../tlk.rs"]
mod tlk;
#[path = "../twoda.rs"]
mod twoda;

/// This crate's tests are not found by Cargo (`autotests = false`): a file
/// in no group would not be compiled.
#[test]
fn every_test_file_is_in_a_group() {
    let left_out = mg_testkit::ungrouped_tests(env!("CARGO_MANIFEST_DIR"));
    assert!(left_out.is_empty(), "in no group of tests/groups/: {left_out:?}");
}
