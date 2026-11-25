//! UI flows driven through the accessibility tree (egui_kittest), on small
//! modules built by the tests; those that need the game's data skip
//! without it.
//!
//! One test binary: each subject is a module here, so that they are
//! linked once and their tests run side by side. A new test goes in the
//! module of its subject; what several share goes in `common`.

mod common;

mod appearance_tab;
mod area_header;
mod area_input_fixes;
mod area_tools;
mod area_view;
mod areas;
mod areas_height;
mod bake;
mod blueprints;
mod board_builds;
mod colored_icons;
mod conversation_lines;
mod conversation_outliner;
mod conversation_panels;
mod conversations;
mod dialog_looks;
mod dialog_pattern;
mod dialog_pictures;
mod dialogs;
mod doc_tabs;
mod dock_pictures;
mod dock_tabs;
mod drawing;
mod fields;
mod focus_frames;
mod form_columns;
mod form_pages;
mod free_doors;
mod galleries;
mod gallery_keys;
mod game_data_holders;
mod gpu_memory;
mod hak_closing;
mod hak_panels;
mod handles;
mod icon_tips;
mod inventories;
mod journal_factions;
mod lists_together;
mod looks;
mod menus;
mod module_panel;
mod modules;
mod nui_panels;
mod open_editors;
mod options_and_keys;
mod options_window;
mod outline_color;
mod outliner;
mod palettes;
mod part_emitters;
mod plugin_kinds;
mod plugins;
mod preview_panel;
mod projects;
mod properties_panel;
mod quick_open;
mod read_only;
mod renderer_check;
mod resources;
mod resources_panel;
mod right_dock;
mod script_bottom;
mod script_header;
mod script_keys;
mod scripts;
mod search;
mod shell;
mod sounds;
mod status_bar;
mod status_hints;
mod table_panels;
mod tables;
mod talk_panels;
mod tile_edges;
mod tile_properties;
mod tiles;
mod tileset_panels;
mod tool_section_keys;
mod tool_sections;
mod transition_setup;
mod transition_triggers;
mod tree;
mod typed_text;
mod updates;
mod welcome;
mod window_pane_sizes;
mod windows;
mod yields;

/// A file of this folder not named above as a module is not compiled: its
/// tests would pass by not running.
#[test]
fn every_test_file_is_a_module() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/app");
    let left_out = mg_testkit::unlisted_modules(&dir);
    assert!(left_out.is_empty(), "not in tests/app/main.rs: {left_out:?}");
}
