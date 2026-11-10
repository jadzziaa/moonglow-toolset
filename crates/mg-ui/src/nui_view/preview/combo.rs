//! NuiCombo: the closed box and its popup.
use super::*;

impl Renderer<'_> {
    pub(super) fn combo(&mut self, c: &mut Control<'_>) {
        let Control { s, a, scale, node, row, body, tint, fg, font, interactive, .. } = *c;
        let p = &c.p;
        c.frame("combo", body);
        let pad = a.number("combo.padding_x", 10.0) * scale;
        let mut content = body.shrink2(vec2(pad, 2.0 * scale));
        let value = {
            // nk_combo_begin_text: a square button inset 4 top and
            // bottom at the right, and the label stops the content
            // padding plus a spacing of 4 before it.
            let button = Rect::from_min_size(
                pos2(body.right() - body.height(), body.top() + 4.0 * scale),
                Vec2::splat((body.height() - 8.0 * scale).max(0.0)),
            );
            content.max.x = button.left() - pad - 4.0 * scale;
            let arrow = button;
            a.paint(p, "combo.button.normal", arrow, scale, false, tint);
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
                c.changed = Some(Some(entry[1].clone()));
                c.normalization = true;
            }
            entry.map(|e| localized_string(&e[0], s, row, a)).unwrap_or_default()
        };
        draw_text(
            p,
            a,
            content,
            &value,
            TextStyle {
                font,
                scale,
                color: fg,
                align: Align2::LEFT_CENTER,
                wrap: flag(&node["multiline"], s, row, false)
                    && flag(&node["wordwrap"], s, row, true),
            },
            true,
        );
        if interactive {
            self.combo_popup(c);
        }
    }

    fn combo_popup(&mut self, c: &mut Control<'_>) {
        let Control { s, a, scale, node, row, body, tint, fg, font, .. } = *c;
        let response = &c.response;
        egui::Popup::menu(response)
            .frame(egui::Frame::NONE)
            .at_position(pos2(body.left(), body.bottom() - 5.0 * scale))
            .gap(0.0)
            // (Open while its bar is clicked or dragged: an entry closes
            // it, and a click outside.)
            .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
            .show(|ui| {
                // The client's popup (NWN EE 8193.37, np_combo): as wide as
                // its widest entry plus 67, 23 a row plus 5, the bevel of a
                // group, its top on the combo's bottom bevel and always a
                // vertical bar; no mark on the selected entry. (Its entries
                // are read and measured here, while it is open: done for
                // every combo of the window in every frame of Interact, a
                // combo of an appearance.2da's 7,000 names took most of a
                // frame with its list closed.)
                let entries: Vec<String> = val(&node["elements"], s, row)
                    .as_array()
                    .map(|e| e.iter().map(|e| localized_string(&e[0], s, row, a)).collect())
                    .unwrap_or_default();
                let (font_id, spacing) = a.font(ui.ctx(), font, scale);
                let widest = entries
                    .iter()
                    .map(|e| native_advances(ui.ctx(), e, &font_id, spacing).iter().sum::<f32>())
                    .fold(0.0, f32::max);
                // At most 297 high; the rest scrolls (np_combo3: 30 entries).
                let full = (23.0 * entries.len() as f32 + 5.0) * scale;
                let size = vec2(widest + 67.0 * scale, full.min(297.0 * scale));
                let (rect, area) = ui.allocate_exact_size(size, Sense::hover());
                // Scrolled as a group, a list and a text are (`scroll_by`):
                // the wheel taken, so that what is under the popup does not
                // scroll with it, and its bar clicked or dragged. Kept in
                // points, as theirs: the same entries at another scale.
                let scroll_id = ui.id().with("combo-scroll");
                let max = (full - size.y).max(0.0);
                let kept = ui.data(|d| d.get_temp::<f32>(scroll_id)).unwrap_or(0.0);
                let width = a.number("window.scrollbar_size_x", 18.0) * scale;
                let inset = 2.0 * scale;
                let strip = Rect::from_min_max(
                    pos2(rect.right() - width - inset, rect.top() + inset),
                    rect.right_bottom() - vec2(inset, inset),
                );
                let bar_response =
                    ui.interact(strip, ui.id().with("combo-bar"), Sense::click_and_drag());
                bar_response.widget_info(|| {
                    egui::WidgetInfo::labeled(egui::WidgetType::Slider, true, "Scroll the list")
                });
                let bar = Bar { strip, axis: 1, button: width, max, step: Some(23.0 * scale) };
                let offset = scroll_by(ui, &bar_response, area.rect, &bar, kept * scale);
                ui.data_mut(|d| d.insert_temp(scroll_id, offset / scale));
                let p = ui.painter();
                p.rect_filled(rect, 0, a.color("window.background", Color32::BLACK));
                a.paint(p, "window.border_image", rect, scale, true, tint);
                self.scrollbar(
                    p,
                    rect,
                    (0.78 * size.y / full).min(0.78),
                    offset / max.max(1.0),
                    false,
                );
                let list = p.with_clip_rect(rect.shrink(2.0 * scale));
                for (i, entry) in entries.iter().enumerate() {
                    let item = Rect::from_min_size(
                        rect.min + vec2(8.0, 5.0 + 23.0 * i as f32) * scale - vec2(0.0, offset),
                        vec2(rect.width() - 30.0 * scale, 23.0 * scale),
                    );
                    if !rect.intersects(item) {
                        continue;
                    }
                    text(
                        &list,
                        a,
                        item,
                        entry,
                        TextStyle {
                            font,
                            scale,
                            color: fg,
                            align: Align2::LEFT_CENTER,
                            wrap: false,
                        },
                    );
                    let pick = ui.interact(
                        item.intersect(rect),
                        ui.id().with(("combo-entry", i)),
                        Sense::click(),
                    );
                    pick.widget_info(|| {
                        egui::WidgetInfo::labeled(egui::WidgetType::Button, true, entry)
                    });
                    if pick.clicked()
                        && let Some(id) =
                            val(&node["elements"], s, row).get(i).map(|e| e[1].clone())
                    {
                        c.changed = Some(Some(id));
                        c.normalization = false;
                        ui.close();
                    }
                }
            });
    }
}
