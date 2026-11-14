//! The editor area's look, for an application whose editor area is an
//! egui_dock `DockArea` (the `dock` feature): egui_dock's [`Style`] made
//! from the tokens (the mock's `.doctabs`, `.doctab`), and what its style
//! cannot say, painted over its tabs once they are drawn ([`marks`]: the
//! active tab's line along its top, the lines between tabs).
//!
//! The docks around the editor area are not egui_dock's (egui_dock has one
//! style for a `DockArea`, and the dock tabs are 26 px where the document
//! tabs are 28): they are the kit's own [`crate::shell::DockTabs`].

use egui_dock::{DockState, Style};

use crate::metrics;
use crate::theme::TokensExt as _;
use crate::tokens::Tokens;

/// egui_dock's style in the theme `ui` wears: the tab bars `bar`, 28 px
/// high, a 1 px `line` under them; the active tab `panel` with its words
/// in `text`, the others in `text-2` on the bar, hovered `tab-hover`;
/// no outlines (the active tab's accent line and the lines between tabs
/// are [`marks`]'), a tab reached with the keyboard ringed in `accent`;
/// the seams between panes 1 px of `line`, `accent` held; the close
/// buttons `text-2`, `text` and `raised` hovered.
pub fn style(ui: &egui::Ui) -> Style {
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
/// is drawn (`tabs`: each tab's rectangle, layer and the part of it in
/// sight, as its button was drawn this frame): a 1 px `line` at each
/// tab's right (the mock's `border-right`), and along the top of each
/// pane's tab in front a 2 px line, `accent` in the pane that has the
/// focus and `line-soft` in the others (the mock's
/// `.group.focus .doctab.active`). Each only where its tab is in sight:
/// egui_dock scrolls a bar of more tabs than it holds and still gives
/// those out of sight their buttons, whose marks were drawn over what
/// lies beside the bar.
pub fn marks<T: PartialEq>(
    ctx: &egui::Context,
    dock: &DockState<T>,
    tabs: &[(T, egui::Rect, egui::LayerId, egui::Rect)],
) {
    marks_with(ctx, dock, dock.focused_leaf(), tabs);
}

/// [`marks`] with the pane that has the focus given.
fn marks_with<T: PartialEq>(
    ctx: &egui::Context,
    dock: &DockState<T>,
    focused: Option<egui_dock::NodePath>,
    tabs: &[(T, egui::Rect, egui::LayerId, egui::Rect)],
) {
    let t = ctx.tokens();
    for (_, rect, layer, shown) in tabs {
        if !shown.is_positive() {
            continue;
        }
        let painter = ctx.layer_painter(*layer).with_clip_rect(*shown);
        painter.vline(rect.right() - 0.5, rect.y_range(), egui::Stroke::new(1.0, t.line));
    }
    for (path, leaf) in dock.iter_leaves() {
        let Some(front) = leaf.tabs.get(leaf.active.0) else { continue };
        let Some((_, rect, layer, shown)) = tabs.iter().find(|(t, ..)| t == front) else {
            continue;
        };
        if !shown.is_positive() {
            continue;
        }
        let color = front_line(&t, focused == Some(path));
        let line = egui::Rect::from_min_size(
            rect.min,
            egui::vec2(rect.width() - 1.0, metrics::ACTIVE_TAB_LINE),
        );
        ctx.layer_painter(*layer).with_clip_rect(*shown).rect_filled(line, 0.0, color);
        crate::icons::record_line(ctx, line, color);
    }
}

/// The color of the line along the top of a pane's tab in front: `accent`
/// in the pane that has the focus, `line-soft` in the others.
pub fn front_line(t: &Tokens, focused: bool) -> egui::Color32 {
    if focused { t.accent } else { t.line_soft }
}

// ------------------------------------------------------------------ document tabs

/// [`style`] for an editor area of document tabs drawn with
/// [`doc_tab_title`] and [`doc_tab_marks`]: no scroll bar under a tab bar
/// with more tabs than it holds (egui_dock's, in egui's colors, took 7.5
/// points of the editor under it; the bar scrolls by the wheel, and the
/// application keeps the tab in front in sight), and the drop targets of
/// a tab being dragged in the theme's colors (their window shapes drawn in
/// `text-2`, as egui_dock draws them in egui's text color, and ringed in
/// `text-3`; the place it would go `accent` tinted and lined).
pub fn doc_tabs_style(ui: &egui::Ui) -> Style {
    let t = ui.tokens();
    let mut s = style(ui);
    s.tab_bar.show_scroll_bar_on_overflow = false;
    let o = &mut s.overlay;
    // (The targets' shapes are lines and a filled strip over whatever lies
    // under them, a toolbar's words too: drawn in `raised` they read as a
    // dark box over those words, and the shapes were lost.)
    o.button_color = t.text_2;
    o.button_border_stroke = egui::Stroke::new(1.0, t.text_3);
    o.hovered_leaf_highlight.color = t.accent.gamma_multiply(0.12);
    o.hovered_leaf_highlight.stroke = egui::Stroke::new(1.0, t.accent);
    s
}

/// The room egui_dock gives a tab's title at each side (its `tab_title`'s
/// spacing), which [`doc_tab_title`] takes off the tab's padding.
pub const DOCK_TITLE_SPACING: f32 = 8.0;

/// The least a document tab's title is cut to where its bar has no room
/// for every tab whole (about eight letters; the whole title in its tip).
pub const DOC_TAB_LEAST_TITLE: f32 = 64.0;

/// A document tab's title as [`doc_tab_title`] lays it out for egui_dock.
#[derive(Clone, Debug)]
pub struct DocTabTitle {
    /// What egui_dock is given to draw (`WidgetText::Galley`).
    pub galley: std::sync::Arc<egui::Galley>,
    /// The title's own width, as laid out (cut or whole).
    pub width: f32,
    /// Whether it was cut short (its whole title then goes in its tip).
    pub cut: bool,
}

/// A document tab's title as egui_dock lays it out: `title` in `font`, cut
/// in the middle where it is wider than `max` ([`middle_cut`]: its start
/// and its end kept, the extension whole), with the room of the tab's
/// other parts inside it (the kind icon before it, the modified dot where
/// `modified` and the close button after it,
/// [`crate::shell::doc_tab_parts`]), so that the tab egui_dock makes of it
/// (the galley and [`DOCK_TITLE_SPACING`] at each side) is
/// [`crate::shell::doc_tab_width`] wide and its words stand where those
/// parts leave them. The room after the close button is kept whether the
/// button shows or not, so that its words do not move as it comes and
/// goes. Its words are `PLACEHOLDER`: egui_dock paints them in its tab's
/// state's color.
pub fn doc_tab_title(
    ctx: &egui::Context,
    font: egui::FontId,
    title: &str,
    max: f32,
    modified: bool,
) -> DocTabTitle {
    use crate::shell::{DOC_TAB_CLOSE, DOC_TAB_GAP, DOC_TAB_PADDING};
    let lead = DOC_TAB_PADDING.0 + metrics::ICON + DOC_TAB_GAP - DOCK_TITLE_SPACING;
    let dot = if modified { DOC_TAB_GAP + crate::marks::DOT } else { 0.0 };
    let trail = dot + DOC_TAB_GAP + DOC_TAB_CLOSE + DOC_TAB_PADDING.1 - DOCK_TITLE_SPACING;
    let format = egui::TextFormat {
        font_id: font.clone(),
        color: egui::Color32::PLACEHOLDER,
        ..Default::default()
    };
    let measure = |text: &str| {
        let galley = ctx.fonts_mut(|f| {
            f.layout_no_wrap(text.to_owned(), font.clone(), egui::Color32::PLACEHOLDER)
        });
        galley.rect.width()
    };
    let shown = middle_cut(title, max.max(1.0), &measure);
    let cut = shown != title;
    let mut job = egui::text::LayoutJob::default();
    job.append(&shown, lead, format);
    let laid = ctx.fonts_mut(|f| f.layout_job(job));
    let width = (laid.rect.width() - lead).max(0.0);
    let mut galley = (*laid).clone();
    galley.rect.min.x = 0.0;
    galley.rect.max.x = lead + width + trail;
    DocTabTitle { galley: std::sync::Arc::new(galley), width, cut }
}

/// The longest an extension kept whole by [`middle_cut`] may be, in
/// characters after its dot.
const EXTENSION_MOST: usize = 8;

/// `title` as a crowded tab bar shows it in `max` (as `width` measures
/// text; the owner's ruling of 2026-11-14, "Tab cut in the middle to
/// keep"): whole where it fits; else as much of its start and of its end
/// as fits, an ellipsis between them ("at…01.nss"), the start a character
/// longer where they cannot be alike. Its extension (after its last dot,
/// one to [`EXTENSION_MOST`] letters or digits, the dot not its first
/// character) is kept whole with the end wherever a character of the name
/// before it fits too; where none does, or the name has no extension, the
/// whole of it is cut in the middle all the same; where not even a
/// character at each side of the ellipsis fits, the ellipsis alone. A
/// name of one character is never cut (an ellipsis is no shorter). Cut by
/// characters (egui lays out one glyph a character: a sequence joined by
/// zero-width joiners, or a letter and its combining marks, may be parted).
fn middle_cut(title: &str, max: f32, width: &dyn Fn(&str) -> f32) -> String {
    let chars: Vec<char> = title.chars().collect();
    if chars.len() < 2 || width(title) <= max {
        return title.to_owned();
    }
    let dot = chars.iter().rposition(|&c| c == '.').filter(|&at| {
        let ext = &chars[at + 1..];
        at > 0
            && !ext.is_empty()
            && ext.len() <= EXTENSION_MOST
            && ext.iter().all(|c| c.is_alphanumeric())
    });
    // `kept` characters of `name`, half at each end of the ellipsis
    // (the start the longer), then `after`.
    let shown = |name: &[char], after: &[char], kept: usize| -> String {
        let (head, tail) = (kept.div_ceil(2), kept / 2);
        let mut s: String = name[..head].iter().collect();
        s.push('…');
        s.extend(&name[name.len() - tail..]);
        s.extend(after);
        s
    };
    // The most characters of `name` that fit with `after`: at least
    // `least`, fewer than all of it (a width grows with what is kept).
    let most = |name: &[char], after: &[char], least: usize| -> Option<String> {
        let fits = |kept: usize| width(&shown(name, after, kept)) <= max;
        if name.len() <= least || !fits(least) {
            return None;
        }
        let (mut fit, mut over) = (least, name.len());
        while over - fit > 1 {
            let mid = (fit + over) / 2;
            if fits(mid) { fit = mid } else { over = mid }
        }
        Some(shown(name, after, fit))
    };
    dot.and_then(|at| most(&chars[..at], &chars[at..], 1))
        .or_else(|| most(&chars, &[], 2))
        .unwrap_or_else(|| "…".to_owned())
}

/// The widest each title of a tab bar may be for the bar's tabs to fit
/// `room` (titles `widths` wide, those `modified` with their dot), cut
/// alike from the widest down: `None` where all fit whole; never under
/// [`DOC_TAB_LEAST_TITLE`] (a bar of more tabs than that leaves room for
/// scrolls).
pub fn doc_tab_cap(room: f32, widths: &[(f32, bool)]) -> Option<f32> {
    use crate::shell::doc_tab_width;
    let total =
        |cap: f32| -> f32 { widths.iter().map(|&(w, m)| doc_tab_width(w.min(cap), m)).sum() };
    if total(f32::INFINITY) <= room {
        return None;
    }
    if total(DOC_TAB_LEAST_TITLE) >= room {
        return Some(DOC_TAB_LEAST_TITLE);
    }
    // (The total grows with the cap: halved between one that fits and
    // one that does not, to a point.)
    let (mut fits, mut over) =
        (DOC_TAB_LEAST_TITLE, widths.iter().map(|w| w.0).fold(0.0, f32::max));
    while over - fits > 0.5 {
        let mid = (fits + over) / 2.0;
        if total(mid) <= room { fits = mid } else { over = mid }
    }
    Some(fits.floor())
}

/// A document tab drawn this frame, for [`doc_tab_marks`]: where egui_dock
/// put its button (`rect`, on `layer`, the part of it in sight `shown`),
/// its kind icon, its title's width as laid out ([`DocTabTitle::width`])
/// and its state.
#[derive(Clone, Debug)]
pub struct DocTabMark<T> {
    pub tab: T,
    pub rect: egui::Rect,
    pub layer: egui::LayerId,
    pub shown: egui::Rect,
    pub icon: crate::Icon,
    pub title_width: f32,
    pub modified: bool,
    /// The tab in front of its pane.
    pub active: bool,
    /// Its close button and how it is drawn; `None` where it is not shown
    /// (a tab neither in front nor under the pointer, or one that cannot
    /// be closed).
    pub close: Option<crate::icons::ButtonState>,
}

/// [`marks`] for document tabs, with their parts (the mock's `.doctab`):
/// the lines between the tabs and along the top of each pane's tab in
/// front as [`marks`] paints them, and in each tab its kind icon
/// (`text-3`, `text-2` in front), the modified dot where modified and its
/// close button where shown, where [`crate::shell::doc_tab_parts`] puts
/// them; each only in the part of its tab in sight. The accent line is
/// over the tab in front of `focused`, the pane with the focus (which the
/// application may know where egui_dock does not yet: the editor area's
/// only pane as the window opens, before any click).
pub fn doc_tab_marks<T: PartialEq + Clone>(
    ctx: &egui::Context,
    dock: &DockState<T>,
    focused: Option<egui_dock::NodePath>,
    tabs: &[DocTabMark<T>],
) {
    let lines: Vec<(T, egui::Rect, egui::LayerId, egui::Rect)> =
        tabs.iter().map(|m| (m.tab.clone(), m.rect, m.layer, m.shown)).collect();
    marks_with(ctx, dock, focused, &lines);
    for m in tabs {
        if m.shown.is_positive() {
            paint_doc_tab_parts(&ctx.layer_painter(m.layer).with_clip_rect(m.shown), m);
        }
    }
}

/// A document tab's kind icon, modified dot and close button, painted
/// where [`crate::shell::doc_tab_parts`] puts them in `m.rect` (the tab
/// being dragged, on its own layer: [`doc_tab_marks`] paints the others).
pub fn paint_doc_tab_parts<T>(painter: &egui::Painter, m: &DocTabMark<T>) {
    let t = painter.ctx().tokens();
    let parts = crate::shell::doc_tab_parts(m.rect, m.title_width, m.modified);
    crate::shell::paint_doc_tab_icon(painter, &t, parts.icon, m.icon, m.active);
    if let Some(dot) = parts.dot {
        crate::marks::paint_modified_dot(painter, &t, dot.center());
    }
    if let Some(state) = m.close {
        crate::shell::paint_doc_tab_close(painter, &t, parts.close, state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The dock's style follows the theme the context wears, both ways.
    #[test]
    fn the_dock_s_style_follows_the_theme() {
        let ctx = egui::Context::default();
        for theme in [crate::Theme::Light, crate::Theme::Dark, crate::Theme::Light] {
            crate::apply(&ctx, theme);
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
                // The drop targets read against the editors under them.
                let d = doc_tabs_style(ui);
                let against = crate::tokens::contrast(d.overlay.button_color, t.panel);
                assert!(against >= 4.5, "{theme:?}: drop targets {against}");
            });
            out.textures_delta.clear();
        }
        assert_eq!(front_line(&Tokens::DARK, true), Tokens::DARK.accent);
        assert_eq!(front_line(&Tokens::LIGHT, false), Tokens::LIGHT.line_soft);
    }

    /// A document tab's title holds the room of its other parts: the tab
    /// egui_dock makes of it is `doc_tab_width` wide, its words where
    /// `doc_tab_parts` puts the title, with or without the dot; cut to
    /// `max`, it says so and is no wider than that.
    #[test]
    fn a_document_tab_s_title_holds_its_parts_room() {
        let ctx = egui::Context::default();
        crate::apply(&ctx, crate::Theme::Dark);
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
            let font = egui::TextStyle::Button.resolve(ui.style());
            for modified in [false, true] {
                let t = doc_tab_title(ui.ctx(), font.clone(), "map_m1q1a", f32::INFINITY, modified);
                assert!(!t.cut && t.width > 40.0);
                let tab = t.galley.rect.width() + 2.0 * DOCK_TITLE_SPACING;
                let want = crate::shell::doc_tab_width(t.width, modified);
                assert!((tab - want).abs() < 0.01, "{tab} {want}");
                // (The words start after the icon and its gap.)
                let first = t.galley.rows[0].glyphs[0].pos.x + DOCK_TITLE_SPACING;
                let parts = crate::shell::doc_tab_parts(
                    egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(tab, 28.0)),
                    t.width,
                    modified,
                );
                assert!((first - parts.title.left()).abs() < 1.5, "{first} {:?}", parts.title);
                assert_eq!(t.galley.rect.min.x, 0.0);
            }
            let whole = doc_tab_title(ui.ctx(), font.clone(), "a_long_script_name.nss", 1e9, false);
            let cut = doc_tab_title(ui.ctx(), font, "a_long_script_name.nss", 64.0, false);
            assert!(cut.cut && cut.width <= 64.0 && cut.width < whole.width, "{cut:?}");
            let text: String = cut.galley.rows[0].glyphs.iter().map(|g| g.chr).collect();
            assert!(text.starts_with('a') && text.ends_with("e.nss"), "{text}");
            assert!(text.contains('…'), "{text}");
        });
        out.textures_delta.clear();
    }

    /// A name too wide is cut in the middle (the owner's ruling, "Tab cut
    /// in the middle to keep"), measured here a unit a character: whole
    /// where it fits, to the hundredth; its start and end kept, the start
    /// the longer, the extension whole while a character of the name fits
    /// beside it, else the whole name cut in the middle; a name without
    /// an extension (or with a dot that does not start one) cut in the
    /// middle all the same; a name of one character never cut; the
    /// ellipsis alone where nothing more fits.
    #[test]
    fn a_name_too_wide_is_cut_in_the_middle_keeping_its_extension() {
        let units = |s: &str| s.chars().count() as f32;
        let cut = |title: &str, max: f32| middle_cut(title, max, &units);
        // Short, and an exact fit.
        assert_eq!(cut("ab.nss", 10.0), "ab.nss");
        assert_eq!(cut("abcdef.nss", 10.0), "abcdef.nss");
        assert_eq!(cut("abcdef.nss", 9.99), "ab…ef.nss");
        // Long: Chapter1's scripts, as the bar of the ruling's picture.
        assert_eq!(cut("at_m1q4c_m1q_001.nss", 9.0), "at…01.nss");
        assert_eq!(cut("at_m1q4c_m1q_001.nss", 10.0), "at_…01.nss");
        assert_eq!(cut("at_m1q4c_m1q_001.nss", 19.0), "at_m1q4…m1q_001.nss");
        // The extension kept with one character of the name, then dropped.
        assert_eq!(cut("abcdefgh.nss", 6.0), "a….nss");
        assert_eq!(cut("abcdefgh.nss", 5.0), "ab…ss");
        // No extension, or none that is one.
        assert_eq!(cut("module_properties_x", 7.0), "mod…s_x");
        assert_eq!(cut("nw_c2_default9 (preview)", 9.0), "nw_c…iew)");
        assert_eq!(cut("1.5 notes here", 7.0), "1.5…ere");
        assert_eq!(cut(".hiddenname", 5.0), ".h…me");
        assert_eq!(cut("name.toolongext", 7.0), "nam…ext");
        // One character, and nothing more fitting than the ellipsis.
        assert_eq!(cut("W", 0.5), "W");
        assert_eq!(cut("abc", 2.0), "…");
        assert_eq!(cut("ab.c", 2.5), "…");
    }

    /// The cut as egui lays it out, for names of letters, wide letters and
    /// emoji in the button font: never wider than asked (but the ellipsis
    /// alone), its start the name's, the extension kept whole wherever a
    /// character of the name fits beside it.
    #[test]
    fn a_cut_name_fits_its_room_in_egui_s_fonts() {
        let ctx = egui::Context::default();
        crate::apply(&ctx, crate::Theme::Dark);
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
            let font = egui::TextStyle::Button.resolve(ui.style());
            let measure = |s: &str| {
                let g = ui.fonts_mut(|f| {
                    f.layout_no_wrap(s.to_owned(), font.clone(), egui::Color32::WHITE)
                });
                g.rect.width()
            };
            for title in [
                "at_m1q4c_m1q_001.nss",
                "WWWWWWWWWWWW.utc",
                "🌲🌲🌲🌲🌲🌲🌲🌲🌲.nss",
                "ÆØÅ_wide_ᚠᚢᚦ_name",
            ] {
                let whole = measure(title);
                let name = title.split('.').next().unwrap();
                let ext = title.strip_prefix(name).unwrap();
                let first = title.chars().next().unwrap();
                let mut kept = 0;
                for step in 1..=40 {
                    let max = whole * step as f32 / 40.0;
                    let shown = middle_cut(title, max, &measure);
                    if shown == "…" {
                        continue;
                    }
                    assert!(measure(&shown) <= max, "{title} in {max}: {shown}");
                    if shown == title {
                        assert!(whole <= max, "{title} whole in {max}");
                        continue;
                    }
                    assert!(shown.starts_with(first) && shown.contains('…'), "{title}: {shown}");
                    let one = format!("{first}…{ext}");
                    if !ext.is_empty() && measure(&one) <= max {
                        assert!(shown.ends_with(ext), "{title} in {max}: {shown}");
                    }
                    kept = kept.max(shown.chars().count());
                }
                assert!(kept > 3, "{title}: cut at some width");
            }
        });
        out.textures_delta.clear();
    }

    /// The titles of a bar are cut alike to fit it, from the widest down,
    /// and never under their least.
    #[test]
    fn a_crowded_bar_s_titles_are_cut_alike() {
        use crate::shell::doc_tab_width;
        let widths = [(100.0, false), (200.0, true), (50.0, false)];
        let whole: f32 = widths.iter().map(|&(w, m)| doc_tab_width(w, m)).sum();
        assert_eq!(doc_tab_cap(whole, &widths), None, "fits whole");
        let cap = doc_tab_cap(whole - 60.0, &widths).unwrap();
        let total: f32 = widths.iter().map(|&(w, m)| doc_tab_width(w.min(cap), m)).sum();
        assert!(total <= whole - 60.0 && total > whole - 62.0, "{cap} {total}");
        assert!(cap > 100.0, "the widest cut first: {cap}");
        assert_eq!(doc_tab_cap(10.0, &widths), Some(DOC_TAB_LEAST_TITLE), "never under the least");
    }

    /// A document tab's parts are painted in it: its icon and its dot
    /// always, its close button where shown; nothing of a tab out of
    /// sight.
    #[test]
    fn a_document_tab_s_parts_are_painted_in_it() {
        let ctx = egui::Context::default();
        crate::apply(&ctx, crate::Theme::Dark);
        crate::record_icons(&ctx);
        let dock = DockState::new(vec!["a", "b"]);
        let layer = egui::LayerId::background();
        let rect = |l: f32| egui::Rect::from_min_size(egui::pos2(l, 0.0), egui::vec2(140.0, 28.0));
        let mark = |tab, at: f32, shown: egui::Rect, close| DocTabMark {
            tab,
            rect: rect(at),
            layer,
            shown,
            icon: crate::Icon::Area,
            title_width: 60.0,
            modified: true,
            active: tab == "a",
            close,
        };
        let tabs = [
            mark("a", 0.0, rect(0.0), Some(crate::icons::ButtonState::Rest)),
            mark("b", 140.0, egui::Rect::NOTHING, None),
        ];
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
            doc_tab_marks(ui.ctx(), &dock, None, &tabs);
        });
        out.textures_delta.clear();
        let drawn = crate::icons_drawn(&ctx);
        let parts = crate::shell::doc_tab_parts(rect(0.0), 60.0, true);
        assert!(drawn.contains(&(crate::Icon::Area, parts.icon)), "{drawn:?}");
        assert!(
            drawn.iter().any(|(i, r)| *i == crate::Icon::Close && parts.close.contains_rect(*r))
        );
        assert_eq!(drawn.len(), 2, "nothing of the tab out of sight: {drawn:?}");
    }

    /// The front tab's line is the accent over the pane given the focus,
    /// where egui_dock has none (the window just opened), `line-soft`
    /// without one.
    #[test]
    fn the_accent_line_follows_the_focus_given() {
        let ctx = egui::Context::default();
        crate::apply(&ctx, crate::Theme::Dark);
        let dock = DockState::new(vec!["a"]);
        let path = dock.iter_leaves().next().unwrap().0;
        let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(140.0, 28.0));
        let tabs = [DocTabMark {
            tab: "a",
            rect,
            layer: egui::LayerId::background(),
            shown: rect,
            icon: crate::Icon::Area,
            title_width: 60.0,
            modified: false,
            active: true,
            close: None,
        }];
        for (focused, want) in [(Some(path), Tokens::DARK.accent), (None, Tokens::DARK.line_soft)] {
            let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
                doc_tab_marks(ui.ctx(), &dock, focused, &tabs);
            });
            out.textures_delta.clear();
            let filled = out.shapes.iter().any(|c| match &c.shape {
                egui::Shape::Rect(r) => {
                    r.fill == want && r.rect.height() == metrics::ACTIVE_TAB_LINE
                }
                _ => false,
            });
            assert!(filled, "{focused:?}");
        }
    }

    /// The marks of a tab scrolled partly or wholly out of its bar are
    /// drawn only in the part in sight, and none for a tab out of sight
    /// (they were drawn over what lies beside the bar).
    #[test]
    fn marks_stay_in_the_part_of_a_tab_in_sight() {
        let ctx = egui::Context::default();
        crate::apply(&ctx, crate::Theme::Dark);
        let dock = DockState::new(vec!["Welcome", "Manual", "Options"]);
        let layer = egui::LayerId::background();
        let rect = |l: f32| egui::Rect::from_min_size(egui::pos2(l, 0.0), egui::vec2(100.0, 28.0));
        let bar = egui::Rect::from_min_max(egui::pos2(150.0, 0.0), egui::pos2(400.0, 28.0));
        let tabs = [
            // In front, half out of sight at the bar's left; out of sight;
            // whole.
            ("Welcome", rect(100.0), layer, rect(100.0).intersect(bar)),
            ("Manual", rect(-50.0), layer, rect(-50.0).intersect(bar)),
            ("Options", rect(200.0), layer, rect(200.0)),
        ];
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| marks(ui.ctx(), &dock, &tabs));
        out.textures_delta.clear();
        let clips: Vec<egui::Rect> = out.shapes.iter().map(|c| c.clip_rect).collect();
        // Two lines between tabs and the front tab's line, nothing for the
        // tab out of sight.
        assert_eq!(clips.len(), 3, "{clips:?}");
        for clip in &clips {
            assert!(bar.contains_rect(*clip), "{clip:?} outside the bar {bar:?}");
        }
    }
}
