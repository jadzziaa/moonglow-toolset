//! Buttons, checkboxes, and the entries of Options and Tabs.
use super::*;

impl Renderer<'_> {
    pub(super) fn button(c: &Control<'_>) {
        let Control { s, a, scale, node, ty, row, body, tint, state_key, .. } = *c;
        let p = &c.p;
        let section = if ty == "button_select" { "selectable" } else { "button" };
        if ty == "button_select" && flag(&node["value"], s, row, false) {
            let prop = match state_key {
                "hover" => "hover_active",
                "active" => "pressed_active",
                _ => "normal_active",
            };
            a.paint(p, &format!("selectable.{prop}"), body, scale, true, tint);
        } else {
            c.frame(section, body);
        }
        let content = body.shrink2(vec2(6.0, 2.0) * scale);
        if ty == "button_image" {
            // Stock NuiButtonImage follows Nuklear's image button:
            // stretch into button padding, rather than NuiImage fit.
            let content = body
                .shrink2(
                    vec2(
                        a.number("button.padding_x", 2.0) + a.number("button.image_padding_x", 0.0),
                        a.number("button.padding_y", 2.0) + a.number("button.image_padding_y", 0.0),
                    ) * scale,
                )
                .shrink(
                    (a.number("button.border", 0.0) + a.number("button.rounding", 4.0)) * scale,
                );
            let name = localized_string(&node["label"], s, row, a);
            // A missing picture becomes the client's gui_error, drawn
            // across the button like any (NWN EE 8193.37, np_bimg).
            if !draw_image(p, a, &name, content, c.props(), scale, tint)
                && !draw_image(p, a, "gui_error", content, c.props(), scale, tint)
            {
                c.label(content, &format!("Missing image: {name}"), Align2::CENTER_CENTER, true);
            }
        } else {
            c.label(
                content,
                &localized_string(&node["label"], s, row, a),
                Align2::CENTER_CENTER,
                false,
            );
        }
    }

    pub(super) fn check(c: &Control<'_>) {
        let Control { s, a, scale, node, row, body, tint, font, state_key, .. } = *c;
        let p = &c.p;
        // Nuklear's toggle selector follows the logical font height,
        // not the dimensions of its source texture.
        let box_size = Vec2::splat(a.font_height(font) * scale);
        let check = Rect::from_center_size(
            body.left_center() + vec2(box_size.x * 0.5, 0.0),
            box_size.min(body.size()),
        );
        a.paint(p, &format!("checkbox.{state_key}"), check, scale, false, tint);
        if flag(&node["value"], s, row, false) {
            let size =
                a.style_picture("checkbox.cursor_normal").map_or(vec2(15.0, 14.0), |p| p.size)
                    * scale;
            a.paint(
                p,
                "checkbox.cursor_normal",
                Rect::from_center_size(check.center(), size.min(check.size())),
                scale,
                false,
                tint,
            );
        }
        c.label(
            Rect::from_min_max(
                pos2(check.right() + a.number("checkbox.spacing", 4.0) * scale, body.top()),
                body.max,
            ),
            &localized_string(&node["label"], s, row, a),
            Align2::LEFT_CENTER,
            false,
        );
    }

    /// Options and Tabs.
    pub(super) fn choices(ui: &mut Ui, c: &mut Control<'_>) {
        let Control { s, a, scale, node, ty, row, tint, font, interactive, source_path, .. } = *c;
        let p = &c.p;
        let choices = &c.choices;
        if let Some(entries) = val(&node["elements"], s, row).as_array() {
            // NWN evaluates overlapping choice rows independently: a short
            // Options allocation can overlap the next Tabs and update both.
            // Still require a click begun here and the same visible UI layer.
            let press_id = ui.id().with(("choice-press", source_path, row));
            if ui.input(|i| i.pointer.primary_pressed()) {
                ui.data_mut(|d| d.remove::<egui::Pos2>(press_id));
                if interactive && let Some(origin) = ui.input(|i| i.pointer.press_origin()) {
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
                c.changed = Some(Some(json!(index)));
            }
            for (i, (entry, item)) in entries.iter().zip(choices).enumerate() {
                let item = *item;
                let active = val(&node["value"], s, row).as_i64() == Some(i as i64);
                if ty == "tabbar" {
                    a.paint(
                        p,
                        if active { "selectable.normal_active" } else { "selectable.normal" },
                        item,
                        scale,
                        true,
                        tint,
                    );
                    c.label(
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
                    a.paint(p, "option.normal", option, scale, false, tint);
                    if active {
                        a.paint(
                            p,
                            "option.cursor_normal",
                            option.shrink(4.0 * scale),
                            scale,
                            false,
                            tint,
                        );
                    }
                    c.label(
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
}
