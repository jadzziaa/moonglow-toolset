//! The drawn icon set: 192 icons in a 24 by 24 box, strokes 1.5 wide with
//! round caps and joins, no fill, drawn at 16 px (the spec's section 8).
//!
//! The set is `icons/icons.json` (each name's SVG markup), made into Rust
//! at build time (`build.rs`): [`Icon`], one variant a name, and each
//! icon's strokes flattened to polylines. [`Icon::paint`] draws one with
//! egui's painter at any size; [`icon`], [`icon_button`], [`icon_toggle`],
//! [`icon_text_button`] and [`icon_text_toggle`] are the widgets.

use egui::{Color32, Pos2, Rect, Response, Sense, Shape, Stroke, Ui, Vec2, WidgetInfo, WidgetType};

use crate::metrics;
use crate::theme::TokensExt as _;
use crate::tokens::Tokens;

/// One stroke of an icon, in its 24 by 24 box.
#[derive(Debug)]
struct IconShape {
    /// The last point joins the first.
    closed: bool,
    /// Filled as well as stroked.
    filled: bool,
    points: &'static [[f32; 2]],
}

include!(concat!(env!("OUT_DIR"), "/icons.rs"));

fn strokes(icon: Icon) -> &'static [IconShape] {
    SHAPES[icon as usize]
}

/// How sharp a corner of a stroke is before it is rounded with a dot (the
/// cosine of the turn): a turn of more than 60°.
const SHARP: f32 = 0.5;

impl Icon {
    /// The icon's strokes as polylines in its 24 by 24 box (`(points,
    /// closed)`), for tests and other painters.
    pub fn polylines(self) -> impl Iterator<Item = (&'static [[f32; 2]], bool)> {
        strokes(self).iter().map(|s| (s.points, s.closed))
    }

    /// Paints the icon into `rect` (its 24 box scaled to the square in the
    /// middle of it) in `color`: strokes 1.5 units wide, scaled, never
    /// under one physical pixel, with round caps and corners; the square's
    /// corner on a physical pixel, so that a 16 px icon is crisp.
    pub fn paint(self, painter: &egui::Painter, rect: Rect, color: Color32) {
        let ppp = painter.pixels_per_point();
        let side = rect.width().min(rect.height());
        let corner = rect.center() - Vec2::splat(side / 2.0);
        let corner = Pos2::new((corner.x * ppp).round() / ppp, (corner.y * ppp).round() / ppp);
        let scale = side / 24.0;
        let width = (metrics::ICON_STROKE * scale).max(1.0 / ppp);
        let stroke = Stroke::new(width, color);
        let at = |p: &[f32; 2]| corner + Vec2::new(p[0], p[1]) * scale;
        let mut shapes = Vec::new();
        for s in strokes(self) {
            let points: Vec<Pos2> = s.points.iter().map(at).collect();
            if s.filled && points.len() > 2 {
                shapes.push(Shape::convex_polygon(points.clone(), color, Stroke::NONE));
            }
            // Round caps and sharp corners: a dot as wide as the stroke
            // (a path of `h.01`, a dot, is all cap).
            let n = points.len();
            for i in 0..n {
                let end = !s.closed && (i == 0 || i == n - 1);
                let corner = if s.closed || (i > 0 && i < n - 1) {
                    let prev = points[(i + n - 1) % n];
                    let next = points[(i + 1) % n];
                    let (a, b) = ((points[i] - prev).normalized(), (next - points[i]).normalized());
                    a.dot(b) < SHARP
                } else {
                    false
                };
                if end || corner {
                    shapes.push(Shape::circle_filled(points[i], width / 2.0, color));
                }
            }
            if s.closed {
                shapes.push(Shape::closed_line(points, stroke));
            } else {
                shapes.push(Shape::line(points, stroke));
            }
        }
        painter.extend(shapes);
    }
}

/// An icon, 16 px, in the current text color; not interactive.
pub fn icon(ui: &mut Ui, icon: Icon) -> Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(metrics::ICON), Sense::hover());
    if ui.is_rect_visible(rect) {
        icon.paint(ui.painter(), rect, ui.visuals().text_color());
    }
    response
}

/// How an icon button is drawn in a state ([`paint_icon_button`]).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ButtonState {
    /// At rest.
    #[default]
    Rest,
    /// Under the pointer.
    Hovered,
    /// Pressed, or holding the keyboard's focus.
    Pressed,
}

impl ButtonState {
    /// The state a widget's response is in.
    pub fn of(response: &Response) -> Self {
        if response.is_pointer_button_down_on() || response.has_focus() {
            ButtonState::Pressed
        } else if response.hovered() || response.highlighted() {
            ButtonState::Hovered
        } else {
            ButtonState::Rest
        }
    }
}

/// Paints an icon button (the mock's `.btn-icon`; a toggle's `.toggle`
/// when `on` is given) in `rect`: frameless at rest, the icon in `text-2`;
/// hovered, `raised` and the icon in `text`; pressed or focused, an accent
/// ring; on, `accent-fill` and the icon in `on-fill`.
pub fn paint_icon_button(
    painter: &egui::Painter,
    tokens: &Tokens,
    rect: Rect,
    icon: Icon,
    state: ButtonState,
    on: bool,
) {
    let t = tokens;
    let radius = metrics::RADIUS;
    let (fill, color) = match (on, state) {
        (true, ButtonState::Rest) => (t.accent_fill, t.on_fill),
        (true, _) => (t.accent_fill_hover, t.on_fill),
        (false, ButtonState::Rest) => (Color32::TRANSPARENT, t.text_2),
        (false, ButtonState::Hovered) => (t.raised, t.text),
        (false, ButtonState::Pressed) => (t.raised_hover, t.text),
    };
    let ring =
        if state == ButtonState::Pressed { Stroke::new(1.0, t.accent) } else { Stroke::NONE };
    painter.rect(rect, radius, fill, ring, egui::StrokeKind::Inside);
    let square = Rect::from_center_size(rect.center(), Vec2::splat(metrics::ICON));
    icon.paint(painter, square, color);
}

fn icon_button_ui(ui: &mut Ui, icon: Icon, tooltip: &str, on: Option<bool>) -> Response {
    let size = Vec2::splat(metrics::ICON_BUTTON);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    let enabled = ui.is_enabled();
    response.widget_info(|| match on {
        Some(on) => WidgetInfo::selected(WidgetType::Button, enabled, on, tooltip),
        None => WidgetInfo::labeled(WidgetType::Button, enabled, tooltip),
    });
    if ui.is_rect_visible(rect) {
        let tokens = ui.tokens();
        let state = ButtonState::of(&response);
        paint_icon_button(ui.painter(), &tokens, rect, icon, state, on == Some(true));
    }
    response.on_hover_text(tooltip).on_disabled_hover_text(tooltip)
}

/// An icon button: 22 px square, frameless until hovered (the mock's
/// `.btn-icon`). `tooltip` is shown on hover, also while disabled, and is
/// its accessible name, by which tests and screen readers find it.
pub fn icon_button(ui: &mut Ui, icon: Icon, tooltip: &str) -> Response {
    icon_button_ui(ui, icon, tooltip, None)
}

/// An icon button that toggles `on` when clicked (the mock's `.toggle`
/// with an icon): `accent-fill` while on. Changed when clicked.
pub fn icon_toggle(ui: &mut Ui, on: &mut bool, icon: Icon, tooltip: &str) -> Response {
    let mut response = icon_button_ui(ui, icon, tooltip, Some(*on));
    if response.clicked() {
        *on = !*on;
        response.mark_changed();
    }
    response
}

fn icon_text_button_ui(ui: &mut Ui, icon: Icon, text: &str, selected: Option<bool>) -> Response {
    let id = ui.next_auto_id().with("mg-uikit-icon");
    let side = Vec2::splat(metrics::ICON);
    let mut button = egui::Button::new((egui::Atom::custom(id, side), text));
    if let Some(selected) = selected {
        button = button.selected(selected);
    }
    let atoms = button.atom_ui(ui);
    if let Some(rect) = atoms.rect(id) {
        let r = &atoms.response;
        let visuals = ui.style().interact_selectable(r, selected == Some(true));
        icon.paint(ui.painter(), rect, visuals.text_color());
    }
    atoms.response
}

/// A button with an icon and words (`Save`), egui's own button: its
/// accessible name is the words alone.
pub fn icon_text_button(ui: &mut Ui, icon: Icon, text: &str) -> Response {
    icon_text_button_ui(ui, icon, text, None)
}

/// A button with an icon and words that toggles `on` when clicked
/// (`accent-fill` while on, egui's selected button). Changed when clicked.
pub fn icon_text_toggle(ui: &mut Ui, on: &mut bool, icon: Icon, text: &str) -> Response {
    let mut response = icon_text_button_ui(ui, icon, text, Some(*on));
    if response.clicked() {
        *on = !*on;
        response.mark_changed();
    }
    response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_set_has_192_icons_named_as_in_the_file() {
        assert_eq!(Icon::ALL.len(), 192);
        let json = include_str!("../icons/icons.json");
        for icon in Icon::ALL {
            assert_eq!(Icon::from_name(icon.name()), Some(icon));
            assert!(json.contains(&format!("\"{}\":", icon.name())), "{}", icon.name());
        }
        assert_eq!(Icon::from_name("no-such-icon"), None);
        assert_eq!(Icon::ChevronDown.name(), "chevron-down");
        assert_eq!(Icon::from_name("eye-off"), Some(Icon::EyeOff));
    }

    /// Every icon has strokes, and they stay inside the 24 box with their
    /// width (0.75 at each side).
    #[test]
    fn every_icon_has_strokes_inside_its_box() {
        let half = metrics::ICON_STROKE / 2.0;
        for icon in Icon::ALL {
            let polys: Vec<_> = icon.polylines().collect();
            assert!(!polys.is_empty(), "{} has no strokes", icon.name());
            for (points, _) in polys {
                assert!(!points.is_empty(), "{}: an empty stroke", icon.name());
                for &[x, y] in points {
                    let inside = |v: f32| (half - 0.001..=24.0 - half + 0.001).contains(&v);
                    assert!(inside(x) && inside(y), "{}: ({x}, {y}) outside", icon.name());
                }
            }
        }
    }

    /// The build read the file as the tests' copy of the parser does.
    #[test]
    fn the_build_s_strokes_are_the_parser_s() {
        let json = include_str!("../icons/icons.json");
        for icon in [Icon::Ability, Icon::Info, Icon::Settings, Icon::Grid] {
            let start = json.find(&format!("\"{}\": \"", icon.name())).unwrap();
            let markup = &json[start + icon.name().len() + 5..];
            let markup = &markup[..markup.find("\",\n").or_else(|| markup.find("\"\n")).unwrap()];
            let markup = markup.replace("\\\"", "\"");
            let parsed = crate::svg::parse_icon(&markup).unwrap();
            let built: Vec<_> = icon.polylines().collect();
            assert_eq!(parsed.len(), built.len(), "{}", icon.name());
            for (p, (points, closed)) in parsed.iter().zip(built) {
                assert_eq!(p.closed, closed);
                assert_eq!(p.points.len(), points.len());
                for (a, b) in p.points.iter().zip(points) {
                    assert!((a[0] - b[0]).abs() < 0.001 && (a[1] - b[1]).abs() < 0.001);
                }
            }
        }
    }
}
