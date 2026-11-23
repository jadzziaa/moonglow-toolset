//! Copy and Copy All on the rows of the lists of results (5.2's W4, a
//! builder's report that Verify's findings could not be copied): Problems,
//! Search Results (a script's lines, Find Instance's objects, Find and
//! Replace Text's strings), Advanced Search, Compiler, Bookmarks and the NUI
//! designer's Checks. A row's menu (a right click, or Shift+F10 on the row
//! chosen where the list has keys) has the row's own commands first, as the
//! module tree's Go To is, then **Copy** (the row as one line of text, its
//! message whole, never as cut short) and **Copy All** (every row listed,
//! one a line, in the list's order); Ctrl+C copies the row chosen where the
//! list chooses one. The text is made only when asked (a click, a key), not
//! each frame: Copy Log's way, at once, as a list of fifty thousand rows is
//! a few milliseconds of `format!`.

use egui::Ui;

/// The menu item that copies its row.
pub const COPY: &str = "Copy";
/// The menu item that copies every row listed.
pub const COPY_ALL: &str = "Copy All";

/// What a row's menu was asked to copy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Asked {
    /// The row.
    Row,
    /// Every row listed.
    All,
}

/// Shift+F10 pressed (egui has no menu key): taken, so that the menu bar's
/// F10 does not take it too.
pub(crate) fn menu_key(ui: &Ui) -> bool {
    ui.input_mut(|i| i.consume_key(egui::Modifiers::SHIFT, egui::Key::F10))
}

/// Ctrl+C in this frame, and no other modifier with it: the backend's Copy
/// (as `ro_gff`'s rows take it; egui-winit sends it for Ctrl+Alt+C and
/// Ctrl+Shift+C too, the Creature Wizard's and others' keys, so the
/// modifiers held are asked) or the keys themselves.
pub(crate) fn copy_pressed(ui: &Ui) -> bool {
    ui.input(copy_in)
}

/// [`copy_pressed`] of one frame's input. (A Copy with nothing held is the
/// Copy key's, or a menu's.)
pub(crate) fn copy_in(i: &egui::InputState) -> bool {
    let only = |m: egui::Modifiers| m.is_none() || m.matches_exact(egui::Modifiers::COMMAND);
    let copy = i.events.iter().any(|e| matches!(e, egui::Event::Copy)) && only(i.modifiers);
    let keys = i.events.iter().any(|e| {
        matches!(e, egui::Event::Key { key: egui::Key::C, pressed: true, modifiers, .. }
            if modifiers.matches_exact(egui::Modifiers::COMMAND))
    });
    copy || keys
}

/// Shift+F10 and Ctrl+C on `row` while it holds the keyboard's focus (a
/// list with no row chosen: its rows' links and labels are focusable):
/// whether its menu was asked for, and whether it was to be copied.
pub(crate) fn focused_keys(ui: &Ui, row: &egui::Response) -> (bool, bool) {
    if !row.has_focus() {
        return (false, false);
    }
    (menu_key(ui), copy_pressed(ui))
}

/// A row's menu: opened by a right click on `row`, or by `keyed` (Shift+F10
/// on the row chosen, its first item taking the keyboard, which Escape
/// gives back to the row), at the pointer or under the row's start; a
/// click on the row closes it. `own` draws the
/// row's own commands first and says whether it drew any (a separator
/// after them); then Copy and Copy All. What was asked to be copied.
pub(crate) fn row_menu(
    ui: &Ui,
    row: &egui::Response,
    keyed: bool,
    own: impl FnOnce(&mut Ui) -> bool,
) -> Option<Asked> {
    row_menu_saying(ui, row, keyed, ALL_TIP, own)
}

/// Copy All's tip where a list does not say otherwise.
const ALL_TIP: &str = "Copy all the rows listed, each on a line of its own";

/// [`row_menu`] with Copy All's tip its list's own (a list in parts says
/// it copies them all).
pub(crate) fn row_menu_saying(
    ui: &Ui,
    row: &egui::Response,
    keyed: bool,
    all_tip: &str,
    own: impl FnOnce(&mut Ui) -> bool,
) -> Option<Asked> {
    let opened = row.secondary_clicked() || keyed;
    let set = if opened {
        Some(egui::SetOpenCommand::Bool(true))
    } else if row.clicked() {
        Some(egui::SetOpenCommand::Bool(false))
    } else {
        None
    };
    let at_id = row.id.with("row-menu-at");
    if opened {
        let pointer = ui.ctx().pointer_interact_pos().filter(|_| !keyed);
        let at = pointer.unwrap_or(row.rect.left_bottom());
        ui.data_mut(|d| d.insert_temp(at_id, at));
    }
    let at = ui.data(|d| d.get_temp::<egui::Pos2>(at_id)).unwrap_or(row.rect.left_bottom());
    // (Opened by the keys: Escape gives them back to the row.)
    let by_keys = row.id.with("row-menu-keyed");
    if opened {
        ui.data_mut(|d| d.insert_temp(by_keys, keyed));
    }
    let menu = egui::Popup::menu(row).open_memory(set).at_position(at);
    let was_open = menu.is_open();
    let escape = ui.input(|i| i.key_pressed(egui::Key::Escape));
    if was_open && escape && ui.data(|d| d.get_temp::<bool>(by_keys)).unwrap_or(false) {
        row.request_focus();
    }
    let mut asked = None;
    menu.style(mg_uikit::menu_style).show(|ui| {
        mg_uikit::menu_rows(ui);
        if own(ui) {
            mg_uikit::menu_separator(ui);
        }
        let copy = ui.button(COPY).on_hover_text("Copy this row as one line of text");
        if keyed {
            copy.request_focus();
        }
        if copy.clicked() {
            asked = Some(Asked::Row);
            ui.close();
        }
        let all = ui.button(COPY_ALL).on_hover_text(all_tip);
        if all.clicked() {
            asked = Some(Asked::All);
            ui.close();
        }
    });
    asked
}

/// Lines joined, one a line, with no line after the last.
pub(crate) fn lines(rows: impl IntoIterator<Item = String>) -> String {
    let mut out = String::new();
    for (i, row) in rows.into_iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        out.push_str(&row);
    }
    out
}
