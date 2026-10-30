//! The script compiler and analysis over every script of the game.
//!
//! One test binary for these files: each is a module here, so that they are
//! linked once and their tests run side by side. A new test file is named
//! in a group (`every_test_file_is_in_a_group` in `formats` says so).

#[path = "../script_common/mod.rs"]
mod script_common;

#[path = "../ncs.rs"]
mod ncs;
#[path = "../nui.rs"]
mod nui;
#[path = "../nwscript_spec.rs"]
mod nwscript_spec;
#[path = "../script_analysis.rs"]
mod script_analysis;
#[path = "../script_frontend.rs"]
mod script_frontend;
#[path = "../scripts.rs"]
mod scripts;
