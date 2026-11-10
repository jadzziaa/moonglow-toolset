//! Pictures: NuiImage, and the picture of a button or a draw layer.
use super::*;

/// A control (or draw layer) whose properties are read: its node, the binds
/// and the list row it is drawn for.
#[derive(Clone, Copy)]
pub(in crate::nui_view) struct Props<'a> {
    pub node: &'a Value,
    pub s: &'a Settings,
    pub row: Option<usize>,
}

/// Aspect constants are the stock nw_inc_nui constants, not egui's image modes.
pub(super) fn image_rect(rect: Rect, size: Vec2, mode: i64, align: Align2, scale: f32) -> Rect {
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

pub(in crate::nui_view) fn draw_image(
    p: &Painter,
    assets: &skin::Assets,
    name: &str,
    rect: Rect,
    props: Props<'_>,
    scale: f32,
    tint: Color32,
) -> bool {
    let Props { node, s, row } = props;
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

impl Renderer<'_> {
    pub(super) fn image(c: &Control<'_>) {
        let Control { s, a, scale, node, row, body, tint, .. } = *c;
        let p = &c.p;
        let name = localized_string(&node["value"], s, row, a);
        if !draw_image(p, a, &name, body, c.props(), scale, tint)
            && let Some(error) = a.picture("gui_error")
        {
            // The client draws its gui_error at its own size in the
            // middle, whatever the aspect and alignment (NWN EE
            // 8193.37, np_imgmiss).
            p.image(
                error.texture.id(),
                Rect::from_center_size(body.center(), error.size * scale),
                Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                tint,
            );
        } else if a.picture(&name).is_none() {
            p.rect_stroke(body, 0, Stroke::new(1.0, Color32::LIGHT_RED), StrokeKind::Inside);
            c.label(body, &format!("Missing image: {name}"), Align2::CENTER_CENTER, true);
        }
    }
}
