//! An editor's header row (proposal sections 6.1 and 8.1; the mock's
//! `.editor-header`): one row, 30 px of `bar` with a 1 px `line` under
//! it, that never wraps and never overflows. From the left: the left
//! group (what you do), a spacer, the right group (how the view looks),
//! the **More** button where something is in it, and the end slot (OK
//! and Cancel, or Close), after a line.
//!
//! The header lays itself out against the width it has, in the same
//! frame (its items are measured, not drawn and then measured), so a
//! width always gives the same row. Where its items do not fit, it
//! gives up, one step at a time and in this order, until they do:
//!
//! 1. a popover unfolded into the row ([`Item::unfolds`], section 5.3)
//!    folds back to its button, from the right group's end (and where
//!    only popovers were folded, one folded early that fits again beside
//!    those folded after it unfolds once more, from the left: a narrow
//!    popover at the right group's end stays unfolded beside a wide one
//!    folded);
//! 2. the left group's word buttons lose their words (an icon with a
//!    tooltip; a part of the caller's takes its compact form,
//!    [`Item::compact`]), from its end to its start, then the right
//!    group's;
//! 3. they move into More in the same order, left group then right
//!    group, but for the first commands ([`Item::primary`]: Save), which
//!    move last of all, so that secondary commands go before them
//!    (section 6.1, the owner's ruling of U1's review).
//!
//! The end slot never yields and is never covered: it keeps the right
//! end at every width. The row's least width is the end slot's and the
//! padding (`end_width + 12`); under it the end slot is clipped to the
//! row, never drawn outside it. What does not fit (items that may not
//! yield, [`Yield::Never`], in a row too narrow; and below the room for
//! More beside the end slot, More and what is in it) is cut, not drawn
//! over the More button or the end slot, and recorded so.
//!
//! An editor whose forms change at widths of their own (the area's
//! compact header under an editor 990 px wide, [`crate::shell::Yields`])
//! chooses the items it gives by them, and forces an item into More with
//! [`Item::in_more`] or keeps one on the row with [`Item::yields`].
//!
//! The header keeps the kit's text size (the body's 13 points) whatever
//! the window's style (the conversation editor's larger text): it is
//! chrome, as the Top Bar is.
//!
//! The header records where each item went ([`Place`]), with the row's,
//! the end slot's, More's and an open popover's rectangles, in its
//! output and for tests ([`records`], [`places`]), so that an editor's
//! test can narrow it to its least width and find every control on the
//! row or in More.
//!
//! Accessible names: the row is a group named by the header's name
//! ("Script header"); each item is named by its words however it is
//! shown; the More button "More", its popover a group named "More in"
//! and the header's name.
//!
//! ```
//! # egui::__run_test_ui(|ui| {
//! use mg_uikit::header::{EditorHeader, Item};
//! use mg_uikit::Icon;
//! let mut bookmark = false;
//! let out = EditorHeader::new("Script header")
//!     .left(Item::button(Icon::Save, "Save").primary().enabled(false))
//!     .left(Item::button(Icon::Compile, "Compile").primary())
//!     .left(Item::separator())
//!     .left(Item::toggle(Some(Icon::Bookmark), "Bookmark", &mut bookmark).icon_only())
//!     .left(Item::icon(Icon::ToScratch, "To Scratch").in_more())
//!     .right(Item::custom("Codepage", 90.0, |ui| {
//!         ui.weak("Windows-1252");
//!     }))
//!     .show(ui);
//! if out.clicked("Compile") {
//!     // compile
//! }
//! # });
//! ```

use std::cell::RefCell;

use egui::{Color32, Id, Rect, Response, Sense, Stroke, Ui, UiBuilder, pos2, vec2};

use crate::button::{ButtonLook, KitButton};
use crate::icons::Icon;
use crate::metrics;
use crate::popover::{self, Hang, Popover};
use crate::theme::TokensExt as _;

/// The header's padding at each end (`.editor-header`: `padding: 0 6px`).
pub const HEADER_PADDING: f32 = 6.0;
/// The gap between the header's items (`.editor-header`: `gap: 4px`).
pub const HEADER_GAP: f32 = 4.0;
/// The seam between a joined group's buttons (`.join`: `gap: 1px`).
pub const JOIN_SEAM: f32 = 1.0;
/// The room at each side of a joined group's words ([`Joined::words`]).
pub const JOIN_WORD_PADDING: f32 = 4.0;
/// The More button's accessible name.
pub const MORE: &str = "More";

/// Where an item of an [`EditorHeader`] went.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Place {
    /// On the row in its first form: a button with its words (an icon
    /// button as it always is), a part of the caller's at its full
    /// width, a joined group, a separator.
    Row,
    /// On the row in its yielded form: a word button's icon alone (its
    /// words its tooltip and name), a popover's or menu's icon and
    /// chevron, a part of the caller's in its compact form.
    Icon,
    /// On the row, its popover's rows unfolded into it.
    Unfolded,
    /// In the More popover.
    More,
    /// Not drawn: the row is too narrow even with everything yielded.
    Cut,
}

/// What an item may give up where the header is too narrow.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Yield {
    /// Nothing: it stays as it is (a field, OK and Cancel's neighbors).
    Never,
    /// Its words (it has an icon) or its full form (it has a compact
    /// one), never its place on the row.
    Icon,
    /// Its words, then its place (into More).
    Full,
    /// Its place, straight into More (an icon alone, words alone).
    More,
}

type AddUi<'a> = Box<dyn FnOnce(&mut Ui) + 'a>;

/// One button of a joined group ([`Item::joined`]): a toggle or a button,
/// its icon alone, named by its words.
#[must_use = "add it to Item::joined"]
pub struct Joined<'a> {
    icon: Icon,
    name: String,
    toggle: Option<&'a mut bool>,
    enabled: bool,
    tip: Option<String>,
    /// Words between the group's buttons, no button ([`Joined::words`]).
    words: bool,
}

impl std::fmt::Debug for Joined<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Joined").field("name", &self.name).field("toggle", &self.toggle).finish()
    }
}

impl<'a> Joined<'a> {
    /// A toggle of `on`, showing `icon`, named `name` (Show Creatures).
    pub fn toggle(icon: Icon, name: impl Into<String>, on: &'a mut bool) -> Self {
        Joined { icon, name: name.into(), toggle: Some(on), enabled: true, tip: None, words: false }
    }

    /// A button showing `icon`, named `name`.
    pub fn button(icon: Icon, name: impl Into<String>) -> Self {
        Joined { icon, name: name.into(), toggle: None, enabled: true, tip: None, words: false }
    }

    /// Words between the group's buttons (`.join-word`: 4 px at each
    /// side, on `raised`, in `text`), no button: the AreaTiles board's
    /// "Appearance" between Previous and Next Tile Appearance. Not a
    /// control: no name in the header's record, no row in More (where
    /// the group's heading is its name).
    pub fn words(words: impl Into<String>) -> Self {
        let name = words.into();
        Joined { icon: Icon::Info, name, toggle: None, enabled: true, tip: None, words: true }
    }

    /// Its width on the row: an icon button's, or its words' and their
    /// padding.
    fn width(&self, ui: &Ui) -> f32 {
        match self.words {
            true => {
                let font = egui::FontId::proportional(metrics::BODY_SIZE);
                let galley = ui.painter().layout_no_wrap(self.name.clone(), font, Color32::WHITE);
                (galley.size().x + 2.0 * JOIN_WORD_PADDING).ceil()
            }
            false => metrics::ICON_BUTTON,
        }
    }

    /// Enabled (`true`, the default) or drawn and said disabled.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Its tooltip (its name where not given).
    pub fn tip(mut self, tip: impl Into<String>) -> Self {
        self.tip = Some(tip.into());
        self
    }

    fn kit_button(&self, first: bool, last: bool, enabled: bool) -> KitButton {
        let mut b = KitButton::new(Some(self.icon), self.name.as_str())
            .look(ButtonLook::Toggle)
            .toggle(self.toggle.as_deref().copied().unwrap_or(false))
            .icon_only(true)
            .joined(first, last)
            .enabled(enabled && self.enabled);
        if self.toggle.is_none() {
            b = b.look(ButtonLook::Raised);
        }
        if let Some(tip) = &self.tip {
            b = b.tip(tip.as_str());
        }
        b
    }
}

/// Paints a joined group's words ([`Joined::words`]) at `ui`'s cursor:
/// `raised`, the control's height, its words in `text` (`text-off` where
/// the group is disabled), 4 px at each side.
fn paint_join_words(ui: &mut Ui, words: &str, enabled: bool) {
    let t = ui.tokens();
    let font = egui::FontId::proportional(metrics::BODY_SIZE);
    let color = if enabled { t.text } else { t.text_off };
    let galley = ui.painter().layout_no_wrap(words.to_string(), font, color);
    let size = vec2((galley.size().x + 2.0 * JOIN_WORD_PADDING).ceil(), metrics::CONTROL_HEIGHT);
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    ui.painter().rect_filled(rect, 0.0, t.raised);
    let at = pos2(rect.left() + JOIN_WORD_PADDING, rect.center().y - galley.size().y / 2.0);
    ui.painter().galley(at.round(), galley, color);
}

enum Kind<'a> {
    Button {
        icon: Option<Icon>,
        look: ButtonLook,
        toggle: Option<&'a mut bool>,
        icon_only: bool,
    },
    Popover {
        icon: Option<Icon>,
        popover: Popover<'a>,
        unfolds: bool,
    },
    Menu {
        icon: Option<Icon>,
        add: AddUi<'a>,
    },
    Custom {
        width: f32,
        add: AddUi<'a>,
        compact: Option<(f32, AddUi<'a>)>,
        more: Option<(f32, AddUi<'a>)>,
    },
    Joined {
        parts: Vec<Joined<'a>>,
    },
    Separator,
}

/// An item of an [`EditorHeader`]: a button, a toggle, a popover's or a
/// menu's button, a joined group of icon buttons, a part of the caller's
/// (a field, a combo box, a segmented control, a note) given its width,
/// or a separator.
#[must_use = "add it with EditorHeader::left or ::right"]
pub struct Item<'a> {
    name: String,
    kind: Kind<'a>,
    yields: Yield,
    in_more: bool,
    primary: bool,
    enabled: bool,
    tip: Option<String>,
    words_icon: bool,
    force_icon: bool,
    shown_words: Option<String>,
    count: Option<String>,
}

impl std::fmt::Debug for Item<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let kind = match &self.kind {
            Kind::Button { .. } => "button",
            Kind::Popover { .. } => "popover",
            Kind::Menu { .. } => "menu",
            Kind::Custom { .. } => "custom",
            Kind::Joined { .. } => "joined",
            Kind::Separator => "separator",
        };
        f.debug_struct("Item")
            .field("name", &self.name)
            .field("kind", &kind)
            .field("yields", &self.yields)
            .field("in_more", &self.in_more)
            .field("primary", &self.primary)
            .finish()
    }
}

impl<'a> Item<'a> {
    fn new(name: String, kind: Kind<'a>, yields: Yield) -> Self {
        Item {
            name,
            kind,
            yields,
            in_more: false,
            primary: false,
            enabled: true,
            tip: None,
            words_icon: true,
            force_icon: false,
            shown_words: None,
            count: None,
        }
    }

    /// A word button with its icon (`.btn`): Save, Compile, Add Row. It
    /// loses its words (its icon alone, still framed), then moves into
    /// More.
    pub fn button(icon: Icon, words: impl Into<String>) -> Self {
        let kind = Kind::Button {
            icon: Some(icon),
            look: ButtonLook::Raised,
            toggle: None,
            icon_only: false,
        };
        Self::new(words.into(), kind, Yield::Full)
    }

    /// A word button without an icon (Area Properties, Remove (3)): it
    /// moves into More whole.
    pub fn words(words: impl Into<String>) -> Self {
        let kind =
            Kind::Button { icon: None, look: ButtonLook::Raised, toggle: None, icon_only: false };
        Self::new(words.into(), kind, Yield::More)
    }

    /// An icon button (`.btn-icon`, frameless until hovered), named and
    /// tipped `name` (Print…, External Editor): it moves into More whole,
    /// as a row with its words.
    pub fn icon(icon: Icon, name: impl Into<String>) -> Self {
        let kind = Kind::Button {
            icon: Some(icon),
            look: ButtonLook::Flat,
            toggle: None,
            icon_only: true,
        };
        Self::new(name.into(), kind, Yield::More)
    }

    /// A toggle of `on` (`.toggle`), with its icon (where given) and
    /// words; [`Item::icon_only`] for an icon toggle. In More, a check
    /// row.
    pub fn toggle(icon: Option<Icon>, words: impl Into<String>, on: &'a mut bool) -> Self {
        let yields = if icon.is_some() { Yield::Full } else { Yield::More };
        let kind =
            Kind::Button { icon, look: ButtonLook::Toggle, toggle: Some(on), icon_only: false };
        Self::new(words.into(), kind, yields)
    }

    /// A popover's button (`.btn` with a chevron) reading the popover's
    /// name, and the popover hung from it: below it left of the header's
    /// middle, below-right right of it. As an icon (it has one), the icon
    /// and the chevron, framed (the narrow board's Lighting); in More,
    /// its rows under a heading of its name.
    pub fn popover(icon: Option<Icon>, popover: Popover<'a>) -> Self {
        let yields = if icon.is_some() { Yield::Full } else { Yield::More };
        let name = popover.name().to_string();
        let mut item = Self::new(name, Kind::Popover { icon, popover, unfolds: false }, yields);
        item.words_icon = false;
        item
    }

    /// A menu's button (`.btn` with a chevron: Export) and `add`, the
    /// menu's rows, in a popover hung from it that a click on one of its
    /// enabled rows closes. In More, `add` under a heading of its words.
    pub fn menu(
        icon: Option<Icon>,
        words: impl Into<String>,
        add: impl FnOnce(&mut Ui) + 'a,
    ) -> Self {
        let yields = if icon.is_some() { Yield::Full } else { Yield::More };
        let mut item = Self::new(words.into(), Kind::Menu { icon, add: Box::new(add) }, yields);
        item.words_icon = false;
        item
    }

    /// A part of the caller's, named `name` in the header's record:
    /// `add` laid out left to right in a slot `width` wide and a control
    /// high (a search field, a number field, a combo box, a segmented
    /// control, a note). It never yields, unless it is given a compact
    /// form ([`Item::compact`]) or a form for More ([`Item::more`]).
    pub fn custom(name: impl Into<String>, width: f32, add: impl FnOnce(&mut Ui) + 'a) -> Self {
        let kind = Kind::Custom { width, add: Box::new(add), compact: None, more: None };
        Self::new(name.into(), kind, Yield::Never)
    }

    /// A joined group of icon buttons (`.join`: 1 px seams, its outer
    /// corners round), named `name` in the header's record: the area's
    /// kind toggles, the Conversation board's five highlights. It yields
    /// as one: whole into More, where it is a heading of its name over a
    /// check row for each toggle and a row for each button.
    pub fn joined(name: impl Into<String>, parts: Vec<Joined<'a>>) -> Self {
        Self::new(name.into(), Kind::Joined { parts }, Yield::More)
    }

    /// A line between groups of the row (`.vsep`); one that would stand
    /// at a group's start or end or beside another, its items gone into
    /// More, is left out, but for one at the right group's end where the
    /// More button follows it (the Script board's codepage, a line, More).
    pub fn separator() -> Self {
        Self::new(String::new(), Kind::Separator, Yield::Never)
    }

    /// A custom item's compact form: `add` in a slot `width` wide, taken
    /// at the step where word buttons lose their words (Snap and Turn
    /// without their value text). With it the item yields so much.
    pub fn compact(mut self, width: f32, add: impl FnOnce(&mut Ui) + 'a) -> Self {
        if let Kind::Custom { compact, more, .. } = &mut self.kind {
            *compact = Some((width, Box::new(add)));
            self.yields = if more.is_some() { Yield::Full } else { Yield::Icon };
        }
        self
    }

    /// A custom item's form in More: `add`, given a popover's width
    /// (laid out top down; at least `width` wide), in place of it on the
    /// row. With it the item moves into More where the row is too
    /// narrow.
    pub fn more(mut self, width: f32, add: impl FnOnce(&mut Ui) + 'a) -> Self {
        if let Kind::Custom { more, compact, .. } = &mut self.kind {
            *more = Some((width, Box::new(add)));
            self.yields = if compact.is_some() { Yield::Full } else { Yield::More };
        }
        self
    }

    /// Always in More, whatever the width (To Scratch).
    pub fn in_more(mut self) -> Self {
        self.in_more = true;
        self
    }

    /// One of the header's first commands (Save, Compile): it moves into
    /// More last of everything that may (section 6.1, "secondary commands
    /// before Save"). Its words go at the left group's step as others'.
    pub fn primary(mut self) -> Self {
        self.primary = true;
        self
    }

    /// What it may give up ([`Yield`]; each kind has its own default).
    pub fn yields(mut self, yields: Yield) -> Self {
        self.yields = yields;
        self
    }

    /// A popover item's rows unfold into the row (section 5.3) where the
    /// row has room for them; they fold back first where it has not.
    pub fn unfolds(mut self) -> Self {
        if let Kind::Popover { unfolds, .. } = &mut self.kind {
            *unfolds = true;
        }
        self
    }

    /// Enabled (`true`, the default) or drawn in the disabled tokens and
    /// said disabled wherever it is: its button on the row; a popover's
    /// switch and rows hung, unfolded and in More; a menu's rows in More;
    /// a part of the caller's (its `Ui` disabled) on the row, compact and
    /// in More; a joined group's buttons on the row and its rows in More.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Its tooltip (an icon's is its words where not given).
    pub fn tip(mut self, tip: impl Into<String>) -> Self {
        self.tip = Some(tip.into());
        self
    }

    /// A button, toggle, popover's or menu's button that shows its icon
    /// alone (a popover's or menu's with its chevron), whatever the width
    /// (it moves into More whole): the area's compact header's Lighting,
    /// Overlays and Audio (section 5.2), Build & Compile.
    pub fn icon_only(mut self) -> Self {
        let has_icon = match &mut self.kind {
            Kind::Button { icon_only, icon: Some(_), .. } => {
                *icon_only = true;
                true
            }
            Kind::Popover { icon: Some(_), unfolds, .. } => {
                *unfolds = false;
                self.force_icon = true;
                true
            }
            Kind::Menu { icon: Some(_), .. } => {
                self.force_icon = true;
                true
            }
            _ => false,
        };
        if has_icon && (self.yields == Yield::Full || self.yields == Yield::Icon) {
            self.yields = Yield::More;
        }
        self
    }

    /// Looking so ([`ButtonLook`]): a button's [`ButtonLook::Primary`]
    /// (Build & Compile) or [`ButtonLook::Flat`].
    pub fn look(mut self, look: ButtonLook) -> Self {
        if let Kind::Button { look: l, toggle: None, .. } = &mut self.kind {
            *l = look;
        }
        self
    }

    /// Whether a button's, popover's or menu's button shows its icon
    /// beside its words (the Talk board's Export does; Lighting, View and
    /// More do not, the default for popovers and menus; the Main board's
    /// Area Properties neither, its icon shown alone where its words
    /// yield). A button made by [`Item::button`] or a toggle with an icon
    /// shows it unless told not to.
    pub fn words_icon(mut self, shown: bool) -> Self {
        if matches!(self.kind, Kind::Popover { .. } | Kind::Menu { .. } | Kind::Button { .. }) {
            self.words_icon = shown;
        }
        self
    }

    /// The words its button shows where they are not its name: the
    /// compact area header's Show popover reads "Show 5 of 10", named
    /// "Show" (its name stays its accessible name, its name in the record
    /// and its heading in More). A button's, popover's or menu's.
    pub fn shown_words(mut self, words: impl Into<String>) -> Self {
        self.shown_words = Some(words.into());
        self
    }

    /// A weak count after its button's words (`.count-w`, the compact
    /// area header's Show "5 of 10"; [`KitButton::count`]): a button's,
    /// popover's or menu's, not shown with its icon alone.
    pub fn count(mut self, count: impl Into<String>) -> Self {
        self.count = Some(count.into());
        self
    }

    /// Its name (its accessible name and its name in the record).
    pub fn name(&self) -> &str {
        &self.name
    }

    fn shown_icon(&self) -> Option<Icon> {
        match &self.kind {
            Kind::Button { icon, .. } | Kind::Popover { icon, .. } | Kind::Menu { icon, .. } => {
                *icon
            }
            _ => None,
        }
    }

    /// The kit button showing it in `form` (`open`: its popover is open).
    fn kit_button(&self, form: Place, open: bool) -> Option<KitButton> {
        let icon_form = form == Place::Icon;
        let mut b = match &self.kind {
            Kind::Button { icon, look, toggle, icon_only } => {
                let shown_icon =
                    if self.words_icon || *icon_only || icon_form { *icon } else { None };
                let mut b = KitButton::new(shown_icon, self.words_shown())
                    .look(*look)
                    .icon_only(*icon_only || icon_form);
                if let Some(on) = toggle {
                    b = b.toggle(**on);
                }
                b
            }
            Kind::Popover { icon, .. } | Kind::Menu { icon, .. } => {
                let shown_icon = if self.words_icon || icon_form { *icon } else { None };
                KitButton::new(shown_icon, self.words_shown())
                    .look(ButtonLook::Raised)
                    .icon_only(icon_form)
                    .opens(open)
            }
            _ => return None,
        };
        b = b.enabled(self.enabled);
        if self.shown_words.is_some() {
            b = b.name(self.name.as_str());
        }
        if let Some(count) = self.count.as_ref().filter(|_| !b.is_icon_only()) {
            b = b.count(count.as_str());
        }
        if let Some(tip) = &self.tip {
            b = b.tip(tip.as_str());
        }
        Some(b)
    }

    /// The words its button shows ([`Item::shown_words`], or its name).
    fn words_shown(&self) -> &str {
        self.shown_words.as_deref().unwrap_or(&self.name)
    }

    /// Its width on the row in `form` (where it has that form).
    fn width(&self, ui: &Ui, form: Place) -> Option<f32> {
        match (&self.kind, form) {
            (Kind::Separator, Place::Row) => Some(crate::shell::VSEP_WIDTH),
            (Kind::Custom { width, .. }, Place::Row) => Some(*width),
            (Kind::Custom { compact: Some((width, _)), .. }, Place::Icon) => Some(*width),
            (Kind::Joined { parts }, Place::Row) => {
                let n = parts.len() as f32;
                let each: f32 = parts.iter().map(|p| p.width(ui)).sum();
                Some(each + (n - 1.0).max(0.0) * JOIN_SEAM)
            }
            (Kind::Popover { popover, unfolds: true, .. }, Place::Unfolded) => {
                popover.can_unfold().then(|| popover.unfolded_width(ui))
            }
            (Kind::Button { icon_only: true, .. }, Place::Row) => None,
            (Kind::Popover { .. } | Kind::Menu { .. }, Place::Row) if self.force_icon => None,
            (Kind::Button { .. } | Kind::Popover { .. } | Kind::Menu { .. }, Place::Row) => {
                self.kit_button(Place::Row, false).map(|b| b.width(ui))
            }
            (Kind::Button { .. } | Kind::Popover { .. } | Kind::Menu { .. }, Place::Icon) => {
                self.shown_icon()?;
                self.kit_button(Place::Icon, false).map(|b| b.width(ui))
            }
            _ => None,
        }
    }

    /// Whether it may give up its first form for its yielded one.
    fn may_yield_form(&self) -> bool {
        matches!(self.yields, Yield::Icon | Yield::Full)
            && !matches!(self.kind, Kind::Joined { .. } | Kind::Separator)
    }

    /// Whether it may move into More.
    fn may_go_into_more(&self) -> bool {
        match &self.kind {
            Kind::Separator => false,
            Kind::Custom { more, .. } => more.is_some() && self.yields != Yield::Never,
            _ => matches!(self.yields, Yield::Full | Yield::More),
        }
    }
}

/// Which side of the row an item stands at.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Side {
    Left,
    Right,
}

/// What an [`EditorHeader`] did.
#[derive(Debug)]
pub struct HeaderOutput<R> {
    /// The row's response (its whole rectangle): it senses a click but
    /// takes no keyboard focus, so Tab goes from item to item. A right
    /// click anywhere on the row, over an item too, is
    /// [`HeaderOutput::secondary_clicked`] ([`HeaderOutput::context_menu`]).
    pub response: Response,
    /// The row was right-clicked this frame, on an item or between them.
    pub secondary_clicked: bool,
    /// Each item drawn on the row, by its name, with its response (a
    /// joined group's buttons by theirs; not a part of the caller's).
    pub responses: Vec<(String, Response)>,
    /// Where each item went, by its name, in the row's order (separators
    /// left out); after a popover unfolded into the row, its switch's and
    /// rows' names, after a joined group its buttons', each in the
    /// group's place.
    pub places: Vec<(String, Place)>,
    /// The names of the buttons clicked (on the row or in More) and of
    /// the action rows chosen in its popovers.
    pub clicked: Vec<String>,
    /// A toggle, a check row or a switch changed.
    pub changed: bool,
    /// The item whose popover is open (or [`MORE`]) after this frame.
    pub open: Option<String>,
    /// Where the end slot was laid out (never covered; under the row's
    /// least width, clipped to the row).
    pub end_rect: Option<Rect>,
    /// Where the popover open from the row (an item's, or More's) was
    /// drawn this frame.
    pub popover_rect: Option<Rect>,
    /// What the end slot gave.
    pub end: R,
}

impl<R> HeaderOutput<R> {
    /// Whether the button or action named `name` was clicked.
    pub fn clicked(&self, name: &str) -> bool {
        self.clicked.iter().any(|c| c == name)
    }

    /// Where the item named `name` went.
    pub fn place(&self, name: &str) -> Option<Place> {
        self.places.iter().find(|(n, _)| n == name).map(|(_, p)| *p)
    }

    /// The response of the item named `name`, where it was drawn on the
    /// row this frame.
    pub fn response(&self, name: &str) -> Option<&Response> {
        self.responses.iter().find(|(n, _)| n == name).map(|(_, r)| r)
    }

    /// The header's own right-click menu (U3's: which tools to show),
    /// opened by a right click anywhere on the row and laid out by `add`
    /// in the kit's menu style.
    pub fn context_menu<T>(&self, add: impl FnOnce(&mut Ui) -> T) -> Option<T> {
        let open = self.secondary_clicked.then_some(egui::SetOpenCommand::Bool(true));
        egui::Popup::menu(&self.response)
            .id(self.response.id.with("mg-uikit-header-menu"))
            .open_memory(open)
            .at_pointer_fixed()
            .style(crate::menu_style)
            .show(add)
            .map(|r| r.inner)
    }
}

/// What a header drew, for tests ([`records`]).
#[derive(Clone, Debug, PartialEq)]
pub struct HeaderRecord {
    /// The header's id (two headers of one name are told apart by it).
    pub id: Id,
    /// The row's rectangle.
    pub row: Rect,
    /// The end slot's rectangle, where it has one.
    pub end: Option<Rect>,
    /// The More button's rectangle, where it is drawn.
    pub more: Option<Rect>,
    /// The open popover's rectangle (an item's or More's).
    pub popover: Option<Rect>,
    /// The item whose popover is open, or [`MORE`].
    pub open: Option<String>,
    /// Where each item went ([`HeaderOutput::places`]).
    pub places: Vec<(String, Place)>,
}

fn recording_id() -> Id {
    Id::new("mg-uikit-record-headers")
}

/// Has `ctx` keep header placement records (`HeaderOutput::places`,
/// [`records`], [`places`]) for tests.
pub fn record_headers(ctx: &egui::Context) {
    ctx.data_mut(|d| d.insert_temp(recording_id(), true));
}

fn is_recording_headers(ctx: &egui::Context) -> bool {
    ctx.data(|d| d.get_temp::<bool>(recording_id())).unwrap_or(false)
}

/// The id under which the records of the headers named `name` are kept.
fn record_id(name: &str) -> Id {
    Id::new(("mg-uikit-header-records", name))
}

/// What the headers named `name` drew in the last pass that drew one,
/// one record each (two docked script editors' headers, say), ordered by
/// where their rows are, top to bottom, then left to right.
pub fn records(ctx: &egui::Context, name: &str) -> Vec<HeaderRecord> {
    let kept: Vec<(u64, HeaderRecord)> =
        ctx.data(|d| d.get_temp(record_id(name))).unwrap_or_default();
    let last = kept.iter().map(|(p, _)| *p).max();
    let mut out: Vec<HeaderRecord> =
        kept.into_iter().filter(|(p, _)| Some(*p) == last).map(|(_, r)| r).collect();
    out.sort_by(|a, b| {
        (a.row.top(), a.row.left()).partial_cmp(&(b.row.top(), b.row.left())).unwrap()
    });
    out
}

/// Where each item of the first header named `name` went
/// ([`records`]; none if none was drawn).
pub fn places(ctx: &egui::Context, name: &str) -> Vec<(String, Place)> {
    records(ctx, name).into_iter().next().map(|r| r.places).unwrap_or_default()
}

/// Keeps `record` among the records of the headers named `name` drawn in
/// `pass`, those of earlier passes let go.
fn keep_record(ctx: &egui::Context, name: &str, pass: u64, record: HeaderRecord) {
    ctx.data_mut(|d| {
        let kept = d.get_temp_mut_or_default::<Vec<(u64, HeaderRecord)>>(record_id(name));
        kept.retain(|(p, r)| *p == pass && r.id != record.id);
        kept.push((pass, record));
    });
}

/// The accessible name of the popover of the More button of the header
/// named `header`.
pub fn more_name(header: &str) -> String {
    format!("More in {header}")
}

/// An editor's header row (module comment).
#[must_use = "show it with .show(ui)"]
pub struct EditorHeader<'a> {
    name: String,
    items: Vec<(Side, Item<'a>)>,
    more_words: bool,
    more_after: Option<String>,
    end_separator: bool,
    set_open: Option<Option<String>>,
}

impl std::fmt::Debug for EditorHeader<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EditorHeader")
            .field("name", &self.name)
            .field("items", &self.items)
            .finish()
    }
}

/// An element of the laid-out row.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Part {
    Item(usize),
    Line,
    Spacer,
    More,
    EndLine,
    End,
}

/// The open popover kept between frames: which, and the pass it was
/// last drawn in.
#[derive(Clone, Debug, Default)]
struct Open {
    name: Option<String>,
    pass: u64,
}

impl<'a> EditorHeader<'a> {
    /// A header named `name` ("Area header", "Script header").
    pub fn new(name: impl Into<String>) -> Self {
        EditorHeader {
            name: name.into(),
            items: Vec::new(),
            more_words: false,
            more_after: None,
            end_separator: true,
            set_open: None,
        }
    }

    /// An item of the left group (what you do), after those given.
    pub fn left(mut self, item: Item<'a>) -> Self {
        self.items.push((Side::Left, item));
        self
    }

    /// An item of the right group (how the view looks), after those
    /// given.
    pub fn right(mut self, item: Item<'a>) -> Self {
        self.items.push((Side::Right, item));
        self
    }

    /// The More button reads "More" with a chevron (the Table and Talk
    /// boards) rather than showing the `more-horizontal` icon alone.
    pub fn more_words(mut self, words: bool) -> Self {
        self.more_words = words;
        self
    }

    /// The More button stands right after the item named `name` (the NUI
    /// board's, after Export…) rather than at the right group's end;
    /// where the row is too narrow for everything, it stands before the
    /// end slot.
    pub fn more_after(mut self, name: impl Into<String>) -> Self {
        self.more_after = Some(name.into());
        self
    }

    /// Whether a line stands before the end slot (`true`, the default).
    pub fn end_separator(mut self, line: bool) -> Self {
        self.end_separator = line;
        self
    }

    /// Opens the popover of the item named `name` (or [`MORE`]) this
    /// frame, from code (a key, a command), closing any other; `None`
    /// closes the one open. Which is open after the frame is
    /// [`HeaderOutput::open`].
    pub fn set_open(mut self, name: Option<&str>) -> Self {
        self.set_open = Some(name.map(str::to_string));
        self
    }

    /// The row across `ui`'s width, with no end slot.
    pub fn show(self, ui: &mut Ui) -> HeaderOutput<()> {
        self.show_with_end(ui, 0.0, |_| ())
    }

    /// The row across `ui`'s width, its end slot `end_width` wide (OK and
    /// Cancel, measured by the caller) drawn by `end` at its right end.
    pub fn show_with_end<R>(
        self,
        ui: &mut Ui,
        end_width: f32,
        end: impl FnOnce(&mut Ui) -> R,
    ) -> HeaderOutput<R> {
        let t = ui.tokens();
        let width = ui.available_width();
        let (rect, response) =
            ui.allocate_exact_size(vec2(width, metrics::EDITOR_HEADER), Sense::CLICK);
        ui.painter().rect_filled(rect, 0.0, t.bar);
        let line_y = rect.bottom() - 0.5;
        ui.painter().hline(rect.x_range(), line_y, Stroke::new(1.0, t.line));
        let id = ui.make_persistent_id(("mg-uikit-header", &self.name));
        let has_end = end_width > 0.0;
        let pass = ui.ctx().cumulative_pass_nr();
        // (The kit's sizes, whatever the window's text: widths are
        // measured with them.)
        let mut kit_style = (**ui.style()).clone();
        crate::theme::kit_sizes(&mut kit_style);
        let kit_style = std::sync::Arc::new(kit_style);
        let mut measure = ui.new_child(UiBuilder::new().max_rect(rect).invisible());
        measure.set_style(std::sync::Arc::clone(&kit_style));

        let n = self.items.len();
        let recording = is_recording_headers(ui.ctx());
        // (Its items' names, separators none, for the record, and what
        // each records after it: a popover's rows where they unfold, a
        // joined group's buttons.)
        let (names, rows_of, joined_items) = if recording {
            let names = (self.items.iter())
                .map(|(_, item)| match item.kind {
                    Kind::Separator => None,
                    _ => Some(item.name.clone()),
                })
                .collect();
            let rows_of = (self.items.iter())
                .map(|(_, item)| match &item.kind {
                    Kind::Popover { popover, .. } => popover.check_names(),
                    Kind::Joined { parts } => {
                        parts.iter().filter(|p| !p.words).map(|p| p.name.clone()).collect()
                    }
                    _ => Vec::new(),
                })
                .collect();
            let joined_items = (self.items.iter())
                .map(|(_, item)| matches!(item.kind, Kind::Joined { .. }))
                .collect();
            (names, rows_of, joined_items)
        } else {
            (Vec::new(), Vec::new(), Vec::new())
        };
        let widths: Vec<[Option<f32>; 3]> = (self.items.iter())
            .map(|(_, item)| {
                [Place::Unfolded, Place::Row, Place::Icon].map(|form| item.width(&measure, form))
            })
            .collect();
        let mut forms: Vec<Place> = (0..n)
            .map(|i| {
                let item = &self.items[i].1;
                if item.in_more {
                    Place::More
                } else if widths[i][0].is_some() {
                    Place::Unfolded
                } else if widths[i][1].is_some() {
                    Place::Row
                } else {
                    Place::Icon
                }
            })
            .collect();

        // The steps, in the order they are taken (module comment).
        let order = |side: Side| -> Vec<usize> {
            (0..n).rev().filter(|&i| self.items[i].0 == side).collect()
        };
        let mut steps: Vec<(usize, Place)> = Vec::new();
        for i in order(Side::Right).into_iter().chain(order(Side::Left)) {
            if forms[i] == Place::Unfolded {
                let to = if widths[i][1].is_some() { Place::Row } else { Place::Icon };
                steps.push((i, to));
            }
        }
        // (The steps that fold an unfolded popover back, all first.)
        let folds = steps.len();
        let sides = || order(Side::Left).into_iter().chain(order(Side::Right));
        for i in sides() {
            let item = &self.items[i].1;
            if !item.in_more
                && item.may_yield_form()
                && widths[i][1].is_some()
                && widths[i][2].is_some()
            {
                steps.push((i, Place::Icon));
            }
        }
        for primary in [false, true] {
            for i in sides() {
                let item = &self.items[i].1;
                if !item.in_more && item.primary == primary && item.may_go_into_more() {
                    steps.push((i, Place::More));
                }
            }
        }

        let more_words = self.more_words;
        let more_button = move || match more_words {
            true => KitButton::new(None, MORE).opens(false),
            false => {
                let b = KitButton::new(Some(Icon::MoreHorizontal), MORE);
                b.icon_only(true).look(ButtonLook::Flat)
            }
        };
        let more_width = more_button().width(&measure);
        let room = rect.width() - 2.0 * HEADER_PADDING;
        let end_line = has_end && self.end_separator;

        // The row's parts for the forms (the separators that stand
        // between items alone), and its width.
        let mut parts = Vec::new();
        let parts_into = |forms: &[Place], more_at_end: bool, out: &mut Vec<Part>| {
            out.clear();
            let any_more = forms.contains(&Place::More);
            let more_after = self.more_after.as_ref().and_then(|name| {
                (0..n).find(|&i| self.items[i].1.name == *name && forms[i] != Place::More)
            });
            let more_here = any_more && (more_at_end || more_after.is_none());
            for side in [Side::Left, Side::Right] {
                if side == Side::Right {
                    out.push(Part::Spacer);
                }
                let mut empty = true;
                let mut pending_line = false;
                for i in (0..n).filter(|&i| self.items[i].0 == side) {
                    if forms[i] == Place::More {
                        continue;
                    }
                    if matches!(self.items[i].1.kind, Kind::Separator) {
                        pending_line = !empty;
                        continue;
                    }
                    if pending_line {
                        out.push(Part::Line);
                        pending_line = false;
                    }
                    out.push(Part::Item(i));
                    empty = false;
                    if any_more && !more_at_end && more_after == Some(i) {
                        out.push(Part::More);
                    }
                }
                // (A line at the right group's end stands before More
                // where More follows it: the Script board's codepage | ⋯.)
                if side == Side::Right && pending_line && more_here {
                    out.push(Part::Line);
                }
            }
            if more_here {
                out.push(Part::More);
            }
            if end_line {
                out.push(Part::EndLine);
            }
            if has_end {
                out.push(Part::End);
            }
        };
        let part_width = |part: Part, forms: &[Place]| -> f32 {
            match part {
                Part::Line | Part::EndLine => crate::shell::VSEP_WIDTH,
                Part::Item(i) => {
                    let slot = match forms[i] {
                        Place::Unfolded => 0,
                        Place::Row => 1,
                        _ => 2,
                    };
                    widths[i][slot].unwrap_or(0.0)
                }
                Part::Spacer => 0.0,
                Part::More => more_width,
                Part::End => end_width,
            }
        };
        let total = |parts: &[Part], forms: &[Place]| -> f32 {
            let sum: f32 = parts.iter().map(|&p| part_width(p, forms)).sum();
            sum + HEADER_GAP * parts.len().saturating_sub(1) as f32
        };
        parts_into(&forms, false, &mut parts);
        let mut fits = total(&parts, &forms) <= room;
        let mut taken = 0;
        for &(i, to) in &steps {
            if fits {
                break;
            }
            forms[i] = to;
            taken += 1;
            parts_into(&forms, false, &mut parts);
            fits = total(&parts, &forms) <= room;
        }
        // (Where only popovers were folded, one folded early that has room
        // again beside those folded after it unfolds once more, from the
        // left: the wide area header's Audio, narrower unfolded than its
        // button, beside Overlays folded.)
        if fits && taken <= folds {
            for &(i, _) in steps[..taken].iter().rev() {
                let was = forms[i];
                forms[i] = Place::Unfolded;
                parts_into(&forms, false, &mut parts);
                if total(&parts, &forms) > room {
                    forms[i] = was;
                }
            }
        }
        parts_into(&forms, !fits, &mut parts);

        // Where each part goes: where everything fits, the left group from
        // the left and the rest against the right end; where it does not,
        // everything from the left but More, its line and the end slot,
        // which keep the right end, and what reaches them is cut.
        let mut rects: Vec<(Part, Rect)> = Vec::new();
        let y = rect.top() + (metrics::EDITOR_HEADER - 1.0) / 2.0;
        let at = |x: f32, w: f32| {
            Rect::from_min_size(
                pos2(x, y - metrics::CONTROL_HEIGHT / 2.0),
                vec2(w, metrics::CONTROL_HEIGHT),
            )
        };
        let spacer = parts.iter().position(|&p| p == Part::Spacer).unwrap_or(0);
        let tail_from = if fits {
            spacer + 1
        } else {
            parts
                .iter()
                .position(|&p| matches!(p, Part::More | Part::EndLine | Part::End))
                .unwrap_or(parts.len())
        };
        let mut x = rect.left() + HEADER_PADDING;
        for &part in &parts[..tail_from.min(parts.len())] {
            let w = part_width(part, &forms);
            if part != Part::Spacer {
                rects.push((part, at(x, w)));
                x += w + HEADER_GAP;
            }
        }
        let tail = &parts[tail_from.min(parts.len())..];
        let tail_width = total(tail, &forms);
        // (The end slot keeps the right end whatever the width; More and
        // its line, where even they do not fit beside it, are left out,
        // and what is in More is cut with them.)
        let mut x = rect.right() - HEADER_PADDING - tail_width;
        let first = rect.left() + HEADER_PADDING - 0.5;
        if fits {
            x = x.max(rect.left() + HEADER_PADDING);
        }
        let tail_left = x;
        for &part in tail {
            let w = part_width(part, &forms);
            if part != Part::Spacer {
                rects.push((part, at(x, w)));
                x += w + HEADER_GAP;
            }
        }
        if !fits {
            let more_cut = rects.iter().any(|(p, r)| *p == Part::More && r.left() < first);
            for form in forms.iter_mut() {
                if more_cut && *form == Place::More {
                    *form = Place::Cut;
                }
            }
            for (part, r) in &rects {
                if let Part::Item(i) = *part
                    && r.right() > tail_left - HEADER_GAP + 0.5
                {
                    forms[i] = Place::Cut;
                }
            }
            rects.retain(|(part, r)| match part {
                Part::End => true,
                Part::More | Part::EndLine => r.left() >= first,
                _ => r.right() <= tail_left - HEADER_GAP + 0.5,
            });
        }

        // Which popover is open: as kept, unless the header was not drawn
        // in the last pass (another tab was in front: it does not reopen
        // by itself), or code asked otherwise.
        let open_id = id.with("open");
        let kept: Open = ui.data(|d| d.get_temp(open_id)).unwrap_or_default();
        let popover_id = |name: &str| id.with(("popover", name));
        let mut open = if kept.pass + 1 >= pass { kept.name.clone() } else { None };
        if open.is_none()
            && let Some(name) = &kept.name
        {
            Popover::forget(ui.ctx(), popover_id(name));
        }
        if let Some(asked) = self.set_open.clone() {
            open = asked;
        }
        let was_open = open.clone();
        let mut out_clicked: Vec<String> = Vec::new();
        let mut responses: Vec<(String, Response)> = Vec::new();
        let mut changed = false;
        let mut end_out = None;
        let mut end_rect = None;
        let more_clicked = RefCell::new(Vec::<String>::new());
        let more_changed = std::cell::Cell::new(false);
        let mut hung: Option<(Popover<'_>, Rect)> = None;
        let mut more_rect = None;
        let mut keyboard_opened = false;
        let mut items: Vec<Option<(Side, Item<'a>)>> = self.items.into_iter().map(Some).collect();
        let mut end = Some(end);
        let clip = rect;
        let row = UiBuilder::new()
            .max_rect(rect)
            .layout(egui::Layout::left_to_right(egui::Align::Center));
        let mut toggle_open = |open: &mut Option<String>, name: &str, response: &Response| {
            if response.clicked() {
                let is_open = open.as_deref() == Some(name);
                *open = if is_open { None } else { Some(name.to_string()) };
                if !is_open && popover::opened_by_keyboard(&response.ctx) {
                    keyboard_opened = true;
                }
            }
        };
        ui.scope_builder(row, |ui| {
            ui.set_style(kit_style);
            crate::icons::name_group(ui, &self.name);
            for (part, r) in &rects {
                let builder = UiBuilder::new()
                    .max_rect(*r)
                    .layout(egui::Layout::left_to_right(egui::Align::Center));
                let mut child = ui.new_child(builder);
                child.set_clip_rect(clip.intersect(child.clip_rect()));
                match *part {
                    Part::Line | Part::EndLine => {
                        crate::shell::vsep(&mut child);
                    }
                    Part::Item(i) => {
                        if forms[i] == Place::Cut {
                            continue;
                        }
                        let Some((_, item)) = items[i].as_mut() else { continue };
                        let name = item.name.clone();
                        let form = forms[i];
                        let enabled = item.enabled;
                        let is_open = open.as_deref() == Some(name.as_str());
                        let button = item.kit_button(form, is_open);
                        let kind = std::mem::replace(&mut item.kind, Kind::Separator);
                        match kind {
                            Kind::Button { toggle, .. } => {
                                let Some(b) = button else { continue };
                                let response = child.add(b);
                                if response.clicked() {
                                    out_clicked.push(name.clone());
                                    if let Some(on) = toggle {
                                        *on = !*on;
                                        changed = true;
                                    }
                                }
                                responses.push((name, response));
                            }
                            Kind::Popover { popover, .. } if form == Place::Unfolded => {
                                let p = popover.enabled(enabled).show_unfolded(&mut child);
                                changed |= p.changed;
                                out_clicked.extend(p.clicked);
                            }
                            Kind::Popover { popover, .. } => {
                                let Some(b) = button else { continue };
                                let response = child.add(b);
                                toggle_open(&mut open, &name, &response);
                                if open.as_deref() == Some(name.as_str()) {
                                    hung = Some((popover.enabled(enabled), response.rect));
                                }
                                responses.push((name, response));
                            }
                            Kind::Menu { add, .. } => {
                                let Some(b) = button else { continue };
                                let response = child.add(b);
                                toggle_open(&mut open, &name, &response);
                                if open.as_deref() == Some(name.as_str()) {
                                    let menu =
                                        Popover::new(name.as_str()).raw(0.0, move |ui, w| {
                                            menu_rows_in(ui, w, add);
                                        });
                                    hung = Some((menu, response.rect));
                                }
                                responses.push((name, response));
                            }
                            Kind::Custom { add, compact, .. } => {
                                if !enabled {
                                    child.disable();
                                }
                                match (form, compact) {
                                    (Place::Icon, Some((_, compact))) => compact(&mut child),
                                    _ => add(&mut child),
                                }
                            }
                            Kind::Joined { parts } => {
                                let count = parts.len();
                                child.spacing_mut().item_spacing.x = JOIN_SEAM;
                                for (k, part) in parts.into_iter().enumerate() {
                                    if part.words {
                                        paint_join_words(&mut child, &part.name, enabled);
                                        continue;
                                    }
                                    let b = part.kit_button(k == 0, k + 1 == count, enabled);
                                    let response = child.add(b);
                                    if response.clicked() {
                                        out_clicked.push(part.name.clone());
                                        if let Some(on) = part.toggle {
                                            *on = !*on;
                                            changed = true;
                                        }
                                    }
                                    responses.push((part.name, response));
                                }
                            }
                            Kind::Separator => {}
                        }
                    }
                    Part::More => {
                        let is_open = open.as_deref() == Some(MORE);
                        let button = match more_words {
                            true => more_button().opens(is_open),
                            false => more_button().open_state(is_open),
                        };
                        let response = child.add(button);
                        toggle_open(&mut open, MORE, &response);
                        more_rect = Some(response.rect);
                        responses.push((MORE.to_string(), response));
                    }
                    Part::End => {
                        if let Some(end) = end.take() {
                            end_out = Some(end(&mut child));
                        }
                        end_rect = Some(*r);
                    }
                    Part::Spacer => {}
                }
            }
        });
        // (The row's scope put the cursor after what its items took, which
        // stand centered in the row: back after the whole row, so that
        // what follows the header begins under it.)
        ui.advance_cursor_after_rect(rect);

        // The More popover: what is in it, in the row's order.
        if open.as_deref() == Some(MORE)
            && let Some(anchor) = more_rect
        {
            let mut more = Popover::new(more_name(&self.name)).hang(Hang::BelowRight);
            let mut last_side = None;
            for (i, slot) in items.iter_mut().enumerate() {
                if forms[i] != Place::More {
                    continue;
                }
                let Some((side, item)) = slot.take() else { continue };
                if last_side.is_some_and(|s| s != side) {
                    more = more.separator();
                }
                last_side = Some(side);
                let Item { name, kind, enabled, .. } = item;
                let icon = match &kind {
                    Kind::Button { icon, .. }
                    | Kind::Popover { icon, .. }
                    | Kind::Menu { icon, .. } => *icon,
                    _ => None,
                };
                let (clicked, changed) = (&more_clicked, &more_changed);
                more = match kind {
                    Kind::Button { toggle: Some(on), .. } => {
                        let w = popover::plain_width(&measure, icon, &name, true);
                        more.raw(w, move |ui, width| {
                            if popover::plain_check(ui, width, on, icon, &name, enabled).clicked() {
                                *on = !*on;
                                changed.set(true);
                                clicked.borrow_mut().push(name);
                            }
                        })
                    }
                    Kind::Button { toggle: None, .. } => {
                        let w = popover::plain_width(&measure, icon, &name, false);
                        more.raw(w, move |ui, width| {
                            if popover::plain_row(ui, width, icon, &name, enabled).clicked() {
                                clicked.borrow_mut().push(name);
                                ui.close();
                            }
                        })
                    }
                    Kind::Popover { popover, .. } => more.append(&name, popover.enabled(enabled)),
                    Kind::Menu { add, .. } => more.heading(name).raw(0.0, move |ui, w| {
                        ui.add_enabled_ui(enabled, |ui| menu_rows_in(ui, w, add));
                    }),
                    Kind::Custom { more: Some((w, add)), .. } => more.raw(w, move |ui, width| {
                        ui.allocate_ui_with_layout(
                            vec2(width, 0.0),
                            egui::Layout::top_down(egui::Align::Min),
                            |ui| {
                                ui.set_width(width);
                                if !enabled {
                                    ui.disable();
                                }
                                add(ui)
                            },
                        );
                    }),
                    Kind::Joined { parts } => {
                        let mut more = more.heading(name);
                        for part in parts.into_iter().filter(|p| !p.words) {
                            let Joined { icon, name, toggle, enabled: on_too, .. } = part;
                            let enabled = enabled && on_too;
                            let icon = Some(icon);
                            more = match toggle {
                                Some(on) => {
                                    let w = popover::plain_width(&measure, icon, &name, true);
                                    more.raw(w, move |ui, width| {
                                        let r = popover::plain_check(
                                            ui, width, on, icon, &name, enabled,
                                        );
                                        if r.clicked() {
                                            *on = !*on;
                                            changed.set(true);
                                            clicked.borrow_mut().push(name);
                                        }
                                    })
                                }
                                None => {
                                    let w = popover::plain_width(&measure, icon, &name, false);
                                    more.raw(w, move |ui, width| {
                                        let r = popover::plain_row(ui, width, icon, &name, enabled);
                                        if r.clicked() {
                                            clicked.borrow_mut().push(name);
                                            ui.close();
                                        }
                                    })
                                }
                            };
                        }
                        more
                    }
                    _ => more,
                };
            }
            hung = Some((more, anchor));
        }
        let mut popover_rect = None;
        if let Some((popover, anchor)) = hung {
            let name = open.clone().unwrap_or_default();
            let layer = popover_id(&name);
            if keyboard_opened {
                Popover::focus_in(ui.ctx(), layer);
            }
            let mut is_open = true;
            let p = popover.show_hung(ui.ctx(), layer, anchor, rect, &mut is_open);
            changed |= p.changed;
            out_clicked.extend(p.clicked);
            popover_rect = p.rect;
            if !is_open {
                open = None;
            }
        } else if open.is_some() {
            // (Its item is gone from the row, or yielded into More.)
            open = None;
        }
        // (Every way a popover closes, or gives way to another, forgets
        // what it kept open: measured again when it opens next.)
        if let Some(name) = was_open.as_deref().or(kept.name.as_deref())
            && open.as_deref() != Some(name)
        {
            Popover::forget(ui.ctx(), popover_id(name));
        }
        out_clicked.extend(more_clicked.take());
        changed |= more_changed.get();
        ui.data_mut(|d| d.insert_temp(open_id, Open { name: open.clone(), pass }));

        // A right click anywhere on the row, over its items too.
        let secondary_clicked =
            ui.input(|i| {
                i.pointer.button_clicked(egui::PointerButton::Secondary)
                    && i.pointer.interact_pos().is_some_and(|p| rect.contains(p))
            }) && ui.ctx().layer_id_at(ui.input(|i| i.pointer.interact_pos()).unwrap_or_default())
                == Some(ui.layer_id());

        let places = if recording {
            let mut places = Vec::new();
            for (((name, form), rows), joined) in
                names.into_iter().zip(forms.iter().copied()).zip(rows_of).zip(joined_items)
            {
                let Some(name) = name else { continue };
                places.push((name, form));
                if joined {
                    // (A joined group's buttons go where it goes.)
                    places.extend(rows.into_iter().map(|row| (row, form)));
                } else if form == Place::Unfolded {
                    places.extend(rows.into_iter().map(|row| (row, Place::Unfolded)));
                }
            }
            keep_record(
                ui.ctx(),
                &self.name,
                pass,
                HeaderRecord {
                    id,
                    row: rect,
                    end: end_rect,
                    more: more_rect,
                    popover: popover_rect,
                    open: open.clone(),
                    places: places.clone(),
                },
            );
            places
        } else {
            Vec::new()
        };
        let end = match (end_out, end.take()) {
            (Some(out), _) => out,
            // (No end slot: the closure has nothing to show.)
            (None, Some(end)) => {
                let nowhere = Rect::from_min_size(rect.right_top(), egui::Vec2::ZERO);
                end(&mut ui.new_child(UiBuilder::new().max_rect(nowhere).invisible()))
            }
            (None, None) => unreachable!("the end slot is drawn once"),
        };
        HeaderOutput {
            response,
            secondary_clicked,
            responses,
            places,
            clicked: out_clicked,
            changed,
            open,
            end_rect,
            popover_rect,
            end,
        }
    }
}

/// A menu's rows (`add`) laid out as a menu's are, `width` wide; a click
/// on one of its enabled rows closes what holds them (a heading's or a
/// disabled row's does not).
fn menu_rows_in(ui: &mut Ui, width: f32, add: AddUi<'_>) {
    let layout = egui::Layout::top_down_justified(egui::Align::Min);
    let inner = ui.allocate_ui_with_layout(vec2(width, 0.0), layout, |ui| {
        if !ui.is_sizing_pass() {
            ui.set_width(width);
        }
        crate::menu_rows(ui);
        add(ui);
    });
    let rect = inner.response.rect;
    // (Not on the pass that sizes it, the click that opened it: the
    // widget clicked this pass, where it is an enabled one of these rows.)
    if ui.is_sizing_pass() {
        return;
    }
    let ctx = ui.ctx().clone();
    let clicked = ctx.viewport(|v| v.interact_widgets.clicked);
    let on_a_row = clicked.and_then(|id| ctx.read_response(id)).is_some_and(|r| {
        // (One of the rows, a button: inside them, not the popover around
        // them; sensing a click and not a drag, as a button does and a
        // label, whose text is selected by a drag, does not.)
        r.enabled()
            && r.sense.senses_click()
            && !r.sense.senses_drag()
            && rect.expand(0.5).contains_rect(r.rect)
    });
    if on_a_row {
        ui.close();
    }
}
