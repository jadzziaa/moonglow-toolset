//! A menu's parts beyond its rows (the mock's `.menu-pop`, `.popover`):
//! the popover's own padding, 4 px above and below its rows and none at
//! their sides ([`MENU_MARGIN`], which [`crate::menu_style`] gives the
//! frame of every menu whose style it is: the menus of a menu bar given
//! it, an [`IconButton::menu`](crate::IconButton::menu), a
//! [`menu_button`](crate::menu_button), a [`context_menu`] and their
//! submenus), so that a row's fill reaches the popover's edges; and, since
//! nothing else in such a menu has a margin of its own,
//!
//! - [`menu_heading`]: a group's heading (`.pop-h`: padding 6 10 2,
//!   `text-3`);
//! - [`menu_note`]: a line saying something, not to be chosen (`text-3`,
//!   10 px in, a row's height for one line);
//! - [`menu_separator`]: a 1 px `line-soft` line with 4 px above and
//!   below it (`.pop-sep`, `.menu-sep`), egui's separator being a 6 px
//!   band;
//! - [`menu_check`] and [`menu_radio`]: a check box or radio as a row of
//!   the menu, its box where a row's icon or tick goes;
//! - [`MenuKeys`] and [`menu_row_with_keys`]: a row's keys at its right,
//!   in `on-fill` where the row is lit (egui draws a row's shortcut text
//!   weak, which does not read on `accent-fill`);
//! - [`menu_form`]: a menu that holds a form (`.pop-body`: padding
//!   4 10 6, a gap of 6 between its parts).
//!
//! Where no theme was applied to the context, each is egui's own (a
//! weak label, a separator, a check box, a radio, a button's shortcut
//! text) and a form is laid out as it comes.

use std::sync::Arc;

use egui::{
    Atom, Button, Galley, Id, InnerResponse, IntoAtoms, Margin, Rect, Response, Sense, TextStyle,
    TextWrapMode, Ui, Vec2, WidgetInfo, WidgetText, WidgetType, pos2, vec2,
};

use crate::controls::{self, BOX, Look};
use crate::metrics;
use crate::theme::{self, TokensExt as _};

/// A menu's padding (`.menu-pop`, `.popover`: `padding: 4px 0`): its rows
/// run from edge to edge, inside the popover's border.
pub const MENU_MARGIN: Margin = Margin::symmetric(0, 4);
/// A menu separator's height: its 1 px line and 4 px above and below
/// (`.pop-sep`: `height: 1px; margin: 4px 0`).
pub const MENU_SEPARATOR: f32 = 9.0;
/// A menu heading's padding (`.pop-h`: `padding: 6px 10px 2px`).
pub const MENU_HEADING_MARGIN: Margin = Margin { left: 10, right: 10, top: 6, bottom: 2 };
/// A menu note's padding: 10 px in at each side, as a heading's, and a
/// line of it as high as a row (4 above and below the 16 px line).
pub const MENU_NOTE_MARGIN: Margin = Margin { left: 10, right: 10, top: 4, bottom: 4 };
/// A form's padding in a menu (`.pop-body`: `padding: 4px 10px 6px`).
pub const MENU_FORM_MARGIN: Margin = Margin { left: 10, right: 10, top: 4, bottom: 6 };
/// The gap between a menu form's parts (`.pop-body`: `gap: 6px`).
pub const MENU_FORM_GAP: f32 = 6.0;

/// A context menu (`response.context_menu(add)`) in the menu's style
/// ([`crate::menu_style`]) where the theme was applied: egui's own gives
/// its popover egui's menu style, which [`crate::menu_rows`] inside it
/// cannot change the padding of (the frame is made before the rows).
/// Its submenus take the same.
pub fn context_menu<R>(
    response: &Response,
    add: impl FnOnce(&mut Ui) -> R,
) -> Option<InnerResponse<R>> {
    egui::Popup::context_menu(response).style(crate::menu_style).show(add)
}

/// A menu's separator: a 1 px `line-soft` line across the menu, 4 px
/// above and below it (`.pop-sep`). egui's own where no theme was
/// applied.
pub fn menu_separator(ui: &mut Ui) -> Response {
    if theme::applied(ui.ctx()).is_none() {
        return ui.separator();
    }
    let t = ui.tokens();
    // (As egui's separator: nothing to widen a menu by while it is sized.)
    let width = if ui.is_sizing_pass() { 0.0 } else { ui.available_width() };
    let response = ui.allocate_response(vec2(width, MENU_SEPARATOR), Sense::hover());
    let rect = response.rect;
    if ui.is_rect_visible(rect) {
        let y = (rect.top() + 4.0).round();
        let line = Rect::from_min_max(pos2(rect.left(), y), pos2(rect.right(), y + 1.0));
        ui.painter().rect_filled(line, 0.0, t.line_soft);
    }
    response
}

/// Words in a menu, `margin` around them, in `text-3`, wrapping as a
/// label does in `ui`, each line [`metrics::LINE_HEIGHT`] high (the
/// mock's 16 px line: the font's own is a little less, with the leading
/// split above and below it); named by the words for a screen reader.
fn menu_text(ui: &mut Ui, text: WidgetText, margin: Margin) -> Response {
    let t = ui.tokens();
    let room = (ui.available_width() - margin.sum().x).max(0.0);
    let galley = text.into_galley(ui, None, room, TextStyle::Body);
    let lines = galley.rows.len().max(1) as f32;
    let high = galley.size().y.max(lines * metrics::LINE_HEIGHT);
    let size = vec2(galley.size().x, high) + margin.sum();
    let response = ui.allocate_response(size, Sense::hover());
    let words = galley.text().to_string();
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Label, true, &words));
    if ui.is_rect_visible(response.rect) {
        let leading = ((high - galley.size().y) / (2.0 * lines)).round();
        let at = response.rect.min + vec2(margin.leftf(), margin.topf() + leading);
        ui.painter().galley(at, galley, t.text_3);
    }
    response
}

/// A heading of a group of a menu's rows (`.pop-h`: `padding: 6px 10px
/// 2px`, `text-3`), the body's size, wrapping at the menu's width. A
/// weak label where no theme was applied.
pub fn menu_heading(ui: &mut Ui, text: impl Into<WidgetText>) -> Response {
    if theme::applied(ui.ctx()).is_none() {
        return ui.label(text.into().weak());
    }
    menu_text(ui, text.into(), MENU_HEADING_MARGIN)
}

/// A line in a menu that says something and is not to be chosen ("None
/// saved yet", "3 objects chosen"): `text-3`, 10 px in as a heading, a
/// line of it as high as a row, wrapping at the menu's width. A weak
/// label where no theme was applied.
pub fn menu_note(ui: &mut Ui, text: impl Into<WidgetText>) -> Response {
    if theme::applied(ui.ctx()).is_none() {
        return ui.label(text.into().weak());
    }
    menu_text(ui, text.into(), MENU_NOTE_MARGIN)
}

/// The id of a menu row's mark's room (a check box's, a radio's).
fn mark_id() -> Id {
    Id::new("mg-uikit-menu-mark")
}

/// A row of a menu with a mark's room (an icon's, 16 px) before its
/// words: the row as `ui.button` makes it in a menu, and where its mark
/// goes.
fn marked_row<'a>(ui: &mut Ui, atoms: impl IntoAtoms<'a>) -> (Response, Option<Rect>) {
    let mut atoms = atoms.into_atoms();
    atoms.push_left(Atom::custom(mark_id(), Vec2::splat(metrics::ICON)));
    let laid = Button::new(atoms).atom_ui(ui);
    let room = laid.rect(mark_id());
    (laid.response, room)
}

/// Whether a menu's row is lit (filled `accent-fill`): under the pointer,
/// pressed or holding the keyboard's focus, as egui's menu style fills
/// it, and enabled.
pub fn menu_row_lit(response: &Response) -> bool {
    response.enabled()
        && (response.hovered()
            || response.highlighted()
            || response.has_focus()
            || response.is_pointer_button_down_on())
}

/// Whether a menu's row is drawn lit: [`menu_row_lit`], or its state's
/// fill is the menu's lit one whatever the pointer does (a sheet's row
/// drawn as hovered).
fn drawn_lit(ui: &Ui, response: &Response) -> bool {
    let lit_fill = theme::applied(ui.ctx()).map(|t| t.tokens().accent_fill);
    menu_row_lit(response)
        || (response.enabled() && Some(ui.style().interact(response).weak_bg_fill) == lit_fill)
}

/// The look of a mark in a row, and whether the row is lit (on
/// `accent-fill`, where the mark takes an `on-fill` border so that a box
/// filled `accent-fill` still shows).
fn lit_look(ui: &Ui, response: &Response, on: bool) -> (Look, bool) {
    let look = Look::of(ui, response, on, response.enabled());
    (look, drawn_lit(ui, response))
}

/// A check box as a row of a menu (the mock's `.pop-row.check` in a
/// menu's rows): as high as a row, lit as one, its box in the room a
/// row's icon or tick takes (16 px, 8 px in), its words after; a click or
/// Space toggles it. A check box to a screen reader, named by its words.
/// egui's check box where no theme was applied.
pub fn menu_check<'a>(ui: &mut Ui, checked: &'a mut bool, atoms: impl IntoAtoms<'a>) -> Response {
    let Some(theme) = theme::applied(ui.ctx()) else {
        return ui.checkbox(checked, atoms);
    };
    let t = theme.tokens();
    let atoms = atoms.into_atoms();
    let text = atoms.text().map(String::from).unwrap_or_default();
    let (mut response, room) = marked_row(ui, atoms);
    if response.clicked() {
        *checked = !*checked;
        response.mark_changed();
    }
    let enabled = response.enabled();
    let on = *checked;
    response.widget_info(|| WidgetInfo::selected(WidgetType::Checkbox, enabled, on, &text));
    if let Some(room) = room.filter(|_| ui.is_rect_visible(response.rect)) {
        let square = Rect::from_center_size(room.center(), Vec2::splat(BOX));
        let (look, lit) = lit_look(ui, &response, on);
        controls::paint_check_lit(ui, &t, square, look, lit);
    }
    response
}

/// A radio as a row of a menu, on where `selected` (as [`menu_check`]: its
/// circle in the room of a row's icon). A radio button to a screen
/// reader. egui's radio where no theme was applied.
pub fn menu_radio<'a>(ui: &mut Ui, selected: bool, atoms: impl IntoAtoms<'a>) -> Response {
    let Some(theme) = theme::applied(ui.ctx()) else {
        return ui.radio(selected, atoms);
    };
    let t = theme.tokens();
    let atoms = atoms.into_atoms();
    let text = atoms.text().map(String::from).unwrap_or_default();
    let (response, room) = marked_row(ui, atoms);
    let enabled = response.enabled();
    response
        .widget_info(|| WidgetInfo::selected(WidgetType::RadioButton, enabled, selected, &text));
    if let Some(room) = room.filter(|_| ui.is_rect_visible(response.rect)) {
        let (look, lit) = lit_look(ui, &response, selected);
        controls::paint_radio_lit(ui, &t, room.center(), look, lit);
    }
    response
}

/// [`menu_radio`] that is on where `*current == alternative`, and
/// clicked, sets it so (as `ui.radio_value`).
pub fn menu_radio_choice<'a, V: PartialEq>(
    ui: &mut Ui,
    current: &mut V,
    alternative: V,
    atoms: impl IntoAtoms<'a>,
) -> Response {
    let mut response = menu_radio(ui, *current == alternative, atoms);
    if response.clicked() && *current != alternative {
        *current = alternative;
        response.mark_changed();
    }
    response
}

/// The id of a menu row's keys' room ([`MenuKeys::atom`]).
fn keys_id() -> Id {
    Id::new("mg-uikit-menu-keys")
}

/// A menu row's keys (`.menu-row .keys`), laid out by the row and painted
/// after it in the color its state gives: `text-3` (egui's weak text) at
/// rest, `on-fill` where the row is lit (`.menu-row.hover .keys`). For a
/// row made by hand: `Button::shortcut_text(keys.atom())`, the row added
/// with `atom_ui`, then [`MenuKeys::paint`]; [`menu_row_with_keys`] does
/// all three.
#[derive(Clone, Debug)]
pub struct MenuKeys {
    galley: Arc<Galley>,
}

impl MenuKeys {
    /// `keys` laid out in `ui`'s button text.
    pub fn new(ui: &Ui, keys: impl Into<String>) -> Self {
        let text = WidgetText::from(keys.into());
        let galley =
            text.into_galley(ui, Some(TextWrapMode::Extend), f32::INFINITY, TextStyle::Button);
        MenuKeys { galley }
    }

    /// The keys as a row's atom (for `Button::shortcut_text`): laid out
    /// and named as egui's shortcut text is (the row's accessible name is
    /// its words and keys, "Undo Ctrl+Z"), drawn clear, for
    /// [`Self::paint`] to draw them over in their color.
    pub fn atom(&self) -> Atom<'static> {
        use egui::AtomExt as _;
        let clear = egui::RichText::new(self.galley.text()).color(egui::Color32::TRANSPARENT);
        clear.atom_id(keys_id())
    }

    /// The keys painted in the room `laid` (the row given [`Self::atom`],
    /// added with `atom_ui`) left them: in `on-fill` where `lit` (the row
    /// is lit; [`menu_row_lit`], or chosen by the caller's own keys), else
    /// in the weak text's color. `ui` is the one the row was added to
    /// (a disabled row's keys fade with it).
    pub fn paint(&self, ui: &Ui, laid: &egui::AtomLayoutResponse, lit: bool) {
        let Some(rect) = laid.rect(keys_id()) else { return };
        if !ui.is_rect_visible(rect) {
            return;
        }
        let color = match (lit, theme::applied(ui.ctx())) {
            (true, Some(theme)) => theme.tokens().on_fill,
            _ => ui.visuals().weak_text_color(),
        };
        ui.painter().galley_with_override_text_color(rect.min, self.galley.clone(), color);
    }
}

/// A menu's row `button` with `keys` at its right (as
/// `button.shortcut_text(keys)`), its keys in `on-fill` while the row is
/// lit ([`menu_row_lit`], or drawn so) and egui's weak text's color
/// otherwise. egui's
/// own shortcut text where no theme was applied; no keys where `keys` is
/// empty.
pub fn menu_row_with_keys<'a>(ui: &mut Ui, button: Button<'a>, keys: &str) -> Response {
    if keys.is_empty() {
        return ui.add(button);
    }
    if theme::applied(ui.ctx()).is_none() {
        return ui.add(button.shortcut_text(keys));
    }
    let keys = MenuKeys::new(ui, keys);
    let laid = button.shortcut_text(keys.atom()).atom_ui(ui);
    keys.paint(ui, &laid, drawn_lit(ui, &laid.response));
    laid.response
}

/// A menu that holds a form rather than rows (a color picker, a field to
/// type a name into, a block of code to copy, an inspector): `add` laid
/// out with the mock's `.pop-body` padding (4 10 6: the menu itself has
/// none at its sides) and a gap of 6 between its parts, in the window's
/// own style rather than the menu's rows' (its buttons, combo boxes and
/// fields framed as in a form, not flat rows 24 high): its controls a
/// control's height or, where a window made its text larger (its style
/// the context's while the menu is drawn), as tall as the window's. `add`
/// itself where no theme was applied.
pub fn menu_form<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
    if theme::applied(ui.ctx()).is_none() {
        return add(ui);
    }
    // (A popup's style starts from the context's: the window's own,
    // before the menu's rows made it theirs.)
    let mut style = (*ui.ctx().global_style()).clone();
    style.spacing.item_spacing.y = MENU_FORM_GAP;
    style.spacing.interact_size.y = style.spacing.interact_size.y.max(metrics::CONTROL_HEIGHT);
    egui::Frame::new()
        .inner_margin(MENU_FORM_MARGIN)
        .show(ui, |ui| {
            ui.set_style(style);
            add(ui)
        })
        .inner
}
