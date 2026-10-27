//! Comparisons with what Aurora wrote (the oracle's captures).
//!
//! One test binary for these files: each is a module here, so that they are
//! linked once and their tests run side by side. A new test file is named
//! in a group (`every_test_file_is_in_a_group` in `formats` says so).

#[path = "../aurora_blueprints.rs"]
mod aurora_blueprints;
#[path = "../aurora_common/mod.rs"]
mod aurora_common;
#[path = "../aurora_creature_wizard.rs"]
mod aurora_creature_wizard;
#[path = "../aurora_creatures.rs"]
mod aurora_creatures;
#[path = "../aurora_dialog.rs"]
mod aurora_dialog;
#[path = "../aurora_encounters.rs"]
mod aurora_encounters;
#[path = "../aurora_factions.rs"]
mod aurora_factions;
#[path = "../aurora_instances.rs"]
mod aurora_instances;
#[path = "../aurora_inventory.rs"]
mod aurora_inventory;
#[path = "../aurora_item_wizard.rs"]
mod aurora_item_wizard;
#[path = "../aurora_journal.rs"]
mod aurora_journal;
#[path = "../aurora_levelup.rs"]
mod aurora_levelup;
#[path = "../aurora_lighting_scheme.rs"]
mod aurora_lighting_scheme;
#[path = "../aurora_new.rs"]
mod aurora_new;
#[path = "../aurora_palette_add.rs"]
mod aurora_palette_add;
#[path = "../aurora_palettes.rs"]
mod aurora_palettes;
#[path = "../aurora_script_wizard.rs"]
mod aurora_script_wizard;
#[path = "../aurora_sounds.rs"]
mod aurora_sounds;
#[path = "../aurora_spell_warnings.rs"]
mod aurora_spell_warnings;
#[path = "../aurora_store_setup.rs"]
mod aurora_store_setup;
#[path = "../aurora_terrain.rs"]
mod aurora_terrain;
#[path = "../aurora_update_instances.rs"]
mod aurora_update_instances;
