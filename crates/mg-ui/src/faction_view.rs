//! The Faction Editor (`repute.fac`, M4's U22, the Factions board): its
//! header (the switch Basic | Advanced, Add Faction…, Remove Faction,
//! Change Name…; OK and Cancel at its end) over, in Advanced, the matrix
//! of how each faction regards each other (its row and column headers
//! frozen, a chosen cell, only the cells in sight laid out), in Basic the
//! chart of how the chosen faction regards the others. The chosen
//! faction's **Faction**, the chosen cell's **Reaction** and the chosen
//! faction's **Reactions** are the Right Dock's ([`faction_panel`],
//! [`reaction_panel`], [`reactions_panel`]); its factions are the
//! Outliner's (`outliner::factions`); the Status Bar's hints and counts
//! are [`shown`]'s. Each change is one undoable command that rewrites the
//! file as Aurora does; the file is read once a revision of the workspace
//! ([`FactionView::walks`] counts each read).

use std::sync::Arc;

use egui::{Color32, Ui};
use mg_core::{ResRef, ResType};
use mg_edit::Command;
use mg_module::factions::{
    DEFAULT_REPUTATION, Factions, STANDARD, faction_users, renumber_faction_users,
};
use mg_resman::ResKey;
use mg_uikit::Controls as _;

use crate::{Moonglow, Tab};

/// The editor's view state.
#[derive(Debug, Clone, Default)]
pub struct FactionView {
    pub advanced: bool,
    /// The chosen faction: the chosen cell's row, who regards.
    pub selected: u32,
    /// The chosen cell's column: whom the chosen faction regards.
    pub target: u32,
    /// Also show how the others regard the selected faction.
    pub full_detail: bool,
    pub adding: Option<AddFaction>,
    pub renaming: Option<(u32, String)>,
    /// What Change Name… was opened on: how many factions there were and
    /// the faction's name. The window doesn't block, so its faction can be
    /// removed (or another slide into its number) behind it: it closes then.
    renaming_of: Option<(usize, String)>,
    /// What the window did since it was opened, for its Cancel.
    cancel: CancelSteps,
    /// The factions as read at a revision of the workspace.
    seen: Option<Arc<Seen>>,
    /// How many times the factions file was read whole.
    walks: usize,
    /// The chosen cell's factions' names, to find them again where the
    /// factions changed under them (a removal, an Undo).
    chosen_names: Option<(String, String)>,
    /// A value being typed into a cell: its row, column and the text.
    typing: Option<(u32, u32, String)>,
    /// The typing just began: its field takes the keyboard.
    typing_began: bool,
    /// Tab (or Shift+Tab) was pressed while a value was typed: the value
    /// is set and the keys stay the matrix's
    /// ([`keep_tab_for_the_matrix`]).
    tab_set: bool,
    /// A cell being dragged: its row, column, the value it had and how far
    /// the pointer went (in values).
    dragging: Option<(u32, u32, u32, f32)>,
    /// What the pointer was over at the last frame: a cell of the matrix,
    /// or in Basic's chart a row's name or its slider (the Status Bar's
    /// hints are for what it is over).
    hovered: Hovered,
    /// The chosen cell is to be brought into sight (moved by the keys or
    /// chosen elsewhere).
    reveal: bool,
}

impl FactionView {
    /// How many times the factions file was read whole (once for each
    /// change of the workspace it is drawn after, never for a frame alone).
    pub fn walks(&self) -> usize {
        self.walks
    }
}

/// What the pointer is over in the editor, for the Status Bar's hints.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum Hovered {
    #[default]
    Nothing,
    /// A cell of the matrix (Advanced).
    Cell,
    /// A row's name in Basic's chart, a click on which chooses its
    /// reaction (not a From row's).
    Name,
    /// A slider in Basic's chart.
    Slider,
}

/// The factions as one revision of the workspace has them, or why they
/// can't be edited.
#[derive(Debug)]
struct Seen {
    identity: u64,
    revision: u64,
    factions: Result<Factions, String>,
    /// Their names, for the Outliner (made once a revision, as the file is
    /// read: [`FactionView::walks`] counts them).
    names: Arc<[String]>,
}

/// The faction editor's changes since it was opened, for its Cancel: the
/// factions file is set back whole (it is the window's), and the objects
/// a removal renumbered are numbered back as they are now, so that what
/// was changed in them elsewhere meanwhile (a creature placed in the
/// area) stays.
#[derive(Debug, Clone, Default, PartialEq)]
struct CancelSteps {
    /// The factions file as the window found it, then after each of its
    /// changes, with the faction that change removed, if one.
    steps: Vec<(Option<u32>, Option<Vec<u8>>)>,
    /// The objects a removal renumbered, as they were before the first:
    /// set back so when the factions file is as none of the window's
    /// changes left it (an Undo took one back, or it was changed
    /// elsewhere), and the numbers can't be told.
    renumbered: std::collections::BTreeMap<ResKey, Option<Vec<u8>>>,
}

/// A removal's old-to-new id map, for the objects that refer to factions.
type Renumbering = Box<dyn Fn(u32) -> Option<u32>>;

/// A removal's renumbering taken back: a faction after `removed` moves up
/// one id again.
fn put_back(removed: &[u32]) -> impl Fn(u32) -> Option<u32> + use<> {
    let removed = removed.to_vec();
    move |mut id| {
        for r in removed.iter().rev() {
            if id >= *r {
                id += 1;
            }
        }
        Some(id)
    }
}

/// The edits that number the objects the window's removals renumbered as
/// they were, for its Cancel (`blueprint::cancel_more`).
pub(crate) fn numbered_back(app: &mut Moonglow) -> Vec<mg_edit::Edit> {
    let cancel = &app.faction_view.cancel;
    let Some(ws) = app.ws.as_mut().filter(|_| cancel.steps.iter().any(|(r, _)| r.is_some())) else {
        return Vec::new();
    };
    if ws.flush().is_err() {
        return Vec::new();
    }
    let now = ws.module.get(&key());
    let Some(at) = cancel.steps.iter().rposition(|(_, after)| after.as_deref() == now) else {
        return (cancel.renumbered.iter())
            .filter(|(k, was)| ws.module.get(k) != was.as_deref())
            .map(|(k, was)| mg_edit::Edit::SetResource { key: *k, data: was.clone() })
            .collect();
    };
    let removed: Vec<u32> = cancel.steps[..=at].iter().filter_map(|(r, _)| *r).collect();
    if removed.is_empty() {
        return Vec::new();
    }
    let mut staged = ws.module.clone();
    renumber_faction_users(&mut staged, put_back(&removed));
    ws.edits_to(&staged)
}

/// The Add Faction window.
#[derive(Debug, Clone, PartialEq)]
pub struct AddFaction {
    pub name: String,
    pub global: bool,
    pub parent: u32,
}

fn key() -> ResKey {
    ResKey::new(ResRef::from_str("repute").expect("valid"), ResType::FAC)
}

/// Aurora's colours: hostile up to 10, friendly from 90, neutral between
/// (kept: `docs/deferred.md`'s entry on the factions' red and blue).
fn color(rep: u32) -> Color32 {
    match rep {
        0..=10 => Color32::from_rgb(170, 40, 40),
        90.. => Color32::from_rgb(50, 70, 190),
        _ => Color32::from_gray(120),
    }
}

/// How Aurora's status bar words a reputation.
pub fn attitude(rep: u32) -> &'static str {
    match rep {
        0..=10 => "hostile",
        90.. => "friendly",
        _ => "neutral",
    }
}

/// "Victim (Danger) is hostile toward Hostile".
fn sentence(f: &Factions, p: u32, t: u32, rep: u32) -> String {
    format!("{} is {} toward {}", name(f, p), attitude(rep), name(f, t))
}

fn name(f: &Factions, id: u32) -> &str {
    f.factions.get(id as usize).map_or("", |x| x.name.as_str())
}

fn rep_of(f: &Factions, p: u32, t: u32) -> u32 {
    f.reputation(p, t).unwrap_or(DEFAULT_REPUTATION)
}

impl Moonglow {
    /// Replaces `repute.fac` (and, after a removal, the objects that were
    /// renumbered) through one command, noted for the editor's Cancel.
    fn store_factions(
        &mut self,
        what: &str,
        f: &Factions,
        renumber: Option<(u32, Renumbering)>,
    ) -> bool {
        let mut cancel = std::mem::take(&mut self.faction_view.cancel);
        let ok = self.store_factions_in(what, f, renumber, &mut cancel);
        self.faction_view.cancel = cancel;
        ok
    }

    fn store_factions_in(
        &mut self,
        what: &str,
        f: &Factions,
        renumber: Option<(u32, Renumbering)>,
        cancel: &mut CancelSteps,
    ) -> bool {
        let Some(ws) = &mut self.ws else { return false };
        if let Err(e) = ws.flush() {
            self.log.error(e.to_string());
            return false;
        }
        let mut staged = ws.module.clone();
        if staged.set_gff(key(), &f.to_gff()).is_err() {
            return false;
        }
        let removed = renumber.as_ref().map(|(id, _)| *id);
        let users = match renumber {
            Some((_, map)) => renumber_faction_users(&mut staged, map),
            None => Vec::new(),
        };
        let edits = ws.edits_to(&staged);
        if edits.is_empty() {
            return false;
        }
        let before_fac = ws.module.get(&key()).map(<[u8]>::to_vec);
        let before_users: Vec<(ResKey, Option<Vec<u8>>)> =
            users.iter().map(|u| (*u, ws.module.get(u).map(<[u8]>::to_vec))).collect();
        let target = (key(), mg_edit::GffPath::root());
        let prev_changed = self.properties_changed.get(&target).cloned();
        let command = Command::new(what, edits);
        // (For the window's Cancel: the factions file as it is; the
        // objects renumbered are numbered back, `numbered_back`.)
        let targets = [target.clone()];
        let noted = crate::blueprint::but_for(&command, &users);
        crate::blueprint::note_command(self, &Tab::Factions, &targets, &noted);
        if let Err(e) = self.apply(command) {
            match prev_changed {
                Some(prev) => {
                    self.properties_changed.insert(target, prev);
                }
                None => {
                    self.properties_changed.remove(&target);
                }
            }
            if !std::mem::take(&mut self.claims.held) {
                self.log.error(e.to_string());
            }
            return false;
        }
        if cancel.steps.is_empty() {
            cancel.steps.push((None, before_fac));
        }
        for (u, was) in before_users {
            cancel.renumbered.entry(u).or_insert(was);
        }
        let Some(ws) = &mut self.ws else { return true };
        let _ = ws.flush();
        cancel.steps.push((removed, ws.module.get(&key()).map(<[u8]>::to_vec)));
        true
    }
}

/// The factions as the workspace has them now (the workspace's copy of
/// `repute.fac`, or the standard factions where the module has no such
/// file), or why the file can't be edited: one that can't be read is not
/// the standard five (the first change would write them over it), and the
/// editor counts on the standard five being there. Read once a revision;
/// the chosen cell then found again by its factions' names (a removal, an
/// Undo, numbered them anew), else kept within the factions. `None`
/// without a module.
fn begin(app: &mut Moonglow) -> Option<Arc<Seen>> {
    let ws = app.ws.as_mut()?;
    let (identity, revision) = (ws.identity(), ws.revision());
    let view = &mut app.faction_view;
    if let Some(seen) =
        view.seen.as_ref().filter(|s| s.identity == identity && s.revision == revision)
    {
        return Some(seen.clone());
    }
    view.walks += 1;
    let exists = ws.module.contains(&key());
    let factions = match ws.doc(&key()) {
        Ok(g) => {
            let f = Factions::read_in(g, crate::text::game_codepage());
            match f.factions.len() {
                n if n < STANDARD.len() => Err(format!(
                    "repute.fac has {n} of the {} standard factions, and can't be edited here.",
                    STANDARD.len()
                )),
                _ => Ok(f),
            }
        }
        Err(_) if !exists => Ok(Factions::of_module(&ws.module)),
        Err(e) => Err(format!("repute.fac can't be read: {e}")),
    };
    let names = match &factions {
        Ok(f) => f.factions.iter().map(|x| x.name.clone()).collect(),
        Err(_) => Arc::from([]),
    };
    let seen = Arc::new(Seen { identity, revision, factions, names });
    if let Ok(f) = &seen.factions {
        let n = f.factions.len() as u32;
        let find = |name: &str| f.factions.iter().position(|x| x.name == name).map(|i| i as u32);
        if let Some((p, t)) = view.chosen_names.take() {
            let (was_p, was_t) = (view.selected, view.target);
            if self::name(f, view.selected) != p {
                view.selected = find(&p).unwrap_or(view.selected);
            }
            if self::name(f, view.target) != t {
                view.target = find(&t).unwrap_or(view.target);
            }
            view.reveal |= (was_p, was_t) != (view.selected, view.target);
        }
        view.selected = view.selected.min(n.saturating_sub(1));
        view.target = view.target.min(n.saturating_sub(1));
        // (A cell typed into or dragged that is gone: let go of.)
        if view.typing.as_ref().is_some_and(|(p, t, _)| *p >= n || *t >= n) {
            view.typing = None;
        }
        if view.dragging.is_some_and(|(p, t, ..)| p >= n || t >= n) {
            view.dragging = None;
        }
    }
    view.seen = Some(seen.clone());
    Some(seen)
}

/// The chosen cell's names noted, to find it again once the factions
/// change ([`begin`]).
fn note_chosen(view: &mut FactionView, f: &Factions) {
    view.chosen_names =
        Some((name(f, view.selected).to_string(), name(f, view.target).to_string()));
}

/// The faction editor's header's name (`mg_uikit::header`).
pub const FACTION_HEADER: &str = "Faction header";

/// The matrix's name (the board's grid's).
pub const MATRIX: &str = "How each faction regards each other";

/// The switch's name and its choices.
const MODE: &str = "Mode";

/// What the header asked for.
#[derive(Default)]
struct Asked {
    add: bool,
    remove: bool,
    rename: bool,
    pressed: Option<crate::blueprint::Pressed>,
}

/// The header (the Factions board's `.editor-header`): the switch Basic |
/// Advanced, a line, Add Faction…, Remove Faction and Change Name…
/// (these two for a faction of the module's own); OK and Cancel at its
/// end. What has no room loses its words, then goes into More.
fn header(ui: &mut Ui, advanced: &mut bool, custom: bool) -> Asked {
    use mg_uikit::Icon;
    use mg_uikit::header::{EditorHeader, Item};
    use std::cell::Cell;
    let choices = [
        (false, "Basic", "Basic: how the selected faction regards the others"),
        (true, "Advanced", "Advanced: how every faction regards every other"),
    ];
    let mode = Cell::new(*advanced);
    let switch_width = {
        let font = egui::TextStyle::Body.resolve(ui.style());
        let words = |w: &str| {
            let color = ui.visuals().text_color();
            ui.fonts_mut(|f| f.layout_no_wrap(w.to_string(), font.clone(), color)).size().x
        };
        let pad = 2.0 * mg_uikit::metrics::CONTROL_PADDING_X;
        let widths: f32 = choices.iter().map(|c| words(c.1) + pad).sum();
        widths.ceil() + (choices.len() - 1) as f32
    };
    let switch = |ui: &mut Ui| {
        let mut chosen = mode.get();
        let room = egui::vec2(switch_width, mg_uikit::metrics::CONTROL_HEIGHT);
        let layout = egui::Layout::left_to_right(egui::Align::Center);
        ui.allocate_ui_with_layout(room, layout, |ui| {
            mg_uikit::segmented(ui, MODE, &mut chosen, &choices);
        });
        mode.set(chosen);
    };
    let not_standard = "A standard faction can't be renamed or removed";
    let header = EditorHeader::new(FACTION_HEADER)
        .left(Item::custom(MODE, switch_width, switch).more(switch_width, switch).primary())
        .left(Item::separator())
        .left(
            Item::button(Icon::Plus, "Add Faction…")
                .tip("Add a faction that starts with its parent's reputations"),
        )
        .left(Item::button(Icon::Delete, "Remove Faction").enabled(custom).tip(match custom {
            true => "Remove the selected faction (not a standard one)",
            false => not_standard,
        }))
        .left(Item::button(Icon::Rename, "Change Name…").enabled(custom).tip(match custom {
            true => "Change the selected faction's name (not a standard one)",
            false => not_standard,
        }));
    let out =
        crate::widgets::header_with_ok_cancel(ui, header, None, crate::blueprint::OK_CANCEL_TIPS);
    *advanced = mode.get();
    Asked {
        add: out.clicked("Add Faction…"),
        remove: out.clicked("Remove Faction"),
        rename: out.clicked("Change Name…"),
        pressed: out.end,
    }
}

/// The faction editor: its header over the matrix (Advanced) or the chart
/// (Basic), and its two windows. Its changes are noted as they are made
/// (`store_factions`), the panels' too.
pub(crate) fn ui(app: &mut Moonglow, ui: &mut Ui) {
    let targets = [(key(), mg_edit::GffPath::root())];
    // (Opened afresh, or nothing changed yet: nothing of before is its
    // Cancel's.)
    if !app.properties_changed.contains_key(&targets[0]) {
        app.faction_view.cancel = CancelSteps::default();
    }
    let pending = app.actions.len();
    // (A floating editor's Outliner folded to a strip at its left, where
    // its window has not the room for its pane: `shell::window_panes`.)
    let owner = crate::outliner::Owner::Factions;
    let strip = app.outliner.strips_now.contains(&owner);
    if strip {
        crate::outliner::strip(app, ui, owner);
    }
    let pressed = editor(app, ui);
    if strip {
        crate::outliner::over(app, ui, owner);
    }
    // (What its Cancel adds: `numbered_back`, `blueprint::cancel_more`.)
    crate::blueprint::window_drawn_but(app, pending, &Tab::Factions, &targets, pressed, &[]);
}

fn editor(app: &mut Moonglow, ui: &mut Ui) -> Option<crate::blueprint::Pressed> {
    let Some(seen) = begin(app) else {
        ui.label("No module is open.");
        return None;
    };
    let f = match &seen.factions {
        Err(why) => {
            ui.colored_label(ui.visuals().error_fg_color, why);
            return None;
        }
        Ok(f) => f,
    };
    let view = &mut app.faction_view;
    let custom = view.selected as usize >= STANDARD.len();
    let mut advanced = view.advanced;
    let asked = header(ui, &mut advanced, custom);
    if advanced != view.advanced {
        view.advanced = advanced;
        view.reveal = true;
    }
    if asked.add {
        view.adding = Some(AddFaction { name: String::new(), global: true, parent: 1 });
    }
    if asked.rename && custom {
        view.renaming = Some((view.selected, name(f, view.selected).to_string()));
    }
    let mut changed: Option<(String, Factions)> = None;
    match view.advanced {
        true => matrix(app, ui, f),
        false => changed = chart(app, ui, f),
    }
    let view = &mut app.faction_view;
    // Add Faction.
    let ctx = ui.ctx().clone();
    if let Some(mut add) = view.adding.clone() {
        let mut close = false;
        let footed = crate::widgets::modal(&ctx, "Add Faction").show_foot(&ctx, |ui| {
            crate::widgets::field_label(ui, "Name");
            let field = ui.add(mg_uikit::text_field(&mut add.name));
            crate::widgets::autofocus(ui, &field);
            ui.check_box(&mut add.global, "Global effect");
            crate::widgets::field_label(ui, "Parent (its reputations are copied)");
            mg_uikit::with_rows(ui, |ui| {
                for p in 1..STANDARD.len() as u32 {
                    ui.selectable_value(&mut add.parent, p, name(f, p));
                }
            });
            ((), crate::widgets::Foot::new().main("OK", !add.name.trim().is_empty()).cancel())
        });
        if let Some(footed) = footed {
            if footed.pressed.main() {
                let mut now = f.clone();
                let id = now.add(add.name.trim(), add.global, add.parent);
                view.selected = id;
                view.reveal = true;
                changed = Some((format!("Add faction {}", add.name.trim()), now));
                close = true;
            }
            if footed.pressed.cancelled() {
                close = true;
            }
        }
        view.adding = if close { None } else { Some(add) };
    }
    // (The faction the window was opened on, or none: a removal or an Undo
    // behind it closes it.)
    let named = |id: u32| f.factions.get(id as usize).map(|x| x.name.clone());
    if let Some((id, _)) = &view.renaming {
        let now = named(*id).map(|name| (f.factions.len(), name));
        let was = view.renaming_of.clone().or_else(|| now.clone());
        if now.is_none() || now != was || (*id as usize) < STANDARD.len() {
            view.renaming = None;
        }
        view.renaming_of = was.filter(|_| view.renaming.is_some());
    } else {
        view.renaming_of = None;
    }
    if let Some((id, mut new_name)) = view.renaming.clone() {
        let mut close = false;
        let footed = crate::widgets::modal(&ctx, "Change Faction Name").show_foot(&ctx, |ui| {
            let field = ui.add(mg_uikit::text_field(&mut new_name));
            crate::widgets::autofocus(ui, &field);
            ((), crate::widgets::Foot::new().main("OK", !new_name.trim().is_empty()).cancel())
        });
        if let Some(footed) = footed {
            if footed.pressed.main() {
                let mut now = f.clone();
                now.factions[id as usize].name = new_name.trim().to_string();
                changed = Some(("Rename faction".into(), now));
                close = true;
            }
            if footed.pressed.cancelled() {
                close = true;
            }
        }
        view.renaming = if close { None } else { Some((id, new_name)) };
        if close {
            view.renaming_of = None;
        }
    }
    if asked.remove && custom {
        remove(app, f, app.faction_view.selected);
    } else if let Some((what, now)) = changed {
        app.store_factions(&what, &now, None);
    }
    // (Noted from the factions as they are now: where a change was stored
    // this frame, the numbers the view set with it are the chosen cell's,
    // and the names it read before it would choose another.)
    let now = app.ws.as_ref().map(|w| (w.identity(), w.revision()));
    match app.faction_view.seen.clone() {
        Some(s) if Some((s.identity, s.revision)) == now => {
            if let Ok(f) = &s.factions {
                note_chosen(&mut app.faction_view, f);
            }
        }
        _ => app.faction_view.chosen_names = None,
    }
    asked.pressed
}

/// Removes faction `id`, where no object of the module is of it (asked of
/// the module with the open documents written into it: an object given the
/// faction a moment ago is one of them), renumbering the objects of those
/// after it; else says which objects are, and removes nothing.
fn remove(app: &mut Moonglow, f: &Factions, id: u32) {
    let users = (app.ws.as_mut())
        .map(|w| match w.flush() {
            Ok(_) => Ok(faction_users(&w.module, id)),
            Err(e) => Err(e.to_string()),
        })
        .unwrap_or(Ok(Vec::new()));
    let users = match users {
        Ok(users) => users,
        Err(e) => {
            app.log.error(e);
            return;
        }
    };
    if users.is_empty() {
        let name = name(f, id).to_string();
        let mut now = f.clone();
        if let Some(map) = now.remove(id) {
            let what = format!("Remove faction {name}");
            if app.store_factions(&what, &now, Some((id, Box::new(map)))) {
                let view = &mut app.faction_view;
                let last = now.factions.len() as u32 - 1;
                view.selected = view.selected.min(last);
                view.target = match view.target {
                    t if t == id => view.selected,
                    t if t > id => t - 1,
                    t => t,
                }
                .min(last);
                // (Numbered anew by the removal: found by number, not by name.
                // The removed faction's row goes to the faction that slid into
                // its number, or the last; its column, where it was the
                // chosen one, likewise.)
                view.chosen_names = None;
            }
        }
    } else {
        let list: Vec<String> = users.iter().map(ToString::to_string).collect();
        app.log.error(format!(
            "{} is used by {}; give those objects another faction first",
            name(f, id),
            list.join(", ")
        ));
    }
}

/// The matrix's measures (the Factions board's `.mx`): the row headers'
/// column, a cell's width and height, the gap between them, and the room
/// at the matrix's top and left.
const HEAD_WIDTH: f32 = 136.0;
const CELL_WIDTH: f32 = 56.0;
const CELL_HEIGHT: f32 = 22.0;
const CELL_GAP: f32 = 4.0;
const MATRIX_TOP: f32 = 6.0;
const MATRIX_LEFT: f32 = 8.0;

/// Whether the matrix has the keys: they were given to it last (a click on
/// a cell, F6), the faction editor is the active editor, and nothing modal,
/// no popover and none of its windows is in front.
pub(crate) fn matrix_has_keys(app: &Moonglow, ctx: &egui::Context) -> bool {
    crate::palette_view::keys_are(ctx, crate::palette_view::Keys::Factions)
        && crate::table_view::active_tab(app).as_ref() == Some(&Tab::Factions)
        && app.faction_view.advanced
        && app.faction_view.adding.is_none()
        && app.faction_view.renaming.is_none()
        && !crate::shell::keys_blocked(ctx)
        && !mg_uikit::popover::Popover::any_open(ctx)
}

/// The field a value is typed into in a cell.
const TYPING_ID: &str = "faction-cell-typing";

/// Tab is not bound in the matrix (the board's hints have none): while it
/// has the keys and nothing else has the keyboard's focus, egui does not
/// give the focus to the first control drawn (the Top Bar's, a panel's),
/// and the keys stay the matrix's. While a value is typed in a cell, Tab
/// and Shift+Tab set it (as Enter does) and the keys stay the matrix's,
/// the chosen cell where it was: the focus does not go on to the next
/// control (the tab's Close, whose Enter would close the editor). Asked
/// as the frame begins, before any control is drawn (`table_view`'s
/// `keep_tab_for_the_grid`, the same for the grid).
pub(crate) fn keep_tab_for_the_matrix(app: &mut Moonglow, ctx: &egui::Context) {
    let tab = ctx.input(|i| {
        let m = i.modifiers;
        i.key_pressed(egui::Key::Tab) && (!m.any() || m.shift_only())
    });
    if !tab {
        return;
    }
    let focused = ctx.memory(|m| m.focused());
    if app.faction_view.typing.is_some() && focused == Some(egui::Id::new(TYPING_ID)) {
        ctx.memory_mut(|m| m.move_focus(egui::FocusDirection::None));
        ctx.input_mut(|i| {
            i.consume_key(egui::Modifiers::NONE, egui::Key::Tab);
            i.consume_key(egui::Modifiers::SHIFT, egui::Key::Tab);
        });
        app.faction_view.tab_set = true;
        return;
    }
    if focused.is_some() {
        return;
    }
    if app.faction_view.typing.is_none() && matrix_has_keys(app, ctx) {
        ctx.memory_mut(|m| m.move_focus(egui::FocusDirection::None));
    }
}

/// The matrix (the Factions board's `.mx-view`): a row for each faction
/// that regards (the PC's own feelings are not stored: it has none), a
/// column for each it regards, each cell its reputation framed red where
/// hostile and blue where friendly; the column headers frozen over it and
/// the row headers at its left as it scrolls, the chosen cell ringed and
/// its row's and column's headers in the accent. Only the cells and
/// headers in sight are laid out. A click chooses a cell (and gives the
/// matrix the keys: the arrows move, a digit starts typing a value, Enter
/// types or sets it, Escape stops typing), a drag sideways changes it (one
/// step, as it is let go), a double click types a value.
fn matrix(app: &mut Moonglow, ui: &mut Ui, f: &Factions) {
    let ctx = ui.ctx().clone();
    let t = mg_uikit::tokens(&ctx);
    let n = f.factions.len() as u32;
    let has_keys = matrix_has_keys(app, &ctx);
    let view = &mut app.faction_view;
    // (A row is a faction's that regards: the PC's is not stored, and with
    // the PC chosen, from the Outliner, no row is the chosen one.)
    view.selected = view.selected.min(n.saturating_sub(1));
    let has_row = view.selected >= 1;
    let tabbed = std::mem::take(&mut view.tab_set);
    let whole = ui.available_rect_before_wrap();
    ui.allocate_rect(whole, egui::Sense::hover());
    view.hovered = Hovered::Nothing;
    let pitch = egui::vec2(CELL_WIDTH + CELL_GAP, CELL_HEIGHT + CELL_GAP);
    let cells_at = egui::pos2(
        whole.left() + MATRIX_LEFT + HEAD_WIDTH + CELL_GAP,
        whole.top() + MATRIX_TOP + CELL_HEIGHT + CELL_GAP,
    );
    let cells_room = egui::Rect::from_min_max(cells_at, whole.max);
    if !cells_room.is_positive() {
        return;
    }
    let mut keys_moved = false;
    if has_keys && view.typing.is_none() && !ctx.egui_wants_keyboard_input() {
        use egui::Key;
        let pressed = ui.input_mut(|i| {
            [Key::ArrowUp, Key::ArrowDown, Key::ArrowLeft, Key::ArrowRight, Key::Enter]
                .into_iter()
                .find(|k| i.consume_key(egui::Modifiers::NONE, *k))
        });
        match pressed {
            Some(Key::ArrowUp) => view.selected = view.selected.saturating_sub(1).max(1),
            Some(Key::ArrowDown) => view.selected = (view.selected + 1).min(n - 1),
            Some(Key::ArrowLeft) => view.target = view.target.saturating_sub(1),
            Some(Key::ArrowRight) => view.target = (view.target + 1).min(n - 1),
            Some(Key::Enter) if has_row => {
                let rep = rep_of(f, view.selected, view.target);
                view.typing = Some((view.selected, view.target, rep.to_string()));
                view.typing_began = true;
                view.reveal = true;
            }
            _ => {}
        }
        // (Tab is not bound, by the board's hints: it moves nothing, and
        // the keys stay the matrix's, not egui's next field's.)
        ui.input_mut(|i| {
            i.consume_key(egui::Modifiers::NONE, Key::Tab);
            i.consume_key(egui::Modifiers::SHIFT, Key::Tab);
        });
        keys_moved = matches!(
            pressed,
            Some(Key::ArrowUp | Key::ArrowDown | Key::ArrowLeft | Key::ArrowRight)
        );
        // (A digit typed: a value begun.)
        let digit = ui.input(|i| {
            i.events.iter().find_map(|e| match e {
                egui::Event::Text(s) if !s.is_empty() && s.chars().all(|c| c.is_ascii_digit()) => {
                    Some(s.clone())
                }
                _ => None,
            })
        });
        if let Some(d) = digit.filter(|_| has_row) {
            view.typing = Some((view.selected, view.target, d));
            view.typing_began = true;
            // (Begun on a cell scrolled out of sight: brought into it.)
            view.reveal = true;
            ctx.request_repaint();
        }
    }
    if keys_moved {
        view.reveal = true;
        ctx.request_repaint();
    }
    let reveal = std::mem::take(&mut view.reveal);
    let rows = n.saturating_sub(1) as usize;
    let cols = n as usize;
    let content = egui::vec2(cols as f32 * pitch.x, rows as f32 * pitch.y);
    let mut child =
        ui.new_child(egui::UiBuilder::new().max_rect(cells_room).id_salt("faction-matrix"));
    let mut clicked = None;
    let mut double = None;
    let mut drag: Option<(u32, u32, f32, bool, bool)> = None;
    let mut set: Option<(u32, u32, u32)> = None;
    let typing_id = egui::Id::new(TYPING_ID);
    let mut typing_drawn = false;
    let mut over_cell = false;
    let shown = egui::ScrollArea::both()
        .id_salt("faction-matrix-scroll")
        .auto_shrink(false)
        .show_viewport(&mut child, |ui, sight| {
            mg_uikit::name_group(ui, MATRIX);
            // (Its box the part in sight: egui gives a group none of its
            // own on every frame.)
            let seen = ui.clip_rect();
            ui.ctx().accesskit_node_builder(ui.unique_id(), |b| {
                b.set_bounds(egui::accesskit::Rect {
                    x0: seen.min.x.into(),
                    y0: seen.min.y.into(),
                    x1: seen.max.x.into(),
                    y1: seen.max.y.into(),
                })
            });
            ui.set_min_size(content);
            let origin = ui.max_rect().min;
            let cell = |r: usize, c: usize| {
                egui::Rect::from_min_size(
                    origin + egui::vec2(c as f32 * pitch.x, r as f32 * pitch.y),
                    egui::vec2(CELL_WIDTH, CELL_HEIGHT),
                )
            };
            if reveal && has_row {
                ui.scroll_to_rect(
                    cell(view.selected as usize - 1, view.target as usize).expand(4.0),
                    None,
                );
            }
            let first_r = ((sight.min.y / pitch.y).floor().max(0.0) as usize).min(rows);
            let last_r = (((sight.max.y / pitch.y).ceil().max(0.0) as usize) + 1).min(rows);
            let first_c = ((sight.min.x / pitch.x).floor().max(0.0) as usize).min(cols);
            let last_c = (((sight.max.x / pitch.x).ceil().max(0.0) as usize) + 1).min(cols);
            let font = egui::FontId::proportional(13.0);
            for r in first_r..last_r {
                let p = r as u32 + 1;
                for c in first_c..last_c {
                    let tt = c as u32;
                    let rect = cell(r, c);
                    let stored = rep_of(f, p, tt);
                    let shown_rep = match view.dragging {
                        Some((dp, dt, start, by)) if (dp, dt) == (p, tt) => {
                            (start as f32 + by).round().clamp(0.0, 100.0) as u32
                        }
                        _ => stored,
                    };
                    let chosen = (p, tt) == (view.selected, view.target);
                    let id = ui.id().with(("faction-cell", p, tt));
                    // (Not egui's focus: the keys are the matrix's, `Keys`.)
                    let resp = ui.interact(rect, id, egui::Sense::CLICK | egui::Sense::DRAG);
                    over_cell |= resp.hovered();
                    // (Its sentence made only where it is asked for: by a
                    // screen reader, or the pointer over it.)
                    resp.widget_info(|| {
                        let words = sentence(f, p, tt, shown_rep);
                        egui::WidgetInfo::slider(true, f64::from(shown_rep), words)
                    });
                    cell_access(ui, id, shown_rep, chosen);
                    if let Some(v) = cell_asked(ui, id, stored) {
                        set = Some((p, tt, v));
                        (view.selected, view.target) = (p, tt);
                    }
                    let painter = ui.painter();
                    let bg = match chosen {
                        true => t.inset.lerp_to_gamma(t.accent, 0.18),
                        false => t.inset,
                    };
                    painter.rect_filled(rect, 2.0, bg);
                    let edge = match shown_rep {
                        0..=10 | 90.. => egui::Stroke::new(2.0, color(shown_rep)),
                        _ => egui::Stroke::new(1.0, t.line_soft),
                    };
                    painter.rect_stroke(rect, 2.0, edge, egui::StrokeKind::Inside);
                    if chosen {
                        let ring = egui::Stroke::new(2.0, t.accent);
                        painter.rect_stroke(rect, 3.0, ring, egui::StrokeKind::Outside);
                    }
                    let typing_here =
                        view.typing.as_ref().is_some_and(|(a, b, _)| (*a, *b) == (p, tt));
                    if typing_here {
                        typing_drawn = true;
                        if let Some((_, _, text)) = view.typing.as_mut() {
                            let field = egui::TextEdit::singleline(text)
                                .id(typing_id)
                                .char_limit(3)
                                .horizontal_align(egui::Align::Max)
                                .frame(egui::Frame::NONE)
                                .margin(egui::Margin::symmetric(5, 2));
                            let r = ui.put(rect.shrink(1.0), field);
                            if view.typing_began {
                                r.request_focus();
                                view.typing_began = false;
                            }
                            let enter =
                                r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                            let escape = ui.input(|i| i.key_pressed(egui::Key::Escape));
                            if escape && (r.has_focus() || r.lost_focus()) {
                                view.typing = None;
                            } else if enter || tabbed || r.lost_focus() {
                                if let Some(v) = typed_value(text) {
                                    set = Some((p, tt, v));
                                }
                                view.typing = None;
                                if tabbed {
                                    r.surrender_focus();
                                }
                            }
                        }
                    } else {
                        let galley =
                            painter.layout_no_wrap(shown_rep.to_string(), font.clone(), t.text);
                        let at = egui::pos2(
                            rect.right() - 6.0 - galley.size().x,
                            rect.center().y - galley.size().y / 2.0,
                        );
                        painter.galley(at, galley, t.text);
                    }
                    let resp = resp.on_hover_ui(|ui| _ = ui.label(sentence(f, p, tt, shown_rep)));
                    if resp.clicked() {
                        clicked = Some((p, tt));
                    }
                    if resp.double_clicked() {
                        double = Some((p, tt, stored));
                    }
                    if resp.drag_started() || resp.dragged() || resp.drag_stopped() {
                        drag = Some((
                            p,
                            tt,
                            resp.drag_delta().x,
                            resp.drag_started(),
                            resp.drag_stopped(),
                        ));
                    }
                }
            }
        });
    if over_cell {
        view.hovered = Hovered::Cell;
    }
    // (A value typed in a cell scrolled out of sight is set, as when its
    // field is left: the matrix's keys are not left waiting on a field
    // that is not drawn. Not while the cell is being brought into sight.)
    if !typing_drawn
        && !reveal
        && let Some((p, tt, text)) = view.typing.take()
    {
        view.typing_began = false;
        if let Some(v) = typed_value(&text) {
            set = Some((p, tt, v));
        }
    }
    let offset = shown.state.offset;
    // The headers, frozen: the columns' over the cells, the rows' at their
    // left, moved with the cells as they scroll.
    let painter = ui.painter_at(whole);
    let small = egui::FontId::proportional(12.0);
    let body = egui::TextStyle::Body.resolve(ui.style());
    let heads_y = whole.top() + MATRIX_TOP;
    let col_strip =
        egui::Rect::from_x_y_ranges(cells_room.x_range(), heads_y..=heads_y + CELL_HEIGHT);
    let row_strip = egui::Rect::from_x_y_ranges(
        whole.left() + MATRIX_LEFT..=whole.left() + MATRIX_LEFT + HEAD_WIDTH,
        cells_room.y_range(),
    );
    let first_c = ((offset.x / pitch.x).floor().max(0.0) as usize).min(cols);
    let last_c = ((((offset.x + cells_room.width()) / pitch.x).ceil() as usize) + 1).min(cols);
    let first_r = ((offset.y / pitch.y).floor().max(0.0) as usize).min(rows);
    let last_r = ((((offset.y + cells_room.height()) / pitch.y).ceil() as usize) + 1).min(rows);
    let col_painter = painter.with_clip_rect(col_strip);
    for c in first_c..last_c {
        let x = cells_room.left() + c as f32 * pitch.x - offset.x;
        let rect =
            egui::Rect::from_min_size(egui::pos2(x, heads_y), egui::vec2(CELL_WIDTH, CELL_HEIGHT));
        let words = name(f, c as u32).to_string();
        let color = if c as u32 == view.target { t.accent } else { t.text_2 };
        let mut job = egui::text::LayoutJob::simple_singleline(words.clone(), small.clone(), color);
        job.wrap = egui::text::TextWrapping::truncate_at_width(CELL_WIDTH - 4.0);
        let galley = col_painter.layout_job(job);
        let at = egui::pos2(
            rect.center().x - galley.size().x / 2.0,
            rect.center().y - galley.size().y / 2.0,
        );
        col_painter.galley(at, galley, color);
        if let Some(seen) =
            rect.intersect(col_strip).is_positive().then_some(rect.intersect(col_strip))
        {
            let r = ui.interact(seen, egui::Id::new(("faction-column", c)), egui::Sense::hover());
            r.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Label, true, &words));
            r.on_hover_text(&words);
        }
    }
    let row_painter = painter.with_clip_rect(row_strip);
    for r in first_r..last_r {
        let p = r as u32 + 1;
        let y = cells_room.top() + r as f32 * pitch.y - offset.y;
        let rect = egui::Rect::from_min_size(
            egui::pos2(row_strip.left(), y),
            egui::vec2(HEAD_WIDTH, CELL_HEIGHT),
        );
        let words = name(f, p).to_string();
        let color = if p == view.selected { t.accent } else { t.text };
        let mut job = egui::text::LayoutJob::simple_singleline(words.clone(), body.clone(), color);
        job.wrap = egui::text::TextWrapping::truncate_at_width(HEAD_WIDTH - 8.0);
        let galley = row_painter.layout_job(job);
        let at = egui::pos2(
            rect.right() - 8.0 - galley.size().x,
            rect.center().y - galley.size().y / 2.0,
        );
        row_painter.galley(at, galley, color);
        if let Some(seen) =
            rect.intersect(row_strip).is_positive().then_some(rect.intersect(row_strip))
        {
            let r = ui.interact(seen, egui::Id::new(("faction-row", p)), egui::Sense::hover());
            r.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Label, true, &words));
            r.on_hover_text(&words);
        }
    }
    if let Some((p, tt)) = clicked {
        (view.selected, view.target) = (p, tt);
        if view.typing.as_ref().is_some_and(|(a, b, _)| (*a, *b) != (p, tt)) {
            view.typing = None;
        }
        crate::palette_view::give_keys(&ctx, crate::palette_view::Keys::Factions);
        ctx.request_repaint();
    }
    if let Some((p, tt, stored)) = double {
        (view.selected, view.target) = (p, tt);
        view.typing = Some((p, tt, stored.to_string()));
        view.typing_began = true;
        view.dragging = None;
        ctx.request_repaint();
    }
    if let Some((p, tt, dx, started, stopped)) = drag {
        if started {
            view.dragging = Some((p, tt, rep_of(f, p, tt), 0.0));
            (view.selected, view.target) = (p, tt);
            crate::palette_view::give_keys(&ctx, crate::palette_view::Keys::Factions);
        }
        if let Some(d) = view.dragging.as_mut().filter(|d| (d.0, d.1) == (p, tt)) {
            // (Half a value a point: the cell's 56 points cover 28.)
            d.3 += dx / 2.0;
        }
        if stopped && let Some((dp, dt, start, by)) = view.dragging.take() {
            let v = (start as f32 + by).round().clamp(0.0, 100.0) as u32;
            if v != start {
                set = Some((dp, dt, v));
            }
        }
        ctx.request_repaint();
    }
    // (Let go of outside the cell it began on: no drag is left behind.)
    if view.dragging.is_some() && drag.is_none() && !ctx.input(|i| i.pointer.any_down()) {
        view.dragging = None;
    }
    if has_keys {
        crate::palette_view::keys_outline(ui, cells_room, crate::palette_view::Keys::Factions);
    }
    let _ = typing_id;
    if let Some((p, tt, v)) = set
        && v != rep_of(f, p, tt)
    {
        let mut now = f.clone();
        now.set_reputation(p, tt, v);
        app.store_factions("Change reputation", &now, None);
    }
}

/// A value typed into a cell, as it is set: a whole number, above 100 set
/// as 100; anything else (letters, a sign, nothing) sets nothing.
fn typed_value(text: &str) -> Option<u32> {
    text.trim().parse::<u32>().ok().map(|v| v.min(100))
}

/// A cell as a screen reader has it: a value from 0 to 100 in steps of
/// one that can be set, made more or less (as a slider is), the chosen
/// cell selected.
fn cell_access(ui: &Ui, id: egui::Id, value: u32, chosen: bool) {
    ui.ctx().accesskit_node_builder(id, |b| {
        use egui::accesskit::Action;
        b.set_selected(chosen);
        b.set_min_numeric_value(0.0);
        b.set_max_numeric_value(100.0);
        b.set_numeric_value_step(1.0);
        b.add_action(Action::SetValue);
        if value < 100 {
            b.add_action(Action::Increment);
        }
        if value > 0 {
            b.add_action(Action::Decrement);
        }
    });
}

/// What a screen reader asked of a cell holding `stored`: a value set, or
/// one more or one less (each one step of Undo, as a value typed is).
fn cell_asked(ui: &Ui, id: egui::Id, stored: u32) -> Option<u32> {
    use egui::accesskit::{Action, ActionData};
    ui.input(|i| {
        let mut v = i64::from(stored);
        let mut asked = false;
        for r in i.accesskit_action_requests(id, Action::SetValue) {
            if let Some(ActionData::NumericValue(x)) = r.data {
                v = x.round().clamp(0.0, 100.0) as i64;
                asked = true;
            }
        }
        let up = i.num_accesskit_action_requests(id, Action::Increment) as i64;
        let down = i.num_accesskit_action_requests(id, Action::Decrement) as i64;
        if up + down > 0 {
            v = (v + up - down).clamp(0, 100);
            asked = true;
        }
        asked.then_some(v as u32)
    })
}

/// The chart (Basic, the moves' Basic chart view): how the chosen faction
/// regards each of the others, a slider a faction (one step, as it is let
/// go) with the word for it; with Full detail, how each of them regards
/// it too. The PC's own feelings are not stored: for it, Full detail alone
/// shows anything. Only the rows in sight are laid out. What it changed,
/// to store.
fn chart(app: &mut Moonglow, ui: &mut Ui, f: &Factions) -> Option<(String, Factions)> {
    let view = &mut app.faction_view;
    let s = view.selected;
    let n = f.factions.len() as u32;
    view.hovered = Hovered::Nothing;
    let mut hovered = Hovered::Nothing;
    let mut changed = None;
    let mut chosen = None;
    egui::Frame::new().inner_margin(egui::Margin { left: 8, right: 8, top: 6, bottom: 6 }).show(
        ui,
        |ui| {
            ui.strong(format!("How {} regards the others", name(f, s)));
            if s == 0 {
                ui.weak(PC_NOTE);
            }
            let mut rows: Vec<(u32, u32, bool)> = Vec::new();
            for t in 0..n {
                if s != 0 {
                    rows.push((s, t, false));
                }
                if view.full_detail && t != 0 {
                    rows.push((t, s, true));
                }
            }
            let foot = mg_uikit::metrics::CONTROL_HEIGHT + 8.0;
            let room = (ui.available_height() - foot).max(mg_uikit::metrics::ROW_HEIGHT);
            egui::ScrollArea::vertical()
                .id_salt("faction-chart")
                .max_height(room)
                .auto_shrink([false, true])
                .show_rows(ui, mg_uikit::metrics::ROW_HEIGHT, rows.len(), |ui, range| {
                    for (p, target, from) in rows[range].iter().copied() {
                        ui.horizontal(|ui| {
                            let label = match (view.full_detail, from) {
                                (false, _) => name(f, target).to_string(),
                                (true, false) => format!("Toward {}", name(f, target)),
                                (true, true) => format!("From {}", name(f, p)),
                            };
                            let head = ui.add_sized(
                                [160.0, mg_uikit::metrics::CONTROL_HEIGHT],
                                egui::Button::new(label).frame(false).truncate(),
                            );
                            if head.clicked() && !from {
                                chosen = Some(target);
                            }
                            if head.hovered() && !from {
                                hovered = Hovered::Name;
                            }
                            let stored = rep_of(f, p, target);
                            let mut rep = stored;
                            let r = mg_uikit::slider(
                                ui,
                                egui::Slider::new(&mut rep, 0..=100)
                                    .clamping(egui::SliderClamping::Edits)
                                    .show_value(true),
                            );
                            r.widget_info(|| {
                                egui::WidgetInfo::slider(
                                    true,
                                    f64::from(rep),
                                    sentence(f, p, target, rep),
                                )
                            });
                            if r.hovered() || r.dragged() {
                                hovered = Hovered::Slider;
                            }
                            let fill = r.rect.with_max_x(r.rect.min.x + 4.0);
                            ui.painter().rect_filled(fill, 0.0, color(rep));
                            ui.weak(attitude(rep));
                            if (r.drag_stopped() || (r.changed() && !r.dragged())) && rep != stored
                            {
                                let mut now = f.clone();
                                now.set_reputation(p, target, rep);
                                changed = Some(("Change reputation".to_string(), now));
                            }
                        });
                    }
                });
            ui.add_space(4.0);
            ui.check_box(&mut view.full_detail, "Full detail")
                .on_hover_text("Also show how the others regard the selected faction");
        },
    );
    if let Some(t) = chosen {
        view.target = t;
    }
    view.hovered = hovered;
    changed
}

/// What Basic's chart and the Reactions panel say with the PC chosen.
const PC_NOTE: &str =
    "The PC faction's own feelings are not stored. Full detail shows how the others regard PCs.";

/// The faction editor's panels' factions, where `editor` is its tab.
fn factions_of(
    app: &mut Moonglow,
    ui: &mut Ui,
    editor: Option<&Tab>,
    keys: crate::palette_view::Keys,
) -> Option<(Arc<Seen>, bool)> {
    if editor != Some(&Tab::Factions) {
        return None;
    }
    crate::palette_view::keys_outline(ui, ui.max_rect(), keys);
    if ui.input(|i| i.pointer.any_pressed()) && ui.rect_contains_pointer(ui.max_rect()) {
        crate::palette_view::give_keys(ui.ctx(), keys);
    }
    let focus = app.dialog_panel_focus.take_if(|k| *k == keys).is_some();
    let seen = begin(app)?;
    Some((seen, focus))
}

/// The room round a panel's body (the board's `.panel-body`, 8).
const PANEL_MARGIN: egui::Margin = egui::Margin::same(8);

/// The width of the panels' labels (the board's `--lbl-w`, 96).
const LABEL_WIDTH: f32 = 96.0;

fn label_cell(ui: &mut Ui, label: &str) {
    ui.allocate_ui_with_layout(
        egui::vec2(LABEL_WIDTH, mg_uikit::metrics::CONTROL_HEIGHT),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            ui.set_min_width(LABEL_WIDTH);
            if !label.is_empty() {
                crate::widgets::field_label(ui, label);
            }
        },
    );
}

/// The Faction panel (the Factions board's `.faction-panel`): the chosen
/// faction's Name and Parent, read-only (Change Name… renames it), and
/// Global effect, for a faction of the module's own.
pub(crate) fn faction_panel(app: &mut Moonglow, ui: &mut Ui, editor: Option<&Tab>) {
    let keys = crate::palette_view::Keys::FactionPanel;
    let Some((seen, focus)) = factions_of(app, ui, editor, keys) else { return };
    let Ok(f) = &seen.factions else { return };
    let s = app.faction_view.selected;
    let Some(faction) = f.factions.get(s as usize) else { return };
    let custom = s as usize >= STANDARD.len();
    let t = mg_uikit::tokens(ui.ctx());
    let parent = match faction.parent {
        Some(p) => name(f, p).to_string(),
        None => "None".to_string(),
    };
    let mut global = faction.global;
    let mut changed = false;
    egui::Frame::new().inner_margin(PANEL_MARGIN).show(ui, |ui| {
        ui.spacing_mut().item_spacing.y = mg_uikit::metrics::FORM_ROW_GAP;
        let width = (ui.available_width() - LABEL_WIDTH - 8.0).max(40.0);
        for (label, value) in [("Name", faction.name.as_str()), ("Parent", parent.as_str())] {
            ui.horizontal(|ui| {
                label_cell(ui, label);
                let mut shown = value;
                let field = mg_uikit::text_field(&mut shown).desired_width(width);
                // (Read-only: in the second text's color, the board's.)
                let r = ui.add(field.text_color(t.text_2));
                r.widget_info(|| {
                    let mut info = egui::WidgetInfo::text_edit(true, value, value, "");
                    info.label = Some(label.to_string());
                    info
                });
            });
        }
        ui.horizontal(|ui| {
            label_cell(ui, "");
            let r = ui
                .add_enabled(custom, mg_uikit::Checkbox::new(&mut global, "Global effect"))
                .on_hover_text("A change of reputation applies to the whole faction")
                .on_disabled_hover_text("A standard faction's is the game's");
            if focus {
                r.request_focus();
            }
            changed = r.changed();
        });
    });
    if changed {
        let mut now = f.clone();
        now.factions[s as usize].global = global;
        app.store_factions("Faction global effect", &now, None);
    }
}

/// What Reaction and Reactions say after their titles (the Factions
/// board's "Victim (Danger) toward Hostile" and "How Victim (Danger)
/// regards the others").
pub(crate) fn panel_detail(app: &Moonglow, panel: crate::shell::Panel) -> Option<String> {
    let view = &app.faction_view;
    let f = view.seen.as_ref()?.factions.as_ref().ok()?;
    let (p, t) = (name(f, view.selected), name(f, view.target));
    Some(match panel {
        // (The PC's own feelings are not stored: no reaction is chosen.)
        crate::shell::Panel::Reaction if view.selected == 0 => return None,
        crate::shell::Panel::Reaction => format!("{p} toward {t}"),
        // (Its rows are the others' feelings toward PCs, "From …".)
        _ if view.selected == 0 => "How the others regard PCs".to_string(),
        _ => format!("How {p} regards the others"),
    })
}

/// The Reaction panel (the Factions board's `.pair-panel`): the chosen
/// cell's sentence ("Victim (Danger) is hostile toward Hostile."), its
/// color's mark, a slider and the value's field (each change one step: the
/// slider's as it is let go), and the scale of Hostile 0 to 10, Neutral 11
/// to 89, Friendly 90 to 100, the reaction's own in the text's color.
pub(crate) fn reaction_panel(app: &mut Moonglow, ui: &mut Ui, editor: Option<&Tab>) {
    let keys = crate::palette_view::Keys::Reaction;
    let Some((seen, focus)) = factions_of(app, ui, editor, keys) else { return };
    let Ok(f) = &seen.factions else { return };
    let t = mg_uikit::tokens(ui.ctx());
    let (p, target) = (app.faction_view.selected, app.faction_view.target);
    let mut set = None;
    egui::Frame::new().inner_margin(PANEL_MARGIN).show(ui, |ui| {
        if p == 0 {
            ui.add(
                egui::Label::new(
                    "The PC faction's own feelings are not stored. Choose another faction to see how it regards PCs.",
                )
                .wrap(),
            );
            return;
        }
        let stored = rep_of(f, p, target);
        ui.add(egui::Label::new(format!("{}.", sentence(f, p, target, stored))).wrap());
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            let (mark, _) = ui.allocate_exact_size(egui::vec2(4.0, 14.0), egui::Sense::hover());
            ui.painter().rect_filled(mark, 1.0, color(stored));
            let field = 48.0;
            ui.spacing_mut().slider_width =
                (ui.available_width() - field - 2.0 * ui.spacing().item_spacing.x).max(40.0);
            let mut rep = stored;
            let r = mg_uikit::slider(
                ui,
                egui::Slider::new(&mut rep, 0..=100)
                    .clamping(egui::SliderClamping::Always)
                    .show_value(false),
            );
            r.widget_info(|| egui::WidgetInfo::slider(true, f64::from(rep), "Reputation"));
            if focus {
                r.request_focus();
            }
            if (r.drag_stopped() || (r.changed() && !r.dragged())) && rep != stored {
                set = Some(rep);
            }
            // (Its id made of the pair's places and names: the factions
            // changed under a value being typed (a plugin's or Find and
            // Replace's write of repute.fac) make it another field, and
            // what was typed is dropped, never set on another pair (M4's
            // U22 review).)
            let name = |i: u32| f.factions.get(i as usize).map(|x| x.name.as_str());
            let pair = (p, target, name(p), name(target));
            let typed = ui.push_id(pair, |ui| {
                ui.set_width(field);
                let (set, _, r) =
                    crate::widgets::drag_number_shown(ui, stored, |d| d.range(0..=100));
                // (Named for a screen reader, as the slider beside it is.)
                ui.ctx().accesskit_node_builder(r.id, |b| b.set_label("Reputation value"));
                set
            });
            if let Some(v) = typed.inner.filter(|v| *v != stored) {
                set = Some(v);
            }
        });
        // The scale: Hostile, Neutral and Friendly in their colors, 11 to
        // 79 to 11, the reaction's own words in the text's color.
        ui.add_space(8.0);
        let room = ui.available_rect_before_wrap();
        let bar = egui::Rect::from_min_size(room.min + egui::vec2(12.0, 0.0), egui::vec2((room.width() - 12.0).max(0.0), 3.0));
        let unit = (bar.width() - 4.0) / 101.0;
        let mut x = bar.left();
        for (share, fill) in [(11.0, color(0)), (79.0, color(50)), (11.0, color(100))] {
            let w = share * unit;
            ui.painter().rect_filled(egui::Rect::from_min_size(egui::pos2(x, bar.top()), egui::vec2(w, 3.0)), 1.0, fill);
            x += w + 2.0;
        }
        ui.allocate_rect(bar.expand2(egui::vec2(0.0, 2.0)), egui::Sense::hover());
        let now = attitude(stored);
        let words = [("Hostile 0 to 10", "hostile"), ("Neutral 11 to 89", "neutral"), ("Friendly 90 to 100", "friendly")];
        let r = ui
            .horizontal(|ui| {
                ui.add_space(12.0);
                let small = egui::FontId::proportional(mg_uikit::metrics::SMALL_SIZE);
                let room = ui.available_width();
                let galleys: Vec<_> = words
                    .iter()
                    .map(|(w, a)| {
                        let c = if *a == now { t.text } else { t.text_3 };
                        (ui.painter().layout_no_wrap(w.to_string(), small.clone(), c), c)
                    })
                    .collect();
                let (rect, r) = ui.allocate_exact_size(egui::vec2(room, 14.0), egui::Sense::hover());
                let total: f32 = galleys.iter().map(|(g, _)| g.size().x).sum();
                let gap = ((rect.width() - total) / 2.0).max(4.0);
                let mut x = rect.left();
                for (g, c) in galleys {
                    let w = g.size().x;
                    ui.painter().galley(egui::pos2(x, rect.top()), g, c);
                    x += w + gap;
                }
                r
            })
            .inner;
        r.widget_info(|| {
            egui::WidgetInfo::labeled(
                egui::WidgetType::Label,
                true,
                "Hostile 0 to 10, neutral 11 to 89, friendly 90 to 100",
            )
        });
    });
    if let Some(v) = set {
        let mut now = f.clone();
        now.set_reputation(p, target, v);
        app.store_factions("Change reputation", &now, None);
    }
}

/// The Reactions panel (the Factions board's `.rx-panel`, Advanced): how
/// the chosen faction regards each faction, a row each (its name, a bar
/// filled to the value in the reaction's color, the value, the word), the
/// chosen cell's row chosen; a click on a row chooses its cell. With Full
/// detail (at its foot), under each, how that faction regards the chosen
/// one. Only the rows in sight are laid out.
pub(crate) fn reactions_panel(app: &mut Moonglow, ui: &mut Ui, editor: Option<&Tab>) {
    let keys = crate::palette_view::Keys::Reactions;
    let Some((seen, focus)) = factions_of(app, ui, editor, keys) else { return };
    let Ok(f) = &seen.factions else { return };
    let t = mg_uikit::tokens(ui.ctx());
    let view = &mut app.faction_view;
    let s = view.selected;
    let n = f.factions.len() as u32;
    let mut rows: Vec<(u32, u32, bool)> = Vec::new();
    for x in 0..n {
        if s != 0 {
            rows.push((s, x, false));
        }
        if view.full_detail && x != 0 && x != s {
            rows.push((x, s, true));
        }
    }
    let whole = ui.max_rect();
    let foot_h = mg_uikit::metrics::CONTROL_HEIGHT + 12.0;
    let list = whole.with_max_y((whole.bottom() - foot_h).max(whole.top()));
    let mut chosen = None;
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(list));
    let pitch = mg_uikit::metrics::ROW_HEIGHT;
    // (The rows touch: `show_rows` measures them with the spacing of the
    // ui it is given, so it is none there, or the rows it lays out stop
    // short of the list's foot.)
    child.spacing_mut().item_spacing.y = 0.0;
    if s == 0 && !view.full_detail {
        child.add(egui::Label::new(PC_NOTE).wrap());
    }
    egui::ScrollArea::vertical().id_salt("faction-reactions").auto_shrink(false).show_rows(
        &mut child,
        pitch,
        rows.len(),
        |ui, range| {
            for (i, (p, x, from)) in rows[range.clone()].iter().copied().enumerate() {
                let (rect, r) = ui.allocate_exact_size(
                    egui::vec2(ui.available_width(), pitch),
                    egui::Sense::click(),
                );
                let rep = rep_of(f, p, x);
                let selected = !from && x == view.target;
                if selected {
                    ui.painter().rect_filled(rect, 0.0, t.accent_fill);
                } else if r.hovered() && !from {
                    ui.painter().rect_filled(rect, 0.0, t.hover_row);
                }
                let (text, weak) =
                    if selected { (t.on_fill, t.on_fill) } else { (t.text, t.text_3) };
                let body = egui::TextStyle::Body.resolve(ui.style());
                let small = egui::FontId::proportional(mg_uikit::metrics::SMALL_SIZE);
                let label = match from {
                    false => name(f, x).to_string(),
                    true => format!("From {}", name(f, p)),
                };
                r.widget_info(|| {
                    let said = format!("{label}: {}", sentence(f, p, x, rep));
                    egui::WidgetInfo::selected(
                        egui::WidgetType::SelectableLabel,
                        true,
                        selected,
                        &said,
                    )
                });
                let cy = rect.center().y;
                let mut job = egui::text::LayoutJob::simple_singleline(
                    label,
                    body.clone(),
                    if from { weak } else { text },
                );
                job.wrap = egui::text::TextWrapping::truncate_at_width(118.0);
                let g = ui.painter().layout_job(job);
                ui.painter().galley(egui::pos2(rect.left() + 8.0, cy - g.size().y / 2.0), g, text);
                let value_w = 26.0;
                let word_w = 52.0;
                let bar_left = rect.left() + 8.0 + 118.0 + 8.0;
                let bar_right = rect.right() - 8.0 - word_w - 8.0 - value_w - 8.0;
                if bar_right > bar_left {
                    let bar = egui::Rect::from_min_max(
                        egui::pos2(bar_left, cy - 3.0),
                        egui::pos2(bar_right, cy + 3.0),
                    );
                    ui.painter().rect_filled(bar, 2.0, t.inset);
                    let edge = if selected { t.on_fill.gamma_multiply(0.3) } else { t.line_soft };
                    ui.painter().rect_stroke(
                        bar,
                        2.0,
                        egui::Stroke::new(1.0, edge),
                        egui::StrokeKind::Outside,
                    );
                    let filled = bar.with_max_x(bar.left() + bar.width() * rep as f32 / 100.0);
                    ui.painter().rect_filled(filled, 2.0, color(rep));
                }
                let v = ui.painter().layout_no_wrap(rep.to_string(), body, text);
                let vx = rect.right() - 8.0 - word_w - 8.0 - v.size().x;
                ui.painter().galley(egui::pos2(vx, cy - v.size().y / 2.0), v, text);
                let w = ui.painter().layout_no_wrap(attitude(rep).to_string(), small, weak);
                ui.painter().galley(
                    egui::pos2(rect.right() - 8.0 - word_w, cy - w.size().y / 2.0),
                    w,
                    weak,
                );
                if focus && i == 0 && range.start == 0 {
                    r.request_focus();
                }
                if r.clicked() && !from {
                    chosen = Some(x);
                }
            }
        },
    );
    // The foot: Full detail.
    let foot = egui::Rect::from_min_max(egui::pos2(whole.left(), list.bottom()), whole.max);
    ui.painter().hline(foot.x_range(), foot.top(), egui::Stroke::new(1.0, t.line));
    let mut foot_ui =
        ui.new_child(egui::UiBuilder::new().max_rect(foot.shrink2(egui::vec2(8.0, 6.0))));
    foot_ui
        .check_box(&mut view.full_detail, "Full detail")
        .on_hover_text("Also show how the others regard the selected faction");
    if let Some(x) = chosen {
        view.target = x;
        view.reveal = true;
        ui.ctx().request_repaint();
    }
}

/// The faction editor's factions as the Outliner lists them (their names,
/// made once a revision with the factions read, not at each frame), and
/// the chosen faction's place; `None` where they can't be read.
pub(crate) fn outline(app: &mut Moonglow) -> Option<(Arc<[String]>, u32)> {
    let seen = begin(app)?;
    seen.factions.as_ref().ok()?;
    Some((seen.names.clone(), app.faction_view.selected))
}

/// The Outliner's row `id` chosen: the chosen faction (the chosen cell's
/// row).
pub(crate) fn choose(app: &mut Moonglow, id: u32) {
    let view = &mut app.faction_view;
    view.selected = id;
    view.reveal = true;
}

/// What the Status Bar shows beside the faction editor (the Factions
/// board's): with the pointer over a cell of the matrix, Click Select a
/// reaction, Drag Change a reaction and Double Click Type a value; in
/// Basic's chart, Click over a row's name (not a From row's), Drag over a
/// slider; Undo and Redo by their keys where there is a
/// step to take back or do again; none while a value or a field is typed
/// in, a popover, dialog or menu is open. Its counts: "49 factions" and
/// "44 added by the module".
pub(crate) fn shown(
    app: &Moonglow,
    k: &crate::shell::HintKeys<'_>,
    ctx: &egui::Context,
) -> crate::shell::Shown {
    use crate::keys::Cmd;
    use crate::shell::{Count, Shown};
    use mg_uikit::status::Hint;
    let view = &app.faction_view;
    let mut counts = Vec::new();
    if let Some(Ok(f)) = view.seen.as_ref().map(|s| &s.factions) {
        let n = f.factions.len();
        counts.push(Count::new(format!("{n} factions")));
        counts
            .push(Count::new(format!("{} added by the module", n.saturating_sub(STANDARD.len()))));
    }
    let mut hints = Vec::new();
    let typing = view.typing.is_some()
        || (ctx.memory(|m| m.focused().is_some()) && ctx.egui_wants_keyboard_input());
    let windows = view.adding.is_some() || view.renaming.is_some();
    if crate::shell::keys_blocked(ctx)
        || mg_uikit::popover::Popover::any_open(ctx)
        || typing
        || windows
    {
        return Shown { hints, counts, note: None };
    }
    // (Each for what the pointer is over: in Basic a click on a row's
    // name chooses its reaction, a drag of its slider changes it.)
    let (click, drag, double) = match view.hovered {
        Hovered::Cell => (true, true, true),
        Hovered::Name => (true, false, false),
        Hovered::Slider => (false, true, false),
        Hovered::Nothing => (false, false, false),
    };
    if click {
        hints.push(Hint::keys(["Click"], "Select a reaction"));
    }
    if drag {
        hints.push(Hint::keys(["Drag"], "Change a reaction"));
    }
    if double {
        hints.push(Hint::keys(["Double Click"], "Type a value"));
    }
    let ws = app.ws.as_ref();
    if ws.is_some_and(|w| w.can_undo().is_some()) {
        hints.extend(k.hint(Cmd::Undo, "Undo"));
    }
    if ws.is_some_and(|w| w.can_redo().is_some()) {
        hints.extend(k.hint(Cmd::Redo, "Redo"));
    }
    Shown { hints, counts, note: None }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn removals_are_numbered_back_last_first() {
        // Factions 0 to 8: 5 removed (6 to 8 move down), then 6 (the 7th
        // as it was).
        let back = put_back(&[5, 6]);
        assert_eq!([4, 5, 6].map(|id| back(id).unwrap()), [4, 6, 8]);
    }

    #[test]
    fn failed_apply_during_remove_keeps_cancel_and_selection_untouched() {
        let mut module = mg_module::Module::new();
        let mut factions = Factions::of_module(&module);
        assert_eq!(factions.add("Five", false, 1), 5);
        assert_eq!(factions.add("Six", false, 1), 6);
        module.set_gff(key(), &factions.to_gff()).unwrap();

        let mut git = mg_gff::Gff::new(*b"GIT ");
        let mut creature = mg_gff::Struct::new(4);
        creature.set("FactionID", mg_gff::Value::Word(6));
        git.root.set("Creature List", mg_gff::Value::List(vec![creature]));
        module.set_gff(ResKey::parse("area001", ResType::GIT).unwrap(), &git).unwrap();

        module.project_file.claims.others = mg_module::claims::Others::ReadOnly;
        module.project_file.user.builder_code = Some("ab".into());
        module.project_file.claim_table = Some(
            mg_module::claims::Claims::parse(
                "claims.2da",
                "2DA V2.0\n\nArea Builder\n0 area001 zz\n",
                &mut Vec::new(),
            )
            .unwrap(),
        );

        let mut app = Moonglow::new(None, Box::new(crate::NoDialogs::default()));
        app.ws = Some(mg_edit::Workspace::new(module));
        app.faction_view.selected = 6;
        app.faction_view.target = 6;
        app.faction_view.chosen_names = Some(("Six".into(), "Six".into()));

        remove(&mut app, &factions, 5);

        assert!(app.faction_view.cancel.steps.is_empty());
        assert!(app.faction_view.cancel.renumbered.is_empty());
        assert!(app.properties_changed.is_empty());
        assert_eq!(app.faction_view.selected, 6);
        assert_eq!(app.faction_view.target, 6);
        assert_eq!(app.faction_view.chosen_names, Some(("Six".into(), "Six".into())));
        assert!(
            app.log
                .entries
                .iter()
                .any(|(lvl, msg)| *lvl == crate::Level::Error && msg.contains("read-only")),
            "{:?}",
            app.log.entries
        );
    }
}
