//! NuiColorPicker: Nuklear's RGB picker.
use super::*;

pub(super) fn gradient(p: &Painter, rect: Rect, colors: [Color32; 4]) {
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

/// The picker in the control's body; what a click or drag in it makes of
/// `color`.
fn picker(
    ui: &mut Ui,
    c: &Control<'_>,
    color: [u8; 4],
    id: egui::Id,
    font_height: f32,
) -> Option<Value> {
    let Control { scale, body: rect, interactive, tint, .. } = *c;
    let p = &c.p;
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

impl Renderer<'_> {
    pub(super) fn color_picker(ui: &mut Ui, c: &mut Control<'_>) {
        let Control { s, a, node, row, font, interactive, source_path, .. } = *c;
        let color = rgba_channels(val(&node["value"], s, row));
        // The client's picker has no alpha: as soon as the window is
        // open its bind reads alpha 255 (NWN EE 8193.37, nui_picker_s
        // authored with 128). A value correction, not an edit.
        if interactive && val(&node["value"], s, row).is_object() && color[3] != 255 {
            let [r, g, b, _] = color;
            c.changed = Some(Some(json!({"r":r,"g":g,"b":b,"a":255})));
            c.normalization = true;
        }
        let id = ui.id().with(("nui-color", source_path, row));
        if let Some(value) = picker(ui, c, color, id, a.font_height(font)) {
            c.changed = Some(Some(value));
        }
    }
}
