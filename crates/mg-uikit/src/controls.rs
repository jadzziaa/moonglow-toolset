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
//!   [`row_style`] (a list's, tree's or table's rows: 22 px apart with no
//!   gap, square fills that meet, hovered `hover-row`), which a container
//!   opts into with [`list_rows`] or [`with_rows`].
//! - A form shown only to be looked at: [`read_only`], which the check
//!   box, radio button and slider honor too.
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
pub(crate) struct Look {
    on: bool,
    hovered: bool,
    focused: bool,
    enabled: bool,
    /// In a form shown only to be looked at ([`read_only`]).
    read_only: bool,
}

impl Look {
    pub(crate) fn of(ui: &Ui, response: &Response, on: bool, enabled: bool) -> Self {
        let read_only = is_read_only(ui);
        Look {
            on,
            // (Read only: quiet, the same under the pointer.)
            hovered: response.hovered() && !read_only,
            focused: response.has_focus(),
            enabled,
            read_only,
        }
    }

    /// Fill, border and mark.
    fn colors(self, t: &Tokens) -> (Color32, Color32, Color32) {
        if self.read_only && self.enabled {
            // (Quiet: on, a `text-3` box with a mark in the panel's
            // color; off, a `text-3` outline with nothing in it.)
            return match self.on {
                true => (t.text_3, t.text_3, t.panel),
                false => (Color32::TRANSPARENT, t.text_3, t.panel),
            };
        }
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

/// The painter a control's box is drawn with: where the control is
/// disabled, without the fade egui gives a disabled widget (the disabled
/// colors are the tokens' own, not a fade of the enabled ones), but still
/// with any opacity a parent gave (a fading window's).
fn box_painter(ui: &Ui, enabled: bool) -> egui::Painter {
    let mut painter = ui.painter().clone();
    if !enabled {
        // (egui multiplies the disabled fade into the parent's opacity:
        // taken out again, the parent's is left.)
        let fade = ui.visuals().disabled_alpha();
        if fade > 0.0 {
            painter.set_opacity((painter.opacity() / fade).min(1.0));
        }
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

/// A segmented control: the mock's `.seg`, one of several choices side
/// by side, 1 point apart, the outer corners rounded (Options › General ›
/// Open editors). Each choice is drawn `raised` with its words in
/// `text-2` (`raised-hover` and `text` under the pointer), the one chosen
/// `accent-fill` with `on-fill` words; the keyboard's focus is an `accent`
/// ring. `choices` are each a value, its words (its name to a screen
/// reader, as a radio button, selected where chosen) and its tooltip
/// (empty: none). A click, or Space or Enter with the keyboard on it,
/// chooses it. The whole control's response, changed where a choice was
/// made. Where no theme was applied, egui's selectable labels.
pub fn segmented<T: PartialEq + Copy>(
    ui: &mut Ui,
    value: &mut T,
    choices: &[(T, &str, &str)],
) -> Response {
    let theme = theme::applied(ui.ctx());
    let mut changed = false;
    let mut inner = ui.horizontal(|ui| {
        if theme.is_some() {
            ui.spacing_mut().item_spacing.x = 1.0;
        }
        for (i, (choice, words, tip)) in choices.iter().enumerate() {
            let on = *value == *choice;
            let ends = (i == 0, i + 1 == choices.len());
            let mut response = match theme {
                Some(theme) => segment(ui, &theme.tokens(), on, words, ends),
                None => ui.selectable_label(on, *words),
            };
            if !tip.is_empty() {
                response = response.on_hover_text(*tip);
            }
            if response.clicked() && !on {
                *value = *choice;
                changed = true;
            }
        }
    });
    if changed {
        inner.response.mark_changed();
    }
    inner.response
}

/// One choice of a [`segmented`] control; `ends` says whether it is the
/// first and the last (their outer corners rounded).
fn segment(ui: &mut Ui, t: &Tokens, on: bool, words: &str, ends: (bool, bool)) -> Response {
    let galley = egui::WidgetText::from(words).into_galley(
        ui,
        Some(egui::TextWrapMode::Extend),
        f32::INFINITY,
        egui::TextStyle::Button,
    );
    let height = metrics::CONTROL_HEIGHT.max(ui.spacing().interact_size.y);
    let size = vec2(galley.size().x + 2.0 * metrics::CONTROL_PADDING_X, height);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    let enabled = ui.is_enabled();
    response.widget_info(|| WidgetInfo::selected(WidgetType::RadioButton, enabled, on, words));
    if !ui.is_rect_visible(rect) {
        return response;
    }
    let hovered = response.hovered() && enabled && !is_read_only(ui);
    let (fill, text) = match (on, hovered, enabled) {
        (true, _, true) => (t.accent_fill, t.on_fill),
        (true, _, false) => (t.accent_fill_off, t.on_fill_off),
        (false, true, _) => (t.raised_hover, t.text),
        (false, false, true) => (t.raised, t.text_2),
        (false, false, false) => (t.raised, t.text_off),
    };
    let r = |rounded: bool| if rounded { metrics::RADIUS } else { 0 };
    let (first, last) = ends;
    let corners = egui::CornerRadius { nw: r(first), sw: r(first), ne: r(last), se: r(last) };
    let painter = box_painter(ui, enabled);
    painter.rect_filled(rect, corners, fill);
    let at = rect.center() - galley.size() / 2.0;
    painter.galley(at, galley, text);
    if response.has_focus() {
        let stroke = Stroke::new(1.0, t.accent);
        painter.rect_stroke(
            rect.expand(2.0),
            metrics::RADIUS + 2,
            stroke,
            egui::StrokeKind::Inside,
        );
    }
    response
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
        let look = Look::of(ui, &prepared.response, *checked || indeterminate, enabled);
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
        let look = Look::of(ui, &prepared.response, checked, enabled);
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

/// A check box's box in a menu's row that may be lit (`accent-fill`
/// under it): there, ringed in `on-fill`, so that a box filled
/// `accent-fill` still shows on the row's fill.
pub(crate) fn paint_check_lit(ui: &Ui, t: &Tokens, square: Rect, look: Look, lit: bool) {
    let painter = box_painter(ui, look.enabled);
    paint_check(&painter, t, square, look, false);
    if lit {
        let stroke = Stroke::new(1.0, t.on_fill);
        painter.rect_stroke(square, metrics::RADIUS, stroke, egui::StrokeKind::Inside);
    }
}

/// A radio's circle in a menu's row that may be lit (as
/// [`paint_check_lit`]).
pub(crate) fn paint_radio_lit(ui: &Ui, t: &Tokens, center: Pos2, look: Look, lit: bool) {
    let painter = box_painter(ui, look.enabled);
    paint_radio(&painter, t, center, look);
    if lit {
        painter.circle_stroke(center, BOX / 2.0 - 0.5, Stroke::new(1.0, t.on_fill));
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

/// A code editor's frame (the mock's `.code`: no border, focused or not,
/// on the ground drawn under it): `TextEdit::frame(code_frame())`, with
/// egui's own margins for a text field (4 by 2). A `TextEdit` given a
/// frame has egui draw neither fill nor border; one given only
/// `background_color` is framed in the selection's stroke (white) while
/// it has the keys.
pub fn code_frame() -> egui::Frame {
    egui::Frame::new().inner_margin(egui::Margin::symmetric(4, 2))
}

/// A slider in the theme (the mock's `.slider`): egui's [`egui::Slider`]
/// (its drag, keys, value field and accessible value are egui's), drawn in
/// a scope where its rail is an `inset` well, the part up to its thumb
/// `accent-fill`, and its thumb ringed in `text-2`: at rest filled as the
/// rail (egui fills the rail and the resting thumb alike), under the
/// pointer or held `slider-thumb`; [read only](read_only), quiet in
/// `text-3` and the same under the pointer. `slider(ui, Slider::new(&mut v, 0..=100))`
/// for `ui.add(..)`; egui's own where no theme was applied.
pub fn slider(ui: &mut Ui, slider: egui::Slider<'_>) -> Response {
    let Some(theme) = theme::applied(ui.ctx()) else { return ui.add(slider) };
    let t = theme.tokens();
    let read_only = is_read_only(ui);
    ui.scope(|ui| {
        let v = ui.visuals_mut();
        let w = &mut v.widgets;
        if read_only {
            // Quiet, and the same under the pointer: the part up to the
            // thumb and the thumb `text-3` on a well of the panel's color.
            v.selection.bg_fill = t.text_3;
            let w = &mut v.widgets;
            w.inactive.bg_fill = t.panel;
            w.hovered = w.inactive;
            w.active = w.inactive;
            for state in [&mut w.inactive, &mut w.hovered, &mut w.active] {
                state.fg_stroke = Stroke::new(1.0, t.text_3);
            }
            return ui.add(slider);
        }
        // (The rail is the resting state's fill; the thumb each state's
        // fill, ringed in its text stroke, which is the value's text too.)
        w.inactive.bg_fill = t.inset;
        w.hovered.bg_fill = t.slider_thumb;
        w.active.bg_fill = t.slider_thumb;
        for state in [&mut w.inactive, &mut w.hovered, &mut w.active] {
            state.fg_stroke = Stroke::new(1.0, t.text_2);
        }
        v.selection.bg_fill = t.accent_fill;
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
        ui.painter().add(Shape::convex_polygon(points, visuals.fg_stroke.color, Stroke::NONE));
        return;
    };
    let t = theme.tokens();
    paint_chevron(ui.painter(), rect.center(), 1.0, t.text_2);
}

/// Whether `style` is one the theme made ([`crate::theme::style`], or
/// one changed from it): its window and selection fills are the
/// tokens'. The style functions below change only such a style, as the
/// `ui` ones change only a context the theme was applied to, so that a
/// context without the suite's look keeps egui's.
fn themed(style: &Style) -> bool {
    let t = tokens_of(style);
    style.visuals.window_fill == t.panel && style.visuals.selection.bg_fill == t.accent_fill
}

/// A menu's style (egui's [`egui::containers::menu::menu_style`], with
/// the mock's `.menu-row`): rows [`metrics::MENU_ROW`] high with no gap
/// between them, hovered or open in `accent-fill` with their words in
/// `on-fill`, 8 px at each end; their fills drawn on the rows' own
/// rectangles, square, so that a lit row meets its neighbors with neither
/// overlap nor seam (egui's 1 px expansion, and a border, reached a pixel
/// into the row beside). The menu's popover padded 4 px above and below
/// and not at the sides ([`crate::menu::MENU_MARGIN`], `.menu-pop`), so
/// that a lit row's fill reaches its edges: a popup's frame is made from
/// the style its popup is given, so this is for `MenuConfig::style` and
/// `Popup::style` (as [`crate::context_menu`] and
/// [`crate::IconButton::menu`] give it); [`menu_rows`] gives the rows the
/// rest inside a menu whose style egui fixed. Egui's own menu style where
/// the style is not the theme's.
pub fn menu_style(style: &mut Style) {
    egui::containers::menu::menu_style(style);
    if !themed(style) {
        return;
    }
    let t = tokens_of(style);
    style.spacing.button_padding = vec2(metrics::CONTROL_PADDING_X, 0.0);
    style.spacing.menu_margin = crate::menu::MENU_MARGIN;
    style.spacing.item_spacing.y = 0.0;
    // (Taller where a window made its controls taller with its text.)
    style.spacing.interact_size.y = style.spacing.interact_size.y.max(metrics::MENU_ROW);
    let w = &mut style.visuals.widgets;
    for state in [&mut w.hovered, &mut w.active, &mut w.open] {
        state.weak_bg_fill = t.accent_fill;
        state.bg_fill = t.accent_fill;
        state.fg_stroke.color = t.on_fill;
    }
    // (As `row_style`: no expansion, and so no border either, which egui
    // takes from the padding and gives back by the expansion.)
    for state in [&mut w.inactive, &mut w.hovered, &mut w.active, &mut w.open] {
        state.expansion = 0.0;
        state.bg_stroke = Stroke::new(0.0, Color32::TRANSPARENT);
        state.corner_radius = egui::CornerRadius::ZERO;
    }
    style.visuals.selection.stroke.width = 0.0;
}

/// The menu's style ([`menu_style`]) given to the rows that follow in
/// `ui` (a context menu's, whose style egui fixes), where the theme was
/// applied.
pub fn menu_rows(ui: &mut Ui) {
    if theme::applied(ui.ctx()).is_some() {
        menu_style(ui.style_mut());
    }
}

/// A menu bar's style: its titles hovered in `raised`, open in
/// `accent-fill` (the mock's `.menu`). Egui's own menu style where the
/// style is not the theme's.
pub fn menu_bar_style(style: &mut Style) {
    egui::containers::menu::menu_style(style);
    if !themed(style) {
        return;
    }
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

/// The rows of a list, a tree or a table (the mock's `.tree-row`,
/// `.list-row`, `.table td`): each [`metrics::ROW_HEIGHT`] high with no
/// gap between them (`item_spacing.y` 0, a control as high as a row), and
/// their fills meeting edge to edge: square, drawn on the row's own
/// rectangle (no expansion: egui's 1 px past it made a hovered row
/// overlap the selected one beside it), hovered in `hover-row` rather
/// than a button's `raised-hover`, no border. Selected rows keep
/// `accent-fill`. A control in a row (a check box, a table's text field)
/// is as high as the row; a style whose controls are taller (made so
/// with its text) keeps them. Unchanged where the style is not the
/// theme's.
pub fn row_style(style: &mut Style) {
    if !themed(style) {
        return;
    }
    let t = tokens_of(style);
    style.spacing.item_spacing.y = 0.0;
    // (Taller where a window made its controls taller with its text: the
    // conversation editor's own text size.)
    style.spacing.interact_size.y = style.spacing.interact_size.y.max(metrics::ROW_HEIGHT);
    let w = &mut style.visuals.widgets;
    w.hovered.weak_bg_fill = t.hover_row;
    w.active.weak_bg_fill = t.hover_row;
    // No expansion, and so no border either: egui gives a button the
    // expansion less the border's width back in its padding where it
    // draws no frame (a row at rest), which kept a row as wide as hovered
    // only while the two were the same. A text field in a table is its
    // well (the mock's `.table td` has no border of its own); with the
    // keys, the pass `apply` installs rings it in `accent` (the
    // selection's stroke, which egui frames it in, is as thin: its text
    // stays where it was).
    for state in [&mut w.inactive, &mut w.hovered, &mut w.active, &mut w.open] {
        state.expansion = 0.0;
        state.bg_stroke = Stroke::new(0.0, Color32::TRANSPARENT);
        state.corner_radius = egui::CornerRadius::ZERO;
    }
    style.visuals.selection.stroke.width = 0.0;
}

/// [`row_style`] given to the rows that follow in `ui`, where the theme
/// was applied: from here on in `ui` (a scroll area's contents, a
/// popup's), rows are [`metrics::ROW_HEIGHT`] apart.
pub fn list_rows(ui: &mut Ui) {
    if theme::applied(ui.ctx()).is_some() {
        row_style(ui.style_mut());
    }
}

/// `add` in a scope of `ui` given [`list_rows`]: rows in a part of a
/// window that has more than rows (a form above them, buttons under),
/// and rows whose pitch egui takes from the `ui` they are put in
/// (`ScrollArea::show_rows` reads the gap between rows there). The scope
/// itself is followed by `ui`'s own gap, as any widget is (the style
/// changed and changed back in `ui` itself left none under the rows).
/// `row_height(ui)` inside is the rows' height and pitch.
pub fn with_rows<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
    ui.scope(|ui| {
        list_rows(ui);
        add(ui)
    })
    .inner
}

/// [`list_rows`] for a table whose rows hold fields (text fields, combo
/// boxes, number fields): the rows [`metrics::TABLE_RULE`] apart, the
/// panel's line between their wells.
pub fn table_rows(ui: &mut Ui) {
    if theme::applied(ui.ctx()).is_some() {
        row_style(ui.style_mut());
        ui.spacing_mut().item_spacing.y = metrics::TABLE_RULE;
    }
}

/// [`with_rows`] for a table whose rows hold fields ([`table_rows`]).
pub fn with_table_rows<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
    ui.scope(|ui| {
        table_rows(ui);
        add(ui)
    })
    .inner
}

/// The height of a row in `ui` (a control's, `interact_size.y`): in
/// [`list_rows`]'s rows, [`metrics::ROW_HEIGHT`], which is also their
/// pitch.
pub fn row_height(ui: &Ui) -> f32 {
    ui.spacing().interact_size.y
}

/// The rows' pitch in `ui`: a row's height and the gap under it.
pub fn row_pitch(ui: &Ui) -> f32 {
    row_height(ui) + ui.spacing().item_spacing.y
}

/// The height and pitch rows will have in `ui` once [`list_rows`] is
/// given them (where the theme was applied, its control height, at least
/// [`metrics::ROW_HEIGHT`], both; else egui's control height, and that
/// and its gap), for a scroll offset worked out before they are drawn.
pub fn row_metrics(ui: &Ui) -> (f32, f32) {
    match theme::applied(ui.ctx()) {
        Some(_) => {
            let height = row_height(ui).max(metrics::ROW_HEIGHT);
            (height, height)
        }
        None => (row_height(ui), row_pitch(ui)),
    }
}

/// `area.show_rows` for rows of [`list_rows`]: each [`row_height`] high
/// and that far apart, the rows' style given to `ui` for it (egui's
/// `show_rows` reads the gap between rows in the `ui` it is given, not in
/// the rows'). Every row `add` draws must be as high as one: a row of
/// controls, a selectable label, a check box.
pub fn show_rows<R>(
    area: egui::ScrollArea,
    ui: &mut Ui,
    total: usize,
    add: impl FnOnce(&mut Ui, std::ops::Range<usize>) -> R,
) -> egui::scroll_area::ScrollAreaOutput<R> {
    with_rows(ui, |ui| {
        let height = row_height(ui);
        area.show_rows(ui, height, total, add)
    })
}

/// The tag a [`read_only`] form's `Ui` carries.
const READ_ONLY: &str = "mg-uikit-read-only";

/// `add` drawn as a form shown only to be looked at (the mock's
/// `.field.ro`, `.ro-val`): quiet, in a child `Ui` whose controls have no
/// fill (the panel's), a quiet border (`line-soft`), their words in
/// `text-2`, and look the same under the pointer; the crate's check box,
/// radio button and slider are quiet too ([`is_read_only`]). The controls
/// still take clicks and keys: what they change is for the caller to keep
/// out. Without the theme, egui's look.
pub fn read_only<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
    let info = egui::UiStackInfo::default().with_tag(READ_ONLY);
    ui.scope_builder(egui::UiBuilder::new().ui_stack_info(info), |ui| {
        if let Some(theme) = theme::applied(ui.ctx()) {
            read_only_visuals(ui.visuals_mut(), &theme.tokens());
        }
        add(ui)
    })
    .inner
}

/// The visuals [`read_only`] gives its controls.
fn read_only_visuals(v: &mut egui::Visuals, t: &Tokens) {
    let mut quiet = v.widgets.inactive;
    quiet.bg_fill = Color32::TRANSPARENT;
    quiet.weak_bg_fill = Color32::TRANSPARENT;
    quiet.bg_stroke = v.widgets.noninteractive.bg_stroke;
    quiet.fg_stroke = Stroke::new(1.0, t.text_2);
    quiet.expansion = 0.0;
    // (No hover: every state looks as the resting one.)
    for w in [&mut v.widgets.inactive, &mut v.widgets.hovered, &mut v.widgets.active] {
        *w = quiet;
    }
    v.extreme_bg_color = v.panel_fill;
    v.text_edit_bg_color = Some(v.panel_fill);
}

/// Whether `ui` is in a [`read_only`] form.
pub fn is_read_only(ui: &Ui) -> bool {
    ui.stack().iter().any(|frame| frame.tags().contains(READ_ONLY))
}

/// The pass [`apply`](crate::apply) installs at the end of each of a
/// context's passes, for what egui's style cannot say:
///
/// - the focused text field's frame turned `accent` (egui draws it in
///   the selection's stroke, which is the color of selected text too).
///   Only a frame egui drew itself, with the fields' fill (or a read-only
///   form's, the panel's) and in that stroke, is changed. A field with a
///   fill of its own (`TextEdit::background_color`) would keep egui's
///   white: such a field takes a frame of its own instead, as a code
///   editor does ([`code_frame`]: no border at all, the mock's `.code`);
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
    // and its border the selection's stroke. (A field with a frame of its
    // own, `TextEdit::frame`, has egui draw neither: a code editor's,
    // [`code_frame`].)
    // (A field's own fill, or a read-only form's: the panel's. In a
    // table's rows, [`row_style`], the selection's stroke is no wider than
    // their fields' borders, none: given the field's 1 px.)
    let selection = style.visuals.selection.stroke.color;
    let fills = [style.visuals.text_edit_bg_color(), style.visuals.panel_fill];
    let mut layers: Vec<egui::LayerId> = ctx.memory(|m| m.layer_ids().collect());
    layers.push(egui::LayerId::background());
    for layer in layers {
        shapes_where(
            ctx,
            layer,
            |r| r.stroke.color == selection && fills.contains(&r.fill),
            |r| {
                r.stroke = Stroke::new(1.0, t.accent);
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
        let look =
            |on, hovered, enabled| Look { on, hovered, focused: false, enabled, read_only: false };
        assert_eq!(look(true, false, true).colors(&t), (t.accent_fill, t.accent_fill, t.on_fill));
        assert_eq!(look(true, true, true).colors(&t).0, t.accent_fill_hover);
        assert_eq!(look(false, false, true).colors(&t), (t.inset, t.text_3, t.on_fill));
        assert_eq!(look(false, true, true).colors(&t).1, t.text_2);
        assert_eq!(look(true, false, false).colors(&t).0, t.accent_fill_off);
        assert_eq!(look(false, false, false).colors(&t).1, t.text_off);
        // Read only: quiet, on or off, never the accent.
        let quiet = |on| Look { read_only: true, ..look(on, false, true) };
        assert_eq!(quiet(true).colors(&t), (t.text_3, t.text_3, t.panel));
        assert_eq!(quiet(false).colors(&t).1, t.text_3);
    }

    /// One frame of `add` in a context with the theme (or none).
    fn frame(theme: Option<crate::Theme>, add: impl FnMut(&mut Ui)) -> egui::FullOutput {
        let ctx = egui::Context::default();
        if let Some(theme) = theme {
            crate::apply(&ctx, theme);
            // (The style is the theme's from the next pass.)
            ctx.run_ui(egui::RawInput::default(), |_| {}).textures_delta.clear();
        }
        let mut out = ctx.run_ui(egui::RawInput::default(), add);
        // (Its textures are not taken by a renderer: let go of them.)
        out.textures_delta.clear();
        out
    }

    /// Rows after `list_rows` are 22 apart with no gap, whatever is in
    /// them, and their fills drawn on their own rectangles, square.
    #[test]
    fn rows_are_22_apart_and_their_fills_meet() {
        for theme in crate::Theme::ALL {
            let mut tops = Vec::new();
            let out = frame(Some(theme), |ui| {
                list_rows(ui);
                for i in 0..4 {
                    let r = match i {
                        0 => ui.selectable_label(true, "Selected"),
                        1 => ui.selectable_label(false, "Plain"),
                        2 => ui.horizontal(|ui| ui.checkbox(&mut true, "Check")).response,
                        _ => egui::CollapsingHeader::new("Header").show(ui, |_| {}).header_response,
                    };
                    tops.push(r.rect);
                }
            });
            for w in tops.windows(2) {
                assert_eq!(w[1].top() - w[0].top(), metrics::ROW_HEIGHT, "{theme:?} {tops:?}");
                assert_eq!(w[0].height(), metrics::ROW_HEIGHT, "{theme:?} {tops:?}");
            }
            // The selected row's fill: its own rectangle, square.
            let t = theme.tokens();
            let fill = out
                .shapes
                .iter()
                .find_map(|c| match &c.shape {
                    Shape::Rect(r) if r.fill == t.accent_fill => Some(r.clone()),
                    _ => None,
                })
                .expect("the selected row's fill");
            assert_eq!(fill.rect.y_range(), tops[0].y_range(), "{theme:?}");
            assert_eq!(fill.corner_radius, egui::CornerRadius::ZERO);
        }
    }

    /// A menu's rows are 24 apart with no gap (the mock's `.menu-row`;
    /// they were 22 and a 4 px gap), a hovered row's fill exactly its own
    /// rectangle, square, and a separator between rows still a line.
    #[test]
    fn menu_rows_are_24_apart_and_their_fills_meet() {
        for theme in crate::Theme::ALL {
            let t = theme.tokens();
            let mut rects = Vec::new();
            let out = frame(Some(theme), |ui| {
                ui.with_layout(egui::Layout::top_down_justified(egui::Align::Min), |ui| {
                    menu_rows(ui);
                    assert_eq!(row_pitch(ui), metrics::MENU_ROW, "{theme:?}");
                    rects.push(ui.button("Undo").rect);
                    // (Drawn as hovered: the style's hovered state at rest.)
                    ui.scope(|ui| {
                        let w = &mut ui.visuals_mut().widgets;
                        w.inactive = w.hovered;
                        rects
                            .push(ui.add(egui::Button::new("Redo").frame_when_inactive(true)).rect);
                    });
                    rects.push(ui.add(egui::Button::new("Cut").selected(true)).rect);
                    ui.separator();
                    rects.push(ui.button("Paste").rect);
                });
            });
            assert_eq!(rects[0].height(), metrics::MENU_ROW, "{theme:?}");
            assert_eq!(rects[1].top() - rects[0].top(), metrics::MENU_ROW);
            assert_eq!(rects[2].top() - rects[1].top(), metrics::MENU_ROW);
            // (The separator's own 6 between the rows, no gap beside it.)
            assert_eq!(rects[3].top() - rects[2].bottom(), 6.0, "{rects:?}");
            let fills: Vec<egui::epaint::RectShape> = out
                .shapes
                .iter()
                .filter_map(|c| match &c.shape {
                    Shape::Rect(r) if r.fill == t.accent_fill => Some(r.clone()),
                    _ => None,
                })
                .collect();
            assert_eq!(fills.len(), 2, "the hovered and the selected rows' fills: {fills:?}");
            for (fill, row) in fills.iter().zip([rects[1], rects[2]]) {
                assert_eq!(fill.rect, row, "{theme:?}: a fill on its row alone");
                assert_eq!(fill.corner_radius, egui::CornerRadius::ZERO);
                assert!(fill.stroke.width == 0.0 || fill.stroke.color.a() == 0);
            }
        }
    }

    /// A menu holding a form keeps a control's height and the mock's
    /// `.pop-body` gap (6) between its parts, its controls a form's (the
    /// window's style: a button framed at rest, not a menu's flat row);
    /// where the window made its controls taller with its text (the
    /// context's style while the menu is drawn), theirs.
    #[test]
    fn a_menu_s_form_keeps_its_gaps() {
        let _ = frame(Some(crate::Theme::Dark), |ui| {
            let window = ui.ctx().global_style();
            menu_style(ui.style_mut());
            crate::menu_form(ui, |ui| {
                assert_eq!(ui.spacing().item_spacing.y, crate::menu::MENU_FORM_GAP);
                assert_eq!(ui.spacing().interact_size.y, metrics::CONTROL_HEIGHT);
                let t = crate::Theme::Dark.tokens();
                let w = &ui.visuals().widgets;
                assert_eq!(w.inactive.weak_bg_fill, window.visuals.widgets.inactive.weak_bg_fill);
                assert_ne!(w.inactive.weak_bg_fill, egui::Color32::TRANSPARENT);
                assert_ne!(w.hovered.weak_bg_fill, t.accent_fill, "a form's button, not a row");
            });
        });
        let _ = frame(None, |ui| {
            let before = ui.style().clone();
            crate::menu_form(ui, |ui| {
                assert_eq!(*ui.style(), before, "egui's own without the theme");
            });
        });
    }

    /// A menu's form under a window's larger text (Options › Conversation
    /// Editor's size: the context's style is the window's while it is
    /// drawn) keeps the window's taller controls, not a control's 22.
    #[test]
    fn a_menu_s_form_keeps_a_larger_text_s_height() {
        let ctx = egui::Context::default();
        crate::apply(&ctx, crate::Theme::Dark);
        ctx.run_ui(egui::RawInput::default(), |_| {}).textures_delta.clear();
        let tall = 34.0;
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
            ui.ctx().global_style_mut(|s| s.spacing.interact_size.y = tall);
            ui.spacing_mut().interact_size.y = tall;
            menu_style(ui.style_mut());
            crate::menu_form(ui, |ui| {
                assert_eq!(ui.spacing().interact_size.y, tall);
                // (A button as tall; a field takes its height from its
                // text, which a larger text makes taller itself.)
                let button = ui.button("Paste").rect;
                assert_eq!(button.height(), tall, "{button:?}");
            });
        });
        out.textures_delta.clear();
    }

    /// A menu bar's titles keep the mock's 22 px (`.menu`), its rows'
    /// 24 not reaching them.
    #[test]
    fn menu_titles_stay_22_high() {
        let mut style = crate::theme::style(crate::Theme::Dark, &Style::default());
        menu_bar_style(&mut style);
        assert_eq!(style.spacing.interact_size.y, metrics::MENU_TITLE);
        let mut rows = crate::theme::style(crate::Theme::Dark, &Style::default());
        menu_style(&mut rows);
        assert_eq!(rows.spacing.interact_size.y, metrics::MENU_ROW);
        assert_eq!(rows.spacing.item_spacing.y, 0.0);
    }

    /// A table of fields: its rows 22 high, a 1 px line of the panel
    /// between them.
    #[test]
    fn a_table_s_rows_of_fields_have_a_line_between() {
        let _ = frame(Some(crate::Theme::Dark), |ui| {
            let tops = with_table_rows(ui, |ui| {
                let a = ui.add(text_field(&mut String::from("a"))).rect;
                let b = ui.add(text_field(&mut String::from("b"))).rect;
                (a, b)
            });
            assert_eq!(tops.0.height(), metrics::ROW_HEIGHT);
            assert_eq!(tops.1.top() - tops.0.bottom(), metrics::TABLE_RULE);
        });
    }

    /// A row is as wide selected (framed) as at rest (not): egui gives an
    /// unframed button its expansion less its border back, so the two
    /// must be equal (they are both nothing in rows).
    #[test]
    fn a_row_keeps_its_width_selected() {
        let mut widths = Vec::new();
        let _ = frame(Some(crate::Theme::Dark), |ui| {
            list_rows(ui);
            widths.push(ui.selectable_label(false, "WP_03").rect.width());
            widths.push(ui.selectable_label(true, "WP_03").rect.width());
        });
        assert_eq!(widths[0], widths[1]);
    }

    /// Without the theme, rows and menus keep egui's own style.
    #[test]
    fn without_the_theme_rows_and_menus_are_egui_s() {
        let _ = frame(None, |ui| {
            let before = ui.style().clone();
            list_rows(ui);
            menu_rows(ui);
            assert_eq!(*ui.style(), before);
            let mut style = (*before).clone();
            row_style(&mut style);
            assert_eq!(style, *before);
        });
        let mut themed = crate::theme::style(crate::Theme::Light, &Style::default());
        row_style(&mut themed);
        assert_eq!(themed.spacing.item_spacing.y, 0.0);
        assert_eq!(themed.visuals.widgets.hovered.expansion, 0.0);
    }

    /// `with_rows` gives the rows their style and leaves the rest its own,
    /// the gap under the rows included.
    #[test]
    fn with_rows_is_scoped() {
        let _ = frame(Some(crate::Theme::Dark), |ui| {
            let gap = ui.spacing().item_spacing.y;
            let (pitch, bottom) = with_rows(ui, |ui| {
                let r = ui.selectable_label(false, "a");
                let _ = ui.selectable_label(false, "b");
                (row_pitch(ui), r.rect.bottom() + metrics::ROW_HEIGHT)
            });
            assert_eq!(pitch, metrics::ROW_HEIGHT);
            assert_eq!(ui.spacing().item_spacing.y, gap);
            assert_eq!(row_pitch(ui), metrics::CONTROL_HEIGHT + gap);
            let next = ui.label("after").rect.top();
            assert_eq!(next, bottom + gap, "the gap under the rows");
        });
    }

    /// A read-only form's `Ui`, and those in it, know it; its controls
    /// are quiet and the same under the pointer; outside it, nothing
    /// changes.
    #[test]
    fn read_only_is_known_inside_and_quiet() {
        for theme in crate::Theme::ALL {
            let t = theme.tokens();
            let _ = frame(Some(theme), |ui| {
                assert!(!is_read_only(ui));
                read_only(ui, |ui| {
                    assert!(is_read_only(ui));
                    ui.horizontal(|ui| assert!(is_read_only(ui)));
                    let w = &ui.visuals().widgets;
                    assert_eq!(w.hovered, w.inactive);
                    assert_eq!(w.inactive.weak_bg_fill, Color32::TRANSPARENT);
                    assert_eq!(w.inactive.fg_stroke.color, t.text_2);
                    assert_eq!(ui.visuals().text_edit_bg_color(), t.panel);
                });
                assert!(!is_read_only(ui));
                assert_ne!(ui.visuals().widgets.inactive.weak_bg_fill, Color32::TRANSPARENT);
            });
        }
    }

    /// A disabled box is drawn without egui's fade but keeps a parent's
    /// opacity.
    #[test]
    fn a_disabled_box_keeps_a_parent_s_opacity() {
        let _ = frame(Some(crate::Theme::Dark), |ui| {
            ui.multiply_opacity(0.5);
            ui.add_enabled_ui(false, |ui| {
                let painter = box_painter(ui, false);
                assert!((painter.opacity() - 0.5).abs() < 1e-4, "{}", painter.opacity());
            });
            assert!((box_painter(ui, true).opacity() - 0.5).abs() < 1e-4);
        });
    }
}
