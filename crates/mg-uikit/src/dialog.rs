//! The parts of a dialog and of a pane with nothing in it (the
//! proposal's M4, Part 2, decision 1): the empty state ([`EmptyState`],
//! the mock's `.empty`), which the toolset's "No area open" is (M4's
//! U24), and the dialog's footer ([`Footer`], the boards' `.dlg-foot`,
//! M4's U26: the pickers'; the wizards' and the other dialogs' with U27),
//! the body's room ([`BODY_PADDING`]), and a wizard's step list
//! ([`StepList`]) and page bar ([`page_bar`]) (U27).

use egui::{Align2, FontId, Rect, Response, Ui, pos2, vec2};

use crate::button::KitButton;
use crate::icons::Icon;
use crate::metrics;
use crate::theme::TokensExt as _;

/// The room around an empty state (`.empty { padding: 24px }`).
pub const EMPTY_PADDING: f32 = 24.0;
/// The room between its parts (`.empty { gap: 8px }`).
pub const EMPTY_GAP: f32 = 8.0;
/// The room over its buttons beyond the gap (`.empty .btn { margin-top:
/// 4px }`).
pub const EMPTY_BUTTON_ROOM: f32 = 4.0;
/// The widest its sentence runs before it wraps.
pub const EMPTY_WIDEST: f32 = 420.0;
/// Its icon's side (`.empty .ic`: 32 px, `.ic-xl`).
pub const EMPTY_ICON: f32 = 32.0;

/// A pane or an editor with nothing in it (`.empty`): its kind's icon
/// large in `text-3`, what it is in `text-2`, a sentence in `text-3`
/// saying how to fill it, and the buttons that do, in the middle of the
/// room it is given, wherever that is. Laid out from what it holds and the
/// room alone (never from what an earlier frame measured), so it stands
/// still; where its buttons do not fit side by side they stand one above
/// the other, none cut.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use mg_uikit::dialog::EmptyState;
/// use mg_uikit::{Icon, KitButton};
/// let shown = EmptyState::new(Icon::Area, "No area open")
///     .why("Double-click an area in the Module panel.")
///     .button(KitButton::new(None, "Area Wizard…"))
///     .show(ui);
/// assert_eq!(shown.len(), 1);
/// # });
/// ```
#[derive(Clone, Debug)]
#[must_use = "show it with EmptyState::show"]
pub struct EmptyState {
    icon: Icon,
    what: String,
    why: Option<String>,
    buttons: Vec<KitButton>,
}

impl EmptyState {
    /// An empty state of `icon`'s kind, saying `what` it is.
    pub fn new(icon: Icon, what: impl Into<String>) -> Self {
        EmptyState { icon, what: what.into(), why: None, buttons: Vec::new() }
    }

    /// With a sentence under what it is (`.empty > span`): how to fill it.
    pub fn why(mut self, why: impl Into<String>) -> Self {
        self.why = Some(why.into());
        self
    }

    /// With `button` after those given before.
    pub fn button(mut self, button: KitButton) -> Self {
        self.buttons.push(button);
        self
    }

    /// Shows it in the middle of the room `ui` has left, and takes that
    /// room: the buttons' responses, in the order given.
    pub fn show(self, ui: &mut Ui) -> Vec<Response> {
        let t = ui.tokens();
        let room = ui.available_rect_before_wrap();
        let inner = (room.width() - 2.0 * EMPTY_PADDING).max(EMPTY_ICON);
        let wrap = inner.min(EMPTY_WIDEST);
        let body = FontId::proportional(metrics::BODY_SIZE);
        let what = ui.painter().layout(self.what.clone(), body.clone(), t.text_2, wrap);
        let why = (self.why.as_ref())
            .map(|w| ui.painter().layout(w.clone(), body.clone(), t.text_3, wrap));
        let widths: Vec<f32> = self.buttons.iter().map(|b| b.width(ui)).collect();
        let row = widths.iter().sum::<f32>() + EMPTY_GAP * widths.len().saturating_sub(1) as f32;
        // (Side by side where they fit, else one above the other.)
        let stacked = row > inner;
        let button_high = metrics::CONTROL_HEIGHT;
        let buttons_high = match (self.buttons.len(), stacked) {
            (0, _) => 0.0,
            (n, true) => n as f32 * button_high + (n - 1) as f32 * EMPTY_GAP,
            (_, false) => button_high,
        };
        let mut high = EMPTY_ICON + EMPTY_GAP + what.size().y;
        if let Some(why) = &why {
            high += EMPTY_GAP + why.size().y;
        }
        if !self.buttons.is_empty() {
            high += EMPTY_GAP + EMPTY_BUTTON_ROOM + buttons_high;
        }
        // (In a room of no set height, a scroll's, from its top.)
        let spare = if room.height().is_finite() { room.height() - high } else { 0.0 };
        let top = room.top() + (spare / 2.0).max(EMPTY_PADDING);
        let middle = room.center().x;
        let mut y = top;
        let icon = Rect::from_center_size(
            pos2(middle, y + EMPTY_ICON / 2.0),
            vec2(EMPTY_ICON, EMPTY_ICON),
        );
        if ui.is_rect_visible(icon) {
            self.icon.paint(ui.painter(), icon, t.text_3);
        }
        y += EMPTY_ICON + EMPTY_GAP;
        let words = |ui: &mut Ui, galley: std::sync::Arc<egui::Galley>, y: &mut f32| {
            let size = galley.size();
            let line = Rect::from_x_y_ranges(room.x_range(), *y..=*y + size.y);
            let at = Align2::CENTER_TOP.align_size_within_rect(size, line);
            ui.put(at, egui::Label::new(galley).selectable(false));
            *y += size.y;
        };
        words(ui, what, &mut y);
        if let Some(why) = why {
            y += EMPTY_GAP;
            words(ui, why, &mut y);
        }
        let mut shown = Vec::new();
        if !self.buttons.is_empty() {
            y += EMPTY_GAP + EMPTY_BUTTON_ROOM;
            let mut x = middle - row / 2.0;
            for (button, width) in self.buttons.into_iter().zip(widths) {
                let rect = match stacked {
                    true => {
                        let r = Rect::from_min_size(
                            pos2(middle - width / 2.0, y),
                            vec2(width, button_high),
                        );
                        y += button_high + EMPTY_GAP;
                        r
                    }
                    false => {
                        let r = Rect::from_min_size(pos2(x, y), vec2(width, button_high));
                        x += width + EMPTY_GAP;
                        r
                    }
                };
                shown.push(ui.put(rect, button));
            }
        }
        // (The room is the empty state's, and as far down as it reaches.)
        let bottom = top + high + EMPTY_PADDING;
        let bottom = if room.height().is_finite() { bottom.max(room.bottom()) } else { bottom };
        ui.advance_cursor_after_rect(Rect::from_min_max(room.min, pos2(room.right(), bottom)));
        shown
    }
}

/// The room round a dialog's body (`.dlg-body { padding: 12px }`, the
/// Dialogs board's "Body: padding 12, gap 8").
pub const BODY_PADDING: f32 = 12.0;

/// A wizard's step list's width (`.steps { width: 190px }`).
pub const STEP_LIST_WIDTH: f32 = 190.0;
/// The room over and under its rows (`.steps { padding: 8px 0 }`).
const STEP_LIST_PADDING: f32 = 8.0;
/// Its heading's room at the left and under it (`.steps .hd { padding: 0
/// 12px 6px }`) and its line (`line-height: 14px`).
const STEP_HEADING_LEFT: f32 = 12.0;
const STEP_HEADING_UNDER: f32 = 6.0;
const STEP_HEADING_LINE: f32 = 14.0;
/// A row's room at its left (`.steps .list-row { padding-left: 10px; gap:
/// 8px }`) beside its 16-point check or the room of one.
const STEP_ROW_LEFT: f32 = 10.0;
const STEP_ROW_GAP: f32 = 8.0;

/// A wizard's page bar's height (`.pg-h { height: 26px }`).
pub const PAGE_BAR_HEIGHT: f32 = 26.0;
/// Its room at its ends (`.pg-h { padding: 0 16px }`) and between its
/// icon and its title (`gap: 8px`).
const PAGE_BAR_PADDING: f32 = 16.0;
const PAGE_BAR_GAP: f32 = 8.0;
/// The room round a wizard's page under its bar (`.wpage .pbody {
/// padding: 12px 16px }`): above and below, at the sides.
pub const PAGE_PADDING: egui::Vec2 = vec2(16.0, 12.0);

/// Where a wizard's step stands ([`StepList`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepState {
    /// Passed: in `text`, a check in `ok` before it.
    Done,
    /// The page shown: the row in `accent-fill`, its words in `on-fill`.
    Current,
    /// Still to come: in `text-2`.
    ToCome,
    /// Not taken with the choices made (a page switched off): in
    /// `text-off`, kept in its place so that the list never changes under
    /// the pointer.
    Off,
}

/// A wizard's step list (`.steps`): [`STEP_LIST_WIDTH`] wide down its
/// room's height at the left of the page, on `panel` with a `line` at
/// its right, a heading in `text-3` ("Step 3 of 9") and a row a step
/// ([`StepState`]). It tells where the wizard is and is not clicked (the
/// board gives its rows no pointer and no hover): Back and Next move.
/// Each row is named for a screen reader by its place and its step
/// ("Step 3: Class and Level", apart from the dialog's title, which a
/// first step may share), the current one selected, a step not taken
/// disabled.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use mg_uikit::dialog::{StepList, StepState};
/// let shown = StepList::new("Step 2 of 3")
///     .step("Category", StepState::Done)
///     .step("Name", StepState::Current)
///     .step("Finish", StepState::ToCome)
///     .show(ui);
/// assert_eq!(shown.rows.len(), 3);
/// # });
/// ```
#[derive(Clone, Debug)]
#[must_use = "show it with StepList::show"]
pub struct StepList {
    heading: String,
    steps: Vec<(String, StepState)>,
}

/// What a [`StepList`] showed: its rect and each row's, in order.
#[derive(Clone, Debug)]
pub struct StepListShown {
    pub rect: Rect,
    pub rows: Vec<Rect>,
}

impl StepList {
    /// A step list under `heading` ("Step 3 of 9").
    pub fn new(heading: impl Into<String>) -> Self {
        StepList { heading: heading.into(), steps: Vec::new() }
    }

    /// With the step `name` in `state` after those given before.
    pub fn step(mut self, name: impl Into<String>, state: StepState) -> Self {
        self.steps.push((name.into(), state));
        self
    }

    /// Shows it at the left of the room `ui` has, down its height (or as
    /// tall as its rows where the room has no set height), and takes that
    /// strip: laid out from its steps and the room alone.
    pub fn show(self, ui: &mut Ui) -> StepListShown {
        let t = ui.tokens();
        let room = ui.available_rect_before_wrap();
        let rows_high = STEP_LIST_PADDING
            + STEP_HEADING_LINE
            + STEP_HEADING_UNDER
            + self.steps.len() as f32 * metrics::ROW_HEIGHT
            + STEP_LIST_PADDING;
        let high = if room.height().is_finite() { room.height().max(rows_high) } else { rows_high };
        let rect = Rect::from_min_size(room.min, vec2(STEP_LIST_WIDTH, high));
        ui.advance_cursor_after_rect(rect);
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 0.0, t.panel);
        let px = 1.0 / ui.ctx().pixels_per_point();
        painter.vline(rect.right() - px / 2.0, rect.y_range(), egui::Stroke::new(px, t.line));
        let small = FontId::proportional(metrics::SMALL_SIZE);
        let body = FontId::proportional(metrics::BODY_SIZE);
        let mut y = rect.top() + STEP_LIST_PADDING;
        let heading = painter.layout_no_wrap(self.heading.clone(), small, t.text_3);
        let at =
            pos2(rect.left() + STEP_HEADING_LEFT, y + (STEP_HEADING_LINE - heading.size().y) / 2.0);
        painter.galley(at, heading, t.text_3);
        let named = ui.interact(
            Rect::from_min_size(pos2(rect.left(), y), vec2(STEP_LIST_WIDTH, STEP_HEADING_LINE)),
            ui.id().with(("mg-step-heading", &self.heading)),
            egui::Sense::hover(),
        );
        let heading_words = self.heading.clone();
        named.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Label, true, &heading_words)
        });
        y += STEP_HEADING_LINE + STEP_HEADING_UNDER;
        let mut rows = Vec::with_capacity(self.steps.len());
        for (i, (name, state)) in self.steps.iter().enumerate() {
            let row = Rect::from_min_size(
                pos2(rect.left(), y),
                vec2(STEP_LIST_WIDTH - px, metrics::ROW_HEIGHT),
            );
            let words = match state {
                StepState::Done => t.text,
                StepState::Current => {
                    painter.rect_filled(row, 0.0, t.accent_fill);
                    t.on_fill
                }
                StepState::ToCome => t.text_2,
                StepState::Off => t.text_off,
            };
            let check = Rect::from_min_size(
                pos2(row.left() + STEP_ROW_LEFT, row.center().y - metrics::ICON / 2.0),
                vec2(metrics::ICON, metrics::ICON),
            );
            if *state == StepState::Done {
                Icon::Check.paint(&painter, check, t.ok);
            }
            let left = check.right() + STEP_ROW_GAP;
            let mut job =
                egui::text::LayoutJob::simple_singleline(name.clone(), body.clone(), words);
            job.wrap =
                egui::text::TextWrapping::truncate_at_width((row.right() - 6.0 - left).max(0.0));
            let galley = painter.layout_job(job);
            painter.galley(pos2(left, row.center().y - galley.size().y / 2.0), galley, words);
            let r = ui.interact(row, ui.id().with(("mg-step", i)), egui::Sense::hover());
            let (enabled, current) = (*state != StepState::Off, *state == StepState::Current);
            let named = format!("Step {}: {name}", i + 1);
            r.widget_info(|| {
                egui::WidgetInfo::selected(
                    egui::WidgetType::SelectableLabel,
                    enabled,
                    current,
                    &named,
                )
            });
            rows.push(row);
            y += metrics::ROW_HEIGHT;
        }
        StepListShown { rect, rows }
    }
}

/// A wizard page's bar (`.pg-h`): [`PAGE_BAR_HEIGHT`] high across the
/// room `ui` has, on `bar` with a `line` under it, the page's icon in
/// `text-2` and its title in `text`, [`PAGE_BAR_PADDING`] in. Takes that
/// row; its response names the title for a screen reader (a heading's
/// label).
pub fn page_bar(ui: &mut Ui, icon: Option<Icon>, title: &str) -> Response {
    let t = ui.tokens();
    let room = ui.available_rect_before_wrap();
    let rect = Rect::from_min_size(room.min, vec2(room.width(), PAGE_BAR_HEIGHT));
    ui.advance_cursor_after_rect(rect);
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 0.0, t.bar);
    let px = 1.0 / ui.ctx().pixels_per_point();
    painter.hline(rect.x_range(), rect.bottom() - px / 2.0, egui::Stroke::new(px, t.line));
    let mut left = rect.left() + PAGE_BAR_PADDING;
    if let Some(icon) = icon {
        let at = Rect::from_min_size(
            pos2(left, rect.center().y - metrics::ICON / 2.0),
            vec2(metrics::ICON, metrics::ICON),
        );
        icon.paint(&painter, at, t.text_2);
        left = at.right() + PAGE_BAR_GAP;
    }
    let body = FontId::proportional(metrics::BODY_SIZE);
    let mut job = egui::text::LayoutJob::simple_singleline(title.to_string(), body, t.text);
    job.wrap = egui::text::TextWrapping::truncate_at_width(
        (rect.right() - PAGE_BAR_PADDING - left).max(0.0),
    );
    let galley = painter.layout_job(job);
    painter.galley(pos2(left, rect.center().y - galley.size().y / 2.0), galley, t.text);
    let r = ui.interact(rect, ui.id().with(("mg-page-bar", title)), egui::Sense::hover());
    r.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Label, true, title));
    r
}

/// A dialog's footer's height (`.dlg-foot { height: 46px }`).
pub const FOOTER_HEIGHT: f32 = 46.0;
/// The room at its ends (`.dlg-foot { padding: 0 12px }`).
pub const FOOTER_PADDING: f32 = 12.0;
/// The room between its parts (`.dlg-foot { gap: 8px }`).
pub const FOOTER_GAP: f32 = 8.0;

/// What a footer's left control is drawn by.
type AddLeft<'a> = Box<dyn FnOnce(&mut Ui) + 'a>;

/// A dialog's footer (`.dlg-foot`, the dialog pattern of the proposal's
/// §6.1 and Part 2's decision 1): [`FOOTER_HEIGHT`] high across the
/// dialog, a `line-soft` rule above it, its count or note at the left in
/// `text-2` (or a control: a setting, a wizard's step), its buttons at
/// the right in the order given, the dialog's own (the main one first and
/// in the accent fill, `ButtonLook::Primary`, Cancel last; a wizard's
/// Back before its Next). The buttons are never cut: they stand from the
/// right end whatever the width, and the note is shortened (its whole
/// words its tooltip) where the room left of them is less than it needs.
/// What Enter and Escape do is the dialog's own, not the footer's.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use mg_uikit::dialog::Footer;
/// use mg_uikit::{ButtonLook, KitButton};
/// let shown = Footer::new()
///     .note("555 portraits")
///     .button(KitButton::new(None, "OK").look(ButtonLook::Primary))
///     .button(KitButton::new(None, "Cancel"))
///     .show(ui);
/// assert_eq!(shown.buttons.len(), 2);
/// assert_eq!(shown.rect.height(), mg_uikit::dialog::FOOTER_HEIGHT);
/// # });
/// ```
#[must_use = "show it with Footer::show"]
pub struct Footer<'a> {
    note: Option<String>,
    left: Option<(f32, AddLeft<'a>)>,
    buttons: Vec<KitButton>,
}

impl std::fmt::Debug for Footer<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Footer")
            .field("note", &self.note)
            .field("left", &self.left.as_ref().map(|(w, _)| *w))
            .field("buttons", &self.buttons)
            .finish()
    }
}

impl Default for Footer<'_> {
    fn default() -> Self {
        Footer::new()
    }
}

/// What a [`Footer`] showed: the buttons' responses in the order given,
/// and the rect it took (the rule along its top).
#[derive(Debug)]
pub struct FooterShown {
    pub buttons: Vec<Response>,
    pub rect: Rect,
    /// The note's rect, where it has one.
    pub note: Option<Rect>,
}

impl<'a> Footer<'a> {
    /// A footer with nothing in it yet.
    pub fn new() -> Self {
        Footer { note: None, left: None, buttons: Vec::new() }
    }

    /// With `note` at its left in `text-2`: a count ("555 portraits"), a
    /// state.
    pub fn note(mut self, note: impl Into<String>) -> Self {
        self.note = Some(note.into());
        self
    }

    /// With a control at its left, `width` wide, made by `add` in that
    /// room (a setting, a wizard's step); after the note where it has one.
    pub fn left(mut self, width: f32, add: impl FnOnce(&mut Ui) + 'a) -> Self {
        self.left = Some((width, Box::new(add)));
        self
    }

    /// With `button` after those given before (the right end's last).
    pub fn button(mut self, button: KitButton) -> Self {
        self.buttons.push(button);
        self
    }

    /// The width its buttons and left control need with its ends' room:
    /// the least a dialog with it is (the note, shortened, needs none).
    pub fn least_width(&self, ui: &Ui) -> f32 {
        let buttons: f32 = self.buttons.iter().map(|b| b.width(ui)).sum();
        let gaps = FOOTER_GAP * self.buttons.len().saturating_sub(1) as f32;
        let left = self.left.as_ref().map_or(0.0, |(w, _)| w + FOOTER_GAP);
        2.0 * FOOTER_PADDING + buttons + gaps + left
    }

    /// Shows it across the room `ui` has, from its cursor down, and takes
    /// that room.
    pub fn show(self, ui: &mut Ui) -> FooterShown {
        let t = ui.tokens();
        let room = ui.available_rect_before_wrap();
        let rect = Rect::from_min_size(room.min, vec2(room.width(), FOOTER_HEIGHT));
        ui.advance_cursor_after_rect(rect);
        ui.painter().hline(rect.x_range(), rect.top() + 0.5, egui::Stroke::new(1.0, t.line_soft));
        let high = metrics::CONTROL_HEIGHT;
        let top = (rect.center().y - high / 2.0).round();
        // (From the right end, the last given there.)
        let widths: Vec<f32> = self.buttons.iter().map(|b| b.width(ui)).collect();
        let mut right = rect.right() - FOOTER_PADDING;
        let mut places = vec![Rect::NOTHING; widths.len()];
        for (i, w) in widths.iter().enumerate().rev() {
            places[i] = Rect::from_min_size(pos2(right - w, top), vec2(*w, high));
            right -= w + FOOTER_GAP;
        }
        let buttons: Vec<Response> =
            self.buttons.into_iter().zip(&places).map(|(b, at)| ui.put(*at, b)).collect();
        // The left: the note, then the control, in what the buttons leave.
        let mut left = rect.left() + FOOTER_PADDING;
        let end = if widths.is_empty() { rect.right() - FOOTER_PADDING } else { right };
        let control = self.left.as_ref().map_or(0.0, |(w, _)| *w + FOOTER_GAP);
        let mut note = None;
        if let Some(words) = self.note {
            let wide = (end - left - control).max(0.0);
            let body = FontId::proportional(metrics::BODY_SIZE);
            let mut job = egui::text::LayoutJob::simple_singleline(words.clone(), body, t.text_2);
            job.wrap = egui::text::TextWrapping::truncate_at_width(wide);
            let galley = ui.painter().layout_job(job);
            let size = galley.size();
            let at = Rect::from_min_size(pos2(left, rect.center().y - size.y / 2.0), size);
            let elided = galley.elided;
            let r = ui.put(at, egui::Label::new(galley).selectable(false));
            if elided {
                r.on_hover_text(words);
            }
            note = Some(at);
            left = at.right() + FOOTER_GAP;
        }
        if let Some((width, add)) = self.left {
            let width = width.min((end - left).max(0.0));
            let at = Rect::from_min_size(pos2(left, top), vec2(width, high));
            let mut child = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(at)
                    .layout(egui::Layout::left_to_right(egui::Align::Center)),
            );
            child.set_clip_rect(at.intersect(ui.clip_rect()));
            add(&mut child);
        }
        FooterShown { buttons, rect, note }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui_kittest::Harness;
    use egui_kittest::kittest::{NodeT as _, Queryable as _};

    /// Its parts stand in the middle of its room, one under the other,
    /// named for a screen reader, the buttons side by side where they fit
    /// and stacked where they do not, never past the room's sides; the
    /// same room gives the same places from frame to frame.
    #[test]
    fn an_empty_state_stands_in_the_middle_of_its_room() {
        let places = |width: f32| {
            let mut out: Vec<(Rect, Vec<Rect>)> = Vec::new();
            let mut h = Harness::builder().with_size(vec2(width, 600.0)).build_ui(|ui| {
                crate::apply(ui.ctx(), crate::Theme::Dark);
                let room = ui.available_rect_before_wrap();
                let shown = EmptyState::new(Icon::Area, "No area open")
                    .why("Double-click an area in the Module panel to open it.")
                    .button(KitButton::new(None, "Area Wizard…"))
                    .button(KitButton::new(None, "Go to Start"))
                    .show(ui);
                out.push((room, shown.iter().map(|r| r.rect).collect()));
            });
            h.run_steps(3);
            let _ = h.get_by_label("No area open");
            let _ = h.get_by_label("Double-click an area in the Module panel to open it.");
            let _ = h.get_by_label("Go to Start");
            drop(h);
            let n = out.len();
            assert_eq!(out[n - 1], out[n - 2], "{width}: the same places frame after frame");
            out.pop().unwrap()
        };
        let (room, wide) = places(800.0);
        assert_eq!(wide[0].top(), wide[1].top(), "side by side: {wide:?}");
        let middle = (wide[0].left() + wide[1].right()) / 2.0;
        assert!((middle - room.center().x).abs() <= 0.5, "in the middle: {wide:?} {room:?}");
        let (above, below) = (wide[0].top() - room.top(), room.bottom() - wide[0].bottom());
        assert!(above > below, "the buttons under the middle: {above} {below}");
        assert!(above < room.height() * 0.65, "and near it: {above} of {room:?}");
        let (room, narrow) = places(150.0);
        assert!(narrow[1].top() > narrow[0].bottom(), "stacked: {narrow:?}");
        for r in &narrow {
            assert!(r.left() >= room.left() && r.right() <= room.right(), "inside: {narrow:?}");
        }
    }

    /// What a footer showed at `width`, frame after frame alike: its rect,
    /// the buttons' rects, the note's.
    fn footer_at(width: f32, note: &str, words: &[&str]) -> (Rect, Vec<Rect>, Option<Rect>) {
        let mut out = Vec::new();
        let mut h = Harness::builder().with_size(vec2(width, 200.0)).build_ui(|ui| {
            crate::apply(ui.ctx(), crate::Theme::Dark);
            ui.add_space(20.0);
            let mut footer = Footer::new().note(note);
            for (i, w) in words.iter().enumerate() {
                let look =
                    if i == 0 { crate::ButtonLook::Primary } else { crate::ButtonLook::Raised };
                footer = footer.button(KitButton::new(None, *w).look(look));
            }
            let shown = footer.show(ui);
            out.push((shown.rect, shown.buttons.iter().map(|r| r.rect).collect(), shown.note));
        });
        h.run_steps(3);
        for w in words {
            let _ = h.get_by_label(w);
        }
        drop(h);
        let n = out.len();
        assert_eq!(out[n - 1], out[n - 2], "the same places frame after frame");
        out.pop().unwrap()
    }

    /// The board's `.dlg-foot`: 46 high across its room, the buttons at
    /// the right end 12 in, 8 apart, in the order given, centered on its
    /// height; the note at the left end, 12 in, centered too.
    #[test]
    fn a_footer_has_its_buttons_at_the_right_in_order_and_its_note_at_the_left() {
        let (rect, buttons, note) = footer_at(600.0, "555 portraits", &["OK", "Cancel"]);
        assert_eq!(rect.height(), FOOTER_HEIGHT);
        assert_eq!(rect.width(), 600.0 - 2.0 * 8.0, "across the room: {rect:?}");
        let (ok, cancel) = (buttons[0], buttons[1]);
        assert_eq!(cancel.right(), rect.right() - FOOTER_PADDING, "{buttons:?}");
        assert_eq!(cancel.left() - ok.right(), FOOTER_GAP, "{buttons:?}");
        assert!((ok.center().y - rect.center().y).abs() <= 0.5, "{ok:?} in {rect:?}");
        let note = note.expect("a note");
        assert_eq!(note.left(), rect.left() + FOOTER_PADDING);
        assert!((note.center().y - rect.center().y).abs() <= 0.5, "{note:?} in {rect:?}");
    }

    /// Never cut: at a dialog's least width a wizard's four buttons stand
    /// whole inside it and side by side, and a long note is shortened
    /// short of them, never under them.
    #[test]
    fn a_footer_never_cuts_its_buttons() {
        let words = ["< Back", "Next >", "Finish", "Cancel"];
        // (A dialog's least: egui's 320 for its body and its margins.)
        let (rect, buttons, note) = footer_at(
            336.0 + 16.0,
            "Step 4 of 9: Appearance and Portrait, a long note that cannot fit",
            &words,
        );
        for b in &buttons {
            assert!(b.left() >= rect.left() && b.right() <= rect.right(), "inside: {buttons:?}");
            assert_eq!(b.height(), metrics::CONTROL_HEIGHT);
        }
        for pair in buttons.windows(2) {
            assert_eq!(pair[1].left() - pair[0].right(), FOOTER_GAP, "{buttons:?}");
        }
        let note = note.expect("a note");
        assert!(note.right() <= buttons[0].left() - FOOTER_GAP + 0.5, "{note:?} {buttons:?}");
        let mut ui_least = 0.0;
        egui::__run_test_ui(|ui| {
            crate::apply(ui.ctx(), crate::Theme::Dark);
            let mut f = Footer::new();
            for w in words {
                f = f.button(KitButton::new(None, w));
            }
            ui_least = f.least_width(ui);
        });
        assert!(ui_least <= 336.0, "a wizard's footer fits a dialog's least: {ui_least}");
    }

    /// The board's `.steps`: 190 wide down its room, the heading then a
    /// row a step, 22 high, from 28 below its top; the same rows whatever
    /// the steps' states (the list never moves under the pointer); each
    /// row named by its step, the current one selected, an Off one
    /// disabled.
    #[test]
    fn a_step_list_stands_its_rows_alike_whatever_their_states() {
        let states = [
            [StepState::Current, StepState::ToCome, StepState::ToCome, StepState::Off],
            [StepState::Done, StepState::Done, StepState::Current, StepState::Off],
        ];
        let mut seen = Vec::new();
        for states in states {
            let out = std::cell::RefCell::new(None);
            let mut h = Harness::builder().with_size(vec2(600.0, 400.0)).build_ui(|ui| {
                crate::apply(ui.ctx(), crate::Theme::Dark);
                let mut list = StepList::new("Step 1 of 4");
                for (name, state) in ["One", "Two", "Three", "Four"].iter().zip(states) {
                    list = list.step(*name, state);
                }
                *out.borrow_mut() = Some(list.show(ui));
            });
            h.run_steps(2);
            let shown = out.borrow_mut().take().unwrap();
            assert_eq!(shown.rect.width(), STEP_LIST_WIDTH, "{shown:?}");
            assert!(shown.rect.height() > 300.0, "down its room: {shown:?}");
            assert_eq!(shown.rows.len(), 4);
            assert_eq!(shown.rows[0].top() - shown.rect.top(), 28.0, "{shown:?}");
            for pair in shown.rows.windows(2) {
                assert_eq!(pair[1].top() - pair[0].top(), metrics::ROW_HEIGHT, "{shown:?}");
            }
            let current = states.iter().position(|s| *s == StepState::Current).unwrap();
            for (i, name) in ["One", "Two", "Three", "Four"].iter().enumerate() {
                let label = format!("Step {}: {name}", i + 1);
                let node = h.get_by_label(&label);
                assert_eq!(
                    node.accesskit_node().toggled(),
                    Some(if i == current {
                        egui::accesskit::Toggled::True
                    } else {
                        egui::accesskit::Toggled::False
                    }),
                    "{name} selected"
                );
                assert_eq!(
                    node.accesskit_node().is_disabled(),
                    states[i] == StepState::Off,
                    "{name} disabled"
                );
            }
            let _ = h.get_by_label("Step 1 of 4");
            seen.push(shown.rows);
        }
        assert_eq!(seen[0], seen[1], "the same rows whatever the states");
    }

    /// The board's `.pg-h`: 26 high across its room, named by its title.
    #[test]
    fn a_page_bar_is_26_high_across_its_room() {
        let mut out = None;
        let mut h = Harness::builder().with_size(vec2(500.0, 200.0)).build_ui(|ui| {
            crate::apply(ui.ctx(), crate::Theme::Dark);
            let room = ui.available_rect_before_wrap();
            let r = page_bar(ui, Some(Icon::Wizard), "Class and Level");
            out = Some((room, r.rect, ui.cursor().top()));
        });
        h.run_steps(2);
        let _ = h.get_by_label("Class and Level");
        let spacing = h.ctx.global_style().spacing.item_spacing.y;
        drop(h);
        let (room, bar, next) = out.unwrap();
        assert_eq!(bar.height(), PAGE_BAR_HEIGHT);
        assert_eq!(bar.x_range(), room.x_range());
        assert_eq!(next, bar.bottom() + spacing);
    }
}
