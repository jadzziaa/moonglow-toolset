//! What stands over a 3D view (proposal section 9.1, the mock's
//! `.vp-tr`, `.compass` and `.vp-btn`): the compass at the view's top
//! right, the view's corner buttons under it (Reorient Camera, Go to
//! Start Location), each drawn from the tokens, so that the light theme
//! has them as the Light Theme board does (section 4.2: the compass ring
//! `overlay` with a `line-soft` edge, its ticks `text-3`, its halves
//! `compass-north` and `compass-south`, the Main board's colors in the
//! dark theme and the light board's `error` and `text-3` in the light,
//! the N `text`).
//!
//! The corner holds its parts in a column 8 px in from the view's top
//! right, 4 px apart, centered on the compass ([`corner_column`] places
//! them).
//!
//! ```
//! # egui::__run_test_ui(|ui| {
//! use mg_uikit::overlay::{ViewButton, paint_compass};
//! use mg_uikit::Icon;
//! let t = mg_uikit::tokens(ui.ctx());
//! paint_compass(ui.painter(), &t, egui::pos2(40.0, 40.0), 0.3);
//! let reorient = ui.add(ViewButton::new(Icon::Camera, "Reorient Camera"));
//! # let _ = reorient;
//! # });
//! ```

use egui::{
    FontId, Painter, Pos2, Rect, Response, Sense, Shape, Stroke, Ui, Vec2, Widget, WidgetInfo,
    WidgetType, pos2, vec2,
};

use crate::icons::{ButtonState, Icon};
use crate::metrics;
use crate::theme::TokensExt as _;
use crate::tokens::Tokens;

/// The compass's side (`.compass`: 56 by 56).
pub const COMPASS: f32 = 56.0;
/// A view button's side (`.vp-btn`: 24 by 24).
pub const VIEW_BUTTON: f32 = 24.0;
/// How far the corner's column is from the view's edges (`.vp-tr`:
/// `top: 8px; right: 8px`).
pub const CORNER_INSET: f32 = 8.0;
/// The gap between the corner's parts (`.vp-tr`: `gap: 4px`).
pub const CORNER_GAP: f32 = 4.0;

/// Paints the compass (the mock's 56 point `.compass`) centered on
/// `center`: its ring, the four ticks, and the needle turned so that its
/// north half (with the N) points `north` radians clockwise from the
/// screen's up, where north is in the view.
pub fn paint_compass(painter: &Painter, t: &Tokens, center: Pos2, north: f32) {
    let scale = COMPASS / 56.0;
    painter.circle(center, 26.0 * scale, t.overlay, Stroke::new(1.0, t.line_soft));
    let tick = Stroke::new(1.5, t.text_3);
    for k in 0..4 {
        let d = Vec2::angled(k as f32 * std::f32::consts::FRAC_PI_2);
        painter.line_segment([center + d * 19.0 * scale, center + d * 23.0 * scale], tick);
    }
    // (The needle's points as the mock's, before it is turned: north up.)
    let turn = |x: f32, y: f32| {
        let (s, c) = north.sin_cos();
        center + vec2(x * c - y * s, x * s + y * c) * scale
    };
    let north_half = vec![turn(0.0, -16.0), turn(4.0, 0.0), turn(-4.0, 0.0)];
    let south_half = vec![turn(0.0, 16.0), turn(4.0, 0.0), turn(-4.0, 0.0)];
    painter.add(Shape::convex_polygon(north_half, t.compass_north, Stroke::NONE));
    painter.add(Shape::convex_polygon(south_half, t.compass_south, Stroke::NONE));
    // The N, turned with the needle, at its north tip (the mock's
    // `y="22" dy="-4"`: its baseline 10 above the middle, its middle some
    // 13).
    let font = FontId::proportional(9.0 * scale);
    // (In `text`: it stands on the disc, `overlay`, the needle under
    // its middle a point wide; `on-fill` was white on the light theme's
    // near-white disc.)
    let galley = painter.layout_no_wrap("N".to_string(), font, t.text);
    let at = turn(0.0, -13.5);
    let half = galley.size() / 2.0;
    let (s, c) = north.sin_cos();
    // (A text shape turns about its top left: placed so its middle is
    // `at`.)
    let offset = vec2(half.x * c - half.y * s, half.x * s + half.y * c);
    let shape = egui::epaint::TextShape::new(at - offset, galley, t.text).with_angle(north);
    painter.add(shape);
}

/// The rectangles of the corner's column at `view`'s top right: the
/// compass's, then each of `buttons` view buttons' under it, centered on
/// it ([`CORNER_GAP`] apart).
pub fn corner_column(view: Rect, buttons: usize) -> (Rect, Vec<Rect>) {
    let compass = Rect::from_min_size(
        pos2(view.right() - CORNER_INSET - COMPASS, view.top() + CORNER_INSET),
        Vec2::splat(COMPASS),
    );
    let mut y = compass.bottom() + CORNER_GAP;
    let rects = (0..buttons)
        .map(|_| {
            let r = Rect::from_center_size(
                pos2(compass.center().x, y + VIEW_BUTTON / 2.0),
                Vec2::splat(VIEW_BUTTON),
            );
            y += VIEW_BUTTON + CORNER_GAP;
            r
        })
        .collect();
    (compass, rects)
}

/// A button over a 3D view (`.vp-btn`): 24 points square, `overlay` with
/// a `line-soft` edge and its icon in `text`, `raised` under the pointer,
/// an `accent` ring while it has the keyboard's focus; named by its name,
/// its tooltip its tip (its name where none is given).
#[derive(Clone, Debug)]
#[must_use = "add it with ui.add or ui.put"]
pub struct ViewButton {
    icon: Icon,
    name: String,
    tip: Option<String>,
    enabled: bool,
}

impl ViewButton {
    /// A button showing `icon`, named `name` ("Reorient Camera").
    pub fn new(icon: Icon, name: impl Into<String>) -> Self {
        ViewButton { icon, name: name.into(), tip: None, enabled: true }
    }

    /// Its tooltip (the keys that do it, say).
    pub fn tip(mut self, tip: impl Into<String>) -> Self {
        self.tip = Some(tip.into());
        self
    }

    /// Enabled (`true`, the default), or drawn in `text-off`.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

impl Widget for ViewButton {
    fn ui(self, ui: &mut Ui) -> Response {
        let sense = if self.enabled { Sense::click() } else { Sense::hover() };
        let (rect, response) = ui.allocate_exact_size(Vec2::splat(VIEW_BUTTON), sense);
        let enabled = self.enabled && ui.is_enabled();
        response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, enabled, &self.name));
        if ui.is_rect_visible(rect) {
            let t = ui.tokens();
            let state = if enabled { ButtonState::of(&response) } else { ButtonState::Rest };
            let fill = match state {
                ButtonState::Rest => t.overlay,
                _ => t.raised,
            };
            let painter = crate::controls::box_painter(ui, enabled);
            let edge = if response.has_focus() { t.accent } else { t.line_soft };
            painter.rect(
                rect,
                metrics::RADIUS,
                fill,
                Stroke::new(1.0, edge),
                egui::StrokeKind::Inside,
            );
            let square = Rect::from_center_size(rect.center(), Vec2::splat(metrics::ICON));
            self.icon.paint(&painter, square, if enabled { t.text } else { t.text_off });
        }
        let tip = self.tip.unwrap_or_else(|| self.name.clone());
        response.on_hover_text(&tip).on_disabled_hover_text(&tip)
    }
}
