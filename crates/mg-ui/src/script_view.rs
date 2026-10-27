//! The script editor: syntax-highlighted text, save, and compile.

use egui::text::{LayoutJob, TextFormat};
use egui::{Color32, FontId, Ui};
use mg_core::ResType;
use mg_edit::{Command, Edit};
use mg_resman::ResKey;
use mg_script::Compiler;
use mg_script::lex::{TokenKind, tokenize};

use crate::script_tools::{
    self as tools, Completion, InfoTab, Message, SideTab, Symbol, SymbolKind,
};
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
    source: (usize, usize),
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
}

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
}

/// The laid-out text of a script editor, reused while the text, width and
/// colours stay the same (a large script takes milliseconds to highlight and
/// lay out).
#[derive(Debug, Clone)]
pub(crate) struct LaidOut {
    text: String,
    wrap: f32,
    palette: Palette,
    /// The line underlined as an error.
    error_line: Option<usize>,
    galley: std::sync::Arc<egui::Galley>,
}

/// The room a side list's row takes beside its name: the scroll bar and
/// the panel's and the row's margins.
const SIDE_MARGINS: f32 = 44.0;

/// The longest name the side lists open wide enough for, characters.
const LONG_NAME: usize = 44;

/// Underlines (as editors mark errors) what is on a 0-based line, from its
/// first character that isn't a space to its last. (The job's sections
/// are cut there: one section can run over several lines, as `append`
/// joins tokens of one color, and the lines around the error were
/// underlined with it.)
fn underline_line(job: &mut LayoutJob, text: &str, line: usize, color: Color32) {
    let start = if line == 0 {
        0
    } else {
        text.match_indices('\n').nth(line - 1).map_or(text.len(), |(i, _)| i + 1)
    };
    let end = text[start..].find('\n').map_or(text.len(), |i| start + i);
    let own = &text[start..end];
    let from = start + (own.len() - own.trim_start().len());
    let to = start + own.trim_end().len();
    if from >= to {
        return;
    }
    let mut sections = Vec::with_capacity(job.sections.len() + 2);
    for s in job.sections.drain(..) {
        let (a, b) = (usize::from(s.byte_range.start), usize::from(s.byte_range.end));
        let mut cuts = vec![a];
        cuts.extend([from, to].into_iter().filter(|&c| c > a && c < b));
        cuts.push(b);
        for pair in cuts.windows(2) {
            let mut piece = s.clone();
            piece.byte_range = egui::text::ByteIndex(pair[0])..egui::text::ByteIndex(pair[1]);
            if pair[0] >= from && pair[1] <= to {
                piece.format.underline = egui::Stroke::new(1.5, color);
            }
            sections.push(piece);
        }
    }
    job.sections = sections;
}

impl ScriptBuffer {
    /// A script's bytes in an editor, read in the script codepage.
    pub(crate) fn read(bytes: &[u8], source: (usize, usize)) -> ScriptBuffer {
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
        }
    }

    /// Takes the file's text as it now is (`raw`, with its line ends) as
    /// both the text and the text saved.
    fn follow(&mut self, raw: String) {
        let text = crate::text::to_editor(&raw).0;
        self.indent = tools::Indent::guess(&text);
        self.text = text.clone();
        self.saved = text;
        self.raw = raw;
    }

    pub(crate) fn is_dirty(&self) -> bool {
        self.text != self.saved
    }

    /// The text as the file has it: with the file's line ends (an
    /// untouched text, the file's own text).
    fn file_text(&self) -> std::borrow::Cow<'_, str> {
        if self.text == self.saved {
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
        bytes
    }

    /// The warning for characters of the text its codepage cannot write.
    pub(crate) fn unwritable(&self, key: ResKey) -> Option<String> {
        crate::text::unwritable_warning(key, self.codepage, &self.text)
    }
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
    /// The theme's colours for each element.
    pub(crate) fn defaults(dark: bool, text: Color32) -> [Color32; 8] {
        let rgb = Color32::from_rgb;
        if dark {
            [
                text,
                rgb(106, 153, 85),
                rgb(197, 134, 192),
                rgb(86, 156, 214),
                rgb(206, 145, 120),
                rgb(206, 145, 120),
                rgb(79, 193, 255),
                Color32::RED,
            ]
        } else {
            [
                text,
                rgb(0, 128, 0),
                rgb(128, 0, 128),
                rgb(0, 0, 200),
                rgb(163, 21, 21),
                rgb(163, 21, 21),
                rgb(0, 112, 193),
                Color32::RED,
            ]
        }
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

    /// For a `Ui`: the app's style and the `Ui`'s theme.
    pub(crate) fn for_ui(style: &ScriptStyle, ui: &Ui) -> Palette {
        Palette::new(style, ui.visuals().dark_mode, ui.visuals().text_color())
    }

    fn font(&self) -> FontId {
        FontId::monospace(self.size)
    }
}

/// The element a token is coloured as.
fn element(kind: TokenKind) -> usize {
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
    let font = palette.font();
    for t in tokenize(text.as_bytes()) {
        let c = palette.colors[element(t.kind)];
        // Tokens end on character boundaries except inside malformed UTF-8,
        // which the text (a String) cannot contain.
        job.append(&text[t.span], 0.0, TextFormat::simple(font.clone(), c));
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
    let job = highlight(text, &Palette::new(style, false, Color32::BLACK));
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

fn editor_id(key: ResKey) -> egui::Id {
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

/// Notes the script Edit › Undo and Redo and the toolbar's are for this
/// frame (`Moonglow::undo_script`): the one whose editor has the keys,
/// as its Ctrl+Z goes, and while a menu is open the one that had them as
/// it was opened (the click on the menu takes the keys from the editor).
/// Drawn before the editor, the toolbar sees the keys where they were
/// before its own click.
pub(crate) fn undo_target(app: &mut Moonglow, ctx: &egui::Context) {
    if app.menu_open.is_none() {
        app.undo_script = with_keys(app, ctx);
    }
    app.undo_script = app.undo_script.filter(|k| app.scripts.contains_key(k));
}

/// Edit › Undo (or, `redo`, Redo) in the script's editor: a step of its
/// own history, as Ctrl+Z takes there (with none, nothing), once the
/// editor is drawn; the editor has the keys again after it.
pub(crate) fn undo(app: &mut Moonglow, ctx: &egui::Context, key: ResKey, redo: bool) {
    app.script_tools.undo = Some((key, redo));
    ctx.request_repaint();
}

/// A step of the editor's own history taken, back or (`redo`) again, as
/// its Ctrl+Z and Ctrl+Y take one (with none, nothing is).
fn step_history(buf: &mut ScriptBuffer, ctx: &egui::Context, key: ResKey, redo: bool) {
    let id = editor_id(key);
    let Some(mut state) = egui::text_edit::TextEditState::load(ctx, id) else { return };
    let end = egui::text::CCursorRange::one(egui::text::CCursor::new(buf.text.chars().count()));
    let now = (state.cursor.char_range().unwrap_or(end), buf.text.clone());
    let mut undoer = state.undoer();
    let Some((range, text)) = (if redo { undoer.redo(&now) } else { undoer.undo(&now) }).cloned()
    else {
        return;
    };
    buf.text = text;
    state.cursor.set_char_range(Some(range));
    state.set_undoer(undoer);
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
    ctx.memory_mut(|m| m.request_focus(id));
}

/// The editor's cursor as a character range (start, end).
fn cursor(ctx: &egui::Context, key: ResKey) -> Option<(usize, usize)> {
    let state = egui::text_edit::TextEditState::load(ctx, editor_id(key))?;
    let r = state.cursor.char_range()?;
    let (a, b) = (r.primary.index, r.secondary.index);
    let (a, b): (usize, usize) = (a.into(), b.into());
    Some((a.min(b), a.max(b)))
}

/// Replaces a character range of the buffer and puts the cursor after it.
fn insert(
    buf: &mut ScriptBuffer,
    ctx: &egui::Context,
    key: ResKey,
    from: usize,
    to: usize,
    text: &str,
) {
    let (bf, bt) = (tools::byte_index(&buf.text, from), tools::byte_index(&buf.text, to));
    buf.text.replace_range(bf..bt, text);
    let at = from + text.chars().count();
    set_cursor(ctx, key, at, at);
}

/// The editor's context menu's conversions (Edit › Convert Indentation
/// to Tabs and to Spaces): to tabs or not, and its name.
const INDENT_CONVERSIONS: [(bool, &str); 2] =
    [(true, "Convert Indentation to Tabs"), (false, "Convert Indentation to Spaces")];

/// Convert Indentation to Tabs (`tabs`) or to Spaces: every line's
/// indentation made so ([`tools::convert_indentation`], a step `width`
/// columns), one step for the editor's Undo; Tab indents so from then on.
/// The cursor stays on its line, at its start.
fn convert_indentation(
    buf: &mut ScriptBuffer,
    ctx: &egui::Context,
    key: ResKey,
    tabs: bool,
    width: usize,
) {
    buf.indent = Some(if tabs { tools::Guess::Tabs } else { tools::Guess::Spaces(width) });
    let converted = tools::convert_indentation(&buf.text, tabs, width);
    if converted == buf.text {
        return;
    }
    let id = editor_id(key);
    let mut state = egui::text_edit::TextEditState::load(ctx, id).unwrap_or_default();
    let before = state.cursor.char_range().unwrap_or_default();
    let line = buf.text.chars().take(before.primary.index.into()).filter(|c| *c == '\n').count();
    let at = egui::text::CCursorRange::one(egui::text::CCursor::new(tools::line_start(
        &converted, line,
    )));
    // (The text before is a point for the editor's Undo to go back to,
    // in one, however soon it is asked: the field makes the text after
    // one itself once it is left as it is.)
    let mut undoer = state.undoer();
    undoer.add_undo(&(before, std::mem::replace(&mut buf.text, converted)));
    state.set_undoer(undoer);
    state.cursor.set_char_range(Some(at));
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
    ui.scope_builder(egui::UiBuilder::new().max_rect(fit), |ui| editor(app, ui, key));
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
    let source = (bytes.as_ptr() as usize, bytes.len());
    app.scripts.entry(key).or_insert_with(|| ScriptBuffer::read(bytes, source));
    if app.script_tools.open_externally.remove(&key)
        && let Some(editor) = app.settings.external_editor.clone()
    {
        open_external(app, key, &editor);
    }
    let Some(ws) = &app.ws else { return };
    let Some(bytes) = ws.module.get(&key) else { return };
    {
        let buf = app.scripts.get_mut(&key).expect("just inserted");
        // (Another language chosen in Options: a script without unsaved
        // changes is read again in its codepage.)
        let codepage = script_codepage();
        let recoded = buf.codepage != codepage && !buf.is_dirty();
        if buf.source != source || recoded {
            buf.source = source;
            if !buf.is_dirty() {
                let module_text = codepage.decode(bytes).into_owned();
                buf.codepage = codepage;
                // Changed underneath (undo, import): follow the module,
                // its line ends too.
                buf.follow(module_text);
            }
        }
    }
    let ctx = ui.ctx().clone();

    // This script edited, or the module changed: what other scripts found
    // in their includes is looked for again (their caches go by their own
    // text alone).
    // (The text's hash, once a frame: the symbols and the live check
    // go by it too, and a script may be some 600 KB.)
    let h = tools::hash(&app.scripts[&key].text);
    {
        let revision = app.ws.as_ref().map_or(0, |w| w.revision());
        let t = &mut app.script_tools;
        let edited = t.seen.insert(key, h).is_some_and(|was| was != h);
        if t.revision != revision {
            t.revision = revision;
            t.symbols.clear();
            app.script_nav.check_again(None);
        } else if edited {
            t.symbols.retain(|k, _| *k == key);
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
        // (A key typed this frame: the symbols of the frame before. The
        // frame after a key, which egui draws, reads them.)
        let typing = ctx.memory(|m| m.has_focus(editor_id(key)))
            && ctx.input(|i| {
                i.events.iter().any(|e| {
                    matches!(e, egui::Event::Text(_) | egui::Event::Key { pressed: true, .. })
                })
            });
        app.script_tools.symbols_for(
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

    let mut save = false;
    let mut compile = false;
    let mut save_as = false;
    let mut to_scratch = false;
    let dirty = app.scripts[&key].is_dirty();
    // Wrapping: a narrow pane keeps its width (the row would widen the
    // editor past the pane, side lists and all).
    ui.horizontal_wrapped(|ui| {
        save = ui
            .add_enabled(dirty, egui::Button::new("Save"))
            .on_hover_text("Save the script into the module")
            .clicked();
        compile = ui
            .button("Compile")
            .on_hover_text("Save and compile (F7 compiles all scripts)")
            .clicked();
        save_as = ui.button("Save As…").clicked();
        if ui.button("Print…").on_hover_text("Print the script (through the web browser)").clicked()
        {
            let text = app.scripts[&key].text.clone();
            let html =
                print_html(&format!("{}.nss", key.resref), &text, &app.settings.script_style);
            let path = app.print_dir.join(format!("{}.html", key.resref));
            match std::fs::create_dir_all(&app.print_dir).and_then(|()| std::fs::write(&path, html))
            {
                Ok(()) => ui.ctx().open_url(egui::OpenUrl::new_tab(file_url(&path))),
                Err(e) => app.log.error(format!("Print: {e}")),
            }
        }
        if ui
            .button("Open…")
            .on_hover_text(
                "Open another script: the module's, its haks' or the game's, found by name",
            )
            .clicked()
        {
            open_script_window(app);
        }
        let editor = app.settings.external_editor.clone();
        if ui
            .add_enabled(editor.is_some(), egui::Button::new("External Editor"))
            .on_hover_text("Edit in the external script editor (Options › Script Editor)")
            .on_disabled_hover_text("Choose an external script editor in Options › Script Editor")
            .clicked()
            && let Some(editor) = editor
        {
            open_external(app, key, &editor);
        }
        let tip = crate::transfer::scratch_tip(
            app.settings.scratch_dir.as_deref(),
            "the script, saved and compiled, with its compiled script if it compiles",
        );
        to_scratch = ui.button("To Scratch").on_hover_text(tip).clicked();
        ui.separator();
        use crate::keys::Cmd;
        let keymap = &app.keymap;
        let ctx = ui.ctx().clone();
        let tip = |text: &str, cmd: Cmd| keymap.titled(text, cmd, &ctx);
        let find_tip = format!("{}; {}", tip("Find", Cmd::Find), tip("Find Next", Cmd::FindNext));
        open_find |= ui.button("Find…").on_hover_text(find_tip).clicked();
        open_replace |= ui.button("Replace…").on_hover_text(tip("Replace", Cmd::Replace)).clicked();
        toggle_bookmark |= ui
            .button("Bookmark")
            .on_hover_text(tip("Toggle a bookmark on the cursor's line", Cmd::Bookmark))
            .clicked();
        ui.separator();
        definition |= ui
            .button("Definition")
            .on_hover_text(format!(
                "{}, or Ctrl+click",
                tip("Go to the definition of the name at the cursor", Cmd::Definition)
            ))
            .clicked();
        references |= ui
            .button("References")
            .on_hover_text(tip(
                "Where the name at the cursor is used in the module's scripts",
                Cmd::References,
            ))
            .clicked();
        rename |= ui
            .button("Rename Symbol…")
            .on_hover_text(tip(
                "Rename the name at the cursor everywhere it's used",
                Cmd::RenameSymbol,
            ))
            .clicked();
        if ui
            .button("Used By")
            .on_hover_text("Where the module runs or includes this script")
            .clicked()
        {
            app.actions.push(Action::FindReferences(key));
        }
        if dirty {
            ui.weak("modified");
        }
        // The codepage the script is read and saved in.
        let codepage = crate::text::codepage_name(app.scripts[&key].codepage);
        ui.weak(codepage).on_hover_text(
            "The codepage the script is read and saved in: the language's in Options › \
             Language (Windows-1250 for Polish), as the game reads a script's text",
        );
    });
    // The error the latest check found, under the toolbar, on a line of
    // its own that is there with no error too: the text does not move as
    // one comes and goes (Jasperre's report). One line, however long the
    // message: the whole of it on hover.
    if !app.settings.no_live_check {
        app.live_check(key, h, &ctx);
        let size = egui::vec2(ui.available_width(), ui.text_style_height(&egui::TextStyle::Body));
        let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
        if let Some((line, text)) = app.live_error(key) {
            let at = line.map_or_else(|| "in an include".into(), |l| format!("line {}", l + 1));
            let said = format!("⚠ {at}: {}", text.replace(['\r', '\n'], " "));
            let label =
                egui::Label::new(egui::RichText::new(&said).color(ui.visuals().error_fg_color))
                    .truncate();
            let row = egui::UiBuilder::new()
                .max_rect(rect)
                .layout(egui::Layout::left_to_right(egui::Align::Center));
            ui.scope_builder(row, |ui| {
                ui.add(label).on_hover_text(format!(
                    "{said}\n\nFound as you type: the script is compiled when typing pauses \
                     (Options › Script Editor)"
                ))
            });
        }
    }
    ui.separator();

    if definition || references || rename {
        let at = cursor(&ctx, key).map_or(0, |c| c.0);
        match app.declaration_at(key, at) {
            Some(d) if definition => app.go_to_declaration(&d),
            Some(d) if references => app.show_references(&d),
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
        app.script_tools.info = InfoTab::Bookmarks;
    }
    if let Some(n) = set_numbered {
        let at = cursor(&ctx, key).map_or(0, |c| c.0);
        let buf = app.scripts.get_mut(&key).expect("open");
        let line = tools::line_of(&buf.text, at);
        // Setting a number again on its line clears it.
        buf.numbered[n] = if buf.numbered[n] == Some(line) { None } else { Some(line) };
        app.script_tools.info = InfoTab::Bookmarks;
    }
    if let Some(line) = go_numbered.and_then(|n| app.scripts[&key].numbered[n]) {
        let at = tools::line_start(&app.scripts[&key].text, line);
        app.script_tools.jump = Some((key, at));
    }
    if complete && let Some((at, _)) = cursor(&ctx, key) {
        let (start, prefix) = tools::word_before(&app.scripts[&key].text, at);
        let items = tools::completions(&symbols, &prefix);
        if !items.is_empty() {
            app.script_tools.completion =
                Some(Completion { script: key, start, items, selected: 0 });
        }
    }

    // Side lists.
    let mut insert_text: Option<String> = None;
    // At most half the tab, so a narrow tab keeps room for the text (and the
    // lists stay inside it).
    let half = (ui.available_width() / 2.0).max(120.0);
    // Wide enough, to begin with, for the longest name the lists have (with
    // the scroll bar and the margins beside it): but for the few constants
    // longer than any function's name (`PLAYER_DEVICE_PROPERTY_…`), which
    // would take half the window; those end in "…" until the panel is
    // widened.
    // (Measured: the longest few, since letters differ in width.)
    let mut longest: Vec<&str> = symbols
        .iter()
        .map(|s| s.name.as_str())
        .filter(|n| n.chars().count() <= LONG_NAME)
        .collect();
    longest.sort_unstable_by_key(|n| std::cmp::Reverse(n.len()));
    let font = egui::TextStyle::Body.resolve(ui.style());
    let widest = longest.iter().take(16).fold(0.0_f32, |widest, name| {
        let text = (*name).to_owned();
        let galley = ui.fonts_mut(|f| f.layout_no_wrap(text, font.clone(), egui::Color32::WHITE));
        widest.max(galley.size().x + SIDE_MARGINS)
    });
    egui::Panel::right(egui::Id::new(("script-side", key)))
        .resizable(true)
        .default_size(widest.max(260.0).min(half))
        .max_size(half)
        .show(ui, |ui| {
            let t = &mut app.script_tools;
            ui.horizontal_wrapped(|ui| {
                for (tab, label) in [
                    (SideTab::Functions, "Functions"),
                    (SideTab::Variables, "Variables"),
                    (SideTab::Constants, "Constants"),
                    (SideTab::Templates, "Templates"),
                ] {
                    ui.selectable_value(&mut t.side, tab, label);
                }
            });
            ui.horizontal(|ui| {
                ui.label("Filter");
                ui.text_edit_singleline(&mut t.filter);
            });
            let filter = mg_core::text::Needle::new(&t.filter);
            let row = ui.text_style_height(&egui::TextStyle::Body) + 4.0;
            // Long names end in "…" in a narrow panel (Help gives them whole).
            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Truncate);
            if t.side == SideTab::Templates {
                let templates = tools::templates(
                    app.install.as_ref(),
                    app.settings.script_templates.as_deref(),
                );
                // (As wide as the panel: the scroll bar stays at its edge.)
                let area = egui::ScrollArea::vertical().auto_shrink([false, false]);
                area.id_salt("templates").show(ui, |ui| {
                    for (name, path) in templates.iter().filter(|(n, _)| filter.finds(n)) {
                        if ui
                            .selectable_label(false, name)
                            .on_hover_text("Double-click to insert")
                            .double_clicked()
                        {
                            insert_text = std::fs::read(path)
                                .ok()
                                .map(|b| crate::text::to_editor(&decode(&b)).0);
                        }
                    }
                });
            } else {
                let kind = match t.side {
                    SideTab::Functions => SymbolKind::Function,
                    SideTab::Variables => SymbolKind::Variable,
                    _ => SymbolKind::Constant,
                };
                let mut list: Vec<&Symbol> =
                    symbols.iter().filter(|s| s.kind == kind && filter.finds(&s.name)).collect();
                list.sort_by(|a, b| a.name.cmp(&b.name));
                let area = egui::ScrollArea::vertical().auto_shrink([false, false]);
                area.id_salt(("symbols", kind)).show_rows(ui, row, list.len(), |ui, range| {
                    // (Each row as wide as the list, so that its
                    // width doesn't change with the names in view.)
                    ui.set_min_width(ui.available_width());
                    for s in &list[range] {
                        let text = if s.custom {
                            egui::RichText::new(&s.name).strong()
                        } else {
                            egui::RichText::new(&s.name)
                        };
                        let r = ui
                            .selectable_label(
                                t.help.as_ref().is_some_and(|h| h.name == s.name),
                                text,
                            )
                            .on_hover_text(&s.signature);
                        if r.clicked() {
                            t.help = Some((*s).clone());
                            t.info = InfoTab::Help;
                        }
                        if r.double_clicked() {
                            insert_text = Some(if s.kind == SymbolKind::Function {
                                format!("{}(", s.name)
                            } else {
                                s.name.clone()
                            });
                        }
                    }
                });
            }
        });

    // Compiler, Help, Bookmarks, Search Results.
    let palette = Palette::for_ui(&app.settings.script_style, ui);
    let mut jump: Option<(ResKey, usize)> = None;
    let mut open_other: Option<(String, usize)> = None;
    // (At most three fifths of the editor: the text keeps its room.)
    let most = (ui.available_height() * 0.6).max(60.0);
    egui::Panel::bottom(egui::Id::new(("script-info", key)))
        .resizable(true)
        .default_size(140.0f32.min(most))
        .max_size(most)
        .show(ui, |ui| {
            let t = &mut app.script_tools;
            ui.horizontal(|ui| {
                for (tab, label) in [
                    (InfoTab::Compiler, "Compiler"),
                    (InfoTab::Help, "Help"),
                    (InfoTab::Bookmarks, "Bookmarks"),
                    (InfoTab::SearchResults, "Search Results"),
                ] {
                    ui.selectable_value(&mut t.info, tab, label);
                }
            });
            // (Help sideways too: its lines are as long as the source's.)
            let area = match t.info {
                InfoTab::Help => egui::ScrollArea::both(),
                _ => egui::ScrollArea::vertical(),
            };
            area.id_salt("script-info-rows").auto_shrink([false, false]).show(ui, |ui| {
                match t.info {
                    InfoTab::Compiler => {
                        for m in t.messages.iter().filter(|m| m.script == key) {
                            let color = if m.error {
                                ui.visuals().error_fg_color
                            } else {
                                ui.visuals().text_color()
                            };
                            // (When it was said, before it: two compiles
                            // in a row read as two.)
                            let text = format!("{} {}", m.at, m.text);
                            let r = ui.add(
                                egui::Label::new(
                                    egui::RichText::new(text).monospace().color(color),
                                )
                                .sense(egui::Sense::click()),
                            );
                            let r = match m.location {
                                Some(_) => r.on_hover_text("Click to go to the line"),
                                None => r,
                            };
                            if r.clicked()
                                && let Some((file, line)) = &m.location
                            {
                                open_other = Some((file.clone(), *line));
                            }
                        }
                    }
                    // As the source has it, in the editor's colors and
                    // font: the comment above, `//`, columns and all
                    // (nwscript.nss lines up its lists), then the
                    // declaration. Not wrapped (the pane scrolls).
                    InfoTab::Help => match &t.help {
                        Some(h) => {
                            let source = if h.doc.is_empty() {
                                h.signature.clone()
                            } else {
                                format!("{}\n{}", h.doc, h.signature)
                            };
                            let mut job = highlight(&source, &palette);
                            job.wrap.max_width = f32::INFINITY;
                            let ground = palette
                                .background
                                .unwrap_or_else(|| ui.visuals().text_edit_bg_color());
                            egui::Frame::new().fill(ground).inner_margin(4.0).show(ui, |ui| {
                                ui.set_min_width(ui.available_width());
                                ui.add(egui::Label::new(job).extend());
                            });
                        }
                        None => {
                            ui.weak("Select a function or constant, or put the cursor on one.");
                        }
                    },
                    InfoTab::Bookmarks => {
                        let buf = &app.scripts[&key];
                        for (n, line) in buf.numbered.iter().enumerate() {
                            let Some(line) = line else { continue };
                            let text = buf.text.lines().nth(*line).unwrap_or_default().trim();
                            if ui
                                .selectable_label(false, format!("[{n}] {:>5}: {text}", line + 1))
                                .on_hover_text(format!("Ctrl+{n}"))
                                .clicked()
                            {
                                jump = Some((key, tools::line_start(&buf.text, *line)));
                            }
                        }
                        for line in &buf.bookmarks {
                            let text = buf.text.lines().nth(*line).unwrap_or_default().trim();
                            if ui
                                .selectable_label(false, format!("{:>5}: {text}", line + 1))
                                .clicked()
                            {
                                jump = Some((key, tools::line_start(&buf.text, *line)));
                            }
                        }
                    }
                    InfoTab::SearchResults => {
                        for (k, line, text) in &t.search.results {
                            if ui
                                .selectable_label(
                                    false,
                                    format!("{}({}): {text}", k.resref, line + 1),
                                )
                                .clicked()
                            {
                                open_other = Some((k.resref.to_string(), line + 1));
                            }
                        }
                    }
                }
            });
        });

    // The editor with line numbers.
    let style_indent = tools::Indent::of(&app.settings.script_style);
    let error_line = app.live_error(key).and_then(|e| e.0);
    let error_color = ui.visuals().error_fg_color;
    let mut ctrl_click = None;
    let Moonglow { scripts, laid_out, script_tools, open_tabs, .. } = app;
    let buf = scripts.get_mut(&key).expect("open");
    // Tab indents as the script is indented (Jasperre's report), else as
    // Options › Script Editor says.
    let mut indent = tools::Indent::of_script(buf.indent, style_indent);
    if let Some((_, tabs)) = script_tools.convert_indent.take_if(|(k, _)| *k == key) {
        convert_indentation(buf, &ctx, key, tabs, indent.width);
        indent = tools::Indent::of_script(buf.indent, style_indent);
    }
    // Edit › Undo or Redo, or the toolbar's: the editor's step, and the
    // keys back in it after (below, once the click that took them is
    // past), so that Ctrl+Z after it is the editor's too, not the
    // module's.
    let stepped = script_tools.undo.take_if(|(k, _)| *k == key);
    if let Some((_, redo)) = stepped {
        step_history(buf, &ctx, key, redo);
        // (As typing closes it.)
        script_tools.completion = None;
    }
    // Scrolled and its cursor where they were, a module's kept tabs
    // opening it again (the cursor's not given the keys).
    let (kept_scroll, kept_cursor) =
        open_tabs.take_view(&crate::Tab::Script(key)).map(|v| v.script()).unwrap_or_default();
    if let Some((a, b)) = kept_cursor {
        let id = editor_id(key);
        let end = buf.text.chars().count();
        let mut state = egui::text_edit::TextEditState::load(&ctx, id).unwrap_or_default();
        let at = |n: usize| egui::text::CCursor::new(n.min(end));
        state.cursor.set_char_range(Some(egui::text::CCursorRange::two(at(a), at(b))));
        state.store(&ctx, id);
    }
    if let Some(text) = insert_text {
        let (a, b) =
            cursor(&ctx, key).unwrap_or((buf.text.chars().count(), buf.text.chars().count()));
        insert(buf, &ctx, key, a, b, &text);
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
        insert(buf, &ctx, key, c.start, at, &text);
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
        let lines = buf.text.chars().skip(a).take(b - a).any(|c| c == '\n');
        let outdent = ui.input_mut(|i| i.consume_key(Modifiers::SHIFT, Key::Tab));
        let tab = !outdent && ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Tab));
        if outdent || (tab && lines) {
            let (text, from, to) = tools::indent_lines(&buf.text, a, b, outdent, indent);
            buf.text = text;
            set_cursor(&ctx, key, from, to);
        } else if tab {
            insert(buf, &ctx, key, a, b, &indent.at(&buf.text, a));
        }
        // Enter begins the new line as far in as the line it is started
        // from (Aurora does; the field alone begins it at the margin).
        if ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Enter)) {
            let start = format!("\n{}", tools::new_line_indent(&buf.text, a));
            insert(buf, &ctx, key, a, b, &start);
            script_tools.scroll = Some(key);
        }
        // Page Down and Page Up move the cursor by the lines in sight, and
        // the text with it (the field alone has no page keys); with Shift,
        // the selection.
        let row = ui.fonts_mut(|f| f.row_height(&palette.font()));
        let page = ((ui.available_height() / row).floor() as isize - 1).max(1);
        for (k, lines) in [(Key::PageDown, page), (Key::PageUp, -page)] {
            let select = ui.input_mut(|i| i.consume_key(Modifiers::SHIFT, k));
            if !select && !ui.input_mut(|i| i.consume_key(Modifiers::NONE, k)) {
                continue;
            }
            let id = editor_id(key);
            let mut state = egui::text_edit::TextEditState::load(&ctx, id).unwrap_or_default();
            let Some(r) = state.cursor.char_range() else { continue };
            let to = tools::line_moved(&buf.text, r.primary.index.into(), lines);
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
    let pending = script_tools.jump.take_if(|(k, _)| *k == key).or(jump).map(|(_, at)| at);
    if let Some(at) = pending {
        set_cursor(&ctx, key, at, at);
        script_tools.scroll = Some(key);
    }
    let cached = laid_out.entry(key).or_insert(None);
    let mut layouter = |ui: &Ui, text: &dyn egui::TextBuffer, wrap: f32| {
        if let Some(c) = cached.as_ref().filter(|c| {
            c.wrap == wrap
                && c.palette == palette
                && c.error_line == error_line
                && c.text == text.as_str()
        }) {
            return c.galley.clone();
        }
        // Code does not wrap (the editor scrolls sideways), which also
        // keeps the line numbers beside their lines.
        let mut job = highlight(text.as_str(), &palette);
        job.wrap.max_width = f32::INFINITY;
        if let Some(line) = error_line {
            underline_line(&mut job, text.as_str(), line, error_color);
        }
        let galley = ui.fonts_mut(|f| f.layout_job(job));
        *cached = Some(LaidOut {
            text: text.as_str().to_string(),
            wrap,
            palette,
            error_line,
            galley: galley.clone(),
        });
        galley
    };
    let lines = buf.text.split('\n').count();
    let numbers: String = (1..=lines)
        .map(|n| {
            if let Some(b) = buf.numbered.iter().position(|l| *l == Some(n - 1)) {
                format!("{b}{n:>5}\n")
            } else if buf.bookmarks.contains(&(n - 1)) {
                format!("◆{n:>5}\n")
            } else {
                format!("{n:>6}\n")
            }
        })
        .collect();
    let mut completion_at = None;
    let mut area = egui::ScrollArea::both().id_salt(("script-scroll", key));
    if let Some(at) = kept_scroll {
        area = area.scroll_offset(at);
    }
    let mut cursor_now = None;
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
            ui.add(
                egui::Label::new(egui::RichText::new(numbers).font(palette.font()).weak())
                    .selectable(false),
            );
            // The selection is drawn here, not by the field: to each
            // line's last character, where the field draws a line end
            // that is selected as half a character more (which reads
            // as a space the line does not have). The field's own is
            // not drawn (no color), and the field's ground and the
            // selection go under its text.
            let under = ui.painter().add(egui::Shape::Noop);
            // (Options › Script Editor's, else the theme's.)
            let ground = palette.background.unwrap_or_else(|| ui.visuals().text_edit_bg_color());
            let selected = std::mem::replace(
                &mut ui.visuals_mut().selection.bg_fill,
                egui::Color32::TRANSPARENT,
            );
            let out = egui::TextEdit::multiline(&mut buf.text)
                .code_editor()
                .desired_width(f32::INFINITY)
                .desired_rows(1)
                .min_size(egui::vec2(0.0, room))
                .id(editor_id(key))
                .background_color(egui::Color32::TRANSPARENT)
                .layouter(&mut layouter)
                .show(ui);
            cursor_now = out.state.cursor.char_range();
            ui.visuals_mut().selection.bg_fill = selected;
            out.response.response.context_menu(|ui| {
                for (tabs, label) in INDENT_CONVERSIONS {
                    if ui.button(label).clicked() {
                        convert = Some(tabs);
                        ui.close();
                    }
                }
            });
            let field = out.response.response.rect;
            let radius = ui.style().interact(&out.response.response).corner_radius;
            let mut shapes = vec![egui::Shape::rect_filled(field, radius, ground)];
            let rows = out.cursor_range.map(|r| selection_rects(&out.galley, &r));
            for rect in rows.unwrap_or_default() {
                let rect = rect.translate(out.galley_pos.to_vec2());
                shapes.push(egui::Shape::rect_filled(rect, 0.0, selected));
            }
            ui.painter().set(under, egui::Shape::Vec(shapes));
            if let Some(r) = out.cursor_range {
                let rect =
                    out.galley.pos_from_cursor(r.primary).translate(out.galley_pos.to_vec2());
                if script_tools.scroll == Some(key) {
                    ui.scroll_to_rect(rect.expand(40.0), Some(egui::Align::Center));
                    script_tools.scroll = None;
                }
                completion_at = Some(rect.left_bottom());
                // Help follows the identifier under the cursor.
                if out.response.response.changed() || out.response.response.clicked() {
                    let word = tools::word_at(&buf.text, r.primary.index.into());
                    if let Some(s) = word.and_then(|w| symbols.iter().find(|s| s.name == w)) {
                        script_tools.help = Some(s.clone());
                        // A double click on it brings its help forward
                        // (over the compiler's messages, say), as in
                        // Aurora.
                        if out.response.response.double_clicked() {
                            script_tools.info = InfoTab::Help;
                        }
                    }
                }
                if out.response.response.changed() {
                    script_tools.completion = None;
                }
                // Ctrl+click goes to the definition.
                if out.response.response.clicked() && ctx.input(|i| i.modifiers.command) {
                    ctrl_click = Some(usize::from(r.primary.index));
                }
            }
        });
    });
    let cursor_now = cursor_now.map(|r| {
        let (a, b): (usize, usize) = (r.primary.index.into(), r.secondary.index.into());
        (a.min(b), a.max(b))
    });
    buf.seen = Some((shown.state.offset, cursor_now));
    if stepped.is_some() {
        ctx.memory_mut(|m| m.request_focus(editor_id(key)));
    }
    if let Some(tabs) = convert {
        script_tools.convert_indent = Some((key, tabs));
        ctx.request_repaint();
    }

    // The completion list at the cursor.
    if let (Some(c), Some(pos)) =
        (script_tools.completion.as_mut().filter(|c| c.script == key), completion_at)
    {
        let mut pick = None;
        egui::Area::new(egui::Id::new(("completion", key)))
            .fixed_pos(pos)
            .order(egui::Order::Foreground)
            .show(&ctx, |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    egui::ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
                        for (i, s) in c.items.iter().enumerate() {
                            let r = ui
                                .selectable_label(i == c.selected, &s.name)
                                .on_hover_text(&s.signature);
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
        if let Some(i) = pick {
            let s = c.items[i].clone();
            let start = c.start;
            script_tools.completion = None;
            let text = if s.kind == SymbolKind::Function { format!("{}(", s.name) } else { s.name };
            let at = cursor(&ctx, key).map_or(start, |x| x.0);
            insert(buf, &ctx, key, start, at, &text);
        }
    }

    // A compiler message or search result in another script opens it.
    if let Some((file, line)) = open_other {
        match tools::nss(&file) {
            Some(k) if k == key => {
                let at = tools::line_start(&buf.text, line.saturating_sub(1));
                script_tools.jump = Some((key, at));
            }
            Some(k) if app.ws.as_ref().is_some_and(|w| w.module.contains(&k)) => {
                let text = app.script_source(&k).unwrap_or_default();
                app.script_tools.jump = Some((k, tools::line_start(&text, line.saturating_sub(1))));
                app.actions.push(Action::OpenTab(Tab::Script(k)));
            }
            Some(k) => app.actions.push(Action::OpenTab(Tab::Resource(k))),
            None => {}
        }
    }

    if let Some(at) = ctrl_click
        && let Some(d) = app.declaration_at(key, at)
    {
        app.go_to_declaration(&d);
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
/// toolbar's is off then).
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
        // Options > Script Editor: saving compiles too.
        let compiled =
            (compile || to_scratch || app.settings.auto_compile) && compile_one(app, key, &text);
        // (After the text is the module's: the scripts that include it.)
        if compile || app.settings.auto_compile {
            app.actions.push(Action::CompileIncluders(key));
        }
        // To Scratch: the source whatever the compiler said (an include
        // file, one with an error, El Grillo's request), its compiled
        // script only if it compiled just now (after the edits above).
        if to_scratch {
            app.actions.push(Action::ScriptToScratch { key, compiled });
        }
    }
}

impl Moonglow {
    /// [`Action::ScriptToScratch`]: the script's source, as saved into the
    /// module, copied into the scratch folder, and with it its compiled
    /// script and debug file if it `compiled` just now; else not (a
    /// compiled script the module has from before is older than the
    /// source). The Compiler tab and the log say what was copied.
    pub(crate) fn script_to_scratch(&mut self, key: ResKey, compiled: bool) {
        let Some(ws) = &mut self.ws else { return };
        if let Err(e) = ws.flush() {
            self.log.error(e.to_string());
            return;
        }
        let files: Vec<ResKey> =
            if compiled { mg_module::transfer::file_set(&ws.module, &key) } else { vec![key] };
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
                    let (removed, failed) =
                        crate::transfer::remove_stale_compiled(&dir, &[key.resref]);
                    if !removed.is_empty() {
                        said.push_str(&format!("; {}", crate::transfer::removed_said(&removed)));
                    }
                    if !failed.is_empty() {
                        said.push_str(&format!("; could not remove {}", failed.join(", ")));
                        error = true;
                    }
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
        tools.info = InfoTab::Compiler;
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
    // (Open here with something the file lacks: Moonglow's editor has it.)
    if app.scripts.contains_key(&key) && project_file(app, key).is_none() {
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
            let dir = std::env::temp_dir().join(format!("moonglow-{}", std::process::id()));
            let path = dir.join(format!("{}.nss", key.resref.to_lowercase()));
            let written =
                std::fs::create_dir_all(&dir).and_then(|()| std::fs::write(&path, buf.bytes()));
            if let Err(e) = written {
                app.log.error(format!("{}: {e}", path.display()));
                return;
            }
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
    !path.starts_with(std::env::temp_dir().join(format!("moonglow-{}", std::process::id())))
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
            buf.text = text;
            app.log.info(format!("{key}: changed in the external editor"));
        }
    }
}

/// Finds the next match of the Find window's text in a script.
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
    compile_with_debug(app, key, text, false, None).map(|(ncs, _)| ncs)
}

/// [`compile_source`], with the debug information (`.ndb`) if `debug`.
fn compile_with_debug(
    app: &Moonglow,
    key: ResKey,
    text: &str,
    debug: bool,
    external: Option<&mg_module::external_compiler::ExternalCompiler>,
) -> Result<(Vec<u8>, Option<Vec<u8>>), mg_script::compiler::CompileError> {
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
    // Editor): the module's scripts as they are, this one as in the editor.
    if let (Some(external), Some(module)) = (external, module) {
        let mut sources: Vec<(String, &[u8])> = module
            .keys_of(ResType::NSS)
            .filter(|k| **k != key)
            .filter_map(|k| Some((k.resref.to_lowercase().to_string(), module.get(k)?)))
            .collect();
        sources.push((name.clone(), &source));
        let failed = |message: String| mg_script::compiler::CompileError { code: -1, message };
        let made = external
            .compile(&sources, std::slice::from_ref(&name))
            .map_err(|e| failed(format!("External compiler: {e}")))?;
        let Some(made) = made.into_iter().next() else {
            return Err(failed(format!("{name}.nss: the external compiler made nothing")));
        };
        return match made.ncs {
            Some(ncs) => Ok((ncs, made.ndb.filter(|_| debug))),
            None => Err(failed(made.message)),
        };
    }
    let mut c = Compiler::new(|n: &str, t: ResType| {
        if t == ResType::NSS && n.eq_ignore_ascii_case(&name) {
            return Some(source.clone());
        }
        let k = ResKey::parse(n, t)?;
        module
            .and_then(|m| m.get(&k))
            .map(<[u8]>::to_vec)
            .or_else(|| resman?.get(&k).ok().map(|d| d.into_owned()))
    });
    c.set_debug_output(debug);
    c.compile(&name).map(|out| (out.ncs, out.ndb))
}

/// Compiles again, into the module, those of `scripts` whose compiled
/// script isn't what their source compiles to now (changed since, or an
/// include of theirs was), so that what goes out with them is current.
/// Returns the names of those compiled again, and of those that have a
/// compiled script but no longer compile (theirs is older than the
/// source). Without the game's data nothing can be compiled, and nothing
/// is said.
pub(crate) fn compile_stale(app: &mut Moonglow, scripts: &[ResKey]) -> (Vec<String>, Vec<String>) {
    let (mut compiled, mut broken) = (Vec::new(), Vec::new());
    if app.game.is_none() {
        return (compiled, broken);
    }
    let mut edits = Vec::new();
    let external = app.external_compiler();
    for &key in scripts {
        let Some(ws) = app.ws.as_ref() else { break };
        let Some(text) = ws.module.get(&key).map(decode) else { continue };
        let ncs = ResKey::new(key.resref, ResType::NCS);
        let old = ws.module.get(&ncs).map(<[u8]>::to_vec);
        match compile_with_debug(app, key, &text, app.settings.debug_info, external.as_ref()) {
            Ok((new, ndb)) if old.as_deref() != Some(&new[..]) => {
                edits.push(Edit::SetResource { key: ncs, data: Some(new) });
                if let Some(ndb) = ndb {
                    let key = ResKey::new(key.resref, ResType::NDB);
                    edits.push(Edit::SetResource { key, data: Some(ndb) });
                }
                compiled.push(key.resref.to_string());
            }
            Ok(_) => {}
            // (One without a compiled script is an include, or is named
            // by the caller as not compiled.)
            Err(_) if old.is_some() => broken.push(key.resref.to_string()),
            Err(_) => {}
        }
    }
    if !edits.is_empty()
        && let Err(e) = app.apply(Command::new("Compile", edits))
    {
        app.log.error(e.to_string());
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
        // here, with the module's scripts written out for it each time:
        // more than one go to the job, which runs it once for them all.)
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
        let Some(text) = self.ws.as_ref().and_then(|ws| ws.module.get(&key).map(decode)) else {
            return false;
        };
        if self.game.is_none() || !mg_script::outline::has_entry_point(&encode(&text)) {
            return false;
        }
        let external = self.external_compiler();
        compile_with_debug(self, key, &text, false, external.as_ref()).is_err()
    }
}

/// Compiles one script (its text as in the editor) into the module;
/// whether it compiled.
fn compile_one(app: &mut Moonglow, key: ResKey, text: &str) -> bool {
    if app.ws.is_none() {
        return false;
    }
    app.script_tools.messages.retain(|m| m.script != key);
    app.script_tools.info = InfoTab::Compiler;
    // An include file (no `main`, no `StartingConditional`) isn't compiled
    // on its own: it is compiled into the scripts that include it, and
    // may lean on what they bring. (Aurora's build compiles it too.)
    if !mg_script::outline::has_entry_point(&encode(text)) {
        let said = format!("{key}: an include file (no main or StartingConditional), not compiled");
        app.log.info(said.clone());
        app.script_tools.messages.push(Message::new(key, said, None, false));
        return false;
    }
    let external = app.external_compiler();
    let result = compile_with_debug(app, key, text, app.settings.debug_info, external.as_ref());
    match result {
        Ok((out, ndb)) => {
            let ncs = ResKey::new(key.resref, ResType::NCS);
            let mut edits = vec![Edit::SetResource { key: ncs, data: Some(out) }];
            if let Some(ndb) = ndb {
                edits.push(Edit::SetResource {
                    key: ResKey::new(key.resref, ResType::NDB),
                    data: Some(ndb),
                });
            }
            app.actions.push(Action::Apply(Command::new(format!("Compile {key}"), edits)));
            app.log.info(format!("{key}: compiled"));
            let said = format!("{key}: compiled");
            app.script_tools.messages.push(Message::new(key, said, None, false));
            true
        }
        Err(e) => {
            app.log.error(e.message.clone());
            let location = e.location();
            app.script_tools.messages.push(Message::new(key, e.message, location, true));
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let mut open = ScriptBuffer::read(b"// \xb9\n", (0, 0));
        open.text.push_str("void main() {}\n");
        scripts.insert(key, open);
        assert_eq!(read(&scripts).as_deref(), Some("// ą\nvoid main() {}\n"), "the editor's");
        crate::text::set_edit_language(Language::ENGLISH);
    }

    /// A CRLF script (BioWare's, the new scripts') is edited with plain
    /// newlines, no `\r` for the cursor to stop on at each line's end
    /// (Jasperre's report), and written back as it was: untouched, byte
    /// for byte; edited, with CRLF on every line, the new ones too.
    #[test]
    fn a_crlf_script_is_edited_with_plain_newlines_and_saved_with_crlf() {
        let file = NEW_SCRIPT.as_bytes();
        let mut buf = ScriptBuffer::read(file, (0, 0));
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
        let mut lf = ScriptBuffer::read(b"void main()\n{\n}\n", (0, 0));
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

    /// An error's line alone is underlined, from its first character to
    /// its last: tokens of one color run together over lines in one
    /// section, and the lines around it were underlined with it.
    #[test]
    fn an_error_underlines_its_own_line() {
        let text = "void main()\n{\n    int n = x;\n}\n";
        let style = ScriptStyle::default();
        let mut job = highlight(text, &Palette::new(&style, true, Color32::WHITE));
        underline_line(&mut job, text, 2, Color32::RED);
        let underlined: String = job
            .sections
            .iter()
            .filter(|s| s.format.underline.width > 0.0)
            .map(|s| &text[s.byte_range.start.0..s.byte_range.end.0])
            .collect();
        assert_eq!(underlined, "int n = x;");
        assert_eq!(job.text, text);
        let covered: usize =
            job.sections.iter().map(|s| s.byte_range.end.0 - s.byte_range.start.0).sum();
        assert_eq!(covered, text.len(), "every byte in one section");
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
        for (key, layer) in game.resman.entries() {
            if key.restype != ResType::NSS || scripts.iter().any(|(k, _)| *k == key) {
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
/// between, the arrows move the selection (with Shift, widen it); Enter or
/// Open opens every one selected, each in its tab (with none selected,
/// Enter opens the first listed), and a double click opens that one (El
/// Grillo's request). The game's and a hak's open to be read only.
fn open_script_ui(app: &mut Moonglow, ctx: &egui::Context) {
    use crate::script_tools::{ScriptFrom, ScriptsShown};
    let Some(mut w) = app.script_tools.open_script.take() else { return };
    let mut open = true;
    let mut chosen: Vec<(ResKey, ScriptFrom)> = Vec::new();
    crate::widgets::dialog(ctx, "Open Script", &mut open)
        .default_size([420.0, 440.0])
        .collapsible(false)
        .show(ctx, |ui| {
            // Down and Up (Page Down and Up) move the selection among the
            // scripts listed, while the name is typed too; with Shift,
            // they widen it.
            let (mut moved, mut widen): (isize, bool) = (0, false);
            for (key, by) in [
                (egui::Key::ArrowDown, 1),
                (egui::Key::ArrowUp, -1),
                (egui::Key::PageDown, 10),
                (egui::Key::PageUp, -10),
            ] {
                // (Shift first: a key without it matches one with it.)
                if ui.input_mut(|i| i.consume_key(egui::Modifiers::SHIFT, key)) {
                    moved += by;
                    widen = true;
                } else if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, key)) {
                    moved += by;
                }
            }
            let field = ui.add(
                egui::TextEdit::singleline(&mut w.filter)
                    .hint_text("Find by name")
                    .desired_width(f32::INFINITY),
            );
            crate::widgets::autofocus(ui, &field);
            // (A name typed starts over: Enter opens the first it lists.)
            if field.changed() {
                w.cursor = 0;
                w.selected.clear();
                w.anchor = None;
            }
            // (Enter opens wherever the keys are in the window: after a
            // click on a script or on what to show they are not the
            // name's.)
            let entered = (field.lost_focus() || !field.has_focus())
                && ui.input(|i| i.key_pressed(egui::Key::Enter));
            ui.horizontal_wrapped(|ui| {
                for (shown, label) in [
                    (ScriptsShown::All, "All Resources"),
                    (ScriptsShown::Module, "Module Resources Only"),
                    (ScriptsShown::Haks, "Hak Pak Resources Only"),
                ] {
                    ui.radio_value(&mut w.shown, shown, label);
                }
            });
            let needle = mg_core::text::Needle::new(&w.filter);
            let listed: Vec<(ResKey, ScriptFrom)> = w
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
            let names: Vec<ResKey> = listed.iter().map(|(k, _)| *k).collect();
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
            // What Enter and Open open: those selected that are listed
            // (a narrower name hides the others), in the list's order.
            let picked: Vec<(ResKey, ScriptFrom)> =
                listed.iter().filter(|(k, _)| w.selected.contains(k)).copied().collect();
            ui.weak(match picked.len() {
                0 => format!("{} scripts", listed.len()),
                n => format!("{} scripts, {n} selected", listed.len()),
            });
            if entered {
                chosen = if picked.is_empty() {
                    listed.get(cursor).copied().into_iter().collect()
                } else {
                    picked.clone()
                };
            }
            let row = ui.spacing().interact_size.y;
            // (Room kept under the list for its button.)
            let footer = ui.spacing().interact_size.y + 2.0 * ui.spacing().item_spacing.y + 6.0;
            let height = (ui.available_height() - footer).max(3.0 * row);
            let mut list = egui::ScrollArea::vertical().auto_shrink(false).max_height(height);
            if moved != 0 {
                let pitch = row + ui.spacing().item_spacing.y;
                let above = (height / 2.0 - pitch).max(0.0);
                list = list.vertical_scroll_offset((cursor as f32 * pitch - above).max(0.0));
            }
            list.show_rows(ui, row, listed.len(), |ui, range| {
                for (at, (key, from)) in range.clone().zip(listed[range].iter().copied()) {
                    ui.horizontal(|ui| {
                        let r = ui
                            .selectable_label(w.selected.contains(&key), key.resref.to_string())
                            .on_hover_text(
                                "Double-click to open; Ctrl+click and Shift+click select more \
                                 to open together",
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
                    });
                }
            });
            ui.separator();
            ui.horizontal(|ui| {
                let button = egui::Button::new(match picked.len() {
                    n if n > 1 => format!("Open {n}"),
                    _ => "Open".to_string(),
                });
                if ui
                    .add_enabled(!picked.is_empty(), button)
                    .on_hover_text("Open the scripts selected, each in its tab (Enter)")
                    .on_disabled_hover_text("Select a script (Ctrl+click or Shift+click: several)")
                    .clicked()
                {
                    chosen = picked.clone();
                }
            });
        });
    for (key, from) in &chosen {
        app.actions.push(match from {
            ScriptFrom::Module => Action::OpenTab(Tab::Script(*key)),
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
        egui::Window::new(title)
            .pivot(egui::Align2::CENTER_CENTER)
            .default_pos(ctx.content_rect().center())
            .collapsible(false)
            .resizable(false)
            .open(crate::widgets::open_unless_escape(&ctx, title, &mut s.open))
            .show(&ctx, |ui| {
                egui::Grid::new("find-grid").num_columns(2).show(ui, |ui| {
                    crate::widgets::field_label(ui, "Find What");
                    let field = ui.text_edit_singleline(&mut s.find);
                    crate::widgets::autofocus(ui, &field);
                    // Find Next (F3) from the field too, as in Aurora
                    // (Sacha's report): it types nothing.
                    if field.has_focus()
                        && ui.input_mut(|i| keys.consume(i, crate::keys::Cmd::FindNext))
                    {
                        f3 = true;
                    }
                    // Enter finds (the next, or all), and the field keeps
                    // the keys for the next Enter.
                    if field.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        entered = true;
                        field.request_focus();
                    }
                    ui.end_row();
                    if s.replace_mode {
                        ui.label("Replace With");
                        ui.text_edit_singleline(&mut s.replace);
                        ui.end_row();
                    }
                });
                ui.checkbox(&mut s.options.match_case, "Match Case");
                ui.checkbox(&mut s.options.whole_word, "Match Whole Word Only");
                ui.checkbox(&mut s.options.backwards, "Search Backward");
                if !s.replace_mode {
                    if ui
                        .checkbox(&mut s.in_open, "Find In Currently Open Scripts")
                        .on_hover_text("Every match in the scripts open now, in Search Results")
                        .changed()
                        && s.in_open
                    {
                        s.in_files = false;
                    }
                    if ui
                        .checkbox(&mut s.in_files, "Find In Files (all scripts in the module)")
                        .changed()
                        && s.in_files
                    {
                        s.in_open = false;
                    }
                }
                let all = (s.in_files || s.in_open) && !s.replace_mode;
                ui.horizontal(|ui| {
                    if all {
                        find_all = ui.button("Find All").clicked() || entered;
                    } else {
                        find_next = ui.button("Find Next").clicked() || entered || f3;
                    }
                    if s.replace_mode {
                        replace = ui.button("Replace").clicked();
                        replace_all = ui.button("Replace All").clicked();
                    }
                });
            });
        app.script_tools.search = s.clone();
        let target = s.script.filter(|k| app.scripts.contains_key(k));
        if find_next && let Some(k) = target {
            find_in_script(app, &ctx, k);
        }
        if replace && let Some(k) = target {
            // Replace the selection when it is a match, then find the next.
            if let Some((a, b)) = cursor(&ctx, k) {
                let buf = app.scripts.get_mut(&k).expect("open");
                if tools::is_match_at(&buf.text, &s.find, a, b, &s.options) {
                    insert(buf, &ctx, k, a, b, &s.replace);
                }
            }
            find_in_script(app, &ctx, k);
        }
        if replace_all && let Some(k) = target {
            let buf = app.scripts.get_mut(&k).expect("open");
            let (text, n) = tools::replace_all(&buf.text, &s.find, &s.replace, &s.options);
            buf.text = text;
            app.log.info(format!("Replaced {n} in {k}"));
        }
        if find_all {
            let Some(ws) = &app.ws else { return };
            let mut keys: Vec<ResKey> = if s.in_open {
                app.scripts.keys().copied().collect()
            } else {
                ws.module.keys_of(ResType::NSS).copied().collect()
            };
            keys.sort();
            let mut results = Vec::new();
            for k in keys {
                let text = script_source(&app.scripts, Some(ws), None, &k).unwrap_or_default();
                for (line, t) in tools::find_lines(&text, &s.find, &s.options) {
                    results.push((k, line, t));
                }
            }
            app.log.info(format!("{:?}: {} lines found", s.find, results.len()));
            app.script_tools.search.results = results;
            app.script_tools.info = InfoTab::SearchResults;
        }
    }

    use crate::widgets::{NameWindow, Named};
    if let Some((from, mut name)) = app.script_save_as.clone() {
        let window = NameWindow {
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
                app.actions.push(Action::OpenTab(Tab::Script(k)));
                None
            }
        };
    }

    if let Some(mut name) = app.new_script.clone() {
        let window = NameWindow {
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
                app.actions.push(Action::Apply(Command::new(
                    format!("New script {k}"),
                    vec![Edit::SetResource { key: k, data }],
                )));
                app.actions.push(Action::OpenTab(Tab::Script(k)));
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
