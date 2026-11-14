/// Due to there being a lot of code to show a dock in a ui every complementing
/// method to ``show`` and ``show_inside`` is put in ``show_extra``.
/// Otherwise ``mod.rs`` would be humongous.
mod show;

// Various components of the `DockArea` which is used when rendering
mod allowed_splits;
mod drag_and_drop;
mod state;
mod tab_removal;

pub use allowed_splits::AllowedSplits;
use egui::{Id, Modifiers, emath::*};
use tab_removal::TabRemoval;

use crate::{NodePath, Style, TabIndex, TabPath, dock_state::DockState};

/// Displays a [`DockState`] in `egui`.
pub struct DockArea<'tree, Tab> {
    id: Id,
    dock_state: &'tree mut DockState<Tab>,
    style: Option<Style>,
    show_add_popup: bool,
    show_add_buttons: bool,
    show_close_buttons: bool,
    tab_context_menus: bool,
    draggable_tabs: bool,
    hidable_tab_bars: bool,
    show_tab_name_on_hover: bool,
    show_window_close_buttons: bool,
    show_window_collapse_buttons: bool,
    show_leaf_close_all_buttons: bool,
    show_leaf_collapse_buttons: bool,
    main_surface_end_room: Option<f32>,
    // (Moonglow's patch: `DockArea::window_end_room`, `window_min_width`.)
    window_end_room: Option<f32>,
    window_min_width: Option<f32>,
    show_secondary_button_hint: bool,
    secondary_button_modifiers: Modifiers,
    secondary_button_on_modifier: bool,
    secondary_button_context_menu: bool,
    allowed_splits: AllowedSplits,
    window_bounds: Option<Rect>,

    to_remove: Vec<TabRemoval>,
    to_detach: Vec<TabPath>,
    new_focused: Option<NodePath>,
    tab_hover_rect: Option<(Rect, TabIndex)>,
}

// Builder
impl<'tree, Tab> DockArea<'tree, Tab> {
    /// Creates a new [`DockArea`] from the provided [`DockState`].
    #[inline(always)]
    pub fn new(tree: &'tree mut DockState<Tab>) -> DockArea<'tree, Tab> {
        Self {
            id: Id::new("egui_dock::DockArea"),
            dock_state: tree,
            style: None,
            show_add_popup: false,
            show_add_buttons: false,
            show_close_buttons: true,
            tab_context_menus: true,
            draggable_tabs: true,
            hidable_tab_bars: false,
            show_tab_name_on_hover: false,
            allowed_splits: AllowedSplits::default(),
            to_remove: Vec::new(),
            to_detach: Vec::new(),
            new_focused: None,
            tab_hover_rect: None,
            window_bounds: None,
            show_window_close_buttons: true,
            show_window_collapse_buttons: true,
            show_leaf_close_all_buttons: true,
            show_leaf_collapse_buttons: true,
            main_surface_end_room: None,
            window_end_room: None,
            window_min_width: None,
            show_secondary_button_hint: true,
            secondary_button_modifiers: Modifiers::SHIFT,
            secondary_button_on_modifier: true,
            secondary_button_context_menu: true,
        }
    }

    /// Sets the [`DockArea`] ID. Useful if you have more than one [`DockArea`].
    #[inline(always)]
    pub fn id(mut self, id: Id) -> Self {
        self.id = id;
        self
    }

    /// Sets the look and feel of the [`DockArea`].
    #[inline(always)]
    pub fn style(mut self, style: Style) -> Self {
        self.style = Some(style);
        self
    }

    /// Shows or hides the add button popup.
    /// By default it's `false`.
    pub fn show_add_popup(mut self, show_add_popup: bool) -> Self {
        self.show_add_popup = show_add_popup;
        self
    }

    /// Shows or hides the tab add buttons.
    /// By default it's `false`.
    pub fn show_add_buttons(mut self, show_add_buttons: bool) -> Self {
        self.show_add_buttons = show_add_buttons;
        self
    }

    /// Shows or hides the tab close buttons.
    /// By default it's `true`.
    pub fn show_close_buttons(mut self, show_close_buttons: bool) -> Self {
        self.show_close_buttons = show_close_buttons;
        self
    }

    /// Whether tabs show a context menu when right-clicked.
    /// By default it's `true`.
    pub fn tab_context_menus(mut self, tab_context_menus: bool) -> Self {
        self.tab_context_menus = tab_context_menus;
        self
    }

    /// Whether tabs can be dragged between nodes and reordered on the tab bar.
    /// By default it's `true`.
    pub fn draggable_tabs(mut self, draggable_tabs: bool) -> Self {
        self.draggable_tabs = draggable_tabs;
        self
    }

    /// Whether tab bars can be hidden with the context menu.
    /// By default it's `false`.
    pub fn hidable_tab_bars(mut self, hidable_tab_bars: bool) -> Self {
        self.hidable_tab_bars = hidable_tab_bars;
        self
    }

    /// Whether tabs show their name when hovered over them.
    /// By default it's `false`.
    pub fn show_tab_name_on_hover(mut self, show_tab_name_on_hover: bool) -> Self {
        self.show_tab_name_on_hover = show_tab_name_on_hover;
        self
    }

    /// What directions can a node be split in: left-right, top-bottom, all, or none.
    /// By default it's all.
    pub fn allowed_splits(mut self, allowed_splits: AllowedSplits) -> Self {
        self.allowed_splits = allowed_splits;
        self
    }

    /// Whether tooltip hints are shown for secondary buttons on tab bars.
    /// By default it's `true`.
    pub fn show_secondary_button_hint(mut self, show_secondary_button_hint: bool) -> Self {
        self.show_secondary_button_hint = show_secondary_button_hint;
        self
    }

    /// The key combination used to activate secondary buttons on tab bars.
    /// By default it's [`Modifiers::SHIFT`].
    pub fn secondary_button_modifiers(mut self, secondary_button_modifiers: Modifiers) -> Self {
        self.secondary_button_modifiers = secondary_button_modifiers;
        self
    }

    /// Whether the secondary buttons on tab bars are activated by the modifier key.
    /// By default it's `true`.
    pub fn secondary_button_on_modifier(mut self, secondary_button_on_modifier: bool) -> Self {
        self.secondary_button_on_modifier = secondary_button_on_modifier;
        self
    }

    /// Whether the secondary buttons on tab bars are activated from a context value by right-clicking primary buttons.
    /// By default it's `true`.
    pub fn secondary_button_context_menu(mut self, secondary_button_context_menu: bool) -> Self {
        self.secondary_button_context_menu = secondary_button_context_menu;
        self
    }

    /// The bounds for any windows inside the [`DockArea`]. Defaults to the screen rect.
    /// By default it's set to [`egui::Context::content_rect`].
    #[inline(always)]
    pub fn window_bounds(mut self, bounds: Rect) -> Self {
        self.window_bounds = Some(bounds);
        self
    }

    /// Enables or disables the close button on windows.
    /// By default it's `true`.
    #[inline(always)]
    #[deprecated = "consider using `show_leaf_close_buttons` instead."]
    pub fn show_window_close_buttons(mut self, show_window_close_buttons: bool) -> Self {
        self.show_window_close_buttons = show_window_close_buttons;
        self
    }

    /// Enables or disables the collapsing header on windows.
    /// By default it's `true`.
    #[inline(always)]
    #[deprecated = "consider using `show_leaf_collapse_buttons` instead."]
    pub fn show_window_collapse_buttons(mut self, show_window_collapse_buttons: bool) -> Self {
        self.show_window_collapse_buttons = show_window_collapse_buttons;
        self
    }

    /// Enables or disables the close all tabs button on tab bars.
    /// By default it's `true`.
    #[inline(always)]
    pub fn show_leaf_close_all_buttons(mut self, show_leaf_close_all_buttons: bool) -> Self {
        self.show_leaf_close_all_buttons = show_leaf_close_all_buttons;
        self
    }

    /// Enables or disables the collapse tabs button on tab bars.
    /// By default it's `true`.
    #[inline(always)]
    pub fn show_leaf_collapse_buttons(mut self, show_leaf_collapse_buttons: bool) -> Self {
        self.show_leaf_collapse_buttons = show_leaf_collapse_buttons;
        self
    }

    /// (Moonglow's patch.) The main surface's tab bars without the leaf's
    /// collapse and close all tabs buttons, whatever
    /// [`DockArea::show_leaf_collapse_buttons`] and
    /// [`DockArea::show_leaf_close_all_buttons`] say (the windows' keep
    /// them), and `room` points left free at their right end, where the
    /// tabs do not go, for the application's own buttons. By default the
    /// main surface's bars are as the windows'.
    #[inline(always)]
    pub fn main_surface_end_room(mut self, room: f32) -> Self {
        self.main_surface_end_room = Some(room.max(0.0));
        self
    }

    /// (Moonglow's patch.) `room` points left free at the right end of a
    /// window's tab bars, before its close all tabs button, where the tabs
    /// do not go, for the application's own buttons. By default a
    /// window's tabs go up to its close all tabs button.
    #[inline(always)]
    pub fn window_end_room(mut self, room: f32) -> Self {
        self.window_end_room = Some(room.max(0.0));
        self
    }

    /// (Moonglow's patch.) The least width a window can be made (its whole
    /// width, frame and all): room for its bar's buttons and a tab. By
    /// default egui's.
    #[inline(always)]
    pub fn window_min_width(mut self, width: f32) -> Self {
        self.window_min_width = Some(width.max(0.0));
        self
    }
}

impl<Tab> std::fmt::Debug for DockArea<'_, Tab> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DockArea").finish_non_exhaustive()
    }
}
