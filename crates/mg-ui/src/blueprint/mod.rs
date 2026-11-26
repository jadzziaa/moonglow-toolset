//! Blueprint editors (Aurora's object Properties dialogs) for the module's
//! blueprints, one tab each. A [`Form`] lays out a blueprint's fields; each
//! change is one undoable command on the blueprint, fields keep the integer
//! type they are stored with, and fields the editor does not show are kept.

use mg_uikit::Controls as _;
use std::collections::HashMap;
use std::sync::Arc;

use egui::Ui;
use mg_core::{ResRef, ResType};
use mg_edit::{Command, Edit, GffPath, lists};
use mg_gff::{FieldType, Struct, Value};
use mg_module::palette::BlueprintKind;
use mg_resman::ResKey;
use mg_rules::Choice;

use crate::dialogs::FileKind;
use crate::text::{decode, encode, with_edited_text};
use crate::widgets::{FieldTarget, LocStringEdit, VarTableEdit, commit_number, resref_field};
use crate::{Action, Moonglow, Tab};

mod creature;
pub(crate) use creature::{with_a_body, with_its_portrait};
mod creature_lists;
mod door;
pub(crate) mod encounter;
pub(crate) mod follow;
mod inventory;
mod inventory_together;
mod item;
pub(crate) mod picker;
mod placeable;
mod replacements;
pub(crate) mod situated;
mod sound;
pub(crate) mod store;
pub(crate) mod tint;
mod trigger;
mod visuals;
mod waypoint;

/// Whether a blueprint type has its own editor.
pub(crate) fn has_editor(t: ResType) -> bool {
    BlueprintKind::from_restype(t).is_some()
}

/// The pages of a blueprint type's editor.
pub fn pages(t: ResType) -> &'static [&'static str] {
    match t {
        ResType::UTW => &waypoint::PAGES,
        ResType::UTS => &sound::PAGES,
        ResType::UTT => &trigger::PAGES,
        ResType::UTE => &encounter::PAGES,
        ResType::UTM => &store::PAGES,
        ResType::UTD => &door::PAGES,
        ResType::UTP => &placeable::PAGES,
        ResType::UTI => &item::PAGES,
        ResType::UTC => &creature::PAGES,
        _ => &[""],
    }
}

/// Each open editor's page, by document and the object's path in it.
pub type Pages = HashMap<(ResKey, GffPath), &'static str>;

/// What an open window with OK and Cancel has changed, for its Cancel.
/// By the window's first object (a blueprint, or the document an editor
/// is of), as [`Pages`].
pub type Changed = HashMap<(ResKey, GffPath), WindowChanges>;

/// One window's entry of [`Changed`].
#[derive(Debug, Clone)]
pub struct WindowChanges {
    /// The window, to tell when it is closed.
    pub tab: Tab,
    /// What it changed, in the order it first changed each.
    pub was: Vec<Was>,
    /// Noted before the window opened (a blueprint just made,
    /// [`note_made`]): kept while it is about to, until it is drawn.
    pub opening: bool,
}

/// A thing a window changed, as it was before the window's first change
/// to it.
#[derive(Debug, Clone, PartialEq)]
pub enum Was {
    /// A field of one of the window's objects (a list changed in: the
    /// list): the object's document and path, the field, and what it held
    /// (`None`: no such field).
    Field(ResKey, GffPath, String, Option<Value>),
    /// A whole resource a command of the window's replaced (the journal
    /// made, the factions' file, what a removed faction renumbered): its
    /// bytes (`None`: there was none).
    Resource(ResKey, Option<Vec<u8>>),
}

/// A Properties window's text field that has the keyboard
/// ([`Moonglow::held_text`]).
#[derive(Debug, Clone)]
pub(crate) struct HeldText {
    /// The field.
    pub id: egui::Id,
    /// The frame it was last drawn in.
    pub drawn: u64,
    /// What it would set if left now (`None`: nothing typed).
    pub command: Option<Command>,
    /// What it would ask for if left now (`None`: a command or nothing typed).
    pub action: Option<Action>,
    /// It is a localized string's text: set as text typed is
    /// (`Action::ApplyTyped`).
    pub typed: bool,
    /// The window it is in, for its Cancel: set once that is drawn.
    pub window: Option<HeldIn>,
    /// It is the Properties panel's (no window's, no Cancel's): a window
    /// drawn after the panel in that frame does not claim it.
    pub panel: bool,
}

/// The window a [`HeldText`] is in: what its Cancel is given to note
/// ([`window_drawn_but`]'s `tab`, `targets` and `kept`).
#[derive(Debug, Clone)]
pub(crate) struct HeldIn {
    pub tab: Tab,
    pub targets: Vec<(ResKey, GffPath)>,
    pub kept: Vec<ResKey>,
}

/// The field of the object at `form` that an edit at `path` (the object's
/// own, or further in) changes, as its window's Cancel sets it back: the
/// struct it is in and its label. In a struct of the object's (an area's
/// sounds and music, its GIT's `AreaProperties`), the field itself: its
/// others, changed elsewhere, are not set back with it. In a list (an
/// item it holds), the list: its items may come and go meanwhile.
fn field_changed(form: &GffPath, path: &GffPath, label: &str) -> Option<(GffPath, String)> {
    if !path.0.starts_with(&form.0) {
        return None;
    }
    let mut at = form.clone();
    for step in &path.0[form.0.len()..] {
        match step {
            mg_edit::Step::Field(l) => at = at.field(l),
            mg_edit::Step::Item(l, _) => return Some((at, l.clone())),
        }
    }
    Some((at, label.to_string()))
}

/// Whether a command reaches some of `targets` (each a document and the
/// path of an object in it) but not all: an edit reaches an object when it
/// is in the object's document at its path or further in (an item it
/// holds), or replaces the document whole. Placed objects share their
/// area's GIT, so their documents alone don't tell them apart.
fn partial(targets: &[(ResKey, GffPath)], cmd: &Command) -> bool {
    let reaches = |(key, at): &(ResKey, GffPath)| {
        cmd.edits.iter().any(|e| match e {
            Edit::SetField { key: k, path, .. }
            | Edit::InsertItem { key: k, path, .. }
            | Edit::RemoveItem { key: k, path, .. } => k == key && path.0.starts_with(&at.0),
            Edit::SetResource { key: k, .. } => k == key,
        })
    };
    let reached = targets.iter().filter(|t| reaches(t)).count();
    reached > 0 && reached < targets.len()
}

/// Notes what a window's command is about to change (it is not applied
/// yet), for the window's Cancel: the fields of the window's objects
/// (`targets`, the first of which names the window) and the resources it
/// replaces whole.
pub(crate) fn note_command(
    app: &mut Moonglow,
    tab: &Tab,
    targets: &[(ResKey, GffPath)],
    cmd: &Command,
) {
    let Some((first, ws)) = targets.first().zip(app.ws.as_mut()) else { return };
    for edit in &cmd.edits {
        let (key, path, label) = match edit {
            Edit::SetField { key, path, label, .. } => (key, path, label),
            Edit::InsertItem { key, path, list, .. } | Edit::RemoveItem { key, path, list, .. } => {
                (key, path, list)
            }
            Edit::SetResource { key, .. } => {
                let entry = app.properties_changed.entry(first.clone()).or_insert_with(|| {
                    WindowChanges { tab: tab.clone(), was: Vec::new(), opening: false }
                });
                if !entry.was.iter().any(|w| matches!(w, Was::Resource(k, _) if k == key)) {
                    // (As the open documents have it, not as last saved.)
                    let _ = ws.flush();
                    entry.was.push(Was::Resource(*key, ws.module.get(key).map(<[u8]>::to_vec)));
                }
                continue;
            }
        };
        // (The deepest of the window's objects the edit is in: an item a
        // placed creature holds is not edited with it.)
        let within = targets
            .iter()
            .filter(|(k, form)| k == key && path.0.starts_with(&form.0))
            .max_by_key(|(_, form)| form.0.len());
        let Some((k, form)) = within else { continue };
        let Some((at, field)) = field_changed(form, path, label) else { continue };
        let entry = app.properties_changed.entry(first.clone()).or_insert_with(|| WindowChanges {
            tab: tab.clone(),
            was: Vec::new(),
            opening: false,
        });
        let known = |w: &Was| match w {
            Was::Field(wk, wp, wl, _) => wk == k && *wp == at && *wl == field,
            // (Set back whole: its fields with it.)
            Was::Resource(wk, _) => wk == k,
        };
        if !entry.was.iter().any(known) {
            let before = ws.doc(k).ok().and_then(|g| at.get(&g.root)?.get(&field).cloned());
            entry.was.push(Was::Field(*k, at, field, before));
        }
    }
}

/// Notes a command that makes the blueprint `key` (not applied yet) as the
/// first change of the blueprint's window, which opens on it at once
/// (Edit Copy, Add to Palette): its Cancel (and Escape) takes back the
/// blueprint (there was none: it is removed) with what else the command
/// changed, as Aurora's window, which makes the blueprint only at its OK,
/// leaves nothing. `also`: the objects of other documents it changes (the
/// placed object Add to Palette makes name it), whose fields it sets back;
/// anything else it changes is set back by the field (or list) of the
/// document's root it is in.
pub(crate) fn note_made(
    app: &mut Moonglow,
    key: ResKey,
    also: &[(ResKey, GffPath)],
    cmd: &Command,
) {
    let mut targets = vec![(key, GffPath::root())];
    targets.extend(also.iter().cloned());
    for edit in &cmd.edits {
        if !targets.iter().any(|(k, p)| k == edit.key() && p.0.is_empty()) {
            targets.push((*edit.key(), GffPath::root()));
        }
    }
    // (Nothing of a window of that name before is its.)
    app.properties_changed.remove(&targets[0]);
    note_command(app, &Tab::Blueprint(key), &targets, cmd);
    if let Some(changes) = app.properties_changed.get_mut(&targets[0]) {
        changes.opening = true;
    }
}

/// [`note_command`] for the commands a window asked for this frame
/// (`app.actions` from `pending` on), but for what they do to the
/// resources `kept`, which the window's Cancel leaves as they are.
fn note_changes(
    app: &mut Moonglow,
    pending: usize,
    tab: &Tab,
    targets: &[(ResKey, GffPath)],
    kept: &[ResKey],
) {
    let asked: Vec<Command> = app.actions[pending.min(app.actions.len())..]
        .iter()
        .filter_map(|a| match a {
            Action::Apply(cmd) | Action::ApplyTyped(cmd) => Some(but_for(cmd, kept)),
            _ => None,
        })
        .collect();
    for cmd in &asked {
        note_command(app, tab, targets, cmd);
    }
}

/// A command without its edits of the resources `kept`.
pub(crate) fn but_for(cmd: &Command, kept: &[ResKey]) -> Command {
    let mut cmd = cmd.clone();
    cmd.edits.retain(|e| !kept.contains(e.key()));
    cmd
}

/// The command that sets back what a window changed, where it is not as
/// it was: `None` with nothing to set back. With `all`, every field the
/// window changed whatever it holds now: a change asked for this frame is
/// not made yet, and is to be set back once it is.
fn cancel_command(app: &mut Moonglow, first: &(ResKey, GffPath), all: bool) -> Option<Command> {
    let changes = app.properties_changed.get(first)?;
    let ws = app.ws.as_mut()?;
    if changes.was.iter().any(|w| matches!(w, Was::Resource(..))) {
        let _ = ws.flush();
    }
    let mut edits = Vec::new();
    // (The last noted first: a resource set back whole, then the fields
    // of it the window had changed before that.)
    for was in changes.was.iter().rev() {
        match was {
            Was::Field(key, path, label, before) => {
                let now = ws.doc(key).ok().and_then(|g| path.get(&g.root)?.get(label).cloned());
                if all || now != *before {
                    edits.push(Edit::SetField {
                        key: *key,
                        path: path.clone(),
                        label: label.clone(),
                        value: before.clone(),
                    });
                }
            }
            Was::Resource(key, before) => {
                if all || ws.module.get(key) != before.as_deref() {
                    edits.push(Edit::SetResource { key: *key, data: before.clone() });
                }
            }
        }
    }
    (!edits.is_empty()).then(|| Command::new("Cancel Changes", edits))
}

/// What was pressed under a window.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Pressed {
    Ok,
    Cancel,
}

/// OK and Cancel footer strip used by the standalone GFF editor (`gff_view`);
/// Properties windows use [`ok_cancel_header`] at the top instead. OK closes
/// the window (changes are already in the module); Cancel reverts everything
/// changed since opening as one undo step and closes it. Read-only blueprints
/// show Close alone.
pub(crate) fn ok_cancel(ui: &mut Ui, first: &(ResKey, GffPath), viewed: bool) -> Option<Pressed> {
    ok_cancel_saying(ui, egui::Id::new(first), viewed, OK_CANCEL_TIPS)
}

/// [`ok_cancel`]'s tips: OK's and Cancel's.
pub(crate) const OK_CANCEL_TIPS: [&str; 2] = [
    "Close the window: its changes are kept",
    "Close the window and set back its changes, as one Undo step",
];

/// [`ok_cancel`] for a window that says what its two buttons do itself
/// (`tips`: OK's and Cancel's).
pub(crate) fn ok_cancel_saying(
    ui: &mut Ui,
    id: egui::Id,
    viewed: bool,
    tips: [&str; 2],
) -> Option<Pressed> {
    let mut pressed = None;
    egui::Panel::bottom(id.with("ok-cancel"))
        .frame(egui::Frame::NONE.inner_margin(egui::Margin::symmetric(0, 4)))
        .show_separator_line(true)
        .show(ui, |ui| {
            row(ui, |ui| {
                if viewed {
                    if ui.button("Close").clicked() {
                        pressed = Some(Pressed::Ok);
                    }
                    return;
                }
                if ui.button("OK").on_hover_text(tips[0]).clicked() {
                    pressed = Some(Pressed::Ok);
                }
                let cancel = ui.button("Cancel").on_hover_text(tips[1]);
                if cancel.clicked() {
                    pressed = Some(Pressed::Cancel);
                }
            });
        });
    pressed
}

/// What a form window's header shows at its left (the Creature board's
/// `.bp-title`): its kind's icon, its name and, quieter, its resref (or
/// what stands for one).
pub(crate) struct Title<'t> {
    /// The header's accessible name ("Creature blueprint header").
    pub header: String,
    pub icon: mg_uikit::Icon,
    pub name: &'t str,
    pub sub: &'t str,
}

/// Header-bar variant of [`ok_cancel_saying`]: title on the left and OK/Cancel
/// (or Close when read-only) on the right, with the same tips and [`Pressed`]
/// result.
pub(crate) fn ok_cancel_header(
    ui: &mut Ui,
    title: &Title<'_>,
    viewed: bool,
    tips: [&str; 2],
) -> Option<Pressed> {
    ok_cancel_header_with(ui, title, viewed, tips, false).0
}

/// [`ok_cancel_header`], optionally with a `Pop In` button before `OK`/`Cancel`
/// (shown when an instance Properties editor is docked as a main-surface tab).
fn ok_cancel_header_with(
    ui: &mut Ui,
    title: &Title<'_>,
    viewed: bool,
    tips: [&str; 2],
    pop_in: bool,
) -> (Option<Pressed>, bool) {
    use mg_uikit::KitButton;
    let font = egui::FontId::proportional(mg_uikit::metrics::BODY_SIZE);
    let shape_plain = |text: &str| {
        ui.painter().layout_no_wrap(text.to_string(), font.clone(), egui::Color32::PLACEHOLDER)
    };
    const GAP: f32 = 8.0;
    let mut buttons: Vec<KitButton> = Vec::new();
    if pop_in {
        buttons.push(
            KitButton::new(Some(crate::icons::POP_IN), "Pop In")
                .tip("Put these properties back in the Properties panel"),
        );
    }
    match viewed {
        true => buttons.push(KitButton::new(None, "Close")),
        false => {
            buttons
                .push(KitButton::new(None, "OK").look(mg_uikit::ButtonLook::Primary).tip(tips[0]));
            buttons.push(KitButton::new(None, "Cancel").tip(tips[1]));
        }
    }
    let gap = mg_uikit::header::HEADER_GAP;
    let end_width = crate::widgets::buttons_end_width(ui, &buttons);
    // The title yields where the row is short (the style guide's
    // ellipsis): the resref first, then the name cut short, the icon kept;
    // its accessible name the whole of it.
    let room = (ui.available_width()
        - 2.0 * mg_uikit::header::HEADER_PADDING
        - end_width
        - 2.0 * gap
        - 2.0)
        .max(0.0);
    let frame = 4.0 + mg_uikit::metrics::ICON + GAP + 4.0;
    let name_galley = shape_plain(title.name);
    let name_w = name_galley.size().x;
    let sub_galley = (!title.sub.is_empty()).then(|| shape_plain(title.sub));
    let sub_w = sub_galley.as_ref().map_or(0.0, |g| GAP + g.size().x);
    let (shown_sub, name_room) = match frame + name_w + sub_w <= room {
        true => (sub_galley, name_w),
        false => (None, (room - frame).min(name_w).max(0.0)),
    };
    let width = frame + name_room + shown_sub.as_ref().map_or(0.0, |g| GAP + g.size().x);
    let (icon, name, sub) = (title.icon, title.name.to_string(), title.sub.to_string());
    let fits_whole = name_w <= name_room;
    let item = mg_uikit::header::Item::custom("Title", width.ceil(), move |ui| {
        ui.spacing_mut().item_spacing.x = GAP;
        ui.add_space(4.0);
        let t = mg_uikit::tokens(ui.ctx());
        let (rect, response) = ui
            .allocate_exact_size(egui::Vec2::splat(mg_uikit::metrics::ICON), egui::Sense::hover());
        icon.paint(ui.painter(), rect, t.text);
        // (Painted; one accessible label, the whole title, on the icon.)
        let name_info = name.clone();
        response.widget_info(move || {
            let whole =
                if sub.is_empty() { name_info.clone() } else { format!("{name_info} {sub}") };
            egui::WidgetInfo::labeled(egui::WidgetType::Label, true, &whole)
        });
        let galley = if fits_whole {
            name_galley
        } else {
            let mut job = egui::text::LayoutJob::simple_singleline(name, font.clone(), t.text);
            job.wrap = egui::text::TextWrapping::truncate_at_width(name_room.ceil() + 0.5);
            ui.painter().layout_job(job)
        };
        let (rect, _) = ui.allocate_exact_size(galley.size(), egui::Sense::hover());
        ui.painter().galley(rect.min, galley, t.text);
        if let Some(galley) = shown_sub {
            let (rect, _) = ui.allocate_exact_size(galley.size(), egui::Sense::hover());
            ui.painter().galley(rect.min, galley, t.text_3);
        }
    });
    let out = mg_uikit::header::EditorHeader::new(title.header.clone())
        .left(item)
        .end_separator(false)
        .show_with_end(ui, end_width, |ui| {
            ui.spacing_mut().item_spacing.x = gap;
            let mut pressed = None;
            let mut popped_in = false;
            for b in buttons {
                let words = b.accessible_name().to_string();
                if ui.add(b).clicked() {
                    if words == "Pop In" {
                        popped_in = true;
                    } else {
                        pressed =
                            Some(if words == "Cancel" { Pressed::Cancel } else { Pressed::Ok });
                    }
                }
            }
            (pressed, popped_in)
        });
    out.end
}

/// After a panel of an editor with OK and Cancel is drawn, before the
/// editor (the journal's Entry and Category, M4's U22): what it asked for
/// this frame (`app.actions` from `pending` on) noted for the editor's
/// Cancel (`tab`, its `targets`), and a text field of it with the
/// keyboard the editor's, as a field of the editor's own is
/// ([`window_drawn_but`]).
pub(crate) fn panel_drawn(
    app: &mut Moonglow,
    ctx: &egui::Context,
    pending: usize,
    tab: &Tab,
    targets: &[(ResKey, GffPath)],
) {
    note_changes(app, pending, tab, targets, &[]);
    let pass = ctx.cumulative_pass_nr();
    if let Some(held) = app.held_text.as_mut().filter(|h| h.window.is_none() && h.drawn == pass) {
        let (tab, targets) = (tab.clone(), targets.to_vec());
        held.window = Some(HeldIn { tab, targets, kept: Vec::new() });
    }
}

/// After a window with [`ok_cancel`] is drawn: notes what it asked for
/// this frame (`app.actions` from `pending` on), and closes it if OK or
/// Cancel was pressed.
pub(crate) fn window_drawn(
    app: &mut Moonglow,
    pending: usize,
    tab: &Tab,
    targets: &[(ResKey, GffPath)],
    pressed: Option<Pressed>,
) {
    window_drawn_but(app, pending, tab, targets, pressed, &[]);
}

/// The command a window's Cancel applies (`None`: nothing to set back):
/// what it changed set back ([`cancel_command`], `all` as there), with
/// what the window adds of its own to bring what it kept in line
/// ([`cancel_more`]). A blueprint it takes back (the copy Edit Copy made)
/// is not the palette's to place any more.
fn cancel_window(
    app: &mut Moonglow,
    first: &(ResKey, GffPath),
    tab: &Tab,
    all: bool,
) -> Option<Command> {
    let mut command = cancel_command(app, first, all)?;
    let more = cancel_more(app, tab, &command.edits);
    command.edits.extend(more);
    let removed = |k: &ResKey| {
        let gone = |e: &Edit| matches!(e, Edit::SetResource { key, data: None } if key == k);
        command.edits.iter().any(gone)
    };
    if app.palette.selected.as_ref().is_some_and(removed) {
        app.palette.selected = None;
    }
    Some(command)
}

/// What a window's Cancel adds to what it sets back (given those edits),
/// for the resources it leaves as they are (`window_drawn_but`'s `kept`):
/// the NUI Creator's event script rebuilt around the handlers brought
/// back; the factions' numbers, as a faction removed and set back had
/// them.
fn cancel_more(app: &mut Moonglow, tab: &Tab, set: &[Edit]) -> Vec<Edit> {
    match tab {
        Tab::Nui(key) => crate::nui_view::cancel_more(app, *key, set),
        Tab::Factions => crate::faction_view::numbered_back(app),
        _ => Vec::new(),
    }
}

/// A window with OK and Cancel closed other than by them (its tab's ×,
/// the tab's Close, Close Others or Close All, Ctrl+W), with Options ›
/// General's "Closing a window cancels its changes" on: what it changed
/// is set back as its Cancel sets it back, as Aurora's close button does,
/// once what was asked for this frame is done
/// ([`cancel_closed`]). A text field of it with the keyboard is dropped,
/// as Cancel drops what was typed. (A window with nothing changed has
/// nothing noted, and just closes.)
pub(crate) fn closing(app: &mut Moonglow, tab: &Tab) {
    let first = app.properties_changed.iter().find(|(_, c)| c.tab == *tab).map(|(f, _)| f.clone());
    let Some(first) = first else { return };
    if let Some(changes) = app.properties_changed.remove(&first) {
        app.closing_cancels.push((first, changes));
    }
    if let Some(held) = app.held_text.take_if(|h| h.window.as_ref().is_some_and(|w| w.tab == *tab))
    {
        app.buffers.remove(&held.id);
    }
}

/// The Cancels of the windows [`closing`] noted, each as one step that
/// Undo takes back: after the frame's other actions (what the window
/// asked for as it closed is set first, then set back).
pub(crate) fn cancel_closed(app: &mut Moonglow) {
    for (first, changes) in std::mem::take(&mut app.closing_cancels) {
        let tab = changes.tab.clone();
        app.properties_changed.insert(first.clone(), changes);
        // (Still open: a window's close button closes it only when each of
        // its tabs may close, and a side editor with unsaved changes asks
        // first. What it changed stays its own Cancel's.)
        if app.dock.find_tab(&tab).is_some() {
            continue;
        }
        let command = cancel_window(app, &first, &tab, false);
        app.properties_changed.remove(&first);
        if let Some(command) = command {
            app.run(Action::Apply(command));
        }
    }
}

/// [`window_drawn`] for a window whose Cancel leaves the resources `kept`
/// as they are (what the builder wrote in them elsewhere is not the
/// window's to take back; what it adds of its own to bring those in line
/// is [`cancel_more`]'s).
pub(crate) fn window_drawn_but(
    app: &mut Moonglow,
    pending: usize,
    tab: &Tab,
    targets: &[(ResKey, GffPath)],
    pressed: Option<Pressed>,
    kept: &[ResKey],
) {
    note_changes(app, pending, tab, targets, kept);
    // (Open: what it noted before it opened is kept as any window's.)
    if let Some(changes) = targets.first().and_then(|f| app.properties_changed.get_mut(f)) {
        changes.opening = false;
    }
    // (A text field of this window's, drawn just now with the keyboard:
    // should it be gone before the keyboard leaves it, what it sets then,
    // after the window is drawn, is this window's to set back. A window
    // drawn after it this frame finds it claimed.)
    if let Some(held) = app.held_text.as_mut().filter(|h| h.window.is_none() && !h.panel) {
        let (tab, targets, kept) = (tab.clone(), targets.to_vec(), kept.to_vec());
        held.window = Some(HeldIn { tab, targets, kept });
    }
    // (Escape, for the window in front: its Cancel, `tabs.rs`.)
    let escaped = app.escape_cancel.as_ref() == Some(tab);
    if escaped {
        app.escape_cancel = None;
    }
    let pressed = pressed.or(escaped.then_some(Pressed::Cancel));
    let Some((pressed, first)) = pressed.zip(targets.first()) else { return };
    if pressed == Pressed::Cancel {
        if let Some(held) =
            app.held_text.take_if(|h| h.window.as_ref().is_some_and(|w| w.tab == *tab))
        {
            app.buffers.remove(&held.id);
        }
        // What the window asked for this frame is set first, then set
        // back.
        let asked = app.actions.len() > pending;
        if let Some(command) = cancel_window(app, first, tab, asked) {
            // (Escape sets changes back without a button pressed: said, and
            // that the one step it is comes back with Undo.)
            if escaped {
                let name = crate::tabs::name(app, tab);
                app.log.info(format!(
                    "Escape canceled {name}: its changes are set back. Edit › Undo brings them back."
                ));
            }
            app.actions.push(Action::Apply(command));
        }
    }
    app.properties_changed.remove(first);
    app.actions.push(Action::CloseTab(tab.clone()));
}

/// A creature appearance's picture, for the row the pointer rests on in
/// the list: the creature as it is (`dressed`, its fields as
/// [`Thumbnails::creature`](crate::model_view::Thumbnails::creature)
/// keeps them: its head, colors, what it wears) in that appearance.
fn creature_picture(app: &mut Moonglow, ui: &mut Ui, dressed: u64, row: usize) {
    let key = crate::model_view::Pictured::Creature(dressed, row as u16);
    picture(ui, app.thumbnails.get(key));
}

/// A placeable appearance's picture (its model).
fn placeable_picture(app: &mut Moonglow, ui: &mut Ui, row: usize) {
    model_picture(app, ui, ("placeables", "ModelName"), row);
}

/// A tileset door's picture (doortypes.2da's model; the generic door, row
/// 0, has none of its own).
/// (Row 0, a door of no tileset's, is drawn as the generic door `generic`
/// chosen, as the game draws it.)
pub(super) fn door_type_picture(app: &mut Moonglow, ui: &mut Ui, row: usize, generic: i64) {
    match usize::try_from(generic) {
        Ok(generic) if row == 0 => generic_door_picture(app, ui, generic),
        _ => model_picture(app, ui, ("doortypes", "Model"), row),
    }
}

/// A generic door's picture (genericdoors.2da's model).
pub(super) fn generic_door_picture(app: &mut Moonglow, ui: &mut Ui, row: usize) {
    model_picture(app, ui, ("genericdoors", "ModelName"), row);
}

/// The picture of the model a 2DA row names in a column.
fn model_picture(app: &mut Moonglow, ui: &mut Ui, (table, column): (&str, &str), row: usize) {
    let model = app.game.as_deref().and_then(|g| {
        let t = g.table(table).ok()?;
        let name = t.get(row, column).map(str::trim).filter(|m| !m.is_empty() && *m != "****")?;
        ResKey::parse(&name.to_ascii_lowercase(), ResType::MDL)
    });
    let made = match model {
        Some(m) => app.thumbnails.get(crate::model_view::Pictured::Resource(m)),
        None => Some(None),
    };
    picture(ui, made);
}

/// An item's inventory icon and picture as `item` (its fields) makes them
/// look, on a woman where `female` (armor and cloaks). (The icon tells
/// apart what the model may not: the game's rings and amulets have none of
/// their own, and lie as a bag.)
///
/// For one of an armor's parts (`pieces`: its models' names,
/// [`mg_preview::armor_part`]), that part alone beside it too: small in
/// the picture of the whole body.
pub(super) fn item_part_picture(
    app: &mut Moonglow,
    ui: &mut Ui,
    item: &mg_gff::Struct,
    female: bool,
    pieces: Option<&'static [&'static str]>,
) {
    let key = app.thumbnails.item(item, female);
    let made = app.thumbnails.get(key);
    let alone = pieces.map(|p| {
        let key = app.thumbnails.item_part(item, female, Some(p));
        app.thumbnails.get(key)
    });
    let icon = app.loader().map(|mut l| l.item_icon_on(ui.ctx(), item, female)).unwrap_or_default();
    ui.horizontal_top(|ui| {
        if !icon.is_empty() {
            crate::images::stacked_drawn(ui, &icon, (1.5, PICTURE_SIDE), "Icon");
        }
        picture(ui, made);
        if let Some(alone) = alone {
            picture(ui, alone);
        }
    });
}

/// How large a picture beside a list's row is, points.
const PICTURE_SIDE: f32 = 180.0;

/// A model's picture beside a list's row, that there is none, or (`None`)
/// that it is being made.
fn picture(ui: &mut Ui, made: Option<Option<egui::TextureId>>) {
    match made {
        Some(Some(id)) => {
            ui.add(egui::Image::new((id, egui::vec2(PICTURE_SIDE, PICTURE_SIDE))));
        }
        Some(None) => {
            ui.weak("(no picture)");
        }
        None => {
            ui.weak("…");
        }
    }
}

/// "≠ 2 of 3" after an entry of a list that some of the objects edited
/// together have (`having` of `total`); nothing where all have it.
pub(crate) fn entry_mark(ui: &mut Ui, having: usize, total: usize) {
    if having >= total {
        return;
    }
    let text = format!("{MIXED} {having} of {total}");
    ui.colored_label(ui.visuals().warn_fg_color, text).on_hover_text(format!(
        "{having} of the {total} have it. A change goes to those, an addition to the rest."
    ));
}

/// Shows something of a 2DA row (a picture) in a choice's list.
pub(super) type Shown<'a> = &'a mut dyn FnMut(&mut Moonglow, &mut Ui, usize);

/// A loading screen's picture (loadscreens.2da's `BMPResRef`), small; a
/// line instead where the row has none (the first: a random one of the
/// area's tileset) or it isn't found.
fn load_screen_picture(app: &mut Moonglow, ui: &mut Ui, row: usize) {
    const WIDTH: f32 = 260.0;
    let name = app.game.as_deref().and_then(|g| {
        let t = g.table("loadscreens").ok()?;
        t.get(row, "BMPResRef").filter(|n| *n != "****" && !n.is_empty()).map(str::to_owned)
    });
    let Some(name) = name else {
        if row == 0 {
            ui.weak("A random one of the area's tileset");
        }
        return;
    };
    match app.loader().and_then(|mut l| l.load_screen(ui.ctx(), &name)) {
        Some(p) if p.size.x > 0.0 => {
            let size = egui::vec2(WIDTH, WIDTH * p.size.y / p.size.x);
            ui.add(egui::Image::new((p.texture.id(), size))).on_hover_text(name);
        }
        _ => {
            ui.weak(format!("({name}: no picture found)"));
        }
    }
}

/// A blueprint's fields, laid out.
pub(crate) struct Form<'a> {
    pub app: &'a mut Moonglow,
    /// The document: the blueprint, or the area's GIT for a placed object.
    pub key: ResKey,
    /// Where the object is in it: the root for a blueprint, its entry in a
    /// GIT list for a placed object.
    pub path: GffPath,
    /// Other objects or blueprints of the same type edited with it
    /// (Aurora's multi-editor): each field set is set on them too. Each is
    /// its document and path.
    pub also: crate::widgets::Also,
    /// The object as it is now.
    pub root: Struct,
    /// What the list pages did to the objects edited together.
    pub together: Together,
    /// Drawn as its object's own tab or window, every page on one scroll
    /// (`false`: the Properties panel's page, Module or Area Properties,
    /// the journal): where its tab brings the Preview panel, its Preview
    /// button shows that (M4's U11).
    pub own_tab: bool,
}

/// What a form's list pages asked of several objects edited together
/// (`docs/lists-together-proposal.md`).
#[derive(Debug, Default)]
pub(crate) struct Together {
    /// The actions (their places in `app.actions`) built for each of the
    /// objects, which may reach some of them only: the guard lets them
    /// through.
    pub each: Vec<usize>,
    /// What the last of them did, for the window's note.
    pub said: Option<String>,
}

/// The text in the editing language (Options › Language).
/// The right-hand side of a page beside a palette: it scrolls (sideways
/// too, in a narrow window) and keeps a gutter on its right, clear of the
/// scroll bar, so its last column is never cut off.
pub(crate) fn side_panel<R>(ui: &mut Ui, id: egui::Id, add: impl FnOnce(&mut Ui) -> R) -> R {
    let gutter = egui::Margin { right: 14, ..Default::default() };
    egui::ScrollArea::both()
        .id_salt(id)
        .show(ui, |ui| {
            egui::Frame::NONE.inner_margin(gutter).show(ui, |ui| ui.vertical(add).inner).inner
        })
        .inner
}

/// A value of an integer field, of the type the field has (else `default`).
pub(crate) fn integer(existing: Option<&Value>, v: i64, default: FieldType) -> Value {
    let t = existing.map_or(default, Value::field_type);
    match t {
        FieldType::Byte => Value::Byte(v.clamp(0, 255) as u8),
        FieldType::Char => Value::Char(v.clamp(-128, 127) as i8),
        FieldType::Word => Value::Word(v.clamp(0, 65535) as u16),
        FieldType::Short => Value::Short(v.clamp(-32768, 32767) as i16),
        FieldType::Dword => Value::Dword(v.clamp(0, u32::MAX as i64) as u32),
        FieldType::Dword64 => Value::Dword64(v.max(0) as u64),
        FieldType::Int64 => Value::Int64(v),
        _ => Value::Int(v.clamp(i32::MIN as i64, i32::MAX as i64) as i32),
    }
}

/// What `make` makes of the game data, kept in egui's memory under `id`
/// while the load order is the one it was made for (`load_order`:
/// [`Moonglow::load_order`]), and made again when a hak, the override or
/// the module's own files change what the game data has.
pub(crate) fn kept_for_load_order<T: Clone + Send + Sync + 'static>(
    ui: &Ui,
    load_order: u64,
    id: egui::Id,
    make: impl FnOnce() -> T,
) -> T {
    if let Some((was, kept)) = ui.data(|d| d.get_temp::<(u64, T)>(id))
        && was == load_order
    {
        return kept;
    }
    let made = make();
    ui.data_mut(|d| d.insert_temp(id, (load_order, made.clone())));
    made
}

/// The id a Properties window keeps a piece of its state by (`name`): its
/// own, by the object it shows (`key`, and `path` in it), so two windows
/// of one area's objects share none. A placed object's window keeps its
/// ids as it follows its object ([`Moonglow::window_ids`]).
fn window_state_id(app: &Moonglow, key: ResKey, path: &GffPath, name: &str) -> egui::Id {
    // (The Properties panel's apart from a window's: its own, though both
    // follow the object.)
    let host = if app.panel_form { "panel" } else { "window" };
    if key.restype == ResType::GIT
        && let Some(id) = app.window_ids.get(&(key.resref, path.clone()))
    {
        return id.with(("blueprint", name, host));
    }
    // (A journal's entry or category, in its panels: by the object, as it
    // moves, M4's U22.)
    if key.restype == ResType::JRL
        && let Some(id) = crate::journal_view::field_id(app, path)
    {
        return id.with(("blueprint", name, host));
    }
    egui::Id::new(("blueprint", key, path, name, host))
}

/// After a frame: what the pages keep while they are drawn, let go of
/// for those that were not (`replacements::let_go_of_unseen`).
pub(crate) fn let_go_of_unseen(app: &mut Moonglow, ctx: &egui::Context) {
    replacements::let_go_of_unseen(app, ctx);
}

/// What `make` makes of a list the game data keeps (`of`: a table, or
/// its rows as choices, made once and handed out as the same `Arc`) and
/// of `key`, kept in egui's memory under `id` while both are the same: a
/// long table's rows sorted by name, narrowed by what is typed, or told
/// which to offer, once and not every frame. The list is held, and told
/// by `Arc::ptr_eq`.
///
/// Do not pass `&Arc<GameData>` here: holding an extra `Arc` in egui memory
/// blocks `crate::exclusive(&mut app.game)`. Use [`kept_for_key`] keyed on
/// `app.load_order` instead.
pub(crate) fn kept_for<S, K, T>(
    ctx: &egui::Context,
    id: egui::Id,
    of: &Arc<S>,
    key: &K,
    make: impl FnOnce() -> T,
) -> T
where
    S: Send + Sync + 'static,
    K: PartialEq + Clone + Send + Sync + 'static,
    T: Clone + Send + Sync + 'static,
{
    if let Some((was, k, kept)) = ctx.data(|d| d.get_temp::<(Arc<S>, K, T)>(id))
        && Arc::ptr_eq(&was, of)
        && k == *key
    {
        return kept;
    }
    let made = make();
    ctx.data_mut(|d| d.insert_temp(id, (of.clone(), key.clone(), made.clone())));
    made
}

/// What `make` makes for `key`, kept in egui's memory under `id` while
/// `key` stays equal without holding an extra `Arc` reference (safe for
/// keys derived from `app.load_order` / workspace `(identity, revision)`).
pub(crate) fn kept_for_key<K, T>(
    ctx: &egui::Context,
    id: egui::Id,
    key: &K,
    make: impl FnOnce() -> T,
) -> T
where
    K: PartialEq + Clone + Send + Sync + 'static,
    T: Clone + Send + Sync + 'static,
{
    if let Some((was, kept)) = ctx.data(|d| d.get_temp::<(K, T)>(id))
        && was == *key
    {
        return kept;
    }
    let made = make();
    ctx.data_mut(|d| d.insert_temp(id, (key.clone(), made.clone())));
    made
}

/// `choices` (a table's: `widgets::table_choices`) in the order of their
/// names ([`mg_rules::by_name`]), sorted once for the list and kept under
/// `what`: sorted every frame, a table of 15,000 appearances or 20,000
/// feats was copied whole and given as many sort keys, each frame.
pub(crate) fn by_name_kept(
    ctx: &egui::Context,
    what: &str,
    choices: &Arc<Vec<Choice>>,
) -> Arc<Vec<Choice>> {
    let id = egui::Id::new(("choices-by-name", what));
    kept_for(ctx, id, choices, &(), || Arc::new(mg_rules::by_name(choices.to_vec())))
}

/// The places in `choices` of those whose text `find` finds, kept under
/// `what` for the list and what is typed.
pub(crate) fn found_kept(
    ctx: &egui::Context,
    what: &str,
    choices: &Arc<Vec<Choice>>,
    find: &str,
) -> Arc<Vec<usize>> {
    let id = egui::Id::new(("choices-found", what));
    kept_for(ctx, id, choices, &find.to_string(), || {
        let needle = mg_core::text::Needle::new(find);
        Arc::new((0..choices.len()).filter(|&i| needle.finds(&choices[i].text)).collect())
    })
}

/// How many rows of text a field of several lines shows to reach from
/// `ui`'s cursor down to `bottom` (its frame and margins taken off): the
/// room its panel gives it, which its width does not change. At least
/// three.
fn rows_to(ui: &Ui, bottom: f32) -> usize {
    let font = egui::TextStyle::Body.resolve(ui.style());
    let row = ui.fonts_mut(|f| f.row_height(&font)).max(1.0);
    // (egui's margin of a field, 2 above and below, and its frame's line.)
    let frame = 2.0 * 2.0 + 2.0;
    let room = bottom - ui.cursor().top() - frame;
    ((room / row).floor().max(3.0)) as usize
}

/// The color of text that is the talk table's (a StrRef's), not a
/// blueprint's own: the theme's `info`.
pub(crate) fn talk_table_color(ui: &Ui) -> egui::Color32 {
    mg_uikit::tokens(ui.ctx()).info
}

fn borrowed_label(ui: &mut Ui, from: &str, why: &str, color: egui::Color32) {
    let text = egui::RichText::new(from).size(mg_uikit::metrics::SMALL_SIZE).color(color);
    ui.add(egui::Label::new(text).selectable(false)).on_hover_text(why);
}

impl Form<'_> {
    /// A table's rows as a dropdown's choices: each row's name (its StrRef
    /// column `name`), else its `label`. None without the game's data or
    /// the table. (The game data's list itself, not a copy: asked for
    /// every frame.)
    pub(crate) fn choices(
        &self,
        table: &str,
        name: Option<&str>,
        label: Option<&str>,
    ) -> std::sync::Arc<Vec<Choice>> {
        crate::widgets::table_choices(self.app.game.as_deref(), table, name, label)
    }

    /// An Appearance list's rows (`choices`, of an object of `kind`), each
    /// with no model to draw marked so after its name
    /// ([`mg_rules::GameData::appearance_choices`]).
    pub(crate) fn appearance_rows(
        &self,
        ctx: &egui::Context,
        kind: mg_rules::ObjectKind,
        choices: &std::sync::Arc<Vec<Choice>>,
    ) -> std::sync::Arc<Vec<Choice>> {
        match self.app.game.as_deref() {
            Some(game) => kept_for(
                ctx,
                egui::Id::new(("appearance-rows", kind)),
                choices,
                &self.app.load_order,
                || game.appearance_choices(kind, choices),
            ),
            None => choices.clone(),
        }
    }

    /// An item's Properties, from the inventory that holds it (item `index`
    /// of `list` in the struct at `at`): a placed object's item is its own,
    /// edited where it is held; a blueprint's is its blueprint (the
    /// module's to edit, the game's to look at).
    pub(crate) fn item_properties(
        &mut self,
        at: &GffPath,
        list: &str,
        index: usize,
        entry: &Struct,
    ) {
        if self.key.restype == ResType::GIT && entry.get("BaseItem").is_some() {
            let path = at.item(list, index);
            self.app
                .actions
                .push(Action::OpenTab(crate::Tab::Instance { area: self.key.resref, path }));
            return;
        }
        let r = inventory::entry_resref(entry);
        if !r.is_empty() {
            crate::palette_view::view_blueprint(self.app, ResKey::new(r, ResType::UTI));
        }
    }

    /// Gallery… beside an appearance: opens the Appearance Gallery at this
    /// object's, where a click gives it another.
    pub(crate) fn gallery_button(&mut self, ui: &mut Ui, kind: crate::appearance_gallery::Kind) {
        if ui
            .button("Gallery…")
            .on_hover_text("Every appearance as a picture, from this one on. Click one to give it.")
            .clicked()
        {
            let gallery = crate::appearance_gallery::Gallery::of(kind, self.key, self.path.clone());
            self.app.placeable_gallery = Some(gallery);
            self.app.actions.push(Action::OpenTab(crate::Tab::PlaceableGallery));
        }
    }

    /// A row of the grid under the object's appearance, there only when
    /// the appearance has no model to draw, in the error color: why
    /// ([`mg_rules::appearance`], as the preview and Verify judge it): a
    /// row the table lacks, one that names no model or is reserved
    /// (`USER`), or a model the module, its haks and the game lack, which
    /// Aurora's area view often crashes on.
    ///
    /// And past a row that has one, each model the object's parts name
    /// that the module, its haks and the game lack ([`mg_rules::models`]):
    /// an item's pieces (where it has a model of its own at all) and an
    /// armor's parts, a body's skeleton, head and parts.
    pub(crate) fn model_warning(&self, ui: &mut Ui, kind: mg_rules::ObjectKind) {
        let Some(game) = self.app.game.as_deref() else { return };
        for problem in game.appearance_problems(kind, &self.root) {
            crate::widgets::field_label(ui, "");
            let text = egui::RichText::new(problem.to_string()).color(ui.visuals().error_fg_color);
            let why = match kind {
                _ if problem.problem.is_part() => {
                    "The game draws the rest without it. Pick another part, or add its model to a \
                     hak."
                }
                // (The game draws no waypoint: its flag is the toolset's.)
                mg_rules::ObjectKind::Waypoint => {
                    "The area view draws no flag. Pick another appearance or add the model to a \
                     hak."
                }
                _ => {
                    "The game draws nothing for it, and Aurora may crash. Pick another, or add the \
                     model."
                }
            };
            ui.add(egui::Label::new(text).wrap()).on_hover_text(why);
            ui.end_row();
        }
    }

    fn id(&self, name: &str) -> egui::Id {
        window_state_id(self.app, self.key, &self.path, name)
    }

    /// The id of its text field of `label` (a panel's F6 gives it the
    /// keyboard's focus).
    pub(crate) fn field_id(&self, label: &str) -> egui::Id {
        self.id(label)
    }

    /// The id its lists are kept by (`situated::pick_list` and the
    /// others): the window's own.
    pub(crate) fn picks(&self) -> egui::Id {
        self.id("pick")
    }

    fn target(&self, label: &str) -> FieldTarget {
        let mut t = FieldTarget::new(self.key, self.path.clone(), label);
        t.also = self.also.clone();
        t
    }

    /// Where a field set goes: the object's document and path, and the
    /// others'.
    pub(super) fn targets(&self) -> impl Iterator<Item = (ResKey, &GffPath)> {
        std::iter::once((self.key, &self.path)).chain(self.also.iter().map(|(k, p)| (*k, p)))
    }

    /// Whether the others edited with this one have another value in
    /// field `label` (not looked at past [`MIXED_LIMIT`] of them).
    pub(crate) fn differs(&mut self, label: &str) -> bool {
        if self.also.is_empty() || self.also.len() > MIXED_LIMIT {
            return false;
        }
        let Some(ws) = self.app.ws.as_mut() else { return false };
        let mine = self.root.get(label);
        self.also.iter().any(|(key, path)| {
            let theirs =
                ws.doc(key).ok().and_then(|d| path.get(&d.root)).and_then(|s| s.get(label));
            theirs != mine
        })
    }

    /// Whether the others edited with this one have another value in
    /// field `label`, as `seen` sees it (a bit of a field of flags).
    fn differs_by<T: PartialEq>(
        &mut self,
        label: &str,
        seen: impl Fn(Option<&Value>) -> T,
    ) -> bool {
        if self.also.is_empty() || self.also.len() > MIXED_LIMIT {
            return false;
        }
        let Some(ws) = self.app.ws.as_mut() else { return false };
        let mine = seen(self.root.get(label));
        self.also.iter().any(|(key, path)| {
            let theirs =
                ws.doc(key).ok().and_then(|d| path.get(&d.root)).and_then(|s| s.get(label));
            seen(theirs) != mine
        })
    }

    /// Marks the field about to be drawn if the others edited with this
    /// one have another value in it: "≠" before it, with a tip. (Painted,
    /// not laid out: a table's cell holds the field alone.) Every field a
    /// form sets on each of the objects edited together has the mark.
    pub(crate) fn mark_mixed(&mut self, ui: &Ui, label: &str) {
        if self.differs(label) {
            self.paint_mixed(ui, label);
        }
    }

    /// [`mark_mixed`](Self::mark_mixed) for one bit of a field of flags
    /// (an area's `Flags`: a box, or a pair of choices, a bit).
    pub(crate) fn mark_mixed_bit(&mut self, ui: &Ui, label: &str, bit: u32) {
        let set = |v: Option<&Value>| match v {
            Some(Value::Dword(f)) => *f & bit,
            Some(Value::Int(f)) => *f as u32 & bit,
            _ => 0,
        };
        if self.differs_by(label, set) {
            self.paint_mixed(ui, &format!("{label}#{bit}"));
        }
    }

    /// "≠", laid out where the next widget goes (a row of choices that
    /// starts at the page's edge, where a painted mark has no room), if
    /// the objects edited together differ in any of the fields `labels`.
    pub(crate) fn mark_mixed_inline(&mut self, ui: &mut Ui, labels: &[&str]) {
        if labels.iter().any(|l| self.differs(l)) {
            mixed_mark_inline(ui, self.id(labels.first().copied().unwrap_or("")).with("mixed"));
        }
    }

    /// [`mark_mixed_inline`](Self::mark_mixed_inline) for one bit of a
    /// field of flags: before a group of choices, the whole group's.
    pub(crate) fn mark_mixed_bit_inline(&mut self, ui: &mut Ui, label: &str, bit: u32) {
        let set = |v: Option<&Value>| match v {
            Some(Value::Dword(f)) => *f & bit,
            Some(Value::Int(f)) => *f as u32 & bit,
            _ => 0,
        };
        if self.differs_by(label, set) {
            mixed_mark_inline(ui, self.id(&format!("{label}#{bit}")).with("mixed"));
        }
    }

    /// A line under a section's heading where the objects edited together
    /// differ in its field `label` (a list of the section's own, set on
    /// each whole): "≠" and what that means.
    pub(crate) fn mixed_line(&mut self, ui: &mut Ui, label: &str) {
        if !self.differs(label) {
            return;
        }
        let color = ui.visuals().warn_fg_color;
        row(ui, |ui| {
            mixed_mark(ui, MIXED_TIP);
            ui.colored_label(
                color,
                "They differ here: this shows the first one's, and what you change here sets \
                 this list on each",
            );
        });
    }

    /// The "≠" before the field about to be drawn: painted in the gutter
    /// to its left, and known to the screen reader and the pointer.
    ///
    /// On the field's label, after its words, where the label is kept
    /// (`widgets::label_before`): a mark in the gutter was 5 by 7 pixels,
    /// and outside the page where a narrow form stacks the label over the
    /// field (M4's U29 review). Else in the gutter, as before.
    fn paint_mixed(&self, ui: &Ui, label: &str) {
        let id = self.id(label).with("mixed");
        let size = mixed_mark_size(ui);
        let rect = match crate::widgets::label_before(ui) {
            Some(l) => egui::Rect::from_min_size(
                egui::pos2(l.right() + 4.0, l.center().y - size.y / 2.0),
                size,
            ),
            None => {
                egui::Rect::from_min_size(ui.cursor().min + egui::vec2(-3.0 - size.x, 1.0), size)
            }
        };
        paint_mixed_mark(ui, rect, id, MIXED_TIP);
    }

    /// Several blueprints edited together (not placed objects).
    pub(crate) fn several_blueprints(&self) -> bool {
        !self.is_instance() && !self.also.is_empty()
    }

    /// Several objects or blueprints edited together.
    pub(crate) fn several(&self) -> bool {
        !self.also.is_empty()
    }

    /// The objects edited (this one first) as the list rules take them
    /// (`mg_edit::lists`), each as it is now. Kept while the module is as
    /// it was read (its revision) and the window's objects are the same:
    /// read every frame, a hundred creatures selected were copied whole,
    /// with all they carry, each frame a list page was shown.
    pub(crate) fn list_targets(&mut self) -> Arc<Vec<lists::Target>> {
        let revision = self.app.ws.as_ref().map(mg_edit::Workspace::revision);
        let kept = &mut self.app.palette.kept;
        if kept.targets_revision != revision {
            kept.targets_revision = revision;
            kept.targets.clear();
            kept.targets_last = None;
        }
        if let Some(((key, path, also), hit)) = &kept.targets_last
            && *key == self.key
            && Arc::ptr_eq(also, &self.also)
            && *path == self.path
            && hit.first().is_some_and(|t| t.at == self.root)
        {
            return hit.clone();
        }
        let map_key = (self.key, self.path.clone(), self.also.clone());
        if let Some(hit) = kept.targets.get(&map_key)
            && hit.first().is_some_and(|t| t.at == self.root)
        {
            kept.targets_last = Some((map_key, hit.clone()));
            return hit.clone();
        }
        let mut out = vec![lists::Target::new(self.key, self.path.clone(), self.root.clone())];
        if let Some(ws) = self.app.ws.as_mut() {
            for (key, path) in self.also.iter() {
                if let Some(s) = ws.doc(key).ok().and_then(|d| path.get(&d.root).cloned()) {
                    out.push(lists::Target::new(*key, path.clone(), s));
                }
            }
        }
        let out = Arc::new(out);
        let kept = &mut self.app.palette.kept;
        kept.targets.insert(map_key.clone(), out.clone());
        kept.targets_last = Some((map_key, out.clone()));
        out
    }

    /// Makes what a list rule made of the objects, one step named `what`:
    /// built for each, it may reach some of them only, and the guard of
    /// several objects edited together lets it through. With several, the
    /// window's note says what it did (`done`: "Added to") and what it
    /// left.
    pub(crate) fn apply_each(&mut self, what: &str, out: lists::Outcome, done: &str) {
        if self.several() {
            self.together.said = Some(format!("{what}: {}.", out.said(done)));
        }
        if out.edits.is_empty() {
            return;
        }
        self.together.each.push(self.app.actions.len());
        self.app.actions.push(Action::Apply(Command::new(what, out.edits)));
    }

    /// [`mark_mixed`](Self::mark_mixed) for a value of a list's entry
    /// (`label` names it, for its id), which the objects having it differ
    /// in where `differs`.
    pub(crate) fn mark_if(&self, ui: &Ui, label: &str, differs: bool) {
        if differs && self.several() {
            self.paint_mixed(ui, label);
        }
    }

    /// Whether this is an object placed in an area (not a blueprint).
    pub(crate) fn is_instance(&self) -> bool {
        !self.path.0.is_empty()
    }

    /// The object's blueprint type (for a placed object, from its GIT
    /// list).
    pub(crate) fn restype(&self) -> ResType {
        instance_type(&self.path).unwrap_or(self.key.restype)
    }

    /// Sets a field (one undoable command named `what`).
    pub(crate) fn set(&mut self, what: &str, label: &str, value: Value) {
        self.set_opt(what, label, Some(value));
    }

    /// Sets a field, or removes it (`None`).
    pub(crate) fn set_opt(&mut self, what: &str, label: &str, value: Option<Value>) {
        let command = self.setting(what, label, value);
        self.app.actions.push(Action::Apply(command));
    }

    /// Sets a localized string's field to the text typed into it: put in
    /// the module's talk table first, where Options › Language says so
    /// (`crate::talk_typed`).
    pub(crate) fn set_typed(&mut self, what: &str, label: &str, value: Value) {
        let command = self.setting(what, label, Some(value));
        self.app.actions.push(Action::ApplyTyped(command));
    }

    /// The command that sets a field, or removes it.
    fn setting(&self, what: &str, label: &str, value: Option<Value>) -> Command {
        let edits = self
            .targets()
            .map(|(key, path)| Edit::SetField {
                key,
                path: path.clone(),
                label: label.to_string(),
                value: value.clone(),
            })
            .collect();
        Command::new(what, edits)
    }

    /// Holds the pending command built by `make` for field `id` while it
    /// has the keyboard, after it is drawn. Should the field disappear
    /// before focus leaves it, `Moonglow::held_text` flushes the command.
    pub(super) fn hold_command(
        &mut self,
        ui: &Ui,
        id: egui::Id,
        loc: bool,
        make: impl FnOnce(&mut Self, &str) -> Option<Command>,
    ) {
        if !ui.memory(|m| m.has_focus(id)) {
            if self.app.held_text.as_ref().is_some_and(|held| held.id == id) {
                self.app.held_text = None;
            }
            return;
        }
        let buf = self.app.buffers.get(&id).cloned();
        let command = buf.as_deref().and_then(|b| make(self, b));
        let drawn = ui.ctx().cumulative_pass_nr();
        // (Its window says which it is once it is drawn: `window_drawn_but`.)
        let panel = false;
        self.app.held_text =
            Some(HeldText { id, drawn, command, action: None, window: None, typed: loc, panel });
    }

    /// Holds an action (e.g. blueprint rename) for focused field `id` in
    /// `Moonglow::held_text` in case the field disappears before losing focus.
    fn hold_action(&mut self, ui: &Ui, id: egui::Id, make: impl FnOnce(&str) -> Option<Action>) {
        if !ui.memory(|m| m.has_focus(id)) {
            if self.app.held_text.as_ref().is_some_and(|held| held.id == id) {
                self.app.held_text = None;
            }
            return;
        }
        let action = self.app.buffers.get(&id).and_then(|b| make(b));
        let drawn = ui.ctx().cumulative_pass_nr();
        let panel = false;
        self.app.held_text =
            Some(HeldText { id, drawn, command: None, action, window: None, typed: false, panel });
    }

    /// Holds uncommitted typing in a `ResRef` field (`script`, `linked`).
    fn hold_resref(&mut self, ui: &Ui, what: &str, label: &str, current: ResRef) {
        let id = self.id(label);
        let expected = current.to_string();
        self.hold_command(ui, id, false, |form, buf| {
            if buf == expected {
                return None;
            }
            let v = ResRef::from_str(buf.trim()).ok()?;
            (v != current).then(|| form.setting(what, label, Some(Value::resref(v))))
        });
    }

    /// Holds a text field's typing while it has the keyboard, after the
    /// field is drawn: `shown` is the field's value as it is stored, and
    /// `value` what the text typed would be set as (`loc`: a localized
    /// string's text, its line ends as [`crate::text::with_loc_line_ends`]
    /// sets them). Should the field be gone before the keyboard leaves it,
    /// that is set ([`Moonglow::held_text`]).
    fn hold_text(
        &mut self,
        ui: &Ui,
        what: &str,
        label: &str,
        shown: &str,
        loc: bool,
        value: impl Fn(String) -> Value,
    ) {
        let id = self.id(label);
        let expected = crate::text::to_editor(shown).0;
        let ends = if loc { crate::text::with_loc_line_ends } else { crate::text::with_line_ends };
        self.hold_command(ui, id, loc, |form, buf| {
            (buf != expected).then(|| form.setting(what, label, Some(value(ends(buf, shown)))))
        });
    }

    /// The type the game's files give field `label` of the struct the
    /// form edits: a blueprint's or a placed object's own field, or a
    /// journal category's or entry's (the form edits one in its list, and
    /// the journal's root has neither's fields: a category's XP the file
    /// lacked was written as a BYTE, 255 for 500, and an entry's End as
    /// one where the game reads a WORD).
    fn schema_type(&self, label: &str) -> Option<FieldType> {
        use mg_gff::Step::Item;
        let fields = match (self.key.restype, &self.path.0[..]) {
            (ResType::JRL, [Item(..)]) => mg_schema::jrl::categories::FIELDS,
            (ResType::JRL, [Item(..), Item(..)]) => mg_schema::jrl::categories::entry_list::FIELDS,
            _ => return mg_schema::root_field_type(self.restype(), label),
        };
        fields.iter().find(|f| f.label == label).map(|f| f.field_type)
    }

    /// Sets an integer field, keeping its stored type (for a field the
    /// blueprint lacks: the type the game's files give it, else `default`).
    pub(crate) fn set_int(&mut self, what: &str, label: &str, v: i64, default: FieldType) {
        let default = self.schema_type(label).unwrap_or(default);
        let value = integer(self.root.get(label), v, default);
        self.set(what, label, value);
    }

    pub(crate) fn int(&self, label: &str) -> i64 {
        self.root.integer(label).unwrap_or(0)
    }

    /// A one-line text field (`CExoString`) of at most `max` characters.
    pub(crate) fn text(&mut self, ui: &mut Ui, what: &str, label: &str, max: usize) {
        self.mark_mixed(ui, label);
        let current = decode(self.root.string(label).unwrap_or_default());
        let id = self.id(label);
        let (shown, _) = crate::text::to_editor(&current);
        let buf = self.app.buffers.entry(id).or_insert_with(|| shown.clone());
        let r =
            ui.add(mg_uikit::text_field(buf).id(id).char_limit(max).desired_width(fit(ui, 240.0)));
        crate::widgets::leaves_at_a_press(ui.ctx(), &r);
        if !crate::widgets::has_keys(&r) && !r.lost_focus() && *buf != shown {
            *buf = shown.clone();
        }
        if r.lost_focus() && *buf != shown {
            // (Its line ends kept, as a field of several lines keeps them.)
            let v = crate::text::with_line_ends(buf, &current);
            self.set(what, label, Value::String(encode(&v)));
        }
        self.hold_text(ui, what, label, &current, false, |text| Value::String(encode(&text)));
    }

    /// A tag: a [`text`](Self::text) of at most the 32 characters the
    /// game keeps (`mg_module::tags::MAX_TAG`), as every tag field is.
    pub(crate) fn tag(&mut self, ui: &mut Ui, what: &str, label: &str) {
        self.text(ui, what, label, mg_module::tags::MAX_TAG);
    }

    /// A blueprint page's first rows in its grid: Name (the localized
    /// name in `name`) and Tag.
    pub(crate) fn name_and_tag(&mut self, ui: &mut Ui, name: &str) {
        crate::widgets::field_label(ui, "Name");
        self.locstring(ui, "Name", name);
        ui.end_row();
        crate::widgets::field_label(ui, "Tag");
        self.tag(ui, "Tag", "Tag");
        ui.end_row();
    }

    /// A multi-line text field (`CExoString`), line ends kept.
    pub(crate) fn memo(&mut self, ui: &mut Ui, what: &str, label: &str) {
        self.mark_mixed(ui, label);
        let current = decode(self.root.string(label).unwrap_or_default());
        let id = self.id(label);
        let rows = if panel_scroll(ui) { Some(3) } else { Some(6) };
        if let Some(v) = crate::widgets::commit_text_in(
            &mut self.app.buffers,
            ui,
            id,
            &current,
            rows,
            f32::INFINITY,
            None,
        ) {
            self.set(what, label, Value::String(encode(&v)));
        }
        self.hold_text(ui, what, label, &current, false, |text| Value::String(encode(&text)));
    }

    /// A localized name: the English text, and `…` for every language.
    pub(crate) fn locstring(&mut self, ui: &mut Ui, what: &str, label: &str) {
        self.locstring_then(ui, what, label, |_, _| {});
    }

    fn finish_loc_edit(
        &mut self,
        ui: &mut Ui,
        what: &str,
        label: &str,
        current: &mg_core::LocString,
        shown: &str,
        typed: Option<String>,
    ) {
        if let Some(v) = typed {
            self.set_typed(what, label, Value::LocString(with_edited_text(current.clone(), &v)));
        }
        let held = current.clone();
        self.hold_text(ui, what, label, shown, true, move |text| {
            Value::LocString(with_edited_text(held.clone(), &text))
        });
    }

    fn open_loc_edit_on_click(
        &mut self,
        ui: &mut Ui,
        what: &str,
        label: &str,
        current: &mg_core::LocString,
    ) {
        if crate::widgets::languages_button(ui) {
            self.app.loc_edit = Some(LocStringEdit::new(self.target(label), what, current));
        }
    }

    /// A [`locstring`](Self::locstring) and `after` (a creature's Random
    /// Name) after its `…`. In a narrow form the text takes the field's
    /// whole width, and any external source badge ("StrRef 5763") moves onto
    /// the row under it with the buttons so short panels do not clip names.
    pub(crate) fn locstring_then(
        &mut self,
        ui: &mut Ui,
        what: &str,
        label: &str,
        after: impl FnOnce(&mut Self, &mut Ui),
    ) {
        if narrow(ui) {
            self.narrow_locstring(ui, what, label, after);
            return;
        }
        self.mark_mixed(ui, label);
        let current = self.root.locstring(label).cloned().unwrap_or_default();
        row(ui, |ui| {
            // (Where its text is not its own, the field says whose inside
            // its right end, "StrRef 24650" or the language's name, in
            // the talk table's color: the Creature board's `.field .keys`.
            // A window's field fills its row's room but for the buttons
            // after it; a narrow form's is `narrow_locstring`.)
            let (shown, borrowed) = crate::widgets::loc_shown(self.app, &current);
            // (A field is never wider than its row's room: egui's isn't;
            // and a few points for the frames past their widgets.)
            let buttons =
                2.0 * (mg_uikit::metrics::ICON_BUTTON + ui.spacing().item_spacing.x) + 6.0;
            let width = (ui.available_width() - buttons).clamp(LOC_FIELD.start, LOC_FIELD.end);
            let id = self.id(label);
            let color = talk_table_color(ui);
            let keys = borrowed.as_ref().map(|(from, why)| {
                let font = egui::FontId::proportional(mg_uikit::metrics::SMALL_SIZE);
                let w = ui.fonts_mut(|f| f.layout_no_wrap(from.clone(), font, color)).size().x;
                (from.clone(), *why, w)
            });
            let at = ui.cursor().min;
            let typed = ui
                .scope(|ui| {
                    if keys.is_some() {
                        ui.visuals_mut().override_text_color = Some(color);
                    }
                    let room = keys.as_ref().map_or(0.0, |k| k.2 + KEYS_PADDING);
                    ui.data_mut(|d| d.insert_temp(egui::Id::new(FIELD_END_ROOM), room));
                    let typed =
                        crate::widgets::commit_loc_text(self.app, ui, id, &shown, false, width);
                    ui.data_mut(|d| d.remove::<f32>(egui::Id::new(FIELD_END_ROOM)));
                    typed
                })
                .inner;
            self.finish_loc_edit(ui, what, label, &current, &shown, typed);
            if let Some((from, why, w)) = keys {
                let h = ui.spacing().interact_size.y;
                let field = egui::Rect::from_min_size(at, egui::vec2(width, h));
                let rect = egui::Rect::from_min_max(
                    egui::pos2(field.right() - w - KEYS_PADDING, field.top()),
                    egui::pos2(field.right() - 4.0, field.bottom()),
                );
                let mut inside = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(rect)
                        .layout(egui::Layout::right_to_left(egui::Align::Center)),
                );
                borrowed_label(&mut inside, &from, why, color);
            }
            self.open_loc_edit_on_click(ui, what, label, &current);
            after(self, ui);
        });
    }

    /// [`locstring_then`](Self::locstring_then) in a narrow form: the
    /// field, then a row of whose its text is, its `…` and `after`.
    fn narrow_locstring(
        &mut self,
        ui: &mut Ui,
        what: &str,
        label: &str,
        after: impl FnOnce(&mut Self, &mut Ui),
    ) {
        self.mark_mixed(ui, label);
        let current = self.root.locstring(label).cloned().unwrap_or_default();
        let (shown, borrowed) = crate::widgets::loc_shown(self.app, &current);
        let color = talk_table_color(ui);
        let id = self.id(label);
        ui.vertical(|ui| {
            let typed = ui
                .scope(|ui| {
                    if borrowed.is_some() {
                        ui.visuals_mut().override_text_color = Some(color);
                    }
                    let width = fit(ui, 240.0);
                    crate::widgets::commit_loc_text(self.app, ui, id, &shown, false, width)
                })
                .inner;
            self.finish_loc_edit(ui, what, label, &current, &shown, typed);
            ui.horizontal_wrapped(|ui| {
                if let Some((from, why)) = &borrowed {
                    borrowed_label(ui, from, why, color);
                }
                self.open_loc_edit_on_click(ui, what, label, &current);
                after(self, ui);
            });
        });
    }

    /// A localized multi-line text (descriptions), under its section
    /// heading: the text's name, the language edited after it, the
    /// languages button at its end (the Creature board's Description).
    pub(crate) fn locstring_memo(&mut self, ui: &mut Ui, what: &str, label: &str) {
        let current = self.root.locstring(label).cloned().unwrap_or_default();
        let language = crate::text::edit_language().name().unwrap_or("?");
        let mut languages = false;
        let in_panel_scroll = panel_scroll(ui);
        let duplicate = panel_page(ui).as_deref() == Some(what);
        if duplicate {
            // (The panel's page heading already names this page: show only the
            // language note and the languages button on one compact row.)
            ui.horizontal(|ui| {
                ui.weak(language);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    languages = crate::widgets::languages_button(ui);
                });
            });
        } else {
            mg_uikit::form::section_heading_with(ui, what, Some(language), |ui| {
                languages = crate::widgets::languages_button(ui);
            });
        }
        if languages {
            self.app.loc_edit = Some(LocStringEdit::new(self.target(label), what, &current));
        }
        self.mark_mixed(ui, label);
        // (Three lines inside the panel's scroll of every page, six
        // elsewhere.)
        let rows = Some(if in_panel_scroll { 3 } else { 6 });
        let id = self.id(label);
        let typed = crate::widgets::loc_text(self.app, ui, id, &current, rows, f32::INFINITY);
        let shown = crate::widgets::loc_shown(self.app, &current).0;
        self.finish_loc_edit(ui, what, label, &current, &shown, typed);
    }

    /// [`locstring_memo`](Self::locstring_memo) as the Journal board's
    /// Entry has it: whose text it is where it is not its own ("StrRef
    /// 63176", in the talk table's color) in the heading's row before the
    /// languages button, not under the text; and the text reaching down to
    /// `bottom` (the room its panel gives it), however wide.
    pub(crate) fn locstring_memo_to(&mut self, ui: &mut Ui, what: &str, label: &str, bottom: f32) {
        let current = self.root.locstring(label).cloned().unwrap_or_default();
        let language = crate::text::edit_language().name().unwrap_or("?");
        let (shown, borrowed) = crate::widgets::loc_shown(self.app, &current);
        let color = talk_table_color(ui);
        let mut languages = false;
        mg_uikit::form::section_heading_with(ui, what, Some(language), |ui| {
            languages = crate::widgets::languages_button(ui);
            if let Some((from, why)) = &borrowed {
                borrowed_label(ui, from, why, color);
            }
        });
        if languages {
            self.app.loc_edit = Some(LocStringEdit::new(self.target(label), what, &current));
        }
        self.mark_mixed(ui, label);
        let id = self.id(label);
        let rows = rows_to(ui, bottom);
        let typed = ui
            .scope(|ui| {
                if borrowed.is_some() {
                    ui.visuals_mut().override_text_color = Some(color);
                }
                let buffers = &mut self.app.buffers;
                crate::widgets::commit_loc_text_in(
                    buffers,
                    ui,
                    id,
                    &shown,
                    Some(rows),
                    f32::INFINITY,
                    None,
                )
            })
            .inner;
        self.finish_loc_edit(ui, what, label, &current, &shown, typed);
    }

    /// [`memo`](Self::memo) reaching down to `bottom` (the room its panel
    /// gives it), however wide: the Journal board's Comments.
    pub(crate) fn memo_to(&mut self, ui: &mut Ui, what: &str, label: &str, bottom: f32) {
        self.mark_mixed(ui, label);
        let current = decode(self.root.string(label).unwrap_or_default());
        let id = self.id(label);
        let rows = rows_to(ui, bottom);
        let buffers = &mut self.app.buffers;
        let typed = crate::widgets::commit_text_in(
            buffers,
            ui,
            id,
            &current,
            Some(rows),
            f32::INFINITY,
            None,
        );
        if let Some(v) = typed {
            self.set(what, label, Value::String(encode(&v)));
        }
        self.hold_text(ui, what, label, &current, false, |text| Value::String(encode(&text)));
    }

    /// [`locstring`](Self::locstring) as the Journal board's Category Name
    /// has it: the whole field's width for the name, the languages button
    /// beside it, and whose text it is where it is not its own (the talk
    /// table's color) said by its tip, not inside the field over the name.
    pub(crate) fn locstring_whole(&mut self, ui: &mut Ui, what: &str, label: &str) {
        self.mark_mixed(ui, label);
        let current = self.root.locstring(label).cloned().unwrap_or_default();
        let (shown, borrowed) = crate::widgets::loc_shown(self.app, &current);
        let color = talk_table_color(ui);
        let id = self.id(label);
        let button = ui.spacing().interact_size.y + ui.spacing().item_spacing.x;
        let width = (ui.available_width() - button).max(60.0);
        let field = ui.scope(|ui| {
            if borrowed.is_some() {
                ui.visuals_mut().override_text_color = Some(color);
            }
            crate::widgets::commit_loc_text(self.app, ui, id, &shown, false, width)
        });
        if let Some((from, why)) = &borrowed {
            field.response.on_hover_text(format!("{from}: {why}"));
        }
        self.finish_loc_edit(ui, what, label, &current, &shown, field.inner);
        self.open_loc_edit_on_click(ui, what, label, &current);
    }

    /// Sets several fields in one command.
    pub(crate) fn set_fields(&mut self, what: &str, fields: Vec<(&str, Value)>) {
        let edits = self
            .targets()
            .flat_map(|(key, path)| {
                fields.iter().map(move |(label, value)| Edit::SetField {
                    key,
                    path: path.clone(),
                    label: label.to_string(),
                    value: Some(value.clone()),
                })
            })
            .collect();
        self.app.actions.push(Action::Apply(Command::new(what, edits)));
    }

    /// Sets several integer fields in one command.
    pub(crate) fn set_many(&mut self, what: &str, fields: &[(&str, i64, FieldType)]) {
        let values: Vec<(&str, Value)> = fields
            .iter()
            .map(|(label, v, t)| {
                let t = self.schema_type(label).unwrap_or(*t);
                (*label, integer(self.root.get(label), *v, t))
            })
            .collect();
        self.set_fields(what, values);
    }

    /// A slider for an integer field.
    pub(crate) fn slider(
        &mut self,
        ui: &mut Ui,
        what: &str,
        label: &str,
        range: std::ops::RangeInclusive<i64>,
    ) {
        self.mark_mixed(ui, label);
        let current = self.int(label);
        let mut v = current;
        // (Its rail shorter where the room is: its value's box beside it.)
        if narrow(ui) {
            let value = 48.0 + ui.spacing().item_spacing.x;
            let rail = ui.spacing().slider_width.min(ui.available_width() - value).max(40.0);
            ui.spacing_mut().slider_width = rail;
        }
        let r = mg_uikit::slider(
            ui,
            egui::Slider::new(&mut v, range).clamping(egui::SliderClamping::Edits),
        );
        if (r.drag_stopped() || (r.changed() && !r.dragged())) && v != current {
            self.set_int(what, label, v, FieldType::Byte);
        }
    }

    /// A float field in `range`, dragged in steps of `speed`.
    pub(crate) fn float(
        &mut self,
        ui: &mut Ui,
        what: &str,
        label: &str,
        range: std::ops::RangeInclusive<f32>,
        speed: f64,
    ) {
        self.mark_mixed(ui, label);
        let current = self.root.float(label).unwrap_or(0.0);
        if let Some(v) = crate::widgets::drag_number(ui, current, |d| {
            d.range(range).speed(speed).max_decimals(2)
        }) {
            self.set(what, label, Value::Float(v));
        }
    }

    /// [`float`](Self::float) with Aurora's pair of arrows after the
    /// field, a click `step` more or less.
    pub(crate) fn spin_float(
        &mut self,
        ui: &mut Ui,
        what: &str,
        label: &str,
        range: std::ops::RangeInclusive<f32>,
        speed: f64,
        step: f64,
    ) {
        self.mark_mixed(ui, label);
        let current = self.root.float(label).unwrap_or(0.0);
        if let Some(v) =
            crate::widgets::spin_with(ui, current, range, step, |d| d.speed(speed).max_decimals(2))
        {
            self.set(what, label, Value::Float(v));
        }
    }

    /// Milliseconds (DWORD) shown and edited as seconds, with Aurora's
    /// arrows (a click `step` seconds).
    pub(crate) fn millis(
        &mut self,
        ui: &mut Ui,
        what: &str,
        label: &str,
        range: std::ops::RangeInclusive<f32>,
        step: f64,
    ) {
        self.mark_mixed(ui, label);
        let current = self.int(label) as f32 / 1000.0;
        if let Some(v) = crate::widgets::spin_with(ui, current, range, step, |d| {
            d.speed(0.1).max_decimals(3).suffix(" s")
        }) {
            self.set_int(what, label, (v * 1000.0).round() as i64, FieldType::Dword);
        }
    }

    /// A colour stored as 0x00BBGGRR (ARE lighting), with a colour picker;
    /// what is picked is stored when the picker closes (one command).
    pub(crate) fn color(&mut self, ui: &mut Ui, what: &str, label: &str) {
        self.mark_mixed(ui, label);
        let current = self.int(label) as u32;
        let pending_id = self.id(label).with("pending");
        let pending: Option<u32> = ui.data(|d| d.get_temp(pending_id));
        let shown = pending.unwrap_or(current);
        let mut rgb = [shown & 0xFF, (shown >> 8) & 0xFF, (shown >> 16) & 0xFF].map(|c| c as u8);
        let (changed, open) = ui
            .push_id(self.id(label), |ui| {
                let popup = ui.auto_id_with("popup");
                let changed = crate::color_button::edit_srgb(ui, &mut rgb).changed();
                (changed, egui::Popup::is_id_open(ui.ctx(), popup))
            })
            .inner;
        let picked = u32::from(rgb[0]) | u32::from(rgb[1]) << 8 | u32::from(rgb[2]) << 16;
        if changed {
            ui.data_mut(|d| d.insert_temp(pending_id, picked));
        }
        if !open && let Some(v) = ui.data(|d| d.get_temp::<u32>(pending_id)) {
            ui.data_mut(|d| d.remove::<u32>(pending_id));
            if v != current {
                self.set_int(what, label, i64::from(v), FieldType::Dword);
            }
        }
    }

    /// A placed outline's own color (Moonglow's `MG_OutlineColor`, a
    /// DWORD 0x00BBGGRR the game and Aurora don't read), with the picker
    /// and its `swatches`, shown as `default` (its kind's) where it has
    /// none. Each pick is stored at once, one command: a swatch, a color
    /// typed or pasted; a drag on the picker when the button is let go
    /// (held, the color is shown, kept for this frame and the next only,
    /// so it never lands later or on another object). Default takes the
    /// field off (of each object edited together), enabled where any of
    /// them has it, of any type.
    pub(crate) fn outline_color(
        &mut self,
        ui: &mut Ui,
        default: [u8; 3],
        swatches: &[(&str, [u8; 3])],
    ) {
        let label = mg_area::OUTLINE_COLOR;
        self.mark_mixed(ui, label);
        let current = mg_rules::placement::outline_color(&self.root);
        let differs = self.differs(label);
        let pending_id = self.id(label).with("pending");
        // (A drag's color, held: kept from the last frame only. A form no
        // longer drawn while it was held leaves it stale, dropped.)
        let pass = ui.ctx().cumulative_pass_nr();
        let pending: Option<[u8; 3]> = ui
            .data(|d| d.get_temp::<([u8; 3], u64)>(pending_id))
            .filter(|(_, at)| pass.saturating_sub(*at) <= 1)
            .map(|(c, _)| c);
        ui.data_mut(|d| d.remove::<([u8; 3], u64)>(pending_id));
        let mut rgb = pending.or(current).unwrap_or(default);
        let mut reset = false;
        let id = self.id(label);
        let picked = row(ui, |ui| {
            let (changed, _open) = ui
                .push_id(id, |ui| {
                    let popup = ui.auto_id_with("popup");
                    let button = crate::color_button::edit_srgb_with(ui, &mut rgb, swatches);
                    button.widget_info(|| {
                        egui::WidgetInfo::labeled(
                            egui::WidgetType::ColorButton,
                            true,
                            OUTLINE_COLOR_LABEL,
                        )
                    });
                    let changed = button.on_hover_text(OUTLINE_COLOR_TIP).changed();
                    (changed, egui::Popup::is_id_open(ui.ctx(), popup))
                })
                .inner;
            let picked = (changed || pending.is_some()).then_some(rgb);
            // (Past MIXED_LIMIT the others aren't looked at: maybe set.
            // A field of another type, from another tool, reads as no
            // color and is still taken off.)
            let set = self.root.get(label).is_some() || differs || self.also.len() > MIXED_LIMIT;
            let button = egui::Button::new(OUTLINE_COLOR_DEFAULT).small();
            reset = ui
                .add_enabled(set, button)
                .on_hover_text("Draws the outline in its usual color again")
                .clicked();
            picked
        })
        .inner;
        if reset {
            self.set_opt(OUTLINE_COLOR_LABEL, label, None);
            return;
        }
        let Some(v) = picked else { return };
        // (Dragged on the picker: stored when the button is let go.)
        if ui.input(|i| i.pointer.any_down()) {
            ui.data_mut(|d| d.insert_temp(pending_id, (v, pass)));
            return;
        }
        if current != Some(v)
            || self.several()
            || self.root.get(label).is_some_and(|_| current.is_none())
        {
            let [r, g, b] = v.map(u32::from);
            self.set(OUTLINE_COLOR_LABEL, label, Value::Dword(r | g << 8 | b << 16));
        }
    }

    /// A checkbox for a flag (BYTE 0 or 1); the value.
    pub(crate) fn check(&mut self, ui: &mut Ui, text: &str, label: &str) -> bool {
        self.check_if(ui, true, text, label)
    }

    /// A [`check`](Self::check) that can be ticked only where `enabled`
    /// (a static placeable's Useable). (Disabled as a widget, not in a
    /// scope of its own: a scope in a form's grid cell stood its row 47
    /// to 51 points from the next, U30's #2.)
    pub(crate) fn check_if(&mut self, ui: &mut Ui, enabled: bool, text: &str, label: &str) -> bool {
        self.mark_mixed(ui, label);
        let current = self.int(label) != 0;
        let mut v = current;
        if ui.add_enabled(enabled, mg_uikit::controls::Checkbox::new(&mut v, text)).changed() {
            self.set_int(text, label, i64::from(v), FieldType::Byte);
        }
        v
    }

    /// [`check`](Self::check) whose ticking also takes the fields `gone`
    /// off each object that has them, in the same command (unticking
    /// leaves them as they are): Static and a placeable's visual
    /// transform. Undo puts them back.
    pub(crate) fn check_taking(
        &mut self,
        ui: &mut Ui,
        text: &str,
        label: &str,
        gone: &[&str],
    ) -> bool {
        self.mark_mixed(ui, label);
        let current = self.int(label) != 0;
        let mut v = current;
        if !ui.check_box(&mut v, text).changed() {
            return v;
        }
        let default = self.schema_type(label).unwrap_or(FieldType::Byte);
        let value = integer(self.root.get(label), i64::from(v), default);
        let mut command = self.setting(text, label, Some(value));
        if v {
            let targets: Vec<(ResKey, GffPath)> =
                self.targets().map(|(k, p)| (k, p.clone())).collect();
            let Some(ws) = self.app.ws.as_mut() else { return v };
            for (key, path) in targets {
                let Some(s) = ws.doc(&key).ok().and_then(|d| path.get(&d.root).cloned()) else {
                    continue;
                };
                for g in gone.iter().filter(|g| s.contains(g)) {
                    command.edits.push(Edit::SetField {
                        key,
                        path: path.clone(),
                        label: (*g).to_string(),
                        value: None,
                    });
                }
            }
        }
        self.app.actions.push(Action::Apply(command));
        v
    }

    /// A number in `range`.
    pub(crate) fn number(
        &mut self,
        ui: &mut Ui,
        what: &str,
        label: &str,
        range: std::ops::RangeInclusive<i64>,
    ) {
        self.mark_mixed(ui, label);
        if let Some(v) = commit_number(ui, self.int(label), range) {
            self.set_int(what, label, v, FieldType::Byte);
        }
    }

    /// [`number`](Self::number) `width` wide, its number at its right end
    /// (the Journal board's ID and XP): egui's field centers it, so it is
    /// drawn with its text unseen and the number painted at the right;
    /// typed in, the field's own text shows (at the right too, from the
    /// layout). Its response.
    pub(crate) fn number_at_right(
        &mut self,
        ui: &mut Ui,
        what: &str,
        label: &str,
        range: std::ops::RangeInclusive<i64>,
        width: f32,
    ) -> egui::Response {
        self.mark_mixed(ui, label);
        let current = self.int(label);
        let size = egui::vec2(width, mg_uikit::metrics::CONTROL_HEIGHT);
        let layout = egui::Layout::right_to_left(egui::Align::Center);
        let (set, r) = ui
            .allocate_ui_with_layout(size, layout, |ui| {
                ui.spacing_mut().interact_size.x = width;
                // (The field's id, as `drag_number_shown` takes it.)
                let editing = ui.memory(|m| m.has_focus(ui.next_auto_id()));
                if !editing {
                    ui.visuals_mut().override_text_color = Some(egui::Color32::TRANSPARENT);
                }
                let (set, shown, r) =
                    crate::widgets::drag_number_shown(ui, current, |d| d.range(range));
                if !editing {
                    let style = ui.style().drag_value_text_style.clone();
                    let font = style.resolve(ui.style());
                    let color = ui.style().interact(&r).text_color();
                    let galley = ui.painter().layout_no_wrap(shown.to_string(), font, color);
                    let inner = r.rect.shrink2(ui.spacing().button_padding);
                    let at = egui::pos2(
                        inner.right() - galley.size().x,
                        inner.center().y - galley.size().y / 2.0,
                    );
                    ui.painter().galley(at, galley, color);
                }
                (set, r)
            })
            .inner;
        if let Some(v) = set {
            self.set_int(what, label, v, FieldType::Byte);
        }
        r
    }

    /// [`number`](Self::number) with Aurora's pair of arrows after the
    /// field (`widgets::spin_number`), as Aurora's number fields have.
    pub(crate) fn spin(
        &mut self,
        ui: &mut Ui,
        what: &str,
        label: &str,
        range: std::ops::RangeInclusive<i64>,
    ) {
        self.mark_mixed(ui, label);
        if let Some(v) = crate::widgets::spin_number(ui, self.int(label), range) {
            self.set_int(what, label, v, FieldType::Byte);
        }
    }

    /// A PLT colour (0–175) picked from a palette's swatches (Aurora's
    /// colour chooser; `palette` as `pal_cloth01`); the game is given, as
    /// the pages that show one have lent it out.
    pub(crate) fn palette_color(
        &mut self,
        ui: &mut Ui,
        game: &mg_rules::GameData,
        what: &str,
        label: &str,
        palette: &'static str,
    ) {
        self.mark_mixed(ui, label);
        let current = self.int(label).clamp(0, 175) as u8;
        let pick = self.palette_pick(ui, game, what, label, palette, current, None);
        if let Some(v) = pick.filter(|&v| v != current) {
            self.set_int(what, label, i64::from(v), FieldType::Byte);
        }
    }

    /// [`Form::palette_color`]'s chooser alone, showing colour `current`
    /// (with `text` in place of its number): the colour picked.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn palette_pick(
        &mut self,
        ui: &mut Ui,
        game: &mg_rules::GameData,
        what: &str,
        label: &str,
        palette: &'static str,
        current: u8,
        text: Option<&str>,
    ) -> Option<u8> {
        let pal = {
            let app = &mut *self.app;
            let l = crate::images::Loader {
                pictures: &mut app.pictures,
                palettes: &app.palettes,
                ws: app.ws.as_ref(),
                game,
            };
            l.palette(palette)
        };
        let id = ("palette-color", self.id(label));
        crate::widgets::palette_color(ui, id, pal.as_deref(), what, current, text)
    }

    /// A choice among 2DA rows (the field holds the row; `default` is its
    /// type when the blueprint lacks it). Long lists get a filter.
    pub(crate) fn choice(
        &mut self,
        ui: &mut Ui,
        what: &str,
        label: &str,
        choices: &[Choice],
        default: FieldType,
    ) {
        self.mark_mixed(ui, label);
        self.choice_shown(ui, what, label, choices, default, None);
    }

    /// A creature's or a placeable's appearance: [`choice`](Self::choice),
    /// with each appearance's picture beside the row the pointer rests on.
    pub(crate) fn appearance_choice(
        &mut self,
        ui: &mut Ui,
        label: &str,
        choices: &[Choice],
        default: FieldType,
        creature: bool,
    ) {
        self.mark_mixed(ui, label);
        if creature {
            let dressed = self.app.thumbnails.creature(&self.root);
            let mut show = move |app: &mut Moonglow, ui: &mut Ui, row| {
                creature_picture(app, ui, dressed, row);
            };
            self.choice_shown(ui, "Appearance", label, choices, default, Some(&mut show));
        } else {
            let mut show = placeable_picture;
            self.choice_shown(ui, "Appearance", label, choices, default, Some(&mut show));
        }
    }

    /// [`choice`](Self::choice), with what `about` shows of a row (its
    /// picture) beside the row the pointer rests on.
    pub(crate) fn pictured_choice(
        &mut self,
        ui: &mut Ui,
        (what, label): (&str, &str),
        choices: &[Choice],
        default: FieldType,
        about: Shown<'_>,
    ) {
        self.mark_mixed(ui, label);
        self.choice_shown(ui, what, label, choices, default, Some(about));
    }

    /// The loading screen (a loadscreens.2da row): its picture shows under
    /// the choice, and each one's on the row the pointer rests on.
    pub(crate) fn load_screen(&mut self, ui: &mut Ui, choices: &[Choice]) {
        self.mark_mixed(ui, "LoadScreenID");
        ui.vertical(|ui| {
            self.choice_shown(
                ui,
                "Loading screen",
                "LoadScreenID",
                choices,
                FieldType::Word,
                Some(&mut load_screen_picture),
            );
            let row = self.int("LoadScreenID");
            if let Ok(row) = usize::try_from(row) {
                load_screen_picture(self.app, ui, row);
            }
        });
    }

    /// [`choice`](Self::choice), with what `about` shows of a row beside
    /// the one the pointer rests on.
    fn choice_shown(
        &mut self,
        ui: &mut Ui,
        what: &str,
        label: &str,
        choices: &[Choice],
        default: FieldType,
        about: Option<Shown<'_>>,
    ) {
        let current = self.int(label);
        let id = self.id(label);
        let app = &mut *self.app;
        let mut shown = about.map(|about| move |ui: &mut Ui, row| about(app, ui, row));
        let mut list = crate::widgets::row_choice(id, choices, current).width(fit(ui, 220.0));
        if let Some(shown) = shown.as_mut() {
            list = list.about(shown);
        }
        if let Some(v) = list.show(ui) {
            self.set_int(what, label, v, default);
        }
    }

    /// A script field: name, picker and Edit.
    pub(crate) fn script(&mut self, ui: &mut Ui, what: &str, label: &str) {
        self.mark_mixed(ui, label);
        let types = [ResType::NSS, ResType::NCS];
        let what = format!("{what} script");
        let current = self.root.resref(label).unwrap_or(ResRef::EMPTY);
        let yield_to = resref_beside_buttons(ui);
        ui.horizontal(|ui| {
            let id = self.id(label);
            let picked = crate::widgets::with_resref_width(ui, yield_to, |ui| {
                resref_field(self.app, ui, id, current, &what, &types)
            });
            if let Some(v) = picked {
                self.set(&what, label, Value::resref(v));
            }
            self.hold_resref(ui, &what, label, current);
            let field = self.id(label);
            if let Some(name) = crate::widgets::edit_script_button(self.app, ui, field, current) {
                self.app.actions.push(Action::EditScript { name, condition: false });
            }
        });
    }

    /// Load Script Set and Save Script Set (`.ini`, `mg_module::script_set`)
    /// for a page's events (label, field); nothing for a page whose events
    /// script sets do not name (a store's). Loading sets every event of the
    /// page, those the file leaves out to none, in one command.
    pub(crate) fn script_set_buttons(&mut self, ui: &mut Ui, events: &[(&str, &str)]) {
        use mg_module::script_set;
        let keyed: Vec<(&'static str, &str)> = events
            .iter()
            .filter_map(|(_, field)| Some((script_set::key(field)?, *field)))
            .collect();
        if keyed.is_empty() {
            return;
        }
        let (mut load, mut save) = (false, false);
        row(ui, |ui| {
            load = ui
                .button("Load Script Set")
                .on_hover_text("Set the scripts from a script set file (.ini)")
                .clicked();
            save = ui
                .button("Save Script Set")
                .on_hover_text("Save these scripts as a script set file (.ini)")
                .clicked();
        });
        if load {
            // The game's script sets are in data/scr.
            let start = self.app.install.as_ref().map(|i| i.root.join("data").join("scr"));
            let Some(path) = self.app.dialogs.open_file(FileKind::ScriptSet, start.as_deref())
            else {
                return;
            };
            let text = match std::fs::read(&path) {
                Ok(b) => decode(&b),
                Err(e) => {
                    self.app.log.error(format!("{}: {e}", path.display()));
                    return;
                }
            };
            let set = script_set::read(&text);
            let mut fields = Vec::new();
            for (key, field) in &keyed {
                let name = set.get(key).map_or("", String::as_str);
                match ResRef::from_str(name) {
                    Ok(r) => fields.push((*field, Value::resref(r))),
                    Err(_) => self
                        .app
                        .log
                        .warn(format!("{}: {key}: {name:?} is not a script name", path.display())),
                }
            }
            self.set_fields("Load script set", fields);
        }
        if save
            && let Some(path) = self
                .app
                .dialogs
                .save_file(FileKind::ScriptSet, Some(std::path::Path::new("scripts.ini")))
        {
            let names: Vec<(&str, String)> = keyed
                .iter()
                .map(|(key, field)| {
                    (*key, self.root.resref(field).unwrap_or(ResRef::EMPTY).to_string())
                })
                .collect();
            let pairs: Vec<(&str, &str)> = names.iter().map(|(k, v)| (*k, v.as_str())).collect();
            let text = encode(&script_set::write(&pairs));
            if let Err(e) = mg_module::write_replacing(&path, &text) {
                self.app.log.error(format!("{}: {e}", path.display()));
            }
        }
    }

    /// The conversation: name, picker and Edit.
    pub(crate) fn conversation(&mut self, ui: &mut Ui) {
        self.linked(ui, "Conversation", "Conversation", &[ResType::DLG], ResType::DLG, Tab::Dialog);
    }

    /// A resource name with a picker (of `types`) and Edit, which opens the
    /// module's `edit` resource in its editor (`tab`), else the game's.
    fn linked(
        &mut self,
        ui: &mut Ui,
        what: &str,
        label: &str,
        types: &[ResType],
        edit: ResType,
        tab: fn(ResKey) -> Tab,
    ) {
        self.mark_mixed(ui, label);
        let current = self.root.resref(label).unwrap_or(ResRef::EMPTY);
        let yield_to = resref_beside_buttons(ui);
        ui.horizontal(|ui| {
            let id = self.id(label);
            let picked = crate::widgets::with_resref_width(ui, yield_to, |ui| {
                resref_field(self.app, ui, id, current, what, types)
            });
            if let Some(v) = picked {
                self.set(what, label, Value::resref(v));
            }
            self.hold_resref(ui, what, label, current);
            let key = ResKey::new(current, edit);
            let in_module = self.app.ws.as_ref().is_some_and(|w| w.module.contains(&key));
            let in_game = self.app.game.as_deref().is_some_and(|g| g.resman.contains(&key));
            if current.is_empty() || !(in_module || in_game) {
                // (Empty, or a name nothing has: New makes it, as File ›
                // New Conversation does, and gives it to the object.)
                let module = self.app.ws.is_some();
                let kind = match self.restype() {
                    ResType::UTC => "creature",
                    ResType::UTP => "placeable",
                    ResType::UTD => "door",
                    _ => "object",
                };
                let tip = if !module {
                    "Open a module to make a conversation in it".to_string()
                } else if current.is_empty() {
                    format!("Make a new conversation for this {kind} and open it")
                } else {
                    format!("The module has no conversation of this name: make it for this {kind} and open it")
                };
                let button = ui.add_enabled(module, egui::Button::new("New").small());
                if button.on_hover_text(tip).on_disabled_hover_text("Open a module to make a conversation in it").clicked() {
                    let name = if current.is_empty() {
                        self.new_conversation_name()
                    } else {
                        current.to_string()
                    };
                    self.app.new_dialog = Some(name);
                    self.app.new_dialog_for =
                        Some((self.targets().map(|(k, p)| (k, p.clone())).collect(), label.to_string()));
                }
            } else if ui.add(egui::Button::new("Edit").small()).clicked() {
                self.app.actions.push(Action::OpenTab(if in_module { tab(key) } else { Tab::Resource(key) }));
            }
        });
    }

    /// The name suggested for a new conversation of this object: its
    /// blueprint's resref or its tag where that is a free name in the
    /// module, else the next free number of the first (or of "conversation").
    fn new_conversation_name(&self) -> String {
        let Some(ws) = self.app.ws.as_ref() else { return String::new() };
        let naming = ws.module.project_file.naming_for(ResType::DLG);
        let taken = |name: &str| {
            ResRef::from_str(name).is_ok_and(|r| ws.module.contains(&ResKey::new(r, ResType::DLG)))
        };
        let blueprint = if self.is_instance() {
            self.root.resref("TemplateResRef")
        } else {
            Some(self.key.resref)
        };
        let tag = decode(self.root.string("Tag").unwrap_or_default());
        let bases: Vec<String> = blueprint
            .map(|r| r.to_string())
            .into_iter()
            .chain(std::iter::once(tag))
            .filter(|b| !b.is_empty())
            .collect();
        for base in &bases {
            if let Some(r) = naming.resref(base, "")
                && !taken(&r.to_string())
            {
                return r.to_string();
            }
        }
        let base = bases
            .iter()
            .find(|b| naming.resref(b, "001").is_some())
            .map_or("conversation", String::as_str);
        let numbered =
            naming.numbered(base, 1, &taken).or_else(|| naming.numbered("conversation", 1, &taken));
        numbered.map(|r| r.to_string()).unwrap_or_default()
    }

    /// The palette category (Aurora: read-only text and `…`): a list of the
    /// type's categories.
    pub(crate) fn category(&mut self, ui: &mut Ui, kind: BlueprintKind) {
        if self.is_instance() {
            ui.weak("(placed in an area)");
            return;
        }
        if self.app.game.is_none() {
            return;
        }
        // The module's own categories (Palette › Categories…), else the
        // game's; in the palette's order (its tree's, as Palette ›
        // Categories… sets it), or alphabetical (Options › General).
        let categories = crate::palette_view::categories(self.app, kind);
        let label = kind.palette_field();
        self.mark_mixed(ui, label);
        let current = self.int(label);
        let width = ui.available_width().clamp(120.0, 260.0);
        let list = crate::widgets::row_choice(self.id(label), &categories, current);
        if let Some(id) = list.width(width).ordered().show(ui) {
            self.set_int("Palette category", label, id, FieldType::Byte);
        }
    }

    /// The Variables page: the list in place ([`crate::widgets::variables_page`]),
    /// read-only for an object of a read-only area.
    pub(crate) fn variables_page(&mut self, ui: &mut Ui) {
        self.mark_mixed(ui, "VarTable");
        let locked =
            crate::claims::area_of(&self.key).and_then(|a| crate::claims::read_only(self.app, a));
        let target = self.target("VarTable");
        let root = std::mem::replace(&mut self.root, Struct::new(0));
        let list = root.list("VarTable").unwrap_or(&[]);
        crate::widgets::variables_page(self.app, ui, target, list, locked);
        self.root = root;
    }

    /// The Variables… button (off for an object of a read-only area).
    pub(crate) fn variables(&mut self, ui: &mut Ui) {
        let count = self.root.list("VarTable").map_or(0, <[_]>::len);
        self.mark_mixed(ui, "VarTable");
        if crate::claims::variables_button(self.app, ui, &self.key, count) {
            let list = self.root.list("VarTable").unwrap_or(&[]);
            self.app.var_edit = Some(VarTableEdit::new(self.target("VarTable"), list));
        }
    }

    /// Update Instances (Aurora's Advanced pages, for blueprints of every
    /// type but waypoints): every object placed from this blueprint, in
    /// every area, made again from it where it stands, facing as it faces
    /// (a trigger or encounter keeping its outline and spawn points). One
    /// command.
    pub(crate) fn update_instances(&mut self, ui: &mut Ui) {
        if self.is_instance() {
            return;
        }
        let r = ui
            .button("Update Instances")
            .on_hover_text("Update all instances created from this blueprint");
        if r.clicked() {
            let keys = self.targets().map(|(k, _)| k).collect();
            self.app.update_instances_of(keys);
        }
    }

    /// The blueprint's resref (Aurora's "Blueprint ResRef"): changing it
    /// renames the blueprint.
    pub(crate) fn blueprint_resref(&mut self, ui: &mut Ui) {
        if self.several_blueprints() {
            ui.weak(format!("{} and {} more", self.key.resref, self.also.len()))
                .on_hover_text("≠ before a field: the others have another value there");
            return;
        }
        if self.is_instance() {
            // A placed object names the blueprint it was made from.
            let field = BlueprintKind::from_restype(self.restype())
                .map_or("TemplateResRef", |k| k.resref_field());
            let template = self.root.resref(field).map(|r| r.to_string()).unwrap_or_default();
            ui.label(template).on_hover_text("The blueprint it was placed from");
            return;
        }
        let current = self.key.resref;
        let id = self.id("TemplateResRef");
        let buf = self.app.buffers.entry(id).or_insert_with(|| current.to_string());
        let mut text = crate::widgets::ResRefText(&mut *buf);
        let r = ui
            .add(crate::widgets::resref_edit(&mut text).id(id).desired_width(fit(ui, 160.0)))
            .on_hover_text("Edit the blueprint ResRef");
        crate::widgets::leaves_at_a_press(ui.ctx(), &r);
        if !crate::widgets::has_keys(&r) && !r.lost_focus() && *buf != current.to_string() {
            *buf = current.to_string();
        }
        if r.lost_focus() && *buf != current.to_string() {
            let text = buf.trim().to_ascii_lowercase();
            match ResRef::from_str(&text) {
                Ok(new) if !new.is_empty() => {
                    self.app.actions.push(Action::RenameBlueprint { from: self.key, to: new })
                }
                Ok(_) => self.app.log.error("A blueprint needs a resref"),
                Err(e) => self.app.log.error(e.to_string()),
            }
        }
        let from = self.key;
        self.hold_action(ui, id, |buf| {
            let text = buf.trim().to_ascii_lowercase();
            let new = ResRef::from_str(&text).ok().filter(|n| !n.is_empty() && *n != current)?;
            Some(Action::RenameBlueprint { from, to: new })
        });
    }
}

/// The objects of type `restype` a command changed (other than by setting
/// `derived`, the field kept from them): blueprints at their root, placed
/// objects at their GIT entry.
pub(crate) fn changed_objects(
    cmd: &Command,
    restype: ResType,
    derived: &[&str],
) -> Vec<(ResKey, GffPath)> {
    let list = mg_module::instances::git_list(restype).map(|(l, _)| l);
    let mut out: Vec<(ResKey, GffPath)> = Vec::new();
    for e in &cmd.edits {
        let (key, path) = match e {
            Edit::SetField { key, path, label, .. } if !derived.contains(&label.as_str()) => {
                (key, path)
            }
            Edit::InsertItem { key, path, .. } | Edit::RemoveItem { key, path, .. } => (key, path),
            _ => continue,
        };
        let object = if key.restype == restype {
            GffPath::root()
        } else if key.restype == ResType::GIT
            && restype == ResType::UTI
            && let Some(n) = held_item_at(path)
        {
            // (An item a placed object holds, changed in its own
            // Properties.)
            GffPath(path.0[..n].to_vec())
        } else if key.restype == ResType::GIT
            && let Some(step @ mg_edit::Step::Item(l, _)) = path.0.first()
            && Some(l.as_str()) == list
        {
            GffPath(vec![step.clone()])
        } else {
            continue;
        };
        if !out.contains(&(*key, object.clone())) {
            out.push((*key, object));
        }
    }
    out
}

/// The objects whose derived values a command leaves to bring up to date
/// (worked out before it is applied, which takes the command).
pub(crate) struct Derived {
    items: Vec<(ResKey, GffPath)>,
    creatures: Vec<(ResKey, GffPath)>,
}

pub(crate) fn derived_of(cmd: &Command) -> Derived {
    Derived {
        items: changed_objects(cmd, ResType::UTI, &["Cost"]),
        creatures: changed_objects(
            cmd,
            ResType::UTC,
            &["MaxHitPoints", "ChallengeRating", "CurrentHitPoints"],
        ),
    }
}

/// After a command: values derived from what it changed (an item's cost, a
/// creature's maximum hit points), as part of it.
pub(crate) fn after_apply(app: &mut Moonglow, derived: Derived) {
    item::refresh_costs(app, derived.items);
    creature::refresh_hit_points(app, derived.creatures);
}

/// Renames a blueprint everywhere (its resource, its `TemplateResRef` or a
/// store's `ResRef`, and the objects placed from it) as one undoable
/// command, and points its editor at the new name.
pub(crate) fn rename(app: &mut Moonglow, from: ResKey, to: ResRef) {
    app.rename_resource(from, to, false);
}

/// A blueprint editor tab.
pub(crate) fn ui(app: &mut Moonglow, ui: &mut Ui, key: ResKey) {
    edit(app, ui, key, GffPath::root());
}

/// The blueprint type of the object at `path` in a GIT (its list's).
pub(crate) fn instance_type(path: &GffPath) -> Option<ResType> {
    // An item held by a placed object (in its inventory, or equipped) is
    // an item whatever holds it.
    if held_item_at(path).is_some_and(|n| n == path.0.len()) {
        return Some(ResType::UTI);
    }
    let Some(mg_edit::Step::Item(list, _)) = path.0.first() else { return None };
    [
        ResType::UTC,
        ResType::UTD,
        ResType::UTE,
        ResType::UTI,
        ResType::UTP,
        ResType::UTS,
        ResType::UTM,
        ResType::UTT,
        ResType::UTW,
    ]
    .into_iter()
    .find(|t| mg_module::instances::git_list(*t).is_some_and(|(l, _)| l == list))
}

/// The lists a placed object holds items in.
pub(crate) const HELD_LISTS: [&str; 2] = ["ItemList", "Equip_ItemList"];

/// Whether `path` (in an area's GIT) is an item a placed object holds or a
/// store sells, which opens in Properties of its own: not an item in a bag
/// held, nor anything further in.
pub(crate) fn held_item(path: &GffPath) -> bool {
    held_item_at(path).is_some_and(|n| n == path.0.len())
}

/// Where `path` goes into an item a placed object holds: how many of its
/// steps lead to the item (the rest go on into the item).
fn held_item_at(path: &GffPath) -> Option<usize> {
    path.0.iter().skip(1).position(|step| {
        matches!(step, mg_edit::Step::Item(list, _) if HELD_LISTS.contains(&list.as_str()))
    })
    .map(|i| i + 2)
}

/// The pages of a placed object's Properties: its blueprint's, less the
/// blueprint's comments.
pub fn instance_pages(t: ResType) -> Vec<&'static str> {
    pages(t).iter().copied().filter(|p| *p != "Comments").collect()
}

/// The width under which a Properties form is laid out narrow (the
/// Properties panel, at the Right Dock's 280 to 460 or so): its scroll
/// vertical alone, text wrapping, a form's two columns sharing the width
/// ([`form_grid`]), rows of several controls wrapping, so that nothing
/// runs past the panel's edge. A window keeps its look above it.
pub(crate) const NARROW_FORM: f32 = 400.0;

/// The width of a localized name's field in a window (`Form::locstring`):
/// its row's room, held to these.
const LOC_FIELD: std::ops::Range<f32> = 160.0..360.0;

/// How wide a resref field may be so that its `…` and an **Edit** button
/// fit beside it on one line, in a narrow Properties panel too (a script's
/// or a conversation's field; Module Properties' events at a window of 420
/// put both past its edge, U30's #14). The room left in the row, not a
/// column's width: no feedback.
fn resref_beside_buttons(ui: &Ui) -> f32 {
    let s = ui.spacing();
    let small = egui::TextStyle::Button.resolve(ui.style());
    let key = (small.size.to_bits(), ui.ctx().pixels_per_point().to_bits());
    let id = egui::Id::new("bp-edit-btn-w");
    let edit =
        ui.data(|d| d.get_temp::<((u32, u32), f32)>(id)).filter(|(k, _)| *k == key).map_or_else(
            || {
                let w = ui
                    .fonts_mut(|f| {
                        f.layout_no_wrap("Edit".into(), small, egui::Color32::PLACEHOLDER)
                    })
                    .size()
                    .x;
                ui.data_mut(|d| d.insert_temp(id, (key, w)));
                w
            },
            |(_, w)| w,
        );
    let buttons = s.interact_size.y + edit + 2.0 * s.button_padding.x;
    let width = ui.available_width() - buttons - 3.0 * s.item_spacing.x - 8.0;
    let least = if narrow(ui) { 36.0 } else { SCRIPT_FIELD_LEAST };
    width.clamp(least, 160.0)
}

/// The least a script's field yields to ([`Form::script`]): about eight
/// characters of a resref.
const SCRIPT_FIELD_LEAST: f32 = 60.0;

/// The room inside a field's right end round the words that say whose its
/// text is ("StrRef 24650"): the board's `.field .keys` padding.
const KEYS_PADDING: f32 = 8.0;

/// Where a field drawn by `widgets::commit_field` finds room to keep at
/// its right end, inside it, for words drawn there (`Form::locstring`).
pub(crate) const FIELD_END_ROOM: &str = "blueprint-field-end-room";

/// Whether the form being drawn is laid out narrow: as [`form_ui`] found
/// its room ([`NARROW_FORM`]), whatever cell or column of it `ui` is (a
/// window's grid cell is narrower than that, and keeps its look); outside
/// a form (Area and Module Properties' pages, a wizard's), never.
pub(crate) fn narrow(ui: &Ui) -> bool {
    ui.data(|d| d.get_temp::<bool>(egui::Id::new(NARROW_ID))).unwrap_or(false)
}

/// Whether the form being drawn is the Properties panel's single-column
/// all-pages scroll (`AllPages { panel: true }` in the wide layout).
pub(crate) fn panel_scroll(ui: &Ui) -> bool {
    ui.data(|d| d.get_temp::<bool>(egui::Id::new(PANEL_SCROLL_ID))).unwrap_or(false)
}

/// The page name whose own heading was already drawn above its body in
/// the Properties panel (`None` for a blueprint window without a panel heading).
fn panel_page(ui: &Ui) -> Option<String> {
    ui.data(|d| d.get_temp::<String>(egui::Id::new(PANEL_PAGE_ID)))
}

/// Where [`form_ui`] keeps whether the form it draws is narrow.
const NARROW_ID: &str = "blueprint-form-narrow";
/// Where [`form_ui`] keeps whether it is drawing the Properties panel's one scroll.
const PANEL_SCROLL_ID: &str = "blueprint-form-panel-scroll";
/// Where [`form_ui`] keeps the current panel page name so sub-headings avoid duplicating it.
const PANEL_PAGE_ID: &str = "blueprint-form-panel-page";

/// `add` drawn as a narrow form's ([`narrow`]): a panel's fields (the
/// journal's Entry and Category, M4's U22) fill the room they are given,
/// never wider.
pub(crate) fn narrow_form<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
    ui.data_mut(|d| d.insert_temp(egui::Id::new(NARROW_ID), true));
    let r = add(ui);
    ui.data_mut(|d| d.remove::<bool>(egui::Id::new(NARROW_ID)));
    r
}

/// A form's grid of labels and values, two columns (the pages' `Grid`s):
/// in a narrow room ([`narrow`]) each column at most half the width, its labels wrapping there, the value column taking the
/// rest (its fields fill what they are given).
pub(crate) fn form_grid(ui: &Ui, id: impl egui::AsIdSalt) -> FormGrid {
    let narrow = narrow(ui);
    let grid = egui::Grid::new(id).num_columns(2);
    let half = (ui.available_width() * 0.5).floor();
    let grid = match narrow {
        true => grid.max_col_width(half),
        false => grid,
    };
    FormGrid { grid, narrow, half }
}

/// A form's grid ([`form_grid`]): egui's, its text wrapping within its
/// columns in a narrow room (with the columns held to their widths, a
/// label wraps at half the width, never at a column's first, unknown
/// width).
pub(crate) struct FormGrid {
    grid: egui::Grid,
    narrow: bool,
    half: f32,
}

impl FormGrid {
    pub(crate) fn spacing(mut self, spacing: impl Into<egui::Vec2>) -> Self {
        self.grid = self.grid.spacing(spacing);
        self
    }

    /// The labels' column at least `mg_uikit::metrics::LABEL_WIDTH` (in a
    /// narrow room, at most half the width still).
    pub(crate) fn labels(mut self) -> Self {
        let least = match self.narrow {
            true => mg_uikit::metrics::LABEL_WIDTH.min(self.half),
            false => mg_uikit::metrics::LABEL_WIDTH,
        };
        self.grid = self.grid.min_col_width(least);
        self
    }

    pub(crate) fn striped(mut self, striped: bool) -> Self {
        self.grid = self.grid.striped(striped);
        self
    }

    pub(crate) fn show<R>(
        self,
        ui: &mut Ui,
        add: impl FnOnce(&mut Ui) -> R,
    ) -> egui::InnerResponse<R> {
        match self.narrow {
            true => ui.scope(|ui| {
                ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
                self.grid.show(ui, add).inner
            }),
            false => self.grid.show(ui, add),
        }
    }
}

/// A form's section (M4's U10, the Creature board's): its heading
/// (`widgets::section_heading`), then its grid of labels and values
/// ([`form_grid`], kept by `id`), the labels' column at least the
/// stylesheet's `--lbl-w` (`mg_uikit::metrics::LABEL_WIDTH`) so that the
/// sections of a page line up.
pub(crate) fn section(
    ui: &mut Ui,
    title: &str,
    id: impl egui::AsIdSalt,
    add: impl FnOnce(&mut Ui),
) {
    crate::widgets::section_heading(ui, title);
    form_grid(ui, id).labels().spacing([12.0, 6.0]).show(ui, add);
}

/// The room from `ui`'s cursor to its right edge (in a grid's cell, to
/// the page's right edge): what a row of fixed widths has before it runs
/// past a column of pages side by side (M4's U10).
pub(crate) fn room_right(ui: &Ui) -> f32 {
    ui.max_rect().right() - ui.cursor().min.x
}

/// A list with what it gives beside it (a creature's spells to choose
/// from, and those it has): the list the wider, the side
/// [`SIDE_SHARE`] of the room (between [`SIDE_LEAST`] and
/// [`SIDE_MOST`]), each clipped to its part; in a narrow form, halves as
/// `Ui::columns` gives them.
pub(crate) fn list_beside<R>(ui: &mut Ui, add: impl FnOnce(&mut [Ui]) -> R) -> R {
    if narrow(ui) {
        return ui.columns(2, add);
    }
    let gap = ui.spacing().item_spacing.x;
    let total = ui.available_width();
    let side = (total * SIDE_SHARE).clamp(SIDE_LEAST, SIDE_MOST).min(total / 2.0);
    let list = (total - side - gap).max(0.0);
    let top = ui.cursor().min;
    let high = ui.available_height();
    let part =
        |x: f32, w: f32| egui::Rect::from_min_size(egui::pos2(x, top.y), egui::vec2(w, high));
    let mut parts = [part(top.x, list), part(top.x + list + gap, side)].map(|r| {
        let mut child = ui.new_child(
            egui::UiBuilder::new().max_rect(r).layout(egui::Layout::top_down(egui::Align::Min)),
        );
        child.set_clip_rect(r.intersect(ui.clip_rect()));
        child
    });
    // (A name longer than the side cut short, its whole in its tip. Not
    // the list's: its grid's columns, laid out by their first frame, would
    // cut every name to that.)
    parts[1].style_mut().wrap_mode = Some(egui::TextWrapMode::Truncate);
    let out = add(&mut parts);
    let used = parts[0].min_rect().union(parts[1].min_rect());
    ui.advance_cursor_after_rect(used);
    out
}

/// The share of the room what a list gives takes beside it
/// ([`list_beside`]: the board's Spells, `minmax(0, 1fr) 140px`).
const SIDE_SHARE: f32 = 0.35;
/// The least and most what a list gives takes beside it.
const SIDE_LEAST: f32 = 150.0;
const SIDE_MOST: f32 = 320.0;

/// A row of controls: in a narrow room ([`narrow`]) wrapping onto the
/// next line where the panel's width ends, else one line as a window has
/// it. (Held in a column of its own: a grid's row grows with what is
/// wrapped in a column, not with a wrapping row placed in it alone, whose
/// lines past the first the grid's next row was drawn over.)
pub(crate) fn row<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> egui::InnerResponse<R> {
    match narrow(ui) {
        true => {
            let shown = ui.vertical(|ui| ui.horizontal_wrapped(add));
            egui::InnerResponse::new(shown.inner.inner, shown.response)
        }
        false => ui.horizontal(add),
    }
}

/// Two parts side by side, top aligned (a page's palette and its list):
/// in a narrow room ([`narrow`]) the first over the second.
pub(crate) fn beside_or_over<R>(
    ui: &mut Ui,
    add: impl FnOnce(&mut Ui) -> R,
) -> egui::InnerResponse<R> {
    let layout = match narrow(ui) {
        true => egui::Layout::top_down(egui::Align::Min),
        false => egui::Layout::left_to_right(egui::Align::Min),
    };
    ui.with_layout(layout, add)
}

/// A part's width beside another, `want`; in a narrow room, where the
/// parts stand one over the other ([`beside_or_over`]), the whole width.
/// (`narrow`: [`narrow`] of the room both parts share, asked before them.)
pub(crate) fn part_width(narrow: bool, ui: &Ui, want: f32) -> f32 {
    if narrow { ui.available_width() } else { want }
}

/// A column of controls between two parts side by side (a list's Add and
/// Remove); in a narrow room, a row of them wrapping.
pub(crate) fn down_or_row<R>(
    narrow: bool,
    ui: &mut Ui,
    add: impl FnOnce(&mut Ui) -> R,
) -> egui::InnerResponse<R> {
    match narrow {
        true => ui.horizontal_wrapped(add),
        false => ui.vertical(add),
    }
}

/// The width a control asks for, `want`, held to the room `ui` has (in
/// the narrow panel: a field fills its column, never past its edge).
pub(crate) fn fit(ui: &Ui, want: f32) -> f32 {
    match narrow(ui) {
        true => want.min(ui.available_width()).max(0.0),
        false => want,
    }
}

/// The Properties of the object at `path` in the document `key` (a
/// blueprint at the root, or an object placed in an area's GIT).
pub(crate) fn edit(app: &mut Moonglow, ui: &mut Ui, key: ResKey, path: GffPath) {
    edit_many(app, ui, key, path, Vec::new());
}

/// The Properties of several objects or blueprints of one type at once
/// (Aurora's multi-editor for placed objects): shown as the first, each
/// field changed set on all.
pub(crate) fn edit_many(
    app: &mut Moonglow,
    ui: &mut Ui,
    key: ResKey,
    path: GffPath,
    also: Vec<(ResKey, GffPath)>,
) {
    let Some(root) = object_now(app, ui, &key, &path) else { return };
    let restype = instance_type(&path).unwrap_or(key.restype);
    // The window this is, and the objects it edits.
    let targets: Vec<(ResKey, GffPath)> =
        std::iter::once((key, path.clone())).chain(also.iter().cloned()).collect();
    let tab = tab_of(key, &path, &also);
    let (held, is_viewed) = looked_at(app, &key);
    // (Named, for a screen reader and the tests: the Properties panel
    // draws the same fields.)
    mg_uikit::name_group(ui, WINDOW_GROUP);
    let (name, sub) = title_of(app, &key, &path, &root, also.len());
    let kind = kind_name(restype);
    let header = match path.0.is_empty() {
        true => format!("{kind} blueprint header"),
        false => format!("{kind} properties header"),
    };
    let title = Title { header, icon: crate::tabs::icon(&tab), name: &name, sub: &sub };
    let pop_in = !path.0.is_empty() && app.docked_instance_tabs.contains(&tab);
    let (pressed, popped_in) =
        ok_cancel_header_with(ui, &title, is_viewed || held.is_some(), OK_CANCEL_TIPS, pop_in);
    if popped_in {
        app.actions.push(Action::PopInProperties(tab.clone()));
    }
    // Multi-selection edits all targets' lists together.
    let pages: Vec<&str> =
        if path.0.is_empty() { pages(restype).to_vec() } else { instance_pages(restype) };
    let page_key = (key, path.clone());
    let asked = app.blueprint_pages.get(&page_key).copied();
    let page = asked.or_else(|| app.open_tabs.page(&tab, &pages)).unwrap_or(pages[0]);
    let mut top = pages.iter().position(|p| *p == page).unwrap_or(0);
    // (The pages one after another on one scroll, the list at their left:
    // `blueprint_pages` keeps the page at the top, and a page put there
    // from elsewhere, Inventory… or a test, is brought to the top.)
    let id = window_state_id(app, key, &path, "pages");
    let mut listed = crate::form_pages::list(ui, id, mg_uikit::pages::PAGES, &pages, &mut top);
    let all = AllPages { id, pages: &pages, top: &mut top, bring: listed.chosen, panel: false };
    let shown = FormShown {
        key,
        path,
        also: Arc::new(also),
        root,
        restype,
        page,
        held,
        is_viewed,
        heading: None,
        all: Some(all),
    };
    let pending = form_ui(app, &mut listed.body, shown);
    // (Unless a page asked for another while it was drawn: brought to the
    // top in the next frame.)
    match app.blueprint_pages.get(&page_key).copied() == asked {
        true => _ = app.blueprint_pages.insert(page_key, pages[top]),
        false => ui.ctx().request_repaint(),
    }
    window_drawn(app, pending, &tab, &targets, pressed);
}

/// What a Properties window's header calls what it edits (its name, and
/// quieter its resref): a blueprint by its name and resref, a placed
/// object by its name and its blueprint's (else its tag), several by the
/// first and how many more.
fn title_of(
    app: &Moonglow,
    key: &ResKey,
    path: &GffPath,
    root: &Struct,
    more: usize,
) -> (String, String) {
    let text = |s: &[u8]| String::from_utf8_lossy(s).into_owned();
    let tag = root.string("Tag").map(text).unwrap_or_default();
    let mut sub = match path.0.is_empty() {
        true => key.resref.to_string(),
        false => {
            let template = ["TemplateResRef", "InventoryRes", "EquippedRes"]
                .iter()
                .find_map(|l| root.resref(l).filter(|r| !r.is_empty()));
            template.map_or(tag.clone(), |r| r.to_lowercase().to_string())
        }
    };
    let mut name = crate::tree::blueprint_name(app.game.as_deref(), root).trim().to_string();
    if name.is_empty() {
        name = if tag.is_empty() { std::mem::take(&mut sub) } else { tag };
    }
    if more > 0 {
        sub = match sub.is_empty() {
            true => format!("and {more} more"),
            false => format!("{sub} and {more} more"),
        };
    }
    (name, sub)
}

/// A blueprint type's kind, as a header names it.
fn kind_name(t: ResType) -> &'static str {
    match t {
        ResType::UTC => "Creature",
        ResType::UTI => "Item",
        ResType::UTP => "Placeable",
        ResType::UTD => "Door",
        ResType::UTT => "Trigger",
        ResType::UTE => "Encounter",
        ResType::UTS => "Sound",
        ResType::UTM => "Store",
        ResType::UTW => "Waypoint",
        _ => "Object",
    }
}

/// A Properties window's pages on one scroll ([`FormShown::all`]): the
/// scroll's id (the list beside it reads it by the same), the pages, the
/// page at the top (asked for, then as shown), and whether the list chose
/// it this frame; `panel`: the Properties panel's (M5's W2, ruling 12:
/// the wide layout's), each page under a section heading beside the
/// panel's rail, laid out narrow as the panel's one page is, not a
/// window's (no Preview of its own, no sideways scroll, one column).
pub(crate) struct AllPages<'p> {
    pub id: egui::Id,
    pub pages: &'p [&'static str],
    pub top: &'p mut usize,
    pub bring: bool,
    pub panel: bool,
}

/// The tab of a Properties window of the object at `path` of `key`, with
/// those edited with it (`also`): a blueprint's, several blueprints', a
/// placed object's or several placed objects'.
pub(crate) fn tab_of(key: ResKey, path: &GffPath, also: &[(ResKey, GffPath)]) -> Tab {
    match (path.0.is_empty(), also.is_empty()) {
        (true, true) => Tab::Blueprint(key),
        (true, false) => {
            Tab::Blueprints(std::iter::once(key).chain(also.iter().map(|(k, _)| *k)).collect())
        }
        (false, true) => Tab::Instance { area: key.resref, path: path.clone() },
        // (Each in its own area: objects of several areas are edited
        // together, Advanced Search's.)
        (false, false) => Tab::Instances {
            objects: std::iter::once((key.resref, path.clone()))
                .chain(also.iter().map(|(k, p)| (k.resref, p.clone())))
                .collect(),
        },
    }
}

/// The name of a blueprint's or placed object's Properties window (or
/// tab), as a group around its pages and fields.
pub const WINDOW_GROUP: &str = "Properties window";

/// The placed encounter's Outline color row: its label (the color
/// button's name, and its undo step's).
pub const OUTLINE_COLOR_LABEL: &str = "Outline color";

/// The Outline color button's tip.
pub const OUTLINE_COLOR_TIP: &str =
    "Moonglow's own color for this outline, which Aurora drops when it saves";

/// The Outline color row's button that takes the color off.
pub const OUTLINE_COLOR_DEFAULT: &str = "Default";

/// An encounter's outline in the area view, without a color of its own
/// (red, green, blue).
pub const ENCOUNTER_OUTLINE: [u8; 3] = [230, 120, 40];

/// The Outline color picker's swatches: colors that stand out from the
/// ground and the grid in both themes, and from the encounters' orange
/// and the triggers' green, blue and red. (No black or dark color: lost
/// on a night area's ground.)
pub const OUTLINE_SWATCHES: [(&str, [u8; 3]); 7] = [
    ("Yellow", [250, 220, 50]),
    ("Lime", [170, 240, 50]),
    ("Cyan", [40, 220, 230]),
    ("Violet", [160, 110, 255]),
    ("Magenta", [240, 70, 200]),
    ("Rose", [255, 140, 160]),
    ("White", [245, 245, 245]),
];

/// The object at `path` of the document `key` as it is now, or `None`
/// with what is wrong said in `ui` (no module, the object gone, the
/// document unread).
fn object_now(app: &mut Moonglow, ui: &mut Ui, key: &ResKey, path: &GffPath) -> Option<Struct> {
    let Some(ws) = &mut app.ws else {
        ui.label("No module is open.");
        return None;
    };
    match ws.doc(key) {
        Ok(g) => match path.get(&g.root) {
            Some(s) => Some(s.clone()),
            None => {
                ui.label("This object is no longer there.");
                None
            }
        },
        Err(e) => {
            ui.colored_label(ui.visuals().error_fg_color, e.to_string());
            None
        }
    }
}

/// Whether the document `key` is only looked at: held by another builder
/// (a placed object of an area another builder holds, in a project that
/// keeps those read-only: why, as `claims::read_only` says it), or the
/// game's (`Workspace::is_viewed`).
pub(crate) fn looked_at(app: &Moonglow, key: &ResKey) -> (Option<String>, bool) {
    let held = crate::claims::area_of(key).and_then(|a| crate::claims::read_only(app, a));
    let is_viewed = app.ws.as_ref().is_some_and(|ws| ws.is_viewed(key));
    (held, is_viewed)
}

/// What a Properties form shows ([`form_ui`]): the objects (the first
/// `key` and `path`, the others `also`), the first as it is now, its type,
/// the page shown, why it is only looked at, and a heading over the page
/// where its host draws one (the Properties panel's page name and the
/// object's place: `(page, readout)`).
pub(crate) struct FormShown<'p> {
    pub key: ResKey,
    pub path: GffPath,
    pub also: crate::widgets::Also,
    pub root: Struct,
    pub restype: ResType,
    pub page: &'p str,
    pub held: Option<String>,
    pub is_viewed: bool,
    pub heading: Option<(&'p str, Option<String>)>,
    /// Every page on one scroll (the window's), not `page` alone (the
    /// Properties panel's).
    pub all: Option<AllPages<'p>>,
}

/// A Properties form's page, as a window (with OK and Cancel, over it)
/// and the Properties panel (no OK or Cancel, its pages a rail) draw it:
/// the note of what is only looked at, the note of several, the page in
/// its scroll; a change only looked at refused and said, and of several
/// a change only the first would take dropped. Returns where the page's
/// actions began in `app.actions` (what a window notes for its Cancel).
pub(crate) fn form_ui(app: &mut Moonglow, ui: &mut Ui, shown: FormShown<'_>) -> usize {
    let FormShown { key, path, also, root, restype, page, held, is_viewed, heading, all } = shown;
    let several = !also.is_empty();
    let viewed = is_viewed || held.is_some();
    let refused_id = egui::Id::new(("view-refused", key));
    if viewed {
        ui.horizontal_wrapped(|ui| {
            // Said strongly for a few seconds after a change was tried.
            let now = ui.input(|i| i.time);
            let refused = ui
                .data(|d| d.get_temp::<f64>(refused_id))
                .is_some_and(|at| now - at < VIEW_REFUSED_SHOWN);
            if refused {
                ui.ctx().request_repaint_after(std::time::Duration::from_millis(250));
            }
            let text = match (&held, refused) {
                (Some(why), true) => {
                    format!("Not changed: {why}, and the project keeps others' areas read-only.")
                }
                (Some(why), false) => format!("{why}: read-only here, shown to look at."),
                (None, true) => "Not changed: this is the game's blueprint, shown to look at. \
                                 Edit Copy makes one of the module's own."
                    .into(),
                (None, false) => {
                    "This is the game's blueprint, shown to look at: it can't be changed here."
                        .into()
                }
            };
            let text = egui::RichText::new(text);
            let text = if refused { text.strong() } else { text };
            ui.colored_label(ui.visuals().warn_fg_color, text);
            if held.is_none()
                && ui
                    .button("Edit Copy…")
                    .on_hover_text("A copy in the module, to change")
                    .clicked()
            {
                app.actions.push(Action::CopyDialog(key));
            }
        });
        ui.separator();
    }
    if !also.is_empty() {
        let what = if path.0.is_empty() { "blueprints" } else { "objects" };
        let cue = mixed_cue(also.len());
        ui.weak(format!(
            "{} {what}: shown as the first; what you change is set on each ({cue})",
            also.len() + 1
        ));
    }
    // (Its own, by its first object: by the document, every Properties
    // window of that area's objects, a single one's too, showed it. And
    // while the window is shown: one not drawn in the pass before is of a
    // window closed since.)
    let note_id = window_state_id(app, key, &path, "several-note");
    let pass = ui.ctx().cumulative_pass_nr();
    let note = ui.data(|d| d.get_temp::<(String, u64)>(note_id));
    match note.filter(|(_, drawn)| several && drawn + 1 >= pass) {
        Some((note, _)) => {
            ui.colored_label(ui.visuals().warn_fg_color, &note);
            ui.data_mut(|d| d.insert_temp(note_id, (note, pass)));
        }
        None => ui.data_mut(|d| d.remove::<(String, u64)>(note_id)),
    }
    let pending = app.actions.len();
    if viewed {
        typing_kept_out(ui, ui.available_rect_before_wrap());
    }
    // (A window's own pages, not the panel's on one scroll.)
    let window = all.as_ref().is_some_and(|a| !a.panel);
    let own_tab = window;
    let mut form = Form { app, key, path, also, root, together: Default::default(), own_tab };
    // (Sideways too: the window fits its pane, `TabViewer::scroll_bars`;
    // in a narrow room, the panel's, down alone, its rows fitting it.)
    // (The Properties panel's always: it has no sideways scroll, at any
    // width the dock or its column is given.)
    // (The window's pages judged on a page's own width, inside its
    // margins: `form_pages::narrow`.)
    let narrow = match window {
        true => crate::form_pages::narrow(ui.available_width()),
        false => ui.available_width() < NARROW_FORM || form.app.panel_form,
    };
    ui.data_mut(|d| d.insert_temp(egui::Id::new(NARROW_ID), narrow));
    let scroll = match narrow {
        true => egui::ScrollArea::vertical(),
        false => egui::ScrollArea::both(),
    };
    match all {
        // (Each page its own scroll: another page opens at its top.)
        None => {
            scroll.id_salt(("form-page", page)).auto_shrink([false, false]).show(ui, |ui| {
                if let Some((title, readout)) = &heading {
                    mg_uikit::section_heading(ui, *title);
                    if let Some(readout) = readout {
                        ui.label(egui::RichText::new(readout).small().weak());
                    }
                    ui.data_mut(|d| {
                        d.insert_temp(egui::Id::new(PANEL_PAGE_ID), (*title).to_string());
                    });
                }
                read_only_if(ui, viewed, |ui| page_ui(&mut form, ui, restype, page));
                ui.data_mut(|d| d.remove::<String>(egui::Id::new(PANEL_PAGE_ID)));
                ui.add_space(12.0);
            });
        }
        // (The window's: every page on one scroll, drawn where in sight.)
        Some(AllPages { id, pages, top, bring, panel }) => {
            let columns = match panel {
                true => 1,
                false => crate::form_pages::page_columns(ui.available_width()),
            };
            let scroll =
                crate::form_pages::scroll(id, pages, !narrow, bring, columns).headings(panel);
            // (The panel's one object's place under its first page's
            // heading, as WideArea draws it under Basic.)
            let readout = heading.and_then(|(_, r)| r).filter(|_| panel);
            if panel {
                ui.data_mut(|d| d.insert_temp(egui::Id::new(PANEL_SCROLL_ID), true));
            }
            scroll.show(ui, top, |ui, i| {
                if i == 0
                    && let Some(readout) = &readout
                {
                    ui.label(egui::RichText::new(readout).small().weak());
                }
                if panel {
                    ui.data_mut(|d| {
                        d.insert_temp(egui::Id::new(PANEL_PAGE_ID), pages[i].to_string());
                    });
                }
                read_only_if(ui, viewed, |ui| page_ui(&mut form, ui, restype, pages[i]));
                if panel {
                    ui.data_mut(|d| d.remove::<String>(egui::Id::new(PANEL_PAGE_ID)));
                }
            });
            ui.data_mut(|d| d.remove::<bool>(egui::Id::new(PANEL_SCROLL_ID)));
        }
    }
    // (The form drawn: what is drawn after it judges its own room.)
    ui.data_mut(|d| d.remove::<bool>(egui::Id::new(NARROW_ID)));
    // The game's blueprint (or a read-only area's object) takes no
    // change, the note above saying so when one is tried.
    if viewed && look_only(form.app, pending, &|k| *k == key) {
        let now = ui.input(|i| i.time);
        ui.data_mut(|d| d.insert_temp(refused_id, now));
        ui.ctx().request_repaint();
    }
    // A change only the first would take (a page's own list edits, an item
    // the first object holds) is dropped rather than made to one of several
    // blueprints or placed objects.
    // A list page's command built for each (`Form::apply_each`) may reach
    // some of them only, as it means to: it is let through.
    if several {
        let one = if form.is_instance() { "object" } else { "blueprint" };
        // (Made only for a change to look at: 5,000 objects selected made
        // it of 5,000 paths each frame.)
        let changes = form.app.actions[pending..]
            .iter()
            .any(|a| matches!(a, Action::Apply(_) | Action::ApplyTyped(_)));
        let targets: Vec<(ResKey, GffPath)> = match changes {
            true => form.targets().map(|(k, p)| (k, p.clone())).collect(),
            false => Vec::new(),
        };
        let each = std::mem::take(&mut form.together.each);
        let app = &mut *form.app;
        let mut dropped = false;
        let mut kept = Vec::new();
        for (n, action) in app.actions.drain(pending..).enumerate() {
            let only_some = match &action {
                Action::Apply(cmd) | Action::ApplyTyped(cmd) => {
                    !each.contains(&(pending + n)) && partial(&targets, cmd)
                }
                _ => false,
            };
            if only_some {
                dropped = true;
            } else {
                kept.push(action);
            }
        }
        // (A change made, with nothing to say of it: the note of the one
        // before is taken down.)
        let changed = kept.iter().any(|a| matches!(a, Action::Apply(_) | Action::ApplyTyped(_)));
        app.actions.extend(kept);
        if dropped {
            let note = format!("That change can only be made to one {one} at a time.");
            ui.data_mut(|d| d.insert_temp(note_id, (note, pass)));
        } else if let Some(said) = form.together.said.take() {
            ui.data_mut(|d| d.insert_temp(note_id, (said, pass)));
        } else if changed {
            ui.data_mut(|d| d.remove::<(String, u64)>(note_id));
        }
    }
    pending
}

/// A page of a blueprint's or placed object's form, by its name.
fn page_ui(form: &mut Form<'_>, ui: &mut Ui, restype: ResType, page: &str) {
    match restype {
        // (Every kind's, drawn alike.)
        _ if page == "Variables" => form.variables_page(ui),
        ResType::UTW => waypoint::page(form, ui, page),
        ResType::UTS => sound::page(form, ui, page),
        ResType::UTT => trigger::page(form, ui, page),
        ResType::UTE => encounter::page(form, ui, page),
        ResType::UTM => store::page(form, ui, page),
        ResType::UTD => door::page(form, ui, page),
        ResType::UTP => placeable::page(form, ui, page),
        ResType::UTI => item::page(form, ui, page),
        ResType::UTC => creature::page(form, ui, page),
        _ => {}
    }
}

/// A form only looked at (the game's blueprint, opened by View; what an
/// area another builder holds has, in a project that keeps those
/// read-only), drawn with [`read_only_if`]: the changes to
/// what `changes` says that were asked for since `pending` are dropped.
/// Its fields still take the keyboard, for their text to be chosen and
/// copied, but nothing typed reaches them ([`typing_kept_out`], before the
/// form is drawn). Its lists still scroll, its pages open and its lists'
/// searches take what is typed. Whether a change was tried (for its note
/// to say so).
pub(crate) fn look_only(
    app: &mut Moonglow,
    pending: usize,
    changes: &dyn Fn(&ResKey) -> bool,
) -> bool {
    let mut tried = false;
    let mut i = pending;
    while i < app.actions.len() {
        // (And what a window's own dialog makes there: Setup Area
        // Transition's waypoints.)
        let changing = matches!(&app.actions[i],
            Action::Apply(cmd) | Action::ApplyTyped(cmd) | Action::ApplyMade(cmd, _)
            if cmd.edits.iter().any(|e| changes(e.key())));
        if changing {
            app.actions.remove(i);
            tried = true;
        } else {
            i += 1;
        }
    }
    tried
}

/// How long the note of a blueprint only looked at says a change was
/// refused, in seconds.
pub(crate) const VIEW_REFUSED_SHOWN: f64 = 4.0;

/// The most objects edited together beside the first whose values are
/// compared with its, field by field, to mark the fields that differ (one
/// more selected in all).
pub(crate) const MIXED_LIMIT: usize = 64;

/// What the note over objects edited together says of the marks, with
/// `others` edited beside the first: where they differ, or that past
/// [`MIXED_LIMIT`] of them the marks are left out.
pub(crate) fn mixed_cue(others: usize) -> String {
    if others > MIXED_LIMIT {
        format!("more than {} selected: mixed markers omitted", MIXED_LIMIT + 1)
    } else {
        format!("{MIXED}: they differ there")
    }
}

/// The mark of a field whose value differs among the objects edited
/// together.
pub(crate) const MIXED: &str = "≠";

/// The size of the [`MIXED`] mark: a badge of the body text's height.
fn mixed_mark_size(ui: &Ui) -> egui::Vec2 {
    let h = ui.text_style_height(&egui::TextStyle::Body).max(14.0);
    egui::vec2(h, h)
}

/// The [`MIXED`] mark at `rect`: "≠" bold on a tint of the warning color,
/// readable in both themes; known to the screen reader as "≠" whose value
/// is "mixed", and to the pointer by its tip (M4's U29 review).
fn paint_mixed_mark(
    ui: &Ui,
    rect: egui::Rect,
    id: egui::Id,
    tip: impl Into<egui::WidgetText>,
) -> egui::Response {
    let color = ui.visuals().warn_fg_color;
    let p = ui.painter();
    p.rect_filled(rect, 3.0, color.gamma_multiply(0.18));
    p.rect_stroke(rect, 3.0, egui::Stroke::new(1.0, color), egui::StrokeKind::Inside);
    let font = egui::FontId::proportional(rect.height() - 1.0);
    p.text(rect.center() + egui::vec2(0.0, -0.5), egui::Align2::CENTER_CENTER, MIXED, font, color);
    let r = ui.interact(rect, id, egui::Sense::hover());
    r.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Label, true, MIXED));
    ui.ctx().accesskit_node_builder(r.id, |node| {
        node.set_label(MIXED);
        node.set_value("mixed");
    });
    r.on_hover_text(tip)
}

/// The [`MIXED`] mark laid out where the next widget goes (before a group
/// of choices: the whole group's).
fn mixed_mark_inline(ui: &mut Ui, id: egui::Id) {
    let size = mixed_mark_size(ui);
    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
    paint_mixed_mark(ui, rect, id, MIXED_TIP);
}

/// The [`MIXED`] mark laid out where the next widget goes, with its tip
/// `tip` (a list's row whose objects differ there).
pub(crate) fn mixed_mark(ui: &mut Ui, tip: impl Into<egui::WidgetText>) -> egui::Response {
    let size = mixed_mark_size(ui);
    let (rect, r) = ui.allocate_exact_size(size, egui::Sense::hover());
    paint_mixed_mark(ui, rect, r.id.with("mixed"), tip)
}

/// The tip of [`MIXED`].
const MIXED_TIP: &str = "The objects differ here: this shows the first one's. What you set goes to \
                         each.";

/// The look of a form that can't be changed (the game's blueprint, opened
/// by View): its fields are flat and dim, and don't answer the pointer.
/// It still scrolls, and its lists and pages can be looked through.
/// Before a form only looked at is drawn in `within`: while a field of it
/// has the keyboard (not a list's search), what would change its text
/// doesn't reach it (typing, pasting, cutting, Backspace and Delete,
/// Enter, its own undo and redo), and what chooses and copies its text
/// does. A field of the game's blueprint once didn't keep the keyboard at
/// all, so nothing of it could be copied (Troubletcat's report after
/// 3.1.0: a base item's ResRef, copied for a script).
pub(crate) fn typing_kept_out(ui: &Ui, within: egui::Rect) {
    let ctx = ui.ctx();
    let focused = ctx.memory(|m| m.focused()).and_then(|id| Some((id, ctx.read_response(id)?)));
    let Some((_, r)) = focused else { return };
    // (A search field is told by the last frame: this one's aren't drawn
    // yet.)
    if r.layer_id != ui.layer_id()
        || !within.intersects(r.rect)
        || crate::widgets::search_has_keyboard(ctx)
    {
        return;
    }
    // (Its right-click menu has no Cut or Paste.)
    crate::widgets::text_kept_out(ctx, r.id);
    ctx.input_mut(|i| {
        i.events.retain(|e| match e {
            egui::Event::Text(_)
            | egui::Event::Paste(_)
            | egui::Event::Cut
            | egui::Event::Ime(_) => false,
            egui::Event::Key { key, modifiers, .. } => {
                let changes =
                    matches!(key, egui::Key::Backspace | egui::Key::Delete | egui::Key::Enter);
                let undo = modifiers.command && matches!(key, egui::Key::Z | egui::Key::Y);
                !changes && !undo
            }
            _ => true,
        });
    });
}

/// `add`, drawn as read only where `viewed` (the mock's `.field.ro`,
/// [`mg_uikit::read_only`]): no fill (the panel's), a quiet border, words
/// in `text-2`, nothing lit under the pointer, check boxes, radio buttons
/// and sliders quiet too.
pub(crate) fn read_only_if<R>(ui: &mut Ui, viewed: bool, add: impl FnOnce(&mut Ui) -> R) -> R {
    if viewed { mg_uikit::read_only(ui, add) } else { add(ui) }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The note over objects edited together says the marks are left out
    /// from the number of them where they are (the others past
    /// [`MIXED_LIMIT`]: one more selected), not at a number of its own.
    #[test]
    fn the_mixed_cue_names_the_number_where_the_marks_go() {
        let omitted = format!("more than {} selected: mixed markers omitted", MIXED_LIMIT + 1);
        assert_eq!(mixed_cue(MIXED_LIMIT), "≠: they differ there", "{} selected", MIXED_LIMIT + 1);
        assert_eq!(mixed_cue(MIXED_LIMIT + 1), omitted, "{} selected", MIXED_LIMIT + 2);
    }

    /// A long table's rows by name and narrowed by what is typed, kept
    /// (the review's B22: copied and sorted every frame), are what sorting
    /// and searching them each time gives; kept while the list is the
    /// same one, and made again for another.
    #[test]
    fn choices_kept_by_name_and_found_are_those_made_each_time() {
        let choices = |names: &[&str]| -> Arc<Vec<Choice>> {
            let rows = names.iter().enumerate();
            Arc::new(rows.map(|(row, n)| Choice { row, text: n.to_string() }).collect())
        };
        let table = choices(&["Zombie", "ant", "Árbol", "Bat", "ant"]);
        let ctx = egui::Context::default();
        let sorted = by_name_kept(&ctx, "test", &table);
        assert_eq!(*sorted, mg_rules::by_name(table.to_vec()));
        assert!(Arc::ptr_eq(&sorted, &by_name_kept(&ctx, "test", &table)), "kept, not made again");
        let needle = mg_core::text::Needle::new("a");
        let each_time: Vec<usize> =
            (0..sorted.len()).filter(|&i| needle.finds(&sorted[i].text)).collect();
        let found = found_kept(&ctx, "test", &sorted, "a");
        assert_eq!(*found, each_time);
        assert!(Arc::ptr_eq(&found, &found_kept(&ctx, "test", &sorted, "a")));
        assert_eq!(found_kept(&ctx, "test", &sorted, "zom").len(), 1, "another search");
        assert_eq!(found_kept(&ctx, "test", &sorted, "").len(), 5);
        // Another list (the tables read again): made for it.
        let other = choices(&["Wolf", "Asp"]);
        let sorted = by_name_kept(&ctx, "test", &other);
        assert_eq!(sorted.iter().map(|c| c.text.as_str()).collect::<Vec<_>>(), ["Asp", "Wolf"]);
        // What is kept for the load order is made again when it changes.
        egui::__run_test_ui(|ui| {
            let id = egui::Id::new("test-kept");
            assert_eq!(kept_for_load_order(ui, 1, id, || 10), 10);
            assert_eq!(kept_for_load_order(ui, 1, id, || 20), 10);
            assert_eq!(kept_for_load_order(ui, 2, id, || 20), 20);
        });
    }

    #[test]
    fn a_change_to_some_of_the_objects_edited_together_is_partial() {
        let git = ResKey::parse("area001", ResType::GIT).unwrap();
        let chests: Vec<(ResKey, GffPath)> =
            (0..2).map(|i| (git, GffPath::root().item("Placeable List", i))).collect();
        let set = |path: &GffPath| Edit::SetField {
            key: git,
            path: path.clone(),
            label: "Plot".into(),
            value: Some(Value::Byte(1)),
        };
        // Set on each: all of them.
        let each = Command::new("Plot", chests.iter().map(|(_, p)| set(p)).collect());
        assert!(!partial(&chests, &each));
        // An item added to the first one's inventory: the same GIT, one of
        // them.
        let held = Edit::InsertItem {
            key: git,
            path: chests[0].1.clone(),
            list: "ItemList".into(),
            index: 0,
            item: Struct::new(0),
        };
        assert!(partial(&chests, &Command::new("Add item", vec![held])));
        // An item the first holds, further in.
        let inner = set(&chests[0].1.item("ItemList", 0));
        assert!(partial(&chests, &Command::new("Stack size", vec![inner])));
        // Another object of the area: none of them.
        let other = set(&GffPath::root().item("Placeable List", 2));
        assert!(!partial(&chests, &Command::new("Plot", vec![other])));
        // Blueprints, each its own document.
        let boxes: Vec<(ResKey, GffPath)> = ["mg_box_a", "mg_box_b"]
            .iter()
            .map(|n| (ResKey::parse(n, ResType::UTP).unwrap(), GffPath::root()))
            .collect();
        let whole = Edit::SetResource { key: boxes[1].0, data: None };
        assert!(partial(&boxes, &Command::new("Replace", vec![whole])));
        let both: Vec<Edit> = boxes
            .iter()
            .map(|(k, p)| Edit::SetField {
                key: *k,
                path: p.clone(),
                label: "Plot".into(),
                value: None,
            })
            .collect();
        assert!(!partial(&boxes, &Command::new("Plot", both)));
    }

    #[test]
    fn a_window_s_change_is_noted_by_its_object_s_field() {
        let chest = GffPath::root().item("Placeable List", 2);
        let noted = |form: &GffPath, path: &GffPath, label: &str| {
            field_changed(form, path, label).map(|(at, l)| (at, l.to_string()))
        };
        // The object's own field, and one further in: the list it is in.
        assert_eq!(noted(&chest, &chest, "Tag"), Some((chest.clone(), "Tag".into())));
        let held = chest.item("ItemList", 0);
        assert_eq!(noted(&chest, &held, "StackSize"), Some((chest.clone(), "ItemList".into())));
        let root = GffPath::root();
        assert_eq!(noted(&root, &chest, "Tag"), Some((root.clone(), "Placeable List".into())));
        // In a struct of the object's, the field itself: an area's music
        // is not set back with its sounds.
        let audio = root.field("AreaProperties");
        assert_eq!(noted(&root, &audio, "MusicDay"), Some((audio.clone(), "MusicDay".into())));
        // Another object's is not this one's.
        let other = GffPath::root().item("Placeable List", 3);
        assert_eq!(field_changed(&chest, &other, "Tag"), None);
    }

    #[test]
    fn kept_for_key_hits_invalidates_and_adds_no_arc_ref() {
        let ctx = egui::Context::default();
        let id = egui::Id::new("test-kept-for-key");
        let sentinel = Arc::new(42u32);
        let key1 = (1u64, "Creature", Some((10u64, 1u64)));
        let v1 = kept_for_key(&ctx, id, &key1, || {
            let _borrowed = &sentinel;
            vec![1, 2, 3]
        });
        assert_eq!(v1, vec![1, 2, 3]);
        assert_eq!(Arc::strong_count(&sentinel), 1);
        let v2 = kept_for_key(&ctx, id, &key1, || vec![9]);
        assert_eq!(v2, vec![1, 2, 3]);
        let key2 = (2u64, "Creature", Some((10u64, 1u64)));
        let v3 = kept_for_key(&ctx, id, &key2, || vec![4, 5]);
        assert_eq!(v3, vec![4, 5]);
        let key3 = (2u64, "Creature", Some((10u64, 2u64)));
        let v4 = kept_for_key(&ctx, id, &key3, || vec![6]);
        assert_eq!(v4, vec![6]);
    }

    #[test]
    fn held_text_captures_script_conversation_resref_and_visuals_fields() {
        let key = ResKey::parse("plc_test", ResType::UTP).unwrap();
        let mut root = Struct::new(u32::MAX);
        root.set("OnDeath", Value::resref(ResRef::from_str("old_sc").unwrap()));
        root.set("Conversation", Value::resref(ResRef::from_str("old_dlg").unwrap()));
        let mut mat_entry = Struct::new(0);
        mat_entry.set("Material", Value::String(b"mat_a".to_vec()));
        mat_entry.set("Param", Value::String(b"u_col".to_vec()));
        mat_entry.set("Type", Value::Byte(1));
        mat_entry.set("Int", Value::Int(0));
        let mut mat = Struct::new(7);
        mat.set("ShaderParams", Value::List(vec![mat_entry]));
        root.set("Material", Value::Struct(mat));
        let mut misc = Struct::new(8);
        misc.set("TextBubbleType", Value::Int(1));
        misc.set("TextBubbleText", Value::String(b"old".to_vec()));
        root.set("MiscVisuals", Value::Struct(misc));
        let mut rep_entry = Struct::new(0);
        rep_entry.set("OldTexture", Value::resref(ResRef::from_str("old_tex").unwrap()));
        rep_entry.set("NewTexture", Value::resref(ResRef::from_str("rep_a").unwrap()));
        let mut tex_rep = Struct::new(9);
        tex_rep.set("TextureReplaceLi", Value::List(vec![rep_entry]));
        root.set("TextureReplace", Value::Struct(tex_rep));

        let ctx = egui::Context::default();
        let mut app = Moonglow::new(None, Box::new(crate::NoDialogs::default()));
        let run_case = |app: &mut Moonglow,
                        root: &Struct,
                        field_key: &str,
                        typed: &str,
                        draw: &dyn Fn(&mut Form<'_>, &mut Ui)| {
            let id = egui::Id::new(("blueprint", key, GffPath::root(), field_key, "window"));
            // Warm up layout so egui::Grid finishes its first-frame invisible sizing pass
            // before focus is requested (matching real interaction).
            let mut warmup = ctx.run_ui(Default::default(), |ui| {
                let mut form = Form {
                    app,
                    key,
                    path: GffPath::root(),
                    also: Default::default(),
                    root: root.clone(),
                    together: Together::default(),
                    own_tab: true,
                };
                draw(&mut form, ui);
            });
            warmup.textures_delta.clear();
            app.held_text = None;
            app.buffers.insert(id, typed.to_string());
            ctx.memory_mut(|m| m.request_focus(id));
            let mut out = ctx.run_ui(Default::default(), |ui| {
                let mut form = Form {
                    app,
                    key,
                    path: GffPath::root(),
                    also: Default::default(),
                    root: root.clone(),
                    together: Together::default(),
                    own_tab: true,
                };
                draw(&mut form, ui);
            });
            out.textures_delta.clear();
            app.held_text.take()
        };

        let held_sc = run_case(&mut app, &root, "OnDeath", "new_sc", &|f, ui| {
            f.script(ui, "OnDeath", "OnDeath");
        })
        .expect("script should populate held_text");
        assert!(held_sc.command.is_some());

        let held_dlg = run_case(&mut app, &root, "Conversation", "new_dlg", &|f, ui| {
            f.conversation(ui);
        })
        .expect("conversation should populate held_text");
        assert!(held_dlg.command.is_some());

        let held_rr = run_case(&mut app, &root, "TemplateResRef", "plc_renamed", &|f, ui| {
            f.blueprint_resref(ui);
        })
        .expect("TemplateResRef should populate held_text");
        assert_eq!(
            held_rr.action,
            Some(Action::RenameBlueprint {
                from: key,
                to: ResRef::from_str("plc_renamed").unwrap()
            })
        );

        let held_sp = run_case(&mut app, &root, "ShaderParams#0#Material", "mat_b", &|f, ui| {
            visuals::page(f, ui, true);
        })
        .expect("ShaderParams should populate held_text");
        assert!(held_sp.command.is_some());

        let held_tb = run_case(&mut app, &root, "TextBubbleText", "updated", &|f, ui| {
            visuals::page(f, ui, true);
        })
        .expect("TextBubbleText should populate held_text");
        assert!(held_tb.command.is_some());

        let held_tr =
            run_case(&mut app, &root, "TextureReplace#old_tex#Some(0)#new", "rep_b", &|f, ui| {
                replacements::textures(f, ui);
            })
            .expect("TextureReplace should populate held_text");
        assert!(held_tr.command.is_some());

        // Pressed::Cancel clears any pending held_text owned by the canceled window.
        let tab = Tab::Blueprint(key);
        app.held_text = Some(HeldText {
            id: egui::Id::new("c"),
            drawn: ctx.cumulative_pass_nr(),
            command: held_sc.command,
            action: None,
            typed: false,
            window: Some(HeldIn {
                tab: tab.clone(),
                targets: vec![(key, GffPath::root())],
                kept: vec![],
            }),
            panel: false,
        });
        window_drawn_but(&mut app, 0, &tab, &[(key, GffPath::root())], Some(Pressed::Cancel), &[]);
        assert!(app.held_text.is_none());
    }
}
