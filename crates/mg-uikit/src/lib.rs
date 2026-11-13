//! The Moonglow suite's look, shared by Moonglow Toolset and Moonglow
//! Viewer: the redesign's [color tokens](mod@tokens), [metrics], [theme]
//! (egui's style made from them, dark and light), [fonts], the drawn
//! [icon set](icons), the [form helpers](form) both applications' forms
//! use, and the shell's parts: the [Top Bar's, the docks' and the
//! document tabs'](shell), the [Status Bar's](status), the key caps,
//! badges and modified dot ([marks]), a [menu]'s parts beyond its
//! rows (headings, notes, separators, check boxes, keys, forms), and a
//! [window]'s (a dialog's title row, the resize grip).
//!
//! Egui is its one UI dependency, and it uses no other Moonglow crate.
//! With the `dock` feature it also takes egui_dock, for the editor area's
//! style and tab marks (the `dock` module, decision O5 of the proposal's M3): the
//! editor area is egui_dock's `DockArea`, the docks around it are the
//! kit's own.
//!
//! ```
//! let ctx = egui::Context::default();
//! mg_uikit::apply(&ctx, mg_uikit::Theme::Light);
//! assert_eq!(mg_uikit::tokens(&ctx), mg_uikit::Tokens::LIGHT);
//! ```

pub mod controls;
#[cfg(feature = "dock")]
pub mod dock;
pub mod fonts;
pub mod form;
pub mod icons;
pub mod marks;
pub mod menu;
pub mod metrics;
pub mod shell;
pub mod status;
pub mod theme;
pub mod tokens;
pub mod tree;
pub mod window;

#[cfg(test)]
#[allow(dead_code)]
mod svg;

pub use controls::{
    Checkbox, Controls, RadioButton, code_frame, collapsing_icon, combo_icon, is_read_only,
    list_rows, menu_bar_style, menu_rows, menu_style, read_only, row_height, row_metrics,
    row_pitch, row_style, segmented, show_rows, slider, table_rows, text_field, with_rows,
    with_table_rows,
};
pub use fonts::install_fonts;
pub use form::{field_label, palette_swatches, section_heading, swatch, table_heading};
pub use icons::{
    Icon, IconButton, Join, Label, form_menu_button, front_lines_drawn, icon, icon_button,
    icon_label, icon_text_button, icon_text_toggle, icon_toggle, icons_drawn, joined_spacing,
    menu_button, name_group, record_icons, say_expanded, submenu_button,
};
pub use marks::{Badge, BadgeKind, KeyCap, modified_dot, paint_modified_dot};
pub use menu::{
    MenuKeys, context_menu, menu_check, menu_form, menu_heading, menu_note, menu_radio,
    menu_radio_choice, menu_row_lit, menu_row_with_keys, menu_separator,
};
pub use theme::{Theme, TokensExt, applied, apply, tokens};
pub use tokens::Tokens;
pub use tree::{TreeGroup, TreeRow, TreeRowShown};
pub use window::{TitleRow, paint_resize_grip, title_row, title_row_width};
