//! The theme: the [tokens](mod@crate::tokens) and the metrics
//! ([`crate::metrics`]) made into egui's [`egui::Style`], for the dark and the light theme.
//!
//! [`apply`] installs the fonts, writes both of egui's style slots (its
//! dark one from [`Theme::Dark`], its light one from [`Theme::Light`]) and
//! selects the chosen theme; [`tokens`] (or `ui.tokens()`, [`TokensExt`])
//! gives the applied theme's tokens to widgets that paint their own.
//!
//! Which rule of the mockups (`moonglow.css`, the spec's sections 3 to 5)
//! each egui value answers:
//!
//! | egui | value | mock |
//! |---|---|---|
//! | `panel_fill` | `panel` | `.panel`: dock and editor bodies |
//! | `window_fill`, `window_stroke` | `panel`, 1 px `line-soft` | `.popover`: panel, a `line-soft` border (windows and popups share them) |
//! | `window_corner_radius`, `menu_corner_radius` | 4 | popover radius 4 |
//! | `window_shadow` | 6 down, 18 blur, `shadow` | the boards' `.dlg` and `.fwin`: `--shadow` (rule 7 said no shadows but under popovers: the dialog and floating window boards draw one) |
//! | `popup_shadow` | 6 down, 18 blur, `shadow` | `--shadow`'s drop shadow (its rim: the border) |
//! | `widgets.noninteractive` | text `text`, lines `line-soft` | body text; rules inside a panel (separators, the heading's rule) |
//! | `widgets.inactive` | fill `raised`, border `raised`, text `text` | `.btn`, `.combo`, `.toggle` at rest, no border; the border is also a text field's (`.field`'s is `line-soft`, a shade lighter) |
//! | `widgets.hovered` | fill `raised-hover` | `.btn:hover` |
//! | `widgets.active` (pressed, or focused) | fill `raised-hover`, border `accent` | `.btn.open`: an accent ring |
//! | `widgets.open` (an open combo box or menu; the top window's title bar) | fill `bar`, border `accent` | `.combo.open`'s ring; the dialogs' title row, `bar` |
//! | `bg_fill` of those (check box, slider rail and thumb, scroll thumb) | `scroll-thumb`, `scroll-thumb-hover` | `.vbar .thumb` |
//! | corner radius of every state | 3 | radius 3 |
//! | `expansion` of every interactive state | 1, the border's width | (none: a row at rest as wide as hovered; a list's, tree's or table's rows have neither, [`crate::controls::row_style`], so that their fills meet) |
//! | `selection` | fill `accent-fill`, text `on-fill` | `.toggle.on`, `.tree-row.sel`, selected text |
//! | `extreme_bg_color`, `text_edit_bg_color` | `inset` | `.field`, `.memo`, list wells |
//! | `code_bg_color` | `inset` | `.code` |
//! | `faint_bg_color` (striped rows) | `current-line` | (a faint step from `panel`) |
//! | `hyperlink_color` | `accent` | `a` |
//! | `warn_fg_color`, `error_fg_color` | `warn`, `error` | `.warn-text`, `.err-text` |
//! | `weak_text_color` | `text-3` | `.t3`, `.note`: counts, hints, placeholders |
//! | `disabled_alpha` | 0.35 dark, 0.47 light | `--text-off`: text at that opacity on `panel` comes out as `text-off` |
//! | `text_cursor` | 2 px `text` | `.code .caret` |
//! | (the pass after each frame, [`crate::controls`]) | the focused field's border `accent`; a tooltip's fill `tooltip-bg` | `.field.focus`, `.tooltip` |
//! | `slider_trailing_fill`, `handle_shape` | on, 10 by 14 | `.slider .fill`, `.slider .thumb` |
//! | `indent_has_left_vline` | off | `.tree-row`: no guide lines |
//! | `spacing.interact_size.y` | 22 | controls and rows 22 |
//! | `spacing.button_padding` | 8 by 3 | `padding: 0 8px` |
//! | `spacing.item_spacing` | 4 by 4 | `.row` gap 4; gaps on a 4 px step |
//! | `spacing.indent` | 14 | `--indent` |
//! | `spacing.window_margin` | 8 | panel padding 8 |
//! | `spacing.menu_margin` | 4 | `.popover`, `.menu-pop`: padding 4 |
//! | `spacing.icon_width`, `icon_spacing` | 14, 6 | `.check .box` 14, gap 6 |
//! | `spacing.slider_rail_height` | 4 | `.slider .track` 4 |
//! | `spacing.tooltip_width` | 300 | `.tooltip` max 300 |
//! | `spacing.scroll` | solid, a 4 px thumb in 8 | `.vbar`: 8 wide, 2 padding, a 4 px thumb |
//! | text styles | Body, Button 13; Small 11; Heading 15, not bold; Monospace 13 | the spec's type (bold only in section headings, which set it) |

use egui::{Color32, CornerRadius, Margin, Shadow, Stroke, Style, TextStyle, Visuals, vec2};

use crate::fonts;
use crate::metrics;
use crate::tokens::Tokens;

/// The suite's two themes.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize,
)]
pub enum Theme {
    /// The dark theme (the default).
    #[default]
    Dark,
    /// The light theme.
    Light,
}

impl Theme {
    /// Both themes, dark first.
    pub const ALL: [Theme; 2] = [Theme::Dark, Theme::Light];

    /// Its name, as a setting shows it.
    pub const fn name(self) -> &'static str {
        match self {
            Theme::Dark => "Dark",
            Theme::Light => "Light",
        }
    }

    /// Its tokens.
    pub const fn tokens(self) -> Tokens {
        match self {
            Theme::Dark => Tokens::DARK,
            Theme::Light => Tokens::LIGHT,
        }
    }

    /// egui's theme of the same lightness.
    pub const fn egui(self) -> egui::Theme {
        match self {
            Theme::Dark => egui::Theme::Dark,
            Theme::Light => egui::Theme::Light,
        }
    }

    /// The theme of egui's `theme`'s lightness.
    pub const fn from_egui(theme: egui::Theme) -> Self {
        match theme {
            egui::Theme::Dark => Theme::Dark,
            egui::Theme::Light => Theme::Light,
        }
    }
}

/// What [`apply`] last did to a context: the theme chosen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Applied {
    theme: Theme,
}

fn applied_id() -> egui::Id {
    egui::Id::new("mg-uikit-theme")
}

/// Gives `ctx` the suite's look in `theme`: installs the fonts (once a
/// context), writes egui's dark style from [`Theme::Dark`] and its light
/// style from [`Theme::Light`] (each from the tokens and metrics, keeping
/// what the style had that is not the look's: the font texture's size,
/// debug options), and selects `theme`.
///
/// Cheap once done (it returns at once while nothing changed), so it is
/// called every frame. What it changes shows from the next frame, which
/// it asks for. The first time, it also installs the pass that paints
/// what the style cannot say ([`crate::controls`]: the focused text
/// field's border, the tooltip's fill).
pub fn apply(ctx: &egui::Context, theme: Theme) {
    fonts::install_fonts(ctx);
    if before_any(ctx) {
        ctx.on_end_pass("mg-uikit", std::sync::Arc::new(crate::controls::end_pass));
    }
    let now = Applied { theme };
    let before = ctx.data(|d| d.get_temp::<Applied>(applied_id()));
    if before == Some(now) && ctx.theme() == theme.egui() {
        return;
    }
    if before.is_none() {
        for t in Theme::ALL {
            let old = ctx.style_of(t.egui());
            ctx.set_style_of(t.egui(), style(t, &old));
        }
    }
    ctx.set_theme(theme.egui());
    ctx.data_mut(|d| d.insert_temp(applied_id(), now));
    ctx.request_repaint();
}

/// Whether `apply` is called on `ctx` for the first time (and marks it).
fn before_any(ctx: &egui::Context) -> bool {
    let id = applied_id().with("passes");
    let first = ctx.data(|d| d.get_temp::<bool>(id)).is_none();
    if first {
        ctx.data_mut(|d| d.insert_temp(id, true));
    }
    first
}

/// The theme [`apply`] gave `ctx`, as egui's theme now selects it (egui
/// can be switched to its other style afterwards, which [`apply`] wrote
/// too); `None` when it was never applied.
pub fn applied(ctx: &egui::Context) -> Option<Theme> {
    ctx.data(|d| d.get_temp::<Applied>(applied_id())).map(|_| Theme::from_egui(ctx.theme()))
}

/// The applied theme's tokens; [`Tokens::DARK`] when no theme was applied.
pub fn tokens(ctx: &egui::Context) -> Tokens {
    applied(ctx).unwrap_or_default().tokens()
}

/// `ui.tokens()` and `ctx.tokens()`: [`tokens`].
pub trait TokensExt {
    /// The applied theme's tokens ([`tokens`]).
    fn tokens(&self) -> Tokens;
}

impl TokensExt for egui::Context {
    fn tokens(&self) -> Tokens {
        tokens(self)
    }
}

impl TokensExt for egui::Ui {
    fn tokens(&self) -> Tokens {
        tokens(self.ctx())
    }
}

/// The opacity of a disabled widget in each theme: what makes `text` on
/// `panel` come out as `text-off`.
const fn disabled_alpha(theme: Theme) -> f32 {
    match theme {
        Theme::Dark => 0.35,
        Theme::Light => 0.47,
    }
}

/// The style of `theme`: `old` with the look replaced.
pub fn style(theme: Theme, old: &Style) -> Style {
    let mut style = old.clone();
    style.visuals = visuals(theme, &old.visuals);
    let s = &mut style.spacing;
    s.item_spacing = vec2(metrics::GAP_S, metrics::GAP_S);
    s.window_margin = Margin::same(metrics::PANEL_PADDING as i8);
    s.menu_margin = Margin::same(metrics::GAP_S as i8);
    s.button_padding = vec2(metrics::CONTROL_PADDING_X, 3.0);
    s.indent = metrics::INDENT;
    s.interact_size.y = metrics::CONTROL_HEIGHT;
    s.icon_width = 14.0;
    s.icon_width_inner = 8.0;
    s.icon_spacing = 6.0;
    s.slider_rail_height = 4.0;
    s.tooltip_width = 300.0;
    s.scroll = egui::style::ScrollStyle {
        bar_width: 4.0,
        bar_inner_margin: 2.0,
        bar_outer_margin: 2.0,
        handle_min_length: 16.0,
        ..egui::style::ScrollStyle::solid()
    };
    style.text_styles = [
        (TextStyle::Small, egui::FontId::proportional(metrics::SMALL_SIZE)),
        (TextStyle::Body, egui::FontId::proportional(metrics::BODY_SIZE)),
        (TextStyle::Button, egui::FontId::proportional(metrics::BODY_SIZE)),
        // (Not bold: section headings, the one bold, set their own face,
        // `form::section_heading`; a window's title is in the body's size,
        // set where it is made.)
        (TextStyle::Heading, egui::FontId::proportional(metrics::HEADING_SIZE)),
        (TextStyle::Monospace, egui::FontId::monospace(metrics::MONO_SIZE)),
    ]
    .into();
    style
}

/// The visuals of `theme`: egui's own of the same lightness (their text
/// rendering), the font texture's size kept from `old`, the rest from
/// the tokens.
pub fn visuals(theme: Theme, old: &Visuals) -> Visuals {
    let t = theme.tokens();
    let mut v = match theme {
        Theme::Dark => Visuals::dark(),
        Theme::Light => Visuals::light(),
    };
    v.text_options.max_texture_side = old.text_options.max_texture_side;
    v.text_options.font_hinting = old.text_options.font_hinting;
    // (Whether the caret blinks is the application's, or a test's: kept.)
    v.text_cursor.blink = old.text_cursor.blink;
    let radius = CornerRadius::same(metrics::RADIUS);
    let line = |c: Color32| Stroke::new(1.0, c);
    let state =
        |fill: Color32, weak: Color32, border: Color32, text: Color32| egui::style::WidgetVisuals {
            bg_fill: fill,
            weak_bg_fill: weak,
            bg_stroke: line(border),
            corner_radius: radius,
            fg_stroke: Stroke::new(1.5, text),
            // (As wide as the border, in every state: egui takes the
            // border from a button's padding and gives the expansion
            // back, also where it draws no frame (a selectable row at
            // rest), so that a row keeps its width as it is hovered.)
            expansion: 1.0,
        };
    let w = &mut v.widgets;
    w.noninteractive = state(t.panel, t.panel, t.line_soft, t.text);
    w.noninteractive.fg_stroke.width = 1.0;
    w.noninteractive.expansion = 0.0;
    w.inactive = state(t.scroll_thumb, t.raised, t.raised, t.text);
    w.hovered = state(t.scroll_thumb_hover, t.raised_hover, t.raised_hover, t.text);
    w.active = state(t.scroll_thumb_hover, t.raised_hover, t.accent, t.text);
    // (An open combo box, an open menu's title; and egui fills the title
    // bar of the window on top with it: `bar`, the mock's title row.)
    w.open = state(t.scroll_thumb_hover, t.bar, t.accent, t.text);
    v.selection.bg_fill = t.accent_fill;
    v.selection.stroke = line(t.on_fill);
    v.ime_composition.active_underline_stroke = Stroke::new(2.0, t.accent);
    v.ime_composition.inactive_underline_stroke = Stroke::new(2.0, t.text_3);
    v.weak_text_color = Some(t.text_3);
    v.hyperlink_color = t.accent;
    v.faint_bg_color = t.current_line;
    v.extreme_bg_color = t.inset;
    v.text_edit_bg_color = Some(t.inset);
    v.code_bg_color = t.inset;
    v.warn_fg_color = t.warn;
    v.error_fg_color = t.error;
    v.window_corner_radius = CornerRadius::same(metrics::POPOVER_RADIUS);
    // (Windows, dialogs and floating editors, have the popovers' shadow:
    // the Dialogs and Floating boards.)
    v.window_shadow = Shadow { offset: [0, 6], blur: 18, spread: 0, color: t.shadow };
    v.window_fill = t.panel;
    v.window_stroke = line(t.line_soft);
    v.window_highlight_topmost = false;
    v.menu_corner_radius = CornerRadius::same(metrics::POPOVER_RADIUS);
    v.panel_fill = t.panel;
    v.popup_shadow = Shadow { offset: [0, 6], blur: 18, spread: 0, color: t.shadow };
    v.text_cursor.stroke = Stroke::new(2.0, t.text);
    v.button_frame = true;
    v.collapsing_header_frame = false;
    v.indent_has_left_vline = false;
    v.striped = false;
    v.slider_trailing_fill = true;
    v.handle_shape = egui::style::HandleShape::Rect { aspect_ratio: 10.0 / 14.0 };
    v.disabled_alpha = disabled_alpha(theme);
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_and_default() {
        assert_eq!(Theme::default(), Theme::Dark);
        assert_eq!(Theme::Dark.name(), "Dark");
        assert_eq!(Theme::Light.name(), "Light");
        for t in Theme::ALL {
            assert_eq!(Theme::from_egui(t.egui()), t);
        }
    }

    /// The setting's value as it is kept (a settings file).
    #[test]
    fn the_setting_round_trips() {
        for t in Theme::ALL {
            let text = serde_json::to_string(&t).unwrap();
            assert_eq!(text, format!("\"{}\"", t.name()));
            assert_eq!(serde_json::from_str::<Theme>(&text).unwrap(), t);
        }
    }
}
