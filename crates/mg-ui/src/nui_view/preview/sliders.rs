//! Sliders and the progress bar.
use super::*;

impl Renderer<'_> {
    pub(super) fn slider(ui: &mut Ui, c: &mut Control<'_>) {
        let Control { s, a, scale, node, ty, row, body, tint, fg, interactive, .. } = *c;
        let p = &c.p;
        let response = &c.response;
        let min = num(&node["min"], s, row, 0.0);
        let max = num(&node["max"], s, row, 1.0);
        let cursor_size =
            (a.style_picture("slider.cursor_normal").map_or(vec2(12.0, 16.0), |p| p.size) * scale)
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
            let step = num(&node["step"], s, row, if ty == "slider" { 1.0 } else { 0.01 });
            let value =
                if step > 0.0 { min + ((value - min) / step).round() * step } else { value };
            let value = value.clamp(min, max);
            c.changed =
                Some(Some(if ty == "slider" { json!(value.round() as i64) } else { json!(value) }));
        }
        // The client clamps a bound value into the slider's range and
        // writes it back (NWN EE 8193.37, np_slider: 100 in 0..8 reads
        // 8; min raised to 5 turns 0 into 5). A correction, not an edit.
        let current = num(&node["value"], s, row, 0.0);
        if interactive
            && ty != "progress"
            && c.changed.is_none()
            && max >= min
            && node["value"].get("bind").is_some()
            && !(min..=max).contains(&current)
        {
            let value = current.clamp(min, max);
            c.changed =
                Some(Some(if ty == "slider" { json!(value.round() as i64) } else { json!(value) }));
            c.normalization = true;
        }
        let ratio = ((num(&node["value"], s, row, 0.0) - min) / (max - min).max(f32::EPSILON))
            .clamp(0.0, 1.0);
        if ty == "progress" {
            c.frame("progress", body);
            let inner = body.shrink(4.0 * scale);
            // Over 1 the client draws past the frame, up to what
            // the window or a group cuts (NWN EE 8193.37, np_prog: 1.5).
            let over =
                ((num(&node["value"], s, row, 0.0) - min) / (max - min).max(f32::EPSILON)).max(0.0);
            let filled = Rect::from_min_size(inner.min, vec2(inner.width() * over, inner.height()));
            let p = ui.painter();
            if node.get("foreground_color").is_some() {
                p.rect_filled(filled, 0, fg);
            } else {
                a.paint(p, "progress.cursor_normal", filled, scale, true, tint);
            }
        } else {
            p.rect_filled(bar, 0, Color32::from_gray(55));
            p.rect_filled(
                Rect::from_min_size(bar.min, vec2(bar.width() * ratio, bar.height())),
                0,
                a.color("slider.bar_filled", GOLD) * tint,
            );
            a.paint(
                p,
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
}
