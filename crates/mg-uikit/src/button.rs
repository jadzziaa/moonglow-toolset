//! The kit's own button (the mock's `.btn`, `.btn.flat`, `.btn.primary`,
//! `.btn-icon` and `.toggle`), drawn from the tokens in every state and
//! measured before it is laid out, for the parts that lay themselves out
//! against the width they have: the editor header ([`crate::header`]),
//! a popover's row unfolded into it ([`crate::popover`]).
//!
//! Unlike egui's button (and the kit's [`crate::IconButton`], which is
//! egui's), a disabled `KitButton` does not fade whole: it is drawn in
//! the disabled tokens (`text-off` on `raised`, or on nothing where it is
//! flat; a toggle that is on in `accent-fill-off` and `on-fill-off`),
//! with any opacity a parent gave kept (a fading window's).

use egui::{
    Color32, FontId, Rect, Response, Sense, Stroke, Ui, Vec2, WidgetInfo, WidgetType, pos2, vec2,
};

use crate::icons::{ButtonState, Icon};
use crate::metrics;
use crate::theme::TokensExt as _;
use crate::tokens::Tokens;

/// The room between a button's icon and its words, and between its words
/// and its chevron (`.btn`: `gap: 6px`).
pub const BUTTON_GAP: f32 = 6.0;
/// How far a word button's icon reaches into its padding, and its
/// chevron into the padding at its right (`.btn .ic { margin-left: -2px
/// }`, `.btn .chev { margin: 0 -2px 0 0 }`).
pub const BUTTON_INSET: f32 = 2.0;
/// The padding at each side of an icon alone with a chevron (a popover's
/// button where its words do not fit: the narrow board's Lighting).
pub const ICON_CHEVRON_PADDING: f32 = 4.0;
/// The room between an icon alone and its chevron.
pub const ICON_CHEVRON_GAP: f32 = 2.0;
/// The room between a button's words and its weak count (`.count-w`:
/// the button's gap, `margin-left: -2px`).
pub const COUNT_GAP: f32 = BUTTON_GAP - 2.0;
/// The least room between a button's words and its keys at its right end
/// (`.btn .k { margin-left: auto }`, after the button's gap).
pub const KEYS_GAP: f32 = 2.0 * BUTTON_GAP;

/// How a [`KitButton`] looks at rest.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ButtonLook {
    /// A word button (`.btn`): `raised`, `raised-hover` under the pointer;
    /// an icon alone so is framed as the words were (the narrow board's
    /// compact Lighting and Area Properties), the icon in `text`.
    #[default]
    Raised,
    /// Frameless until hovered (`.btn.flat`, `.btn-icon`): an icon alone
    /// in `text-2`, words in `text`.
    Flat,
    /// A toggle (`.toggle`): `raised` while off, `accent-fill` while on
    /// ([`KitButton::on`]).
    Toggle,
    /// The main action (`.btn.primary`): `accent-fill` and `on-fill`;
    /// disabled, a word button's (`.btn:disabled` wins: `raised` and
    /// `text-off`).
    Primary,
}

/// A button drawn by the kit in its own tokens (module comment): an icon,
/// words, or both, and a chevron where it opens a popover or a menu.
/// Its accessible name is its words (or [`KitButton::name`]) however it
/// is labelled; an icon alone shows them as its tooltip unless given one.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use mg_uikit::{ButtonLook, Icon, KitButton};
/// let save = KitButton::new(Some(Icon::Save), "Save").enabled(false);
/// let width = save.width(ui);
/// let response = ui.add(save);
/// assert_eq!(response.rect.width(), width);
/// # });
/// ```
#[derive(Clone, Debug)]
#[must_use = "add it with ui.add"]
pub struct KitButton {
    icon: Option<Icon>,
    words: String,
    name: Option<String>,
    tip: Option<String>,
    icon_only: bool,
    look: ButtonLook,
    on: bool,
    chevron: bool,
    open: Option<bool>,
    enabled: bool,
    drawn_as: Option<ButtonState>,
    corners: Option<egui::CornerRadius>,
    count: Option<String>,
    /// Its words' size where not the body's ([`KitButton::text_size`]).
    size: Option<f32>,
    /// The width it takes at least, its icon and words kept at its left
    /// ([`KitButton::at_least`]).
    least: Option<f32>,
    /// Its command's keys at its right end ([`KitButton::keys`]).
    keys: Option<String>,
    /// Its icon in `text-2` ([`KitButton::quiet_icon`]).
    quiet_icon: bool,
    /// Its height where not a control's ([`KitButton::height`]).
    high: Option<f32>,
}

impl KitButton {
    /// A button showing `icon` (where given) before `words`.
    pub fn new(icon: Option<Icon>, words: impl Into<String>) -> Self {
        KitButton {
            icon,
            words: words.into(),
            name: None,
            tip: None,
            icon_only: false,
            look: ButtonLook::Raised,
            on: false,
            chevron: false,
            open: None,
            enabled: true,
            drawn_as: None,
            corners: None,
            count: None,
            size: None,
            least: None,
            keys: None,
            quiet_icon: false,
            high: None,
        }
    }

    /// `height` points high, not a control's 22 (`.welcome .acts .btn`:
    /// 26), its icon and words in its middle.
    pub fn height(mut self, height: f32) -> Self {
        self.high = Some(height);
        self
    }

    /// At least `width` wide, its icon and words at its left (`.acts
    /// .btn`: a column's buttons stretched to the column, the Welcome
    /// page's Start).
    pub fn at_least(mut self, width: f32) -> Self {
        self.least = Some(width);
        self
    }

    /// With `keys` (its command's, as bound: `Ctrl+N`) at its right end,
    /// in `text-3` at 11 points (`.btn .k`), not part of its name. None
    /// where `keys` is empty.
    pub fn keys(mut self, keys: impl Into<String>) -> Self {
        self.keys = Some(keys.into()).filter(|k| !k.is_empty());
        self
    }

    /// Its icon in `text-2` beside words in `text` (`.acts .btn .ic`).
    pub fn quiet_icon(mut self) -> Self {
        self.quiet_icon = true;
        self
    }

    /// Its words at `size` points, not the body's, and its height grown
    /// with them: a button among a panel's content at an editor's own
    /// larger text (the conversation editor's Add Set, M4's U15 review).
    pub fn text_size(mut self, size: f32) -> Self {
        self.size = Some(size);
        self
    }

    /// How much larger than the body its words are, at least 1.
    fn scale(&self) -> f32 {
        self.size.map_or(1.0, |s| (s / metrics::BODY_SIZE).max(1.0))
    }

    /// With a weak count after its words (`.count-w`: `text-3`, 11 px;
    /// the compact area header's Show, "5 of 10"), not part of its name.
    /// Not shown with its icon alone.
    pub fn count(mut self, count: impl Into<String>) -> Self {
        self.count = Some(count.into());
        self
    }

    /// Showing its icon alone where `only` (it has one): its words are
    /// its accessible name and, unless [`KitButton::tip`] gives one, its
    /// tooltip.
    pub fn icon_only(mut self, only: bool) -> Self {
        self.icon_only = only && self.icon.is_some();
        self
    }

    /// Looking so ([`ButtonLook`]).
    pub fn look(mut self, look: ButtonLook) -> Self {
        self.look = look;
        self
    }

    /// A toggle that is on (`on`) or off.
    pub fn toggle(mut self, on: bool) -> Self {
        self.look = ButtonLook::Toggle;
        self.on = on;
        self
    }

    /// With a chevron after it: it opens a popover or a menu, said open
    /// (`open`) or closed, and drawn open while it is (`.btn.open`:
    /// `raised-hover` and an `accent` ring).
    pub fn opens(mut self, open: bool) -> Self {
        self.chevron = true;
        self.open = Some(open);
        self
    }

    /// Said open (`open`) or closed, and drawn open while it is, without a
    /// chevron: an icon alone that opens a popover (More's `⋯`).
    pub fn open_state(mut self, open: bool) -> Self {
        self.open = Some(open);
        self
    }

    /// Enabled (`true`, the default) or drawn and said disabled, which
    /// also makes it ignore clicks.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Named `name` for a screen reader (and tests) in place of its words.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Its tooltip (also shown while disabled).
    pub fn tip(mut self, tip: impl Into<String>) -> Self {
        self.tip = Some(tip.into());
        self
    }

    /// One of a joined group's buttons (`.join`: 1 px seams, the outer
    /// corners round): its left corners round where it is the `first`,
    /// its right corners where it is the `last`, square elsewhere.
    pub fn joined(mut self, first: bool, last: bool) -> Self {
        let r = |round: bool| if round { metrics::RADIUS } else { 0 };
        self.corners =
            Some(egui::CornerRadius { nw: r(first), sw: r(first), ne: r(last), se: r(last) });
        self
    }

    /// Drawn in `state` whatever the pointer does (a catalog's hovered
    /// button).
    pub fn drawn_as(mut self, state: ButtonState) -> Self {
        self.drawn_as = Some(state);
        self
    }

    /// Its accessible name.
    pub fn accessible_name(&self) -> &str {
        self.name.as_deref().unwrap_or(&self.words)
    }

    /// Whether it shows its icon alone.
    pub fn is_icon_only(&self) -> bool {
        self.icon_only
    }

    /// Its keys laid out (`.btn .k`: 11 points).
    fn keys_galley(ui: &Ui, keys: &str, color: Color32) -> std::sync::Arc<egui::Galley> {
        let font = FontId::proportional(metrics::SMALL_SIZE);
        ui.painter().layout_no_wrap(keys.to_string(), font, color)
    }

    fn font(&self) -> FontId {
        FontId::proportional(metrics::BODY_SIZE * self.scale())
    }

    /// Measures the button once and returns its laid-out text parts using
    /// `Color32::PLACEHOLDER` so `epaint::GalleyCache` hits across sizing,
    /// hover, and disabled states.
    fn measure_parts(&self, ui: &Ui) -> ButtonParts {
        if self.icon_only {
            let width = match self.chevron {
                false => metrics::ICON_BUTTON,
                true => {
                    2.0 * ICON_CHEVRON_PADDING
                        + metrics::ICON
                        + ICON_CHEVRON_GAP
                        + metrics::ICON_SMALL
                        - BUTTON_INSET
                }
            };
            return ButtonParts { width, words: None, count: None, keys: None };
        }
        let words =
            ui.painter().layout_no_wrap(self.words.clone(), self.font(), Color32::PLACEHOLDER);
        let mut width = 2.0 * metrics::CONTROL_PADDING_X + words.size().x;
        let count = self.count.as_ref().map(|count| {
            let font = FontId::proportional(metrics::SMALL_SIZE);
            let galley = ui.painter().layout_no_wrap(count.clone(), font, Color32::PLACEHOLDER);
            width += COUNT_GAP + galley.size().x;
            galley
        });
        if self.icon.is_some() {
            width += metrics::ICON - BUTTON_INSET + BUTTON_GAP;
        }
        if self.chevron {
            width += BUTTON_GAP + metrics::ICON_SMALL - BUTTON_INSET;
        }
        let keys = self.keys.as_ref().map(|keys| {
            let galley = Self::keys_galley(ui, keys, Color32::PLACEHOLDER);
            width += KEYS_GAP + galley.size().x;
            galley
        });
        ButtonParts {
            width: width.ceil().max(self.least.unwrap_or(0.0)),
            words: Some(words),
            count,
            keys,
        }
    }

    /// The width it takes (the same in every state): 22 for an icon
    /// alone, 36 for an icon alone with a chevron; words 8 px from each
    /// side, an icon 6 px before them and a chevron 6 px after.
    pub fn width(&self, ui: &Ui) -> f32 {
        self.measure_parts(ui).width
    }

    /// The fill, the color of its icon and words, the color of its
    /// chevron and its ring, in `state`.
    fn colors(
        &self,
        t: &Tokens,
        state: ButtonState,
        enabled: bool,
    ) -> (Color32, Color32, Color32, Stroke) {
        let open = self.open == Some(true);
        let lit = state != ButtonState::Rest;
        let none = Color32::TRANSPARENT;
        let flat = self.look == ButtonLook::Flat;
        let (fill, text) = if !enabled {
            match self.look {
                ButtonLook::Toggle if self.on => (t.accent_fill_off, t.on_fill_off),
                _ if flat => (none, t.text_off),
                _ => (t.raised, t.text_off),
            }
        } else {
            let quiet = if self.icon_only { t.text_2 } else { t.text };
            match self.look {
                ButtonLook::Toggle if self.on => {
                    (if lit { t.accent_fill_hover } else { t.accent_fill }, t.on_fill)
                }
                ButtonLook::Primary => {
                    (if lit { t.accent_fill_hover } else { t.accent_fill }, t.on_fill)
                }
                ButtonLook::Toggle => match lit {
                    true => (t.raised_hover, t.text),
                    false => (t.raised, quiet),
                },
                _ if flat => match (state, open) {
                    (ButtonState::Rest, false) => (none, quiet),
                    (ButtonState::Hovered, false) => (t.raised, t.text),
                    _ => (t.raised_hover, t.text),
                },
                _ => match lit || open {
                    true => (t.raised_hover, t.text),
                    false => (t.raised, t.text),
                },
            }
        };
        let chevron = if !enabled {
            text
        } else if text == t.on_fill {
            t.on_fill
        } else {
            t.text_2
        };
        let ring = if enabled && (open || state == ButtonState::Pressed) {
            Stroke::new(1.0, t.accent)
        } else {
            Stroke::NONE
        };
        (fill, text, chevron, ring)
    }
}

struct ButtonParts {
    width: f32,
    words: Option<std::sync::Arc<egui::Galley>>,
    count: Option<std::sync::Arc<egui::Galley>>,
    keys: Option<std::sync::Arc<egui::Galley>>,
}

impl egui::Widget for KitButton {
    fn ui(self, ui: &mut Ui) -> Response {
        let parts = self.measure_parts(ui);
        let height = self.high.unwrap_or(metrics::CONTROL_HEIGHT) * self.scale();
        let sense = if self.enabled { Sense::click() } else { Sense::hover() };
        let (rect, response) = ui.allocate_exact_size(vec2(parts.width, height), sense);
        let enabled = self.enabled && ui.is_enabled();
        let toggle = self.look == ButtonLook::Toggle;
        response.widget_info(|| {
            let name = self.accessible_name();
            match toggle {
                true => WidgetInfo::selected(WidgetType::Button, enabled, self.on, name),
                false => WidgetInfo::labeled(WidgetType::Button, enabled, name),
            }
        });
        if let Some(open) = self.open {
            crate::icons::say_expanded(&response, open);
        }
        // (Its keys told to a screen reader too, as they are drawn.)
        if let Some(keys) = &self.keys {
            ui.ctx().accesskit_node_builder(response.id, |n| n.set_keyboard_shortcut(keys.clone()));
        }
        if ui.is_rect_visible(rect) {
            let t = ui.tokens();
            let state = self.drawn_as.unwrap_or_else(|| match self.enabled {
                true => ButtonState::of(&response),
                false => ButtonState::Rest,
            });
            let (fill, text, chevron, ring) = self.colors(&t, state, enabled);
            let painter = crate::controls::box_painter(ui, enabled);
            let corners = self.corners.unwrap_or(metrics::RADIUS.into());
            let focused = enabled && response.has_focus();
            let accent_filled = fill == t.accent_fill || fill == t.accent_fill_hover;
            let outer_ring = if focused && !accent_filled {
                Stroke::new(1.0, t.accent)
            } else if !accent_filled {
                ring
            } else {
                Stroke::NONE
            };
            painter.rect(rect, corners, fill, outer_ring, egui::StrokeKind::Inside);
            if focused && accent_filled {
                painter.rect_stroke(
                    rect.shrink(1.5),
                    corners,
                    Stroke::new(1.5, t.on_fill),
                    egui::StrokeKind::Inside,
                );
            }
            let y = rect.center().y;
            if self.icon_only {
                let icon = self.icon.unwrap_or(Icon::Help);
                let x = match self.chevron {
                    false => rect.center().x,
                    true => rect.left() + ICON_CHEVRON_PADDING + metrics::ICON / 2.0,
                };
                let square = Rect::from_center_size(pos2(x, y), Vec2::splat(metrics::ICON));
                icon.paint(&painter, square, text);
                if self.chevron {
                    let cx = square.right() + ICON_CHEVRON_GAP + metrics::ICON_SMALL / 2.0;
                    crate::controls::paint_chevron(&painter, pos2(cx, y), 1.0, chevron);
                }
            } else {
                let mut x = rect.left() + metrics::CONTROL_PADDING_X;
                if let Some(icon) = self.icon {
                    x -= BUTTON_INSET;
                    let square = Rect::from_min_size(
                        pos2(x, y - metrics::ICON / 2.0),
                        Vec2::splat(metrics::ICON),
                    );
                    let quiet = self.quiet_icon && enabled && text == t.text;
                    icon.paint(&painter, square, if quiet { t.text_2 } else { text });
                    x += metrics::ICON + BUTTON_GAP;
                }
                if let Some(galley) = parts.words {
                    let at = pos2(x, y - galley.size().y / 2.0);
                    x += galley.size().x;
                    painter.galley(at, galley, text);
                }
                if let Some(galley) = parts.count {
                    let weak = if enabled { t.text_3 } else { t.text_off };
                    let at = pos2(x + COUNT_GAP, y - galley.size().y / 2.0);
                    x += COUNT_GAP + galley.size().x;
                    painter.galley(at, galley, weak);
                }
                if self.chevron {
                    let cx = x + BUTTON_GAP + metrics::ICON_SMALL / 2.0;
                    crate::controls::paint_chevron(&painter, pos2(cx, y), 1.0, chevron);
                }
                if let Some(galley) = parts.keys {
                    let weak = if enabled { t.text_3 } else { t.text_off };
                    let right = rect.right() - metrics::CONTROL_PADDING_X;
                    let at = pos2(right - galley.size().x, y - galley.size().y / 2.0);
                    painter.galley(at, galley, weak);
                }
            }
        }
        match (self.tip, self.icon_only) {
            (Some(tip), _) => response.on_hover_text(&tip).on_disabled_hover_text(tip),
            (None, true) => {
                let tip = self.name.unwrap_or(self.words);
                response.on_hover_text(&tip).on_disabled_hover_text(tip)
            }
            (None, false) => response,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Theme;

    fn colors(button: KitButton, theme: Theme, state: ButtonState) -> (Color32, Color32) {
        let (fill, text, _, _) = button.colors(&theme.tokens(), state, button.enabled);
        (fill, text)
    }

    /// A disabled button is drawn in the disabled tokens, in each look
    /// and theme (the mock's `:disabled` rules), not a fade of its
    /// enabled colors.
    #[test]
    fn a_disabled_button_takes_the_disabled_tokens() {
        for theme in Theme::ALL {
            let t = theme.tokens();
            let none = Color32::TRANSPARENT;
            let off = |b: KitButton| colors(b.enabled(false), theme, ButtonState::Rest);
            let words = KitButton::new(Some(Icon::Save), "Save");
            assert_eq!(off(words.clone()), (t.raised, t.text_off), "{theme:?}");
            assert_eq!(off(words.clone().look(ButtonLook::Flat)), (none, t.text_off));
            assert_eq!(off(words.clone().icon_only(true)), (t.raised, t.text_off));
            let flat_icon = words.clone().icon_only(true).look(ButtonLook::Flat);
            assert_eq!(off(flat_icon), (none, t.text_off));
            assert_eq!(off(words.clone().toggle(false)), (t.raised, t.text_off));
            assert_eq!(off(words.clone().toggle(true)), (t.accent_fill_off, t.on_fill_off));
            assert_eq!(off(words.look(ButtonLook::Primary)), (t.raised, t.text_off));
        }
    }

    /// The enabled looks at rest and hovered (`.btn`, `.btn-icon`,
    /// `.toggle`, `.toggle.on`).
    #[test]
    fn the_enabled_looks_follow_the_stylesheet() {
        let t = Theme::Dark.tokens();
        let rest = ButtonState::Rest;
        let hover = ButtonState::Hovered;
        let words = KitButton::new(Some(Icon::Save), "Save");
        assert_eq!(colors(words.clone(), Theme::Dark, rest), (t.raised, t.text));
        assert_eq!(colors(words.clone(), Theme::Dark, hover), (t.raised_hover, t.text));
        let icon = words.clone().icon_only(true).look(ButtonLook::Flat);
        assert_eq!(colors(icon.clone(), Theme::Dark, rest), (Color32::TRANSPARENT, t.text_2));
        assert_eq!(colors(icon, Theme::Dark, hover), (t.raised, t.text));
        // (A word button's icon alone stays framed, as the narrow board's
        // compact buttons are.)
        let framed = words.clone().icon_only(true);
        assert_eq!(colors(framed, Theme::Dark, rest), (t.raised, t.text));
        let on = words.clone().toggle(true);
        assert_eq!(colors(on, Theme::Dark, rest), (t.accent_fill, t.on_fill));
        let off = words.toggle(false).icon_only(true);
        assert_eq!(colors(off, Theme::Dark, rest), (t.raised, t.text_2));
    }
}
