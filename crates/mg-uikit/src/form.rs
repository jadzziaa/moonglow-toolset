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
    let heading = table_heading(ui, text);
    section_heading_rule(ui, heading)
}

fn section_heading_rule(ui: &mut Ui, heading: egui::Response) -> egui::Response {
    let applied = theme::applied(ui.ctx());
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

/// How a color swatch button is lit ([`paint_swatch_button`]).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SwatchLook {
    /// Under the pointer (or pressed).
    pub hovered: bool,
    /// Its picker open.
    pub open: bool,
    /// Holding the keyboard's focus.
    pub focused: bool,
}

/// The room around a swatch button's color (the mock's `.swatch`:
/// `padding: 3px`).
pub const SWATCH_PADDING: f32 = 3.0;

/// Paints a color swatch button in `rect` (the mock's `.swatch`): a
/// button `raised`, `raised-hover` hovered or open, with its color
/// [`SWATCH_PADDING`] in, painted by `color` into the rectangle it is
/// given and lined with `swatch-line`. Open or focused, a 1 px `accent`
/// border inside the button's own edge (a tight grid of swatches does not
/// cover it, as one drawn outside was). The color's rectangle.
pub fn paint_swatch_button(
    painter: &egui::Painter,
    t: &crate::Tokens,
    rect: egui::Rect,
    look: SwatchLook,
    color: impl FnOnce(&egui::Painter, egui::Rect),
) -> egui::Rect {
    let fill = if look.hovered || look.open { t.raised_hover } else { t.raised };
    let corner = egui::CornerRadius::same(metrics::RADIUS);
    painter.rect_filled(rect, corner, fill);
    let inner = rect.shrink(SWATCH_PADDING);
    color(painter, inner);
    let line = egui::Stroke::new(1.0, t.swatch_line);
    painter.rect_stroke(inner, egui::CornerRadius::same(2), line, egui::StrokeKind::Outside);
    if look.open || look.focused {
        let ring = egui::Stroke::new(1.0, t.accent);
        painter.rect_stroke(rect, corner, ring, egui::StrokeKind::Inside);
    }
    inner
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

/// The room between a form's columns (`.form-cols`: `gap: 16px 32px`).
pub const FORM_COLUMNS_GAP: f32 = 32.0;
/// The room between two sections one over the other in a column (`.sec-h`
/// after anything: `margin-top: 12px`).
pub const SECTIONS_GAP: f32 = metrics::GAP_L;

/// How many columns a form's sections flow into in a room `width` wide:
/// as many as are at least [`metrics::FORM_COLUMN`]'s least (440) with
/// [`FORM_COLUMNS_GAP`] between them, one at the least (the columns share
/// the room: `.form-cols`' `minmax(440px, 1fr)`).
pub fn form_columns(width: f32) -> usize {
    let least = *metrics::FORM_COLUMN.start();
    (((width + FORM_COLUMNS_GAP) / (least + FORM_COLUMNS_GAP)).floor() as usize).max(1)
}

/// What [`FormColumns`] did.
#[derive(Clone, Debug, PartialEq)]
pub struct ColumnsOutput {
    /// How many columns it drew.
    pub columns: usize,
    /// The column each section stood in, in the sections' order.
    pub column_of: Vec<usize>,
    /// Where each section was drawn.
    pub rects: Vec<egui::Rect>,
}

/// A form's sections flowing into columns where there is width (SPEC
/// rule 6, `.form-cols`; the Creature board's Basic page): as many columns
/// as [`form_columns`] gives (or as asked), each section kept whole, in
/// their order down the first column and on down the next (the order the
/// keyboard goes in too), split where the columns come out most even by
/// the sections' heights as last drawn. One column stacks them.
///
/// Each section is drawn in a `Ui` with an id of its own (its widgets
/// keep their ids, the keyboard and what is typed, when it moves to
/// another column).
#[derive(Clone, Debug)]
#[must_use = "show it with .show(ui, sections, |ui, i| ...)"]
pub struct FormColumns {
    id: egui::Id,
    columns: Option<usize>,
    keys: Vec<egui::Id>,
}

impl FormColumns {
    /// Columns kept by `id_salt` (the sections' heights as last drawn).
    pub fn new(id_salt: impl egui::AsId) -> Self {
        FormColumns { id: egui::Id::new(id_salt), columns: None, keys: Vec::new() }
    }

    /// In `columns` columns, not as many as fit (a page that orders its
    /// sections by how many columns it has asks [`form_columns`] first).
    pub fn columns(mut self, columns: usize) -> Self {
        self.columns = Some(columns.max(1));
        self
    }

    /// Each section known by a key of its own, in the order they are given
    /// to [`FormColumns::show`] (a page that orders its sections by how
    /// many columns it has): its widgets' ids and its height kept by its
    /// key, not its place.
    pub fn keys(
        mut self,
        keys: impl IntoIterator<Item = impl std::hash::Hash + std::fmt::Debug>,
    ) -> Self {
        self.keys = keys.into_iter().map(egui::Id::new).collect();
        self
    }

    /// Draws `sections` sections at `ui`'s cursor across its width, each
    /// by `add` (`ui`, the section's index).
    pub fn show(
        self,
        ui: &mut Ui,
        sections: usize,
        mut add: impl FnMut(&mut Ui, usize),
    ) -> ColumnsOutput {
        let width = ui.available_width().max(1.0);
        let columns = self.columns.unwrap_or_else(|| form_columns(width)).min(sections.max(1));
        let kept_id = self.id.with("mg-uikit-form-columns");
        let key = |s: usize| self.keys.get(s).copied().unwrap_or(egui::Id::new(s));
        let kept: Vec<(egui::Id, f32)> = ui.data(|d| d.get_temp(kept_id)).unwrap_or_default();
        let heights: Vec<f32> = (0..sections)
            .map(|s| kept.iter().find(|(k, _)| *k == key(s)).map_or(100.0, |(_, h)| *h))
            .collect();
        let column_of = split(&heights, columns);
        let gap = if columns > 1 { FORM_COLUMNS_GAP } else { 0.0 };
        let column_w = ((width - gap * (columns - 1) as f32) / columns as f32).max(1.0);
        let origin = ui.cursor().min;
        let mut bottoms = vec![origin.y; columns];
        let mut rects = Vec::with_capacity(sections);
        let mut measured = heights.clone();
        for s in 0..sections {
            let c = column_of[s];
            let x = origin.x + c as f32 * (column_w + gap);
            let y = if bottoms[c] > origin.y { bottoms[c] + SECTIONS_GAP } else { bottoms[c] };
            let room =
                egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(column_w, f32::INFINITY));
            let mut child = ui.new_child(
                egui::UiBuilder::new()
                    .id(self.id.with(("mg-uikit-section", key(s))))
                    .max_rect(room)
                    .layout(egui::Layout::top_down(egui::Align::Min)),
            );
            add(&mut child, s);
            let r = child.min_rect();
            let r = egui::Rect::from_min_size(r.min, egui::vec2(column_w, r.height()));
            measured[s] = r.height().round();
            bottoms[c] = r.bottom();
            rects.push(r);
        }
        let bottom = bottoms.iter().copied().fold(origin.y, f32::max);
        ui.advance_cursor_after_rect(egui::Rect::from_min_max(
            origin,
            egui::pos2(origin.x + width, bottom),
        ));
        // (The split by heights now measured: another the next frame.)
        if measured.iter().zip(&heights).any(|(a, b)| (a - b).abs() > 0.5) {
            // (Split otherwise by what was measured: the pass drawn again,
            // never shown split by the heights before.)
            if split(&measured, columns) != column_of {
                ui.ctx().request_discard("a form's sections measured anew");
            }
            let mut kept = kept;
            for (s, h) in measured.iter().enumerate() {
                match kept.iter_mut().find(|(k, _)| *k == key(s)) {
                    Some(e) => e.1 = *h,
                    None => kept.push((key(s), *h)),
                }
            }
            ui.data_mut(|d| d.insert_temp(kept_id, kept));
        }
        ColumnsOutput { columns, column_of, rects }
    }
}

/// The column of each of the sections `heights` high, in order, into
/// `columns` columns, each a run of sections: the split whose tallest
/// column is the least (the earlier split where two tie). Every column
/// has a section where there are as many sections as columns.
fn split(heights: &[f32], columns: usize) -> Vec<usize> {
    let n = heights.len();
    let k = columns.clamp(1, n.max(1));
    if n == 0 {
        return Vec::new();
    }
    // (best[j][i]: the least tallest column of the first i sections in j
    // columns; cut[j][i] where the last of those columns begins.)
    let gap = SECTIONS_GAP;
    let run = |a: usize, b: usize| -> f32 {
        heights[a..b].iter().sum::<f32>() + gap * (b - a).saturating_sub(1) as f32
    };
    let mut best = vec![vec![f32::INFINITY; n + 1]; k + 1];
    let mut cut = vec![vec![0usize; n + 1]; k + 1];
    best[0][0] = 0.0;
    for j in 1..=k {
        for i in j..=n {
            for a in (j - 1)..i {
                let tallest = best[j - 1][a].max(run(a, i));
                if tallest < best[j][i] - 0.01 {
                    best[j][i] = tallest;
                    cut[j][i] = a;
                }
            }
        }
    }
    let mut column_of = vec![0; n];
    let mut i = n;
    for j in (1..=k).rev() {
        let a = cut[j][i];
        for c in column_of.iter_mut().take(i).skip(a) {
            *c = j - 1;
        }
        i = a;
    }
    column_of
}

/// A form's section heading with a note after it in light text and
/// controls at its end (the Creature board's Description: "Description",
/// "English", the languages button at the right): [`section_heading`]'s
/// look. The note is painted, the heading named with it in parentheses
/// for a screen reader ("Description (English)"), so that a note naming a
/// language is not taken for a text's own mark of one. The heading's
/// response.
pub fn section_heading_with(
    ui: &mut Ui,
    text: impl Into<String>,
    note: Option<&str>,
    end: impl FnOnce(&mut Ui),
) -> egui::Response {
    let applied = theme::applied(ui.ctx());
    let text = text.into();
    let heading = ui
        .horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = metrics::GAP_M;
            let heading = table_heading(ui, text.clone());
            if let Some(note) = note {
                let color = match applied {
                    Some(t) => t.tokens().text_3,
                    None => ui.visuals().weak_text_color(),
                };
                let font = egui::FontId::proportional(metrics::BODY_SIZE);
                let galley = ui.painter().layout_no_wrap(note.to_string(), font, color);
                let (rect, _) = ui.allocate_exact_size(galley.size(), egui::Sense::hover());
                let at = egui::pos2(rect.left(), heading.rect.center().y - galley.size().y / 2.0);
                ui.painter().galley(at, galley, color);
                let name = format!("{text} ({note})");
                heading.widget_info(|| {
                    egui::WidgetInfo::labeled(egui::WidgetType::Label, true, &name)
                });
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), end);
            heading
        })
        .inner;
    section_heading_rule(ui, heading)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_form_flows_into_as_many_columns_of_440_as_fit() {
        assert_eq!(form_columns(300.0), 1);
        assert_eq!(form_columns(911.0), 1, "two of 440 and the gap need 912");
        assert_eq!(form_columns(912.0), 2);
        assert_eq!(form_columns(1383.0), 2);
        assert_eq!(form_columns(1384.0), 3);
    }

    #[test]
    fn sections_split_into_runs_whose_tallest_column_is_least() {
        // The Creature board's Basic: Identity and Race and Appearance,
        // then Palette, Portrait and Conversation.
        assert_eq!(split(&[110.0, 170.0, 70.0, 80.0, 90.0], 2), vec![0, 0, 1, 1, 1]);
        assert_eq!(split(&[100.0; 4], 2), vec![0, 0, 1, 1]);
        assert_eq!(split(&[100.0; 3], 1), vec![0, 0, 0]);
        assert_eq!(split(&[100.0; 2], 3), vec![0, 1], "no more columns than sections");
        assert_eq!(split(&[], 2), Vec::<usize>::new());
    }

    /// Sections measured otherwise than they were split by are split
    /// again before the frame is shown: the first frame has the right split
    /// (the review's B8).
    #[test]
    fn the_first_frame_shows_the_split_by_the_heights_measured() {
        // (Each pass's split, by the frame it is of.)
        let passes = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let o = passes.clone();
        let h = egui_kittest::Harness::builder().with_size(egui::vec2(1000.0, 600.0)).build_ui(
            move |ui| {
                let r = FormColumns::new("first").show(ui, 3, |ui, i| {
                    ui.label(format!("Section {i}"));
                    ui.add_space([20.0, 20.0, 200.0][i]);
                });
                o.borrow_mut().push((ui.ctx().cumulative_frame_nr(), r.column_of));
            },
        );
        drop(h);
        let passes = passes.borrow();
        let first = passes[0].0;
        let shown = passes.iter().rev().find(|(f, _)| *f == first).map(|(_, c)| c.clone());
        assert_eq!(
            shown,
            Some(vec![0, 0, 1]),
            "the first frame shown split by the heights: {passes:?}"
        );
    }

    #[test]
    fn sections_stand_in_their_columns_in_order() {
        let mut out = None;
        let mut h =
            egui_kittest::Harness::builder().with_size(egui::vec2(1000.0, 600.0)).build_ui(|ui| {
                let o = FormColumns::new("t").show(ui, 3, |ui, i| {
                    ui.label(format!("Section {i}"));
                    ui.add_space([20.0, 20.0, 200.0][i]);
                });
                out = Some(o);
            });
        h.run();
        drop(h);
        let out = out.unwrap();
        assert_eq!(out.columns, 2, "984 of room: two columns");
        // (As first drawn, its sections of one height to it, the split was
        // after the first; by their heights, after the second.)
        assert_eq!(out.column_of, vec![0, 0, 1], "the two short, the tall section alone beside");
        assert!(out.rects[2].left() >= out.rects[0].right() + FORM_COLUMNS_GAP - 0.5);
        assert!(out.rects[1].top() >= out.rects[0].bottom() + SECTIONS_GAP - 0.5);
        assert!(out.rects[0].width() >= 440.0);
    }
}
