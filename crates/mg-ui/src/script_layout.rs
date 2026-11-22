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

/// A script editor's text and line numbers, laid out a line at a time and
/// kept between frames.
#[derive(Debug, Clone, Default)]
pub(crate) struct LaidOut {
    made_with: Option<MadeWith>,
    /// Each line, and its galley.
    lines: Vec<Line>,
    galleys: Vec<Arc<Galley>>,
    /// The whole text's galley.
    galley: Option<Arc<Galley>>,
    /// Each line's number, laid out ([`LaidOut::numbers`]).
    numbers: Vec<Arc<Galley>>,
}

impl LaidOut {
    /// Readies the editor's galleys for a frame drawn with `palette`: those
    /// laid out otherwise (other colors, another font size or scale, egui's
    /// fonts made again) are let go of.
    pub(crate) fn begin(&mut self, ui: &Ui, palette: &Palette) {
        let made_with = MadeWith {
            palette: *palette,
            weak: ui.visuals().weak_text_color(),
            pixels_per_point: ui.ctx().pixels_per_point(),
            fonts: fonts_made(ui.ctx()),
        };
        if self.made_with != Some(made_with) {
            *self = LaidOut { made_with: Some(made_with), ..LaidOut::default() };
        }
    }

    /// The text laid out, colored by its tokens (the live error's wavy
    /// underline is painted over it by the editor, M4's U13: no line is
    /// laid out again as the error comes and goes). Of the text laid out
    /// before, the lines before the change are kept, and those after it
    /// once the tokens there begin as they did (a block comment opened
    /// colors the lines down to its end); between them, those as they were
    /// somewhere there before (a Replace All's lines between its
    /// replacements).
    pub(crate) fn text(&mut self, ui: &Ui, text: &str, palette: &Palette) -> Arc<Galley> {
        let before = self.galley.clone();
        if let Some(g) = &before
            && g.text() == text
        {
            return g.clone();
        }
        let old = before.as_ref().map_or("", |g| g.text());
        // The change: from the first byte that differs to the last (as
        // the old text has them, and as the new one does).
        let p = common_prefix(old.as_bytes(), text.as_bytes());
        let room = old.len().min(text.len()) - p;
        let s = common_suffix(&old.as_bytes()[p..], &text.as_bytes()[p..]).min(room);
        let (old_end, new_end) = (old.len() - s, text.len() - s);
        let delta = text.len() as isize - old.len() as isize;
        let first = self.lines.partition_point(|l| l.start <= p).saturating_sub(1);
        let reached = self.lines.partition_point(|l| l.start <= old_end);
        let was: std::collections::HashMap<u64, &Arc<Galley>> =
            (first..reached).map(|i| (self.lines[i].print, &self.galleys[i])).collect();
        let (mut start, mut within) =
            self.lines.get(first).map_or((0, Within::Code), |l| (l.start, l.within));
        let (mut fresh, mut fresh_galleys) = (Vec::new(), Vec::new());
        let mut tokens = Vec::new();
        let mut tail = self.lines.len();
        ui.fonts_mut(|f| {
            loop {
                // A line after the change, begun as the old one in its place
                // began: the rest are as they were.
                if before.is_some() && start > new_end {
                    let at = (start as isize - delta) as usize;
                    if let Ok(j) = self.lines.binary_search_by_key(&at, |l| l.start)
                        && self.lines[j].within == within
                    {
                        tail = j;
                        return;
                    }
                }
                let end = text[start..].find('\n').map_or(text.len(), |i| start + i);
                let next = lex_line(text, start, end, within, &mut tokens);
                let print = fingerprint(text, start, end, &tokens);
                let galley = match was.get(&print) {
                    Some(g) => (*g).clone(),
                    None => f.layout_job(line_job(text, &tokens, palette)),
                };
                fresh.push(Line { start, within, print });
                fresh_galleys.push(galley);
                if end == text.len() {
                    return;
                }
                (start, within) = (end + 1, next);
            }
        });
        drop(was);
        let kept = first + fresh.len();
        self.lines.splice(first..tail, fresh);
        self.galleys.splice(first..tail, fresh_galleys);
        for l in &mut self.lines[kept..] {
            l.start = (l.start as isize + delta) as usize;
        }
        let mut job = LayoutJob::single_section(
            text.to_owned(),
            TextFormat::simple(palette.font(), palette.colors[0]),
        );
        job.wrap.max_width = f32::INFINITY;
        let ppp = ui.ctx().pixels_per_point();
        let galley = Arc::new(Galley::concat(Arc::new(job), &self.galleys, ppp));
        self.galley = Some(galley.clone());
        galley
    }

    /// Lays out the numbers of `lines` lines (each line's own, kept while
    /// the palette is: those of lines added laid out as they come).
    pub(crate) fn numbers(&mut self, ui: &Ui, lines: usize, palette: &Palette) {
        if self.numbers.len() == lines {
            return;
        }
        let color = self.made_with.map_or(mg_uikit::Tokens::DARK.text_3, |m| m.weak);
        self.numbers.truncate(lines);
        ui.fonts_mut(|f| {
            for n in self.numbers.len() + 1..=lines {
                let mut job = LayoutJob::default();
                job.append(&n.to_string(), 0.0, palette.format(color));
                self.numbers.push(f.layout_job(palette.pitched(job)));
            }
        });
    }

    /// The gutter's width: the board's `.code .ln` (12 points, the widest
    /// number, 10 points), for the numbers laid out last.
    pub(crate) fn gutter_width(&self) -> f32 {
        let widest = self.numbers.last().map_or(0.0, |g| g.size().x);
        GUTTER_LEFT + widest + GUTTER_RIGHT
    }

    /// Paints the line numbers in the gutter whose left edge is `left`,
    /// beside the rows of `galley` (the text, laid out at `at`, a row a
    /// line), those in sight alone: each right-aligned, the line
    /// `current`'s in `current_color` (`.ln.cur`), the others in the
    /// numbers' color; and each bookmark's mark in the room before them
    /// ([`paint_marks`]).
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
        let right = left + GUTTER_LEFT + self.numbers.last().map_or(0.0, |g| g.size().x);
        let first = galley.rows.partition_point(|r| at.y + r.pos.y + r.size.y < clip.top());
        let painter = ui.painter();
        for (i, row) in galley.rows.iter().enumerate().skip(first) {
            let top = at.y + row.pos.y;
            if top > clip.bottom() {
                break;
            }
            let Some(number) = self.numbers.get(i) else { break };
            let pos = egui::pos2(right - number.size().x, top);
            if current == Some(i) {
                painter.galley_with_override_text_color(pos, number.clone(), current_color);
            } else {
                painter.galley(pos, number.clone(), current_color);
            }
        }
        paint_marks(ui, left, galley, at, marks, self.made_with.map_or(current_color, |m| m.weak));
    }
}

/// The gutter named for a screen reader by what it shows ("Line numbers 1
/// to 3000"), not by every number in it.
pub(crate) fn name_gutter(gutter: &egui::Response, lines: usize) {
    let said = format!("Line numbers 1 to {}", lines.max(1));
    gutter.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Label, true, &said));
}

/// The text field described for a screen reader by the cursor's line
/// ("Line 12 of 3000"): the gutter's number for it, said.
pub(crate) fn describe_line(field: &egui::Response, line: usize, lines: usize) {
    let said = format!("Line {line} of {}", lines.max(1));
    field.ctx.accesskit_node_builder(field.id, |node| node.set_description(said));
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
    /// Where it starts (bytes), and what in.
    start: usize,
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

/// How many bytes `a` and `b` begin with alike.
fn common_prefix(a: &[u8], b: &[u8]) -> usize {
    let n = a.len().min(b.len());
    let mut i = 0;
    // (64 at a time, compared as slices are: quick.)
    while i + 64 <= n && a[i..i + 64] == b[i..i + 64] {
        i += 64;
    }
    while i < n && a[i] == b[i] {
        i += 1;
    }
    i
}

/// How many bytes `a` and `b` end with alike.
fn common_suffix(a: &[u8], b: &[u8]) -> usize {
    let n = a.len().min(b.len());
    let (a, b) = (&a[a.len() - n..], &b[b.len() - n..]);
    let mut i = 0;
    while i + 64 <= n && a[n - i - 64..n - i] == b[n - i - 64..n - i] {
        i += 64;
    }
    while i < n && a[n - i - 1] == b[n - i - 1] {
        i += 1;
    }
    i
}

/// A line's layout: its tokens' pieces in their colors, the line
/// [`Palette::pitch`] high.
fn line_job(text: &str, tokens: &[Token], palette: &Palette) -> LayoutJob {
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

/// Each bookmark's mark in the room before the line numbers of the
/// gutter whose left edge is `left` ([`GUTTER_LEFT`]), on its line's row
/// of `galley` (laid out at `at`): a numbered bookmark's number, else the
/// `bookmark` icon, in `color` (the numbers').
fn paint_marks(
    ui: &Ui,
    left: f32,
    galley: &Galley,
    at: egui::Pos2,
    marks: Marks<'_>,
    color: Color32,
) {
    let clip = ui.clip_rect();
    let place = |line: usize| {
        let row = galley.rows.get(line)?;
        let rect = egui::Rect::from_min_size(
            at + row.pos.to_vec2() + egui::vec2(left - at.x - row.pos.x, 0.0),
            egui::vec2(GUTTER_LEFT - MARK_GAP, row.size.y),
        );
        clip.intersects(rect).then_some(rect)
    };
    for line in marks.bookmarks {
        if marks.numbered.contains(&Some(*line)) {
            continue;
        }
        let Some(rect) = place(*line) else { continue };
        let side = rect.height().min(rect.width());
        let square = egui::Rect::from_center_size(rect.center(), egui::Vec2::splat(side));
        mg_uikit::Icon::Bookmark.paint(ui.painter(), square, color);
    }
    let font = galley.job.sections.first().map(|s| s.format.font_id.clone());
    for (n, line) in marks.numbered.iter().enumerate() {
        let (Some(line), Some(font)) = (line, &font) else { continue };
        let Some(rect) = place(*line) else { continue };
        let digit = ui.painter().layout_no_wrap(n.to_string(), font.clone(), color);
        ui.painter().galley(rect.center() - digit.size() / 2.0, digit, color);
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
                let kept = laid.galleys.iter().filter(|g| before.iter().any(|b| Arc::ptr_eq(b, g)));
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
        assert!(nine.iter().zip(&twelve).all(|(a, b)| Arc::ptr_eq(a, b)), "kept");
        let ours = twelve.last().unwrap();
        assert_eq!(ours.text(), "12");
        assert_eq!(ours.size(), last.size());
        assert_eq!(ours.size().y, palette.pitch());
        assert_eq!(ours.rows[0].visuals.mesh.vertices, last.rows[0].visuals.mesh.vertices);
        assert_eq!(width, GUTTER_LEFT + last.size().x + GUTTER_RIGHT);
        assert_eq!((GUTTER_LEFT, GUTTER_RIGHT), (12.0, 10.0));
        let (five, _, _) = frame(5);
        assert_eq!(five.len(), 5);
        assert!(five.iter().zip(&twelve).all(|(a, b)| Arc::ptr_eq(a, b)), "kept");
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
