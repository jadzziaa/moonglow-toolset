//! The parts of a dialog and of a pane with nothing in it (the
//! proposal's M4, Part 2, decision 1): so far the empty state
//! ([`EmptyState`], the mock's `.empty`), which the toolset's "No area
//! open" is (M4's U24). The dialog's footer and step list come with the
//! dialogs (U26, U27).

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

#[cfg(test)]
mod tests {
    use super::*;
    use egui_kittest::Harness;
    use egui_kittest::kittest::Queryable as _;

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
}
