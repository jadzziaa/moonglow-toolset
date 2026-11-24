//! The Bottom Dock's tables of results (M4's U28): Search Results' Find
//! Instance hits, Hak Pak Conflict Analysis' resources and the Claims
//! Table's areas, on the 2DA grid's look (`table_view::look`): a header row
//! whose heads sort (a click by that column, again the other way, an arrow
//! saying which), rows a row high of which only those in sight are laid
//! out, the one chosen filled and ringed while the table has the keys. A
//! click chooses a row and gives the table the keys; Up, Down, Page Up,
//! Page Down, Home and End choose another and bring it into sight, Enter
//! and a double click do the row's action, Ctrl + C copies its words.
//!
//! The order the rows are listed in (sorted, filtered) is made again only
//! when what they are of or the sort changes ([`Table::order`]): nothing is
//! sorted or walked each frame.

use std::cmp::Ordering;
use std::sync::Arc;

use egui::Ui;

use crate::palette_view::Keys;
use crate::table_view::look;

/// A column of a table: its head, its width (none: it shares what the
/// others leave, by `share`), and whether its words stand at its right.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Column {
    pub name: &'static str,
    pub width: Option<f32>,
    pub share: f32,
    pub right: bool,
    pub sorts: bool,
    /// A shared column's least width (narrower, the table scrolls across).
    pub least: f32,
}

impl Column {
    /// A column `width` wide.
    pub(crate) const fn fixed(name: &'static str, width: f32) -> Column {
        Column { name, width: Some(width), share: 0.0, right: false, sorts: true, least: width }
    }

    /// A column that takes `share` of what the fixed ones leave.
    pub(crate) const fn shared(name: &'static str, share: f32) -> Column {
        Column { name, width: None, share, right: false, sorts: true, least: SHARED_LEAST }
    }

    /// The same, at least `least` wide.
    pub(crate) const fn at_least(mut self, least: f32) -> Column {
        self.least = least;
        self
    }

    /// The least width of `cols` side by side.
    pub(crate) fn least_of(cols: &[Column]) -> f32 {
        cols.iter().map(|c| c.width.unwrap_or(c.least)).sum()
    }

    /// The same, not sorted by (a column of buttons).
    pub(crate) const fn unsorted(mut self) -> Column {
        self.sorts = false;
        self
    }
}

/// The columns' ranges across `width` from `left`: the fixed ones as they
/// are (held to the room where it is less), the rest shared.
pub(crate) fn columns(
    cols: &[Column],
    left: f32,
    width: f32,
) -> Vec<std::ops::RangeInclusive<f32>> {
    let fixed: f32 = cols.iter().filter_map(|c| c.width).sum();
    let shares: f32 = cols.iter().filter(|c| c.width.is_none()).map(|c| c.share).sum();
    let rest = (width - fixed).max(0.0);
    let mut x = left;
    cols.iter()
        .map(|c| {
            let w = match c.width {
                Some(w) => w,
                None if shares > 0.0 => (rest * c.share / shares).floor(),
                None => 0.0,
            };
            let r = x..=x + w;
            x += w;
            r
        })
        .collect()
}

/// What a table's order was made for: the caller's key, the column sorted
/// by and whether the other way.
type OrderKey = (u64, usize, bool);

/// A table's state: the sort, the row chosen and the order made.
#[derive(Debug, Clone, Default)]
pub(crate) struct Table {
    /// The column sorted by, and whether the other way.
    pub sort: usize,
    pub descending: bool,
    /// The row chosen, by its place in what the table is of (not in its
    /// order).
    pub selected: Option<usize>,
    /// The order made, for what (the caller's key, the sort).
    order: Option<(OrderKey, Arc<Vec<usize>>)>,
    /// The row last brought into sight, and in which order.
    scrolled: Option<(usize, OrderKey)>,
    /// The rows laid out at the last frame (their places in the order).
    seen: (usize, usize),
    /// How many times the order was made (the tests': a frame that changes
    /// nothing makes none).
    builds: u64,
}

impl Table {
    /// The rows' order: those of `0..len` that `keep` keeps, sorted by
    /// `cmp` in the table's column and way (rows alike stay in their own
    /// order). Made again only when `key` (what they are of: a revision,
    /// a filter) or the sort is not what it was made for.
    pub(crate) fn order(
        &mut self,
        key: u64,
        len: usize,
        keep: impl Fn(usize) -> bool,
        cmp: impl Fn(usize, usize, usize) -> Ordering,
    ) -> Arc<Vec<usize>> {
        let made_for = (key, self.sort, self.descending);
        if let Some((at, order)) = &self.order
            && *at == made_for
        {
            return order.clone();
        }
        let mut order: Vec<usize> = (0..len).filter(|i| keep(*i)).collect();
        let (sort, down) = (self.sort, self.descending);
        order.sort_by(|a, b| {
            let o = cmp(*a, *b, sort);
            if down { o.reverse() } else { o }
        });
        let order = Arc::new(order);
        self.builds += 1;
        self.order = Some((made_for, order.clone()));
        order
    }

    /// How many times the order was made.
    pub(crate) fn builds(&self) -> u64 {
        self.builds
    }

    /// The rows laid out at the last frame, by their places in the order.
    pub(crate) fn seen(&self) -> (usize, usize) {
        self.seen
    }

    /// The row chosen let go of, and the order forgotten (what it is of
    /// is new).
    pub(crate) fn clear(&mut self) {
        self.selected = None;
        self.order = None;
        self.scrolled = None;
    }
}

/// What a table did this frame.
#[derive(Debug, Default)]
pub(crate) struct Shown {
    /// A row's action asked for (Enter, a double click): its place in what
    /// the table is of.
    pub activated: Option<usize>,
    /// A row clicked (chosen).
    pub clicked: Option<usize>,
    /// The table's rect (its header and rows).
    pub rect: Option<egui::Rect>,
}

/// What a table is asked to draw: its name (a screen reader's, and the
/// tests'), its columns, the keys it takes, and its rows' height.
pub(crate) struct Spec<'a> {
    pub id: egui::Id,
    pub name: &'a str,
    pub cols: &'a [Column],
    pub keys: Keys,
    /// Whether the keys may act (its panel is in front of them: no menu,
    /// popover or dialog over it).
    pub may_take_keys: bool,
}

/// Whether the keys are the table's: they were given to it (a click, F6,
/// its command) and nothing modal, no menu and no popover is in front,
/// and no text field has the keyboard (what is typed there is the
/// field's). A button with the keyboard's focus does not keep them from
/// it (a head reached by Tab from a form, M4's U28 review: egui counts
/// any focus as wanting the keys, and the rows took none); where the
/// table takes a key that moves its row, that focus lets go, or egui
/// would move it to the next widget that way too.
pub(crate) fn has_keys(ctx: &egui::Context, keys: Keys) -> bool {
    let has = crate::palette_view::keys_are(ctx, keys)
        && !crate::shell::keys_blocked(ctx)
        && !mg_uikit::popover::Popover::any_open(ctx)
        && !text_has_keys(ctx);
    // (A popup open as the frame began takes this frame's keys, though it
    // has closed by now: Enter that chose in a form's combo box list went
    // to the row under it too, M4's U29 review.)
    if has && crate::widgets::popup_was_open(ctx) {
        return false;
    }
    let left = egui::Id::new(FOCUS_LEFT);
    if !has {
        // (Its rows no longer have the keys: Tab goes as egui sends it.)
        let of = ctx.data(|d| d.get_temp::<Left>(left));
        if of.is_some_and(|(of, ..)| of == keys) {
            ctx.data_mut(|d| d.remove::<Left>(left));
        }
        return false;
    }
    if let Some(focused) = ctx.memory(|m| m.focused()) {
        use egui::Key;
        let moving =
            [Key::ArrowUp, Key::ArrowDown, Key::PageUp, Key::PageDown, Key::Home, Key::End];
        if ctx.input(|i| moving.iter().any(|k| i.key_pressed(*k))) {
            ctx.memory_mut(|m| m.surrender_focus(focused));
            ctx.data_mut(|d| d.insert_temp::<Left>(left, (keys, focused, false)));
        }
    }
    true
}

const FOCUS_LEFT: &str = "report-table-focus-left";

/// The control a list's rows took the keys from: its keys, the control,
/// and whether Shift + Tab comes back to that control itself (a form's
/// field left by Enter) rather than to the one before it (a head).
type Left = (Keys, egui::Id, bool);

/// A form's field `id` gave the keys to `keys`' rows as its Enter
/// searched (egui lets a one-line field's focus go on Enter): Shift + Tab
/// from the rows comes back to it, Tab goes on from it (M4's U28 review).
pub(crate) fn rows_took_keys_from_field(ctx: &egui::Context, keys: Keys, id: egui::Id) {
    ctx.data_mut(|d| d.insert_temp::<Left>(egui::Id::new(FOCUS_LEFT), (keys, id, true)));
}

/// Tab or Shift + Tab while a table's rows hold the keys they took from
/// a control (its head, reached from a form): the focus goes on from
/// that control, back into the form or on past the table, not from the
/// window's first or last control (M4's U28 review). Called before any
/// control is drawn, as egui hands Tab on while widgets are laid out.
pub(crate) fn tab_from_the_rows(ctx: &egui::Context) {
    let left = egui::Id::new(FOCUS_LEFT);
    if ctx.memory(|m| m.focused()).is_some() {
        ctx.data_mut(|d| d.remove::<Left>(left));
        return;
    }
    if !ctx.input(|i| i.key_pressed(egui::Key::Tab)) {
        return;
    }
    let was = ctx.data_mut(|d| {
        let was = d.get_temp::<Left>(left);
        d.remove::<Left>(left);
        was
    });
    if let Some((keys, id, itself)) = was
        && crate::palette_view::keys_are(ctx, keys)
    {
        let back = ctx.input(|i| i.modifiers.shift);
        ctx.memory_mut(|m| {
            m.request_focus(id);
            if itself && back {
                // (On the field itself, not the control before it.)
                m.move_focus(egui::FocusDirection::None);
            }
        });
    }
}

/// Whether a text field has the keyboard (its edit state is kept under
/// the focused widget's id).
pub(crate) fn text_has_keys(ctx: &egui::Context) -> bool {
    let focused = ctx.memory(|m| m.focused());
    focused.is_some_and(|id| egui::text_edit::TextEditState::load(ctx, id).is_some())
}

/// A table of `order`'s rows in `ui`'s room (its header row, then the rows
/// in sight): `words(i)` a row's name and value as a screen reader says
/// them and Ctrl + C copies them (as a row's menu's Copy does, a right
/// click or Shift+F10 on the row chosen opening it, and its Copy All every
/// row in the order shown); `paint(ui, i, cells, selected)` draws a row's
/// cells (`cells` its columns' rects; `ui` a child of the row's room for a
/// cell's own controls, the Claims Table's buttons).
///
/// Narrower than its columns need (the fixed ones and a least share of
/// [`SHARED_LEAST`] for each other one: the least window), the table
/// scrolls across, whole, rather than cut its columns to nothing.
pub(crate) fn show(
    ui: &mut Ui,
    table: &mut Table,
    spec: &Spec,
    order: &[usize],
    words: impl Fn(usize) -> (String, String),
    paint: impl FnMut(&mut Ui, usize, &[egui::Rect], bool),
) -> Shown {
    let least = Column::least_of(spec.cols);
    if ui.available_width() >= least {
        return show_in(ui, table, spec, order, words, paint);
    }
    let high = ui.available_height();
    egui::ScrollArea::horizontal()
        .id_salt(spec.id.with("across"))
        .auto_shrink([false, true])
        .show(ui, |ui| {
            let room = egui::vec2(least, high - ui.spacing().scroll.bar_width);
            let builder =
                egui::UiBuilder::new().max_rect(egui::Rect::from_min_size(ui.cursor().min, room));
            ui.scope_builder(builder, |ui| {
                ui.set_min_size(room);
                show_in(ui, table, spec, order, words, paint)
            })
            .inner
        })
        .inner
}

/// The least width of a column that shares the room (a narrow table
/// scrolls across rather than give it less).
const SHARED_LEAST: f32 = 90.0;

fn show_in(
    ui: &mut Ui,
    table: &mut Table,
    spec: &Spec,
    order: &[usize],
    words: impl Fn(usize) -> (String, String),
    mut paint: impl FnMut(&mut Ui, usize, &[egui::Rect], bool),
) -> Shown {
    let ctx = ui.ctx().clone();
    mg_uikit::name_group(ui, spec.name);
    let t = mg_uikit::tokens(&ctx);
    let mut shown = Shown::default();
    let has = spec.may_take_keys && has_keys(&ctx, spec.keys);
    let pitch = mg_uikit::row_metrics(ui).1;
    // A row as Ctrl+C, its menu's Copy and a line of Copy All copy it.
    let line = |r: usize| {
        let (name, value) = words(r);
        if value.is_empty() { name } else { format!("{name}\t{value}") }
    };
    let mut keyed = false;
    // The keys: the row chosen moved, its action, its words copied.
    if has && !order.is_empty() {
        use egui::Key;
        let last = order.len() as isize - 1;
        let page = (((ui.available_height() - look::HEADER_HIGH) / pitch) as isize - 1).max(1);
        let steps = [
            (Key::ArrowDown, 1),
            (Key::ArrowUp, -1),
            (Key::PageDown, page),
            (Key::PageUp, -page),
            (Key::End, isize::MAX / 2),
            (Key::Home, isize::MIN / 2),
        ];
        let pressed = ui.input_mut(|i| {
            steps.into_iter().find(|(k, _)| i.consume_key(egui::Modifiers::NONE, *k))
        });
        if let Some((_, by)) = pressed {
            let at = table.selected.and_then(|s| order.iter().position(|r| *r == s));
            // (From none chosen: Down, Page Down and End as from before the
            // first row, the others as from past the last.)
            let from = at.map_or(if by > 0 { -1 } else { last + 1 }, |at| at as isize);
            let to = from.saturating_add(by).clamp(0, last) as usize;
            table.selected = Some(order[to]);
        }
        if let Some(s) = table.selected {
            if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::Enter)) {
                shown.activated = Some(s);
            }
            // (Ctrl+C alone: Ctrl+Alt+C is the Creature Wizard's.)
            if crate::row_copy::copy_pressed(ui) {
                ctx.copy_text(line(s));
            }
            // (Shift+F10: the chosen row's menu, brought into sight.)
            keyed = crate::row_copy::menu_key(ui);
            if keyed {
                table.scrolled = None;
            }
        }
    }
    // The header row: a head a column, a click sorting by it.
    // (Never wider than its room: a narrow pane cuts the shared columns'
    // words, each whole in the row's tip and its name.)
    let width = ui.available_width();
    let (head, _) =
        ui.allocate_exact_size(egui::vec2(width, look::HEADER_HIGH), egui::Sense::hover());
    let font = egui::TextStyle::Body.resolve(ui.style());
    let mut sorted = None;
    for (i, (x, col)) in
        columns(spec.cols, head.left(), width).into_iter().zip(spec.cols).enumerate()
    {
        let cell = egui::Rect::from_x_y_ranges(x, head.y_range());
        look::header_cell(ui, cell);
        let on = table.sort == i && col.sorts;
        let color = if on { t.text } else { t.text_2 };
        if col.sorts {
            let r = ui.interact(cell, spec.id.with(("head", i)), egui::Sense::click());
            let said = match (on, table.descending) {
                (true, false) => format!("{}, sorted up", col.name),
                (true, true) => format!("{}, sorted down", col.name),
                (false, _) => col.name.to_string(),
            };
            r.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &said));
            let r = r.on_hover_text(match on {
                true => format!("Sort by {} the other way", col.name.to_lowercase()),
                false => format!("Sort by {}", col.name.to_lowercase()),
            });
            if r.clicked() {
                sorted = Some(i);
            }
        }
        let inner = cell.shrink2(egui::vec2(8.0, 0.0));
        let arrow = if on { 16.0 } else { 0.0 };
        // (One line, cut with an ellipsis where the column is narrow: a
        // wrapped head ran under the header row, M4's U28 review.)
        let mut job =
            egui::text::LayoutJob::simple_singleline(col.name.to_string(), font.clone(), color);
        job.wrap = egui::text::TextWrapping::truncate_at_width((inner.width() - arrow).max(1.0));
        let galley = ui.painter().layout_job(job);
        let x = match col.right {
            true => inner.right() - galley.size().x - arrow,
            false => inner.left(),
        };
        let w = galley.size().x;
        let at = egui::pos2(x, cell.center().y - galley.size().y / 2.0);
        ui.painter().with_clip_rect(cell).galley(at, galley, color);
        if on {
            let icon = match table.descending {
                true => mg_uikit::Icon::ArrowDown,
                false => mg_uikit::Icon::ArrowUp,
            };
            let r = egui::Rect::from_center_size(
                egui::pos2(x + w + 10.0, cell.center().y),
                egui::vec2(12.0, 12.0),
            );
            icon.paint(&ui.painter().with_clip_rect(cell), r, t.accent);
        }
    }
    if let Some(sort) = sorted {
        table.descending = table.sort == sort && !table.descending;
        table.sort = sort;
    }
    // The row chosen brought into sight where the keys (or the code)
    // chose it, or the order changed under it.
    let made_for = table.order.as_ref().map_or((0, 0, false), |(k, _)| *k);
    let mut scroll = egui::ScrollArea::vertical()
        .id_salt(spec.id.with("rows"))
        .auto_shrink([false, false])
        // (No taller than its room: egui's least would lay rows out past
        // the dock's bottom.)
        .min_scrolled_height(0.0);
    if let Some(s) = table.selected
        && table.scrolled != Some((s, made_for))
    {
        table.scrolled = Some((s, made_for));
        if let Some(at) = order.iter().position(|r| *r == s) {
            let (from, end) = table.seen;
            if at < from || at + 1 >= end || end == 0 {
                let room = ui.available_height();
                scroll = scroll.vertical_scroll_offset((at as f32 * pitch - room / 2.0).max(0.0));
            }
        }
    }
    let selected = table.selected;
    let mut seen = (0, 0);
    crate::shell::first_row_ends(ui, ui.cursor().top() + pitch);
    let out = mg_uikit::show_rows(scroll, ui, order.len(), |ui, range| {
        seen = (range.start, range.end);
        ui.spacing_mut().item_spacing.y = 0.0;
        let mut lines = look::Lines::default();
        let mut clicked = None;
        let mut double = None;
        let mut copy = None;
        let mut cells: Vec<egui::Rect> = Vec::with_capacity(spec.cols.len());
        let start_left = ui.cursor().left();
        let xs = columns(spec.cols, start_left, width);
        for &r in &order[range] {
            let (rect, resp) =
                ui.allocate_exact_size(egui::vec2(width, pitch), egui::Sense::click());
            let on = selected == Some(r);
            resp.widget_info(|| {
                let (name, value) = words(r);
                let mut info =
                    egui::WidgetInfo::selected(egui::WidgetType::SelectableLabel, true, on, &name);
                if !value.is_empty() {
                    info.current_text_value = Some(value);
                }
                info
            });
            let state = match (on, resp.hovered()) {
                (true, _) => look::Cell::Current { ringed: has },
                (false, true) => look::Cell::Hovered,
                (false, false) => look::Cell::Plain,
            };
            look::cell(ui.painter(), &t, rect, state);
            cells.clear();
            if (rect.left() - start_left).abs() < f32::EPSILON {
                cells.extend(
                    xs.iter().map(|x| egui::Rect::from_x_y_ranges(x.clone(), rect.y_range())),
                );
            } else {
                cells.extend(
                    columns(spec.cols, rect.left(), rect.width())
                        .into_iter()
                        .map(|x| egui::Rect::from_x_y_ranges(x, rect.y_range())),
                );
            }
            for c in &cells {
                lines
                    .cell(*c, if state == look::Cell::Plain { state } else { look::Cell::Hovered });
            }
            if let look::Cell::Current { ringed: true } = state {
                lines.cell(rect, state);
            }
            lines.row(rect.left(), rect.right(), rect.bottom());
            let mut row = ui.new_child(egui::UiBuilder::new().max_rect(rect).id_salt(("row", r)));
            row.set_clip_rect(rect.intersect(ui.clip_rect()));
            paint(&mut row, r, &cells, on);
            // (A right click chooses the row too, so that Ctrl+C after its
            // menu copies the row it was opened on.)
            if resp.clicked() || resp.secondary_clicked() {
                clicked = Some(r);
            }
            // (A third click soon after is a double click too: egui counts
            // it a triple, which would leave the row unopened.)
            if resp.double_clicked() || resp.triple_clicked() {
                double = Some(r);
            }
            // Its menu: Copy and Copy All (in the order shown).
            copy = match crate::row_copy::row_menu(ui, &resp, keyed && on, |_| false) {
                Some(crate::row_copy::Asked::Row) => Some(line(r)),
                Some(crate::row_copy::Asked::All) => {
                    Some(crate::row_copy::lines(order.iter().map(|r| line(*r))))
                }
                None => copy,
            };
        }
        lines.paint(ui.painter(), &t);
        if let Some(text) = copy {
            ctx.copy_text(text);
        }
        (clicked, double)
    });
    table.seen = seen;
    let (clicked, double) = out.inner;
    if let Some(r) = clicked {
        table.selected = Some(r);
        // (Not brought into sight: it is in sight, where it was clicked.)
        table.scrolled = Some((r, made_for));
        crate::palette_view::give_keys(&ctx, spec.keys);
        shown.clicked = Some(r);
    }
    if double.is_some() {
        shown.activated = double;
    }
    shown.rect = Some(head.union(out.inner_rect));
    shown
}

/// Text painted in a cell, cut at its edge with an ellipsis, at its left
/// (or its right), in `color`.
pub(crate) fn text(
    ui: &Ui,
    cell: egui::Rect,
    words: impl Into<String>,
    color: egui::Color32,
    right: bool,
) {
    let inner = cell.shrink2(egui::vec2(8.0, 0.0));
    let font = egui::TextStyle::Body.resolve(ui.style());
    let mut job = egui::text::LayoutJob::simple_singleline(words.into(), font, color);
    job.wrap = egui::text::TextWrapping::truncate_at_width(inner.width().max(1.0));
    let galley = ui.painter().layout_job(job);
    let y = cell.center().y - galley.size().y / 2.0;
    let x = if right { inner.right() - galley.size().x } else { inner.left() };
    ui.painter().with_clip_rect(cell).galley(egui::pos2(x, y), galley, color);
}

/// Two words compared as a builder reads a list: case aside.
pub(crate) fn by_words(a: &str, b: &str) -> Ordering {
    if a.is_ascii() && b.is_ascii() {
        a.bytes()
            .map(|c| c.to_ascii_lowercase())
            .cmp(b.bytes().map(|c| c.to_ascii_lowercase()))
            .then_with(|| a.cmp(b))
    } else {
        a.to_lowercase().cmp(&b.to_lowercase()).then_with(|| a.cmp(b))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The order is made once for a key and a sort, and again when either
    /// changes; rows alike keep their order; the way turns it.
    #[test]
    fn the_order_is_made_again_only_when_its_key_or_sort_changes() {
        let names = ["b", "a", "c", "a"];
        let mut t = Table::default();
        let cmp = |a: usize, b: usize, _| names[a].cmp(names[b]);
        let o = t.order(1, names.len(), |_| true, cmp);
        assert_eq!(*o, vec![1, 3, 0, 2]);
        let _ = t.order(1, names.len(), |_| true, cmp);
        assert_eq!(t.builds(), 1, "the same key and sort: kept");
        t.descending = true;
        let o = t.order(1, names.len(), |_| true, cmp);
        assert_eq!(*o, vec![2, 0, 1, 3], "the other way");
        let o = t.order(2, names.len(), |i| names[i] != "a", cmp);
        assert_eq!(*o, vec![2, 0]);
        assert_eq!(t.builds(), 3);
    }

    /// The columns share what the fixed ones leave, and never go wider
    /// than they are given.
    #[test]
    fn the_columns_share_the_room_the_fixed_ones_leave() {
        let cols = [Column::fixed("A", 100.0), Column::shared("B", 1.0), Column::shared("C", 3.0)];
        let x = columns(&cols, 10.0, 500.0);
        assert_eq!(x[0], 10.0..=110.0);
        assert_eq!(x[1], 110.0..=210.0);
        assert_eq!(x[2], 210.0..=510.0);
    }
}
