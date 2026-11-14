//! The small marks the shell and the editors put beside words: the key cap
//! ([`KeyCap`], the mock's `.kbd`), the count or tag badge ([`Badge`],
//! `.badge`) and the modified dot ([`modified_dot`], `.dot-mod` and the
//! document tab's `.dot`). None of them is interactive: each is a label
//! for a screen reader (and tests), named by its words.
//!
//! Each is drawn by a paint function with its colors given ([`paint_key_cap`],
//! [`paint_badge`], [`paint_modified_dot`]), which a widget drawing its own
//! parts (a status switch's count, a hint's keys) calls, and as a widget
//! for `ui.add`.

use egui::{
    Color32, CornerRadius, FontId, Galley, Painter, Pos2, Rect, Response, Sense, Stroke, Ui, Vec2,
    WidgetInfo, WidgetType, vec2,
};
use std::sync::Arc;

use crate::metrics;
use crate::theme::TokensExt as _;
use crate::tokens::Tokens;

/// A key cap's and a badge's height (`.kbd`, `.badge`: 16 px).
pub const MARK_HEIGHT: f32 = 16.0;
/// A key cap's padding at each side (`.kbd`: `padding: 0 4px`).
pub const KEY_CAP_PADDING: f32 = 4.0;
/// A badge's padding at each side (`.badge`: `padding: 0 5px`).
pub const BADGE_PADDING: f32 = 5.0;
/// A badge's least width (`.badge`: `min-width: 16px`): one digit's badge
/// is round.
pub const BADGE_MIN_WIDTH: f32 = 16.0;
/// The modified dot's diameter (`.dot-mod`, `.doctab .dot`: 7 px).
pub const DOT: f32 = 7.0;

/// The small text of key caps and badges (11 px).
fn small_font() -> FontId {
    FontId::proportional(metrics::SMALL_SIZE)
}

/// `text` laid out in the small font, in `color`.
fn small_galley(painter: &Painter, text: &str, color: Color32) -> Arc<Galley> {
    painter.layout_no_wrap(text.to_owned(), small_font(), color)
}

/// `galley` drawn in the middle of `rect` (its own line box centered: the
/// 11 px face's 14 px line in the 16 px mark).
fn paint_centered(painter: &Painter, rect: Rect, galley: Arc<Galley>, color: Color32) {
    let pos = rect.center() - galley.size() / 2.0;
    let pos = Pos2::new(pos.x.round(), pos.y.round());
    painter.galley(pos, galley, color);
}

/// A key cap's colors in a theme: fill, border and text (`.kbd`: dark,
/// `raised`, `line-soft` and `text`; light, `inset`, `line-soft` and
/// `text-2`: the `key-bg` and `key-text` tokens).
pub fn key_cap_colors(t: &Tokens) -> (Color32, Color32, Color32) {
    (t.key_bg, t.line_soft, t.key_text)
}

/// The width a key cap of `text` takes (its words and 4 px at each side).
pub fn key_cap_width(ui: &Ui, text: &str) -> f32 {
    let galley = small_galley(ui.painter(), text, Color32::PLACEHOLDER);
    galley.size().x + 2.0 * KEY_CAP_PADDING
}

/// Paints a key cap of `text` in `rect` (16 px high): `key-bg`, a 1 px
/// `line-soft` border, radius 3, the words in 11 px `key-text`.
pub fn paint_key_cap(painter: &Painter, t: &Tokens, rect: Rect, text: &str) {
    let (fill, border, color) = key_cap_colors(t);
    painter.rect(rect, metrics::RADIUS, fill, Stroke::new(1.0, border), egui::StrokeKind::Inside);
    let galley = small_galley(painter, text, color);
    paint_centered(painter, rect, galley, color);
}

/// Paints a key cap of a key held down now in `rect` (`.kbd.held`, the
/// gizmo board's Shift while the handles show): `accent-fill` with its
/// border, the words in `on-fill`.
pub fn paint_held_key_cap(painter: &Painter, t: &Tokens, rect: Rect, text: &str) {
    let fill = t.accent_fill;
    painter.rect(rect, metrics::RADIUS, fill, Stroke::new(1.0, fill), egui::StrokeKind::Inside);
    let galley = small_galley(painter, text, t.on_fill);
    paint_centered(painter, rect, galley, t.on_fill);
}

/// A key cap (the mock's `.kbd`): a key's name in a small box, as the
/// Status Bar's hints and a tooltip's keys show them. Not interactive; a
/// label named by its words.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// ui.add(mg_uikit::KeyCap::new("Ctrl+P"));
/// # });
/// ```
#[derive(Clone, Debug)]
#[must_use = "add it with ui.add"]
pub struct KeyCap {
    text: String,
}

impl KeyCap {
    /// A key cap reading `text` (`Ctrl`, `Right Drag`, `F9`).
    pub fn new(text: impl Into<String>) -> Self {
        KeyCap { text: text.into() }
    }
}

impl egui::Widget for KeyCap {
    fn ui(self, ui: &mut Ui) -> Response {
        let size = vec2(key_cap_width(ui, &self.text), MARK_HEIGHT);
        let (rect, response) = ui.allocate_exact_size(size, Sense::hover());
        let text = self.text;
        response.widget_info(|| WidgetInfo::labeled(WidgetType::Label, ui.is_enabled(), &text));
        if ui.is_rect_visible(rect) {
            paint_key_cap(ui.painter(), &ui.tokens(), rect, &text);
        }
        response
    }
}

/// What a badge says, by its color (the mock's `.badge` and its
/// variants).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum BadgeKind {
    /// A plain count: `raised-hover` with `text` (`.badge`).
    #[default]
    Plain,
    /// `accent-fill` with `on-fill` (`.badge.accent`).
    Accent,
    /// Errors: `error` with `on-status` (`.badge.error`).
    Error,
    /// Warnings: `warn` with `on-status`.
    Warn,
    /// Success: `ok` with `on-status`.
    Ok,
    /// Information: `info` with `on-status`.
    Info,
}

impl BadgeKind {
    /// Every kind, plain first.
    pub const ALL: [BadgeKind; 6] = [
        BadgeKind::Plain,
        BadgeKind::Accent,
        BadgeKind::Error,
        BadgeKind::Warn,
        BadgeKind::Ok,
        BadgeKind::Info,
    ];

    /// Its fill and its text's color in a theme. The status colors take
    /// `on-status` (proposal section 4.2: dark text in the dark theme,
    /// white in the light).
    pub fn colors(self, t: &Tokens) -> (Color32, Color32) {
        match self {
            BadgeKind::Plain => (t.raised_hover, t.text),
            BadgeKind::Accent => (t.accent_fill, t.on_fill),
            BadgeKind::Error => (t.error, t.on_status),
            BadgeKind::Warn => (t.warn, t.on_status),
            BadgeKind::Ok => (t.ok, t.on_status),
            BadgeKind::Info => (t.info, t.on_status),
        }
    }
}

/// A count or a tag in a small filled shape (the mock's `.badge`): a
/// pill, or with [`Badge::tag`] a box of radius 3. Not interactive; a
/// label named by its words, unless it is painted inside a widget that
/// says its count otherwise (a status switch's).
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use mg_uikit::{Badge, BadgeKind};
/// ui.add(Badge::new("3").kind(BadgeKind::Error));
/// ui.add(Badge::new("NEW").kind(BadgeKind::Info).tag());
/// # });
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
#[must_use = "add it with ui.add"]
pub struct Badge {
    text: String,
    kind: BadgeKind,
    tag: bool,
}

impl Badge {
    /// A plain badge reading `text`.
    pub fn new(text: impl Into<String>) -> Self {
        Badge { text: text.into(), kind: BadgeKind::Plain, tag: false }
    }

    /// A plain badge of a count.
    pub fn count(n: usize) -> Self {
        Self::new(n.to_string())
    }

    /// Colored as `kind` says.
    pub fn kind(mut self, kind: BadgeKind) -> Self {
        self.kind = kind;
        self
    }

    /// A tag's shape (`.badge.tag`): a box of radius 3, not a pill.
    pub fn tag(mut self) -> Self {
        self.tag = true;
        self
    }

    /// Its words.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Its kind.
    pub fn badge_kind(&self) -> BadgeKind {
        self.kind
    }

    /// The width it takes (its words and 5 px at each side, at least 16).
    pub fn width(&self, ui: &Ui) -> f32 {
        let galley = small_galley(ui.painter(), &self.text, Color32::PLACEHOLDER);
        (galley.size().x + 2.0 * BADGE_PADDING).max(BADGE_MIN_WIDTH)
    }

    /// Paints it in `rect` (16 px high) in the tokens `t`.
    pub fn paint(&self, painter: &Painter, t: &Tokens, rect: Rect) {
        paint_badge(painter, t, rect, &self.text, self.kind, self.tag);
    }
}

/// Paints a badge of `text` in `rect`: `kind`'s fill as a pill (radius 8),
/// or a box of radius 3 for a `tag`, the words in 11 px in `kind`'s text
/// color.
pub fn paint_badge(
    painter: &Painter,
    t: &Tokens,
    rect: Rect,
    text: &str,
    kind: BadgeKind,
    tag: bool,
) {
    let (fill, color) = kind.colors(t);
    let radius = if tag { CornerRadius::same(metrics::RADIUS) } else { CornerRadius::same(8) };
    painter.rect_filled(rect, radius, fill);
    let galley = small_galley(painter, text, color);
    paint_centered(painter, rect, galley, color);
}

impl egui::Widget for Badge {
    fn ui(self, ui: &mut Ui) -> Response {
        let size = vec2(self.width(ui), MARK_HEIGHT);
        let (rect, response) = ui.allocate_exact_size(size, Sense::hover());
        let text = self.text.clone();
        response.widget_info(|| WidgetInfo::labeled(WidgetType::Label, ui.is_enabled(), &text));
        if ui.is_rect_visible(rect) {
            self.paint(ui.painter(), &ui.tokens(), rect);
        }
        response
    }
}

/// Paints the modified dot (`.dot-mod`) centered on `center`: 7 px of
/// `select`, ringed in 1 px of `select-ring` where the theme has one (the
/// light theme: its `select` alone is 1.75:1 on the bar).
pub fn paint_modified_dot(painter: &Painter, t: &Tokens, center: Pos2) {
    let r = DOT / 2.0;
    if t.select_ring != Color32::TRANSPARENT {
        painter.circle_filled(center, r + 1.0, t.select_ring);
    }
    painter.circle_filled(center, r, t.select);
}

/// The modified dot as a widget: a 7 px square (the ring drawn past it in
/// the light theme), a label named "Modified".
pub fn modified_dot(ui: &mut Ui) -> Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(DOT), Sense::hover());
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Label, ui.is_enabled(), "Modified"));
    if ui.is_rect_visible(rect) {
        paint_modified_dot(ui.painter(), &ui.tokens(), rect.center());
    }
    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tokens::contrast;

    /// A badge's words on its fill meet 4.5:1 in both themes, every kind
    /// (the colored ones with `on-status`: dark text in the dark theme,
    /// white in the light, proposal section 4.2).
    #[test]
    fn a_badge_s_words_read_on_its_fill() {
        for (theme, t) in [("dark", Tokens::DARK), ("light", Tokens::LIGHT)] {
            for kind in BadgeKind::ALL {
                let (fill, text) = kind.colors(&t);
                let r = contrast(text, fill);
                assert!(r >= 4.5, "{theme}: {kind:?} is {r:.2}:1");
            }
        }
        // The colored ones' words: dark in the dark theme, white in the
        // light.
        let words = |t: &Tokens| BadgeKind::Error.colors(t).1;
        assert_eq!(words(&Tokens::DARK), Color32::from_rgb(0x15, 0x17, 0x1a));
        assert_eq!(words(&Tokens::LIGHT), Color32::WHITE);
    }

    /// A key cap's words on its fill, both themes.
    #[test]
    fn a_key_cap_s_words_read_on_its_fill() {
        for t in [Tokens::DARK, Tokens::LIGHT] {
            let (fill, _, text) = key_cap_colors(&t);
            assert!(contrast(text, fill) >= 4.5);
        }
        assert_eq!(key_cap_colors(&Tokens::DARK).0, Tokens::DARK.raised);
        assert_eq!(key_cap_colors(&Tokens::LIGHT).0, Tokens::LIGHT.inset);
    }
}
