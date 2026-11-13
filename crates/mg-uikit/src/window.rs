//! Windows and dialogs: a dialog's title row (the mock's `.dlg-bar`) and a
//! window's resize grip (`.fw-grip`), on egui alone.
//!
//! A dialog is an `egui::Window` with no title bar of egui's own
//! (`title_bar(false)`, its frame given no margin), its first
//! [`metrics::TITLE_ROW`] points kept for the row. As egui's own title bar,
//! the row is painted once the body is laid out, as wide as the window has
//! become ([`title_row`] in the rectangle kept for it). The title is the
//! window's own name (`egui::Window::new(title)`): the row paints it and
//! names nothing, so a screen reader and a test find the window by it once.
//!
//! ```
//! # egui::__run_test_ctx(|ctx| {
//! egui::Window::new("Rename")
//!     .title_bar(false)
//!     .frame(egui::Frame::window(&ctx.global_style()).inner_margin(0))
//!     .show(ctx, |ui| {
//!         let (kept, _) = ui.allocate_exact_size(
//!             egui::vec2(120.0, mg_uikit::metrics::TITLE_ROW),
//!             egui::Sense::hover(),
//!         );
//!         ui.label("New name");
//!         let row = egui::Rect::from_x_y_ranges(ui.min_rect().x_range(), kept.y_range());
//!         let shown = mg_uikit::title_row(ui, row, "Rename", Some("Close the dialog (Esc)"));
//!         let _closed = shown.closed();
//!     });
//! # });
//! ```

use egui::{Color32, Pos2, Rect, Response, Sense, Stroke, Ui, Vec2, WidgetInfo, WidgetType};

use crate::icons::{ButtonState, Icon, paint_icon_button};
use crate::metrics;
use crate::theme::TokensExt;

/// The room at the left of a title row's title (the mock's `padding-left`).
pub const TITLE_PADDING: f32 = 12.0;

/// The room at the right of a title row's Close button.
pub const TITLE_END: f32 = 3.0;

/// The name of a title row's Close button, by which a screen reader and a
/// test find it.
pub const CLOSE_NAME: &str = "Close";

/// What [`title_row`] showed.
#[derive(Debug)]
pub struct TitleRow {
    /// The row's rectangle.
    pub rect: Rect,
    /// Its Close button's response, where it has one.
    pub close: Option<Response>,
}

impl TitleRow {
    /// Whether its Close button was clicked.
    pub fn closed(&self) -> bool {
        self.close.as_ref().is_some_and(Response::clicked)
    }
}

/// The least width of a title row for `title` and, where `close`, its
/// Close button: what a window that grows to its content keeps for it.
pub fn title_row_width(ui: &Ui, title: &str, close: bool) -> f32 {
    let galley = title_galley(ui, title, f32::INFINITY);
    let end = if close { 4.0 + metrics::ICON_BUTTON + TITLE_END } else { TITLE_PADDING };
    TITLE_PADDING + galley.size().x + end
}

/// A dialog's title row in `rect` (its window's whole width and
/// [`metrics::TITLE_ROW`] high, at its top): the `bar` fill, a `line`
/// under it, `title` at the left in the body's face, cut short with an
/// ellipsis where the row is narrower, and, where `close` gives its tip,
/// a Close button at the right ([`CLOSE_NAME`], an icon button). The row
/// takes no drag of its own: the window is moved by it (egui moves a
/// window with no title bar by any part not taken; keep the body's
/// drags from it to move it by the row alone). Without the theme
/// applied, the dark tokens.
pub fn title_row(ui: &mut Ui, rect: Rect, title: &str, close: Option<&str>) -> TitleRow {
    let t = ui.tokens();
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 0.0, t.bar);
    let px = 1.0 / ui.ctx().pixels_per_point();
    painter.hline(rect.x_range(), rect.bottom() - px / 2.0, Stroke::new(px, t.line));
    let button = Vec2::splat(metrics::ICON_BUTTON);
    let close_rect = close.map(|_| {
        Rect::from_min_size(
            Pos2::new(rect.right() - TITLE_END - button.x, rect.center().y - button.y / 2.0),
            button,
        )
    });
    let words_end = close_rect.map_or(rect.right() - TITLE_PADDING, |r| r.left() - 4.0);
    let galley = title_galley(ui, title, (words_end - rect.left() - TITLE_PADDING).max(1.0));
    let at = Pos2::new(rect.left() + TITLE_PADDING, rect.center().y - galley.size().y / 2.0);
    painter.galley(at, galley, t.text);
    let close = close.zip(close_rect).map(|(tip, r)| {
        let id = ui.id().with(("mg-uikit-title-close", title));
        let response = ui.interact(r, id, Sense::click());
        let enabled = ui.is_enabled();
        response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, enabled, CLOSE_NAME));
        if ui.is_rect_visible(r) {
            paint_icon_button(ui.painter(), &t, r, Icon::Close, ButtonState::of(&response), false);
        }
        response.on_hover_text(tip)
    });
    TitleRow { rect, close }
}

/// The title laid out in the body's face, cut to `max` with an ellipsis.
fn title_galley(ui: &Ui, title: &str, max: f32) -> std::sync::Arc<egui::Galley> {
    let font = egui::TextStyle::Body.resolve(ui.style());
    let mut job = egui::text::LayoutJob::single_section(
        title.to_string(),
        egui::TextFormat { font_id: font, color: Color32::PLACEHOLDER, ..Default::default() },
    );
    job.wrap = egui::text::TextWrapping {
        max_width: max,
        max_rows: 1,
        break_anywhere: true,
        overflow_character: Some('…'),
    };
    ui.fonts_mut(|f| f.layout_job(job))
}

/// The side of a window's resize grip.
pub const GRIP: f32 = 12.0;

/// Paints a window's resize grip (the mock's `.fw-grip`: two short
/// diagonal strokes) in the bottom right corner of `window` (its outer
/// rectangle), in `text-off`, or `text-2` where `hot` (the corner under
/// the pointer, or being dragged). The grip is a mark only: the window's
/// own edges and corner resize it.
pub fn paint_resize_grip(painter: &egui::Painter, window: Rect, hot: bool) {
    let t = crate::tokens(painter.ctx());
    let color = if hot { t.text_2 } else { t.text_off };
    let corner = window.max - Vec2::splat(1.0 + GRIP);
    let at = |x: f32, y: f32| corner + Vec2::new(x, y);
    let stroke = Stroke::new(1.0, color);
    painter.line_segment([at(11.0, 4.0), at(4.0, 11.0)], stroke);
    painter.line_segment([at(11.0, 8.0), at(8.0, 11.0)], stroke);
}

/// The rectangle a window's grip ([`paint_resize_grip`]) is painted in.
pub fn resize_grip_rect(window: Rect) -> Rect {
    Rect::from_min_size(window.max - Vec2::splat(1.0 + GRIP), Vec2::splat(GRIP))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_grip_sits_in_the_window_s_corner() {
        let window = Rect::from_min_size(Pos2::new(100.0, 50.0), egui::vec2(400.0, 300.0));
        let grip = resize_grip_rect(window);
        assert!(window.contains_rect(grip));
        assert_eq!(grip.max, window.max - Vec2::splat(1.0));
    }
}
