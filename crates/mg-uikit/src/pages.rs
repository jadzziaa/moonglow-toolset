//! A panel's pages as a rail of icons at its left edge (the mock's
//! `.rail`, Blender's Properties): 32 px of `bar` with a 1 px `line` at its
//! right; from the top, 4 px in and 2 px apart, 24 px page buttons
//! (frameless in `text-2` at rest, `raised` and `text` under the pointer,
//! `accent-fill` and `on-fill` for the page shown), each with its page's
//! icon. The Properties panel's pages (M4's U6).
//!
//! A form's pages on one scroll (M4's U9: the blueprint windows, Module
//! and Area Properties; the Creature board): [`PageList`] at the form's
//! left, the pages as rows of an icon and the name, the page at the top
//! of the scroll filled in `accent-fill`, the others in view marked by a
//! 2 px `line-soft` line at their left (the WideCreature board); and
//! [`PageScroll`], the pages one after another on one scroll, each under
//! its [`PageBar`] (26 px of `bar` with its icon and name), drawn only
//! where in sight. Each page's height is measured where it is drawn and
//! kept; the scroll keeps its place as a page and how far into it, so a
//! page measured anew (one never drawn yet, or an inventory grown) moves
//! nothing in view. A page chosen in the list lands its bar at the top.
//!
//! The rail is one group named "Pages" (or its own name); each page a
//! button named by its page's name, said selected or not, its tooltip the
//! name. Where the rail is shorter than its pages (a section dragged
//! low), it scrolls under the wheel, and the page shown is kept in sight.
//! With the keyboard on a page, Up and Down show the page before or after
//! and take the keyboard there.
//!
//! ```
//! # egui::__run_test_ui(|ui| {
//! use mg_uikit::pages::PageRail;
//! use mg_uikit::Icon;
//! let mut page = 0;
//! let changed = PageRail::new("properties-pages")
//!     .page(Icon::Info, "Basic")
//!     .page(Icon::Inventory, "Inventory")
//!     .page(Icon::Script, "Scripts")
//!     .show(ui, &mut page)
//!     .changed;
//! # let _ = changed;
//! # });
//! ```

use std::ops::Range;

use egui::{Color32, Rect, Response, Sense, Stroke, Ui, Vec2, WidgetInfo, WidgetType, pos2, vec2};

use crate::icons::{ButtonState, Icon};
use crate::metrics;
use crate::theme::TokensExt as _;

/// The rail's accessible name where it is not given one.
pub const PAGES: &str = "Pages";
/// The rail's width, its line included (`.rail`: `width: 32px`).
pub const PAGE_RAIL: f32 = 32.0;
/// A page button's side (`.rail .tool`: 24 by 24).
pub const PAGE_BUTTON: f32 = 24.0;
/// The room above the first page (`.rail`: `padding: 4px 0`).
pub const RAIL_PADDING: f32 = 4.0;
/// The gap between the pages (`.rail`: `gap: 2px`).
pub const RAIL_GAP: f32 = 2.0;

/// What a [`PageRail`] did.
#[derive(Debug)]
pub struct RailOutput {
    /// The rail's response (its whole strip).
    pub response: Response,
    /// Whether a page was clicked and the page changed.
    pub changed: bool,
    /// Each page's response, in order.
    pub pages: Vec<Response>,
}

/// A panel's pages as icons at its left edge (module comment).
#[derive(Clone, Debug)]
#[must_use = "show it with .show(ui, &mut page)"]
pub struct PageRail {
    id: egui::Id,
    name: String,
    pages: Vec<(Icon, String)>,
}

impl PageRail {
    /// A rail named "Pages", its scroll kept by `id_salt`.
    pub fn new(id_salt: impl egui::AsId) -> Self {
        PageRail { id: egui::Id::new(id_salt), name: PAGES.to_string(), pages: Vec::new() }
    }

    /// Named `name` for a screen reader (and tests) in place of "Pages".
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// A page, after those given: its icon and its name.
    pub fn page(mut self, icon: Icon, name: impl Into<String>) -> Self {
        self.pages.push((icon, name.into()));
        self
    }

    /// The height its pages take, its padding included.
    pub fn height(&self) -> f32 {
        let n = self.pages.len() as f32;
        2.0 * RAIL_PADDING + n * PAGE_BUTTON + (n - 1.0).max(0.0) * RAIL_GAP
    }

    /// Draws the rail down the height of `ui` at its left ([`PAGE_RAIL`]
    /// wide, its line included); `page` is the index of the page shown,
    /// set to the one clicked.
    pub fn show(self, ui: &mut Ui, page: &mut usize) -> RailOutput {
        let t = ui.tokens();
        let top = ui.max_rect().top();
        let left = ui.cursor().min.x;
        let rect =
            Rect::from_min_max(pos2(left, top), pos2(left + PAGE_RAIL, ui.max_rect().bottom()));
        let response = ui.interact(rect, self.id.with("mg-uikit-rail"), Sense::hover());
        ui.painter().rect_filled(rect, 0.0, t.bar);
        ui.painter().vline(rect.right() - 0.5, rect.y_range(), Stroke::new(1.0, t.line));
        let inner = Rect::from_min_max(rect.min, pos2(rect.right() - 1.0, rect.bottom()));
        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(inner)
                .layout(egui::Layout::top_down(egui::Align::Center)),
        );
        child.set_clip_rect(inner.intersect(ui.clip_rect()));
        let mut changed = false;
        let mut pages = Vec::new();
        let shown = *page;
        let name = self.name.clone();
        // (Where they do not all fit, they scroll; the page shown kept in
        // sight as it changes.)
        let kept = child.ctx().data(|d| d.get_temp::<usize>(self.id.with("shown")));
        let mut in_sight = false;
        let out = egui::ScrollArea::vertical()
            .id_salt(self.id.with("scroll"))
            .auto_shrink([false, false])
            .min_scrolled_height(0.0)
            .max_height(inner.height())
            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
            .show(&mut child, |ui| {
                crate::icons::name_group(ui, &name);
                // (The padding as room taken, which the scroll counts.)
                ui.spacing_mut().item_spacing.y = RAIL_GAP;
                ui.allocate_exact_size(vec2(PAGE_BUTTON, RAIL_PADDING - RAIL_GAP), Sense::hover());
                for (i, (icon, name)) in self.pages.iter().enumerate() {
                    let r = page_ui(ui, *icon, name, i == shown);
                    // (A page reached by Tab brought into sight: the rail
                    // lower than its pages at the least window.)
                    if r.gained_focus() && !ui.clip_rect().contains_rect(r.rect) {
                        r.scroll_to_me_animation(None, egui::style::ScrollAnimation::none());
                    }
                    if r.clicked() && i != *page {
                        *page = i;
                        changed = true;
                    }
                    if i == shown {
                        in_sight = ui.clip_rect().contains_rect(r.rect);
                        if kept != Some(shown) && !in_sight {
                            r.scroll_to_me_animation(None, egui::style::ScrollAnimation::none());
                            // (Asked once: a rail lower than a page, in a
                            // window at its least, asked each frame.)
                            in_sight = true;
                        }
                    }
                    pages.push(r);
                }
                ui.allocate_exact_size(vec2(PAGE_BUTTON, RAIL_PADDING - RAIL_GAP), Sense::hover());
            });
        paint_overflow_cues(
            &child,
            inner,
            t.bar,
            out.state.offset.y > 1.0,
            out.state.offset.y + out.inner_rect.height() < out.content_size.y - 1.0,
        );
        // (Noted once it is in sight: until then each frame brings it in.)
        if in_sight {
            child.ctx().data_mut(|d| d.insert_temp(self.id.with("shown"), shown));
        }
        changed |= move_focus_with_arrows(ui, &pages, page);
        ui.advance_cursor_after_rect(rect);
        RailOutput { response, changed, pages }
    }
}

/// Paints a band with a chevron (`text-3`) at the top and the bottom of a
/// vertical page selector where its pages go on beyond `rect`, the band
/// filled `surface`, the selector's own ground (a rail's `bar`, a
/// list's `panel`), so that the pages pass under it.
fn paint_overflow_cues(ui: &Ui, rect: Rect, surface: Color32, more_above: bool, more_below: bool) {
    if !more_above && !more_below {
        return;
    }
    let t = ui.tokens();
    let painter = ui.painter_at(rect);
    let band_h = 14.0;
    if more_above {
        let band = Rect::from_min_max(
            rect.min,
            pos2(rect.right(), (rect.top() + band_h).min(rect.bottom())),
        );
        painter.rect_filled(band, 0.0, surface);
        Icon::ChevronUp.paint(
            &painter,
            Rect::from_center_size(band.center(), vec2(12.0, 12.0)),
            t.text_3,
        );
    }
    if more_below {
        let band = Rect::from_min_max(
            pos2(rect.left(), (rect.bottom() - band_h).max(rect.top())),
            rect.max,
        );
        painter.rect_filled(band, 0.0, surface);
        Icon::ChevronDown.paint(
            &painter,
            Rect::from_center_size(band.center(), vec2(12.0, 12.0)),
            t.text_3,
        );
    }
}

/// Moves focus and selection with `ArrowUp`/`ArrowDown` while keeping focus
/// inside a vertical page selector (`PageRail` or `PageList`).
fn move_focus_with_arrows(ui: &Ui, pages: &[Response], page: &mut usize) -> bool {
    let Some(k) = pages.iter().position(Response::has_focus) else { return false };
    let filter = egui::EventFilter { vertical_arrows: true, ..Default::default() };
    ui.memory_mut(|m| m.set_focus_lock_filter(pages[k].id, filter));
    let none = egui::Modifiers::NONE;
    let down = ui.input_mut(|i| i.consume_key(none, egui::Key::ArrowDown));
    let up = ui.input_mut(|i| i.consume_key(none, egui::Key::ArrowUp));
    let to = match (up, down) {
        (true, false) => k.checked_sub(1),
        (false, true) => (k + 1 < pages.len()).then_some(k + 1),
        _ => None,
    };
    let Some(to) = to else { return false };
    *page = to;
    pages[to].request_focus();
    pages[to].scroll_to_me_animation(None, egui::style::ScrollAnimation::none());
    true
}

/// A [`PageList`]'s width, its line included (the Creature board's
/// `.pages`: `width: 150px`).
pub const PAGE_LIST: f32 = 150.0;
/// A [`PageList`]'s row height (`.list-row`: `--row-h`).
pub const PAGE_ROW: f32 = metrics::ROW_HEIGHT;
/// The room before a row's icon (`.pages .list-row`: `padding-left: 10px`).
pub const PAGE_ROW_INDENT: f32 = 10.0;
/// The gap between a row's icon and its name (`.pages .list-row`: `gap: 8px`).
pub const PAGE_ROW_GAP: f32 = 8.0;
/// The width of the mark of a page in view (`.list-row.inview`: an inset
/// 2 px line at its left).
pub const IN_VIEW_MARK: f32 = 2.0;
/// What a page's group on a [`PageScroll`] is named after its page's
/// name ("Basic page").
pub const PAGE_GROUP: &str = " page";
/// A [`PageBar`]'s height (`.pg-h`: `height: 26px`).
pub const PAGE_BAR: f32 = 26.0;
/// The room before a [`PageBar`]'s icon (`.pg-h`: `padding: 0 16px`).
pub const PAGE_BAR_PADDING: f32 = 16.0;
/// A page's margins round its fields under its bar (`.page > .content`:
/// `padding: 0 12px 12px 16px`, the bar's `margin-bottom: 10px`).
pub const PAGE_MARGIN: egui::Margin = egui::Margin { left: 16, right: 12, top: 10, bottom: 12 };
/// The room a page in columns ([`PageScroll::columns`]) that is not its
/// column's last is given for what fills the room it has (a list, a
/// memo): its bar and margins included.
pub const PAGE_FILL: f32 = 320.0;
/// The line between columns of pages (`.cw .strip`: `gap: 1px` over
/// `line`).
pub const PAGE_COLUMN_LINE: f32 = 1.0;
/// A page's margins on a scroll of pages under section headings
/// ([`PageScroll::headings`], the WideArea board's Properties: its
/// `.content`'s `padding: 8px 4px 8px 8px`); a page after the first
/// stands [`HEADING_PAGE_GAP`] under the one before, and the last has
/// `8` under it.
pub const HEADING_PAGE_MARGIN: egui::Margin = egui::Margin { left: 8, right: 4, top: 8, bottom: 0 };
/// The room between pages under section headings (WideArea's `.sec-gap`:
/// `margin-top: 14px`).
pub const HEADING_PAGE_GAP: f32 = 14.0;

/// What a [`PageList`] did.
#[derive(Debug)]
pub struct ListOutput {
    /// The list's response (its whole column).
    pub response: Response,
    /// Whether a page was chosen (clicked, or Up or Down on a row).
    pub changed: bool,
    /// Each page's row, in order.
    pub pages: Vec<Response>,
}

/// A form's pages at its left, words beside icons (module comment): the
/// page at the top filled, the other pages in view marked.
#[derive(Clone, Debug)]
#[must_use = "show it with .show(ui, &mut page)"]
pub struct PageList {
    id: egui::Id,
    name: String,
    pages: Vec<(Icon, String)>,
    in_view: Vec<usize>,
    width: f32,
}

impl PageList {
    /// A list named "Pages", its scroll kept by `id_salt`.
    pub fn new(id_salt: impl egui::AsId) -> Self {
        let id = egui::Id::new(id_salt);
        PageList {
            id,
            name: PAGES.to_string(),
            pages: Vec::new(),
            in_view: Vec::new(),
            width: PAGE_LIST,
        }
    }

    /// Named `name` for a screen reader (and tests) in place of "Pages".
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// A page, after those given: its icon and its name.
    pub fn page(mut self, icon: Icon, name: impl Into<String>) -> Self {
        self.pages.push((icon, name.into()));
        self
    }

    /// [`PAGE_LIST`]'s width replaced by `width`, its line included (the
    /// Options board's list is 160 wide, for "Conversation Editor").
    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    /// The pages in view on the scroll beside it (marked, the page at the
    /// top aside): [`PageScroll::seen`].
    pub fn in_view(mut self, pages: Range<usize>) -> Self {
        self.in_view = pages.collect();
        self
    }

    /// The pages in view on the scroll beside it, each named (marked, the
    /// page at the top aside): [`Seen::pages_in_view`], which in columns
    /// need not follow one another.
    pub fn pages_in_view(mut self, pages: impl IntoIterator<Item = usize>) -> Self {
        self.in_view = pages.into_iter().collect();
        self
    }

    /// Draws the list down the height of `ui` at its left ([`PAGE_LIST`]
    /// wide, its line included); `page` is the index of the page at the
    /// top, set to the one chosen.
    pub fn show(self, ui: &mut Ui, page: &mut usize) -> ListOutput {
        let t = ui.tokens();
        let top = ui.max_rect().top();
        let left = ui.cursor().min.x;
        let rect =
            Rect::from_min_max(pos2(left, top), pos2(left + self.width, ui.max_rect().bottom()));
        let response = ui.interact(rect, self.id.with("mg-uikit-list"), Sense::hover());
        ui.painter().rect_filled(rect, 0.0, t.panel);
        ui.painter().vline(rect.right() - 0.5, rect.y_range(), Stroke::new(1.0, t.line));
        let inner = Rect::from_min_max(rect.min, pos2(rect.right() - 1.0, rect.bottom()));
        let mut child = ui.new_child(
            egui::UiBuilder::new().max_rect(inner).layout(egui::Layout::top_down(egui::Align::Min)),
        );
        child.set_clip_rect(inner.intersect(ui.clip_rect()));
        let mut changed = false;
        let mut pages = Vec::new();
        let shown = *page;
        let name = self.name.clone();
        let kept = child.ctx().data(|d| d.get_temp::<usize>(self.id.with("shown")));
        let mut brought = false;
        let out = egui::ScrollArea::vertical()
            .id_salt(self.id.with("scroll"))
            .auto_shrink([false, false])
            .min_scrolled_height(0.0)
            .max_height(inner.height())
            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
            .show(&mut child, |ui| {
                crate::icons::name_group(ui, &name);
                ui.spacing_mut().item_spacing.y = 0.0;
                ui.allocate_exact_size(vec2(1.0, RAIL_PADDING), Sense::hover());
                for (i, (icon, name)) in self.pages.iter().enumerate() {
                    let marked = self.in_view.contains(&i) && i != shown;
                    let r = list_row_ui(ui, *icon, name, i == shown, marked);
                    if r.clicked() {
                        // (A click on the page at the top brings its bar
                        // back to the top too.)
                        *page = i;
                        changed = true;
                    }
                    if i == shown && kept != Some(shown) {
                        if !ui.clip_rect().contains_rect(r.rect) {
                            r.scroll_to_me_animation(None, egui::style::ScrollAnimation::none());
                        }
                        // (Once: a list lower than a row would ask each
                        // frame.)
                        brought = true;
                    }
                    pages.push(r);
                }
                ui.allocate_exact_size(vec2(1.0, RAIL_PADDING), Sense::hover());
            });
        paint_overflow_cues(
            &child,
            inner,
            t.panel,
            out.state.offset.y > 1.0,
            out.state.offset.y + out.inner_rect.height() < out.content_size.y - 1.0,
        );
        if brought {
            child.ctx().data_mut(|d| d.insert_temp(self.id.with("shown"), shown));
        }
        changed |= move_focus_with_arrows(ui, &pages, page);
        ui.advance_cursor_after_rect(rect);
        ListOutput { response, changed, pages }
    }
}

/// A row of a [`PageList`]: the list's width, [`PAGE_ROW`] high.
fn list_row_ui(ui: &mut Ui, icon: Icon, name: &str, on: bool, marked: bool) -> Response {
    let width = ui.available_width();
    let (rect, response) = ui.allocate_exact_size(vec2(width, PAGE_ROW), Sense::click());
    let enabled = ui.is_enabled();
    response.widget_info(|| WidgetInfo::selected(WidgetType::Button, enabled, on, name));
    if ui.is_rect_visible(rect) {
        let t = ui.tokens();
        let hovered = enabled && (response.hovered() || response.is_pointer_button_down_on());
        let fill = match (on, hovered) {
            (true, _) => t.accent_fill,
            (false, true) => t.hover_row,
            (false, false) => egui::Color32::TRANSPARENT,
        };
        let (icon_color, text_color) = if on { (t.on_fill, t.on_fill) } else { (t.text_2, t.text) };
        let painter = ui.painter();
        painter.rect_filled(rect, 0.0, fill);
        if marked {
            let mark = Rect::from_min_size(rect.min, vec2(IN_VIEW_MARK, rect.height()));
            painter.rect_filled(mark, 0.0, t.line_soft);
        }
        if response.has_focus() {
            let (ring_rect, stroke) = if on {
                (rect.shrink(1.5), Stroke::new(1.5, t.on_fill))
            } else {
                (rect.shrink(0.5), Stroke::new(1.0, t.accent))
            };
            painter.rect_stroke(ring_rect, metrics::RADIUS, stroke, egui::StrokeKind::Inside);
        }
        let x = rect.left() + PAGE_ROW_INDENT;
        let square = Rect::from_center_size(
            pos2(x + metrics::ICON / 2.0, rect.center().y),
            Vec2::splat(metrics::ICON),
        );
        icon.paint(painter, square, icon_color);
        let font = egui::FontId::proportional(metrics::BODY_SIZE);
        let text_left = x + metrics::ICON + PAGE_ROW_GAP;
        let room = (rect.right() - 6.0 - text_left).max(0.0);
        let mut job = egui::text::LayoutJob::simple_singleline(name.to_string(), font, text_color);
        job.wrap = egui::text::TextWrapping::truncate_at_width(room);
        let galley = painter.layout_job(job);
        let pos = pos2(text_left, rect.center().y - galley.size().y / 2.0);
        painter.with_clip_rect(rect).galley(pos, galley, text_color);
    }
    response.on_hover_text(name)
}

/// The bar over a page on one scroll (the Creature board's `.pg-h`):
/// 26 px of `bar` across the page's width with a 1 px `line` above (but
/// over the first page) and under it, the page's icon in `text-2` and its
/// name in `text`; drawn, not a widget (the page list names the pages).
#[derive(Clone, Debug)]
#[must_use = "show it with .show(ui)"]
pub struct PageBar {
    icon: Icon,
    name: String,
    first: bool,
}

impl PageBar {
    /// The bar of the page `name`.
    pub fn new(icon: Icon, name: impl Into<String>) -> Self {
        PageBar { icon, name: name.into(), first: false }
    }

    /// The first page's: no line above it.
    pub fn first(mut self, first: bool) -> Self {
        self.first = first;
        self
    }

    /// Draws the bar across `ui`'s width at its cursor; its rectangle.
    pub fn show(self, ui: &mut Ui) -> Rect {
        let width = ui.available_width().max(0.0);
        let (rect, _) = ui.allocate_exact_size(vec2(width, PAGE_BAR), Sense::hover());
        if ui.is_rect_visible(rect) {
            let t = ui.tokens();
            let painter = ui.painter();
            painter.rect_filled(rect, 0.0, t.bar);
            let line = Stroke::new(1.0, t.line);
            if !self.first {
                painter.hline(rect.x_range(), rect.top() + 0.5, line);
            }
            painter.hline(rect.x_range(), rect.bottom() - 0.5, line);
            let x = rect.left() + PAGE_BAR_PADDING;
            let square = Rect::from_center_size(
                pos2(x + metrics::ICON / 2.0, rect.center().y),
                Vec2::splat(metrics::ICON),
            );
            self.icon.paint(painter, square, t.text_2);
            let font = egui::FontId::proportional(metrics::BODY_SIZE);
            let galley = painter.layout_no_wrap(self.name, font, t.text);
            let pos =
                pos2(x + metrics::ICON + PAGE_ROW_GAP, rect.center().y - galley.size().y / 2.0);
            painter.with_clip_rect(rect.intersect(ui.clip_rect())).galley(pos, galley, t.text);
        }
        rect
    }
}

/// What a [`PageScroll`] showed, for the [`PageList`] beside it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Seen {
    /// The page at the top of the scroll.
    pub top: usize,
    /// The pages in view, in part or whole: from the first to the last
    /// (in columns, [`PageScroll::columns`], some between them may be out
    /// of view: [`Seen::pages_in_view`]).
    pub in_view: Range<usize>,
    /// The pages in view, in part or whole, in order.
    pub pages_in_view: Vec<usize>,
}

/// What a [`PageScroll`] did.
#[derive(Debug)]
pub struct ScrollOutput {
    /// The page at the top and the pages in view.
    pub seen: Seen,
    /// Where each page was drawn this frame (`None`: out of sight, not
    /// drawn), its bar included.
    pub drawn: Vec<Option<Rect>>,
    /// The scroll's room on screen.
    pub viewport: Rect,
}

/// What a [`PageScroll`] keeps between frames.
#[derive(Clone, Debug, Default)]
struct Kept {
    /// Each page's height as last measured (or the estimate).
    heights: Vec<f32>,
    /// Each page's width as last measured, and as measured the frame
    /// before: a page wider than the scroll widens it once it is so two
    /// frames running (a widget's first frame laid out wider brought a
    /// sideways bar, and took it away, for a frame).
    widths: Vec<(f32, f32)>,
    /// Where the scroll is: a page and how far into it (above its top
    /// where negative: a page above it, not yet measured, in view).
    anchor: (usize, f32),
    /// The page at the top it last reported (a page asked for otherwise is
    /// a page to go to).
    reported: Option<usize>,
    /// What it showed.
    seen: Seen,
    /// The page the keyboard was in, drawn while it is.
    focus_page: Option<usize>,
    /// The widget with the keyboard last frame.
    focused: Option<egui::Id>,
    /// The pass a Tab was last pressed in.
    tab_pass: Option<u64>,
    /// In columns ([`PageScroll::columns`]): each page's height as last
    /// measured (a page that fills the room it is given, a list's, at
    /// [`PAGE_FILL`]), or the estimate.
    natural: Vec<f32>,
    /// In columns: whether each page fills the room it is given (it
    /// measured as high as its room), as last drawn.
    fills: Vec<bool>,
    /// In columns: whether each page has been measured.
    measured: Vec<bool>,
    /// In columns: the column each page stands in, and how many columns
    /// they were flowed into: kept while that count and the pages stay,
    /// so that a page growing moves no page into another column.
    flowed: (Vec<usize>, usize),
    /// Each page's height changes, to hold one that flips ([`Flip`]).
    flips: Vec<Flip>,
}

/// How a page's height has been changing, to hold one that will not
/// settle. A page whose height follows where it stands on screen (a part
/// of it sized by the room left in sight, as the Appearance page's model
/// beside its fields was) measures anew each time it is placed: it flips
/// between two heights, or grows each pass, and each change had the pass
/// drawn again, every frame, for as long as it was in view (egui's
/// warning of a discard a thousand frames in a row, M4's U11 review).
#[derive(Clone, Copy, Debug, Default)]
struct Flip {
    /// Its height before its last change, and the frame of that change.
    before: Option<f32>,
    frame: u64,
    /// The frames in a row it has changed in.
    run: u32,
    /// The height it is held at, and the frame it was held from.
    held: Option<(f32, u64)>,
}

/// The frames a page that will not settle is held at a height before it
/// is measured again: about a third of a second, so a page that grows for
/// a while (pictures coming in) is followed at that pace.
const HELD_FRAMES: u64 = 20;

impl Flip {
    /// The height to place the page by, from `kept` (as placed so far) and
    /// `measured` in frame `frame`: as measured, but where it changes back
    /// to the height it had before at once, or changes in two frames
    /// running, the larger, held for [`HELD_FRAMES`] (a pass is drawn
    /// again for it at most twice in that time).
    fn hold(&mut self, kept: f32, measured: f32, frame: u64) -> f32 {
        let near = |a: f32, b: f32| (a - b).abs() <= 0.5;
        if let Some((height, since)) = self.held {
            if frame < since + HELD_FRAMES {
                return height;
            }
            self.held = None;
            self.run = 0;
        }
        if near(measured, kept) {
            return measured;
        }
        let soon = frame <= self.frame + 2;
        let back = soon && self.before.is_some_and(|b| near(b, measured));
        self.run = if frame == self.frame {
            self.run
        } else if soon {
            self.run + 1
        } else {
            1
        };
        if back || self.run >= 2 {
            let height = measured.max(kept);
            self.held = Some((height, frame));
            return height;
        }
        self.before = Some(kept);
        self.frame = frame;
        measured
    }
}

/// A form's pages one after another on one scroll (module comment),
/// each under its [`PageBar`] and in its margins ([`PAGE_MARGIN`]),
/// drawn only where in sight: those in view, the page with the
/// keyboard (whose field would lose what is typed if it went undrawn),
/// and on a Tab or Shift + Tab the pages on either side (the keyboard
/// goes from page to page; a widget it reaches out of sight is scrolled
/// into view).
///
/// Each page is drawn in a room one view high (what fills the height a
/// page has, a list or a memo, fills a view, as on a page alone) with
/// ids of its own. After the last page, room to bring its bar to the top.
#[derive(Clone, Debug)]
#[must_use = "show it with .show(ui, &mut page, |ui, i| ...)"]
pub struct PageScroll {
    id: egui::Id,
    pages: Vec<(Icon, String)>,
    sideways: bool,
    bring: bool,
    estimate: f32,
    columns: usize,
    headings: bool,
}

impl PageScroll {
    /// A scroll of pages, kept by `id_salt` (the [`PageList`] beside it
    /// reads it by the same: [`PageScroll::seen`]).
    pub fn new(id_salt: impl egui::AsId) -> Self {
        PageScroll {
            id: egui::Id::new(id_salt),
            pages: Vec::new(),
            sideways: false,
            bring: false,
            estimate: 400.0,
            columns: 1,
            headings: false,
        }
    }

    /// Each page under its name as a section heading (`section_heading`,
    /// the WideArea board's Properties: `.sec-h`), in
    /// [`HEADING_PAGE_MARGIN`], [`HEADING_PAGE_GAP`] apart, in place of a
    /// [`PageBar`] in [`PAGE_MARGIN`]: a panel's pages beside a
    /// [`PageRail`], which is their index.
    pub fn headings(mut self, headings: bool) -> Self {
        self.headings = headings;
        self
    }

    /// A page's margins: [`PAGE_MARGIN`] under its bar, or under its
    /// heading [`HEADING_PAGE_MARGIN`], the gap above a page after the
    /// first and room under the last.
    fn margin(&self, i: usize) -> egui::Margin {
        if !self.headings {
            return PAGE_MARGIN;
        }
        let mut m = HEADING_PAGE_MARGIN;
        if i > 0 {
            m.top = HEADING_PAGE_GAP as i8;
        }
        if i + 1 == self.pages.len() {
            m.bottom = HEADING_PAGE_MARGIN.top;
        }
        m
    }

    /// Draws page `i` (`name`) in `child`: its bar, or its heading, and
    /// what `add` draws, in its margins.
    fn page_ui(&self, child: &mut Ui, i: usize, first: bool, add: &mut impl FnMut(&mut Ui, usize)) {
        let (icon, name) = &self.pages[i];
        if !self.headings {
            PageBar::new(*icon, name.clone()).first(first).show(child);
        }
        egui::Frame::NONE.inner_margin(self.margin(i)).show(child, |ui| {
            ui.set_min_width(ui.available_width());
            if self.headings {
                crate::form::section_heading(ui, name.clone());
            }
            add(ui, i);
        });
    }

    /// A page, after those given: its bar's icon and name.
    pub fn page(mut self, icon: Icon, name: impl Into<String>) -> Self {
        self.pages.push((icon, name.into()));
        self
    }

    /// Scrolls sideways too (a window too narrow for its pages).
    pub fn sideways(mut self, sideways: bool) -> Self {
        self.sideways = sideways;
        self
    }

    /// The pages side by side in `columns` columns of the scroll's width
    /// (the WideCreature board's), a line between them: each page goes
    /// under the column that ends highest, as last measured (the first
    /// pages heading the columns); the last page of a column reaches down
    /// to the scroll's foot (what fills the room it has, a list or a memo,
    /// fills it), the others are given [`PAGE_FILL`] for what fills. Drawn
    /// in the pages' order, which the keyboard goes in. No sideways scroll,
    /// and no room after the last page: a page brought to the top comes as
    /// near it as the scroll goes, and is the page at the top while its bar
    /// is in view.
    pub fn columns(mut self, columns: usize) -> Self {
        self.columns = columns.max(1);
        self
    }

    /// Brings the page given to [`PageScroll::show`] to the top though it
    /// is the page reported last (chosen again in the list, scrolled
    /// part of the way into).
    pub fn bring(mut self, bring: bool) -> Self {
        self.bring = bring;
        self
    }

    /// What the scroll kept by `id_salt` showed last: the page at the top
    /// and the pages in view (`None` before it is first drawn).
    pub fn seen(ctx: &egui::Context, id_salt: impl egui::AsId) -> Option<Seen> {
        let id = egui::Id::new(id_salt).with("mg-uikit-page-scroll");
        ctx.data(|d| d.get_temp::<Kept>(id)).filter(|k| k.reported.is_some()).map(|k| k.seen)
    }

    /// Draws the pages in the rest of `ui`, each by `add` (`ui`, the
    /// page's index) under its bar. `page`: the page to show, if it is
    /// not the one reported last (a page chosen in the list, or asked for
    /// from elsewhere), its bar brought to the top; set to the page at
    /// the top.
    pub fn show(
        self,
        ui: &mut Ui,
        page: &mut usize,
        mut add: impl FnMut(&mut Ui, usize),
    ) -> ScrollOutput {
        let n = self.pages.len();
        let kept_id = self.id.with("mg-uikit-page-scroll");
        let mut kept = ui.data(|d| d.get_temp::<Kept>(kept_id)).unwrap_or_default();
        kept.heights.resize(n, self.estimate);
        kept.widths.resize(n, (0.0, 0.0));
        kept.flips.resize(n, Flip::default());
        let frame = ui.ctx().cumulative_frame_nr();
        if n == 0 {
            return ScrollOutput {
                seen: Seen::default(),
                drawn: Vec::new(),
                viewport: Rect::NOTHING,
            };
        }
        let asked = (*page).min(n - 1);
        if self.bring || (kept.reported != Some(asked) && (kept.reported.is_some() || asked != 0)) {
            kept.anchor = (asked, 0.0);
        }
        kept.anchor.0 = kept.anchor.0.min(n - 1);
        if self.columns > 1 {
            return self.show_columns(ui, page, add, kept);
        }
        let top_of = |heights: &[f32], i: usize| heights[..i].iter().sum::<f32>();
        let (anchor, into) = kept.anchor;
        let start = top_of(&kept.heights, anchor) + into;
        let scroll_id_salt = self.id.with("scroll");
        let area = match self.sideways {
            true => egui::ScrollArea::both(),
            false => egui::ScrollArea::vertical(),
        };
        let mut area = area.id_salt(scroll_id_salt).auto_shrink([false, false]);
        // (Set only where it differs from egui's own: its animations to
        // a target go on. egui's is kept by the scroll's id under the
        // `Ui` it is in: a window dragged out has another, at the top.)
        let scroll_id = ui.make_persistent_id(scroll_id_salt);
        let now = egui::scroll_area::State::load(ui.ctx(), scroll_id).map(|s| s.offset.y);
        if now.is_none_or(|o| (o - start).abs() > 0.5) {
            area = area.vertical_scroll_offset(start);
        }
        // (A Tab's pass and the one after it: Shift + Tab gives the
        // keyboard to the widget before in the next pass.)
        let pass = ui.ctx().cumulative_pass_nr();
        if ui.input(|i| i.key_pressed(egui::Key::Tab)) {
            kept.tab_pass = Some(pass);
        }
        let tab = kept.tab_pass.is_some_and(|at| pass <= at + 1);
        let mut focused = None;
        let mut drawn: Vec<Option<Rect>> = vec![None; n];
        let mut heights = kept.heights.clone();
        let mut remeasured_above = false;
        let mut viewport_screen = Rect::NOTHING;
        let mut focus_page = None;
        let mut focus_moved = None;
        // (A page's room is the scroll's height whether or not a sideways
        // bar takes some of it: a page measured one way, then the other,
        // brought the bar and took it away again each frame.)
        let room_h = ui.available_height().max(1.0);
        let out = area.show_viewport(ui, |ui, viewport| {
            let origin = ui.max_rect().min;
            let width = ui.available_width();
            let view_h = room_h;
            viewport_screen = viewport.translate(origin.to_vec2());
            // (What is in view by the kept heights, and the pages beside
            // it on a Tab.)
            let mut tops = Vec::with_capacity(n);
            let mut y = 0.0;
            for h in &kept.heights {
                tops.push(y);
                y += h;
            }
            let shown: Vec<bool> = (0..n)
                .map(|i| tops[i] < viewport.max.y && tops[i] + kept.heights[i] > viewport.min.y)
                .collect();
            let first = shown.iter().position(|s| *s).unwrap_or(anchor);
            let last = shown.iter().rposition(|s| *s).unwrap_or(anchor);
            let mut right = origin.x + width;
            // (Pages before the one held are placed up from it by their
            // kept heights, at the places the kept heights give them; those
            // after, down from it by what they measure.)
            let mut y = tops[anchor];
            for i in 0..n {
                if i <= anchor {
                    y = tops[i];
                }
                // (The page before the first in sight only where the
                // keyboard is in the scroll already: from the page list,
                // drawn before the scroll, a Tab would go to it.)
                let inside = kept.focus_page.is_some();
                let draw = shown[i]
                    || kept.focus_page == Some(i)
                    || (tab && ((inside && i + 1 == first) || i == last + 1));
                if !draw {
                    y += kept.heights[i];
                    continue;
                }
                let rect = Rect::from_min_size(origin + vec2(0.0, y), vec2(width, view_h));
                let mut child = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(rect)
                        // (An id of its own, not of the pages drawn before
                        // it: its widgets' ids, the keyboard's and what is
                        // typed with them, stay as pages come into sight
                        // and go.)
                        .id(self.id.with(("mg-uikit-page", i)))
                        .layout(egui::Layout::top_down(egui::Align::Min)),
                );
                let name = &self.pages[i].1;
                // (Each page a group named by it, "Basic page", for a
                // screen reader and the tests, its bounds the part of it
                // in sight.)
                crate::icons::name_group(&child, &format!("{name}{PAGE_GROUP}"));
                let before = ui.memory(|m| m.focused());
                self.page_ui(&mut child, i, i == 0, &mut add);
                // (The keyboard come to a widget of this page as it was
                // drawn: a Tab, a click.)
                let after = ui.memory(|m| m.focused());
                if after.is_some() && after != before {
                    focus_moved = Some(i);
                }
                let r = child.min_rect();
                // (The group's bounds: the part of the page in sight.)
                let seen = r.intersect(viewport_screen);
                let seen =
                    if seen.is_positive() { seen } else { Rect::from_min_size(r.min, Vec2::ZERO) };
                ui.ctx().accesskit_node_builder(child.unique_id(), |node| {
                    node.set_bounds(egui::accesskit::Rect {
                        x0: seen.min.x.into(),
                        y0: seen.min.y.into(),
                        x1: seen.max.x.into(),
                        y1: seen.max.y.into(),
                    });
                });
                let (was, _) = kept.widths[i];
                kept.widths[i] = (r.width(), was);
                if (r.width() - was).abs() < 0.5 {
                    right = right.max(r.right());
                }
                let measured = r.height().max(PAGE_BAR).round();
                let measured = kept.flips[i].hold(heights[i], measured, frame);
                drawn[i] = Some(Rect::from_min_size(r.min, vec2(width.max(r.width()), measured)));
                // (A page above the one held drawn at its kept height's
                // place: measured otherwise, the pass is drawn again.)
                if i < anchor && (measured - heights[i]).abs() > 0.5 {
                    remeasured_above = true;
                }
                heights[i] = measured;
                y += measured;
            }
            let (f, fp) = keyboard_page(ui, &kept, focus_moved, &drawn, viewport_screen);
            focused = f;
            focus_page = fp;
            // (Room after the last page for its bar to come to the top.)
            let total: f32 = heights.iter().sum();
            let tail = (view_h - heights[n - 1]).max(0.0);
            let all = Rect::from_min_max(origin, pos2(right, origin.y + total + tail));
            ui.advance_cursor_after_rect(all);
        });
        // Where egui left it (the wheel, the bar, a widget scrolled to):
        // as a page and how far into it, by the heights now measured; held
        // to the page it was on where the page it comes to is not measured
        // yet (its height a guess: its height found when it is drawn moves
        // nothing in view).
        let into_now = into + (out.state.offset.y - start);
        let held_top = top_of(&heights, anchor);
        let at = (held_top + into_now).max(0.0);
        let page_at = |at: f32| {
            let mut y = 0.0;
            // (A page less than a point of it in sight, as the heights
            // measured to whole points leave one, is past.)
            for (i, h) in heights.iter().enumerate() {
                if at < y + h - 0.5 || i == n - 1 {
                    return (i, (at - y).max(0.0));
                }
                y += h;
            }
            (n - 1, 0.0)
        };
        let (top, top_into) = page_at(at);
        // (Held to the page at the top where its bar is in view, at the
        // top (a page brought there stays there as it is measured); else
        // to the first page whose bar is in view, where one is: a page
        // above it that grows, an inventory filling, or measured for the
        // first time, grows upward and moves nothing in view; else to the
        // page at the top.)
        let view_h = out.inner_rect.height();
        let bar_in_view = ((top + 1)..n).filter(|_| top_into > 0.5).find(|&p| {
            let y = top_of(&heights, p);
            y > at && y < at + view_h
        });
        let new_anchor = match bar_in_view {
            Some(p) => (p, at - top_of(&heights, p)),
            None => (top, top_into),
        };
        let offset = top_of(&heights, new_anchor.0) + new_anchor.1;
        if remeasured_above {
            ui.ctx().request_discard("a page above the one in view measured");
        }
        let view = out.inner_rect;
        let view_top = offset;
        let view_bottom = offset + view.height();
        let mut y = 0.0;
        let mut first = None;
        let mut last = top;
        for (i, h) in heights.iter().enumerate() {
            if y < view_bottom && y + h > view_top {
                first.get_or_insert(i);
                last = i;
            }
            y += h;
        }
        let first = first.unwrap_or(top);
        let seen =
            Seen { top, in_view: first..last + 1, pages_in_view: (first..last + 1).collect() };
        if seen != kept.seen || new_anchor != kept.anchor {
            ui.ctx().request_repaint();
        }
        kept.heights = heights;
        kept.anchor = new_anchor;
        kept.reported = Some(top);
        kept.seen = seen.clone();
        kept.focus_page = focus_page;
        kept.focused = focused;
        ui.data_mut(|d| d.insert_temp(kept_id, kept));
        *page = top;
        ScrollOutput { seen, drawn, viewport: viewport_screen }
    }

    /// [`PageScroll::show`] in columns ([`PageScroll::columns`]), from
    /// where it has taken the page asked for.
    fn show_columns(
        self,
        ui: &mut Ui,
        page: &mut usize,
        mut add: impl FnMut(&mut Ui, usize),
        mut kept: Kept,
    ) -> ScrollOutput {
        let n = self.pages.len();
        let k = self.columns;
        let kept_id = self.id.with("mg-uikit-page-scroll");
        kept.natural.resize(n, self.estimate);
        kept.fills.resize(n, false);
        let frame = ui.ctx().cumulative_frame_nr();
        kept.measured.resize(n, false);
        // (Flowed anew only where the columns or the pages changed, or a
        // page was measured for the first time: else each page keeps its
        // column, and a page growing moves only those under it.)
        if kept.flowed.1 != k || kept.flowed.0.len() != n {
            kept.flowed = (flow(&kept.natural, k), k);
        }
        let (anchor, into) = kept.anchor;
        let room_h = ui.available_height().max(1.0);
        let before = ColumnLayout::of(&kept.flowed.0, &kept.natural, &kept.fills, k, room_h);
        let start = before.tops[anchor] + into;
        let scroll_id_salt = self.id.with("scroll");
        let mut area =
            egui::ScrollArea::vertical().id_salt(scroll_id_salt).auto_shrink([false, false]);
        let scroll_id = ui.make_persistent_id(scroll_id_salt);
        let now = egui::scroll_area::State::load(ui.ctx(), scroll_id).map(|s| s.offset.y);
        if now.is_none_or(|o| (o - start).abs() > 0.5) {
            area = area.vertical_scroll_offset(start);
        }
        let pass = ui.ctx().cumulative_pass_nr();
        if ui.input(|i| i.key_pressed(egui::Key::Tab)) {
            kept.tab_pass = Some(pass);
        }
        let tab = kept.tab_pass.is_some_and(|at| pass <= at + 1);
        let mut focused = None;
        let mut focus_page = None;
        let mut focus_moved = None;
        let mut drawn: Vec<Option<Rect>> = vec![None; n];
        let mut natural = kept.natural.clone();
        let mut fills = kept.fills.clone();
        let mut measured_now = kept.measured.clone();
        let mut viewport_screen = Rect::NOTHING;
        let mut redraw = false;
        let out = area.show_viewport(ui, |ui, viewport| {
            let origin = ui.max_rect().min;
            let width = ui.available_width();
            let line = PAGE_COLUMN_LINE;
            let column_w = ((width - line * (k - 1) as f32) / k as f32).max(1.0);
            viewport_screen = viewport.translate(origin.to_vec2());
            let mut lowest = 0.0_f32;
            let shown: Vec<bool> = (0..n)
                .map(|i| {
                    let top = before.tops[i];
                    top < viewport.max.y && top + before.heights[i] > viewport.min.y
                })
                .collect();
            let first = shown.iter().position(|s| *s).unwrap_or(anchor);
            let last = shown.iter().rposition(|s| *s).unwrap_or(anchor);
            // (On a Tab, the pages before and after the one with the
            // keyboard too: in columns they need not be in view beside it.)
            let keys = kept.focus_page;
            for i in 0..n {
                let inside = keys.is_some();
                let beside = keys.is_some_and(|f| i + 1 == f || i == f + 1);
                // (And each page not measured yet, once, wherever it is: a
                // column's spare room is shared by what each page is.)
                let draw = shown[i]
                    || keys == Some(i)
                    || !kept.measured[i]
                    || (tab && (beside || (inside && i + 1 == first) || i == last + 1));
                if !draw {
                    continue;
                }
                let c = before.column[i];
                // (A page measured for the first time is given a room past
                // any page's own height, which tells one that fills it
                // from one that does not; one that fills is given its
                // share of what its column has to spare, what it has
                // under what fills kept; one that does not, what it
                // measures and more.)
                let over = (kept.natural[i] - PAGE_FILL).max(0.0);
                let room = match (kept.measured[i], kept.fills[i]) {
                    (false, _) => PROBE_ROOM,
                    (true, true) => before.heights[i] - over,
                    (true, false) => before.heights[i].max(kept.natural[i]),
                }
                .max(PAGE_BAR);
                let at = origin + vec2(c as f32 * (column_w + line), before.tops[i]);
                let rect = Rect::from_min_size(at, vec2(column_w, room));
                let mut child = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(rect)
                        .id(self.id.with(("mg-uikit-page", i)))
                        .layout(egui::Layout::top_down(egui::Align::Min)),
                );
                child.set_max_width(column_w);
                // (Clipped to its column, as the board's `.pcol` is: what a
                // page lays out by the room it sees, a model beside its
                // fields, takes its column's.)
                let column =
                    Rect::from_x_y_ranges(at.x..=at.x + column_w, ui.clip_rect().y_range());
                child.set_clip_rect(column.intersect(ui.clip_rect()));
                let name = &self.pages[i].1;
                crate::icons::name_group(&child, &format!("{name}{PAGE_GROUP}"));
                let was = ui.memory(|m| m.focused());
                self.page_ui(&mut child, i, before.tops[i] < 0.5, &mut add);
                let after = ui.memory(|m| m.focused());
                if after.is_some() && after != was {
                    focus_moved = Some(i);
                }
                let r = child.min_rect();
                let seen = r.intersect(viewport_screen);
                let seen =
                    if seen.is_positive() { seen } else { Rect::from_min_size(r.min, Vec2::ZERO) };
                ui.ctx().accesskit_node_builder(child.unique_id(), |node| {
                    node.set_bounds(egui::accesskit::Rect {
                        x0: seen.min.x.into(),
                        y0: seen.min.y.into(),
                        x1: seen.max.x.into(),
                        y1: seen.max.y.into(),
                    });
                });
                let measured = r.height().max(PAGE_BAR).round();
                drawn[i] = Some(Rect::from_min_size(r.min, vec2(column_w, measured)));
                // (What places the pages: what a page measures, or for one
                // that filled its room, [`PAGE_FILL`], the least it is
                // given, so that what it is given to spare is not taken
                // back as its own height.)
                // (Whether it fills, by its first measure; what places it:
                // what it measures, or for one that fills, [`PAGE_FILL`]
                // and what it has under what fills.)
                if !kept.measured[i] {
                    fills[i] = measured >= PROBE_ROOM * FILL_TELL;
                }
                let placed = match fills[i] {
                    true => PAGE_FILL + (measured - room).max(0.0),
                    false => measured,
                };
                natural[i] = kept.flips[i].hold(natural[i], placed, frame);
                if !measured_now[i] {
                    measured_now[i] = true;
                    // (Its first height: the pages flowed again by it.)
                    redraw = true;
                }
                lowest = lowest.max(r.top() - origin.y + measured);
            }
            // (The lines between the columns, down the view.)
            let stroke = Stroke::new(line, ui.tokens().line);
            for c in 1..k {
                let x = origin.x + c as f32 * column_w + (c - 1) as f32 * line + line / 2.0;
                ui.painter().vline(x, viewport_screen.y_range(), stroke);
            }
            let (f, fp) = keyboard_page(ui, &kept, focus_moved, &drawn, viewport_screen);
            focused = f;
            focus_page = fp;
            let after = ColumnLayout::of(&kept.flowed.0, &natural, &fills, k, room_h);
            // (A page drawn where the heights now measured put it
            // elsewhere: the pass drawn again, before it is shown.)
            redraw |= (0..n).any(|i| {
                drawn[i].is_some()
                    && ((after.tops[i] - before.tops[i]).abs() > 0.5
                        || (after.heights[i] - before.heights[i]).abs() > 0.5)
            });
            // (Down to the foot, or to a page drawn past it.)
            let all = Rect::from_min_max(
                origin,
                pos2(origin.x + width, origin.y + after.foot.max(lowest)),
            );
            ui.advance_cursor_after_rect(all);
        });
        // (A page measured for the first time flows the pages again.)
        if measured_now.iter().zip(&kept.measured).any(|(now, was)| *now && !*was) {
            kept.flowed = (flow(&natural, k), k);
        }
        if redraw {
            ui.ctx().request_discard("a page in columns measured anew");
        }
        let layout = ColumnLayout::of(&kept.flowed.0, &natural, &fills, k, room_h);
        let view_h = out.inner_rect.height();
        // (Where egui left it, as a place in the page held: by the heights
        // now measured, a page above it grown, it stays where it was in
        // view.)
        let into_now = into + (out.state.offset.y - start);
        let at = (layout.tops[anchor] + into_now).max(0.0);
        // (The page held while its bar is in view: brought there, or as
        // the wheel goes; else the first page whose bar is in view; else
        // the first in view.)
        let bar_in_view =
            |p: usize| layout.tops[p] >= at - 0.5 && layout.tops[p] + PAGE_BAR <= at + view_h + 0.5;
        let in_view =
            |p: usize| layout.tops[p] < at + view_h && layout.tops[p] + layout.heights[p] > at;
        let top = match bar_in_view(anchor) {
            true => anchor,
            false => (0..n)
                .find(|&p| bar_in_view(p))
                .or_else(|| (0..n).find(|&p| in_view(p)))
                .unwrap_or(anchor),
        };
        let new_anchor = (top, at - layout.tops[top]);
        let pages_in_view: Vec<usize> = (0..n).filter(|&p| in_view(p)).collect();
        let first = pages_in_view.first().copied().unwrap_or(top);
        let last = pages_in_view.last().copied().unwrap_or(top);
        let seen = Seen { top, in_view: first..last + 1, pages_in_view };
        if seen != kept.seen
            || (new_anchor.0 != kept.anchor.0)
            || (new_anchor.1 - kept.anchor.1).abs() > 0.5
        {
            ui.ctx().request_repaint();
        }
        kept.natural = natural;
        kept.fills = fills;
        kept.measured = measured_now;
        kept.anchor = new_anchor;
        kept.reported = Some(top);
        kept.seen = seen.clone();
        kept.focus_page = focus_page;
        kept.focused = focused;
        ui.data_mut(|d| d.insert_temp(kept_id, kept));
        *page = top;
        ScrollOutput { seen, drawn, viewport: viewport_screen }
    }
}

/// The room a page in columns is given the first time it is drawn: past
/// any page's own height, so that a page that measures much of it
/// ([`FILL_TELL`]) fills the room it is given, a list's. (Drawn so in a
/// pass that is drawn again before it is shown.)
const PROBE_ROOM: f32 = 4000.0;
/// How much of [`PROBE_ROOM`] a page that fills the room it is given
/// measures at least (a list may take a share of its room).
const FILL_TELL: f32 = 0.5;

/// Each page's column, the pages `heights` high flowed into `columns`
/// columns: each under the column that ends highest (the first of two
/// within half a point), in order.
fn flow(heights: &[f32], columns: usize) -> Vec<usize> {
    let mut bottoms = vec![0.0_f32; columns];
    let mut column = Vec::with_capacity(heights.len());
    for h in heights {
        let mut c = 0;
        for (j, b) in bottoms.iter().enumerate() {
            if *b < bottoms[c] - 0.5 {
                c = j;
            }
        }
        column.push(c);
        bottoms[c] += h;
    }
    column
}

/// Where pages stand in columns ([`PageScroll::columns`]), each in its
/// column ([`flow`]): one under another by their heights, what a column
/// has to spare down to the view's foot shared among its pages that fill
/// their room, or given to its last page where none does; the scroll
/// reaches down to the tallest column ([`ColumnLayout::foot`]).
#[derive(Clone, Debug, PartialEq)]
struct ColumnLayout {
    column: Vec<usize>,
    tops: Vec<f32>,
    /// How high each page stands, its share of its column's spare room
    /// with it.
    heights: Vec<f32>,
    /// The foot the columns reach down to.
    foot: f32,
}

impl ColumnLayout {
    fn of(column: &[usize], natural: &[f32], fills: &[bool], columns: usize, view_h: f32) -> Self {
        let n = natural.len();
        let mut bottoms = vec![0.0_f32; columns];
        let mut fill_count = vec![0usize; columns];
        let mut last = vec![None; columns];
        for i in 0..n {
            let c = column[i].min(columns - 1);
            bottoms[c] += natural[i];
            if fills[i] {
                fill_count[c] += 1;
            }
            last[c] = Some(i);
        }
        let foot = bottoms.iter().copied().fold(view_h, f32::max);
        let mut heights = natural.to_vec();
        for i in 0..n {
            let c = column[i].min(columns - 1);
            // (What the column has to spare in the view: a column taller
            // than the view gives a short one nothing to stretch to.)
            let spare = (view_h - bottoms[c]).max(0.0);
            if fills[i] {
                heights[i] += spare / fill_count[c] as f32;
            } else if fill_count[c] == 0 && last[c] == Some(i) {
                heights[i] += spare;
            }
        }
        let mut y = vec![0.0_f32; columns];
        let mut tops = Vec::with_capacity(n);
        for i in 0..n {
            let c = column[i].min(columns - 1);
            tops.push(y[c]);
            y[c] += heights[i];
        }
        ColumnLayout {
            column: column.iter().map(|c| (*c).min(columns - 1)).collect(),
            tops,
            heights,
            foot,
        }
    }
}

/// The page with the keyboard after a pass: where it came this pass,
/// else where it was while it stays on the same widget; and the widget
/// with it. One the keyboard came to out of sight, by a key, is brought
/// into view.
fn keyboard_page(
    ui: &Ui,
    kept: &Kept,
    focus_moved: Option<usize>,
    drawn: &[Option<Rect>],
    viewport_screen: Rect,
) -> (Option<egui::Id>, Option<usize>) {
    // (The page with the keyboard: where it came this frame, else
    // where it was while it stays on the same widget. One the
    // keyboard came to out of sight, by a key, is brought into
    // view: its rectangle read only then, as reading a response
    // after a press elsewhere takes the keyboard from it.)
    let focused = ui.memory(|m| m.focused());
    let pressed = ui.input(|i| i.pointer.any_pressed() || i.pointer.any_released());
    // (Come to another widget between passes, by Shift + Tab, an
    // arrow key or code: found by its place among the pages drawn,
    // where no press can have it taken.)
    let came = focused.is_some() && focused != kept.focused && focus_moved.is_none();
    let response = |f: egui::Id| ui.ctx().read_response(f);
    let response = focused
        .filter(|_| !pressed && (focus_moved.is_some() || came))
        .and_then(response)
        .filter(|r| r.layer_id == ui.layer_id());
    // (A widget of the scroll's: in sight, or clipped away by the
    // scroll whole. A page drawn out of sight may lie under the
    // header or a panel beside the scroll, whose widgets are seen.)
    let ours =
        |r: &Response| viewport_screen.contains(r.rect.center()) || r.interact_rect.area() <= 0.0;
    let at = response.as_ref().filter(|r| ours(r)).map(|r| r.rect.center());
    let found = at.and_then(|c| drawn.iter().position(|d| d.is_some_and(|d| d.contains(c))));
    let focus_page = match focus_moved {
        Some(i) => Some(i),
        None if came => found,
        None if focused.is_some() && focused == kept.focused => kept.focus_page,
        None => None,
    };
    if let Some(r) = response
        && at.is_some()
        && focus_page.is_some()
        && !viewport_screen.contains_rect(r.rect)
    {
        ui.scroll_to_rect_animation(r.rect, None, egui::style::ScrollAnimation::none());
    }
    (focused, focus_page)
}

/// A page's button: 24 px square, painted in its state's tokens.
fn page_ui(ui: &mut Ui, icon: Icon, name: &str, on: bool) -> Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(PAGE_BUTTON), Sense::click());
    let enabled = ui.is_enabled();
    response.widget_info(|| WidgetInfo::selected(WidgetType::Button, enabled, on, name));
    if ui.is_rect_visible(rect) {
        let t = ui.tokens();
        let state = if enabled { ButtonState::of(&response) } else { ButtonState::Rest };
        let (fill, color) = match (on, state) {
            (true, ButtonState::Rest) => (t.accent_fill, t.on_fill),
            (true, _) => (t.accent_fill_hover, t.on_fill),
            (false, ButtonState::Rest) => (egui::Color32::TRANSPARENT, t.text_2),
            (false, _) => (t.raised, t.text),
        };
        let painter = ui.painter();
        painter.rect(rect, metrics::RADIUS, fill, Stroke::NONE, egui::StrokeKind::Inside);
        if response.has_focus() {
            let (ring_rect, stroke) = if on {
                (rect.shrink(1.5), Stroke::new(1.5, t.on_fill))
            } else {
                (rect, Stroke::new(1.0, t.accent))
            };
            painter.rect_stroke(ring_rect, metrics::RADIUS, stroke, egui::StrokeKind::Inside);
        }
        let square = Rect::from_center_size(rect.center(), vec2(metrics::ICON, metrics::ICON));
        icon.paint(painter, square, color);
    }
    response.on_hover_text(name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui_kittest::Harness;
    use egui_kittest::kittest::{NodeT as _, Queryable as _};

    #[test]
    fn the_rail_s_pages_are_named_buttons_at_its_measures() {
        let mut page = 1;
        let mut clicked = false;
        let mut h = Harness::builder().with_size(vec2(200.0, 300.0)).build_ui(|ui| {
            crate::apply(ui.ctx(), crate::Theme::Dark);
            let out = PageRail::new("rail")
                .page(Icon::Info, "Basic")
                .page(Icon::Inventory, "Inventory")
                .page(Icon::Lock, "Lock")
                .show(ui, &mut page);
            clicked |= out.changed;
        });
        h.run();
        let _ = h.get_by_label(PAGES);
        let basic = h.get_by_label("Basic").rect();
        let inventory = h.get_by_label("Inventory").rect();
        assert_eq!(basic.size(), Vec2::splat(PAGE_BUTTON), "24 by 24");
        assert_eq!(inventory.top() - basic.bottom(), RAIL_GAP, "2 apart");
        // (The harness's panel keeps 8 round its ui.)
        assert_eq!(basic.top() - 8.0, RAIL_PADDING, "4 under the top");
        assert_eq!(basic.center().x - 8.0, (PAGE_RAIL - 1.0) / 2.0, "centered in the rail");
        assert_eq!(
            h.get_by_label("Inventory").accesskit_node().toggled(),
            Some(egui::accesskit::Toggled::True)
        );
        assert_eq!(
            h.get_by_label("Basic").accesskit_node().toggled(),
            Some(egui::accesskit::Toggled::False)
        );
        h.get_by_label("Lock").click();
        h.run();
        drop(h);
        assert!(clicked, "a click changes the page");
        assert_eq!(page, 2);
    }

    #[test]
    fn a_rail_shorter_than_its_pages_scrolls_to_the_page_shown() {
        let mut page = 4;
        // (The rail in a room 50 high at 100 down: two pages fit.)
        let room = egui::Rect::from_min_size(pos2(20.0, 100.0), vec2(100.0, 50.0));
        let mut h = Harness::builder().with_size(vec2(200.0, 300.0)).build_ui(|ui| {
            crate::apply(ui.ctx(), crate::Theme::Dark);
            let mut part = ui.new_child(egui::UiBuilder::new().max_rect(room));
            part.set_clip_rect(room);
            let _ = PageRail::new("short")
                .page(Icon::Info, "Basic")
                .page(Icon::Inventory, "Inventory")
                .page(Icon::Lock, "Lock")
                .page(Icon::Trap, "Trap")
                .page(Icon::Script, "Scripts")
                .show(&mut part, &mut page);
        });
        h.run();
        let scripts = h.get_by_label("Scripts").rect();
        assert!(room.contains_rect(scripts), "in sight: {scripts:?}");
        let basic = h.get_by_label("Basic").rect();
        assert!(!room.intersects(basic), "the first scrolled out: {basic:?}");
    }

    #[test]
    fn up_and_down_take_the_page_and_the_keyboard_along_the_rail() {
        let mut page = 0;
        let mut h = Harness::builder().with_size(vec2(200.0, 300.0)).build_ui(|ui| {
            crate::apply(ui.ctx(), crate::Theme::Dark);
            let _ = PageRail::new("keys")
                .page(Icon::Info, "Basic")
                .page(Icon::Inventory, "Inventory")
                .page(Icon::Lock, "Lock")
                .show(ui, &mut page);
        });
        h.run();
        h.get_by_label("Basic").focus();
        h.run();
        h.key_press(egui::Key::ArrowDown);
        h.run();
        assert!(h.get_by_label("Inventory").accesskit_node().is_focused(), "the keyboard along");
        h.key_press(egui::Key::ArrowDown);
        h.run();
        h.key_press(egui::Key::ArrowDown);
        h.run();
        assert!(h.get_by_label("Lock").accesskit_node().is_focused(), "held at the last");
        h.key_press(egui::Key::ArrowUp);
        h.run();
        drop(h);
        assert_eq!(page, 1, "Down, Down (held), Up: the second");
    }

    /// Six pages on one scroll in a room 300 high beside their list,
    /// page `i` `heights[i]` high (a block named "page i" in it, at its
    /// top, and a text field named "field i" under it).
    fn pages_harness(
        heights: std::rc::Rc<std::cell::RefCell<Vec<f32>>>,
        page: std::rc::Rc<std::cell::Cell<usize>>,
    ) -> Harness<'static> {
        pages_harness_with(heights, page, false)
    }

    /// [`pages_harness`], with `extras` a button "Header" in a band 30
    /// high over the list and the scroll (the same layer), and a button
    /// "Floating" in a layer of its own over the scroll's top.
    fn pages_harness_with(
        heights: std::rc::Rc<std::cell::RefCell<Vec<f32>>>,
        page: std::rc::Rc<std::cell::Cell<usize>>,
        extras: bool,
    ) -> Harness<'static> {
        let high = if extras { 346.0 } else { 316.0 };
        Harness::builder().with_size(vec2(600.0, high)).build_ui(move |ui| {
            crate::apply(ui.ctx(), crate::Theme::Dark);
            let names = ["One", "Two", "Three", "Four", "Five", "Six"];
            let seen = PageScroll::seen(ui.ctx(), "form").map(|s| s.in_view).unwrap_or(0..0);
            let mut top = page.get();
            let mut room = ui.available_rect_before_wrap();
            if extras {
                let band = Rect::from_min_size(room.min, vec2(room.width(), 30.0));
                let mut header = ui.new_child(egui::UiBuilder::new().max_rect(band));
                header.horizontal(|ui| {
                    ui.add_space(200.0);
                    let _ = ui.button("Header");
                });
                room.min.y += 30.0;
                egui::Area::new(egui::Id::new("floating"))
                    .order(egui::Order::Foreground)
                    .fixed_pos(pos2(450.0, room.top() + 60.0))
                    .show(ui.ctx(), |ui| {
                        let _ = ui.button("Floating");
                    });
            }
            let mut left = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(Rect::from_min_size(room.min, vec2(PAGE_LIST, room.height()))),
            );
            let mut list = PageList::new("form-list").in_view(seen);
            let mut scroll = PageScroll::new("form");
            for n in names {
                list = list.page(Icon::Info, n);
                scroll = scroll.page(Icon::Info, n);
            }
            let chosen = list.show(&mut left, &mut top).changed;
            let mut body =
                ui.new_child(egui::UiBuilder::new().max_rect(Rect::from_min_max(
                    pos2(room.left() + PAGE_LIST, room.top()),
                    room.max,
                )));
            let heights = heights.borrow().clone();
            scroll.bring(chosen).show(&mut body, &mut top, |ui, i| {
                ui.add_sized(vec2(100.0, 20.0), egui::Label::new(format!("page {i}")));
                let mut text = String::new();
                let id = egui::Id::new(("field", i));
                ui.data_mut(|d| text = d.get_temp::<String>(id).unwrap_or_default());
                let field = egui::TextEdit::singleline(&mut text).id(id.with("edit"));
                ui.add(field).widget_info(|| {
                    WidgetInfo::labeled(WidgetType::TextEdit, true, format!("field {i}"))
                });
                ui.data_mut(|d| d.insert_temp(id, text));
                let rest = heights[i] - 20.0 - 22.0 - PAGE_BAR - 22.0;
                ui.allocate_exact_size(vec2(10.0, rest.max(0.0)), Sense::hover());
            });
            page.set(top);
        })
    }

    #[test]
    fn the_list_s_rows_name_the_pages_and_mark_those_in_view() {
        let heights = std::rc::Rc::new(std::cell::RefCell::new(vec![120.0; 6]));
        let page = std::rc::Rc::new(std::cell::Cell::new(0));
        let mut h = pages_harness(heights, page.clone());
        h.run();
        let one = h.get_by_label("One").rect();
        let two = h.get_by_label("Two").rect();
        assert_eq!(one.height(), PAGE_ROW, "22 high");
        assert_eq!(two.top(), one.bottom(), "one under another");
        assert_eq!(one.width(), PAGE_LIST - 1.0, "the list's width, its line aside");
        let toggled = |h: &Harness<'_>, n: &str| h.get_by_label(n).accesskit_node().toggled();
        assert_eq!(toggled(&h, "One"), Some(egui::accesskit::Toggled::True), "the page at the top");
        assert_eq!(toggled(&h, "Two"), Some(egui::accesskit::Toggled::False));
        let seen = PageScroll::seen(&h.ctx, "form").unwrap();
        assert_eq!(
            seen,
            Seen { top: 0, in_view: 0..3, pages_in_view: vec![0, 1, 2] },
            "300 high: three of 120 in view"
        );
        let _ = h.get_by_label(PAGES);
    }

    #[test]
    fn a_page_chosen_lands_its_bar_at_the_top_and_out_of_sight_is_not_drawn() {
        let heights = std::rc::Rc::new(std::cell::RefCell::new(vec![
            500.0, 900.0, 200.0, 700.0, 150.0, 120.0,
        ]));
        let page = std::rc::Rc::new(std::cell::Cell::new(0));
        let mut h = pages_harness(heights, page.clone());
        h.run();
        // (Where a page's block stands under its bar at the top.)
        let at_top = h.get_by_label("page 0").rect().top();
        assert!(h.query_by_label("page 3").is_none(), "a page out of sight is not drawn");
        h.get_by_label("Four").click();
        h.run();
        assert_eq!(page.get(), 3, "the page at the top");
        // (The room's top is the harness's 8; the bar 26, the margin 10.)
        let block = h.get_by_label("page 3").rect();
        assert_eq!(block.top(), at_top, "its bar at the top");
        assert!(h.query_by_label("page 0").is_none(), "the first out of sight, not drawn");
        // The last page, shorter than the room, comes to the top too.
        h.get_by_label("Six").click();
        h.run();
        assert_eq!(page.get(), 5);
        let block = h.get_by_label("page 5").rect();
        assert_eq!(block.top(), at_top, "the last at the top");
        // A page asked for from elsewhere is brought there as well.
        page.set(1);
        h.run();
        assert_eq!(h.get_by_label("page 1").rect().top(), at_top);
    }

    #[test]
    fn a_page_measured_anew_moves_nothing_in_view() {
        // (Pages never drawn are 400 by the estimate; the fourth, measured
        // when it comes into sight, is 150.)
        let heights = std::rc::Rc::new(std::cell::RefCell::new(vec![
            300.0, 300.0, 300.0, 150.0, 600.0, 300.0,
        ]));
        let page = std::rc::Rc::new(std::cell::Cell::new(4));
        let mut h = pages_harness(heights, page.clone());
        h.run();
        let before = h.get_by_label("page 4").rect().top();
        h.hover_at(pos2(400.0, 150.0));
        h.event(egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: vec2(0.0, 60.0),
            phase: egui::TouchPhase::Move,
            modifiers: egui::Modifiers::NONE,
        });
        // (Frame by frame: the frame the page above first comes into view
        // shows it against the page below, not at its estimated place.)
        for _ in 0..20 {
            h.step();
            if h.query_by_label("page 3").is_some() {
                break;
            }
        }
        let (three, four) =
            (h.get_by_label("Four page").rect(), h.get_by_label("Five page").rect());
        assert_eq!(three.bottom(), four.top(), "the page above against the page below, at once");
        h.run();
        let seen = PageScroll::seen(&h.ctx, "form");
        assert!(
            h.query_by_label("page 3").is_some(),
            "the page above come into sight: {seen:?} {before}"
        );
        let after = h.get_by_label("page 4").rect().top();
        assert!(
            (after - before - 60.0).abs() < 1.0,
            "moved by the wheel alone: {before} to {after}"
        );
    }

    #[test]
    fn tab_goes_from_page_to_page_and_the_page_typed_in_stays_drawn() {
        let heights = std::rc::Rc::new(std::cell::RefCell::new(vec![420.0; 6]));
        let page = std::rc::Rc::new(std::cell::Cell::new(0));
        let mut h = pages_harness(heights, page.clone());
        h.run();
        assert!(h.query_by_label("field 1").is_none(), "the second page out of sight");
        h.get_by_label("field 0").click();
        h.run();
        h.key_press(egui::Key::Tab);
        h.run();
        let field = h.get_by_label("field 1");
        assert!(field.accesskit_node().is_focused(), "Tab on to the next page's field");
        let room = Rect::from_min_size(pos2(8.0, 8.0), vec2(584.0, 300.0));
        assert!(room.contains_rect(field.rect()), "scrolled into view: {:?}", field.rect());
        // Shift + Tab back to the page before, out of sight above (the
        // second page brought to the top, the keyboard kept).
        page.set(1);
        h.run();
        assert!(h.query_by_label("page 0").is_none(), "the first page out of sight");
        h.key_press_modifiers(egui::Modifiers::SHIFT, egui::Key::Tab);
        h.run();
        let field = h.get_by_label("field 0");
        assert!(field.accesskit_node().is_focused(), "Shift + Tab back to the first page's");
        assert!(room.contains_rect(field.rect()), "scrolled into view: {:?}", field.rect());
        h.key_press(egui::Key::Tab);
        h.run();
        assert!(h.get_by_label("field 1").accesskit_node().is_focused(), "and on again");
        h.get_by_label("field 1").type_text("kept");
        h.run();
        // Scrolled far away with the wheel: the page with the keyboard is
        // still drawn, what is typed kept.
        h.hover_at(pos2(400.0, 150.0));
        for _ in 0..6 {
            h.event(egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: vec2(0.0, -300.0),
                phase: egui::TouchPhase::Move,
                modifiers: egui::Modifiers::NONE,
            });
            // (The wheel's smooth scroll takes some frames.)
            h.run_steps(12);
        }
        h.run();
        assert!(page.get() >= 4, "scrolled away: at {}", page.get());
        let field = h.get_by_label("field 1");
        assert!(field.accesskit_node().is_focused(), "the keyboard still there");
        assert_eq!(field.accesskit_node().value().as_deref(), Some("kept"));
    }

    /// The page list's mark of each page in view, as painted: a 2-point
    /// `line-soft` stripe at its row's left, the page at the top filled
    /// instead.
    #[test]
    fn the_list_paints_its_marks_of_the_pages_in_view() {
        let heights = std::rc::Rc::new(std::cell::RefCell::new(vec![120.0; 6]));
        let page = std::rc::Rc::new(std::cell::Cell::new(0));
        let mut h = pages_harness(heights, page);
        h.run();
        let t = crate::tokens::Tokens::DARK;
        let marks: Vec<Rect> = (h.output().shapes.iter())
            .filter_map(|c| match &c.shape {
                egui::Shape::Rect(r) if r.fill == t.line_soft && r.rect.width() == IN_VIEW_MARK => {
                    Some(r.rect)
                }
                _ => None,
            })
            .collect();
        let rows = |n: &str| h.get_by_label(n).rect();
        let at =
            |r: Rect| marks.iter().any(|m| m.left() == r.left() && m.center().y == r.center().y);
        assert!(at(rows("Two")) && at(rows("Three")), "the others in view marked: {marks:?}");
        assert!(!at(rows("One")) && !at(rows("Four")), "not the top's, not those out of view");
    }

    /// A page chosen again in the list, though it is the page at the top,
    /// brings its bar back to the top.
    #[test]
    fn a_page_chosen_again_comes_back_to_the_top() {
        let heights = std::rc::Rc::new(std::cell::RefCell::new(vec![600.0; 6]));
        let page = std::rc::Rc::new(std::cell::Cell::new(0));
        let mut h = pages_harness(heights, page.clone());
        h.run();
        let at_top = h.get_by_label("page 0").rect().top();
        h.get_by_label("Four").click();
        h.run();
        h.hover_at(pos2(400.0, 150.0));
        h.event(egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: vec2(0.0, -120.0),
            phase: egui::TouchPhase::Move,
            modifiers: egui::Modifiers::NONE,
        });
        h.run_steps(12);
        h.run();
        assert_eq!(page.get(), 3, "still the page at the top");
        assert!(h.get_by_label("page 3").rect().top() < at_top - 60.0, "scrolled into it");
        h.get_by_label("Four").click();
        h.run();
        assert_eq!(h.get_by_label("page 3").rect().top(), at_top, "its bar at the top again");
    }

    /// Tab from the page list goes into the page at the top (the page
    /// before it, out of sight, is not drawn for it), the page staying.
    #[test]
    fn tab_from_the_list_goes_into_the_page_at_the_top() {
        let heights = std::rc::Rc::new(std::cell::RefCell::new(vec![420.0; 6]));
        let page = std::rc::Rc::new(std::cell::Cell::new(3));
        let mut h = pages_harness(heights, page.clone());
        h.run();
        h.get_by_label("Six").focus();
        h.run();
        h.key_press(egui::Key::Tab);
        h.run();
        assert!(h.get_by_label("field 3").accesskit_node().is_focused(), "the top page's field");
        assert_eq!(page.get(), 3, "the page at the top unchanged");
    }

    /// The keyboard come to a widget outside the scroll moves no page: one
    /// in the header over it (where a page drawn out of sight lies, its
    /// keyboard kept), or one of another layer over a page in sight (that
    /// page then not kept drawn when the scroll goes away).
    #[test]
    fn the_keyboard_outside_the_scroll_moves_no_page() {
        let heights = std::rc::Rc::new(std::cell::RefCell::new(vec![420.0; 6]));
        let page = std::rc::Rc::new(std::cell::Cell::new(0));
        let mut h = pages_harness_with(heights, page.clone(), true);
        h.run();
        // (The keyboard in the second page's field, the third brought to
        // the top: the second drawn just over the scroll, under the
        // header.)
        page.set(1);
        h.run();
        h.get_by_label("field 1").click();
        h.run();
        page.set(2);
        h.run();
        assert!(h.query_by_label("page 1").is_some(), "the keyboard's page, out of sight");
        let top = h.get_by_label("page 2").rect().top();
        h.get_by_label("Header").focus();
        h.run();
        assert_eq!(page.get(), 2, "the header's button moves no page");
        assert_eq!(h.get_by_label("page 2").rect().top(), top, "nor the scroll");
        // Another layer's widget over the page in sight.
        h.get_by_label("Floating").focus();
        h.run();
        page.set(4);
        h.run();
        assert!(h.query_by_label("page 2").is_none(), "not taken for the page under it");
    }

    /// A page above the one whose bar is in view, growing (an inventory
    /// filling), moves nothing in view.
    #[test]
    fn a_page_above_growing_moves_nothing_in_view() {
        let heights = std::rc::Rc::new(std::cell::RefCell::new(vec![300.0; 6]));
        let page = std::rc::Rc::new(std::cell::Cell::new(3));
        let mut h = pages_harness(heights.clone(), page.clone());
        h.run();
        h.hover_at(pos2(400.0, 150.0));
        h.event(egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: vec2(0.0, 100.0),
            phase: egui::TouchPhase::Move,
            modifiers: egui::Modifiers::NONE,
        });
        h.run_steps(12);
        h.run();
        let before = h.get_by_label("page 3").rect().top();
        assert!(h.query_by_label("page 2").is_some(), "the page above in view");
        heights.borrow_mut()[2] += 200.0;
        h.run();
        assert_eq!(h.get_by_label("page 3").rect().top(), before, "the page below where it was");
    }

    /// A page whose height will not settle, flipping between two from pass
    /// to pass or sized by the room in sight under its top (as the
    /// Appearance page's model was beside its fields, which grew each time
    /// it was placed), has the pass drawn again at most twice in twenty
    /// frames, not in every frame for as long as it is in view (M4's U11
    /// review: egui's warning of a discard a thousand frames in a row),
    /// above the page held in one column and in columns; a page that grows
    /// after it is still taken as measured.
    #[test]
    fn a_page_whose_height_flips_is_not_drawn_again_every_frame() {
        for (columns, follows) in [(1, false), (2, false), (1, true), (2, true)] {
            let grown = std::rc::Rc::new(std::cell::Cell::new(0.0f32));
            let g = grown.clone();
            let page = std::rc::Rc::new(std::cell::Cell::new(2));
            let mut h = Harness::builder().with_size(vec2(700.0, 400.0)).build_ui(move |ui| {
                crate::apply(ui.ctx(), crate::Theme::Dark);
                let mut scroll = PageScroll::new("flips").columns(columns);
                for n in ["One", "Two", "Three", "Four", "Five", "Six"] {
                    scroll = scroll.page(Icon::Info, n);
                }
                let flip = ui.ctx().cumulative_pass_nr() % 2 == 0;
                let mut top = page.get();
                scroll.show(ui, &mut top, |ui, i| {
                    ui.label(format!("page {i}"));
                    // (Flipping each pass, or as tall as the room in sight
                    // under its top, as the blueprint pages' model beside their fields
                    // was sized before M4's U11.)
                    let tall = match (i, flip, follows) {
                        (1, _, true) => (ui.clip_rect().bottom() - ui.cursor().top()).max(300.0),
                        (1, true, false) => 300.0,
                        (1, false, false) => 340.0,
                        _ => 260.0,
                    } + if i == 1 { g.get() } else { 0.0 };
                    ui.allocate_exact_size(vec2(10.0, tall), Sense::hover());
                    ui.label(format!("end {i}"));
                });
                page.set(top);
            });
            let extra = |h: &mut Harness<'_>| {
                let (p, f) = (h.ctx.cumulative_pass_nr(), h.ctx.cumulative_frame_nr());
                for _ in 0..20 {
                    h.step();
                }
                (h.ctx.cumulative_pass_nr() - p) - (h.ctx.cumulative_frame_nr() - f)
            };
            // (Page 2 at the top, then the wheel up a little: the end of
            // page 1, above the page held, in view.)
            h.run_steps(4);
            h.hover_at(pos2(400.0, 150.0));
            h.event(egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: vec2(0.0, 100.0),
                phase: egui::TouchPhase::Move,
                modifiers: egui::Modifiers::NONE,
            });
            h.run_steps(12);
            assert!(h.query_by_label("page 2").is_some(), "{columns}: page 2 in view");
            assert!(h.query_by_label("end 1").is_some(), "{columns}: page 1's end in view");
            let first = extra(&mut h);
            let what = if follows { "following the room in sight" } else { "flipping" };
            assert!(
                first <= 2,
                "{columns} columns, {what}: drawn again {first} times in 20 frames"
            );
            // A real change after it settled: measured and drawn again once
            // or twice, then still.
            grown.set(500.0);
            let after = extra(&mut h);
            assert!((1..=3).contains(&after), "{columns} columns, {what}: grown: {after} in 20");
            let later = extra(&mut h);
            assert!(later <= 2, "{columns} columns, {what}: then {later} in 20 frames");
        }
    }

    /// A page brought to the top stays there as it is measured anew,
    /// taller or shorter (a list on it filled), the pages after it moving.
    #[test]
    fn a_page_brought_to_the_top_stays_as_it_changes() {
        let heights = std::rc::Rc::new(std::cell::RefCell::new(vec![200.0; 6]));
        let page = std::rc::Rc::new(std::cell::Cell::new(0));
        let mut h = pages_harness(heights.clone(), page.clone());
        h.run();
        let at_top = h.get_by_label("page 0").rect().top();
        h.get_by_label("Three").click();
        h.run();
        for change in [-80.0, 150.0] {
            heights.borrow_mut()[2] += change;
            h.run();
            assert_eq!(page.get(), 2, "still the page at the top");
            assert_eq!(h.get_by_label("page 2").rect().top(), at_top, "its bar at the top");
        }
    }

    /// Six pages in `columns` columns of a scroll `high` high beside their
    /// list, page `i` `heights[i]` high (as [`pages_harness`]'s), or where
    /// that is below 0 filling the room it is given.
    fn columns_harness(
        heights: std::rc::Rc<std::cell::RefCell<Vec<f32>>>,
        page: std::rc::Rc<std::cell::Cell<usize>>,
        columns: usize,
        high: f32,
    ) -> Harness<'static> {
        Harness::builder().with_size(vec2(700.0, high + 16.0)).build_ui(move |ui| {
            crate::apply(ui.ctx(), crate::Theme::Dark);
            let names = ["One", "Two", "Three", "Four", "Five", "Six"];
            let seen =
                PageScroll::seen(ui.ctx(), "cols").map(|s| s.pages_in_view).unwrap_or_default();
            let mut top = page.get();
            let room = ui.available_rect_before_wrap();
            let mut left = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(Rect::from_min_size(room.min, vec2(PAGE_LIST, room.height()))),
            );
            let mut list = PageList::new("cols-list").pages_in_view(seen);
            let mut scroll = PageScroll::new("cols").columns(columns);
            for n in names {
                list = list.page(Icon::Info, n);
                scroll = scroll.page(Icon::Info, n);
            }
            let chosen = list.show(&mut left, &mut top).changed;
            let mut body =
                ui.new_child(egui::UiBuilder::new().max_rect(Rect::from_min_max(
                    pos2(room.left() + PAGE_LIST, room.top()),
                    room.max,
                )));
            let heights = heights.borrow().clone();
            scroll.bring(chosen).show(&mut body, &mut top, |ui, i| {
                ui.add_sized(vec2(100.0, 20.0), egui::Label::new(format!("page {i}")));
                let mut text = String::new();
                let id = egui::Id::new(("cols-field", i));
                ui.data_mut(|d| text = d.get_temp::<String>(id).unwrap_or_default());
                let field = egui::TextEdit::singleline(&mut text).id(id.with("edit"));
                ui.add(field).widget_info(|| {
                    WidgetInfo::labeled(WidgetType::TextEdit, true, format!("field {i}"))
                });
                ui.data_mut(|d| d.insert_temp(id, text));
                let rest = match heights[i] < 0.0 {
                    true => ui.available_height(),
                    false => heights[i] - 20.0 - 22.0 - PAGE_BAR - 22.0,
                };
                ui.allocate_exact_size(vec2(10.0, rest.max(0.0)), Sense::hover());
            });
            page.set(top);
        })
    }

    /// In columns each page goes under the column that ends highest, the
    /// first pages heading them; the pages in view need not follow one
    /// another; those out of view are not drawn.
    #[test]
    fn pages_in_columns_stand_under_the_column_that_ends_highest() {
        let heights = std::rc::Rc::new(std::cell::RefCell::new(vec![
            100.0, 300.0, 100.0, 100.0, 100.0, 120.0,
        ]));
        let page = std::rc::Rc::new(std::cell::Cell::new(0));
        let mut h = columns_harness(heights, page.clone(), 2, 300.0);
        h.run();
        let at = |h: &Harness<'_>, n: &str| h.get_by_label(&format!("{n} page")).rect();
        let (p0, p1, p2, p3) = (at(&h, "One"), at(&h, "Two"), at(&h, "Three"), at(&h, "Four"));
        assert_eq!(p0.top(), p1.top(), "the first two head the columns");
        assert!(p1.left() > p0.left() + 200.0, "side by side: {p0:?} {p1:?}");
        assert_eq!(p2.left(), p0.left(), "the third under the first, its column ending higher");
        assert!((p2.top() - p0.bottom()).abs() <= 1.0, "under it: {p0:?} {p2:?}");
        assert!((p3.top() - p2.bottom()).abs() <= 1.0, "and the fourth under that");
        assert!(h.query_by_label("page 4").is_none(), "a page out of view is not drawn");
        let seen = PageScroll::seen(&h.ctx, "cols").unwrap();
        assert_eq!(seen.top, 0);
        assert_eq!(seen.pages_in_view, vec![0, 1, 2, 3]);
        // (The line between the columns, painted down the view.)
        let line = (h.output().shapes.iter()).any(|c| match &c.shape {
            egui::Shape::LineSegment { points, stroke } => {
                stroke.width == PAGE_COLUMN_LINE
                    && points[0].x == points[1].x
                    && points[0].x > p0.right()
                    && points[0].x < p1.left()
            }
            _ => false,
        });
        assert!(line, "a line between the columns");
    }

    /// In columns a page chosen comes as near the top as the scroll goes
    /// and is the page at the top while its bar is in view; the pages
    /// scrolled out of view are not drawn.
    #[test]
    fn in_columns_a_page_chosen_is_the_page_at_the_top() {
        let heights = std::rc::Rc::new(std::cell::RefCell::new(vec![
            100.0, 300.0, 100.0, 100.0, 100.0, 120.0,
        ]));
        let page = std::rc::Rc::new(std::cell::Cell::new(0));
        let mut h = columns_harness(heights, page.clone(), 2, 300.0);
        h.run();
        h.get_by_label("Five").click();
        h.run();
        assert_eq!(page.get(), 4, "the page chosen at the top");
        let toggled = |h: &Harness<'_>, n: &str| h.get_by_label(n).accesskit_node().toggled();
        assert_eq!(toggled(&h, "Five"), Some(egui::accesskit::Toggled::True));
        assert!(h.query_by_label("page 4").is_some(), "drawn");
        assert!(h.query_by_label("page 0").is_none(), "the first scrolled out of view");
        // Every page in view: the page chosen is the page at the top,
        // though nothing scrolls.
        let heights = std::rc::Rc::new(std::cell::RefCell::new(vec![100.0; 6]));
        let page = std::rc::Rc::new(std::cell::Cell::new(0));
        let mut h = columns_harness(heights, page.clone(), 3, 600.0);
        h.run();
        let first = h.get_by_label("page 0").rect().top();
        h.get_by_label("Four").click();
        h.run();
        assert_eq!(page.get(), 3, "the page chosen at the top");
        assert_eq!(toggled(&h, "Four"), Some(egui::accesskit::Toggled::True));
        assert_eq!(h.get_by_label("page 0").rect().top(), first, "nothing scrolled");
    }

    /// In columns Tab goes from page to page in the pages' order, from one
    /// column to the next and back.
    #[test]
    fn in_columns_tab_goes_page_after_page() {
        let heights = std::rc::Rc::new(std::cell::RefCell::new(vec![100.0; 6]));
        let page = std::rc::Rc::new(std::cell::Cell::new(0));
        let mut h = columns_harness(heights, page, 3, 300.0);
        h.run();
        h.get_by_label("field 0").click();
        h.run();
        for next in 1..5 {
            h.key_press(egui::Key::Tab);
            h.run();
            let name = format!("field {next}");
            let field = h.get_by_label(&name);
            assert!(field.accesskit_node().is_focused(), "Tab on to page {next}'s field");
        }
    }

    /// A column's last page reaches down to the scroll's foot: what fills
    /// the room it is given fills the column.
    #[test]
    fn a_column_s_last_page_reaches_down_to_the_foot() {
        let heights =
            std::rc::Rc::new(std::cell::RefCell::new(vec![50.0, 50.0, 50.0, 50.0, 50.0, -1.0]));
        let page = std::rc::Rc::new(std::cell::Cell::new(0));
        let mut h = columns_harness(heights, page, 2, 700.0);
        h.run();
        let (two, six) = (h.get_by_label("Two page").rect(), h.get_by_label("Six page").rect());
        assert_eq!(six.left(), two.left(), "the last page in the second column");
        assert!(six.top() > two.bottom(), "under the others there");
        assert!(six.height() > PAGE_FILL, "more than another page is given: {six:?}");
        assert!((six.bottom() - 708.0).abs() <= 1.0, "down to the foot: {six:?}");
        let seen = PageScroll::seen(&h.ctx, "cols").unwrap();
        assert_eq!(seen.pages_in_view, vec![0, 1, 2, 3, 4, 5], "every page in view");
    }

    /// In columns too the scroll keeps its place by a page: one above the
    /// page at the top that grows moves nothing in view.
    #[test]
    fn in_columns_a_page_above_growing_moves_nothing_in_view() {
        // (The second column longer: the third page under the first.)
        let heights = std::rc::Rc::new(std::cell::RefCell::new(vec![
            200.0, 260.0, 200.0, 200.0, 200.0, 200.0,
        ]));
        let page = std::rc::Rc::new(std::cell::Cell::new(0));
        let mut h = columns_harness(heights.clone(), page.clone(), 2, 300.0);
        h.run();
        h.get_by_label("Three").click();
        h.run();
        assert_eq!(page.get(), 2);
        // (The wheel up a little: the first page's foot in view above it.)
        h.hover_at(pos2(400.0, 150.0));
        h.event(egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: vec2(0.0, 50.0),
            phase: egui::TouchPhase::Move,
            modifiers: egui::Modifiers::NONE,
        });
        h.run_steps(12);
        h.run();
        assert_eq!(page.get(), 2, "its bar still in view");
        let before = h.get_by_label("page 2").rect().top();
        let one = h.get_by_label("One page").rect();
        assert_eq!(h.get_by_label("Three page").rect().left(), one.left(), "under the first");
        assert!(one.height() > 0.0, "the page above in view");
        heights.borrow_mut()[0] += 60.0;
        h.run();
        assert_eq!(page.get(), 2, "still the page at the top");
        assert_eq!(h.get_by_label("page 2").rect().top(), before, "where it was");
    }

    type Rcv<T> = std::rc::Rc<std::cell::RefCell<T>>;
    /// Up to eight pages in `columns` columns of a scroll `high` high,
    /// page `i` `heights[i]` high (filling its room where below 0), a
    /// field "field i" at its top and "last i" at its foot (the review's
    /// probe harness).
    fn eight_harness(
        heights: Rcv<Vec<f32>>,
        page: std::rc::Rc<std::cell::Cell<usize>>,
        columns: std::rc::Rc<std::cell::Cell<usize>>,
        high: f32,
    ) -> Harness<'static> {
        Harness::builder().with_size(vec2(700.0, high + 16.0)).build_ui(move |ui| {
            crate::apply(ui.ctx(), crate::Theme::Dark);
            let names = ["One", "Two", "Three", "Four", "Five", "Six", "Seven", "Eight"];
            let n = heights.borrow().len();
            let seen =
                PageScroll::seen(ui.ctx(), "cols").map(|s| s.pages_in_view).unwrap_or_default();
            let mut top = page.get();
            let room = ui.available_rect_before_wrap();
            let mut left = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(Rect::from_min_size(room.min, vec2(PAGE_LIST, room.height()))),
            );
            let mut list = PageList::new("cols-list").pages_in_view(seen);
            let mut scroll = PageScroll::new("cols").columns(columns.get());
            for nm in &names[..n] {
                list = list.page(Icon::Info, *nm);
                scroll = scroll.page(Icon::Info, *nm);
            }
            let chosen = list.show(&mut left, &mut top).changed;
            let mut body =
                ui.new_child(egui::UiBuilder::new().max_rect(Rect::from_min_max(
                    pos2(room.left() + PAGE_LIST, room.top()),
                    room.max,
                )));
            let heights = heights.borrow().clone();
            scroll.bring(chosen).show(&mut body, &mut top, |ui, i| {
                ui.add_sized(vec2(100.0, 20.0), egui::Label::new(format!("page {i}")));
                let mut text = String::new();
                let id = egui::Id::new(("cols-field", i));
                ui.data_mut(|d| text = d.get_temp::<String>(id).unwrap_or_default());
                let field = egui::TextEdit::singleline(&mut text).id(id.with("edit"));
                ui.add(field).widget_info(|| {
                    WidgetInfo::labeled(WidgetType::TextEdit, true, format!("field {i}"))
                });
                ui.data_mut(|d| d.insert_temp(id, text));
                // (A page that fills its room: no last field, the rest of
                // its room taken.)
                if heights[i] < 0.0 {
                    let rest = ui.available_height();
                    ui.allocate_exact_size(vec2(10.0, rest.max(0.0)), Sense::hover());
                    return;
                }
                let rest = heights[i] - 20.0 - 22.0 - PAGE_BAR - 22.0 - 30.0;
                ui.allocate_exact_size(vec2(10.0, rest.max(0.0)), Sense::hover());
                // A last field at the page's foot.
                let mut t2 = String::new();
                let id2 = egui::Id::new(("cols-last", i));
                ui.data_mut(|d| t2 = d.get_temp::<String>(id2).unwrap_or_default());
                ui.add(egui::TextEdit::singleline(&mut t2).id(id2.with("edit"))).widget_info(
                    || WidgetInfo::labeled(WidgetType::TextEdit, true, format!("last {i}")),
                );
                ui.data_mut(|d| d.insert_temp(id2, t2));
            });
            page.set(top);
        })
    }

    /// The field with the keyboard.
    fn focused_name(h: &Harness<'_>) -> String {
        for i in 0..8 {
            for w in ["field", "last"] {
                let n = format!("{w} {i}");
                if let Some(node) = h.query_by_label(&n)
                    && node.accesskit_node().is_focused()
                {
                    return n;
                }
            }
        }
        "none".into()
    }

    /// The wheel turned by `dy` over the scroll, settled.
    fn wheel(h: &mut Harness<'_>, dy: f32) {
        h.hover_at(pos2(400.0, 150.0));
        h.event(egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: vec2(0.0, dy),
            phase: egui::TouchPhase::Move,
            modifiers: egui::Modifiers::NONE,
        });
        h.run_steps(12);
        h.run();
    }

    fn rc<T>(v: T) -> std::rc::Rc<std::cell::RefCell<T>> {
        std::rc::Rc::new(std::cell::RefCell::new(v))
    }

    fn cell<T: Copy>(v: T) -> std::rc::Rc<std::cell::Cell<T>> {
        std::rc::Rc::new(std::cell::Cell::new(v))
    }

    /// In columns Tab goes on to the next page and Shift + Tab back to the
    /// one before, out of view in the other column (the pages in view
    /// not one after another: the review's A1).
    #[test]
    fn in_columns_tab_reaches_the_page_beside_out_of_view() {
        let heights = rc(vec![100.0, 500.0, 100.0, 100.0, 100.0, 100.0, 100.0, 100.0]);
        let mut h = eight_harness(heights.clone(), cell(0), cell(2), 300.0);
        h.run();
        wheel(&mut h, -250.0);
        let seen = PageScroll::seen(&h.ctx, "cols").unwrap();
        assert!(!seen.pages_in_view.contains(&2), "page 2 out of view: {seen:?}");
        h.get_by_label("last 1").click();
        h.run();
        h.key_press(egui::Key::Tab);
        h.run();
        assert_eq!(focused_name(&h), "field 2", "Tab from page 1's last field to page 2's first");
        let mut h = eight_harness(heights, cell(0), cell(2), 300.0);
        h.run();
        wheel(&mut h, -210.0);
        h.get_by_label("field 3").click();
        h.run();
        h.key_press_modifiers(egui::Modifiers::SHIFT, egui::Key::Tab);
        h.run();
        h.run();
        assert_eq!(focused_name(&h), "last 2", "Shift + Tab from page 3 to page 2's last field");
    }

    /// In columns a page keeps its column while the column count stays:
    /// the page held, Four, neither moves sideways nor down when a page in
    /// the other column, One, grows (the review's A2).
    #[test]
    fn in_columns_a_page_keeps_its_column_as_others_grow() {
        let heights = rc(vec![100.0, 150.0, 100.0, 100.0, 100.0, 100.0, 100.0, 100.0]);
        let page = cell(0);
        // (Every page in view: One, growing, drawn and measured.)
        let mut h = eight_harness(heights.clone(), page.clone(), cell(2), 900.0);
        h.run();
        h.get_by_label("Four").click();
        h.run();
        let before = h.get_by_label("Four page").rect();
        // (One, in the other column: flowed anew, Four would go under it.)
        heights.borrow_mut()[0] += 100.0;
        h.run();
        let after = h.get_by_label("Four page").rect();
        assert_eq!(page.get(), 3, "still the page at the top");
        assert_eq!(after.left(), before.left(), "in its column");
        assert_eq!(after.top(), before.top(), "where it was in view");
    }

    /// The column count changed: the pages flowed anew in the frame it
    /// changed in (no frame of the old flow at the new width), the page at
    /// the top and the keyboard kept (the review's B8).
    #[test]
    fn a_column_count_changed_flows_the_pages_in_its_frame() {
        let heights = rc(vec![100.0, 200.0, 150.0, 300.0, 100.0, 250.0, 120.0, 100.0]);
        let (page, cols) = (cell(0), cell(1));
        let mut h = eight_harness(heights, page.clone(), cols.clone(), 300.0);
        h.run();
        h.get_by_label("Six").click();
        h.run();
        h.get_by_label("field 5").click();
        h.run();
        for k in [2, 3, 1, 3] {
            cols.set(k);
            h.step();
            // (In the one frame: each page in view against the one above it
            // in its column, none over another.)
            let rects: Vec<Rect> = (0..8)
                .filter_map(|i| {
                    let name = ["One", "Two", "Three", "Four", "Five", "Six", "Seven", "Eight"][i];
                    h.query_by_label(&format!("{name}{PAGE_GROUP}")).map(|n| n.rect())
                })
                .filter(|r| r.height() > 0.0)
                .collect();
            for (a, ra) in rects.iter().enumerate() {
                for rb in &rects[a + 1..] {
                    assert!(!ra.intersect(*rb).is_positive(), "{k} columns: {ra:?} over {rb:?}");
                }
            }
            h.run();
            assert_eq!(page.get(), 5, "{k} columns: the page at the top kept");
            assert_eq!(focused_name(&h), "field 5", "{k} columns: the keyboard kept");
        }
    }

    /// In columns a page above the one in view that grows is shown at its
    /// new height in the frame it grows in: the pass drawn again, the page
    /// under it never over it (the review's B2).
    #[test]
    fn in_columns_a_page_grown_is_drawn_again_at_once() {
        let heights = rc(vec![100.0, 300.0, 100.0, 100.0, 100.0, 100.0, 100.0, 100.0]);
        let mut h = eight_harness(heights.clone(), cell(0), cell(2), 600.0);
        h.run();
        let one = h.get_by_label("One page").rect();
        let three = h.get_by_label("Three page").rect();
        assert_eq!(three.left(), one.left(), "Three under One");
        heights.borrow_mut()[0] += 80.0;
        h.step();
        let (one, three) = (h.get_by_label("One page").rect(), h.get_by_label("Three page").rect());
        assert_eq!(three.top(), one.bottom(), "Three against One, in the frame One grew");
    }

    /// A column's spare room goes to its pages that fill their room: a
    /// list not its column's last is given more than [`PAGE_FILL`] where
    /// the column has room (the review's A3); and when the view grows
    /// shorter, a page that fills is given less again, not kept at what
    /// it was given (decision 6's, the review's B9).
    #[test]
    fn a_column_s_spare_room_goes_to_its_pages_that_fill() {
        let heights = rc(vec![100.0, 400.0, -1.0, 100.0, 100.0, 100.0]);
        let mut h = eight_harness(heights, cell(0), cell(2), 700.0);
        h.run();
        let three = h.get_by_label("Three page").rect();
        let five = h.get_by_label("Five page").rect();
        assert_eq!(five.left(), three.left(), "Five under the list, Three");
        assert!(three.height() > PAGE_FILL + 50.0, "the column's spare given to it: {three:?}");
        assert!((five.bottom() - 708.0).abs() <= 1.0, "the column down to the foot: {five:?}");
        // A shorter view: the list given less.
        h.set_size(vec2(700.0, 416.0));
        h.run();
        let three = h.get_by_label("Three page").rect();
        assert!(three.height() < 300.0, "the list given less in a shorter view: {three:?}");
    }

    /// A rail and a list whose pages go on below them show the cue at
    /// their foot: a band of their own ground (the rail's `bar`, the
    /// list's `panel`, opaque, so the pages pass under it) with a chevron
    /// in `text-3`; nothing at their top.
    #[test]
    fn overflowing_rail_and_list_settle_cleanly() {
        for theme in crate::Theme::ALL {
            let t = theme.tokens();
            let (rail_room, list_room, h) = overflowing(theme);
            let bands: Vec<(Rect, Color32)> = (h.output().shapes.iter())
                .filter_map(|c| match &c.shape {
                    egui::Shape::Rect(r) if (r.rect.height() - 14.0).abs() < 0.5 => {
                        Some((r.rect, r.fill))
                    }
                    _ => None,
                })
                .collect();
            for (room, surface) in [(rail_room, t.bar), (list_room, t.panel)] {
                // (A list is as wide as its own width, past a narrower room.)
                let here: Vec<_> =
                    bands.iter().filter(|(r, _)| room.x_range().contains(r.left())).collect();
                assert_eq!(here.len(), 1, "{theme:?}: one band, at the foot: {bands:?}");
                let (band, fill) = here[0];
                assert_eq!(*fill, surface, "{theme:?}: the band on its own ground");
                assert!((band.bottom() - room.bottom()).abs() < 0.5, "{theme:?}: {band:?}");
            }
        }
    }

    /// A rail and a list of four pages in rooms too short for them.
    fn overflowing(theme: crate::Theme) -> (Rect, Rect, Harness<'static>) {
        let mut page = 0usize;
        let rail_room = egui::Rect::from_min_size(pos2(10.0, 10.0), vec2(44.0, 52.0));
        let list_room = egui::Rect::from_min_size(pos2(70.0, 10.0), vec2(120.0, 60.0));
        let mut h = Harness::builder().with_size(vec2(220.0, 100.0)).build_ui(move |ui| {
            crate::apply(ui.ctx(), theme);
            let mut r_ui = ui.new_child(egui::UiBuilder::new().max_rect(rail_room));
            r_ui.set_clip_rect(rail_room);
            let _ = PageRail::new("cue-rail")
                .page(Icon::Info, "One")
                .page(Icon::Inventory, "Two")
                .page(Icon::Lock, "Three")
                .page(Icon::Trap, "Four")
                .show(&mut r_ui, &mut page);

            let mut l_ui = ui.new_child(egui::UiBuilder::new().max_rect(list_room));
            l_ui.set_clip_rect(list_room);
            let _ = PageList::new("cue-list")
                .page(Icon::Info, "One")
                .page(Icon::Inventory, "Two")
                .page(Icon::Lock, "Three")
                .page(Icon::Trap, "Four")
                .show(&mut l_ui, &mut page);
        });
        h.run();
        h.run();
        (rail_room, list_room, h)
    }
}
