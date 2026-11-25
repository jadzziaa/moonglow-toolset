//! The script editor's text laid out a line at a time.
//!
//! egui's `TextEdit` asks for the whole text laid out (a galley) each time
//! the text changes. Laid out whole, the game's `nwscript.nss` (14,000
//! lines) took 14 ms a key, and 100 ms at a typist's pace: egui keeps the
//! laid-out lines of a text only while they are asked for at every frame,
//! and the editor's frames between keys did not ask. Here each line keeps
//! its own galley while its text and its colors stay the same, and a change lays out again only the lines it touched; the
//! editor's galley is the lines' put together (`Galley::concat`, as egui
//! puts the lines of a text together itself). The line numbers beside
//! them are kept the same way.

use std::sync::Arc;

use egui::text::{LayoutJob, LayoutSection, TextFormat};
use egui::{Color32, Galley, Ui};
use mg_script::lex::{Token, TokenKind, tokenize};

use crate::script_view::{Palette, element};

/// The id of what [`watch_fonts`] keeps.
fn fonts_id() -> egui::Id {
    egui::Id::new("script-layout-fonts")
}

/// Counts the times egui made its fonts again (its font atlas full, or the
/// text options changed): a galley laid out before is drawn from the old
/// atlas's places, wrong, and the editors' kept lines are laid out again.
/// Called once a frame (a tiny text egui keeps laid out while it is asked
/// for: another galley for it is another font atlas).
pub(crate) fn watch_fonts(ctx: &egui::Context) {
    let probe = ctx
        .fonts_mut(|f| f.layout_no_wrap("0".into(), egui::FontId::monospace(10.0), Color32::WHITE));
    ctx.data_mut(|d| {
        let (held, made) = d.get_temp_mut_or_insert_with(fonts_id(), || (probe.clone(), 0_u64));
        if !Arc::ptr_eq(held, &probe) {
            *held = probe;
            *made += 1;
        }
    });
}

/// How many times egui has made its fonts again ([`watch_fonts`]).
pub(crate) fn fonts_made(ctx: &egui::Context) -> u64 {
    ctx.data(|d| d.get_temp::<(Arc<Galley>, u64)>(fonts_id()).map_or(0, |(_, made)| made))
}

/// What the galleys were laid out with: any of it changed, every line is
/// laid out again.
#[derive(Debug, Clone, Copy, PartialEq)]
struct MadeWith {
    palette: Palette,
    /// The line numbers' color.
    weak: Color32,
    pixels_per_point: f32,
    /// [`fonts_made`].
    fonts: u64,
}

/// A script editor's text and line numbers, laid out lazily a line at a time
/// and kept between frames.
#[derive(Debug, Clone, Default)]
pub(crate) struct LaidOut {
    made_with: Option<MadeWith>,
    /// Text hash corresponding to `lines` (`None` until [`LaidOut::sync_full`]
    /// or [`LaidOut::apply_edit`] runs).
    synced_hash: Option<u64>,
    /// Each line, with its byte and character bounds.
    lines: Vec<Line>,
    /// Shaped galley per line (`None` for off-screen lines not yet painted or queried).
    galleys: Vec<Option<Arc<Galley>>>,
    /// Full concatenated galley, built lazily only when AccessKit or unit tests ask.
    galley: Option<Arc<Galley>>,
    /// Each line's number galley (`None` until painted or queried).
    numbers: Vec<Option<Arc<Galley>>>,
    /// Total UTF-8 character count of `lines`.
    total_chars: usize,
    /// Widest line in display columns (tabs counted as 4), used to estimate
    /// horizontal scroll bounds before off-screen lines are shaped.
    max_cols: usize,
    /// Widest shaped line galley seen so far in points.
    max_width: f32,
    /// Measured monospace column advance (`"M"`).
    char_advance: f32,
    /// Pixel-snapped line height for `made_with`.
    row_height: f32,
    /// Active IME preedit character range and sub-range inside it.
    ime_preedit: Option<(egui::text::CCursorRange, Option<std::ops::Range<egui::text::CCursor>>)>,
    /// Last pointer/key interaction time for caret blink phase.
    last_interaction_time: f64,
}

/// Output returned by [`code_edit`] for one frame.
pub(crate) struct CodeEditOutput {
    pub(crate) response: egui::Response,
    pub(crate) text_origin: egui::Pos2,
    pub(crate) gutter_rect: egui::Rect,
    pub(crate) cursor_range: Option<egui::text::CCursorRange>,
    pub(crate) current_band: Option<egui::Rangef>,
    pub(crate) cursor_line_col: (usize, usize),
}

fn count_chars(s: &str) -> usize {
    if s.is_ascii() { s.len() } else { s.chars().count() }
}

fn est_line_cols(s: &str, char_len: usize) -> usize {
    let tabs = s.bytes().filter(|&b| b == b'\t').count();
    char_len + tabs * 3
}

impl LaidOut {
    /// Readies the editor's galleys for a frame drawn with `palette`: those
    /// laid out otherwise (other colors, another font size or scale, egui's
    /// fonts made again) are let go of.
    pub(crate) fn begin(&mut self, ui: &Ui, palette: &Palette) {
        let ppp = ui.ctx().pixels_per_point();
        let made_with = MadeWith {
            palette: *palette,
            weak: ui.visuals().weak_text_color(),
            pixels_per_point: ppp,
            fonts: fonts_made(ui.ctx()),
        };
        if self.made_with != Some(made_with) {
            *self = LaidOut {
                made_with: Some(made_with),
                row_height: palette.row_height(ppp),
                ..LaidOut::default()
            };
        } else if self.row_height == 0.0 {
            self.row_height = palette.row_height(ppp);
        }
    }

    pub(crate) fn line_count(&self) -> usize {
        self.lines.len().max(1)
    }

    pub(crate) fn row_height(&self) -> f32 {
        self.row_height.max(1.0)
    }

    pub(crate) fn row_y(&self, line: usize, ppp: f32) -> f32 {
        ((line as f32 * self.row_height) * ppp).round() / ppp
    }

    /// Ensures line metadata (`lines`, `total_chars`, `max_cols`) matches `text`
    /// (`h`), leaving off-screen galleys `None` for lazy shaping on demand.
    pub(crate) fn sync_full(&mut self, ui: &Ui, text: &str, h: u64, palette: &Palette) {
        self.begin(ui, palette);
        if self.char_advance <= 0.0 {
            self.char_advance = ui
                .painter()
                .layout_no_wrap("M".into(), palette.font(), Color32::WHITE)
                .size()
                .x
                .max(1.0);
        }
        if self.synced_hash == Some(h) {
            return;
        }
        let mut was: std::collections::HashMap<u64, Arc<Galley>> = std::collections::HashMap::new();
        for (line, galley) in self.lines.iter().zip(self.galleys.drain(..)) {
            if let Some(g) = galley {
                was.insert(line.print, g);
            }
        }
        self.lines.clear();
        let mut tokens = Vec::new();
        let (mut start, mut within, mut char_pos) = (0, Within::Code, 0);
        let mut max_cols = 0;
        let mut max_width: f32 = 0.0;
        loop {
            let end = text[start..].find('\n').map_or(text.len(), |i| start + i);
            let next = lex_line(text, start, end, within, &mut tokens);
            let print = fingerprint(text, start, end, &tokens);
            let slice = &text[start..end];
            let char_len = count_chars(slice);
            let cols = est_line_cols(slice, char_len);
            max_cols = max_cols.max(cols);
            let galley = was.get(&print).cloned();
            if let Some(g) = &galley {
                max_width = max_width.max(g.size().x);
            }
            self.lines.push(Line {
                start,
                end,
                char_start: char_pos,
                char_len,
                cols,
                within,
                print,
            });
            self.galleys.push(galley);
            char_pos += char_len;
            if end == text.len() {
                break;
            }
            char_pos += 1;
            (start, within) = (end + 1, next);
        }
        self.total_chars = char_pos;
        self.max_cols = max_cols;
        self.max_width = max_width;
        self.galley = None;
        self.synced_hash = Some(h);
    }

    /// Whether the lines are those of `text` (`h`, its hash): what places
    /// in the text may be read from them. A script's text replaced where
    /// its editor was not drawn (a tab behind another following the
    /// module, the NUI Creator writing it) leaves them those of the text
    /// before.
    pub(crate) fn in_sync(&self, text: &str, h: u64) -> bool {
        self.synced_hash == Some(h) && self.lines.last().is_some_and(|l| l.end == text.len())
    }

    /// Whether the lines are stamped as those of the text hashed `h`.
    pub(crate) fn stamped_in_sync(&self, h: u64) -> bool {
        self.synced_hash == Some(h)
    }

    /// Whether the lines' bounds are `text`'s lines' (its line ends, the
    /// characters before each), for tests.
    pub(crate) fn lines_are(&self, text: &str) -> bool {
        let mut start = 0;
        let mut chars = 0;
        for (i, l) in self.lines.iter().enumerate() {
            let end =
                text.get(start..).and_then(|t| t.find('\n')).map_or(text.len(), |e| start + e);
            let Some(own) = text.get(start..end) else { return false };
            if (l.start, l.end, l.char_start, l.char_len)
                != (start, end, chars, own.chars().count())
            {
                return false;
            }
            chars += l.char_len + 1;
            if end == text.len() {
                return i + 1 == self.lines.len() && self.total_chars == chars - 1;
            }
            start = end + 1;
        }
        false
    }

    /// Incrementally updates line metadata after `text` changed in byte range
    /// `edit_start..old_end` -> `edit_start..new_end`, without shaping any line.
    /// The lines must be those of the text before the edit (`old_hash`):
    /// otherwise they are only marked stale, for [`LaidOut::sync_full`] to
    /// make again (an edit applied to another text's lines stamped them as
    /// the new text's, wrong: review D1 of the adversarial branch).
    pub(crate) fn apply_edit(
        &mut self,
        text: &str,
        edit_start: usize,
        old_end: usize,
        new_end: usize,
        old_hash: u64,
        new_hash: u64,
    ) {
        let old_len = (text.len() + old_end).checked_sub(new_end);
        if self.synced_hash != Some(old_hash)
            || self.lines.last().map(|l| l.end) != old_len
            || old_end < edit_start
            || new_end < edit_start
        {
            self.synced_hash = None;
            return;
        }
        let delta = new_end as isize - old_end as isize;
        let first = self.lines.partition_point(|l| l.start <= edit_start).saturating_sub(1);
        let reached = self.lines.partition_point(|l| l.start <= old_end).min(self.lines.len());
        let mut was: std::collections::HashMap<u64, Arc<Galley>> = std::collections::HashMap::new();
        for i in first..reached {
            if let Some(g) = &self.galleys[i] {
                was.insert(self.lines[i].print, g.clone());
            }
        }
        let mut start = self.lines[first].start;
        let mut within = self.lines[first].within;
        let mut char_pos = self.lines[first].char_start;
        let (mut fresh, mut fresh_galleys) = (Vec::new(), Vec::new());
        let mut fresh_max_cols = 0;
        let mut tokens = Vec::new();
        let mut tail = self.lines.len();
        loop {
            if start > new_end {
                let at = (start as isize - delta) as usize;
                if let Ok(j) = self.lines.binary_search_by_key(&at, |l| l.start)
                    && self.lines[j].within == within
                {
                    tail = j;
                    break;
                }
            }
            let end = text[start..].find('\n').map_or(text.len(), |i| start + i);
            let next = lex_line(text, start, end, within, &mut tokens);
            let print = fingerprint(text, start, end, &tokens);
            let slice = &text[start..end];
            let char_len = count_chars(slice);
            let cols = est_line_cols(slice, char_len);
            fresh_max_cols = fresh_max_cols.max(cols);
            fresh.push(Line { start, end, char_start: char_pos, char_len, cols, within, print });
            fresh_galleys.push(was.get(&print).cloned());
            char_pos += char_len;
            if end == text.len() {
                break;
            }
            char_pos += 1;
            (start, within) = (end + 1, next);
        }
        drop(was);
        let shrank_max = self.lines[first..tail].iter().any(|l| l.cols >= self.max_cols);
        let kept = first + fresh.len();
        if tail < self.lines.len() {
            let char_delta = char_pos as isize - self.lines[tail].char_start as isize;
            self.lines.splice(first..tail, fresh);
            self.galleys.splice(first..tail, fresh_galleys);
            for l in &mut self.lines[kept..] {
                l.start = (l.start as isize + delta) as usize;
                l.end = (l.end as isize + delta) as usize;
                l.char_start = (l.char_start as isize + char_delta) as usize;
            }
            self.total_chars = (self.total_chars as isize + char_delta) as usize;
        } else {
            self.lines.splice(first..tail, fresh);
            self.galleys.splice(first..tail, fresh_galleys);
            self.total_chars = char_pos;
        }
        if fresh_max_cols > self.max_cols {
            self.max_cols = fresh_max_cols;
        } else if shrank_max {
            self.max_cols = self.lines.iter().map(|l| l.cols).max().unwrap_or(0);
        }
        self.galley = None;
        self.synced_hash = Some(new_hash);
    }

    /// Shapes line `i` on demand if `None`, returning its cached galley.
    pub(crate) fn ensure_line(
        &mut self,
        ui: &Ui,
        text: &str,
        i: usize,
        palette: &Palette,
    ) -> Arc<Galley> {
        if let Some(g) = &self.galleys[i] {
            return g.clone();
        }
        let line = self.lines[i];
        let mut tokens = Vec::new();
        lex_line(text, line.start, line.end, line.within, &mut tokens);
        let job = line_job(text, line.start, line.end, &tokens, palette);
        let g = ui.fonts_mut(|f| f.layout_job(job));
        self.max_width = self.max_width.max(g.size().x);
        self.galleys[i] = Some(g.clone());
        g
    }

    fn ensure_number(&mut self, ui: &Ui, i: usize, palette: &Palette) -> Arc<Galley> {
        if let Some(Some(g)) = self.numbers.get(i) {
            return g.clone();
        }
        if i >= self.numbers.len() {
            self.numbers.resize(i + 1, None);
        }
        let color = self.made_with.map_or(mg_uikit::Tokens::DARK.text_3, |m| m.weak);
        let g = ui.fonts_mut(|f| {
            let mut job = LayoutJob::default();
            job.append(&(i + 1).to_string(), 0.0, palette.format(color));
            f.layout_job(palette.pitched(job))
        });
        self.numbers[i] = Some(g.clone());
        g
    }

    pub(crate) fn ensure_gutter_width(&mut self, ui: &Ui, lines: usize, palette: &Palette) -> f32 {
        let lines = lines.max(1);
        if self.numbers.len() != lines {
            self.numbers.truncate(lines);
            if self.numbers.len() < lines {
                self.numbers.resize(lines, None);
            }
        }
        let widest = self.ensure_number(ui, lines - 1, palette).size().x;
        GUTTER_LEFT + widest + GUTTER_RIGHT
    }

    /// Maps character index `c` (0..=`total_chars`) to `(line, column_chars)`.
    pub(crate) fn line_col(&self, char_idx: usize) -> (usize, usize) {
        if self.lines.is_empty() {
            return (0, 0);
        }
        let clamped = char_idx.min(self.total_chars);
        let li = self.lines.partition_point(|l| l.char_start <= clamped).saturating_sub(1);
        let line = &self.lines[li];
        let col = (clamped - line.char_start).min(line.char_len);
        (li, col)
    }

    pub(crate) fn ccursor_at(&self, line: usize, col: usize) -> egui::text::CCursor {
        let li = line.min(self.lines.len().saturating_sub(1));
        let l = &self.lines[li];
        egui::text::CCursor::new(l.char_start + col.min(l.char_len))
    }

    pub(crate) fn byte_of_char(&self, text: &str, char_idx: usize) -> usize {
        if self.lines.is_empty() {
            return crate::script_tools::byte_index(text, char_idx);
        }
        let (li, col) = self.line_col(char_idx);
        let line = &self.lines[li];
        let slice = &text[line.start..line.end];
        if slice.is_ascii() {
            line.start + col.min(slice.len())
        } else {
            line.start + slice.char_indices().nth(col).map_or(slice.len(), |(b, _)| b)
        }
    }

    pub(crate) fn char_of_byte(&self, text: &str, byte_idx: usize) -> usize {
        if self.lines.is_empty() {
            return crate::script_tools::char_index(text, byte_idx);
        }
        let clamped = byte_idx.min(text.len());
        let li = self.lines.partition_point(|l| l.start <= clamped).saturating_sub(1);
        let line = &self.lines[li];
        let rel = clamped.saturating_sub(line.start).min(line.end - line.start);
        let slice = &text[line.start..line.start + rel];
        line.char_start + count_chars(slice)
    }

    pub(crate) fn line_moved_char(&self, from_char: usize, delta_lines: isize) -> usize {
        if self.lines.is_empty() {
            return 0;
        }
        let (li, col) = self.line_col(from_char);
        let target = li as isize + delta_lines;
        if target < 0 {
            return 0;
        }
        let Some(dst) = self.lines.get(target as usize) else {
            return self.total_chars;
        };
        dst.char_start + col.min(dst.char_len)
    }

    pub(crate) fn cursor_local_rect(
        &mut self,
        ui: &Ui,
        text: &str,
        palette: &Palette,
        c: egui::text::CCursor,
    ) -> egui::Rect {
        if self.lines.is_empty() {
            return egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(0.0, self.row_height()));
        }
        let (li, col) = self.line_col(usize::from(c.index));
        let g = self.ensure_line(ui, text, li, palette);
        let x = g.rows.first().map_or(0.0, |r| r.x_offset(egui::text::CharIndex::from(col)));
        let y = self.row_y(li, ui.ctx().pixels_per_point());
        egui::Rect::from_min_max(egui::pos2(x, y), egui::pos2(x, y + self.row_height()))
    }

    fn ccursor_from_local_pos(
        &mut self,
        ui: &Ui,
        text: &str,
        palette: &Palette,
        local: egui::Pos2,
    ) -> egui::text::CCursor {
        let li = ((local.y / self.row_height()).floor().max(0.0) as usize)
            .min(self.lines.len().saturating_sub(1));
        let g = self.ensure_line(ui, text, li, palette);
        let col = g.rows.first().map_or(0, |r| usize::from(r.char_at(local.x)));
        self.ccursor_at(li, col)
    }

    /// Total UTF-8 character count of the currently synced text.
    pub(crate) fn total_chars(&self) -> usize {
        self.total_chars
    }

    /// The text laid out whole (concatenated across all lines): used by tests
    /// and when AccessKit is enabled.
    pub(crate) fn text(&mut self, ui: &Ui, text: &str, palette: &Palette) -> Arc<Galley> {
        let h = crate::script_tools::hash(text);
        self.sync_full(ui, text, h, palette);
        if let Some(g) = &self.galley
            && g.text() == text
        {
            return g.clone();
        }
        let mut tokens = Vec::new();
        ui.fonts_mut(|f| {
            for (i, line) in self.lines.iter().enumerate() {
                if self.galleys[i].is_none() {
                    lex_line(text, line.start, line.end, line.within, &mut tokens);
                    let job = line_job(text, line.start, line.end, &tokens, palette);
                    self.galleys[i] = Some(f.layout_job(job));
                }
            }
        });
        let slices: Vec<Arc<Galley>> =
            self.galleys.iter().map(|g| g.as_ref().expect("all lines shaped").clone()).collect();
        let mut job = LayoutJob::single_section(
            text.to_owned(),
            TextFormat::simple(palette.font(), palette.colors[0]),
        );
        job.wrap.max_width = f32::INFINITY;
        let ppp = ui.ctx().pixels_per_point();
        let galley = Arc::new(Galley::concat(Arc::new(job), &slices, ppp));
        self.max_width = galley.size().x;
        self.galley = Some(galley.clone());
        galley
    }

    /// Lays out all `lines` line numbers immediately (for unit tests).
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn numbers(&mut self, ui: &Ui, lines: usize, palette: &Palette) {
        if self.numbers.len() == lines && self.numbers.iter().all(Option::is_some) {
            return;
        }
        let color = self.made_with.map_or(mg_uikit::Tokens::DARK.text_3, |m| m.weak);
        self.numbers.resize(lines, None);
        ui.fonts_mut(|f| {
            for (i, slot) in self.numbers.iter_mut().enumerate() {
                if slot.is_none() {
                    let mut job = LayoutJob::default();
                    job.append(&(i + 1).to_string(), 0.0, palette.format(color));
                    *slot = Some(f.layout_job(palette.pitched(job)));
                }
            }
        });
    }

    /// The gutter's width: the board's `.code .ln` (12 points, the widest
    /// number, 10 points), for the numbers laid out last.
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn gutter_width(&self) -> f32 {
        let widest = self.numbers.last().and_then(|o| o.as_ref()).map_or(0.0, |g| g.size().x);
        GUTTER_LEFT + widest + GUTTER_RIGHT
    }

    /// Paints visible line numbers and bookmark icons in `gutter` alongside the
    /// code rows intersecting `ui.clip_rect()`.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn paint_gutter_visible(
        &mut self,
        ui: &Ui,
        left: f32,
        text_origin: egui::Pos2,
        first_vis: usize,
        last_vis: usize,
        current: Option<usize>,
        current_color: Color32,
        palette: &Palette,
        marks: Marks<'_>,
    ) {
        let lines = self.line_count();
        let gutter_w = self.ensure_gutter_width(ui, lines, palette);
        let right = left + gutter_w - GUTTER_RIGHT;
        let ppp = ui.ctx().pixels_per_point();
        let weakest = self.made_with.map_or(current_color, |m| m.weak);
        for i in first_vis..last_vis.min(lines) {
            let top = text_origin.y + self.row_y(i, ppp);
            let number = self.ensure_number(ui, i, palette);
            let pos = egui::pos2(right - number.size().x, top);
            let painter = ui.painter();
            if current == Some(i) {
                painter.galley_with_override_text_color(pos, number, current_color);
            } else {
                painter.galley(pos, number, current_color);
            }
        }
        let row_h = self.row_height();
        paint_mark_badges(ui, palette.font(), weakest, marks, |line| {
            (first_vis..last_vis.min(lines)).contains(&line).then(|| {
                let top = text_origin.y + self.row_y(line, ppp);
                egui::Rect::from_min_size(
                    egui::pos2(left, top),
                    egui::vec2(GUTTER_LEFT - MARK_GAP, row_h),
                )
            })
        });
    }

    /// Paints visible line numbers and bookmark icons for callers holding a whole
    /// [`Galley`] (such as the read-only NUI source viewer).
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn paint_gutter(
        &self,
        ui: &Ui,
        left: f32,
        galley: &Galley,
        at: egui::Pos2,
        current: Option<usize>,
        current_color: Color32,
        marks: Marks<'_>,
    ) {
        let clip = ui.clip_rect();
        let right = left
            + GUTTER_LEFT
            + self.numbers.last().and_then(|o| o.as_ref()).map_or(0.0, |g| g.size().x);
        let first = galley.rows.partition_point(|r| at.y + r.pos.y + r.size.y < clip.top());
        let painter = ui.painter();
        for (i, row) in galley.rows.iter().enumerate().skip(first) {
            let top = at.y + row.pos.y;
            if top > clip.bottom() {
                break;
            }
            let Some(number) = self.numbers.get(i).and_then(|o| o.as_ref()) else { break };
            let pos = egui::pos2(right - number.size().x, top);
            if current == Some(i) {
                painter.galley_with_override_text_color(pos, number.clone(), current_color);
            } else {
                painter.galley(pos, number.clone(), current_color);
            }
        }
        let font = galley
            .job
            .sections
            .first()
            .map(|s| s.format.font_id.clone())
            .unwrap_or_else(|| egui::TextStyle::Monospace.resolve(ui.style()));
        let weakest = self.made_with.map_or(current_color, |m| m.weak);
        paint_mark_badges(ui, font, weakest, marks, |line| {
            let row = galley.rows.get(line)?;
            let top = at.y + row.pos.y;
            if top + row.size.y < clip.top() || top > clip.bottom() {
                return None;
            }
            Some(egui::Rect::from_min_size(
                egui::pos2(left, top),
                egui::vec2(GUTTER_LEFT - MARK_GAP, row.size.y),
            ))
        });
    }
}

fn paint_mark_badges(
    ui: &Ui,
    font: egui::FontId,
    weakest: Color32,
    marks: Marks<'_>,
    mut place: impl FnMut(usize) -> Option<egui::Rect>,
) {
    let painter = ui.painter();
    for &line in marks.bookmarks {
        if marks.numbered.contains(&Some(line)) {
            continue;
        }
        let Some(rect) = place(line) else { continue };
        let side = rect.height().min(rect.width());
        let square = egui::Rect::from_center_size(rect.center(), egui::Vec2::splat(side));
        mg_uikit::Icon::Bookmark.paint(painter, square, weakest);
    }
    for (n, line) in marks.numbered.iter().enumerate() {
        let Some(line) = *line else { continue };
        let Some(rect) = place(line) else { continue };
        let digit = painter.layout_no_wrap(n.to_string(), font.clone(), weakest);
        painter.galley(rect.center() - digit.size() / 2.0, digit, weakest);
    }
}

/// The gutter named for a screen reader by what it shows ("Line numbers 1
/// to 3000"), not by every number in it.
pub(crate) fn name_gutter(gutter: &egui::Response, lines: usize) {
    gutter.widget_info(|| {
        let said = format!("Line numbers 1 to {}", lines.max(1));
        egui::WidgetInfo::labeled(egui::WidgetType::Label, true, &said)
    });
}

/// The text field described for a screen reader by the cursor's line
/// ("Line 12 of 3000"): the gutter's number for it, said.
pub(crate) fn describe_line(field: &egui::Response, line: usize, lines: usize) {
    field.ctx.accesskit_node_builder(field.id, |node| {
        node.set_description(format!("Line {line} of {}", lines.max(1)));
    });
}

/// The room before the line numbers (`.code .ln`'s `padding-left`), where
/// a bookmark's mark is painted.
pub(crate) const GUTTER_LEFT: f32 = 12.0;

/// Between a bookmark's mark and the widest line number: the marks are
/// drawn in the rest of [`GUTTER_LEFT`] (the board draws no mark, so no
/// measure of its own; a 12-point icon there touched the numbers).
pub(crate) const MARK_GAP: f32 = 2.0;

/// The room after them (`.code .ln`'s `padding-right`).
pub(crate) const GUTTER_RIGHT: f32 = 10.0;

/// The text's frame: the room before the text and after it (`.code .tx`'s
/// `padding: 0 12px 0 8px`), none above or below (the board's first line
/// at the top of the code), and no border or fill (the kit's
/// `code_frame`, whose margins are a text field's).
pub(crate) fn text_frame() -> egui::Frame {
    egui::Frame::new().inner_margin(egui::Margin { left: 8, right: 12, top: 0, bottom: 0 })
}

/// A script's bookmarks, as the gutter marks them: the lines (from 0)
/// bookmarked, and the line of each numbered bookmark (0 to 9).
#[derive(Clone, Copy)]
pub(crate) struct Marks<'a> {
    pub bookmarks: &'a std::collections::BTreeSet<usize>,
    pub numbered: &'a [Option<usize>; 10],
}

/// An empty line: no text, one section for its font (its height).
fn empty_job(palette: &Palette, color: Color32) -> LayoutJob {
    let mut job = LayoutJob::default();
    job.sections.push(LayoutSection {
        leading_space: 0.0,
        byte_range: egui::text::ByteIndex(0)..egui::text::ByteIndex(0),
        format: palette.format(color),
    });
    palette.pitched(job)
}

/// What a line begins inside, as the script's tokens go: code, or a
/// block comment or raw string begun on a line before.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Within {
    Code,
    Comment,
    RawString,
}

/// A line of the text, laid out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Line {
    /// Where it starts and ends in bytes (excluding trailing `\n`).
    start: usize,
    end: usize,
    /// Where it starts and how many characters long it is.
    char_start: usize,
    char_len: usize,
    /// Estimated display columns (tabs counted as 4).
    cols: usize,
    within: Within,
    /// Its text and its tokens' colors (each one's element and place in
    /// the line): two lines alike in these are laid out alike.
    print: u64,
}

/// The tokens of the line `start..end` (its line end not in it), begun
/// `within` what the lines before left open, into `out` with their spans
/// in the text; and what the next line begins within. (As the whole
/// text's tokens are, cut at the line's ends: a block comment ends at the
/// first `*/`, a raw string at a `"` that isn't one of a `""`.)
fn lex_line(text: &str, start: usize, end: usize, within: Within, out: &mut Vec<Token>) -> Within {
    out.clear();
    let line = &text.as_bytes()[start..end];
    let whole = |kind| Token { kind, span: start..end };
    let mut from = 0;
    match within {
        Within::Code => {}
        Within::Comment => match line.windows(2).position(|w| w == b"*/") {
            Some(k) => {
                from = k + 2;
                let kind = TokenKind::BlockComment { terminated: true };
                out.push(Token { kind, span: start..start + from });
            }
            None => {
                if !line.is_empty() {
                    out.push(whole(TokenKind::BlockComment { terminated: false }));
                }
                return Within::Comment;
            }
        },
        Within::RawString => {
            let mut j = 0;
            loop {
                match line[j..].iter().position(|b| *b == b'"') {
                    Some(q) if line.get(j + q + 1) == Some(&b'"') => j += q + 2,
                    Some(q) => {
                        from = j + q + 1;
                        let kind = TokenKind::RawString { terminated: true };
                        out.push(Token { kind, span: start..start + from });
                        break;
                    }
                    None => {
                        if !line.is_empty() {
                            out.push(whole(TokenKind::RawString { terminated: false }));
                        }
                        return Within::RawString;
                    }
                }
            }
        }
    }
    let at = start + from;
    out.extend(
        tokenize(&line[from..])
            .into_iter()
            .map(|t| Token { kind: t.kind, span: at + t.span.start..at + t.span.end }),
    );
    match out.last().map(|t| t.kind) {
        Some(TokenKind::BlockComment { terminated: false }) => Within::Comment,
        Some(TokenKind::RawString { terminated: false }) => Within::RawString,
        _ => Within::Code,
    }
}

/// A line's fingerprint ([`Line::print`]), of its tokens ([`lex_line`]).
fn fingerprint(text: &str, start: usize, end: usize, tokens: &[Token]) -> u64 {
    // (FNV-1a over the tokens' pieces: element, start and end.)
    let mut colors: u64 = 0xcbf2_9ce4_8422_2325;
    for t in tokens {
        let (a, b) = (t.span.start - start, t.span.end - start);
        let piece = element(t.kind) as u64 | (a as u64) << 8 | (b as u64) << 36;
        colors = (colors ^ piece).wrapping_mul(0x0000_0100_0000_01b3);
    }
    egui::epaint::util::hash((&text[start..end], colors))
}

/// A line's layout: its tokens' pieces in their colors, the line
/// [`Palette::pitch`] high.
fn line_job(
    text: &str,
    _line_start: usize,
    _line_end: usize,
    tokens: &[Token],
    palette: &Palette,
) -> LayoutJob {
    let mut job = LayoutJob::default();
    for t in tokens.iter().filter(|t| t.span.end > t.span.start) {
        let color = palette.colors[element(t.kind)];
        job.append(&text[t.span.clone()], 0.0, palette.format(color));
    }
    // (An empty line: the height of its font. An empty text has no
    // sections, as it had laid out whole.)
    if job.sections.is_empty() && !text.is_empty() {
        return empty_job(palette, palette.colors[0]);
    }
    palette.pitched(job)
}

/// Replaces the characters `del` of the editor's text with `ins` as a step
/// of its history (`before` and `after`, the cursor it gives back on Undo
/// and on Redo), keeping the laid-out lines in step.
#[allow(clippy::too_many_arguments)]
fn replace(
    buf: &mut crate::script_view::ScriptBuffer,
    laid: &mut LaidOut,
    del: egui::text::CCursorRange,
    ins: &str,
    before: egui::text::CCursorRange,
    after: egui::text::CCursorRange,
    time: f64,
    mergeable: bool,
) {
    let [min, max] = del.sorted_cursors();
    let bf = laid.byte_of_char(&buf.text, usize::from(min.index));
    let bt = laid.byte_of_char(&buf.text, usize::from(max.index));
    let oh = buf.text_hash();
    let (at, old_end, new_end) =
        buf.replace_range_undo(bf..bt, ins, before, after, time, mergeable);
    let nh = buf.text_hash();
    laid.apply_edit(&buf.text, at, old_end, new_end, oh, nh);
}

/// [`replace`] for a composition's text (IME): the composition one step.
fn compose(
    buf: &mut crate::script_view::ScriptBuffer,
    laid: &mut LaidOut,
    del: egui::text::CCursorRange,
    ins: &str,
    after: egui::text::CCursorRange,
    time: f64,
) {
    let [min, max] = del.sorted_cursors();
    let bf = laid.byte_of_char(&buf.text, usize::from(min.index));
    let bt = laid.byte_of_char(&buf.text, usize::from(max.index));
    let oh = buf.text_hash();
    let (at, old_end, new_end) = buf.replace_range_composing(bf..bt, ins, del, after, time);
    let nh = buf.text_hash();
    laid.apply_edit(&buf.text, at, old_end, new_end, oh, nh);
}

/// Where a word move left from the character `cur` lands (Ctrl+Left, Alt+Left
/// on a Mac): egui's word before it, over line ends as egui finds it in the
/// whole text, from the lines before it only as far back as a line with
/// something other than spaces (not the whole text each key).
fn word_left(laid: &LaidOut, text: &str, cur: usize) -> usize {
    use egui::text_selection::text_cursor_state::ccursor_previous_word;
    let (li, _) = laid.line_col(cur);
    let to = laid.byte_of_char(text, cur);
    let mut k = li;
    loop {
        let from = laid.lines[k];
        let slice = &text[from.start..to];
        if k == 0 || slice.chars().any(|c| !c.is_whitespace()) {
            let n = count_chars(slice);
            let at = ccursor_previous_word(slice, egui::text::CCursor::new(n));
            return from.char_start + usize::from(at.index);
        }
        k -= 1;
    }
}

/// Where a word move right from the character `cur` lands (Ctrl+Right), as
/// [`word_left`] does: over line ends, as egui's in the whole text.
fn word_right(laid: &LaidOut, text: &str, cur: usize) -> usize {
    use egui::text_selection::text_cursor_state::ccursor_next_word;
    let (li, _) = laid.line_col(cur);
    let from = laid.byte_of_char(text, cur);
    let mut k = li;
    loop {
        let to = laid.lines[k];
        let slice = &text[from..to.end];
        if k + 1 == laid.lines.len() || slice.chars().any(|c| !c.is_whitespace()) {
            let at = ccursor_next_word(slice, egui::text::CCursor::new(0));
            return (cur + usize::from(at.index)).min(laid.total_chars);
        }
        k += 1;
    }
}

/// The start of the line of the character `c`.
fn line_start_of(laid: &LaidOut, c: usize) -> usize {
    let (li, _) = laid.line_col(c);
    laid.lines.get(li).map_or(0, |l| l.char_start)
}

/// The end of the line of the character `c` (before its line end).
fn line_end_of(laid: &LaidOut, c: usize) -> usize {
    let (li, _) = laid.line_col(c);
    laid.lines.get(li).map_or(0, |l| l.char_start + l.char_len)
}

/// The cursor a line up (`up`) or down from `range`'s, at its column as the
/// points across go (kept over short lines, `h_pos`); past the first line
/// the text's start, past the last its end.
fn vertical(
    ui: &Ui,
    laid: &mut LaidOut,
    text: &str,
    palette: &Palette,
    range: &mut egui::text::CCursorRange,
    up: bool,
) {
    let (li, _) = laid.line_col(usize::from(range.primary.index));
    let h_pos = range
        .h_pos
        .unwrap_or_else(|| laid.cursor_local_rect(ui, text, palette, range.primary).min.x);
    let lines = laid.lines.len();
    let to = if up { li.checked_sub(1) } else { Some(li + 1).filter(|l| *l < lines) };
    range.primary = match to {
        Some(l) => {
            let g = laid.ensure_line(ui, text, l, palette);
            let col = g.rows.first().map_or(0, |r| usize::from(r.char_at(h_pos)));
            laid.ccursor_at(l, col)
        }
        None if up => egui::text::CCursor::new(0),
        None => egui::text::CCursor::new(laid.total_chars),
    };
    range.h_pos = Some(h_pos);
}

impl LaidOut {
    /// Paints a line under the characters `from..to` at the foot of their
    /// rows (a composition's, IME), as egui underlines one.
    #[allow(clippy::too_many_arguments)]
    fn paint_underline(
        &mut self,
        ui: &Ui,
        painter: &egui::Painter,
        text: &str,
        palette: &Palette,
        origin: egui::Pos2,
        (from, to): (usize, usize),
        stroke: egui::Stroke,
    ) {
        if from >= to {
            return;
        }
        let ((fl, fc), (tl, tc)) = (self.line_col(from), self.line_col(to));
        let ppp = ui.ctx().pixels_per_point();
        for li in fl..=tl {
            let g = self.ensure_line(ui, text, li, palette);
            let Some(row) = g.rows.first() else { continue };
            let left = if li == fl { row.x_offset(egui::text::CharIndex::from(fc)) } else { 0.0 };
            let right =
                if li == tl { row.x_offset(egui::text::CharIndex::from(tc)) } else { row.size.x };
            let y = origin.y + self.row_y(li, ppp) + self.row_height();
            painter.line_segment(
                [egui::pos2(origin.x + left, y), egui::pos2(origin.x + right, y)],
                stroke,
            );
        }
    }
}

/// Viewport-virtualized multiline script editor built directly over [`LaidOut`]
/// and [`ScriptBuffer`], avoiding `egui::TextEdit`'s full-galley clones and
/// full-document undo snapshots. It answers the keys and the pointer as
/// egui's `TextEdit` does (its `events` and `pointer_interaction`); where it
/// does otherwise, it says so. `reveal`: the cursor brought into sight, the
/// text changed before it was drawn (the menus' Undo, Tab, a completion put
/// in: review N4).
#[allow(clippy::too_many_arguments)]
pub(crate) fn code_edit(
    ui: &mut Ui,
    key: mg_resman::ResKey,
    buf: &mut crate::script_view::ScriptBuffer,
    laid: &mut LaidOut,
    palette: &Palette,
    room: f32,
    reveal: bool,
) -> CodeEditOutput {
    use egui::text::{CCursor, CCursorRange};
    use egui::{Event, Key, Modifiers};

    let ctx = ui.ctx().clone();
    let id = crate::script_view::editor_id(key);
    let h = buf.text_hash();
    laid.sync_full(ui, &buf.text, h, palette);
    let mut state = egui::text_edit::TextEditState::load(&ctx, id).unwrap_or_default();
    let max_char = egui::text::CharIndex::from(laid.total_chars);
    let prev_cursor_range = state.cursor.char_range().map(|mut r| {
        r.primary.index = r.primary.index.min(max_char);
        r.secondary.index = r.secondary.index.min(max_char);
        r
    });
    let mut cursor_range = prev_cursor_range.unwrap_or_default();

    let os = ui.os();
    let has_focus = ctx.memory(|m| m.has_focus(id));
    let mut text_changed = false;
    let mut any_cursor_edit = false;
    let prev_text_for_a11y = if ctx.accesskit_node_builder(id, |_| ()).is_some() {
        Some(buf.text.clone())
    } else {
        None
    };

    if has_focus {
        let filter = egui::EventFilter {
            horizontal_arrows: true,
            vertical_arrows: true,
            tab: true,
            escape: false,
        };
        ctx.memory_mut(|m| m.set_focus_lock_filter(id, filter));
    } else {
        // (A composition goes with the keys: review M2.)
        laid.ime_preedit = None;
        buf.undo.end_composing();
    }

    // Keyboard, clipboard and IME events, before the editor's size is
    // taken, as egui's TextEdit has them: lines a paste adds are in the
    // room the scroll area is given this frame, and the cursor scrolled to
    // there at once (with smooth scrolling off, the scroll stopped short of
    // a paste at the text's end: review N6).
    if has_focus {
        let now = ui.input(|i| i.time);
        let events = ui.input(|i| {
            i.events
                .iter()
                .filter(|e| {
                    matches!(
                        e,
                        Event::Key { .. }
                            | Event::Text(_)
                            | Event::Paste(_)
                            | Event::Copy
                            | Event::Cut
                            | Event::Ime(_)
                            | Event::AccessKitActionRequest(_)
                    )
                })
                .cloned()
                .collect::<Vec<_>>()
        });
        for ev in &events {
            let total = laid.total_chars;
            let was = cursor_range;
            match ev {
                Event::Copy => {
                    if !cursor_range.is_empty() {
                        let [min, max] = cursor_range.sorted_cursors();
                        let bf = laid.byte_of_char(&buf.text, usize::from(min.index));
                        let bt = laid.byte_of_char(&buf.text, usize::from(max.index));
                        ui.copy_text(buf.text[bf..bt].to_owned());
                    }
                }
                Event::Cut => {
                    if !cursor_range.is_empty() {
                        let [min, max] = cursor_range.sorted_cursors();
                        let bf = laid.byte_of_char(&buf.text, usize::from(min.index));
                        let bt = laid.byte_of_char(&buf.text, usize::from(max.index));
                        ui.copy_text(buf.text[bf..bt].to_owned());
                        let next = CCursorRange::one(min);
                        replace(buf, laid, was, "", was, next, now, false);
                        cursor_range = next;
                        text_changed = true;
                    }
                }
                Event::Paste(ins) => {
                    if !ins.is_empty() {
                        let min = cursor_range.sorted_cursors()[0];
                        let next = CCursorRange::one(CCursor::new(min.index + count_chars(ins)));
                        replace(buf, laid, was, ins, was, next, now, false);
                        cursor_range = next;
                        text_changed = true;
                    }
                }
                Event::Text(ins) => {
                    if !ins.is_empty() && ins != "\n" && ins != "\r" {
                        let mergeable = cursor_range.is_empty();
                        let min = cursor_range.sorted_cursors()[0];
                        let next = CCursorRange::one(CCursor::new(min.index + count_chars(ins)));
                        replace(buf, laid, was, ins, was, next, now, mergeable);
                        cursor_range = next;
                        text_changed = true;
                    }
                }
                // A composition (IME) goes where the cursor is, as in
                // egui: its text replaces the selection, and each next text
                // the one before (the cursor's range while it goes on); the
                // whole of it is one step of the history (review M1, M2).
                Event::Ime(ime) => match ime {
                    egui::ImeEvent::Preedit { text: pre, .. }
                        if pre.is_empty() && laid.ime_preedit.is_none() => {}
                    egui::ImeEvent::Commit(c) if c.is_empty() && laid.ime_preedit.is_none() => {}
                    egui::ImeEvent::Preedit { text: pre, .. } | egui::ImeEvent::Commit(pre)
                        if pre == "\n" || pre == "\r" => {}
                    egui::ImeEvent::Preedit { text: pre, active_range_chars } => {
                        let min = cursor_range.sorted_cursors()[0];
                        let end = CCursor::new(min.index + count_chars(pre));
                        let next = CCursorRange::two(min, end);
                        compose(buf, laid, was, pre, next, now);
                        cursor_range = next;
                        laid.ime_preedit = (!pre.is_empty()).then(|| {
                            let active = active_range_chars.as_ref();
                            (next, active.map(|r| CCursor::new(r.start)..CCursor::new(r.end)))
                        });
                        text_changed = true;
                    }
                    egui::ImeEvent::Commit(commit) => {
                        let min = cursor_range.sorted_cursors()[0];
                        let next = CCursorRange::one(CCursor::new(min.index + count_chars(commit)));
                        compose(buf, laid, was, commit, next, now);
                        buf.undo.end_composing();
                        cursor_range = next;
                        laid.ime_preedit = None;
                        text_changed = true;
                    }
                    // (The characters around the selection, the selection
                    // kept, as egui's `delete_surrounding_chars`: review N5.)
                    egui::ImeEvent::DeleteSurrounding { before_chars, after_chars } => {
                        let [min, max] = cursor_range.sorted_cursors();
                        if *after_chars > 0 {
                            let to =
                                CCursor::new((usize::from(max.index) + after_chars).min(total));
                            let del = CCursorRange::two(max, to);
                            replace(buf, laid, del, "", was, was, now, false);
                        }
                        let back = (*before_chars).min(usize::from(min.index));
                        if back > 0 {
                            let mut moved = cursor_range;
                            moved.primary -= back;
                            moved.secondary -= back;
                            let del = CCursorRange::two(CCursor::new(min.index - back), min);
                            replace(buf, laid, del, "", was, moved, now, false);
                            cursor_range = moved;
                        }
                        laid.ime_preedit = None;
                        text_changed = true;
                    }
                    _ => {}
                },
                Event::Key { key, pressed: true, modifiers, .. } => {
                    // (A Mac's Ctrl+A, E, P, N, B and F move the cursor, as
                    // in egui: review N8.)
                    let mac_ctrl =
                        os == egui::os::OperatingSystem::Mac && modifiers.ctrl && !modifiers.shift;
                    let cur = usize::from(cursor_range.primary.index);
                    let [min, max] = cursor_range.sorted_cursors();
                    let (min_c, max_c) = (usize::from(min.index), usize::from(max.index));
                    // (Deletes the characters `a..b`, the cursor left at
                    // `a`; Undo gives the cursor back as it was, not the
                    // characters taken selected: review N2.)
                    let mut delete = |buf: &mut crate::script_view::ScriptBuffer,
                                      laid: &mut LaidOut,
                                      a: usize,
                                      b: usize,
                                      merge: bool|
                     -> CCursorRange {
                        let next =
                            CCursorRange::one(CCursor { prefer_next_row: true, ..CCursor::new(a) });
                        if a < b {
                            let del = CCursorRange::two(CCursor::new(a), CCursor::new(b));
                            replace(buf, laid, del, "", was, next, now, merge);
                            text_changed = true;
                        }
                        next
                    };
                    match key {
                        Key::A if modifiers.command => {
                            cursor_range =
                                CCursorRange::two(CCursor::new(0), CCursor::new(laid.total_chars));
                        }
                        Key::P | Key::N if mac_ctrl => {
                            let up = *key == Key::P;
                            vertical(ui, laid, &buf.text, palette, &mut cursor_range, up);
                            cursor_range.secondary = cursor_range.primary;
                        }
                        Key::A | Key::E | Key::B | Key::F if mac_ctrl => {
                            let c = match key {
                                Key::A => line_start_of(laid, cur),
                                Key::E => line_end_of(laid, cur),
                                Key::B => cur.saturating_sub(1),
                                _ => (cur + 1).min(total),
                            };
                            cursor_range = CCursorRange::one(CCursor::new(c));
                        }
                        Key::ArrowLeft | Key::ArrowRight
                            if modifiers.is_none() && !cursor_range.is_empty() =>
                        {
                            cursor_range =
                                CCursorRange::one(if *key == Key::ArrowLeft { min } else { max });
                        }
                        Key::ArrowLeft | Key::ArrowRight | Key::Home | Key::End => {
                            let left = matches!(key, Key::ArrowLeft | Key::Home);
                            let arrow = matches!(key, Key::ArrowLeft | Key::ArrowRight);
                            let c = if arrow && (modifiers.alt || modifiers.ctrl) {
                                // (Words over line ends, as egui: review N10.)
                                if left {
                                    word_left(laid, &buf.text, cur)
                                } else {
                                    word_right(laid, &buf.text, cur)
                                }
                            } else if (arrow && modifiers.mac_cmd) || (!arrow && !modifiers.ctrl) {
                                if left { line_start_of(laid, cur) } else { line_end_of(laid, cur) }
                            } else if !arrow {
                                if left { 0 } else { total }
                            } else if left {
                                cur.saturating_sub(1)
                            } else {
                                (cur + 1).min(total)
                            };
                            cursor_range.primary = CCursor::new(c);
                            cursor_range.h_pos = None;
                            if !modifiers.shift {
                                cursor_range.secondary = cursor_range.primary;
                            }
                        }
                        Key::ArrowUp | Key::ArrowDown => {
                            let up = *key == Key::ArrowUp;
                            if modifiers.command {
                                cursor_range.primary = CCursor::new(if up { 0 } else { total });
                                cursor_range.h_pos = None;
                            } else {
                                vertical(ui, laid, &buf.text, palette, &mut cursor_range, up);
                            }
                            if !modifiers.shift {
                                cursor_range.secondary = cursor_range.primary;
                            }
                        }
                        Key::Z | Key::Y
                            if modifiers.matches_logically(Modifiers::COMMAND)
                                || modifiers
                                    .matches_logically(Modifiers::COMMAND | Modifiers::SHIFT) =>
                        {
                            let redo = *key == Key::Y || modifiers.shift;
                            let oh = buf.text_hash();
                            if let Some((next_range, at, old_end, new_end)) =
                                buf.undo.step(&mut buf.text, redo)
                            {
                                buf.invalidate_hash();
                                let nh = buf.text_hash();
                                laid.apply_edit(&buf.text, at, old_end, new_end, oh, nh);
                                cursor_range = next_range;
                                text_changed = true;
                            }
                        }
                        // (Back to the line's start, a Mac's Cmd+Backspace
                        // and Ctrl+U, as egui; at its start, the line end
                        // before: review N9.)
                        Key::Backspace | Key::U
                            if (*key == Key::Backspace && modifiers.mac_cmd)
                                || (*key == Key::U && modifiers.ctrl) =>
                        {
                            let from = line_start_of(laid, min_c);
                            cursor_range = if from == max_c {
                                delete(buf, laid, max_c.saturating_sub(1), max_c, false)
                            } else {
                                delete(buf, laid, from, max_c, false)
                            };
                        }
                        // (To the line's end, a Mac's Cmd+Delete and
                        // Ctrl+K; at its end, the line end.)
                        Key::Delete | Key::K
                            if (*key == Key::Delete && modifiers.mac_cmd)
                                || (*key == Key::K && modifiers.ctrl) =>
                        {
                            let to = line_end_of(laid, max_c);
                            cursor_range = if to == min_c {
                                delete(buf, laid, min_c, (min_c + 1).min(total), false)
                            } else {
                                delete(buf, laid, min_c, to, false)
                            };
                        }
                        // (A Mac's Ctrl+H, the character before, as egui's;
                        // on Windows and Linux Ctrl+H is Replace Text.)
                        Key::H if mac_ctrl => {
                            cursor_range = if !cursor_range.is_empty() {
                                delete(buf, laid, min_c, max_c, false)
                            } else {
                                delete(buf, laid, cur.saturating_sub(1), cur, true)
                            };
                        }
                        Key::Backspace | Key::W if *key == Key::Backspace || modifiers.ctrl => {
                            let word = *key == Key::W || modifiers.alt || modifiers.ctrl;
                            cursor_range = if !cursor_range.is_empty() {
                                delete(buf, laid, min_c, max_c, false)
                            } else if word {
                                delete(buf, laid, word_left(laid, &buf.text, cur), cur, false)
                            } else {
                                delete(buf, laid, cur.saturating_sub(1), cur, true)
                            };
                        }
                        Key::Delete
                            if !modifiers.shift || os != egui::os::OperatingSystem::Windows =>
                        {
                            cursor_range = if !cursor_range.is_empty() {
                                delete(buf, laid, min_c, max_c, false)
                            } else if modifiers.alt || modifiers.ctrl {
                                delete(buf, laid, cur, word_right(laid, &buf.text, cur), false)
                            } else {
                                // (Key after key one step, as Backspace's:
                                // review N11.)
                                delete(buf, laid, cur, (cur + 1).min(total), true)
                            };
                        }
                        Key::Enter if modifiers.is_none() => {
                            let next = CCursorRange::one(CCursor::new(min.index + 1));
                            replace(buf, laid, was, "\n", was, next, now, false);
                            cursor_range = next;
                            text_changed = true;
                        }
                        _ => {}
                    }
                }
                Event::AccessKitActionRequest(_)
                    if ctx.accesskit_node_builder(id, |_| ()).is_some() =>
                {
                    let full = laid.text(ui, &buf.text, palette);
                    cursor_range.on_event(os, ev, &full, id);
                }
                _ => {}
            }
            if cursor_range != was {
                any_cursor_edit = true;
            }
        }
    }

    // The room: what the text needs, after the keys (above).
    let lines = laid.line_count();
    let gutter_width = laid.ensure_gutter_width(ui, lines, palette);
    let (gutter_rect, numbers) =
        ui.allocate_exact_size(egui::vec2(gutter_width, room), egui::Sense::hover());
    name_gutter(&numbers, lines);
    let est_w = (laid.max_cols as f32 + 1.0) * laid.char_advance;
    let content_w = laid.max_width.max(est_w);
    let field_w = (content_w + 20.0).max(ui.available_width());
    let field_h = (lines as f32 * laid.row_height()).max(room);
    let (_, field_rect) = ui.allocate_space(egui::vec2(field_w, field_h));
    let mut response = ui.interact(field_rect, id, egui::Sense::click_and_drag());
    // (Space or Enter in the editor are no click, as in egui's TextEdit:
    // Help followed the word at the cursor on Space, and Ctrl+Enter went to
    // its definition: review N1.)
    response.flags -= egui::response::Flags::FAKE_PRIMARY_CLICKED;
    let text_origin = egui::pos2(field_rect.left() + 8.0, field_rect.top());

    // Pointer interaction (preserving right-click selection for context menus).
    if response.hovered() {
        ui.set_cursor_icon(egui::CursorIcon::Text);
        ui.output_mut(|o| o.mutable_text_under_cursor = true);
    }
    if let Some(pos) = response.interact_pointer_pos() {
        let local = egui::pos2(pos.x - text_origin.x, pos.y - text_origin.y);
        let pointer_cursor = laid.ccursor_from_local_pos(ui, &buf.text, palette, local);
        let mut moved = true;
        if response.double_clicked() {
            use egui::text_selection::text_cursor_state::{
                ccursor_next_word, ccursor_previous_word,
            };
            ctx.memory_mut(|m| m.request_focus(id));
            let (li, col) = laid.line_col(usize::from(pointer_cursor.index));
            let l = laid.lines[li];
            let s = &buf.text[l.start..l.end];
            let rel = CCursor::new(col);
            let a = usize::from(ccursor_previous_word(s, rel).index) + l.char_start;
            let b = usize::from(ccursor_next_word(s, rel).index) + l.char_start;
            cursor_range = CCursorRange::two(CCursor::new(a), CCursor::new(b));
        } else if response.triple_clicked() {
            ctx.memory_mut(|m| m.request_focus(id));
            let (li, _) = laid.line_col(usize::from(pointer_cursor.index));
            let l = laid.lines[li];
            let end =
                (l.char_start + l.char_len + usize::from(li + 1 < lines)).min(laid.total_chars);
            cursor_range = CCursorRange::two(CCursor::new(l.char_start), CCursor::new(end));
        } else if response.hovered() && ui.input(|i| i.pointer.primary_pressed()) {
            ctx.memory_mut(|m| m.request_focus(id));
            // (Shift with no cursor before: from the click, as egui; it
            // selected from the text's start: review N7.)
            if ui.input(|i| i.modifiers.shift) && prev_cursor_range.is_some() {
                cursor_range.primary = pointer_cursor;
                cursor_range.h_pos = None;
            } else {
                cursor_range = CCursorRange::one(pointer_cursor);
            }
        } else if ui.is_being_dragged(id) && ui.input(|i| i.pointer.primary_down()) {
            cursor_range.primary = pointer_cursor;
            cursor_range.h_pos = None;
        } else if response.hovered()
            && ui.input(|i| i.pointer.button_pressed(egui::PointerButton::Secondary))
        {
            // (A right press gives the editor the keys, for its menu
            // (`widgets::text_menu`). In the selection it keeps it, for the
            // menu's Cut, Copy and Convert Indentation of its lines; outside
            // it, the caret goes where it was pressed, as a left press puts
            // it.)
            ctx.memory_mut(|m| m.request_focus(id));
            let [min, max] = cursor_range.sorted_cursors();
            if pointer_cursor.index < min.index || pointer_cursor.index > max.index {
                cursor_range = CCursorRange::one(pointer_cursor);
            } else {
                moved = false;
            }
        } else {
            moved = false;
        }
        any_cursor_edit |= moved;
    }

    // A composition is shown only while the cursor is on it: anything else
    // that moved the cursor or edited the text ended it (review M2).
    let preedit = laid.ime_preedit.clone().filter(|(r, _)| {
        r.as_sorted_char_range() == cursor_range.as_sorted_char_range() && !r.is_empty()
    });
    if preedit.is_none() && laid.ime_preedit.is_some() {
        laid.ime_preedit = None;
        buf.undo.end_composing();
    }

    if text_changed {
        response.mark_changed();
    }
    let selection_changed = prev_cursor_range != Some(cursor_range);
    let final_cursor_range =
        if has_focus || prev_cursor_range.is_some() || any_cursor_edit || text_changed {
            state.cursor.set_char_range(Some(cursor_range));
            Some(cursor_range)
        } else {
            None
        };

    let mut primary_cursor_rect = laid
        .cursor_local_rect(ui, &buf.text, palette, cursor_range.primary)
        .translate(text_origin.to_vec2());
    primary_cursor_rect = primary_cursor_rect.expand(1.5);
    if (has_focus && (response.changed() || selection_changed)) || reveal {
        ui.scroll_to_rect(primary_cursor_rect, None);
    }

    // Reserve ground shape before visible text & gutter.
    let ground = palette.background.unwrap_or_else(|| ui.visuals().text_edit_bg_color());
    let radius = ui.style().interact(&response).corner_radius;
    let ground_shape = ui.painter().add(egui::Shape::Noop);
    let mut under_shapes =
        vec![egui::Shape::rect_filled(field_rect.union(gutter_rect), radius, ground)];

    // Visible range from clip_rect.
    let clip = ui.clip_rect();
    let lines = laid.line_count();
    let row_h = laid.row_height();
    let first_vis = ((clip.top() - text_origin.y) / row_h).floor().max(0.0) as usize;
    let last_vis =
        (((clip.bottom() - text_origin.y) / row_h).ceil().max(0.0) as usize + 1).min(lines);
    let first_vis = first_vis.min(last_vis);

    let code_tokens = match palette.background {
        Some(g) if crate::script_view::ground_is_dark([g.r(), g.g(), g.b()]) => {
            mg_uikit::Tokens::DARK
        }
        Some(_) => mg_uikit::Tokens::LIGHT,
        None => mg_uikit::tokens(&ctx),
    };
    let (cursor_line, cursor_col) = laid.line_col(usize::from(cursor_range.primary.index));
    let ppp = ctx.pixels_per_point();
    let current = final_cursor_range.filter(|r| r.is_empty()).map(|_| cursor_line);
    let current_band = current.map(|li| {
        let y = text_origin.y + laid.row_y(li, ppp);
        egui::Rangef::new(y, y + row_h)
    });
    if let Some(y) = current_band {
        let sight = clip.x_range();
        under_shapes.push(egui::Shape::rect_filled(
            egui::Rect::from_x_y_ranges(sight, y),
            0.0,
            code_tokens.current_line,
        ));
        let text_x = egui::Rangef::new(gutter_rect.right().max(sight.min), sight.max);
        for edge in crate::script_view::hairlines(y, ppp) {
            under_shapes.push(egui::Shape::rect_filled(
                egui::Rect::from_x_y_ranges(text_x, edge),
                0.0,
                code_tokens.current_line_edge,
            ));
        }
    }

    // Visible-only selection rectangles (a composition is underlined
    // instead, as egui shows one: review N5).
    let selected_color = code_tokens.code_select;
    if let Some(r) = final_cursor_range.filter(|r| !r.is_empty() && preedit.is_none()) {
        let [min, max] = r.sorted_cursors();
        let (min_li, min_col) = laid.line_col(usize::from(min.index));
        let (max_li, max_col) = laid.line_col(usize::from(max.index));
        let vis_from = min_li.max(first_vis);
        let vis_to = max_li.min(last_vis.saturating_sub(1));
        for li in vis_from..=vis_to {
            let g = laid.ensure_line(ui, &buf.text, li, palette);
            let Some(row) = g.rows.first() else { continue };
            let left =
                if li == min_li { row.x_offset(egui::text::CharIndex::from(min_col)) } else { 0.0 };
            let right = if li == max_li {
                row.x_offset(egui::text::CharIndex::from(max_col))
            } else {
                row.size.x
            };
            if li == max_li && li != min_li && right <= left {
                continue;
            }
            let right = if right <= left && li != max_li { left + 3.0 } else { right };
            let y = text_origin.y + laid.row_y(li, ppp);
            let rect = egui::Rect::from_min_max(
                egui::pos2(text_origin.x + left, y),
                egui::pos2(text_origin.x + right, y + row.size.y),
            );
            under_shapes.push(egui::Shape::rect_filled(rect, 0.0, selected_color));
        }
    }
    ui.painter().set(ground_shape, egui::Shape::Vec(under_shapes));

    // Paint ONLY visible lines and visible gutter rows.
    let text_painter = ui.painter_at(field_rect.intersect(clip).expand(1.0));
    for li in first_vis..last_vis {
        let g = laid.ensure_line(ui, &buf.text, li, palette);
        let y = text_origin.y + laid.row_y(li, ppp);
        text_painter.galley(egui::pos2(text_origin.x, y), g, palette.colors[0]);
    }
    let marks = Marks { bookmarks: &buf.bookmarks, numbered: &buf.numbered };
    laid.paint_gutter_visible(
        ui,
        gutter_rect.left(),
        text_origin,
        first_vis,
        last_vis,
        current_band.and(current),
        palette.colors[0],
        palette,
        marks,
    );

    // A composition underlined, its active part more strongly; the caret
    // where the composition puts its own (none while a part is active), as
    // egui's `paint_ime_preedit_text_visuals` (review N5).
    let mut caret = Some(primary_cursor_rect);
    if let Some((r, active)) = &preedit {
        let [start, end] = r.sorted_cursors();
        let (s, e) = (usize::from(start.index), usize::from(end.index));
        let ime = ui.visuals().ime_composition;
        let active = active.as_ref().map(|a| {
            let len = e - s;
            (s + usize::from(a.start.index).min(len), s + usize::from(a.end.index).min(len))
        });
        let mut underline = |from: usize, to: usize, stroke| {
            let painter = &text_painter;
            laid.paint_underline(ui, painter, &buf.text, palette, text_origin, (from, to), stroke);
        };
        match active {
            Some((a, b)) if a < b => {
                underline(s, a, ime.inactive_underline_stroke);
                underline(a, b, ime.active_underline_stroke);
                underline(b, e, ime.inactive_underline_stroke);
                caret = None;
            }
            Some((a, _)) => {
                underline(s, e, ime.inactive_underline_stroke);
                let rect = laid.cursor_local_rect(ui, &buf.text, palette, CCursor::new(a));
                caret = Some(rect.translate(text_origin.to_vec2()).expand(1.5));
            }
            None => {
                underline(s, e, ime.inactive_underline_stroke);
                caret = None;
            }
        }
    }

    if has_focus {
        let now = ui.input(|i| i.time);
        if text_changed || any_cursor_edit {
            laid.last_interaction_time = now;
        }
        if ui.input(|i| i.focused)
            && let Some(caret) = caret
        {
            egui::text_selection::visuals::paint_text_cursor(
                ui,
                &text_painter,
                caret,
                now - laid.last_interaction_time,
            );
        }
        let to_global = ctx.layer_transform_to_global(ui.layer_id()).unwrap_or_default();
        ui.output_mut(|o| {
            o.ime = Some(egui::output::IMEOutput {
                purpose: egui::IMEPurpose::Normal,
                rect: to_global * field_rect,
                cursor_rect: to_global * primary_cursor_rect,
                should_interrupt_composition: false,
            });
        });
    }

    state.store(&ctx, id);

    if let Some(prev) = &prev_text_for_a11y {
        let full = laid.text(ui, &buf.text, palette);
        if response.changed() {
            response.widget_info(|| {
                egui::WidgetInfo::text_edit(ui.is_enabled(), prev.as_str(), buf.text.as_str(), "")
            });
        } else if selection_changed && let Some(r) = final_cursor_range {
            let info = egui::WidgetInfo::text_selection_changed(
                ui.is_enabled(),
                r.as_sorted_char_range(),
                buf.text.as_str(),
            );
            response.output_event(egui::output::OutputEvent::TextSelectionChanged(info));
        } else {
            response.widget_info(|| {
                egui::WidgetInfo::text_edit(ui.is_enabled(), prev.as_str(), buf.text.as_str(), "")
            });
        }
        egui::text_selection::accesskit_text::update_accesskit_for_text_widget(
            &ctx,
            id,
            final_cursor_range,
            egui::accesskit::Role::MultilineTextInput,
            egui::emath::TSTransform::from_translation(text_origin.to_vec2()),
            &full,
        );
        describe_line(&response, cursor_line + 1, lines);
    }

    CodeEditOutput {
        response,
        text_origin,
        gutter_rect,
        cursor_range: final_cursor_range,
        current_band,
        cursor_line_col: (cursor_line, cursor_col),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::script_view::highlight;
    use crate::settings::ScriptStyle;

    /// A frame of `ctx` running `f`.
    fn run(ctx: &egui::Context, f: impl FnMut(&mut Ui)) {
        ctx.run_ui(egui::RawInput::default(), f).textures_delta.clear();
    }

    /// Lays `text` out the way the editor did before (the whole text, one
    /// job: egui splits it at its lines itself), each line the editor's
    /// pitch high, and line by line, and compares the rows: where each is,
    /// its size, its line end, its characters and their colors. (Laid out
    /// whole with egui's `line_height`, a line's text is at the top of its
    /// row; line by line, it is centered: each character, and its mesh,
    /// lower by the same few points.)
    fn same_as_whole(ctx: &egui::Context, text: &str) {
        let palette = Palette::new(&ScriptStyle::default(), true, Color32::WHITE);
        let mut laid = LaidOut::default();
        let mut whole = None;
        let mut lines = None;
        let mut font_row = 0.0;
        run(ctx, |ui| {
            laid.begin(ui, &palette);
            lines = Some(laid.text(ui, text, &palette));
            let mut job = highlight(text, &palette);
            job.wrap.max_width = f32::INFINITY;
            for s in &mut job.sections {
                s.format.line_height = Some(palette.pitch());
            }
            // (An empty text: no section to give the height.)
            job.first_row_min_height = palette.pitch();
            whole = Some(ui.fonts_mut(|f| f.layout_job(job)));
            font_row = ui.fonts_mut(|f| f.row_height(&palette.font()));
        });
        let (whole, lines) = (whole.unwrap(), lines.unwrap());
        assert_eq!(lines.text(), text);
        assert_eq!(whole.rows.len(), lines.rows.len(), "{text:?}");
        // (How much lower: about half the room the pitch leaves the font's
        // row, the same for every character.)
        let lower = (lines.rows.iter().zip(&whole.rows))
            .find_map(|(b, a)| Some(b.glyphs.first()?.pos.y - a.glyphs.first()?.pos.y))
            .unwrap_or(0.0);
        let half = 0.5 * (palette.pitch() - font_row);
        assert!(
            text.trim().is_empty() || (lower >= 1.0 && (lower - half).abs() <= 1.0),
            "{lower} lower, half the room {half}"
        );
        for (i, (a, b)) in whole.rows.iter().zip(&lines.rows).enumerate() {
            let at = format!("row {i} of {text:?}");
            assert_eq!(a.pos, b.pos, "{at}");
            assert_eq!(a.size, b.size, "{at}");
            assert_eq!(a.size.y, palette.pitch(), "{at}");
            assert_eq!(a.ends_with_newline, b.ends_with_newline, "{at}");
            let chars = |r: &egui::epaint::text::PlacedRow, down: f32| {
                r.glyphs.iter().map(|g| (g.chr, g.pos + egui::vec2(0.0, down))).collect::<Vec<_>>()
            };
            assert_eq!(chars(a, lower), chars(b, 0.0), "{at}");
            let mesh = |r: &egui::epaint::text::PlacedRow, down: f32| {
                (r.visuals.mesh.vertices.iter())
                    .map(|v| (v.pos + egui::vec2(0.0, down), v.uv, v.color))
                    .collect::<Vec<_>>()
            };
            assert_eq!(mesh(a, lower), mesh(b, 0.0), "{at}: colors");
        }
        assert_eq!(whole.rect, lines.rect);
        assert_eq!(whole.size(), lines.size());
    }

    #[test]
    fn a_text_laid_out_line_by_line_is_laid_out_as_a_whole() {
        let ctx = egui::Context::default();
        for text in [
            "",
            "\n",
            "\n\n",
            "void main() {}",
            "void main()\n{\n}\n",
            "void main()\n{\n}\n\n",
            "/* a block\n   over lines */ int n = 0x1F;\n// a comment\nstring s = r\"raw\nover\n\
             lines\"; float f = 1.5f;\n#include \"x\"\n  é ü \t tab\n",
            "string s = \"unterminated\nint n;\n/* never closed\nstill\n",
            "x = \"a\\\nb\";\n",
            "s = r\"a \"\"quoted\"\"\n\"\" word\" + h\"x\";\nr\"ends at the line's end\"\n\"\n",
            "/*/ still a comment */ int a;\n/* a *\n/ still */ b;\n/* /* */ c */\n",
            "/* r\"not a string\n*/ s = \"/* not a comment\";\n// */ r\"\nraw */\n\"",
            "\r\n\t\n  \n",
        ] {
            same_as_whole(&ctx, text);
        }
    }

    /// Each line's tokens, lexed from what the line before left open, are
    /// the whole text's cut at the lines' ends: every script of the game
    /// (a sample of them every day, all with `MOONGLOW_FULL_CORPUS`).
    #[test]
    fn the_game_s_scripts_lexed_line_by_line_are_lexed_as_a_whole() {
        let Some(root) = mg_testkit::nwn_root() else { return };
        let game =
            mg_rules::GameData::open(&mg_resman::GameInstall::new(&root, None, "en")).unwrap();
        let names = mg_testkit::sweep(game.resman.list(mg_core::ResType::NSS), 8);
        let mut lines = 0;
        for name in &names {
            let data = game.resman.get(&mg_resman::ResKey::new(*name, mg_core::ResType::NSS));
            let text = crate::text::to_editor(&String::from_utf8_lossy(&data.unwrap())).0;
            // (Pieces: element and span, empty ones left out.)
            let pieces = |tokens: &[Token], start: usize, end: usize| -> Vec<_> {
                (tokens.iter())
                    .map(|t| (element(t.kind), t.span.start.max(start), t.span.end.min(end)))
                    .filter(|(_, a, b)| a < b)
                    .collect()
            };
            let whole = tokenize(text.as_bytes());
            let (mut start, mut within, mut tokens, mut w) = (0, Within::Code, Vec::new(), 0);
            loop {
                let end = text[start..].find('\n').map_or(text.len(), |i| start + i);
                within = lex_line(&text, start, end, within, &mut tokens);
                while whole[w..].first().is_some_and(|t| t.span.end <= start) {
                    w += 1;
                }
                let over = whole[w..].iter().take_while(|t| t.span.start < end.max(start + 1));
                let over: Vec<Token> = over.cloned().collect();
                assert_eq!(
                    pieces(&tokens, start, end),
                    pieces(&over, start, end),
                    "{name}: {:?}",
                    &text[start..end]
                );
                lines += 1;
                if end == text.len() {
                    break;
                }
                start = end + 1;
            }
        }
        assert!(names.len() >= mg_testkit::at_least(4000, 8), "{} scripts", names.len());
        eprintln!("{} scripts, {lines} lines", names.len());
    }

    /// The game's longest script, if it is installed.
    #[test]
    fn nwscript_laid_out_line_by_line_is_laid_out_as_a_whole() {
        let Some(root) = mg_testkit::nwn_root() else { return };
        let game =
            mg_rules::GameData::open(&mg_resman::GameInstall::new(&root, None, "en")).unwrap();
        let data = game.resman.get_named("nwscript", mg_core::ResType::NSS).unwrap();
        let text = crate::text::to_editor(&String::from_utf8_lossy(&data)).0;
        same_as_whole(&egui::Context::default(), &text);
    }

    /// Each edit lays out only the lines whose text or colors it changed,
    /// and what it gives is what laying the new text out whole gives.
    #[test]
    fn an_edit_lays_out_the_lines_it_changed() {
        let ctx = egui::Context::default();
        let palette = Palette::new(&ScriptStyle::default(), true, Color32::WHITE);
        let base: String = (0..200).map(|i| format!("int n{i} = {i}; // line {i}\n")).collect();
        let mut laid = LaidOut::default();
        let mut frame = |text: &str| {
            let mut out = None;
            run(&ctx, |ui| {
                laid.begin(ui, &palette);
                let before = laid.galleys.clone();
                let g = laid.text(ui, text, &palette);
                let kept = laid
                    .galleys
                    .iter()
                    .flatten()
                    .filter(|g| before.iter().flatten().any(|b| Arc::ptr_eq(b, g)));
                out = Some((g, laid.galleys.len() - kept.count()));
            });
            out.unwrap()
        };
        let (_, made) = frame(&base);
        assert_eq!(made, 201, "every line at first, and the empty one after the last");
        // A character typed on a line: that line.
        let typed = base.replacen("int n50 ", "int n50x ", 1);
        assert_eq!(frame(&typed).1, 1);
        // A line added: that line.
        let added = typed.replacen("int n60 ", "int n60_;\nint n60 ", 1);
        assert_eq!(frame(&added).1, 1);
        // A block comment opened: the lines it colors, up to where it ends.
        let opened = added.replacen("int n100 ", "/* int n100 ", 1).replacen(
            "int n104 = 104;",
            "int n104 = 104; */",
            1,
        );
        assert_eq!(frame(&opened).1, 5);
        // Lines changed far apart (Replace All): those lines.
        let replaced = opened.replace("int n150 ", "int m150 ").replace("int n3 ", "int m3 ");
        assert_eq!(frame(&replaced).1, 2);
        // Each laid out as the whole text is.
        let (g, _) = frame(&replaced);
        assert_eq!(g.text(), replaced);
        same_as_whole(&ctx, &replaced);
    }

    /// The lines an edit leaves ([`LaidOut::apply_edit`]) are those the new
    /// text laid out afresh has ([`LaidOut::sync_full`]): over thousands of
    /// edits made as the editor makes them (a range replaced, a text
    /// replaced whole as Replace All and Tab do, Undo and Redo), with
    /// characters of two bytes and more, CRLF line ends, edits at lines'
    /// ends and of their line ends, block comments and raw strings opened
    /// and closed over lines, and the empty text.
    #[test]
    fn an_edit_s_lines_are_the_new_text_s_laid_out_afresh() {
        use crate::script_view::ScriptBuffer;
        use egui::text::{CCursor, CCursorRange};
        let ctx = egui::Context::default();
        const PIECES: [&str; 18] = [
            "x",
            "é",
            "ü€",
            "𝄞",
            "\n",
            "\r\n",
            "\r",
            "\t",
            " ",
            "/*",
            "*/",
            "r\"",
            "\"",
            "\"\"",
            "//",
            "int n = 1;",
            "{\n    é;\n}\n",
            "\n\n",
        ];
        // (A small generator of its own: the same edits on every run.)
        let mut seed: u64 = 0x2545_f491_4f6c_dd1d;
        let mut next = move |n: usize| {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            (seed % n.max(1) as u64) as usize
        };
        // (The palette the window's, as the editor's.)
        let same = |ui: &Ui, laid: &LaidOut, text: &str, at: &str| {
            let palette = Palette::for_ui(&ScriptStyle::default(), ui);
            let mut fresh = LaidOut::default();
            let h = crate::script_tools::hash(text);
            fresh.sync_full(ui, text, h, &palette);
            assert!(laid.in_sync(text, h), "{at}: not in step with {text:?}");
            assert_eq!(laid.lines, fresh.lines, "{at}: {text:?}");
            assert_eq!(laid.total_chars, fresh.total_chars, "{at}: {text:?}");
            assert_eq!(laid.max_cols, fresh.max_cols, "{at}: {text:?}");
        };
        let mut edits = 0;
        run(&ctx, |ui| {
            let palette = Palette::for_ui(&ScriptStyle::default(), ui);
            for start in ["", "é", "void main()\r\n{\r\n    int é = 1;\r\n}\r\n", "/* a\nb */ c\n"]
            {
                let mut buf = ScriptBuffer::read(b"", (0, 0, 0));
                buf.set_text(start.to_owned());
                let mut laid = LaidOut::default();
                laid.sync_full(ui, &buf.text, buf.text_hash(), &palette);
                same(ui, &laid, &buf.text, "at first");
                for round in 0..600 {
                    let chars = laid.total_chars();
                    let at = format!("{start:?}, edit {round}");
                    let was = buf.text_hash();
                    let (a, b) = (next(chars + 1), next(chars + 1));
                    let (a, b) = (a.min(b), a.max(b).min(a + 6));
                    match next(10) {
                        // (Replace All and Tab: the text replaced whole.)
                        0 | 1 => {
                            let mut whole = buf.text.clone();
                            let piece = PIECES[next(PIECES.len())];
                            let ba = laid.byte_of_char(&whole, a);
                            whole.insert_str(ba, piece);
                            if next(2) == 0 {
                                whole = whole.replacen('x', "é", 1);
                            }
                            let cur = CCursorRange::default();
                            let (s, o, n) = buf.replace_diff_undo(whole, cur, cur, 0.0);
                            laid.apply_edit(&buf.text, s, o, n, was, buf.text_hash());
                        }
                        // (Undo and Redo.)
                        2 => {
                            if let Some((_, s, o, n)) = buf.undo.step(&mut buf.text, next(3) == 0) {
                                buf.invalidate_hash();
                                laid.apply_edit(&buf.text, s, o, n, was, buf.text_hash());
                            }
                        }
                        // (A range replaced: typed over, deleted, pasted.)
                        _ => {
                            let piece = if next(4) == 0 { "" } else { PIECES[next(PIECES.len())] };
                            let (ba, bb) =
                                (laid.byte_of_char(&buf.text, a), laid.byte_of_char(&buf.text, b));
                            let r = CCursorRange::two(CCursor::new(a), CCursor::new(b));
                            let (s, o, n) =
                                buf.replace_range_undo(ba..bb, piece, r, r, 0.0, next(2) == 0);
                            laid.apply_edit(&buf.text, s, o, n, was, buf.text_hash());
                        }
                    }
                    same(ui, &laid, &buf.text, &at);
                    edits += 1;
                }
                // Back to the start, Undo by Undo: the empty text too.
                while buf.undo.can_step(false) {
                    let was = buf.text_hash();
                    let (_, s, o, n) = buf.undo.step(&mut buf.text, false).unwrap();
                    buf.invalidate_hash();
                    laid.apply_edit(&buf.text, s, o, n, was, buf.text_hash());
                    same(ui, &laid, &buf.text, "undone");
                }
                let all = laid.total_chars();
                let was = buf.text_hash();
                let r = CCursorRange::two(CCursor::new(0), CCursor::new(all));
                let end = buf.text.len();
                let (s, o, n) = buf.replace_range_undo(0..end, "", r, r, 0.0, false);
                laid.apply_edit(&buf.text, s, o, n, was, buf.text_hash());
                same(ui, &laid, "", "all deleted");
            }
        });
        assert_eq!(edits, 2400);
    }

    /// Lines of another text than the one edited (a script rewritten where
    /// its editor was not drawn) are not stamped as the edited text's: they
    /// are left to be laid out afresh.
    #[test]
    fn an_edit_of_another_text_than_the_lines_one_leaves_them_stale() {
        let ctx = egui::Context::default();
        run(&ctx, |ui| {
            let palette = Palette::for_ui(&ScriptStyle::default(), ui);
            let before = "int a;\nint b;\nint c;\n";
            let mut laid = LaidOut::default();
            laid.sync_full(ui, before, crate::script_tools::hash(before), &palette);
            // (Rewritten meanwhile, shorter, a character of two bytes first.)
            let rewritten = "é;\nint c;\n";
            let edited = "é;\nint total;\n";
            let (was, now) =
                (crate::script_tools::hash(rewritten), crate::script_tools::hash(edited));
            assert!(!laid.in_sync(rewritten, was));
            laid.apply_edit(edited, 8, 9, 13, was, now);
            assert!(!laid.in_sync(edited, now), "stamped as the edited text's");
            laid.sync_full(ui, edited, now, &palette);
            assert!(laid.in_sync(edited, now));
            assert_eq!(laid.line_count(), 3);
        });
    }

    /// A word move lands where egui's lands in the whole text, from every
    /// character of texts with blank, indented and CR LF lines (they read
    /// only the lines back to one with something but spaces: review K's N10).
    /// The places between a CR and its LF are left out: the editor's lines
    /// end at the pair, which egui takes as one grapheme.
    #[test]
    fn word_moves_land_where_egui_s_do_in_the_whole_text() {
        use egui::text::CCursor;
        use egui::text_selection::text_cursor_state::{ccursor_next_word, ccursor_previous_word};
        let ctx = egui::Context::default();
        let texts = [
            "",
            "\n",
            "int abc;\n\n  int de;\n",
            "\n\n\n  x\n\n",
            "a.b(c);\r\n\r\n\t\tif (é) { n_1 = 2; }\r\n   \r\n",
            "  \n  \n// note é\n\r\n\"s t\"; x\r",
            "word",
            "   ",
        ];
        let mut moves = 0;
        run(&ctx, |ui| {
            let palette = Palette::for_ui(&ScriptStyle::default(), ui);
            for text in texts {
                let mut laid = LaidOut::default();
                laid.sync_full(ui, text, crate::script_tools::hash(text), &palette);
                for c in 0..=laid.total_chars {
                    // (Not between a CR and its LF: one line end to the
                    // editor, one grapheme to egui.)
                    if c > 0
                        && text.chars().nth(c - 1) == Some('\r')
                        && text.chars().nth(c) == Some('\n')
                    {
                        continue;
                    }
                    let left = ccursor_previous_word(text, CCursor::new(c)).index;
                    let right = ccursor_next_word(text, CCursor::new(c)).index;
                    assert_eq!(
                        word_left(&laid, text, c),
                        usize::from(left),
                        "{text:?} left of {c}"
                    );
                    assert_eq!(
                        word_right(&laid, text, c),
                        usize::from(right).min(laid.total_chars),
                        "{text:?} right of {c}"
                    );
                    moves += 2;
                }
            }
        });
        assert!(moves > 150, "{moves}");
    }

    /// The line numbers: one a line, each as the number laid out alone
    /// in the numbers' color and the editor's pitch; those of lines added
    /// laid out as they come, the others kept; the gutter as wide as the
    /// board's (12 points, the widest number, 10 points).
    #[test]
    fn line_numbers_are_laid_out_as_lines_come() {
        let ctx = egui::Context::default();
        let palette = Palette::new(&ScriptStyle::default(), true, Color32::WHITE);
        let mut laid = LaidOut::default();
        let mut frame = |lines: usize| {
            let mut alone = None;
            run(&ctx, |ui| {
                laid.begin(ui, &palette);
                laid.numbers(ui, lines, &palette);
                let color = ui.visuals().weak_text_color();
                let mut job = LayoutJob::default();
                job.append(&lines.to_string(), 0.0, palette.format(color));
                job.first_row_min_height = palette.pitch();
                alone = Some(ui.fonts_mut(|f| f.layout_job(job)));
            });
            (laid.numbers.clone(), alone.unwrap(), laid.gutter_width())
        };
        let (nine, _, _) = frame(9);
        assert_eq!(nine.len(), 9);
        let (twelve, last, width) = frame(12);
        assert_eq!(twelve.len(), 12);
        assert!(
            nine.iter()
                .zip(&twelve)
                .all(|(a, b)| Arc::ptr_eq(a.as_ref().unwrap(), b.as_ref().unwrap())),
            "kept"
        );
        let ours = twelve.last().unwrap().as_ref().unwrap();
        assert_eq!(ours.text(), "12");
        assert_eq!(ours.size(), last.size());
        assert_eq!(ours.size().y, palette.pitch());
        assert_eq!(ours.rows[0].visuals.mesh.vertices, last.rows[0].visuals.mesh.vertices);
        assert_eq!(width, GUTTER_LEFT + last.size().x + GUTTER_RIGHT);
        assert_eq!((GUTTER_LEFT, GUTTER_RIGHT), (12.0, 10.0));
        let (five, _, _) = frame(5);
        assert_eq!(five.len(), 5);
        assert!(
            five.iter()
                .zip(&twelve)
                .all(|(a, b)| Arc::ptr_eq(a.as_ref().unwrap(), b.as_ref().unwrap())),
            "kept"
        );
    }

    /// The editor's lines are the board's 18 points apart at its 13-point
    /// text (`.code`), in that ratio at another size (Options › Script
    /// Editor's), at each interface size as egui rounds a row to whole
    /// pixels; each line's text centered in its row (half the room above
    /// it, half below, to a pixel), as the board's CSS puts it.
    #[test]
    fn the_monospace_font_is_as_high_as_the_pitch_takes_it() {
        let ctx = egui::Context::default();
        run(&ctx, |ui| {
            for size in 6..=48 {
                let size = f32::from(size as u8);
                let font = egui::FontId::monospace(size);
                let high = ui.fonts_mut(|f| f.row_height(&font));
                let near = (high - size * crate::script_view::MONO_HEIGHT).abs();
                assert!(near <= 1.0 / 32.0 + 0.01, "size {size}: {high} high");
            }
        });
    }

    /// The board's pitch (18 at 13, in that ratio), but never less than
    /// the glyphs and a point of room above and under them: Options lets
    /// the text be 6 to 48, and at 6 to 9 the board's ratio leaves the
    /// glyphs touching the row's top.
    #[test]
    fn lines_are_the_board_s_pitch_apart_their_text_centered() {
        let sizes = [(6, 9.0), (7, 11.0), (8, 12.0), (9, 13.0), (10, 14.0), (11, 15.0)];
        let sizes = sizes.into_iter().chain([(12, 17.0), (13, 18.0), (17, 24.0), (24, 33.0)]);
        for (size, pitch) in sizes {
            let style = ScriptStyle { font_size: size, ..ScriptStyle::default() };
            let palette = Palette::new(&style, true, mg_uikit::Tokens::DARK.syn_text);
            assert_eq!(palette.pitch(), pitch, "size {size}");
            for scale in crate::UI_SCALES {
                let ppp = f32::from(scale) / 100.0;
                let ctx = egui::Context::default();
                ctx.set_pixels_per_point(ppp);
                let mut laid = LaidOut::default();
                let mut galley = None;
                run(&ctx, |ui| {
                    laid.begin(ui, &palette);
                    galley = Some(laid.text(ui, "void main()\n{\n\n    int n;\n}", &palette));
                });
                let galley = galley.unwrap();
                let row = palette.row_height(ppp);
                assert!((row - pitch).abs() <= 0.5 / ppp + 1e-4, "{row} at {ppp}");
                // (egui puts what it lays out on 1/32 of a point.)
                let near = |a: f32, b: f32| (a - b).abs() <= 1.0 / 32.0;
                let at = format!("size {size} at {ppp}");
                assert_eq!(galley.rows.len(), 5, "{at}");
                for (i, r) in galley.rows.iter().enumerate() {
                    assert!(near(r.pos.y, i as f32 * row), "{at}: row {i} at {}", r.pos.y);
                    assert!(near(r.size.y, row), "{at}: row {i} {} high", r.size.y);
                    for g in &r.glyphs {
                        let above = g.pos.y - g.font_ascent;
                        let below = r.size.y - above - g.font_height;
                        assert!(above > 0.5, "{at}: {above} above {:?}", g.chr);
                        assert!(
                            (above - below).abs() <= 1.0 / ppp + 0.01,
                            "{at}: {above} above, {below} below"
                        );
                    }
                }
            }
        }
    }

    /// The live error's wave (`script_view::wave_y`) under the baseline and
    /// within its row at every size Options allows and every interface
    /// size: the board's 3 points under the baseline at 13, never into the
    /// next line (at 9 and under, 3 points would have run into it). It may
    /// touch the descenders: it lies at their foot, as the board draws it
    /// and as editors do (the coordinator's ruling, W3's second fix round:
    /// a wave clear of them would need a 24-point pitch at 13, or the text
    /// off center).
    #[test]
    fn the_error_wave_stays_in_its_row() {
        let half = mg_uikit::WAVE_HEIGHT / 2.0 + 0.5;
        for size in 6..=48u8 {
            let style = ScriptStyle { font_size: size, ..ScriptStyle::default() };
            let palette = Palette::new(&style, true, mg_uikit::Tokens::DARK.syn_text);
            for scale in crate::UI_SCALES {
                let ppp = f32::from(scale) / 100.0;
                let ctx = egui::Context::default();
                ctx.set_pixels_per_point(ppp);
                let mut laid = LaidOut::default();
                let mut galley = None;
                run(&ctx, |ui| {
                    laid.begin(ui, &palette);
                    galley = Some(laid.text(ui, "int n = 1;\nint m;", &palette));
                });
                let galley = galley.unwrap();
                let r = &galley.rows[0];
                let baseline = r.pos.y + r.glyphs[0].pos.y;
                let y = crate::script_view::wave_y(baseline, r.pos.y + r.size.y, f32::from(size));
                let at = format!("size {size} at {ppp}: baseline {baseline}, wave {y}");
                // (egui puts rows on 1/32 of a point.)
                let next = galley.rows[1].pos.y;
                assert!(y + half <= next + 1.0 / 32.0, "{at}: into the next line at {next}");
                assert!(y >= baseline, "{at}: over the text");
            }
        }
    }

    /// The current line's hairlines on whole pixels at every interface size
    /// (at 1.25 a 1-point line would blur over two), inside the row, at
    /// least a pixel thick, the point's thickness as near as pixels go.
    #[test]
    fn the_current_line_s_hairlines_are_on_whole_pixels() {
        let whole = |v: f32, ppp: f32| ((v * ppp) - (v * ppp).round()).abs() < 1e-3;
        for scale in crate::UI_SCALES {
            let ppp = f32::from(scale) / 100.0;
            // (A row as egui places it: on whole pixels.)
            let top = (37.0 * ppp).round() / ppp;
            let palette =
                Palette::new(&ScriptStyle::default(), true, mg_uikit::Tokens::DARK.syn_text);
            let row = egui::Rangef::new(top, top + palette.row_height(ppp));
            for line in crate::script_view::hairlines(row, ppp) {
                let at = format!("{line:?} at {ppp} in {row:?}");
                assert!(whole(line.min, ppp) && whole(line.max, ppp), "{at}");
                assert!(line.min >= row.min - 1e-3 && line.max <= row.max + 1e-3, "{at}");
                let pixels = (line.span() * ppp).round();
                assert!(pixels >= 1.0 && (line.span() - 1.0).abs() <= 0.5 / ppp + 1e-3, "{at}");
            }
        }
    }

    /// egui's fonts made again (its font atlas full): the lines kept are
    /// laid out again, where they'd be drawn from the old atlas's places.
    #[test]
    fn fonts_made_again_lay_the_lines_out_again() {
        let ctx = egui::Context::default();
        let palette = Palette::new(&ScriptStyle::default(), true, Color32::WHITE);
        let mut laid = LaidOut::default();
        let mut frame = |ctx: &egui::Context| {
            let mut out = None;
            run(ctx, |ui| {
                watch_fonts(ui.ctx());
                laid.begin(ui, &palette);
                out = Some(laid.text(ui, "void main()\n{\n}\n", &palette));
            });
            out.unwrap()
        };
        let first = frame(&ctx);
        assert!(Arc::ptr_eq(&first, &frame(&ctx)), "kept");
        // (Other text options make the fonts again, as a full atlas does.)
        ctx.all_styles_mut(|s| s.visuals.text_options.max_texture_side /= 2);
        run(&ctx, |_| {});
        assert!(!Arc::ptr_eq(&first, &frame(&ctx)), "laid out again");
    }
}
