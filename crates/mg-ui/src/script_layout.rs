//! The script editor's text laid out a line at a time.
//!
//! egui's `TextEdit` asks for the whole text laid out (a galley) each time
//! the text changes. Laid out whole, the game's `nwscript.nss` (14,000
//! lines) took 14 ms a key, and 100 ms at a typist's pace: egui keeps the
//! laid-out lines of a text only while they are asked for at every frame,
//! and the editor's frames between keys did not ask. Here each line keeps
//! its own galley while its text, its colors and its underline stay the
//! same, and a change lays out again only the lines it touched; the
//! editor's galley is the lines' put together (`Galley::concat`, as egui
//! puts the lines of a text together itself). The line numbers beside
//! them are kept the same way.

use std::sync::Arc;

use egui::text::{LayoutJob, LayoutSection, TextFormat};
use egui::{Color32, Galley, Ui};
use mg_script::lex::{Token, TokenKind, tokenize};

use crate::script_view::{Palette, element, underline_line};

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
fn fonts_made(ctx: &egui::Context) -> u64 {
    ctx.data(|d| d.get_temp::<(Arc<Galley>, u64)>(fonts_id()).map_or(0, |(_, made)| made))
}

/// What the galleys were laid out with: any of it changed, every line is
/// laid out again.
#[derive(Debug, Clone, Copy, PartialEq)]
struct MadeWith {
    palette: Palette,
    /// The underline of the error's line.
    error: Color32,
    /// The line numbers' color.
    weak: Color32,
    pixels_per_point: f32,
    /// [`fonts_made`].
    fonts: u64,
}

/// What a line number has before it: nothing, a bookmark's room (its
/// icon is painted there: [`paint_bookmarks`]) or a numbered bookmark's
/// number.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mark {
    None,
    Bookmark,
    Numbered(usize),
}

/// A script editor's text and line numbers, laid out a line at a time and
/// kept between frames.
#[derive(Debug, Clone, Default)]
pub(crate) struct LaidOut {
    made_with: Option<MadeWith>,
    /// Each line, and its galley.
    lines: Vec<Line>,
    galleys: Vec<Arc<Galley>>,
    /// The line underlined as an error, and the whole text's galley.
    error_line: Option<usize>,
    galley: Option<Arc<Galley>>,
    /// Each line number's mark and galley, the marked lines, and the
    /// numbers' galley (with the empty row after the last, as the text
    /// they were drawn from ended with a line end).
    numbers: Vec<(Mark, Arc<Galley>)>,
    marked: Vec<(usize, Mark)>,
    numbers_galley: Option<Arc<Galley>>,
}

impl LaidOut {
    /// Readies the editor's galleys for a frame drawn with `palette`: those
    /// laid out otherwise (other colors, another font size or scale, egui's
    /// fonts made again) are let go of.
    pub(crate) fn begin(&mut self, ui: &Ui, palette: &Palette, error: Color32) {
        let made_with = MadeWith {
            palette: *palette,
            error,
            weak: ui.visuals().weak_text_color(),
            pixels_per_point: ui.ctx().pixels_per_point(),
            fonts: fonts_made(ui.ctx()),
        };
        if self.made_with != Some(made_with) {
            *self = LaidOut { made_with: Some(made_with), ..LaidOut::default() };
        }
    }

    /// The text laid out, colored by its tokens, `error_line` (0-based)
    /// underlined. Of the text laid out before, the lines before the
    /// change are kept, and those after it once the tokens there begin as
    /// they did (a block comment opened colors the lines down to its
    /// end); between them, those as they were somewhere there before (a
    /// Replace All's lines between its replacements).
    pub(crate) fn text(
        &mut self,
        ui: &Ui,
        text: &str,
        palette: &Palette,
        error_line: Option<usize>,
    ) -> Arc<Galley> {
        let before = self.galley.clone();
        if let Some(g) = &before
            && self.error_line == error_line
            && g.text() == text
        {
            return g.clone();
        }
        let old = before.as_ref().map_or("", |g| g.text());
        let error = self.made_with.map_or(mg_uikit::Tokens::DARK.syn_error, |m| m.error);
        // The change: from the first byte that differs to the last (as
        // the old text has them, and as the new one does).
        let p = common_prefix(old.as_bytes(), text.as_bytes());
        let room = old.len().min(text.len()) - p;
        let s = common_suffix(&old.as_bytes()[p..], &text.as_bytes()[p..]).min(room);
        let (old_end, new_end) = (old.len() - s, text.len() - s);
        let delta = text.len() as isize - old.len() as isize;
        let first = self.lines.partition_point(|l| l.start <= p).saturating_sub(1);
        let reached = self.lines.partition_point(|l| l.start <= old_end);
        let was: std::collections::HashMap<(u64, bool), &Arc<Galley>> = (first..reached)
            .map(|i| ((self.lines[i].print, self.lines[i].underlined), &self.galleys[i]))
            .collect();
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
                let underlined = error_line == Some(first + fresh.len());
                let galley = match was.get(&(print, underlined)) {
                    Some(g) => (*g).clone(),
                    None => {
                        let underline = underlined.then_some(error);
                        f.layout_job(line_job(text, &tokens, palette, underline))
                    }
                };
                fresh.push(Line { start, within, print, underlined });
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
        // The error's line underlined, and the one that was no longer.
        let again: Vec<usize> = (self.lines.iter().enumerate())
            .filter(|(i, l)| l.underlined != (error_line == Some(*i)))
            .map(|(i, _)| i)
            .collect();
        for i in again {
            let l = self.lines[i];
            let end = self.lines.get(i + 1).map_or(text.len(), |n| n.start - 1);
            lex_line(text, l.start, end, l.within, &mut tokens);
            let underline = (error_line == Some(i)).then_some(error);
            let job = line_job(text, &tokens, palette, underline);
            self.galleys[i] = ui.fonts_mut(|f| f.layout_job(job));
            self.lines[i].underlined = underline.is_some();
        }
        let mut job = LayoutJob::single_section(
            text.to_owned(),
            TextFormat::simple(palette.font(), palette.colors[0]),
        );
        job.wrap.max_width = f32::INFINITY;
        let ppp = ui.ctx().pixels_per_point();
        let galley = Arc::new(Galley::concat(Arc::new(job), &self.galleys, ppp));
        self.error_line = error_line;
        self.galley = Some(galley.clone());
        galley
    }

    /// The line numbers of `lines` lines, each after its mark: a numbered
    /// bookmark's number, else room for a bookmark's icon (as `{n:>6}` with
    /// the mark in its first place), with an empty row after the last.
    pub(crate) fn numbers(
        &mut self,
        ui: &Ui,
        lines: usize,
        bookmarks: &std::collections::BTreeSet<usize>,
        numbered: &[Option<usize>; 10],
        palette: &Palette,
    ) -> Arc<Galley> {
        let mut marked: Vec<(usize, Mark)> =
            bookmarks.iter().filter(|l| **l < lines).map(|l| (*l, Mark::Bookmark)).collect();
        // (A numbered bookmark's number before a bookmark's icon: the first
        // number of the line's.)
        for (n, line) in numbered.iter().enumerate().rev() {
            if let Some(line) = line.filter(|l| *l < lines) {
                marked.retain(|(l, _)| *l != line);
                marked.push((line, Mark::Numbered(n)));
            }
        }
        marked.sort_unstable_by_key(|(l, _)| *l);
        if let Some(g) = &self.numbers_galley
            && self.numbers.len() == lines
            && self.marked == marked
        {
            return g.clone();
        }
        let font = palette.font();
        let color = self.made_with.map_or(mg_uikit::Tokens::DARK.text_3, |m| m.weak);
        let number = |f: &mut egui::epaint::text::FontsView<'_>, n: usize, mark: Mark| {
            let text = match mark {
                Mark::None => format!("{n:>6}"),
                Mark::Bookmark => format!(" {n:>5}"),
                Mark::Numbered(b) => format!("{b}{n:>5}"),
            };
            f.layout_job(LayoutJob::simple(text, font.clone(), color, f32::INFINITY))
        };
        let mark_of = |line: usize| {
            marked.binary_search_by_key(&line, |(l, _)| *l).map_or(Mark::None, |i| marked[i].1)
        };
        ui.fonts_mut(|f| {
            self.numbers.truncate(lines);
            // Lines marked before or now: their marks as they are now.
            for (line, _) in self.marked.iter().chain(&marked) {
                let mark = mark_of(*line);
                if let Some(entry) = self.numbers.get_mut(*line)
                    && entry.0 != mark
                {
                    *entry = (mark, number(f, *line + 1, mark));
                }
            }
            for line in self.numbers.len()..lines {
                let mark = mark_of(line);
                self.numbers.push((mark, number(f, line + 1, mark)));
            }
        });
        let mut galleys: Vec<Arc<Galley>> = self.numbers.iter().map(|(_, g)| g.clone()).collect();
        let empty = empty_job(font, color);
        galleys.push(ui.fonts_mut(|f| f.layout_job(empty)));
        let mut text = String::new();
        for g in &galleys {
            text.push_str(g.text());
            text.push('\n');
        }
        text.pop();
        let job = LayoutJob::simple(text, palette.font(), color, f32::INFINITY);
        let ppp = ui.ctx().pixels_per_point();
        let galley = Arc::new(Galley::concat(Arc::new(job), &galleys, ppp));
        self.marked = marked;
        self.numbers_galley = Some(galley.clone());
        galley
    }
}

/// An empty line: no text, one section for its font (its height).
fn empty_job(font: egui::FontId, color: Color32) -> LayoutJob {
    let mut job = LayoutJob::default();
    job.sections.push(LayoutSection {
        leading_space: 0.0,
        byte_range: egui::text::ByteIndex(0)..egui::text::ByteIndex(0),
        format: TextFormat::simple(font, color),
    });
    job.wrap.max_width = f32::INFINITY;
    job
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
    /// Whether it is underlined as the error's line.
    underlined: bool,
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

/// A line's layout: its tokens' pieces in their colors, underlined from its
/// first character that isn't a space to its last.
fn line_job(
    text: &str,
    tokens: &[Token],
    palette: &Palette,
    underline: Option<Color32>,
) -> LayoutJob {
    let font = palette.font();
    let mut job = LayoutJob::default();
    for t in tokens.iter().filter(|t| t.span.end > t.span.start) {
        let color = palette.colors[element(t.kind)];
        job.append(&text[t.span.clone()], 0.0, TextFormat::simple(font.clone(), color));
    }
    // (An empty line: the height of its font. An empty text has no
    // sections, as it had laid out whole.)
    if job.sections.is_empty() && !text.is_empty() {
        return empty_job(font, palette.colors[0]);
    }
    job.wrap.max_width = f32::INFINITY;
    if let Some(color) = underline {
        let own = job.text.clone();
        underline_line(&mut job, &own, 0, color);
    }
    job
}

/// A bookmark's icon (`bookmark`, as high as a line, in the numbers'
/// color) in the first place of each bookmarked line's number, of the
/// line numbers `numbers` drawn at `at` (those with a numbered bookmark's
/// number show that).
pub(crate) fn paint_bookmarks(
    ui: &Ui,
    numbers: &Galley,
    at: egui::Pos2,
    bookmarks: &std::collections::BTreeSet<usize>,
    numbered: &[Option<usize>; 10],
) {
    let weak = ui.visuals().weak_text_color();
    let color = numbers.job.sections.first().map_or(weak, |s| s.format.color);
    let clip = ui.clip_rect();
    for line in bookmarks {
        if numbered.contains(&Some(*line)) {
            continue;
        }
        let Some(row) = numbers.rows.get(*line) else { continue };
        let first = row.glyphs.first().map_or(row.rect().height(), |g| g.advance_width);
        let rect = row.rect().translate(at.to_vec2());
        let side = rect.height().min(mg_uikit::metrics::ICON);
        let square = egui::Rect::from_center_size(
            egui::pos2(rect.left() + first / 2.0, rect.center().y),
            egui::Vec2::splat(side),
        );
        if clip.intersects(square) {
            mg_uikit::Icon::Bookmark.paint(ui.painter(), square, color);
        }
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
    /// job: egui splits it at its lines itself) and line by line, and
    /// compares the rows: where each is, its size, its line end, its
    /// characters and their colors.
    fn same_as_whole(ctx: &egui::Context, text: &str, error_line: Option<usize>) {
        let palette = Palette::new(&ScriptStyle::default(), true, Color32::WHITE);
        let mut laid = LaidOut::default();
        let mut whole = None;
        let mut lines = None;
        run(ctx, |ui| {
            laid.begin(ui, &palette, Color32::RED);
            lines = Some(laid.text(ui, text, &palette, error_line));
            let mut job = highlight(text, &palette);
            job.wrap.max_width = f32::INFINITY;
            if let Some(line) = error_line {
                underline_line(&mut job, text, line, Color32::RED);
            }
            whole = Some(ui.fonts_mut(|f| f.layout_job(job)));
        });
        let (whole, lines) = (whole.unwrap(), lines.unwrap());
        assert_eq!(lines.text(), text);
        assert_eq!(whole.rows.len(), lines.rows.len(), "{text:?}");
        for (i, (a, b)) in whole.rows.iter().zip(&lines.rows).enumerate() {
            let at = format!("row {i} of {text:?}");
            assert_eq!(a.pos, b.pos, "{at}");
            assert_eq!(a.size, b.size, "{at}");
            assert_eq!(a.ends_with_newline, b.ends_with_newline, "{at}");
            let chars = |r: &egui::epaint::text::PlacedRow| {
                r.glyphs.iter().map(|g| (g.chr, g.pos)).collect::<Vec<_>>()
            };
            assert_eq!(chars(a), chars(b), "{at}");
            assert_eq!(a.visuals.mesh.vertices, b.visuals.mesh.vertices, "{at}: colors");
        }
        assert_eq!(whole.rect, lines.rect);
        assert_eq!(whole.size(), lines.size());
        assert_eq!(whole.intrinsic_size(), lines.intrinsic_size());
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
            same_as_whole(&ctx, text, None);
            same_as_whole(&ctx, text, Some(1));
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
        same_as_whole(&egui::Context::default(), &text, Some(200));
    }

    /// Each edit lays out only the lines whose text or colors it changed,
    /// and what it gives is what laying the new text out whole gives.
    #[test]
    fn an_edit_lays_out_the_lines_it_changed() {
        let ctx = egui::Context::default();
        let palette = Palette::new(&ScriptStyle::default(), true, Color32::WHITE);
        let base: String = (0..200).map(|i| format!("int n{i} = {i}; // line {i}\n")).collect();
        let mut laid = LaidOut::default();
        let mut frame = |text: &str, error_line: Option<usize>| {
            let mut out = None;
            run(&ctx, |ui| {
                laid.begin(ui, &palette, Color32::RED);
                let before = laid.galleys.clone();
                let g = laid.text(ui, text, &palette, error_line);
                let kept = laid.galleys.iter().filter(|g| before.iter().any(|b| Arc::ptr_eq(b, g)));
                out = Some((g, laid.galleys.len() - kept.count()));
            });
            out.unwrap()
        };
        let (_, made) = frame(&base, None);
        assert_eq!(made, 201, "every line at first, and the empty one after the last");
        // A character typed on a line: that line.
        let typed = base.replacen("int n50 ", "int n50x ", 1);
        assert_eq!(frame(&typed, None).1, 1);
        // A line added: that line.
        let added = typed.replacen("int n60 ", "int n60_;\nint n60 ", 1);
        assert_eq!(frame(&added, None).1, 1);
        // The error's line: that line, and the line underlined before.
        assert_eq!(frame(&added, Some(10)).1, 1);
        assert_eq!(frame(&added, Some(12)).1, 2);
        // A block comment opened: the lines it colors, up to where it ends.
        let opened = added.replacen("int n100 ", "/* int n100 ", 1).replacen(
            "int n104 = 104;",
            "int n104 = 104; */",
            1,
        );
        assert_eq!(frame(&opened, Some(12)).1, 5);
        // Lines changed far apart (Replace All): those lines.
        let replaced = opened.replace("int n150 ", "int m150 ").replace("int n3 ", "int m3 ");
        assert_eq!(frame(&replaced, Some(12)).1, 2);
        // Each laid out as the whole text is.
        let (g, _) = frame(&replaced, Some(12));
        assert_eq!(g.text(), replaced);
        same_as_whole(&ctx, &replaced, Some(12));
    }

    /// The line numbers, laid out a number at a time, as the label of all
    /// of them was: each mark in its number's first place.
    #[test]
    fn line_numbers_laid_out_one_at_a_time_are_as_all_together() {
        let ctx = egui::Context::default();
        let palette = Palette::new(&ScriptStyle::default(), true, Color32::WHITE);
        let mut laid = LaidOut::default();
        let bookmarks: std::collections::BTreeSet<usize> = [2, 6, 11].into();
        let mut numbered = [None; 10];
        numbered[2] = Some(11);
        let mut frame = |lines: usize, bookmarks: &std::collections::BTreeSet<usize>| {
            let mut out = None;
            run(&ctx, |ui| {
                laid.begin(ui, &palette, Color32::RED);
                let g = laid.numbers(ui, lines, bookmarks, &numbered, &palette);
                let text: String = (1..=lines)
                    .map(|n| {
                        if let Some(b) = numbered.iter().position(|l| *l == Some(n - 1)) {
                            format!("{b}{n:>5}\n")
                        } else if bookmarks.contains(&(n - 1)) {
                            format!(" {n:>5}\n")
                        } else {
                            format!("{n:>6}\n")
                        }
                    })
                    .collect();
                let color = ui.visuals().weak_text_color();
                let job = LayoutJob::simple(text, palette.font(), color, f32::INFINITY);
                out = Some((g, ui.fonts_mut(|f| f.layout_job(job))));
            });
            out.unwrap()
        };
        for (lines, marks) in [(20, bookmarks.clone()), (25, [3].into()), (12, [3, 7].into())] {
            let (ours, all) = frame(lines, &marks);
            assert_eq!(ours.text(), all.text());
            assert_eq!(ours.rows.len(), all.rows.len());
            for (a, b) in ours.rows.iter().zip(&all.rows) {
                assert_eq!(
                    (a.pos, a.size, a.ends_with_newline),
                    (b.pos, b.size, b.ends_with_newline)
                );
                assert_eq!(a.visuals.mesh.vertices, b.visuals.mesh.vertices);
            }
            assert_eq!(ours.size(), all.size());
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
                laid.begin(ui, &palette, Color32::RED);
                out = Some(laid.text(ui, "void main()\n{\n}\n", &palette, None));
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
