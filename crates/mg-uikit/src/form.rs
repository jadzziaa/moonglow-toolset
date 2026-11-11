//! The form helpers the suite's applications share, for their forms to
//! look alike: field labels, section headings, and a PLT palette's
//! swatches.
//!
//! Their chrome colors come from the applied theme's tokens
//! ([`crate::theme::applied`]); where no theme was applied they are
//! egui's visuals' as before the redesign (the strong text color, the
//! separators' stroke), so that an application that has not applied a
//! theme looks as it did.

use egui::{Color32, Ui};

use crate::fonts;
use crate::metrics;
use crate::theme;

/// The room above a section heading that follows other content (its rule
/// is under it; no separator above).
pub const SECTION_GAP: f32 = 10.0;
/// How much larger than the body text a section heading is, where no
/// theme was applied (with one, [`metrics::HEADING_SIZE`]).
const HEADING_SCALE: f32 = 1.25;
/// How much of its pane's width a section heading's rule spans, where no
/// theme was applied (with one, all of it).
const HEADING_RULE: f32 = 0.75;
/// The room between a section heading's rule and what follows, where no
/// theme was applied (with one, [`metrics::GAP_M`]).
const HEADING_ROOM: f32 = 6.0;

/// A form's field label (the first column of an editor's grid: "Tag",
/// "Name"…): in the strong text color (`text`, white in the dark theme),
/// to stand apart from the values beside it; not bold, which is the
/// headings'.
pub fn field_label(ui: &mut Ui, text: impl Into<String>) -> egui::Response {
    let text = egui::RichText::new(text);
    match theme::applied(ui.ctx()) {
        Some(t) => ui.label(text.color(t.tokens().text)),
        None => ui.label(text.strong()),
    }
}

/// A heading in a table's cell (a group of rows: Options › Keyboard's):
/// as a section's, without its rule and room, which a cell has no place
/// for.
pub fn table_heading(ui: &mut Ui, text: impl Into<String>) -> egui::Response {
    let bold = fonts::bold();
    let applied = theme::applied(ui.ctx());
    let size = match applied {
        Some(_) => metrics::HEADING_SIZE,
        None => egui::TextStyle::Body.resolve(ui.style()).size * HEADING_SCALE,
    };
    let mut text = egui::RichText::new(text).size(size);
    text = match applied {
        Some(t) => text.color(t.tokens().text),
        None => text.strong(),
    };
    // (Until the fonts are in, the first frame, the color and size alone.)
    if ui.fonts(|f| f.families().contains(&bold)) {
        text = text.family(bold);
    }
    ui.label(text)
}

/// A form's section heading ("Lighting Scheme", "Environment"): in Ubuntu
/// Bold (headings alone are bold), the strong text color, and larger than
/// the field labels under it; under it a rule a pixel thick, then a little
/// room before what follows. With a theme applied, the mock's `.sec-h`:
/// 15 px, the rule `line-soft` across the pane, 4 px above it and 8 below;
/// without, as before the redesign: a quarter larger than the body, the
/// rule three quarters of the pane, 6 px below.
pub fn section_heading(ui: &mut Ui, text: impl Into<String>) -> egui::Response {
    let applied = theme::applied(ui.ctx());
    let heading = table_heading(ui, text);
    if applied.is_some() {
        ui.add_space(metrics::GAP_S - ui.spacing().item_spacing.y);
    }
    // (A pane with no width of its own yet: the heading's.)
    let pane = ui.available_width();
    let share = if applied.is_some() { 1.0 } else { HEADING_RULE };
    let width = if pane.is_finite() { pane * share } else { heading.rect.width() };
    let (rule, _) = ui.allocate_exact_size(egui::vec2(width, 1.0), egui::Sense::hover());
    let color = match applied {
        Some(t) => t.tokens().line_soft,
        None => ui.visuals().widgets.noninteractive.bg_stroke.color,
    };
    ui.painter().hline(rule.x_range(), rule.center().y, egui::Stroke::new(1.0, color));
    let room =
        if applied.is_some() { metrics::GAP_M - ui.spacing().item_spacing.y } else { HEADING_ROOM };
    ui.add_space(room);
    heading
}

/// Paints a swatch: the tones side by side.
pub fn swatch(ui: &Ui, rect: egui::Rect, tones: &[Color32; 4]) {
    let w = rect.width() / tones.len() as f32;
    for (i, c) in tones.iter().enumerate() {
        let r = egui::Rect::from_min_size(
            rect.min + egui::vec2(w * i as f32, 0.0),
            egui::vec2(w, rect.height()),
        );
        ui.painter().rect_filled(r, 0.0, *c);
    }
}

/// How many colors a PLT palette has (`pal_*`: a row each).
pub const PALETTE_COLORS: u8 = 176;

/// A PLT palette's 176 colors as swatches, sixteen to a row, each named
/// "`what` N" (its tooltip and accessible name) and drawn from its
/// `tones`; `current` outlined where `outline` is set. The color clicked.
/// (The list a color chooser opens: the application's own dropdown holds
/// it, and closes on a click.)
pub fn palette_swatches(
    ui: &mut Ui,
    tones: impl Fn(u8) -> [Color32; 4],
    what: &str,
    current: u8,
    outline: bool,
) -> Option<u8> {
    let mut clicked = None;
    ui.spacing_mut().item_spacing = egui::vec2(2.0, 2.0);
    let stroke = match theme::applied(ui.ctx()) {
        Some(t) => egui::Stroke::new(1.0, t.tokens().accent),
        None => ui.visuals().selection.stroke,
    };
    for row in 0..PALETTE_COLORS / 16 {
        ui.horizontal(|ui| {
            for i in row * 16..row * 16 + 16 {
                let size = egui::vec2(16.0, 14.0);
                let (rect, r) = ui.allocate_exact_size(size, egui::Sense::click());
                swatch(ui, rect, &tones(i));
                if i == current && outline {
                    ui.painter().rect_stroke(rect, 0.0, stroke, egui::StrokeKind::Outside);
                }
                let name = format!("{what} {i}");
                r.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &name));
                if r.on_hover_text(&name).clicked() {
                    clicked = Some(i);
                }
            }
        });
    }
    clicked
}
