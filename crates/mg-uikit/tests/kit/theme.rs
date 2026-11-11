//! What `apply` does to a context, and the form helpers with and without
//! a theme.

use egui::{Color32, TextStyle};
use egui_kittest::Harness;
use mg_uikit::{Theme, Tokens, TokensExt as _, fonts, metrics};

/// The color a label with `text` was painted in, from a frame's shapes.
fn text_color(h: &Harness<'_>, text: &str) -> Option<Color32> {
    h.output().shapes.iter().find_map(|s| match &s.shape {
        egui::Shape::Text(t) if t.galley.text() == text => {
            Some(t.override_text_color.unwrap_or(t.galley.job.sections[0].format.color))
        }
        _ => None,
    })
}

/// What it sets, in both of egui's styles, and the tokens following it.
#[test]
fn apply_sets_both_styles_and_the_tokens_follow() {
    let ctx = egui::Context::default();
    assert_eq!(mg_uikit::applied(&ctx), None);
    assert_eq!(mg_uikit::tokens(&ctx), Tokens::DARK);
    mg_uikit::apply(&ctx, Theme::Light);
    assert_eq!(ctx.theme(), egui::Theme::Light);
    assert_eq!(mg_uikit::applied(&ctx), Some(Theme::Light));
    assert_eq!(ctx.tokens(), Tokens::LIGHT);
    for theme in Theme::ALL {
        let t = theme.tokens();
        let style = ctx.style_of(theme.egui());
        let v = &style.visuals;
        assert_eq!(v.dark_mode, theme == Theme::Dark);
        assert_eq!(v.panel_fill, t.panel);
        assert_eq!(v.window_fill, t.panel);
        assert_eq!(v.window_stroke.color, t.line_soft);
        assert_eq!(v.window_shadow, egui::Shadow::NONE);
        assert_eq!(v.popup_shadow.color, t.shadow);
        assert_eq!(v.extreme_bg_color, t.inset);
        assert_eq!(v.text_edit_bg_color(), t.inset);
        assert_eq!(v.selection.bg_fill, t.accent_fill);
        assert_eq!(v.selection.stroke.color, t.on_fill);
        assert_eq!(v.hyperlink_color, t.accent);
        assert_eq!(v.error_fg_color, t.error);
        assert_eq!(v.warn_fg_color, t.warn);
        assert_eq!(v.weak_text_color(), t.text_3);
        assert_eq!(v.text_color(), t.text);
        assert_eq!(v.strong_text_color(), t.text);
        assert_eq!(v.widgets.noninteractive.bg_stroke.color, t.line_soft);
        assert_eq!(v.widgets.inactive.weak_bg_fill, t.raised);
        assert_eq!(v.widgets.hovered.weak_bg_fill, t.raised_hover);
        assert_eq!(v.widgets.active.bg_stroke.color, t.accent);
        assert_eq!(v.widgets.open.bg_stroke.color, t.accent);
        assert_eq!(v.widgets.open.weak_bg_fill, t.bar, "the top window's title bar");
        for w in [&v.widgets.noninteractive, &v.widgets.inactive, &v.widgets.hovered] {
            assert_eq!(w.corner_radius, egui::CornerRadius::same(metrics::RADIUS));
        }
        assert_eq!(v.menu_corner_radius, egui::CornerRadius::same(metrics::POPOVER_RADIUS));
        let s = &style.spacing;
        assert_eq!(s.interact_size.y, metrics::CONTROL_HEIGHT);
        assert_eq!(s.item_spacing, egui::vec2(4.0, 4.0));
        assert_eq!(s.indent, metrics::INDENT);
        assert_eq!(s.button_padding.x, metrics::CONTROL_PADDING_X);
        let size = |ts: TextStyle| style.text_styles[&ts].size;
        assert_eq!(size(TextStyle::Body), metrics::BODY_SIZE);
        assert_eq!(size(TextStyle::Button), metrics::BODY_SIZE);
        assert_eq!(size(TextStyle::Small), metrics::SMALL_SIZE);
        assert_eq!(size(TextStyle::Heading), metrics::HEADING_SIZE);
        assert_eq!(size(TextStyle::Monospace), metrics::MONO_SIZE);
    }
    // egui switched to its other style: the tokens follow it.
    ctx.set_theme(egui::Theme::Dark);
    assert_eq!(mg_uikit::applied(&ctx), Some(Theme::Dark));
    assert_eq!(ctx.tokens(), Tokens::DARK);
    mg_uikit::apply(&ctx, Theme::Light);
    assert_eq!(ctx.tokens(), Tokens::LIGHT);
}

/// What the style had that is not the look's is kept (the script
/// editor halves the font texture's size).
#[test]
fn apply_keeps_what_is_not_the_look() {
    let ctx = egui::Context::default();
    ctx.all_styles_mut(|s| {
        s.visuals.text_options.max_texture_side = 1234;
        s.animation_time = 0.5;
    });
    mg_uikit::apply(&ctx, Theme::Dark);
    for theme in Theme::ALL {
        let style = ctx.style_of(theme.egui());
        assert_eq!(style.visuals.text_options.max_texture_side, 1234);
        assert_eq!(style.animation_time, 0.5);
    }
}

/// Applied before the context's first frame or in it: the heading text
/// style (window titles, `ui.heading`) is the body's face, not bold; the
/// bold family is in the fonts for the section headings, which are the
/// one bold.
#[test]
fn the_heading_style_is_not_bold_and_the_bold_is_there_for_sections() {
    for before in [true, false] {
        let ctx_before = std::cell::Cell::new(before);
        let mut h = Harness::builder().with_size(egui::vec2(300.0, 100.0)).build_ui(|ui| {
            if !ctx_before.get() {
                mg_uikit::apply(ui.ctx(), Theme::Dark);
            }
            ui.heading("Lighting Scheme");
            mg_uikit::section_heading(ui, "Basic");
        });
        if before {
            mg_uikit::apply(&h.ctx, Theme::Dark);
            ctx_before.set(false);
        }
        h.run();
        for theme in Theme::ALL {
            let style = h.ctx.style_of(theme.egui());
            let heading = &style.text_styles[&TextStyle::Heading];
            assert_eq!(heading.family, egui::FontFamily::Proportional);
            assert_eq!(heading.size, metrics::HEADING_SIZE);
        }
        assert!(fonts::has_bold(&h.ctx));
        let bold = h.output().shapes.iter().any(|s| match &s.shape {
            egui::Shape::Text(t) if t.galley.text() == "Basic" => {
                t.galley.job.sections[0].format.font_id.family == fonts::bold()
            }
            _ => false,
        });
        assert!(bold, "the section heading is bold");
    }
}

/// Unapplied, the form helpers are as before the redesign (egui's strong
/// text, in either of its themes); applied, the tokens'.
#[test]
fn the_form_helpers_follow_the_theme_only_once_applied() {
    for egui_theme in [egui::Theme::Dark, egui::Theme::Light] {
        let mut h = Harness::builder().with_theme(egui_theme).build_ui(|ui| {
            mg_uikit::field_label(ui, "Tag");
        });
        h.run();
        let strong = h.ctx.style_of(egui_theme).visuals.strong_text_color();
        assert_eq!(text_color(&h, "Tag"), Some(strong), "{egui_theme:?}");
    }
    for theme in Theme::ALL {
        let mut h = Harness::new_ui(|ui| {
            mg_uikit::apply(ui.ctx(), theme);
            mg_uikit::field_label(ui, "Tag");
            mg_uikit::section_heading(ui, "Basic");
        });
        h.run();
        assert_eq!(text_color(&h, "Tag"), Some(theme.tokens().text), "{theme:?}");
        assert_eq!(text_color(&h, "Basic"), Some(theme.tokens().text), "{theme:?}");
    }
}

/// Calling it every frame costs nothing once done: it asks for no more
/// frames, so the harness settles.
#[test]
fn apply_every_frame_settles() {
    let mut h = Harness::new_ui(|ui| {
        mg_uikit::apply(ui.ctx(), Theme::Light);
        ui.label("Aribeth");
    });
    let steps = h.run();
    assert!(steps < 6, "{steps} frames");
    assert_eq!(h.ctx.theme(), egui::Theme::Light);
}
