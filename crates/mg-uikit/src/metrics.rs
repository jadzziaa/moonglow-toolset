//! The redesign's sizes in points (the mockup spec, section 5, and the
//! stylesheet's metrics): heights of rows, controls and bars, gaps on a
//! 4 px step, corner radii, type sizes and the docks' default sizes.

/// A tree, list or table row.
pub const ROW_HEIGHT: f32 = 22.0;
/// The line between a table's rows of fields (the mock's `.table td`
/// `border-bottom`): a table whose rows are text fields, combo boxes and
/// number fields leaves this much of the panel between them, which
/// their wells, borderless in rows, would otherwise run together across.
pub const TABLE_RULE: f32 = 1.0;
/// A control: button, toggle, combo box, text field, slider.
pub const CONTROL_HEIGHT: f32 = 22.0;
/// A control's padding at each side (`padding: 0 8px`).
pub const CONTROL_PADDING_X: f32 = 8.0;
/// An icon.
pub const ICON: f32 = 16.0;
/// An icon button: the icon in a square of this side.
pub const ICON_BUTTON: f32 = 22.0;
/// A small icon (a chevron in a combo box, a tab's close cross).
pub const ICON_SMALL: f32 = 12.0;
/// A large icon (an empty state's).
pub const ICON_LARGE: f32 = 32.0;
/// The icons' stroke in their 24 by 24 box.
pub const ICON_STROKE: f32 = 1.5;

/// The top bar (menus, quick actions, Quick Open, build actions).
pub const TOP_BAR: f32 = 30.0;
/// The editor area's document tabs.
pub const DOCUMENT_TABS: f32 = 28.0;
/// An editor's header row.
pub const EDITOR_HEADER: f32 = 30.0;
/// A dock panel's tab bar.
pub const DOCK_TABS: f32 = 26.0;
/// The tool shelf (a vertical strip of an editor's tools).
pub const TOOL_SHELF: f32 = 36.0;
/// A tool shelf's buttons.
pub const TOOL_BUTTON: f32 = 28.0;
/// The status bar.
pub const STATUS_BAR: f32 = 24.0;
/// A menu's row (the mock's `.menu-row`), with no gap between rows.
pub const MENU_ROW: f32 = 24.0;
/// A menu's title in the top bar (the mock's `.menu`).
pub const MENU_TITLE: f32 = 22.0;
/// The active document tab's accent line along its top.
pub const ACTIVE_TAB_LINE: f32 = 2.0;
/// A dialog's title row (the mock's `.dlg-bar`).
pub const TITLE_ROW: f32 = 28.0;

/// Corner radius of controls, rows and tooltips.
pub const RADIUS: u8 = 3;
/// Corner radius of popovers, menus and windows.
pub const POPOVER_RADIUS: u8 = 4;
/// The smallest gap.
pub const GAP_S: f32 = 4.0;
/// The middle gap.
pub const GAP_M: f32 = 8.0;
/// The largest gap.
pub const GAP_L: f32 = 12.0;
/// A panel's padding.
pub const PANEL_PADDING: f32 = 8.0;
/// A form grid's gap between its label and value columns.
pub const FORM_COLUMN_GAP: f32 = 12.0;
/// A form grid's gap between rows.
pub const FORM_ROW_GAP: f32 = 6.0;
/// A tree's indent per level.
pub const INDENT: f32 = 14.0;
/// A form's label column (the stylesheet's `--lbl-w`).
pub const LABEL_WIDTH: f32 = 104.0;
/// A form's column, at its narrowest and widest, when a form flows into
/// columns.
pub const FORM_COLUMN: std::ops::RangeInclusive<f32> = 440.0..=520.0;

/// The left dock's default width (at 1600 by 900).
pub const LEFT_DOCK: f32 = 260.0;
/// The right dock's default width.
pub const RIGHT_DOCK: f32 = 320.0;
/// The bottom dock's default height.
pub const BOTTOM_DOCK: f32 = 180.0;
/// The height of the Left Dock's upper panel (Module or Resources, its tab
/// bar included) over an Outliner, at first (`Main.html`'s
/// `.module-panel`: `flex: 0 0 496px`); the Outliner takes the rest.
pub const MODULE_PANEL: f32 = 496.0;

/// Body text (Ubuntu Light): everything but what follows.
pub const BODY_SIZE: f32 = 13.0;
/// A line of body text.
pub const LINE_HEIGHT: f32 = 16.0;
/// Section headings (Ubuntu Bold, the only bold).
pub const HEADING_SIZE: f32 = 15.0;
/// Badges, key caps and the status bar's weak text.
pub const SMALL_SIZE: f32 = 11.0;
/// Monospace (Hack): code, resource names.
pub const MONO_SIZE: f32 = 13.0;
