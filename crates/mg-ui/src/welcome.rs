//! The Welcome page (`Tab::Welcome`) and the editor area's empty state,
//! "No area open" (`Tab::NoArea`): M4's U24, built to the Welcome board
//! (`target/ui-mock/src/Welcome.html`) and the kit's empty state.
//!
//! The page is laid out from the room it is given and what it holds alone,
//! never from what an earlier frame measured, so it comes to rest at once:
//! three columns (Recent Modules; Start, Without a Module and Status;
//! Learn and Help) where each has its least width, two where the window
//! is narrower (Recent and Learn; Start and Status), one under that. It
//! stands in the middle of its room where it fits, and scrolls where it
//! does not, the rows out of sight not laid out.

use std::cell::Cell;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use egui::text::{LayoutJob, TextFormat, TextWrapping};
use egui::{FontId, Galley, Rect, Sense, Ui, WidgetInfo, WidgetType, pos2, vec2};
use mg_uikit::dialog::EmptyState;
use mg_uikit::{ButtonLook, Icon, KitButton, TokensExt as _, metrics};

use crate::Moonglow;
use crate::commands::Id;
use crate::updates::{Line, Offer};

/// The page's room at its sides (`.welcome .page { padding: 24px 40px }`),
/// less the 6 the editor area gives each tab (`tabs::Viewer::ui`).
const SIDE: f32 = 34.0;
/// The same in a narrow window (under [`NARROW`] wide), the narrow
/// boards' gutter.
const SIDE_NARROW: f32 = 10.0;
/// The width under which the sides are [`SIDE_NARROW`].
const NARROW: f32 = 720.0;
/// The room over and under the page where it fits.
const PAGE_Y: f32 = 24.0;
/// The heading row's line (`.sec-h { line-height: 20px }`).
const HEADING_LINE: f32 = 20.0;
/// Between the heading row's words (`.page-h { gap: 10px }`).
const HEADING_GAP: f32 = 10.0;
/// The least room for the line after the version before it goes under
/// the heading.
const HEADING_LINE_LEAST: f32 = 160.0;
/// Between the heading row and the columns (`.page { gap: 20px }`).
const UNDER_HEADING: f32 = 20.0;
/// Between the columns (`column-gap: 40px`).
const COLUMN_GAP: f32 = 40.0;
/// The columns' shares (`1.45fr 1fr 1fr`).
const SHARES: [f32; 3] = [1.45, 1.0, 1.0];
/// The least width of each column before the page folds to two.
const LEAST: [f32; 3] = [280.0, 260.0, 200.0];
/// The least width of the page (between its sides) for two columns.
const TWO_LEAST: f32 = 560.0;
/// Between a column's parts (`.col { gap: 6px }`).
const PART_GAP: f32 = 6.0;
/// A section heading's height: its line, 4 under it and its rule.
const SECTION_HIGH: f32 = HEADING_LINE + 4.0 + 1.0;
/// Under a section heading's rule (`.col .sec-h { margin-bottom: 4px }`).
const SECTION_ROOM: f32 = 4.0;
/// Over a section that follows another in a column (Status's
/// `margin-top: 18px`).
const SECTION_OVER: f32 = 18.0;
/// A recent module's row: 6 over and under, its name's line, 1, and its
/// folder's (`.rrow`).
const ROW_HIGH: f32 = 6.0 + 16.0 + 1.0 + 14.0 + 6.0;
/// How far a row reaches past its column at each side (`margin: 0 -8px`),
/// and its room inside (`padding: 6px 8px`).
const ROW_OUT: f32 = 8.0;
/// Between a row's icon and its words (`column-gap: 10px`).
const ROW_ICON_GAP: f32 = 10.0;
/// A Start button's height (`.welcome .acts .btn { height: 26px }`).
const ACT_HIGH: f32 = 26.0;
/// Between the Start buttons (`.acts { gap: 4px }`).
const ACT_GAP: f32 = 4.0;
/// Over a sub-heading (`.pop-h2 { margin-top: 10px }`) and its line.
const SUB_OVER: f32 = 10.0;
const SUB_LINE: f32 = 16.0;
/// A status line's room over and under (`.state { padding: 8px 0 2px }`),
/// between its icon and words (`column-gap: 8px`) and its rows
/// (`row-gap: 6px`).
const STATE_OVER: f32 = 8.0;
const STATE_UNDER: f32 = 2.0;
const STATE_ICON_GAP: f32 = 8.0;
const STATE_ROW_GAP: f32 = 6.0;
/// A link's height (`.links a { height: 22px }`), the gap after its icon
/// and over the first (`margin-top: 4px`).
const LINK_HIGH: f32 = 22.0;
const LINK_ICON_GAP: f32 = 8.0;
const LINKS_OVER: f32 = 4.0;
/// Over the note under the recent modules (`margin-top: 6px`).
const NOTE_OVER: f32 = 6.0;
/// The body's line.
const LINE: f32 = metrics::LINE_HEIGHT;
/// A small line (`.small { line-height: 14px }`).
const SMALL_LINE: f32 = 14.0;

/// The note under the recent modules (the board's words).
pub(crate) const NOTE: &str =
    "Also in File › Recent Modules. A module file or folder dropped on the window opens too.";
/// Said where no module was opened yet.
pub(crate) const NO_RECENT: &str = "No module opened yet.";

/// What the two pages keep between frames (`Moonglow::welcome`).
#[derive(Default)]
pub(crate) struct Kept {
    /// Whether the Welcome page's update line was in sight, and the pass it
    /// was drawn in: only the last pass's counts ([`in_sight`]).
    told: Option<(u64, bool)>,
    /// Height of `update-notice` panel when drawn in this pass (`0.0` when hidden).
    pub(crate) bar_height: f32,
    /// Frame number when the Welcome page discarded to reveal the notice bar
    /// under a floating window, preventing intra-frame ping-pong back to `true`.
    discarded_to_bar: Option<u64>,
    /// Whether the module has areas, by the workspace's identity and
    /// revision (No area open's Go to Start).
    areas: Option<((u64, u64), bool)>,
    /// What the files dragged over the window become, by their paths and
    /// whether a module is open (`transfer::drop_ui`, once per drag).
    pub(crate) drag: Option<(Vec<PathBuf>, bool, crate::transfer::Dropped)>,
}

impl Kept {
    /// Clears module-scoped Welcome/No-Area state when a module is closed.
    pub(crate) fn clear_module(&mut self) {
        self.areas = None;
    }
}

/// What the pages count: every walk of the module, scan of the manual,
/// install check and folder read they make (`Moonglow::welcome_counts`).
#[derive(Clone, Copy)]
pub(crate) enum Counted {
    /// The module's areas listed (`mg_module::areas::every`).
    AreaWalk,
    /// A chapter's title found in its text (`manual::title`).
    TitleScan,
    /// Whether this copy installs a release (`Updates::why_not`).
    InstallCheck,
    /// A dragged folder read for a module (`transfer::holds_module`).
    FolderRead,
    /// A pass thrown away because the update line came into or out of
    /// sight against what the strip took it to be.
    Discard,
}

thread_local! {
    static COUNTS: Cell<[u64; 5]> = const { Cell::new([0; 5]) };
}

/// Counts one of `what`.
pub(crate) fn count(what: Counted) {
    COUNTS.with(|c| {
        let mut n = c.get();
        n[what as usize] += 1;
        c.set(n);
    });
}

impl Moonglow {
    /// What the Welcome page, No area open and the drop overlay have
    /// walked, scanned, checked and read on this thread: the module's
    /// areas, the manual's chapter titles, whether a release installs, a
    /// dragged folder, and the passes the page threw away over its update
    /// line (in that order). The tests' look at it.
    #[doc(hidden)]
    pub fn welcome_counts(&self) -> [u64; 5] {
        COUNTS.with(Cell::get)
    }
}

/// The manual's chapter titles, found once.
fn chapter_title(i: usize) -> &'static str {
    static TITLES: OnceLock<Vec<&'static str>> = OnceLock::new();
    let titles = TITLES.get_or_init(|| {
        (crate::manual::CHAPTERS.iter())
            .map(|(_, text)| {
                count(Counted::TitleScan);
                crate::manual::title(text)
            })
            .collect()
    });
    titles.get(i).copied().unwrap_or_default()
}

/// The Start buttons.
pub(crate) const START: [(Id, Icon); 3] =
    [(Id::NewModule, Icon::New), (Id::OpenModule, Icon::Open), (Id::OpenFolder, Icon::Folder)];
/// What opens without a module (the board's "Without a module").
pub(crate) const WITHOUT: [(Id, Icon); 8] = [
    (Id::OpenHak, Icon::Hak),
    (Id::NewHak, Icon::Hak),
    (Id::OpenTileset, Icon::Tileset),
    (Id::NewTileset, Icon::Tileset),
    (Id::OpenTable, Icon::Table),
    (Id::TalkTable, Icon::TalkTable),
    (Id::Resources, Icon::Search),
    (Id::PlaceableGallery, Icon::Gallery),
];
/// Help's links.
pub(crate) const HELP: [(Id, Icon); 3] = [
    (Id::CommandPalette, Icon::Command),
    (Id::CheckUpdates, Icon::Update),
    (Id::About, Icon::Info),
];
/// Learn's chapters, in the board's order (the plugin writer's two left
/// to the manual's contents).
pub(crate) const LEARN: [&str; 16] = [
    "01-getting-started.md",
    "14-coming-from-aurora.md",
    "02-main-window.md",
    "03-modules.md",
    "04-areas.md",
    "05-blueprints.md",
    "06-conversations.md",
    "07-scripts.md",
    "08-journal-and-factions.md",
    "18-nui-creator.md",
    "09-build-and-test.md",
    "12-differences.md",
    "10-options.md",
    "11-command-line.md",
    "13-troubleshooting.md",
    "15-plugins.md",
];

/// What a click on the page does, done after it is drawn.
enum Click {
    Command(Id),
    Recent(PathBuf),
    Chapter(usize),
    Offer(Offer),
}

/// A link's kind: a chapter (an index of `manual::CHAPTERS`) or a command.
#[derive(Clone, Copy)]
enum Link {
    Chapter(usize),
    Command(Id, Icon),
}

/// A button's kind.
#[derive(Clone, Copy)]
enum Button {
    Start(Id, Icon),
    Without(Id, Icon),
    Manual,
    Options,
    Offer(Offer),
}

/// One thing on the page, where it is (from the page's top left).
enum Part {
    Words(Arc<Galley>, Rect),
    Section(Arc<Galley>, Rect),
    /// The update line's words: whether they are in sight is kept.
    Told(Arc<Galley>, Rect),
    /// A sub-heading (Without a module, Help), a heading to a screen reader.
    Sub(Arc<Galley>, Rect),
    Icon(Icon, egui::Color32, Rect),
    Spinner(Rect),
    Recent(usize, Rect),
    Link(Link, Rect),
    Button(Button, Rect),
}

/// The page laid out: its parts and its height.
struct Page {
    parts: Vec<Part>,
    high: f32,
}

/// How many columns the page stands in at `inner` wide (between its
/// sides).
fn columns(inner: f32) -> usize {
    let share = (inner - 2.0 * COLUMN_GAP) / SHARES.iter().sum::<f32>();
    if SHARES.iter().zip(LEAST).all(|(s, least)| s * share >= least) {
        3
    } else if inner >= TWO_LEAST {
        2
    } else {
        1
    }
}

/// The home folder, looked up once.
fn home() -> Option<&'static Path> {
    static HOME: OnceLock<Option<PathBuf>> = OnceLock::new();
    HOME.get_or_init(|| {
        let var = if cfg!(windows) { "USERPROFILE" } else { "HOME" };
        std::env::var_os(var).map(PathBuf::from).filter(|p| p.is_absolute())
    })
    .as_deref()
}

/// `path` with the home folder as `~` (the board's).
pub(crate) fn homely(path: &Path) -> String {
    match home().and_then(|h| path.strip_prefix(h).ok()) {
        Some(rest) if rest.as_os_str().is_empty() => "~".into(),
        Some(rest) => format!("~{}{}", std::path::MAIN_SEPARATOR, rest.to_string_lossy()),
        None => path.to_string_lossy().into_owned(),
    }
}

/// Whether a recent module is a module's file (`.mod`, `.nwm`, in any
/// case), told by its name, not the disk; else a folder (a module folder,
/// a nasher project).
fn is_file(path: &Path) -> bool {
    path.extension().is_some_and(|e| e.eq_ignore_ascii_case("mod") || e.eq_ignore_ascii_case("nwm"))
}

/// A recent module's name as the page shows it: a folder's with a `/`
/// after it, a module file's none (finding 13.3: `Chapter1.nwm/`).
pub(crate) fn recent_name(path: &Path) -> String {
    let mut name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    if !is_file(path) {
        name.push('/');
    }
    name
}

/// The folder a recent module is in, as the page shows it.
pub(crate) fn recent_folder(path: &Path) -> String {
    path.parent().filter(|p| !p.as_os_str().is_empty()).map(homely).unwrap_or_default()
}

/// One line of `text`, cut with an ellipsis at `width`.
fn one_line(ui: &Ui, text: String, size: f32, color: egui::Color32, width: f32) -> Arc<Galley> {
    let mut job =
        LayoutJob::single_section(text, TextFormat::simple(FontId::proportional(size), color));
    job.wrap = TextWrapping {
        max_width: width.max(1.0),
        max_rows: 1,
        break_anywhere: true,
        overflow_character: Some('…'),
    };
    ui.painter().layout_job(job)
}

/// `text` wrapped at `width`.
fn wrapped(ui: &Ui, text: String, color: egui::Color32, width: f32) -> Arc<Galley> {
    let font = FontId::proportional(metrics::BODY_SIZE);
    ui.painter().layout(text, font, color, width.max(1.0))
}

/// The bold heading font where the fonts are in (not on the first frame).
fn bold(ui: &Ui) -> FontId {
    let bold = mg_uikit::fonts::bold();
    match ui.fonts(|f| f.families().contains(&bold)) {
        true => FontId::new(metrics::HEADING_SIZE, bold),
        false => FontId::proportional(metrics::HEADING_SIZE),
    }
}

/// Lays the page out `width` wide.
fn plan(app: &Moonglow, ui: &Ui, width: f32, line: Option<&Line>) -> Page {
    let t = ui.tokens();
    let side = if width < NARROW { SIDE_NARROW } else { SIDE };
    let inner = (width - 2.0 * side).max(LEAST[2]);
    let mut parts = Vec::new();
    // The heading row: the name, the version, the line.
    let font = bold(ui);
    let name = ui.painter().layout_no_wrap("Moonglow Toolset".into(), font, t.text);
    let body = FontId::proportional(metrics::BODY_SIZE);
    let version = mg_update::Version::current().to_string();
    let version = ui.painter().layout_no_wrap(version, body, t.text_3);
    let words = "A module toolset for Neverwinter Nights: Enhanced Edition.";
    let mut x = side;
    let middle = |g: &Arc<Galley>, x: f32, y: f32| {
        Rect::from_min_size(pos2(x, y + (HEADING_LINE - g.size().y) / 2.0), g.size())
    };
    parts.push(Part::Words(name.clone(), middle(&name, x, 0.0)));
    x += name.size().x + HEADING_GAP;
    parts.push(Part::Words(version.clone(), middle(&version, x, 0.0)));
    x += version.size().x + HEADING_GAP;
    let room = side + inner - x;
    let mut y = HEADING_LINE;
    if room >= HEADING_LINE_LEAST {
        let g = wrapped(ui, words.into(), t.text_2, room);
        let high = g.size().y.max(LINE);
        let at = pos2(x, (HEADING_LINE - LINE) / 2.0);
        parts.push(Part::Words(g.clone(), Rect::from_min_size(at, g.size())));
        y = y.max(high + (HEADING_LINE - LINE) / 2.0);
    } else {
        let g = wrapped(ui, words.into(), t.text_2, inner);
        parts.push(Part::Words(g.clone(), Rect::from_min_size(pos2(side, y), g.size())));
        y += g.size().y;
    }
    let top = y + UNDER_HEADING;
    // The columns, and the sections in each.
    let n = columns(inner);
    let share = (inner - (n - 1) as f32 * COLUMN_GAP)
        / match n {
            3 => SHARES.iter().sum::<f32>(),
            k => k as f32,
        };
    type Section = fn(&Moonglow, &Ui, &mut Vec<Part>, Option<&Line>, f32, f32, f32) -> f32;
    let (recent, start, status, learn): (Section, Section, Section, Section) =
        (recent_section, start_section, status_section, learn_section);
    let stacks: Vec<Vec<Section>> = match n {
        3 => vec![vec![recent], vec![start, status], vec![learn]],
        2 => vec![vec![recent, learn], vec![start, status]],
        _ => vec![vec![recent, start, status, learn]],
    };
    let mut x = side;
    let mut bottom = top;
    for (i, stack) in stacks.iter().enumerate() {
        let w = match n {
            3 => SHARES[i] * share,
            _ => share,
        };
        let mut y = top;
        for (k, section) in stack.iter().enumerate() {
            if k > 0 {
                y += PART_GAP + SECTION_OVER;
            }
            y = section(app, ui, &mut parts, line, x, y, w);
        }
        bottom = bottom.max(y);
        x += w + COLUMN_GAP;
    }
    Page { parts, high: bottom }
}

/// A section heading at `y`; where its content starts.
fn heading(ui: &Ui, parts: &mut Vec<Part>, text: &str, x: f32, y: f32, w: f32) -> f32 {
    let g = ui.painter().layout_no_wrap(text.into(), bold(ui), ui.tokens().text);
    parts.push(Part::Section(g, Rect::from_min_size(pos2(x, y), vec2(w, SECTION_HIGH))));
    y + SECTION_HIGH + SECTION_ROOM + PART_GAP
}

/// Recent Modules: a row each, and the note.
fn recent_section(
    app: &Moonglow,
    ui: &Ui,
    parts: &mut Vec<Part>,
    _: Option<&Line>,
    x: f32,
    y: f32,
    w: f32,
) -> f32 {
    let t = ui.tokens();
    let mut y = heading(ui, parts, "Recent Modules", x, y, w);
    let recent = &app.settings.recent;
    if recent.is_empty() {
        let g = wrapped(ui, NO_RECENT.into(), t.text_3, w);
        let high = g.size().y;
        parts.push(Part::Words(g.clone(), Rect::from_min_size(pos2(x, y), g.size())));
        y += high;
    }
    for i in 0..recent.len() {
        let row = Rect::from_min_size(pos2(x - ROW_OUT, y), vec2(w + 2.0 * ROW_OUT, ROW_HIGH));
        parts.push(Part::Recent(i, row));
        y += ROW_HIGH;
    }
    y += PART_GAP + NOTE_OVER;
    let g = wrapped(ui, NOTE.into(), t.text_3, w);
    let high = g.size().y;
    parts.push(Part::Words(g.clone(), Rect::from_min_size(pos2(x, y), g.size())));
    y + high
}

/// Start, and Without a Module.
fn start_section(
    app: &Moonglow,
    ui: &Ui,
    parts: &mut Vec<Part>,
    _: Option<&Line>,
    x: f32,
    y: f32,
    w: f32,
) -> f32 {
    let t = ui.tokens();
    let mut y = heading(ui, parts, "Start", x, y, w);
    for (k, (id, icon)) in START.into_iter().enumerate() {
        if k > 0 {
            y += ACT_GAP;
        }
        parts.push(Part::Button(
            Button::Start(id, icon),
            Rect::from_min_size(pos2(x, y), vec2(w, ACT_HIGH)),
        ));
        y += ACT_HIGH;
    }
    y += PART_GAP + SUB_OVER;
    let g = ui.painter().layout_no_wrap(
        "Without a module".into(),
        FontId::proportional(metrics::BODY_SIZE),
        t.text_2,
    );
    parts.push(Part::Sub(
        g.clone(),
        Rect::from_min_size(pos2(x, y + (SUB_LINE - g.size().y) / 2.0), g.size()),
    ));
    y += SUB_LINE + PART_GAP;
    // (Two side by side where each fits its half, else one above the
    // other.)
    let half = (w - ACT_GAP) / 2.0;
    let widest = (WITHOUT.iter())
        .map(|(id, icon)| without_button(app, *id, *icon, 0.0).width(ui))
        .fold(0.0, f32::max);
    let two = widest <= half;
    for (k, (id, icon)) in WITHOUT.into_iter().enumerate() {
        let (column, row) = if two { (k % 2, k / 2) } else { (0, k) };
        let at = pos2(x + column as f32 * (half + ACT_GAP), y + row as f32 * (ACT_HIGH + ACT_GAP));
        let size = vec2(if two { half } else { w }, ACT_HIGH);
        parts.push(Part::Button(Button::Without(id, icon), Rect::from_min_size(at, size)));
    }
    let rows = if two { WITHOUT.len().div_ceil(2) } else { WITHOUT.len() };
    y + rows as f32 * ACT_HIGH + (rows - 1) as f32 * ACT_GAP
}

/// Status: the game data, and the update's line.
fn status_section(
    app: &Moonglow,
    ui: &Ui,
    parts: &mut Vec<Part>,
    line: Option<&Line>,
    x: f32,
    y: f32,
    w: f32,
) -> f32 {
    let t = ui.tokens();
    let mut y = heading(ui, parts, "Status", x, y, w);
    let words_x = x + metrics::ICON + STATE_ICON_GAP;
    let words_w = (w - metrics::ICON - STATE_ICON_GAP).max(1.0);
    // The game data.
    y += STATE_OVER;
    let icon = Rect::from_min_size(pos2(x, y), vec2(metrics::ICON, metrics::ICON));
    if app.game.is_some() {
        parts.push(Part::Icon(Icon::CheckCircle, t.ok, icon));
        let g = one_line(ui, "Game data found".into(), metrics::BODY_SIZE, t.text, words_w);
        parts.push(Part::Words(g.clone(), Rect::from_min_size(pos2(words_x, y), g.size())));
        y += LINE;
        if let Some(install) = &app.install {
            y += 1.0;
            let g = one_line(ui, homely(&install.root), metrics::SMALL_SIZE, t.text_3, words_w);
            parts.push(Part::Words(g.clone(), Rect::from_min_size(pos2(words_x, y), g.size())));
            y += SMALL_LINE;
        }
    } else {
        parts.push(Part::Icon(Icon::Warning, t.warn, icon));
        let said = "No game data: choose your Neverwinter Nights installation in Tools › Options.";
        let g = wrapped(ui, said.into(), t.text, words_w);
        let high = g.size().y;
        parts.push(Part::Words(g.clone(), Rect::from_min_size(pos2(words_x, y), g.size())));
        y += high;
    }
    y += STATE_ROW_GAP;
    let options = KitButton::new(None, "Options…").width(ui);
    parts.push(Part::Button(
        Button::Options,
        Rect::from_min_size(pos2(words_x, y), vec2(options, metrics::CONTROL_HEIGHT)),
    ));
    y += metrics::CONTROL_HEIGHT + STATE_UNDER;
    // The update.
    let Some(line) = line else { return y };
    y += PART_GAP + STATE_OVER;
    let icon = Rect::from_min_size(pos2(x, y), vec2(metrics::ICON, metrics::ICON));
    match (line.busy, line.error) {
        (true, _) => parts.push(Part::Spinner(icon)),
        (false, true) => parts.push(Part::Icon(Icon::Error, t.error, icon)),
        (false, false) => parts.push(Part::Icon(Icon::Update, t.info, icon)),
    }
    let color = if line.error { t.error } else { t.text };
    let g = wrapped(ui, line.words.clone(), color, words_w);
    let high = g.size().y;
    parts.push(Part::Told(g.clone(), Rect::from_min_size(pos2(words_x, y), g.size())));
    y += high;
    if !line.offers.is_empty() {
        y += STATE_ROW_GAP;
        // (Side by side, on as many rows as they need.)
        let mut at = words_x;
        for offer in &line.offers {
            let width = offer_button(*offer).width(ui);
            if at > words_x && at + width > words_x + words_w {
                at = words_x;
                y += metrics::CONTROL_HEIGHT + ACT_GAP;
            }
            let rect = Rect::from_min_size(pos2(at, y), vec2(width, metrics::CONTROL_HEIGHT));
            parts.push(Part::Button(Button::Offer(*offer), rect));
            at += width + ACT_GAP;
        }
        y += metrics::CONTROL_HEIGHT;
    }
    y + STATE_UNDER
}

/// Learn: the manual and its chapters; Help.
fn learn_section(
    _: &Moonglow,
    ui: &Ui,
    parts: &mut Vec<Part>,
    _: Option<&Line>,
    x: f32,
    y: f32,
    w: f32,
) -> f32 {
    let t = ui.tokens();
    let mut y = heading(ui, parts, "Learn", x, y, w);
    parts.push(Part::Button(Button::Manual, Rect::from_min_size(pos2(x, y), vec2(w, ACT_HIGH))));
    y += ACT_HIGH + PART_GAP + LINKS_OVER;
    let link =
        |y: f32| Rect::from_min_size(pos2(x - ROW_OUT, y), vec2(w + 2.0 * ROW_OUT, LINK_HIGH));
    for file in LEARN {
        if let Some(i) = crate::manual::CHAPTERS.iter().position(|(f, _)| *f == file) {
            parts.push(Part::Link(Link::Chapter(i), link(y)));
            y += LINK_HIGH;
        }
    }
    y += PART_GAP + SUB_OVER;
    let g = ui.painter().layout_no_wrap(
        "Help".into(),
        FontId::proportional(metrics::BODY_SIZE),
        t.text_2,
    );
    parts.push(Part::Sub(
        g.clone(),
        Rect::from_min_size(pos2(x, y + (SUB_LINE - g.size().y) / 2.0), g.size()),
    ));
    y += SUB_LINE + PART_GAP;
    for (id, icon) in HELP {
        parts.push(Part::Link(Link::Command(id, icon), link(y)));
        y += LINK_HIGH;
    }
    y
}

/// A Without a Module button.
fn without_button(app: &Moonglow, id: Id, icon: Icon, width: f32) -> KitButton {
    let mut b = KitButton::new(Some(icon), id.label(app))
        .look(ButtonLook::Flat)
        .quiet_icon()
        .height(ACT_HIGH)
        .at_least(width)
        .enabled(id.enabled(app));
    if !id.hint().is_empty() {
        b = b.tip(id.hint());
    }
    b
}

/// An update line's button.
fn offer_button(offer: Offer) -> KitButton {
    let (words, tip) = offer.text();
    let look = if offer == Offer::InstallNow { ButtonLook::Primary } else { ButtonLook::Raised };
    KitButton::new(None, words).look(look).tip(tip)
}

/// Whether the Welcome page is in front of its pane with its update line
/// in sight at the last frame: the notice's strip is not drawn then, the
/// page's Status telling it. (Where the page scrolls the line out of
/// sight, the strip tells it. The strip's room given back only brings the
/// page's line further into sight, so the two do not take turns.)
///
/// Only what the last pass drew counts: where the page was not drawn in
/// front there (another tab in front, the first frame), the line is taken
/// as in sight, and the page discards the pass where it finds otherwise
/// (`welcome`), so no frame shows the strip where the page tells it, or
/// neither.
pub(crate) fn in_sight(app: &Moonglow, ctx: &egui::Context) -> bool {
    let Some(at) = app.dock.find_tab(&crate::Tab::Welcome) else { return false };
    let path = egui_dock::NodePath { surface: at.surface, node: at.node };
    app.dock.leaf(path).is_ok_and(|l| l.active == at.tab) && told_last(app, ctx)
}

/// Whether the page's update line was in sight at the last pass, taken as
/// in sight where the page was not drawn then.
fn told_last(app: &Moonglow, ctx: &egui::Context) -> bool {
    match app.welcome.told {
        Some((pass, shown)) if pass + 1 == ctx.cumulative_pass_nr() => shown,
        _ => true,
    }
}

/// Whether a window (a floating editor, a plugin's window) stands over
/// any of `r` at the last frame. Not a dialog (About, Check for Updates):
/// one opened over the line would bring the strip in and move the page
/// under it, and closing it would move the page back.
fn under_a_window(ui: &Ui, r: Rect) -> bool {
    let own = ui.layer_id();
    let ctx = ui.ctx();
    let over: Vec<egui::LayerId> = ctx.memory(|m| {
        (m.layer_ids())
            .filter(|l| l.order == egui::Order::Middle && *l != own)
            .filter(|l| {
                m.areas().is_visible(l) && m.area_rect(l.id).is_some_and(|a| a.intersects(r))
            })
            .collect()
    });
    over.into_iter().any(|l| !crate::widgets::is_dialog(ctx, l))
}

/// The Welcome page.
pub(crate) fn welcome(app: &mut Moonglow, ui: &mut Ui) {
    let ctx = ui.ctx().clone();
    let room = ui.available_rect_before_wrap();
    let line = crate::updates::line(app);
    // (What the strip took the line to be this pass. The page is in front:
    // the dock is not asked, which is empty while the tabs are drawn.)
    let assumed = told_last(app, &ctx);
    let mut told = false;
    // (No row, link or button acts by the keys while a menu, a dialog or a
    // modal has them: Enter on a focused row under About opens nothing.)
    let blocked = crate::shell::keys_blocked(&ctx);
    let page = plan(app, ui, room.width(), line.as_ref());
    // (In the middle of its room where it fits, else from its top, the
    // tab scrolling it.)
    let top_for = |r: Rect| {
        let fits = r.height().is_finite() && page.high + 2.0 * PAGE_Y <= r.height();
        match fits {
            true => r.top() + ((r.height() - page.high) / 2.0).round(),
            false => r.top() + PAGE_Y,
        }
    };
    let fits = room.height().is_finite() && page.high + 2.0 * PAGE_Y <= room.height();
    let top = top_for(room);
    let origin = pos2(room.left(), top).to_vec2();
    let bar = if assumed { 0.0 } else { app.welcome.bar_height };
    let room_no_bar = Rect::from_min_max(pos2(room.min.x, room.min.y - bar), room.max);
    let origin_no_bar = pos2(room_no_bar.left(), top_for(room_no_bar)).to_vec2();
    let clip = ui.clip_rect();
    let frame = ctx.cumulative_frame_nr();
    let t = ui.tokens();
    let mut click = None;
    for part in page.parts {
        match part {
            Part::Words(g, r) => {
                let r = r.translate(origin);
                ui.put(r, egui::Label::new(g).selectable(false));
            }
            Part::Told(g, r) => {
                let draw_r = r.translate(origin);
                // (Out of sight, wholly or in part, or under a window, the
                // strip under the Top Bar tells it, and the line and its
                // buttons are not drawn: it is told once. When the notice
                // strip is already visible (`!assumed`), check both the shifted
                // clip position and the un-shifted upper position against
                // floating windows so showing the strip does not uncover it and
                // trigger a discard loop.)
                let visible = clip.contains_rect(draw_r);
                told = if assumed {
                    visible && !under_a_window(ui, draw_r)
                } else {
                    let check_r = r.translate(origin_no_bar);
                    app.welcome.discarded_to_bar != Some(frame)
                        && visible
                        && !under_a_window(ui, draw_r)
                        && !under_a_window(ui, check_r)
                };
                if told && assumed {
                    ui.put(draw_r, egui::Label::new(g).selectable(false));
                }
            }
            Part::Section(g, r) => {
                let r = r.translate(origin);
                let size = g.size();
                let words = Rect::from_min_size(
                    pos2(r.left(), r.top() + (HEADING_LINE - size.y) / 2.0),
                    size,
                );
                heading_node(ui, words, g);
                let rule = r.bottom() - 0.5;
                ui.painter().hline(r.x_range(), rule, egui::Stroke::new(1.0, t.line_soft));
            }
            Part::Sub(g, r) => heading_node(ui, r.translate(origin), g),
            Part::Icon(icon, color, r) => {
                let r = r.translate(origin);
                if ui.is_rect_visible(r) {
                    icon.paint(ui.painter(), r, color);
                }
            }
            Part::Spinner(r) => {
                let r = r.translate(origin);
                ui.put(r, egui::Spinner::new().size(metrics::ICON).color(t.text_2));
            }
            Part::Recent(i, r) => {
                let r = r.translate(origin);
                let Some(path) = app.settings.recent.get(i) else { continue };
                // (By its place in the list: a list edited by hand can name
                // a module twice.)
                let id = ui.id().with(("welcome-recent", i));
                if recent_row(ui, id, path, r) {
                    click = Some(Click::Recent(path.clone()));
                }
            }
            Part::Link(link, r) => {
                let r = r.translate(origin);
                if let Some(c) = link_row(app, ui, link, r) {
                    click = Some(c);
                }
            }
            Part::Button(button, r) => {
                let r = r.translate(origin);
                if matches!(button, Button::Offer(_)) && !(told && assumed) {
                    continue;
                }
                let salt = match button {
                    Button::Start(id, _) | Button::Without(id, _) => egui::Id::new(id.id()),
                    Button::Manual => egui::Id::new("manual"),
                    Button::Options => egui::Id::new("options"),
                    Button::Offer(offer) => egui::Id::new(offer.text().0),
                };
                let (b, c) = match button {
                    Button::Start(id, icon) => {
                        let keys = app.keymap.label_of(&id.id(), &ctx);
                        let mut b = KitButton::new(Some(icon), id.label(app))
                            .quiet_icon()
                            .height(ACT_HIGH)
                            .at_least(r.width())
                            .keys(keys)
                            .enabled(id.enabled(app));
                        if !id.hint().is_empty() {
                            b = b.tip(id.hint());
                        }
                        (b, Click::Command(id))
                    }
                    Button::Without(id, icon) => {
                        (without_button(app, id, icon, r.width()), Click::Command(id))
                    }
                    Button::Manual => {
                        let b = KitButton::new(Some(Icon::Book), Id::Manual.label(app))
                            .quiet_icon()
                            .height(ACT_HIGH)
                            .at_least(r.width())
                            .tip("The manual's contents");
                        (b, Click::Command(Id::Manual))
                    }
                    Button::Options => {
                        let b = KitButton::new(None, "Options…")
                            .tip("Where the game is, and the rest of Moonglow's options");
                        (b, Click::Command(Id::Options))
                    }
                    Button::Offer(offer) => (offer_button(offer), Click::Offer(offer)),
                };
                // (Each in a scope of its own, so that its id stays as the
                // update line comes and goes: the keyboard focus with it.)
                let builder = egui::UiBuilder::new().id_salt(salt).max_rect(r);
                if ui.scope_builder(builder, |ui| ui.put(r, b)).inner.clicked() {
                    click = Some(c);
                }
            }
        }
    }
    ui.advance_cursor_after_rect(Rect::from_min_size(
        room.min,
        vec2(room.width(), (top - room.top()) + page.high + if fits { 0.0 } else { PAGE_Y }),
    ));
    // (Where the strip took the line wrongly this pass, the pass is drawn
    // again with it right: never a frame with both, or neither.)
    let told = line.is_some() && told;
    app.welcome.told = Some((ctx.cumulative_pass_nr(), told));
    if line.is_some() && told != assumed {
        if assumed && !told {
            app.welcome.discarded_to_bar = Some(frame);
        }
        count(Counted::Discard);
        ctx.request_discard("the Welcome page's update line came into or out of sight");
    }
    // (The keys only: a click of the pointer on the page beside a dialog
    // is the page's, as anywhere else in the window.)
    if blocked && !ctx.input(|i| i.pointer.any_click()) {
        click = None;
    }
    match click {
        Some(Click::Command(id)) => id.run(app, &ctx),
        Some(Click::Recent(path)) => app.actions.push(crate::Action::OpenModule(path)),
        Some(Click::Chapter(i)) => crate::manual::open_chapter(app, i),
        Some(Click::Offer(offer)) => {
            if let Some(line) = &line {
                crate::updates::act(app, &ctx, line, offer);
            }
        }
        None => {}
    }
}

/// The accessible name of a recent module's row: its name and the folder
/// it is in.
pub(crate) fn recent_label(path: &Path) -> String {
    match recent_folder(path) {
        folder if folder.is_empty() => recent_name(path),
        folder => format!("{} in {folder}", recent_name(path)),
    }
}

/// A heading's words `g` at `r`, a heading to a screen reader.
fn heading_node(ui: &mut Ui, r: Rect, g: Arc<Galley>) {
    let words = g.text().to_string();
    let response = ui.put(r, egui::Label::new(g).selectable(false));
    ui.ctx().accesskit_node_builder(response.id, |n| {
        n.set_role(egui::accesskit::Role::Heading);
        n.set_label(words);
    });
}

/// A row that is out of sight: only taken for the keyboard (Tab reaches
/// it), and brought into sight when it gets the keys. Whether it was
/// pressed (Enter or Space).
fn out_of_sight(ui: &mut Ui, id: egui::Id, r: Rect) -> Option<bool> {
    if ui.is_rect_visible(r) {
        return None;
    }
    let response = ui.interact(r, id, Sense::click());
    if response.gained_focus() {
        response.scroll_to_me(None);
    }
    Some(response.clicked())
}

/// A recent module's row at `r`: its kind's icon, its name in the accent,
/// the folder it is in under it; the whole path its tooltip. Whether it
/// was clicked (or Enter or Space pressed on it). Out of sight, only taken
/// for the keyboard (the rows in sight alone are laid out).
fn recent_row(ui: &mut Ui, id: egui::Id, path: &Path, r: Rect) -> bool {
    if let Some(pressed) = out_of_sight(ui, id, r) {
        return pressed;
    }
    let t = ui.tokens();
    let response = ui.interact(r, id, Sense::click());
    if response.gained_focus() {
        response.scroll_to_me(None);
    }
    let label = recent_label(path);
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, &label));
    let painter = ui.painter();
    if response.hovered() || response.has_focus() {
        painter.rect_filled(r, metrics::RADIUS, t.hover_row);
    }
    if response.has_focus() {
        let stroke = egui::Stroke::new(1.0, t.accent);
        painter.rect_stroke(r.shrink(0.5), metrics::RADIUS, stroke, egui::StrokeKind::Inside);
    }
    let x = r.left() + ROW_OUT;
    let icon = if is_file(path) { Icon::Module } else { Icon::Folder };
    let words_x = x + metrics::ICON + ROW_ICON_GAP;
    let words_w = r.right() - ROW_OUT - words_x;
    let top = r.top() + 6.0;
    let name = one_line(ui, recent_name(path), metrics::BODY_SIZE, t.accent, words_w);
    let folder = one_line(ui, recent_folder(path), metrics::SMALL_SIZE, t.text_3, words_w);
    let painter = ui.painter();
    let icon_rect = Rect::from_min_size(
        pos2(x, r.center().y - metrics::ICON / 2.0),
        vec2(metrics::ICON, metrics::ICON),
    );
    icon.paint(painter, icon_rect, t.text_2);
    painter.galley(pos2(words_x, top + (LINE - name.size().y) / 2.0), name, t.accent);
    let under = top + LINE + 1.0;
    painter.galley(pos2(words_x, under + (SMALL_LINE - folder.size().y) / 2.0), folder, t.text_3);
    response.on_hover_text(path.to_string_lossy().into_owned()).clicked()
}

/// A link's row at `r`: its icon in `text-2`, its words in the accent.
fn link_row(app: &Moonglow, ui: &mut Ui, link: Link, r: Rect) -> Option<Click> {
    let id = match link {
        Link::Chapter(i) => ui.id().with(("welcome-chapter", i)),
        Link::Command(command, _) => ui.id().with(("welcome-link", command.id())),
    };
    let (words, icon, tip, click, enabled) = match link {
        Link::Chapter(i) => {
            (chapter_title(i).to_string(), Icon::File, None, Click::Chapter(i), true)
        }
        Link::Command(id, icon) => {
            let tip = Some(id.hint()).filter(|h| !h.is_empty());
            (id.label(app), icon, tip, Click::Command(id), id.enabled(app))
        }
    };
    if enabled && let Some(pressed) = out_of_sight(ui, id, r) {
        return pressed.then_some(click);
    }
    let t = ui.tokens();
    let sense = if enabled { Sense::click() } else { Sense::hover() };
    let response = ui.interact(r, id, sense);
    if response.gained_focus() {
        response.scroll_to_me(None);
    }
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Link, enabled, &words));
    let painter = ui.painter();
    if enabled && (response.hovered() || response.has_focus()) {
        painter.rect_filled(r, metrics::RADIUS, t.hover_row);
    }
    if response.has_focus() {
        let stroke = egui::Stroke::new(1.0, t.accent);
        painter.rect_stroke(r.shrink(0.5), metrics::RADIUS, stroke, egui::StrokeKind::Inside);
    }
    let x = r.left() + ROW_OUT;
    let words_x = x + metrics::ICON + LINK_ICON_GAP;
    let color = if enabled { t.accent } else { t.text_off };
    let g = one_line(ui, words, metrics::BODY_SIZE, color, r.right() - ROW_OUT - words_x);
    let painter = ui.painter();
    let icon_rect = Rect::from_min_size(
        pos2(x, r.center().y - metrics::ICON / 2.0),
        vec2(metrics::ICON, metrics::ICON),
    );
    icon.paint(painter, icon_rect, if enabled { t.text_2 } else { t.text_off });
    painter.galley(pos2(words_x, r.center().y - g.size().y / 2.0), g, color);
    let response = match tip {
        Some(tip) => response.on_hover_text(tip),
        None => response,
    };
    response.clicked().then_some(click)
}

/// The middle of the window while a module is open and nothing is in it:
/// the kit's empty state, with the Area Wizard and Go to Start.
pub(crate) fn no_area(app: &mut Moonglow, ui: &mut Ui) {
    let ctx = ui.ctx().clone();
    // (Walked once for each change of the module, not each frame.)
    let stamp = app.ws.as_ref().map(|ws| (ws.identity(), ws.revision()));
    let areas = match (stamp, app.welcome.areas) {
        (Some(s), Some((kept, areas))) if kept == s => areas,
        (Some(s), _) => {
            count(Counted::AreaWalk);
            let ws = app.ws.as_ref().expect("a module");
            let areas = !mg_module::areas::every(&ws.module).is_empty();
            app.welcome.areas = Some((s, areas));
            areas
        }
        (None, _) => false,
    };
    let keys = app.keymap.label_of(&Id::QuickOpen.id(), &ctx);
    let why = match keys.is_empty() {
        true => "Double-click an area in the Module panel, or find one with Quick Open.".into(),
        false => format!("Double-click an area in the Module panel, or find one with {keys}."),
    };
    let wizard = KitButton::new(None, Id::AreaWizard.label(app))
        .tip("Make a new area")
        .enabled(Id::AreaWizard.enabled(app));
    // (Named as the moves row names it; the command is Go to Start
    // Location. Nothing to start in without an area.)
    let start = KitButton::new(None, "Go to Start")
        .tip(Id::GoToStart.hint())
        .enabled(areas && Id::GoToStart.enabled(app));
    let shown =
        EmptyState::new(Icon::Area, "No area open").why(why).button(wizard).button(start).show(ui);
    if shown[0].clicked() {
        Id::AreaWizard.run(app, &ctx);
    }
    if shown[1].clicked() {
        Id::GoToStart.run(app, &ctx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A recent module's name has a `/` after a folder alone, a `.mod`'s
    /// and a `.nwm`'s none, in any case.
    #[test]
    fn a_recent_module_s_name_says_a_folder_by_its_slash() {
        let name = |p: &str| recent_name(Path::new(p));
        assert_eq!(name("/m/Chapter1.nwm"), "Chapter1.nwm");
        assert_eq!(name("/m/Prelude.NWM"), "Prelude.NWM");
        assert_eq!(name("/m/castle.mod"), "castle.mod");
        assert_eq!(name("/m/Keep.MOD"), "Keep.MOD");
        assert_eq!(name("/m/castle"), "castle/");
        assert_eq!(name("/m/tdod-module"), "tdod-module/");
        assert_eq!(name("/m/castle.erf"), "castle.erf/");
    }

    /// The page is never taller where it is wider: at every width from 300
    /// to 2000 points, at three zooms, with ten recent modules and with
    /// none, with game data and without, with a long update line and
    /// none. (So it never scrolls in a wider window where it fitted a
    /// narrower one.)
    #[test]
    fn the_page_is_never_taller_where_it_is_wider() {
        use crate::updates::{GetState, Getting, Release, Version};
        let release = Release {
            tag: "v99.0.0".into(),
            version: Version::parse("99.0.0").unwrap(),
            page: "https://example.invalid/release".into(),
            notes: String::new(),
            assets: Vec::new(),
        };
        let why: String =
            "the download stopped half way through, and then again a second time".into();
        for zoom in [1.0, 1.25, 1.5] {
            for case in 0..4 {
                let mut app = Moonglow::new(None, Box::new(crate::NoDialogs::default()));
                if case % 2 == 0 {
                    app.settings.recent =
                        (0..10).map(|i| PathBuf::from(format!("/srv/m/module {i}.mod"))).collect();
                }
                if case >= 2 {
                    let state = GetState::Failed(why.clone());
                    app.updates.getting =
                        Some(Getting::shown(release.clone(), true, state, 0, None));
                }
                let line = crate::updates::line(&app);
                assert_eq!(line.is_some(), case >= 2);
                let mut highs: Vec<(u32, f32)> = Vec::new();
                let mut h =
                    egui_kittest::Harness::builder().with_size(vec2(400.0, 400.0)).build_ui(|ui| {
                        mg_uikit::apply(ui.ctx(), mg_uikit::Theme::Dark);
                        ui.ctx().set_pixels_per_point(zoom);
                        highs = (300..=2000)
                            .map(|w| (w, plan(&app, ui, w as f32, line.as_ref()).high))
                            .collect();
                    });
                h.run_steps(3);
                drop(h);
                for pair in highs.windows(2) {
                    let ((narrow, before), (wide, after)) = (pair[0], pair[1]);
                    assert!(
                        after <= before,
                        "x{zoom} case {case}: {after} high at {wide}, {before} at {narrow}"
                    );
                }
                assert!(highs[0].1 > highs[highs.len() - 1].1, "folded where narrow");
            }
        }
    }

    /// Three columns where each has its least width, two where the page is
    /// narrower, one under that; never more where it is narrower.
    #[test]
    fn the_page_folds_from_three_columns_to_one() {
        assert_eq!(columns(1600.0 - 2.0 * SIDE), 3);
        assert_eq!(columns(1000.0 - 2.0 * SIDE), 2);
        assert_eq!(columns(500.0), 1);
        let mut before = 3;
        for w in (200..2000).rev() {
            let n = columns(w as f32);
            assert!(n <= before, "{w}: {n} after {before}");
            before = n;
        }
    }
}
