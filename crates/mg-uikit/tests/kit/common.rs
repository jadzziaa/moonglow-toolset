#![allow(dead_code)]

use egui::{Color32, Shape};
use egui_kittest::Harness;
use mg_uikit::Theme;
use mg_uikit::header;

pub(crate) fn visit_shape(shape: &Shape, f: &mut impl FnMut(&Shape)) {
    match shape {
        Shape::Vec(shapes) => shapes.iter().for_each(|s| visit_shape(s, f)),
        s => f(s),
    }
}

/// Every shape drawn in a frame, flattened.
pub(crate) fn shapes(h: &Harness<'_>) -> Vec<Shape> {
    let mut all = Vec::new();
    h.output()
        .shapes
        .iter()
        .for_each(|s| visit_shape(&s.shape, &mut |shape| all.push(shape.clone())));
    all
}

/// The words drawn in a frame, where.
pub(crate) fn texts(h: &Harness<'_>) -> Vec<(String, egui::Rect)> {
    let mut all = Vec::new();
    for clipped in h.output().shapes.iter() {
        visit_shape(&clipped.shape, &mut |shape| {
            if let Shape::Text(t) = shape {
                all.push((t.galley.text().to_string(), t.galley.rect.translate(t.pos.to_vec2())));
            }
        });
    }
    all
}

/// The words drawn in a frame, with their fallback color.
pub(crate) fn colored_texts(h: &Harness<'_>) -> Vec<(String, Color32)> {
    let mut all = Vec::new();
    for clipped in h.output().shapes.iter() {
        visit_shape(&clipped.shape, &mut |shape| {
            if let Shape::Text(t) = shape {
                all.push((t.galley.text().to_string(), t.fallback_color));
            }
        });
    }
    all
}

/// The visible color the words `text` were painted in.
pub(crate) fn text_color(h: &Harness<'_>, text: &str) -> Option<Color32> {
    shapes(h).into_iter().find_map(|s| match s {
        // Not a row's keys as egui lays them out, drawn transparent for the kit to draw over.
        Shape::Text(t) if t.galley.text() == text => {
            let own = t.galley.job.sections.first().map(|s| s.format.color);
            let own = own.filter(|c| *c != Color32::PLACEHOLDER);
            let color = t.override_text_color.or(own).unwrap_or(t.fallback_color);
            Some(color).filter(|c| c.a() > 0)
        }
        _ => None,
    })
}

/// The rectangles drawn in a frame, with their fills and borders.
pub(crate) fn rects(h: &Harness<'_>) -> Vec<egui::epaint::RectShape> {
    let mut all = Vec::new();
    for clipped in h.output().shapes.iter() {
        visit_shape(&clipped.shape, &mut |shape| {
            if let Shape::Rect(r) = shape {
                all.push(r.clone());
            }
        });
    }
    all
}

/// The rects filled in a frame.
pub(crate) fn fills(h: &Harness<'_>) -> Vec<(egui::Rect, Color32)> {
    rects(h).into_iter().map(|r| (r.rect, r.fill)).collect()
}

/// The fills of the rectangles drawn at `r` in the last frame.
pub(crate) fn rect_fills(h: &Harness<'_>, r: egui::Rect) -> Vec<Color32> {
    rects(h)
        .into_iter()
        .filter(|s| (s.rect.min - r.min).length() < 0.6 && (s.rect.max - r.max).length() < 0.6)
        .map(|s| s.fill)
        .collect()
}

/// The colors of the path strokes painted in a frame (an icon's lines).
pub(crate) fn stroke_colors(h: &Harness<'_>) -> Vec<Color32> {
    let mut all = Vec::new();
    for clipped in h.output().shapes.iter() {
        visit_shape(&clipped.shape, &mut |shape| {
            if let Shape::Path(p) = shape
                && let egui::epaint::ColorMode::Solid(c) = p.stroke.color
            {
                all.push(c);
            }
        });
    }
    all
}

/// Dark-themed header harness with header layout recording enabled.
pub(crate) fn harness<'a>(size: egui::Vec2, app: impl FnMut(&mut egui::Ui) + 'a) -> Harness<'a> {
    let mut app = app;
    Harness::builder().with_size(size).build_ui(move |ui| {
        mg_uikit::apply(ui.ctx(), Theme::Dark);
        header::record_headers(ui.ctx());
        app(ui);
    })
}
