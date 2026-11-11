//! The color tokens of the redesign: one [`Color32`] for each custom
//! property of the mockups' stylesheet (`moonglow.css`, the `.mg` block,
//! dark) and its light theme (the `.mg.light` block), plus the colors the
//! stylesheet had fixed that need a token of their own for the light
//! theme (the mockup spec, section 12).
//!
//! Three surface levels ([`Tokens::base`] behind, [`Tokens::panel`] the
//! bodies, [`Tokens::bar`] the bars), [`Tokens::raised`] for controls,
//! [`Tokens::inset`] for wells, 1 px lines, one accent. Every text token
//! meets 4.5:1 on the surfaces it is used on in both themes but
//! [`Tokens::text_off`] (disabled); the exceptions the mockups document are
//! in this module's tests.

use egui::Color32;

/// An opaque color from its hex value (`0x15171a`).
const fn hex(rgb: u32) -> Color32 {
    Color32::from_rgb((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8)
}

/// A color with the stylesheet's alpha (`rgba(21, 23, 26, 0.86)`).
const fn rgba(r: u8, g: u8, b: u8, alpha: f32) -> Color32 {
    Color32::from_rgba_unmultiplied_const(r, g, b, (alpha * 255.0 + 0.5) as u8)
}

macro_rules! tokens {
    ($( $(#[doc = $doc:literal])* $name:ident: $css:literal = $dark:expr, $light:expr; )*) => {
        /// The theme's colors, one field per token (the stylesheet's custom
        /// property, `--raised-hover` as `raised_hover`). [`Tokens::DARK`]
        /// and [`Tokens::LIGHT`] hold the mockups' values.
        #[derive(Clone, Copy, Debug, PartialEq)]
        pub struct Tokens {
            $( $(#[doc = $doc])* pub $name: Color32, )*
        }

        impl Tokens {
            /// The dark theme (the default).
            pub const DARK: Tokens = Tokens { $( $name: $dark, )* };
            /// The light theme.
            pub const LIGHT: Tokens = Tokens { $( $name: $light, )* };

            /// Every token: its custom property's name in the stylesheet
            /// (`raised-hover`) and its value in this theme, in the order
            /// of the fields.
            pub fn all(&self) -> Vec<(&'static str, Color32)> {
                vec![ $( ($css, self.$name), )* ]
            }
        }
    };
}

tokens! {
    // --- surfaces
    /// Behind the docks; the 3D view's surround.
    base: "base" = hex(0x15171a), hex(0xc4c9d0);
    /// Dock and editor bodies; windows and popovers.
    panel: "panel" = hex(0x1e2125), hex(0xe4e7eb);
    /// The top bar, tab bars, editor headers, the status bar; table headers.
    bar: "bar" = hex(0x262a2f), hex(0xd8dce1);
    /// Buttons, combo boxes, toggles that are off.
    raised: "raised" = hex(0x30353c), hex(0xc9ced5);
    /// The same, hovered.
    raised_hover: "raised-hover" = hex(0x3b414a), hex(0xbcc2ca);
    /// Text fields, list wells, the code editor.
    inset: "inset" = hex(0x131518), hex(0xf4f5f7);
    /// The 1 px separators between regions.
    line: "line" = hex(0x0c0d0f), hex(0xadb3bb);
    /// Control borders, rules inside a panel.
    line_soft: "line-soft" = hex(0x383d45), hex(0xb9bfc7);
    /// A hovered tree, list or popover row.
    hover_row: "hover-row" = hex(0x2a2e34), hex(0xd5d9df);
    /// The code editor's current line.
    current_line: "current-line" = hex(0x1b1e22), hex(0xe9edf3);
    /// Under a 3D view's overlays (buttons, labels, the compass ring).
    overlay: "overlay" = rgba(21, 23, 26, 0.86), rgba(232, 235, 239, 0.90);

    // --- text
    /// Primary text, field labels.
    text: "text" = hex(0xe6e8ec), hex(0x14171b);
    /// Secondary text: inactive tabs, read-only values, icons at rest.
    text_2: "text-2" = hex(0xaab0ba), hex(0x363c44);
    /// Weak text: counts, hints, notes, placeholders.
    text_3: "text-3" = hex(0x8a919e), hex(0x4c535b);
    /// Disabled (the one text token under 4.5:1).
    text_off: "text-off" = hex(0x5f6672), hex(0x838a95);

    // --- accent and state
    /// The accent as text, an icon or a line (a focused field's border,
    /// the active tab's top line).
    accent: "accent" = hex(0x62a8ea), hex(0x175896);
    /// Toggled on, the selected row, the main action.
    accent_fill: "accent-fill" = hex(0x1d5488), hex(0x2765a1);
    /// The same, hovered.
    accent_fill_hover: "accent-fill-hover" = hex(0x24629d), hex(0x1f598f);
    /// Text and icons on [`Tokens::accent_fill`].
    on_fill: "on-fill" = hex(0xffffff), hex(0xffffff);
    /// Selection in a 3D view; the modified dot.
    select: "select" = hex(0xffb02e), hex(0xf09000);
    /// Errors (as text: 4.5:1 on panel and bar).
    error: "error" = hex(0xf0666a), hex(0xb4232b);
    /// Warnings.
    warn: "warn" = hex(0xe8a33d), hex(0x8a5000);
    /// Success.
    ok: "ok" = hex(0x4cc35a), hex(0x1b6b2d);
    /// Information.
    info: "info" = hex(0x5b9bd5), hex(0x1f5a98);

    // --- drawn on a thumbnail (the renders keep their dark background in
    // both themes, so these keep one value)
    /// A thumbnail's own render background, (41, 46, 54).
    thumb_bg: "thumb-bg" = hex(0x292e36), hex(0x292e36);
    /// A favorite's bookmark on a tile.
    thumb_flag: "thumb-flag" = hex(0xe8a33d), hex(0xe8a33d);
    /// The "No model" badge on a tile.
    thumb_badge: "thumb-badge" = hex(0xf0666a), hex(0xf0666a);
    /// Its text.
    thumb_badge_text: "thumb-badge-text" = hex(0x15171a), hex(0x15171a);

    // --- shadow (the stylesheet's `--shadow`: a drop shadow and a rim)
    /// The drop shadow under popovers, menus and tooltips (6 px down, 18
    /// px blur), the only shadow there is.
    shadow: "shadow" = rgba(0, 0, 0, 0.55), rgba(28, 34, 44, 0.22);
    /// The 1 px rim of the same shadow, around the popover.
    shadow_rim: "shadow-rim" = rgba(0, 0, 0, 0.4), rgba(28, 34, 44, 0.10);

    // --- the stylesheet's fixed colors given tokens (the spec's section 12)
    /// A toggle or checkbox that is on but disabled (`#22364a`).
    accent_fill_off: "accent-fill-off" = hex(0x22364a), hex(0xc4d4e5);
    /// Its mark or text (dark: [`Tokens::text_off`]).
    on_fill_off: "on-fill-off" = hex(0x5f6672), hex(0x7189a3);
    /// A hovered document or dock tab (`#2b2f35`).
    tab_hover: "tab-hover" = hex(0x2b2f35), hex(0xdee1e6);
    /// The lines between a table's cells (`#25292e`).
    cell_line: "cell-line" = hex(0x25292e), hex(0xd3d7dc);
    /// A table's selected cell, under an accent outline (`#1a2a3a`).
    cell_select: "cell-select" = hex(0x1a2a3a), hex(0xd3e2f2);
    /// Text on a status badge (error, warn, ok, info fills; dark
    /// `#15171a`, light white).
    on_status: "on-status" = hex(0x15171a), hex(0xffffff);
    /// Text on [`Tokens::select`]: the 3D view's name tag.
    on_select: "on-select" = hex(0x15171a), hex(0x15171a);
    /// A scroll bar's thumb (dark: [`Tokens::raised_hover`]).
    scroll_thumb: "scroll-thumb" = hex(0x3b414a), hex(0xb3b9c2);
    /// The same, hovered or dragged (`#555c67`).
    scroll_thumb_hover: "scroll-thumb-hover" = hex(0x555c67), hex(0x8d949e);
    /// The hairlines above and below the code editor's current line
    /// (`#262a30`).
    current_line_edge: "current-line-edge" = hex(0x262a30), hex(0xdce1e8);
    /// Selected text in the code editor (`#264f78`).
    code_select: "code-select" = hex(0x264f78), hex(0xadd6ff);
    /// The line inside a color swatch, around its color.
    swatch_line: "swatch-line" = rgba(0, 0, 0, 0.35), rgba(0, 0, 0, 0.25);
    /// A key cap's fill (dark: [`Tokens::raised`], light:
    /// [`Tokens::inset`]); its border is [`Tokens::line_soft`].
    key_bg: "key-bg" = hex(0x30353c), hex(0xf4f5f7);
    /// A key cap's text (dark: [`Tokens::text`], light: [`Tokens::text_2`]).
    key_text: "key-text" = hex(0xe6e8ec), hex(0x363c44);
    /// A tooltip's fill (dark: [`Tokens::base`], light: [`Tokens::inset`]).
    tooltip_bg: "tooltip-bg" = hex(0x15171a), hex(0xf4f5f7);
    /// A switch's track when off (dark: [`Tokens::raised_hover`]); on, it
    /// is [`Tokens::accent_fill`].
    switch_off: "switch-off" = hex(0x3b414a), hex(0xa9b0b9);
    /// A switch's knob when off (dark: [`Tokens::text_2`]); on, it is
    /// [`Tokens::on_fill`].
    switch_knob: "switch-knob" = hex(0xaab0ba), hex(0xffffff);
    /// A slider's thumb (dark: [`Tokens::text_2`]).
    slider_thumb: "slider-thumb" = hex(0xaab0ba), hex(0xffffff);
    /// The ring around a slider's thumb (none in the dark theme).
    slider_thumb_ring: "slider-thumb-ring" = Color32::TRANSPARENT, hex(0x8d949e);
    /// The ring that carries the modified dot ([`Tokens::select`]) on a
    /// light bar (none in the dark theme).
    select_ring: "select-ring" = Color32::TRANSPARENT, hex(0xa35c00);

    // --- the script editor's present colors (Options › Script Editor)
    /// Script text.
    syn_text: "syn-text" = hex(0xe6e8ec), hex(0x14171b);
    /// Comments.
    syn_comment: "syn-comment" = hex(0x6a9955), hex(0x008000);
    /// Preprocessor directives (`#include`).
    syn_directive: "syn-directive" = hex(0xc586c0), hex(0x800080);
    /// Keywords.
    syn_keyword: "syn-keyword" = hex(0x569cd6), hex(0x0000c8);
    /// Numbers.
    syn_number: "syn-number" = hex(0xce9178), hex(0xa31515);
    /// Strings.
    syn_string: "syn-string" = hex(0xce9178), hex(0xa31515);
    /// Constants.
    syn_constant: "syn-constant" = hex(0x4fc1ff), hex(0x0070c1);
    /// Errors.
    syn_error: "syn-error" = hex(0xff0000), hex(0xff0000);

    // --- the viewer's ASCII model colors
    /// Plain text.
    mdl_plain: "mdl-plain" = hex(0xd4d4d4), hex(0x202020);
    /// Structure (`node`, `endnode`).
    mdl_structure: "mdl-structure" = hex(0xc586c0), hex(0xaf00db);
    /// Keywords.
    mdl_keyword: "mdl-keyword" = hex(0x9cdcfe), hex(0x001080);
    /// A keyed value.
    mdl_keyed: "mdl-keyed" = hex(0x4ec9b0), hex(0x267f99);
    /// A node's type.
    mdl_node_type: "mdl-node-type" = hex(0xdcdcaa), hex(0x795e26);
    /// Numbers.
    mdl_number: "mdl-number" = hex(0xb5cea8), hex(0x098658);
    /// Comments.
    mdl_comment: "mdl-comment" = hex(0x6a9955), hex(0x008000);
}

impl Default for Tokens {
    fn default() -> Self {
        Self::DARK
    }
}

/// WCAG 2's contrast ratio of two opaque colors (1 to 21).
pub fn contrast(a: Color32, b: Color32) -> f32 {
    fn luminance(c: Color32) -> f32 {
        let channel = |v: u8| {
            let v = f32::from(v) / 255.0;
            if v <= 0.040_45 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) }
        };
        0.2126 * channel(c.r()) + 0.7152 * channel(c.g()) + 0.0722 * channel(c.b())
    }
    let (la, lb) = (luminance(a), luminance(b));
    (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
}

/// `top` laid over `under` (`top`'s alpha, unmultiplied, as a browser
/// draws `opacity`), opaque.
pub fn over(top: Color32, alpha: f32, under: Color32) -> Color32 {
    let mix = |t: u8, u: u8| (f32::from(t) * alpha + f32::from(u) * (1.0 - alpha)).round() as u8;
    Color32::from_rgb(mix(top.r(), under.r()), mix(top.g(), under.g()), mix(top.b(), under.b()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The text tokens on the surfaces they are used on (the spec's
    /// section 5 and the light theme's ratios): 4.5:1 or more in both
    /// themes.
    #[test]
    fn text_meets_four_and_a_half_to_one() {
        for (theme, t) in [("dark", Tokens::DARK), ("light", Tokens::LIGHT)] {
            let pairs = [
                ("text", t.text, "panel", t.panel),
                ("text", t.text, "bar", t.bar),
                ("text", t.text, "raised", t.raised),
                ("text", t.text, "raised-hover", t.raised_hover),
                ("text", t.text, "inset", t.inset),
                ("text", t.text, "base", t.base),
                ("text", t.text, "hover-row", t.hover_row),
                ("text", t.text, "tooltip-bg", t.tooltip_bg),
                ("text-2", t.text_2, "panel", t.panel),
                ("text-2", t.text_2, "bar", t.bar),
                ("text-2", t.text_2, "raised", t.raised),
                ("text-2", t.text_2, "inset", t.inset),
                ("text-3", t.text_3, "panel", t.panel),
                ("text-3", t.text_3, "bar", t.bar),
                ("text-3", t.text_3, "base", t.base),
                ("text-3", t.text_3, "inset", t.inset),
                ("accent", t.accent, "panel", t.panel),
                ("accent", t.accent, "bar", t.bar),
                ("error", t.error, "panel", t.panel),
                ("error", t.error, "bar", t.bar),
                ("warn", t.warn, "panel", t.panel),
                ("warn", t.warn, "bar", t.bar),
                ("ok", t.ok, "panel", t.panel),
                ("ok", t.ok, "bar", t.bar),
                ("info", t.info, "panel", t.panel),
                ("info", t.info, "bar", t.bar),
                ("on-fill", t.on_fill, "accent-fill", t.accent_fill),
                ("on-fill", t.on_fill, "accent-fill-hover", t.accent_fill_hover),
                ("on-status", t.on_status, "error", t.error),
                ("on-status", t.on_status, "warn", t.warn),
                ("on-status", t.on_status, "ok", t.ok),
                ("on-status", t.on_status, "info", t.info),
                ("on-select", t.on_select, "select", t.select),
                ("key-text", t.key_text, "key-bg", t.key_bg),
                ("thumb-badge-text", t.thumb_badge_text, "thumb-badge", t.thumb_badge),
            ];
            for (fg, a, bg, b) in pairs {
                let r = contrast(a, b);
                assert!(r >= 4.5, "{theme}: {fg} on {bg} is {r:.2}:1");
            }
            // A selected row's count: the text at 85 %.
            let count = over(t.on_fill, 0.85, t.accent_fill);
            assert!(contrast(count, t.accent_fill) >= 4.5, "{theme}: a selected row's count");
        }
    }

    /// The exceptions the mockups document, held to what they are.
    #[test]
    fn the_documented_exceptions() {
        for (theme, t) in [("dark", Tokens::DARK), ("light", Tokens::LIGHT)] {
            // Disabled text is exempt, and is under 4.5:1 (that is how it
            // reads as disabled).
            for (bg, b) in [("panel", t.panel), ("raised", t.raised), ("bar", t.bar)] {
                let r = contrast(t.text_off, b);
                assert!((2.0..4.5).contains(&r), "{theme}: text-off on {bg} is {r:.2}:1");
            }
            // A chevron or a key cap's plus on a control: 3:1, as a
            // graphic.
            let r = contrast(t.text_3, t.raised);
            assert!(r >= 3.0, "{theme}: text-3 on raised is {r:.2}:1");
            // The ring that carries the modified dot, as a graphic.
            if t.select_ring != Color32::TRANSPARENT {
                for b in [t.bar, t.panel] {
                    assert!(contrast(t.select_ring, b) >= 3.0, "{theme}: the dot's ring");
                }
            }
        }
        // The dark theme's count on a hovered row: 4.3:1, documented
        // under 4.5 (a count is weak text, and the row is hovered).
        let r = contrast(Tokens::DARK.text_3, Tokens::DARK.hover_row);
        assert!((4.0..4.5).contains(&r), "dark: text-3 on hover-row is {r:.2}:1");
        assert!(contrast(Tokens::LIGHT.text_3, Tokens::LIGHT.hover_row) >= 4.5);
        // The modified dot on a light bar needs its ring (1.75:1 alone);
        // on the dark bar it stands alone.
        assert!(contrast(Tokens::LIGHT.select, Tokens::LIGHT.bar) < 3.0);
        assert!(contrast(Tokens::DARK.select, Tokens::DARK.bar) >= 4.5);
        assert_eq!(Tokens::DARK.select_ring, Color32::TRANSPARENT);
    }

    /// The ratio itself, against known values.
    #[test]
    fn contrast_is_wcag_s() {
        assert!((contrast(Color32::BLACK, Color32::WHITE) - 21.0).abs() < 0.01);
        assert!((contrast(Color32::WHITE, Color32::WHITE) - 1.0).abs() < 0.001);
        // The light theme's text on panel, as the mockups' ratios give it.
        assert!((contrast(Tokens::LIGHT.text, Tokens::LIGHT.panel) - 14.49).abs() < 0.02);
        assert!((contrast(Tokens::DARK.text_3, Tokens::DARK.bar) - 4.55).abs() < 0.02);
    }
}
