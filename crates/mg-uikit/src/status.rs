//! The Status Bar's parts (the mock's `.statusbar` and `.sb-*`), each
//! drawn once from the tokens, for the shell to compose into its 24 px
//! row: [`bar`] (the row itself, named "Status Bar"), [`StatusSwitch`] (a
//! Bottom Dock panel's switch, with a count badge), [`separator`],
//! [`Hints`] (the active tool's keys as key caps with a word, dropping
//! whole from the end where there is no room), [`StatusItem`] (the
//! right-hand items: counts, the module's name with its modified dot, the
//! game folder) and [`StatusJob`] (a running job with its bar).
//!
//! ```
//! # egui::__run_test_ui(|ui| {
//! use mg_uikit::status::{self, Hint, Hints, StatusItem, StatusSwitch};
//! use mg_uikit::{Badge, BadgeKind, Icon};
//! let mut log = false;
//! let hints = [Hint::keys(["Drag"], "Move"), Hint::keys(["Shift", "Right Drag"], "Turn")];
//! status::bar(
//!     ui,
//!     |ui| {
//!         if ui.add(StatusSwitch::new(Icon::Log, "Log", log)).clicked() {
//!             log = !log;
//!         }
//!         let problems = Badge::count(3).kind(BadgeKind::Error);
//!         ui.add(StatusSwitch::new(Icon::Problems, "Problems", false).badge(problems));
//!         status::separator(ui);
//!     },
//!     |ui, room| Hints::new(&hints).show(ui, room),
//!     |ui| {
//!         ui.add(StatusItem::new("Chapter1 (modified)").modified(true));
//!         status::separator(ui);
//!         ui.add(StatusItem::new(".../Neverwinter Nights").weak());
//!     },
//! );
//! # });
//! ```

use std::sync::Arc;

use egui::{
    Color32, FontId, Painter, Pos2, Rect, Response, Sense, Stroke, Ui, Vec2, WidgetInfo,
    WidgetType, pos2, vec2,
};

use crate::icons::{ButtonState, Icon};
use crate::marks::{self, Badge};
use crate::metrics;
use crate::theme::TokensExt as _;
use crate::tokens::Tokens;

/// The Status Bar's accessible name.
pub const NAME: &str = "Status Bar";
/// The gap between the Status Bar's parts (`.statusbar`: `gap: 8px`).
pub const GAP: f32 = 8.0;
/// The Status Bar's padding at each end (`padding: 0 6px`).
pub const PADDING: f32 = 6.0;
/// A switch's height (`.sb-switch`: 20 px).
pub const SWITCH_HEIGHT: f32 = 20.0;
/// A switch's padding at each side (6 px; 4 with its icon alone, as the
/// narrow board's).
pub const SWITCH_PADDING: f32 = 6.0;
/// The gap between a switch's icon, words and badge (`gap: 5px`).
pub const SWITCH_GAP: f32 = 5.0;
/// A switch's icon (`.sb-switch .ic`: 14 px).
pub const SWITCH_ICON: f32 = 14.0;
/// The line under a switch that is on (`inset 0 -2px 0 var(--accent)`).
pub const SWITCH_ON_LINE: f32 = 2.0;
/// A separator's height (`.sb-sep`: 1 by 14).
pub const SEPARATOR_HEIGHT: f32 = 14.0;
/// The gap between hints (`.sb-hints`: `column-gap: 8px`).
pub const HINT_GAP: f32 = 8.0;
/// The hints' padding at each side (`padding: 0 4px`).
pub const HINTS_PADDING: f32 = 4.0;
/// The gap between a hint's keys, joiners and word (`.hint`: `gap: 3px`).
pub const HINT_PART_GAP: f32 = 3.0;
/// The word's own margin after the gap (`.hint .w`: `margin-left: 2px`).
pub const HINT_WORD_MARGIN: f32 = 2.0;
/// A hint's height (`.hint`: 20 px).
pub const HINT_HEIGHT: f32 = 20.0;
/// A job's bar (`.sb-job .bar`: 60 by 4, radius 2).
pub const JOB_BAR: Vec2 = Vec2::new(60.0, 4.0);
/// The gap between a job's words and its bar (`gap: 6px`).
pub const JOB_GAP: f32 = 6.0;
/// The gap between a right-hand item's dot and words (`.sb-item`: 5 px).
pub const ITEM_GAP: f32 = 5.0;

fn small_font() -> FontId {
    FontId::proportional(metrics::SMALL_SIZE)
}

fn body_font() -> FontId {
    FontId::proportional(metrics::BODY_SIZE)
}

/// The Status Bar's frame: `bar`, 6 px at each end (`.statusbar`); for a
/// bottom panel [`metrics::STATUS_BAR`] high.
pub fn frame(t: &Tokens) -> egui::Frame {
    egui::Frame::new().fill(t.bar).inner_margin(egui::Margin::symmetric(PADDING as i8, 0))
}

/// The Status Bar's row (`.statusbar`): `left` laid out from the left
/// (the panels' switches), `right` against the right end in its own order
/// (counts, a job, the module, the game folder), and `middle` given the
/// width between them, [`GAP`] from each (for [`Hints::show`], which
/// takes it whole); its parts [`GAP`] apart, centered in the row's height.
/// A group named "Status Bar", its switches and items found within it by
/// their own names. The caller gives it its 24 px and its [`frame`].
///
/// The right group's width is the last frame's (it is laid out after the
/// middle); when it changes, the frame is drawn again.
pub fn bar<L, M, R>(
    ui: &mut Ui,
    left: impl FnOnce(&mut Ui) -> L,
    middle: impl FnOnce(&mut Ui, f32) -> M,
    right: impl FnOnce(&mut Ui) -> R,
) -> (L, M, R) {
    crate::shell::three_parts(ui, NAME, (GAP, GAP), left, right, |ui, room| {
        middle(ui, room.width())
    })
}

/// [`bar`] with the right-hand group's width given (`right_width`: its
/// items' widths, [`StatusItem::width`], [`SEPARATOR_WIDTH`] for each
/// separator, and [`GAP`] between them), so that it is laid out by it at
/// once: a module's name changing (its modified dot) asks for no second
/// pass, which would run without the frame's input (a click on that frame
/// would be lost).
pub fn bar_sized<L, M, R>(
    ui: &mut Ui,
    right_width: f32,
    left: impl FnOnce(&mut Ui) -> L,
    middle: impl FnOnce(&mut Ui, f32) -> M,
    right: impl FnOnce(&mut Ui) -> R,
) -> (L, M, R) {
    let gaps = (GAP, GAP);
    crate::shell::three_parts_sized(ui, NAME, gaps, Some(right_width), left, right, |ui, room| {
        middle(ui, room.width())
    })
}

/// A separator's width ([`separator`]).
pub const SEPARATOR_WIDTH: f32 = 1.0;

/// A vertical line between the Status Bar's groups (`.sb-sep`: 1 by 14,
/// `line-soft`).
pub fn separator(ui: &mut Ui) -> Response {
    let size = vec2(SEPARATOR_WIDTH, SEPARATOR_HEIGHT);
    let (rect, response) = ui.allocate_exact_size(size, Sense::hover());
    if ui.is_rect_visible(rect) {
        ui.painter().rect_filled(rect, 0.0, ui.tokens().line_soft);
    }
    response
}

/// A Bottom Dock panel's switch in the Status Bar (`.sb-switch`): its
/// icon (14 px), its words and a count badge where it has one; 20 px
/// high. At rest `text-2` with no fill; hovered `raised` and `text`; on
/// (its panel shown), `raised`, `text` and a 2 px `accent` line along its
/// bottom; reached with the keyboard, an `accent` ring. With its icon
/// alone ([`StatusSwitch::icon_only`], the narrow window's), its words
/// are its tooltip.
///
/// A button that says whether it is on (accesskit's toggled), named by
/// its words alone: `get_by_label("Log")` finds it with or without a
/// count, and its count is its value.
#[derive(Clone, Debug)]
#[must_use = "add it with ui.add"]
pub struct StatusSwitch {
    icon: Icon,
    text: String,
    on: bool,
    badge: Option<Badge>,
    icon_only: bool,
    tooltip: Option<String>,
    drawn_as: Option<ButtonState>,
}

impl StatusSwitch {
    /// The switch of the panel `text`, shown with `icon`, on while `on`.
    pub fn new(icon: Icon, text: impl Into<String>, on: bool) -> Self {
        StatusSwitch {
            icon,
            text: text.into(),
            on,
            badge: None,
            icon_only: false,
            tooltip: None,
            drawn_as: None,
        }
    }

    /// With a count (or any badge) after its words.
    pub fn badge(mut self, badge: Badge) -> Self {
        self.badge = Some(badge);
        self
    }

    /// With a badge only where `badge` is one (a count of none shows
    /// none).
    pub fn badge_if(mut self, badge: Option<Badge>) -> Self {
        self.badge = badge;
        self
    }

    /// Its icon alone where `only` (and its badge), its words then its
    /// tooltip.
    pub fn icon_only(mut self, only: bool) -> Self {
        self.icon_only = only;
        self
    }

    /// Shown on hover (and, with its icon alone, in place of its words).
    pub fn tooltip(mut self, tooltip: impl Into<String>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }

    /// Drawn as in `state` whatever the pointer does (a sheet of the
    /// states).
    pub fn drawn_as(mut self, state: ButtonState) -> Self {
        self.drawn_as = Some(state);
        self
    }

    /// The width it takes in `ui`.
    pub fn width(&self, ui: &Ui) -> f32 {
        let pad = if self.icon_only { 4.0 } else { SWITCH_PADDING };
        let mut w = 2.0 * pad + SWITCH_ICON;
        if !self.icon_only {
            let galley =
                ui.painter().layout_no_wrap(self.text.clone(), body_font(), Color32::WHITE);
            w += SWITCH_GAP + galley.size().x;
        }
        if let Some(badge) = &self.badge {
            w += SWITCH_GAP + badge.width(ui);
        }
        w
    }
}

impl egui::Widget for StatusSwitch {
    fn ui(self, ui: &mut Ui) -> Response {
        let size = vec2(self.width(ui), SWITCH_HEIGHT);
        let (rect, response) = ui.allocate_exact_size(size, Sense::click());
        let enabled = ui.is_enabled();
        let text = self.text.clone();
        let on = self.on;
        response.widget_info(|| WidgetInfo::selected(WidgetType::Button, enabled, on, &text));
        if let Some(badge) = &self.badge {
            let count = badge.text().to_owned();
            response.ctx.accesskit_node_builder(response.id, |node| node.set_value(count));
        }
        if ui.is_rect_visible(rect) {
            let t = ui.tokens();
            let state = self.drawn_as.unwrap_or_else(|| ButtonState::of(&response));
            let p = ui.painter();
            let lit = state != ButtonState::Rest || self.on;
            let (fill, color) =
                if lit { (t.raised, t.text) } else { (Color32::TRANSPARENT, t.text_2) };
            let color = if enabled { color } else { t.text_off };
            p.rect_filled(rect, metrics::RADIUS, fill);
            if self.on {
                let line = Rect::from_min_max(
                    pos2(rect.left(), rect.bottom() - SWITCH_ON_LINE),
                    rect.right_bottom(),
                );
                p.rect_filled(line, egui::CornerRadius { sw: 3, se: 3, nw: 0, ne: 0 }, t.accent);
            }
            if state == ButtonState::Pressed && response.has_focus() {
                p.rect_stroke(
                    rect,
                    metrics::RADIUS,
                    Stroke::new(1.0, t.accent),
                    egui::StrokeKind::Inside,
                );
            }
            let pad = if self.icon_only { 4.0 } else { SWITCH_PADDING };
            let mut x = rect.left() + pad;
            let icon = Rect::from_min_size(
                pos2(x, rect.center().y - SWITCH_ICON / 2.0),
                Vec2::splat(SWITCH_ICON),
            );
            self.icon.paint(p, icon, color);
            x += SWITCH_ICON;
            if !self.icon_only {
                let galley = p.layout_no_wrap(self.text.clone(), body_font(), color);
                x += SWITCH_GAP;
                let pos = pos2(x, (rect.center().y - galley.size().y / 2.0).round());
                x += galley.size().x;
                p.galley(pos, galley, color);
            }
            if let Some(badge) = &self.badge {
                x += SWITCH_GAP;
                let r = Rect::from_min_size(
                    pos2(x, rect.center().y - marks::MARK_HEIGHT / 2.0),
                    vec2(badge.width(ui), marks::MARK_HEIGHT),
                );
                badge.paint(p, &t, r);
            }
        }
        let tip = self.tooltip.clone().or_else(|| self.icon_only.then(|| self.text.clone()));
        match tip {
            Some(tip) => response.on_hover_text(tip),
            None => response,
        }
    }
}

/// How a hint's own glyph is painted: into the rectangle given, with the
/// tokens.
pub type GlyphPainter = Arc<dyn Fn(&Painter, Rect, &Tokens) + Send + Sync>;

/// A part of a [`Hint`] before its word.
#[derive(Clone)]
pub enum HintPart {
    /// A key cap (`Shift`, `Right Drag`).
    Key(String),
    /// A joiner between key caps, in 11 px `text-3` (`.hint .plus`: `+`,
    /// or `/` between keys that do the same).
    Join(&'static str),
    /// An icon from the set, 16 px in `text-2`, unspoken (a screen reader
    /// hears the keys and the word: the icon's file name is no word to
    /// say). [`HintPart::NamedIcon`] for one to be said.
    Icon(Icon),
    /// An icon from the set as [`HintPart::Icon`], named `name` for a
    /// screen reader (as a [`HintPart::Glyph`] is).
    NamedIcon {
        /// The icon.
        icon: Icon,
        /// What a screen reader says for it.
        name: String,
    },
    /// A glyph of the caller's own (Main.png's gizmo ring before Turn):
    /// its width in a 16 px high box, and how it is painted there (with
    /// the tokens), named `name` for a screen reader.
    Glyph {
        /// Its width.
        width: f32,
        /// What a screen reader says for it.
        name: String,
        /// Paints it into the rectangle given.
        paint: GlyphPainter,
    },
}

impl std::fmt::Debug for HintPart {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HintPart::Key(k) => write!(f, "Key({k:?})"),
            HintPart::Join(j) => write!(f, "Join({j:?})"),
            HintPart::Icon(i) => write!(f, "Icon({})", i.name()),
            HintPart::NamedIcon { icon, name } => write!(f, "NamedIcon({}, {name:?})", icon.name()),
            HintPart::Glyph { name, width, .. } => write!(f, "Glyph({name:?}, {width})"),
        }
    }
}

impl HintPart {
    /// What a screen reader says for it (nothing for an unnamed icon).
    fn spoken(&self) -> String {
        match self {
            HintPart::Key(k) => k.clone(),
            HintPart::Join(j) => (*j).to_string(),
            HintPart::Icon(_) => String::new(),
            HintPart::NamedIcon { name, .. } | HintPart::Glyph { name, .. } => name.clone(),
        }
    }
}

/// One of the Status Bar's hints (`.hint`): what to press, as key caps
/// with joiners between them, and the word for what it does (`Shift` +
/// `Right Drag` Turn), in 11 px `text-2`.
#[derive(Clone, Debug)]
pub struct Hint {
    parts: Vec<HintPart>,
    word: String,
}

impl Hint {
    /// A hint of `word` with no keys yet (add them with [`Hint::key`],
    /// [`Hint::plus`], [`Hint::or`], [`Hint::part`]).
    pub fn new(word: impl Into<String>) -> Self {
        Hint { parts: Vec::new(), word: word.into() }
    }

    /// `keys` pressed together (joined by `+`), then `word`: `Shift` +
    /// `Right Drag` Turn.
    pub fn keys<S: Into<String>>(
        keys: impl IntoIterator<Item = S>,
        word: impl Into<String>,
    ) -> Self {
        let mut hint = Hint::new(word);
        for key in keys {
            hint = hint.plus(key);
        }
        hint
    }

    /// Any one of `keys` (joined by `/`), then `word`: `Q` / `E` Turn.
    pub fn either<S: Into<String>>(
        keys: impl IntoIterator<Item = S>,
        word: impl Into<String>,
    ) -> Self {
        let mut hint = Hint::new(word);
        for key in keys {
            hint = hint.or(key);
        }
        hint
    }

    /// A key cap, joined to what is before it by nothing.
    pub fn key(mut self, key: impl Into<String>) -> Self {
        self.parts.push(HintPart::Key(key.into()));
        self
    }

    /// A key cap, after a `+` where keys are before it.
    pub fn plus(self, key: impl Into<String>) -> Self {
        self.joined("+", key)
    }

    /// A key cap, after a `/` where keys are before it.
    pub fn or(self, key: impl Into<String>) -> Self {
        self.joined("/", key)
    }

    fn joined(mut self, join: &'static str, key: impl Into<String>) -> Self {
        if !self.parts.is_empty() {
            self.parts.push(HintPart::Join(join));
        }
        self.key(key)
    }

    /// Any part ([`HintPart`]): an icon, a glyph of the caller's own.
    pub fn part(mut self, part: HintPart) -> Self {
        self.parts.push(part);
        self
    }

    /// Its word.
    pub fn word(&self) -> &str {
        &self.word
    }

    /// What a screen reader says for it, and its accessible name: its
    /// parts and its word, in order (`Shift + Right Drag Turn`).
    pub fn spoken(&self) -> String {
        let mut words: Vec<String> =
            self.parts.iter().map(HintPart::spoken).filter(|w| !w.is_empty()).collect();
        words.push(self.word.clone());
        words.join(" ")
    }

    /// Each part's width, then the word's, in `ui`'s fonts.
    fn widths(&self, ui: &Ui) -> (Vec<f32>, f32) {
        let painter = ui.painter();
        let parts = self
            .parts
            .iter()
            .map(|part| match part {
                HintPart::Key(k) => marks::key_cap_width(ui, k),
                HintPart::Join(j) => {
                    painter.layout_no_wrap((*j).to_string(), small_font(), Color32::WHITE).size().x
                }
                HintPart::Icon(_) | HintPart::NamedIcon { .. } => metrics::ICON,
                HintPart::Glyph { width, .. } => *width,
            })
            .collect();
        let word = painter.layout_no_wrap(self.word.clone(), small_font(), Color32::WHITE).size().x;
        (parts, word)
    }

    /// The width it takes in `ui`.
    pub fn width(&self, ui: &Ui) -> f32 {
        let (parts, word) = self.widths(ui);
        let gaps = HINT_PART_GAP * parts.len() as f32;
        parts.iter().sum::<f32>() + gaps + HINT_WORD_MARGIN + word
    }

    /// Paints it from `left`, centered on `y`.
    fn paint(&self, ui: &Ui, t: &Tokens, left: f32, y: f32) {
        let (widths, _) = self.widths(ui);
        let p = ui.painter();
        let mut x = left;
        for (part, w) in self.parts.iter().zip(widths) {
            let rect = Rect::from_min_size(pos2(x, y - marks::MARK_HEIGHT / 2.0), vec2(w, 16.0));
            match part {
                HintPart::Key(k) => marks::paint_key_cap(p, t, rect, k),
                HintPart::Join(j) => {
                    let galley = p.layout_no_wrap((*j).to_string(), small_font(), t.text_3);
                    let pos = pos2(x, (y - galley.size().y / 2.0).round());
                    p.galley(pos, galley, t.text_3);
                }
                HintPart::Icon(icon) | HintPart::NamedIcon { icon, .. } => {
                    icon.paint(p, rect, t.text_2)
                }
                HintPart::Glyph { paint, .. } => paint(p, rect, t),
            }
            x += w + HINT_PART_GAP;
        }
        let galley = p.layout_no_wrap(self.word.clone(), small_font(), t.text_2);
        let pos = pos2(x + HINT_WORD_MARGIN, (y - galley.size().y / 2.0).round());
        p.galley(pos, galley, t.text_2);
    }
}

/// How many of the hints of `widths` fit in `room`, `gap` apart, in
/// order: they drop whole from the end, never one cut (the mock's
/// `.sb-hints`, which wraps a hint that does not fit onto a hidden line).
pub fn hints_that_fit(widths: &[f32], gap: f32, room: f32) -> usize {
    let mut used = 0.0;
    for (i, w) in widths.iter().enumerate() {
        let next = used + if i > 0 { gap } else { 0.0 } + w;
        if next > room + 0.01 {
            return i;
        }
        used = next;
    }
    widths.len()
}

/// The active tool's hints in the Status Bar (`.sb-hints`), ordered by
/// importance: as many as fit in the room given, whole, from the first;
/// the rest dropped. A group named "Hints" whose each hint is a label
/// named by its parts and word ([`Hint::spoken`]).
#[derive(Clone, Copy, Debug)]
#[must_use = "show it with .show(ui, room)"]
pub struct Hints<'a> {
    hints: &'a [Hint],
}

impl<'a> Hints<'a> {
    /// The hints `hints`, the most important first.
    pub fn new(hints: &'a [Hint]) -> Self {
        Hints { hints }
    }

    /// How many of them fit in `room` (their padding taken from it).
    pub fn fitting(&self, ui: &Ui, room: f32) -> usize {
        let widths: Vec<f32> = self.hints.iter().map(|h| h.width(ui)).collect();
        hints_that_fit(&widths, HINT_GAP, room - 2.0 * HINTS_PADDING)
    }

    /// Draws those that fit in `room` (a width: what the right-hand items
    /// leave), [`HINT_HEIGHT`] high; returns how many were drawn. Takes
    /// the room whole (the hints are the Status Bar's part that gives).
    pub fn show(self, ui: &mut Ui, room: f32) -> usize {
        let shown = self.fitting(ui, room);
        let room = room.max(0.0);
        let (rect, _) = ui.allocate_exact_size(vec2(room, HINT_HEIGHT), Sense::hover());
        let builder = egui::UiBuilder::new()
            .max_rect(rect)
            .layout(egui::Layout::left_to_right(egui::Align::Center));
        ui.scope_builder(builder, |ui| {
            crate::icons::name_group(ui, "Hints");
            ui.spacing_mut().item_spacing.x = HINT_GAP;
            ui.add_space(HINTS_PADDING);
            let t = ui.tokens();
            for hint in &self.hints[..shown] {
                let size = vec2(hint.width(ui), HINT_HEIGHT);
                let (rect, response) = ui.allocate_exact_size(size, Sense::hover());
                let spoken = hint.spoken();
                response.widget_info(|| WidgetInfo::labeled(WidgetType::Label, true, &spoken));
                if ui.is_rect_visible(rect) {
                    hint.paint(ui, &t, rect.left(), rect.center().y);
                }
            }
        });
        shown
    }
}

/// One of the Status Bar's right-hand items (`.sb-item`): the editor's
/// counts (`11 by 11 tiles`), the module's name with the modified dot
/// before it, in 13 px `text-2`; weak ([`StatusItem::weak`], the game
/// folder), 11 px `text-3`. A label named by its words, with a tooltip
/// where given (the module's full path).
#[derive(Clone, Debug)]
#[must_use = "add it with ui.add"]
pub struct StatusItem {
    text: String,
    weak: bool,
    modified: bool,
    tooltip: Option<String>,
}

impl StatusItem {
    /// An item reading `text`.
    pub fn new(text: impl Into<String>) -> Self {
        StatusItem { text: text.into(), weak: false, modified: false, tooltip: None }
    }

    /// Weak: 11 px `text-3` (`.sb-item.weak`).
    pub fn weak(mut self) -> Self {
        self.weak = true;
        self
    }

    /// The modified dot before its words where `modified`.
    pub fn modified(mut self, modified: bool) -> Self {
        self.modified = modified;
        self
    }

    /// Shown on hover.
    pub fn tooltip(mut self, tooltip: impl Into<String>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }

    fn font(&self) -> FontId {
        if self.weak { small_font() } else { body_font() }
    }

    /// The width it takes in `ui`.
    pub fn width(&self, ui: &Ui) -> f32 {
        let galley = ui.painter().layout_no_wrap(self.text.clone(), self.font(), Color32::WHITE);
        let dot = if self.modified { marks::DOT + ITEM_GAP } else { 0.0 };
        dot + galley.size().x
    }
}

impl egui::Widget for StatusItem {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = ui.tokens();
        let color = if self.weak { t.text_3 } else { t.text_2 };
        let galley = ui.painter().layout_no_wrap(self.text.clone(), self.font(), color);
        let size = vec2(self.width(ui), galley.size().y.max(marks::MARK_HEIGHT));
        let (rect, response) = ui.allocate_exact_size(size, Sense::hover());
        let text = self.text.clone();
        response.widget_info(|| WidgetInfo::labeled(WidgetType::Label, true, &text));
        if ui.is_rect_visible(rect) {
            let mut x = rect.left();
            if self.modified {
                let center = pos2(x + marks::DOT / 2.0, rect.center().y);
                marks::paint_modified_dot(ui.painter(), &t, center);
                x += marks::DOT + ITEM_GAP;
            }
            let pos = Pos2::new(x, (rect.center().y - galley.size().y / 2.0).round());
            ui.painter().galley(pos, galley, color);
        }
        match self.tooltip {
            Some(tip) => response.on_hover_text(tip),
            None => response,
        }
    }
}

/// A running job in the Status Bar (`.sb-job`): its words in `text-2`, a
/// 60 by 4 bar in an `inset` well with the part done in `accent`. A
/// progress indicator named by its words, the part done its value (none
/// where it is not known: the bar then empty).
#[derive(Clone, Debug)]
#[must_use = "add it with ui.add"]
pub struct StatusJob {
    text: String,
    done: Option<f32>,
}

impl StatusJob {
    /// A job named `text`, `done` of it done (0 to 1) where known.
    pub fn new(text: impl Into<String>, done: Option<f32>) -> Self {
        StatusJob { text: text.into(), done }
    }
}

impl egui::Widget for StatusJob {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = ui.tokens();
        let galley = ui.painter().layout_no_wrap(self.text.clone(), body_font(), t.text_2);
        let size = vec2(galley.size().x + JOB_GAP + JOB_BAR.x, galley.size().y);
        let (rect, response) = ui.allocate_exact_size(size, Sense::hover());
        let (text, done) = (self.text.clone(), self.done);
        response.widget_info(|| {
            let mut info = WidgetInfo::labeled(WidgetType::ProgressIndicator, true, &text);
            info.value = done.map(f64::from);
            info
        });
        if ui.is_rect_visible(rect) {
            let p = ui.painter();
            let pos = pos2(rect.left(), (rect.center().y - galley.size().y / 2.0).round());
            let words = galley.size().x;
            p.galley(pos, galley, t.text_2);
            let bar = Rect::from_min_size(
                pos2(rect.left() + words + JOB_GAP, rect.center().y - JOB_BAR.y / 2.0),
                JOB_BAR,
            );
            p.rect_filled(bar, 2, t.inset);
            if let Some(done) = self.done {
                let part =
                    Rect::from_min_size(bar.min, vec2(JOB_BAR.x * done.clamp(0.0, 1.0), 4.0));
                p.rect_filled(part, 2, t.accent);
            }
        }
        response
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Hints drop whole from the end: at a width between two hints' ends
    /// the first fits and the second is left out, never cut; at none,
    /// none; with room for all, all.
    #[test]
    fn hints_drop_whole_from_the_end() {
        let widths = [60.0, 80.0, 50.0];
        assert_eq!(hints_that_fit(&widths, 8.0, 1000.0), 3);
        assert_eq!(hints_that_fit(&widths, 8.0, 60.0 + 8.0 + 80.0 + 8.0 + 50.0), 3);
        assert_eq!(hints_that_fit(&widths, 8.0, 60.0 + 8.0 + 80.0 + 8.0 + 49.0), 2);
        assert_eq!(hints_that_fit(&widths, 8.0, 100.0), 1);
        assert_eq!(hints_that_fit(&widths, 8.0, 59.0), 0);
        assert_eq!(hints_that_fit(&widths, 8.0, 0.0), 0);
        assert_eq!(hints_that_fit(&[], 8.0, 10.0), 0);
        // A small hint after a wide one that does not fit stays out too:
        // they are in order of importance.
        assert_eq!(hints_that_fit(&[60.0, 300.0, 10.0], 8.0, 200.0), 1);
    }

    /// A hint's name is its keys, joiners and word in order; an icon's
    /// the name its caller gave it, and an unnamed icon says nothing (not
    /// its file's name: "rotate Turn").
    #[test]
    fn a_hint_says_its_keys_and_word() {
        assert_eq!(Hint::keys(["Shift", "Right Drag"], "Turn").spoken(), "Shift + Right Drag Turn");
        assert_eq!(Hint::either(["Q", "E"], "Turn by a step").spoken(), "Q / E Turn by a step");
        assert_eq!(Hint::keys(["Drag"], "Move").spoken(), "Drag Move");
        let named = HintPart::NamedIcon { icon: Icon::Rotate, name: "Turn handle".into() };
        assert_eq!(Hint::new("Turn").part(named).spoken(), "Turn handle Turn");
        assert_eq!(Hint::new("Turn").part(HintPart::Icon(Icon::Rotate)).spoken(), "Turn");
    }
}
