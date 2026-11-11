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

pub mod controls;
pub mod fonts;
pub mod form;
pub mod icons;
pub mod metrics;
pub mod theme;
pub mod tokens;

#[cfg(test)]
#[allow(dead_code)]
mod svg;

pub use controls::{
    Checkbox, Controls, RadioButton, code_frame, collapsing_icon, combo_icon, is_read_only,
    list_rows, menu_bar_style, menu_rows, menu_style, read_only, row_height, row_metrics,
    row_pitch, row_style, show_rows, slider, table_rows, text_field, with_rows, with_table_rows,
};
pub use fonts::install_fonts;
pub use form::{field_label, palette_swatches, section_heading, swatch, table_heading};
pub use icons::{
    Icon, IconButton, Label, icon, icon_button, icon_label, icon_text_button, icon_text_toggle,
    icon_toggle, icons_drawn, menu_button, name_group, record_icons, submenu_button,
};
pub use theme::{Theme, TokensExt, applied, apply, tokens};
pub use tokens::Tokens;
