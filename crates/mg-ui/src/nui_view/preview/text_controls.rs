//! Labels, NuiText and the text input.
use super::*;

// NWN's TextEdit max is a UTF-8 byte budget, not egui's character limit.
// Apply it at insertion, preserving the suffix and cursor on rejected input.
pub(super) struct NativeTextBuffer {
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

impl Renderer<'_> {
    pub(super) fn label(&mut self, c: &Control<'_>) {
        let Control { s, a, scale, node, row, body, font, .. } = *c;
        if body.width() >= a.font_height(font) * scale * 0.5 {
            let value = localized_string(&node["value"], s, row, a);
            let align = alignment(
                val(&node["text_halign"], s, row).as_i64().unwrap_or(0),
                val(&node["text_valign"], s, row).as_i64().unwrap_or(0),
            );
            c.label(body, &value, align, false);
        }
    }

    /// NuiText: wrapped text in a group that scrolls it.
    pub(super) fn text_block(&mut self, ui: &mut Ui, c: &Control<'_>) {
        let Control {
            s,
            a,
            scale,
            node,
            row,
            body,
            tint,
            fg,
            font,
            path,
            source_path,
            disabled,
            editable,
            ..
        } = *c;
        let p = &c.p;
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
            a.paint(p, "window.border_image", body, scale, true, tint);
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
            let face = Face { id: &font_id, spacing, color: fg };
            native_text_layout(p.ctx(), a, &value, face, pitch, width)
        };
        let mut content = Rect::from_min_max(
            body.min + vec2(5.0, 6.0) * scale,
            view.max - vec2(18.0, 2.0) * scale,
        );
        let extra = 36.0 * scale;
        let mut galley = layout(content.width());
        let vertical =
            matches!(mode, 2 | 3) || (mode == 4 && galley.size().y + extra > content.height());
        if vertical {
            content.max.x = body.right() - bar_x - 18.0 * scale;
            galley = layout(content.width().max(1.0));
        }
        // (Wrapped past the bottom with no bar to scroll it: cut, as the
        // client draws it. What counts is where the glyphs end, not the
        // last line's spacing below them (a box whose last line shows whole
        // was said cut: the Design session's survey). The box's height that
        // shows it all: the glyphs, 6 above and 2 below, and a horizontal
        // bar's.)
        let ink = galley.mesh_bounds.max.y;
        if !vertical && ink > content.height() + 0.5 {
            let bar = if horizontal { bar_y } else { 0.0 };
            a.record_cut_below(&value, (ink + 8.0 * scale + bar) / scale);
        }
        let key = format!("{path}/{row:?}");
        let max_x = if horizontal { extra } else { 0.0 };
        let max_y =
            if vertical { (galley.size().y + extra - content.height()).max(0.0) } else { 0.0 };
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
                        offset[axis] = ((pointer[axis] - start) / (end - start).max(1.0))
                            .clamp(0.0, 1.0)
                            * maximum;
                    }
                }
            }
            offset[axis] = offset[axis].clamp(0.0, maximum);
            let length = if axis == 1 { galley.size().y + extra } else { content.width() + extra };
            self.scrollbar(
                p,
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
        p.with_clip_rect(clip.intersect(p.clip_rect())).galley(content.min - offset, galley, fg);
    }

    pub(super) fn textedit(&mut self, ui: &mut Ui, c: &mut Control<'_>) {
        let Control { s, a, scale, node, row, body, fg, font, interactive, source_path, .. } = *c;
        let p = &c.p;
        c.frame("edit", body);
        let pad = a.number("edit.padding_x", 10.0) * scale;
        let content = body.shrink2(vec2(pad, 2.0 * scale));
        let value = {
            let value = localized_string(&node["value"], s, row, a);
            if value.is_empty() { localized_string(&node["label"], s, row, a) } else { value }
        };
        if interactive {
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
            // (No page keys, `widgets::page_keys`: the game's NUI text
            // field takes neither Page Down nor Page Up. docs/findings.md)
            let editor = if multiline {
                egui::TextEdit::multiline(&mut value)
            } else {
                egui::TextEdit::singleline(&mut value)
            };
            let editor = if multiline && !wrap { editor.layouter(&mut unwrapped) } else { editor };
            let previous_clip = ui.clip_rect();
            ui.set_clip_rect(previous_clip.intersect(content));
            let input = ui.put(
                content,
                editor
                    .id(ui.id().with(("nui-input", source_path, row)))
                    .font(a.font(ui.ctx(), font, scale).0)
                    .text_color(fg)
                    .vertical_align(if multiline { egui::Align::Min } else { egui::Align::Center })
                    .frame(egui::Frame::NONE)
                    .margin(0)
                    .hint_text(localized_string(&node["label"], s, row, a)),
            );
            ui.set_clip_rect(previous_clip);
            if input.changed() {
                c.changed = Some(Some(value.text.into()));
            }
        } else {
            // The client greys a placeholder (an empty input's label).
            let placeholder = localized_string(&node["value"], s, row, a).is_empty();
            draw_text(
                p,
                a,
                content,
                &value,
                TextStyle {
                    font,
                    scale,
                    color: if placeholder { fg.gamma_multiply(0.5) } else { fg },
                    align: if flag(&node["multiline"], s, row, false) {
                        Align2::LEFT_TOP
                    } else {
                        Align2::LEFT_CENTER
                    },
                    wrap: flag(&node["multiline"], s, row, false)
                        && flag(&node["wordwrap"], s, row, true),
                },
                false,
            );
        }
    }
}
