use egui::{Id, Ui, WidgetText};

use crate::{NodePath, TabStyle};

/// Defines how a tab should behave and be rendered inside a [`Tree`](crate::Tree).
pub trait TabViewer {
    /// The type of tab in which you can store state to be drawn in your tabs.
    type Tab;

    /// Unique ID for this tab.
    ///
    /// Assuming `Tab` implements `Hash`, you can simply do `Id::new(tab)`.
    /// If that is not possible, but tab names are guaranteed to be unique,
    /// one could also implement this as `Id::new(self.title(tab).text())`.
    fn id(&mut self, tab: &mut Self::Tab) -> Id;

    /// The title to be displayed in the tab bar.
    fn title(&mut self, tab: &mut Self::Tab) -> WidgetText;

    /// Actual tab content.
    fn ui(&mut self, ui: &mut Ui, tab: &mut Self::Tab);

    /// Content inside the context menu shown when the tab is right-clicked.
    ///
    /// `_path` specifies which [`Surface`](crate::Surface) and [`Node`](crate::Node)
    /// that this particular context menu belongs to.
    fn context_menu(&mut self, _ui: &mut Ui, _tab: &mut Self::Tab, _path: NodePath) {}

    /// Called after each tab button is shown, so you can add a tooltip, check for clicks, etc.
    fn on_tab_button(&mut self, _tab: &mut Self::Tab, _response: &egui::Response) {}

    /// (Moonglow's patch.) Called for the tab being dragged, after its
    /// title is drawn on `layer` (a layer of its own) in `rect`, where the
    /// tab stood, and before the layer is moved to follow the pointer:
    /// what is painted there now moves with the title.
    fn on_tab_dragged(
        &mut self,
        _ctx: &egui::Context,
        _tab: &mut Self::Tab,
        _layer: egui::LayerId,
        _rect: egui::Rect,
    ) {
    }

    /// (Moonglow's patch.) Paints one of a window's tab bar buttons, the
    /// collapse arrow at its left or the close all at its right, in
    /// `response.rect`, after egui_dock has taken its clicks and before it
    /// paints it: true when it was painted here, and egui_dock then paints
    /// nothing of it (its fill, symbol and border) and gives it no tip of
    /// its own. What it does is egui_dock's either way. `response` may be
    /// given a name and a tip (`Response::widget_info`, `on_hover_text`).
    fn window_bar_button(
        &mut self,
        _ui: &mut Ui,
        _button: WindowBarButton,
        _response: &egui::Response,
    ) -> bool {
        false
    }

    /// This is called when the `_tab` gets closed by the user.
    ///
    /// Returns an `OnCloseResponse` which determines what happens to the tab after this function gets called.
    fn on_close(&mut self, _tab: &mut Self::Tab) -> OnCloseResponse {
        OnCloseResponse::Close
    }

    /// Returns `true` if the user of your app should be able to close a given `_tab`.
    ///
    /// By default, `true` is always returned.
    fn is_closeable(&self, _tab: &Self::Tab) -> bool {
        true
    }

    /// Returns `true` if the user of your app should be able to close a given `_tab`.
    ///
    /// By default, `true` is always returned.
    #[deprecated = "Use the `TabViewer::is_closeable` function instead."]
    fn closeable(&mut self, _tab: &mut Self::Tab) -> bool {
        true
    }

    /// This is called every frame after [`ui`](Self::ui) is called, if the `_tab` is active.
    ///
    /// Returns `true` if the tab should be forced to close, `false` otherwise.
    ///
    /// In the event this function returns true the tab will be removed without calling `on_close`.
    fn force_close(&mut self, _tab: &mut Self::Tab) -> bool {
        false
    }

    /// This is called when the add button is pressed.
    ///
    /// `_path` specifies which [`Surface`](crate::Surface) and on which
    /// [`Node`](crate::Node) this particular add button was pressed.
    fn on_add(&mut self, _path: NodePath) {}

    /// Called when the rectangle of the tab content changes.
    ///
    /// This can happen when the window is resized, panels are docked or undocked,
    /// or when the layout of the dock area is changed in any way that affects
    /// the available space for the tab content.
    ///
    /// This is useful for tabs that need to adjust their content based on the
    /// available space.
    fn on_rect_changed(&mut self, _tab: &mut Self::Tab) {}

    /// Content of the popup under the add button. Useful for selecting what type of tab to add.
    ///
    /// This requires that [`DockArea::show_add_buttons`](crate::DockArea::show_add_buttons) and
    /// [`DockArea::show_add_popup`](crate::DockArea::show_add_popup) are set to `true`.
    fn add_popup(&mut self, _ui: &mut Ui, _path: NodePath) {}

    /// Sets custom style for given tab.
    fn tab_style_override(&self, _tab: &Self::Tab, _global_style: &TabStyle) -> Option<TabStyle> {
        None
    }

    /// Specifies a tab's ability to be shown in a window.
    ///
    /// Returns `false` if this tab should never be turned into a window.
    fn allowed_in_windows(&self, _tab: &mut Self::Tab) -> bool {
        true
    }

    /// Whether the tab body will be cleared with the color specified in
    /// [`TabBarStyle::bg_fill`](crate::TabBarStyle::bg_fill).
    fn clear_background(&self, _tab: &Self::Tab) -> bool {
        true
    }

    /// Returns `true` if the horizontal and vertical scroll bars will be shown for `tab`.
    ///
    /// By default, both scroll bars are shown.
    fn scroll_bars(&self, _tab: &Self::Tab) -> [bool; 2] {
        [true, true]
    }
}

/// Determines what happens to a tab when a user attempts to close it.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum OnCloseResponse {
    /// Closes the tab.
    Close,
    /// Focuses on the tab.
    Focus,
    /// Ignores the close request.
    Ignore,
}

/// (Moonglow's patch.) Which of a window's tab bar buttons
/// [`TabViewer::window_bar_button`] paints.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindowBarButton {
    /// The arrow at the bar's left that collapses its pane to the bar
    /// (`collapsed`: it is, and the arrow opens it again).
    Collapse {
        /// Whether the pane is collapsed now.
        collapsed: bool,
        /// Whether the pane is the window's only one.
        whole_window: bool,
    },
    /// The close all at the bar's right: it closes the pane's tabs, the
    /// whole window where the pane is its only one (`whole_window`).
    CloseAll {
        /// Whether some tab of the pane can't be closed.
        disabled: bool,
        /// Whether the pane is the window's only one.
        whole_window: bool,
    },
}
