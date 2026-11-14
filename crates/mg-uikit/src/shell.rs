//! The shell's parts (proposal section 5, the mock's `.topbar`,
//! `.panel-tabs`, `.docktab`, `.rail.edge` and `.doctab`), each drawn once
//! from the tokens, for an application to compose its frame from:
//!
//! - the Top Bar: [`top_bar`] (the 30 px row: the menus and quick actions
//!   at the left, the build actions at the right, and between them a slot
//!   for a centered field, Quick Open), its menus' titles in
//!   [`crate::menu_bar_style`] (22 px) and their rows in
//!   [`crate::menu_style`] (24 px with no gap), and [`vsep`];
//! - a dock's tab bar, [`DockTabs`] (26 px, its tools at the right), and a
//!   panel's header where it has one tab, [`PanelHeader`];
//! - a folded dock's strip, [`FoldStrip`] (24 px of its panels' icons
//!   under a chevron that unfolds it, or without the chevron:
//!   [`FoldStrip::chevron`]), and the button that folds it,
//!   [`fold_button`] (with its tip, [`fold_button_tip`], among a tab bar's
//!   tools); a dock's tabs show their icons alone where their words and
//!   the tools do not fit ([`DockTabs::tools_width`]);
//! - the document tabs' parts, painted over egui_dock's tabs:
//!   [`doc_tab_parts`], [`paint_doc_tab_icon`], [`paint_doc_tab_close`],
//!   with [`crate::paint_modified_dot`];
//! - [`Yields`]: what the shell gives up at a window's and an editor's
//!   width (section 5.2) and the wide layout (section 5.3).
//!
//! The Status Bar's parts are in [`crate::status`], the key caps and badges
//! in [`crate::marks`].
//!
//! Accessible names. A dock's tab bar is a tab list named by its dock
//! (`"Left Dock"`), its tabs named by their words and said selected or
//! not; a folded dock's strip is a group of the same name (the strip
//! stands in for the tab bar while the dock is folded), its chevron named
//! `"Show the Left Dock"` and said closed; a panel header is a group named
//! by its title. A test finds a dock's parts within its group
//! (`h.get_by_label("Left Dock").get_by_label("Module")`), so that two
//! docks with a tab of one name, or a tab and a Status Bar switch, do not
//! collide.
//!
//! How the shell is meant to put them together (a sketch):
//!
//! ```
//! # egui::__run_test_ui(|ui| {
//! use mg_uikit::shell::{self, DockTabs, FoldStrip, Side, Yields};
//! use mg_uikit::{Icon, IconButton};
//! let yields = Yields::of(1600.0, 1000.0);
//! shell::top_bar(
//!     ui,
//!     yields.quick_open_cap,
//!     |ui| {
//!         // The menus (an `egui::MenuBar` styled with `menu_bar_style`).
//!         shell::vsep(ui);
//!         ui.add(IconButton::new(Icon::Save, "Save").icon_only(true));
//!     },
//!     |ui, width| {
//!         // Quick Open, `width` wide (an icon button under 240).
//!         ui.label(format!("{width}"));
//!     },
//!     |ui| {
//!         ui.add(IconButton::new(Icon::Compile, "Compile").icon_only(!yields.build_words));
//!     },
//! );
//! if yields.left_dock_folded {
//!     let clicked =
//!         FoldStrip::new("Left Dock", Side::Left).panel(Icon::Module, "Module", "Module").show(ui);
//!     # let _ = clicked;
//! } else {
//!     let mut tab = 0;
//!     DockTabs::new("Left Dock").tab("Module").tab("Resources").show(ui, &mut tab, |ui| {
//!         shell::fold_button(ui, Side::Left, "Left Dock");
//!     });
//! }
//! # });
//! ```

use egui::{
    Color32, FontId, Rect, Response, Sense, Stroke, Ui, Vec2, WidgetInfo, WidgetType, pos2, vec2,
};

use crate::icons::{ButtonState, Icon, IconButton};
use crate::marks::{self, Badge};
use crate::metrics;
use crate::theme::TokensExt as _;
use crate::tokens::Tokens;

// ------------------------------------------------------------------ yields

/// The window's width under which the build actions lose their words and
/// Quick Open is capped at [`QUICK_OPEN_NARROW`] (section 5.2).
pub const BUILD_WORDS_FROM: f32 = 1300.0;
/// The window's width under which the Right Dock narrows to
/// [`RIGHT_DOCK_NARROW`] with its panels as tabs, the Status Bar's
/// switches become icons and the game folder drops.
pub const RIGHT_DOCK_FROM: f32 = 1400.0;
/// The window's width under which the Left Dock folds to its strip.
pub const LEFT_DOCK_FROM: f32 = 1500.0;
/// The window's width from which the shell takes the wide layout (section
/// 5.3, decided U1).
pub const WIDE_FROM: f32 = 2560.0;
/// The editor's width under which an area's header goes compact.
pub const HEADER_COMPACT_UNDER: f32 = 990.0;
/// The editor's width under which Snap and Turn lose their value text.
pub const VALUE_TEXT_UNDER: f32 = 550.0;
/// The editor's width under which Together and Onto Objects move into More.
pub const INTO_MORE_UNDER: f32 = 490.0;
/// Quick Open's widest (`.topbar .quickopen`: `max-width: 440px`).
pub const QUICK_OPEN_MAX: f32 = 440.0;
/// Quick Open's widest in a narrow window.
pub const QUICK_OPEN_NARROW: f32 = 300.0;
/// The width under which Quick Open is an icon button rather than a field.
pub const QUICK_OPEN_ICON_UNDER: f32 = 240.0;
/// The Right Dock's width in a narrow window.
pub const RIGHT_DOCK_NARROW: f32 = 280.0;
/// The Left Dock's default width in the wide layout.
pub const LEFT_DOCK_WIDE: f32 = 320.0;
/// The Right Dock's default width in the wide layout (`WideArea.html`'s
/// `--right-w: 741px`: two columns and the line between them).
pub const RIGHT_DOCK_WIDE: f32 = 741.0;
/// The Right Dock's first column's widest where it stands in two
/// (`WideArea.html`'s `.dock-col.pal-col`: `flex: 0 0 340px`); the second
/// column takes the rest.
pub const RIGHT_COLUMN: f32 = 340.0;
/// The height of the Left Dock's upper panel over the Outliner at first in
/// the wide layout (`WideArea.html` and `GalleryWide.html`'s
/// `.module-panel`: `flex: 0 0 760px`, in a 1440 window).
pub const MODULE_PANEL_WIDE: f32 = 760.0;
/// The editor area's least width beside the docks: the narrowest editor
/// section 5.2 designs an area's header for (Together and Onto Objects in
/// More, [`INTO_MORE_UNDER`]). A dock gives way before the editor area
/// goes under it: the Right Dock folds to its strip in a window too narrow
/// for it beside the Left Dock's strip ([`RIGHT_DOCK_STRIP_UNDER`]).
pub const EDITOR_LEAST: f32 = INTO_MORE_UNDER;
/// The window's width under which the Right Dock folds to its strip: the
/// Left Dock's strip, the editor area at its least and the Right Dock at
/// its narrow width do not fit side by side (each dock's 1 px line
/// counted): 796.
pub const RIGHT_DOCK_STRIP_UNDER: f32 = STRIP + 1.0 + EDITOR_LEAST + RIGHT_DOCK_NARROW + 1.0;

/// What the shell yields at a window's width W and its editor's width E
/// (proposal section 5.2, measured on the board "Area Editor at
/// 1280×800"; and the wide layout of section 5.3 from [`WIDE_FROM`]). A
/// pure function of the two widths: what depends on the room left
/// (Quick Open becoming an icon under [`QUICK_OPEN_ICON_UNDER`], the
/// Status Bar's hints dropping) is the parts' own.
///
/// Nothing changes at 1280 itself: it is only the board's width.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Yields {
    /// Compile, Verify and Test Module keep their words (W ≥ 1300).
    pub build_words: bool,
    /// Quick Open's widest: 440, or 300 under W 1300.
    pub quick_open_cap: f32,
    /// The Left Dock folds to its strip (W < 1500), without the user's
    /// setting changing: they may unfold it.
    pub left_dock_folded: bool,
    /// The Left Dock's default width: 260, or 320 in the wide layout.
    pub left_dock_width: f32,
    /// The Right Dock's width: its default 320 (741 in the wide layout),
    /// or 280 under W 1400 whatever width was chosen.
    pub right_dock_width: f32,
    /// The Right Dock is held at [`RIGHT_DOCK_NARROW`] (W < 1400): drawn
    /// at that width, the width chosen kept for a wider window.
    pub right_dock_narrow: bool,
    /// The Right Dock folds to its strip (W < [`RIGHT_DOCK_STRIP_UNDER`]),
    /// without the user's setting changing: they may unfold it.
    pub right_dock_folded: bool,
    /// The height at first of the Left Dock's panel over the Outliner:
    /// 496, or 760 in the wide layout.
    pub module_panel_height: f32,
    /// The Right Dock's panels become tabs of one panel (W < 1400).
    pub right_dock_tabbed: bool,
    /// The Status Bar's switches show their icons alone (W < 1400).
    pub status_switch_icons: bool,
    /// The Status Bar shows the game folder (W ≥ 1400).
    pub game_folder: bool,
    /// The wide layout (W ≥ 2560): the Left Dock 320 by default, the
    /// Right Dock in two columns where it is wide enough
    /// ([`Yields::right_dock_columns`]).
    pub wide: bool,
    /// An area's header is compact (E < 990): the kind toggles in a Show
    /// popover, Day and Night one toggle, icon buttons.
    pub header_compact: bool,
    /// Snap and Turn show their value text (E ≥ 550).
    pub value_text: bool,
    /// Together and Onto Objects move into More (E < 490).
    pub into_more: bool,
}

impl Yields {
    /// What the shell yields in a window `window` wide whose editor is
    /// `editor` wide.
    pub fn of(window: f32, editor: f32) -> Yields {
        let narrow = window < RIGHT_DOCK_FROM;
        let wide = window >= WIDE_FROM;
        Yields {
            build_words: window >= BUILD_WORDS_FROM,
            quick_open_cap: if window < BUILD_WORDS_FROM {
                QUICK_OPEN_NARROW
            } else {
                QUICK_OPEN_MAX
            },
            left_dock_folded: window < LEFT_DOCK_FROM,
            left_dock_width: if wide { LEFT_DOCK_WIDE } else { metrics::LEFT_DOCK },
            right_dock_width: match (narrow, wide) {
                (true, _) => RIGHT_DOCK_NARROW,
                (false, true) => RIGHT_DOCK_WIDE,
                (false, false) => metrics::RIGHT_DOCK,
            },
            right_dock_narrow: narrow,
            right_dock_folded: window < RIGHT_DOCK_STRIP_UNDER,
            module_panel_height: if wide { MODULE_PANEL_WIDE } else { metrics::MODULE_PANEL },
            right_dock_tabbed: narrow,
            status_switch_icons: narrow,
            game_folder: !narrow,
            wide,
            header_compact: editor < HEADER_COMPACT_UNDER,
            value_text: editor >= VALUE_TEXT_UNDER,
            into_more: editor < INTO_MORE_UNDER,
        }
    }

    /// Whether a Right Dock `width` wide stands in two columns (Palette |
    /// Preview): in the wide layout, wider than two narrow docks (560).
    pub fn right_dock_columns(&self, width: f32) -> bool {
        self.wide && width > 2.0 * RIGHT_DOCK_NARROW
    }

    /// The first column's width where a Right Dock `width` wide stands in
    /// two ([`Yields::right_dock_columns`]), a 1 px line between them: half
    /// the dock, at most [`RIGHT_COLUMN`] (the board's 340 of 741); the
    /// second column takes the rest, so neither is under
    /// [`RIGHT_DOCK_NARROW`].
    pub fn right_first_column(width: f32) -> f32 {
        ((width - 1.0) / 2.0).floor().min(RIGHT_COLUMN)
    }

    /// Whether Quick Open, given `width`, is an icon button (under 240).
    pub fn quick_open_as_icon(width: f32) -> bool {
        width < QUICK_OPEN_ICON_UNDER
    }
}

// ------------------------------------------------------------------ top bar

/// The Top Bar's accessible name.
pub const TOP_BAR: &str = "Top Bar";
/// The Top Bar's padding at each end (`.topbar`: `padding: 0 6px`).
pub const TOP_BAR_PADDING: f32 = 6.0;
/// The gap between the Top Bar's parts (`.topbar`: `gap: 2px`).
pub const TOP_BAR_GAP: f32 = 2.0;
/// The gap between the right-hand group's parts (`.topbar .right`: 4 px).
pub const TOP_BAR_RIGHT_GAP: f32 = 4.0;
/// The room kept at each side of the centered field (`.mid`: `0 12px`).
pub const TOP_BAR_MID_PADDING: f32 = 12.0;

/// The Top Bar's frame: `bar`, 6 px at each end; for a top panel
/// [`metrics::TOP_BAR`] high.
pub fn top_bar_frame(t: &Tokens) -> egui::Frame {
    egui::Frame::new().fill(t.bar).inner_margin(egui::Margin::symmetric(TOP_BAR_PADDING as i8, 0))
}

/// The Top Bar's row (`.topbar`): `left` laid out from the left (the
/// menus, the quick actions), `right` against the right end (the build
/// actions, in their order), and between them `middle` in a slot centered
/// in the room the two leave, 12 px from each, at most `cap` wide (Quick
/// Open: [`Yields::quick_open_cap`]); `middle` is given the slot's width,
/// under [`QUICK_OPEN_ICON_UNDER`] to draw an icon button instead. A group
/// named "Top Bar"; the parts are made in the order shown, left to right,
/// so that Tab goes through them so.
///
/// The right group's width is the last frame's (it is laid out after the
/// middle); when it changes, the frame is drawn again.
pub fn top_bar<L, M, R>(
    ui: &mut Ui,
    cap: f32,
    left: impl FnOnce(&mut Ui) -> L,
    middle: impl FnOnce(&mut Ui, f32) -> M,
    right: impl FnOnce(&mut Ui) -> R,
) -> (L, M, R) {
    let gaps = (TOP_BAR_GAP, TOP_BAR_RIGHT_GAP);
    three_parts(ui, TOP_BAR, gaps, left, right, |ui, room| {
        let width = (room.width() - 2.0 * TOP_BAR_MID_PADDING).max(0.0).min(cap).floor();
        let center = pos2(room.center().x.round(), room.center().y);
        let slot = Rect::from_center_size(center, vec2(width, room.height()));
        let builder = egui::UiBuilder::new()
            .max_rect(slot)
            .layout(egui::Layout::left_to_right(egui::Align::Center));
        ui.scope_builder(builder, |ui| middle(ui, width)).inner
    })
}

/// A row of three parts, as the Top Bar and the Status Bar are: `left`
/// from the left, `right` against the right end in its own order, and
/// `middle` given the room between them (after the row's `gaps.0` at each
/// side of it; `gaps.1` between the right group's parts). The row is a
/// group named `name`. The right group's width is the last frame's, kept
/// under the row's id; when it changes, the frame is drawn again.
pub(crate) fn three_parts<L, M, R>(
    ui: &mut Ui,
    name: &str,
    gaps: (f32, f32),
    left: impl FnOnce(&mut Ui) -> L,
    right: impl FnOnce(&mut Ui) -> R,
    middle: impl FnOnce(&mut Ui, Rect) -> M,
) -> (L, M, R) {
    three_parts_sized(ui, name, gaps, None, left, right, middle)
}

/// [`three_parts`], the right group's width given where the caller knows
/// it (`Some`): laid out by it at once, never by the last frame's, so a
/// change of it (a module's name, its modified dot) asks for no second
/// pass. (A pass asked for again runs without the frame's input: on the
/// frame of a click, the click would be lost.)
pub(crate) fn three_parts_sized<L, M, R>(
    ui: &mut Ui,
    name: &str,
    gaps: (f32, f32),
    right_width: Option<f32>,
    left: impl FnOnce(&mut Ui) -> L,
    right: impl FnOnce(&mut Ui) -> R,
    middle: impl FnOnce(&mut Ui, Rect) -> M,
) -> (L, M, R) {
    let layout = egui::Layout::left_to_right(egui::Align::Center);
    ui.with_layout(layout, |ui| {
        crate::icons::name_group(ui, name);
        let id = ui.unique_id().with("mg-uikit-right-group");
        let given = right_width.is_some();
        if let Some(width) = right_width {
            ui.data_mut(|d| d.insert_temp(id, width));
        }
        let right_width = last_width(ui, id);
        ui.spacing_mut().item_spacing.x = gaps.0;
        let l = left(ui);
        // (After the left group's gap, and before the right group's.)
        let start = ui.cursor().min.x;
        let full = ui.max_rect();
        let right_start = (full.right() - right_width).max(start);
        let end = (right_start - gaps.0).max(start);
        let room = Rect::from_x_y_ranges(start..=end, full.y_range());
        let builder = egui::UiBuilder::new().max_rect(room).layout(layout);
        let m = ui.scope_builder(builder, |ui| middle(ui, room)).inner;
        let r = at_right_end(ui, id, full, gaps.1, !given, right);
        (l, m, r)
    })
    .inner
}

/// The width the group kept under `id` took in the last frame (none the
/// first time).
fn last_width(ui: &Ui, id: egui::Id) -> f32 {
    ui.data(|d| d.get_temp(id)).unwrap_or(0.0)
}

/// `add` laid out left to right, in the order it makes them (and so Tab
/// goes through them), against the right end of `full`, its parts `gap`
/// apart: placed by the width it took in the last frame, kept under `id`;
/// when that changes, the frame is drawn again.
fn at_right_end<R>(
    ui: &mut Ui,
    id: egui::Id,
    full: Rect,
    gap: f32,
    measured_again: bool,
    add: impl FnOnce(&mut Ui) -> R,
) -> R {
    let width = last_width(ui, id);
    let left = (full.right() - width).max(full.left());
    let place = Rect::from_x_y_ranges(left..=full.right(), full.y_range());
    let layout = egui::Layout::left_to_right(egui::Align::Center);
    let builder = egui::UiBuilder::new().max_rect(place).layout(layout);
    // (A child that takes no room of `ui`'s: before its width is known it
    // stands past the end, and would widen what holds it, a dock's panel
    // for good.)
    let mut child = ui.new_child(builder);
    child.spacing_mut().item_spacing.x = gap;
    let r = add(&mut child);
    let measured = child.min_rect().width();
    if measured_again && (measured - width).abs() > 0.5 {
        ui.data_mut(|d| d.insert_temp(id, measured));
        ui.ctx().request_discard("a bar's right-hand group changed width");
    }
    r
}

/// A vertical line between groups of a bar (`.vsep`: 1 by 16 of
/// `line-soft`, 4 px of room at each side).
pub fn vsep(ui: &mut Ui) -> Response {
    let (rect, response) = ui.allocate_exact_size(vec2(9.0, 16.0), Sense::hover());
    if ui.is_rect_visible(rect) {
        let line = Rect::from_center_size(rect.center(), vec2(1.0, 16.0));
        ui.painter().rect_filled(line, 0.0, ui.tokens().line_soft);
    }
    response
}

// ------------------------------------------------------------------ fold strip

/// Which side of the window a dock stands at.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Side {
    /// The Left Dock.
    Left,
    /// The Right Dock.
    Right,
}

impl Side {
    /// A chevron pointing into the window (unfold).
    fn inward(self) -> Icon {
        match self {
            Side::Left => Icon::ChevronRight,
            Side::Right => Icon::ChevronLeft,
        }
    }

    /// A chevron pointing out of the window (fold).
    fn outward(self) -> Icon {
        match self {
            Side::Left => Icon::ChevronLeft,
            Side::Right => Icon::ChevronRight,
        }
    }
}

/// A folded dock's strip's width (`.edge`: 24 px).
pub const STRIP: f32 = 24.0;
/// The room above the strip's first button (`.edge`: `padding: 4px 0`).
pub const STRIP_PADDING: f32 = 4.0;
/// The gap between the strip's buttons (`gap: 2px`).
pub const STRIP_GAP: f32 = 2.0;
/// The line under the strip's chevron (`.edge .sep`: 14 by 1, 3 px of
/// room above and below).
pub const STRIP_SEP: f32 = 14.0;

/// What was clicked on a [`FoldStrip`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StripClick {
    /// The chevron: unfold the dock as it was.
    Unfold,
    /// A panel's icon (its index): unfold the dock on that panel.
    Panel(usize),
}

/// A folded dock (`.rail.edge`, the narrow board's): a 24 px strip of
/// `bar` holding, from the top, a chevron pointing into the window that
/// unfolds the dock, a line, and an icon button for each of its panels,
/// which unfolds it on that panel. A group named by its dock; the chevron
/// named "Show the Left Dock" and said closed (accesskit's expanded), each
/// panel's button by its name, with its tooltip. Without the chevron (and
/// its line) where the dock is unfolded by a control of its own elsewhere
/// ([`FoldStrip::chevron`]): one control to unfold it.
#[derive(Clone, Debug)]
#[must_use = "show it with .show(ui)"]
pub struct FoldStrip {
    dock: String,
    side: Side,
    unfold_tip: Option<String>,
    panels: Vec<(Icon, String, String)>,
    chevron: bool,
}

impl FoldStrip {
    /// The strip of the dock named `dock` (`"Left Dock"`) at `side`.
    pub fn new(dock: impl Into<String>, side: Side) -> Self {
        let (dock, panels) = (dock.into(), Vec::new());
        FoldStrip { dock, side, unfold_tip: None, panels, chevron: true }
    }

    /// Whether the strip has its chevron (and the line under it) at its
    /// top: it has, unless `shown` is false, where the application draws
    /// the control that unfolds the dock elsewhere, so that the strip does
    /// not show a second. Its panels' buttons then start at its top.
    pub fn chevron(mut self, shown: bool) -> Self {
        self.chevron = shown;
        self
    }

    /// A panel of the dock: its icon, its name (the button's accessible
    /// name) and its tooltip.
    pub fn panel(mut self, icon: Icon, name: impl Into<String>, tip: impl Into<String>) -> Self {
        self.panels.push((icon, name.into(), tip.into()));
        self
    }

    /// The chevron's tooltip (its name, "Show the Left Dock", where not
    /// given; with the keys that do it, say).
    pub fn unfold_tip(mut self, tip: impl Into<String>) -> Self {
        self.unfold_tip = Some(tip.into());
        self
    }

    /// The chevron's accessible name.
    pub fn unfold_name(&self) -> String {
        format!("Show the {}", self.dock)
    }

    /// Draws the strip down the height of `ui` (its width
    /// [`STRIP`], filled with `bar`); returns what was clicked.
    pub fn show(self, ui: &mut Ui) -> Option<StripClick> {
        let t = ui.tokens();
        let top = ui.max_rect().top();
        let left = ui.cursor().min.x;
        let rect = Rect::from_min_max(pos2(left, top), pos2(left + STRIP, ui.max_rect().bottom()));
        ui.painter().rect_filled(rect, 0.0, t.bar);
        let mut clicked = None;
        let builder = egui::UiBuilder::new()
            .max_rect(rect.shrink2(vec2(1.0, STRIP_PADDING)))
            .layout(egui::Layout::top_down(egui::Align::Center));
        ui.scope_builder(builder, |ui| {
            crate::icons::name_group(ui, &self.dock);
            ui.spacing_mut().item_spacing.y = STRIP_GAP;
            if self.chevron {
                let name = self.unfold_name();
                let tip = self.unfold_tip.clone().unwrap_or_else(|| name.clone());
                let chevron =
                    IconButton::new(self.side.inward(), name).icon_only(true).expanded(false);
                if ui.add(chevron).on_hover_text(tip).clicked() {
                    clicked = Some(StripClick::Unfold);
                }
                let (line, _) = ui.allocate_exact_size(vec2(STRIP_SEP, 7.0), Sense::hover());
                let y = line.center().y.round() - 0.5;
                ui.painter().hline(line.x_range(), y, Stroke::new(1.0, t.line_soft));
            }
            for (i, (icon, name, tip)) in self.panels.iter().enumerate() {
                let button = IconButton::new(*icon, name.as_str()).icon_only(true);
                if ui.add(button).on_hover_text(tip.as_str()).clicked() {
                    clicked = Some(StripClick::Panel(i));
                }
            }
        });
        ui.advance_cursor_after_rect(rect);
        clicked
    }
}

/// The button that folds a dock (the one fold button a dock has, issue
/// 10's rule), among its tab bar's tools: a chevron pointing out of the
/// window, named "Fold the Left Dock" (`dock`) and said open.
pub fn fold_button(ui: &mut Ui, side: Side, dock: &str) -> Response {
    fold_button_tip(ui, side, dock, &format!("Fold the {dock}"))
}

/// [`fold_button`] with `tip` as its tooltip in place of its name (its
/// name and the keys that do it, say: "Fold the Left Dock (Ctrl+Alt+1)"),
/// for a fold button laid out among other buttons (a dock's tab bar or
/// a panel header's tools): Moonglow Toolset's side docks have it at
/// their tab bar's end.
pub fn fold_button_tip(ui: &mut Ui, side: Side, dock: &str, tip: &str) -> Response {
    let name = format!("Fold the {dock}");
    let button = IconButton::new(side.outward(), name.as_str()).icon_only(true).expanded(true);
    ui.add(button).on_hover_text(tip)
}

// ------------------------------------------------------------------ dock tabs

/// A dock tab's padding at each side (`.docktab`: `padding: 0 10px`).
pub const DOCK_TAB_PADDING: f32 = 10.0;
/// The room between a panel header's title and its detail
/// ([`PanelHeader::detail`]): the title's padding at its right
/// (`.docktab.title`'s `padding: 0 10px`, in `.panel-tabs`, which has no
/// gap) and the 2 px the boards give the detail at its left.
pub const PANEL_DETAIL_GAP: f32 = DOCK_TAB_PADDING + 2.0;
/// The gap between a dock tab's words and its badge (`gap: 6px` and the
/// badge's own 2).
pub const DOCK_TAB_BADGE_GAP: f32 = 8.0;
/// The room before a tab bar's tools (`.panel-tabs .tools`: 4 px) and
/// after them (the bar's `padding-right: 2px`).
pub const TOOLS_PADDING: (f32, f32) = (4.0, 2.0);

/// One of a [`DockTabs`]' tabs: its words, and a count badge where it has
/// one (`.docktab .badge`).
#[derive(Clone, Debug)]
pub struct DockTab {
    text: String,
    badge: Option<Badge>,
    name: Option<String>,
    icon: Option<Icon>,
}

impl DockTab {
    /// A tab reading `text`.
    pub fn new(text: impl Into<String>) -> Self {
        DockTab { text: text.into(), badge: None, name: None, icon: None }
    }

    /// Its icon, shown alone where the bar is too narrow for every tab's
    /// words beside its tools ([`DockTabs::tools_width`]): the icon its
    /// panel has on the folded dock's strip.
    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    /// With a badge after its words; its accessible value.
    pub fn badge(mut self, badge: Badge) -> Self {
        self.badge = Some(badge);
        self
    }

    /// Named `name` for a screen reader (and tests) in place of its words.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    fn width(&self, ui: &Ui) -> f32 {
        let font = FontId::proportional(metrics::BODY_SIZE);
        let words = ui.painter().layout_no_wrap(self.text.clone(), font, Color32::WHITE).size().x;
        let badge = self.badge.as_ref().map_or(0.0, |b| DOCK_TAB_BADGE_GAP + b.width(ui));
        // (And the 1 px line at its right.)
        2.0 * DOCK_TAB_PADDING + words + badge + 1.0
    }

    /// Its width showing its icon alone (and its badge).
    fn icon_width(&self, ui: &Ui) -> f32 {
        let badge = self.badge.as_ref().map_or(0.0, |b| DOCK_TAB_BADGE_GAP + b.width(ui));
        2.0 * DOCK_TAB_ICON_PADDING + metrics::ICON + badge + 1.0
    }
}

/// An icon tab's padding at each side ([`DockTab::icon`]).
pub const DOCK_TAB_ICON_PADDING: f32 = 8.0;

/// What a [`DockTabs`] did.
#[derive(Debug)]
pub struct DockTabsOutput<R> {
    /// The bar's response (its whole rectangle).
    pub response: Response,
    /// The tabs showed their icons alone ([`DockTabs::tools_width`]).
    pub icons: bool,
    /// A tab was clicked and is now the one selected.
    pub changed: bool,
    /// What the tools gave.
    pub inner: R,
}

/// A dock's tab bar (`.panel-tabs` with `.docktab`s): 26 px of `bar` with
/// a 1 px `line` under it, its tabs from the left, each 10 px at each side
/// with a 1 px `line` at its right; at rest `text-2`, hovered `tab-hover`
/// and `text`, the one selected `panel` and `text` reaching over the line
/// under the bar (it is one with the panel under it); the tools (icon
/// buttons) at the right end.
///
/// A tab list named by its dock, each tab a tab named by its words (or
/// its own name, [`DockTab::name`]) and said selected or not, reached
/// with Tab and chosen with Enter or Space as a button; on a tab with
/// the keyboard's focus, Left and Right choose the tab before or after
/// it (round the ends, as a tab list's arrows go) and take the focus
/// there.
#[derive(Clone, Debug)]
#[must_use = "show it with .show(ui, &mut selected, tools)"]
pub struct DockTabs {
    dock: String,
    tabs: Vec<DockTab>,
    drawn_hovered: Option<usize>,
    tools_width: Option<f32>,
}

impl DockTabs {
    /// The tab bar of the dock named `dock` (`"Left Dock"`).
    pub fn new(dock: impl Into<String>) -> Self {
        DockTabs { dock: dock.into(), tabs: Vec::new(), drawn_hovered: None, tools_width: None }
    }

    /// The tools' width (`tools` will lay out so much): where the tabs'
    /// words and the tools do not fit the bar side by side, and every tab
    /// has an icon ([`DockTab::icon`]), the tabs show their icons alone,
    /// each its name in its tip (and still named by it). Measured each
    /// frame from the words and the bar's width alone, so at one width it
    /// is always the one or the other.
    pub fn tools_width(mut self, width: f32) -> Self {
        self.tools_width = Some(width);
        self
    }

    /// The width its tabs take side by side: with their words, or (where
    /// `icons`) their icons alone. (What is left of a bar for its tools,
    /// with [`TOOLS_PADDING`]: a caller whose tools can yield lays out
    /// those that fit.)
    pub fn tabs_width(&self, ui: &Ui, icons: bool) -> f32 {
        let each = |t: &DockTab| if icons { t.icon_width(ui) } else { t.width(ui) };
        self.tabs.iter().map(each).sum()
    }

    /// Whether the tabs show their icons alone in a bar `bar` wide.
    pub fn icons_in(&self, ui: &Ui, bar: f32) -> bool {
        let Some(tools) = self.tools_width else { return false };
        if self.tabs.is_empty() || self.tabs.iter().any(|t| t.icon.is_none()) {
            return false;
        }
        let words: f32 = self.tabs.iter().map(|t| t.width(ui)).sum();
        words + TOOLS_PADDING.0 + tools + TOOLS_PADDING.1 > bar
    }

    /// A tab reading `text`.
    pub fn tab(self, text: impl Into<String>) -> Self {
        self.with(DockTab::new(text))
    }

    /// A tab as `tab` says (with a badge, a name).
    pub fn with(mut self, tab: DockTab) -> Self {
        self.tabs.push(tab);
        self
    }

    /// The tab `index` drawn as hovered whatever the pointer does (a sheet
    /// of the states).
    pub fn drawn_hovered(mut self, index: usize) -> Self {
        self.drawn_hovered = Some(index);
        self
    }

    /// Draws the bar across `ui`'s width, the tab `*selected` selected;
    /// a tab clicked becomes it. `tools` lays out the tools against the
    /// right end, in the order it makes them.
    pub fn show<R>(
        self,
        ui: &mut Ui,
        selected: &mut usize,
        tools: impl FnOnce(&mut Ui) -> R,
    ) -> DockTabsOutput<R> {
        let t = ui.tokens();
        let (bar, response) = bar_rect(ui);
        paint_bar(ui, &t, bar);
        let icons = self.icons_in(ui, bar.width());
        let mut changed = false;
        // (The tab the arrow keys went to, given the focus once its id is
        // known.)
        let mut focus_to = None;
        let mut ids = Vec::with_capacity(self.tabs.len());
        let builder = egui::UiBuilder::new()
            .max_rect(bar)
            .layout(egui::Layout::left_to_right(egui::Align::Center));
        let inner = ui
            .scope_builder(builder, |ui| {
                crate::icons::name_group(ui, &self.dock);
                ui.ctx().accesskit_node_builder(ui.unique_id(), |node| {
                    node.set_role(egui::accesskit::Role::TabList);
                });
                ui.spacing_mut().item_spacing.x = 0.0;
                for (i, tab) in self.tabs.iter().enumerate() {
                    let wide = if icons { tab.icon_width(ui) } else { tab.width(ui) };
                    let size = vec2(wide, bar.height());
                    let (rect, r) = ui.allocate_exact_size(size, Sense::click());
                    ids.push(r.id);
                    let on = *selected == i;
                    if r.clicked() && !on {
                        *selected = i;
                        changed = true;
                    }
                    // Left and Right on the tab with the focus: the tab
                    // before or after it, round the ends.
                    // (egui's own arrows move the focus to the widget that
                    // way: the tab's, not egui's.)
                    let n = self.tabs.len();
                    if r.has_focus() && n > 1 {
                        let arrows =
                            egui::EventFilter { horizontal_arrows: true, ..Default::default() };
                        ui.memory_mut(|m| m.set_focus_lock_filter(r.id, arrows));
                        let step = ui.input_mut(|input| {
                            let none = egui::Modifiers::NONE;
                            if input.consume_key(none, egui::Key::ArrowRight) {
                                Some(1)
                            } else if input.consume_key(none, egui::Key::ArrowLeft) {
                                Some(n - 1)
                            } else {
                                None
                            }
                        });
                        if let Some(step) = step {
                            ui.memory_mut(|m| m.move_focus(egui::FocusDirection::None));
                            let to = (i + step) % n;
                            *selected = to;
                            changed = true;
                            focus_to = Some(to);
                        }
                    }
                    let on = *selected == i;
                    let name = tab.name.clone().unwrap_or_else(|| tab.text.clone());
                    r.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, &name));
                    let value = tab.badge.as_ref().map(|b| b.text().to_owned());
                    r.ctx.accesskit_node_builder(r.id, |node| {
                        node.set_role(egui::accesskit::Role::Tab);
                        node.set_selected(on);
                        if let Some(value) = value {
                            node.set_value(value);
                        }
                    });
                    let hovered = self.drawn_hovered == Some(i) || r.hovered() || r.has_focus();
                    paint_dock_tab(ui, &t, rect, tab, on, hovered, r.has_focus(), icons);
                    if icons {
                        r.on_hover_text(&name);
                    }
                }
                tools_at_right_end(ui, bar, tools)
            })
            .inner;
        // (A tab before the one that had the focus is drawn already: it
        // is shown selected from the next frame, the focus with it.)
        if let Some(id) = focus_to.and_then(|to| ids.get(to).copied()) {
            ui.memory_mut(|m| m.request_focus(id));
            ui.ctx().request_repaint();
        }
        DockTabsOutput { response, icons, changed, inner }
    }
}

/// A tab bar's tools (icon buttons) against its right end, in the order
/// `tools` makes them, after [`TOOLS_PADDING`]'s 4 px and 2 px from the
/// end.
fn tools_at_right_end<R>(ui: &mut Ui, bar: Rect, tools: impl FnOnce(&mut Ui) -> R) -> R {
    let id = ui.unique_id().with("mg-uikit-tools");
    let full = bar.with_max_x(bar.right() - TOOLS_PADDING.1);
    let full =
        full.with_min_x(full.left().max(ui.cursor().min.x + TOOLS_PADDING.0).min(full.right()));
    at_right_end(ui, id, full, 0.0, true, tools)
}

/// The bar's rectangle: `ui`'s width, [`metrics::DOCK_TABS`] high, taken
/// from `ui`.
fn bar_rect(ui: &mut Ui) -> (Rect, Response) {
    let size = vec2(ui.available_width(), metrics::DOCK_TABS);
    ui.allocate_exact_size(size, Sense::hover())
}

/// A tab bar's ground: `bar`, the 1 px `line` along its bottom.
fn paint_bar(ui: &Ui, t: &Tokens, bar: Rect) {
    let p = ui.painter();
    p.rect_filled(bar, 0.0, t.bar);
    let line = Rect::from_min_max(pos2(bar.left(), bar.bottom() - 1.0), bar.right_bottom());
    p.rect_filled(line, 0.0, t.line);
}

/// Paints a dock tab in `rect` (the bar's height).
#[allow(clippy::too_many_arguments)]
fn paint_dock_tab(
    ui: &Ui,
    t: &Tokens,
    rect: Rect,
    tab: &DockTab,
    on: bool,
    hovered: bool,
    focused: bool,
    icon_alone: bool,
) {
    let p = ui.painter();
    // (The tab's own part, its line at the right apart.)
    let body = Rect::from_min_max(rect.min, pos2(rect.right() - 1.0, rect.bottom()));
    let (fill, color) = match (on, hovered) {
        (true, _) => (t.panel, t.text),
        (false, true) => (t.tab_hover, t.text),
        (false, false) => (Color32::TRANSPARENT, t.text_2),
    };
    // (Selected, over the bar's line: one with the panel under it.)
    let ground = if on { body } else { body.with_max_y(body.bottom() - 1.0) };
    p.rect_filled(ground, 0.0, fill);
    p.vline(rect.right() - 0.5, rect.top()..=rect.bottom() - 1.0, Stroke::new(1.0, t.line));
    if focused {
        let ring = body.with_max_y(body.bottom() - 1.0).shrink(1.0);
        p.rect_stroke(ring, 0.0, Stroke::new(1.0, t.accent), egui::StrokeKind::Inside);
    }
    let (x, words) = match tab.icon.filter(|_| icon_alone) {
        Some(icon) => {
            let x = rect.left() + DOCK_TAB_ICON_PADDING;
            let side = metrics::ICON;
            let at = pos2(x, (rect.center().y - 0.5 - side / 2.0).round());
            icon.paint(p, Rect::from_min_size(at, vec2(side, side)), color);
            (x, side)
        }
        None => {
            let font = FontId::proportional(metrics::BODY_SIZE);
            let galley = p.layout_no_wrap(tab.text.clone(), font, color);
            let y = (rect.center().y - 0.5 - galley.size().y / 2.0).round();
            let x = rect.left() + DOCK_TAB_PADDING;
            let words = galley.size().x;
            p.galley(pos2(x, y), galley, color);
            (x, words)
        }
    };
    if let Some(badge) = &tab.badge {
        let left = x + words + DOCK_TAB_BADGE_GAP;
        let r = Rect::from_min_size(
            pos2(left, (rect.center().y - 0.5 - marks::MARK_HEIGHT / 2.0).round()),
            vec2(badge.width(ui), marks::MARK_HEIGHT),
        );
        badge.paint(p, t, r);
    }
}

/// A panel's header where its dock shows it alone (`.panel-tabs` with a
/// `.docktab.title`): 26 px of `bar` with a 1 px `line` under it, the
/// panel's title in `text` 10 px from the left, a detail after it in weak
/// `text-3` (the Outliner's area, the Properties panel's kind), the tools
/// at the right end. A group named by its title (or its own name), in
/// which its tools are found.
#[derive(Clone, Debug)]
#[must_use = "show it with .show(ui, tools)"]
pub struct PanelHeader {
    title: String,
    detail: Option<String>,
    name: Option<String>,
}

impl PanelHeader {
    /// The header of the panel `title`.
    pub fn new(title: impl Into<String>) -> Self {
        PanelHeader { title: title.into(), detail: None, name: None }
    }

    /// A detail after the title, in weak text.
    pub fn detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    /// Named `name` for a screen reader (and tests) in place of its title.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Draws it across `ui`'s width; `tools` lays out the tools against
    /// the right end, in the order it makes them.
    pub fn show<R>(self, ui: &mut Ui, tools: impl FnOnce(&mut Ui) -> R) -> egui::InnerResponse<R> {
        let t = ui.tokens();
        let (bar, response) = bar_rect(ui);
        paint_bar(ui, &t, bar);
        let builder = egui::UiBuilder::new()
            .max_rect(bar)
            .layout(egui::Layout::left_to_right(egui::Align::Center));
        // (A child over the bar taken, not a scope: a scope moves the
        // cursor to its own end, back up into the bar, and what is laid
        // out under the header went under it by the bar's height less the
        // title's.)
        let inner = {
            let ui = &mut ui.new_child(builder);
            {
                let name = self.name.clone().unwrap_or_else(|| self.title.clone());
                crate::icons::name_group(ui, &name);
                let p = ui.painter();
                let font = FontId::proportional(metrics::BODY_SIZE);
                let title = p.layout_no_wrap(self.title.clone(), font.clone(), t.text);
                let y = |h: f32| (bar.center().y - 0.5 - h / 2.0).round();
                let x = bar.left() + DOCK_TAB_PADDING;
                let mut end = x + title.size().x;
                p.galley(pos2(x, y(title.size().y)), title, t.text);
                if let Some(detail) = &self.detail {
                    let galley = p.layout_no_wrap(detail.clone(), font, t.text_3);
                    // (`.t3.row` after `.docktab.title`, in `.panel-tabs`,
                    // which has no gap: the title's 10 px padding at its
                    // right, and the detail's own 2 at its left.)
                    let at = end + PANEL_DETAIL_GAP;
                    end = at + galley.size().x;
                    p.galley(pos2(at, y(galley.size().y)), galley, t.text_3);
                }
                ui.add_space(end - bar.left());
                tools_at_right_end(ui, bar, tools)
            }
        };
        egui::InnerResponse::new(inner, response)
    }
}

// ------------------------------------------------------------------ document tabs

/// A document tab's padding at its left and right (`.doctab`: `padding:
/// 0 6px 0 10px`).
pub const DOC_TAB_PADDING: (f32, f32) = (10.0, 6.0);
/// The gap between a document tab's parts (`gap: 6px`).
pub const DOC_TAB_GAP: f32 = 6.0;
/// A document tab's close button (`.doctab .x`: 16 by 16, its cross 12).
pub const DOC_TAB_CLOSE: f32 = 16.0;

/// Where a document tab's parts go (`.doctab`: its kind icon, its title,
/// the modified dot where modified, the close button).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DocTabParts {
    /// The kind icon's 16 px square.
    pub icon: Rect,
    /// The title's room.
    pub title: Rect,
    /// The modified dot's 7 px square, where modified.
    pub dot: Option<Rect>,
    /// The close button's 16 px square.
    pub close: Rect,
}

/// A document tab's width for a title `title` wide (its padding, icon,
/// title, dot where `modified`, close button and the gaps between).
pub fn doc_tab_width(title: f32, modified: bool) -> f32 {
    let dot = if modified { DOC_TAB_GAP + marks::DOT } else { 0.0 };
    DOC_TAB_PADDING.0
        + metrics::ICON
        + DOC_TAB_GAP
        + title
        + dot
        + DOC_TAB_GAP
        + DOC_TAB_CLOSE
        + DOC_TAB_PADDING.1
}

/// The parts of a document tab in `tab` whose title is `title` wide,
/// centered on the tab's height; laid out from the left, so that a tab
/// narrower than [`doc_tab_width`] keeps its icon and title and loses
/// the room at its right end.
pub fn doc_tab_parts(tab: Rect, title: f32, modified: bool) -> DocTabParts {
    let y = tab.center().y;
    let square =
        |x: f32, side: f32| Rect::from_min_size(pos2(x, y - side / 2.0), Vec2::splat(side));
    let mut x = tab.left() + DOC_TAB_PADDING.0;
    let icon = square(x, metrics::ICON);
    x += metrics::ICON + DOC_TAB_GAP;
    let title_rect = Rect::from_min_size(pos2(x, tab.top()), vec2(title, tab.height()));
    x += title;
    let dot = modified.then(|| {
        x += DOC_TAB_GAP;
        let r = square(x, marks::DOT);
        x += marks::DOT;
        r
    });
    x += DOC_TAB_GAP;
    let close = square(x, DOC_TAB_CLOSE);
    DocTabParts { icon, title: title_rect, dot, close }
}

/// Paints a document tab's kind icon in its square (`.doctab .ic`:
/// `text-3`, and `text-2` on the active tab).
pub fn paint_doc_tab_icon(
    painter: &egui::Painter,
    t: &Tokens,
    rect: Rect,
    icon: Icon,
    active: bool,
) {
    icon.paint(painter, rect, if active { t.text_2 } else { t.text_3 });
}

/// Paints a document tab's close button in its square (`.doctab .x`: a
/// 12 px cross in `text-2`; hovered, `raised` behind it and the cross in
/// `text`). Shown on the active tab and the tab hovered (the caller's
/// choice).
pub fn paint_doc_tab_close(painter: &egui::Painter, t: &Tokens, rect: Rect, state: ButtonState) {
    let (fill, color) = match state {
        ButtonState::Rest => (Color32::TRANSPARENT, t.text_2),
        ButtonState::Hovered | ButtonState::Pressed => (t.raised, t.text),
    };
    painter.rect_filled(rect, metrics::RADIUS, fill);
    let cross = Rect::from_center_size(rect.center(), Vec2::splat(metrics::ICON_SMALL));
    Icon::Close.paint(painter, cross, color);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The thresholds of section 5.2, each side of each, and the wide
    /// layout's (decided U1).
    #[test]
    fn the_shell_yields_at_the_measured_widths() {
        let at = |w: f32| Yields::of(w, 1000.0);
        // 1300: the build actions' words and Quick Open's cap.
        assert!(!at(1299.0).build_words && at(1300.0).build_words);
        assert_eq!(at(1299.0).quick_open_cap, QUICK_OPEN_NARROW);
        assert_eq!(at(1300.0).quick_open_cap, QUICK_OPEN_MAX);
        // 1400: the Right Dock, the switches and the game folder.
        let (under, at_1400) = (at(1399.0), at(1400.0));
        assert_eq!((under.right_dock_width, at_1400.right_dock_width), (280.0, 320.0));
        assert!(under.right_dock_narrow && !at_1400.right_dock_narrow);
        assert!(under.right_dock_tabbed && !at_1400.right_dock_tabbed);
        assert!(under.status_switch_icons && !at_1400.status_switch_icons);
        assert!(!under.game_folder && at_1400.game_folder);
        // (1300 and 1400 are each their own: the cap is back at 1300, the
        // Right Dock still narrow until 1400.)
        assert!(at(1300.0).right_dock_tabbed && at(1300.0).build_words);
        // 1500: the Left Dock folds below it.
        assert!(at(1499.0).left_dock_folded && !at(1500.0).left_dock_folded);
        assert!(at(1399.0).left_dock_folded);
        // 1280 is only the board's width: nothing changes there.
        assert_eq!(at(1279.0), at(1280.0));
        // At 1280, the narrow board: everything of 5.2 but the Quick Open
        // icon (a matter of room).
        let narrow = at(1280.0);
        assert!(!narrow.build_words && narrow.left_dock_folded && narrow.right_dock_tabbed);
        // The wide layout from 2560.
        assert!(!at(2559.0).wide && at(2560.0).wide);
        assert_eq!(at(2559.0).left_dock_width, metrics::LEFT_DOCK);
        assert_eq!(at(2560.0).left_dock_width, LEFT_DOCK_WIDE);
        assert!(!at(2559.0).right_dock_columns(600.0));
        assert!(at(2560.0).right_dock_columns(561.0) && !at(2560.0).right_dock_columns(560.0));
        assert_eq!(at(2559.0).right_dock_width, metrics::RIGHT_DOCK);
        assert_eq!(at(2560.0).right_dock_width, RIGHT_DOCK_WIDE);
        assert_eq!(at(2559.0).module_panel_height, metrics::MODULE_PANEL);
        assert_eq!(at(2560.0).module_panel_height, MODULE_PANEL_WIDE);
        // The columns: the board's 340 and 400 of 741; half each at the
        // least two columns take, neither under 280.
        assert_eq!(Yields::right_first_column(741.0), 340.0);
        assert_eq!(Yields::right_first_column(561.0), 280.0);
        assert!(561.0 - 1.0 - Yields::right_first_column(561.0) >= RIGHT_DOCK_NARROW);
        // The Right Dock folds to its strip under 797 (the Left Dock's
        // strip, the editor's least and the narrow dock side by side).
        assert_eq!(RIGHT_DOCK_STRIP_UNDER, 796.0);
        assert!(at(795.0).right_dock_folded && !at(796.0).right_dock_folded);
        assert!(!at(1280.0).right_dock_folded);
        // A wide window keeps everything.
        let wide = at(3440.0);
        assert!(wide.build_words && !wide.left_dock_folded && wide.game_folder);
        // Quick Open becomes an icon under 240 of room.
        assert!(Yields::quick_open_as_icon(239.0) && !Yields::quick_open_as_icon(240.0));
    }

    /// The editor's thresholds (the area header's, M4's to apply).
    #[test]
    fn the_editor_yields_at_its_widths() {
        let at = |e: f32| Yields::of(1600.0, e);
        assert!(at(989.0).header_compact && !at(990.0).header_compact);
        assert!(!at(549.0).value_text && at(550.0).value_text);
        assert!(at(489.0).into_more && !at(490.0).into_more);
        // At 1280 the editor is 974 wide: compact.
        assert!(Yields::of(1280.0, 974.0).header_compact);
    }

    /// A document tab's parts in order, the stylesheet's measures between
    /// them, and its width their sum.
    #[test]
    fn a_document_tab_s_parts() {
        let tab = Rect::from_min_size(pos2(100.0, 0.0), vec2(300.0, 28.0));
        let parts = doc_tab_parts(tab, 80.0, true);
        assert_eq!(parts.icon.left(), 110.0);
        assert_eq!(parts.title.left(), 132.0);
        let dot = parts.dot.unwrap();
        assert_eq!(dot.left(), 132.0 + 80.0 + 6.0);
        assert_eq!(parts.close.left(), dot.right() + 6.0);
        assert_eq!(parts.close.right() + 6.0 - tab.left(), doc_tab_width(80.0, true));
        assert_eq!(parts.icon.center().y, 14.0);
        let plain = doc_tab_parts(tab, 80.0, false);
        assert_eq!(plain.dot, None);
        assert_eq!(plain.close.left(), 132.0 + 80.0 + 6.0);
        assert_eq!(doc_tab_width(80.0, true) - doc_tab_width(80.0, false), 13.0);
    }
}
