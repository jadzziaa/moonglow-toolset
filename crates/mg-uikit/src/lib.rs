//! The Moonglow suite's look, made for Moonglow Toolset and Moonglow
//! Viewer (the toolset uses it; the viewer, which pins the toolset's
//! crates at a release tag, takes it when it moves to a release that has
//! it, the redesign's M6): the redesign's [color tokens](mod@tokens), [metrics], [theme]
//! (egui's style made from them, dark and light), [fonts], the drawn
//! [icon set](icons), the [form helpers](form) both applications' forms
//! use, and the shell's parts: the [Top Bar's, the docks' and the
//! document tabs'](shell), the [Status Bar's](status), the key caps,
//! badges and modified dot ([marks]), a [menu]'s parts beyond its
//! rows (headings, notes, separators, check boxes, keys, forms), a
//! [window]'s (a dialog's title row, the resize grip), and an editor's:
//! its [header] row (which yields against its width and keeps a More
//! popover), the [popover] hung from a button (or unfolded into the
//! header), the tool [shelf], and the kit's own [button], drawn disabled
//! in its own tokens rather than faded; what stands over a 3D view
//! (the compass and the view's corner buttons, [overlay]); and a
//! [dialog]'s parts, so far the empty state of a pane with nothing in it;
//! and the [gallery] cell, its grid and the pictures' size slider.
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

pub mod button;
pub mod controls;
pub mod dialog;
#[cfg(feature = "dock")]
pub mod dock;
pub mod fonts;
pub mod form;
pub mod gallery;
pub mod header;
pub mod hues;
pub mod icons;
pub mod marks;
pub mod menu;
pub mod metrics;
pub mod overlay;
pub mod pages;
pub mod popover;
pub mod scrolling;
pub mod shelf;
pub mod shell;
pub mod status;
pub mod theme;
pub mod tokens;
pub mod tree;
pub mod window;

#[cfg(test)]
#[allow(dead_code)]
mod svg;

pub use button::{ButtonLook, KitButton};
pub use controls::{
    Checkbox, Controls, RadioButton, code_frame, collapsing_icon, combo_icon, combo_style,
    is_read_only, list_rows, menu_bar_style, menu_rows, menu_style, read_only, row_height,
    row_metrics, row_pitch, row_style, segmented, segmented_fill, show_rows, slider, table_rows,
    text_field, with_rows, with_table_rows,
};
pub use fonts::install_fonts;
pub use form::{field_label, palette_swatches, section_heading, swatch, table_heading};
pub use icons::{
    Icon, IconButton, Join, Label, form_menu_button, front_lines_drawn, icon, icon_button,
    icon_label, icon_text_button, icon_text_toggle, icon_toggle, icons_drawn, joined_spacing,
    menu_button, name_group, record_icons, say_expanded, submenu_button,
};
pub use marks::{
    Badge, BadgeKind, KeyCap, WAVE_HEIGHT, modified_dot, paint_modified_dot, paint_wavy_line,
};
pub use menu::{
    MenuKeys, context_menu, menu_check, menu_form, menu_heading, menu_note, menu_radio,
    menu_radio_choice, menu_row_lit, menu_row_with_keys, menu_separator,
};
pub use theme::{Theme, TokensExt, applied, apply, tokens};
pub use tokens::Tokens;
pub use tree::{TreeGroup, TreeRow, TreeRowShown};
pub use window::{
    TitleRow, paint_resize_grip, title_row, title_row_width, title_row_width_with, title_row_with,
};
