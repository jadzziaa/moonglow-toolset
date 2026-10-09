//! Game artwork and fonts on an editor canvas. Layout is local; NWScript is never run.
use super::*;
use egui::{Align2, Color32, Painter, Rect, Sense, Stroke, StrokeKind, Vec2, pos2, vec2};
use std::collections::BTreeSet;

const GOLD: Color32 = Color32::from_rgb(173, 142, 96);

// NWN's TextEdit max is a UTF-8 byte budget, not egui's character limit.
// Apply it at insertion, preserving the suffix and cursor on rejected input.
struct NativeTextBuffer {
    text: String,
    max_bytes: usize,
}

impl egui::TextBuffer for NativeTextBuffer {
    fn type_id(&self) -> std::any::TypeId {
        std::any::TypeId::of::<Self>()
    }
    fn is_mutable(&self) -> bool {
        true
    }
    fn as_str(&self) -> &str {
        &self.text
    }
    fn insert_text(&mut self, text: &str, char_index: egui::text::CharIndex) -> usize {
        let mut end = text.len().min(self.max_bytes.saturating_sub(self.text.len()));
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        egui::TextBuffer::insert_text(&mut self.text, &text[..end], char_index)
    }
    fn delete_char_range(&mut self, range: std::ops::Range<egui::text::CharIndex>) {
        egui::TextBuffer::delete_char_range(&mut self.text, range);
    }
}

pub(super) fn val<'a>(v: &'a Value, s: &'a Settings, row: Option<usize>) -> &'a Value {
    let out = resolved(v, s);
    if v.get("bind").is_some()
        && let (Some(row), Some(a)) = (row, out.as_array())
    {
        return a.get(row).unwrap_or(&Value::Null);
    }
    out
}
pub(super) fn num(v: &Value, s: &Settings, row: Option<usize>, default: f32) -> f32 {
    val(v, s, row).as_f64().filter(|n| n.is_finite()).map_or(default, |n| n as f32)
}
pub(super) fn flag(v: &Value, s: &Settings, row: Option<usize>, default: bool) -> bool {
    val(v, s, row).as_bool().unwrap_or(default)
}
pub(super) fn string(v: &Value, s: &Settings, row: Option<usize>) -> String {
    format_string(v, s, row, None)
}
pub(super) fn localized_string(
    v: &Value,
    s: &Settings,
    row: Option<usize>,
    assets: &skin::Assets,
) -> String {
    format_string(v, s, row, Some(assets))
}
fn format_string(
    v: &Value,
    s: &Settings,
    row: Option<usize>,
    assets: Option<&skin::Assets>,
) -> String {
    let resolved = val(v, s, row);
    let mut text = match resolved {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        v if v.get("strref").is_some_and(Value::is_i64) => {
            let id = v["strref"].as_i64().unwrap();
            assets.map_or_else(|| format!("[Missing StrRef {id}]"), |a| a.string(id))
        }
        v => v.to_string(),
    };
    if v["bind"].is_string() {
        if let Some(n) = resolved.as_i64() {
            if v["number_flags"].as_i64().unwrap_or(0) & 1 != 0 {
                text = format!("{n:x}");
            }
        } else if let Some(n) = resolved.as_f64() {
            let precision = v["number_precision"].as_u64().unwrap_or(0).min(16) as usize;
            text = native_decimal(n, precision);
        }
        let flags = v["text_flags"].as_i64().unwrap_or(0);
        // Native bind text_flags operate on ASCII case, preserving UTF-8
        // characters (Creator-authored nui_format_s: Żółć stays unchanged).
        if flags & 1 != 0 {
            text.make_ascii_lowercase();
        }
        if flags & 2 != 0 {
            text.make_ascii_uppercase();
        }
    }
    text
}

fn native_decimal(mut n: f64, precision: usize) -> String {
    // NWN rounds exact decimal half ties away from zero. Rust uses ties-to-even.
    // Detect the tie in the exact binary value, not with rounded n * 10^p:
    // that multiplication can incorrectly turn a neighbouring value into a tie.
    let bits = n.abs().to_bits();
    let exponent = ((bits >> 52) & 0x7ff) as i32;
    if n.is_finite() && exponent != 0 {
        let mantissa = (bits & ((1_u64 << 52) - 1)) | (1_u64 << 52);
        let numerator = u128::from(mantissa) * 5_u128.pow(precision as u32);
        let denominator_bits = 1075 - exponent - precision as i32;
        if (1..128).contains(&denominator_bits) {
            let denominator = 1_u128 << denominator_bits;
            if numerator % denominator == denominator / 2 {
                n = if n.is_sign_negative() { n.next_down() } else { n.next_up() };
            }
        }
    }
    format!("{n:.precision$}")
}

/// Nuklear's clamp, as NWN wraps NuiText: spaces only; a word stays only if
/// it ends inside the line; a word with no space before it is cut a glyph
/// past the edge; the text's last glyph may overflow too.
#[test]
fn native_wrap_follows_nuklear_text_clamp() {
    let wrap = |text: &str, width: f32| native_wrap(text, &vec![10.0; text.chars().count()], width);
    assert_eq!(wrap("aaa bbbb", 65.0), "aaa \nbbbb");
    assert_eq!(wrap("aaa bbb", 65.0), "aaa bbb");
    assert_eq!(wrap("abcdefghij", 35.0), "abcd\nefgh\nij");
    assert_eq!(wrap("a-b-c-d e_f", 45.0), "a-b-c\n-d \ne_f");
    assert_eq!(wrap("Żółć żółć", 45.0), "Żółć \nżółć");
}

#[test]
fn native_decimal_changes_exact_ties_without_double_rounding() {
    for sign in [1.0, -1.0] {
        let tie = 42.125_f64 * sign;
        for neighbour in [tie.next_down(), tie.next_up()] {
            assert_eq!(native_decimal(neighbour, 2), format!("{neighbour:.2}"));
        }
    }
    for value in [0.0_f64, -0.0, f64::MIN_POSITIVE, f64::MAX, 1.005, -1.005] {
        for precision in [0, 2, 4, 16] {
            assert_eq!(native_decimal(value, precision), format!("{value:.precision$}"));
        }
    }
}

#[test]
fn native_text_flags_change_ascii_case_and_preserve_unicode() {
    let mut s = Settings::default();
    s.bindings.insert(
        "value".into(),
        Binding { value: json!("MiXeD Ala Żółć 123"), ..Default::default() },
    );
    for (flags, expected) in [
        (0, "MiXeD Ala Żółć 123"),
        (1, "mixed ala Żółć 123"),
        (2, "MIXED ALA Żółć 123"),
        (3, "MIXED ALA Żółć 123"),
    ] {
        assert_eq!(string(&json!({"bind":"value","text_flags":flags}), &s, None), expected);
    }
}
pub(super) fn rgba(v: &Value, default: Color32) -> Color32 {
    if !v.is_object() {
        return default;
    }
    let [r, g, b, a] = rgba_channels(v);
    Color32::from_rgba_unmultiplied(r, g, b, a)
}
fn rgba_channels(v: &Value) -> [u8; 4] {
    ["r", "g", "b", "a"].map(|k| v[k].as_f64().map_or(255, |x| x.clamp(0.0, 255.0) as u8))
}
fn alignment(h: i64, v: i64) -> Align2 {
    use egui::Align;
    let a = |v| match v {
        1 => Align::Min,
        2 => Align::Max,
        _ => Align::Center,
    };
    Align2([a(h), a(v)])
}

#[allow(clippy::too_many_arguments)]
pub(super) fn text(
    p: &Painter,
    assets: &skin::Assets,
    rect: Rect,
    label: &str,
    font: &str,
    scale: f32,
    color: Color32,
    align: Align2,
    wrap: bool,
) {
    draw_text(p, assets, rect, label, font, scale, color, align, wrap, true);
}

/// `clamp`: drawn by nk_draw_text, which cuts glyphs; an edit field only clips.
#[allow(clippy::too_many_arguments)]
fn draw_text(
    p: &Painter,
    assets: &skin::Assets,
    rect: Rect,
    label: &str,
    font: &str,
    scale: f32,
    color: Color32,
    align: Align2,
    wrap: bool,
    clamp: bool,
) {
    if !rect.is_positive() || label.is_empty() {
        return;
    }
    let (font_id, spacing) = assets.font(p.ctx(), font, scale);
    let layout = |label: &str| {
        let mut job = native_job(p.ctx(), label, &font_id, spacing, color, None);
        job.wrap.max_width = if wrap { rect.width() } else { f32::INFINITY };
        p.layout_job(job)
    };
    let mut galley = layout(label);
    // nk_draw_text cuts a left-aligned line after the glyph that reaches its
    // end (nk_text_clamp; NWN EE 8193.37, np_misc: a 200-wide combo shows
    // "A very long dropdow"); centred and right-aligned text is clipped instead.
    if clamp
        && !wrap
        && align.x() == egui::Align::Min
        && !label.contains('\n')
        && galley.size().x > rect.width()
    {
        let mut width = 0.0;
        let fit = label
            .char_indices()
            .zip(native_advances(p.ctx(), label, &font_id, spacing))
            .find(|(_, advance)| {
                let past = width >= rect.width();
                width += advance;
                past
            })
            .map_or(label.len(), |((i, _), _)| i);
        galley = layout(&label[..fit]);
    }
    // NWN clips overflowing single-line text at the right edge; centering an
    // oversized galley would hide the start of a label instead.
    let mut position = align.align_size_within_rect(galley.size(), rect).min;
    position.x = position.x.max(rect.left());
    p.with_clip_rect(rect.intersect(p.clip_rect())).galley(position, galley, color);
}

/// One glyph's advance as the client draws it: the font's own plus the skin's
/// `spacing_h`, snapped to a whole pixel (`pixel_snap`). Where NWN EE 8193.37
/// wraps NuiText, and how long its lines are, follow this and not the
/// unsnapped advance.
fn native_advances(
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
const SNAPPED_TEXT: usize = 8192;

/// A job whose glyphs advance by `native_advances`: egui's own advance, and
/// before each next glyph the space that makes up the difference.
pub(super) fn native_job(
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
fn native_text_layout(
    p: &Painter,
    text: &str,
    font_id: &egui::FontId,
    spacing: f32,
    pitch: f32,
    color: Color32,
    width: f32,
) -> std::sync::Arc<egui::Galley> {
    let mut lines = String::with_capacity(text.len() + 16);
    for (n, paragraph) in text.split('\n').enumerate() {
        if n > 0 {
            lines.push('\n');
        }
        let advances = native_advances(p.ctx(), paragraph, font_id, spacing);
        lines.push_str(&native_wrap(paragraph, &advances, width));
    }
    p.layout_job(native_job(p.ctx(), &lines, font_id, spacing, color, Some(pitch)))
}

/// One paragraph with a line break wherever the client breaks it: Nuklear's
/// `nk_text_clamp` with a space as the only separator. A glyph is taken while
/// the line is still short of `width`, so a word stays on the line only if it
/// ends inside it, while a word cut for having no space before it may run one
/// glyph past the edge.
fn native_wrap(paragraph: &str, advances: &[f32], width: f32) -> String {
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
            used += advances[end];
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

/// Aspect constants are the stock nw_inc_nui constants, not egui's image modes.
fn image_rect(rect: Rect, size: Vec2, mode: i64, align: Align2, scale: f32) -> Rect {
    let fit = (rect.width() / size.x).min(rect.height() / size.y);
    let size = match mode {
        1 => size * (rect.width() / size.x).max(rect.height() / size.y),
        2 => size * fit.min(scale),
        3 => size,
        4 => size * scale,
        5 => rect.size(),
        _ => size * fit,
    };
    align.align_size_within_rect(size, rect)
}

#[allow(clippy::too_many_arguments)]
pub(super) fn draw_image(
    p: &Painter,
    assets: &skin::Assets,
    name: &str,
    rect: Rect,
    node: &Value,
    s: &Settings,
    row: Option<usize>,
    scale: f32,
    tint: Color32,
) -> bool {
    let Some(image) = assets.picture(name) else {
        return false;
    };
    let region = val(&node["image_region"], s, row);
    let (source, uv) = if region.is_object() {
        let x = num(&region["x"], s, row, 0.0).clamp(0.0, image.size.x);
        let y = num(&region["y"], s, row, 0.0).clamp(0.0, image.size.y);
        let w = num(&region["w"], s, row, image.size.x).clamp(0.0, image.size.x - x);
        let h = num(&region["h"], s, row, image.size.y).clamp(0.0, image.size.y - y);
        (
            vec2(w, h),
            Rect::from_min_size(
                pos2(x / image.size.x, y / image.size.y),
                vec2(w / image.size.x, h / image.size.y),
            ),
        )
    } else {
        (image.size, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)))
    };
    if source.x <= 0.0 || source.y <= 0.0 {
        return true;
    }
    let a = alignment(
        val(&node["image_halign"], s, row).as_i64().unwrap_or(0),
        val(&node["image_valign"], s, row).as_i64().unwrap_or(0),
    );
    let target = image_rect(
        rect,
        // NWN applies the crop only to UVs. Native nui_crop_s keeps the
        // original texture's aspect for both half-width regions.
        image.size,
        if node["type"] == "button_image" {
            5
        } else {
            val(&node["image_aspect"], s, row).as_i64().unwrap_or(0)
        },
        a,
        scale,
    );
    p.with_clip_rect(rect.intersect(p.clip_rect())).image(image.texture.id(), target, uv, tint);
    true
}

fn natural(node: &Value, s: &Settings, row: Option<usize>, depth: usize) -> Vec2 {
    if depth > 32 {
        return vec2(120.0, 30.0);
    }
    let ty = node["type"].as_str().unwrap_or("");
    // Confirmed by the Creator-generated Controls window in stock NWN EE.
    // An unspecified control width is not a request to fill its column.
    let mut size = vec2(150.0, 30.0);
    if ty == "group" {
        // NuiGroup deliberately does not advise its parent about its child's
        // size (stock nw_inc_nui). It occupies the available span instead.
        size = Vec2::ZERO;
    } else if let Some(children) = node["children"].as_array() {
        let sizes: Vec<_> = children.iter().map(|c| natural(c, s, row, depth + 1)).collect();
        let gap = children
            .iter()
            .take(children.len().saturating_sub(1))
            .map(|child| child_gap(ty, child))
            .sum::<f32>();
        size = if sizes.is_empty() {
            vec2(100.0, 36.0)
        } else if ty == "row" {
            vec2(
                sizes.iter().map(|s| s.x).sum::<f32>() + gap,
                sizes.iter().map(|s| s.y).fold(0.0, f32::max),
            )
        } else {
            vec2(
                sizes.iter().map(|s| s.x).fold(0.0, f32::max),
                sizes.iter().map(|s| s.y).sum::<f32>() + gap,
            )
        };
    } else if ty == "list" {
        // A list with no width is as wide as its column (NWN EE 8193.37: 304
        // beside two 150 tabs, 150 under a lone button); it doesn't widen it.
        size = vec2(150.0, 100.0);
    } else if ty == "chart" {
        size = vec2(220.0, 100.0);
    } else if ty == "color_picker" {
        size = vec2(300.0, 100.0);
    } else if matches!(ty, "options" | "tabbar") {
        let count = val(&node["elements"], s, row).as_array().map_or(0, Vec::len);
        let gap = 4.0 * count.saturating_sub(1) as f32;
        // NWN Orientations: vertical Toggles and Options retain horizontal
        // natural width (two entries:304); wide Options hit tested natively.
        let width = count as f32 * 150.0 + gap;
        size = if val(&node["direction"], s, row).as_i64() == Some(1) {
            vec2(width, count as f32 * 30.0 + gap)
        } else {
            vec2(width, 30.0)
        };
    } else if matches!(ty, "label" | "image" | "spacer") {
        // These controls take their span from the containing layout; their
        // content must not inject the button's default 150-unit width.
        size.x = 0.0;
    } else if ty == "text" {
        size.y = 64.0;
    } else if matches!(ty, "button" | "button_select" | "button_image") {
        // NUI_STYLE_PRIMARY_HEIGHT: a button with no height is 50 high in the
        // client (less where its row has less room).
        size.y = 50.0;
    }
    let padding = layout_padding(node, s, row);
    let margin = extra_margin(node, s, row);
    size += Vec2::splat(2.0 * padding);
    let width = num(&node["width"], s, row, -1.0);
    let height = num(&node["height"], s, row, -1.0);
    if width >= 0.0 {
        size.x = width;
    }
    if height >= 0.0 {
        size.y = height;
    }
    let aspect = num(&node["aspect"], s, row, 0.0);
    if aspect > 0.0 && width >= 0.0 && height < 0.0 {
        size.y = size.x / aspect;
    }
    if aspect > 0.0 && height >= 0.0 && width < 0.0 {
        size.x = size.y * aspect;
    }
    (size + Vec2::splat(2.0 * margin)).clamp(Vec2::ZERO, vec2(4096.0, 4096.0))
}

/// How much farther out a control's own margin puts it than the client's
/// default margin of 2 (which the layout's padding and gaps already hold): a
/// margin m moves it m - 2, 0 and 1 included (NWN EE 8193.37: margins 0..20
/// measured against the same control without one).
fn extra_margin(node: &Value, s: &Settings, row: Option<usize>) -> f32 {
    if node.get("margin").is_none_or(Value::is_null) {
        return 0.0;
    }
    num(&node["margin"], s, row, 2.0).clamp(0.0, 256.0) - 2.0
}

fn layout_padding(node: &Value, s: &Settings, row: Option<usize>) -> f32 {
    // Stock row/column layout inset, observed in the Creator Widths demo.
    let default = if matches!(node["type"].as_str(), Some("row" | "col")) { 2.0 } else { 0.0 };
    num(&node["padding"], s, row, default).clamp(0.0, 256.0)
}

fn child_gap(parent_type: &str, child: &Value) -> f32 {
    // Native audit demo: an explicit vertical spacer advances the column by
    // its own height; it does not add a second widget-row gap after itself.
    if parent_type == "col"
        && matches!(child["type"].as_str(), Some("spacer" | "options" | "tabbar"))
    {
        0.0
    } else {
        4.0
    }
}

// Measured in GUI-authored Orientations on NWN EE89.8193.37-17:
// vertical height30/90/150 reserves layout space without stretching items;
// overflow remains painted/clickable. Horizontal height30/60 caps at35.
// These are stock control row metrics, separate from the outer allocation.
fn choice_rects(body: Rect, count: usize, vertical: bool, tabs: bool, scale: f32) -> Vec<Rect> {
    let gap = 4.0 * scale;
    let height = if vertical {
        35.0 * scale
    } else {
        (body.height() - 5.0 * scale).clamp(0.0, 35.0 * scale)
    };
    let step = if vertical { height + gap } else { (body.width() + gap) / count.max(1) as f32 };
    (0..count)
        .map(|i| {
            let offset = i as f32 * step;
            Rect::from_min_size(
                body.min + if vertical { vec2(0.0, offset) } else { vec2(offset, 0.0) },
                // Vertical tabs are as wide as a horizontal one (NWN EE: 150
                // each), whatever room the control has; vertical options are
                // hit across their whole width.
                vec2(
                    if vertical && tabs {
                        body.width().min(150.0 * scale)
                    } else if vertical {
                        body.width()
                    } else {
                        (step - gap).max(0.0)
                    },
                    height,
                ),
            )
        })
        .collect()
}

pub(super) fn window_size(window: &Value, s: &Settings) -> Vec2 {
    let geometry = resolved(&window["geometry"], s);
    let limits = resolved(&window["size_constraint"], s);
    let axis = |value: &Value, min: &str, max: &str, default| {
        let mut value = num(value, s, None, default).clamp(80.0, 4096.0);
        let lower = num(&limits[min], s, None, 0.0);
        let upper = num(&limits[max], s, None, 0.0);
        // NuiWindow: zero means unrestricted on that individual axis.
        if lower > 0.0 {
            value = value.max(lower);
        }
        if upper > 0.0 {
            value = value.min(upper);
        }
        value
    };
    vec2(axis(&geometry["w"], "x", "w", 420.0), axis(&geometry["h"], "y", "h", 240.0))
}

fn fills_width(node: &Value, s: &Settings, row: Option<usize>) -> bool {
    num(&node["width"], s, row, -1.0) < 0.0
        && !(num(&node["aspect"], s, row, 0.0) > 0.0 && num(&node["height"], s, row, -1.0) >= 0.0)
        && matches!(
            node["type"].as_str(),
            Some("label" | "image" | "spacer" | "col" | "row" | "group" | "list" | "color_picker")
        )
}

/// What takes the room a column has left, when it has no height of its own.
/// The client's root column fills its window: in NWN EE 8193.37 an empty Row
/// with no height pushes everything after it to the window's bottom, and a
/// Label, Text or picker with none is centred in a tall slot.
fn fills_height(node: &Value, s: &Settings, row: Option<usize>) -> bool {
    num(&node["height"], s, row, -1.0) < 0.0
        && !(num(&node["aspect"], s, row, 0.0) > 0.0 && num(&node["width"], s, row, -1.0) >= 0.0)
        && match node["type"].as_str() {
            Some(
                "group" | "spacer" | "label" | "text" | "image" | "list" | "chart" | "color_picker",
            ) => true,
            // A row or column is as high as its controls want, unless none of
            // them wants a height (an empty one too).
            Some("row" | "col") => node["children"]
                .as_array()
                .is_none_or(|c| c.iter().all(|c| fills_height(c, s, row))),
            _ => false,
        }
}

/// Native List demo: variable cells share the remaining span equally, while
/// fixed cells retain their pixel width. Cell fWidth is not a proportional weight.
fn list_widths(cells: &[Value], width: f32, scale: f32) -> Vec<f32> {
    let fixed: f32 = cells
        .iter()
        .filter(|c| !c[2].as_bool().unwrap_or(true))
        .map(|c| c[1].as_f64().unwrap_or(150.0).max(0.0) as f32 * scale)
        .sum();
    let count = cells.iter().filter(|c| c[2].as_bool().unwrap_or(true)).count();
    let gaps = 4.0 * scale * cells.len().saturating_sub(1) as f32;
    let share = ((width - gaps - fixed) / count.max(1) as f32).max(0.0);
    cells
        .iter()
        .map(|c| {
            if c[2].as_bool().unwrap_or(true) {
                share
            } else {
                c[1].as_f64().unwrap_or(150.0).max(0.0) as f32 * scale
            }
        })
        .collect()
}

fn gradient(p: &Painter, rect: Rect, colors: [Color32; 4]) {
    let mut mesh = egui::Mesh::default();
    for (pos, color) in [rect.left_top(), rect.right_top(), rect.right_bottom(), rect.left_bottom()]
        .into_iter()
        .zip(colors)
    {
        mesh.colored_vertex(pos, color);
    }
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    p.add(mesh);
}

#[allow(clippy::too_many_arguments)]
fn color_picker(
    ui: &mut Ui,
    p: &Painter,
    rect: Rect,
    color: [u8; 4],
    id: egui::Id,
    scale: f32,
    font_height: f32,
    interactive: bool,
    tint: Color32,
) -> Option<Value> {
    // Picker values are straight RGBA, not premultiplied paint colors.
    // Keep RGB even at alpha zero; a hue edit must not change brightness.
    let [r, g, b, alpha] = color;
    let mut hsv = egui::ecolor::Hsva::from_srgb([r, g, b]);
    hsv.a = f32::from(alpha) / 255.0;
    if hsv.s == 0.0 {
        hsv.h = ui.ctx().data_mut(|d| d.get_temp::<f32>(id)).unwrap_or(0.0);
    }
    // Stock NuiColorPicker has no internal spacing. Nuklear's RGB picker
    // still reserves the hidden alpha bar, and both bars use font->height.
    // See nk_do_color_picker in Nuklear 9f7750296f176e506c2b24ff55bc24495e2db750.
    let bar = font_height * scale;
    let square = Rect::from_min_max(rect.min, pos2(rect.right() - 2.0 * bar, rect.bottom()));
    let hue = Rect::from_min_max(square.right_top(), square.right_bottom() + vec2(bar, 0.0));
    if !square.is_positive() {
        return None;
    }
    let mut changed = false;
    for (name, area) in [("saturation-value", square), ("hue", hue)] {
        let response = ui.interact(
            area,
            id.with(name),
            if interactive { Sense::click_and_drag() } else { Sense::hover() },
        );
        response.widget_info(|| {
            egui::WidgetInfo::labeled(
                egui::WidgetType::Slider,
                interactive,
                format!("Color picker {name}"),
            )
        });
        if interactive
            && (response.clicked() || response.dragged())
            && let Some(pointer) = response.interact_pointer_pos()
        {
            if name == "hue" {
                hsv.h =
                    ((pointer.y - hue.top()) / (hue.height() - scale).max(scale)).clamp(0.0, 1.0);
            } else {
                hsv.s = ((pointer.x - square.left()) / (square.width() - scale).max(scale))
                    .clamp(0.0, 1.0);
                hsv.v = (1.0 - (pointer.y - square.top()) / (square.height() - scale).max(scale))
                    .clamp(0.0, 1.0);
            }
            changed = true;
        }
    }
    ui.ctx().data_mut(|d| d.insert_temp(id, hsv.h));
    let pure = Color32::from(egui::ecolor::Hsva::new(hsv.h, 1.0, 1.0, 1.0));
    // Compose two gradients. One quad with black lower corners interpolates
    // along its triangle diagonal and produces a visible grey seam instead
    // of the native saturation/value surface.
    gradient(p, square, [Color32::WHITE * tint, pure * tint, pure * tint, Color32::WHITE * tint]);
    gradient(
        p,
        square,
        [Color32::TRANSPARENT, Color32::TRANSPARENT, Color32::BLACK, Color32::BLACK],
    );
    for segment in 0..6 {
        let top = segment as f32 / 6.0;
        let bottom = (segment + 1) as f32 / 6.0;
        let a = Color32::from(egui::ecolor::Hsva::new(top, 1.0, 1.0, 1.0)) * tint;
        let b = Color32::from(egui::ecolor::Hsva::new(bottom, 1.0, 1.0, 1.0)) * tint;
        gradient(
            p,
            Rect::from_min_max(
                pos2(hue.left(), hue.top() + top * hue.height()),
                pos2(hue.right(), hue.top() + bottom * hue.height()),
            ),
            [a, a, b, b],
        );
    }
    let cursor = square.min + vec2(hsv.s * square.width(), (1.0 - hsv.v) * square.height());
    // The engine picker uses an open crosshair and a line on the hue bar.
    let stroke = Stroke::new(scale, Color32::WHITE * tint);
    for (from, to) in [(-7.0, -2.0), (8.0, 3.0)] {
        p.line_segment([cursor + vec2(from, 0.0) * scale, cursor + vec2(to, 0.0) * scale], stroke);
        p.line_segment([cursor + vec2(0.0, from) * scale, cursor + vec2(0.0, to) * scale], stroke);
    }
    let hue_y = hue.top() + hsv.h * square.height();
    p.line_segment(
        [pos2(hue.left() - scale, hue_y), pos2(hue.right() + 2.0 * scale, hue_y)],
        stroke,
    );
    if changed {
        // What the client's picker writes: opaque, whatever alpha it had.
        let [r, g, b, _] = hsv.to_srgba_unmultiplied();
        Some(json!({"r":r,"g":g,"b":b,"a":255}))
    } else {
        None
    }
}

pub(super) fn canvas(
    ui: &mut Ui,
    v: &mut Value,
    s: &Settings,
    state: &mut State,
    assets: &mut skin::Assets,
) {
    if state.preview_scale == 0.0 {
        state.preview_scale = 1.0;
    }
    ui.horizontal_wrapped(|ui| {
        // Wrap whole controls, never their labels. Nested enabled UIs here can
        // inherit the remaining canvas height and create a very tall wrapped row.
        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
        ui.strong(if assets.loaded { "Game preview" } else { "Preview · game skin unavailable" });
        if ui.toggle_value(&mut state.preview_interactive, "Interact").changed() {
            state.runtime = None;
            state.resize = None;
            state.list_scroll.clear();
            state.group_scroll_x.clear();
            state.group_scroll_y.clear();
            state.text_scroll_x.clear();
            state.text_scroll_y.clear();
        }
        if state.preview_interactive && ui.button("Reset").clicked() {
            state.runtime = None;
            state.list_scroll.clear();
            state.group_scroll_x.clear();
            state.group_scroll_y.clear();
            state.text_scroll_x.clear();
            state.text_scroll_y.clear();
        }
        ui.menu_button("Scale", |ui| {
            for (label, scale) in [("100%", 1.0), ("125%", 1.25), ("150%", 1.5), ("200%", 2.0)] {
                if ui.selectable_value(&mut state.preview_scale, scale, label).clicked() {
                    ui.close();
                }
            }
        });
        let states = ui.add_enabled(!state.preview_interactive, egui::Button::new("State"));
        egui::Popup::menu(&states).show(|ui| {
            for (i, label) in
                ["Follow pointer", "Normal", "Hover", "Pressed", "Disabled"].iter().enumerate()
            {
                if ui.selectable_value(&mut state.preview_state, i, *label).clicked() {
                    ui.close();
                }
            }
        });
        for (label, zoom) in [("Fit", 0.0), ("100%", 1.0)] {
            ui.selectable_value(&mut state.zoom, zoom, label);
        }
        ui.toggle_value(&mut state.preview_info, "Resources");
        ui.toggle_value(&mut state.preview_clean, "Clean view");
        ui.menu_button("Screen", |ui| {
            ui.checkbox(&mut state.screen_preview, "Preview on screen");
            for (w, h) in [(1280.0, 720.0), (1920.0, 1080.0), (2560.0, 1440.0)] {
                if ui
                    .selectable_value(&mut state.screen_size, vec2(w, h), format!("{w} × {h}"))
                    .clicked()
                {
                    state.screen_preview = true;
                    ui.close();
                }
            }
            ui.weak("Drag the window title to set its position.");
        });
    });
    if state.preview_info {
        egui::ScrollArea::vertical().id_salt("nui-resource-info").max_height(120.0).show(ui,|ui| {
            for (name,origin) in &assets.origins {ui.small(format!("{name} — {origin}"));}
            for issue in &assets.issues {ui.colored_label(Color32::LIGHT_RED,issue);}
            ui.small("Stock bevel: 4 source pixels. Font rasterization and layout are local; compare final output in NWN.");
        });
    } else if !assets.issues.is_empty() {
        ui.colored_label(
            Color32::LIGHT_RED,
            format!("{} resource issue(s) · see Resources", assets.issues.len()),
        );
    }
    ui.small(if state.preview_interactive {
        "Local simulation · Edit in panels · Changes reset the simulation · NWScript requires NWN"
    } else {
        "Select to edit · Drag handles to resize · Esc cancels a resize"
    });
    if state.resize.as_ref().is_some_and(|r| r.source != *v)
        || ui.input(|i| i.key_pressed(egui::Key::Escape))
    {
        state.resize = None;
    }
    if state.window_move.as_ref().is_some_and(|r| r.source != *v)
        || ui.input(|i| i.key_pressed(egui::Key::Escape))
    {
        state.window_move = None;
    }
    if let Some(m) = &mut state.window_move
        && let Some(pointer) = ui.input(|i| i.pointer.latest_pos())
    {
        m.update(pointer);
    }
    if let Some(resize) = &mut state.resize
        && let Some(pointer) = ui.input(|i| i.pointer.latest_pos())
    {
        resize.move_to(pointer);
    }
    let mut runtime = state.runtime.take();
    if state.preview_interactive && runtime.as_ref().is_none_or(|r| !r.matches(v, s)) {
        runtime = Some(interaction::Session::new(v, s));
        state.list_scroll.clear();
        state.group_scroll_x.clear();
        state.group_scroll_y.clear();
        state.text_scroll_x.clear();
        state.text_scroll_y.clear();
    }
    if state.preview_interactive
        && let Some(runtime) = &mut runtime
    {
        runtime.constrain_geometry();
    }
    let (mut shown, shown_settings) = if state.preview_interactive {
        let r = runtime.as_ref().unwrap();
        (r.doc.clone(), &r.settings)
    } else {
        (v.clone(), s)
    };
    if let Some(resize) = &state.resize {
        resize.apply(&mut shown);
    }
    if let Some(m) = &state.window_move {
        m.apply(&mut shown);
    }
    let context_prefix =
        layouts::canvas_context(&shown, shown_settings, state).map(|(context, prefix)| {
            shown = context;
            prefix
        });
    let empty_rows = Default::default();
    let row_values = runtime.as_ref().map_or(&empty_rows, |r| &r.row_values);
    let mut changes = Vec::new();
    let mut selected_bounds = None;
    let authored = v;
    let v = &shown;
    let s = shown_settings;
    let geometry = resolved(&v["geometry"], s);
    let size = window_size(v, s);
    let available = (ui.available_size() - vec2(0.0, 28.0)).max(vec2(80.0, 80.0));
    if state.screen_size.x <= 0.0 {
        state.screen_size = vec2(1920.0, 1080.0);
    }
    // The client caps UI scale at its screen height over 720 unless
    // ui.unconstrain-scale is set (NWN EE 8193.37: 150% on a 993-high screen
    // draws at 1.38).
    let ui_scale = if state.screen_preview {
        state.preview_scale.min(state.screen_size.y / 720.0)
    } else {
        state.preview_scale
    };
    let extent = if state.screen_preview { state.screen_size / ui_scale } else { size };
    let zoom = if state.zoom == 0.0 {
        ((available.x - 40.0) / (extent.x * ui_scale))
            .min((available.y - 48.0) / (extent.y * ui_scale))
            .clamp(0.1, 1.0)
    } else {
        state.zoom
    };
    let scale = state.resize.as_ref().map_or(zoom * ui_scale, |r| r.scale);
    let mut dropped = None;
    let mut unsupported = BTreeSet::new();
    let mut close_requested = false;
    let mut collapse_requested = None;
    egui::ScrollArea::both()
        .id_salt("nui-canvas")
        .auto_shrink([false, false])
        .max_height(available.y)
        .show(ui, |ui| {
            let (space, background) = ui.allocate_exact_size(
                (extent * scale + vec2(40.0, 48.0)).max(available),
                Sense::click(),
            );
            state.preview_rect = Some(space.intersect(ui.clip_rect()));
            if context_prefix.is_none() && background.clicked() && state.resize.is_none() {
                state.selected.clear();
            }
            let p = ui.painter().with_clip_rect(space.intersect(ui.clip_rect()));
            p.rect_filled(space, 0, Color32::from_rgb(22, 24, 27));
            let screen = Rect::from_center_size(space.center(), extent * scale);
            if state.screen_preview {
                p.rect_stroke(screen, 0, Stroke::new(1.0, Color32::DARK_GRAY), StrokeKind::Inside);
            }
            if state.preview_interactive && runtime.as_ref().is_some_and(|r| r.closed) {
                p.text(
                    space.center(),
                    Align2::CENTER_CENTER,
                    "Window closed · Reset to reopen",
                    egui::FontId::proportional(16.0),
                    Color32::LIGHT_GRAY,
                );
                return;
            }
            let collapsed = flag(&v["collapsed"], s, None, false);
            let collapsible = authored["collapsed"].as_bool() != Some(false);
            let closable = flag(&v["closable"], s, None, true);
            // `false` is how a script asks for no title (and, with collapsing
            // and closing off too, no title bar), not the word "false".
            let title = if *resolved(&v["title"], s) == false {
                String::new()
            } else {
                localized_string(&v["title"], s, None, assets)
            };
            let font = string(&v["font"], s, None);
            let header_height =
                if title.is_empty() && !collapsible && !closable { 0.0 } else { 33.0 * scale };
            let shown_size = if collapsed { vec2(size.x, header_height / scale) } else { size };
            let rect = match state.resize.as_ref().filter(|r| r.path.is_empty()) {
                Some(resize) => Rect::from_min_size(space.min + resize.anchor, shown_size * scale),
                None if state.screen_preview => {
                    let x = num(&geometry["x"], s, None, -1.0);
                    let y = num(&geometry["y"], s, None, -1.0);
                    let offset = vec2(
                        if x < 0.0 { (extent.x - shown_size.x) / 2.0 } else { x },
                        if y < 0.0 { (extent.y - shown_size.y) / 2.0 } else { y },
                    );
                    Rect::from_min_size(screen.min + offset * scale, shown_size * scale)
                }
                None => Rect::from_center_size(space.center(), shown_size * scale),
            };
            let border = flag(&v["border"], s, None, true);
            if !flag(&v["transparent"], s, None, false) {
                p.rect_filled(rect, 0, assets.color("window.background", Color32::BLACK));
            }
            if border && !assets.paint(&p, "window.border_image", rect, scale, true, Color32::WHITE)
            {
                p.rect_stroke(rect, 0, Stroke::new(scale, GOLD), StrokeKind::Inside);
            }
            let header = Rect::from_min_size(rect.min, vec2(rect.width(), header_height));
            if header_height > 0.0 {
                assets.paint(&p, "window.header.normal", header, scale, true, Color32::WHITE);
                // Under its 33-point title bar the client frames the body on its
                // own: two lines between them (NWN EE 8193.37, np_group).
                if border && !collapsed {
                    let body = Rect::from_min_max(header.left_bottom(), rect.max);
                    assets.paint(&p, "window.border_image", body, scale, true, Color32::WHITE);
                }
                let mut title_rect = header.shrink2(vec2(10.0, 2.0) * scale);
                let button_count = usize::from(closable) + usize::from(collapsible);
                title_rect.max.x -= 26.0 * button_count as f32 * scale;
                text(
                    &p,
                    assets,
                    title_rect,
                    &title,
                    &font,
                    scale,
                    assets.color("window.header.label_normal", Color32::LIGHT_GRAY),
                    Align2::LEFT_CENTER,
                    false,
                );
                if closable {
                    let close = Rect::from_center_size(
                        header.right_center() - vec2(17.0 * scale, 0.0),
                        vec2(16.0, 16.0) * scale,
                    );
                    assets.paint(
                        &p,
                        "window.header.close_button.normal",
                        close,
                        scale,
                        false,
                        Color32::WHITE,
                    );
                    if state.preview_interactive
                        && ui
                            .interact(close, ui.id().with("close-window"), Sense::click())
                            .clicked()
                    {
                        close_requested = true;
                    }
                }
                if collapsible {
                    let collapse = Rect::from_center_size(
                        header.right_center()
                            - vec2((17.0 + if closable { 26.0 } else { 0.0 }) * scale, 0.0),
                        vec2(16.0, 16.0) * scale,
                    );
                    let r = ui.interact(collapse, ui.id().with("collapse-window"), Sense::click());
                    r.widget_info(|| {
                        egui::WidgetInfo::labeled(
                            egui::WidgetType::Button,
                            true,
                            if collapsed {
                                "Expand preview window"
                            } else {
                                "Collapse preview window"
                            },
                        )
                    });
                    assets.paint(
                        &p,
                        "window.header.minimize_button.normal",
                        collapse,
                        scale,
                        false,
                        Color32::WHITE,
                    );
                    if r.clicked() && state.preview_interactive {
                        collapse_requested = Some(!collapsed);
                    }
                }
                let mut title_hit = header;
                title_hit.max.x -= (8.0 + 26.0 * button_count as f32) * scale;
                let movable = context_prefix.is_none()
                    && state.screen_preview
                    && !authored["geometry"]["bind"].is_string();
                let r = ui.interact(
                    title_hit,
                    ui.id().with("nui-window"),
                    if movable { Sense::click_and_drag() } else { Sense::click() },
                );
                if movable
                    && r.drag_started()
                    && let Some(origin) = ui.input(|i| i.pointer.press_origin())
                {
                    state.window_move = Some(interaction::WindowMove {
                        source: authored.clone(),
                        origin,
                        initial: (rect.min - screen.min) / scale,
                        position: (rect.min - screen.min) / scale,
                        scale,
                    });
                }
                r.widget_info(|| {
                    egui::WidgetInfo::labeled(egui::WidgetType::Button, true, "Canvas window")
                });
                if context_prefix.is_none() && r.clicked() && state.resize.is_none() {
                    state.selected.clear();
                }
            }
            if context_prefix.is_none() && state.selected.is_empty() && state.guides() {
                p.rect_stroke(
                    rect.expand(2.0),
                    0,
                    Stroke::new(1.5, ui.visuals().selection.bg_fill),
                    StrokeKind::Outside,
                );
            }
            let inner = Rect::from_min_max(
                header.left_bottom() + vec2(8.0, 8.0) * scale,
                rect.right_bottom() - vec2(8.0, 8.0) * scale,
            );
            ui.scope_builder(egui::UiBuilder::new().max_rect(inner), |ui| {
                ui.set_clip_rect(inner.intersect(ui.clip_rect()));
                let mut renderer = Renderer {
                    s,
                    assets,
                    scale,
                    state,
                    dropped: &mut dropped,
                    unsupported: &mut unsupported,
                    window_font: &font,
                    changes: &mut changes,
                    row_values,
                    selected_bounds: &mut selected_bounds,
                    ancestor_disabled: !flag(&v["accepts_input"], s, None, true),
                    context_prefix: context_prefix.as_deref(),
                };
                if !flag(&v["collapsed"], s, None, false) {
                    let desired = natural(&v["root"], s, None, 0) * scale;
                    let mut root = inner;
                    // A column made only of Groups has no child width advice.
                    // Its padding alone must not collapse the entire canvas.
                    let inset = 2.0 * layout_padding(&v["root"], s, None) * scale;
                    if desired.x > inset {
                        root.max.x = (root.left() + desired.x).min(inner.right());
                    }
                    renderer.node(ui, &v["root"], "/root", root, None, 0);
                }
            });
            if !collapsed && flag(&v["resizable"], s, None, true) {
                let corner = rect.right_bottom();
                p.add(egui::Shape::convex_polygon(
                    vec![
                        corner - vec2(16.0 * scale, 0.0),
                        corner,
                        corner - vec2(0.0, 16.0 * scale),
                    ],
                    assets.color("window.scaler", GOLD),
                    Stroke::NONE,
                ));
            }
            if context_prefix.is_none() && state.selected.is_empty() {
                selected_bounds = (!flag(&v["collapsed"], s, None, false)).then_some(rect);
            }
            interaction::resize_handles(ui, authored, state, selected_bounds, scale, space.min);
        });
    ui.small(format!(
        "{} × {} · UI scale {}% · Game resources; local layout preview",
        size.x as u32,
        size.y as u32,
        (ui_scale * 100.0).round() as u32
    ));
    if !unsupported.is_empty() {
        ui.colored_label(
            Color32::YELLOW,
            format!(
                "NWN preview required: {}",
                unsupported.into_iter().collect::<Vec<_>>().join(", ")
            ),
        );
    }
    if let Some((path, ty)) = dropped {
        state.selected = path;
        design::insert(authored, &mut state.selected, ty);
        if ty == "swap" {
            state.new_swap_slot = Some(state.selected.clone());
        }
    }
    if state.resize.is_some() && ui.input(|i| i.pointer.any_released()) {
        let resize = state.resize.take().unwrap();
        if resize.source == *authored && resize.size != resize.initial {
            resize.apply(authored);
        }
    }
    if state.window_move.is_some() && ui.input(|i| i.pointer.any_released()) {
        let m = state.window_move.take().unwrap();
        if m.source == *authored && m.position != m.initial {
            m.apply(authored);
        }
    }
    if let Some(r) = &mut runtime {
        if let Some(collapsed) = collapse_requested {
            r.collapse(collapsed);
            ui.ctx().request_repaint();
        }
        if close_requested {
            r.dispatch("close", "", None, 0);
            r.closed = true;
            ui.ctx().request_repaint();
        }
        if !changes.is_empty() {
            ui.ctx().request_repaint();
        }
        for change in changes {
            r.apply(change);
        }
        if state.preview_interactive && !r.last_event.is_empty() {
            ui.small(&r.last_event);
        }
    }
    state.runtime = runtime;
}

struct Renderer<'a> {
    s: &'a Settings,
    assets: &'a skin::Assets,
    scale: f32,
    state: &'a mut State,
    dropped: &'a mut Option<(String, &'static str)>,
    unsupported: &'a mut BTreeSet<String>,
    window_font: &'a str,
    changes: &'a mut Vec<interaction::Change>,
    row_values: &'a std::collections::BTreeMap<(String, usize), Value>,
    selected_bounds: &'a mut Option<Rect>,
    ancestor_disabled: bool,
    context_prefix: Option<&'a str>,
}

impl Renderer<'_> {
    fn node(
        &mut self,
        ui: &mut Ui,
        node: &Value,
        path: &str,
        outer: Rect,
        row: Option<usize>,
        depth: usize,
    ) {
        if depth > 32 || !outer.is_positive() || !outer.intersects(ui.clip_rect()) {
            return;
        }
        let source_path = path;
        let mapped = layouts::canvas_path(path, self.context_prefix);
        let editable = mapped.is_some();
        let path = mapped.as_deref().unwrap_or(path);
        let mut local_node;
        let node = if let Some(value) = row.and_then(|r| self.row_values.get(&(path.into(), r))) {
            local_node = node.clone();
            local_node["value"] = value.clone();
            &local_node
        } else {
            node
        };
        let ty = node["type"].as_str().unwrap_or("unknown");
        let layout = matches!(ty, "col" | "row" | "group" | "list");
        let s = self.s;
        let scale = self.scale;
        let margin = extra_margin(node, s, row) * scale;
        let rect = outer.shrink(margin);
        if !rect.is_positive() {
            return;
        }
        let live = self.state.preview_interactive && editable;
        let padding = layout_padding(node, s, row) * scale;
        let body = rect.shrink(padding);
        let choices = if matches!(ty, "options" | "tabbar") {
            choice_rects(
                body,
                val(&node["elements"], s, row).as_array().map_or(0, Vec::len),
                val(&node["direction"], s, row).as_i64() == Some(1),
                ty == "tabbar",
                scale,
            )
        } else {
            Vec::new()
        };
        let painted_bounds = choices.iter().fold(rect, |bounds, item| bounds.union(*item));
        let response = ui.interact(
            if live { painted_bounds } else { rect },
            ui.id().with(("canvas", source_path, row)),
            if !editable {
                Sense::hover()
            } else if !live || matches!(ty, "slider" | "sliderf") {
                Sense::click_and_drag()
            } else {
                Sense::click()
            },
        );
        response.widget_info(|| {
            egui::WidgetInfo::labeled(
                egui::WidgetType::Button,
                true,
                format!(
                    "{} {}{}",
                    if editable { "Canvas" } else { "Context" },
                    design::node_name(node),
                    row.map_or(String::new(), |r| format!(" · row {r}"))
                ),
            )
        });
        if editable && response.clicked() && !live && self.state.resize.is_none() {
            structure::select(self.state, path, ui.input(|i| i.modifiers.command));
        }
        if editable && !live {
            response
                .dnd_set_drag_payload(structure::Drag { path: path.into(), node: node.clone() });
        }
        if editable && layout {
            structure::drag_target(ui, &response, path, self.state, true);
        }
        if editable
            && layout
            && let Some(payload) = response.dnd_release_payload::<design::Insert>()
        {
            *self.dropped = Some((path.into(), payload.0));
        }
        let selected = editable && self.state.selected == path;
        if selected && self.selected_bounds.is_none() {
            *self.selected_bounds = Some(rect);
        }
        let visible = flag(&node["visible"], s, row, true);
        let parent_disabled = self.ancestor_disabled;
        let disabled = parent_disabled
            || !flag(&node["enabled"], s, row, true)
            || (!live && selected && self.state.preview_state == 4);
        self.ancestor_disabled = disabled;
        let mode = if !live && selected && self.state.preview_state > 0 {
            self.state.preview_state
        } else if response.is_pointer_button_down_on() {
            3
        } else if response.hovered() {
            2
        } else {
            1
        };
        let state_key = if disabled {
            "normal"
        } else {
            match mode {
                2 => "hover",
                3 => "active",
                // An encouraged control breathes between its normal and hover
                // looks (NWN EE 8193.37, np_misc); a still preview shows the peak.
                _ if flag(&node["encouraged"], s, row, false) => "hover",
                _ => "normal",
            }
        };
        let tint = if disabled { Color32::from_gray(115) } else { Color32::WHITE };
        let p = ui.painter().with_clip_rect(painted_bounds.intersect(ui.clip_rect()));
        let a = self.assets;
        let default_color = a.color("text.color", Color32::WHITE);
        let text_section = match ty {
            "button" | "button_image" => "button",
            "button_select" => "selectable",
            "check" => "checkbox",
            "textedit" => "edit",
            "combo" => "combo",
            _ => "text",
        };
        let default_color = a.color(&format!("{text_section}.text_{state_key}"), default_color);
        let fg = rgba(val(&node["foreground_color"], s, row), default_color) * tint;
        let font = localized_string(&node["font"], s, row, a);
        let font = if font.is_empty() { self.window_font } else { &font };
        let label = |r: Rect, content: &str, align: Align2, wrap: bool| {
            text(&p, a, r, content, font, scale, fg, align, wrap)
        };
        let frame = |section: &str, r: Rect| {
            if !a.paint(&p, &format!("{section}.{state_key}"), r, scale, true, tint) {
                p.rect_filled(r, 0, Color32::BLACK);
                p.rect_stroke(r, 0, Stroke::new(scale, GOLD * tint), StrokeKind::Inside);
            }
        };
        let interactive = live && visible && !disabled;
        let mut changed: Option<Option<Value>> = None;
        let mut normalization = false;
        let mouse = ui.input(|i| {
            [
                response.contains_pointer(),
                i.pointer.primary_down(),
                i.pointer.secondary_down(),
                i.pointer.middle_down(),
            ]
        });
        if visible {
            draw::paint(ui.painter(), node, body, scale, s, a, row, true, mouse);
        }
        if interactive && response.clicked() {
            match ty {
                "button" | "button_image" => changed = Some(None),
                "check" | "button_select" => {
                    changed = Some(Some(json!(!flag(&node["value"], s, row, false))))
                }
                _ => {}
            }
        }
        if !visible {
            if self.state.guides() {
                p.rect_stroke(rect, 0, Stroke::new(1.0, Color32::DARK_GRAY), StrokeKind::Inside);
                text(
                    &p,
                    a,
                    rect,
                    "Hidden",
                    font,
                    scale,
                    Color32::GRAY,
                    Align2::CENTER_CENTER,
                    false,
                );
            }
        } else {
            match ty {
                "col" | "row" | "group" => {
                    if ty == "group" && flag(&node["border"], s, row, true) {
                        a.paint(&p, "window.border_image", body, scale, true, tint);
                    }
                    let mut inner = if ty == "group" { body.shrink(4.0 * scale) } else { body };
                    let mut scrolled = Vec2::ZERO;
                    if ty == "group" {
                        // Nuklear scrolls a group until its content's far edge (with
                        // the group's 2-point padding each side) meets the end of the
                        // bar's track, which falls short of the view by the bar's two
                        // end buttons (NWN EE 8193.37, np_group: 374 + 4 − (124 − 36)
                        // = 290 down).
                        let content = natural(&node["children"][0], s, row, depth + 1) * scale
                            + Vec2::splat(4.0 * scale);
                        let scroll = val(&node["scrollbars"], s, row).as_i64().unwrap_or(4);
                        // x: the vertical bar's width; y: the horizontal bar's height.
                        let bars = vec2(
                            a.number("window.scrollbar_size_x", 18.0),
                            a.number("window.scrollbar_size_y", 18.0),
                        ) * scale;
                        let mut on = [matches!(scroll, 1 | 3), matches!(scroll, 2 | 3)];
                        if scroll == 4 {
                            on[0] = content.x > inner.width();
                            on[1] = content.y > inner.height() - if on[0] { bars.y } else { 0.0 };
                            on[0] = content.x > inner.width() - if on[1] { bars.x } else { 0.0 };
                        }
                        if on[0] {
                            inner.max.y -= bars.y;
                        }
                        if on[1] {
                            inner.max.x -= bars.x;
                        }
                        let key = format!("{path}/{row:?}");
                        for axis in [0, 1] {
                            let horizontal = axis == 0;
                            // A bar's end buttons are as long as the bar is thick.
                            let thick = if horizontal { bars.y } else { bars.x };
                            let max = (content[axis] - (inner.size()[axis] - 2.0 * thick)).max(0.0);
                            let stored = if horizontal {
                                &self.state.group_scroll_x
                            } else {
                                &self.state.group_scroll_y
                            };
                            // Logical units keep the visible content stable when zoom changes.
                            let mut offset = stored.get(&key).copied().unwrap_or(0.0) * scale;
                            if on[axis] {
                                let mut area = body;
                                if horizontal && on[1] {
                                    area.max.x -= bars.x;
                                } else if !horizontal && on[0] {
                                    area.max.y -= bars.y;
                                }
                                let inset = 2.0 * scale;
                                let mut strip = area.shrink(inset);
                                strip.min[1 - axis] = strip.max[1 - axis] - thick;
                                let scroll_response = ui.interact(
                                    strip,
                                    ui.id().with(("group-scroll", axis, source_path, row)),
                                    Sense::click_and_drag(),
                                );
                                scroll_response.widget_info(|| {
                                    egui::WidgetInfo::labeled(
                                        egui::WidgetType::Slider,
                                        !disabled,
                                        format!(
                                            "Scroll group {} {path}",
                                            if horizontal { "horizontally" } else { "vertically" }
                                        ),
                                    )
                                });
                                if !disabled && editable {
                                    if ui.rect_contains_pointer(body) {
                                        offset -= ui.input_mut(|i| {
                                            let delta = i.smooth_scroll_delta[axis];
                                            i.smooth_scroll_delta[axis] = 0.0;
                                            delta
                                        });
                                    }
                                    if (scroll_response.clicked() || scroll_response.dragged())
                                        && let Some(pointer) =
                                            scroll_response.interact_pointer_pos()
                                    {
                                        let start = strip.min[axis] + thick;
                                        let end = strip.max[axis] - thick;
                                        if pointer[axis] < start {
                                            offset -= 20.0 * scale;
                                        } else if pointer[axis] > end {
                                            offset += 20.0 * scale;
                                        } else {
                                            offset = ((pointer[axis] - start)
                                                / (end - start).max(1.0))
                                            .clamp(0.0, 1.0)
                                                * max;
                                        }
                                    }
                                }
                                offset = offset.clamp(0.0, max);
                                self.scrollbar(
                                    &p,
                                    area,
                                    (inner.size()[axis] - 2.0 * thick) / content[axis].max(1.0),
                                    offset / max.max(1.0),
                                    horizontal,
                                );
                            } else {
                                offset = 0.0;
                            }
                            scrolled[axis] = offset;
                            if horizontal {
                                self.state.group_scroll_x.insert(key.clone(), offset / scale);
                            } else {
                                self.state.group_scroll_y.insert(key.clone(), offset / scale);
                            }
                        }
                    }
                    if let Some(children) = node["children"].as_array() {
                        if children.is_empty() && self.state.guides() {
                            p.rect_stroke(
                                inner,
                                0,
                                Stroke::new(1.0, GOLD.gamma_multiply(0.5)),
                                StrokeKind::Inside,
                            );
                            label(inner, "+ Drop a control", Align2::CENTER_CENTER, false);
                        }
                        let gap = 4.0 * scale;
                        let total_gap = children
                            .iter()
                            .take(children.len().saturating_sub(1))
                            .map(|child| child_gap(ty, child) * scale)
                            .sum::<f32>();
                        let sizes: Vec<_> = children
                            .iter()
                            .map(|n| natural(n, s, row, depth + 1) * scale)
                            .collect();
                        let fixed: f32 = children
                            .iter()
                            .zip(&sizes)
                            .filter(|(n, _)| !fills_width(n, s, row))
                            .map(|(_, v)| v.x)
                            .sum();
                        let flex = children.iter().filter(|n| fills_width(n, s, row)).count();
                        let share =
                            ((inner.width() - fixed - total_gap) / flex.max(1) as f32).max(0.0);
                        let height_flex =
                            children.iter().filter(|n| fills_height(n, s, row)).count();
                        let fixed_height: f32 = children
                            .iter()
                            .zip(&sizes)
                            .filter(|(n, _)| !fills_height(n, s, row))
                            .map(|(_, size)| size.y)
                            .sum();
                        let height_share = ((inner.height() - fixed_height - total_gap)
                            / height_flex.max(1) as f32)
                            .max(0.0);
                        let mut cursor = inner.min - scrolled;
                        let old_clip = ui.clip_rect();
                        // Only a group (like the window and a list) clips what is
                        // inside it; a row or a column doesn't: draw layers in it
                        // reach past it in the client (np_scissor2).
                        if ty == "group" {
                            ui.set_clip_rect(inner.intersect(old_clip));
                        }
                        for (i, child) in children.iter().enumerate() {
                            let mut size = sizes[i];
                            if ty == "row" && fills_width(child, s, row) {
                                size.x = share;
                            } else if ty != "row" && fills_width(child, s, row) {
                                size.x = if ty == "group" {
                                    inner.width().max(size.x)
                                } else {
                                    inner.width()
                                };
                            }
                            if ty == "group"
                                && matches!(child["type"].as_str(), Some("col" | "row"))
                            {
                                // A group's layout fills it, or runs past it and scrolls.
                                size.y = inner.height().max(size.y);
                            } else if fills_height(child, s, row) {
                                size.y = if ty == "row" || ty == "group" {
                                    inner.height()
                                } else {
                                    // Never less than its own: the window scrolls instead.
                                    height_share.max(size.y)
                                };
                            } else if ty == "row" && num(&child["height"], s, row, -1.0) < 0.0 {
                                // A control's own height gives way to its row's.
                                size.y = size.y.min(inner.height());
                            }
                            if num(&child["aspect"], s, row, 0.0) > 0.0
                                && num(&child["height"], s, row, -1.0) < 0.0
                            {
                                size.y = size.x / num(&child["aspect"], s, row, 1.0);
                            }
                            self.node(
                                ui,
                                child,
                                &format!("{source_path}/children/{i}"),
                                Rect::from_min_size(cursor, size),
                                row,
                                depth + 1,
                            );
                            if ty == "row" {
                                cursor.x += size.x + gap;
                            } else {
                                cursor.y += size.y + child_gap(ty, child) * scale;
                            }
                        }
                        ui.set_clip_rect(old_clip);
                    }
                }
                "list" => {
                    if flag(&node["border"], s, row, true) {
                        a.paint(&p, "window.border_image", body, scale, true, tint);
                    }
                    let mut inner = body.shrink(4.0 * scale);
                    let height = num(&node["row_height"], s, row, 25.0).clamp(1.0, 4096.0) * scale;
                    let stride = height + 4.0 * scale;
                    let count = val(&node["row_count"], s, row);
                    let count = count
                        .as_array()
                        .map_or_else(|| count.as_u64().unwrap_or(0) as usize, |v| v.len());
                    // Explicit Y/BOTH reserves a scrollbar even if all rows fit.
                    let scrolling =
                        matches!(val(&node["scrollbars"], s, row).as_i64().unwrap_or(2), 2 | 3);
                    // A list's horizontal bar is drawn but has nothing to move:
                    // cells past its width are cut (NWN EE 8193.37, np_list2).
                    let mut y_body = body;
                    if matches!(val(&node["scrollbars"], s, row).as_i64(), Some(1 | 3)) {
                        let height = a.number("window.scrollbar_size_y", 18.0) * scale;
                        let mut area = body;
                        if scrolling {
                            area.max.x -= a.number("window.scrollbar_size_x", 18.0) * scale;
                        }
                        self.scrollbar(&p, area, 1.0, 0.0, true);
                        inner.max.y -= height;
                        y_body.max.y -= height;
                    }
                    let content_height = (count as f32 * stride - 4.0 * scale).max(0.0);
                    // The client scrolls a list by whole rows, its last stop leaving
                    // as many rows on top as fit with 36 points to spare (NWN EE
                    // 8193.37: 10 rows in 100 end at row 9, 40 in 200 at row 35,
                    // 40 in 300 at row 32).
                    let fit = ((inner.height() - 36.0 * scale) / stride).floor().max(0.0);
                    let max_scroll = ((count as f32 - fit).max(0.0) * stride).max(0.0);
                    let scroll_key = format!("{path}/{row:?}");
                    let mut offset = self
                        .state
                        .list_scroll
                        .get(&scroll_key)
                        .copied()
                        .unwrap_or(0.0)
                        .clamp(0.0, max_scroll);
                    if scrolling {
                        let width = a.number("window.scrollbar_size_x", 18.0) * scale;
                        inner.max.x -= width;
                        let track = Rect::from_min_max(
                            pos2(inner.right(), inner.top()),
                            y_body.max - vec2(4.0, 4.0) * scale,
                        );
                        let scroll_response = ui.interact(
                            track,
                            ui.id().with(("list-scroll", source_path, row)),
                            Sense::click_and_drag(),
                        );
                        scroll_response.widget_info(|| {
                            egui::WidgetInfo::labeled(
                                egui::WidgetType::Slider,
                                !disabled,
                                format!("Scroll list {path}"),
                            )
                        });
                        if !disabled && editable {
                            if ui.rect_contains_pointer(body) {
                                offset -= ui.input_mut(|i| {
                                    let d = i.smooth_scroll_delta.y;
                                    i.smooth_scroll_delta.y = 0.0;
                                    d
                                });
                            }
                            if (scroll_response.clicked() || scroll_response.dragged())
                                && let Some(pointer) = scroll_response.interact_pointer_pos()
                            {
                                // Map against the usable track between the native
                                // end buttons so its bottom reaches the final row.
                                offset = ((pointer.y - track.top() - width)
                                    / (track.height() - 2.0 * width).max(1.0))
                                .clamp(0.0, 1.0)
                                    * max_scroll;
                            }
                        }
                        offset = offset.clamp(0.0, max_scroll);
                        self.scrollbar(
                            &p,
                            y_body,
                            inner.height() / content_height.max(1.0),
                            offset / max_scroll.max(1.0),
                            false,
                        );
                    } else {
                        offset = 0.0;
                    }
                    self.state.list_scroll.insert(scroll_key, offset);
                    if let Some(cells) = node["row_template"].as_array() {
                        if cells.is_empty() && self.state.guides() {
                            label(
                                inner,
                                "Empty list · Add controls to the row template",
                                Align2::CENTER_CENTER,
                                true,
                            );
                        }
                        let widths = list_widths(cells, inner.width(), scale);
                        let old_clip = ui.clip_rect();
                        ui.set_clip_rect(inner.intersect(old_clip));
                        let first = (offset / stride).floor() as usize;
                        // Native list rows include a four-unit gap and do not
                        // expose the next row's border when only that gap fits.
                        let end = count.min(
                            ((offset + inner.height() + 4.0 * scale) / stride).floor().max(0.0)
                                as usize,
                        );
                        for r in first..end.min(first.saturating_add(512)) {
                            let mut x = inner.left();
                            for (i, cell) in cells.iter().enumerate() {
                                let width = widths[i];
                                self.node(
                                    ui,
                                    &cell[0],
                                    &format!("{source_path}/row_template/{i}/0"),
                                    Rect::from_min_size(
                                        pos2(x, inner.top() + r as f32 * stride - offset),
                                        vec2(width, height),
                                    ),
                                    Some(r),
                                    depth + 1,
                                );
                                x += width + 4.0 * scale;
                            }
                        }
                        ui.set_clip_rect(old_clip);
                    }
                }
                "button" | "button_image" | "button_select" => {
                    let section = if ty == "button_select" { "selectable" } else { "button" };
                    if ty == "button_select" && flag(&node["value"], s, row, false) {
                        let prop = match state_key {
                            "hover" => "hover_active",
                            "active" => "pressed_active",
                            _ => "normal_active",
                        };
                        a.paint(&p, &format!("selectable.{prop}"), body, scale, true, tint);
                    } else {
                        frame(section, body);
                    }
                    let content = body.shrink2(vec2(6.0, 2.0) * scale);
                    if ty == "button_image" {
                        // Stock NuiButtonImage follows Nuklear's image button:
                        // stretch into button padding, rather than NuiImage fit.
                        let content = body
                            .shrink2(
                                vec2(
                                    a.number("button.padding_x", 2.0)
                                        + a.number("button.image_padding_x", 0.0),
                                    a.number("button.padding_y", 2.0)
                                        + a.number("button.image_padding_y", 0.0),
                                ) * scale,
                            )
                            .shrink(
                                (a.number("button.border", 0.0) + a.number("button.rounding", 4.0))
                                    * scale,
                            );
                        let name = localized_string(&node["label"], s, row, a);
                        if !draw_image(&p, a, &name, content, node, s, row, scale, tint) {
                            label(
                                content,
                                &format!("Missing image: {name}"),
                                Align2::CENTER_CENTER,
                                true,
                            );
                        }
                    } else {
                        label(
                            content,
                            &localized_string(&node["label"], s, row, a),
                            Align2::CENTER_CENTER,
                            false,
                        );
                    }
                }
                "label" => {
                    if body.width() >= a.font_height(font) * scale * 0.5 {
                        let value = localized_string(&node["value"], s, row, a);
                        let align = alignment(
                            val(&node["text_halign"], s, row).as_i64().unwrap_or(0),
                            val(&node["text_valign"], s, row).as_i64().unwrap_or(0),
                        );
                        label(body, &value, align, false);
                    }
                }
                "text" => {
                    // NuiText is a group around wrapped text. As measured in the
                    // client (NWN EE 8193.37): its text starts 5 in and 6 down
                    // from the border and wraps 18 short of its right (or of its vertical
                    // bar), a line is
                    // the font's height plus 4, lines
                    // break at spaces only (a word wider than the line is cut
                    // where it stops fitting), and what scrolls is the laid-out
                    // text plus 36 both ways: X always has 36 to scroll, a short
                    // text none in Y. AUTO shows only the vertical bar.
                    if flag(&node["border"], s, row, true) {
                        a.paint(&p, "window.border_image", body, scale, true, tint);
                    }
                    let mode = val(&node["scrollbars"], s, row).as_i64().unwrap_or(4);
                    let bar_y = a.number("window.scrollbar_size_y", 18.0) * scale;
                    let bar_x = a.number("window.scrollbar_size_x", 18.0) * scale;
                    let horizontal = matches!(mode, 1 | 3);
                    let mut view = body;
                    if horizontal {
                        view.max.y -= bar_y;
                    }
                    let value = localized_string(&node["value"], s, row, a);
                    let (font_id, spacing) = a.font(p.ctx(), font, scale);
                    let pitch = (a.font_height(font) + 4.0) * scale;
                    let layout = |width: f32| {
                        native_text_layout(&p, &value, &font_id, spacing, pitch, fg, width)
                    };
                    let mut content = Rect::from_min_max(
                        body.min + vec2(5.0, 6.0) * scale,
                        view.max - vec2(18.0, 2.0) * scale,
                    );
                    let extra = 36.0 * scale;
                    let mut galley = layout(content.width());
                    let vertical = matches!(mode, 2 | 3)
                        || (mode == 4 && galley.size().y + extra > content.height());
                    if vertical {
                        content.max.x = body.right() - bar_x - 18.0 * scale;
                        galley = layout(content.width().max(1.0));
                    }
                    let key = format!("{path}/{row:?}");
                    let max_x = if horizontal { extra } else { 0.0 };
                    let max_y = if vertical {
                        (galley.size().y + extra - content.height()).max(0.0)
                    } else {
                        0.0
                    };
                    let mut offset = vec2(
                        self.state.text_scroll_x.get(&key).copied().unwrap_or(0.0) * scale,
                        self.state.text_scroll_y.get(&key).copied().unwrap_or(0.0) * scale,
                    );
                    // The bars' rectangles: in BOTH they stop short of each other.
                    let y_area = Rect::from_min_max(body.min, pos2(body.right(), view.bottom()));
                    let x_right = if vertical { body.right() - bar_x } else { body.right() };
                    let x_area = Rect::from_min_max(body.min, pos2(x_right, body.bottom()));
                    for (axis, on, maximum, area, name) in [
                        (1, vertical, max_y, y_area, "vertically"),
                        (0, horizontal, max_x, x_area, "horizontally"),
                    ] {
                        if !on {
                            offset[axis] = 0.0;
                            continue;
                        }
                        let size = if axis == 1 { bar_x } else { bar_y };
                        let inset = 2.0 * scale;
                        let strip = if axis == 1 {
                            Rect::from_min_max(
                                pos2(area.right() - size - inset, area.top() + inset),
                                area.right_bottom() - Vec2::splat(inset),
                            )
                        } else {
                            Rect::from_min_max(
                                pos2(area.left() + inset, area.bottom() - size - inset),
                                area.right_bottom() - Vec2::splat(inset),
                            )
                        };
                        let response = ui.interact(
                            strip,
                            ui.id().with(("text-scroll", axis, source_path, row)),
                            Sense::click_and_drag(),
                        );
                        response.widget_info(|| {
                            egui::WidgetInfo::labeled(
                                egui::WidgetType::Slider,
                                !disabled,
                                format!("Scroll text {name} {path}"),
                            )
                        });
                        if !disabled && editable {
                            if ui.rect_contains_pointer(body) {
                                offset[axis] -= ui.input_mut(|i| {
                                    let delta = i.smooth_scroll_delta[axis];
                                    i.smooth_scroll_delta[axis] = 0.0;
                                    delta
                                });
                            }
                            if (response.clicked() || response.dragged())
                                && let Some(pointer) = response.interact_pointer_pos()
                            {
                                let start = strip.min[axis] + size;
                                let end = strip.max[axis] - size;
                                if pointer[axis] < start {
                                    offset[axis] -= a.font_height(font) * scale;
                                } else if pointer[axis] > end {
                                    offset[axis] += a.font_height(font) * scale;
                                } else {
                                    offset[axis] = ((pointer[axis] - start)
                                        / (end - start).max(1.0))
                                    .clamp(0.0, 1.0)
                                        * maximum;
                                }
                            }
                        }
                        offset[axis] = offset[axis].clamp(0.0, maximum);
                        let length = if axis == 1 {
                            galley.size().y + extra
                        } else {
                            content.width() + extra
                        };
                        self.scrollbar(
                            &p,
                            area,
                            content.size()[axis] / length.max(1.0),
                            offset[axis] / maximum.max(1.0),
                            axis == 0,
                        );
                    }
                    self.state.text_scroll_x.insert(key.clone(), offset.x / scale);
                    self.state.text_scroll_y.insert(key, offset.y / scale);
                    // Scroll the laid-out content, retaining a fixed viewport and wrap
                    // width. A glyph past the wrap width still shows, up to the bars.
                    let right = if vertical { body.right() - bar_x } else { body.right() };
                    let clip = Rect::from_min_max(
                        pos2(content.left(), content.top()),
                        pos2(right - 2.0 * scale, view.bottom() - 2.0 * scale),
                    );
                    p.with_clip_rect(clip.intersect(p.clip_rect())).galley(
                        content.min - offset,
                        galley,
                        fg,
                    );
                }
                "textedit" | "combo" => {
                    let section = if ty == "combo" { "combo" } else { "edit" };
                    frame(section, body);
                    let pad = a.number(&format!("{section}.padding_x"), 10.0) * scale;
                    let mut content = body.shrink2(vec2(pad, 2.0 * scale));
                    let value = if ty == "combo" {
                        // nk_combo_begin_text: a square button inset 4 top and
                        // bottom at the right, and the label stops the content
                        // padding plus a spacing of 4 before it.
                        let button = Rect::from_min_size(
                            pos2(body.right() - body.height(), body.top() + 4.0 * scale),
                            Vec2::splat((body.height() - 8.0 * scale).max(0.0)),
                        );
                        content.max.x = button.left() - pad - 4.0 * scale;
                        let arrow = button;
                        a.paint(&p, "combo.button.normal", arrow, scale, false, tint);
                        let entries = val(&node["elements"], s, row).as_array();
                        let selected = val(&node["value"], s, row);
                        // Native Choices demo: an absent selection becomes the first
                        // option's ID. Empty options retain the value. This is a client
                        // value correction, not a click or an edit to the authored JUI.
                        let entry = entries.and_then(|entries| {
                            entries.iter().find(|e| e[1] == *selected).or_else(|| entries.first())
                        });
                        if interactive
                            && let Some(entry) = entry
                            && entry[1] != *selected
                        {
                            changed = Some(Some(entry[1].clone()));
                            normalization = true;
                        }
                        entry.map(|e| localized_string(&e[0], s, row, a)).unwrap_or_default()
                    } else {
                        let value = localized_string(&node["value"], s, row, a);
                        if value.is_empty() {
                            localized_string(&node["label"], s, row, a)
                        } else {
                            value
                        }
                    };
                    if ty == "textedit" && interactive {
                        let mut value = NativeTextBuffer {
                            text: localized_string(&node["value"], s, row, a),
                            max_bytes: num(&node["max"], s, row, 255.0).max(0.0) as usize,
                        };
                        let multiline = flag(&node["multiline"], s, row, false);
                        let wrap = flag(&node["wordwrap"], s, row, true);
                        let font_id = a.font(ui.ctx(), font, scale).0;
                        // TextEdit's default multiline layouter always wraps. Keep
                        // explicit newlines while disabling automatic line breaks.
                        let mut unwrapped = |ui: &egui::Ui, text: &dyn egui::TextBuffer, _: f32| {
                            let mut job = egui::text::LayoutJob::simple(
                                text.as_str().to_owned(),
                                font_id.clone(),
                                fg,
                                f32::INFINITY,
                            );
                            job.keep_trailing_whitespace = true;
                            ui.fonts_mut(|fonts| fonts.layout_job(job))
                        };
                        let editor = if multiline {
                            egui::TextEdit::multiline(&mut value)
                        } else {
                            egui::TextEdit::singleline(&mut value)
                        };
                        let editor = if multiline && !wrap {
                            editor.layouter(&mut unwrapped)
                        } else {
                            editor
                        };
                        let previous_clip = ui.clip_rect();
                        ui.set_clip_rect(previous_clip.intersect(content));
                        let input = ui.put(
                            content,
                            editor
                                .id(ui.id().with(("nui-input", source_path, row)))
                                .font(a.font(ui.ctx(), font, scale).0)
                                .text_color(fg)
                                .vertical_align(if multiline {
                                    egui::Align::Min
                                } else {
                                    egui::Align::Center
                                })
                                .frame(egui::Frame::NONE)
                                .margin(0)
                                .hint_text(localized_string(&node["label"], s, row, a)),
                        );
                        ui.set_clip_rect(previous_clip);
                        if input.changed() {
                            changed = Some(Some(value.text.into()));
                        }
                    } else {
                        // The client greys a placeholder (an empty input's label).
                        let placeholder = ty == "textedit"
                            && localized_string(&node["value"], s, row, a).is_empty();
                        draw_text(
                            &p,
                            a,
                            content,
                            &value,
                            font,
                            scale,
                            if placeholder { fg.gamma_multiply(0.5) } else { fg },
                            if ty == "textedit" && flag(&node["multiline"], s, row, false) {
                                Align2::LEFT_TOP
                            } else {
                                Align2::LEFT_CENTER
                            },
                            flag(&node["multiline"], s, row, false)
                                && flag(&node["wordwrap"], s, row, true),
                            ty == "combo",
                        );
                    }
                    if ty == "combo" && interactive {
                        // The client's popup (NWN EE 8193.37, np_combo): as wide as
                        // its widest entry plus 67, 23 a row plus 5, the bevel of a
                        // group, its top on the combo's bottom bevel and always a
                        // vertical bar; no mark on the selected entry.
                        let entries: Vec<String> = val(&node["elements"], s, row)
                            .as_array()
                            .map(|e| e.iter().map(|e| localized_string(&e[0], s, row, a)).collect())
                            .unwrap_or_default();
                        let (font_id, spacing) = a.font(ui.ctx(), font, scale);
                        let widest = entries
                            .iter()
                            .map(|e| {
                                native_advances(ui.ctx(), e, &font_id, spacing).iter().sum::<f32>()
                            })
                            .fold(0.0, f32::max);
                        let size = vec2(
                            widest + 67.0 * scale,
                            (23.0 * entries.len() as f32 + 5.0) * scale,
                        );
                        egui::Popup::menu(&response)
                            .frame(egui::Frame::NONE)
                            .at_position(pos2(body.left(), body.bottom() - 5.0 * scale))
                            .gap(0.0)
                            .show(|ui| {
                                let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
                                let p = ui.painter();
                                p.rect_filled(
                                    rect,
                                    0,
                                    a.color("window.background", Color32::BLACK),
                                );
                                a.paint(p, "window.border_image", rect, scale, true, tint);
                                self.scrollbar(p, rect, 0.78, 0.0, false);
                                for (i, entry) in entries.iter().enumerate() {
                                    let item = Rect::from_min_size(
                                        rect.min + vec2(8.0, 5.0 + 23.0 * i as f32) * scale,
                                        vec2(rect.width() - 30.0 * scale, 23.0 * scale),
                                    );
                                    text(
                                        p,
                                        a,
                                        item,
                                        entry,
                                        font,
                                        scale,
                                        fg,
                                        Align2::LEFT_CENTER,
                                        false,
                                    );
                                    let pick = ui.interact(
                                        item,
                                        ui.id().with(("combo-entry", i)),
                                        Sense::click(),
                                    );
                                    pick.widget_info(|| {
                                        egui::WidgetInfo::labeled(
                                            egui::WidgetType::Button,
                                            true,
                                            entry,
                                        )
                                    });
                                    if pick.clicked()
                                        && let Some(id) = val(&node["elements"], s, row)
                                            .get(i)
                                            .map(|e| e[1].clone())
                                    {
                                        changed = Some(Some(id));
                                        normalization = false;
                                        ui.close();
                                    }
                                }
                            });
                    }
                }
                "check" => {
                    // Nuklear's toggle selector follows the logical font height,
                    // not the dimensions of its source texture.
                    let box_size = Vec2::splat(a.font_height(font) * scale);
                    let check = Rect::from_center_size(
                        body.left_center() + vec2(box_size.x * 0.5, 0.0),
                        box_size.min(body.size()),
                    );
                    a.paint(&p, &format!("checkbox.{state_key}"), check, scale, false, tint);
                    if flag(&node["value"], s, row, false) {
                        let size = a
                            .style_picture("checkbox.cursor_normal")
                            .map_or(vec2(15.0, 14.0), |p| p.size)
                            * scale;
                        a.paint(
                            &p,
                            "checkbox.cursor_normal",
                            Rect::from_center_size(check.center(), size.min(check.size())),
                            scale,
                            false,
                            tint,
                        );
                    }
                    label(
                        Rect::from_min_max(
                            pos2(
                                check.right() + a.number("checkbox.spacing", 4.0) * scale,
                                body.top(),
                            ),
                            body.max,
                        ),
                        &localized_string(&node["label"], s, row, a),
                        Align2::LEFT_CENTER,
                        false,
                    );
                }
                "slider" | "sliderf" | "progress" => {
                    let min = num(&node["min"], s, row, 0.0);
                    let max = num(&node["max"], s, row, 1.0);
                    let cursor_size = (a
                        .style_picture("slider.cursor_normal")
                        .map_or(vec2(12.0, 16.0), |p| p.size)
                        * scale)
                        .min(body.size().max(Vec2::ZERO));
                    let bar = Rect::from_center_size(
                        body.center(),
                        vec2((body.width() - cursor_size.x).max(0.001), 3.0 * scale),
                    );
                    if interactive
                        && ty != "progress"
                        && (response.clicked() || response.dragged())
                        && max > min
                        && let Some(pointer) = response.interact_pointer_pos()
                    {
                        let ratio = ((pointer.x - bar.left()) / bar.width()).clamp(0.0, 1.0);
                        let value = min + ratio * (max - min);
                        let step =
                            num(&node["step"], s, row, if ty == "slider" { 1.0 } else { 0.01 });
                        let value = if step > 0.0 {
                            min + ((value - min) / step).round() * step
                        } else {
                            value
                        };
                        let value = value.clamp(min, max);
                        changed = Some(Some(if ty == "slider" {
                            json!(value.round() as i64)
                        } else {
                            json!(value)
                        }));
                    }
                    // The client clamps a bound value into the slider's range and
                    // writes it back (NWN EE 8193.37, np_slider: 100 in 0..8 reads
                    // 8; min raised to 5 turns 0 into 5). A correction, not an edit.
                    let current = num(&node["value"], s, row, 0.0);
                    if interactive
                        && ty != "progress"
                        && changed.is_none()
                        && max >= min
                        && node["value"].get("bind").is_some()
                        && !(min..=max).contains(&current)
                    {
                        let value = current.clamp(min, max);
                        changed = Some(Some(if ty == "slider" {
                            json!(value.round() as i64)
                        } else {
                            json!(value)
                        }));
                        normalization = true;
                    }
                    let ratio = ((num(&node["value"], s, row, 0.0) - min)
                        / (max - min).max(f32::EPSILON))
                    .clamp(0.0, 1.0);
                    if ty == "progress" {
                        frame("progress", body);
                        let inner = body.shrink(4.0 * scale);
                        let filled = Rect::from_min_size(
                            inner.min,
                            vec2(inner.width() * ratio, inner.height()),
                        );
                        if node.get("foreground_color").is_some() {
                            p.rect_filled(filled, 0, fg);
                        } else {
                            a.paint(&p, "progress.cursor_normal", filled, scale, true, tint);
                        }
                    } else {
                        p.rect_filled(bar, 0, Color32::from_gray(55));
                        p.rect_filled(
                            Rect::from_min_size(bar.min, vec2(bar.width() * ratio, bar.height())),
                            0,
                            a.color("slider.bar_filled", GOLD) * tint,
                        );
                        a.paint(
                            &p,
                            "slider.cursor_normal",
                            Rect::from_center_size(
                                pos2(bar.left() + bar.width() * ratio, bar.center().y),
                                cursor_size,
                            ),
                            scale,
                            false,
                            tint,
                        );
                    }
                }
                "image" => {
                    let name = localized_string(&node["value"], s, row, a);
                    if !draw_image(&p, a, &name, body, node, s, row, scale, tint) {
                        p.rect_stroke(
                            body,
                            0,
                            Stroke::new(1.0, Color32::LIGHT_RED),
                            StrokeKind::Inside,
                        );
                        label(body, &format!("Missing image: {name}"), Align2::CENTER_CENTER, true);
                    }
                }
                "options" | "tabbar" => {
                    if let Some(entries) = val(&node["elements"], s, row).as_array() {
                        // NWN evaluates overlapping choice rows independently: a short
                        // Options allocation can overlap the next Tabs and update both.
                        // Still require a click begun here and the same visible UI layer.
                        let press_id = ui.id().with(("choice-press", source_path, row));
                        if ui.input(|i| i.pointer.primary_pressed()) {
                            ui.data_mut(|d| d.remove::<egui::Pos2>(press_id));
                            if interactive
                                && let Some(origin) = ui.input(|i| i.pointer.press_origin())
                            {
                                ui.data_mut(|d| d.insert_temp(press_id, origin));
                            }
                        }
                        let origin = ui.data(|d| d.get_temp::<egui::Pos2>(press_id));
                        if ui.input(|i| i.pointer.primary_released()) {
                            ui.data_mut(|d| d.remove::<egui::Pos2>(press_id));
                        }
                        if interactive
                            && ui.input(|i| i.pointer.primary_clicked())
                            && let Some(origin) = origin
                            && let Some(index) = choices.iter().position(|item| {
                                item.is_positive()
                                    && item.contains(origin)
                                    && ui.rect_contains_pointer(item.intersect(ui.clip_rect()))
                            })
                        {
                            changed = Some(Some(json!(index)));
                        }
                        for (i, (entry, item)) in entries.iter().zip(&choices).enumerate() {
                            let item = *item;
                            let active = val(&node["value"], s, row).as_i64() == Some(i as i64);
                            if ty == "tabbar" {
                                a.paint(
                                    &p,
                                    if active {
                                        "selectable.normal_active"
                                    } else {
                                        "selectable.normal"
                                    },
                                    item,
                                    scale,
                                    true,
                                    tint,
                                );
                                label(
                                    item,
                                    &localized_string(entry, s, row, a),
                                    Align2::CENTER_CENTER,
                                    false,
                                );
                            } else {
                                let diameter = a.font_height(font) * scale;
                                let option = Rect::from_center_size(
                                    item.left_center() + vec2(diameter * 0.5, 0.0),
                                    Vec2::splat(diameter),
                                );
                                a.paint(&p, "option.normal", option, scale, false, tint);
                                if active {
                                    a.paint(
                                        &p,
                                        "option.cursor_normal",
                                        option.shrink(4.0 * scale),
                                        scale,
                                        false,
                                        tint,
                                    );
                                }
                                label(
                                    Rect::from_min_max(
                                        pos2(option.right() + 4.0 * scale, item.top()),
                                        item.max,
                                    ),
                                    &localized_string(entry, s, row, a),
                                    Align2::LEFT_CENTER,
                                    false,
                                );
                            }
                        }
                    }
                }
                "color_picker" => {
                    let color = rgba_channels(val(&node["value"], s, row));
                    // The client's picker has no alpha: as soon as the window is
                    // open its bind reads alpha 255 (NWN EE 8193.37, nui_picker_s
                    // authored with 128). A value correction, not an edit.
                    if interactive && val(&node["value"], s, row).is_object() && color[3] != 255 {
                        let [r, g, b, _] = color;
                        changed = Some(Some(json!({"r":r,"g":g,"b":b,"a":255})));
                        normalization = true;
                    }
                    if let Some(value) = color_picker(
                        ui,
                        &p,
                        body,
                        color,
                        ui.id().with(("nui-color", source_path, row)),
                        scale,
                        a.font_height(font),
                        interactive,
                        tint,
                    ) {
                        changed = Some(Some(value));
                    }
                }
                "chart" => {
                    if let Some(slots) = val(&node["value"], s, row).as_array() {
                        // Measured in the client (nui_chart_s, np_chart3): each series
                        // scales between its own minimum and maximum; a line's points
                        // are a width / count apart from the left edge; columns touch.
                        let chart = body.shrink(4.0 * scale);
                        if slots.is_empty() {
                            label(body, "No chart data.", Align2::CENTER_CENTER, false);
                        }
                        for slot in slots {
                            let Some(data) = val(&slot["data"], s, row).as_array() else {
                                continue;
                            };
                            let values: Vec<_> = data
                                .iter()
                                .filter_map(Value::as_f64)
                                .filter(|n| n.is_finite())
                                .collect();
                            let min = values.iter().copied().fold(f64::INFINITY, f64::min);
                            let max = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
                            if !min.is_finite() || max <= min {
                                continue;
                            }
                            let color =
                                rgba(val(&slot["color"], s, row), Color32::LIGHT_BLUE) * tint;
                            let y = |v: f64| {
                                chart.bottom() - chart.height() * ((v - min) / (max - min)) as f32
                            };
                            let step = chart.width() / data.len().max(1) as f32;
                            let points: Vec<_> = data
                                .iter()
                                .enumerate()
                                .filter_map(|(i, v)| {
                                    v.as_f64()
                                        .filter(|n| n.is_finite())
                                        .map(|v| pos2(chart.left() + i as f32 * step, y(v)))
                                })
                                .collect();
                            if slot["type"] == 1 {
                                // Nuklear's nk_chart_push_column, as the client draws
                                // it (NWN EE 8193.37, np_chart3): a column is
                                // |v / range| high, stands at (v + |min|) / range from
                                // the bottom, or hangs (v - max) / range from the top
                                // when negative. With all values above zero they hang
                                // from the top; nothing keeps them inside the chart.
                                let (h, range) = (f64::from(chart.height()), max - min);
                                for (i, v) in data.iter().enumerate() {
                                    let Some(v) = v.as_f64().filter(|n| n.is_finite()) else {
                                        continue;
                                    };
                                    let height = h * (v / range).abs();
                                    let top = if v >= 0.0 {
                                        f64::from(chart.bottom())
                                            - h * (v + min.abs()) / range.abs()
                                    } else {
                                        f64::from(chart.top()) + h * ((v - max) / range).abs()
                                            - height
                                    };
                                    let x = chart.left() + i as f32 * step;
                                    p.rect_filled(
                                        Rect::from_min_size(
                                            pos2(x, top as f32),
                                            vec2(step, height as f32),
                                        ),
                                        0,
                                        color,
                                    );
                                }
                            } else {
                                p.add(egui::Shape::line(points.clone(), Stroke::new(scale, color)));
                                for point in points {
                                    p.rect_filled(
                                        Rect::from_center_size(point, Vec2::splat(4.0 * scale)),
                                        0,
                                        color,
                                    );
                                }
                            }
                        }
                        // The native chart shows the first slot's legend only.
                        if let Some(slot) = slots.first() {
                            let legend = localized_string(&slot["legend"], s, row, a);
                            text(
                                &p,
                                a,
                                chart,
                                &legend,
                                font,
                                scale,
                                rgba(val(&slot["color"], s, row), Color32::LIGHT_BLUE) * tint,
                                Align2::LEFT_TOP,
                                false,
                            );
                        }
                    }
                }
                "spacer" => {}
                other => {
                    self.unsupported.insert(other.into());
                    p.rect_stroke(body, 0, Stroke::new(1.0, GOLD), StrokeKind::Inside);
                    label(
                        body,
                        &format!("{} · preview in NWN", design::node_name(node)),
                        Align2::CENTER_CENTER,
                        true,
                    );
                }
            }
        }
        if visible {
            draw::paint(ui.painter(), node, body, scale, s, a, row, false, mouse);
        }
        self.ancestor_disabled = parent_disabled;
        if let Some(value) = changed
            && value.as_ref().is_none_or(|v| v != val(&node["value"], s, row))
        {
            self.changes.push(interaction::Change { path: path.into(), row, value, normalization });
        }
        if self.state.guides()
            && (selected
                || (!live && response.hovered())
                || response.dnd_hover_payload::<design::Insert>().is_some())
        {
            ui.painter().rect_stroke(
                rect,
                0,
                Stroke::new(if selected { 2.0 } else { 1.0 }, ui.visuals().selection.bg_fill),
                StrokeKind::Inside,
            );
        }
        let tooltip = if disabled { &node["disabled_tooltip"] } else { &node["tooltip"] };
        let tooltip = localized_string(tooltip, s, row, a);
        if !tooltip.is_empty() {
            response.on_hover_text(tooltip);
        }
    }
    fn scrollbar(&self, p: &Painter, rect: Rect, fraction: f32, offset: f32, horizontal: bool) {
        let width = self.assets.number(
            if horizontal { "window.scrollbar_size_y" } else { "window.scrollbar_size_x" },
            18.0,
        ) * self.scale;
        let inset = 2.0 * self.scale;
        let strip = if horizontal {
            Rect::from_min_max(
                pos2(rect.left() + inset, rect.bottom() - width - inset),
                rect.right_bottom() - Vec2::splat(inset),
            )
        } else {
            Rect::from_min_max(
                pos2(rect.right() - width - inset, rect.top() + inset),
                rect.right_bottom() - Vec2::splat(inset),
            )
        };
        let axis = usize::from(!horizontal);
        let mut dec = strip;
        dec.max[axis] = (dec.min[axis] + width).min(strip.max[axis]);
        let mut inc = strip;
        inc.min[axis] = (inc.max[axis] - width).max(dec.max[axis]);
        let mut track = strip;
        track.min[axis] = dec.max[axis];
        track.max[axis] = inc.min[axis];
        let section = if horizontal { "scrollh" } else { "scrollv" };
        for (property, r) in
            [("normal", track), ("dec_button.normal", dec), ("inc_button.normal", inc)]
        {
            self.assets.paint(
                p,
                &format!("{section}.{property}"),
                r,
                self.scale,
                true,
                Color32::WHITE,
            );
        }
        let mut thumb = track.shrink(self.scale * 2.0);
        let length = thumb.size()[axis];
        let size = length * fraction.clamp(0.05, 1.0);
        thumb.min[axis] += (length - size) * offset.clamp(0.0, 1.0);
        thumb.max[axis] = thumb.min[axis] + size;
        self.assets.paint_tiled(
            p,
            &format!("{section}.cursor_normal"),
            thumb,
            self.scale,
            horizontal,
            Color32::WHITE,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn nui_preview_aspect_alignment_and_list_binds_follow_stock_constants() {
        let target = Rect::from_min_size(pos2(10.0, 20.0), vec2(100.0, 100.0));
        let size = vec2(200.0, 100.0);
        assert_eq!(image_rect(target, size, 0, alignment(0, 0), 1.0).size(), vec2(100.0, 50.0));
        assert_eq!(image_rect(target, size, 1, alignment(0, 0), 1.0).size(), vec2(200.0, 100.0));
        assert_eq!(image_rect(target, size, 5, alignment(0, 0), 1.0), target);
        assert_eq!(image_rect(target, size, 3, alignment(1, 1), 1.0).min, target.min);
        let mut s = Settings::default();
        s.bindings
            .insert("rows".into(), Binding { value: json!(["A", "B"]), ..Default::default() });
        let v = json!({"bind":"rows"});
        assert_eq!(val(&v, &s, Some(1)), "B");
        assert!(val(&v, &s, Some(2)).is_null());
        assert!(val(&v, &s, None).is_array());
    }
}

#[cfg(test)]
#[path = "native_preview_tests.rs"]
mod native_tests;
