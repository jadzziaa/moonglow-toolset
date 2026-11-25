//! The gallery cell ("Quiet Tiles", the proposal's section 8.4 and the
//! mockups' `.qg` and `.qc`): one picture cell for every picture grid of
//! the suite, the grid it stands in and the one size slider.
//!
//! - [`GalleryCell`]: a picture as a **tile** (radius 4, a 1 point `line`
//!   rim drawn inside it over the picture, on `thumb-bg`), 6 points under
//!   it the **name** (12 on 15, `text-2`, centered, by [the name
//!   rule](name_lines)), an optional **detail** line right under it (11 on
//!   14, `text-3`). Nothing is drawn around a whole cell in any state:
//!   hover turns the rim `raised-hover` and the name `text`; selected
//!   draws a 2 point `accent` ring (`text-3` where the grid has not the
//!   keys); a picture still being made shows the kind's icon dimmed
//!   (`text-off`, 24 points), "No picture" adds a caption under it, "No
//!   model" a badge at the tile's top left, a favorite a bookmark at its
//!   top right; disabled, the picture is faded and the name `text-off`.
//!   A model or resource name is set in Hack 11 on one line. The small
//!   variant (Tile Properties' variants): radius 3, the name 11 on 14, 3
//!   points under the tile.
//! - [`GridLayout`]: where each cell of a grid stands, by arithmetic
//!   alone, so that a cell out of sight has a place (to scroll to, to
//!   step to with the keys) without being laid out: cells that grow to
//!   fill the row ([`GridLayout::grow`], 10 by 14 apart), fixed cells in
//!   justified rows ([`GridLayout::justified`]), a fixed number of columns
//!   ([`GridLayout::columns`]) and the small variant's fixed cells 6 apart
//!   ([`GridLayout::small`]).
//! - [`GalleryGrid`]: a grid in someone's scroll, laying out and painting
//!   only the rows in sight, one Tab stop for the whole grid; with the
//!   keys, Up and Down go a row, Left and Right a cell, Home and End to
//!   the first and the last, Page Up and Page Down a page of rows, Enter
//!   and Space act on the cell (a click's), and the cell that takes the
//!   keys is brought into sight. [`step`] is the keys' arithmetic, for
//!   other lists of cells (the palettes') to share.
//! - [`size_slider`]: the slider of the pictures' side (64 to 220 for the
//!   galleries, 64 to 224 for Tile Properties' variants).
//! - [`beside`]: where a cell's enlargement (its tooltip) stands: beside
//!   the panel the grid is in, never over it, never outside the window.

use std::ops::Range;

use egui::{
    Align2, Color32, CornerRadius, FontId, Id, Pos2, Rect, Response, Sense, Stroke, StrokeKind, Ui,
    Vec2, WidgetInfo, WidgetType, pos2, vec2,
};

use crate::icons::Icon;
use crate::theme::TokensExt as _;

/// The galleries' pictures' side, points: the slider's least, most and
/// first (today's 128).
pub const SIDE_MIN: f32 = 64.0;
pub const SIDE_MAX: f32 = 220.0;
pub const SIDE_DEFAULT: f32 = 128.0;
/// Tile Properties' variants' own slider (the small variant): its least,
/// most (the pictures' own 224 pixels) and first.
pub const SMALL_MIN: f32 = 64.0;
pub const SMALL_MAX: f32 = 224.0;
pub const SMALL_DEFAULT: f32 = 84.0;

/// Between cells that grow: columns 10 apart, rows 14.
pub const GAP: Vec2 = Vec2::new(10.0, 14.0);
/// Between the small variant's cells, both ways; and the least gap of
/// justified rows of the small variant.
pub const SMALL_GAP: f32 = 6.0;
/// The least gap between justified cells (the portrait picker's).
pub const JUSTIFIED_GAP: f32 = 8.0;
/// A tile's radius, the small variant's.
pub const RADIUS: u8 = 4;
pub const SMALL_RADIUS: u8 = 3;
/// From the tile to the name; the small variant's.
pub const NAME_GAP: f32 = 6.0;
pub const SMALL_NAME_GAP: f32 = 3.0;
/// The name's size and line; the small variant's; a model name's size.
pub const NAME_SIZE: f32 = 12.0;
pub const NAME_LINE: f32 = 15.0;
pub const SMALL_NAME_SIZE: f32 = 11.0;
pub const SMALL_NAME_LINE: f32 = 14.0;
pub const MODEL_SIZE: f32 = 11.0;
/// The detail line's size and line.
pub const DETAIL_SIZE: f32 = 11.0;
pub const DETAIL_LINE: f32 = 14.0;
/// The kind's icon on a tile with no picture.
pub const KIND_ICON: f32 = 24.0;
/// A lone cell that grows is at most this many times the side asked for.
pub const GROW_AT_MOST: f32 = 2.0;
/// The gap between a panel and an enlargement standing beside it.
pub const BESIDE_GAP: f32 = 6.0;

/// What a cell's tile shows.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Picture {
    /// The picture: drawn in its own shape in the tile (letterboxed).
    Ready(egui::TextureId),
    /// Not made yet: the kind's icon, dimmed.
    Making,
    /// Made, with nothing to draw: the kind's icon and "No picture".
    Missing,
    /// The part `uv` of a texture filling the tile whatever its shape (a
    /// portrait's: the upper 100/128 of its canvas, in a 54 by 84 tile;
    /// M4's U26).
    Fill(egui::TextureId, Rect),
}

/// What a cell looks like as the grid sees it, from the pointer and the
/// keys (the caller adds what is chosen).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CellState {
    /// Under the pointer.
    pub hovered: bool,
    /// The cell the grid's keys are at.
    pub cursor: bool,
    /// The grid has the keys.
    pub keys: bool,
}

/// A name's lines by the name rule (the owner's of 2026-11-10): a name
/// never breaks inside a word; a name of one word is one line, cut with
/// an ellipsis; a name of several words wraps between words on up to
/// `lines` lines and ends in an ellipsis after the last; a word wider than
/// a line on its own is cut with an ellipsis on its line. With `lines`
/// 1 (a model name, one word whatever it holds) the whole name is one
/// line, cut. `measure` gives a text's width; `width` is the room.
pub fn name_lines(
    name: &str,
    width: f32,
    lines: usize,
    measure: &mut dyn FnMut(&str) -> f32,
) -> Vec<String> {
    let name = name.trim();
    if name.is_empty() || lines == 0 {
        return Vec::new();
    }
    let words: Vec<&str> = name.split_whitespace().collect();
    if lines == 1 || words.len() == 1 {
        let one = if lines == 1 { name.to_string() } else { words[0].to_string() };
        return vec![cut(&one, width, false, measure)];
    }
    // Every line the words make, a word too wide alone cut on its own.
    let mut all: Vec<String> = Vec::new();
    let mut line = String::new();
    for word in words {
        let joined = if line.is_empty() { word.to_string() } else { format!("{line} {word}") };
        if measure(&joined) <= width {
            line = joined;
            continue;
        }
        if !line.is_empty() {
            all.push(std::mem::take(&mut line));
        }
        if measure(word) <= width {
            line = word.to_string();
        } else {
            all.push(cut(word, width, false, measure));
        }
    }
    if !line.is_empty() {
        all.push(line);
    }
    if all.len() > lines {
        all.truncate(lines);
        let last = all.pop().unwrap_or_default();
        all.push(cut(&last, width, true, measure));
    }
    all
}

/// `text` cut to `width` with an ellipsis where it is wider (with
/// `always`, ended with one whatever its width: a name going on).
fn cut(text: &str, width: f32, always: bool, measure: &mut dyn FnMut(&str) -> f32) -> String {
    if !always && measure(text) <= width {
        return text.to_string();
    }
    if text.ends_with('…') && measure(text) <= width {
        return text.to_string();
    }
    let mut lo = 0;
    let mut hi = text.len();
    let mut buf = String::with_capacity(text.len() + '…'.len_utf8());
    while lo < hi {
        let mut mid = lo + (hi - lo).div_ceil(2);
        while !text.is_char_boundary(mid) {
            mid += 1;
        }
        buf.clear();
        buf.push_str(text[..mid].trim_end());
        buf.push('…');
        if measure(&buf) <= width {
            lo = mid;
        } else {
            let mut prev = mid - 1;
            while !text.is_char_boundary(prev) {
                prev -= 1;
            }
            hi = prev;
        }
    }
    buf.clear();
    buf.push_str(text[..lo].trim_end());
    buf.push('…');
    buf
}

/// How cells are sized in a grid.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Sizing {
    /// As many as fit at the side asked for, grown to fill the row.
    Grow,
    /// Fixed tiles of this size, the room left over shared between them.
    Justified,
    /// This many columns sharing the row.
    Columns,
    /// Fixed tiles, a fixed gap, from the left.
    Fixed,
}

/// Where each cell of a grid stands: the columns, a cell's size (its tile
/// and its label), the gaps, all by arithmetic, for cells out of sight
/// too.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GridLayout {
    /// Cells across.
    pub columns: usize,
    /// A cell's tile (the picture's room).
    pub tile: Vec2,
    /// A cell's whole size: its tile and its label under it.
    pub cell: Vec2,
    /// Between cells, across and down.
    pub gap: Vec2,
}

/// Cells across `room` at least `side` wide, `gap` apart, and the width
/// that makes them fill it: the grow rule. Never under one; a lone cell
/// as wide as the room up to [`GROW_AT_MOST`] times `side`; never wider
/// than the share of the room, so a row has no ragged right edge.
pub fn grown(room: f32, side: f32, gap: f32) -> (usize, f32) {
    let side = side.max(1.0);
    let columns = (((room + gap) / (side + gap)).floor() as usize).max(1);
    let share = ((room - gap * (columns - 1) as f32) / columns as f32).floor();
    (columns, share.min(side * GROW_AT_MOST).max(1.0))
}

impl GridLayout {
    /// Cells that grow (the galleries'): as many as fit across `room` at
    /// `side`, [`GAP`] apart, sharing the row; square tiles, a label of
    /// `label` points under each ([`label_height`]).
    pub fn grow(room: f32, side: f32, label: f32) -> GridLayout {
        let (columns, w) = grown(room, side, GAP.x);
        GridLayout { columns, tile: Vec2::splat(w), cell: vec2(w, w + label), gap: GAP }
    }

    /// Cells of a fixed `tile` in justified rows: as many columns as fit
    /// at the least gap `min_gap`, the room left over shared out between
    /// them (the last cell of a row reaches the room's edge); a last row
    /// not full keeps the same columns. Rows `min_gap` apart.
    pub fn justified(room: f32, tile: Vec2, label: f32, min_gap: f32) -> GridLayout {
        let columns = (((room + min_gap) / (tile.x + min_gap)).floor() as usize).max(1);
        let gap = match columns {
            1 => min_gap,
            n => ((room - n as f32 * tile.x) / (n - 1) as f32).max(min_gap),
        };
        GridLayout { columns, tile, cell: vec2(tile.x, tile.y + label), gap: vec2(gap, min_gap) }
    }

    /// `columns` columns sharing `room`, [`GAP`] apart, square tiles.
    pub fn columns(room: f32, columns: usize, label: f32) -> GridLayout {
        let columns = columns.max(1);
        let w = ((room - GAP.x * (columns - 1) as f32) / columns as f32).floor().max(1.0);
        GridLayout { columns, tile: Vec2::splat(w), cell: vec2(w, w + label), gap: GAP }
    }

    /// The small variant: square tiles of `side`, [`SMALL_GAP`] apart from
    /// the left, as many as fit across `room`.
    pub fn small(room: f32, side: f32, label: f32) -> GridLayout {
        let columns = (((room + SMALL_GAP) / (side + SMALL_GAP)).floor() as usize).max(1);
        let gap = Vec2::splat(SMALL_GAP);
        GridLayout { columns, tile: Vec2::splat(side), cell: vec2(side, side + label), gap }
    }

    /// Rows `count` cells take.
    pub fn rows(&self, count: usize) -> usize {
        count.div_ceil(self.columns)
    }

    /// The height `count` cells take.
    pub fn height(&self, count: usize) -> f32 {
        let rows = self.rows(count);
        match rows {
            0 => 0.0,
            n => n as f32 * self.cell.y + (n - 1) as f32 * self.gap.y,
        }
    }

    /// The width a row takes.
    pub fn width(&self) -> f32 {
        self.columns as f32 * self.cell.x + (self.columns - 1) as f32 * self.gap.x
    }

    /// Cell `index`'s rectangle, the grid's top left at `origin`.
    pub fn rect(&self, origin: Pos2, index: usize) -> Rect {
        let (row, column) = (index / self.columns, index % self.columns);
        let at = origin
            + vec2(
                column as f32 * (self.cell.x + self.gap.x),
                row as f32 * (self.cell.y + self.gap.y),
            );
        Rect::from_min_size(at, self.cell)
    }

    /// The rows of `count` cells, the grid's top at `top`, that `y`
    /// reaches into (the rows in sight: `y` the clip's).
    pub fn rows_in(&self, top: f32, y: egui::Rangef, count: usize) -> Range<usize> {
        let rows = self.rows(count);
        let pitch = self.cell.y + self.gap.y;
        if rows == 0 || y.max < top || pitch <= 0.0 {
            return 0..0;
        }
        let first = (((y.min - top) / pitch).floor().max(0.0) as usize).min(rows);
        let last = ((((y.max - top) / pitch).floor().max(0.0) as usize) + 1).min(rows);
        first..last.max(first)
    }

    /// The cells of rows `rows` (out of `count`).
    pub fn cells_of(&self, rows: Range<usize>, count: usize) -> Range<usize> {
        (rows.start * self.columns).min(count)..(rows.end * self.columns).min(count)
    }
}

/// The room under a tile for its label: the gap, `lines` lines of the
/// name (none: no name), and the detail line where the grid has one.
pub fn label_height(small: bool, lines: usize, detail: bool) -> f32 {
    if lines == 0 && !detail {
        return 0.0;
    }
    let (gap, line) = match small {
        true => (SMALL_NAME_GAP, SMALL_NAME_LINE),
        false => (NAME_GAP, NAME_LINE),
    };
    gap + lines as f32 * line + if detail { DETAIL_LINE } else { 0.0 }
}

/// A key a grid of cells takes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GridKey {
    Up,
    Down,
    Left,
    Right,
    Home,
    End,
    PageUp,
    PageDown,
}

impl GridKey {
    /// The key pressed, with no modifier, among those a grid takes.
    pub fn pressed(ui: &Ui) -> Option<GridKey> {
        use egui::Key;
        let keys = [
            (Key::ArrowUp, GridKey::Up),
            (Key::ArrowDown, GridKey::Down),
            (Key::ArrowLeft, GridKey::Left),
            (Key::ArrowRight, GridKey::Right),
            (Key::Home, GridKey::Home),
            (Key::End, GridKey::End),
            (Key::PageUp, GridKey::PageUp),
            (Key::PageDown, GridKey::PageDown),
        ];
        ui.input(|i| {
            let plain = i.modifiers.is_none();
            keys.into_iter().find(|(k, _)| plain && i.key_pressed(*k)).map(|(_, g)| g)
        })
    }
}

/// Where `key` takes the keys from cell `at` of `count`, `columns` across
/// and `page` rows a page: Up and Down a row (staying where there is no
/// row that way; Down onto the last row's last cell where the row under
/// is shorter), Left and Right one cell (on to the row before or after),
/// Home and End the first and the last, Page Up and Page Down `page` rows
/// (to the first or the last where fewer are left). `None` with no cells.
pub fn step(at: usize, count: usize, columns: usize, page: usize, key: GridKey) -> Option<usize> {
    if count == 0 {
        return None;
    }
    let (at, columns, page) = (at.min(count - 1), columns.max(1), page.max(1));
    let last = count - 1;
    Some(match key {
        GridKey::Up => at.checked_sub(columns).unwrap_or(at),
        GridKey::Down if at + columns <= last => at + columns,
        // (Onto the last row, shorter than this one: its last cell.)
        GridKey::Down if at / columns < last / columns => last,
        GridKey::Down => at,
        GridKey::Left => at.saturating_sub(1),
        GridKey::Right => (at + 1).min(last),
        GridKey::Home => 0,
        GridKey::End => last,
        GridKey::PageUp => at.saturating_sub(page * columns),
        GridKey::PageDown => (at + page * columns).min(last),
    })
}

/// A cell of a gallery: its picture in a tile, its name under it.
/// [`GalleryCell::paint`] draws it into a rectangle a grid laid out
/// ([`GridLayout::rect`]) and names it for a screen reader by its
/// response.
#[derive(Clone, Debug)]
pub struct GalleryCell<'a> {
    name: &'a str,
    /// The accessible name, where it is not the name shown.
    said: Option<&'a str>,
    detail: Option<&'a str>,
    picture: Picture,
    kind: Option<Icon>,
    selected: bool,
    disabled: bool,
    no_model: bool,
    favorite: bool,
    small: bool,
    model: bool,
    lines: usize,
    radius: Option<u8>,
}

impl<'a> GalleryCell<'a> {
    /// A cell named `name`, showing `picture`.
    pub fn new(name: &'a str, picture: Picture) -> Self {
        GalleryCell {
            name,
            said: None,
            detail: None,
            picture,
            kind: None,
            selected: false,
            disabled: false,
            no_model: false,
            favorite: false,
            small: false,
            model: false,
            lines: 2,
            radius: None,
        }
    }

    /// Its accessible name, where it says more than the name shown (a
    /// palette's row: the name, its ResRef and its challenge rating).
    pub fn said(mut self, said: &'a str) -> Self {
        self.said = Some(said);
        self
    }

    /// The detail line (`CR 5`, a row's number).
    pub fn detail(mut self, detail: Option<&'a str>) -> Self {
        self.detail = detail.filter(|d| !d.is_empty());
        self
    }

    /// The kind's icon, shown while the picture is made and where there is
    /// none.
    pub fn kind(mut self, kind: Icon) -> Self {
        self.kind = Some(kind);
        self
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// "No model": the badge at the tile's top left.
    pub fn no_model(mut self, no_model: bool) -> Self {
        self.no_model = no_model;
        self
    }

    /// A favorite: the bookmark at the tile's top right.
    pub fn favorite(mut self, favorite: bool) -> Self {
        self.favorite = favorite;
        self
    }

    /// The small variant (Tile Properties' variants).
    pub fn small(mut self, small: bool) -> Self {
        self.small = small;
        self
    }

    /// A model's or resource's name: Hack 11, one line.
    pub fn model(mut self, model: bool) -> Self {
        self.model = model;
        self
    }

    /// The name's most lines (2; 1 for a model name; 0 for none).
    pub fn lines(mut self, lines: usize) -> Self {
        self.lines = lines;
        self
    }

    /// The tile's radius, where it is not the variant's (a portrait's 3).
    pub fn radius(mut self, radius: u8) -> Self {
        self.radius = Some(radius);
        self
    }

    fn name_font(&self) -> FontId {
        match (self.model, self.small) {
            (true, _) => FontId::monospace(MODEL_SIZE),
            (false, true) => FontId::proportional(SMALL_NAME_SIZE),
            (false, false) => FontId::proportional(NAME_SIZE),
        }
    }

    /// Paints the cell into `rect` (`tile` its tile's size, at its top),
    /// lit by `state` and `response`'s pointer, and names it on
    /// `response` for a screen reader: a button, selected or not, by its
    /// name (or [`GalleryCell::said`]).
    pub fn paint(&self, ui: &Ui, rect: Rect, tile: Vec2, state: CellState, response: &Response) {
        let said = self.said.unwrap_or(self.name).to_string();
        let (selected, enabled) = (self.selected, !self.disabled);
        response.widget_info(|| {
            let mut info = WidgetInfo::selected(WidgetType::Button, enabled, selected, &said);
            info.label = Some(said.clone());
            info
        });
        // egui says a widget selected as toggled; a screen reader reads a
        // picture in a gallery as selected.
        ui.ctx().accesskit_node_builder(response.id, |n| n.set_selected(selected));
        if !ui.is_rect_visible(rect) {
            return;
        }
        let t = ui.ctx().tokens();
        let painter = ui.painter_at(rect.expand(2.0));
        let tile = Rect::from_min_size(rect.min, tile.min(rect.size()));
        let radius = CornerRadius::same(self.radius.unwrap_or(match self.small {
            true => SMALL_RADIUS,
            false => RADIUS,
        }));
        let hovered = (state.hovered || response.hovered()) && enabled;
        painter.rect_filled(tile, radius, t.thumb_bg);
        let fade = if enabled { Color32::WHITE } else { Color32::from_white_alpha(102) };
        let icon_color = t.text_off;
        match self.picture {
            Picture::Ready(id) => {
                let uv = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
                let at = letterboxed(ui.ctx(), id, tile);
                painter.image(id, at, uv, fade);
            }
            Picture::Fill(id, uv) => {
                painter.with_clip_rect(tile).image(id, tile, uv, fade);
            }
            Picture::Making | Picture::Missing => {
                let side = KIND_ICON.min(tile.width() * 0.5).min(tile.height() * 0.5);
                let icon = Rect::from_center_size(tile.center(), Vec2::splat(side));
                if let Some(kind) = self.kind {
                    kind.paint(&painter, icon, icon_color);
                }
                if self.picture == Picture::Missing && tile.height() >= 48.0 {
                    let font = FontId::proportional(DETAIL_SIZE);
                    let at = pos2(tile.center().x, icon.bottom() + 4.0);
                    let galley = painter.layout_no_wrap("No picture".into(), font, icon_color);
                    let galley =
                        if galley.size().x > tile.width() - 8.0 { None } else { Some(galley) };
                    if let Some(g) = galley {
                        let at = at - vec2(g.size().x / 2.0, 0.0);
                        painter.galley(at, g, icon_color);
                    }
                }
            }
        }
        // The rim, outside the tile against the ground it stands on (the
        // owner's ruling: the light theme's `accent` would vanish on the
        // dark `thumb-bg` inside it): hovered, selected (with the keys or
        // without), the keys' cursor. Hovered in the light theme it is
        // `text-off` (2.8:1 on `panel`, 3.2:1 on `inset`; `raised-hover`
        // there, 1.5:1, hardly showed), weaker than selected's `text-3`.
        let hover = match crate::theme::applied(ui.ctx()) {
            Some(crate::Theme::Light) => t.text_off,
            _ => t.raised_hover,
        };
        let (width, color) = match (selected, state.keys, hovered, state.cursor && state.keys) {
            (true, true, _, _) => (2.0, t.accent),
            (true, false, _, _) => (2.0, t.text_3),
            (false, _, _, true) => (1.0, t.accent),
            (false, _, true, _) => (1.0, hover),
            _ => (1.0, t.line),
        };
        painter.rect_stroke(tile, radius, Stroke::new(width, color), StrokeKind::Outside);
        if self.no_model && tile.width() >= 40.0 {
            let font = FontId::proportional(DETAIL_SIZE);
            let galley = painter.layout_no_wrap("No model".into(), font, t.thumb_badge_text);
            let badge = Rect::from_min_size(
                tile.min + vec2(5.0, 5.0),
                vec2(galley.size().x + 8.0, 16.0).min(tile.size() - vec2(10.0, 10.0)),
            );
            painter.rect_filled(badge, CornerRadius::same(2), t.thumb_badge);
            let at = pos2(badge.left() + 4.0, badge.center().y - galley.size().y / 2.0);
            painter.with_clip_rect(badge).galley(at, galley, t.thumb_badge_text);
        }
        if self.favorite {
            let side = crate::metrics::ICON_SMALL;
            let at = pos2(tile.right() - 5.0 - side, tile.top() + 5.0);
            let mark = Rect::from_min_size(at, Vec2::splat(side));
            // (Filled: the bookmark's shape, then its stroke.)
            let (l, r, top, bottom) =
                (mark.left() + 2.5, mark.right() - 2.5, mark.top(), mark.bottom());
            let notch = pos2(mark.center().x, bottom - side * 0.3);
            let shape = vec![pos2(l, top), pos2(r, top), pos2(r, bottom), notch, pos2(l, bottom)];
            painter.add(egui::Shape::convex_polygon(shape, t.thumb_flag, Stroke::NONE));
            Icon::Bookmark.paint(&painter, mark, t.thumb_flag);
        }
        // The name, then the detail right under its last line (the
        // GalleryWide board's "Dire Wolf" over "CR 5"), never past the foot.
        let name_color = match (enabled, hovered || selected) {
            (false, _) => t.text_off,
            (true, true) => t.text,
            (true, false) => t.text_2,
        };
        let lines = if self.model { self.lines.min(1) } else { self.lines };
        let (gap, line) = match (self.small, self.model) {
            (true, _) => (SMALL_NAME_GAP, SMALL_NAME_LINE),
            (false, _) => (NAME_GAP, NAME_LINE),
        };
        let font = self.name_font();
        let room = rect.width();
        let shown = ui.fonts_mut(|f| {
            let mut measure = |s: &str| {
                f.layout_no_wrap(s.to_string(), font.clone(), Color32::PLACEHOLDER).size().x
            };
            name_lines(self.name, room, lines, &mut measure)
        });
        let mut y = tile.bottom() + gap;
        for text in shown {
            let galley = painter.layout_no_wrap(text, font.clone(), Color32::PLACEHOLDER);
            let at =
                pos2(rect.center().x - galley.size().x / 2.0, y + (line - galley.size().y) / 2.0);
            painter.galley(at, galley, name_color);
            y += line;
        }
        if let Some(detail) = self.detail {
            let font = FontId::proportional(DETAIL_SIZE);
            let color = if enabled { t.text_3 } else { t.text_off };
            let text = ui.fonts_mut(|f| {
                let mut measure = |s: &str| {
                    f.layout_no_wrap(s.to_string(), font.clone(), Color32::PLACEHOLDER).size().x
                };
                cut(detail, room, false, &mut measure)
            });
            let galley = painter.layout_no_wrap(text, font, Color32::PLACEHOLDER);
            let top = y.min(rect.bottom() - DETAIL_LINE);
            let at = pos2(rect.center().x - galley.size().x / 2.0, top);
            painter.galley(at, galley, color);
        }
    }
}

/// Where a picture of texture `id` is drawn in `room`: as large as fits,
/// in its own shape (a texture twice as wide as tall isn't stretched to a
/// square), centered; the whole room where its size isn't known.
pub fn letterboxed(ctx: &egui::Context, id: egui::TextureId, room: Rect) -> Rect {
    let size = ctx.tex_manager().read().meta(id).map(|m| m.size);
    let Some([w, h]) = size.filter(|[w, h]| *w > 0 && *h > 0) else { return room };
    let (w, h) = (w as f32, h as f32);
    let scale = (room.width() / w).min(room.height() / h);
    Rect::from_center_size(room.center(), vec2(w * scale, h * scale))
}

/// What a [`GalleryGrid`] did in a frame.
#[derive(Clone, Debug, PartialEq)]
pub struct GridOutput {
    /// Where the grid stands, and how its cells are laid out.
    pub rect: Rect,
    pub layout: Option<GridLayout>,
    /// The cells laid out and painted (those in sight), and those of the
    /// rows ahead (in sight and the rows below, whose pictures a caller
    /// asks for).
    pub drawn: Range<usize>,
    pub ahead: Range<usize>,
    /// The cell the keys moved to this frame.
    pub moved: Option<usize>,
    /// Enter or Space on the keys' cell (what its click does).
    pub activated: Option<usize>,
    /// Whether the grid has the keys.
    pub keys: bool,
}

/// A grid's keys' cursor kept from frame to frame, with what it was kept
/// for: the one chosen the grid was given, its count, the pass.
#[derive(Clone, Copy)]
struct Kept {
    cursor: usize,
    given: Option<usize>,
    count: usize,
    pass: u64,
}

/// A grid of cells in a scroll of someone else's: one rectangle taken for
/// all the cells, only the rows in sight laid out and painted (and their
/// cells' responses made), one Tab stop for the grid, the keys moving a
/// cursor the grid keeps, brought into sight as it moves.
#[derive(Debug)]
pub struct GalleryGrid {
    id: Id,
    count: usize,
    sizing: Sizing,
    side: f32,
    tile: Vec2,
    columns: usize,
    label: f32,
    min_gap: f32,
    /// Between rows where not the layout's own ([`GalleryGrid::row_gap`]).
    row_gap: Option<f32>,
    ahead: usize,
    keys_allowed: bool,
    cursor: Option<usize>,
    name: String,
}

impl GalleryGrid {
    /// A grid of `count` cells, its id `id`, that grow from `side` with two
    /// lines of name under each ([`GalleryGrid::label`] to change it).
    pub fn new(id: Id, count: usize, side: f32) -> GalleryGrid {
        GalleryGrid {
            id,
            count,
            sizing: Sizing::Grow,
            side,
            tile: Vec2::splat(side),
            columns: 1,
            label: label_height(false, 2, false),
            min_gap: JUSTIFIED_GAP,
            row_gap: None,
            ahead: 1,
            keys_allowed: true,
            cursor: None,
            name: "Pictures".into(),
        }
    }

    /// The room under each tile ([`label_height`]).
    pub fn label(mut self, height: f32) -> Self {
        self.label = height;
        self
    }

    /// Fixed tiles of `tile`, justified at the least gap `min_gap`.
    pub fn justified(mut self, tile: Vec2, min_gap: f32) -> Self {
        (self.sizing, self.tile, self.min_gap) = (Sizing::Justified, tile, min_gap);
        self
    }

    /// Rows `gap` apart, whatever the layout's own (Select Portrait's
    /// portraits: justified at least 8 apart across, rows 3 apart; M4's
    /// U26).
    pub fn row_gap(mut self, gap: f32) -> Self {
        self.row_gap = Some(gap);
        self
    }

    /// Exactly `columns` columns sharing the row.
    pub fn columns(mut self, columns: usize) -> Self {
        (self.sizing, self.columns) = (Sizing::Columns, columns);
        self
    }

    /// The small variant's fixed tiles of `side`, [`SMALL_GAP`] apart.
    pub fn small(mut self) -> Self {
        self.sizing = Sizing::Fixed;
        self.label = label_height(true, 1, false);
        self
    }

    /// Rows below those in sight whose cells are counted ahead (their
    /// pictures asked for before they come into sight): 1 at first.
    pub fn ahead(mut self, rows: usize) -> Self {
        self.ahead = rows;
        self
    }

    /// Whether the keys may act (not while a menu, a dialog or a modal is
    /// in front, which the caller knows).
    pub fn keys_allowed(mut self, allowed: bool) -> Self {
        self.keys_allowed = allowed;
        self
    }

    /// The cell the keys start from (the one chosen) where the grid has
    /// kept none.
    pub fn cursor(mut self, cursor: Option<usize>) -> Self {
        self.cursor = cursor;
        self
    }

    /// The grid's accessible name.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// The layout of the cells in `room`.
    pub fn layout(&self, room: f32) -> GridLayout {
        let mut layout = match self.sizing {
            Sizing::Grow => GridLayout::grow(room, self.side, self.label),
            Sizing::Justified => GridLayout::justified(room, self.tile, self.label, self.min_gap),
            Sizing::Columns => GridLayout::columns(room, self.columns, self.label),
            Sizing::Fixed => GridLayout::small(room, self.side, self.label),
        };
        if let Some(gap) = self.row_gap {
            layout.gap.y = gap;
        }
        layout
    }

    /// The grid's id: its keys' cursor is kept under it.
    pub fn id(&self) -> Id {
        self.id
    }

    /// Lays out the grid in `ui`'s width (its visible part) and calls
    /// `cell` for each cell in sight, in order: with the cell's index, its
    /// rectangle, its tile's size, its response (a click and a drag,
    /// never a Tab stop of its own) and its state.
    pub fn show(
        self,
        ui: &mut Ui,
        mut cell: impl FnMut(&mut Ui, usize, Rect, Vec2, &Response, CellState),
    ) -> GridOutput {
        // (The row of the pane's visible width: a pane once wider than it
        // shows keeps that width, and the cells would run past its edge.)
        let seen = ui.clip_rect().right() - ui.cursor().left();
        let room = ui.available_width().min(seen).max(1.0);
        let layout = self.layout(room);
        let height = layout.height(self.count);
        let (rect, _) = ui.allocate_exact_size(vec2(room, height), Sense::hover());
        let id = self.id;
        // (The grid's response is its own, its one Tab stop, named for a
        // screen reader.)
        let grid = ui.interact(rect, id, Sense::focusable_noninteractive());
        let name = self.name.clone();
        grid.widget_info(|| WidgetInfo::labeled(WidgetType::Other, true, &name));
        // (A list to a screen reader, of all its cells: those out of sight
        // are not built, and each built one says its place among them.)
        said_as_list(ui.ctx(), id, self.count);
        let keys = grid.has_focus() && self.keys_allowed;
        if grid.has_focus() {
            let filter = egui::EventFilter {
                horizontal_arrows: true,
                vertical_arrows: true,
                tab: false,
                escape: false,
            };
            ui.memory_mut(|m| m.set_focus_lock_filter(id, filter));
        }
        // (The cursor kept is the grid's while it is shown frame after
        // frame on the same list from the same one chosen: a gallery
        // opened again, or its list or the one chosen changed, starts
        // from the one chosen.)
        let pass = ui.ctx().cumulative_pass_nr();
        let kept: Option<Kept> = ui.data(|d| d.get_temp(id.with("cursor")));
        let kept = kept
            .filter(|k| k.pass + 1 >= pass && k.given == self.cursor && k.count == self.count)
            .map(|k| k.cursor);
        let mut cursor = kept.or(self.cursor).filter(|c| *c < self.count);
        let mut out = GridOutput {
            rect,
            layout: Some(layout),
            drawn: 0..0,
            ahead: 0..0,
            moved: None,
            activated: None,
            keys,
        };
        // The keys: the cursor moves, and is brought into sight.
        if keys && self.count > 0 {
            let clip = ui.clip_rect();
            let page = ((clip.height() / (layout.cell.y + layout.gap.y)).floor() as usize).max(1);
            if let Some(key) = GridKey::pressed(ui) {
                let from = cursor.unwrap_or(0);
                let to = match cursor {
                    None => Some(0),
                    Some(_) => step(from, self.count, layout.columns, page, key),
                };
                if let Some(to) = to.filter(|to| Some(*to) != cursor) {
                    cursor = Some(to);
                    out.moved = Some(to);
                    ui.scroll_to_rect(layout.rect(rect.min, to), None);
                }
            }
            let act = ui.input(|i| {
                i.modifiers.is_none()
                    && (i.key_pressed(egui::Key::Enter) || i.key_pressed(egui::Key::Space))
            });
            if act {
                out.activated = cursor;
            }
        }
        // The rows in sight, and those ahead.
        let clip = ui.clip_rect();
        let rows = layout.rows_in(rect.top(), clip.y_range(), self.count);
        out.drawn = layout.cells_of(rows.clone(), self.count);
        let rows_ahead = rows.start..(rows.end + self.ahead).min(layout.rows(self.count));
        out.ahead = layout.cells_of(rows_ahead, self.count);
        let mut clicked = None;
        for index in out.drawn.clone() {
            let at = layout.rect(rect.min, index);
            let r = ui.interact(at, id.with(index), Sense::CLICK | Sense::DRAG);
            if r.clicked() || r.drag_started() {
                clicked = Some(index);
            }
            let state = CellState { hovered: r.hovered(), cursor: cursor == Some(index), keys };
            cell(ui, index, at, layout.tile, &r, state);
            said_in_list(ui.ctx(), r.id, index, self.count);
        }
        // A cell clicked takes the keys' cursor, and the grid the keys.
        if let Some(i) = clicked {
            cursor = Some(i);
            ui.memory_mut(|m| m.request_focus(id));
        }
        // (The keys' cell, built, told to a screen reader as the grid's
        // active one.)
        if let Some(c) = cursor.filter(|c| out.drawn.contains(c)) {
            let cell = id.with(c).accesskit_id();
            ui.ctx().accesskit_node_builder(id, |n| n.set_active_descendant(cell));
        }
        match cursor {
            Some(cursor) => ui.data_mut(|d| {
                let kept = Kept { cursor, given: self.cursor, count: self.count, pass };
                d.insert_temp(id.with("cursor"), kept);
            }),
            None => ui.data_mut(|d| d.remove::<Kept>(id.with("cursor"))),
        }
        out
    }
}

/// The pictures' size slider: `side` in `range` (its value not shown,
/// `text` beside it where given), in steps of a point; the side moved to,
/// when it is moved. Its accessible value is the side.
pub fn size_slider(
    ui: &mut Ui,
    side: &mut f32,
    range: std::ops::RangeInclusive<f32>,
    text: Option<&str>,
) -> Response {
    let mut slider = egui::Slider::new(side, range).show_value(false).step_by(1.0);
    if let Some(text) = text {
        slider = slider.text(text);
    }
    crate::controls::slider(ui, slider)
}

/// Says the grid `id` is a list of `count` cells to a screen reader
/// ([`said_in_list`] for each cell built).
pub fn said_as_list(ctx: &egui::Context, id: Id, count: usize) {
    ctx.accesskit_node_builder(id, |n| {
        n.set_role(egui::accesskit::Role::List);
        n.set_size_of_set(count);
    });
}

/// Says the cell `id` is the `index`th (from 0) of a list of `count`.
pub fn said_in_list(ctx: &egui::Context, id: Id, index: usize, count: usize) {
    ctx.accesskit_node_builder(id, |n| {
        n.set_position_in_set(index + 1);
        n.set_size_of_set(count);
    });
}

/// Where an enlargement `size` large stands for a cell of a grid in
/// `panel` (a dock, a window), inside `window`: beside the panel, never
/// over it, at the cell's top (`cell`), on the side with room (left of
/// the panel first: a Right Dock's; right of it for a Left Dock's), held
/// inside the window top to bottom. With room on neither side: above the
/// cell, else below it, inside the window and never over the cell; with
/// room nowhere, none. Its top-left corner.
pub fn beside(cell: Rect, panel: Rect, window: Rect, size: Vec2) -> Option<Pos2> {
    let left = panel.left() - BESIDE_GAP - size.x;
    let right = panel.right() + BESIDE_GAP;
    let x = if left >= window.left() {
        Some(left)
    } else if right + size.x <= window.right() {
        Some(right)
    } else {
        None
    };
    if let Some(x) = x {
        let y = cell.top().min(window.bottom() - size.y).max(window.top());
        return Some(pos2(x, y));
    }
    // (No room either way: over or under the cell, across the panel.)
    if size.x > window.width() {
        return None;
    }
    let x = (cell.center().x - size.x / 2.0).clamp(window.left(), window.right() - size.x);
    let above = cell.top() - BESIDE_GAP - size.y;
    let below = cell.bottom() + BESIDE_GAP;
    if above >= window.top() {
        Some(pos2(x, above))
    } else if below + size.y <= window.bottom() {
        Some(pos2(x, below))
    } else {
        None
    }
}

/// Shows `content` as a cell's enlargement beside `panel` ([`beside`]): a
/// tooltip's frame, `width` wide, in the tooltip layer, taking no pointer.
/// `what` tells what it shows (the cell, whether its picture is made...):
/// where it changes, the enlargement is measured unseen for a frame first,
/// so it never stands where the last one's size put it (out of the window,
/// or over the cell). `None` where there is no room for it.
pub fn show_beside<R>(
    ctx: &egui::Context,
    id: Id,
    cell: Rect,
    panel: Rect,
    width: f32,
    what: impl std::hash::Hash + std::fmt::Debug,
    content: impl FnOnce(&mut Ui) -> R,
) -> Option<R> {
    let window = ctx.content_rect();
    // (Its size as drawn last time, and of what; a first guess before.)
    let what = Id::new(what);
    let kept: Option<(Id, Vec2)> = ctx.data(|d| d.get_temp(id.with("size")));
    let fresh = kept.is_none_or(|(was, _)| was != what);
    let size = kept.map_or(vec2(width, width + 60.0), |(_, size)| size);
    // (Measured unseen, where in the window does not matter.)
    let at = match fresh {
        true => window.left_top(),
        false => beside(cell, panel, window, size)?,
    };
    let shown = egui::Area::new(id)
        .order(egui::Order::Tooltip)
        .interactable(false)
        .fixed_pos(at)
        .pivot(Align2::LEFT_TOP)
        .constrain(false)
        .sizing_pass(fresh)
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style())
                .fill(ui.ctx().tokens().tooltip_bg)
                .show(ui, |ui| {
                    ui.set_width((width - 2.0 * ui.spacing().window_margin.leftf()).max(0.0));
                    content(ui)
                })
                .inner
        });
    let drawn = shown.response.rect.size();
    if kept != Some((what, drawn)) {
        ctx.data_mut(|d| d.insert_temp(id.with("size"), (what, drawn)));
        ctx.request_repaint();
    }
    Some(shown.inner)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every character 7 points wide, an ellipsis too.
    fn fixed(s: &str) -> f32 {
        7.0 * s.chars().count() as f32
    }

    #[test]
    fn a_name_of_one_word_is_one_line_cut_with_an_ellipsis() {
        let lines = name_lines("Bookshelf", 1000.0, 2, &mut fixed);
        assert_eq!(lines, ["Bookshelf"]);
        // (Ten characters' room: nine and the ellipsis.)
        let lines = name_lines("Supercalifragilistic", 70.0, 2, &mut fixed);
        assert_eq!(lines, ["Supercali…"]);
        assert!(fixed(&lines[0]) <= 70.0);
        // A model name is one word whatever it holds.
        let lines = name_lines("tcn01_a02_01 x", 70.0, 1, &mut fixed);
        assert_eq!(lines, ["tcn01_a02…"]);
    }

    #[test]
    fn several_words_wrap_between_words_on_two_lines_then_an_ellipsis() {
        // Two lines whole: never broken inside a word.
        let lines = name_lines("Plague Victim Pyre", 100.0, 2, &mut fixed);
        assert_eq!(lines, ["Plague Victim", "Pyre"]);
        for l in &lines {
            assert!(fixed(l) <= 100.0, "{l}");
        }
        // Three lines' worth: the second ends in an ellipsis.
        let lines = name_lines("Flag Pennant Banner Wall Hanging", 100.0, 2, &mut fixed);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], "Flag Pennant");
        assert!(lines[1].ends_with('…') && fixed(&lines[1]) <= 100.0, "{lines:?}");
        assert!(lines[1].starts_with("Banner"), "{lines:?}");
        // A word too wide for a line on its own is cut on its line.
        let lines = name_lines("Big Supercalifragilistic", 70.0, 2, &mut fixed);
        assert_eq!(lines, ["Big", "Supercali…"]);
        // One line fits all.
        assert_eq!(name_lines("Wrecked Ballista", 200.0, 2, &mut fixed), ["Wrecked Ballista"]);
        assert!(name_lines("  ", 200.0, 2, &mut fixed).is_empty());
    }

    /// Cells that grow share the row with no ragged edge: the row's cells
    /// and gaps fill the room to within a point a column, and none is
    /// wider than the share, nor a lone cell more than twice the side.
    /// (The Appearance Gallery's test of its own `fitted`, which this
    /// replaces.)
    #[test]
    fn grown_cells_fill_the_width() {
        // 500 points, pictures of 128 with 4 between: three fit, and grow
        // to fill it.
        let (per_row, side) = grown(500.0, 128.0, 4.0);
        assert_eq!(per_row, 3);
        assert!(3.0 * side + 2.0 * 4.0 <= 500.0 && 3.0 * side + 2.0 * 4.0 > 496.0, "{side}");
        // Narrower than one: the one, as wide as the room.
        assert_eq!(grown(100.0, 128.0, 4.0), (1, 100.0));
        // A lone picture isn't blown up past twice its size.
        assert_eq!(grown(250.0, 128.0, 4.0).0, 1);
    }

    #[test]
    fn justified_rows_have_no_ragged_edge() {
        for room in [200.0_f32, 263.0, 320.0, 466.0, 480.0, 741.0, 1000.0] {
            for side in [64.0_f32, 128.0, 220.0] {
                let l = GridLayout::grow(room, side, 45.0);
                let row = l.width();
                assert!(row <= room, "{room} {side}: {row}");
                if (l.columns as f32) * side * GROW_AT_MOST + (l.columns - 1) as f32 * GAP.x >= room
                {
                    assert!(room - row < l.columns as f32, "{room} {side}: {row}");
                }
                assert!(l.cell.x >= side.min(room) - 1.0 && l.cell.x <= side * GROW_AT_MOST);
                assert_eq!(l.gap, GAP);
            }
        }
        // Fixed cells, justified: the last cell of a row reaches the edge,
        // the gaps at least the least; the last row keeps the columns.
        let l = GridLayout::justified(500.0, vec2(54.0, 84.0), 0.0, JUSTIFIED_GAP);
        assert_eq!(l.columns, 8);
        let last = l.rect(Pos2::ZERO, 7);
        assert!((last.right() - 500.0).abs() < 0.01, "{last:?}");
        assert!(l.gap.x >= JUSTIFIED_GAP);
        assert_eq!(l.rect(Pos2::ZERO, 9).left(), l.rect(Pos2::ZERO, 1).left());
        // The small variant: fixed, 6 apart from the left.
        let s = GridLayout::small(300.0, 84.0, 17.0);
        assert_eq!((s.columns, s.gap), (3, Vec2::splat(SMALL_GAP)));
        assert_eq!(s.rect(Pos2::ZERO, 1).left(), 90.0);
        // Columns: exactly so many.
        assert_eq!(GridLayout::columns(700.0, 7, 0.0).columns, 7);
    }

    #[test]
    fn rows_in_sight_and_their_cells() {
        let l = GridLayout::grow(320.0, 128.0, 36.0);
        assert_eq!(l.columns, 2);
        let pitch = l.cell.y + l.gap.y;
        let count = 5000;
        assert_eq!(l.rows_in(0.0, egui::Rangef::new(0.0, pitch - 1.0), count), 0..1);
        let r = l.rows_in(0.0, egui::Rangef::new(10.0 * pitch + 1.0, 12.0 * pitch), count);
        assert_eq!(r, 10..13);
        assert_eq!(l.cells_of(r, count), 20..26);
        // Above the grid, and past its end.
        assert_eq!(l.rows_in(1000.0, egui::Rangef::new(0.0, 500.0), count), 0..0);
        assert_eq!(l.rows_in(0.0, egui::Rangef::new(1e9, 2e9), count), 2500..2500);
    }

    #[test]
    fn the_keys_step_by_row_by_one_and_by_page() {
        use GridKey::*;
        // Ten cells, three across: rows 0-2, 3-5, 6-8, 9.
        let s = |at, key| step(at, 10, 3, 2, key);
        assert_eq!(s(1, Down), Some(4));
        assert_eq!(s(4, Up), Some(1));
        assert_eq!(s(1, Up), Some(1), "no row above");
        assert_eq!(s(7, Down), Some(9), "onto the shorter last row");
        assert_eq!(s(9, Down), Some(9));
        assert_eq!(s(2, Right), Some(3), "on to the next row");
        assert_eq!(s(3, Left), Some(2));
        assert_eq!(s(0, Left), Some(0));
        assert_eq!(s(9, Right), Some(9));
        assert_eq!((s(5, Home), s(5, End)), (Some(0), Some(9)));
        assert_eq!(s(1, PageDown), Some(7));
        assert_eq!(s(8, PageDown), Some(9));
        assert_eq!(s(8, PageUp), Some(2));
        assert_eq!(s(2, PageUp), Some(0));
        assert_eq!(step(0, 0, 3, 2, Down), None);
    }

    #[test]
    fn an_enlargement_stands_beside_the_panel_inside_the_window() {
        let window = Rect::from_min_size(Pos2::ZERO, vec2(1600.0, 900.0));
        let size = vec2(200.0, 260.0);
        // A Right Dock: on its left, at the cell's top.
        let dock = Rect::from_min_max(pos2(1160.0, 30.0), pos2(1600.0, 876.0));
        let cell = Rect::from_min_size(pos2(1180.0, 411.0), vec2(128.0, 160.0));
        let at = beside(cell, dock, window, size);
        assert_eq!(at, Some(pos2(1160.0 - BESIDE_GAP - 200.0, 411.0)));
        // A Left Dock: on its right.
        let left = Rect::from_min_max(pos2(0.0, 30.0), pos2(260.0, 876.0));
        let cell = Rect::from_min_size(pos2(20.0, 100.0), vec2(128.0, 160.0));
        assert_eq!(beside(cell, left, window, size).unwrap().x, 260.0 + BESIDE_GAP);
        // A cell low in the window: held inside it.
        let low = Rect::from_min_size(pos2(1180.0, 800.0), vec2(128.0, 160.0));
        let at = beside(low, dock, window, size).unwrap();
        assert_eq!(at.y + size.y, 900.0);
        // Never over the panel, never outside the window.
        for x in [0.0_f32, 100.0, 700.0, 1400.0] {
            for w in [200.0_f32, 600.0, 1590.0] {
                let panel = Rect::from_min_size(pos2(x, 30.0), vec2(w.min(1600.0 - x), 800.0));
                let Some(at) = beside(cell, panel, window, size) else { continue };
                let shown = Rect::from_min_size(at, size);
                assert!(window.contains_rect(shown), "{x} {w}: {shown:?}");
                let room = (panel.left() - window.left()).max(window.right() - panel.right());
                if room >= size.x + BESIDE_GAP {
                    assert!(!shown.intersects(panel), "{x} {w}: {shown:?} over {panel:?}");
                }
            }
        }
    }

    #[test]
    fn with_room_on_neither_side_an_enlargement_never_covers_its_cell() {
        let window = Rect::from_min_size(Pos2::ZERO, vec2(800.0, 700.0));
        let size = vec2(200.0, 260.0);
        // A panel across the whole window: no room left or right of it.
        let panel = Rect::from_min_max(pos2(0.0, 30.0), pos2(800.0, 676.0));
        // A cell low: above it, inside the window.
        let low = Rect::from_min_size(pos2(700.0, 400.0), vec2(90.0, 120.0));
        let at = beside(low, panel, window, size).expect("above the cell");
        let shown = Rect::from_min_size(at, size);
        assert!(window.contains_rect(shown), "{shown:?}");
        assert!(!shown.intersects(low), "{shown:?} over {low:?}");
        assert_eq!(shown.bottom(), low.top() - BESIDE_GAP, "right above it");
        // A cell high: below it.
        let high = Rect::from_min_size(pos2(10.0, 40.0), vec2(90.0, 120.0));
        let shown = Rect::from_min_size(beside(high, panel, window, size).unwrap(), size);
        assert!(window.contains_rect(shown) && !shown.intersects(high), "{shown:?}");
        assert_eq!(shown.top(), high.bottom() + BESIDE_GAP, "right under it");
        // A cell as tall as the window allows neither: none.
        let tall = Rect::from_min_size(pos2(10.0, 100.0), vec2(90.0, 500.0));
        assert_eq!(beside(tall, panel, window, size), None);
        // Nor a window narrower than the enlargement.
        let narrow = Rect::from_min_size(Pos2::ZERO, vec2(150.0, 700.0));
        assert_eq!(beside(low, panel, narrow, size), None);
        // Every cell of the panel: never over it, never outside the window.
        for x in (0..8).map(|i| i as f32 * 100.0) {
            for y in (0..6).map(|i| 30.0 + i as f32 * 100.0) {
                let cell = Rect::from_min_size(pos2(x, y), vec2(90.0, 120.0));
                if let Some(at) = beside(cell, panel, window, size) {
                    let shown = Rect::from_min_size(at, size);
                    assert!(window.contains_rect(shown), "{cell:?}: {shown:?}");
                    assert!(!shown.intersects(cell), "{cell:?}: {shown:?}");
                }
            }
        }
    }
}
