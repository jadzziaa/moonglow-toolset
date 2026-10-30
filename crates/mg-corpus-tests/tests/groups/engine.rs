//! The engine tests: modules run in the game's headless server.
//!
//! One test binary for these files: each is a module here, so that they are
//! linked once and their tests run side by side. A new test file is named
//! in a group (`every_test_file_is_in_a_group` in `formats` says so).

#[path = "../common/mod.rs"]
mod common;

#[path = "../engine_2da.rs"]
mod engine_2da;
#[path = "../engine_area_edits.rs"]
mod engine_area_edits;
#[path = "../engine_area_light.rs"]
mod engine_area_light;
#[path = "../engine_area_list.rs"]
mod engine_area_list;
#[path = "../engine_area_variables.rs"]
mod engine_area_variables;
#[path = "../engine_armor_colors.rs"]
mod engine_armor_colors;
#[path = "../engine_armor_torso.rs"]
mod engine_armor_torso;
#[path = "../engine_big_hak.rs"]
mod engine_big_hak;
#[path = "../engine_blueprints.rs"]
mod engine_blueprints;
#[path = "../engine_custom_light.rs"]
mod engine_custom_light;
#[path = "../engine_dialog.rs"]
mod engine_dialog;
#[path = "../engine_doctor.rs"]
mod engine_doctor;
#[path = "../engine_ee_fields.rs"]
mod engine_ee_fields;
#[path = "../engine_encoding.rs"]
mod engine_encoding;
#[path = "../engine_equipped_items.rs"]
mod engine_equipped_items;
#[path = "../engine_facings.rs"]
mod engine_facings;
#[path = "../engine_factions.rs"]
mod engine_factions;
#[path = "../engine_git_runtime.rs"]
mod engine_git_runtime;
#[path = "../engine_grass.rs"]
mod engine_grass;
#[path = "../engine_hak_names.rs"]
mod engine_hak_names;
#[path = "../engine_item_cost.rs"]
mod engine_item_cost;
#[path = "../engine_item_flags.rs"]
mod engine_item_flags;
#[path = "../engine_journal.rs"]
mod engine_journal;
#[path = "../engine_light_state.rs"]
mod engine_light_state;
#[path = "../engine_local_variables.rs"]
mod engine_local_variables;
#[path = "../engine_locked.rs"]
mod engine_locked;
#[path = "../engine_modules.rs"]
mod engine_modules;
#[path = "../engine_new_module.rs"]
mod engine_new_module;
#[path = "../engine_object_by_tag.rs"]
mod engine_object_by_tag;
#[path = "../engine_rename.rs"]
mod engine_rename;
#[path = "../engine_resman.rs"]
mod engine_resman;
#[path = "../engine_resource_names.rs"]
mod engine_resource_names;
#[path = "../engine_return_types.rs"]
mod engine_return_types;
#[path = "../engine_rules.rs"]
mod engine_rules;
#[path = "../engine_saves.rs"]
mod engine_saves;
#[path = "../engine_special_abilities.rs"]
mod engine_special_abilities;
#[path = "../engine_static_placeables.rs"]
mod engine_static_placeables;
#[path = "../engine_terrain.rs"]
mod engine_terrain;
#[path = "../engine_test_start.rs"]
mod engine_test_start;
#[path = "../engine_tileset_holes.rs"]
mod engine_tileset_holes;
#[path = "../engine_tlk.rs"]
mod engine_tlk;
#[path = "../engine_transfer.rs"]
mod engine_transfer;
#[path = "../engine_verify.rs"]
mod engine_verify;
#[path = "../engine_visual_transforms.rs"]
mod engine_visual_transforms;
