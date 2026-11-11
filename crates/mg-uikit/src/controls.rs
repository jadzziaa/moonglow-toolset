//! The controls egui's style cannot give the mockups' look (the
//! proposal's M1 list), as widgets of the crate's own that behave as
//! egui's do: the same ids, keyboard focus, clicks and accessible names,
//! so that what finds or drives egui's finds and drives these.
//!
//! - [`Checkbox`] and [`RadioButton`]: on, the box is `accent-fill` with a
//!   white mark (egui fills a box the same on or off); off, a bordered
//!   well. [`Controls::check_box`], [`Controls::radio_button`] and
//!   [`Controls::radio_choice`] are `ui.checkbox`, `ui.radio` and
//!   `ui.radio_value`.
//! - Chevrons for the combo box and the tree ([`combo_icon`],
//!   [`collapsing_icon`]; egui draws triangles).
//! - A text field's focus border in `accent` (egui's is the selection's
//!   stroke, which is also the color of selected text): [`apply`](crate::apply)
//!   installs a pass that turns the focused field's frame `accent`.
//! - Rows: [`menu_style`] (a menu's rows, hovered `accent-fill`) and
//!   [`row_style`] (a list's or tree's rows, hovered `hover-row`).
//!
//! Where no theme was applied to the context, the check box and radio are
//! egui's own and the chevrons egui's triangles, so that a context without
//! the suite's look (the NUI windows drawn off the screen) draws as before.

use egui::{
    Atom, AtomLayout, Atoms, Color32, Id, IntoAtoms, NumExt as _, Pos2, Rect, Response, Sense,
    Shape, Stroke, Style, Ui, Vec2, Widget, WidgetInfo, WidgetType, pos2, vec2,
};

use crate::metrics;
use crate::theme;
use crate::tokens::Tokens;

/// The side of a check box's box and a radio's circle (the mock's
/// `.check .box`).
pub const BOX: f32 = 14.0;

/// The tokens of a style made by [`crate::theme::style`] (its lightness
/// says which theme).
pub fn tokens_of(style: &Style) -> Tokens {
    match style.visuals.dark_mode {
        true => Tokens::DARK,
        false => Tokens::LIGHT,
    }
}

/// How a check box or radio is drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Look {
    on: bool,
    hovered: bool,
    focused: bool,
    enabled: bool,
}

impl Look {
    fn of(response: &Response, on: bool, enabled: bool) -> Self {
        Look { on, hovered: response.hovered(), focused: response.has_focus(), enabled }
    }

    /// Fill, border and mark.
    fn colors(self, t: &Tokens) -> (Color32, Color32, Color32) {
        match (self.on, self.enabled) {
            (true, true) => {
                let fill = if self.hovered { t.accent_fill_hover } else { t.accent_fill };
                (fill, fill, t.on_fill)
            }
            (true, false) => (t.accent_fill_off, t.accent_fill_off, t.on_fill_off),
            (false, true) => (t.inset, if self.hovered { t.text_2 } else { t.text_3 }, t.on_fill),
            (false, false) => (Color32::TRANSPARENT, t.text_off, t.text_off),
        }
    }
}

/// The painter a control's box is drawn with: at full opacity where the
/// control is disabled (the disabled colors are the tokens' own, not a
/// fade of the enabled ones).
fn box_painter(ui: &Ui, enabled: bool) -> egui::Painter {
    let mut painter = ui.painter().clone();
    if !enabled {
        painter.set_opacity(1.0);
    }
    painter
}

/// The keyboard's focus on a box: an accent ring around it.
fn focus_ring(painter: &egui::Painter, t: &Tokens, rect: Rect, round: bool) {
    let stroke = Stroke::new(1.0, t.accent);
    if round {
        painter.circle_stroke(rect.center(), rect.width() / 2.0 + 2.0, stroke);
    } else {
        let radius = metrics::RADIUS + 2;
        painter.rect_stroke(rect.expand(2.0), radius, stroke, egui::StrokeKind::Inside);
    }
}

/// A check box: the mock's `.check` (on, `accent-fill` with a white mark;
/// off, a well bordered in `text-3`; mixed, a dash; disabled, the
/// `*-off` tokens). `ui.add(Checkbox::new(&mut on, "Plot"))`, or
/// [`Controls::check_box`]. Behaves as [`egui::Checkbox`]: a click or
/// Space toggles it, and its accessible name is its words.
#[must_use = "You should put this widget in a ui with `ui.add(widget);`"]
pub struct Checkbox<'a> {
    checked: &'a mut bool,
    atoms: Atoms<'a>,
    indeterminate: bool,
}

impl<'a> Checkbox<'a> {
    /// A check box of `checked`, labeled `atoms`.
    pub fn new(checked: &'a mut bool, atoms: impl IntoAtoms<'a>) -> Self {
        Checkbox { checked, atoms: atoms.into_atoms(), indeterminate: false }
    }

    /// A check box with no words.
    pub fn without_text(checked: &'a mut bool) -> Self {
        Self::new(checked, ())
    }

    /// Shown as neither on nor off (several objects that differ): a dash.
    /// A click still toggles it.
    #[inline]
    pub fn indeterminate(mut self, indeterminate: bool) -> Self {
        self.indeterminate = indeterminate;
        self
    }
}

impl std::fmt::Debug for Checkbox<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Checkbox")
            .field("checked", &self.checked)
            .field("text", &self.atoms.text())
            .field("indeterminate", &self.indeterminate)
            .finish()
    }
}

impl Widget for Checkbox<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let Checkbox { checked, mut atoms, indeterminate } = self;
        let Some(theme) = theme::applied(ui.ctx()) else {
            return ui.add(egui::Checkbox::new(checked, atoms).indeterminate(indeterminate));
        };
        let t = theme.tokens();
        let mut min_size = Vec2::splat(ui.spacing().interact_size.y);
        min_size.y = min_size.y.at_least(BOX);
        let icon_size = vec2(BOX, BOX.at_least(min_size.y));
        let rect_id = Id::new("egui::checkbox");
        atoms.push_left(Atom::custom(rect_id, icon_size));
        let text = atoms.text().map(String::from);
        let mut prepared = AtomLayout::new(atoms)
            .sense(Sense::click())
            .min_size(min_size)
            .frame(egui::Frame::new())
            .allocate(ui);
        if prepared.response.clicked() {
            *checked = !*checked;
            prepared.response.mark_changed();
        }
        let enabled = ui.is_enabled();
        prepared.response.widget_info(|| {
            let text = text.as_deref().unwrap_or("");
            match indeterminate {
                true => WidgetInfo::labeled(WidgetType::Checkbox, enabled, text),
                false => WidgetInfo::selected(WidgetType::Checkbox, enabled, *checked, text),
            }
        });
        if !ui.is_rect_visible(prepared.response.rect) {
            return prepared.response;
        }
        prepared.fallback_text_color = ui.style().interact(&prepared.response).text_color();
        let look = Look::of(&prepared.response, *checked || indeterminate, enabled);
        let response = prepared.paint(ui);
        if let Some(rect) = response.rect(rect_id) {
            let square = Rect::from_center_size(
                pos2(rect.left() + BOX / 2.0, rect.center().y),
                Vec2::splat(BOX),
            );
            paint_check(&box_painter(ui, enabled), &t, square, look, indeterminate);
        }
        response.response
    }
}

/// Paints a check box's box in `square` (14 px).
fn paint_check(painter: &egui::Painter, t: &Tokens, square: Rect, look: Look, mixed: bool) {
    let (fill, border, mark) = look.colors(t);
    painter.rect(square, metrics::RADIUS, fill, Stroke::new(1.0, border), egui::StrokeKind::Inside);
    if look.on {
        let inner = Rect::from_center_size(square.center(), Vec2::splat(12.0));
        let stroke = Stroke::new(1.5, mark);
        if mixed {
            let y = inner.center().y;
            painter.hline(inner.shrink(2.0).x_range(), y, stroke);
        } else {
            // The drawn set's check (M20 6 9 17 4 12), at 12 of 24.
            let at = |x: f32, y: f32| inner.min + vec2(x, y) * (inner.width() / 24.0);
            painter.add(Shape::line(vec![at(4.5, 12.5), at(9.5, 17.0), at(19.5, 7.0)], stroke));
        }
    }
    if look.focused {
        focus_ring(painter, t, square, false);
    }
}

/// A radio button: the mock's `.check` with a round box (on, a white dot
/// in an `accent-fill` ring; off, a well bordered in `text-3`).
/// `ui.add(RadioButton::new(on, "Fast"))`, or
/// [`Controls::radio_button`] and [`Controls::radio_choice`]. Behaves as
/// [`egui::RadioButton`].
#[must_use = "You should put this widget in a ui with `ui.add(widget);`"]
pub struct RadioButton<'a> {
    checked: bool,
    atoms: Atoms<'a>,
}

impl<'a> RadioButton<'a> {
    /// A radio button, on when `checked`, labeled `atoms`.
    pub fn new(checked: bool, atoms: impl IntoAtoms<'a>) -> Self {
        RadioButton { checked, atoms: atoms.into_atoms() }
    }
}

impl std::fmt::Debug for RadioButton<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RadioButton")
            .field("checked", &self.checked)
            .field("text", &self.atoms.text())
            .finish()
    }
}

impl Widget for RadioButton<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let RadioButton { checked, mut atoms } = self;
        let Some(theme) = theme::applied(ui.ctx()) else {
            return ui.add(egui::RadioButton::new(checked, atoms));
        };
        let t = theme.tokens();
        let mut min_size = Vec2::splat(ui.spacing().interact_size.y);
        min_size.y = min_size.y.at_least(BOX);
        let icon_size = vec2(BOX, BOX.at_least(min_size.y));
        let rect_id = Id::new("egui::radio_button");
        atoms.push_left(Atom::custom(rect_id, icon_size));
        let text = atoms.text().map(String::from);
        let mut prepared =
            AtomLayout::new(atoms).sense(Sense::click()).min_size(min_size).allocate(ui);
        let enabled = ui.is_enabled();
        prepared.response.widget_info(|| {
            let text = text.as_deref().unwrap_or("");
            WidgetInfo::selected(WidgetType::RadioButton, enabled, checked, text)
        });
        if !ui.is_rect_visible(prepared.response.rect) {
            return prepared.response;
        }
        prepared.fallback_text_color = ui.style().interact(&prepared.response).text_color();
        let look = Look::of(&prepared.response, checked, enabled);
        let response = prepared.paint(ui);
        if let Some(rect) = response.rect(rect_id) {
            let center = pos2(rect.left() + BOX / 2.0, rect.center().y);
            paint_radio(&box_painter(ui, enabled), &t, center, look);
        }
        response.response
    }
}

/// Paints a radio's circle around `center`.
fn paint_radio(painter: &egui::Painter, t: &Tokens, center: Pos2, look: Look) {
    let (fill, border, mark) = look.colors(t);
    let r = BOX / 2.0;
    if look.on {
        // (The mock's 4 px border of `accent-fill` around a white middle.)
        painter.circle_filled(center, r, fill);
        painter.circle_filled(center, r - 4.0, mark);
    } else {
        painter.circle(center, r - 0.5, fill, Stroke::new(1.0, border));
    }
    if look.focused {
        let square = Rect::from_center_size(center, Vec2::splat(BOX));
        focus_ring(painter, t, square, true);
    }
}

/// `ui.check_box`, `ui.radio_button` and `ui.radio_choice`: the crate's
/// [`Checkbox`] and [`RadioButton`] as egui's `ui.checkbox`, `ui.radio`
/// and `ui.radio_value` are.
pub trait Controls {
    /// A [`Checkbox`] of `checked`: as `ui.checkbox`.
    fn check_box<'a>(&mut self, checked: &'a mut bool, atoms: impl IntoAtoms<'a>) -> Response;
    /// A [`RadioButton`]: as `ui.radio`.
    fn radio_button<'a>(&mut self, selected: bool, atoms: impl IntoAtoms<'a>) -> Response;
    /// A [`RadioButton`] that is on where `*current == alternative`, and
    /// clicked, sets it so: as `ui.radio_value`.
    fn radio_choice<'a, V: PartialEq>(
        &mut self,
        current: &mut V,
        alternative: V,
        atoms: impl IntoAtoms<'a>,
    ) -> Response;
}

impl Controls for Ui {
    fn check_box<'a>(&mut self, checked: &'a mut bool, atoms: impl IntoAtoms<'a>) -> Response {
        self.add(Checkbox::new(checked, atoms))
    }

    fn radio_button<'a>(&mut self, selected: bool, atoms: impl IntoAtoms<'a>) -> Response {
        self.add(RadioButton::new(selected, atoms))
    }

    fn radio_choice<'a, V: PartialEq>(
        &mut self,
        current: &mut V,
        alternative: V,
        atoms: impl IntoAtoms<'a>,
    ) -> Response {
        let mut response = self.radio_button(*current == alternative, atoms);
        if response.clicked() && *current != alternative {
            *current = alternative;
            response.mark_changed();
        }
        response
    }
}

/// A text field's margins (the mock's `.field`: 6 px at each side; the
/// body's 15 px line in a 22 px box, a pixel more above it than under).
pub const FIELD_MARGIN: egui::Margin = egui::Margin { left: 6, right: 6, top: 4, bottom: 3 };

/// A one-line text field as the mock's `.field` (22 px high): egui's `TextEdit::singleline` with
/// [`FIELD_MARGIN`], for `ui.add(text_field(&mut name).desired_width(..))`
/// in its place. Its focus border is `accent` (the pass [`crate::apply`]
/// installs).
pub fn text_field(text: &mut dyn egui::TextBuffer) -> egui::TextEdit<'_> {
    egui::TextEdit::singleline(text).margin(FIELD_MARGIN)
}

/// A slider in the theme (the mock's `.slider`): egui's [`egui::Slider`]
/// (its drag, keys, value field and accessible value are egui's), drawn in
/// a scope where its rail is an `inset` well, the part up to its thumb
/// `accent-fill`, and its thumb ringed in `text-2`: at rest filled as the
/// rail (egui fills the rail and the resting thumb alike), under the
/// pointer or held `slider-thumb`. `slider(ui, Slider::new(&mut v, 0..=100))`
/// for `ui.add(..)`; egui's own where no theme was applied.
pub fn slider(ui: &mut Ui, slider: egui::Slider<'_>) -> Response {
    let Some(theme) = theme::applied(ui.ctx()) else { return ui.add(slider) };
    let t = theme.tokens();
    ui.scope(|ui| {
        let v = ui.visuals_mut();
        v.selection.bg_fill = t.accent_fill;
        let w = &mut v.widgets;
        // (The rail is the resting state's fill; the thumb each state's
        // fill, ringed in its text stroke, which is the value's text too.)
        w.inactive.bg_fill = t.inset;
        w.hovered.bg_fill = t.slider_thumb;
        w.active.bg_fill = t.slider_thumb;
        for state in [&mut w.inactive, &mut w.hovered, &mut w.active] {
            state.fg_stroke = Stroke::new(1.0, t.text_2);
        }
        ui.add(slider)
    })
    .inner
}

/// Paints a chevron (the drawn set's `chevron-right`, 12 px, a stroke as
/// the mock's `.chev`) centered on `center`, turned by `turn` quarter
/// turns clockwise from pointing right (1: down).
pub fn paint_chevron(painter: &egui::Painter, center: Pos2, turn: f32, color: Color32) {
    let (sin, cos) = (turn * std::f32::consts::FRAC_PI_2).sin_cos();
    let at = |x: f32, y: f32| {
        // The 24 box's (9 6, 15 12, 9 18) about its middle, at 12 px.
        let (x, y) = ((x - 12.0) * 0.5, (y - 12.0) * 0.5);
        center + vec2(x * cos - y * sin, x * sin + y * cos)
    };
    let stroke = Stroke::new(1.25, color);
    painter.add(Shape::line(vec![at(9.0, 6.0), at(15.0, 12.0), at(9.0, 18.0)], stroke));
}

/// A tree's or collapsing header's arrow as a chevron (the mock's
/// `.tree-row .arrow`: `text-3`, `text` hovered), right when closed and
/// down when open: `CollapsingHeader::icon(collapsing_icon)` or
/// `CollapsingState::show_toggle_button(ui, collapsing_icon)`. Egui's
/// triangle where no theme was applied.
pub fn collapsing_icon(ui: &mut Ui, openness: f32, response: &Response) {
    let Some(theme) = theme::applied(ui.ctx()) else {
        return egui::collapsing_header::paint_default_icon(ui, openness, response);
    };
    let t = theme.tokens();
    let hovered = response.hovered() || response.has_focus();
    let color = if hovered { t.text } else { t.text_3 };
    paint_chevron(ui.painter(), response.rect.center(), openness, color);
}

/// A combo box's chevron (the mock's `.combo .chev`: `text-2` pointing
/// down): `ComboBox::icon(combo_icon)`. Egui's triangle where no theme
/// was applied.
pub fn combo_icon(ui: &Ui, rect: Rect, visuals: &egui::style::WidgetVisuals, _open: bool) {
    let Some(theme) = theme::applied(ui.ctx()) else {
        // (egui's own: a triangle pointing down.)
        let rect =
            Rect::from_center_size(rect.center(), vec2(rect.width() * 0.7, rect.height() * 0.45));
        let points = vec![rect.left_top(), rect.right_top(), rect.center_bottom()];
        ui.painter().add(Shape::convex_polygon(points, visuals.fg_stroke.color, visuals.fg_stroke));
        return;
    };
    let t = theme.tokens();
    paint_chevron(ui.painter(), rect.center(), 1.0, t.text_2);
}

/// A menu's style (egui's [`egui::containers::menu::menu_style`], with
/// the mock's `.menu-row`): rows hovered or open in `accent-fill` with
/// their words in `on-fill`, 8 px at each end. For `MenuConfig::style`,
/// and inside a context menu, where egui's own is fixed, through
/// [`menu_rows`].
pub fn menu_style(style: &mut Style) {
    egui::containers::menu::menu_style(style);
    let t = tokens_of(style);
    style.spacing.button_padding = vec2(metrics::CONTROL_PADDING_X, 0.0);
    let w = &mut style.visuals.widgets;
    for state in [&mut w.hovered, &mut w.active, &mut w.open] {
        state.weak_bg_fill = t.accent_fill;
        state.bg_fill = t.accent_fill;
        state.fg_stroke.color = t.on_fill;
    }
    no_borders(w);
}

/// The menu's style ([`menu_style`]) given to the rows that follow in
/// `ui` (a context menu's, whose style egui fixes).
pub fn menu_rows(ui: &mut Ui) {
    menu_style(ui.style_mut());
}

/// A menu bar's style: its titles hovered in `raised`, open in
/// `accent-fill` (the mock's `.menu`).
pub fn menu_bar_style(style: &mut Style) {
    egui::containers::menu::menu_style(style);
    let t = tokens_of(style);
    style.spacing.button_padding = vec2(metrics::CONTROL_PADDING_X, 0.0);
    let w = &mut style.visuals.widgets;
    w.hovered.weak_bg_fill = t.raised;
    w.hovered.bg_fill = t.raised;
    for state in [&mut w.active, &mut w.open] {
        state.weak_bg_fill = t.accent_fill;
        state.bg_fill = t.accent_fill;
        state.fg_stroke.color = t.on_fill;
    }
    no_borders(w);
}

/// No border to see on a widget in any state, each as wide as the theme's
/// (egui's menus take them away, which made a row at rest 2 px narrower
/// than hovered: the width a border takes from the padding is given back
/// by the expansion, [`crate::theme::visuals`]).
fn no_borders(w: &mut egui::style::Widgets) {
    for state in [&mut w.inactive, &mut w.hovered, &mut w.active, &mut w.open] {
        state.bg_stroke = Stroke::new(1.0, Color32::TRANSPARENT);
    }
}

/// A list's or tree's rows (selectable labels, a combo box's list): hovered
/// in `hover-row` rather than a button's `raised-hover`, no border (the
/// mock's `.tree-row:hover`, `.list-row:hover`). Selected rows keep
/// `accent-fill`.
pub fn row_style(style: &mut Style) {
    let t = tokens_of(style);
    let w = &mut style.visuals.widgets;
    w.hovered.weak_bg_fill = t.hover_row;
    w.hovered.bg_stroke.color = Color32::TRANSPARENT;
    w.active.weak_bg_fill = t.hover_row;
    w.active.bg_stroke.color = Color32::TRANSPARENT;
}

/// [`row_style`] given to the rows that follow in `ui`.
pub fn list_rows(ui: &mut Ui) {
    if theme::applied(ui.ctx()).is_some() {
        row_style(ui.style_mut());
    }
}

/// The pass [`apply`](crate::apply) installs at the end of each of a
/// context's passes, for what egui's style cannot say:
///
/// - the focused text field's frame turned `accent` (egui draws it in
///   the selection's stroke, which is the color of selected text too).
///   Only a frame egui drew itself, with the fields' fill and in that
///   stroke, is changed: a field without a frame (a code editor's) keeps
///   none;
/// - a tooltip's fill, `tooltip-bg` (egui's is every popup's,
///   `window_fill`): the frame of each tooltip's layer.
pub(crate) fn end_pass(ui: &mut Ui) {
    let ctx = ui.ctx();
    let Some(theme) = theme::applied(ctx) else { return };
    let t = theme.tokens();
    let style = ctx.global_style();
    focused_field_border(ctx, &t, &style);
    tooltip_fill(ctx, &t, &style);
}

/// The indices of the shapes in `layer` that `pick` takes.
fn shapes_where(
    ctx: &egui::Context,
    layer: egui::LayerId,
    pick: impl Fn(&egui::epaint::RectShape) -> bool,
    change: impl Fn(&mut egui::epaint::RectShape),
) {
    // (A frame with a shadow is drawn as a group: the shadow, then the
    // frame.)
    fn visit(
        shape: &mut Shape,
        pick: &impl Fn(&egui::epaint::RectShape) -> bool,
        change: &impl Fn(&mut egui::epaint::RectShape),
    ) {
        match shape {
            Shape::Rect(r) if pick(r) => change(r),
            Shape::Vec(shapes) => shapes.iter_mut().for_each(|s| visit(s, pick, change)),
            _ => {}
        }
    }
    fn any(shape: &Shape, pick: &impl Fn(&egui::epaint::RectShape) -> bool) -> bool {
        match shape {
            Shape::Rect(r) => pick(r),
            Shape::Vec(shapes) => shapes.iter().any(|s| any(s, pick)),
            _ => false,
        }
    }
    ctx.graphics_mut(|g| {
        let Some(list) = g.get_mut(layer) else { return };
        // (Read first, and only the shapes found changed: a layer of a
        // code editor's thousands is not written over each frame.)
        let found: Vec<usize> = (list.all_entries().enumerate())
            .filter(|(_, c)| any(&c.shape, &pick))
            .map(|(i, _)| i)
            .collect();
        for i in found {
            list.mutate_shape(egui::layers::ShapeIdx(i), |c| visit(&mut c.shape, &pick, &change));
        }
    });
}

fn focused_field_border(ctx: &egui::Context, t: &Tokens, style: &Style) {
    // (Only while a text field has the keys. Its rectangle is not asked
    // of egui: `Context::read_response` counts as the widget asking for
    // the keys, and kept them on a field no longer drawn.)
    let Some(id) = ctx.memory(|m| m.focused()) else { return };
    if egui::text_edit::TextEditState::load(ctx, id).is_none() {
        return;
    }
    // A frame drawn as egui draws a focused field's: its fill the fields'
    // and its border the selection's stroke (a field with no frame, a
    // code editor's, has none).
    let selection = style.visuals.selection.stroke;
    let fill = style.visuals.text_edit_bg_color();
    let mut layers: Vec<egui::LayerId> = ctx.memory(|m| m.layer_ids().collect());
    layers.push(egui::LayerId::background());
    for layer in layers {
        shapes_where(
            ctx,
            layer,
            |r| r.stroke == selection && r.fill == fill,
            |r| {
                r.stroke.color = t.accent;
            },
        );
    }
}

fn tooltip_fill(ctx: &egui::Context, t: &Tokens, style: &Style) {
    let layers: Vec<egui::LayerId> =
        ctx.memory(|m| m.layer_ids().filter(|l| l.order == egui::Order::Tooltip).collect());
    let (fill, stroke) = (style.visuals.window_fill, style.visuals.window_stroke);
    for layer in layers {
        // (The frame: the popup's fill and border.)
        shapes_where(
            ctx,
            layer,
            |r| r.fill == fill && r.stroke == stroke,
            |r| r.fill = t.tooltip_bg,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_box_s_colors_follow_its_state() {
        let t = Tokens::DARK;
        let look = |on, hovered, enabled| Look { on, hovered, focused: false, enabled };
        assert_eq!(look(true, false, true).colors(&t), (t.accent_fill, t.accent_fill, t.on_fill));
        assert_eq!(look(true, true, true).colors(&t).0, t.accent_fill_hover);
        assert_eq!(look(false, false, true).colors(&t), (t.inset, t.text_3, t.on_fill));
        assert_eq!(look(false, true, true).colors(&t).1, t.text_2);
        assert_eq!(look(true, false, false).colors(&t).0, t.accent_fill_off);
        assert_eq!(look(false, false, false).colors(&t).1, t.text_off);
    }
}
