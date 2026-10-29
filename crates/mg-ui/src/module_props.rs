//! Module Properties (module.ifo), with Aurora's tabs: Basic, Events,
//! Advanced, Description and Custom Content. Every change is one undoable
//! command.

use egui::Ui;
use mg_core::{LocString, ResRef, ResType};
use mg_edit::{Command, Edit, GffPath};
use mg_gff::{Struct, Value};
use mg_resman::{GameInstall, ResKey};
use mg_schema::{ExoString, Field, GffValue, StructExt, ifo};

use crate::text::{decode, encode, with_english};
use crate::widgets::{FieldTarget, LocStringEdit, VarTableEdit, resref_field, variables_button};
use crate::{Action, Moonglow};

pub(crate) fn info_key() -> ResKey {
    mg_module::Module::info_key()
}

fn target(label: &str) -> FieldTarget {
    FieldTarget::new(info_key(), GffPath::root(), label)
}

/// A command setting one module-info field.
fn set<T: GffValue>(label: &str, f: &Field<T>, v: T) -> Action {
    set_value(label, f.label, v.into_value())
}

fn set_value(what: &str, label: &str, value: Value) -> Action {
    Action::Apply(Command::new(
        what,
        vec![Edit::SetField {
            key: info_key(),
            path: GffPath::root(),
            label: label.to_string(),
            value: Some(value),
        }],
    ))
}

/// A one-line text field that commits when focus leaves it.
fn text_field(app: &mut Moonglow, ui: &mut Ui, id: &str, current: &str) -> Option<String> {
    edit_field(app, ui, id, current, false, false)
}

/// [`edit_field`] for a localized string: its text in the language
/// edited, else what [`crate::widgets::loc_shown`] finds, marked as not
/// its own.
fn loc_field(
    app: &mut Moonglow,
    ui: &mut Ui,
    id: &str,
    ls: &LocString,
    multiline: bool,
) -> Option<String> {
    match crate::widgets::loc_shown(app, ls) {
        (shown, None) => edit_field(app, ui, id, &shown, multiline, true),
        (shown, Some((from, why))) => crate::widgets::borrowed_field(ui, &from, why, |ui| {
            edit_field(app, ui, id, &shown, multiline, true)
        }),
    }
}

/// A text field (one line or several) that commits when focus leaves it.
/// Multi-line text keeps the line-end style it had (CRLF or LF); a line
/// typed into a text of one line gets a localized string's (`loc`: plain,
/// as Aurora writes) or a `CExoString`'s (CRLF).
fn edit_field(
    app: &mut Moonglow,
    ui: &mut Ui,
    id: &str,
    current: &str,
    multiline: bool,
    loc: bool,
) -> Option<String> {
    let id = egui::Id::new(("ifo", id));
    let (rows, width) = if multiline { (Some(12), f32::INFINITY) } else { (None, 320.0) };
    let buffers = &mut app.buffers;
    match loc {
        true => crate::widgets::commit_loc_text_in(buffers, ui, id, current, rows, width, None),
        false => crate::widgets::commit_text_in(buffers, ui, id, current, rows, width, None),
    }
}

/// A number with a range and Aurora's arrows, committed when it changes (a
/// drag: when released; an arrow held: when let go).
fn number<T: egui::emath::Numeric + GffValue>(
    app: &mut Moonglow,
    ui: &mut Ui,
    what: &str,
    f: &Field<T>,
    current: T,
    range: std::ops::RangeInclusive<T>,
) {
    if let Some(v) = crate::widgets::spin_number(ui, current, range) {
        app.actions.push(set(what, f, v));
    }
}

/// The dialog's tabs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Page {
    #[default]
    Basic,
    Events,
    Advanced,
    Description,
    CustomContent,
}

/// The dialog's tabs and their names.
pub(crate) const PAGES: [(Page, &str); 5] = [
    (Page::Basic, "Basic"),
    (Page::Events, "Events"),
    (Page::Advanced, "Advanced"),
    (Page::Description, "Description"),
    (Page::CustomContent, "Custom Content"),
];

/// Module events in Aurora's order, with its labels. Aurora hides
/// OnModuleStart; it is shown here only when set.
pub(crate) const EVENTS: [(&str, &str); 22] = [
    ("OnClientEnter", "Mod_OnClientEntr"),
    ("OnModuleLoad", "Mod_OnModLoad"),
    ("OnHeartbeat", "Mod_OnHeartbeat"),
    ("OnUserDefined", "Mod_OnUsrDefined"),
    ("OnClientLeave", "Mod_OnClientLeav"),
    ("OnActivateItem", "Mod_OnActvtItem"),
    ("OnAcquireItem", "Mod_OnAcquirItem"),
    ("OnUnAcquireItem", "Mod_OnUnAqreItem"),
    ("OnPlayerDeath", "Mod_OnPlrDeath"),
    ("OnPlayerDying", "Mod_OnPlrDying"),
    ("OnPlayerRespawn", "Mod_OnSpawnBtnDn"),
    ("OnPlayerRest", "Mod_OnPlrRest"),
    ("OnPlayerLevelUp", "Mod_OnPlrLvlUp"),
    ("OnCutsceneAbort", "Mod_OnCutsnAbort"),
    ("OnPlayerEquipItem", "Mod_OnPlrEqItm"),
    ("OnPlayerUnEquipItem", "Mod_OnPlrUnEqItm"),
    ("OnPlayerChat", "Mod_OnPlrChat"),
    ("OnNuiEvent", "Mod_OnNuiEvent"),
    ("OnPlayerGuiEvent", "Mod_OnPlrGuiEvt"),
    ("OnPlayerTarget", "Mod_OnPlrTarget"),
    ("OnPlayerTileAction", "Mod_OnPlrTileAct"),
    ("OnModuleStart", "Mod_OnModStart"),
];

/// Module Properties, over its OK and Cancel (`blueprint::ok_cancel`).
pub(crate) fn ui(app: &mut Moonglow, ui: &mut Ui) {
    let targets = [(info_key(), mg_edit::GffPath::root())];
    let pressed = app.ws.is_some().then(|| crate::blueprint::ok_cancel(ui, &targets[0], false));
    let pending = app.actions.len();
    pages(app, ui);
    let tab = crate::Tab::ModuleProperties;
    crate::blueprint::window_drawn(app, pending, &tab, &targets, pressed.flatten());
}

fn pages(app: &mut Moonglow, ui: &mut Ui) {
    let Some(ws) = &mut app.ws else {
        ui.label("No module is open.");
        return;
    };
    let root: Struct = match ws.doc(&info_key()) {
        Ok(g) => g.root.clone(),
        Err(e) => {
            ui.colored_label(ui.visuals().error_fg_color, e.to_string());
            return;
        }
    };
    ui.horizontal(|ui| {
        for (page, label) in PAGES {
            ui.selectable_value(&mut app.module_page, page, label);
        }
    });
    ui.separator();
    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| match app.module_page {
        Page::Basic => basic(app, ui, &root),
        Page::Events => events(app, ui, &root),
        Page::Advanced => advanced(app, ui, &root),
        Page::Description => description(app, ui, &root),
        Page::CustomContent => custom_content(app, ui, &root),
    });
}

fn basic(app: &mut Moonglow, ui: &mut Ui, root: &Struct) {
    egui::Grid::new("ifo-basic").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
        crate::widgets::field_label(ui, "Name");
        let name: LocString = root.read(&ifo::MOD_NAME);
        ui.horizontal(|ui| {
            if let Some(v) = loc_field(app, ui, "name", &name, false) {
                app.actions.push(set(
                    "Module name",
                    &ifo::MOD_NAME,
                    with_english(name.clone(), &v),
                ));
            }
            if ui.small_button("…").on_hover_text("Edit text in multiple languages").clicked() {
                app.loc_edit =
                    Some(LocStringEdit::new(target(ifo::MOD_NAME.label), "Module name", &name));
            }
        });
        ui.end_row();

        crate::widgets::field_label(ui, "Tag");
        let tag = decode(root.read(&ifo::MOD_TAG).as_bytes());
        if let Some(v) = text_field(app, ui, "tag", &tag) {
            app.actions.push(set("Module tag", &ifo::MOD_TAG, ExoString(encode(&v))));
        }
        ui.end_row();

        // Set by placing the start location in an area, as in Aurora.
        crate::widgets::field_label(ui, "Start area");
        ui.label(root.read(&ifo::MOD_ENTRY_AREA).to_string());
        ui.end_row();
        crate::widgets::field_label(ui, "Start location");
        ui.label(format!(
            "{:.2}, {:.2}, {:.2}",
            root.read(&ifo::MOD_ENTRY_X),
            root.read(&ifo::MOD_ENTRY_Y),
            root.read(&ifo::MOD_ENTRY_Z)
        ));
        ui.end_row();
    });
}

fn events(app: &mut Moonglow, ui: &mut Ui, root: &Struct) {
    egui::Grid::new("ifo-events").num_columns(3).spacing([12.0, 4.0]).show(ui, |ui| {
        for (label, field) in EVENTS {
            let current = root.resref(field).unwrap_or(ResRef::EMPTY);
            if field == "Mod_OnModStart" && current.is_empty() {
                continue;
            }
            crate::widgets::field_label(ui, label);
            let id = egui::Id::new(("ifo-event", field));
            let types = [ResType::NSS, ResType::NCS];
            if let Some(v) = resref_field(app, ui, id, current, "Select a script", &types) {
                app.actions.push(set_value(&format!("{label} script"), field, Value::resref(v)));
            }
            if let Some(name) = crate::widgets::edit_script_button(app, ui, id, current) {
                app.actions.push(Action::EditScript { name, condition: false });
            }
            ui.end_row();
        }
    });
}

fn advanced(app: &mut Moonglow, ui: &mut Ui, root: &Struct) {
    egui::Grid::new("ifo-advanced").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
        let rows: [(&str, &Field<u8>, std::ops::RangeInclusive<u8>); 6] = [
            ("Starting month", &ifo::MOD_START_MONTH, 1..=12),
            ("Starting day", &ifo::MOD_START_DAY, 1..=31),
            ("Starting hour", &ifo::MOD_START_HOUR, 0..=23),
            ("Minutes per hour", &ifo::MOD_MIN_PER_HOUR, 1..=240),
            ("Dawn start hour", &ifo::MOD_DAWN_HOUR, 0..=23),
            ("Dusk start hour", &ifo::MOD_DUSK_HOUR, 0..=23),
        ];
        for (label, f, range) in rows {
            crate::widgets::field_label(ui, label);
            number(app, ui, label, f, root.read(f), range);
            ui.end_row();
        }
        crate::widgets::field_label(ui, "Starting year");
        number(
            app,
            ui,
            "Starting year",
            &ifo::MOD_START_YEAR,
            root.read(&ifo::MOD_START_YEAR),
            0..=30000,
        );
        ui.end_row();

        crate::widgets::field_label(ui, "XP scale");
        let mut xp = root.read(&ifo::MOD_XP_SCALE);
        let r = ui.add(egui::Slider::new(&mut xp, 0..=200).clamping(egui::SliderClamping::Edits));
        if r.drag_stopped() || (r.changed() && !r.dragged()) {
            app.actions.push(set("Experience scale", &ifo::MOD_XP_SCALE, xp));
        }
        ui.end_row();

        crate::widgets::field_label(ui, "Starting movie");
        let movie = root.read(&ifo::MOD_START_MOVIE);
        let shown = if movie.is_empty() { "(none)".to_string() } else { movie.to_string() };
        // (Read only while the list is used: open, or with the keys.)
        let movies = || {
            let names = (app.install.as_ref())
                .map(|i| GameInstall::file_names(&i.movie_dirs(), "bik"))
                .unwrap_or_default();
            let movies = names.iter().filter_map(|m| ResRef::from_str(m).ok());
            std::iter::once(ResRef::EMPTY).chain(movies).collect::<Vec<_>>()
        };
        let mut pick = movie;
        crate::widgets::dropdown("ifo-movie").selected_text(shown).show_with(
            ui,
            &mut pick,
            movies,
            |ui, pick| {
                for m in movies() {
                    let name = if m.is_empty() { "(none)".to_string() } else { m.to_string() };
                    ui.selectable_value(pick, m, name);
                }
            },
        );
        if pick != movie {
            app.actions.push(set("Starting movie", &ifo::MOD_START_MOVIE, pick));
        }
        ui.end_row();

        crate::widgets::field_label(ui, "Variables");
        let list = root.items(&ifo::VAR_TABLE);
        if variables_button(ui, list.len()) {
            app.var_edit = Some(VarTableEdit::new(target(ifo::VAR_TABLE.label), list));
        }
        ui.end_row();
    });
}

fn description(app: &mut Moonglow, ui: &mut Ui, root: &Struct) {
    let desc: LocString = root.read(&ifo::MOD_DESCRIPTION);
    ui.horizontal(|ui| {
        let language = crate::text::edit_language().name().unwrap_or("?");
        crate::widgets::field_label(ui, format!("Description ({language})"));
        if ui.small_button("…").on_hover_text("Edit text in multiple languages").clicked() {
            let t = target(ifo::MOD_DESCRIPTION.label);
            app.loc_edit = Some(LocStringEdit::new(t, "Module description", &desc));
        }
    });
    if let Some(v) = loc_field(app, ui, "desc", &desc, true) {
        app.actions.push(set("Module description", &ifo::MOD_DESCRIPTION, with_english(desc, &v)));
    }
}

/// The hak list of Module Properties › Custom Content: the rows chosen,
/// and where the keys are.
#[derive(Debug, Default)]
pub struct HakList {
    pub selected: std::collections::BTreeSet<usize>,
    /// The row clicked last: Shift + click and Shift + the arrows choose
    /// from it.
    anchor: Option<usize>,
    /// The row the arrow keys are at.
    cursor: Option<usize>,
    /// The cursor's row is to be brought into view.
    reveal: bool,
    /// The haks in the hak folders ([`haks_on_disk`]), and when they were
    /// read (egui's time).
    on_disk: Option<(f64, std::sync::Arc<Vec<String>>)>,
}

/// How long the hak folders' listing is kept before they are read again
/// (a hak copied in meanwhile shows then): not read every frame.
const ON_DISK_KEPT: f64 = 2.0;

/// The haks in the hak folders (lower case, without `.hak`): the list
/// read at most [`ON_DISK_KEPT`] seconds ago, else read now. `None`
/// without a game install.
fn haks_on_disk(app: &mut Moonglow, now: f64) -> Option<std::sync::Arc<Vec<String>>> {
    let install = app.install.as_ref()?;
    match &app.hak_list.on_disk {
        Some((read, haks)) if (0.0..ON_DISK_KEPT).contains(&(now - read)) => Some(haks.clone()),
        _ => {
            let haks = std::sync::Arc::new(GameInstall::file_names(&install.hak_dirs(), "hak"));
            app.hak_list.on_disk = Some((now, haks.clone()));
            Some(haks)
        }
    }
}

/// A row of the hak list being dragged.
#[derive(Debug, Clone, Copy, PartialEq)]
struct HakDrag(usize);

/// How fast (points a second; up positive, as `Ui::scroll_with_delta`
/// takes it) a list seen over `seen` scrolls for a row dragged with the
/// pointer at `y`: within a row's height of an edge, faster the nearer,
/// and fastest a few rows past it; elsewhere not at all.
pub(crate) fn edge_scroll(seen: egui::Rangef, y: f32, row: f32) -> f32 {
    // (Eight rows a second at the edge.)
    let rows_a_second = 8.0 * row;
    let depth = |into: f32| (into / row).clamp(0.0, 3.0);
    if y < seen.min + row {
        rows_a_second * depth(seen.min + row - y)
    } else if y > seen.max - row {
        -rows_a_second * depth(y - (seen.max - row))
    } else {
        0.0
    }
}

/// The order of `n` rows with those `chosen` moved together to stand
/// before row `before` (as the rows are numbered now; `n`: at the end),
/// in their order among themselves; and where they stand then.
pub(crate) fn moved(
    n: usize,
    chosen: &std::collections::BTreeSet<usize>,
    before: usize,
) -> (Vec<usize>, std::collections::BTreeSet<usize>) {
    let rest: Vec<usize> = (0..n).filter(|i| !chosen.contains(i)).collect();
    let at = rest.iter().filter(|i| **i < before).count();
    let mut order = rest[..at].to_vec();
    order.extend(chosen.iter().copied().filter(|i| *i < n));
    let end = order.len();
    order.extend(&rest[at..]);
    (order, (at..end).collect())
}

fn custom_content(app: &mut Moonglow, ui: &mut Ui, root: &Struct) {
    use std::collections::BTreeSet;
    let items = root.items(&ifo::MOD_HAK_LIST);
    let haks: Vec<String> =
        items.iter().map(|h| decode(h.read(&ifo::mod_hak_list::MOD_HAK).as_bytes())).collect();
    let n = haks.len();
    let list = || ifo::MOD_HAK_LIST.label.to_string();
    let command = |edits: Vec<Edit>, label: &str| Action::Apply(Command::new(label, edits));
    // The haks there are to attach, and those of the module's that are not
    // among them: not found, which the game will not load the module with.
    let on_disk = haks_on_disk(app, ui.ctx().input(|i| i.time));
    let missing = |h: &str| {
        on_disk.as_ref().is_some_and(|all| !all.iter().any(|x| x.eq_ignore_ascii_case(h)))
    };
    let mut state = std::mem::take(&mut app.hak_list);
    state.selected.retain(|i| *i < n);
    state.cursor = state.cursor.filter(|c| *c < n);
    state.anchor = state.anchor.filter(|a| *a < n);
    // What is asked of the list this frame: the chosen rows moved to stand
    // before a row, or removed. One undoable step each.
    let mut move_to: Option<usize> = None;
    let mut remove = false;

    ui.horizontal(|ui| {
        crate::widgets::section_heading(ui, format!("Hak Files ({n})"));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.weak("Top overrides lower").on_hover_text(
                "Where two haks have a resource of the same name, the game takes the one \
                 from the hak higher in this list",
            );
        });
    });
    let row_height = ui.spacing().interact_size.y;
    let frame = egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.set_width(ui.available_width());
        let tall = (row_height + ui.spacing().item_spacing.y) * 14.0;
        let mut rects = Vec::with_capacity(n);
        egui::ScrollArea::vertical().id_salt("ifo-haks").max_height(tall).show(ui, |ui| {
            ui.set_width(ui.available_width());
            // (An older module's single hak, `Mod_Hak`, read where it has
            // no list: the haks layered, `mg_module::haks_of`.)
            match mg_module::haks_of(root).first().filter(|_| n == 0) {
                Some(old) => {
                    _ = ui.weak(format!(
                        "No list of haks: the module names its hak, {old}, in the older single \
                     field (Mod_Hak), which is read where there is no list. A hak added here \
                     starts the list in its place: add {old} to it too, to keep it."
                    ))
                }
                None if n == 0 => _ = ui.weak("No haks: the module uses the game's own resources."),
                None => {}
            }
            for (i, h) in haks.iter().enumerate() {
                let row = ui.horizontal(|ui| {
                    // The handle to drag the row by: two columns of dots.
                    let (grip, handle) =
                        ui.allocate_exact_size(egui::vec2(12.0, row_height), egui::Sense::drag());
                    let dot = ui.visuals().weak_text_color();
                    for (dx, dy) in [
                        (-2.5, -4.0),
                        (2.5, -4.0),
                        (-2.5, 0.0),
                        (2.5, 0.0),
                        (-2.5, 4.0),
                        (2.5, 4.0),
                    ] {
                        ui.painter().circle_filled(grip.center() + egui::vec2(dx, dy), 1.2, dot);
                    }
                    let handle = handle.on_hover_cursor(egui::CursorIcon::Grab);
                    let number = egui::RichText::new(format!("{:>2}", i + 1)).weak().monospace();
                    ui.label(number);
                    let chosen = state.selected.contains(&i);
                    let name = ui.add(
                        egui::Button::selectable(chosen, egui::RichText::new(h).monospace())
                            .sense(egui::Sense::click_and_drag()),
                    );
                    if missing(h) {
                        ui.colored_label(ui.visuals().warn_fg_color, "not found").on_hover_text(
                            "No hak of this name is in the hak folders: the game will not \
                             load the module until there is, or it is removed here",
                        );
                    }
                    if state.reveal && state.cursor == Some(i) {
                        name.scroll_to_me(None);
                    }
                    if name.clicked() {
                        let (command, shift) =
                            ui.input(|i| (i.modifiers.command, i.modifiers.shift));
                        // The rule of every list of several (Ctrl+Shift
                        // adds the range, as Windows' lists do).
                        let listed: Vec<usize> = (0..n).collect();
                        crate::widgets::select_in_list(
                            &mut state.selected,
                            &mut state.anchor,
                            &listed,
                            i,
                            command,
                            shift,
                        );
                        state.cursor = Some(i);
                        // (The arrow keys are the list's now, not a camera's.)
                        crate::palette_view::give_keys(ui.ctx(), crate::palette_view::Keys::List);
                    }
                    if name.drag_started() || handle.drag_started() {
                        // A row not among those chosen is dragged alone.
                        if !state.selected.contains(&i) {
                            state.selected = BTreeSet::from([i]);
                            state.anchor = Some(i);
                            state.cursor = Some(i);
                        }
                        egui::DragAndDrop::set_payload(ui.ctx(), HakDrag(i));
                    }
                });
                rects.push(row.response.rect);
            }
            // A row dragged near the list's top or bottom edge, or past
            // it, scrolls the list, faster the nearer (as list controls
            // do): to put it among rows out of sight.
            if egui::DragAndDrop::has_payload_of_type::<HakDrag>(ui.ctx())
                && let Some(pointer) = ui.ctx().pointer_latest_pos()
            {
                let seen = ui.clip_rect();
                let speed = edge_scroll(seen.y_range(), pointer.y, row_height);
                if speed != 0.0 && seen.x_range().contains(pointer.x) {
                    let dt = ui.input(|i| i.stable_dt).min(0.1);
                    ui.scroll_with_delta(egui::vec2(0.0, speed * dt));
                    ui.ctx().request_repaint();
                }
            }
            // A row dragged: the line where it would go, and there when
            // it is let go.
            if egui::DragAndDrop::has_payload_of_type::<HakDrag>(ui.ctx())
                && let Some(pointer) = ui.ctx().pointer_latest_pos()
                && let (Some(first), Some(last)) = (rects.first(), rects.last())
                && (first.top() - row_height..=last.bottom() + row_height).contains(&pointer.y)
            {
                let before = rects.iter().position(|r| pointer.y < r.center().y).unwrap_or(n);
                let y = rects.get(before).map_or(last.bottom(), |r| r.top());
                let stroke = egui::Stroke::new(2.0, ui.visuals().selection.bg_fill);
                ui.painter().hline(first.left()..=ui.clip_rect().right(), y, stroke);
                if ui.input(|i| i.pointer.any_released()) {
                    egui::DragAndDrop::clear_payload(ui.ctx());
                    move_to = Some(before);
                }
            }
        });
    });
    // The keys, after a click in the list and with the pointer over it:
    // Up and Down choose (with Shift, more), Alt + Up and Down move what is
    // chosen, Delete removes it, Home and End go to the ends.
    let over = ui.rect_contains_pointer(frame.response.rect);
    let typing = ui.ctx().egui_wants_keyboard_input();
    if over && n > 0 && !typing && crate::palette_view::has_arrows(ui.ctx()) {
        use egui::Key;
        let (up, down, home, end, delete, alt, shift) = ui.input(|i| {
            (
                i.key_pressed(Key::ArrowUp),
                i.key_pressed(Key::ArrowDown),
                i.key_pressed(Key::Home),
                i.key_pressed(Key::End),
                i.key_pressed(Key::Delete),
                i.modifiers.alt,
                i.modifiers.shift,
            )
        });
        let (low, high) = (state.selected.first().copied(), state.selected.last().copied());
        if alt && (up || down) {
            match (up, low, high) {
                (true, Some(low), _) if low > 0 => move_to = Some(low - 1),
                (false, _, Some(high)) if high + 1 < n => move_to = Some(high + 2),
                _ => {}
            }
        } else if up || down || home || end {
            let at = state.cursor.or(if up { low } else { high });
            let to = match (home, end, at) {
                (true, ..) => 0,
                (_, true, _) => n - 1,
                (_, _, None) => 0,
                (_, _, Some(at)) if up => at.saturating_sub(1),
                (_, _, Some(at)) => (at + 1).min(n - 1),
            };
            match state.anchor.filter(|_| shift) {
                Some(a) => state.selected = (a.min(to)..=a.max(to)).collect(),
                None => {
                    state.selected = BTreeSet::from([to]);
                    state.anchor = Some(to);
                }
            }
            state.cursor = Some(to);
            state.reveal = true;
            ui.ctx().request_repaint();
        } else if delete && !state.selected.is_empty() {
            remove = true;
        }
    } else {
        state.reveal = false;
    }

    let available: Vec<&String> = (on_disk.iter().flat_map(|all| all.iter()))
        .filter(|h| !haks.iter().any(|x| x.eq_ignore_ascii_case(h)))
        .collect();
    // The list's commands, under it: what adds to it and checks it, and
    // what is done with the rows chosen.
    ui.horizontal(|ui| {
        // (Commands, not a choice: the arrows move along them, Enter adds.)
        crate::widgets::dropdown("ifo-add-hak").selected_text("Add hak…").show_menu(
            ui,
            |ui, keys| {
                for h in &available {
                    let r = ui.selectable_label(false, h.as_str());
                    if keys.row(&r) || r.clicked() {
                        let mut item = ifo::MOD_HAK_LIST.new_item();
                        item.write(&ifo::mod_hak_list::MOD_HAK, ExoString(encode(h)));
                        let insert = Edit::insert_item(
                            info_key(),
                            GffPath::root(),
                            list(),
                            haks.len(),
                            item,
                        );
                        app.actions.push(command(vec![insert], "Add hak"));
                    }
                }
            },
        );
        if ui
            .button("Add Haks and Talk Table…")
            .on_hover_text(
                "Haks and a talk table from anywhere: copied into your hak and tlk folders \
                 and attached to the module in one step",
            )
            .clicked()
        {
            start_attach(app);
        }
        if ui.add_enabled(!haks.is_empty(), egui::Button::new("Check for Conflicts…")).clicked() {
            app.actions.push(Action::HakReport);
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let (low, high) = (state.selected.first().copied(), state.selected.last().copied());
            let some = !state.selected.is_empty();
            if ui
                .add_enabled(some, egui::Button::new("Remove"))
                .on_hover_text("Take the haks chosen off the list (Delete)")
                .clicked()
            {
                remove = true;
            }
            let can_down = high.is_some_and(|h| h + 1 < n);
            if ui
                .add_enabled(can_down, egui::Button::new("⏷"))
                .on_hover_text("Move the haks chosen down (Alt + Down)")
                .clicked()
            {
                move_to = high.map(|h| h + 2);
            }
            let can_up = low.is_some_and(|l| l > 0);
            if ui
                .add_enabled(can_up, egui::Button::new("⏶"))
                .on_hover_text("Move the haks chosen up (Alt + Up)")
                .clicked()
            {
                move_to = low.map(|l| l - 1);
            }
        });
    });
    ui.weak(
        "Drag rows to reorder them (Ctrl or Shift + click chooses more), or click the list and \
         use the keys: Up and Down, Alt + Up and Down to move, Delete to remove.",
    );
    // One undoable step for a move or a removal, of all the rows chosen.
    let rewritten = |order: &[usize]| Edit::SetField {
        key: info_key(),
        path: GffPath::root(),
        label: list(),
        value: Some(Value::List(order.iter().map(|i| items[*i].clone()).collect())),
    };
    if let Some(before) = move_to.filter(|_| !state.selected.is_empty()) {
        let (order, now) = moved(n, &state.selected, before);
        if order.iter().copied().ne(0..n) {
            let what = if state.selected.len() == 1 { "Move hak" } else { "Move haks" };
            app.actions.push(command(vec![rewritten(&order)], what));
            state.cursor = now.first().copied();
            state.anchor = state.cursor;
            state.selected = now;
            state.reveal = true;
        }
    } else if remove {
        let kept: Vec<usize> = (0..n).filter(|i| !state.selected.contains(i)).collect();
        let what = if state.selected.len() == 1 { "Remove hak" } else { "Remove haks" };
        app.actions.push(command(vec![rewritten(&kept)], what));
        // (The row after those removed is at hand.)
        let next = state.selected.first().copied().filter(|i| *i < kept.len());
        state.selected = next.into_iter().collect();
        state.cursor = next;
        state.anchor = next;
    }
    app.hak_list = state;

    ui.add_space(crate::widgets::SECTION_GAP);
    crate::widgets::section_heading(ui, "Talk Table");
    ui.horizontal(|ui| {
        crate::widgets::field_label(ui, "Custom TLK");
        let tlk = decode(root.read(&ifo::MOD_CUSTOM_TLK).as_bytes());
        let shown = if tlk.is_empty() { "(none)".to_string() } else { tlk.clone() };
        // (Read only while the list is used: open, or with the keys.) The
        // game reads talk tables from haks and the module too. The one
        // chosen as the module names it.
        let tlks = || {
            let mut tlks = (app.install.as_ref())
                .map(|i| GameInstall::file_names(&i.tlk_dirs(), "tlk"))
                .unwrap_or_default();
            if let Some(game) = &app.game {
                tlks.extend(game.resman.list(mg_core::ResType::TLK).iter().map(|r| r.to_string()));
                tlks.sort();
                tlks.dedup();
            }
            let named = |t: String| if t.eq_ignore_ascii_case(&tlk) { tlk.clone() } else { t };
            std::iter::once(String::new()).chain(tlks.into_iter().map(named)).collect::<Vec<_>>()
        };
        let mut pick = tlk.clone();
        crate::widgets::dropdown("ifo-tlk").selected_text(shown).show_with(
            ui,
            &mut pick,
            tlks,
            |ui, pick| {
                for t in tlks() {
                    let name = if t.is_empty() { "(none)".to_string() } else { t.clone() };
                    ui.selectable_value(pick, t, name);
                }
            },
        );
        if !pick.eq_ignore_ascii_case(&tlk) {
            app.actions.push(set("Custom TLK", &ifo::MOD_CUSTOM_TLK, ExoString(encode(&pick))));
        }
        let label = if tlk.is_empty() { "New…" } else { "Edit…" };
        if ui.button(label).on_hover_text("Tools › Talk Table").clicked() {
            app.actions.push(Action::OpenTab(crate::Tab::TalkTable));
        }
    });
}

/// Add Haks and Talk Table: the files chosen, where each goes, and whether
/// to replace other files of the same names there.
#[derive(Debug, Clone)]
pub struct AttachDraft {
    pub placements: Vec<mg_module::attach::Placement>,
    pub replace: bool,
}

fn start_attach(app: &mut Moonglow) {
    let Some(user) = app.install.as_ref().and_then(|i| i.user_dir.clone()) else {
        app.log.error("Adding haks needs the game's user folder (Tools › Options)");
        return;
    };
    let files = app.dialogs.open_files(crate::dialogs::FileKind::Content, None);
    if files.is_empty() {
        return;
    }
    match mg_module::attach::placements(&user, &files) {
        Ok(placements) => app.attach = Some(AttachDraft { placements, replace: false }),
        Err(e) => app.log.error(format!("Add Haks and Talk Table: {e}")),
    }
}

/// The Add Haks and Talk Table window: the haks in their order (highest
/// first), the talk table, and what's copied where.
pub(crate) fn attach_window(app: &mut Moonglow, ctx: &egui::Context) {
    use mg_module::attach::There;
    let Some(mut draft) = app.attach.take() else { return };
    let mut done = None;
    crate::widgets::modal(ctx, "Add Haks and Talk Table").show(ctx, |ui| {
        ui.label("Haks go at the top of the module's list, highest priority first:");
        let n = draft.placements.len();
        let mut swap = None;
        for (i, p) in draft.placements.iter().enumerate() {
            ui.horizontal(|ui| {
                ui.monospace(if p.is_tlk {
                    format!("{}.tlk", p.name)
                } else {
                    format!("{}.hak", p.name)
                });
                let folder = if p.is_tlk { "tlk" } else { "hak" };
                let what = match p.there {
                    There::Nothing => format!("copied to your {folder} folder"),
                    There::Same => format!("already in your {folder} folder"),
                    There::Different => format!("a different one is in your {folder} folder"),
                };
                let from = match &p.entry {
                    Some(entry) => format!("{entry} in {}", p.from.display()),
                    None => p.from.display().to_string(),
                };
                ui.weak(what).on_hover_text(format!(
                    "{from}
→ {}",
                    p.to.display()
                ));
                if !p.is_tlk {
                    let next_hak = (i + 1..n).find(|&j| !draft.placements[j].is_tlk);
                    let prev_hak = (0..i).rev().find(|&j| !draft.placements[j].is_tlk);
                    if ui
                        .add_enabled(prev_hak.is_some(), egui::Button::new("Move Up").small())
                        .clicked()
                    {
                        swap = prev_hak.map(|j| (i, j));
                    }
                    if ui
                        .add_enabled(next_hak.is_some(), egui::Button::new("Move Down").small())
                        .clicked()
                    {
                        swap = next_hak.map(|j| (i, j));
                    }
                } else {
                    ui.weak("(the module's talk table)");
                }
            });
        }
        if let Some((a, b)) = swap {
            draft.placements.swap(a, b);
        }
        if draft.placements.iter().any(|p| p.there == There::Different) {
            ui.checkbox(&mut draft.replace, "Replace the files of the same names that are there");
        }
        ui.horizontal(|ui| {
            if ui.button("OK").clicked() {
                done = Some(true);
            }
            if crate::widgets::cancel(ui) {
                done = Some(false);
            }
        });
    });
    match done {
        None => app.attach = Some(draft),
        Some(false) => {}
        Some(true) => attach(app, &draft),
    }
}

/// Copies the files and attaches them, as one undoable step (the copies
/// stay).
fn attach(app: &mut Moonglow, draft: &AttachDraft) {
    let copied = match mg_module::attach::copy(&draft.placements, draft.replace) {
        Ok(n) => n,
        Err(e) => {
            app.log.error(format!("Add Haks and Talk Table: {e}"));
            return;
        }
    };
    let Some(ws) = &mut app.ws else { return };
    let Ok(info) = ws.doc(&info_key()) else { return };
    let haks: Vec<String> =
        draft.placements.iter().filter(|p| !p.is_tlk).map(|p| p.name.clone()).collect();
    let mut edits = Vec::new();
    if !haks.is_empty() {
        edits.push(Edit::SetField {
            key: info_key(),
            path: GffPath::root(),
            label: ifo::MOD_HAK_LIST.label.to_string(),
            value: Some(mg_module::attach::hak_list(&info.root, &haks)),
        });
    }
    if let Some(t) = draft.placements.iter().find(|p| p.is_tlk) {
        edits.push(Edit::SetField {
            key: info_key(),
            path: GffPath::root(),
            label: ifo::MOD_CUSTOM_TLK.label.to_string(),
            value: Some(ExoString(encode(&t.name)).into_value()),
        });
    }
    app.log.info(format!("Copied {copied} files into the user folder"));
    app.actions.push(Action::Apply(Command::new("Add haks and talk table", edits)));
}

#[cfg(test)]
mod tests {
    /// Rows chosen move together to stand before another, in their order.
    #[test]
    fn chosen_rows_move_together() {
        use std::collections::BTreeSet;
        let set = |rows: &[usize]| rows.iter().copied().collect::<BTreeSet<usize>>();
        // One row up, down, to the top and to the end.
        assert_eq!(super::moved(4, &set(&[2]), 1), (vec![0, 2, 1, 3], set(&[1])));
        assert_eq!(super::moved(4, &set(&[1]), 3), (vec![0, 2, 1, 3], set(&[2])));
        assert_eq!(super::moved(4, &set(&[3]), 0), (vec![3, 0, 1, 2], set(&[0])));
        assert_eq!(super::moved(4, &set(&[0]), 4), (vec![1, 2, 3, 0], set(&[3])));
        // Two that are apart: together, where they are let go.
        assert_eq!(super::moved(5, &set(&[0, 3]), 2), (vec![1, 0, 3, 2, 4], set(&[1, 2])));
        // Let go on itself: as it was.
        assert_eq!(super::moved(4, &set(&[1, 2]), 2).0, [0, 1, 2, 3]);
        assert_eq!(super::moved(4, &set(&[1]), 1).0, [0, 1, 2, 3]);
        assert_eq!(super::moved(4, &set(&[1]), 2).0, [0, 1, 2, 3]);
    }

    /// A row dragged near an edge of the list scrolls it that way, faster
    /// the nearer and past it, up to a most.
    #[test]
    fn a_drag_near_an_edge_scrolls() {
        let seen = egui::Rangef::new(100.0, 400.0);
        let speed = |y| super::edge_scroll(seen, y, 20.0);
        assert_eq!(speed(250.0), 0.0, "in the middle");
        assert_eq!(speed(120.0), 0.0, "a row from the top");
        assert!(speed(110.0) > 0.0, "near the top: up");
        assert!(speed(395.0) < 0.0, "near the bottom: down");
        assert!(speed(100.0) > speed(110.0), "faster at the edge");
        assert!(speed(60.0) > speed(100.0), "and past it");
        assert_eq!(speed(0.0), speed(-500.0), "up to a most");
        assert_eq!(speed(395.0), -speed(105.0));
    }

    /// The hak folders are listed for Custom Content once a couple of
    /// seconds, not every frame: a hak copied in shows then.
    #[test]
    fn the_hak_folders_are_read_again_after_a_while() {
        let dir = mg_testkit::scratch_dir("ui-hak-folders");
        let user = dir.join("user");
        std::fs::create_dir_all(user.join("hak")).unwrap();
        std::fs::write(user.join("hak/first.hak"), "").unwrap();
        let mut app = crate::Moonglow::new(None, Box::new(crate::NoDialogs::default()));
        assert!(super::haks_on_disk(&mut app, 0.0).is_none(), "no install");
        app.install = Some(mg_resman::GameInstall::new(dir.join("game"), Some(user.clone()), "en"));
        let listed =
            |app: &mut crate::Moonglow, now| super::haks_on_disk(app, now).unwrap().to_vec();
        assert_eq!(listed(&mut app, 1.0), ["first"]);
        std::fs::write(user.join("hak/second.hak"), "").unwrap();
        assert_eq!(listed(&mut app, 2.5), ["first"], "kept a while");
        assert_eq!(listed(&mut app, 3.0), ["first", "second"]);
    }
}
