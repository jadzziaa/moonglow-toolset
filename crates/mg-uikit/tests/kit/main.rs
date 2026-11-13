//! mg-uikit's tests that drive egui: the tokens against the mockups'
//! tables, what `apply` does to a context, the icon widgets, the shell's
//! parts (found by their names and operated), a tree's rows, and the kit
//! sheet (every standard control under the theme; its pictures, run by
//! hand: `cargo test -p mg-uikit --test kit -- --ignored`, write
//! `target/test-output/uikit/`).

mod menus;
mod sheet;
mod shell;
mod theme;
mod tokens;
mod tree;
mod widgets;
mod window;
