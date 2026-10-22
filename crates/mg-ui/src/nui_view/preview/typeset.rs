//! Text as the client draws it: whole-pixel advances and Nuklear's clamp.
use super::*;
use std::sync::Arc;

/// How a text is drawn: its font (the skin's name for it), the canvas scale,
/// its color, where it sits in its rectangle and whether it wraps there.
#[derive(Clone, Copy)]
pub(in crate::nui_view) struct TextStyle<'a> {
    pub font: &'a str,
    pub scale: f32,
    pub color: Color32,
    pub align: Align2,
    pub wrap: bool,
}

pub(in crate::nui_view) fn text(
    p: &Painter,
    assets: &skin::Assets,
    rect: Rect,
    label: &str,
    style: TextStyle<'_>,
) {
    draw_text(p, assets, rect, label, style, true);
}

/// `clamp`: drawn by nk_draw_text, which cuts glyphs; an edit field only clips.
pub(super) fn draw_text(
    p: &Painter,
    assets: &skin::Assets,
    rect: Rect,
    label: &str,
    style: TextStyle<'_>,
    clamp: bool,
) {
    let TextStyle { font, scale, color, align, wrap } = style;
    if !rect.is_positive() || label.is_empty() {
        return;
    }
    let ctx = p.ctx();
    let (font_id, spacing) = assets.font(ctx, font, scale);
    let face = Face { id: &font_id, spacing, color };
    let width = if wrap { rect.width() } else { f32::INFINITY };
    let mut galley =
        assets.galley(ctx, &face.key(label, width, LINE), || line(ctx, label, face, width));
    // (Half a point either way is the rounding of a fit, not a cut.)
    if assets.recording()
        && (galley.size().x > rect.width() + 0.5 || galley.size().y > rect.height() + 0.5)
    {
        assets.record_cut(label);
    }
    // nk_draw_text cuts a left-aligned line after the glyph that reaches its
    // end (nk_text_clamp; NWN EE 8193.37, np_misc: a 200-wide combo shows
    // "A very long dropdow"); centred and right-aligned text is clipped instead.
    if clamp
        && !wrap
        && align.x() == egui::Align::Min
        && !label.contains('\n')
        && galley.size().x > rect.width()
    {
        galley = assets.galley(ctx, &face.key(label, rect.width(), CUT), || {
            line(ctx, cut(ctx, label, face, rect.width()), face, f32::INFINITY)
        });
    }
    // NWN clips overflowing single-line text at the right edge; centering an
    // oversized galley would hide the start of a label instead.
    let mut position = align.align_size_within_rect(galley.size(), rect).min;
    position.x = position.x.max(rect.left());
    p.with_clip_rect(rect.intersect(p.clip_rect())).galley(position, galley, color);
}

/// A font as the preview draws with it: egui's, the skin's `spacing_h` at
/// the canvas scale, and the color.
#[derive(Clone, Copy)]
pub(super) struct Face<'a> {
    pub id: &'a egui::FontId,
    pub spacing: f32,
    pub color: Color32,
}

/// The layouts `skin::Assets` keeps galleys of: a line (wrapped by egui at a
/// width or not at all), a line cut at a width, and NuiText's own wrapping.
const LINE: u8 = 0;
const CUT: u8 = 1;
const BLOCK: u8 = 2;

impl<'a> Face<'a> {
    fn key(self, text: &'a str, width: f32, shape: u8) -> skin::TextKey<'a> {
        skin::TextKey {
            text,
            font: self.id,
            spacing: self.spacing,
            color: self.color,
            line_height: None,
            width,
            shape,
        }
    }
}

/// A line laid out afresh; `width` is where egui wraps it (infinite: nowhere).
fn line(ctx: &egui::Context, label: &str, face: Face<'_>, width: f32) -> Arc<egui::Galley> {
    let mut job = native_job(ctx, label, face.id, face.spacing, face.color, None);
    job.wrap.max_width = width;
    ctx.fonts_mut(|f| f.layout_job(job))
}

/// What nk_text_clamp leaves of a line `width` wide: up to and with the
/// glyph that reaches its end.
fn cut<'a>(ctx: &egui::Context, label: &'a str, face: Face<'_>, width: f32) -> &'a str {
    let mut used = 0.0;
    let fit = label
        .char_indices()
        .zip(native_advances(ctx, label, face.id, face.spacing))
        .find(|(_, advance)| {
            let past = used >= width;
            used += advance;
            past
        })
        .map_or(label.len(), |((i, _), _)| i);
    &label[..fit]
}

/// One glyph's advance as the client draws it: the font's own plus the skin's
/// `spacing_h`, snapped to a whole pixel (`pixel_snap`). Where NWN EE 8193.37
/// wraps NuiText, and how long its lines are, follow this and not the
/// unsnapped advance.
pub(super) fn native_advances(
    ctx: &egui::Context,
    text: &str,
    font_id: &egui::FontId,
    spacing: f32,
) -> Vec<f32> {
    ctx.fonts_mut(|f| {
        text.chars()
            .map(|c| if c == '\n' { 0.0 } else { (f.glyph_width(font_id, c) + spacing).round() })
            .collect()
    })
}

/// Longer texts are laid out with the plain spacing: a section per glyph is
/// what the snapped advances cost.
pub(super) const SNAPPED_TEXT: usize = 8192;

/// A job whose glyphs advance by `native_advances`: egui's own advance, and
/// before each next glyph the space that makes up the difference.
pub(in crate::nui_view) fn native_job(
    ctx: &egui::Context,
    text: &str,
    font_id: &egui::FontId,
    spacing: f32,
    color: Color32,
    line_height: Option<f32>,
) -> egui::text::LayoutJob {
    let mut format =
        egui::TextFormat { font_id: font_id.clone(), color, line_height, ..Default::default() };
    if text.len() > SNAPPED_TEXT {
        format.extra_letter_spacing = spacing;
        return egui::text::LayoutJob::single_section(text.to_owned(), format);
    }
    let mut job = egui::text::LayoutJob::default();
    let mut lead = 0.0;
    ctx.fonts_mut(|f| {
        for c in text.chars() {
            job.append(c.encode_utf8(&mut [0; 4]), lead, format.clone());
            lead = if c == '\n' {
                0.0
            } else {
                let w = f.glyph_width(font_id, c);
                (w + spacing).round() - w
            };
        }
    });
    job
}

/// Text laid out as the client's NuiText lays it out: lines break at spaces
/// only, and a word wider than the whole line is cut where it stops fitting
/// (egui would also break after hyphens and underscores). Each line is `pitch`
/// high.
pub(super) fn native_text_layout(
    ctx: &egui::Context,
    assets: &skin::Assets,
    text: &str,
    face: Face<'_>,
    pitch: f32,
    width: f32,
) -> Arc<egui::Galley> {
    let key = skin::TextKey { line_height: Some(pitch), ..face.key(text, width, BLOCK) };
    assets.galley(ctx, &key, || block(ctx, text, face, pitch, width))
}

/// NuiText laid out afresh.
fn block(
    ctx: &egui::Context,
    text: &str,
    face: Face<'_>,
    pitch: f32,
    width: f32,
) -> Arc<egui::Galley> {
    let mut lines = String::with_capacity(text.len() + 16);
    for (n, paragraph) in text.split('\n').enumerate() {
        if n > 0 {
            lines.push('\n');
        }
        let advances = native_advances(ctx, paragraph, face.id, face.spacing);
        lines.push_str(&native_wrap(paragraph, &advances, width));
    }
    let job = native_job(ctx, &lines, face.id, face.spacing, face.color, Some(pitch));
    ctx.fonts_mut(|f| f.layout_job(job))
}

/// One paragraph with a line break wherever the client breaks it: Nuklear's
/// `nk_text_clamp` with a space as the only separator. A glyph is taken while
/// the line is still short of `width`, so a word stays on the line only if it
/// ends inside it, while a word cut for having no space before it may run one
/// glyph past the edge.
pub(super) fn native_wrap(paragraph: &str, advances: &[f32], width: f32) -> String {
    let chars: Vec<char> = paragraph.chars().collect();
    if width <= 0.0 {
        return paragraph.to_owned();
    }
    let mut out = String::with_capacity(paragraph.len() + 8);
    let mut start = 0;
    while start < chars.len() {
        let mut end = start;
        let mut used = 0.0;
        let mut after_space = None;
        while used < width && end < chars.len() {
            used += advances.get(end).copied().unwrap_or(0.0);
            if chars[end] == ' ' {
                after_space = Some(end + 1);
            }
            end += 1;
        }
        let cut = if end == chars.len() { end } else { after_space.unwrap_or(end) };
        if start > 0 {
            out.push('\n');
        }
        out.extend(&chars[start..cut]);
        start = cut;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Nuklear's clamp, as NWN wraps NuiText: spaces only; a word stays only if
    /// it ends inside the line; a word with no space before it is cut a glyph
    /// past the edge; the text's last glyph may overflow too.
    #[test]
    pub(super) fn native_wrap_follows_nuklear_text_clamp() {
        let wrap =
            |text: &str, width: f32| native_wrap(text, &vec![10.0; text.chars().count()], width);
        assert_eq!(wrap("aaa bbbb", 65.0), "aaa \nbbbb");
        assert_eq!(wrap("aaa bbb", 65.0), "aaa bbb");
        assert_eq!(wrap("abcdefghij", 35.0), "abcd\nefgh\nij");
        assert_eq!(wrap("a-b-c-d e_f", 45.0), "a-b-c\n-d \ne_f");
        assert_eq!(wrap("Żółć żółć", 45.0), "Żółć \nżółć");
    }

    /// What a galley is, as far as drawing goes: its size, and each row's
    /// place and glyphs.
    fn drawn(galley: &egui::Galley) -> impl PartialEq + std::fmt::Debug {
        let rows: Vec<_> = galley
            .rows
            .iter()
            .map(|r| {
                let glyphs: Vec<_> = r.glyphs.iter().map(|g| (g.chr, g.pos, g.uv_rect)).collect();
                (r.pos, r.size, glyphs)
            })
            .collect();
        (galley.size(), rows)
    }

    #[test]
    fn kept_text_is_what_laying_it_out_anew_gives() {
        let ctx = egui::Context::default();
        let assets = skin::Assets::default();
        let font = egui::FontId::proportional(17.3);
        let sample = "Żółć gęślą jaźń, two  spaces\nand a second line that is long enough to wrap";
        let pass = |ctx: &egui::Context, kept: &mut Vec<Arc<egui::Galley>>| {
            let mut n = 0;
            for color in [Color32::WHITE, Color32::RED] {
                let face = Face { id: &font, spacing: 0.6, color };
                for width in [90.0, 140.0, f32::INFINITY] {
                    let fresh = [
                        line(ctx, sample, face, width),
                        line(ctx, cut(ctx, sample, face, width), face, f32::INFINITY),
                        block(ctx, sample, face, 21.0, width),
                        block(ctx, sample, face, 25.0, width),
                    ];
                    let cached = [
                        assets.galley(ctx, &face.key(sample, width, LINE), || fresh[0].clone()),
                        assets.galley(ctx, &face.key(sample, width, CUT), || fresh[1].clone()),
                        native_text_layout(ctx, &assets, sample, face, 21.0, width),
                        native_text_layout(ctx, &assets, sample, face, 25.0, width),
                    ];
                    for (fresh, cached) in fresh.iter().zip(&cached) {
                        assert_eq!(drawn(cached), drawn(fresh), "{color:?} {width}");
                        // Asked for again, it is the one kept.
                        if let Some(kept) = kept.get(n) {
                            assert!(Arc::ptr_eq(kept, cached), "{color:?} {width}");
                        } else {
                            kept.push(cached.clone());
                        }
                        n += 1;
                    }
                }
            }
            // The sample does wrap, and its lines differ with the width.
            let face = Face { id: &font, spacing: 0.6, color: Color32::WHITE };
            let narrow = native_text_layout(ctx, &assets, sample, face, 21.0, 90.0);
            let wide = native_text_layout(ctx, &assets, sample, face, 21.0, 140.0);
            assert!(narrow.rows.len() > wide.rows.len() && wide.rows.len() > 2);
        };
        // (This test looks at galleys; no backend takes the textures.)
        let run = |kept: &mut Vec<Arc<egui::Galley>>, draw: bool| {
            let mut out = ctx.run_ui(Default::default(), |ui| {
                if draw {
                    pass(ui.ctx(), kept);
                }
            });
            out.textures_delta.clear();
        };
        let mut kept = Vec::new();
        for _ in 0..3 {
            run(&mut kept, true);
        }
        // egui builds its font atlas again when the pixels per point change:
        // nothing made for the old one is handed out.
        let old = std::mem::take(&mut kept);
        ctx.set_pixels_per_point(2.0);
        run(&mut kept, true);
        assert!(old.iter().zip(&kept).all(|(old, new)| !Arc::ptr_eq(old, new)));
        // And so after a pass that drew no text of the preview.
        let old = std::mem::take(&mut kept);
        run(&mut kept, false);
        run(&mut kept, true);
        assert!(old.iter().zip(&kept).all(|(old, new)| !Arc::ptr_eq(old, new)));
    }
}
