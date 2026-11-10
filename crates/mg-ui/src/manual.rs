//! Help › User Manual: the manual (`docs/manual`), built into the program,
//! a chapter at a time beside the list of chapters; a link to another
//! chapter opens it in place. And Help › About.

use egui::Ui;
use egui_commonmark::{CommonMarkCache, CommonMarkViewer};

use crate::Moonglow;

/// The chapters: file name and text, the contents first.
pub(crate) const CHAPTERS: [(&str, &str); 19] = [
    ("README.md", include_str!("../../../docs/manual/README.md")),
    ("01-getting-started.md", include_str!("../../../docs/manual/01-getting-started.md")),
    ("02-main-window.md", include_str!("../../../docs/manual/02-main-window.md")),
    ("03-modules.md", include_str!("../../../docs/manual/03-modules.md")),
    ("04-areas.md", include_str!("../../../docs/manual/04-areas.md")),
    ("05-blueprints.md", include_str!("../../../docs/manual/05-blueprints.md")),
    ("06-conversations.md", include_str!("../../../docs/manual/06-conversations.md")),
    ("07-scripts.md", include_str!("../../../docs/manual/07-scripts.md")),
    ("08-journal-and-factions.md", include_str!("../../../docs/manual/08-journal-and-factions.md")),
    ("09-build-and-test.md", include_str!("../../../docs/manual/09-build-and-test.md")),
    ("10-options.md", include_str!("../../../docs/manual/10-options.md")),
    ("11-command-line.md", include_str!("../../../docs/manual/11-command-line.md")),
    ("12-differences.md", include_str!("../../../docs/manual/12-differences.md")),
    ("13-troubleshooting.md", include_str!("../../../docs/manual/13-troubleshooting.md")),
    ("14-coming-from-aurora.md", include_str!("../../../docs/manual/14-coming-from-aurora.md")),
    ("15-plugins.md", include_str!("../../../docs/manual/15-plugins.md")),
    ("16-writing-plugins.md", include_str!("../../../docs/manual/16-writing-plugins.md")),
    ("17-plugin-api.md", include_str!("../../../docs/manual/17-plugin-api.md")),
    ("18-nui-creator.md", include_str!("../../../docs/manual/18-nui-creator.md")),
];

/// A chapter without its frontmatter. The manual is part of an Open
/// Knowledge Format bundle (`docs/index.md`): each chapter opens with a
/// YAML block between `---` lines, which is for tools and not for the
/// reader.
pub(crate) fn body(text: &str) -> &str {
    let mut end = 0;
    for (i, line) in text.split_inclusive('\n').enumerate() {
        end += line.len();
        match (i, line.trim_end() == "---") {
            (0, false) => return text,
            (0, true) => {}
            (_, true) => return text[end..].trim_start(),
            _ => {}
        }
    }
    text
}

/// A chapter's title: its first heading.
pub(crate) fn title(text: &str) -> &str {
    body(text).lines().find_map(|l| l.strip_prefix("# ")).unwrap_or_default()
}

/// The pictures the chapters show, by the name they are given there, built
/// in as the chapters are (the viewer has no loader for a file beside the
/// chapter: a picture was a failed image).
const PICTURES: [(&str, &[u8]); 1] = [(
    "../images/2da-table-described.png",
    include_bytes!("../../../docs/images/2da-table-described.png"),
)];

/// The manual tab's state.
#[derive(Default)]
pub(crate) struct Manual {
    pub(crate) chapter: usize,
    cache: CommonMarkCache,
    /// The chapter as shown, and the chapter and width it was made for.
    shown: Option<(usize, u32, Shown)>,
    /// The heading a link asked for: scrolled to as the chapter is shown.
    scroll_to: Option<ScrollTo>,
    /// The [`PICTURES`] shown so far (`None`: one that does not read).
    pictures: std::collections::HashMap<usize, Option<egui::TextureHandle>>,
}

/// A chapter as shown: its [`blocks`], and each block's heading
/// ([`anchors`]).
type Shown = (Vec<Block>, Vec<Option<String>>);

/// A heading being scrolled to.
#[derive(Debug)]
struct ScrollTo {
    /// Its name in links ([`slug`]).
    heading: String,
    /// How far down the page it was when it was last scrolled to.
    at: Option<f32>,
    /// The frames it was scrolled to in so far.
    frames: u8,
}

impl std::fmt::Debug for Manual {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Manual { chapter, scroll_to, .. } = self;
        f.debug_struct("Manual")
            .field("chapter", chapter)
            .field("scroll_to", scroll_to)
            .finish_non_exhaustive()
    }
}

/// A stretch of a chapter as the viewer shows it: Markdown, drawn by
/// `egui_commonmark` (from a heading to the next, so that a link can
/// scroll to it), a table, drawn here with its cells wrapped (the viewer
/// draws a cell on one line, so a table wider than the page was cut off
/// at its edge, and the text after it too), or a picture.
#[derive(Debug, Clone, PartialEq)]
enum Block {
    Text(String),
    Table {
        head: Vec<String>,
        rows: Vec<Vec<String>>,
        /// Each column's width, in points.
        widths: Vec<f32>,
    },
    /// One of [`PICTURES`], and what it shows in words.
    Picture {
        at: usize,
        alt: String,
    },
}

/// A heading's text, if `line` is one (`## Tiles`).
fn heading(line: &str) -> Option<&str> {
    let text = line.trim_start_matches('#');
    let level = line.len() - text.len();
    ((1..=6).contains(&level) && text.starts_with(' ')).then(|| text.trim())
}

/// A heading's name in a link (`#the-camera`), as the manual's links are
/// written (GitHub's rule): lower case, letters, digits, `-` and `_` kept,
/// a space a `-`, everything else left out.
fn slug(heading: &str) -> String {
    heading
        .chars()
        .filter_map(|c| match c {
            ' ' => Some('-'),
            '-' | '_' => Some(c),
            c if c.is_alphanumeric() => Some(c),
            _ => None,
        })
        .flat_map(char::to_lowercase)
        .collect()
}

/// A chapter's headings by their names in links, in order (a name a
/// second heading has too is numbered: `keys-1`), outside its code blocks.
fn heading_slugs(text: &str) -> Vec<String> {
    let mut seen = std::collections::HashMap::<String, usize>::new();
    let mut fenced = false;
    let mut out = Vec::new();
    for line in text.lines() {
        if line.trim_start().starts_with("```") {
            fenced = !fenced;
        }
        if let Some(heading) = heading(line).filter(|_| !fenced) {
            let name = slug(heading);
            let n = seen.entry(name.clone()).or_insert(0);
            out.push(if *n == 0 { name } else { format!("{name}-{n}") });
            *n += 1;
        }
    }
    out
}

/// The heading each block opens with, by its name in links: what a link
/// to it scrolls to.
fn anchors(blocks: &[Block]) -> Vec<Option<String>> {
    let opens =
        |b: &Block| matches!(b, Block::Text(t) if t.lines().next().and_then(heading).is_some());
    let text: String = (blocks.iter())
        .filter_map(|b| match b {
            Block::Text(t) => Some(t.as_str()),
            _ => None,
        })
        .collect();
    let mut slugs = heading_slugs(&text).into_iter();
    // (A block holds one heading, its first line: `blocks` cuts at each.)
    blocks.iter().map(|b| opens(b).then(|| slugs.next()).flatten()).collect()
}

/// Where a link of the manual leads: the chapter, and the heading in it
/// (`03-modules.md`, `03-modules.md#claimed-areas`, or `#tiles` in the
/// chapter `shown`). `None` for a link that leaves the manual.
fn destination(link: &str, shown: usize) -> Option<(usize, Option<&str>)> {
    let (file, heading) = match link.split_once('#') {
        Some((file, heading)) => (file, Some(heading).filter(|h| !h.is_empty())),
        None => (link, None),
    };
    let chapter = match file {
        "" => shown,
        file => CHAPTERS.iter().position(|(f, _)| *f == file)?,
    };
    Some((chapter, heading))
}

/// Every link the chapters hold, as written (what follows `](`).
fn links(text: &str) -> impl Iterator<Item = &str> {
    text.split("](").skip(1).filter_map(|rest| rest.split_once(')')).map(|(link, _)| link)
}

/// The links that stay in the manual, each once: the viewer is told of
/// them, and hands the others to the system's browser.
fn own_links() -> &'static [&'static str] {
    static LINKS: std::sync::LazyLock<Vec<&'static str>> = std::sync::LazyLock::new(|| {
        let own: std::collections::BTreeSet<&str> = (CHAPTERS.iter())
            .flat_map(|(_, text)| links(text))
            .filter(|link| destination(link, 0).is_some())
            .collect();
        own.into_iter().collect()
    });
    &LINKS
}

/// The narrowest a table's column is drawn, its words wrapped: a table
/// whose columns can't each have this much is shown as a list.
const NARROWEST: f32 = 90.0;

/// The room a table's column takes besides its text (the space between
/// columns), and the table's own (its frame).
const COLUMN_PADDING: f32 = 10.0;
const TABLE_FRAME: f32 = 16.0;

/// Each column's width for a table whose columns' text is `natural` wide
/// (drawn on one line), in a page `width` wide: as wide as its text where
/// the page has room for all; else the room shared out by the square
/// root of each one's width (a column of long text wider than one of
/// keys, but not by as much as its text is longer), a column narrower
/// than its share keeping its own width and the others sharing what it
/// leaves, none narrower than [`NARROWEST`]. `None` where the columns
/// can't each be that.
fn column_widths(natural: &[f32], width: f32) -> Option<Vec<f32>> {
    let n = natural.len();
    let room = width - TABLE_FRAME - COLUMN_PADDING * n as f32;
    if natural.iter().sum::<f32>() <= room {
        return Some(natural.to_vec());
    }
    if n == 0 || room < NARROWEST * n as f32 {
        return None;
    }
    let mut widths: Vec<Option<f32>> = vec![None; n];
    loop {
        let open: Vec<usize> = (0..n).filter(|&c| widths[c].is_none()).collect();
        if open.is_empty() {
            break;
        }
        let left = room - widths.iter().flatten().sum::<f32>();
        let weight = |c: usize| natural[c].max(1.0).sqrt();
        let total: f32 = open.iter().map(|&c| weight(c)).sum();
        let share = |c: usize| left * weight(c) / total;
        let mut fixed = false;
        for &c in &open {
            if natural[c] <= share(c) {
                widths[c] = Some(natural[c]);
                fixed = true;
            } else if share(c) < NARROWEST {
                widths[c] = Some(NARROWEST);
                fixed = true;
            }
        }
        if !fixed {
            for c in open {
                widths[c] = Some(share(c));
            }
            break;
        }
    }
    Some(widths.into_iter().map(|w| w.unwrap_or(NARROWEST)).collect())
}

/// A table row's cells: split at `|`, but not at one escaped (`\|`) or
/// inside a code span.
fn cells(row: &str) -> Vec<String> {
    let row = row.trim();
    let row = row.strip_prefix('|').unwrap_or(row);
    let mut out = vec![String::new()];
    let (mut code, mut chars) = (false, row.chars().peekable());
    while let Some(c) = chars.next() {
        match c {
            '`' => code = !code,
            '\\' if chars.peek() == Some(&'|') => {
                chars.next();
                out.last_mut().expect("one cell").push('|');
                continue;
            }
            '|' if !code => {
                out.push(String::new());
                continue;
            }
            _ => {}
        }
        out.last_mut().expect("one cell").push(c);
    }
    // (What follows the last `|` is no cell.)
    if out.len() > 1 && out.last().is_some_and(|c| c.trim().is_empty()) {
        out.pop();
    }
    out.into_iter().map(|c| c.trim().to_string()).collect()
}

/// A cell's text as it is drawn: its prose and its code apart (of a link
/// only its text shows, and no `*`).
fn drawn_text(cell: &str) -> (String, String) {
    let (mut prose, mut code_text) = (String::new(), String::new());
    let (mut code, mut chars) = (false, cell.chars().peekable());
    while let Some(c) = chars.next() {
        match c {
            '`' => code = !code,
            '*' if !code => {}
            '[' if !code => {}
            ']' if !code && chars.peek() == Some(&'(') => {
                for c in chars.by_ref() {
                    if c == ')' {
                        break;
                    }
                }
            }
            c if code => code_text.push(c),
            c => prose.push(c),
        }
    }
    (prose, code_text)
}

/// About how wide a cell's text is drawn, in points, without the fonts at
/// hand: code is wider than prose.
#[cfg(test)]
fn drawn_width(cell: &str) -> f32 {
    let (prose, code) = drawn_text(cell);
    6.0 * prose.chars().count() as f32 + 8.0 * code.chars().count() as f32
}

/// How wide a cell's text is drawn on one line, in the manual's fonts
/// (and a little room: a heading's is bold, code has its background).
fn measured(ctx: &egui::Context, style: &egui::Style, cell: &str) -> f32 {
    let (prose, code) = drawn_text(cell);
    let width = |text: String, font: egui::TextStyle| {
        let font = font.resolve(style);
        ctx.fonts_mut(|f| f.layout_no_wrap(text, font, egui::Color32::WHITE).size().x)
    };
    let code_width = if code.is_empty() { 0.0 } else { width(code, egui::TextStyle::Monospace) };
    width(prose, egui::TextStyle::Body) + code_width + 8.0
}

/// A chapter as the viewer shows it in `width` points: its Markdown, and
/// its tables apart, their columns' widths fitted to the page
/// ([`column_widths`]). A table too wide even so is shown as a list (a
/// row an item; with more than two columns, the further cells under it,
/// each after its column's heading).
fn blocks(text: &str, width: f32, measure: &dyn Fn(&str) -> f32) -> Vec<Block> {
    let is_row = |l: &str| l.trim_start().starts_with('|');
    let is_rule = |l: &str| {
        is_row(l) && l.contains('-') && l.chars().all(|c| matches!(c, '|' | '-' | ':' | ' '))
    };
    let lines: Vec<&str> = text.lines().collect();
    let mut blocks = Vec::new();
    let (mut out, mut i, mut fenced) = (String::with_capacity(text.len()), 0, false);
    while i < lines.len() {
        let line = lines[i];
        if line.trim_start().starts_with("```") {
            fenced = !fenced;
        }
        let table = !fenced && is_row(line) && lines.get(i + 1).is_some_and(|l| is_rule(l));
        // A heading opens a block of its own, which a link scrolls to.
        if !fenced && heading(line).is_some() {
            if !out.trim().is_empty() {
                blocks.push(Block::Text(std::mem::take(&mut out)));
            }
            out.clear();
        }
        // A picture on a line of its own, where it is built in (one that
        // is not stays as written).
        if !fenced && let Some((at, alt)) = picture(line) {
            if !out.trim().is_empty() {
                blocks.push(Block::Text(std::mem::take(&mut out)));
            }
            out.clear();
            blocks.push(Block::Picture { at, alt: alt.into() });
            i += 1;
            continue;
        }
        if !table {
            out.push_str(line);
            out.push('\n');
            i += 1;
            continue;
        }
        let end = (i..lines.len()).find(|&j| !is_row(lines[j])).unwrap_or(lines.len());
        let head = cells(line);
        let rows: Vec<Vec<String>> = lines[i + 2..end].iter().map(|l| cells(l)).collect();
        let columns = head.len().max(rows.iter().map(Vec::len).max().unwrap_or(0));
        let widest = |c: usize| {
            rows.iter()
                .chain([&head])
                .filter_map(|r| r.get(c))
                .map(|cell| measure(cell))
                .fold(0.0, f32::max)
        };
        let natural: Vec<f32> = (0..columns).map(widest).collect();
        if let Some(widths) = column_widths(&natural, width) {
            if !out.trim().is_empty() {
                blocks.push(Block::Text(std::mem::take(&mut out)));
            }
            out.clear();
            blocks.push(Block::Table { head, rows, widths });
        } else {
            if columns == 2 && head.iter().all(|h| !h.is_empty()) {
                out.push_str(&format!("**{}** — **{}**\n\n", head[0], head[1]));
            }
            for row in &rows {
                let Some(first) = row.first() else { continue };
                match (columns, row.get(1).filter(|c| !c.is_empty())) {
                    (2, Some(second)) => out.push_str(&format!("- {first} — {second}\n")),
                    _ => out.push_str(&format!("- {first}\n")),
                }
                if columns > 2 {
                    for (c, cell) in row.iter().enumerate().skip(1).filter(|(_, c)| !c.is_empty()) {
                        match head.get(c).filter(|h| !h.is_empty()) {
                            Some(h) => out.push_str(&format!("  - {h}: {cell}\n")),
                            None => out.push_str(&format!("  - {cell}\n")),
                        }
                    }
                }
            }
            out.push('\n');
        }
        i = end;
    }
    if !out.trim().is_empty() {
        blocks.push(Block::Text(out));
    }
    blocks
}

/// A line that is a picture and nothing else (`![what it shows](file)`),
/// where the file is one of [`PICTURES`]: which, and what it shows.
fn picture(line: &str) -> Option<(usize, &str)> {
    let (alt, file) = line.trim().strip_prefix("![")?.strip_suffix(')')?.split_once("](")?;
    Some((PICTURES.iter().position(|(name, _)| *name == file)?, alt))
}

/// Where a link clicked in what was just shown leads (the viewer forgets
/// a click as it shows the next block): the chapter, and the heading.
fn link_clicked(ui: &Ui, cache: &CommonMarkCache, shown: usize) -> Option<(usize, Option<String>)> {
    // (No click, the pointer's or a screen reader's, no link clicked: the
    // links are not gone through after each block of every frame.)
    let asked = |e: &egui::Event| matches!(e, egui::Event::AccessKitActionRequest(_));
    if !ui.input(|i| i.pointer.any_click() || i.events.iter().any(asked)) {
        return None;
    }
    let (link, _) = cache.link_hooks().iter().find(|(_, clicked)| **clicked)?;
    let (chapter, heading) = destination(link, shown)?;
    Some((chapter, heading.map(str::to_owned)))
}

/// A picture of the manual, as wide as the page at most.
fn picture_ui(
    ui: &mut Ui,
    kept: &mut std::collections::HashMap<usize, Option<egui::TextureHandle>>,
    at: usize,
    alt: &str,
) {
    let texture = kept.entry(at).or_insert_with(|| {
        let (name, bytes) = PICTURES.get(at)?;
        let rgba = mg_image::png::read_picture(bytes, 4096 * 4096).ok()?;
        let size = [rgba.width as usize, rgba.height as usize];
        let image = egui::ColorImage::from_rgba_unmultiplied(size, &rgba.data);
        Some(ui.ctx().load_texture(format!("manual/{name}"), image, egui::TextureOptions::LINEAR))
    });
    match texture {
        Some(texture) => {
            let image = egui::Image::new(&*texture).max_width(ui.available_width());
            ui.add(image.alt_text(alt));
        }
        None => {
            ui.weak(alt);
        }
    }
    ui.add_space(8.0);
}

/// A table, its cells wrapped to its columns' widths, every other row
/// shaded, its head in bold.
fn table_ui(
    ui: &mut Ui,
    cache: &mut CommonMarkCache,
    head: &[String],
    rows: &[Vec<String>],
    widths: &[f32],
    shown: usize,
    go: &mut Option<(usize, Option<String>)>,
) {
    let shade = ui.visuals().faint_bg_color;
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.spacing_mut().item_spacing.y = 2.0;
        let bold = |c: &String| if c.is_empty() { String::new() } else { format!("**{c}**") };
        let head: Vec<String> = head.iter().map(bold).collect();
        for (r, row) in std::iter::once(&head).chain(rows).enumerate() {
            let behind = ui.painter().add(egui::Shape::Noop);
            let drawn = ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = COLUMN_PADDING;
                for (c, width) in widths.iter().enumerate() {
                    let cell = row.get(c).map_or("", String::as_str);
                    let size = egui::vec2(*width, 0.0);
                    let layout = egui::Layout::top_down(egui::Align::Min);
                    ui.allocate_ui_with_layout(size, layout, |ui| {
                        ui.set_width(*width);
                        ui.push_id((r, c), |ui| {
                            CommonMarkViewer::new().show(ui, cache, cell);
                        });
                        if go.is_none() {
                            *go = link_clicked(ui, cache, shown);
                        }
                    });
                }
            });
            if r % 2 == 1 {
                let rect = drawn.response.rect.expand2(egui::vec2(4.0, 0.0));
                ui.painter().set(behind, egui::epaint::RectShape::filled(rect, 2.0, shade));
            }
        }
    });
}

pub(crate) fn ui(app: &mut Moonglow, ui: &mut Ui) {
    let m = &mut app.manual;
    let mut go = None;
    ui.horizontal_top(|ui| {
        ui.vertical(|ui| {
            ui.set_width(190.0);
            for (i, (_, text)) in CHAPTERS.iter().enumerate() {
                let name = if i == 0 { "Contents" } else { title(text) };
                if ui.selectable_label(m.chapter == i, name).clicked() {
                    go = Some((i, None));
                }
            }
        });
        ui.separator();
        let chapter = m.chapter.min(CHAPTERS.len() - 1);
        // Every link that stays in the manual, to a chapter or to a heading
        // (of another chapter or of this one): the viewer hands a link it
        // was not told of to the system's browser, which was asked to open
        // `03-modules.md#claimed-areas`.
        if m.cache.link_hooks().is_empty() {
            for link in own_links() {
                m.cache.add_link_hook(*link);
            }
        }
        ui.vertical(|ui| {
            egui::ScrollArea::vertical().id_salt(("manual", m.chapter)).auto_shrink(false).show(
                ui,
                |ui| {
                    let width = ui.available_width().min(760.0);
                    ui.set_max_width(width);
                    // (Made again when the page is wider or narrower by a step.)
                    let step = (width / 20.0) as u32;
                    if !matches!(&m.shown, Some((c, s, ..)) if (*c, *s) == (chapter, step)) {
                        let (ctx, style) = (ui.ctx().clone(), ui.style().clone());
                        let measure = |cell: &str| measured(&ctx, &style, cell);
                        let shown = blocks(body(CHAPTERS[chapter].1), step as f32 * 20.0, &measure);
                        let anchors = anchors(&shown);
                        m.shown = Some((chapter, step, (shown, anchors)));
                    }
                    let Some((_, _, (shown, anchors))) = &m.shown else { return };
                    // (A heading the chapter doesn't have: its top.)
                    let scroll_to = m.scroll_to.as_ref().map(|to| to.heading.clone());
                    let mut heading_at = None;
                    for (i, block) in shown.iter().enumerate() {
                        // (A heading is as far under the text before it as
                        // when one block held both.)
                        if i > 0 && anchors.get(i).is_some_and(Option::is_some) {
                            ui.add_space(-ui.spacing().item_spacing.y);
                        }
                        if scroll_to.is_some() && anchors.get(i) == Some(&scroll_to) {
                            heading_at = Some(ui.cursor().top());
                        }
                        ui.push_id(("block", i), |ui| match block {
                            Block::Text(text) => {
                                CommonMarkViewer::new().show(ui, &mut m.cache, text);
                                if go.is_none() {
                                    go = link_clicked(ui, &m.cache, chapter);
                                }
                            }
                            Block::Table { head, rows, widths } => {
                                table_ui(ui, &mut m.cache, head, rows, widths, chapter, &mut go);
                                ui.add_space(8.0);
                            }
                            Block::Picture { at, alt } => picture_ui(ui, &mut m.pictures, *at, alt),
                        });
                    }
                    // Asked for after the blocks (a code block is a scroll
                    // area of its own, and one shown after the heading takes
                    // what was asked for before it), and again until the
                    // heading stays where it is in the page: a chapter new
                    // to the window is laid out a little higher or lower in
                    // its first frame than after.
                    let Some(to) = &mut m.scroll_to else { return };
                    let Some(top) = heading_at else {
                        m.scroll_to = None;
                        return;
                    };
                    let across = ui.max_rect().x_range();
                    let heading = egui::Rect::from_x_y_ranges(across, top..=top);
                    ui.scroll_to_rect(heading, Some(egui::Align::TOP));
                    let in_page = top - ui.min_rect().top();
                    to.frames += 1;
                    if to.at.is_some_and(|at| (at - in_page).abs() < 0.5) || to.frames >= 6 {
                        m.scroll_to = None;
                    } else {
                        to.at = Some(in_page);
                        ui.ctx().request_repaint();
                    }
                },
            );
        });
    });
    if let Some((chapter, heading)) = go {
        // (Another chapter is shown from its top, in a scroll area of its
        // own; a heading of this one is scrolled to.)
        m.chapter = chapter;
        m.scroll_to = heading.map(|heading| ScrollTo { heading, at: None, frames: 0 });
        ui.ctx().request_repaint();
    }
}

/// Help › About Moonglow Toolset, while open.
pub(crate) fn about_window(app: &mut Moonglow, ctx: &egui::Context) {
    if !app.about {
        return;
    }
    let mut open = true;
    let mut check = false;
    crate::widgets::dialog(ctx, "About Moonglow Toolset", &mut open)
        .collapsible(false)
        .resizable(false)
        .show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.heading("Moonglow Toolset");
                ui.label(format!("Version {}", mg_core::release::version()));
                if ui
                    .button("Check for Updates…")
                    .on_hover_text("Ask GitHub whether a newer Moonglow is out")
                    .clicked()
                {
                    check = true;
                }
                ui.add_space(6.0);
                ui.label("A module toolset for Neverwinter Nights: Enhanced Edition.");
            });
            ui.add_space(10.0);
            ui.label(
                "Moonglow is free software: you can redistribute it and/or modify it under \
                 the terms of the GNU General Public License, version 3. It comes with no \
                 warranty.",
            );
            ui.add_space(6.0);
            ui.label(
                "It includes Beamdog's NWScript compiler and (for plugins) Luau, and is built \
                 with egui, wgpu, symphonia, rodio and other open-source libraries; their \
                 licenses are in THIRD-PARTY-LICENSES.txt, beside the program.",
            );
            ui.add_space(6.0);
            ui.label(
                "Moonglow is not affiliated with Beamdog or Wizards of the Coast, and \
                 contains no game data: it reads your installation of the game.",
            );
        });
    if !open {
        app.about = false;
    }
    if check {
        crate::updates::check_now(app);
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Block, CHAPTERS, COLUMN_PADDING, NARROWEST, PICTURES, TABLE_FRAME, anchors, blocks, body,
        cells, column_widths, destination, drawn_width, heading, heading_slugs, links, own_links,
        picture, slug, title,
    };

    /// A chapter is shown from its heading, not from its frontmatter.
    #[test]
    fn a_chapter_is_shown_without_its_frontmatter() {
        assert_eq!(body("---\ntype: Manual Page\n---\n\n# Keys\n"), "# Keys\n");
        assert_eq!(body("---\r\ntitle: a\r\n---\r\n# Keys\r\n"), "# Keys\r\n");
        assert_eq!(body("# Keys\n\n---\n\nMore.\n"), "# Keys\n\n---\n\nMore.\n");
        assert_eq!(body("---\nnever closed\n"), "---\nnever closed\n");
        for (file, text) in CHAPTERS {
            // (A Windows checkout has the chapters with CRLF line ends.)
            assert!(body(text).len() < text.len(), "{file} has frontmatter");
            assert!(body(text).starts_with("# "), "{file} is shown from its heading");
            let crlf = text.replace("\r\n", "\n").replace('\n', "\r\n");
            assert!(body(&crlf).starts_with("# "), "{file} is shown from its heading (CRLF)");
            assert!(!title(text).is_empty(), "{file}");
        }
    }

    fn text(s: &str) -> Block {
        Block::Text(s.to_string())
    }

    /// A table is shown apart from the text around it, its columns as wide
    /// as their text where the page has room, else sharing the page and
    /// their cells wrapped; one whose columns can't each have room enough
    /// to wrap in is a list. What is in a code block is left alone.
    #[test]
    fn a_table_s_cells_are_wrapped_to_the_page() {
        let narrow = "Keys:\n\n| Keys | Command |\n| --- | --- |\n| Ctrl+N | New module |\n";
        let [before, Block::Table { head, rows, widths }] =
            &blocks(narrow, 600.0, &drawn_width)[..]
        else {
            panic!("{:?}", blocks(narrow, 600.0, &drawn_width));
        };
        assert_eq!(before, &text("Keys:\n\n"));
        assert_eq!(head, &["Keys", "Command"]);
        assert_eq!(rows, &[vec!["Ctrl+N".to_string(), "New module".to_string()]]);
        assert_eq!(widths, &[36.0, 60.0], "each as wide as its text");
        // Too narrow for each column to wrap in: a list.
        assert_eq!(
            blocks(narrow, 100.0, &drawn_width),
            [text("Keys:\n\n**Keys** — **Command**\n\n- Ctrl+N — New module\n\n")]
        );
        let wide =
            "Before.\n\n| | |\n| --- | --- |\n| `a\\|b` | one `x | y` two |\n| c | |\n\nAfter.\n";
        assert_eq!(
            blocks(wide, 60.0, &drawn_width),
            [text("Before.\n\n- `a|b` — one `x | y` two\n- c\n\n\nAfter.\n")]
        );
        let three = "| What | Linux | macOS |\n| --- | --- | --- |\n| Settings | `~/a` | `~/b` |\n";
        assert_eq!(
            blocks(three, 60.0, &drawn_width),
            [text("- Settings\n  - Linux: `~/a`\n  - macOS: `~/b`\n\n")]
        );
        let code = "```text\n| not | a table |\n| --- | --- |\n| x | y |\n```\n";
        assert_eq!(blocks(code, 10.0, &drawn_width), [text(code)]);
        assert_eq!(cells("| a | b |"), ["a", "b"]);
        assert_eq!(cells("| a | |"), ["a", ""]);
    }

    /// The page shared out: a column of keys narrower than one of long
    /// text, but not by as much as its text is shorter; one narrower than
    /// its share keeps its own width.
    #[test]
    fn a_table_s_columns_share_the_page() {
        let room = |n: usize| 680.0 - TABLE_FRAME - COLUMN_PADDING * n as f32;
        let w = column_widths(&[360.0, 1380.0], 680.0).unwrap();
        assert!((w.iter().sum::<f32>() - room(2)).abs() < 0.01, "{w:?}");
        assert!(w[0] > 200.0 && w[0] < 300.0 && w[1] > w[0], "{w:?}");
        let w = column_widths(&[40.0, 900.0, 900.0], 680.0).unwrap();
        assert_eq!(w[0], 40.0, "the narrow column keeps its width");
        assert!((w[1] - w[2]).abs() < 0.01 && (w.iter().sum::<f32>() - room(3)).abs() < 0.01);
        assert_eq!(column_widths(&[10.0, 20.0], 680.0), Some(vec![10.0, 20.0]));
        assert_eq!(column_widths(&[500.0; 8], 680.0), None, "a list");
        let w = column_widths(&[5.0, 3000.0], 300.0).unwrap();
        assert!(w.iter().all(|w| *w >= 5.0) && w[1] >= NARROWEST, "{w:?}");
    }

    /// Every chapter's tables are shown as tables on a page of the
    /// window's usual width, none wider than the page.
    #[test]
    fn every_chapter_fits_the_page() {
        for (file, text) in CHAPTERS {
            for block in blocks(body(text), 680.0, &drawn_width) {
                if let Block::Table { widths, .. } = block {
                    let needs: f32 = widths.iter().map(|w| w + COLUMN_PADDING).sum::<f32>();
                    assert!(needs + TABLE_FRAME <= 680.5, "{file}: {widths:?}");
                }
            }
            let listed = blocks(body(text), 680.0, &drawn_width).iter().any(|b| match b {
                Block::Text(t) => t.lines().any(|l| l.trim_start().starts_with("| ---")),
                Block::Table { .. } | Block::Picture { .. } => false,
            });
            assert!(!listed, "{file}: a table left in the text");
        }
    }

    /// Every chapter of `docs/manual` is built in, and every link between
    /// chapters names one of them.
    #[test]
    fn links_between_chapters_resolve() {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/manual");
        let mut files: Vec<String> = std::fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            // (`index.md` and `log.md` are the bundle's listing and history, not chapters.)
            .filter(|f| f.ends_with(".md") && f != "index.md" && f != "log.md")
            .collect();
        files.sort();
        let mut built: Vec<String> = CHAPTERS.iter().map(|(f, _)| f.to_string()).collect();
        built.sort();
        assert_eq!(files, built, "chapters in docs/manual and in CHAPTERS");
        // Every link is the web's or the manual's own: a chapter, or a
        // heading of one (of the chapter it is in, for `#heading`), which
        // the viewer is told of and the window follows. And every picture
        // is built in.
        let mut own = 0;
        let mut lost = Vec::new();
        // (Each chapter's headings as shown, made once.)
        let headings: Vec<Vec<String>> = (CHAPTERS.iter())
            .map(|(_, text)| {
                let shown = blocks(body(text), 680.0, &drawn_width);
                let named: Vec<String> = anchors(&shown).into_iter().flatten().collect();
                assert_eq!(named, heading_slugs(body(text)), "a block opens with each heading");
                named
            })
            .collect();
        for (at, (file, text)) in CHAPTERS.iter().enumerate() {
            for link in links(text) {
                if link.contains("://") {
                    continue;
                }
                if let Some((n, _)) =
                    PICTURES.iter().enumerate().find(|(_, (name, _))| *name == link)
                {
                    let line = text.lines().find(|l| l.contains(link)).unwrap();
                    assert_eq!(picture(line).map(|(n, _)| n), Some(n), "{file}: {line}");
                    continue;
                }
                let Some((chapter, heading)) = destination(link, at) else {
                    panic!("{file} links to {link}, which isn't a chapter");
                };
                assert!(own_links().contains(&link), "{file}: {link} is not hooked");
                own += 1;
                let Some(heading) = heading else { continue };
                if !headings[chapter].iter().any(|s| s == heading) {
                    lost.push(format!(
                        "{file} links to {link}: {} has no such heading",
                        CHAPTERS[chapter].0
                    ));
                }
            }
        }
        assert!(lost.is_empty(), "{}", lost.join("\n"));
        assert!(own > 250, "{own} links within the manual");
    }

    /// A heading's name in a link, as the chapters' links write it.
    #[test]
    fn a_heading_is_named_as_its_links_name_it() {
        assert_eq!(slug("Tabs and windows"), "tabs-and-windows");
        assert_eq!(slug("The area viewer says \"No GPU\""), "the-area-viewer-says-no-gpu");
        assert_eq!(slug("`ctx.selection`, `ctx.open`"), "ctxselection-ctxopen");
        assert_eq!(slug("Build, verify and test"), "build-verify-and-test");
        assert_eq!(heading("## Tiles"), Some("Tiles"));
        assert_eq!(heading("#hashtag"), None);
        assert_eq!(heading("####### seven"), None);
        let page = "# A\n\none\n\n## Keys\n\n```sh\n# not a heading\n```\n\n## Keys\n\ntwo\n";
        assert_eq!(heading_slugs(page), ["a", "keys", "keys-1"]);
        // A block from each heading to the next; a code block's `#` opens none.
        let shown = blocks(page, 600.0, &drawn_width);
        assert_eq!(
            shown,
            [
                text("# A\n\none\n\n"),
                text("## Keys\n\n```sh\n# not a heading\n```\n\n"),
                text("## Keys\n\ntwo\n")
            ]
        );
        let named: Vec<Option<String>> = anchors(&shown);
        assert_eq!(named, [Some("a".into()), Some("keys".into()), Some("keys-1".into())]);
        assert_eq!(destination("03-modules.md#claimed-areas", 0), Some((3, Some("claimed-areas"))));
        assert_eq!(destination("#tiles", 4), Some((4, Some("tiles"))));
        assert_eq!(destination("07-scripts.md", 4), Some((7, None)));
        assert_eq!(destination("https://luau.org", 4), None);
        assert_eq!(destination("../images/x.png", 4), None);
    }

    /// A picture on a line of its own is shown from the program, between
    /// the text before it and after.
    #[test]
    fn a_chapter_s_picture_is_built_in() {
        let (name, bytes) = PICTURES[0];
        assert!(bytes.starts_with(b"\x89PNG"), "{name}");
        assert!(mg_image::png::read_picture(bytes, 4096 * 4096).is_ok(), "{name}");
        let page = format!("Before.\n\n![What it shows]({name})\n\nAfter.\n");
        assert_eq!(
            blocks(&page, 600.0, &drawn_width),
            [
                text("Before.\n\n"),
                Block::Picture { at: 0, alt: "What it shows".into() },
                text("\nAfter.\n")
            ]
        );
        // One the program doesn't have stays as written.
        let other = "![x](../images/none.png)\n";
        assert_eq!(blocks(other, 600.0, &drawn_width), [text(other)]);
    }
}
