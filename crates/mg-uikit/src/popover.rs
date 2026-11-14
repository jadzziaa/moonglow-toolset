//! A popover (proposal sections 6 and 8.1; the mock's `.popover`): a
//! panel hung from its button, below it with their left edges aligned or
//! below-right with their right edges aligned, so that a popover right of
//! the middle does not cover the view's top-right overlays; nothing
//! clips it (it is a layer of its own). It holds, from the top:
//!
//! - a title (`.pop-title`: 28 px, its icon in `text-2`, its words, a
//!   line under it), with a master switch at its right where it has one
//!   (`.switch`, 26 by 14): while the switch is off its rows are drawn and
//!   said disabled ([`Popover::rows_follow_switch`]);
//! - sub headings (`.pop-h`), check rows (`.pop-row.check`: 24 px, 10 px
//!   in, a box, an optional icon, the words; `.sub` 32 px in; lit
//!   `hover-row` under the pointer), action rows (an icon, the words and
//!   a weak note at the right end), separators (`.pop-sep`) and rows of
//!   the caller's own (a segmented control, a combo box);
//!
//! and is described once and shown three ways: hung from its button
//! ([`Popover::show`], [`Popover::show_hung`]), in place as a catalog
//! draws it ([`Popover::show_in_place`]), and unfolded into an editor
//! header's row as toggles where the row has room (section 5.3;
//! [`Popover::show_unfolded`], measured before by
//! [`Popover::unfolded_width`]): a check row becomes a toggle, an action
//! a word button, a separator a line, the master switch a toggle with the
//! title's icon, and a row of the caller's its unfolded form (a popover
//! with a row that has none does not unfold, [`Popover::can_unfold`]).
//!
//! A hung popover closes on a click outside it and its button, on
//! Escape, and when an action row is chosen; a check row leaves it open.
//! It is the kit's own layer, not an egui popup, so that a combo box's
//! list opened from inside it (egui keeps one popup open at a time)
//! leaves it open: while any egui popup is open, a click outside and
//! Escape are that popup's.
//!
//! Accessible names: the popover is a group named by its name; a check
//! row is a check box named by its words, the switch one named by its
//! own name ("Show overlays"); unfolded, the same names on toggles.

use egui::{
    Align2, Color32, FontId, Id, Margin, Rect, Response, Sense, Stroke, Ui, UiBuilder, Vec2,
    WidgetInfo, WidgetType, pos2, vec2,
};

use crate::button::KitButton;
use crate::controls::{self, BOX, Look};
use crate::icons::Icon;
use crate::metrics;
use crate::theme::TokensExt as _;

/// A popover's least width (`.popover`: `min-width: 220px`).
pub const POPOVER_MIN_WIDTH: f32 = 220.0;
/// The room between a popover and its button (`top: calc(100% + 4px)`).
pub const POPOVER_GAP: f32 = 4.0;
/// A popover title's height (`.pop-title`: `height: 28px`), its line
/// included.
pub const TITLE_HEIGHT: f32 = 28.0;
/// The room under a title's line (`.pop-title`: `margin-bottom: 4px`).
pub const TITLE_MARGIN: f32 = 4.0;
/// A row's padding at each side (`.pop-row`, `.pop-title`: `0 10px`).
pub const ROW_PADDING: f32 = 10.0;
/// The gap between a row's parts (`.pop-row`: `gap: 8px`).
pub const ROW_GAP: f32 = 8.0;
/// A sub row's room at its left (`.pop-row.sub`: `padding-left: 32px`).
pub const SUB_INDENT: f32 = 32.0;
/// The room before a row's note at its right end (`.pop-row .end`:
/// `padding-left: 16px`).
pub const END_GAP: f32 = 16.0;
/// The master switch's size (`.switch`: 26 by 14, its knob 10 and 2 px
/// in).
pub const SWITCH: Vec2 = vec2(26.0, 14.0);
/// The gap between an unfolded popover's parts in a header (the header's
/// own, `.editor-header`: `gap: 4px`).
pub const UNFOLDED_GAP: f32 = 4.0;

/// Where a popover hangs from its button.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Hang {
    /// Below it, their left edges aligned (`.popover.below`).
    Below,
    /// Below it, their right edges aligned, opening leftward
    /// (`.popover.below-right`): it never reaches right of its button.
    BelowRight,
    /// Below where the button is left of the middle of what holds it (the
    /// header, or the window), below-right where it is right of it.
    #[default]
    Auto,
}

impl Hang {
    /// The hang `self` comes to for a button at `anchor` in `frame`.
    pub fn resolved(self, anchor: Rect, frame: Rect) -> Hang {
        match self {
            Hang::Auto if anchor.center().x > frame.center().x => Hang::BelowRight,
            Hang::Auto => Hang::Below,
            hang => hang,
        }
    }

    /// The popover's pivot and the point it is pinned to, for a button at
    /// `anchor` (resolved).
    fn pin(self, anchor: Rect) -> (Align2, egui::Pos2) {
        match self {
            Hang::BelowRight => (Align2::RIGHT_TOP, anchor.right_bottom() + vec2(0.0, POPOVER_GAP)),
            _ => (Align2::LEFT_TOP, anchor.left_bottom() + vec2(0.0, POPOVER_GAP)),
        }
    }
}

/// A check row of a [`Popover`] (`.pop-row.check`).
#[derive(Debug)]
#[must_use = "add it with Popover::check"]
pub struct PopCheck<'a> {
    value: &'a mut bool,
    words: String,
    icon: Option<Icon>,
    tip: Option<String>,
    sub: bool,
    enabled: bool,
    unfolded_words: bool,
}

impl<'a> PopCheck<'a> {
    /// A row checking `value`, reading `words` (its accessible name).
    pub fn new(value: &'a mut bool, words: impl Into<String>) -> Self {
        let words = words.into();
        PopCheck {
            value,
            words,
            icon: None,
            tip: None,
            sub: false,
            enabled: true,
            unfolded_words: false,
        }
    }

    /// With `icon` after its box (`.ic.lead`, `text-2`); unfolded, the
    /// toggle's icon.
    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Its tooltip.
    pub fn tip(mut self, tip: impl Into<String>) -> Self {
        self.tip = Some(tip.into());
        self
    }

    /// A sub row, 32 px in (`.pop-row.sub`): an option of the row above.
    pub fn sub(mut self) -> Self {
        self.sub = true;
        self
    }

    /// Enabled (`true`, the default) or drawn and said disabled.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Unfolded into a header, its toggle shows its words beside its icon
    /// (section 5.3: Grid, Minimap carry their words; Grass is an icon).
    /// A row without an icon always shows its words.
    pub fn unfolded_words(mut self, words: bool) -> Self {
        self.unfolded_words = words;
        self
    }

    fn toggle(&self, enabled: bool) -> KitButton {
        let icon_only = !self.unfolded_words && self.icon.is_some();
        let mut b = KitButton::new(self.icon, self.words.as_str())
            .toggle(*self.value)
            .icon_only(icon_only)
            .enabled(enabled);
        if let Some(tip) = &self.tip {
            b = b.tip(tip.as_str());
        }
        b
    }
}

/// An action row of a [`Popover`] (`.pop-row` with an icon and an
/// `.end` note): chosen, it closes the popover, and the popover's output
/// names it ([`PopoverOutput::clicked`]).
#[derive(Clone, Debug)]
#[must_use = "add it with Popover::action"]
pub struct PopAction {
    words: String,
    icon: Option<Icon>,
    end: Option<String>,
    tip: Option<String>,
    sub: bool,
    enabled: bool,
}

impl PopAction {
    /// A row reading `words` (its accessible name, and the name the
    /// output gives when it is chosen).
    pub fn new(words: impl Into<String>) -> Self {
        let words = words.into();
        PopAction { words, icon: None, end: None, tip: None, sub: false, enabled: true }
    }

    /// With `icon` before its words (`text-2`).
    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    /// With `note` at its right end, weak (`.end`, `text-3`): its keys, a
    /// mode.
    pub fn end(mut self, note: impl Into<String>) -> Self {
        self.end = Some(note.into());
        self
    }

    /// Its tooltip.
    pub fn tip(mut self, tip: impl Into<String>) -> Self {
        self.tip = Some(tip.into());
        self
    }

    /// A sub row, 32 px in.
    pub fn sub(mut self) -> Self {
        self.sub = true;
        self
    }

    /// Enabled (`true`, the default) or drawn and said disabled.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    fn button(&self, enabled: bool) -> KitButton {
        let mut b = KitButton::new(self.icon, self.words.as_str()).enabled(enabled);
        if let Some(tip) = &self.tip {
            b = b.tip(tip.as_str());
        }
        b
    }
}

/// What a popover's row of the caller's draws: in the popover, and its
/// unfolded form with its width.
type AddUi<'a> = Box<dyn FnOnce(&mut Ui) + 'a>;
/// A row drawn across the popover's width (given).
type AddRow<'a> = Box<dyn FnOnce(&mut Ui, f32) + 'a>;

enum Row<'a> {
    Heading(String),
    Check(PopCheck<'a>),
    Action(PopAction),
    Separator,
    Custom {
        add: AddUi<'a>,
        unfolded: Option<(f32, AddUi<'a>)>,
    },
    /// The kit's own (an editor header's More): given the popover's
    /// width, needing `needed`; never unfolds.
    Raw {
        needed: f32,
        add: AddRow<'a>,
    },
}

impl std::fmt::Debug for Row<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Row::Heading(h) => f.debug_tuple("Heading").field(h).finish(),
            Row::Check(c) => f.debug_tuple("Check").field(c).finish(),
            Row::Action(a) => f.debug_tuple("Action").field(a).finish(),
            Row::Separator => f.write_str("Separator"),
            Row::Custom { unfolded, .. } => {
                f.debug_struct("Custom").field("unfolds", &unfolded.is_some()).finish()
            }
            Row::Raw { needed, .. } => f.debug_struct("Raw").field("needed", needed).finish(),
        }
    }
}

/// What a popover did this frame.
#[derive(Clone, Debug, Default)]
pub struct PopoverOutput {
    /// It is open (after this frame: a click outside closed it).
    pub open: bool,
    /// Where it was drawn (hung or in place; unfolded, the row's part).
    pub rect: Option<Rect>,
    /// The words of the action rows chosen.
    pub clicked: Vec<String>,
    /// A check row, the switch or an action changed something.
    pub changed: bool,
}

impl PopoverOutput {
    /// Whether the action row reading `words` was chosen.
    pub fn clicked(&self, words: &str) -> bool {
        self.clicked.iter().any(|c| c == words)
    }
}

/// A popover (module comment): its title and switch, its rows, where it
/// hangs.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use mg_uikit::popover::{PopCheck, Popover};
/// use mg_uikit::{Icon, KitButton};
/// let (mut shown, mut grid, mut minimap) = (true, false, true);
/// let button = ui.add(KitButton::new(None, "Overlays").opens(false));
/// let out = Popover::new("Overlays")
///     .title(Some(Icon::Overlays), "Overlays")
///     .switch(&mut shown, "Show overlays")
///     .check(PopCheck::new(&mut grid, "Grid").icon(Icon::Grid))
///     .check(PopCheck::new(&mut minimap, "Minimap").icon(Icon::Minimap))
///     .show(ui, &button);
/// # let _ = out;
/// # });
/// ```
#[derive(Debug)]
#[must_use = "show it with .show(ui, &button)"]
pub struct Popover<'a> {
    name: String,
    title: Option<(Option<Icon>, String)>,
    switch: Option<(&'a mut bool, String)>,
    rows_follow_switch: bool,
    rows: Vec<Row<'a>>,
    hang: Hang,
}

impl<'a> Popover<'a> {
    /// A popover named `name` (its group's accessible name).
    pub fn new(name: impl Into<String>) -> Self {
        Popover {
            name: name.into(),
            title: None,
            switch: None,
            rows_follow_switch: true,
            rows: Vec::new(),
            hang: Hang::Auto,
        }
    }

    /// Its name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// With a title (`.pop-title`): `icon` (where given) and `words`.
    pub fn title(mut self, icon: Option<Icon>, words: impl Into<String>) -> Self {
        self.title = Some((icon, words.into()));
        self
    }

    /// With a master switch of `on` at its title's right (a title is made
    /// of the popover's name where none was given), named `name` ("Show
    /// overlays").
    pub fn switch(mut self, on: &'a mut bool, name: impl Into<String>) -> Self {
        if self.title.is_none() {
            self.title = Some((None, self.name.clone()));
        }
        self.switch = Some((on, name.into()));
        self
    }

    /// Whether its rows are disabled while its switch is off (`true`, the
    /// default): they keep their values, and come back as they were.
    pub fn rows_follow_switch(mut self, follow: bool) -> Self {
        self.rows_follow_switch = follow;
        self
    }

    /// Where it hangs ([`Hang::Auto`] where not given).
    pub fn hang(mut self, hang: Hang) -> Self {
        self.hang = hang;
        self
    }

    /// A sub heading (`.pop-h`).
    pub fn heading(mut self, words: impl Into<String>) -> Self {
        self.rows.push(Row::Heading(words.into()));
        self
    }

    /// A check row.
    pub fn check(mut self, check: PopCheck<'a>) -> Self {
        self.rows.push(Row::Check(check));
        self
    }

    /// An action row.
    pub fn action(mut self, action: PopAction) -> Self {
        self.rows.push(Row::Action(action));
        self
    }

    /// A separator (`.pop-sep`).
    pub fn separator(mut self) -> Self {
        self.rows.push(Row::Separator);
        self
    }

    /// A row of the caller's (laid out left to right, 10 px in, at least
    /// a control high): a popover with one does not unfold
    /// ([`Popover::custom_unfolded`] gives it a form for the row).
    pub fn custom(mut self, add: impl FnOnce(&mut Ui) + 'a) -> Self {
        self.rows.push(Row::Custom { add: Box::new(add), unfolded: None });
        self
    }

    /// A row of the caller's, `add` in the popover and `unfolded`, exactly
    /// `width` wide and a control high, in a header's row.
    pub fn custom_unfolded(
        mut self,
        add: impl FnOnce(&mut Ui) + 'a,
        width: f32,
        unfolded: impl FnOnce(&mut Ui) + 'a,
    ) -> Self {
        let unfolded: AddUi<'a> = Box::new(unfolded);
        self.rows.push(Row::Custom { add: Box::new(add), unfolded: Some((width, unfolded)) });
        self
    }

    /// A row of the kit's own, drawn across the popover's width (given
    /// to `add`), which needs `needed` (an editor header's More).
    pub(crate) fn raw(mut self, needed: f32, add: impl FnOnce(&mut Ui, f32) + 'a) -> Self {
        self.rows.push(Row::Raw { needed, add: Box::new(add) });
        self
    }

    /// Its rows moved after `self`'s, under a heading of `heading`: its
    /// switch as a check row, its rows disabled while that is off where
    /// they follow it (an editor header's popover moved into More).
    pub(crate) fn append(mut self, heading: &str, other: Popover<'a>) -> Self {
        let enabled = other.rows_enabled();
        self.rows.push(Row::Heading(heading.to_string()));
        if let Some((on, name)) = other.switch {
            self.rows.push(Row::Check(PopCheck::new(on, name)));
        }
        for row in other.rows {
            self.rows.push(match row {
                Row::Check(c) => {
                    let e = c.enabled && enabled;
                    Row::Check(c.enabled(e))
                }
                Row::Action(a) => {
                    let e = a.enabled && enabled;
                    Row::Action(a.enabled(e))
                }
                Row::Custom { add, .. } if !enabled => Row::Custom {
                    add: Box::new(move |ui: &mut Ui| {
                        ui.disable();
                        add(ui)
                    }),
                    unfolded: None,
                },
                row => row,
            });
        }
        self
    }

    /// Whether it has no rows (and no title).
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty() && self.title.is_none()
    }

    /// The words of its check rows (in order): the names its unfolded
    /// form and the popover give their toggles and boxes.
    pub fn check_names(&self) -> Vec<String> {
        let rows = self.rows.iter().filter_map(|r| match r {
            Row::Check(c) => Some(c.words.clone()),
            Row::Action(a) => Some(a.words.clone()),
            _ => None,
        });
        self.switch.iter().map(|(_, n)| n.clone()).chain(rows).collect()
    }

    /// Whether every row has a form for the header's row (rows of the
    /// caller's given one by [`Popover::custom_unfolded`]).
    pub fn can_unfold(&self) -> bool {
        self.rows.iter().all(|r| !matches!(r, Row::Custom { unfolded: None, .. } | Row::Raw { .. }))
    }

    fn rows_enabled(&self) -> bool {
        match &self.switch {
            Some((on, _)) => **on || !self.rows_follow_switch,
            None => true,
        }
    }

    /// The parts of its unfolded form, in order, each with its width; a
    /// separator only between parts.
    fn unfolded_parts(&self, ui: &Ui) -> Vec<(usize, f32)> {
        // (usize::MAX: the switch; usize::MAX - 1: a separator.)
        let mut parts = Vec::new();
        if let (Some((on, name)), Some((icon, _))) = (&self.switch, &self.title) {
            let b = KitButton::new(Some(icon.unwrap_or(Icon::Eye)), name.as_str()).toggle(**on);
            parts.push((usize::MAX, b.icon_only(true).width(ui)));
        }
        let enabled = self.rows_enabled();
        let mut pending_sep = false;
        for (i, row) in self.rows.iter().enumerate() {
            let width = match row {
                Row::Heading(_) => continue,
                Row::Separator => {
                    pending_sep = !parts.is_empty();
                    continue;
                }
                Row::Check(c) => c.toggle(enabled && c.enabled).width(ui),
                Row::Action(a) => a.button(enabled && a.enabled).width(ui),
                Row::Custom { unfolded: Some((w, _)), .. } => *w,
                Row::Custom { unfolded: None, .. } | Row::Raw { .. } => continue,
            };
            if pending_sep {
                parts.push((usize::MAX - 1, crate::shell::VSEP_WIDTH));
                pending_sep = false;
            }
            parts.push((i, width));
        }
        parts
    }

    /// Its unfolded form's width ([`Popover::show_unfolded`]).
    pub fn unfolded_width(&self, ui: &Ui) -> f32 {
        let parts = self.unfolded_parts(ui);
        let sum: f32 = parts.iter().map(|(_, w)| w).sum();
        sum + UNFOLDED_GAP * parts.len().saturating_sub(1) as f32
    }

    /// Lays out its unfolded form left to right in `ui`, its parts 4 px
    /// apart: [`Popover::unfolded_width`] wide.
    pub fn show_unfolded(self, ui: &mut Ui) -> PopoverOutput {
        let parts = self.unfolded_parts(ui);
        let enabled = self.rows_enabled();
        let mut out = PopoverOutput::default();
        let start = ui.cursor().min;
        let Popover { title, switch, rows, .. } = self;
        let mut rows: Vec<Option<Row<'a>>> = rows.into_iter().map(Some).collect();
        let mut switch = switch;
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = UNFOLDED_GAP;
            for (i, _) in parts {
                if i == usize::MAX {
                    if let Some((on, name)) = switch.take() {
                        let icon = title.as_ref().and_then(|(i, _)| *i).unwrap_or(Icon::Eye);
                        let b = KitButton::new(Some(icon), name.as_str()).toggle(*on);
                        if ui.add(b.icon_only(true)).clicked() {
                            *on = !*on;
                            out.changed = true;
                        }
                    }
                    continue;
                }
                if i == usize::MAX - 1 {
                    crate::shell::vsep(ui);
                    continue;
                }
                match rows[i].take() {
                    Some(Row::Check(c)) => {
                        if ui.add(c.toggle(enabled && c.enabled)).clicked() {
                            *c.value = !*c.value;
                            out.changed = true;
                        }
                    }
                    Some(Row::Action(a)) => {
                        if ui.add(a.button(enabled && a.enabled)).clicked() {
                            out.clicked.push(a.words.clone());
                            out.changed = true;
                        }
                    }
                    Some(Row::Custom { unfolded: Some((width, add)), .. }) => {
                        let (rect, _) = ui.allocate_exact_size(
                            vec2(width, metrics::CONTROL_HEIGHT),
                            Sense::hover(),
                        );
                        let builder = UiBuilder::new()
                            .max_rect(rect)
                            .layout(egui::Layout::left_to_right(egui::Align::Center));
                        ui.scope_builder(builder, |ui| {
                            if !enabled {
                                ui.disable();
                            }
                            add(ui)
                        });
                    }
                    _ => {}
                }
            }
        });
        let end = ui.min_rect().right_bottom();
        out.rect = Some(Rect::from_min_max(start, end));
        out
    }

    /// Shown hung from `button` while open, opened and closed by its
    /// clicks (the open state kept under the button's id); [`Hang::Auto`]
    /// against the window's middle.
    pub fn show(self, ui: &mut Ui, button: &Response) -> PopoverOutput {
        let id = Self::id_of(button.id);
        let mut open = Self::is_open(ui.ctx(), button.id);
        if button.clicked() {
            open = !open;
        }
        let frame = ui.ctx().content_rect();
        let out = self.show_hung(ui.ctx(), id, button.rect, frame, &mut open);
        ui.ctx().data_mut(|d| d.insert_temp(id, open));
        out
    }

    /// The id under which the popover of the button `button_id` keeps its
    /// state.
    pub fn id_of(button_id: Id) -> Id {
        button_id.with("mg-uikit-popover")
    }

    /// Whether the popover of the button `button_id` is open
    /// ([`Popover::show`]).
    pub fn is_open(ctx: &egui::Context, button_id: Id) -> bool {
        ctx.data(|d| d.get_temp(Self::id_of(button_id))).unwrap_or(false)
    }

    /// Whether a kit popover was open in this pass or the one before
    /// ([`Popover::show_hung`], [`Popover::show`]): the kit's popovers are
    /// not egui popups, so `egui::Popup::is_any_open` does not see them.
    /// What lets Escape or keys through to the window under a popup asks
    /// this too, so that an Escape that closes a popover cancels nothing
    /// else.
    pub fn any_open(ctx: &egui::Context) -> bool {
        let shown: Option<u64> = ctx.data(|d| d.get_temp(any_open_id()));
        shown.is_some_and(|pass| pass + 1 >= ctx.cumulative_pass_nr())
    }

    /// Shown hung from `anchor` (its button's rectangle) while `open`, as
    /// the layer `id`, [`Hang::Auto`] resolved against `frame`; `open` set
    /// false where a click outside it and `anchor`, Escape or an action
    /// row closes it.
    pub fn show_hung(
        self,
        ctx: &egui::Context,
        id: Id,
        anchor: Rect,
        frame: Rect,
        open: &mut bool,
    ) -> PopoverOutput {
        let shown_id = id.with("shown");
        if !*open {
            ctx.data_mut(|d| d.remove::<bool>(shown_id));
            return PopoverOutput::default();
        }
        // (A popup of egui's open over it, a combo box's list opened from
        // a row, takes the click outside and Escape: as it was when this
        // frame began, before the rows ran.)
        let popup_over = egui::Popup::is_any_open(ctx);
        let first = !ctx.data(|d| d.get_temp::<bool>(shown_id)).unwrap_or(false);
        let pass = ctx.cumulative_pass_nr();
        ctx.data_mut(|d| d.insert_temp(any_open_id(), pass));
        ctx.data_mut(|d| d.insert_temp(shown_id, true));
        let (pivot, pin) = self.hang.resolved(anchor, frame).pin(anchor);
        let name = self.name.clone();
        let area = egui::Area::new(id)
            .kind(egui::UiKind::Popup)
            .order(egui::Order::Foreground)
            .pivot(pivot)
            .fixed_pos(pin)
            // (Kept inside the window: in a narrow one, a popover hung
            // below-right of a button near its left edge is moved right,
            // never flipped.)
            .constrain_to(ctx.content_rect())
            .sizing_pass(first);
        let shown = area.show(ctx, |ui| {
            let frame = egui::Frame::menu(ui.style()).inner_margin(crate::menu::MENU_MARGIN);
            frame
                .show(ui, |ui| {
                    crate::icons::name_group(ui, &name);
                    let scope = ui.scope_builder(UiBuilder::new().closable(), |ui| self.body(ui));
                    (scope.inner, scope.response.should_close())
                })
                .inner
        });
        let (mut out, asked_to_close) = shown.inner;
        let rect = shown.response.rect;
        let outside = ctx.input(|i| {
            i.pointer.any_click()
                && i.pointer
                    .interact_pos()
                    .is_some_and(|p| !rect.contains(p) && !anchor.contains(p))
        });
        let escape = ctx.input(|i| i.key_pressed(egui::Key::Escape));
        if (asked_to_close && !first) || (!popup_over && !first && (outside || escape)) {
            *open = false;
            ctx.data_mut(|d| d.remove::<bool>(shown_id));
        }
        out.open = *open;
        out.rect = Some(rect);
        out
    }

    /// Drawn in place in `ui` (`.popover.static`: a catalog's), its frame
    /// and shadow as hung.
    pub fn show_in_place(self, ui: &mut Ui) -> PopoverOutput {
        let name = self.name.clone();
        let needed = self.needed_width(ui);
        let frame = egui::Frame::menu(ui.style()).inner_margin(crate::menu::MENU_MARGIN);
        let shown = frame.show(ui, |ui| {
            crate::icons::name_group(ui, &name);
            ui.set_max_width(needed);
            self.body(ui)
        });
        let mut out = shown.inner;
        out.open = true;
        out.rect = Some(shown.response.rect);
        out
    }

    /// The popover's body: the title, then the rows, each the popover's
    /// width (its widest row, at least 220).
    fn body(self, ui: &mut Ui) -> PopoverOutput {
        let mut out = PopoverOutput::default();
        let needed = self.needed_width(ui);
        ui.set_min_width(needed);
        let width = if ui.is_sizing_pass() { needed } else { ui.available_width().max(needed) };
        ui.spacing_mut().item_spacing.y = 0.0;
        let enabled = self.rows_enabled();
        let Popover { title, switch, rows, .. } = self;
        let layout = egui::Layout::top_down(egui::Align::Min);
        ui.with_layout(layout, |ui| {
            if let Some((icon, words)) = &title {
                title_row(ui, width, *icon, words, switch, &mut out);
            }
            for row in rows {
                match row {
                    Row::Heading(words) => {
                        crate::menu::menu_heading(ui, words);
                    }
                    Row::Separator => {
                        crate::menu::menu_separator(ui);
                    }
                    Row::Check(c) => {
                        if check_row(ui, width, &c, enabled && c.enabled).clicked() {
                            *c.value = !*c.value;
                            out.changed = true;
                        }
                    }
                    Row::Action(a) => {
                        if action_row(ui, width, &a, enabled && a.enabled).clicked() {
                            out.clicked.push(a.words.clone());
                            out.changed = true;
                            ui.close();
                        }
                    }
                    Row::Raw { add, .. } => add(ui, width),
                    Row::Custom { add, .. } => {
                        let margin = Margin::symmetric(ROW_PADDING as i8, 1);
                        egui::Frame::new().inner_margin(margin).show(ui, |ui| {
                            ui.set_width(width - 2.0 * ROW_PADDING);
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = ROW_GAP;
                                ui.set_min_height(metrics::CONTROL_HEIGHT);
                                if !enabled {
                                    ui.disable();
                                }
                                add(ui);
                            });
                        });
                    }
                }
            }
        });
        out
    }

    /// The width its title and rows need side by side (at least 220).
    fn needed_width(&self, ui: &Ui) -> f32 {
        let text = |s: &str| words_width(ui, s);
        let mut width = POPOVER_MIN_WIDTH;
        if let Some((icon, words)) = &self.title {
            let mut w = 2.0 * ROW_PADDING + text(words);
            if icon.is_some() {
                w += metrics::ICON + ROW_GAP;
            }
            if self.switch.is_some() {
                w += END_GAP + SWITCH.x;
            }
            width = width.max(w);
        }
        for row in &self.rows {
            let w = match row {
                Row::Check(c) => {
                    let lead = if c.sub { SUB_INDENT } else { ROW_PADDING };
                    let icon = if c.icon.is_some() { metrics::ICON + ROW_GAP } else { 0.0 };
                    lead + BOX + ROW_GAP + icon + text(&c.words) + ROW_PADDING
                }
                Row::Action(a) => {
                    let lead = if a.sub { SUB_INDENT } else { ROW_PADDING };
                    let icon = if a.icon.is_some() { metrics::ICON + ROW_GAP } else { 0.0 };
                    let end = a.end.as_deref().map_or(0.0, |e| END_GAP + text(e));
                    lead + icon + text(&a.words) + end + ROW_PADDING
                }
                Row::Heading(h) => 2.0 * ROW_PADDING + text(h),
                Row::Raw { needed, .. } => *needed,
                _ => 0.0,
            };
            width = width.max(w);
        }
        width.ceil()
    }
}

/// The id under which the pass a kit popover was last shown open in is
/// kept ([`Popover::any_open`]).
fn any_open_id() -> Id {
    Id::new("mg-uikit-popover-any-open")
}

/// The width of `words` in the body's font.
fn words_width(ui: &Ui, words: &str) -> f32 {
    let font = FontId::proportional(metrics::BODY_SIZE);
    ui.painter().layout_no_wrap(words.to_string(), font, Color32::WHITE).size().x
}

/// Paints `words` at `x`, centered on `y`, in `color`; returns where they
/// end.
fn paint_words(painter: &egui::Painter, x: f32, y: f32, words: &str, color: Color32) -> f32 {
    let font = FontId::proportional(metrics::BODY_SIZE);
    let galley = painter.layout_no_wrap(words.to_string(), font, color);
    let width = galley.size().x;
    painter.galley(pos2(x, y - galley.size().y / 2.0), galley, color);
    x + width
}

/// The title row: its icon, its words, the switch at its right, a line
/// under it and 4 px under that.
fn title_row(
    ui: &mut Ui,
    width: f32,
    icon: Option<Icon>,
    words: &str,
    switch: Option<(&mut bool, String)>,
    out: &mut PopoverOutput,
) {
    let t = ui.tokens();
    let (rect, _) = ui.allocate_exact_size(vec2(width, TITLE_HEIGHT), Sense::hover());
    let y = rect.center().y - 0.5;
    let mut x = rect.left() + ROW_PADDING;
    if let Some(icon) = icon {
        let square =
            Rect::from_min_size(pos2(x, y - metrics::ICON / 2.0), Vec2::splat(metrics::ICON));
        icon.paint(ui.painter(), square, t.text_2);
        x += metrics::ICON + ROW_GAP;
    }
    paint_words(ui.painter(), x, y, words, t.text);
    let line_y = rect.bottom() - 0.5;
    ui.painter().hline(rect.x_range(), line_y, Stroke::new(1.0, t.line_soft));
    if let Some((on, name)) = switch {
        let at = pos2(rect.right() - ROW_PADDING - SWITCH.x, y - SWITCH.y / 2.0);
        let switch_rect = Rect::from_min_size(at, SWITCH);
        let id = ui.id().with(("mg-uikit-switch", &name));
        let response = ui.interact(switch_rect, id, Sense::click());
        let enabled = ui.is_enabled();
        response.widget_info(|| WidgetInfo::selected(WidgetType::Checkbox, enabled, *on, &name));
        if response.clicked() {
            *on = !*on;
            out.changed = true;
        }
        paint_switch(ui, switch_rect, *on, &response);
        let _ = response.on_hover_text(name.as_str());
    }
    ui.add_space(TITLE_MARGIN);
}

/// Paints a master switch (`.switch`): its track `raised-hover`, or
/// `accent-fill` while on, its 10 px knob 2 px in, `text-2` at the left
/// while off, `on-fill` at the right while on; an `accent` ring while it
/// has the keyboard's focus.
pub fn paint_switch(ui: &Ui, rect: Rect, on: bool, response: &Response) {
    let t = ui.tokens();
    let painter = ui.painter();
    let track = if on { t.accent_fill } else { t.raised_hover };
    painter.rect_filled(rect, rect.height() / 2.0, track);
    let r = 5.0;
    let cx = if on { rect.right() - 2.0 - r } else { rect.left() + 2.0 + r };
    let knob = if on { t.on_fill } else { t.text_2 };
    painter.circle_filled(pos2(cx, rect.center().y), r, knob);
    if response.has_focus() {
        let ring = Stroke::new(1.0, t.accent);
        painter.rect_stroke(
            rect.expand(2.0),
            rect.height() / 2.0 + 2.0,
            ring,
            egui::StrokeKind::Inside,
        );
    }
}

/// A row's rectangle across the popover, 24 px high, lit `hover-row`
/// under the pointer where enabled; and its response.
fn row(ui: &mut Ui, width: f32, enabled: bool) -> Response {
    let sense = if enabled { Sense::click() } else { Sense::hover() };
    let (rect, response) = ui.allocate_exact_size(vec2(width, metrics::MENU_ROW), sense);
    if enabled && (response.hovered() || response.has_focus()) {
        ui.painter().rect_filled(rect, 0.0, ui.tokens().hover_row);
    }
    response
}

fn check_row(ui: &mut Ui, width: f32, c: &PopCheck<'_>, enabled: bool) -> Response {
    let response = row(ui, width, enabled);
    let enabled = enabled && ui.is_enabled();
    response
        .widget_info(|| WidgetInfo::selected(WidgetType::Checkbox, enabled, *c.value, &c.words));
    let rect = response.rect;
    if ui.is_rect_visible(rect) {
        let t = ui.tokens();
        let y = rect.center().y;
        let mut x = rect.left() + if c.sub { SUB_INDENT } else { ROW_PADDING };
        let square = Rect::from_min_size(pos2(x, y - BOX / 2.0), Vec2::splat(BOX));
        let look = Look::of(ui, &response, *c.value, enabled);
        controls::paint_check_lit(ui, &t, square, look, false);
        x += BOX + ROW_GAP;
        let painter = controls::box_painter(ui, enabled);
        let text = if enabled { t.text } else { t.text_off };
        if let Some(icon) = c.icon {
            let sq =
                Rect::from_min_size(pos2(x, y - metrics::ICON / 2.0), Vec2::splat(metrics::ICON));
            icon.paint(&painter, sq, if enabled { t.text_2 } else { t.text_off });
            x += metrics::ICON + ROW_GAP;
        }
        paint_words(&painter, x, y, &c.words, text);
    }
    match &c.tip {
        Some(tip) => response.on_hover_text(tip).on_disabled_hover_text(tip),
        None => response,
    }
}

fn action_row(ui: &mut Ui, width: f32, a: &PopAction, enabled: bool) -> Response {
    let response = row(ui, width, enabled);
    let enabled = enabled && ui.is_enabled();
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, enabled, &a.words));
    let rect = response.rect;
    if ui.is_rect_visible(rect) {
        let t = ui.tokens();
        let painter = controls::box_painter(ui, enabled);
        let y = rect.center().y;
        let mut x = rect.left() + if a.sub { SUB_INDENT } else { ROW_PADDING };
        if let Some(icon) = a.icon {
            let sq =
                Rect::from_min_size(pos2(x, y - metrics::ICON / 2.0), Vec2::splat(metrics::ICON));
            icon.paint(&painter, sq, if enabled { t.text_2 } else { t.text_off });
            x += metrics::ICON + ROW_GAP;
        }
        paint_words(&painter, x, y, &a.words, if enabled { t.text } else { t.text_off });
        if let Some(end) = &a.end {
            let w = words_width(ui, end);
            let ex = rect.right() - ROW_PADDING - w;
            paint_words(&painter, ex, y, end, if enabled { t.text_3 } else { t.text_off });
        }
    }
    match &a.tip {
        Some(tip) => response.on_hover_text(tip).on_disabled_hover_text(tip),
        None => response,
    }
}

/// A row of a popover or More that stands for another control, drawn as
/// an action row (`.pop-row`): an icon, words, lit under the pointer
/// (the editor header's buttons moved into More). Its response.
pub(crate) fn plain_row(
    ui: &mut Ui,
    width: f32,
    icon: Option<Icon>,
    words: &str,
    enabled: bool,
) -> Response {
    action_row(ui, width, &PopAction { icon, ..PopAction::new(words) }, enabled)
}

/// A check row standing for a toggle (the editor header's toggles moved
/// into More). Its response; clicked, the caller flips `on`.
pub(crate) fn plain_check(
    ui: &mut Ui,
    width: f32,
    on: &mut bool,
    icon: Option<Icon>,
    words: &str,
    enabled: bool,
) -> Response {
    let words = words.to_string();
    let check = PopCheck { icon, ..PopCheck::new(on, words) };
    check_row(ui, width, &check, enabled)
}

/// The width a plain row ([`plain_row`], [`plain_check`]) of `words`
/// needs.
pub(crate) fn plain_width(ui: &Ui, icon: Option<Icon>, words: &str, check: bool) -> f32 {
    let icon = if icon.is_some() { metrics::ICON + ROW_GAP } else { 0.0 };
    let check = if check { BOX + ROW_GAP } else { 0.0 };
    2.0 * ROW_PADDING + check + icon + words_width(ui, words)
}
