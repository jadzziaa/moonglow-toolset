//! mg-uikit's tests that drive egui: the tokens against the mockups'
//! tables, what `apply` does to a context, the icon widgets, the notices
//! of the icons and fonts in the packages' licenses, and the kit
//! sheet (every standard control under the theme; its pictures, run by
//! hand: `cargo test -p mg-uikit --test kit -- --ignored`, write
//! `target/test-output/uikit/`).

mod licenses;
mod sheet;
mod theme;
mod tokens;
mod widgets;
