//! Models: reading, compiling, decompiling and linting every model of the game.
//!
//! One test binary for these files: each is a module here, so that they are
//! linked once and their tests run side by side. A new test file is named
//! in a group (`every_test_file_is_in_a_group` in `formats` says so).

#[path = "../mdl_common/mod.rs"]
mod mdl_common;

#[path = "../game_shaders.rs"]
mod game_shaders;
#[path = "../mdl_compile.rs"]
mod mdl_compile;
#[path = "../mdl_decompile.rs"]
mod mdl_decompile;
#[path = "../mdl_lint.rs"]
mod mdl_lint;
#[path = "../models.rs"]
mod models;
#[path = "../models_nwnmdlcomp.rs"]
mod models_nwnmdlcomp;
#[path = "../waterfalls.rs"]
mod waterfalls;
