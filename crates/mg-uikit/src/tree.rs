//! A tree's rows as the mock draws them (`.tree-row`): a group's row
//! ([`TreeGroup`]: its arrow, its kind's icon, its name and its count
//! right-aligned, over its rows, which open and close under it), and a
//! row that opens nothing ([`TreeRow`]: the arrow's column left empty).
//! Each is [`metrics::ROW_HEIGHT`] high and as wide as what holds it, its
//! fill drawn across the whole row: `hover-row` under the pointer,
//! `accent-fill` marked (its words and icons then `on-fill`, the count at
//! 85%).
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

/// Where a row's name begins, from the row's left edge: after the arrow's
/// column, and after the icon where it has one (42 and 22).
pub fn name_offset(icon: bool) -> f32 {
    let icon = if icon { metrics::ICON + ROW_GAP } else { 0.0 };
    ROW_PADDING_LEFT + ARROW + ROW_GAP + icon
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
}

impl Line {
    fn new(name: String) -> Self {
        Line { name, icon: None, count: None, label: None, marked: false, dim: false }
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
        let words = ui.is_rect_visible(whole).then(|| Words {
            name: Self::galley(ui, &self.name),
            count: self.count.as_ref().map(|c| Self::galley(ui, c)),
        });
        let needed = words.as_ref().map_or(0.0, |w| {
            let count = w.count.as_ref().map_or(0.0, |c| COUNT_GAP + c.size().x);
            name_offset(self.icon.is_some()) + w.name.size().x + count + ROW_PADDING_RIGHT
        });
        let (auto, taken) = ui.allocate_space(vec2(needed, height));
        let rect = taken.with_max_x(taken.right().max(right));
        let response = ui.interact(rect, id.unwrap_or(auto), sense);
        Allocated { rect, taken, response, words }
    }

    fn galley(ui: &Ui, text: &str) -> std::sync::Arc<egui::Galley> {
        WidgetText::from(text).into_galley(
            ui,
            Some(TextWrapMode::Extend),
            f32::INFINITY,
            TextStyle::Button,
        )
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
            p.rect_filled(rect, 0.0, fill);
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
        let name_end = x + name.size().x;
        p.galley(Pos2::new(x, (middle - name.size().y / 2.0).round()), name, text);
        if let Some(galley) = words.count.clone() {
            // (Against the edge in sight: the tree scrolled sideways keeps
            // its counts at the panel's right.)
            let right = rect.right().min(ui.clip_rect().right()) - ROW_PADDING_RIGHT;
            let x = (right - galley.size().x).max(name_end + COUNT_GAP);
            p.galley(Pos2::new(x, (middle - galley.size().y / 2.0).round()), galley, weak);
        }
        row
    }
}

/// A row's words laid out: its name, and its count.
struct Words {
    name: std::sync::Arc<egui::Galley>,
    count: Option<std::sync::Arc<egui::Galley>>,
}

/// A row's room taken ([`Line::allocate`]).
struct Allocated {
    /// The whole row, as drawn.
    rect: Rect,
    /// The room it took.
    taken: Rect,
    response: Response,
    /// Its words, where it is in sight.
    words: Option<Words>,
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

    /// Its row and its rows under it (`add_body`, while it is open).
    pub fn show<R>(
        self,
        ui: &mut Ui,
        add_body: impl FnOnce(&mut Ui) -> R,
    ) -> CollapsingResponse<R> {
        let TreeGroup { id_salt, line, default_open, open } = self;
        ui.vertical(|ui| {
            let id = ui.make_persistent_id(id_salt);
            let mut row = line.allocate(ui, Sense::click(), Some(id));
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
            let header_response = row.response;
            // (Its rows as wide as the room the row took, not as the row
            // is drawn: `allocate`.)
            let taken = header_response.clone().with_new_rect(row.taken);
            let body = state.show_body_indented(&taken, ui, add_body);
            crate::say_expanded(&header_response, body.is_some());
            CollapsingResponse {
                header_response,
                body_response: body.as_ref().map(|b| b.response.clone()),
                body_returned: body.map(|b| b.inner),
                openness,
            }
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

    /// Draws the row; its response.
    pub fn show(self, ui: &mut Ui) -> Response {
        let TreeRow { line, sense } = self;
        let row = line.allocate(ui, sense, None);
        let (enabled, selected) = (ui.is_enabled(), line.marked);
        if line.dim {
            row.response
                .widget_info(|| WidgetInfo::labeled(WidgetType::Label, enabled, line.label()));
        } else {
            row.response.widget_info(|| {
                WidgetInfo::selected(WidgetType::SelectableLabel, enabled, selected, line.label())
            });
        }
        line.paint(ui, row, None).response
    }
}
