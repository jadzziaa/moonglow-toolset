//! The dock's look in the suite's theme (the mock's `.doctabs`,
//! `.doctab`, `.docktab`): egui_dock's style made from the tokens, and what
//! its style cannot say, painted over its tabs once they are drawn (the
//! active tab's line along its top, the lines between tabs).

use egui_dock::{DockState, Style};
use mg_uikit::{Tokens, TokensExt as _, metrics};

use crate::Tab;

/// egui_dock's style in the theme `ui` wears: the tab bars `bar`, 28 px
/// high, a 1 px `line` under them; the active tab `panel` with its words
/// in `text`, the others in `text-2` on the bar, hovered `tab-hover`;
/// no outlines (the active tab's accent line and the lines between tabs
/// are [`marks`]'), a tab reached with the keyboard ringed in `accent`;
/// the seams between panes 1 px of `line`, `accent` held; the close
/// buttons `text-2`, `text` and `raised` hovered.
pub(crate) fn style(ui: &egui::Ui) -> Style {
    let t = ui.tokens();
    let mut s = Style::from_egui(ui.style());
    s.main_surface_border_stroke = egui::Stroke::NONE;
    let bar = &mut s.tab_bar;
    bar.bg_fill = t.bar;
    bar.height = metrics::DOCUMENT_TABS;
    bar.hline_color = t.line;
    bar.corner_radius = egui::CornerRadius::ZERO;
    let tab = |fill, text, outline| egui_dock::TabInteractionStyle {
        outline_color: outline,
        corner_radius: egui::CornerRadius::ZERO,
        bg_fill: fill,
        text_color: text,
    };
    let none = egui::Color32::TRANSPARENT;
    s.tab.active = tab(t.panel, t.text, none);
    s.tab.focused = tab(t.panel, t.text, none);
    s.tab.inactive = tab(t.bar, t.text_2, none);
    s.tab.hovered = tab(t.tab_hover, t.text, none);
    s.tab.active_with_kb_focus = tab(t.panel, t.text, t.accent);
    s.tab.focused_with_kb_focus = tab(t.panel, t.text, t.accent);
    s.tab.inactive_with_kb_focus = tab(t.bar, t.text, t.accent);
    s.tab.hline_below_active_tab_name = false;
    s.tab.spacing = 0.0;
    s.tab.tab_body.bg_fill = t.panel;
    s.tab.tab_body.stroke = egui::Stroke::NONE;
    s.tab.tab_body.corner_radius = egui::CornerRadius::ZERO;
    let b = &mut s.buttons;
    b.close_tab_color = t.text_2;
    b.close_tab_active_color = t.text;
    b.close_tab_bg_fill = t.raised;
    for (color, active, fill, border) in [
        (
            &mut b.add_tab_color,
            &mut b.add_tab_active_color,
            &mut b.add_tab_bg_fill,
            &mut b.add_tab_border_color,
        ),
        (
            &mut b.close_all_tabs_color,
            &mut b.close_all_tabs_active_color,
            &mut b.close_all_tabs_bg_fill,
            &mut b.close_all_tabs_border_color,
        ),
        (
            &mut b.collapse_tabs_color,
            &mut b.collapse_tabs_active_color,
            &mut b.collapse_tabs_bg_fill,
            &mut b.collapse_tabs_border_color,
        ),
        (
            &mut b.minimize_window_color,
            &mut b.minimize_window_active_color,
            &mut b.minimize_window_bg_fill,
            &mut b.minimize_window_border_color,
        ),
    ] {
        *color = t.text_2;
        *active = t.text;
        *fill = t.raised;
        *border = t.line;
    }
    b.close_all_tabs_disabled_color = t.text_off;
    b.show_tab_bar_color = t.text_2;
    b.show_tab_bar_active_color = t.text;
    let sep = &mut s.separator;
    sep.width = 1.0;
    sep.color_idle = t.line;
    sep.color_hovered = t.accent;
    sep.color_dragged = t.accent;
    s.overlay.selection_color = t.accent.gamma_multiply(0.35);
    s
}

/// What egui_dock's style cannot say, painted over the tabs once the dock
/// is drawn (`tabs`: each tab's rectangle and layer, as its button was
/// drawn this frame): a 1 px `line` at each tab's right (the mock's
/// `border-right`), and along the top of each pane's tab in front a 2 px
/// line, `accent` in the pane that has the focus and `line-soft` in the
/// others (the mock's `.group.focus .doctab.active`).
pub(crate) fn marks(
    ctx: &egui::Context,
    dock: &DockState<Tab>,
    tabs: &[(Tab, egui::Rect, egui::LayerId)],
) {
    let t = ctx.tokens();
    let focused = dock.focused_leaf();
    for (_, rect, layer) in tabs {
        let painter = ctx.layer_painter(*layer);
        painter.vline(rect.right() - 0.5, rect.y_range(), egui::Stroke::new(1.0, t.line));
    }
    for (path, leaf) in dock.iter_leaves() {
        let Some(front) = leaf.tabs.get(leaf.active.0) else { continue };
        let Some((_, rect, layer)) = tabs.iter().find(|(t, ..)| t == front) else { continue };
        let color = front_line(&t, focused == Some(path));
        let line = egui::Rect::from_min_size(
            rect.min,
            egui::vec2(rect.width() - 1.0, metrics::ACTIVE_TAB_LINE),
        );
        ctx.layer_painter(*layer).rect_filled(line, 0.0, color);
    }
}

/// The color of the line along the top of a pane's tab in front.
fn front_line(t: &Tokens, focused: bool) -> egui::Color32 {
    if focused { t.accent } else { t.line_soft }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The dock's style follows the theme the context wears, both ways.
    #[test]
    fn the_dock_s_style_follows_the_theme() {
        let ctx = egui::Context::default();
        for theme in [mg_uikit::Theme::Light, mg_uikit::Theme::Dark, mg_uikit::Theme::Light] {
            mg_uikit::apply(&ctx, theme);
            let t = theme.tokens();
            let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
                let s = style(ui);
                assert_eq!(s.tab_bar.bg_fill, t.bar, "{theme:?}");
                assert_eq!(s.tab_bar.height, metrics::DOCUMENT_TABS);
                assert_eq!(s.tab_bar.hline_color, t.line);
                assert_eq!(s.tab.active.bg_fill, t.panel);
                assert_eq!(s.tab.active.text_color, t.text);
                assert_eq!(s.tab.inactive.text_color, t.text_2);
                assert_eq!(s.tab.hovered.bg_fill, t.tab_hover);
                assert_eq!(s.tab.tab_body.bg_fill, t.panel);
                assert_eq!(s.separator.color_idle, t.line);
                assert_eq!(s.buttons.close_tab_color, t.text_2);
            });
            out.textures_delta.clear();
        }
        assert_eq!(front_line(&Tokens::DARK, true), Tokens::DARK.accent);
        assert_eq!(front_line(&Tokens::LIGHT, false), Tokens::LIGHT.line_soft);
    }
}
