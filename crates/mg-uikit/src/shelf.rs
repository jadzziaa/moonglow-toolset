//! An editor's tool shelf (proposal section 5, the mock's `.shelf`): a
//! vertical strip of the editor's modes or tools at the left of its body,
//! 36 px of `bar` with a 1 px `line` at its right; from the top, 4 px in
//! and 2 px apart, 28 px tools (frameless in `text-2` at rest, `raised`
//! and `text` under the pointer, `accent-fill` and `on-fill` while on,
//! `text-off` while disabled, not faded) and separators (20 by 1 of
//! `line-soft`, 3 px above and below).
//!
//! The shelf is one group named "Tools" (or its own name); each tool a
//! button named by its name, said on (selected) or not, with its tooltip
//! (its name where none is given; the keys that choose it, say).
//!
//! ```
//! # egui::__run_test_ui(|ui| {
//! use mg_uikit::shelf::{Tool, ToolShelf};
//! use mg_uikit::Icon;
//! let clicked = ToolShelf::new()
//!     .tool(Tool::new(Icon::Select, "Select Objects").on(true))
//!     .tool(Tool::new(Icon::SelectTiles, "Select Tiles").tip("Select tiles (F10)"))
//!     .tool(Tool::new(Icon::Paint, "Paint").tip("Paint (B)"))
//!     .separator()
//!     .tool(Tool::new(Icon::Terrain, "Terrain"))
//!     .show(ui)
//!     .clicked;
//! # let _ = clicked;
//! # });
//! ```

use egui::{Rect, Response, Sense, Stroke, Ui, Vec2, WidgetInfo, WidgetType, pos2, vec2};

use crate::icons::{ButtonState, Icon};
use crate::metrics;
use crate::theme::TokensExt as _;

/// The shelf's accessible name where it is not given one.
pub const TOOLS: &str = "Tools";
/// The room above the shelf's first tool (`.shelf`: `padding: 4px 0`).
pub const SHELF_PADDING: f32 = 4.0;
/// The gap between the shelf's tools (`.shelf`: `gap: 2px`).
pub const SHELF_GAP: f32 = 2.0;
/// A separator's line (`.shelf .sep`: 20 by 1).
pub const SHELF_SEP: f32 = 20.0;
/// The room above and below a separator's line (`margin: 3px 0`).
pub const SHELF_SEP_MARGIN: f32 = 3.0;

/// A tool of a [`ToolShelf`].
#[derive(Clone, Debug)]
#[must_use = "add it with ToolShelf::tool"]
pub struct Tool {
    icon: Icon,
    name: String,
    tip: Option<String>,
    on: bool,
    enabled: bool,
    drawn_as: Option<ButtonState>,
}

impl Tool {
    /// A tool showing `icon`, named `name` (Select Objects).
    pub fn new(icon: Icon, name: impl Into<String>) -> Self {
        Tool { icon, name: name.into(), tip: None, on: false, enabled: true, drawn_as: None }
    }

    /// Its tooltip ("Select tiles (F10)"); its name where not given.
    pub fn tip(mut self, tip: impl Into<String>) -> Self {
        self.tip = Some(tip.into());
        self
    }

    /// On (the tool in hand).
    pub fn on(mut self, on: bool) -> Self {
        self.on = on;
        self
    }

    /// Enabled (`true`, the default) or drawn in `text-off` and said
    /// disabled (Paint in a read-only area).
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Drawn in `state` whatever the pointer does (a catalog's hovered
    /// tool).
    pub fn drawn_as(mut self, state: ButtonState) -> Self {
        self.drawn_as = Some(state);
        self
    }
}

#[derive(Clone, Debug)]
enum Entry {
    Tool(Tool),
    Separator,
}

/// What a [`ToolShelf`] did.
#[derive(Debug)]
pub struct ShelfOutput {
    /// The shelf's response (its whole strip).
    pub response: Response,
    /// The tool clicked (its index among the tools, separators not
    /// counted).
    pub clicked: Option<usize>,
    /// Each tool's response, in order.
    pub tools: Vec<Response>,
}

/// An editor's tool shelf (module comment).
#[derive(Clone, Debug)]
#[must_use = "show it with .show(ui)"]
pub struct ToolShelf {
    name: String,
    entries: Vec<Entry>,
}

impl Default for ToolShelf {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolShelf {
    /// A shelf named "Tools".
    pub fn new() -> Self {
        ToolShelf { name: TOOLS.to_string(), entries: Vec::new() }
    }

    /// Named `name` for a screen reader (and tests) in place of "Tools".
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// A tool, after those given.
    pub fn tool(mut self, tool: Tool) -> Self {
        self.entries.push(Entry::Tool(tool));
        self
    }

    /// A separator, after the tools given.
    pub fn separator(mut self) -> Self {
        self.entries.push(Entry::Separator);
        self
    }

    /// Draws the shelf down the height of `ui` at its left (its width
    /// [`metrics::TOOL_SHELF`], its line included).
    pub fn show(self, ui: &mut Ui) -> ShelfOutput {
        let t = ui.tokens();
        let top = ui.max_rect().top();
        let left = ui.cursor().min.x;
        let rect = Rect::from_min_max(
            pos2(left, top),
            pos2(left + metrics::TOOL_SHELF, ui.max_rect().bottom()),
        );
        let response =
            ui.interact(rect, ui.id().with(("mg-uikit-shelf", &self.name)), Sense::hover());
        ui.painter().rect_filled(rect, 0.0, t.bar);
        let x = rect.right() - 0.5;
        ui.painter().vline(x, rect.y_range(), Stroke::new(1.0, t.line));
        let mut clicked = None;
        let mut tools = Vec::new();
        let inner = Rect::from_min_max(rect.min, pos2(rect.right() - 1.0, rect.bottom()));
        let builder = egui::UiBuilder::new()
            .max_rect(inner.shrink2(vec2(0.0, SHELF_PADDING)))
            .layout(egui::Layout::top_down(egui::Align::Center));
        ui.scope_builder(builder, |ui| {
            crate::icons::name_group(ui, &self.name);
            ui.spacing_mut().item_spacing.y = SHELF_GAP;
            for entry in self.entries {
                match entry {
                    Entry::Separator => {
                        let h = 1.0 + 2.0 * SHELF_SEP_MARGIN;
                        let (r, _) = ui.allocate_exact_size(vec2(SHELF_SEP, h), Sense::hover());
                        let y = r.center().y.round() - 0.5;
                        ui.painter().hline(r.x_range(), y, Stroke::new(1.0, t.line_soft));
                    }
                    Entry::Tool(tool) => {
                        let r = tool_ui(ui, &tool);
                        if r.clicked() {
                            clicked = Some(tools.len());
                        }
                        tools.push(r);
                    }
                }
            }
        });
        ui.advance_cursor_after_rect(rect);
        ShelfOutput { response, clicked, tools }
    }
}

/// A tool's button: 28 px square, painted in its state's tokens.
fn tool_ui(ui: &mut Ui, tool: &Tool) -> Response {
    let sense = if tool.enabled { Sense::click() } else { Sense::hover() };
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(metrics::TOOL_BUTTON), sense);
    let enabled = tool.enabled && ui.is_enabled();
    response.widget_info(|| WidgetInfo::selected(WidgetType::Button, enabled, tool.on, &tool.name));
    if ui.is_rect_visible(rect) {
        let t = ui.tokens();
        let state = tool.drawn_as.unwrap_or_else(|| match enabled {
            true => ButtonState::of(&response),
            false => ButtonState::Rest,
        });
        let (fill, color) = match (enabled, tool.on, state) {
            (false, _, _) => (egui::Color32::TRANSPARENT, t.text_off),
            (true, true, ButtonState::Rest) => (t.accent_fill, t.on_fill),
            (true, true, _) => (t.accent_fill_hover, t.on_fill),
            (true, false, ButtonState::Rest) => (egui::Color32::TRANSPARENT, t.text_2),
            (true, false, _) => (t.raised, t.text),
        };
        let ring = if response.has_focus() { Stroke::new(1.0, t.accent) } else { Stroke::NONE };
        let painter = crate::controls::box_painter(ui, enabled);
        painter.rect(rect, metrics::RADIUS, fill, ring, egui::StrokeKind::Inside);
        let square = Rect::from_center_size(rect.center(), Vec2::splat(metrics::ICON));
        tool.icon.paint(&painter, square, color);
    }
    let tip = tool.tip.clone().unwrap_or_else(|| tool.name.clone());
    response.on_hover_text(&tip).on_disabled_hover_text(&tip)
}
