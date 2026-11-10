//! NuiChart: lines and columns as Nuklear draws them.
use super::*;

/// The points of a line through `data`'s numbers, a `step` apart from
/// `left`, each at the height `y` gives its value: every one of them
/// while there are no more than two a pixel, and of more the lowest and
/// the highest value of those that fall on each pixel, in the order they
/// came (what a line through all of them covers of that pixel). `pixels`
/// is a point's.
///
/// (A point and a mark were made for every value, in view or not: a chart
/// bound to a log of a million samples was three vectors of a million and
/// a million shapes a frame.)
pub(super) fn line_points(
    data: &[Value],
    left: f32,
    step: f32,
    pixels: f32,
    y: impl Fn(f64) -> f32,
) -> Vec<egui::Pos2> {
    let number = |v: &Value| v.as_f64().filter(|n| n.is_finite());
    let at = |i: usize, v: f64| pos2(left + i as f32 * step, y(v));
    let across = (data.len() as f32 * step * pixels).ceil().max(1.0);
    if data.len() as f32 <= 2.0 * across {
        return data.iter().enumerate().filter_map(|(i, v)| Some(at(i, number(v)?))).collect();
    }
    let mut points = Vec::new();
    // The pixel being gathered: its lowest and highest values, with where
    // they came.
    let mut gathered: Option<(usize, (usize, f64), (usize, f64))> = None;
    let mut put = |g: (usize, (usize, f64), (usize, f64))| {
        let (_, low, high) = g;
        let (first, second) = if low.0 <= high.0 { (low, high) } else { (high, low) };
        points.push(at(first.0, first.1));
        if second.0 != first.0 {
            points.push(at(second.0, second.1));
        }
    };
    for (i, v) in data.iter().enumerate() {
        let Some(v) = number(v) else { continue };
        let pixel = (i as f32 * step * pixels) as usize;
        match &mut gathered {
            Some((on, low, high)) if *on == pixel => {
                if v < low.1 {
                    *low = (i, v);
                }
                if v > high.1 {
                    *high = (i, v);
                }
            }
            _ => {
                if let Some(done) = gathered.replace((pixel, (i, v), (i, v))) {
                    put(done);
                }
            }
        }
    }
    if let Some(done) = gathered {
        put(done);
    }
    points
}

impl Renderer<'_> {
    pub(super) fn chart(&mut self, ui: &mut Ui, c: &Control<'_>) {
        let Control { s, a, scale, node, row, body, tint, font, .. } = *c;
        let p = &c.p;
        let response = &c.response;
        let mut hover = None;
        if let Some(slots) = val(&node["value"], s, row).as_array() {
            // Measured in the client (nui_chart_s, np_chart3): each series
            // scales between its own minimum and maximum; a line's points
            // are a width / count apart from the left edge; columns touch.
            let chart = body.shrink(4.0 * scale);
            if slots.is_empty() {
                c.label(body, "No chart data.", Align2::CENTER_CENTER, false);
            }
            let pointer = ui.input(|i| i.pointer.hover_pos());
            for slot in slots {
                let Some(data) = val(&slot["data"], s, row).as_array() else {
                    continue;
                };
                let values = || data.iter().filter_map(Value::as_f64).filter(|n| n.is_finite());
                let min = values().fold(f64::INFINITY, f64::min);
                let max = values().fold(f64::NEG_INFINITY, f64::max);
                if !min.is_finite() || max <= min {
                    continue;
                }
                let color = rgba(val(&slot["color"], s, row), Color32::LIGHT_BLUE) * tint;
                let y = |v: f64| chart.bottom() - chart.height() * ((v - min) / (max - min)) as f32;
                let step = chart.width() / data.len().max(1) as f32;
                if slot["type"] == 1 {
                    // Nuklear's nk_chart_push_column, as the client draws
                    // it (NWN EE 8193.37, np_chart3): a column is
                    // |v / range| high, stands at (v + |min|) / range from
                    // the bottom, or hangs (v - max) / range from the top
                    // when negative. With all values above zero they hang
                    // from the top; nothing keeps them inside the chart.
                    let (h, range) = (f64::from(chart.height()), max - min);
                    // Only the window or a group cuts them, not the chart.
                    let p = ui.painter();
                    let clip = ui.clip_rect();
                    // (A chart with no width has columns of none.)
                    let columns = if step > 0.0 { &data[..] } else { &[] };
                    for (i, v) in columns.iter().enumerate() {
                        // Nuklear moves each column right by its index: a
                        // one-point gap between them.
                        let x = chart.left() + i as f32 * (step + scale);
                        // (They run on to the right past the chart, a point
                        // or more apart: those past what is cut are not made.)
                        if x > clip.right() {
                            break;
                        }
                        let Some(v) = v.as_f64().filter(|n| n.is_finite()) else {
                            continue;
                        };
                        if x + step < clip.left() {
                            continue;
                        }
                        let height = h * (v / range).abs();
                        let top = if v >= 0.0 {
                            f64::from(chart.bottom()) - h * (v + min.abs()) / range.abs()
                        } else {
                            f64::from(chart.top()) + h * ((v - max) / range).abs() - height
                        };
                        let column =
                            Rect::from_min_size(pos2(x, top as f32), vec2(step, height as f32));
                        // The client lights the column under the pointer and
                        // shows its value (NWN EE 8193.37, np_chart2: "2.00").
                        let hovered = pointer.is_some_and(|at| column.intersect(clip).contains(at));
                        if hovered {
                            hover = Some(v);
                        }
                        p.rect_filled(column, 0, if hovered { Color32::WHITE } else { color });
                    }
                } else {
                    let pixels = ui.ctx().pixels_per_point();
                    let points = line_points(data, chart.left(), step, pixels, y);
                    let marks = points.clone();
                    p.add(egui::Shape::line(points, Stroke::new(scale, color)));
                    for point in marks {
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
                    p,
                    a,
                    chart,
                    &legend,
                    TextStyle {
                        font,
                        scale,
                        color: rgba(val(&slot["color"], s, row), Color32::LIGHT_BLUE) * tint,
                        align: Align2::LEFT_TOP,
                        wrap: false,
                    },
                );
            }
        }
        if let Some(v) = hover {
            response.clone().on_hover_text_at_pointer(format!("{v:.2}"));
        }
    }
}
