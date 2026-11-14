//! A tree's rows as the mock draws them (`.tree-row`): a group's row
//! ([`TreeGroup`]: its arrow, its kind's icon, its name and its count
//! right-aligned, over its rows, which open and close under it), and a
//! row that opens nothing ([`TreeRow`]: the arrow's column left empty).
//! Each is [`metrics::ROW_HEIGHT`] high and as wide as what holds it, its
//! fill drawn across the whole row (in a scroll area from its left edge,
//! the indent of a row under its group too, as the board fills it):
//! `hover-row` under the pointer, `accent-fill` marked (its words and
//! icons then `on-fill`, the count at 85%). A row can drag
//! ([`TreeRow::sense`]), leave its arrow's column to a button of the
//! caller's ([`TreeRow::arrow_apart`], [`arrow_column`]) and show marks
//! after its name ([`TreeRow::show_parts`]).
//!
//! An Outliner's rows (`LightArea_crop_outl.png`) add to these: a show or
//! hide **eye** at the row's right, after its count ([`TreeGroup::eye`],
//! [`TreeRow::eye`]: a toggle of its own, which takes its click before the
//! row), a quiet look for a kind hidden in the view that keeps the row
//! interactive ([`TreeGroup::quiet`], [`TreeRow::quiet`]), and a weak
//! second word after the name ([`TreeRow::second`]: an object's tag), the
//! two cut with an ellipsis to the row's width ([`TreeRow::fit`]) rather
//! than widening what scrolls them.
//!
//! ```
//! # egui::__run_test_ui(|ui| {
//! let shown = mg_uikit::TreeGroup::new("areas", "Areas")
//!     .icon(mg_uikit::Icon::Area)
//!     .count(113)
//!     .default_open(true)
//!     .show(ui, |ui| ui.label("map_m1q1a"));
//! assert!(shown.body_returned.is_some());
//! # });
//! ```

use egui::collapsing_header::{CollapsingResponse, CollapsingState};
use egui::{
    Color32, Pos2, Rect, Response, Sense, TextStyle, TextWrapMode, Ui, WidgetInfo, WidgetText,
    WidgetType, pos2, vec2,
};

use crate::icons::Icon;
use crate::metrics;
use crate::theme::TokensExt as _;

/// The room at a row's left before its arrow (`.tree-row`: `padding: 0
/// 6px 0 4px`).
pub const ROW_PADDING_LEFT: f32 = 4.0;
/// The room at a row's right after its count.
pub const ROW_PADDING_RIGHT: f32 = 6.0;
/// The arrow's column (`.tree-row .arrow`: 14 px).
pub const ARROW: f32 = 14.0;
/// The gap between a row's parts (`.tree-row`: `gap: 4px`).
pub const ROW_GAP: f32 = 4.0;
/// The least room before a count (`.tree-row .count`: `padding-left:
/// 6px`).
pub const COUNT_GAP: f32 = 6.0;
/// An eye's side (`.tree-row .eye`: 18 px, its icon 14).
pub const EYE: f32 = 18.0;
/// An eye's icon's side.
pub const EYE_ICON: f32 = 14.0;
/// The room before a weak second word (`.tree-row .sub`: `padding-left:
/// 4px`, after the row's 4 px gap).
pub const SUB_GAP: f32 = 4.0;
/// The most of a row's room for words its name takes where a weak second
/// word follows it (`.name:has(+ .sub)`: `max-width: 70%`).
pub const NAME_SHARE: f32 = 0.7;

/// A show or hide eye at a row's right ([`TreeGroup::eye`],
/// [`TreeRow::eye`]): whether what it stands for is shown, its name for a
/// screen reader (and tests), and its tip.
#[derive(Clone, Debug)]
struct Eye {
    shown: bool,
    name: String,
    tip: String,
}

/// Where a row's name begins, from the row's left edge: after the arrow's
/// column, and after the icon where it has one (42 and 22).
pub fn name_offset(icon: bool) -> f32 {
    let icon = if icon { metrics::ICON + ROW_GAP } else { 0.0 };
    ROW_PADDING_LEFT + ARROW + ROW_GAP + icon
}

/// The arrow's column of the row laid out next in `ui` (at its cursor), as
/// high as a row: where a [`TreeRow::arrow_apart`] row's caller puts the
/// button that opens it out.
pub fn arrow_column(ui: &Ui) -> Rect {
    let at = ui.cursor().min;
    Rect::from_min_size(pos2(at.x + ROW_PADDING_LEFT, at.y), vec2(ARROW, crate::row_height(ui)))
}

/// The parts a group's or a row's line shows.
#[derive(Clone, Debug)]
struct Line {
    name: String,
    icon: Option<Icon>,
    count: Option<String>,
    label: Option<String>,
    marked: bool,
    dim: bool,
    /// A weak second word after its name ([`TreeRow::second`]).
    sub: Option<String>,
    /// Its words cut with an ellipsis to the row's width, rather than
    /// widening what holds it ([`TreeRow::fit`]).
    fit: bool,
    /// A show or hide eye at its right, after its count.
    eye: Option<Eye>,
    /// Its arrow's column another widget's: the row answers the pointer
    /// from its name on.
    arrow_apart: bool,
    /// Where in the painter's shapes its fill goes (under what was drawn
    /// before the row over it), else drawn with the row.
    fill_under: Option<egui::layers::ShapeIdx>,
}

impl Line {
    fn new(name: String) -> Self {
        let (marked, dim, arrow_apart) = (false, false, false);
        let (icon, count, label, fill_under) = (None, None, None, None);
        let (sub, fit, eye) = (None, false, None);
        Line { name, icon, count, label, marked, dim, sub, fit, eye, arrow_apart, fill_under }
    }

    /// Its accessible name: its own, else the name with the count after
    /// it in brackets ("Areas (113)").
    fn label(&self) -> String {
        match (&self.label, &self.count) {
            (Some(label), _) => label.clone(),
            (None, Some(count)) => format!("{} ({count})", self.name),
            (None, None) => self.name.clone(),
        }
    }

    /// Takes a row's room in `ui`: the width its parts need, and
    /// [`crate::row_height`] high; it is drawn and answers the pointer as
    /// wide as `ui` (and as the part of it in sight, where `ui` is
    /// scrolled sideways) where that is wider. (Taken as wide as that, a
    /// tree's rows widened what scrolls them, and a tree that keeps the
    /// widest it was scrolled sideways for good once its scroll bar came.)
    /// The row's whole rectangle, the room taken, and its words laid out
    /// where it is in sight (a row out of sight lays out none: a tree can
    /// hold thousands).
    fn allocate(&self, ui: &mut Ui, sense: Sense, id: Option<egui::Id>) -> Allocated {
        // (In a scroll area scrolled sideways, what holds it has moved
        // left with what it shows: the row reaches the edge in sight.)
        let right = match ui.stack().contained_in(egui::UiKind::ScrollArea) {
            true => ui.max_rect().right().max(ui.clip_rect().right()),
            false => ui.max_rect().right(),
        };
        let (at, height) = (ui.cursor().min, crate::row_height(ui));
        let whole = Rect::from_min_max(at, pos2(right.max(at.x + 1.0), at.y + height));
        let words = ui.is_rect_visible(whole).then(|| self.words(ui, whole.width()));
        let needed = words.as_ref().map_or(0.0, |w| {
            let count = w.count.as_ref().map_or(0.0, |c| COUNT_GAP + c.size().x);
            let sub = w.sub.as_ref().map_or(0.0, |s| SUB_GAP + s.size().x);
            let (name, eye) = (w.name.size().x, self.eye_room());
            name_offset(self.icon.is_some()) + name + sub + count + eye + ROW_PADDING_RIGHT
        });
        let (auto, taken) = ui.allocate_space(vec2(needed, height));
        let mut rect = taken.with_max_x(taken.right().max(right));
        if self.arrow_apart {
            rect.min.x = (rect.min.x + name_offset(self.icon.is_some())).min(rect.max.x - 1.0);
        }
        let response = ui.interact(rect, id.unwrap_or(auto), sense);
        // (Drawn from the row's own left edge.)
        let rect = rect.with_min_x(taken.left());
        // (Its fill from the left edge of what scrolls it, as the board
        // fills a row indented under its group: the indent is the row's
        // own padding there.)
        let fill = match ui.stack().contained_in(egui::UiKind::ScrollArea) {
            true => rect.with_min_x(rect.left().min(ui.clip_rect().left())),
            false => rect,
        };
        Allocated { rect, taken, fill, response, words }
    }

    fn galley(ui: &Ui, text: &str) -> std::sync::Arc<egui::Galley> {
        Self::galley_cut(ui, text, f32::INFINITY)
    }

    /// `text` laid out on one line, cut with an ellipsis at `max` points.
    fn galley_cut(ui: &Ui, text: &str, max: f32) -> std::sync::Arc<egui::Galley> {
        let wrap = if max.is_finite() { TextWrapMode::Truncate } else { TextWrapMode::Extend };
        WidgetText::from(text).into_galley(ui, Some(wrap), max.max(1.0), TextStyle::Button)
    }

    /// The room its eye takes at its right, with the gap before it.
    fn eye_room(&self) -> f32 {
        if self.eye.is_some() { EYE + ROW_GAP } else { 0.0 }
    }

    /// Its words laid out for a row `width` wide: whole, or where it is
    /// [`Line::fit`], cut to the room its icon, count and eye leave, the
    /// name to at most [`NAME_SHARE`] of it where a weak second word
    /// follows (which takes the rest), as the board's CSS shares it.
    fn words(&self, ui: &Ui, width: f32) -> Words {
        let count = self.count.as_ref().map(|c| Self::galley(ui, c));
        if !self.fit {
            let sub = self.sub.as_ref().map(|s| Self::galley(ui, s));
            return Words { name: Self::galley(ui, &self.name), sub, count };
        }
        let counted = count.as_ref().map_or(0.0, |c| COUNT_GAP + c.size().x);
        let fixed = name_offset(self.icon.is_some()) + counted + self.eye_room();
        let room = (width - fixed - ROW_PADDING_RIGHT).max(1.0);
        let Some(sub) = self.sub.as_ref().filter(|s| !s.is_empty()) else {
            return Words { name: Self::galley_cut(ui, &self.name, room), sub: None, count };
        };
        let whole = Self::galley(ui, &self.name);
        let name = if whole.size().x <= room * NAME_SHARE {
            whole
        } else {
            Self::galley_cut(ui, &self.name, room * NAME_SHARE)
        };
        let left = room - name.size().x - SUB_GAP;
        let sub = (left >= 8.0).then(|| Self::galley_cut(ui, sub, left));
        Words { name, sub, count }
    }

    /// Paints the line in `rect`: its fill, the arrow `openness` turned
    /// (none where `None`), its icon, its name and its count against the
    /// right edge of what is in sight.
    fn paint(&self, ui: &Ui, row: Allocated, openness: Option<f32>) -> Allocated {
        let (rect, response) = (row.rect, &row.response);
        let Some(words) = row.words.as_ref().filter(|_| ui.is_rect_visible(rect)) else {
            return row;
        };
        let t = ui.ctx().tokens();
        let themed = crate::theme::applied(ui.ctx()).is_some();
        let hovered = response.hovered() || response.has_focus();
        let (fill, text, arrow, icon, weak) = if self.marked {
            let on = if themed { t.on_fill } else { ui.visuals().selection.stroke.color };
            let fill = if themed { t.accent_fill } else { ui.visuals().selection.bg_fill };
            (fill, on, on, on, on.gamma_multiply(0.85))
        } else if themed {
            let fill = if hovered { t.hover_row } else { Color32::TRANSPARENT };
            let arrow = if hovered { t.text } else { t.text_3 };
            let (text, icon) = if self.dim { (t.text_3, t.text_3) } else { (t.text, t.text_2) };
            (fill, text, arrow, icon, t.text_3)
        } else {
            let v = ui.visuals();
            let fill = if hovered { v.widgets.hovered.weak_bg_fill } else { Color32::TRANSPARENT };
            let text = if self.dim { v.weak_text_color() } else { v.text_color() };
            (fill, text, v.weak_text_color(), text, v.weak_text_color())
        };
        let p = ui.painter();
        if fill != Color32::TRANSPARENT {
            let shape = egui::Shape::rect_filled(row.fill, 0.0, fill);
            match self.fill_under {
                Some(at) => p.set(at, shape),
                None => _ = p.add(shape),
            }
        }
        let left = rect.left() + ROW_PADDING_LEFT;
        let middle = rect.center().y;
        if let Some(openness) = openness {
            crate::controls::paint_chevron(p, pos2(left + ARROW / 2.0, middle), openness, arrow);
        }
        if let Some(i) = self.icon {
            let side = metrics::ICON;
            let at = pos2(left + ARROW + ROW_GAP, middle - side / 2.0);
            i.paint(p, Rect::from_min_size(at, vec2(side, side)), icon);
        }
        let name = words.name.clone();
        let x = rect.left() + name_offset(self.icon.is_some());
        let mut name_end = row.name_end(self.icon.is_some());
        p.galley(Pos2::new(x, (middle - name.size().y / 2.0).round()), name, text);
        if let Some(galley) = words.sub.clone() {
            let at = Pos2::new(name_end + SUB_GAP, (middle - galley.size().y / 2.0).round());
            name_end = at.x + galley.size().x;
            p.galley(at, galley, weak);
        }
        if let Some(galley) = words.count.clone() {
            // (Against the edge in sight: the tree scrolled sideways keeps
            // its counts at the panel's right; before the eye.)
            let right =
                rect.right().min(ui.clip_rect().right()) - ROW_PADDING_RIGHT - self.eye_room();
            let x = (right - galley.size().x).max(name_end + COUNT_GAP);
            p.galley(Pos2::new(x, (middle - galley.size().y / 2.0).round()), galley, weak);
        }
        row
    }
}

/// A row's words laid out: its name, its weak second word, and its count.
struct Words {
    name: std::sync::Arc<egui::Galley>,
    sub: Option<std::sync::Arc<egui::Galley>>,
    count: Option<std::sync::Arc<egui::Galley>>,
}

/// Where a row's eye goes: at its right, against the edge in sight, after
/// its count.
fn eye_rect(ui: &Ui, rect: Rect) -> Rect {
    let right = rect.right().min(ui.clip_rect().right()) - ROW_PADDING_RIGHT;
    Rect::from_center_size(pos2(right - EYE / 2.0, rect.center().y), vec2(EYE, EYE))
}

/// A row's eye ([`Eye`]) at `rect`, over the row (`row_id`; the eye's id is
/// apart from it, and added after it, so it takes the clicks on it before
/// the row). A toggle for a screen reader (said pressed while shown),
/// named by its own name. Its response; clicked, the caller shows or hides
/// what it stands for. Drawn after the row ([`paint_eye`]), over its fill.
fn eye_button(ui: &mut Ui, rect: Rect, row_id: egui::Id, eye: &Eye) -> Response {
    let response = ui.interact(rect, row_id.with("eye"), Sense::click());
    let enabled = ui.is_enabled();
    response.widget_info(|| {
        WidgetInfo::selected(WidgetType::Button, enabled, eye.shown, eye.name.clone())
    });
    if eye.tip.is_empty() { response } else { response.on_hover_text(&eye.tip) }
}

/// Paints a row's eye ([`eye_button`], `response`): its icon, `eye` while
/// shown, `eye-off` in `text-3` while hidden; `raised` under it while
/// hovered; on a `marked` row in `on-fill`.
fn paint_eye(ui: &Ui, response: &Response, eye: &Eye, marked: bool) {
    let rect = response.rect;
    if ui.is_rect_visible(rect) {
        let t = ui.ctx().tokens();
        let themed = crate::theme::applied(ui.ctx()).is_some();
        let v = ui.visuals();
        let color = match (marked, themed, eye.shown) {
            (true, true, _) => t.on_fill,
            (true, false, _) => v.selection.stroke.color,
            (false, true, true) => t.text_2,
            (false, true, false) => t.text_3,
            (false, false, true) => v.text_color(),
            (false, false, false) => v.weak_text_color(),
        };
        let p = ui.painter();
        if response.hovered() || response.has_focus() {
            let fill = if themed { t.raised } else { v.widgets.hovered.weak_bg_fill };
            p.rect_filled(rect, 3.0, fill);
        }
        let icon = if eye.shown { Icon::Eye } else { Icon::EyeOff };
        icon.paint(p, Rect::from_center_size(rect.center(), vec2(EYE_ICON, EYE_ICON)), color);
    }
}

/// A row's room taken ([`Line::allocate`]).
struct Allocated {
    /// The whole row (as it answers the pointer, but for an arrow's
    /// column apart: [`Line::arrow_apart`]).
    rect: Rect,
    /// The room it took.
    taken: Rect,
    /// Where its fill is drawn: the whole row, and in a scroll area out
    /// to its left edge.
    fill: Rect,
    response: Response,
    /// Its words, where it is in sight.
    words: Option<Words>,
}

impl Allocated {
    /// Where its name ends (where it would begin, its words not laid
    /// out).
    fn name_end(&self, icon: bool) -> f32 {
        let name = self.words.as_ref().map_or(0.0, |w| w.name.size().x);
        self.rect.left() + name_offset(icon) + name
    }
}

/// A tree's group (`.tree-row` with an arrow, over its rows): its row
/// shows the arrow (right closed, down open), its kind's icon, its name
/// and its count right-aligned; a click on it (or Enter or Space with the
/// keyboard's focus) opens or closes it, and its rows, indented by
/// [`metrics::INDENT`], open and close under it as egui's collapsing
/// header's do (with the same state: `ui.make_persistent_id(id_salt)`).
///
/// Named for a screen reader (and tests) as a collapsing header by its
/// name and its count ("Areas (113)", or [`TreeGroup::label`]), and said
/// expanded or not.
#[derive(Clone, Debug)]
#[must_use = "show it with .show(ui, add_body)"]
pub struct TreeGroup {
    id_salt: egui::Id,
    line: Line,
    default_open: bool,
    open: Option<bool>,
}

impl TreeGroup {
    /// The group `name`, its open state kept under `id_salt`.
    pub fn new(id_salt: impl std::hash::Hash + std::fmt::Debug, name: impl Into<String>) -> Self {
        TreeGroup {
            id_salt: egui::Id::new(id_salt),
            line: Line::new(name.into()),
            default_open: false,
            open: None,
        }
    }

    /// Its kind's icon, after the arrow.
    pub fn icon(mut self, icon: Icon) -> Self {
        self.line.icon = Some(icon);
        self
    }

    /// Its count, right-aligned (`113`, `3 of 46`).
    pub fn count(mut self, count: impl ToString) -> Self {
        self.line.count = Some(count.to_string());
        self
    }

    /// Named `label` for a screen reader in place of its name and count.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.line.label = Some(label.into());
        self
    }

    /// Open the first time it is shown.
    pub fn default_open(mut self, open: bool) -> Self {
        self.default_open = open;
        self
    }

    /// Opened or closed this frame (`Some`), else as it was.
    pub fn open(mut self, open: Option<bool>) -> Self {
        self.open = open;
        self
    }

    /// Its row marked (`accent-fill`): the one in hand, or the keys' row.
    pub fn marked(mut self, marked: bool) -> Self {
        self.line.marked = marked;
        self
    }

    /// Its name and icon quiet (`text-3`, `.tree-row.dim`): an Outliner's
    /// kind hidden in the view. It opens and closes as ever.
    pub fn quiet(mut self, quiet: bool) -> Self {
        self.line.dim = quiet;
        self
    }

    /// A show or hide eye at its right, after its count: `shown` (the
    /// `eye` icon) or hidden (`eye-off`), named `name` for a screen reader,
    /// its tip `tip`. Its click is the eye's, not the row's
    /// ([`TreeGroup::show_eyed`] says it was clicked).
    pub fn eye(mut self, shown: bool, name: impl Into<String>, tip: impl Into<String>) -> Self {
        self.line.eye = Some(Eye { shown, name: name.into(), tip: tip.into() });
        self
    }

    /// Its row and its rows under it (`add_body`, while it is open).
    pub fn show<R>(
        self,
        ui: &mut Ui,
        add_body: impl FnOnce(&mut Ui) -> R,
    ) -> CollapsingResponse<R> {
        self.show_eyed(ui, add_body).0
    }

    /// [`TreeGroup::show`], and its eye's response where it has one
    /// ([`TreeGroup::eye`]), added after its row and before its rows (the
    /// keyboard's focus goes from the row to its eye, then into it).
    pub fn show_eyed<R>(
        self,
        ui: &mut Ui,
        add_body: impl FnOnce(&mut Ui) -> R,
    ) -> (CollapsingResponse<R>, Option<Response>) {
        let TreeGroup { id_salt, line, default_open, open } = self;
        ui.vertical(|ui| {
            let id = ui.make_persistent_id(id_salt);
            let mut row = line.allocate(ui, Sense::click(), Some(id));
            let eye =
                (line.eye.as_ref()).map(|eye| eye_button(ui, eye_rect(ui, row.rect), id, eye));
            let mut state = CollapsingState::load_with_default_open(ui.ctx(), id, default_open);
            if let Some(open) = open {
                if open != state.is_open() {
                    state.toggle(ui);
                    row.response.mark_changed();
                }
            } else if row.response.clicked() {
                state.toggle(ui);
                row.response.mark_changed();
            }
            let enabled = ui.is_enabled();
            row.response.widget_info(|| {
                WidgetInfo::labeled(WidgetType::CollapsingHeader, enabled, line.label())
            });
            let openness = state.openness(ui.ctx());
            let row = line.paint(ui, row, Some(openness));
            if let (Some(e), Some(r)) = (&line.eye, &eye) {
                paint_eye(ui, r, e, line.marked);
            }
            let header_response = row.response;
            // (Its rows as wide as the room the row took, not as the row
            // is drawn: `allocate`.)
            let taken = header_response.clone().with_new_rect(row.taken);
            let body = state.show_body_indented(&taken, ui, add_body);
            crate::say_expanded(&header_response, body.is_some());
            let shown = CollapsingResponse {
                header_response,
                body_response: body.as_ref().map(|b| b.response.clone()),
                body_returned: body.map(|b| b.inner),
                openness,
            };
            (shown, eye)
        })
        .inner
    }
}

/// A tree's row that opens nothing (`.tree-row` with its arrow's column
/// empty): an icon where it has one, its name, a count where it has one;
/// selectable where it is [`TreeRow::selected`] or clicked (a selectable
/// label for a screen reader, said selected or not), or a label alone
/// ([`TreeRow::dim`]: a kind with nothing in it).
#[derive(Clone, Debug)]
#[must_use = "show it with .show(ui)"]
pub struct TreeRow {
    line: Line,
    sense: Sense,
}

impl TreeRow {
    /// The row `name`, clicked as a selectable label.
    pub fn new(name: impl Into<String>) -> Self {
        TreeRow { line: Line::new(name.into()), sense: Sense::click() }
    }

    /// Its icon, after the arrow's column.
    pub fn icon(mut self, icon: Icon) -> Self {
        self.line.icon = Some(icon);
        self
    }

    /// Its count, right-aligned.
    pub fn count(mut self, count: impl ToString) -> Self {
        self.line.count = Some(count.to_string());
        self
    }

    /// Named `label` for a screen reader in place of its name and count.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.line.label = Some(label.into());
        self
    }

    /// Selected (`accent-fill`).
    pub fn selected(mut self, selected: bool) -> Self {
        self.line.marked = selected;
        self
    }

    /// Quiet (`text-3`) and not interactive: a label.
    pub fn dim(mut self) -> Self {
        self.line.dim = true;
        self.sense = Sense::hover();
        self
    }

    /// Its name and icon quiet (`text-3`, `.tree-row.dim`) where `quiet`,
    /// interactive as ever (where [`TreeRow::dim`] makes it a label): an
    /// Outliner's kind hidden in the view.
    pub fn quiet(mut self, quiet: bool) -> Self {
        self.line.dim = quiet;
        self
    }

    /// A weak second word after its name (`.sub`, `text-3`): an object's
    /// tag. The row's words are then cut to its width ([`TreeRow::fit`]),
    /// the name to at most [`NAME_SHARE`] of their room, the second word to
    /// the rest. Its accessible name stays its name (or its
    /// [`TreeRow::label`]).
    pub fn second(mut self, sub: impl Into<String>) -> Self {
        self.line.sub = Some(sub.into());
        self.line.fit = true;
        self
    }

    /// Its words cut with an ellipsis to the row's width, rather than
    /// widening what holds it (and scrolling it sideways): an Outliner's.
    pub fn fit(mut self) -> Self {
        self.line.fit = true;
        self
    }

    /// A show or hide eye at its right, after its count, as
    /// [`TreeGroup::eye`]'s ([`TreeRowShown::eye`] is its response).
    pub fn eye(mut self, shown: bool, name: impl Into<String>, tip: impl Into<String>) -> Self {
        self.line.eye = Some(Eye { shown, name: name.into(), tip: tip.into() });
        self
    }

    /// Its arrow's column holds a widget of the caller's (a button that
    /// opens the row out, added before the row at [`arrow_column`], so
    /// that the keyboard's focus comes to it before the row): the row
    /// answers the pointer, and is said to be, from its name on.
    pub fn arrow_apart(mut self) -> Self {
        self.line.arrow_apart = true;
        self
    }

    /// Its fill drawn at `at` among the painter's shapes: a place kept
    /// (`Painter::add(Shape::Noop)`) before what is drawn over the row
    /// before it, the button in its arrow's column
    /// ([`TreeRow::arrow_apart`]), which its fill would cover.
    pub fn fill_under(mut self, at: egui::layers::ShapeIdx) -> Self {
        self.line.fill_under = Some(at);
        self
    }

    /// What it answers ([`Sense::click`] where not given): a row that is
    /// dragged too (a blueprint's, into an area) senses clicks and drags.
    pub fn sense(mut self, sense: Sense) -> Self {
        self.sense = sense;
        self
    }

    /// Draws the row; its response.
    pub fn show(self, ui: &mut Ui) -> Response {
        self.show_parts(ui).response
    }

    /// Draws the row; its response, and where the caller puts what the
    /// row holds besides its name: a button in its arrow's column (an
    /// area's, that opens it out to what is placed in it) and marks after
    /// its name (who holds it, a note). Put there in a child `Ui`
    /// (`Ui::new_child`) after the row, they answer the pointer before it
    /// and take nothing from the rows' layout: the caller widens what
    /// holds the row by what they take (`Ui::expand_to_include_x`).
    pub fn show_parts(self, ui: &mut Ui) -> TreeRowShown {
        let TreeRow { line, sense } = self;
        let row = line.allocate(ui, sense, None);
        let eye = (line.eye.as_ref()).map(|eye| {
            let rect = eye_rect(ui, row.rect);
            eye_button(ui, rect, row.response.id, eye)
        });
        let (enabled, selected) = (ui.is_enabled(), line.marked);
        // (A row that takes no click, as [`TreeRow::dim`]'s, is a label.)
        if !sense.senses_click() {
            row.response
                .widget_info(|| WidgetInfo::labeled(WidgetType::Label, enabled, line.label()));
        } else {
            row.response.widget_info(|| {
                WidgetInfo::selected(WidgetType::SelectableLabel, enabled, selected, line.label())
            });
        }
        let rect = row.rect;
        let arrow = Rect::from_min_size(
            pos2(rect.left() + ROW_PADDING_LEFT, rect.top()),
            vec2(ARROW, rect.height()),
        );
        let after = pos2(row.name_end(line.icon.is_some()) + ROW_GAP, rect.top());
        let response = line.paint(ui, row, None).response;
        if let (Some(e), Some(r)) = (&line.eye, &eye) {
            paint_eye(ui, r, e, line.marked);
        }
        TreeRowShown { response, arrow, after, eye }
    }
}

/// A [`TreeRow`] drawn ([`TreeRow::show_parts`]).
#[derive(Clone, Debug)]
pub struct TreeRowShown {
    /// The row's response: the whole row's.
    pub response: Response,
    /// Its arrow's column, as high as the row (left empty by the row).
    pub arrow: Rect,
    /// Where the room after its name begins, at the row's top.
    pub after: Pos2,
    /// Its eye's response, where it has one ([`TreeRow::eye`]).
    pub eye: Option<Response>,
}
