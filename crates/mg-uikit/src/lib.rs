//! The Moonglow suite's look, shared by Moonglow Toolset and Moonglow
//! Viewer: the redesign's [color tokens](mod@tokens), [metrics], [theme]
//! (egui's style made from them, dark and light), [fonts], the drawn
//! [icon set](icons), and the [form helpers](form) both applications'
//! forms use.
//!
//! Egui is its one UI dependency, and it uses no other Moonglow crate.
//!
//! ```
//! let ctx = egui::Context::default();
//! mg_uikit::apply(&ctx, mg_uikit::Theme::Light);
//! assert_eq!(mg_uikit::tokens(&ctx), mg_uikit::Tokens::LIGHT);
//! ```

pub mod fonts;
pub mod form;
pub mod icons;
pub mod metrics;
pub mod theme;
pub mod tokens;

#[cfg(test)]
#[allow(dead_code)]
mod svg;

pub use fonts::install_fonts;
pub use form::{field_label, palette_swatches, section_heading, swatch, table_heading};
pub use icons::{Icon, icon, icon_button, icon_text_button, icon_text_toggle, icon_toggle};
pub use theme::{Theme, TokensExt, applied, apply, tokens};
pub use tokens::Tokens;
