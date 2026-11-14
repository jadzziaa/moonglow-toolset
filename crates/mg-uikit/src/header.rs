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
//!    folds back to its button, from the right group's end;
//! 2. the left group's word buttons lose their words (an icon with a
//!    tooltip), from its end to its start, then the right group's;
//! 3. they move into More, in the same order, so that secondary commands
//!    go before the first ones (Save); then the right group's.
//!
//! The end slot never yields and is never covered. What does not fit
//! even so (items that may not yield, [`Yield::Never`], in a row too
//! narrow) is cut, not drawn over the More button or the end slot, and
//! recorded so.
//!
//! An editor whose forms change at widths of their own (the area's
//! compact header under an editor 990 px wide, [`crate::shell::Yields`])
//! chooses the items it gives by them, and forces an item into More with
//! [`Item::in_more`] or keeps one on the row with [`Item::yields`].
//!
//! The header records where each item went ([`Place`]: on the row with
//! its words, as an icon, unfolded, in More, or cut), in its output and
//! under its name for tests ([`places`]), so that an editor's test can
//! narrow it to its least width and find every control on the row or in
//! More.
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
//!     .left(Item::button(Icon::Save, "Save").enabled(false))
//!     .left(Item::button(Icon::Compile, "Compile"))
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

use egui::{Id, Rect, Response, Sense, Stroke, Ui, UiBuilder, pos2, vec2};

use crate::button::{ButtonLook, KitButton};
use crate::icons::Icon;
use crate::metrics;
use crate::popover::{self, Hang, Popover};
use crate::theme::TokensExt as _;

/// The header's padding at each end (`.editor-header`: `padding: 0 6px`).
pub const HEADER_PADDING: f32 = 6.0;
/// The gap between the header's items (`.editor-header`: `gap: 4px`).
pub const HEADER_GAP: f32 = 4.0;
/// The More button's accessible name.
pub const MORE: &str = "More";

/// Where an item of an [`EditorHeader`] went.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Place {
    /// On the row as given: with its words (an item that has words), or
    /// as it is (an icon button, a field, a separator).
    Row,
    /// On the row, its icon alone (its words its tooltip and name).
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
    /// Its words (it has an icon), never its place on the row.
    Icon,
    /// Its words, then its place (into More).
    Full,
    /// Its place, straight into More (an icon alone, words alone).
    More,
}

type AddUi<'a> = Box<dyn FnOnce(&mut Ui) + 'a>;

enum Kind<'a> {
    Button { icon: Option<Icon>, look: ButtonLook, toggle: Option<&'a mut bool>, icon_only: bool },
    Popover { icon: Option<Icon>, popover: Popover<'a>, unfolds: bool },
    Menu { icon: Option<Icon>, add: AddUi<'a> },
    Custom { width: f32, add: AddUi<'a>, more: Option<(f32, AddUi<'a>)> },
    Separator,
}

/// An item of an [`EditorHeader`]: a button, a toggle, a popover's or a
/// menu's button, a part of the caller's (a field, a combo box, a
/// segmented control, a note) given its width, or a separator.
#[must_use = "add it with EditorHeader::left or ::right"]
pub struct Item<'a> {
    name: String,
    kind: Kind<'a>,
    yields: Yield,
    in_more: bool,
    enabled: bool,
    tip: Option<String>,
    words_icon: bool,
    force_icon: bool,
}

impl std::fmt::Debug for Item<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let kind = match &self.kind {
            Kind::Button { .. } => "button",
            Kind::Popover { .. } => "popover",
            Kind::Menu { .. } => "menu",
            Kind::Custom { .. } => "custom",
            Kind::Separator => "separator",
        };
        f.debug_struct("Item")
            .field("name", &self.name)
            .field("kind", &kind)
            .field("yields", &self.yields)
            .field("in_more", &self.in_more)
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
            enabled: true,
            tip: None,
            words_icon: true,
            force_icon: false,
        }
    }

    /// A word button with its icon (`.btn`): Save, Compile, Add Row. It
    /// loses its words, then moves into More.
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

    /// An icon button (`.btn-icon`), named and tipped `name` (Print…,
    /// External Editor): it moves into More whole, as a row with its
    /// words.
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
    /// and the chevron; in More, its rows under a heading of its name.
    pub fn popover(icon: Option<Icon>, popover: Popover<'a>) -> Self {
        let yields = if icon.is_some() { Yield::Full } else { Yield::More };
        let name = popover.name().to_string();
        let mut item = Self::new(name, Kind::Popover { icon, popover, unfolds: false }, yields);
        item.words_icon = false;
        item
    }

    /// A menu's button (`.btn` with a chevron: Export) and `add`, the
    /// menu's rows, in a popover hung from it that a click in it closes.
    /// In More, `add` under a heading of its words.
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
    /// control, a note). It never yields, unless it is given a form for
    /// More ([`Item::more`]).
    pub fn custom(name: impl Into<String>, width: f32, add: impl FnOnce(&mut Ui) + 'a) -> Self {
        let kind = Kind::Custom { width, add: Box::new(add), more: None };
        Self::new(name.into(), kind, Yield::Never)
    }

    /// A line between groups of the row (`.vsep`); one that would stand
    /// at a group's start or end or beside another, its items gone into
    /// More, is left out, but for one at the right group's end where the
    /// More button follows it (the Script board's codepage, a line, More).
    pub fn separator() -> Self {
        Self::new(String::new(), Kind::Separator, Yield::Never)
    }

    /// A custom item's form in More: `add`, given a popover's width
    /// (laid out top down; at least `width` wide), in place of it on the
    /// row. With it the item moves into More where the row is too
    /// narrow.
    pub fn more(mut self, width: f32, add: impl FnOnce(&mut Ui) + 'a) -> Self {
        if let Kind::Custom { more, .. } = &mut self.kind {
            *more = Some((width, Box::new(add)));
            self.yields = Yield::More;
        }
        self
    }

    /// Always in More, whatever the width (To Scratch).
    pub fn in_more(mut self) -> Self {
        self.in_more = true;
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
    /// said disabled, on the row and in More.
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

    /// Whether a popover's or menu's button shows its icon beside its
    /// words (the Talk board's Export does; Lighting, View and More do
    /// not, the default); a button's always does.
    pub fn words_icon(mut self, shown: bool) -> Self {
        self.words_icon = shown;
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
                let mut b = KitButton::new(shown_icon, self.name.as_str())
                    .look(*look)
                    .icon_only(*icon_only || icon_form);
                if let Some(on) = toggle {
                    b = b.toggle(**on);
                }
                b
            }
            Kind::Popover { icon, .. } | Kind::Menu { icon, .. } => {
                let shown_icon = if self.words_icon || icon_form { *icon } else { None };
                let look = if icon_form { ButtonLook::Flat } else { ButtonLook::Raised };
                KitButton::new(shown_icon, self.name.as_str())
                    .look(look)
                    .icon_only(icon_form)
                    .opens(open)
            }
            _ => return None,
        };
        b = b.enabled(self.enabled);
        if let Some(tip) = &self.tip {
            b = b.tip(tip.as_str());
        }
        Some(b)
    }

    /// Its width on the row in `form` (where it has that form).
    fn width(&self, ui: &Ui, form: Place) -> Option<f32> {
        match (&self.kind, form) {
            (Kind::Separator, Place::Row) => Some(crate::shell::VSEP_WIDTH),
            (Kind::Custom { width, .. }, Place::Row) => Some(*width),
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
    /// The row's response (its whole rectangle): it senses a click and a
    /// right click (the header's own menu, `response.context_menu`) but
    /// takes no keyboard focus, so Tab goes from item to item.
    pub response: Response,
    /// Where each item went, by its name, in the row's order (separators
    /// left out); after a popover unfolded into the row, its switch's and
    /// rows' names, [`Place::Unfolded`] too.
    pub places: Vec<(String, Place)>,
    /// The names of the buttons clicked (on the row or in More) and of
    /// the action rows chosen in its popovers.
    pub clicked: Vec<String>,
    /// A toggle, a check row or a switch changed.
    pub changed: bool,
    /// Where the end slot was drawn (never covered).
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
}

/// The id under which the places of the header named `name` are kept.
fn record_id(name: &str) -> Id {
    Id::new(("mg-uikit-header-places", name))
}

/// Where each item of the header named `name` went in its last frame
/// ([`HeaderOutput::places`]; none if it was not drawn). Where two
/// headers of one name are drawn in a frame, the last one's.
pub fn places(ctx: &egui::Context, name: &str) -> Vec<(String, Place)> {
    ctx.data(|d| d.get_temp(record_id(name))).unwrap_or_default()
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
    Spacer,
    More,
    EndLine,
    End,
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

        // The widths of each item's forms, and its first form.
        let n = self.items.len();
        // (Its items' names, separators none, for the record.)
        let names: Vec<Option<String>> = (self.items.iter())
            .map(|(_, item)| match item.kind {
                Kind::Separator => None,
                _ => Some(item.name.clone()),
            })
            .collect();
        // (A popover's rows, recorded where they unfold into the row.)
        let rows_of: Vec<Vec<String>> = (self.items.iter())
            .map(|(_, item)| match &item.kind {
                Kind::Popover { popover, .. } => popover.check_names(),
                _ => Vec::new(),
            })
            .collect();
        let widths: Vec<[Option<f32>; 3]> = (self.items.iter())
            .map(|(_, item)| {
                [Place::Unfolded, Place::Row, Place::Icon].map(|form| item.width(ui, form))
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
        let sides = || order(Side::Left).into_iter().chain(order(Side::Right));
        for i in sides() {
            let item = &self.items[i].1;
            let words = widths[i][1].is_some() && !matches!(item.kind, Kind::Custom { .. });
            if !item.in_more
                && words
                && widths[i][2].is_some()
                && matches!(item.yields, Yield::Icon | Yield::Full)
            {
                steps.push((i, Place::Icon));
            }
        }
        for i in sides() {
            let item = &self.items[i].1;
            if !item.in_more && item.may_go_into_more() {
                steps.push((i, Place::More));
            }
        }

        let more_words = self.more_words;
        let more_button = move || {
            let b = KitButton::new(Some(Icon::MoreHorizontal), MORE);
            match more_words {
                true => KitButton::new(None, MORE).opens(false),
                false => b.icon_only(true).look(ButtonLook::Flat),
            }
        };
        let more_width = more_button().width(ui);
        let room = rect.width() - 2.0 * HEADER_PADDING;
        let end_line = has_end && self.end_separator;

        // The row's parts for the forms (the separators that stand
        // between items alone), and its width.
        let parts_of = |forms: &[Place], more_at_end: bool| -> Vec<Part> {
            let any_more = forms.contains(&Place::More);
            let more_after = self.more_after.as_ref().and_then(|name| {
                (0..n).find(|&i| self.items[i].1.name == *name && forms[i] != Place::More)
            });
            let mut parts = Vec::new();
            for side in [Side::Left, Side::Right] {
                if side == Side::Right {
                    parts.push(Part::Spacer);
                }
                let mut group: Vec<Part> = Vec::new();
                let mut pending_line = false;
                for i in (0..n).filter(|&i| self.items[i].0 == side) {
                    if forms[i] == Place::More {
                        continue;
                    }
                    if matches!(self.items[i].1.kind, Kind::Separator) {
                        pending_line = !group.is_empty();
                        continue;
                    }
                    if pending_line {
                        group.push(Part::Item(usize::MAX));
                        pending_line = false;
                    }
                    group.push(Part::Item(i));
                    if any_more && !more_at_end && more_after == Some(i) {
                        group.push(Part::More);
                    }
                }
                // (A line at the right group's end stands before More
                // where More follows it: the Script board's codepage | ⋯.)
                let more_here = any_more && (more_at_end || more_after.is_none());
                if side == Side::Right && pending_line && more_here {
                    group.push(Part::Item(usize::MAX));
                }
                parts.extend(group);
            }
            if any_more && (more_at_end || more_after.is_none()) {
                parts.push(Part::More);
            }
            if end_line {
                parts.push(Part::EndLine);
            }
            if has_end {
                parts.push(Part::End);
            }
            parts
        };
        let part_width = |part: Part, forms: &[Place]| -> f32 {
            match part {
                Part::Item(usize::MAX) => crate::shell::VSEP_WIDTH,
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
                Part::EndLine => crate::shell::VSEP_WIDTH,
                Part::End => end_width,
            }
        };
        let total = |parts: &[Part], forms: &[Place]| -> f32 {
            let sum: f32 = parts.iter().map(|&p| part_width(p, forms)).sum();
            sum + HEADER_GAP * parts.len().saturating_sub(1) as f32
        };
        let mut fits = total(&parts_of(&forms, false), &forms) <= room;
        for &(i, to) in &steps {
            if fits {
                break;
            }
            forms[i] = to;
            fits = total(&parts_of(&forms, false), &forms) <= room;
        }
        let parts = parts_of(&forms, !fits);

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
                    && i != usize::MAX
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

        // Draw, in the row's order.
        let open_id = id.with("open");
        let mut open: Option<String> = ui.data(|d| d.get_temp(open_id)).unwrap_or(None);
        let mut out_clicked: Vec<String> = Vec::new();
        let mut changed = false;
        let mut end_out = None;
        let mut end_rect = None;
        let more_clicked = RefCell::new(Vec::<String>::new());
        let more_changed = std::cell::Cell::new(false);
        let mut hung: Option<(Popover<'_>, Rect, bool)> = None;
        let mut more_rect = None;
        let mut items: Vec<Option<(Side, Item<'a>)>> = self.items.into_iter().map(Some).collect();
        let mut end = Some(end);
        let clip = rect;
        let row = UiBuilder::new()
            .max_rect(rect)
            .layout(egui::Layout::left_to_right(egui::Align::Center));
        ui.scope_builder(row, |ui| {
            crate::icons::name_group(ui, &self.name);
            for (part, r) in &rects {
                let builder = UiBuilder::new()
                    .max_rect(*r)
                    .layout(egui::Layout::left_to_right(egui::Align::Center));
                match *part {
                    Part::Item(usize::MAX) | Part::EndLine => {
                        ui.scope_builder(builder, crate::shell::vsep);
                    }
                    Part::Item(i) => {
                        if forms[i] == Place::Cut {
                            continue;
                        }
                        let Some((_, item)) = items[i].as_mut() else { continue };
                        let name = item.name.clone();
                        let form = forms[i];
                        let is_open = open.as_deref() == Some(name.as_str());
                        let mut child = ui.new_child(builder);
                        child.set_clip_rect(clip);
                        let button = item.kit_button(form, is_open);
                        let kind = std::mem::replace(&mut item.kind, Kind::Separator);
                        match kind {
                            Kind::Button { toggle, .. } => {
                                let Some(b) = button else { continue };
                                if child.add(b).clicked() {
                                    out_clicked.push(name);
                                    if let Some(on) = toggle {
                                        *on = !*on;
                                        changed = true;
                                    }
                                }
                            }
                            Kind::Popover { popover, .. } if form == Place::Unfolded => {
                                let p = popover.show_unfolded(&mut child);
                                changed |= p.changed;
                                out_clicked.extend(p.clicked);
                            }
                            Kind::Popover { popover, .. } => {
                                let Some(b) = button else { continue };
                                let response = child.add(b);
                                if response.clicked() {
                                    open = if is_open { None } else { Some(name.clone()) };
                                }
                                if open.as_deref() == Some(name.as_str()) {
                                    hung = Some((popover, response.rect, false));
                                }
                            }
                            Kind::Menu { add, .. } => {
                                let Some(b) = button else { continue };
                                let response = child.add(b);
                                if response.clicked() {
                                    open = if is_open { None } else { Some(name.clone()) };
                                }
                                if open.as_deref() == Some(name.as_str()) {
                                    let menu =
                                        Popover::new(name.as_str()).raw(0.0, move |ui, w| {
                                            menu_rows_in(ui, w, add);
                                        });
                                    hung = Some((menu, response.rect, false));
                                }
                            }
                            Kind::Custom { add, .. } => add(&mut child),
                            Kind::Separator => {}
                        }
                    }
                    Part::More => {
                        let is_open = open.as_deref() == Some(MORE);
                        let mut child = ui.new_child(builder);
                        let button = match more_words {
                            true => more_button().opens(is_open),
                            false => more_button().open_state(is_open),
                        };
                        let response = child.add(button);
                        if response.clicked() {
                            open = if is_open { None } else { Some(MORE.to_string()) };
                        }
                        more_rect = Some(response.rect);
                    }
                    Part::End => {
                        let mut child = ui.new_child(builder);
                        if let Some(end) = end.take() {
                            end_out = Some(end(&mut child));
                        }
                        end_rect = Some(*r);
                    }
                    Part::Spacer => {}
                }
            }
        });

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
                        let w = popover::plain_width(ui, icon, &name, true);
                        more.raw(w, move |ui, width| {
                            if popover::plain_check(ui, width, on, icon, &name, enabled).clicked() {
                                *on = !*on;
                                changed.set(true);
                                clicked.borrow_mut().push(name);
                            }
                        })
                    }
                    Kind::Button { toggle: None, .. } => {
                        let w = popover::plain_width(ui, icon, &name, false);
                        more.raw(w, move |ui, width| {
                            if popover::plain_row(ui, width, icon, &name, enabled).clicked() {
                                clicked.borrow_mut().push(name);
                                ui.close();
                            }
                        })
                    }
                    Kind::Popover { popover, .. } => more.append(&name, popover),
                    Kind::Menu { add, .. } => more.heading(name).raw(0.0, move |ui, w| {
                        menu_rows_in(ui, w, add);
                    }),
                    Kind::Custom { more: Some((w, add)), .. } => more.raw(w, move |ui, width| {
                        ui.allocate_ui_with_layout(
                            vec2(width, 0.0),
                            egui::Layout::top_down(egui::Align::Min),
                            |ui| {
                                ui.set_width(width);
                                add(ui)
                            },
                        );
                    }),
                    _ => more,
                };
            }
            hung = Some((more, anchor, true));
        }
        let mut popover_rect = None;
        if let Some((popover, anchor, _)) = hung {
            let name = open.clone().unwrap_or_default();
            let mut is_open = true;
            let p = popover.show_hung(
                ui.ctx(),
                id.with(("popover", &name)),
                anchor,
                rect,
                &mut is_open,
            );
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
        out_clicked.extend(more_clicked.take());
        changed |= more_changed.get();
        ui.data_mut(|d| d.insert_temp(open_id, open));

        let mut places: Vec<(String, Place)> = Vec::new();
        for ((name, form), rows) in names.into_iter().zip(forms).zip(rows_of) {
            let Some(name) = name else { continue };
            places.push((name, form));
            if form == Place::Unfolded {
                places.extend(rows.into_iter().map(|row| (row, Place::Unfolded)));
            }
        }
        ui.data_mut(|d| d.insert_temp(record_id(&self.name), places.clone()));
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
            places,
            clicked: out_clicked,
            changed,
            end_rect,
            popover_rect,
            end,
        }
    }
}

/// A menu's rows (`add`) laid out as a menu's are, `width` wide; a click
/// in them closes what holds them.
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
    // (Not on the pass that sizes it: the click that opened it.)
    let clicked = !ui.is_sizing_pass()
        && ui.input(|i| {
            i.pointer.any_click() && i.pointer.interact_pos().is_some_and(|p| rect.contains(p))
        });
    if clicked {
        ui.close();
    }
}
