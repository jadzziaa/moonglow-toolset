//! The script editor: syntax-highlighted text, save, and compile.

use egui::text::{LayoutJob, TextFormat};
use egui::{Color32, FontId, Ui};
use mg_core::ResType;
use mg_edit::{Command, Edit};
use mg_resman::ResKey;
use mg_script::Compiler;
use mg_script::lex::{TokenKind, tokenize};
use mg_uikit::Controls as _;

use crate::script_tools::{self as tools, Completion, InfoTab, Message, SymbolKind};
use crate::settings::ScriptStyle;
use crate::text::{decode_script as decode, encode_script as encode, script_codepage};
use crate::{Action, Moonglow, Tab};

/// A script's text in its editor.
///
/// The editor's text has plain newlines (`\n`) for line ends, whatever the
/// file has: a CRLF script (BioWare's, Aurora's, the new scripts' own)
/// kept its `\r` at each line's end in the editor, where the cursor
/// stopped on it, End went past it, and a copy took it along (Jasperre's
/// report: what looked like a trailing space on every line). The file's
/// line ends are put back as it is written ([`ScriptBuffer::bytes`]).
/// Incremental, byte-range undo/redo history for [`ScriptBuffer`], avoiding
/// full-file clones on every frame and keystroke.
#[derive(Debug, Clone, Default)]
pub(crate) struct ScriptUndo {
    undo: Vec<UndoStep>,
    redo: Vec<UndoStep>,
    total_bytes: usize,
    break_next: bool,
    /// The last step is a composition's (IME) still going on: its next
    /// text replaces what that step put in, as one step
    /// ([`ScriptUndo::record_composing`]).
    composing: bool,
}

#[derive(Debug, Clone)]
struct UndoStep {
    at_byte: usize,
    removed: String,
    inserted: String,
    before: egui::text::CCursorRange,
    after: egui::text::CCursorRange,
    time: f64,
    mergeable: bool,
}

impl ScriptUndo {
    const MAX_STEPS: usize = 256;
    const MAX_BYTES: usize = 32 * 1024 * 1024;
    const MIN_STEPS: usize = 16;

    pub(crate) fn clear(&mut self) {
        self.undo.clear();
        self.redo.clear();
        self.total_bytes = 0;
        self.break_next = false;
        self.composing = false;
    }

    pub(crate) fn mark_break(&mut self) {
        self.break_next = true;
        self.composing = false;
    }

    /// Records a composition's text (IME) put in at `at_byte` over
    /// `removed`: while the composition goes on, its last step takes each
    /// text in place of the one before, so that the whole composition is
    /// one step, from the text before it to what it commits (each text
    /// was a step of its own: review M1). An empty text over nothing (a
    /// composition given up) leaves no step.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn record_composing(
        &mut self,
        at_byte: usize,
        removed: String,
        inserted: String,
        before: egui::text::CCursorRange,
        after: egui::text::CCursorRange,
        time: f64,
    ) {
        if self.composing
            && let Some(s) = self.undo.last_mut()
            && s.at_byte == at_byte
            && s.inserted == removed
        {
            self.total_bytes = self.total_bytes - s.inserted.len() + inserted.len();
            s.inserted = inserted;
            s.after = after;
            s.time = time;
            if s.removed.is_empty() && s.inserted.is_empty() {
                self.undo.pop();
                self.composing = false;
            }
            return;
        }
        if removed.is_empty() && inserted.is_empty() {
            return;
        }
        self.record_edit(at_byte, removed, inserted, before, after, time, false);
        self.composing = true;
    }

    /// The composition committed or ended: what comes next is a step of
    /// its own.
    pub(crate) fn end_composing(&mut self) {
        self.composing = false;
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn record_edit(
        &mut self,
        at_byte: usize,
        removed: String,
        inserted: String,
        before: egui::text::CCursorRange,
        after: egui::text::CCursorRange,
        time: f64,
        mergeable: bool,
    ) {
        self.composing = false;
        for s in self.redo.drain(..) {
            self.total_bytes = self.total_bytes.saturating_sub(s.removed.len() + s.inserted.len());
        }
        let coalesced = !self.break_next
            && mergeable
            && self.undo.last_mut().is_some_and(|prev| {
                if !prev.mergeable || (time - prev.time).abs() >= 1.0 {
                    return false;
                }
                if removed.is_empty()
                    && !inserted.contains('\n')
                    && prev.removed.is_empty()
                    && !prev.inserted.contains('\n')
                    && at_byte == prev.at_byte + prev.inserted.len()
                {
                    self.total_bytes += inserted.len();
                    prev.inserted.push_str(&inserted);
                    prev.after = after;
                    prev.time = time;
                    true
                } else if inserted.is_empty()
                    && !removed.contains('\n')
                    && prev.inserted.is_empty()
                    && !prev.removed.contains('\n')
                    && at_byte + removed.len() == prev.at_byte
                {
                    self.total_bytes += removed.len();
                    let mut merged = removed.clone();
                    merged.push_str(&prev.removed);
                    prev.removed = merged;
                    prev.at_byte = at_byte;
                    prev.after = after;
                    prev.time = time;
                    true
                } else if inserted.is_empty()
                    && !removed.contains('\n')
                    && prev.inserted.is_empty()
                    && !prev.removed.contains('\n')
                    && at_byte == prev.at_byte
                {
                    // (Delete, key after key: what each took comes after
                    // what the last did.)
                    self.total_bytes += removed.len();
                    prev.removed.push_str(&removed);
                    prev.after = after;
                    prev.time = time;
                    true
                } else {
                    false
                }
            });
        if coalesced {
            return;
        }
        self.break_next = false;
        self.total_bytes += removed.len() + inserted.len();
        self.undo.push(UndoStep { at_byte, removed, inserted, before, after, time, mergeable });
        while self.undo.len() > Self::MAX_STEPS
            || (self.total_bytes > Self::MAX_BYTES && self.undo.len() > Self::MIN_STEPS)
        {
            let dropped = self.undo.remove(0);
            self.total_bytes =
                self.total_bytes.saturating_sub(dropped.removed.len() + dropped.inserted.len());
        }
    }

    pub(crate) fn step(
        &mut self,
        text: &mut String,
        redo: bool,
    ) -> Option<(egui::text::CCursorRange, usize, usize, usize)> {
        self.break_next = true;
        self.composing = false;
        // (A step whose place is not in the text, or not between its
        // characters, is of another text than this one: the history is let
        // go, not applied to bytes it was not made for, which panicked.)
        let fits = |s: &UndoStep, put: &str| {
            let end = s.at_byte + put.len();
            text.get(s.at_byte..end) == Some(put)
        };
        let next = if redo { self.redo.last() } else { self.undo.last() };
        if next.is_some_and(|s| !fits(s, if redo { &s.removed } else { &s.inserted })) {
            self.clear();
            return None;
        }
        if redo {
            let s = self.redo.pop()?;
            let old_end = s.at_byte + s.removed.len();
            let new_end = s.at_byte + s.inserted.len();
            text.replace_range(s.at_byte..old_end, &s.inserted);
            let cursor = s.after;
            let at = s.at_byte;
            self.undo.push(s);
            Some((cursor, at, old_end, new_end))
        } else {
            let s = self.undo.pop()?;
            let old_end = s.at_byte + s.inserted.len();
            let new_end = s.at_byte + s.removed.len();
            text.replace_range(s.at_byte..old_end, &s.removed);
            let cursor = s.before;
            let at = s.at_byte;
            self.redo.push(s);
            Some((cursor, at, old_end, new_end))
        }
    }

    /// Whether there is a step back (or, `redo`, again) to take: each one
    /// changes the text (the history keeps edits alone, not the cursor's
    /// moves), for the editor's menu's Undo and Redo.
    pub(crate) fn can_step(&self, redo: bool) -> bool {
        if redo { !self.redo.is_empty() } else { !self.undo.is_empty() }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ScriptBuffer {
    pub(crate) text: String,
    /// The text as last loaded or saved.
    pub(crate) saved: String,
    /// The file's text as last loaded or saved, with its own line ends:
    /// what an untouched text is written as, and whose line ends an
    /// edited one takes ([`crate::text::with_line_ends`]).
    raw: String,
    /// The module's bytes the text was last compared with (address and
    /// length), to decode them again only when they change.
    source: crate::tree::Stamp,
    /// Bookmarked lines (0-based).
    pub(crate) bookmarks: std::collections::BTreeSet<usize>,
    /// Numbered bookmarks 1 to 9 (index 0 unused): Ctrl+Shift+N sets one,
    /// Ctrl+N goes there.
    pub(crate) numbered: [Option<usize>; 10],
    /// The file the external script editor has open, and when it was last
    /// written: the text is read back whenever it changes.
    pub(crate) external: Option<(std::path::PathBuf, Option<std::time::SystemTime>)>,
    /// The codepage the text was read in, and is written in: the
    /// script codepage when it was read ([`script_codepage`]). Another
    /// language chosen in Options reads a script without changes again;
    /// one with unsaved changes is written as it was read, then read
    /// again.
    pub(crate) codepage: mg_core::Codepage,
    /// Where the editor was scrolled to and its cursor (characters; none
    /// until it has had one), as drawn last: what a module's kept tabs
    /// keep of it.
    pub(crate) seen: Option<(egui::Vec2, Option<(usize, usize)>)>,
    /// How the script is indented, guessed when its text was read
    /// ([`tools::Indent::guess`]) or set by converting it: what Tab puts
    /// in (`None`: as Options › Script Editor says).
    pub(crate) indent: Option<tools::Guess>,
    /// The last Convert Indentation: the text's hash before it and how it
    /// was indented then, and after it and how then. The editor's Undo
    /// taking the conversion back (or Redo making it again) gives Tab that
    /// text's indentation again ([`ScriptBuffer::follow_conversion`]).
    pub(crate) conversion: Option<[(u64, Option<tools::Guess>); 2]>,
    /// The cursor's line and column (each from 1, the column in
    /// characters), the Status Bar's count ("Line 17, column 12"), as the
    /// editor last found them in its laid-out text, and the text's hash and
    /// the cursor's place they were found for.
    pub(crate) place: Option<(u64, usize, (usize, usize))>,
    /// The live error's line as its wavy underline spans it (characters:
    /// its first that is not a space to after its last; none for a line
    /// of spaces), and the text's hash and the line it was found for.
    error_span: Option<(u64, u64, Option<Underline>)>,
    /// Delta undo/redo stack for the editor.
    pub(crate) undo: ScriptUndo,
    /// Cached `(text.len(), text.as_ptr(), hash)` to detect both explicit
    /// invalidation and direct mutations to `text`.
    cached_hash: std::cell::Cell<Option<(usize, usize, u64)>>,
    /// Cached `(text_hash, is_dirty)` pair to avoid comparing 650 KB buffers on idle frames.
    cached_dirty: std::cell::Cell<Option<(u64, bool)>>,
}

/// The line the live error's underline is under and its span there.
type Underline = (usize, Span);

/// A span of a text, its first character and the one after its last.
type Span = (usize, usize);

/// A script's text: what its editor holds (unsaved typing too), else the
/// module's, else the game's, read in the scripts' codepage, with plain
/// newlines for line ends (as the editor has them). Every view of
/// a script's text reads it here (the conversation editor's Script Preview
/// read the module's in the game's codepage, and past an open editor).
pub(crate) fn script_source(
    scripts: &std::collections::HashMap<ResKey, ScriptBuffer>,
    ws: Option<&mg_edit::Workspace>,
    game: Option<&mg_rules::GameData>,
    key: &ResKey,
) -> Option<String> {
    if let Some(b) = scripts.get(key) {
        return Some(b.text.clone());
    }
    // (With plain newlines, as an editor has it: positions in it are the
    // editor's, whose script it is or will be.)
    ws.and_then(|w| w.module.get(key).map(decode))
        .or_else(|| game?.resman.get(key).ok().map(|d| decode(&d)))
        .map(|raw| crate::text::to_editor(&raw).0)
}

impl Moonglow {
    /// [`script_source`] of the app's editors, module and game.
    pub(crate) fn script_source(&self, key: &ResKey) -> Option<String> {
        script_source(&self.scripts, self.ws.as_ref(), self.game.as_deref(), key)
    }

    /// Whether a script is open: its editor has a tab. Its editor's state
    /// (`Moonglow::scripts`) is no sign of it: kept for the session after
    /// a tab closed with nothing unsaved, it is in no tab to ask about
    /// changes made to it (Replace All in a Find window left on it made it
    /// unsaved, and the module's Save wrote it). Not inside the DockArea
    /// pass, where `app.dock` is empty.
    pub(crate) fn script_open(&self, key: &ResKey) -> bool {
        self.dock.find_tab(&Tab::Script(*key)).is_some()
    }

    /// Whether a script's laid-out text is kept (for tests).
    #[doc(hidden)]
    pub fn script_laid_out_for_test(&self, key: ResKey) -> bool {
        self.laid_out.contains_key(&key)
    }

    /// The line (from 0) the live error's wavy underline was last drawn
    /// under in `key`'s editor (`underlined_line`): the Status Bar's
    /// message goes there.
    pub(crate) fn error_underline_line(&self, key: ResKey) -> Option<usize> {
        self.scripts.get(&key)?.error_span.as_ref()?.2.map(|(l, _)| l)
    }

    /// Whether a check as you type is being compiled on a worker, and
    /// whether a script's symbols are being read on one; how many checks
    /// and symbol jobs have gone to workers (for tests).
    #[doc(hidden)]
    pub fn script_workers_for_test(&self) -> ((bool, bool), (usize, usize)) {
        let busy = (self.script_nav.checking(), self.script_tools.reading_symbols());
        (busy, (self.script_nav.checks_started(), self.script_tools.symbol_jobs_started))
    }

    /// Whether the symbols a script's editor has for it name `name` (for
    /// tests).
    #[doc(hidden)]
    pub fn script_has_symbol_for_test(&self, key: ResKey, name: &str) -> bool {
        self.script_tools.symbols.get(&key).is_some_and(|(_, s)| s.find_by_name(name).is_some())
    }

    /// Whether a script's laid-out lines are those of its text (none laid
    /// out: true), as they must be wherever places are read from them (for
    /// tests).
    #[doc(hidden)]
    pub fn script_layout_in_step_for_test(&self, key: ResKey) -> bool {
        let (Some(laid), Some(b)) = (self.laid_out.get(&key), self.scripts.get(&key)) else {
            return true;
        };
        !laid.stamped_in_sync(b.text_hash()) || laid.lines_are(&b.text)
    }

    /// A script's bookmarked lines (from 0), for tests.
    #[doc(hidden)]
    pub fn script_bookmarks_for_test(&self, key: ResKey) -> Vec<usize> {
        self.scripts.get(&key).map(|b| b.bookmarks.iter().copied().collect()).unwrap_or_default()
    }

    /// Sets a bookmark on the line of the character `at` of an open
    /// script, as its Bookmark would (for tests).
    #[doc(hidden)]
    pub fn scripts_mut_bookmark_for_test(&mut self, key: ResKey, at: usize) {
        if let Some(b) = self.scripts.get_mut(&key) {
            b.bookmarks.insert(tools::line_of(&b.text, at));
            self.script_tools.made += 1;
        }
    }

    /// How far a script's editor is scrolled down, as it was last drawn
    /// (for tests).
    #[doc(hidden)]
    pub fn script_scroll_for_test(&self, key: ResKey) -> Option<f32> {
        self.scripts.get(&key)?.seen.map(|(offset, _)| offset.y)
    }

    /// Where a script's editor drew its gutter last, left to right, and
    /// its current line's band, top to bottom (none: not drawn), for tests.
    #[doc(hidden)]
    pub fn script_gutter_for_test(
        ctx: &egui::Context,
        key: ResKey,
    ) -> Option<(egui::Rangef, Option<egui::Rangef>)> {
        ctx.data(|d| d.get_temp(gutter_id(key)))
    }
}

/// What [`Moonglow::script_gutter_for_test`] reads.
fn gutter_id(key: ResKey) -> egui::Id {
    egui::Id::new(("script-gutter", key))
}

/// What of the 0-based `line` of `text` an error's wavy underline spans
/// (as editors mark errors): its bytes from its first character that is
/// not a space to after its last; none where it has none, or past the
/// text's end.
pub(crate) fn error_span(text: &str, line: usize) -> Option<(usize, usize)> {
    let start = match line {
        0 => 0,
        _ => text.match_indices('\n').nth(line - 1).map(|(i, _)| i + 1)?,
    };
    let end = text[start..].find('\n').map_or(text.len(), |i| start + i);
    let own = &text[start..end];
    let from = start + (own.len() - own.trim_start().len());
    let to = start + own.trim_end().len();
    (from < to).then_some((from, to))
}

/// The line the live error's wavy underline goes under (from 0), of
/// `text`: its own line (`line`) where it has text; a blank line's, or a
/// line past the end's (an error at the end of the file), the nearest line
/// above with text; an error in an include (`line` none), the `#include`
/// line naming `include`, else the first line with text. `None` for a text
/// with no text at all.
pub(crate) fn underlined_line(
    text: &str,
    line: Option<usize>,
    include: Option<&str>,
) -> Option<usize> {
    // (The lines gone through only as far as needed: the text is not cut
    // into all its lines on every key of a long script.)
    let has = |t: &str| !t.trim().is_empty();
    let first = || text.split('\n').position(has);
    match line {
        Some(l) => {
            let upto: Vec<&str> = text.split('\n').take(l + 1).collect();
            (0..upto.len()).rev().find(|i| has(upto[*i])).or_else(first)
        }
        None => {
            let name = include.map(str::to_ascii_lowercase).unwrap_or_default();
            let names = |t: &str| {
                let t = t.trim_start();
                t.starts_with("#include")
                    && !name.is_empty()
                    && t.to_ascii_lowercase().contains(&format!("\"{name}\""))
            };
            text.split('\n').position(names).or_else(first)
        }
    }
}

/// Paints the live error's wavy underline under `line` of the editor's laid-out
/// text (laid out at `at`), the span [`error_span`] gives in characters, in the
/// theme's `error` token; returns the wave's rectangle for its hover tip.
fn paint_error_underline(
    ui: &Ui,
    laid: &mut crate::script_layout::LaidOut,
    text: &str,
    at: egui::Pos2,
    (from, to): (usize, usize),
    palette: &Palette,
) -> egui::Rect {
    let (li, _) = laid.line_col(from);
    let ppp = ui.ctx().pixels_per_point();
    let row_y = at.y + laid.row_y(li, ppp);
    let row_h = laid.row_height();
    if !ui.clip_rect().y_range().intersects(egui::Rangef::new(row_y, row_y + row_h)) {
        return egui::Rect::NOTHING;
    }
    let start = laid
        .cursor_local_rect(ui, text, palette, egui::text::CCursor::new(from))
        .translate(at.to_vec2());
    let end = laid
        .cursor_local_rect(ui, text, palette, egui::text::CCursor::new(to))
        .translate(at.to_vec2());
    let t = mg_uikit::tokens(ui.ctx());
    let g = laid.ensure_line(ui, text, li, palette);
    let baseline =
        g.rows.first().and_then(|r| r.glyphs.first().map(|gl| row_y + r.pos.y + gl.pos.y));
    let y = baseline.map_or(start.bottom() - 0.5, |b| wave_y(b, start.bottom(), palette.size));
    let x = egui::Rangef::new(start.left(), end.left().max(start.left() + 2.0));
    // (Only what is in sight: a line of thousands of characters is not
    // made into a wave a few points at a time all along it.)
    let sight = ui.clip_rect().x_range();
    let shown = egui::Rangef::new(x.min.max(sight.min), x.max.min(sight.max));
    if shown.min < shown.max {
        mg_uikit::paint_wavy_line(ui.painter(), shown, y, egui::Stroke::new(WAVE_STROKE, t.error));
    }
    egui::Rect::from_x_y_ranges(shown, start.top()..=start.bottom())
}

/// Where the live error's wave runs (its middle) under a line whose
/// glyphs stand on `baseline`, its row ending at `row_bottom`, its text
/// `size` points: the board's 3 points under the baseline at 13, in that
/// ratio at another size, but never into the next line (the wave and its
/// stroke kept within the row).
pub(crate) fn wave_y(baseline: f32, row_bottom: f32, size: f32) -> f32 {
    let below = baseline + 3.0 * size / 13.0;
    let lowest = row_bottom - mg_uikit::WAVE_HEIGHT / 2.0 - WAVE_STROKE / 2.0;
    below.min(lowest).max(baseline)
}

/// The current line's two hairlines (`.tx.cur`: `inset 0 1px`) at the
/// top and the foot of its row `y`, each on whole pixels at
/// `pixels_per_point` (a 1-point line at 1.25 would be blurred over two):
/// the point rounded to pixels, at least one.
pub(crate) fn hairlines(y: egui::Rangef, pixels_per_point: f32) -> [egui::Rangef; 2] {
    let px = |v: f32| (v * pixels_per_point).round() / pixels_per_point;
    let thick = pixels_per_point.round().max(1.0) / pixels_per_point;
    let (top, foot) = (px(y.min), px(y.max));
    [egui::Rangef::new(top, top + thick), egui::Rangef::new(foot - thick, foot)]
}

/// The live error's wave's stroke, in points.
const WAVE_STROKE: f32 = 1.0;

impl ScriptBuffer {
    /// A script's bytes in an editor, read in the script codepage.
    pub(crate) fn read(bytes: &[u8], source: crate::tree::Stamp) -> ScriptBuffer {
        let codepage = script_codepage();
        let raw = codepage.decode(bytes).into_owned();
        let text = crate::text::to_editor(&raw).0;
        let indent = tools::Indent::guess(&text);
        ScriptBuffer {
            text: text.clone(),
            saved: text,
            raw,
            source,
            bookmarks: Default::default(),
            numbered: [None; 10],
            external: None,
            codepage,
            seen: None,
            indent,
            conversion: None,
            place: None,
            error_span: None,
            undo: ScriptUndo::default(),
            cached_hash: std::cell::Cell::new(None),
            cached_dirty: std::cell::Cell::new(None),
        }
    }

    pub(crate) fn text_hash(&self) -> u64 {
        let key = (self.text.len(), self.text.as_ptr() as usize);
        if let Some((len, ptr, h)) = self.cached_hash.get()
            && len == key.0
            && ptr == key.1
        {
            return h;
        }
        let h = tools::hash(&self.text);
        self.cached_hash.set(Some((key.0, key.1, h)));
        h
    }

    pub(crate) fn invalidate_hash(&self) {
        self.cached_hash.set(None);
        self.cached_dirty.set(None);
    }

    pub(crate) fn set_text(&mut self, text: String) {
        self.text = text;
        self.undo.clear();
        self.invalidate_hash();
    }

    pub(crate) fn replace_range_undo(
        &mut self,
        bytes: std::ops::Range<usize>,
        replacement: &str,
        before: egui::text::CCursorRange,
        after: egui::text::CCursorRange,
        time: f64,
        mergeable: bool,
    ) -> (usize, usize, usize) {
        let removed = self.text[bytes.clone()].to_string();
        let at_byte = bytes.start;
        let old_end = bytes.end;
        self.text.replace_range(bytes, replacement);
        let new_end = at_byte + replacement.len();
        self.invalidate_hash();
        self.undo.record_edit(
            at_byte,
            removed,
            replacement.to_string(),
            before,
            after,
            time,
            mergeable,
        );
        (at_byte, old_end, new_end)
    }

    /// [`ScriptBuffer::replace_range_undo`] for a composition's text (IME):
    /// the whole composition one step ([`ScriptUndo::record_composing`]).
    pub(crate) fn replace_range_composing(
        &mut self,
        bytes: std::ops::Range<usize>,
        replacement: &str,
        before: egui::text::CCursorRange,
        after: egui::text::CCursorRange,
        time: f64,
    ) -> (usize, usize, usize) {
        let removed = self.text[bytes.clone()].to_string();
        let at_byte = bytes.start;
        let old_end = bytes.end;
        self.text.replace_range(bytes, replacement);
        self.invalidate_hash();
        let inserted = replacement.to_string();
        self.undo.record_composing(at_byte, removed, inserted, before, after, time);
        (at_byte, old_end, at_byte + replacement.len())
    }

    pub(crate) fn replace_diff_undo(
        &mut self,
        next: String,
        before: egui::text::CCursorRange,
        after: egui::text::CCursorRange,
        time: f64,
    ) -> (usize, usize, usize) {
        let a = self.text.as_bytes();
        let b = next.as_bytes();
        let n = a.len().min(b.len());
        let mut p = 0;
        while p + 64 <= n && a[p..p + 64] == b[p..p + 64] {
            p += 64;
        }
        while p < n && a[p] == b[p] {
            p += 1;
        }
        while p > 0 && (!self.text.is_char_boundary(p) || !next.is_char_boundary(p)) {
            p -= 1;
        }
        let room = a.len().min(b.len()) - p;
        let (sa, sb) = (&a[a.len() - room..], &b[b.len() - room..]);
        let mut s = 0;
        while s + 64 <= room && sa[room - s - 64..room - s] == sb[room - s - 64..room - s] {
            s += 64;
        }
        while s < room && sa[room - s - 1] == sb[room - s - 1] {
            s += 1;
        }
        let mut old_end = a.len() - s;
        let mut new_end = b.len() - s;
        while old_end < a.len()
            && new_end < b.len()
            && (!self.text.is_char_boundary(old_end) || !next.is_char_boundary(new_end))
        {
            old_end += 1;
            new_end += 1;
        }
        let removed = self.text[p..old_end].to_string();
        let inserted = next[p..new_end].to_string();
        self.text = next;
        self.invalidate_hash();
        self.undo.record_edit(p, removed, inserted, before, after, time, false);
        (p, old_end, new_end)
    }

    /// After the editor's Undo or Redo: where the text (`hash`) is as it
    /// was before or after the last Convert Indentation, Tab indents as it
    /// did then.
    pub(crate) fn follow_conversion(&mut self, hash: u64) {
        if let Some(&(_, then)) = self.conversion.iter().flatten().find(|(h, _)| *h == hash) {
            self.indent = then;
        }
    }

    /// Takes the file's text as it now is (`raw`, with its line ends) as
    /// both the text and the text saved; the editor's history goes only
    /// where that text is another than the editor's.
    fn follow(&mut self, raw: String) {
        let text = crate::text::to_editor(&raw).0;
        // (The module's text the editor's own, as after its Save, Build or
        // Compile wrote it there: the editor's history, indentation and
        // places stand. Clearing them took a Save's Ctrl+Z away: review
        // D2 of the adversarial branch.)
        if text == self.text {
            self.saved = text;
            self.raw = raw;
            self.cached_dirty.set(None);
            return;
        }
        self.indent = tools::Indent::guess(&text);
        self.conversion = None;
        self.text = text.clone();
        self.saved = text;
        self.raw = raw;
        self.undo.clear();
        self.invalidate_hash();
    }

    /// Follows the module's script (`bytes`) where it changed underneath
    /// (undo, import, a command of the NUI Creator or a plugin) and the
    /// text has no unsaved changes: its line ends too. Another language
    /// chosen in Options: a script without unsaved changes is read again
    /// in its codepage. An editor does so as it is drawn, and whatever
    /// takes places in every open script's text (References, Rename
    /// Symbol) before it does: a tab behind another is not drawn.
    pub(crate) fn follow_module(&mut self, bytes: &[u8], source: crate::tree::Stamp) {
        let codepage = script_codepage();
        let recoded = self.codepage != codepage && !self.is_dirty();
        if self.source != source || recoded {
            self.source = source;
            if !self.is_dirty() {
                let module_text = codepage.decode(bytes).into_owned();
                self.codepage = codepage;
                self.follow(module_text);
            }
        }
    }

    pub(crate) fn is_dirty(&self) -> bool {
        let h = self.text_hash();
        if let Some((was, dirty)) = self.cached_dirty.get()
            && was == h
        {
            return dirty;
        }
        let dirty = self.text != self.saved;
        self.cached_dirty.set(Some((h, dirty)));
        dirty
    }

    /// The text as the file has it: with the file's line ends (an
    /// untouched text, the file's own text).
    fn file_text(&self) -> std::borrow::Cow<'_, str> {
        if !self.is_dirty() {
            std::borrow::Cow::Borrowed(&self.raw)
        } else {
            std::borrow::Cow::Owned(crate::text::with_line_ends(&self.text, &self.raw))
        }
    }

    /// The text as the script's bytes, in the codepage it was read in and
    /// with the file's line ends.
    pub(crate) fn bytes(&self) -> Vec<u8> {
        self.codepage.encode_lossy(&self.file_text())
    }

    /// Marks the text saved: the bytes it is saved as.
    pub(crate) fn mark_saved(&mut self) -> Vec<u8> {
        let raw = self.file_text().into_owned();
        let bytes = self.codepage.encode_lossy(&raw);
        self.raw = raw;
        self.saved = self.text.clone();
        self.cached_dirty.set(None);
        bytes
    }

    /// The warning for characters of the text its codepage cannot write.
    pub(crate) fn unwritable(&self, key: ResKey) -> Option<String> {
        crate::text::unwritable_warning(key, self.codepage, &self.text)
    }
}

/// Whether a ground of the builder's own is dark (its relative luminance
/// under a fifth: white text reads better on it than black).
pub(crate) fn ground_is_dark([r, g, b]: [u8; 3]) -> bool {
    let c = |v: u8| {
        let v = f32::from(v) / 255.0;
        if v <= 0.040_45 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) }
    };
    0.2126 * c(r) + 0.7152 * c(g) + 0.0722 * c(b) < 0.18
}

/// The script editor's colours (one per [`SCRIPT_ELEMENTS`](crate::settings::SCRIPT_ELEMENTS))
/// and font size.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Palette {
    pub(crate) colors: [Color32; 8],
    pub(crate) size: f32,
    /// The editor's ground; `None`: the theme's (its text fields').
    pub(crate) background: Option<Color32>,
}

impl Palette {
    /// The theme's colours for each element (its `syn-*` tokens, but the
    /// text's, `text`).
    pub(crate) fn defaults(dark: bool, text: Color32) -> [Color32; 8] {
        let t = if dark { mg_uikit::Tokens::DARK } else { mg_uikit::Tokens::LIGHT };
        [
            text,
            t.syn_comment,
            t.syn_directive,
            t.syn_keyword,
            t.syn_number,
            t.syn_string,
            t.syn_constant,
            t.syn_error,
        ]
    }

    /// The colours of a style: its own where set, else the theme's.
    pub(crate) fn new(style: &ScriptStyle, dark: bool, text: Color32) -> Palette {
        let mut colors = Palette::defaults(dark, text);
        for (c, own) in colors.iter_mut().zip(style.colors) {
            if let Some([r, g, b]) = own {
                *c = Color32::from_rgb(r, g, b);
            }
        }
        let background = style.background.map(|[r, g, b]| Color32::from_rgb(r, g, b));
        Palette { colors, size: f32::from(style.font_size.clamp(6, 48)), background }
    }

    /// For a `Ui`: the app's style and the `Ui`'s theme; on a ground of
    /// the builder's own (Options › Script Editor), the colors of the
    /// theme of its lightness, so that a dark ground in the light theme
    /// (or a light one in the dark) keeps its words readable.
    pub(crate) fn for_ui(style: &ScriptStyle, ui: &Ui) -> Palette {
        match style.background {
            Some(ground) => {
                let dark = ground_is_dark(ground);
                let t = if dark { mg_uikit::Tokens::DARK } else { mg_uikit::Tokens::LIGHT };
                Palette::new(style, dark, t.syn_text)
            }
            None => Palette::new(style, ui.visuals().dark_mode, ui.visuals().text_color()),
        }
    }

    pub(crate) fn font(&self) -> FontId {
        FontId::monospace(self.size)
    }

    /// The distance between two lines, in points: the board's 18 at its
    /// 13-point text (`.code`: `font-size: 13px; line-height: 18px`; M5's
    /// W3, the owner's ruling 21), in that ratio at another size.
    /// Never less than the glyphs with [`LEAST_ROOM`] above and under
    /// them: at 6 to 9 points the board's ratio would leave them touching
    /// the row's top (the board draws only its 13).
    pub(crate) fn pitch(&self) -> f32 {
        let least = (self.size * MONO_HEIGHT + 2.0 * LEAST_ROOM).ceil();
        (self.size * PITCH_RATIO).round().max(least)
    }

    /// [`Self::pitch`] as a line comes out at `pixels_per_point` (egui
    /// rounds a row's height to whole pixels): what a line's place is
    /// counted in.
    pub(crate) fn row_height(&self, pixels_per_point: f32) -> f32 {
        (self.pitch() * pixels_per_point).round() / pixels_per_point
    }

    /// A piece of a line in `color`: in the font, centered in the line's
    /// height (the board's half of the room above the text, half below).
    pub(crate) fn format(&self, color: Color32) -> TextFormat {
        TextFormat { valign: egui::Align::Center, ..TextFormat::simple(self.font(), color) }
    }

    /// Makes `job`, one line of the text, a line [`Self::pitch`] high,
    /// its text centered in it ([`Self::format`]).
    pub(crate) fn pitched(&self, mut job: LayoutJob) -> LayoutJob {
        job.first_row_min_height = self.pitch();
        job.wrap.max_width = f32::INFINITY;
        job
    }
}

/// The board's line pitch to its text size (`.code`: 18 to 13).
const PITCH_RATIO: f32 = 18.0 / 13.0;

/// The monospace font's (Hack's) row height to its size, as egui lays it
/// out (15.125 at 13).
pub(crate) const MONO_HEIGHT: f32 = 1.164;

/// The least room above a line's glyphs, and under them, in points.
const LEAST_ROOM: f32 = 1.0;

/// `text`'s lines highlighted ([`highlight`], whole: a comment may run over
/// lines), each a job of its own [`Palette::pitch`] high: the editor's
/// lines as the read-only view and Options' preview draw them.
pub(crate) fn highlight_lines(text: &str, palette: &Palette) -> Vec<LayoutJob> {
    let whole = highlight(text, palette);
    let mut jobs = vec![LayoutJob::default()];
    for section in &whole.sections {
        let piece = &whole.text[section.byte_range.start.0..section.byte_range.end.0];
        for (at, part) in piece.split('\n').enumerate() {
            if at > 0 {
                jobs.push(LayoutJob::default());
            }
            if !part.is_empty() {
                jobs.last_mut().expect("one").append(part, 0.0, section.format.clone());
            }
        }
    }
    jobs.into_iter()
        .map(|mut job| {
            // (An empty line: the height of its font, centered.)
            if job.sections.is_empty() {
                job.append("", 0.0, palette.format(palette.colors[0]));
            }
            palette.pitched(job)
        })
        .collect()
}

/// `text` highlighted and laid out a line at a time, each line
/// [`Palette::pitch`] high (Options' preview of the editor's look).
pub(crate) fn highlight_galley(
    ui: &Ui,
    text: &str,
    palette: &Palette,
) -> std::sync::Arc<egui::Galley> {
    let lines: Vec<_> = highlight_lines(text, palette)
        .into_iter()
        .map(|j| ui.fonts_mut(|f| f.layout_job(j)))
        .collect();
    let mut job =
        LayoutJob::simple(text.to_owned(), palette.font(), palette.colors[0], f32::INFINITY);
    job.wrap.max_width = f32::INFINITY;
    std::sync::Arc::new(egui::Galley::concat(
        std::sync::Arc::new(job),
        &lines,
        ui.ctx().pixels_per_point(),
    ))
}

/// The element a token is coloured as.
pub(crate) fn element(kind: TokenKind) -> usize {
    match kind {
        TokenKind::LineComment | TokenKind::BlockComment { .. } => 1,
        TokenKind::Directive => 2,
        TokenKind::Keyword => 3,
        TokenKind::Int | TokenKind::Float => 4,
        TokenKind::String { .. } | TokenKind::RawString { .. } | TokenKind::HashedString { .. } => {
            5
        }
        TokenKind::BuiltinConstant => 6,
        TokenKind::Unknown => 7,
        _ => 0,
    }
}

/// Lays the text out with token colours.
pub(crate) fn highlight(text: &str, palette: &Palette) -> LayoutJob {
    let mut job = LayoutJob::default();
    for t in tokenize(text.as_bytes()) {
        let c = palette.colors[element(t.kind)];
        // Tokens end on character boundaries except inside malformed UTF-8,
        // which the text (a String) cannot contain.
        job.append(&text[t.span], 0.0, palette.format(c));
    }
    job
}

/// Text for HTML.
fn escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

/// A script as an HTML page that prints itself (Print: the browser's print
/// dialog), highlighted in the editor's colours on white paper.
pub(crate) fn print_html(name: &str, text: &str, style: &crate::settings::ScriptStyle) -> String {
    use std::fmt::Write;
    // (The light theme's colors: dark text on white paper.)
    let job = highlight(text, &Palette::new(style, false, mg_uikit::Tokens::LIGHT.syn_text));
    let mut body = String::new();
    for s in &job.sections {
        let c = s.format.color;
        let t = escape(&job.text[s.byte_range.start.0..s.byte_range.end.0]);
        let _ = write!(
            body,
            "<span style=\"color:#{:02x}{:02x}{:02x}\">{t}</span>",
            c.r(),
            c.g(),
            c.b()
        );
    }
    let n = escape(name);
    format!(
        "<!DOCTYPE html>\n<html><head><meta charset=\"utf-8\"><title>{n}</title>\
         <style>body{{font-family:monospace;font-size:10pt}}pre{{white-space:pre-wrap}}</style>\
         </head><body onload=\"window.print()\"><h3>{n}</h3><pre>{body}</pre></body></html>\n"
    )
}

/// A file's `file://` URL.
pub(crate) fn file_url(path: &std::path::Path) -> String {
    let p = path.to_string_lossy().replace('\\', "/");
    let p = p.replace('%', "%25").replace(' ', "%20").replace('#', "%23");
    if p.starts_with('/') { format!("file://{p}") } else { format!("file:///{p}") }
}

/// Where a selection is drawn, row by row, in the galley's own space: from
/// its first character to its last, and on the rows between to each row's
/// last character (a row with none: a sliver, so that an empty line in a
/// selection is seen to be in it).
#[cfg(test)]
pub(crate) fn selection_rects(
    galley: &egui::Galley,
    range: &egui::text::CCursorRange,
) -> Vec<egui::Rect> {
    if range.is_empty() {
        return Vec::new();
    }
    let [min, max] = range.sorted_cursors();
    let (min, max) = (galley.layout_from_cursor(min), galley.layout_from_cursor(max));
    let mut out = Vec::new();
    for ri in min.row..=max.row {
        let Some(placed) = galley.rows.get(ri) else { continue };
        let row = &placed.row;
        let left = if ri == min.row { row.x_offset(min.column) } else { 0.0 };
        let right = if ri == max.row { row.x_offset(max.column) } else { row.size.x };
        // (The last row with nothing of it selected: the selection ends
        // at its start.)
        if ri == max.row && ri != min.row && right <= left {
            continue;
        }
        let right = if right <= left && ri != max.row { left + 3.0 } else { right };
        let rect = egui::Rect::from_min_max(egui::pos2(left, 0.0), egui::pos2(right, row.size.y));
        out.push(rect.translate(placed.pos.to_vec2()));
    }
    out
}

pub(crate) fn editor_id(key: ResKey) -> egui::Id {
    egui::Id::new(("script", key))
}

/// Whether a script's editor has the keyboard: Undo and Redo are its own
/// then, whatever was done in it last.
pub(crate) fn has_keys(app: &Moonglow, ctx: &egui::Context) -> bool {
    with_keys(app, ctx).is_some()
}

/// The script whose editor has the keyboard (its text is focused), if
/// one's does: Undo, Redo and the Save key are that editor's.
pub(crate) fn with_keys(app: &Moonglow, ctx: &egui::Context) -> Option<ResKey> {
    let focused = ctx.memory(|m| m.focused())?;
    app.scripts.keys().find(|k| editor_id(**k) == focused).copied()
}

/// Notes the script Edit › Undo and Redo and the Top Bar's are for this
/// frame (`Moonglow::undo_script`): the one whose editor has the keys,
/// as its Ctrl+Z goes, and while a menu is open the one that had them as
/// it was opened (the click on the menu takes the keys from the editor).
/// Drawn before the editor, the Top Bar sees the keys where they were
/// before its own click.
pub(crate) fn undo_target(app: &mut Moonglow, ctx: &egui::Context) {
    // (An editor not drawn the frame its keys were to come back: none.)
    app.script_tools.keys_back = None;
    if app.menu_open.is_none() {
        // (Or the script the Find or Replace window works on, while that
        // window has the keys: what Replace All just changed is what Undo
        // is for, not the module's last step somewhere else.)
        app.undo_script = with_keys(app, ctx).or(app.script_tools.find_keys);
    }
    app.undo_script = app.undo_script.filter(|k| app.script_open(k));
}

/// The keys back in the editor of the script that had them as a menu
/// was opened (`Moonglow::undo_script`), where nothing else took them
/// meanwhile: a menu closed with nothing chosen. (Once the editor is
/// drawn: a click on the menu's name would take them again as it is.)
pub(crate) fn give_keys_back(app: &mut Moonglow, ctx: &egui::Context) {
    if ctx.memory(|m| m.focused().is_none()) {
        app.script_tools.keys_back = app.undo_script.filter(|k| app.script_open(k));
    }
}

/// Edit › Undo (or, `redo`, Redo) in the script's editor: a step of its
/// own history, as Ctrl+Z takes there (with none, nothing), once the
/// editor is drawn; the editor has the keys again after it.
pub(crate) fn undo(app: &mut Moonglow, ctx: &egui::Context, key: ResKey, redo: bool) {
    app.script_tools.undo = Some((key, redo));
    ctx.request_repaint();
}

/// A step of the editor's own history taken, back or (`redo`) again, as
/// its Ctrl+Z and Ctrl+Y take one (with none, nothing is). Each step
/// changes the text ([`ScriptUndo`] keeps edits alone, not the cursor's
/// moves), so Undo from a menu takes something back.
fn step_history(
    buf: &mut ScriptBuffer,
    laid: Option<&mut crate::script_layout::LaidOut>,
    ctx: &egui::Context,
    key: ResKey,
    redo: bool,
) {
    let was = buf.text_hash();
    let Some((range, at, old_end, new_end)) = buf.undo.step(&mut buf.text, redo) else { return };
    buf.invalidate_hash();
    let h = buf.text_hash();
    buf.follow_conversion(h);
    if let Some(laid) = laid {
        laid.apply_edit(&buf.text, at, old_end, new_end, was, h);
    }
    let id = editor_id(key);
    let mut state = egui::text_edit::TextEditState::load(ctx, id).unwrap_or_default();
    state.cursor.set_char_range(Some(range));
    state.store(ctx, id);
}

/// Moves an editor's cursor (and selection) and focuses it.
fn set_cursor(ctx: &egui::Context, key: ResKey, from: usize, to: usize) {
    let id = editor_id(key);
    let mut state = egui::text_edit::TextEditState::load(ctx, id).unwrap_or_default();
    state.cursor.set_char_range(Some(egui::text::CCursorRange::two(
        egui::text::CCursor::new(from),
        egui::text::CCursor::new(to),
    )));
    state.store(ctx, id);
    // (An editor not drawn, its tab behind another, takes the keys when it
    // is: focus on a widget not drawn named a node the accessibility tree
    // has not, which AccessKit's consumer panics at.)
    if ctx.read_response(id).is_some() {
        ctx.memory_mut(|m| m.request_focus(id));
    } else {
        ctx.data_mut(|d| {
            let waiting = d.get_temp_mut_or_default::<Vec<ResKey>>(focus_when_drawn());
            if !waiting.contains(&key) {
                waiting.push(key);
            }
        });
    }
}

/// Where [`set_cursor`] leaves the keys for the editors not drawn: the
/// scripts', until each is drawn or its tab closed (`forget_closed`).
fn focus_when_drawn() -> egui::Id {
    egui::Id::new("scripts to focus when drawn")
}

/// Whether `key`'s editor was left the keys while it was not drawn; no
/// longer after.
fn take_focus_left(ctx: &egui::Context, key: ResKey) -> bool {
    ctx.data_mut(|d| {
        let waiting = d.get_temp_mut_or_default::<Vec<ResKey>>(focus_when_drawn());
        let had = waiting.len();
        waiting.retain(|k| *k != key);
        waiting.len() != had
    })
}

/// What is kept for scripts whose tabs are closed is let go of: their
/// laid-out text (a galley a line: megabytes a script, kept for every
/// script ever opened until the module closed), and the keys left for
/// their editors (a script Find went to behind another tab, its tab closed
/// and opened again later, took the keys and scrolled then).
fn forget_closed(app: &mut Moonglow, ctx: &egui::Context) {
    let Moonglow { laid_out, dock, scripts, ws, .. } = app;
    let open = |k: &ResKey| dock.find_tab(&Tab::Script(*k)).is_some();
    if !laid_out.is_empty() {
        laid_out.retain(|k, _| open(k));
    }
    // (And the editor's state of a closed script the module no longer
    // has, with nothing unsaved: kept for the session to open it again.)
    if let Some(ws) = ws {
        scripts.retain(|k, b| b.is_dirty() || open(k) || ws.module.get(k).is_some());
    }
    ctx.data_mut(|d| d.get_temp_mut_or_default::<Vec<ResKey>>(focus_when_drawn()).retain(open));
}

/// The editor's cursor as a character range (start, end).
fn cursor(ctx: &egui::Context, key: ResKey) -> Option<(usize, usize)> {
    let state = egui::text_edit::TextEditState::load(ctx, editor_id(key))?;
    let r = state.cursor.char_range()?;
    let (a, b) = (usize::from(r.primary.index), usize::from(r.secondary.index));
    Some((a.min(b), a.max(b)))
}

/// Replaces a character range of the buffer and puts the cursor after it.
#[allow(clippy::too_many_arguments)]
fn insert(
    buf: &mut ScriptBuffer,
    laid: Option<&mut crate::script_layout::LaidOut>,
    ctx: &egui::Context,
    key: ResKey,
    from: usize,
    to: usize,
    text: &str,
) {
    let (a, b) = (from.min(to), from.max(to));
    // (Places read from the laid-out lines only where they are this
    // text's: a script rewritten in a tab behind another has those of the
    // text before, until its editor is drawn.)
    let was = buf.text_hash();
    let (bf, bt) = match laid.as_deref() {
        Some(l) if l.in_sync(&buf.text, was) => {
            (l.byte_of_char(&buf.text, a), l.byte_of_char(&buf.text, b))
        }
        _ => (tools::byte_index(&buf.text, a), tools::byte_index(&buf.text, b)),
    };
    let at_chars = a + text.chars().count();
    let before =
        egui::text::CCursorRange::two(egui::text::CCursor::new(from), egui::text::CCursor::new(to));
    let after = egui::text::CCursorRange::one(egui::text::CCursor::new(at_chars));
    let now = ctx.input(|i| i.time);
    let (at, old_end, new_end) = buf.replace_range_undo(bf..bt, text, before, after, now, false);
    if let Some(laid) = laid {
        let h = buf.text_hash();
        laid.apply_edit(&buf.text, at, old_end, new_end, was, h);
    }
    set_cursor(ctx, key, at_chars, at_chars);
}

/// Breaks contiguous typing merge in the editor's undo stack before a bulk or
/// replace step (`Replace`, `Replace All`, `Convert Indentation`).
fn undo_point(buf: &mut ScriptBuffer, _ctx: &egui::Context, _key: ResKey) {
    buf.undo.mark_break();
}

/// The editor's context menu's conversions (Edit › Convert Indentation
/// to Tabs and to Spaces): to tabs or not, and its name.
const INDENT_CONVERSIONS: [(bool, &str); 2] =
    [(true, "Convert Indentation to Tabs"), (false, "Convert Indentation to Spaces")];

/// Convert Indentation to Tabs (`tabs`) or to Spaces: every line's
/// indentation made so ([`tools::convert_indentation`], a step `width`
/// columns), or with text `selected` the lines it touches; one step for the
/// editor's Undo. Tab indents so from then on (a selection's: as the
/// script now is, guessed again), and as before once Undo takes it back.
/// The cursor stays on its line, at its start.
fn convert_indentation(
    buf: &mut ScriptBuffer,
    laid: Option<&mut crate::script_layout::LaidOut>,
    ctx: &egui::Context,
    key: ResKey,
    tabs: bool,
    selected: bool,
    width: usize,
) {
    let id = editor_id(key);
    let mut state = egui::text_edit::TextEditState::load(ctx, id).unwrap_or_default();
    let before = state.cursor.char_range().unwrap_or_default();
    let (a, b) = match selected {
        true => (usize::from(before.primary.index), usize::from(before.secondary.index)),
        false => (0, 0),
    };
    let converted = tools::convert_indentation_in(&buf.text, tabs, width, a, b);
    let made = match a == b {
        true => Some(if tabs { tools::Guess::Tabs } else { tools::Guess::Spaces(width) }),
        false => tools::Indent::guess(&converted),
    };
    if converted == buf.text {
        buf.indent = made;
        return;
    }
    buf.conversion = Some([(buf.text_hash(), buf.indent), (tools::hash(&converted), made)]);
    buf.indent = made;
    let line =
        buf.text.chars().take(usize::from(before.primary.index)).filter(|c| *c == '\n').count();
    let at_c = egui::text::CCursorRange::one(egui::text::CCursor::new(tools::line_start(
        &converted, line,
    )));
    let now = ctx.input(|i| i.time);
    let was = buf.text_hash();
    let (at, old_end, new_end) = buf.replace_diff_undo(converted, before, at_c, now);
    if let Some(laid) = laid {
        let h = buf.text_hash();
        laid.apply_edit(&buf.text, at, old_end, new_end, was, h);
    }
    state.cursor.set_char_range(Some(at_c));
    state.store(ctx, id);
}

pub(crate) fn ui(app: &mut Moonglow, ui: &mut Ui, key: ResKey) {
    // Fit the pane, whose clip rectangle is its true size: the tab does not
    // scroll (`Viewer::scroll_bars`), and the dock's non-scrolling scroll area
    // grows to the size its content had before (so the side lists ran past
    // the pane's edge). A child area, as a Ui never shrinks below its size.
    let margin = ui.max_rect().left() - ui.clip_rect().left();
    let mut fit = ui.max_rect();
    fit.max.x = fit.max.x.min(ui.clip_rect().max.x - margin).max(fit.min.x + 120.0);
    fit.max.y = fit.max.y.min(ui.clip_rect().max.y - margin).max(fit.min.y + 120.0);
    ui.scope_builder(egui::UiBuilder::new().max_rect(fit), |ui| {
        // (Cut at its room: in a small window, what is wider than it (the
        // bottom pane's tabs) would draw over the window's right pane,
        // Reference and Help, drawn before it.)
        ui.set_clip_rect(fit.intersect(ui.clip_rect()));
        // (Why it could not be saved, at the foot; its OK and Cancel at the
        // header's end: `header`.)
        crate::side_doc::failed_foot(app, ui, &Tab::Script(key));
        editor(app, ui, key);
    });
}

fn editor(app: &mut Moonglow, ui: &mut Ui, key: ResKey) {
    // The external editor's saves, if it has the script (looked for twice a
    // second).
    reload_external(app, key);
    if app.scripts.get(&key).is_some_and(|b| b.external.is_some()) {
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(500));
    }
    let Some(ws) = &app.ws else { return };
    let Some(bytes) = ws.module.get(&key) else {
        ui.label(format!("{key} is no longer in the module."));
        return;
    };
    let source = crate::tree::stored_stamp(ws, &key, bytes);
    app.scripts.entry(key).or_insert_with(|| ScriptBuffer::read(bytes, source));
    if app.script_tools.open_externally.remove(&key)
        && let Some(editor) = app.settings.external_editor.clone()
    {
        open_external(app, key, &editor);
    }
    let Some(ws) = &app.ws else { return };
    let Some(bytes) = ws.module.get(&key) else { return };
    app.scripts
        .get_mut(&key)
        .expect("just inserted")
        .follow_module(bytes, crate::tree::stored_stamp(ws, &key, bytes));
    let ctx = ui.ctx().clone();

    // This script edited, or the module changed: what other scripts found
    // in their includes is looked for again (their caches go by their own
    // text alone).
    // (The text's hash, once a frame: the symbols and the live check
    // go by it too, and a script may be some 600 KB.)
    let h = app.scripts[&key].text_hash();
    // (A conversion taken back by the editor's Undo: Tab as before it.)
    app.scripts.get_mut(&key).expect("just inserted").follow_conversion(h);
    {
        let revision = app.ws.as_ref().map_or(0, |w| w.revision());
        let t = &mut app.script_tools;
        let edited = t.seen.insert(key, h).is_some_and(|was| was != h);
        if t.revision != revision {
            t.revision = revision;
            t.clear_symbols();
            app.script_nav.check_again(None);
        } else if edited {
            t.invalidate_dependents(key);
            app.script_nav.check_again(Some(key));
        }
    }

    // The symbols the script can use (its own, its includes', nwscript's).
    let symbols = {
        let text = &app.scripts[&key].text;
        let (game, ws) = (&app.game, &app.ws);
        let mut source = |name: &str| -> Option<String> {
            script_source(&app.scripts, ws.as_ref(), game.as_deref(), &tools::nss(name)?)
        };
        // (A key typed this frame, or text composed, pasted or cut: the
        // symbols of the frame before. The frame after it, which egui
        // draws, reads them.)
        let typing = ctx.memory(|m| m.has_focus(editor_id(key)))
            && ctx.input(|i| {
                i.events.iter().any(|e| {
                    matches!(
                        e,
                        egui::Event::Text(_)
                            | egui::Event::Key { pressed: true, .. }
                            | egui::Event::Ime(_)
                            | egui::Event::Paste(_)
                            | egui::Event::Cut
                    )
                })
            });
        // (A frame more, for the symbols of the text as typed, and the
        // Reference panel's list with them: M4's U12 review.)
        if typing {
            ctx.request_repaint();
        }
        app.script_tools.symbols_for_ui(
            key,
            h,
            typing,
            text,
            || {
                let g = game.as_ref()?;
                let data = g.resman.get_named("nwscript", ResType::NSS).ok()?;
                Some(mg_script::spec::Spec::parse(&data))
            },
            &mut source,
            &ctx,
            app.background_jobs,
        )
    };

    // Shortcuts while the editor has focus.
    let focused = ctx.memory(|m| m.has_focus(editor_id(key)));
    if focused {
        app.script_tools.last_typed = Some(key);
    }
    let mut open_find = false;
    let mut open_replace = false;
    let mut find_next = false;
    let mut toggle_bookmark = false;
    let mut complete = false;
    let mut set_numbered = None;
    let mut go_numbered = None;
    let (mut definition, mut references, mut rename) = (false, false, false);
    if focused {
        use crate::keys::Cmd;
        use egui::{Key, KeyboardShortcut, Modifiers};
        let pressed = |m, k| ui.input_mut(|i| i.consume_shortcut(&KeyboardShortcut::new(m, k)));
        // The keys of Options › Keyboard.
        let keys = &app.keymap;
        let cmd = |c: Cmd| ui.input_mut(|i| keys.consume(i, c));
        rename = cmd(Cmd::RenameSymbol);
        open_find = cmd(Cmd::Find);
        open_replace = cmd(Cmd::Replace);
        find_next = cmd(Cmd::FindNext);
        toggle_bookmark = cmd(Cmd::Bookmark);
        complete = cmd(Cmd::Complete);
        references = cmd(Cmd::References);
        definition = cmd(Cmd::Definition);
        const DIGITS: [Key; 9] = [
            Key::Num1,
            Key::Num2,
            Key::Num3,
            Key::Num4,
            Key::Num5,
            Key::Num6,
            Key::Num7,
            Key::Num8,
            Key::Num9,
        ];
        for (i, k) in DIGITS.into_iter().enumerate() {
            if pressed(Modifiers::COMMAND | Modifiers::SHIFT, k) {
                set_numbered = Some(i + 1);
            } else if pressed(Modifiers::COMMAND, k) {
                go_numbered = Some(i + 1);
            }
        }
    }

    let dirty = app.scripts[&key].is_dirty();
    let asked = header(app, ui, key, dirty);
    // OK and Cancel, as the other editors have: a script is saved when
    // asked (its Save puts its text into the module), so OK saves it and
    // Cancel drops what is unsaved, asking first (its × keeps that for the
    // module's Save). Closing: nothing more is drawn of it.
    if let Some(pressed) = asked.pressed
        && crate::side_doc::pressed_ok_cancel(app, &Tab::Script(key), false, pressed, |_| {})
    {
        return;
    }
    open_find |= asked.find;
    open_replace |= asked.replace;
    toggle_bookmark |= asked.bookmark;
    definition |= asked.definition;
    references |= asked.references;
    rename |= asked.rename;
    let (save, compile, save_as, to_scratch) =
        (asked.save, asked.compile, asked.save_as, asked.to_scratch);
    // The check as you type: its error is the Status Bar's message while
    // the script is the editor the bar follows (the docked one in front,
    // or a floating one under the pointer or pressed in last:
    // `shell::update_shown`), at the end of a floating script's bottom
    // tab bar too, and a wavy underline on its line with the error in its
    // tip (below), whatever the host (M4's U13: the row of its own under
    // the header is gone).
    if !app.settings.no_live_check {
        app.live_check(key, h, &ctx);
    }

    if definition || references || rename {
        let at = cursor(&ctx, key).map_or(0, |c| c.0);
        match app.declaration_at(key, at) {
            Some(d) if definition => {
                app.go_to_declaration(&d);
                ask_help_forward(app, &ctx, key);
            }
            Some(d) if references => app.show_references(&d, Some(key)),
            Some(d) => {
                let name = d.name.clone();
                app.script_nav.rename = Some((d, name));
            }
            None => app.log.info("No declaration for the name at the cursor"),
        }
    }

    let tools_state = &mut app.script_tools;
    if open_find || open_replace {
        let s = &mut tools_state.search;
        s.open = true;
        s.replace_mode = open_replace;
        s.script = Some(key);
        if let Some((a, b)) = cursor(&ctx, key)
            && a != b
        {
            let t = &app.scripts[&key].text;
            s.find = t.chars().skip(a).take(b - a).collect();
        }
    }
    if find_next {
        tools_state.search.script = Some(key);
        find_in_script(app, &ctx, key);
    }
    if toggle_bookmark {
        let at = cursor(&ctx, key).map_or(0, |c| c.0);
        let buf = app.scripts.get_mut(&key).expect("open");
        let line = tools::line_of(&buf.text, at);
        if !buf.bookmarks.remove(&line) {
            buf.bookmarks.insert(line);
        }
        app.script_tools.ask_forward(Some(key), InfoTab::Bookmarks, false);
    }
    if let Some(n) = set_numbered {
        let at = cursor(&ctx, key).map_or(0, |c| c.0);
        let buf = app.scripts.get_mut(&key).expect("open");
        let line = tools::line_of(&buf.text, at);
        // Setting a number again on its line clears it.
        buf.numbered[n] = if buf.numbered[n] == Some(line) { None } else { Some(line) };
        app.script_tools.ask_forward(Some(key), InfoTab::Bookmarks, false);
    }
    if let Some(line) = go_numbered.and_then(|n| app.scripts[&key].numbered[n]) {
        let at = tools::line_start(&app.scripts[&key].text, line);
        app.script_tools.jump = Some((key, at));
    }
    if complete && let Some((at, _)) = cursor(&ctx, key) {
        let (start, prefix) = tools::word_before(&app.scripts[&key].text, at);
        let items = symbols.completions(&prefix);
        if !items.is_empty() {
            app.script_tools.completion =
                Some(Completion { script: key, start, items, selected: 0 });
        }
    }

    // What the Reference panel put in (a double click on a name), drawn
    // before the editor: at the cursor.
    let insert_text = app.script_tools.insert.take_if(|(k, _)| *k == key).map(|(_, text)| text);

    // (Compiler, Bookmarks and Search Results are the Bottom Dock's panels
    // since M4's U13, or its window's bottom pane's where it floats:
    // `script_panels`, `search_results`, drawn before the editor.)
    let palette = Palette::for_ui(&app.settings.script_style, ui);

    // The editor with line numbers.
    let style_indent = tools::Indent::of(&app.settings.script_style);
    // (Options › Script Editor › Show declarations in tips, 5.2.0.)
    let symbol_tips = !app.settings.no_symbol_tips;
    // (The live error: its line (none: in an include, `live_include` saying
    // which and where), and what it says for the underline's tip.)
    let live_error = app.live_error(key).cloned();
    let live_include = app.live_include(key).cloned();
    let mut ctrl_click = None;
    // (The plugins' commands for scripts, for its right-click menu.)
    let commands = app.plugin_menu_commands_cached();
    let script = [mg_plugin::Selected::Resource(key)];
    let script_kinds = crate::plugins::kinds_of(&script);
    let mut plugin_chosen = None;
    // (Its right-click menu's Undo and Redo keys, Options › Keyboard's:
    // worked out only while the menu is open or asked for.)
    let menu_wanted = crate::widgets::text_menu_wanted(&ctx, editor_id(key));
    let undo_keys =
        menu_wanted.then(|| (app.keymap.label_of("undo", &ctx), app.keymap.label_of("redo", &ctx)));
    let Moonglow { scripts, laid_out, script_tools, open_tabs, .. } = app;
    let buf = scripts.get_mut(&key).expect("open");
    let cached = laid_out.entry(key).or_default();
    cached.sync_full(ui, &buf.text, h, &palette);
    // Tab indents as the script is indented (Jasperre's report), else as
    // Options › Script Editor says.
    let mut indent = tools::Indent::of_script(buf.indent, style_indent);
    if let Some((_, tabs, selected)) = script_tools.convert_indent.take_if(|(k, ..)| *k == key) {
        convert_indentation(buf, Some(cached), &ctx, key, tabs, selected, indent.width);
        indent = tools::Indent::of_script(buf.indent, style_indent);
    }
    // Edit › Undo or Redo, or the Top Bar's: the editor's step, and the
    // keys back in it after (below, once the click that took them is
    // past), so that Ctrl+Z after it is the editor's too, not the
    // module's.
    let stepped = script_tools.undo.take_if(|(k, _)| *k == key);
    if let Some((_, redo)) = stepped {
        step_history(buf, Some(cached), &ctx, key, redo);
        // (As typing closes it.)
        script_tools.completion = None;
    }
    // Scrolled and its cursor where they were, a module's kept tabs
    // opening it again (the cursor's not given the keys).
    let kept = open_tabs.take_view(&crate::Tab::Script(key));
    let (kept_scroll, kept_cursor) = kept.as_ref().map(|v| v.script()).unwrap_or_default();
    // Its bookmarks as they were (those its text still has lines for).
    if let Some(v) = &kept {
        (buf.bookmarks, buf.numbered) = v.script_marks(cached.line_count());
        // (Bookmarks, drawn before it, shows them.)
        script_tools.made += 1;
    }
    if let Some((a, b)) = kept_cursor {
        let id = editor_id(key);
        let end = cached.total_chars();
        let mut state = egui::text_edit::TextEditState::load(&ctx, id).unwrap_or_default();
        let at = |n: usize| egui::text::CCursor::new(n.min(end));
        state.cursor.set_char_range(Some(egui::text::CCursorRange::two(at(a), at(b))));
        state.store(&ctx, id);
    }
    if let Some(text) = insert_text {
        let (a, b) = cursor(&ctx, key).unwrap_or((cached.total_chars(), cached.total_chars()));
        insert(buf, Some(cached), &ctx, key, a, b, &text);
    }
    // The cursor gone from the word the list completes (Home, a click
    // elsewhere) closes it: what it would replace is no longer that word.
    if let Some(c) = script_tools.completion.as_ref().filter(|c| c.script == key)
        && cursor(&ctx, key).is_none_or(|(at, _)| tools::word_before(&buf.text, at).0 != c.start)
    {
        script_tools.completion = None;
    }
    // Completion keys go to the list, not the text.
    let mut accept = None;
    if let Some(c) = script_tools.completion.as_mut().filter(|c| c.script == key) {
        use egui::{Key, Modifiers};
        let n = c.items.len();
        ui.input_mut(|i| {
            if i.consume_key(Modifiers::NONE, Key::ArrowDown) {
                c.selected = (c.selected + 1) % n;
            }
            if i.consume_key(Modifiers::NONE, Key::ArrowUp) {
                c.selected = (c.selected + n - 1) % n;
            }
            if i.consume_key(Modifiers::NONE, Key::Enter)
                || i.consume_key(Modifiers::NONE, Key::Tab)
            {
                accept = Some(c.selected);
            }
        });
        if ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape)) {
            script_tools.completion = None;
        }
    }
    if let Some(i) = accept
        && let Some(c) = script_tools.completion.take()
    {
        let s = &c.items[i];
        let text =
            if s.kind == SymbolKind::Function { format!("{}(", s.name) } else { s.name.clone() };
        let at = cursor(&ctx, key).map_or(c.start, |x| x.0);
        insert(buf, Some(cached), &ctx, key, c.start, at, &text);
    }
    // Text pasted has the editor's line ends, plain newlines (a copy from
    // a Windows program has CRLF).
    if ui.memory(|m| m.has_focus(editor_id(key))) {
        ui.input_mut(|i| {
            for e in &mut i.events {
                if let egui::Event::Paste(s) | egui::Event::Text(s) = e
                    && s.contains('\r')
                {
                    *s = s.replace("\r\n", "\n").replace('\r', "\n");
                }
            }
        });
    }
    let mut page_scroll = None;
    // Tab on a selection of several lines indents them; Shift+Tab outdents
    // the selected lines, or the cursor's. Tab alone puts in a step of
    // indentation (the script's own, else Options › Script Editor's:
    // four spaces, unless chosen), not the field's own tab.
    if script_tools.completion.is_none()
        && ui.memory(|m| m.has_focus(editor_id(key)))
        && let Some((a, b)) = cursor(&ctx, key)
    {
        use egui::{Key, Modifiers};
        let outdent = ui.input_mut(|i| i.consume_key(Modifiers::SHIFT, Key::Tab));
        let tab = !outdent && ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Tab));
        // (Looked for when Tab is pressed, in the selection: every
        // character before the cursor was gone through each frame.)
        let lines = tab && tools::spans_lines(&buf.text, a, b);
        if outdent || (tab && lines) {
            let (text, from, to) = tools::indent_lines(&buf.text, a, b, outdent, indent);
            let before = egui::text::CCursorRange::two(
                egui::text::CCursor::new(a),
                egui::text::CCursor::new(b),
            );
            let after = egui::text::CCursorRange::two(
                egui::text::CCursor::new(from),
                egui::text::CCursor::new(to),
            );
            let now = ctx.input(|i| i.time);
            let was = buf.text_hash();
            let (at, old_end, new_end) = buf.replace_diff_undo(text, before, after, now);
            let nh = buf.text_hash();
            cached.apply_edit(&buf.text, at, old_end, new_end, was, nh);
            set_cursor(&ctx, key, from, to);
        } else if tab {
            insert(buf, Some(cached), &ctx, key, a, b, &indent.at(&buf.text, a));
        }
        // Enter begins the new line as far in as the line it is started
        // from (Aurora does; the field alone begins it at the margin).
        if ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Enter)) {
            let start = format!("\n{}", tools::new_line_indent(&buf.text, a));
            insert(buf, Some(cached), &ctx, key, a, b, &start);
            script_tools.scroll = Some(key);
        }
        // Page Down and Page Up move the cursor by the lines in sight, and
        // the text with it (the field alone has no page keys); with Shift,
        // the selection.
        let row = palette.row_height(ui.ctx().pixels_per_point());
        let page = ((ui.available_height() / row).floor() as isize - 1).max(1);
        for (k, lines) in [(Key::PageDown, page), (Key::PageUp, -page)] {
            let select = ui.input_mut(|i| i.consume_key(Modifiers::SHIFT, k));
            if !select && !ui.input_mut(|i| i.consume_key(Modifiers::NONE, k)) {
                continue;
            }
            let id = editor_id(key);
            let mut state = egui::text_edit::TextEditState::load(&ctx, id).unwrap_or_default();
            let Some(r) = state.cursor.char_range() else { continue };
            let to = cached.line_moved_char(usize::from(r.primary.index), lines);
            let anchor = if select { r.secondary } else { egui::text::CCursor::new(to) };
            state.cursor.set_char_range(Some(egui::text::CCursorRange {
                primary: egui::text::CCursor::new(to),
                secondary: anchor,
                h_pos: None,
            }));
            state.store(&ctx, id);
            page_scroll = Some(lines as f32 * row);
        }
    }
    let pending = script_tools.jump.take_if(|(k, _)| *k == key).map(|(_, at)| at);
    if let Some(at) = pending {
        set_cursor(&ctx, key, at, at);
        script_tools.scroll = Some(key);
    }
    // (The text changed before the editor is drawn, by the menus' Undo,
    // Tab, Enter or a completion put in: the cursor brought into sight, as
    // typing brings it: review N4.)
    let reveal = stepped.is_some() || buf.text_hash() != h;
    // (The keys `set_cursor` left for this editor while it was not drawn,
    // its cursor brought into sight with them.)
    let left_focus = take_focus_left(&ctx, key);
    let reveal = reveal || left_focus;
    if left_focus {
        ctx.memory_mut(|m| m.request_focus(editor_id(key)));
    }
    let h = buf.text_hash();
    cached.sync_full(ui, &buf.text, h, &palette);
    let mut completion_at = None;
    // (With smooth scrolling off, a scroll to the cursor taken in the frame
    // that asks for it, not as an animation done the frame after: review
    // K's N6.)
    let smooth = mg_uikit::scrolling::is_smooth(ui.ctx());
    let mut area = egui::ScrollArea::both().id_salt(("script-scroll", key)).animated(smooth);
    if let Some(at) = kept_scroll {
        area = area.scroll_offset(at);
    }
    let mut cursor_now = None;
    // (The cursor's line and column, each from 1, as the text laid out has
    // them: a row a line, the text not wrapped.)
    let mut place_now = None;
    // (The editor's context menu's Convert Indentation, done next frame.)
    let mut convert = None;
    let shown = area.auto_shrink([false, false]).show(ui, |ui| {
        // The editor fills the room down to the pane under it, however
        // short the script.
        let room = ui.available_height();
        if let Some(d) = page_scroll {
            ui.scroll_with_delta(egui::vec2(0.0, -d));
        }
        ui.horizontal_top(|ui| {
            // (The gutter and the text side by side, the board's room
            // between them: `script_layout::GUTTER_RIGHT` and the text's
            // frame, nothing more.)
            ui.spacing_mut().item_spacing.x = 0.0;
            // (The editor draws the selection itself, to each line's last
            // character, under its text and the line numbers, with the
            // ground and the current line's band: `script_layout::code_edit`.)
            let out = crate::script_layout::code_edit(ui, key, buf, cached, &palette, room, reveal);
            // (A right click in the selection keeps it, for the menu's Cut,
            // Copy and Convert Indentation of its lines; outside it, the
            // caret goes where it was pressed: `code_edit`.)
            crate::widgets::own_text_menu(&ctx, editor_id(key));
            cursor_now = out.cursor_range;
            place_now = Some((out.cursor_line_col.0 + 1, out.cursor_line_col.1 + 1));
            // Its right-click menu: the editor's Undo and Redo, Cut, Copy,
            // Paste and Select All (the text fields' menu), then Convert
            // Indentation and the plugins' commands for scripts.
            if let Some((undo_keys, redo_keys)) = undo_keys.filter(|_| menu_wanted) {
                // (The editor's own history: each of its steps changes the
                // text, so whether there is one is all the menu asks.)
                let (can_undo, can_redo) = (buf.undo.can_step(false), buf.undo.can_step(true));
                let undo = crate::widgets::TextUndo { can_undo, can_redo, undo_keys, redo_keys };
                let (step, _) =
                    crate::widgets::text_menu(&ctx, editor_id(key), Some(undo), |ui, rows| {
                        mg_uikit::menu_separator(ui);
                        for (tabs, label) in INDENT_CONVERSIONS {
                            if rows.row(ui, label, "", true) {
                                convert = Some(tabs);
                            }
                        }
                        // The plugins' commands for scripts, run on this one.
                        plugin_chosen = crate::plugins::menu_commands(ui, &commands, &script_kinds);
                    });
                if let Some(step) = step {
                    script_tools.undo = Some((key, step == crate::widgets::TextAction::Redo));
                    ctx.request_repaint();
                }
            }
            ctx.data_mut(|d| {
                d.insert_temp(gutter_id(key), (out.gutter_rect.x_range(), out.current_band))
            });
            // The live error's wavy underline on its line, and the error
            // in its tip: in either host (a floating script has no Status
            // Bar of its own).
            // Where its own line has no text (an error at the file's end)
            // or it is in an include, a line near it carries the underline
            // (`underlined_line`).
            let mut error_tip = false;
            if let Some((line, said)) = &live_error {
                let under = match &buf.error_span {
                    Some((hash, of, under))
                        if *hash == h
                            && *of == egui::epaint::util::hash((line, said, &live_include)) =>
                    {
                        *under
                    }
                    _ => {
                        let asked = egui::epaint::util::hash((line, said, &live_include));
                        let include = live_include.as_ref().map(|(n, _)| n.as_str());
                        let under = underlined_line(&buf.text, *line, include).and_then(|l| {
                            let (a, b) = error_span(&buf.text, l)?;
                            let chars = (
                                cached.char_of_byte(&buf.text, a),
                                cached.char_of_byte(&buf.text, b),
                            );
                            Some((l, chars))
                        });
                        buf.error_span = Some((h, asked, under));
                        under
                    }
                };
                if let Some((_, span)) = under {
                    let wave = paint_error_underline(
                        ui,
                        cached,
                        &buf.text,
                        out.text_origin,
                        span,
                        &palette,
                    );
                    if ui.rect_contains_pointer(wave.expand2(egui::vec2(0.0, 2.0))) {
                        let said = said.replace(['\r', '\n'], " ");
                        let tip = match &live_include {
                            Some((name, at)) => {
                                format!("{said}\nIn the include {name}, line {at}.")
                            }
                            None => format!("{said}\nFound as you type."),
                        };
                        out.response.clone().on_hover_text_at_pointer(tip);
                        error_tip = true;
                    }
                }
            }
            // A name's declaration in a tip (5.2.0, Options › Script Editor
            // › Show declarations in tips): over the name under the pointer
            // (the live error's tip first), or over the cursor where the
            // arrow keys put it on one; not while the completion list is
            // open.
            // (Neither while the editor's right-click menu is open or asked
            // for: a tip would stand over its rows, and the menu takes the
            // keys that would close it.)
            let menu_up = menu_wanted || crate::widgets::text_menu_open(&ctx);
            if symbol_tips && script_tools.completion.is_none() {
                let origin = out.text_origin.to_vec2();
                let pointer =
                    (out.response.hover_pos()).filter(|_| !error_tip && !menu_up).and_then(|p| {
                        let slot = egui::Id::new(("name-at-pointer", key));
                        let at = (p - origin).to_vec2();
                        name_at(ui, slot, h, &symbols, cached, &buf.text, &palette, at)
                    });
                if let Some(s) = pointer {
                    let detail =
                        symbol_detail(&ctx, egui::Id::new(("symbol-tip", key)), s, &palette);
                    out.response.clone().on_hover_ui_at_pointer(|ui| {
                        detail_ui(ui, &detail, TIP_MOST);
                    });
                } else if let Some(r) = out.cursor_range
                    && caret_tip(&ctx, key, r.primary.index.into(), menu_up)
                    && let caret = cached.cursor_local_rect(ui, &buf.text, &palette, r.primary)
                    // (Only while the cursor is in sight: not over the
                    // menus or another panel where the text is scrolled.)
                    && ui.clip_rect().contains_rect(caret.translate(origin))
                {
                    let at = caret.center_bottom() - egui::vec2(0.0, 1.0);
                    let slot = egui::Id::new(("name-at-caret", key));
                    let found =
                        name_at(ui, slot, h, &symbols, cached, &buf.text, &palette, at.to_vec2());
                    if let Some(s) = found {
                        let slot = egui::Id::new(("symbol-tip", key));
                        let detail = symbol_detail(&ctx, slot, s, &palette);
                        let id = editor_id(key).with("caret-tip");
                        let anchor = caret.translate(origin);
                        egui::Tooltip::always_open(ctx.clone(), ui.layer_id(), id, anchor)
                            .show(|ui| detail_ui(ui, &detail, TIP_MOST));
                    }
                }
            }
            if let Some(r) = out.cursor_range {
                let rect = cached
                    .cursor_local_rect(ui, &buf.text, &palette, r.primary)
                    .translate(out.text_origin.to_vec2());
                if script_tools.scroll == Some(key) {
                    ui.scroll_to_rect(rect.expand(40.0), Some(egui::Align::Center));
                    script_tools.scroll = None;
                }
                completion_at = Some(rect.left_bottom());
                // Help follows the identifier under the cursor.
                if out.response.changed() || out.response.clicked() {
                    let word = tools::word_at(&buf.text, usize::from(r.primary.index));
                    if let Some(s) = word.and_then(|w| symbols.find_by_name(&w)) {
                        script_tools.help = Some(s.clone());
                        // A double click on it brings its Help forward
                        // (to the front of the panels under the text), as
                        // in Aurora.
                        if out.response.double_clicked() {
                            script_tools.help_forward = Some(key);
                            ctx.request_repaint();
                        }
                    }
                }
                if out.response.changed() {
                    script_tools.completion = None;
                }
                // Ctrl+click goes to the definition.
                if out.response.clicked() && ctx.input(|i| i.modifiers.command) {
                    ctrl_click = Some(r.primary.index);
                }
            }
        });
    });
    // The cursor's line and column (the Status Bar's count), kept with
    // the text's hash and the cursor's place they were found for.
    let at = cursor_now.map_or(0, |r| usize::from(r.primary.index));
    if let Some(place) = place_now
        && buf.place.is_none_or(|(hash, was, _)| hash != h || was != at)
    {
        buf.place = Some((h, at, place));
    }
    let cursor_now = cursor_now.map(|r| {
        let (a, b): (usize, usize) = (r.primary.index.into(), r.secondary.index.into());
        (a.min(b), a.max(b))
    });
    buf.seen = Some((shown.state.offset, cursor_now));
    if let Some(i) = plugin_chosen {
        app.actions.push(crate::plugins::run_on(&commands[i], script.to_vec()));
    }
    if stepped.is_some() || script_tools.keys_back.take_if(|k| *k == key).is_some() {
        ctx.memory_mut(|m| m.request_focus(editor_id(key)));
    }
    if let Some(tabs) = convert {
        script_tools.convert_indent = Some((key, tabs, true));
        ctx.request_repaint();
    }

    // The completion list at the cursor.
    if let (Some(c), Some(pos)) =
        (script_tools.completion.as_mut().filter(|c| c.script == key), completion_at)
    {
        let mut pick = None;
        // As wide as its widest name, measured when the list opens or
        // changes ([`completion_width`]) and set every frame: egui's area
        // keeps the size it had (a list of one short name once made the
        // next list as narrow, its names wrapped a few letters a line).
        let (width, height, at_x) = completion_size(&ctx, ui, key, &c.items, pos.x);
        // The chosen name's declaration and its documentation's first line
        // beside the list (5.2.0, the owner's ruling on Jasperre's ask).
        if let Some(s) = c.items.get(c.selected) {
            let frame = egui::Frame::popup(&ctx.global_style()).total_margin().sum();
            let list = egui::Rect::from_min_size(
                egui::pos2(at_x, pos.y),
                egui::vec2(width + frame.x, height + frame.y),
            );
            let detail =
                symbol_detail(&ctx, egui::Id::new(("completion-detail", key)), s, &palette);
            completion_detail(&ctx, key, list, &detail);
        }
        egui::Area::new(egui::Id::new(("completion", key)))
            .fixed_pos(egui::pos2(at_x, pos.y))
            .order(egui::Order::Foreground)
            .show(&ctx, |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    mg_uikit::list_rows(ui);
                    ui.set_width(width);
                    ui.set_height(height);
                    // (Its height the rows': egui's least height of a
                    // scroll area, 64, made a list of one name as tall as
                    // three.)
                    egui::ScrollArea::vertical()
                        .min_scrolled_height(height)
                        .max_height(height)
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            // (Each row across the list.)
                            let rows = egui::Layout::top_down_justified(egui::Align::LEFT);
                            ui.with_layout(rows, |ui| {
                                for (i, s) in c.items.iter().enumerate() {
                                    // (One line a row: a name with no room is
                                    // cut with an ellipsis, whole in its tip.)
                                    let row = egui::Button::selectable(i == c.selected, &s.name)
                                        .truncate();
                                    let r = ui.add(row);
                                    let r =
                                        match (r.intrinsic_size(), s.signature.contains(&s.name)) {
                                            (Some(size), false)
                                                if size.x > r.rect.width() + 0.5 =>
                                            {
                                                r.on_hover_text(format!(
                                                    "{}\n{}",
                                                    s.name, s.signature
                                                ))
                                            }
                                            _ if s.signature.is_empty() => r.on_hover_text(&s.name),
                                            _ => r.on_hover_text(&s.signature),
                                        };
                                    if i == c.selected {
                                        r.scroll_to_me(None);
                                    }
                                    if r.clicked() {
                                        pick = Some(i);
                                    }
                                }
                            });
                        });
                });
            });
        if let Some(i) = pick {
            let s = c.items[i].clone();
            let start = c.start;
            script_tools.completion = None;
            let text = if s.kind == SymbolKind::Function { format!("{}(", s.name) } else { s.name };
            let at = cursor(&ctx, key).map_or(start, |x| x.0);
            insert(buf, Some(cached), &ctx, key, start, at, &text);
        }
    }

    if let Some(at) = ctrl_click
        && let Some(d) = app.declaration_at(key, usize::from(at))
    {
        app.go_to_declaration(&d);
        ask_help_forward(app, &ctx, key);
    }

    if save_as {
        app.script_save_as = Some((key, String::new()));
    }
    let how = if to_scratch {
        Some(ScriptSave::ToScratch)
    } else if compile {
        Some(ScriptSave::Compile)
    } else {
        save.then_some(ScriptSave::Save)
    };
    if let Some(how) = how {
        save_script(app, key, how);
    }
    follow_panels(app, &ctx, key);
}

/// The completion list's height at most: its rows scroll past it.
const COMPLETION_HEIGHT: f32 = 200.0;

/// The completion list's least width (its rows' room: a short name's
/// list is no narrower).
const COMPLETION_LEAST: f32 = 160.0;

/// The completion list's width and height (its rows' room, inside its
/// frame) and where it stands across (`x`, the cursor's). Its height its
/// rows', at most [`COMPLETION_HEIGHT`]; its width its widest name's, among
/// all its names and not only those in sight (it keeps its width as it
/// scrolls), with the scroll bar's room where it has more rows than fit;
/// at least [`COMPLETION_LEAST`], at most the room to the window's right
/// edge (a longer name is cut). Where even the least has no room there,
/// it stands farther left. Measured once a list (kept with its names'
/// hash), not each frame. Both are set every frame: egui's area would
/// otherwise keep the size of the list before.
fn completion_size(
    ctx: &egui::Context,
    ui: &Ui,
    key: ResKey,
    items: &[crate::script_tools::Symbol],
    x: f32,
) -> (f32, f32, f32) {
    let id = egui::Id::new(("completion-width", key));
    let style = ctx.global_style();
    let font = egui::TextStyle::Button.resolve(&style);
    let asked =
        egui::epaint::util::hash((items.iter().map(|s| &s.name).collect::<Vec<_>>(), &font));
    let (widest, height) = match ctx.data(|d| d.get_temp::<(u64, f32, f32)>(id)) {
        Some((hash, w, h)) if hash == asked => (w, h),
        _ => {
            let color = ui.visuals().text_color();
            let names = ctx.fonts_mut(|f| {
                (items.iter())
                    .map(|s| f.layout_no_wrap(s.name.clone(), font.clone(), color).size().x)
                    .fold(0.0_f32, f32::max)
            });
            // (A row's padding at each side, and the scroll bar's room
            // where its rows don't all fit: decided by their count, not by
            // the bar shown last frame, which would feed back.)
            let mut rows = ui.style().as_ref().clone();
            mg_uikit::row_style(&mut rows);
            let n = items.len() as f32;
            let all = n * rows.spacing.interact_size.y + (n - 1.0) * rows.spacing.item_spacing.y;
            let scrolls = all > COMPLETION_HEIGHT;
            let bar = if scrolls {
                let s = &rows.spacing.scroll;
                s.allocated_width().max(s.bar_width + s.bar_outer_margin)
            } else {
                0.0
            };
            let w = (names + 2.0 * rows.spacing.button_padding.x + bar).ceil();
            let h = all.min(COMPLETION_HEIGHT);
            ctx.data_mut(|d| d.insert_temp(id, (asked, w, h)));
            (w, h)
        }
    };
    let frame = egui::Frame::popup(&style);
    let sides = frame.total_margin().sum().x;
    let edge = ctx.content_rect().right() - 4.0;
    let most = (edge - x - sides).max(COMPLETION_LEAST);
    let width = widest.clamp(COMPLETION_LEAST, most);
    (width, height, x.min(edge - width - sides).max(ctx.content_rect().left()))
}

/// What a name's tip and the completion list's detail show (5.2.0): its
/// declaration in the editor's colors and font, and its documentation's
/// first line (`script_reference::first_doc_line`). Made once a name, kept
/// in the context's data ([`symbol_detail`]).
pub(crate) struct SymbolDetail {
    declaration: LayoutJob,
    first: Option<String>,
    /// The editor's own ground, where it has one, and its comments' color:
    /// the tip and the detail drawn on it, the first line in that color, so
    /// the editor's colors stand on the ground they were chosen for (a dark
    /// editor in the Light theme, review of 5.2.0).
    ground: Option<(Color32, Color32)>,
}

/// [`SymbolDetail`] of `s` in `palette`, kept in the context's data at
/// `slot` while it is the one asked for there: not made again each frame.
fn symbol_detail(
    ctx: &egui::Context,
    slot: egui::Id,
    s: &crate::script_tools::Symbol,
    palette: &Palette,
) -> std::sync::Arc<SymbolDetail> {
    let colors = (palette.colors, palette.size.to_bits(), palette.background);
    let asked = egui::epaint::util::hash((&s.name, &s.signature, &s.doc, colors));
    let kept = ctx.data(|d| d.get_temp::<(u64, std::sync::Arc<SymbolDetail>)>(slot));
    if let Some((was, detail)) = kept
        && was == asked
    {
        return detail;
    }
    let detail = std::sync::Arc::new(SymbolDetail {
        declaration: highlight(&s.signature, palette),
        first: crate::script_reference::first_doc_line(&s.doc),
        ground: palette.background.map(|g| (g, palette.colors[1])),
    });
    ctx.data_mut(|d| d.insert_temp(slot, (asked, detail.clone())));
    detail
}

/// The most a name's tip is across: its declaration wraps there.
const TIP_MOST: f32 = 480.0;
/// The completion list's detail at most across, and at least where it
/// stands beside the list (else under it).
const DETAIL_MOST: f32 = 420.0;
const DETAIL_LEAST: f32 = 200.0;
/// The completion list's detail's name, for a screen reader (a group).
pub(crate) const COMPLETION_DETAIL: &str = "Completion detail";
/// The room between the completion list and its detail.
const DETAIL_GAP: f32 = 4.0;

/// `detail`'s galleys at `width`: the declaration wrapped there, the
/// documentation's line under it in the body's font, quieter.
fn detail_galleys(
    ctx: &egui::Context,
    detail: &SymbolDetail,
    width: f32,
) -> (std::sync::Arc<egui::Galley>, Option<std::sync::Arc<egui::Galley>>) {
    let t = mg_uikit::tokens(ctx);
    let mut job = detail.declaration.clone();
    job.wrap.max_width = width;
    let declaration = ctx.fonts_mut(|f| f.layout_job(job));
    let first = detail.first.as_ref().map(|line| {
        let font = egui::FontId::proportional(mg_uikit::metrics::BODY_SIZE);
        let color = detail.ground.map_or(t.text_2, |(_, comment)| comment);
        ctx.fonts_mut(|f| f.layout(line.clone(), font, color, width))
    });
    (declaration, first)
}

/// A name's tip ([`SymbolDetail`]), at most `most` across.
/// On the editor's own ground where it has one, over the whole of the
/// tip's inside.
fn detail_ui(ui: &mut Ui, detail: &SymbolDetail, most: f32) {
    let (declaration, first) = detail_galleys(ui.ctx(), detail, most);
    let show = |ui: &mut Ui| {
        ui.add(egui::Label::new(declaration));
        if let Some(first) = first {
            ui.add(egui::Label::new(first));
        }
    };
    match detail.ground {
        Some((ground, _)) => {
            // (Under what it shows, the tip's own margin filled too.)
            let under = ui.painter().add(egui::Shape::Noop);
            show(ui);
            let margin = ui.style().spacing.menu_margin;
            let corner = ui.visuals().menu_corner_radius;
            let rect = ui.min_rect() + margin;
            ui.painter().set(under, egui::epaint::RectShape::filled(rect, corner, ground));
        }
        None => show(ui),
    }
}

/// The completion list's detail ([`SymbolDetail`] of the name chosen in
/// it), beside the list `list` (its frame's rect) at its right, else at
/// its left, else under it (over it where there is no room under), held
/// inside the window: from 100% to 300% interface size, the least window
/// too. Where it stands is worked out from its own measures, not from a
/// frame before. Drawn over the text, not taking the pointer.
fn completion_detail(ctx: &egui::Context, key: ResKey, list: egui::Rect, detail: &SymbolDetail) {
    let style = ctx.global_style();
    let mut frame = egui::Frame::popup(&style);
    if let Some((ground, _)) = detail.ground {
        frame = frame.fill(ground);
    }
    let sides = frame.total_margin().sum();
    let screen = ctx.content_rect().shrink(4.0);
    let right = screen.right() - list.right() - DETAIL_GAP;
    let left = list.left() - DETAIL_GAP - screen.left();
    let (side, room) = if right >= DETAIL_LEAST {
        (0, right)
    } else if left >= DETAIL_LEAST {
        (1, left)
    } else {
        (2, screen.width())
    };
    let width = (room.min(DETAIL_MOST) - sides.x).max(1.0);
    let (declaration, first) = detail_galleys(ctx, detail, width);
    // (As wide as its widest line, at most `width`.)
    let inner_w = first.as_ref().map_or(0.0, |f| f.size().x).max(declaration.size().x).min(width);
    let gap = style.spacing.item_spacing.y;
    let inner_h = declaration.size().y + first.as_ref().map_or(0.0, |f| f.size().y + gap);
    let size = egui::vec2(inner_w, inner_h) + sides;
    let mut at = match side {
        0 => egui::pos2(list.right() + DETAIL_GAP, list.top()),
        1 => egui::pos2(list.left() - DETAIL_GAP - size.x, list.top()),
        _ => {
            let x = list.left().min(screen.right() - size.x).max(screen.left());
            match list.bottom() + DETAIL_GAP + size.y <= screen.bottom() {
                true => egui::pos2(x, list.bottom() + DETAIL_GAP),
                false => egui::pos2(x, list.top() - DETAIL_GAP - size.y),
            }
        }
    };
    // (Held inside the window down too: a tall list leaves no room above
    // or below it in a small one, and the detail then covers the list's
    // end rather than leave the window.)
    at.y = at.y.min(screen.bottom() - size.y).max(screen.top());
    egui::Area::new(egui::Id::new(("completion-detail-area", key)))
        .fixed_pos(at)
        .order(egui::Order::Foreground)
        .interactable(false)
        .show(ctx, |ui| {
            frame.show(ui, |ui| {
                mg_uikit::name_group(ui, COMPLETION_DETAIL);
                ui.set_width(inner_w);
                ui.set_height(inner_h);
                ui.spacing_mut().item_spacing.y = gap;
                ui.add(egui::Label::new(declaration));
                if let Some(first) = first {
                    ui.add(egui::Label::new(first));
                }
            });
        });
}

/// The name of the script's symbols under `at` (in the editor's laid-out
/// text, from its top left): the identifier whose glyphs `at` is over, if one
/// of `symbols` has its name. Found again only where `at` is over
/// another character or the text changed (`h`), kept in the context's data
/// meanwhile (at `slot`): the line is not looked through each frame. (Only
/// the line under `at` is laid out for it: the editor lays out its lines
/// one at a time, `script_layout::LaidOut`.)
#[allow(clippy::too_many_arguments)]
fn name_at<'a>(
    ui: &Ui,
    slot: egui::Id,
    h: u64,
    symbols: &'a crate::script_tools::SymbolSet,
    laid: &mut crate::script_layout::LaidOut,
    text: &str,
    palette: &Palette,
    at: egui::Vec2,
) -> Option<&'a crate::script_tools::Symbol> {
    let row_h = laid.row_height();
    if at.y < 0.0 || at.x < 0.0 {
        return None;
    }
    let line = (at.y / row_h).floor() as usize;
    if line >= laid.line_count() || laid.total_chars() == 0 {
        return None;
    }
    let galley = laid.ensure_line(ui, text, line, palette);
    let column = usize::from(galley.rows.first()?.char_at(at.x));
    // (Kept with the set it was found in, held: review D3's.)
    let asked = (h, line, column);
    // (The symbol's name, and where its glyphs are across its line.)
    type Found = Option<(String, egui::Rangef)>;
    type Kept = ((u64, usize, usize), crate::script_tools::SymbolSet, Found);
    let ctx = ui.ctx();
    let kept = ctx.data(|d| d.get_temp::<Kept>(slot));
    let found = match kept {
        Some((was, set, found)) if was == asked && set.same(symbols) => found,
        _ => {
            let found = find_name(symbols, &galley, column);
            ctx.data_mut(|d| d.insert_temp::<Kept>(slot, (asked, symbols.clone(), found.clone())));
            found
        }
    };
    let (name, across) = found?;
    let top = laid.row_y(line, ctx.pixels_per_point());
    let down = egui::Rangef::new(top, top + row_h);
    (across.contains(at.x) && down.contains(at.y)).then(|| symbols.find_by_name(&name)).flatten()
}

/// [`name_at`]'s search: the identifier at `column` of a line's galley
/// (one row), from its glyphs.
fn find_name(
    symbols: &crate::script_tools::SymbolSet,
    galley: &egui::Galley,
    column: usize,
) -> Option<(String, egui::Rangef)> {
    let row = galley.rows.first()?;
    let glyphs = &row.row.glyphs;
    let column = column.min(glyphs.len());
    let ident = |g: &egui::epaint::text::Glyph| g.chr.is_ascii_alphanumeric() || g.chr == '_';
    let start = glyphs[..column].iter().rposition(|g| !ident(g)).map_or(0, |i| i + 1);
    let end = glyphs[column..].iter().position(|g| !ident(g)).map_or(glyphs.len(), |i| column + i);
    if start >= end || glyphs[start].chr.is_ascii_digit() {
        return None;
    }
    let name: String = glyphs[start..end].iter().map(|g| g.chr).collect();
    symbols.find_by_name(&name)?;
    let x0 = row.pos.x + glyphs[start].pos.x;
    let last = &glyphs[end - 1];
    let x1 = row.pos.x + last.pos.x + last.advance_width;
    Some((name, egui::Rangef::new(x0, x1)))
}

/// Whether the cursor, at `at` (a character index), is where the arrow keys
/// (or Home, End, Page Up and Page Down) put it, nothing typed, clicked or
/// pressed since: the tip over the name there shows then (5.2.0), and goes
/// at the next key, click, text or wheel scroll, or where `kept_off` (the
/// editor's right-click menu open or asked for). Kept in the context's
/// data.
fn caret_tip(ctx: &egui::Context, key: ResKey, at: usize, kept_off: bool) -> bool {
    use egui::Key;
    let slot = egui::Id::new(("caret-tip", key));
    let focused = ctx.memory(|m| m.has_focus(editor_id(key))) && !kept_off;
    let (moved, other) = ctx.input(|i| {
        let mut moved = false;
        let mut other = i.pointer.any_pressed();
        for e in &i.events {
            match e {
                egui::Event::Key { key, pressed: true, .. } => match key {
                    Key::ArrowLeft
                    | Key::ArrowRight
                    | Key::ArrowUp
                    | Key::ArrowDown
                    | Key::Home
                    | Key::End
                    | Key::PageUp
                    | Key::PageDown => moved = true,
                    _ => other = true,
                },
                egui::Event::Text(_)
                | egui::Event::Paste(_)
                | egui::Event::Ime(_)
                | egui::Event::MouseWheel { .. } => other = true,
                _ => {}
            }
        }
        (moved, other)
    });
    let kept = ctx.data(|d| d.get_temp::<usize>(slot));
    let now = match (focused, moved && !other, other) {
        (false, ..) | (true, false, true) => None,
        (true, true, _) => Some(at),
        (true, false, false) => kept.filter(|k| *k == at),
    };
    if now != kept {
        ctx.data_mut(|d| match now {
            Some(at) => {
                d.insert_temp(slot, at);
            }
            None => {
                d.remove::<usize>(slot);
            }
        });
    }
    now.is_some()
}

/// The Reference and Help panels after the script's editor (M4's U12):
/// drawn before it, they are drawn again where it changed what they show
/// (its symbols made anew, a name chosen in Help) or found none to show
/// yet; and Help asked to the front (a double click on a name, Go to
/// Definition) is drawn so at once (`shell::help_forward`: the docks know
/// the editor in front, which the editors' own drawing does not).
/// Help asked to the front by `key`'s editor (Go to Definition): for the
/// docks to bring forward ([`ScriptTools::help_forward`]), drawn again
/// once so that they do.
fn ask_help_forward(app: &mut Moonglow, ctx: &egui::Context, key: ResKey) {
    app.script_tools.help_forward = Some(key);
    ctx.request_repaint();
}

fn follow_panels(app: &mut Moonglow, ctx: &egui::Context, key: ResKey) {
    let made = app.script_tools.symbols.get(&key).map(|(_, s)| s);
    let shown = app.script_tools.help.as_ref().map(|h| h.name.clone());
    let pass = ctx.cumulative_pass_nr();
    // (Only where they were drawn this pass, for this script: one not
    // drawn (behind the other's tab, the dock folded) has nothing to
    // catch up, and asking would draw the window again every frame.)
    let reference = app.script_tools.reference_drawn.as_ref().is_some_and(|(p, k, was, chosen)| {
        let changed = match (was, made) {
            (Some(was), Some(made)) => !was.same(made),
            (was, made) => was.is_some() != made.is_some(),
        };
        *p == pass && *k == key && (changed || *chosen != shown)
    });
    let help = (app.script_tools.help_drawn.as_ref())
        .is_some_and(|(p, name)| *p == pass && *name != shown);
    if reference || help {
        ctx.request_repaint();
    }
}

/// The script editor's header's name (`mg_uikit::header`; a screen
/// reader's and the tests' group).
pub const SCRIPT_HEADER: &str = "Script header";

/// What the header asked for that the editor does ([`header`]).
#[derive(Default)]
struct Asked {
    save: bool,
    compile: bool,
    save_as: bool,
    to_scratch: bool,
    find: bool,
    replace: bool,
    bookmark: bool,
    definition: bool,
    references: bool,
    rename: bool,
    /// OK or Cancel, at the header's end.
    pressed: Option<crate::blueprint::Pressed>,
}

/// The script editor's header (M4's U12, the Script board's): Save and
/// Compile with their words, the first commands (into More last); Save
/// As…, Print…, Open… and External Editor as icons; Find… and Replace…, a
/// group of their own; Bookmark; Definition, References, Rename Symbol…
/// and Used By; To Scratch always in More; at the right end the codepage
/// the script is read and saved in, a line, and More; then OK and Cancel
/// in the end slot (the proposal's ruling 10 of section 15.2: every tab
/// with OK and Cancel keeps them at the header's right end), which never
/// yields. What has no room loses its words, then goes into More (the
/// left group first, then the codepage; `mg_uikit::header`). It does
/// Print…, Open…, External Editor and Used By itself; the rest it asks
/// the editor to do.
fn header(app: &mut Moonglow, ui: &mut Ui, key: ResKey, dirty: bool) -> Asked {
    use crate::icons::editor as icon;
    use crate::keys::Cmd;
    use mg_uikit::header::{EditorHeader, Item};
    let ctx = ui.ctx().clone();
    let keymap = &app.keymap;
    let tip = |text: &str, cmd: Cmd| keymap.titled(text, cmd, &ctx);
    let external = app.settings.external_editor.clone();
    let external_tip = match external {
        Some(_) => "Edit the script in the external script editor",
        None => "Choose an external script editor in Options › Script Editor",
    };
    let scratch = crate::transfer::scratch_tip(
        app.settings.scratch_dir.as_deref(),
        "the script, saved, and its compiled script",
    );
    // The codepage the script is read and saved in, quiet, at the right
    // end (named by its words, as before the header).
    let codepage = crate::text::codepage_name(app.scripts[&key].codepage);
    let codepage_tip = "The codepage the script is read and saved in, set by Options › Language";
    let small = egui::FontId::proportional(mg_uikit::metrics::SMALL_SIZE);
    let words = ui.painter().layout_no_wrap(codepage.to_string(), small.clone(), Color32::WHITE);
    let width = words.size().x.ceil() + 2.0;
    let t3 = mg_uikit::tokens(&ctx).text_3;
    let shown = {
        let codepage = codepage.clone();
        move |ui: &mut Ui| {
            let words = egui::RichText::new(codepage).font(small.clone()).color(t3);
            ui.label(words).on_hover_text(codepage_tip);
        }
    };
    let shown_more = shown.clone();
    let find_tip = match keymap.label(Cmd::FindNext, &ctx) {
        next if next.is_empty() => tip("Find text in the script", Cmd::Find),
        next => format!("{}, the next match ({next})", tip("Find text in the script", Cmd::Find)),
    };
    let name = codepage;
    let header = EditorHeader::new(SCRIPT_HEADER)
        .left(
            Item::button(icon::SAVE, "Save")
                .primary()
                .enabled(dirty)
                .tip("Save the script into the module"),
        )
        .left(
            Item::button(icon::COMPILE, "Compile")
                .primary()
                .tip("Save and compile the script (F7 compiles all scripts)"),
        )
        .left(Item::icon(icon::SAVE_AS, "Save As…").tip("Save the script under another name"))
        .left(Item::icon(icon::PRINT, "Print…").tip("Print the script through the web browser"))
        .left(
            Item::icon(icon::OPEN, "Open…")
                .tip("Open another script by name: the module's, its haks' or the game's"),
        )
        .left(
            Item::icon(icon::EXTERNAL, "External Editor")
                .enabled(external.is_some())
                .tip(external_tip),
        )
        .left(Item::separator())
        .left(Item::icon(icon::FIND, "Find…").tip(find_tip))
        .left(
            Item::icon(icon::REPLACE, "Replace…")
                .tip(tip("Replace text in the script", Cmd::Replace)),
        )
        .left(Item::separator())
        .left(
            Item::icon(icon::BOOKMARK, "Bookmark")
                .tip(tip("Toggle a bookmark on the cursor's line", Cmd::Bookmark)),
        )
        .left(Item::separator())
        .left(Item::icon(icon::DEFINITION, "Definition").tip(format!(
            "{}, or Ctrl+click",
            tip("Go to the definition of the name at the cursor", Cmd::Definition)
        )))
        .left(Item::icon(icon::REFERENCES, "References").tip(tip(
            "Where the name at the cursor is used in the module's scripts",
            Cmd::References,
        )))
        .left(
            Item::icon(icon::RENAME, "Rename Symbol…")
                .tip(tip("Rename the name at the cursor everywhere it's used", Cmd::RenameSymbol)),
        )
        .left(
            Item::icon(icon::USED_BY, "Used By")
                .tip("Where the module runs or includes this script"),
        )
        .left(Item::icon(crate::icons::TO_SCRATCH, "To Scratch").tip(scratch).in_more())
        .right(Item::custom(name, width, shown).more(width, shown_more))
        .right(Item::separator());
    let out =
        crate::widgets::header_with_ok_cancel(ui, header, None, crate::side_doc::OK_CANCEL_TIPS);
    if out.clicked("Print…") {
        let text = app.scripts[&key].text.clone();
        let html = print_html(&format!("{}.nss", key.resref), &text, &app.settings.script_style);
        // (In the run's own folder, not a name anyone could guess in the
        // system's temporary folder.)
        let dir = match &app.print_dir {
            Some(dir) => Ok(dir.clone()),
            None => mg_module::build::own_scratch_dir().map(|d| d.join("print")),
        };
        let written = dir.and_then(|dir| {
            let path = dir.join(format!("{}.html", key.resref));
            std::fs::create_dir_all(&dir).and_then(|()| std::fs::write(&path, html))?;
            Ok(path)
        });
        match written {
            Ok(path) => ctx.open_url(egui::OpenUrl::new_tab(file_url(&path))),
            Err(e) => app.log.error(format!("Print: {e}")),
        }
    }
    if out.clicked("Open…") {
        open_script_window(app);
    }
    if out.clicked("External Editor")
        && let Some(editor) = external
    {
        open_external(app, key, &editor);
    }
    if out.clicked("Used By") {
        app.actions.push(Action::FindReferences(key));
    }
    Asked {
        save: out.clicked("Save"),
        compile: out.clicked("Compile"),
        save_as: out.clicked("Save As…"),
        to_scratch: out.clicked("To Scratch"),
        find: out.clicked("Find…"),
        replace: out.clicked("Replace…"),
        bookmark: out.clicked("Bookmark"),
        definition: out.clicked("Definition"),
        references: out.clicked("References"),
        rename: out.clicked("Rename Symbol…"),
        pressed: out.end,
    }
}

/// What a script editor's button (or the Save key) does with the script.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ScriptSave {
    /// Save: the text into the module, compiled with Automatically
    /// Compile Scripts on Save.
    Save,
    /// Compile: saved and compiled.
    Compile,
    /// To Scratch: saved and compiled, then copied into the scratch
    /// folder (the source whatever the compiler says).
    ToScratch,
}

/// A script editor's Save, Compile or To Scratch: its text into the module
/// (an Undo step; the module's file is written by File › Save), compiled as
/// `how` and the options say. A Save with nothing unsaved does nothing (the
/// header's is off then).
pub(crate) fn save_script(app: &mut Moonglow, key: ResKey, how: ScriptSave) {
    let (save, compile, to_scratch) =
        (how == ScriptSave::Save, how == ScriptSave::Compile, how == ScriptSave::ToScratch);
    let Some(buf) = app.scripts.get_mut(&key) else { return };
    if save && !buf.is_dirty() {
        return;
    }
    {
        let text = buf.text.clone();
        // Saved alone (compiling on save is off): the Compiler tab says
        // so, in place of what it said before, or a builder takes the last
        // compile's "compiled" for this text's (Jasperre's report).
        if save && !app.settings.auto_compile {
            app.script_tools.saved_not_compiled(key);
        }
        if buf.is_dirty() {
            let (data, warning) = (buf.mark_saved(), buf.unwritable(key));
            app.actions.push(Action::Apply(Command::new(
                format!("Edit {key}"),
                vec![Edit::SetResource { key, data: Some(data) }],
            )));
            if let Some(w) = warning {
                app.log.warn(w);
            }
        }
        // Options > Script Editor: saving compiles too. (Whether it
        // compiled, and made a debug file.)
        let made = match compile || to_scratch || app.settings.auto_compile {
            true => compile_one(app, key, &text),
            false => None,
        };
        // (After the text is the module's: the scripts that include it.)
        if compile || app.settings.auto_compile {
            app.actions.push(Action::CompileIncluders(key));
        }
        // To Scratch: the source whatever the compiler said (an include
        // file, one with an error, El Grillo's request), its compiled
        // script only if it compiled just now (after the edits above).
        if to_scratch {
            let (compiled, debug) = (made.is_some(), made == Some(true));
            app.actions.push(Action::ScriptToScratch { key, compiled, debug });
        }
    }
}

impl Moonglow {
    /// [`Action::ScriptToScratch`]: the script's source, as saved into the
    /// module, copied into the scratch folder, and with it its compiled
    /// script and debug file if it `compiled` just now; else not (a
    /// compiled script the module has from before is older than the
    /// source). The Compiler tab and the log say what was copied.
    pub(crate) fn script_to_scratch(&mut self, key: ResKey, compiled: bool, debug: bool) {
        let Some(ws) = &mut self.ws else { return };
        if let Err(e) = ws.flush() {
            self.log.error(e.to_string());
            return;
        }
        // (A debug file only if this compile made one: not the module's
        // from an older compile with debug information.)
        let mut files = vec![key];
        if compiled {
            files.push(ResKey::new(key.resref, ResType::NCS));
            if debug {
                files.push(ResKey::new(key.resref, ResType::NDB));
            }
        }
        let Some(dir) = self.scratch_folder() else { return };
        let Some(ws) = &self.ws else { return };
        let (text, error) = match mg_module::transfer::export_files(&ws.module, &files, &dir) {
            Ok(written) => {
                let names: Vec<String> = written
                    .iter()
                    .filter_map(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
                    .collect();
                let names = crate::transfer::listed(&names);
                let mut said = format!("To Scratch: copied {names} to {}", dir.display());
                let mut error = false;
                if !compiled {
                    said.push_str(", not compiled (no compiled script copied)");
                }
                let (removed, failed) =
                    crate::transfer::remove_stale_compiled(&dir, &[key.resref], &written);
                if !removed.is_empty() {
                    said.push_str(&format!("; {}", crate::transfer::removed_said(&removed)));
                }
                if !failed.is_empty() {
                    said.push_str(&format!("; could not remove {}", failed.join(", ")));
                    error = true;
                }
                (said, error)
            }
            Err(e) => (format!("To Scratch: could not write to {}: {e}", dir.display()), true),
        };
        if error {
            self.log.error(text.clone());
        } else {
            self.log.info(text.clone());
        }
        let tools = &mut self.script_tools;
        tools.messages.push(Message::new(key, text, None, error));
        tools.ask_forward(Some(key), InfoTab::Compiler, error);
    }
}

/// The file of a nasher project or a module folder that holds `key`'s
/// source, where the external editor can work on it in place: its own file, as
/// long as Moonglow has nothing of the script that the file lacks (text
/// typed and not saved, or a change not yet written to the project).
fn project_file(app: &Moonglow, key: ResKey) -> Option<std::path::PathBuf> {
    let ws = app.ws.as_ref()?;
    let path = ws.module.file_of(&key)?.to_path_buf();
    let buf = app.scripts.get(&key)?;
    let on_disk = std::fs::read(&path).ok()?;
    (!buf.is_dirty() && on_disk == buf.bytes()).then_some(path)
}

/// Opens a script in the external editor alone, with no editor of
/// Moonglow's (Options › Script Editor): where the script has a file of
/// its own to edit in place (a nasher project's source, a module folder's
/// file) that Moonglow holds nothing more of, and files changed outside
/// are read again, which is how its saves come back. Whether it was.
pub(crate) fn open_only_external(app: &mut Moonglow, key: ResKey) -> bool {
    let Some(editor) = app.settings.external_editor.clone() else { return false };
    if app.settings.no_auto_reload {
        return false;
    }
    let Some(path) =
        app.ws.as_ref().and_then(|ws| ws.module.file_of(&key)).map(|p| p.to_path_buf())
    else {
        return false;
    };
    // (Open here with something the file lacks: Moonglow's editor has it.
    // So does a script only the module has as yet, made by New Script or
    // Save As and not written by File › Save.)
    if app.scripts.contains_key(&key) && project_file(app, key).is_none() {
        return false;
    }
    let module = app.ws.as_ref().and_then(|ws| ws.module.get(&key));
    if !app.scripts.contains_key(&key) && module != std::fs::read(&path).ok().as_deref() {
        return false;
    }
    match std::process::Command::new(&editor).arg(&path).spawn() {
        Ok(running) => {
            crate::test_module::let_run(running);
            app.log.info(format!("{key}: editing {} in {}", path.display(), editor.display()));
            true
        }
        Err(e) => {
            app.log.error(format!("{}: {e}", editor.display()));
            false
        }
    }
}

/// Opens the script in the external editor: a nasher project's own source
/// file, in place ([`project_file`]); else a scratch copy. The editor's
/// saves come back into the buffer ([`reload_external`]), or, for a
/// project's file, as its files changed outside do (Options › General).
fn open_external(app: &mut Moonglow, key: ResKey, editor: &std::path::Path) {
    let in_place = project_file(app, key);
    let in_project =
        app.ws.as_ref().is_some_and(|ws| ws.module.file_of(&key).is_some()) && in_place.is_none();
    let reloads = !app.settings.no_auto_reload;
    let Some(buf) = app.scripts.get_mut(&key) else { return };
    let path = match in_place {
        Some(path) => path,
        None => {
            // (In the run's own folder: the system's temporary folder's
            // `moonglow-PID` could be made, or linked elsewhere, by another
            // user of the machine first, to read the script or give the
            // editor another.)
            let name = format!("{}.nss", key.resref.to_lowercase());
            let written = mg_module::build::own_scratch_dir().and_then(|own| {
                let dir = own.join("edit");
                std::fs::create_dir_all(&dir)?;
                std::fs::write(dir.join(&name), buf.bytes())?;
                Ok(dir.join(&name))
            });
            let path = match written {
                Ok(path) => path,
                Err(e) => {
                    app.log.error(format!("{key}: the copy for the external editor: {e}"));
                    return;
                }
            };
            if in_project {
                app.log.info(format!(
                    "{key}: a copy is opened, not the project's file: Moonglow has changes to \
                     it that the file lacks (save first to edit the file in place)"
                ));
            }
            path
        }
    };
    // (A project's file read again as files changed outside are: not by
    // this buffer as well, which would then differ from the module's.)
    if in_place_of(&path) && reloads {
        buf.external = None;
        match std::process::Command::new(editor).arg(&path).spawn() {
            Ok(running) => {
                crate::test_module::let_run(running);
                app.log.info(format!("{key}: editing {} in {}", path.display(), editor.display()));
            }
            Err(e) => app.log.error(format!("{}: {e}", editor.display())),
        }
        return;
    }
    let modified = std::fs::metadata(&path).and_then(|m| m.modified()).ok();
    buf.external = Some((path.clone(), modified));
    match std::process::Command::new(editor).arg(&path).spawn() {
        Ok(running) => {
            crate::test_module::let_run(running);
            app.log.info(format!("{key}: editing in {}", editor.display()));
        }
        Err(e) => app.log.error(format!("{}: {e}", editor.display())),
    }
}

/// Whether `path` is a file of its own (a project's), not Moonglow's
/// scratch copy.
fn in_place_of(path: &std::path::Path) -> bool {
    !mg_module::build::own_scratch_dir().is_ok_and(|d| path.starts_with(d))
}

/// Reads back a script the external editor has saved since.
pub(crate) fn reload_external(app: &mut Moonglow, key: ResKey) {
    let Some(buf) = app.scripts.get_mut(&key) else { return };
    let Some((path, seen)) = &mut buf.external else { return };
    let modified = std::fs::metadata(&*path).and_then(|m| m.modified()).ok();
    if modified.is_none() || modified == *seen {
        return;
    }
    *seen = modified;
    if let Ok(bytes) = std::fs::read(&*path) {
        let text = crate::text::to_editor(&buf.codepage.decode(&bytes)).0;
        if text != buf.text {
            buf.set_text(text);
            app.log.info(format!("{key}: changed in the external editor"));
        }
    }
}

/// Finds the next match of the Find window's text in a script.
/// Opens the script `file` names (a script's name, an include's as a
/// compiler message gives it) at its `line` (from 1): the module's in its
/// editor, brought to the front (a compiler message's click, a search
/// hit's in Search Results); another (the game's) read-only, as a
/// resource.
pub(crate) fn open_line(app: &mut Moonglow, file: &str, line: usize) {
    match tools::nss(file) {
        Some(k) if app.ws.as_ref().is_some_and(|w| w.module.contains(&k)) => {
            let text = app.script_source(&k).unwrap_or_default();
            app.script_tools.jump = Some((k, tools::line_start(&text, line.saturating_sub(1))));
            app.actions.push(Action::OpenTab(Tab::Script(k)));
        }
        Some(k) => app.actions.push(Action::OpenTab(Tab::Resource(k))),
        None => {}
    }
}

pub(crate) fn find_in_script(app: &mut Moonglow, ctx: &egui::Context, key: ResKey) {
    let Some(buf) = app.scripts.get(&key) else { return };
    let s = &app.script_tools.search;
    let from = match cursor(ctx, key) {
        Some((a, _)) if s.options.backwards => a,
        Some((_, b)) => b,
        None => 0,
    };
    match tools::find(&buf.text, &s.find, from, &s.options) {
        Some((a, b)) => {
            set_cursor(ctx, key, a, b);
            app.script_tools.scroll = Some(key);
        }
        None => app.log.info(format!("{:?} not found in {key}", s.find)),
    }
}

/// Compiles a script's source, resolving includes in the module, then the
/// game.
pub(crate) fn compile_source(
    app: &Moonglow,
    key: ResKey,
    text: &str,
) -> Result<Vec<u8>, mg_script::compiler::CompileError> {
    // (What Moonglow writes itself, a wizard's script: by the built-in
    // compiler.)
    compile_with_debug(app, key, text, false, None).map(|(ncs, ..)| ncs)
}

/// A compiled script, its debug information if made, and what the compile
/// warned of (the built-in compiler's).
type Made = (Vec<u8>, Option<Vec<u8>>, Vec<mg_script::CompileWarning>);

/// [`compile_source`], with the debug information (`.ndb`) if `debug`.
fn compile_with_debug(
    app: &Moonglow,
    key: ResKey,
    text: &str,
    debug: bool,
    external: Option<&mg_module::external_compiler::ExternalCompiler>,
) -> Result<Made, mg_script::compiler::CompileError> {
    compile_reading(app, key, text, debug, external).0
}

/// [`compile_with_debug`], and the module's other scripts the compile read
/// (its includes, however indirectly; [`sources`]).
fn compile_reading(
    app: &Moonglow,
    key: ResKey,
    text: &str,
    debug: bool,
    external: Option<&mg_module::external_compiler::ExternalCompiler>,
) -> (Result<Made, mg_script::compiler::CompileError>, Vec<ResKey>) {
    // (An editor's text in the codepage it was read in; other text in the
    // script codepage. A string's bytes go into the compiled script as
    // they are.)
    let codepage = (app.scripts.get(&key).filter(|b| b.text == text))
        .map_or_else(script_codepage, |b| b.codepage);
    let source = codepage.encode_lossy(text);
    let module = app.ws.as_ref().map(|w| &w.module);
    let resman = app.game.as_deref().map(|g| &g.resman);
    let name = key.resref.to_lowercase().to_string();
    // By the external compiler, when one is chosen (Options › Script
    // Editor): this script as in the editor, and the module's scripts it
    // includes (however indirectly) as they are; not the others.
    if let (Some(external), Some(module)) = (external, module) {
        // (The game's includes are followed to the module's they include.)
        let no_game = mg_resman::ResMan::new();
        let game = resman.unwrap_or(&no_game);
        let sources =
            mg_module::build::external_sources(module, game, &[key], Some((key, &source)));
        let read = (sources.iter())
            .filter_map(|(n, _)| ResKey::parse(n, ResType::NSS))
            .filter(|k| *k != key)
            .collect();
        let failed = |message: String| mg_script::compiler::CompileError { code: -1, message };
        let made = match external.compile(&sources, std::slice::from_ref(&name)) {
            Ok(made) => made,
            Err(e) => return (Err(failed(format!("External compiler: {e}"))), read),
        };
        let Some(made) = made.into_iter().next() else {
            return (Err(failed(format!("{name}.nss: the external compiler made nothing"))), read);
        };
        let made = match made.ncs {
            Some(ncs) => Ok((ncs, made.ndb.filter(|_| debug), Vec::new())),
            None => Err(failed(made.message)),
        };
        return (made, read);
    }
    let read = std::cell::RefCell::new(Vec::new());
    let mut c = Compiler::new(sources(module, resman, &name, &source, Some(&read)));
    c.set_debug_output(debug);
    let made = c.compile(&name).map(|out| (out.ncs, out.ndb, out.warnings));
    drop(c);
    (made, read.into_inner())
}

/// What a compile reads (the built-in compiler's resolver): the script
/// compiled, `name`, as `source` (its editor's text, or the module's), and
/// every other script it includes as the module has it, else the game.
/// Never an open editor's unsaved text: the compiled script goes into the
/// module beside the sources it was made from, which the build, an export
/// and the test loop compile again from (as Aurora reads an include
/// saved). Compile and the check as you type alike. The module's scripts
/// read go into `read`.
pub(crate) fn sources<'a>(
    module: Option<&'a mg_module::Module>,
    resman: Option<&'a mg_resman::ResMan>,
    name: &'a str,
    source: &'a [u8],
    read: Option<&'a std::cell::RefCell<Vec<ResKey>>>,
) -> impl FnMut(&str, ResType) -> Option<Vec<u8>> + 'a {
    move |n: &str, t: ResType| {
        if t == ResType::NSS && n.eq_ignore_ascii_case(name) {
            return Some(source.to_vec());
        }
        let k = ResKey::parse(n, t)?;
        if let Some(own) = module.and_then(|m| m.get(&k)) {
            if let Some(read) = read {
                read.borrow_mut().push(k);
            }
            return Some(own.to_vec());
        }
        resman?.get(&k).ok().map(|d| d.into_owned())
    }
}

/// Compiles again, into the module, those of `scripts` whose compiled
/// script isn't what their source compiles to now (changed since, or an
/// include of theirs was), so that what goes out with them is current.
/// Returns the names of those compiled again, and of those that have a
/// compiled script but no longer compile (theirs is older than the
/// source). Without the game's data nothing can be compiled, and nothing
/// is said.
pub(crate) fn compile_stale(app: &mut Moonglow, scripts: &[ResKey]) -> (Vec<String>, Vec<String>) {
    compile_stale_into(app, scripts, false)
}

/// [`compile_stale`], the compiled scripts put in as what is made from the
/// module (`made`: outside the edit history, not unsaved work, as
/// [`mg_edit::Workspace::derive`] puts them) or as a command.
pub(crate) fn compile_stale_into(
    app: &mut Moonglow,
    scripts: &[ResKey],
    derived: bool,
) -> (Vec<String>, Vec<String>) {
    let mut c = Compiled::default();
    compile_edits(app, scripts, &mut c);
    put_compiled(app, c, derived)
}

/// What [`compile_edits`] made of scripts, to be put in the module
/// ([`put_compiled`]): perhaps a few at a time, over several frames.
#[derive(Debug, Default)]
pub(crate) struct Compiled {
    /// Those compiled again, and those that have a compiled script but no
    /// longer compile.
    pub(crate) compiled: Vec<String>,
    pub(crate) broken: Vec<String>,
    /// Their compiled scripts (and debug files) that changed.
    pub(crate) edits: Vec<Edit>,
    /// Each script's compiled script, changed or not (for Compile All).
    made: Vec<(ResKey, Vec<u8>)>,
    /// The compiler's warnings, each once.
    warned: Vec<String>,
}

impl Compiled {
    /// Leaves out what was made of these scripts (changed since they were
    /// compiled).
    pub(crate) fn drop_scripts(&mut self, gone: &[ResKey]) {
        let of = |k: &ResKey| gone.iter().any(|g| g.resref == k.resref);
        self.edits.retain(|e| !matches!(e, Edit::SetResource { key, .. } if of(key)));
        self.made.retain(|(k, _)| !of(k));
        let named = |n: &String| gone.iter().any(|g| g.resref.to_string() == *n);
        self.compiled.retain(|n| !named(n));
        self.broken.retain(|n| !named(n));
    }
}

/// Compiles `scripts` (those whose compiled script isn't what their
/// source compiles to now), adding what they make to `c`.
pub(crate) fn compile_edits(app: &mut Moonglow, scripts: &[ResKey], c: &mut Compiled) {
    if app.game.is_none() {
        return;
    }
    let external = app.external_compiler();
    for &key in scripts {
        let Some(ws) = app.ws.as_ref() else { break };
        let Some(text) = ws.module.get(&key).map(decode) else { continue };
        let ncs = ResKey::new(key.resref, ResType::NCS);
        let old = ws.module.get(&ncs).map(<[u8]>::to_vec);
        let result =
            compile_with_debug(app, key, &text, app.settings.debug_info, external.as_ref());
        // (Each warning once: an include's is said by all its includers.)
        if let Ok((.., said)) = &result {
            for w in said {
                if !c.warned.contains(&w.message) {
                    c.warned.push(w.message.clone());
                }
            }
        }
        match result {
            Ok((new, ndb, _)) if old.as_deref() != Some(&new[..]) => {
                c.made.push((key, new.clone()));
                c.edits.push(Edit::SetResource { key: ncs, data: Some(new) });
                if let Some(ndb) = ndb {
                    let key = ResKey::new(key.resref, ResType::NDB);
                    c.edits.push(Edit::SetResource { key, data: Some(ndb) });
                }
                c.compiled.push(key.resref.to_string());
            }
            Ok((same, ..)) => c.made.push((key, same)),
            // (One without a compiled script is an include, or is named
            // by the caller as not compiled.)
            Err(_) if old.is_some() => c.broken.push(key.resref.to_string()),
            Err(_) => {}
        }
    }
}

/// Puts what [`compile_edits`] made in the module: as what is made from
/// it (`derived`: outside the edit history, not unsaved work) or as one
/// command. Returns the names of those compiled again and of those that
/// no longer compile.
pub(crate) fn put_compiled(
    app: &mut Moonglow,
    c: Compiled,
    derived: bool,
) -> (Vec<String>, Vec<String>) {
    let Compiled { compiled, broken, edits, made, warned } = c;
    if derived && !edits.is_empty() {
        let set = |m: &mut mg_module::Module| {
            for e in edits {
                if let Edit::SetResource { key, data: Some(data) } = e {
                    m.set(key, data);
                }
            }
        };
        if let Some(Err(e)) = app.ws.as_mut().map(|ws| ws.derive(set)) {
            app.log.error(e.to_string());
        }
    } else if !edits.is_empty()
        && let Err(e) = app.apply(Command::new("Compile", edits))
    {
        app.log.error(e.to_string());
    }
    // (For Compile All to pass over.)
    app.note_compiled(&made);
    for w in warned {
        app.log.warn(w);
    }
    (compiled, broken)
}

/// How many scripts a save (here or by another program) compiles on the
/// window's thread: more would hold it, and are left to Compile All.
pub(crate) const COMPILED_AT_ONCE: usize = 24;

impl Moonglow {
    /// `scripts` and the module's scripts that include any of them,
    /// however indirectly: what a change to `scripts` leaves with an
    /// older compiled script. `None` where that is more than
    /// [`COMPILED_AT_ONCE`], or more than one for an external compiler:
    /// those are compiled as a job instead, off the window's thread (the
    /// log says so).
    pub(crate) fn with_includers(&mut self, scripts: &[ResKey]) -> Option<Vec<ResKey>> {
        let ws = self.ws.as_mut()?;
        if let Err(e) = ws.flush() {
            self.log.error(e.to_string());
            return None;
        }
        let mut all = scripts.to_vec();
        all.extend(mg_module::refs::includers(&ws.module, scripts));
        // (An external compiler is run once for each script compiled
        // here, with what it includes written out for it each time: more
        // than one go to the job, which runs it once for them all.)
        let slow = self.settings.external_compiler.is_some() && all.len() > 1;
        if all.len() > COMPILED_AT_ONCE || slow {
            self.log.info(format!(
                "{} scripts (those changed and those that include them) are compiled in the \
                 background",
                all.len()
            ));
            self.compile_job(Some(all));
            return None;
        }
        Some(all)
    }

    /// Compiles again the scripts that include `script`, saved or
    /// compiled just now, and names them in the log.
    pub(crate) fn compile_includers(&mut self, script: ResKey) {
        if self.game.is_none() {
            return;
        }
        let Some(mut all) = self.with_includers(&[script]) else { return };
        all.retain(|k| *k != script);
        if all.is_empty() {
            return;
        }
        let (compiled, _) = compile_stale(self, &all);
        if !compiled.is_empty() {
            self.log.info(format!(
                "Compiled, as they include {}: {}",
                script.resref,
                crate::transfer::listed(&compiled)
            ));
        }
        let failed: Vec<String> = all
            .iter()
            .filter(|k| !compiled.contains(&k.resref.to_string()))
            .filter(|k| self.script_fails(**k))
            .map(|k| k.resref.to_string())
            .collect();
        if !failed.is_empty() {
            self.log.error(format!(
                "Did not compile (Compile in the script's editor says why): {}",
                crate::transfer::listed(&failed)
            ));
        }
    }

    /// Whether a script of the module that should compile (it has a
    /// `main` or a `StartingConditional`) doesn't.
    pub(crate) fn script_fails(&mut self, key: ResKey) -> bool {
        self.script_failure(key).is_some()
    }

    /// Why a script of the module that should compile doesn't
    /// ([`Moonglow::script_fails`]): the compiler's error.
    pub(crate) fn script_failure(
        &mut self,
        key: ResKey,
    ) -> Option<mg_script::compiler::CompileError> {
        let text = self.ws.as_ref().and_then(|ws| ws.module.get(&key).map(decode))?;
        if self.game.is_none() || !mg_script::outline::has_entry_point(&encode(&text)) {
            return None;
        }
        let external = self.external_compiler();
        compile_with_debug(self, key, &text, false, external.as_ref()).err()
    }

    /// A script that failed to compile on saving the module: its error in
    /// Compiler, asked to the front, opening the Bottom Dock where the
    /// script is the docked one in front (M4's U13 review).
    pub(crate) fn note_failure(&mut self, key: ResKey, e: mg_script::compiler::CompileError) {
        let t = &mut self.script_tools;
        t.messages.retain(|m| m.script != key);
        let location = e.location();
        t.messages.push(Message::new(key, e.message, location, true));
        t.ask_forward(Some(key), InfoTab::Compiler, true);
    }
}

/// Compiles one script (its text as in the editor) into the module;
/// whether it compiled (`Some`), and made a debug file (`Some(true)`).
fn compile_one(app: &mut Moonglow, key: ResKey, text: &str) -> Option<bool> {
    app.ws.as_ref()?;
    app.script_tools.messages.retain(|m| m.script != key);
    app.script_tools.ask_forward(Some(key), InfoTab::Compiler, false);
    // An include file (no `main`, no `StartingConditional`) isn't compiled
    // on its own: it is compiled into the scripts that include it, and
    // may lean on what they bring. (Aurora's build compiles it too.)
    if !mg_script::outline::has_entry_point(&encode(text)) {
        let said = format!("{key}: an include file (no main or StartingConditional), not compiled");
        app.log.info(said.clone());
        app.script_tools.messages.push(Message::new(key, said, None, false));
        return None;
    }
    let external = app.external_compiler();
    let (result, read) =
        compile_reading(app, key, text, app.settings.debug_info, external.as_ref());
    // Its includes as the module has them: one open with changes not saved
    // is said, or the check as you type and Compile seem to disagree.
    let unsaved: Vec<String> = (read.iter())
        .filter(|k| app.scripts.get(k).is_some_and(ScriptBuffer::is_dirty))
        .map(|k| k.resref.to_string())
        .collect();
    if !unsaved.is_empty() {
        let said = format!(
            "{key}: {} read as saved, not with the changes in {} (save to compile with them)",
            crate::transfer::listed(&unsaved),
            if unsaved.len() == 1 { "its editor" } else { "their editors" },
        );
        app.log.info(said.clone());
        app.script_tools.messages.push(Message::new(key, said, None, false));
    }
    match result {
        Ok((out, ndb, warnings)) => {
            let ncs = ResKey::new(key.resref, ResType::NCS);
            let made = vec![(key, out.clone())];
            let mut edits = vec![Edit::SetResource { key: ncs, data: Some(out) }];
            let debug = ndb.is_some();
            if let Some(ndb) = ndb {
                edits.push(Edit::SetResource {
                    key: ResKey::new(key.resref, ResType::NDB),
                    data: Some(ndb),
                });
            }
            app.actions.push(Action::Apply(Command::new(format!("Compile {key}"), edits)));
            // (Once its text and compiled script are the module's.)
            app.actions.push(Action::NoteCompiled(made));
            app.log.info(format!("{key}: compiled"));
            let said = format!("{key}: compiled");
            app.script_tools.messages.push(Message::new(key, said, None, false));
            for w in warnings {
                app.log.warn(w.message.clone());
                let location = Some((w.file, w.line));
                app.script_tools.messages.push(Message::new(key, w.message, location, false));
            }
            Some(debug)
        }
        Err(e) => {
            app.log.error(e.message.clone());
            let location = e.location();
            app.script_tools.messages.push(Message::new(key, e.message, location, true));
            // (A failure opens the Bottom Dock on it, where the script is
            // the docked one in front.)
            app.script_tools.ask_forward(Some(key), InfoTab::Compiler, true);
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn light_and_dark_syntax_colors_stay_readable_on_code_select() {
        let luminance = |color: Color32| -> f32 {
            let ch = |u: u8| {
                let s = f32::from(u) / 255.0;
                if s <= 0.04045 { s / 12.92 } else { ((s + 0.055) / 1.055).powf(2.4) }
            };
            0.2126 * ch(color.r()) + 0.7152 * ch(color.g()) + 0.0722 * ch(color.b())
        };
        let contrast = |a: Color32, b: Color32| -> f32 {
            let (l1, l2) = (luminance(a), luminance(b));
            (l1.max(l2) + 0.05) / (l1.min(l2) + 0.05)
        };

        let light = mg_uikit::Tokens::LIGHT;
        let light_colors = Palette::defaults(false, light.syn_text);
        for (i, &fg) in light_colors.iter().enumerate() {
            let on_select = contrast(fg, light.code_select);
            let on_accent = contrast(fg, light.accent_fill);
            let min = match i {
                0 | 2..=5 => 4.5,
                1 | 6 => 3.3,
                _ => 2.6,
            };
            assert!(
                on_select >= min && on_select > on_accent,
                "light syntax[{i}] contrast on code_select={on_select:.2}, accent_fill={on_accent:.2}"
            );
        }
        assert!(contrast(light_colors[0], light.code_select) >= 7.0);

        let dark = mg_uikit::Tokens::DARK;
        let dark_colors = Palette::defaults(true, dark.syn_text);
        assert!(contrast(dark_colors[0], dark.code_select) >= 5.5);
    }

    /// A step of the history that does not fit the text (the text set
    /// from elsewhere, its history kept: its place past the end, inside a
    /// character, or not over what it put there) lets the history go and
    /// changes nothing; it panicked (review K's B1).
    #[test]
    fn a_step_of_another_text_lets_the_history_go() {
        use egui::text::{CCursor, CCursorRange};
        let one = CCursorRange::one(CCursor::new(0));
        for (typed_into, then) in [("int a;\n", "x"), ("int a;\n", "é"), ("int a;", "abcd€x")] {
            for redo in [false, true] {
                let mut buf = ScriptBuffer::read(b"", (0, 0, 0));
                buf.set_text(typed_into.to_owned());
                let end = buf.text.len();
                buf.replace_range_undo(end..end, "xyz", one, one, 0.0, false);
                if redo {
                    buf.undo.step(&mut buf.text, false).expect("taken back");
                }
                // (Set from elsewhere, as the field was assigned.)
                buf.text = then.to_owned();
                assert_eq!(buf.undo.step(&mut buf.text, redo), None, "{then:?} {redo}");
                assert_eq!(buf.text, then);
                assert!(!buf.undo.can_step(false) && !buf.undo.can_step(true), "let go");
            }
        }
    }

    /// A ground of the builder's own takes the colors of its lightness,
    /// whichever theme the window wears; with none, the theme's.
    #[test]
    fn the_script_s_colors_follow_a_ground_of_one_s_own() {
        assert!(ground_is_dark([24, 32, 48]) && !ground_is_dark([250, 250, 240]));
        for theme in mg_uikit::Theme::ALL {
            let ctx = egui::Context::default();
            mg_uikit::apply(&ctx, theme);
            let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
                let dark = ScriptStyle { background: Some([24, 32, 48]), ..Default::default() };
                let p = Palette::for_ui(&dark, ui);
                assert_eq!(p.colors[0], mg_uikit::Tokens::DARK.syn_text, "{theme:?}");
                let light = ScriptStyle { background: Some([250, 250, 240]), ..Default::default() };
                assert_eq!(Palette::for_ui(&light, ui).colors[0], mg_uikit::Tokens::LIGHT.syn_text);
                let own = Palette::for_ui(&ScriptStyle::default(), ui);
                assert_eq!(own.colors[0], ui.visuals().text_color());
            });
            out.textures_delta.clear();
        }
    }

    /// A script's text as every view reads it: its editor's (typing not
    /// yet saved too), else the module's in the scripts' codepage (Polish:
    /// Windows-1250, where 0xB9 is ą), not the game's text codepage, which
    /// the conversation editor's Script Preview read it in.
    #[test]
    fn a_script_s_text_is_its_editor_s_else_the_module_s_in_the_scripts_codepage() {
        use mg_core::Language;
        crate::text::set_edit_language(Language::POLISH);
        let key = ResKey::parse("hello", ResType::NSS).unwrap();
        let mut m = mg_module::Module::new();
        m.set(key, b"// \xb9\n".to_vec());
        let ws = mg_edit::Workspace::new(m);
        let mut scripts = std::collections::HashMap::new();
        let read = |scripts: &_| script_source(scripts, Some(&ws), None, &key);
        assert_eq!(read(&scripts).as_deref(), Some("// ą\n"));
        let mut open = ScriptBuffer::read(b"// \xb9\n", (0, 0, 0));
        open.text.push_str("void main() {}\n");
        scripts.insert(key, open);
        assert_eq!(read(&scripts).as_deref(), Some("// ą\nvoid main() {}\n"), "the editor's");
        crate::text::set_edit_language(Language::ENGLISH);
    }

    /// The keys left for a script's editor not drawn (Find went to it
    /// behind another tab) are let go of when its tab is closed: opened
    /// again later, it took them and scrolled (review S's 1).
    #[test]
    fn the_keys_left_for_a_closed_script_are_let_go_of() {
        let ctx = egui::Context::default();
        let mut app = Moonglow::new(None, Box::new(crate::NoDialogs::default()));
        let (kept, closed) = (
            ResKey::parse("kept", ResType::NSS).unwrap(),
            ResKey::parse("closed", ResType::NSS).unwrap(),
        );
        app.dock.main_surface_mut().push_to_first_leaf(Tab::Script(kept));
        set_cursor(&ctx, kept, 0, 0);
        set_cursor(&ctx, closed, 0, 0);
        forget_closed(&mut app, &ctx);
        assert!(!take_focus_left(&ctx, closed), "its tab closed");
        assert!(take_focus_left(&ctx, kept), "its tab open");
        assert!(!take_focus_left(&ctx, kept), "taken once");
    }

    /// A CRLF script (BioWare's, the new scripts') is edited with plain
    /// newlines, no `\r` for the cursor to stop on at each line's end
    /// (Jasperre's report), and written back as it was: untouched, byte
    /// for byte; edited, with CRLF on every line, the new ones too.
    #[test]
    fn a_crlf_script_is_edited_with_plain_newlines_and_saved_with_crlf() {
        let file = NEW_SCRIPT.as_bytes();
        let mut buf = ScriptBuffer::read(file, (0, 0, 0));
        assert_eq!(buf.text, "void main()\n{\n\n}\n");
        assert!(!buf.is_dirty());
        assert_eq!(buf.bytes(), file, "untouched: the file's bytes");
        buf.text = "void main()\n{\n    int n;\n}\n".into();
        assert!(buf.is_dirty());
        let saved = buf.mark_saved();
        assert_eq!(saved, b"void main()\r\n{\r\n    int n;\r\n}\r\n");
        assert!(!buf.is_dirty());
        assert_eq!(buf.bytes(), saved);
        // A plain-newline script stays one.
        let mut lf = ScriptBuffer::read(b"void main()\n{\n}\n", (0, 0, 0));
        lf.text.insert_str(0, "// x\n");
        assert_eq!(lf.mark_saved(), b"// x\nvoid main()\n{\n}\n");
        // The templates have no trailing spaces either.
        for t in [NEW_SCRIPT, NEW_CONDITION] {
            assert!(t.split("\r\n").all(|l| l.trim_end() == l), "{t:?}");
        }
    }

    /// A selection is drawn to each line's last character: no wider than
    /// the line (the field's own drew a selected line end as more of the
    /// line), with a sliver on an empty line.
    #[test]
    fn a_selection_ends_at_each_line_s_last_character() {
        let ctx = egui::Context::default();
        let mut galley = None;
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
            let font = egui::FontId::monospace(14.0);
            galley = Some(ui.painter().layout_no_wrap(
                "ab;\n\ncdef\n".into(),
                font,
                egui::Color32::WHITE,
            ));
        });
        // (The font's texture is not uploaded anywhere here.)
        out.textures_delta.clear();
        let galley = galley.unwrap();
        let cursor = egui::text::CCursor::new;
        let range = |a, b| egui::text::CCursorRange::two(cursor(a), cursor(b));
        let rows = &galley.rows;
        // From the first character to after "cd": three rows.
        let found = selection_rects(&galley, &range(0, 7));
        assert_eq!(found.len(), 3);
        assert_eq!(found[0].width(), rows[0].row.size.x, "the line, and no more");
        assert_eq!(found[1].width(), 3.0, "an empty line's sliver");
        assert!(found[2].width() > 0.0 && found[2].width() < rows[2].row.size.x);
        assert_eq!(found[1].top(), rows[1].pos.y);
        // Whole lines, to the start of the next: nothing on that one.
        assert_eq!(selection_rects(&galley, &range(0, 5)).len(), 2);
        // Backwards is the same; nothing selected is nothing drawn.
        assert_eq!(selection_rects(&galley, &range(7, 0)), found);
        assert!(selection_rects(&galley, &range(2, 2)).is_empty());
    }

    #[test]
    fn print_pages_are_highlighted_and_escaped() {
        let style = crate::settings::ScriptStyle::default();
        let html = print_html("x<y>.nss", "// a < b\nvoid main() {}", &style);
        assert!(html.contains("<title>x&lt;y&gt;.nss</title>"));
        assert!(html.contains("// a &lt; b"));
        assert!(html.contains("onload=\"window.print()\""));
        assert!(html.matches("<span style=\"color:#").count() > 3, "{html}");
        assert_eq!(file_url(std::path::Path::new("/tmp/a b/c.html")), "file:///tmp/a%20b/c.html");
        assert_eq!(file_url(std::path::Path::new("C:\\x\\y.html")), "file:///C:/x/y.html");
    }

    /// The wavy underline spans what is on the error's line, from its first
    /// character that is not a space to its last, and no other line's;
    /// none on a line of spaces, or past the text's end.
    #[test]
    fn an_error_s_underline_spans_its_own_line() {
        let text = "void main()\n{\n    int n = x;  \n   \n}";
        let at = |line| error_span(text, line).map(|(a, b)| &text[a..b]);
        assert_eq!(at(0), Some("void main()"));
        assert_eq!(at(2), Some("int n = x;"));
        assert_eq!(at(3), None, "spaces alone");
        assert_eq!(at(4), Some("}"));
        assert_eq!(at(5), None, "past the end");
    }

    #[test]
    fn highlighting_keeps_the_text() {
        let src = "void main() { // hi\n  int x = 0x1F; string s = \"é\"; }";
        let style = ScriptStyle::default();
        let job = highlight(src, &Palette::new(&style, true, Color32::WHITE));
        assert_eq!(job.text, src);
        assert!(job.sections.len() > 10);
    }
}

#[cfg(test)]
mod perf {
    /// Timing only: `cargo test --release -p mg-ui --lib highlight_speed -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn highlight_speed() {
        let Some(root) = mg_testkit::nwn_root() else { return };
        let game =
            mg_rules::GameData::open(&mg_resman::GameInstall::new(&root, None, "en")).unwrap();
        let data = game.resman.get_named("nwscript", mg_core::ResType::NSS).unwrap();
        let text = String::from_utf8_lossy(&data).into_owned();
        let t = std::time::Instant::now();
        let palette = super::Palette::new(
            &crate::settings::ScriptStyle::default(),
            true,
            egui::Color32::WHITE,
        );
        let job = super::highlight(&text, &palette);
        println!("highlight: {:?}, {} sections", t.elapsed(), job.sections.len());
    }

    /// Timing only: the symbols of nwscript.nss edited as a script (its
    /// own, and the game's beside them), read again after a key.
    #[test]
    #[ignore]
    fn symbols_speed() {
        let Some(root) = mg_testkit::nwn_root() else { return };
        let game =
            mg_rules::GameData::open(&mg_resman::GameInstall::new(&root, None, "en")).unwrap();
        let data = game.resman.get_named("nwscript", mg_core::ResType::NSS).unwrap();
        let text = String::from_utf8_lossy(&data).into_owned();
        let key = mg_resman::ResKey::parse("big", mg_core::ResType::NSS).unwrap();
        let mut tools = crate::script_tools::ScriptTools::default();
        let spec = || Some(mg_script::spec::Spec::parse(&data));
        let _ = tools.symbols_for(key, 0, false, &text, spec, &mut |_| None);
        for h in 1..4 {
            let t = std::time::Instant::now();
            let o = mg_script::outline::outline(text.as_bytes());
            let outline = t.elapsed();
            let t = std::time::Instant::now();
            let own = crate::script_tools::script_symbols(&text, &mut |_| None);
            let own_took = t.elapsed();
            let t = std::time::Instant::now();
            let all = tools.symbols_for(key, h, false, &text, || None, &mut |_| None);
            println!(
                "outline {outline:?} ({} functions); script_symbols {own_took:?} ({}); \
                 symbols_for {:?} ({})",
                o.functions.len(),
                own.len(),
                t.elapsed(),
                all.len()
            );
        }
    }
}

/// Opens the Open Script window, with every script there is to open: the
/// module's, and what the game would load besides (its haks' and its own).
pub(crate) fn open_script_window(app: &mut Moonglow) {
    use crate::script_tools::{OpenScript, ScriptFrom};
    let mut scripts: Vec<(ResKey, ScriptFrom)> = Vec::new();
    if let Some(ws) = &app.ws {
        scripts.extend(ws.module.keys_of(ResType::NSS).map(|k| (*k, ScriptFrom::Module)));
    }
    if let Some(game) = app.game.as_deref() {
        let layers = game.resman.layers();
        // (The module's, as a set: each of the game's thousands of scripts
        // was looked for among them one by one.)
        let own: std::collections::HashSet<ResKey> = scripts.iter().map(|(k, _)| *k).collect();
        for (key, layer) in game.resman.entries() {
            if key.restype != ResType::NSS || own.contains(&key) {
                continue;
            }
            let label = layers.get(layer).map_or("", |l| l.label.as_str());
            // (The module's own layer: listed already.)
            if label == "module" {
                continue;
            }
            let from = if label.starts_with("hak:") { ScriptFrom::Hak } else { ScriptFrom::Game };
            scripts.push((key, from));
        }
    }
    scripts.sort_by_key(|(k, _)| k.resref.to_string());
    let shown = app.script_tools.open_script.take().map(|o| o.shown).unwrap_or_default();
    app.script_tools.open_script =
        Some(OpenScript { filter: String::new(), shown, scripts, ..Default::default() });
}

/// The Open Script window: a name to find, which scripts to show (as
/// Aurora's Resources to Show), and the list. A click selects a script,
/// Ctrl+click adds one or takes it away, Shift+click selects those
/// between, the arrows, Page Down and Up, Home and End move the selection
/// (with Shift, widen it), Ctrl+A selects every one listed; Enter or Open
/// opens every one selected, each in its tab (with none selected, Enter
/// opens the first listed), and a double click opens that one (El
/// Grillo's request); a right click has a menu. Home, End and Ctrl+A are
/// the name's own while a name is typed in it. Only scripts listed stay
/// selected. The game's and a hak's open to be read only.
fn open_script_ui(app: &mut Moonglow, ctx: &egui::Context) {
    use crate::script_tools::{ScriptFrom, ScriptsShown};
    let Some(mut w) = app.script_tools.open_script.take() else { return };
    let mut open = true;
    let mut chosen: Vec<(ResKey, ScriptFrom)> = Vec::new();
    let footed = crate::widgets::dialog(ctx, "Open Script", &mut open)
        .default_size([420.0, 440.0])
        .collapsible(false)
        .show_foot(ctx, |ui| {
            // Down and Up (Page Down and Up) move the selection among the
            // scripts listed, while the name is typed too; with Shift,
            // they widen it. Home and End too, and Ctrl+A selects all,
            // unless the name has the keyboard with a name in it (they
            // are its own then: to its start and end, all its text).
            let name_id = ui.make_persistent_id("open_script_name");
            let typing = !w.filter.is_empty() && ui.memory(|m| m.has_focus(name_id));
            let (mut moved, mut widen): (isize, bool) = (0, false);
            let far = isize::MAX / 4;
            let mut keys = vec![
                (egui::Key::ArrowDown, 1),
                (egui::Key::ArrowUp, -1),
                (egui::Key::PageDown, 10),
            ];
            keys.push((egui::Key::PageUp, -10));
            if !typing {
                keys.extend([(egui::Key::End, far), (egui::Key::Home, -far)]);
            }
            for (key, by) in keys {
                // (Shift first: a key without it matches one with it.)
                if ui.input_mut(|i| i.consume_key(egui::Modifiers::SHIFT, key)) {
                    moved = moved.saturating_add(by);
                    widen = true;
                } else if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, key)) {
                    moved = moved.saturating_add(by);
                }
            }
            let mut select_all =
                !typing && ui.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::A));
            // (Escape clears the name typed first, then closes.)
            let field = crate::widgets::SearchField::new(name_id, &mut w.filter)
                .hint("Find by name")
                .find_over()
                .show(ui)
                .response;
            crate::widgets::autofocus(ui, &field);
            // (Enter with nothing selected, where Open is off: it opens
            // the first listed. With scripts selected it is Open's, the
            // footer's.)
            let entered = (field.lost_focus() || !field.has_focus())
                && ui.input(|i| i.key_pressed(egui::Key::Enter));
            // (A name typed starts over.)
            if field.changed() {
                w.cursor = 0;
                w.selected.clear();
                w.anchor = None;
            }
            ui.horizontal_wrapped(|ui| {
                for (shown, label) in [
                    (ScriptsShown::All, "All Resources"),
                    (ScriptsShown::Module, "Module Resources Only"),
                    (ScriptsShown::Haks, "Hak Pak Resources Only"),
                ] {
                    ui.radio_choice(&mut w.shown, shown, label);
                }
            });
            // The scripts listed, made again when the name typed or what
            // is shown changes, not every frame: with the game's own they
            // are thousands, and with all of them selected each selected
            // one was looked for among all those listed, every frame.
            let wanted = (w.filter.clone(), w.shown);
            if w.listed_for.as_ref() != Some(&wanted) {
                let needle = mg_core::text::Needle::new(&w.filter);
                w.listed = w
                    .scripts
                    .iter()
                    .filter(|(_, from)| match w.shown {
                        ScriptsShown::All => true,
                        ScriptsShown::Module => *from == ScriptFrom::Module,
                        ScriptsShown::Haks => *from == ScriptFrom::Hak,
                    })
                    .filter(|(k, _)| needle.finds(&k.resref.to_string()))
                    .copied()
                    .collect();
                w.names = w.listed.iter().map(|(k, _)| *k).collect();
                // (Scripts hidden by what is shown, or by the name, are no
                // longer selected: Open would not open them. What is
                // selected from then on is selected among those listed.)
                let among: std::collections::HashSet<ResKey> = w.names.iter().copied().collect();
                w.selected.retain(|k| among.contains(k));
                if w.anchor.is_some_and(|a| !among.contains(&a)) {
                    w.anchor = None;
                }
                w.listed_for = Some(wanted);
            }
            // (Out of the window's state while it is drawn, and back after.)
            let (listed, names) = (std::mem::take(&mut w.listed), std::mem::take(&mut w.names));
            let last = listed.len().saturating_sub(1) as isize;
            let cursor = (w.cursor as isize).saturating_add(moved).clamp(0, last) as usize;
            w.cursor = cursor;
            if moved != 0
                && let Some(k) = names.get(cursor)
            {
                crate::widgets::select_in_list(
                    &mut w.selected,
                    &mut w.anchor,
                    &names,
                    *k,
                    false,
                    widen,
                );
            }
            // (What Open opens, those selected, is gathered when they are
            // opened, counted until then.)
            let picked_count = w.selected.len();
            if entered && picked_count == 0 {
                chosen = listed.get(cursor).copied().into_iter().collect();
            }
            let note = match picked_count {
                0 => format!("{} scripts", listed.len()),
                n => format!("{} scripts, {n} selected", listed.len()),
            };
            let (row, pitch) = mg_uikit::row_metrics(ui);
            let height = ui.available_height().max(3.0 * row);
            let mut list = egui::ScrollArea::vertical().auto_shrink(false).max_height(height);
            if moved != 0 {
                let above = (height / 2.0 - pitch).max(0.0);
                list = list.vertical_scroll_offset((cursor as f32 * pitch - above).max(0.0));
            }
            mg_uikit::show_rows(list, ui, listed.len(), |ui, range| {
                for (at, (key, from)) in range.clone().zip(listed[range].iter().copied()) {
                    ui.horizontal(|ui| {
                        let r = ui
                            .selectable_label(w.selected.contains(&key), key.resref.to_string())
                            .on_hover_text(
                                "Double-click to open. Ctrl+click and Shift+click select more.",
                            );
                        match from {
                            ScriptFrom::Module => {}
                            ScriptFrom::Hak => _ = ui.weak("hak"),
                            ScriptFrom::Game => _ = ui.weak("game"),
                        }
                        if r.clicked() {
                            let (command, shift) =
                                ui.input(|i| (i.modifiers.command, i.modifiers.shift));
                            crate::widgets::select_in_list(
                                &mut w.selected,
                                &mut w.anchor,
                                &names,
                                key,
                                command,
                                shift,
                            );
                            w.cursor = at;
                        }
                        if r.double_clicked() {
                            chosen = vec![(key, from)];
                        }
                        // A right click: on a script not selected, that
                        // one alone first, as a click.
                        if r.secondary_clicked() && !w.selected.contains(&key) {
                            crate::widgets::select_in_list(
                                &mut w.selected,
                                &mut w.anchor,
                                &names,
                                key,
                                false,
                                false,
                            );
                            w.cursor = at;
                        }
                        mg_uikit::context_menu(&r, |ui| {
                            let several = w.selected.len() > 1;
                            let label = match several {
                                true => format!("Open {} Selected", w.selected.len()),
                                false => "Open".to_string(),
                            };
                            if ui.button(label).clicked() {
                                chosen = (listed.iter())
                                    .filter(|(k, _)| w.selected.contains(k) || *k == key)
                                    .copied()
                                    .collect();
                                ui.close();
                            }
                            if ui.button("Select All Listed").clicked() {
                                select_all = true;
                                ui.close();
                            }
                            if ui.button("Copy Name").clicked() {
                                ui.ctx().copy_text(key.resref.to_string());
                                ui.close();
                            }
                        });
                    });
                }
            });
            if select_all {
                w.selected = names.iter().copied().collect();
                w.anchor = names.first().copied();
            }
            (w.listed, w.names) = (listed, names);
            let words = match picked_count {
                n if n > 1 => format!("Open {n}"),
                _ => "Open".to_string(),
            };
            (
                (),
                crate::widgets::Foot::new()
                    .note(note)
                    .main(words, picked_count > 0)
                    .tip("Open the scripts selected, each in its tab (Enter)")
                    .cancel(),
            )
        });
    if let Some(footed) = &footed {
        if footed.pressed.main() {
            chosen = (w.listed.iter()).filter(|(k, _)| w.selected.contains(k)).copied().collect();
        }
        if footed.pressed.cancelled() {
            open = false;
        }
    }
    for (key, from) in &chosen {
        app.actions.push(match from {
            // (As from the module tree: in the external editor too, where
            // Options has scripts open there.)
            ScriptFrom::Module => Action::OpenResource(*key),
            // (Not the module's to change: read as the resource browser
            // shows it.)
            _ => Action::OpenTab(Tab::Resource(*key)),
        });
    }
    if chosen.is_empty() && open {
        app.script_tools.open_script = Some(w);
    }
}

/// The script editor's windows: Find Text / Replace, Open Script, New
/// Script, Save As.
pub(crate) fn windows(app: &mut Moonglow, ui: &mut Ui) {
    let ctx = ui.ctx().clone();
    forget_closed(app, &ctx);
    open_script_ui(app, &ctx);
    if app.script_tools.search.open {
        let mut s = app.script_tools.search.clone();
        let mut find_next = false;
        let mut replace = false;
        let mut replace_all = false;
        let mut find_all = false;
        let mut entered = false;
        let mut f3 = false;
        let keys = &app.keymap;
        let title = if s.replace_mode { "Replace Text" } else { "Find Text" };
        let mut layer = None;
        let mut find_field = None;
        let shown = crate::widgets::dialog(&ctx, title, &mut s.open)
            .collapsible(false)
            .resizable(false)
            .show_foot(&ctx, |ui| {
                layer = Some(ui.layer_id());
                egui::Grid::new("find-grid").num_columns(2).show(ui, |ui| {
                    crate::widgets::field_label(ui, "Find What");
                    let field = ui.add(mg_uikit::text_field(&mut s.find));
                    crate::widgets::autofocus(ui, &field);
                    // Find Next (F3) from the field too, as in Aurora
                    // (Sacha's report): it types nothing.
                    if field.has_focus()
                        && ui.input_mut(|i| keys.consume(i, crate::keys::Cmd::FindNext))
                    {
                        f3 = true;
                    }
                    find_field = Some(field.id);
                    ui.end_row();
                    if s.replace_mode {
                        ui.label("Replace With");
                        ui.add(mg_uikit::text_field(&mut s.replace));
                        ui.end_row();
                    }
                });
                ui.check_box(&mut s.options.match_case, "Match Case");
                ui.check_box(&mut s.options.whole_word, "Match Whole Word Only");
                ui.check_box(&mut s.options.backwards, "Search Backward");
                if !s.replace_mode {
                    if ui
                        .check_box(&mut s.in_open, "Find In Currently Open Scripts")
                        .on_hover_text("Every match in the scripts open now, in Search Results")
                        .changed()
                        && s.in_open
                    {
                        s.in_files = false;
                    }
                    if ui
                        .check_box(&mut s.in_files, "Find In Files (all scripts in the module)")
                        .changed()
                        && s.in_files
                    {
                        s.in_open = false;
                    }
                }
                let all = (s.in_files || s.in_open) && !s.replace_mode;
                // (Enter finds the next, or all.)
                let mut foot = crate::widgets::Foot::new()
                    .main(if all { "Find All" } else { "Find Next" }, true);
                if s.replace_mode {
                    foot = foot.button("Replace", true).button("Replace All", true);
                }
                ((), foot)
            });
        if let Some(shown) = &shown {
            let pressed = &shown.pressed;
            if pressed.main() {
                // (The field keeps the keys for the next Enter.)
                entered = true;
                if let Some(id) = find_field {
                    ctx.memory_mut(|m| m.request_focus(id));
                }
            }
            let all = (s.in_files || s.in_open) && !s.replace_mode;
            find_all = all && entered;
            find_next = (!all && entered) || f3;
            replace = pressed.clicked("Replace");
            replace_all = pressed.clicked("Replace All");
            if pressed.cancelled() {
                s.open = false;
            }
        }
        app.script_tools.search = s.clone();
        // (Open: a script whose tab closed keeps its editor's state, which
        // changed here would be unsaved with no tab to ask about it.)
        let target = s.script.filter(|k| app.script_open(k) && app.scripts.contains_key(k));
        // Whether the window is where the user is at work: the keys in
        // one of its fields, or a click in it (a button of its takes no
        // keys), until the keys or a click go somewhere else. Edit › Undo
        // is then its script's (`undo_target`, the frame after). The click on
        // the menu itself is somewhere else: it is taken when the button is
        // let go, in the frame whose `undo_target` has run and the menu
        // opens, which holds the script while it is open.

        let focused = ctx.memory(|m| m.focused()).and_then(|id| ctx.read_response(id));
        let keys_in = focused.as_ref().map(|r| r.layer_id) == layer && layer.is_some();
        let rect = shown.as_ref().map(|r| r.response.rect);
        let clicked = ctx.input(|i| i.pointer.any_click().then(|| i.pointer.interact_pos()));
        let click_in = clicked.flatten().zip(rect).map(|(at, rect)| rect.contains(at));
        if !s.open {
            app.script_tools.find_keys = None;
        } else if keys_in || click_in == Some(true) {
            app.script_tools.find_keys = target;
        } else if focused.is_some() || click_in == Some(false) {
            app.script_tools.find_keys = None;
        }
        if find_next && let Some(k) = target {
            find_in_script(app, &ctx, k);
        }
        // (Each a step of the editor's own Undo: the text before it is a
        // point to go back to, as before Convert Indentation.)
        if replace && let Some(k) = target {
            // Replace the selection when it is a match, then find the next.
            if let Some((a, b)) = cursor(&ctx, k) {
                let buf = app.scripts.get_mut(&k).expect("open");
                if tools::is_match_at(&buf.text, &s.find, a, b, &s.options) {
                    undo_point(buf, &ctx, k);
                    insert(buf, app.laid_out.get_mut(&k), &ctx, k, a, b, &s.replace);
                }
            }
            find_in_script(app, &ctx, k);
        }
        if replace_all && let Some(k) = target {
            let now = ctx.input(|i| i.time);
            // (Undo gives the cursor back where it was, not at the top:
            // review N3; Redo leaves it there too, within the text.)
            let (a, b) = cursor(&ctx, k).unwrap_or((0, 0));
            let at = |n: usize| egui::text::CCursor::new(n);
            let cur = egui::text::CCursorRange::two(at(a), at(b));
            let buf = app.scripts.get_mut(&k).expect("open");
            let (text, n) = tools::replace_all(&buf.text, &s.find, &s.replace, &s.options);
            if text != buf.text {
                undo_point(buf, &ctx, k);
                let chars = text.chars().count();
                let after = egui::text::CCursorRange::two(at(a.min(chars)), at(b.min(chars)));
                let was = buf.text_hash();
                let (at, old_end, new_end) = buf.replace_diff_undo(text, cur, after, now);
                if let Some(laid) = app.laid_out.get_mut(&k) {
                    let h = buf.text_hash();
                    laid.apply_edit(&buf.text, at, old_end, new_end, was, h);
                }
            }
            app.log.info(format!("Replaced {n} in {k}"));
        }
        if find_all {
            let Some(ws) = &app.ws else { return };
            // (Those with tabs: a closed one keeps its editor's state.)
            let mut keys: Vec<ResKey> = if s.in_open {
                let tabs = app.dock.iter_all_tabs();
                tabs.filter_map(|(_, t)| if let Tab::Script(k) = t { Some(*k) } else { None })
                    .collect()
            } else {
                ws.module.keys_of(ResType::NSS).copied().collect()
            };
            // (A script in two tabs, were one ever, searched once.)
            keys.sort();
            keys.dedup();
            let (hits, found) = crate::search_results::lines_found(app, &keys, &s.find, &s.options);
            let cut = tools::results_cut(found);
            let what = format!("{:?}: {found} lines found{cut}", s.find);
            app.log.info(what.clone());
            app.show_search_results(what, hits, s.script);
        }
    } else {
        app.script_tools.find_keys = None;
    }

    use crate::widgets::{NameWindow, Named};
    if let Some((from, mut name)) = app.script_save_as.clone() {
        let window = NameWindow {
            new: false,
            title: "Save Script As",
            label: "New name (up to 16 characters)",
            restype: ResType::NSS,
            taken: "The module has a script of this name; it will be replaced.",
            replaces: true,
            button: "Save",
        };
        let module = app.ws.as_ref().map(|w| &w.module);
        app.script_save_as = match window.show(&ctx, module, &mut name) {
            Named::Open => Some((from, name)),
            Named::Cancelled => None,
            Named::Chosen(k) => {
                let (data, warning) = match app.scripts.get(&from) {
                    Some(b) => (b.bytes(), b.unwritable(k)),
                    None => (Vec::new(), None),
                };
                app.actions.push(Action::Apply(Command::new(
                    format!("Save {from} as {k}"),
                    vec![Edit::SetResource { key: k, data: Some(data) }],
                )));
                if let Some(w) = warning {
                    app.log.warn(w);
                }
                app.actions.push(Action::OpenResource(k));
                None
            }
        };
    }

    if let Some(mut name) = app.new_script.clone() {
        let window = NameWindow {
            new: true,
            title: "New Script",
            label: "Name (up to 16 characters)",
            restype: ResType::NSS,
            taken: "The module already has a script of this name.",
            replaces: false,
            button: "Create",
        };
        let module = app.ws.as_ref().map(|w| &w.module);
        app.new_script = match window.show(&ctx, module, &mut name) {
            Named::Open => Some(name),
            Named::Cancelled => None,
            Named::Chosen(k) => {
                let data = Some(NEW_SCRIPT.as_bytes().to_vec());
                let cmd = Command::new(
                    format!("New script {k}"),
                    vec![Edit::SetResource { key: k, data }],
                );
                let made = crate::plugin_events::Made::resources("new-script", vec![k]);
                app.actions.push(Action::ApplyMade(cmd, made));
                app.actions.push(Action::OpenResource(k));
                None
            }
        };
    }
}

/// A new script that answers whether a conversation line shows.
pub(crate) const NEW_CONDITION: &str =
    "int StartingConditional()\r\n{\r\n\r\n    return TRUE;\r\n}\r\n";

impl Moonglow {
    /// [`Action::EditScript`].
    pub(crate) fn edit_script(&mut self, name: mg_core::ResRef, condition: bool) {
        let key = ResKey::new(name, ResType::NSS);
        if name.is_empty() {
            return;
        }
        if self.ws.as_ref().is_some_and(|w| w.module.contains(&key)) {
            // (As a script opened from the module tree: in the external
            // editor, where Options has scripts open there.)
            self.actions.push(Action::OpenResource(key));
        } else if self.game.as_deref().is_some_and(|g| g.resman.contains(&key)) {
            self.actions.push(Action::OpenTab(Tab::Resource(key)));
        } else if self.ws.is_some() {
            let text = if condition { NEW_CONDITION } else { NEW_SCRIPT };
            let edit = Edit::SetResource { key, data: Some(text.as_bytes().to_vec()) };
            match self.apply(Command::new(format!("New script {key}"), vec![edit])) {
                Ok(_) => {
                    self.log.info(format!("Script '{name}' not found: created it"));
                    self.actions.push(Action::OpenResource(key));
                }
                Err(e) => self.log.error(e.to_string()),
            }
        }
    }
}

/// A new script, as Aurora starts one.
pub(crate) const NEW_SCRIPT: &str = "void main()\r\n{\r\n\r\n}\r\n";
