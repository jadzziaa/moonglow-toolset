//! Areas and tilesets of the shipped modules (the area tests open a GPU device).
//!
//! One test binary for these files: each is a module here, so that they are
//! linked once and their tests run side by side. A new test file is named
//! in a group (`every_test_file_is_in_a_group` in `formats` says so).

#[path = "../area_common/mod.rs"]
mod area_common;

#[path = "../areas.rs"]
mod areas;
#[path = "../hlsl.rs"]
mod hlsl;
#[path = "../minimaps.rs"]
mod minimaps;
#[path = "../object_walkmeshes.rs"]
mod object_walkmeshes;
#[path = "../tiles.rs"]
mod tiles;
#[path = "../tileset_authoring.rs"]
mod tileset_authoring;
