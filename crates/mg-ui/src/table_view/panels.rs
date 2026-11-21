//! The 2DA editor's panels (M4's U16, the Table board): the Right Dock's
//! **Cell** (the selected cell's value, typed in, its list's or flags'
//! choice, the choosers its column's kind has, its kind's words and the
//! "Described by" note) and **References** (Find References to Row N, run
//! on a thread of its own: the game's tables' cells and the module's
//! fields and scripts that name the row), and what the Outliner lists of
//! a table (its columns, each with its kind's icon and words and an eye
//! that hides it as Hide Column does: `outliner::columns`). Drawn before
//! the table's grid (the docks first; a floating table's window's panes
//! before its editor), they show what the grid left the frame before, and
//! the frame after shows what the grid changed (no case was found where
//! none follows: the stamps U16 first drew them with were removed at its
//! review, as U15's were). The Status Bar's counts and hints are
//! [`shown`].
//!
//! A selection of several cells: Cell shows and edits the cell with the
//! keys (the one outlined, where the selection ends), as the board draws
//! it beside "6 cells" in the Status Bar; Delete, Ctrl+C and Ctrl+V in the
//! grid act on the whole of it.

use std::hash::{Hash, Hasher};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use egui::Ui;
use mg_module::doctor::ColumnKind;
use mg_module::rename::Usage;
use mg_module::table_desc::{Description, Kind};
use mg_uikit::Icon;

use super::{Reference, TableDoc};
use crate::{Action, Moonglow, Tab};

/// What is typed in the Cell panel's Value, kept by its cell (its row and
/// column as they were at `since`, the table's count of changes): set in
/// that cell as the field is left, followed through the rows and columns
/// numbered anew meanwhile (`Editing::follow`), let go of where its cell
/// is gone.
#[derive(Debug, Clone)]
pub(crate) struct CellTyped {
    pub(crate) cell: (usize, usize),
    pub(crate) since: u64,
    pub(crate) text: String,
}

/// Find References to Row being run on a thread of its own: the row, the
/// flag that stops it (asked between tables and between resources), and
/// what it finds.
#[derive(Debug)]
pub(crate) struct Looking {
    row: usize,
    stop: Arc<AtomicBool>,
    /// (Taken as it is joined.)
    #[allow(clippy::type_complexity)]
    found: Option<std::thread::JoinHandle<Option<(Vec<Reference>, Vec<Usage>)>>>,
}

impl Looking {
    /// Stops it: the thread ends at the next table or resource, what it
    /// found let go of.
    pub(crate) fn stop(self) {
        drop(self);
    }
}

/// (Let go of anyhow, its table closed with it: stopped, not left running
/// through a large module for no one.)
impl Drop for Looking {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

/// What the Outliner shows of the table's columns: their names (the
/// table's changes), which are hidden, the selected cell's column, and
/// whether the description is read (read again, the table's revision
/// moves). (The game's kinds go by the table's name, which is its own for
/// as long as it is open: Save As File… changes where it is kept.)
pub(crate) fn columns_mark(doc: &TableDoc) -> u64 {
    let mut h = std::hash::DefaultHasher::new();
    (doc.edit.changes(), &doc.hidden, doc.selected.map(|s| s.1)).hash(&mut h);
    (doc.described.as_ref().map(|d| d.read.is_ok()), doc.revision).hash(&mut h);
    h.finish()
}

/// Sets what was typed in the Cell panel in its own cell, wherever that
/// cell is now; nothing where it is gone.
pub(crate) fn commit_typed(doc: &mut TableDoc) {
    let Some(typed) = doc.cell_typed.take() else { return };
    let Some((row, column)) = doc.edit.follow(typed.cell, typed.since) else { return };
    let value = Some(typed.text.trim().to_string()).filter(|v| !v.is_empty());
    match doc.edit.set(row, column, value) {
        Ok(true) => doc.changed(),
        Ok(false) => {}
        Err(e) => doc.problem = Some(e.to_string()),
    }
}

/// The Cell panel's Value field's id: its table's and its cell's, so that
/// what is typed in one cell's field is never shown or set in another.
fn value_id(table: u32, (row, column): (usize, usize)) -> egui::Id {
    egui::Id::new(("table-cell-value", table, row, column))
}

/// What is typed in the Cell panel follows its cell where rows or columns
/// were numbered anew since it was typed (a row taken away above it, its
/// Undo, a column moved, a change made elsewhere): the text, its cursor and
/// the keyboard's focus go to the field of the cell's new place, and the
/// selection with them; where the cell is gone they are let go of, never
/// set in the cell that took its number.
pub(crate) fn follow_typed(doc: &mut TableDoc, ctx: &egui::Context) {
    let changes = doc.edit.changes();
    let Some(typed) = doc.cell_typed.as_mut() else { return };
    if typed.since == changes {
        return;
    }
    let (was, id) = (typed.cell, doc.id);
    let old = value_id(id, was);
    match doc.edit.follow(was, typed.since) {
        Some(now) => {
            typed.cell = now;
            typed.since = changes;
            if now != was {
                let new = value_id(id, now);
                if let Some(state) = egui::text_edit::TextEditState::load(ctx, old) {
                    state.store(ctx, new);
                }
                if ctx.memory(|m| m.has_focus(old)) {
                    ctx.memory_mut(|m| m.request_focus(new));
                }
                if doc.selected == Some(was) {
                    doc.selected = Some(now);
                    doc.anchor = None;
                }
            }
        }
        None => {
            doc.cell_typed = None;
            ctx.memory_mut(|m| m.surrender_focus(old));
        }
    }
}

/// A table's panel begins: outlined where F6 gave it the keys, a press in
/// it giving the grid the keys (the arrows, Delete, Ctrl+C and Undo stay
/// the table's after a click on a flag, as they were under its grid).
/// Whether F6 came to it (`Moonglow::dialog_panel_focus`, taken by this
/// panel alone): its first control takes the keyboard's focus.
fn begin_panel(app: &mut Moonglow, ui: &Ui, id: u32, keys: crate::palette_view::Keys) -> bool {
    crate::palette_view::keys_outline(ui, ui.max_rect(), keys);
    // (F6 here: Undo and Redo are the table's, as with its grid's keys.)
    if crate::palette_view::keys_are(ui.ctx(), keys) {
        app.undo_keys = Some(Tab::Table(id));
    }
    if ui.input(|i| i.pointer.any_pressed()) && ui.rect_contains_pointer(ui.max_rect()) {
        super::take_keys(ui.ctx(), id);
    }
    let to = app.dialog_panel_focus == Some(keys);
    if to {
        app.dialog_panel_focus = None;
    }
    to
}

/// The table an editor shown in the docks or its window is: its place
/// among the open tables, where `editor` is a table's.
fn table_at(app: &Moonglow, editor: Option<&Tab>) -> Option<usize> {
    let Some(Tab::Table(id)) = editor else { return None };
    app.tables.iter().position(|d| d.id == *id)
}

/// The Cell panel's header's detail (the board's "Row 9, TargetType"),
/// and the References panel's ("Row 9, Bulls_Strength").
pub(crate) fn detail(app: &Moonglow, editor: Option<&Tab>, references: bool) -> Option<String> {
    let doc = &app.tables[table_at(app, editor)?];
    let Some((row, column)) = doc.selected else {
        return Some("No cell selected".to_string());
    };
    let t = doc.edit.table();
    Some(match references {
        false => format!("Row {row}, {}", t.columns().get(column).map_or("", String::as_str)),
        true => match row_name(t, row) {
            Some(name) => format!("Row {row}, {name}"),
            None => format!("Row {row}"),
        },
    })
}

/// A row's name: its Label's (or Name's) cell, else its first that says
/// something.
fn row_name(t: &mg_2da::TwoDa, row: usize) -> Option<String> {
    let named = ["Label", "LABEL", "Name", "NAME"].iter().find_map(|c| t.get(row, c));
    named.or_else(|| (0..t.columns().len()).find_map(|c| t.cell(row, c))).map(str::to_string)
}

/// What a column holds, in words, as the Outliner and the Cell panel say
/// it (the board's "any of 7 flags", "a talk-table string's number"): its
/// description's, else what the checks know of the game's table of its
/// name. `None` where neither says.
pub(crate) fn kind_words(
    description: Option<&Description>,
    kinds: &[(&'static str, ColumnKind)],
    column: &str,
) -> Option<String> {
    if let Some(d) = description.and_then(|d| d.column(column)) {
        return Some(d.kind.words());
    }
    let known = kinds.iter().find(|(k, _)| k.eq_ignore_ascii_case(column)).map(|(_, k)| *k)?;
    Some(match known {
        ColumnKind::StrRef => "a talk-table string's number".to_string(),
        ColumnKind::Model => "a model's name".to_string(),
        ColumnKind::Row(t) => format!("a row of {t}.2da"),
        ColumnKind::RowBy { by, .. } => format!("a row of the table its {by} chooses"),
        ColumnKind::Table => "a table's name, without .2da".to_string(),
        ColumnKind::Script => "a script's name, which the game runs".to_string(),
    })
}

/// A column's kind's icon (the Table board's Outliner): a list's, flags',
/// a talk-table string's, a script's, a row's table's; any other the
/// column's.
pub(crate) fn kind_icon(
    description: Option<&Description>,
    kinds: &[(&'static str, ColumnKind)],
    column: &str,
) -> Icon {
    match description.and_then(|d| d.column(column)).map(|d| &d.kind) {
        Some(Kind::List { .. }) => return Icon::List,
        Some(Kind::Flags { .. }) => return Icon::Flag,
        Some(Kind::StrRef) => return Icon::TalkTable,
        Some(Kind::Script) => return Icon::Script,
        Some(Kind::Row(_)) => return Icon::Table,
        Some(_) => return Icon::Column,
        None => {}
    }
    match kinds.iter().find(|(k, _)| k.eq_ignore_ascii_case(column)).map(|(_, k)| *k) {
        Some(ColumnKind::StrRef) => Icon::TalkTable,
        Some(ColumnKind::Script) => Icon::Script,
        Some(ColumnKind::Row(_) | ColumnKind::RowBy { .. } | ColumnKind::Table) => Icon::Table,
        Some(ColumnKind::Model) => Icon::Model,
        None => Icon::Column,
    }
}

/// The panels' own room around what they hold (the board's 8 and 6).
const MARGIN: egui::Margin = egui::Margin { left: 8, right: 8, top: 6, bottom: 6 };
/// The board's label column (`.cell-form`'s 72).
const LABEL: f32 = 72.0;
/// The least a field keeps beside its label: under it, the label stands
/// over the field.
const FIELD_LEAST: f32 = 120.0;

/// A row of the form: `label` in its column (wider for wider words) and
/// what `add` draws beside it, or the label over it where the room beside
/// it is less than [`FIELD_LEAST`]: the room tested is the room given,
/// the row's gap counted, so the panel is never taller where wider.
fn form_row<R>(ui: &mut Ui, label: &str, add: impl FnOnce(&mut Ui) -> R) -> R {
    let font = egui::TextStyle::Body.resolve(ui.style());
    let words = ui.painter().layout_no_wrap(label.to_string(), font, egui::Color32::WHITE);
    let width = LABEL.max(words.size().x.ceil() + 8.0);
    let gap = ui.spacing().item_spacing.x;
    let t = mg_uikit::tokens(ui.ctx());
    let text = egui::RichText::new(label).color(t.text_2);
    if ui.available_width() - width - gap >= FIELD_LEAST {
        ui.horizontal(|ui| {
            let size = egui::vec2(width, mg_uikit::metrics::CONTROL_HEIGHT);
            let left = egui::Layout::left_to_right(egui::Align::Center);
            ui.allocate_ui_with_layout(size, left, |ui| {
                ui.set_min_size(size);
                ui.add(egui::Label::new(text).truncate());
            });
            add(ui)
        })
        .inner
    } else {
        ui.add(egui::Label::new(text).truncate());
        add(ui)
    }
}

/// A weak note that wraps at the panel's width.
fn note(ui: &mut Ui, text: impl Into<String>) -> egui::Response {
    let t = mg_uikit::tokens(ui.ctx());
    ui.add(egui::Label::new(egui::RichText::new(text.into()).color(t.text_3)).wrap())
}

/// A note in `color` that wraps at the panel's width.
fn said(ui: &mut Ui, text: impl Into<String>, color: egui::Color32) -> egui::Response {
    ui.add(egui::Label::new(egui::RichText::new(text.into()).color(color)).wrap())
}

/// A section's heading (`.sec-h`): the column's name, strong, and its
/// kind's words weak after it, cut to the panel's width.
fn heading(ui: &mut Ui, name: &str, words: Option<&str>) {
    let t = mg_uikit::tokens(ui.ctx());
    ui.add_space(6.0);
    let mut job = egui::text::LayoutJob::default();
    let body = egui::TextStyle::Body.resolve(ui.style());
    // (Bold, as the board's `.sec-h`: the kit's one bold, for headings.)
    let bold = egui::FontId::new(body.size, mg_uikit::fonts::bold());
    let strong = egui::TextFormat { font_id: bold, color: t.text, ..Default::default() };
    job.append(name, 0.0, strong);
    if let Some(words) = words {
        let weak = egui::TextFormat { font_id: body, color: t.text_3, ..Default::default() };
        job.append(words, 8.0, weak);
    }
    job.wrap = egui::text::TextWrapping::truncate_at_width(ui.available_width());
    let galley = ui.painter().layout_job(job);
    let (rect, r) = ui.allocate_exact_size(galley.size(), egui::Sense::hover());
    ui.painter().galley(rect.min, galley, t.text);
    // (Named apart from the column's name over the grid.)
    let said = match words {
        Some(words) => format!("{name} column: {words}"),
        None => format!("{name} column"),
    };
    r.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Label, true, &said));
    let y = ui.cursor().top() + 1.0;
    let x = ui.max_rect().x_range();
    ui.painter().hline(x, y, egui::Stroke::new(1.0, t.line_soft));
    ui.add_space(6.0);
}

/// A row of a list's or flags' choices (`.check.flag`): its box or ring,
/// its name, and its value weak in the monospace font at the right end;
/// clicked anywhere on it, it changes.
fn choice_row(ui: &mut Ui, on: bool, radio: bool, name: &str, value: &str) -> egui::Response {
    let t = mg_uikit::tokens(ui.ctx());
    let height = mg_uikit::row_height(ui);
    // (A click on it, by the pointer or a screen reader; no stop of Tab of
    // its own: its list is one, [`choice_list`].)
    let (rect, r) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), height), egui::Sense::CLICK);
    let kind = if radio { egui::WidgetType::RadioButton } else { egui::WidgetType::Checkbox };
    r.widget_info(|| egui::WidgetInfo::selected(kind, ui.is_enabled(), on, name));
    if r.hovered() {
        ui.painter().rect_filled(rect, 2.0, t.hover_row);
    }
    let mono = egui::TextStyle::Monospace.resolve(ui.style());
    let shown = ui.painter().layout_no_wrap(value.to_string(), mono, t.text_3);
    let right = rect.right() - 4.0;
    ui.painter().galley(
        egui::pos2(right - shown.size().x, rect.center().y - shown.size().y / 2.0),
        shown.clone(),
        t.text_3,
    );
    let mark = egui::Rect::from_center_size(
        egui::pos2(rect.left() + 9.0, rect.center().y),
        egui::Vec2::splat(14.0),
    );
    match (radio, on) {
        (true, _) => {
            let stroke = egui::Stroke::new(1.0, if on { t.accent } else { t.line });
            ui.painter().circle(mark.center(), 6.5, t.inset, stroke);
            if on {
                ui.painter().circle_filled(mark.center(), 3.5, t.accent);
            }
        }
        (false, true) => {
            ui.painter().rect_filled(mark, 3.0, t.accent_fill);
            Icon::Check.paint(ui.painter(), mark.shrink(1.0), t.on_fill);
        }
        (false, false) => {
            let stroke = egui::Stroke::new(1.0, t.line);
            ui.painter().rect(mark, 3.0, t.inset, stroke, egui::StrokeKind::Inside);
        }
    }
    let body = egui::TextStyle::Body.resolve(ui.style());
    let room = (right - shown.size().x - 8.0 - (mark.right() + 8.0)).max(0.0);
    let mut job = egui::text::LayoutJob::simple_singleline(name.to_string(), body, t.text);
    job.wrap = egui::text::TextWrapping::truncate_at_width(room);
    let galley = ui.painter().layout_job(job);
    let at = egui::pos2(mark.right() + 8.0, rect.center().y - galley.size().y / 2.0);
    ui.painter().galley(at, galley, t.text);
    r
}

/// How many of a list's rows the Cell panel shows before they scroll in
/// a frame of their own (the panel stays short beside a list of hundreds).
const LIST_ROWS: f32 = 8.5;

/// A list's or flags' choices as one list (M4's U16 review: the old
/// dropdown's keys kept): `rows` (name, value, on), check boxes for flags
/// (`radio` false) or radio buttons for a list's values. One stop of Tab;
/// with the keyboard's focus, Up and Down, Page Up and Page Down, Home
/// and End move along it, letters go to a value by its name, and Space or
/// Enter chooses (ticks) the one there; a click chooses one too. Past
/// [`LIST_ROWS`] rows they scroll in a frame of their own. Shown for
/// another of the column's cells (`row`, the table's row), the keys' row
/// is the first one on, brought into sight. The row chosen, by the keys or
/// a click, where one is.
fn choice_list(
    ui: &mut Ui,
    id: egui::Id,
    row: usize,
    rows: &[(String, String, bool)],
    radio: bool,
) -> Option<usize> {
    if rows.is_empty() {
        return None;
    }
    let cursor_id = id.with("cursor");
    let focused = ui.memory(|m| m.has_focus(id));
    let first_on = rows.iter().position(|r| r.2).unwrap_or(0);
    let kept: Option<(usize, usize)> = ui.data(|d| d.get_temp(cursor_id));
    let (mut cursor, mut moved) = match kept {
        Some((at, of)) if of == row => (at.min(rows.len() - 1), false),
        _ => (first_on, true),
    };
    let mut chosen = None;
    if focused {
        // (The arrows are the list's, not egui's to move the focus with.)
        let filter = egui::EventFilter { vertical_arrows: true, ..Default::default() };
        ui.memory_mut(|m| m.set_focus_lock_filter(id, filter));
        use egui::Key;
        let page = LIST_ROWS as usize;
        let keys = [Key::ArrowDown, Key::ArrowUp, Key::PageDown, Key::PageUp, Key::Home, Key::End];
        let pressed =
            ui.input_mut(|i| keys.into_iter().find(|k| i.consume_key(egui::Modifiers::NONE, *k)));
        let last = rows.len() - 1;
        let to = match pressed {
            Some(Key::ArrowDown) => Some((cursor + 1).min(last)),
            Some(Key::ArrowUp) => Some(cursor.saturating_sub(1)),
            Some(Key::PageDown) => Some((cursor + page).min(last)),
            Some(Key::PageUp) => Some(cursor.saturating_sub(page)),
            Some(Key::Home) => Some(0),
            Some(Key::End) => Some(last),
            _ => None,
        };
        let choose = ui.input_mut(|i| {
            i.consume_key(egui::Modifiers::NONE, Key::Space)
                || i.consume_key(egui::Modifiers::NONE, Key::Enter)
        });
        if choose {
            chosen = Some(cursor);
        }
        // Letters typed go to the value whose name begins with them.
        let texts: Vec<String> = rows.iter().map(|r| mg_core::text::fold(&r.0)).collect();
        let seek = |from: Option<usize>, typed: &str| {
            let from = from.unwrap_or(texts.len() - 1);
            crate::dialog_view::type_ahead(&texts, from, &mg_core::text::fold(typed))
        };
        let typed = crate::tree::typed_row(ui, id.with("typed"), Some(cursor), &seek);
        if let Some(to) = to.or(typed) {
            moved = to != cursor;
            cursor = to;
        }
    }
    let pitch = mg_uikit::row_pitch(ui);
    let scroll = egui::ScrollArea::vertical()
        .id_salt(id.with("scroll"))
        .auto_shrink([false, true])
        .max_height(LIST_ROWS * pitch);
    let shown = scroll.show(ui, |ui| {
        mg_uikit::list_rows(ui);
        let mut clicked = None;
        for (i, (name, value, on)) in rows.iter().enumerate() {
            let r = choice_row(ui, *on, radio, name, value);
            if focused && i == cursor {
                let t = mg_uikit::tokens(ui.ctx());
                let ring = egui::Stroke::new(1.0, t.accent);
                ui.painter().rect_stroke(r.rect, 2.0, ring, egui::StrokeKind::Inside);
            }
            if moved && i == cursor {
                ui.scroll_to_rect(r.rect, None);
            }
            if r.clicked() {
                clicked = Some(i);
            }
        }
        clicked
    });
    // (Its one stop of Tab, over its rows; a click on a row gives it the
    // keys, at that row.)
    let list = ui.interact(shown.inner_rect, id, egui::Sense::FOCUSABLE);
    list.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Other, true, "Values"));
    if let Some(i) = shown.inner {
        cursor = i;
        chosen = Some(i);
        list.request_focus();
    }
    ui.data_mut(|d| d.insert_temp(cursor_id, (cursor, row)));
    chosen
}

/// The Cell panel (M4's U16, the Table board's Right Dock, under its
/// header "Cell" and the cell's row and column): the selected cell's
/// **Value**, typed in (set as the field is left: Enter, Tab, a click
/// elsewhere; Escape lets go of it), and by its column's kind the value's
/// name (a list's, flags', a row's, a talk-table string's, a model's
/// layer) and the choosers (a list's values or the flags as rows of their
/// own, a row of the table named, Choose String…, New Line… and Talk
/// Table, Choose Model…); the column's kind in words, its note and what
/// is wrong with the cell; and at its foot what the table is described by.
/// Its edits are the table's steps, as the grid's: the Top Bar's Undo and
/// Ctrl+Z take them back, Cancel sets them back with the rest.
pub(crate) fn cell(app: &mut Moonglow, ui: &mut Ui, editor: Option<&Tab>) {
    let Some(at) = table_at(app, editor) else { return };
    let ctx = ui.ctx().clone();
    let id = app.tables[at].id;
    let tab = Tab::Table(id);
    // (The pointer over it: Undo and Redo are this table's, as over its
    // grid.)
    crate::side_doc::note_pointer(app, ui, &tab);
    follow_typed(&mut app.tables[at], &ctx);
    // (Typed for a cell no longer selected, by a Go to or the Outliner:
    // set in its own cell.)
    let selected = app.tables[at].selected;
    if app.tables[at].cell_typed.as_ref().is_some_and(|t| Some(t.cell) != selected) {
        commit_typed(&mut app.tables[at]);
    }
    let faults = super::faults(app, at);
    let name = app.tables[at].name();
    let kinds = super::kinds(app, &name);
    if begin_panel(app, ui, id, crate::palette_view::Keys::Cell)
        && let Some(cell) = selected
    {
        ctx.memory_mut(|m| m.request_focus(value_id(id, cell)));
    }
    let scroll = egui::ScrollArea::vertical().id_salt(("table-cell-panel", id)).auto_shrink(false);
    scroll.show(ui, |ui| {
        egui::Frame::new().inner_margin(MARGIN).show(ui, |ui| {
            ui.set_width(ui.available_width());
            cell_body(app, ui, at, &kinds, &faults);
            ui.add_space(6.0);
            described(app, ui, at);
        });
    });
}

/// The Cell panel's form and choosers ([`cell`]).
fn cell_body(
    app: &mut Moonglow,
    ui: &mut Ui,
    at: usize,
    kinds: &[(&'static str, ColumnKind)],
    faults: &[mg_module::doctor::CellFault],
) {
    let game = app.game.clone();
    let name = app.tables[at].name();
    let id = app.tables[at].id;
    let Some((row, column)) = app.tables[at].selected else {
        note(
            ui,
            "Click a cell to select it; double click, Enter or F2 to type in it; Delete \
             empties it.",
        );
        return;
    };
    let Some(column_name) = app.tables[at].edit.table().columns().get(column).cloned() else {
        return;
    };
    let description = app.tables[at].description();
    let kind = super::column_kind(kinds, description.as_deref(), &column_name);
    let addable = kind == Some(ColumnKind::StrRef) && crate::talk_view::can_add(app).is_some();
    let talk = app.talk.as_ref().filter(|_| !app.talk_view.outside);
    let table = app.tables[at].edit.shared();
    let value = table.cell(row, column).map(str::to_string);
    let described = description.as_deref().and_then(|d| d.column(&column_name));
    let t = mg_uikit::tokens(ui.ctx());
    let mut set: Option<Option<String>> = None;
    let mut open_talk = false;
    let mut new_line: Option<String> = None;
    // Value: the cell's text, typed in.
    let field_id = value_id(id, (row, column));
    let typed = app.tables[at].cell_typed.as_ref().filter(|t| t.cell == (row, column));
    let mut text = typed.map_or_else(|| value.clone().unwrap_or_default(), |t| t.text.clone());
    let response = form_row(ui, "Value", |ui| {
        let field = mg_uikit::text_field(&mut text)
            .id(field_id)
            .font(egui::TextStyle::Monospace)
            .hint_text("****")
            .desired_width(ui.available_width());
        ui.add(field).on_hover_text("The cell's value: Enter sets it, Escape lets it go")
    });
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::TextEdit, true, "Value"));
    let doc = &mut app.tables[at];
    if response.changed() {
        let since = doc.edit.changes();
        doc.cell_typed = Some(CellTyped { cell: (row, column), since, text: text.clone() });
    }
    // (Set once the table handed out for the choosers is let go of, below:
    // held, the change would copy the whole table, `Editing::shared`.)
    let mut commit = false;
    if response.lost_focus() {
        // (Enter and Escape are the field's: taken, so that the grid, drawn
        // after, does not begin typing in its cell or let go of the keys,
        // nor the tab take Escape for its Cancel.)
        let escape = ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape));
        ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter));
        match escape {
            true => doc.cell_typed = None,
            false => commit = true,
        }
    }
    // The value's name, by its column's kind.
    let doc = &app.tables[at];
    let files = doc.described.as_ref();
    let named_table = |n: &str| -> Option<Arc<mg_2da::TwoDa>> {
        let own = files.and_then(|d| super::project_table(app, d, n));
        own.or_else(|| game.as_deref().and_then(|g| g.table(n).ok()))
    };
    match described.map(|d| &d.kind) {
        Some(kind @ Kind::Flags { entries }) => {
            let bits = value.as_deref().and_then(mg_module::table_desc::whole).unwrap_or(0);
            let (on, rest) = mg_module::table_desc::flags_of(entries, bits);
            let mut names: Vec<String> = on.iter().map(|e| e.name.clone()).collect();
            if rest != 0 {
                names.push(format!("{rest:#X}"));
            }
            let shown = if names.is_empty() { "none".to_string() } else { names.join(" | ") };
            form_row(ui, "Flags", |ui| ui.add(egui::Label::new(shown).truncate()));
            heading(ui, &column_name, Some(&kind.words()));
            let flags: Vec<(&str, &str, i64)> = (entries.iter())
                .filter_map(|e| {
                    let v = mg_module::table_desc::whole(&e.value).filter(|v| *v != 0)?;
                    Some((e.name.as_str(), e.value.as_str(), v))
                })
                .collect();
            let rows: Vec<(String, String, bool)> = (flags.iter())
                .map(|(n, val, v)| (n.to_string(), val.to_string(), bits & v == *v))
                .collect();
            let list = egui::Id::new(("table-choices", id, column));
            if let Some(i) = choice_list(ui, list, row, &rows, false) {
                let v = flags[i].2;
                let next = if rows[i].2 { bits & !v } else { bits | v };
                set = Some(kind.flags_cell(next, value.as_deref()));
            }
            if rest != 0 {
                said(ui, format!("Bits no flag names: {rest:#X}"), t.warn);
            }
        }
        Some(kind @ Kind::List { entries, other }) => {
            let now = value.as_deref().and_then(|v| kind.named(v));
            let shown = match (&now, &value) {
                (Some(name), _) => name.clone(),
                (None, Some(v)) if *other => format!("{v} (its own value)"),
                (None, Some(v)) => format!("{v} (none of the list)"),
                (None, None) => "none".to_string(),
            };
            form_row(ui, "Name", |ui| ui.add(egui::Label::new(shown).truncate()));
            heading(ui, &column_name, Some(&kind.words()));
            let mut rows = vec![("none".to_string(), "****".to_string(), value.is_none())];
            rows.extend(entries.iter().map(|e| {
                (e.name.clone(), e.value.clone(), now.as_deref() == Some(e.name.as_str()))
            }));
            let list = egui::Id::new(("table-choices", id, column));
            match choice_list(ui, list, row, &rows, true) {
                Some(0) if value.is_some() => set = Some(None),
                Some(i) if i > 0 && !rows[i].2 => set = Some(Some(entries[i - 1].value.clone())),
                _ => {}
            }
        }
        Some(kind @ Kind::Row(target)) => {
            heading(ui, &column_name, Some(&kind.words()));
            let named =
                value.as_deref().and_then(mg_2da::parse_int).and_then(|v| usize::try_from(v).ok());
            let other = named_table(target);
            let width = ui.available_width();
            ui.horizontal_wrapped(|ui| {
                let own = (name.as_str(), &*table);
                super::row_choice(ui, id, own, (target, other.as_deref()), named, width, &mut set);
            });
        }
        Some(kind) => heading(ui, &column_name, Some(&kind.words())),
        None => {
            let words = kind_words(description.as_deref(), kinds, &column_name);
            heading(ui, &column_name, words.as_deref());
        }
    }
    // The choosers the checks know of the column's kind.
    match kind {
        Some(ColumnKind::StrRef) => {
            let text = value.as_deref().and_then(|v| v.parse::<u32>().ok()).and_then(|n| {
                super::pending_or_talk_string(&doc.pending, talk, game.as_deref(), n)
            });
            match text {
                Some(text) => {
                    _ = ui.add(egui::Label::new(format!("\u{201c}{text}\u{201d}")).wrap())
                }
                None => _ = note(ui, "No string of this number"),
            }
            let strings = (talk, game.as_deref());
            ui.horizontal_wrapped(|ui| {
                if ui
                    .add(mg_uikit::KitButton::new(Some(Icon::TalkTable), "Talk Table"))
                    .on_hover_text("Open the talk table editor")
                    .clicked()
                {
                    open_talk = true;
                }
                let v = value.as_deref();
                super::string_choice(ui, id, strings, v, addable, &mut set, &mut new_line);
            });
        }
        Some(ColumnKind::Model) => {
            ui.horizontal_wrapped(|ui| {
                super::model_choice(ui, id, game.as_deref(), value.as_deref(), &mut set);
            });
        }
        Some(k @ (ColumnKind::Row(_) | ColumnKind::RowBy { .. })) => match k.row_table(&table, row)
        {
            Some(target) => {
                let now = value.as_deref().and_then(mg_2da::parse_int);
                let named = now.and_then(|v| mg_module::doctor::row_named(&name, &column_name, v));
                let other = game.as_deref().and_then(|g| g.table(target).ok());
                let width = ui.available_width();
                ui.horizontal_wrapped(|ui| {
                    let own = (name.as_str(), &*table);
                    super::row_choice(
                        ui,
                        id,
                        own,
                        (target, other.as_deref()),
                        named,
                        width,
                        &mut set,
                    );
                });
            }
            None => _ = note(ui, "Not a row, for this row's kind"),
        },
        _ => {}
    }
    // The row of the table whose rows these are.
    if let Some(of) = description.as_deref().and_then(|d| d.rows.as_deref())
        && let Some(other) = named_table(of)
        && row < other.len()
    {
        note(ui, format!("Row {} of {of}.2da", super::row_label(&other, row)));
    }
    if let Some(n) = described.and_then(|d| d.note.as_deref()) {
        note(ui, n);
    }
    if let Some(f) = super::row_faults(faults, row).iter().find(|f| f.column == column) {
        said(ui, &f.message, t.error);
    }
    if let Some(text) = new_line {
        let line =
            mg_module::talk::Line { text, feminine: None, sound: String::new(), sound_length: 0.0 };
        // (Added to the talk table as this table is saved, where a cell
        // still names it: added here, the table's Cancel left it in the
        // talk table, named by nothing.)
        let mut pending = std::mem::take(&mut app.tables[at].pending);
        let pushed = pending.push(app, line);
        app.tables[at].pending = pending;
        match pushed {
            Ok(strref) => set = Some(Some(strref.0.to_string())),
            Err(e) => app.tables[at].problem = Some(format!("New Line: {e}")),
        }
    }
    // (The table handed out for the choosers let go of first: held, a
    // change would copy the whole table, `Editing::shared`.)
    drop(table);
    if commit {
        commit_typed(&mut app.tables[at]);
    }
    if let Some(value) = set {
        let doc = &mut app.tables[at];
        doc.typing = None;
        doc.cell_typed = None;
        match doc.edit.set(row, column, value) {
            Ok(true) => doc.changed(),
            Ok(false) => {}
            Err(e) => doc.problem = Some(e.to_string()),
        }
    }
    if open_talk {
        app.actions.push(Action::ShowModuleTalk);
    }
}

/// The Cell panel's foot: what the table is described by (the board's
/// "Described by Moonglow (the game's spells.2da): 34 of 59 columns. A
/// description of yours comes first."), whose rows its rows are, what the
/// description could not read or names and the table lacks; or, with none,
/// how to give it one.
fn described(app: &Moonglow, ui: &mut Ui, at: usize) {
    let doc = &app.tables[at];
    let t = mg_uikit::tokens(ui.ctx());
    let Some(d) = &doc.described else {
        note(
            ui,
            format!(
                "No description: a {}.2da.toml beside the table says what each column holds.",
                doc.name()
            ),
        );
        return;
    };
    let file = match &d.path {
        Some(path) => {
            path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
        }
        None => format!("Moonglow (the game's {}.2da)", doc.name()),
    };
    match &d.read {
        Ok(description) => {
            let table = doc.edit.table();
            let count = table.columns().iter().filter(|c| description.column(c).is_some()).count();
            let all = table.columns().len();
            let after = match &d.path {
                Some(_) => "Read again when the file changes.",
                None => "A description of yours comes first.",
            };
            let r = note(ui, format!("Described by {file}: {count} of {all} columns. {after}"));
            if let Some(path) = &d.path {
                r.on_hover_text(path.display().to_string());
            }
            if let Some(rows) = &description.rows {
                note(ui, format!("Its rows are {rows}.2da's."));
            }
            for n in &description.notes {
                said(ui, n, t.warn);
            }
            let missing = description.missing_columns(table);
            if !missing.is_empty() {
                said(
                    ui,
                    format!("Described, and not in the table: {}", missing.join(", ")),
                    t.warn,
                );
            }
        }
        Err(e) => _ = said(ui, format!("{file} was not read: {e}"), t.error),
    }
}

/// Starts Find References to Row `row` of table `at` on a thread of its
/// own, from the table as it is now and a copy of the module's list of
/// resources (their bytes shared, not copied): the cells of the game's
/// tables known to name its rows ([`super::references_until`]), then the
/// module's fields and scripts (`mg_module::row_uses`). One under way is
/// stopped first.
pub(crate) fn find_references(app: &mut Moonglow, at: usize, row: usize) {
    let name = app.tables[at].name();
    let own = app.tables[at].edit.shared();
    let game = app.game.clone();
    let module = app.ws.as_ref().map(|ws| ws.module.clone());
    let cp = super::codepage(app);
    let stop = Arc::new(AtomicBool::new(false));
    let stopped = stop.clone();
    let hold = app.hold_reference_looks.clone();
    let found = std::thread::spawn(move || {
        let stop = || stopped.load(Ordering::Relaxed);
        // (Held at its start by a test, until it lets go or it is stopped.)
        while hold.load(Ordering::Relaxed) && !stop() {
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        let found = super::references_until(game.as_deref(), &name, &own, row, &stop)?;
        let uses = match &module {
            Some(m) => {
                mg_module::row_uses::row_uses_until(m, &name, row, game.as_deref(), cp, &stop)?
            }
            None => Vec::new(),
        };
        Some((found, uses))
    });
    let doc = &mut app.tables[at];
    if let Some(was) = doc.looking.take() {
        was.stop();
    }
    doc.references = None;
    doc.looking = Some(Looking { row, stop, found: Some(found) });
}

/// Each frame, for every table: a Find References to Row that has ended
/// is taken in, one for a row no longer selected is stopped, and where one
/// is under way a frame is asked for soon (whether its tab is drawn or
/// not, as the description's look: `super::follow_looks`).
pub(crate) fn follow_looking(doc: &mut TableDoc, ctx: &egui::Context) {
    let Some(looking) = &doc.looking else { return };
    if doc.selected.map(|s| s.0) != Some(looking.row) {
        doc.looking.take().expect("looking").stop();
        return;
    }
    if !looking.found.as_ref().is_some_and(std::thread::JoinHandle::is_finished) {
        ctx.request_repaint_after(std::time::Duration::from_millis(30));
        return;
    }
    let mut looking = doc.looking.take().expect("looking");
    let row = looking.row;
    if let Some(Ok(Some((tables, uses)))) = looking.found.take().map(|f| f.join()) {
        doc.references = Some((row, tables, uses));
    }
}

/// How many of each group the References panel lists at most (a table's
/// row may be named thousands of times).
const LISTED: usize = 200;

/// The References panel (M4's U16, the Table board's Right Dock under
/// Cell, its header "References" and the row's number and name): **Find
/// References to Row N**, and what it found for that row: the game's
/// tables' cells known to name it ("packspbar1.2da row 42, SpellIndex")
/// and the module's fields and scripts, each a link that goes there. It
/// looks on a thread of its own, the window going on: "Looking…" while it
/// does, stopped as another row is chosen or the table changes; "Nothing
/// names row N" where nothing does.
pub(crate) fn references(app: &mut Moonglow, ui: &mut Ui, editor: Option<&Tab>) {
    let Some(at) = table_at(app, editor) else { return };
    let ctx = ui.ctx().clone();
    let id = app.tables[at].id;
    crate::side_doc::note_pointer(app, ui, &Tab::Table(id));
    follow_looking(&mut app.tables[at], &ctx);
    let focus = begin_panel(app, ui, id, crate::palette_view::Keys::TableReferences);
    let mut find = None;
    let mut go: Option<Usage> = None;
    let scroll =
        egui::ScrollArea::vertical().id_salt(("table-references-panel", id)).auto_shrink(false);
    scroll.show(ui, |ui| {
        egui::Frame::new().inner_margin(MARGIN).show(ui, |ui| {
            ui.set_width(ui.available_width());
            let doc = &app.tables[at];
            let Some((row, _)) = doc.selected else {
                note(ui, "Select a cell: the tables, blueprints, areas and scripts that name its row are found here.");
                return;
            };
            let button = mg_uikit::KitButton::new(Some(Icon::References), format!("Find References to Row {row}"))
                .tip("Where other tables, blueprints, areas and scripts name this row");
            let r = ui.add(button);
            if focus {
                r.request_focus();
            }
            if r.clicked() {
                find = Some(row);
            }
            ui.add_space(4.0);
            if doc.looking.as_ref().is_some_and(|l| l.row == row) {
                ui.horizontal(|ui| {
                    ui.add(egui::Spinner::new().size(14.0));
                    note(ui, "Looking through the game's tables and the module…");
                });
                return;
            }
            let Some((_, found, uses)) = doc.references.as_ref().filter(|r| r.0 == row) else {
                return;
            };
            if found.is_empty() && uses.is_empty() {
                note(ui, format!("Nothing names row {row}: no table's known columns, nor the module's fields and scripts."));
                return;
            }
            let t = mg_uikit::tokens(ui.ctx());
            let group = |ui: &mut Ui, words: String| {
                ui.add_space(4.0);
                ui.add(egui::Label::new(egui::RichText::new(words).color(t.text_2)).truncate());
            };
            let line = |ui: &mut Ui, icon: Icon, words: &str, link: bool| -> egui::Response {
                ui.horizontal(|ui| {
                    let size = egui::Vec2::splat(mg_uikit::metrics::ICON);
                    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
                    icon.paint(ui.painter(), rect, t.text_3);
                    match link {
                        true => {
                            let text = egui::RichText::new(words).color(t.accent);
                            let label = egui::Label::new(text).truncate().sense(egui::Sense::click());
                            let r = ui.add(label);
                            r.widget_info(|| {
                                egui::WidgetInfo::labeled(egui::WidgetType::Link, true, words)
                            });
                            r.on_hover_cursor(egui::CursorIcon::PointingHand)
                        }
                        false => ui.add(egui::Label::new(words).truncate()),
                    }
                })
                .inner
            };
            if !found.is_empty() {
                group(ui, format!("In the game's tables ({})", found.len()));
                for r in found.iter().take(LISTED) {
                    line(ui, Icon::Table, &format!("{}.2da row {}, {}", r.table, r.row, r.column), false);
                }
                if found.len() > LISTED {
                    note(ui, format!("and {} more", found.len() - LISTED));
                }
            }
            if !uses.is_empty() {
                group(ui, format!("In the module ({})", uses.len()));
                for u in uses.iter().take(LISTED) {
                    let icon = match u.instance() {
                        Some((list, _)) => mg_area::ObjectKind::ALL
                            .into_iter()
                            .find(|k| k.list() == list)
                            .map_or(Icon::Area, crate::icons::object),
                        None => crate::browser::kind_icon(u.from.restype),
                    };
                    if line(ui, icon, &u.place, true).on_hover_text("Go there").clicked() {
                        go = Some(u.clone());
                    }
                }
                if uses.len() > LISTED {
                    note(ui, format!("and {} more in the module", uses.len() - LISTED));
                }
            }
        });
    });
    if let Some(row) = find {
        find_references(app, at, row);
        ctx.request_repaint();
    }
    if let Some(u) = go {
        app.go_to_usage(&u);
    }
}

/// What the Status Bar shows beside a table (M4's U16, the Table board's):
/// how many cells are selected where more than one ("6 cells"), its rows
/// and columns and where it is kept ("840 rows, 59 columns, in the
/// module"; in a hak, in a file); and the grid's keys, fixed today: Enter
/// Type, Shift + Click Select to, Delete Empty, Ctrl + C Copy (Cmd on
/// macOS), Ctrl + V Paste, Tab Next column, Esc Leave grid. The keys only
/// while the grid has them and nothing is typed in it (Shift + Click
/// always), none while a modal, a dialog or a menu is in front.
pub(crate) fn shown(
    app: &Moonglow,
    k: &crate::shell::HintKeys<'_>,
    ctx: &egui::Context,
    id: u32,
) -> crate::shell::Shown {
    use crate::shell::{Count, Shown};
    use mg_uikit::status::Hint;
    let Some(doc) = app.tables.iter().find(|d| d.id == id) else { return Shown::default() };
    let mut counts = Vec::new();
    if let Some((r0, r1, c0, c1)) = doc.area() {
        let cells = (r1 - r0 + 1) * (c1 - c0 + 1);
        if cells > 1 {
            counts.push(Count::new(format!("{cells} cells")));
        }
    }
    let t = doc.edit.table();
    let home = match &doc.home {
        super::Home::Module => "in the module".to_string(),
        super::Home::Hak(hak) => {
            let file = app.haks.iter().find(|d| d.id == *hak).and_then(|d| {
                d.hak.path.as_ref()?.file_name().map(|f| f.to_string_lossy().into_owned())
            });
            format!("in {}", file.unwrap_or_else(|| "a hak".into()))
        }
        super::Home::File(_) => "in a file".to_string(),
    };
    counts.push(Count::new(format!("{} rows, {} columns, {home}", t.len(), t.columns().len())));
    let mut hints = Vec::new();
    let keys = super::grid_has_keys(app, ctx, id, &Tab::Table(id));
    let typing = doc.typing.is_some() || ctx.egui_wants_keyboard_input();
    if crate::shell::keys_blocked(ctx) {
        return Shown { hints, counts, note: None };
    }
    let ctrl = k.ctrl();
    if keys && !typing {
        hints.push(Hint::keys(["Enter"], "Type"));
    }
    hints.push(Hint::keys(["Shift", "Click"], "Select to"));
    if keys && !typing {
        hints.push(Hint::keys(["Delete"], "Empty"));
        hints.push(Hint::keys([ctrl, "C"], "Copy"));
        hints.push(Hint::keys([ctrl, "V"], "Paste"));
        hints.push(Hint::keys(["Tab"], "Next column"));
        hints.push(Hint::keys(["Esc"], "Leave grid"));
    }
    Shown { hints, counts, note: None }
}

/// What the Status Bar shows beside a table only looked at (the resource
/// browser's view of the game's or a hak's copy): its rows and columns and
/// whose it is, and the keys it has (Shift + Click Select to; with the
/// keys, Ctrl + C Copy, Tab Next column, Esc Leave grid), none of those
/// that change a table.
pub(crate) fn shown_read_only(
    k: &crate::shell::HintKeys<'_>,
    ctx: &egui::Context,
    (doc, has): (&TableDoc, bool),
    whose: &str,
) -> crate::shell::Shown {
    use crate::shell::{Count, Shown};
    use mg_uikit::status::Hint;
    let mut counts = Vec::new();
    if let Some((r0, r1, c0, c1)) = doc.area() {
        let cells = (r1 - r0 + 1) * (c1 - c0 + 1);
        if cells > 1 {
            counts.push(Count::new(format!("{cells} cells")));
        }
    }
    let t = doc.edit.table();
    counts.push(Count::new(format!("{} rows, {} columns, {whose}", t.len(), t.columns().len())));
    let mut hints = Vec::new();
    if crate::shell::keys_blocked(ctx) {
        return Shown { hints, counts, note: None };
    }
    let keys = has && !ctx.egui_wants_keyboard_input();
    hints.push(Hint::keys(["Shift", "Click"], "Select to"));
    if keys {
        hints.push(Hint::keys([k.ctrl(), "C"], "Copy"));
        hints.push(Hint::keys(["Tab"], "Next column"));
        hints.push(Hint::keys(["Esc"], "Leave grid"));
    }
    Shown { hints, counts, note: None }
}
