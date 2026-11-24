//! The Outliner (the UI redesign's M3, task T13; ruling R2): under the
//! Left Dock's tabs, split from them by a seam (`shell::left_dock`), what
//! the active editor holds. In M3 the area's: its placed objects by kind
//! (`LightArea_crop_outl.png`), each kind with its count and a show or
//! hide **eye**, which is the area header's Show toggle of the same kind
//! (one state, `AreaView::show`, shared by every area view:
//! `area_view::show_kind`); a kind hidden is quiet; a kind with nothing
//! placed has no arrow. A kind opened lists its objects, each a name and
//! its tag weak after it, cut to the panel's width.
//!
//! Its rows are the module tree's ([`tree::contents_ui`] with
//! [`tree::Look::Outliner`], from the same copy of what the area holds,
//! `Moonglow::area_contents`, read again only where the area's GIT
//! changed: [`tree::fresh_contents`]), and so are what its rows do
//! ([`tree::act_on_object`]): a click selects the object in the area's
//! view (the camera where it is), Ctrl and Shift choose several, a double
//! click opens its Properties, the right-click menu is the tree's; Enter
//! goes to it as the tree's does. Only the rows in sight are laid out. The
//! module tree keeps its areas opened out beside it (ruling R2).
//!
//! The selection in the view is followed: as it changes, the kind of the
//! one selected last opens and its row is brought into sight. Its kinds are
//! closed at first but for the selection's (an area can hold hundreds of
//! one kind); Expand All and Collapse All on its header open and close
//! them all.
//!
//! The keys (a click on a row, F6, the strip's icon): Up and Down from row
//! to row, Page Up and Down, Home and End, Right and Left open and close a
//! kind, Enter goes to an object, letters go to a row by its name, Delete
//! deletes the object (those chosen with it), Escape hands them back, as
//! the module tree's.
//!
//! Beside a conversation (M4's U15, the Conversation board's) it lists the
//! conversation's **Bookmarks** ([`bookmarks`]): its header names the
//! conversation, one group "Bookmarks" with its icon and count holds a row
//! for each bookmarked line ("Entry 40: Welcome, <FullName>. I r…", cut to
//! the panel's width), in the order they were set, each following its line
//! across a delete, its undo and a change made elsewhere
//! (`dialog_view::bookmark_rows`). A click on a row, or Enter, goes to the
//! line in the tree, opening out what hides it. With none, the group is a
//! quiet row with no arrow and its count 0, as an area's kind with nothing
//! placed. A floating conversation brings it into its window, at its left
//! ([`window_ui`]): the main window's Outliner follows the docked editor
//! (M3's T13: a floating window never counts there), and a kind that floats
//! takes what it brings into its window (U2), the Outliner too.

use std::collections::BTreeSet;

use egui::Ui;
use mg_core::ResRef;
use mg_resman::ResKey;

use crate::Moonglow;
use crate::palette_view::{Keys, give_keys, keys_are};
use crate::tree::{self, Asked, Look, Placed, Step, TreeAt};

/// The Outliner's name: the group its header and rows are in, apart from
/// the area header's (whose "Show Creatures" its eye shares a name with).
pub const OUTLINER: &str = "Outliner";

/// The Outliner's header's name, apart from the Outliner's own group.
pub(crate) const OUTLINER_HEADER: &str = "Outliner Header";

/// The Outliner's own state (its rows' open states are egui's).
#[derive(Debug, Default)]
pub(crate) struct Outliner {
    /// The area's view's selection as last seen, with its area.
    seen: Option<(ResRef, Vec<Placed>)>,
    /// That selection as a set, for the rows it marks (made as it changes,
    /// not every frame: every object of a large area can be selected).
    chosen: BTreeSet<Placed>,
    /// The object selected last is still to be brought into sight (its
    /// kind opened; its row once laid out).
    reveal: bool,
    /// The keys' row.
    pub(crate) cursor: Option<TreeAt>,
    /// The keys' row has just moved: brought into sight.
    pub(crate) cursor_moved: bool,
    /// The keys' row goes to the first, once the rows are listed (F6, the
    /// strip's icon, with no row of its own yet).
    pub(crate) to_first: bool,
    /// A kind the keys open or close.
    fold: Option<(TreeAt, bool)>,
    /// Expand All or Collapse All by their keys, for the next frame.
    pub(crate) fold_all: Option<bool>,
    /// The object a Shift+click chooses from (`tree::Anchor`: in its area,
    /// while none of its objects came or went): set by a click here, let
    /// go of as the selection changes elsewhere and with another area
    /// listed.
    anchor: Option<tree::Anchor>,
    /// The area listed last: its rows' state (the keys' row, the anchor,
    /// what the keys open) is let go of with another area in front.
    listed_area: Option<ResRef>,
    /// The list that has the keys (`Keys::Marks`): a conversation's
    /// bookmarks or a table's columns, the Outliner's or a floating
    /// editor's window's.
    pub(crate) marks_keys: Option<Owner>,
    /// The keys' row in that list, with its editor (0 the group's own row,
    /// then each bookmark's or column's): of no other editor.
    marks_cursor: Option<(Owner, usize)>,
    /// The keys' row of this editor's list has just moved: brought into
    /// sight by that list alone (a docked one and a floating one's can
    /// both be drawn).
    pub(crate) marks_moved: Option<Owner>,
    /// The group the keys open or close.
    marks_fold: Option<(Owner, bool)>,
    /// The lists drawn this frame, each with whether it is in a window (in
    /// the order the windows' panes are drawn), and those of the frame
    /// before (F6's stops, the keys left where none is drawn).
    pub(crate) marks_now: Vec<(Owner, bool)>,
    pub(crate) marks_last: Vec<(Owner, bool)>,
    /// The floating editors whose window's Outliner is folded to a strip
    /// at their left this frame (`shell::window_panes`: the tree or the
    /// grid would keep less than its room beside the pane).
    pub(crate) strips_now: std::collections::HashSet<Owner>,
    /// Those whose strip's Outliner is unfolded over the editor.
    pub(crate) strips_open: std::collections::HashSet<Owner>,
    /// Where the strip was drawn this frame, with its editor.
    pub(crate) strip_at: Option<(Owner, egui::Rect)>,
    /// A table's columns as listed (their names, icons and kinds' words),
    /// by its id, with the mark of the table they were made for
    /// (`table_view::panels::columns_mark`) and the load order's count
    /// then (`Moonglow::load_order`: the game's data read again, a hak
    /// added, the module's layer refreshed, whose tables give the game's
    /// kinds): made again only as either changes, not at every frame (a
    /// table can have a hundred columns).
    columns_seen: std::collections::HashMap<u32, ((u64, u64), std::sync::Arc<ColumnsSeen>)>,
}

/// A table's columns as the Outliner lists them ([`Outliner::columns_seen`]).
#[derive(Debug)]
struct ColumnsSeen {
    names: Vec<String>,
    /// Each column's icon and its kind's words (where its kind is known).
    looks: Vec<(mg_uikit::Icon, Option<String>)>,
}

/// The room over the first of the Outliner's rows of a table's columns
/// (the board's 2; the rows are the kit's, 22 high).
const COLUMNS_TOP: f32 = 2.0;

/// The editor whose list the Outliner shows with keys of its own
/// (`Keys::Marks`): a conversation's bookmarks (M4's U15), a table's
/// columns (U16).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Owner {
    Dialog(ResKey),
    Table(u32),
    /// A NUI designer's Layers (M4's U18): its window's key.
    Nui(ResKey),
    /// A tileset's parts (M4's U20): its id.
    Tileset(u32),
    /// A hak's contents by type (M4's U21): its id.
    Hak(u32),
    /// The faction editor's factions (M4's U22).
    Factions,
}

impl Owner {
    /// The editor of a tab that brings such a list into its window.
    pub(crate) fn of(tab: &crate::Tab) -> Option<Owner> {
        match tab {
            crate::Tab::Dialog(key) => Some(Owner::Dialog(*key)),
            crate::Tab::Table(id) => Some(Owner::Table(*id)),
            crate::Tab::Nui(key) => Some(Owner::Nui(*key)),
            crate::Tab::Tileset(id) => Some(Owner::Tileset(*id)),
            crate::Tab::Hak(id) => Some(Owner::Hak(*id)),
            crate::Tab::Factions => Some(Owner::Factions),
            _ => None,
        }
    }

    /// What the Outliner lists for it.
    pub(crate) fn contents(self) -> Contents {
        match self {
            Owner::Dialog(key) => Contents::Bookmarks(key),
            Owner::Table(id) => Contents::Columns(id),
            Owner::Nui(key) => Contents::Layers(key),
            Owner::Tileset(id) => Contents::Tileset(id),
            Owner::Hak(id) => Contents::Hak(id),
            Owner::Factions => Contents::Factions,
        }
    }
}

/// What the Outliner lists, by the active docked editor (the proposal's
/// decision 3, S5): an area's objects (M3's T13), a conversation's
/// bookmarks (M4's U15). Each later editor's Outliner is an arm here.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Contents {
    Area(ResRef),
    Bookmarks(ResKey),
    /// A table's columns (M4's U16): the table's id.
    Columns(u32),
    /// A NUI designer's Layers (M4's U18): its window's key.
    Layers(ResKey),
    /// A tileset's parts (M4's U20): its id.
    Tileset(u32),
    /// A hak's contents by type (M4's U21): its id.
    Hak(u32),
    /// The faction editor's factions (M4's U22).
    Factions,
}

impl Contents {
    /// The editor whose list it is, where it is one with keys of its own
    /// (`Keys::Marks`): not an area's.
    pub(crate) fn owner(self) -> Option<Owner> {
        match self {
            Contents::Area(_) => None,
            Contents::Bookmarks(key) => Some(Owner::Dialog(key)),
            Contents::Columns(id) => Some(Owner::Table(id)),
            Contents::Layers(key) => Some(Owner::Nui(key)),
            Contents::Tileset(id) => Some(Owner::Tileset(id)),
            Contents::Hak(id) => Some(Owner::Hak(id)),
            Contents::Factions => Some(Owner::Factions),
        }
    }
}

/// What the Outliner lists now: the active docked editor's
/// (`shell::editor_brings`), where it brings one.
pub(crate) fn contents(app: &Moonglow) -> Option<Contents> {
    match app.active_editor() {
        Some(crate::Tab::Area(area)) if app.ws.is_some() => Some(Contents::Area(*area)),
        Some(crate::Tab::Dialog(key)) if app.ws.is_some() => Some(Contents::Bookmarks(*key)),
        // (A table's, whatever is open: a table is no module's alone.)
        Some(crate::Tab::Table(id)) => Some(Contents::Columns(*id)),
        // (A NUI designer's Layers, where it brings its panels: its Design
        // page in the Editor view, M4's U18.)
        // (Its source group's too, M4's U19: a part of the designer.)
        Some(crate::Tab::Nui(key) | crate::Tab::NuiSource(key)) if app.ws.is_some() => {
            Some(Contents::Layers(*key))
        }
        // (A tileset's parts, whatever is open: a tileset is a file of its
        // own, M4's U20.)
        Some(crate::Tab::Tileset(id)) => Some(Contents::Tileset(*id)),
        // (A hak's types, whatever is open: a hak is a file of its own,
        // M4's U21.)
        Some(crate::Tab::Hak(id)) => Some(Contents::Hak(*id)),
        // (The module's factions, M4's U22.)
        Some(crate::Tab::Factions) if app.ws.is_some() => Some(Contents::Factions),
        _ => None,
    }
}

/// The keys of the Outliner as it lists now: an area's rows'
/// (`Keys::Outliner`), a conversation's bookmarks' (`Keys::Marks`).
pub(crate) fn keys_of(app: &Moonglow) -> Keys {
    match contents(app) {
        Some(
            Contents::Bookmarks(_)
            | Contents::Columns(_)
            | Contents::Layers(_)
            | Contents::Tileset(_)
            | Contents::Hak(_)
            | Contents::Factions,
        ) => Keys::Marks,
        _ => Keys::Outliner,
    }
}

/// The area a row of the Outliner (or the module tree) is in.
fn area_of(at: TreeAt) -> Option<ResRef> {
    match at {
        TreeAt::Kind(area, _) | TreeAt::Object(area, ..) => Some(area),
        TreeAt::Group(_) | TreeAt::Resource(_) => None,
    }
}

/// The area the Outliner lists: the active editor's, where it is an
/// area's tab (`shell::editor_brings`).
pub(crate) fn area(app: &Moonglow) -> Option<ResRef> {
    match contents(app) {
        Some(Contents::Area(area)) => Some(area),
        _ => None,
    }
}

/// The keys given to the Outliner (Next Pane, the folded strip's icon): at
/// the row they were at, else its first.
pub(crate) fn take_keys(app: &mut Moonglow, ctx: &egui::Context) {
    if let Some(owner) = contents(app).and_then(Contents::owner) {
        take_marks(app, ctx, owner);
        return;
    }
    app.outliner.marks_keys = None;
    give_keys(ctx, Keys::Outliner);
    if app.outliner.cursor.is_none() {
        app.outliner.to_first = true;
    }
    app.outliner.cursor_moved = true;
    ctx.request_repaint();
}

/// The Outliner, under the Left Dock's tabs: its header (its name, the
/// area's, Expand All and Collapse All) over its rows, which scroll; a
/// group of its own, named [`OUTLINER`].
pub(crate) fn ui(app: &mut Moonglow, ui: &mut Ui) {
    match contents(app) {
        Some(Contents::Area(_)) => _ = ui.scope(|ui| panel(app, ui)),
        Some(Contents::Bookmarks(key)) => _ = ui.scope(|ui| bookmarks(app, ui, key, false)),
        Some(Contents::Columns(id)) => _ = ui.scope(|ui| columns(app, ui, id, false)),
        Some(Contents::Layers(key)) => _ = ui.scope(|ui| layers(app, ui, key, false)),
        Some(Contents::Tileset(id)) => _ = ui.scope(|ui| tileset(app, ui, id, false)),
        Some(Contents::Hak(id)) => _ = ui.scope(|ui| hak(app, ui, id, false)),
        Some(Contents::Factions) => _ = ui.scope(|ui| factions(app, ui, false)),
        None => {}
    }
}

/// The Outliner alone in the Left Dock (`shell::outliner_alone`: a
/// table's columns, no module open), the dock's fold button (`fold` its
/// tip) at the end of its header, where the tab bar's would be. Whether
/// the button was clicked.
pub(crate) fn alone(app: &mut Moonglow, ui: &mut Ui, fold: &str) -> bool {
    let id = egui::Id::new(ALONE_FOLD);
    ui.data_mut(|d| d.insert_temp(id, (fold.to_string(), false)));
    self::ui(app, ui);
    ui.data_mut(|d| d.remove_temp::<(String, bool)>(id)).is_some_and(|(_, clicked)| clicked)
}

/// Where [`alone`] keeps the fold button's tip, and whether it was
/// clicked, while the Outliner is drawn.
const ALONE_FOLD: &str = "outliner-alone-fold";

/// The Left Dock's fold button, at the end of the Outliner's header where
/// it is drawn alone in the dock ([`alone`]); not in a window's.
fn alone_fold(ui: &mut Ui, window: bool) {
    let id = egui::Id::new(ALONE_FOLD);
    let Some((tip, _)) = ui.data(|d| d.get_temp::<(String, bool)>(id)).filter(|_| !window) else {
        return;
    };
    let side = mg_uikit::shell::Side::Left;
    if mg_uikit::shell::fold_button_tip(ui, side, crate::shell::LEFT_DOCK, &tip).clicked() {
        ui.data_mut(|d| d.insert_temp(id, (tip, true)));
    }
}

/// The columns of the table at `at` as the Outliner lists them, made again
/// only where the table's columns mark has changed ([`Outliner::columns_seen`]).
fn columns_seen(app: &mut Moonglow, at: usize) -> std::sync::Arc<ColumnsSeen> {
    let doc = &app.tables[at];
    let (id, mark) = (doc.id, (crate::table_view::panels::columns_mark(doc), app.load_order));
    if let Some((was, seen)) = app.outliner.columns_seen.get(&id)
        && *was == mark
    {
        return seen.clone();
    }
    let kinds = crate::table_view::kinds(app, &doc.name());
    let description = doc.description();
    let d = description.as_deref();
    let names: Vec<String> = doc.edit.table().columns().to_vec();
    let looks = (names.iter())
        .map(|c| {
            let icon = crate::table_view::panels::kind_icon(d, &kinds, c);
            (icon, crate::table_view::panels::kind_words(d, &kinds, c))
        })
        .collect();
    let seen = std::sync::Arc::new(ColumnsSeen { names, looks });
    // (Those of tables closed since go.)
    app.outliner.columns_seen.retain(|t, _| app.tables.iter().any(|d| d.id == *t));
    app.outliner.columns_seen.insert(id, (mark, seen.clone()));
    seen
}

/// A floating conversation's Outliner, in its window (`shell::window_panes`,
/// a left pane): its bookmarks, as the main window's Outliner lists those of
/// a docked one.
pub(crate) fn window_ui(app: &mut Moonglow, ui: &mut Ui, owner: Owner) {
    match owner {
        Owner::Dialog(key) => _ = ui.scope(|ui| bookmarks(app, ui, key, true)),
        Owner::Table(id) => _ = ui.scope(|ui| columns(app, ui, id, true)),
        Owner::Nui(key) => _ = ui.scope(|ui| layers(app, ui, key, true)),
        Owner::Tileset(id) => _ = ui.scope(|ui| tileset(app, ui, id, true)),
        Owner::Hak(id) => _ = ui.scope(|ui| hak(app, ui, id, true)),
        Owner::Factions => _ = ui.scope(|ui| factions(app, ui, true)),
    }
}

fn panel(app: &mut Moonglow, ui: &mut Ui) {
    let Some(area) = area(app) else { return };
    let ctx = ui.ctx().clone();
    mg_uikit::name_group(ui, OUTLINER);
    // Another area listed: what was kept of the rows of the one before
    // goes (the keys' row, the anchor, a kind to open, what was typed),
    // so that no key acts on a row of an area not shown; the keys, if
    // they are the Outliner's, at its first row.
    if app.outliner.listed_area != Some(area) {
        let o = &mut app.outliner;
        o.listed_area = Some(area);
        o.anchor = None;
        o.fold = None;
        o.to_first = keys_are(&ctx, Keys::Outliner);
        o.cursor = None;
        ctx.data_mut(|d| d.remove_temp::<(String, f64)>(egui::Id::new("outliner-typed")));
    }
    // (A row of another area kept anyhow: not the keys'.)
    if app.outliner.cursor.and_then(area_of).is_some_and(|a| a != area) {
        app.outliner.cursor = None;
    }
    // The view's selection followed: changed (in the view, or by a row
    // here), the one selected last revealed. (Compared in place, copied
    // only as it changes.)
    let empty: &[Placed] = &[];
    let now = app.area_views.get(&area).map_or(empty, |v| v.selection.as_slice());
    let last = now.last().copied();
    let changed = app.outliner.seen.as_ref().is_none_or(|(a, s)| *a != area || s != now);
    if changed {
        let now = now.to_vec();
        app.outliner.reveal = !now.is_empty();
        // (Selected elsewhere: a Shift+click goes from the one selected
        // last there, not from a row clicked here before.)
        app.outliner.anchor = None;
        // (The keys' row at an object goes to the one selected now, as
        // the module tree's does; nothing selected, as after a delete, it
        // stays where it was put.)
        if matches!(app.outliner.cursor, Some(TreeAt::Object(..)))
            && let Some((kind, index)) = last
        {
            app.outliner.cursor = Some(TreeAt::Object(area, kind, index));
        }
        app.outliner.chosen = now.iter().copied().collect();
        app.outliner.seen = Some((area, now));
    }
    let here = keys_are(&ctx, Keys::Outliner) && tree::keys_free(ui);
    // The header: its name, the area's (as its tab names it), Expand All
    // and Collapse All (and their keys while the keys are the Outliner's).
    let detail = match app.ws.as_ref() {
        Some(ws) => app.area_names.label(ws, app.game.as_deref(), area, app.settings.area_names),
        None => area.to_string(),
    };
    let keymap = &app.keymap;
    let header = mg_uikit::shell::PanelHeader::new(OUTLINER).detail(detail).name(OUTLINER_HEADER);
    let mut fold = header.show(ui, |ui| crate::palette_view::fold_icons(ui, "kind", keymap)).inner;
    fold = fold.or(app.outliner.fold_all.take());
    if here && let Some(asked) = crate::palette_view::fold_keys(ui, &app.keymap) {
        app.outliner.fold_all = Some(asked);
        ctx.request_repaint();
    }
    // (Folded to its header, in a dock too short for its rows and the
    // panel over it: `shell::left_dock`. Its rows come back with the room.)
    if ui.available_height() < mg_uikit::row_height(ui) {
        return;
    }
    let shown = crate::area_view::kinds_shown(app, area);
    let menu_commands = app.plugin_menu_commands_cached();
    let in_hand = last.map(|(kind, index)| (kind, index, app.outliner.reveal));
    let mut keys = tree::ContentKeys {
        cursor: app.outliner.cursor,
        moved: std::mem::take(&mut app.outliner.cursor_moved),
        fold: app.outliner.fold.take(),
        ..Default::default()
    };
    let mut go = None;
    let read_only = crate::claims::read_only(app, area).is_some();
    let Moonglow { ws, game, area_contents, area_names, outliner, .. } = app;
    let chosen = (outliner.chosen.len() > 1).then_some(&outliner.chosen);
    let Some(ws) = ws.as_ref() else { return };
    let contents = tree::fresh_contents(area_contents, area_names, ws, game.as_deref(), area);

    let mut rows =
        egui::ScrollArea::vertical().id_salt("outliner-rows").auto_shrink(false).show(ui, |ui| {
            mg_uikit::list_rows(ui);
            // (Its row's arrow 2 px under the header, as the board has it.)
            ui.add_space(2.0);
            let look = Look::Outliner { shown, fold };
            let commands = &menu_commands;
            tree::contents_ui(
                ui, area, contents, None, &mut go, in_hand, chosen, &mut keys, commands, look,
                read_only,
            )
        });
    let went = rows.inner;
    if went {
        app.outliner.reveal = false;
    } else if app.outliner.reveal {
        // (Its kind's rows are still opening: brought into sight once
        // they are laid out.)
        ctx.request_repaint();
    }
    crate::widgets::home_and_end(ui, &mut rows);
    crate::palette_view::keys_outline(ui, rows.inner_rect, Keys::Outliner);
    // An eye clicked: its kind shown or hidden, in every view.
    if let Some(kind) = keys.eye {
        crate::area_view::show_kind(app, area, kind, !shown[kind.index()]);
    }
    // The rows as listed, for the keys and for Shift's run.
    let mut listed = Vec::new();
    tree::area_key_rows(&mut listed, area, &keys.listed, 0);
    if std::mem::take(&mut app.outliner.to_first) {
        app.outliner.cursor = listed.first().map(|r| r.0);
        app.outliner.cursor_moved = true;
        ctx.request_repaint();
    }
    // A row clicked with Ctrl (added to or taken from those selected) or
    // Shift (those from the one clicked before): selected in the view.
    if let Some((TreeAt::Object(_, kind, index), command, shift)) = keys.picked {
        let objects: Vec<Placed> = (listed.iter())
            .filter_map(|r| match r.0 {
                TreeAt::Object(_, k, i) => Some((k, i)),
                _ => None,
            })
            .collect();
        let mut set = app.outliner.chosen.clone();
        let kept = tree::Anchor::in_area(app.outliner.anchor, &app.area_contents, area);
        let mut anchor = kept.or(last);
        crate::widgets::select_in_list(
            &mut set,
            &mut anchor,
            &objects,
            (kind, index),
            command,
            shift,
        );
        app.outliner.anchor = anchor.map(|a| tree::Anchor::new(&app.area_contents, area, a));
        // (Kinds hidden in the view among them: shown, as a plain click
        // shows its kind.)
        let chosen: Vec<Placed> = set.into_iter().collect();
        tree::show_kinds_of(app, area, &chosen);
        tree::select_in_view(app, area, chosen);
        made_here(app, area);
    }
    if let Some(at) = keys.clicked {
        app.outliner.cursor = Some(at);
        give_keys(&ctx, Keys::Outliner);
    }
    // The keys, while they are the Outliner's (with no row of its own yet
    // too: an area with nothing placed has none, and its Escape is still
    // the Outliner's, as the Status Bar's hints count it).
    if keys_are(&ctx, Keys::Outliner) && tree::keys_free(ui) {
        use egui::Key;
        let arrows = [
            Key::ArrowDown,
            Key::ArrowUp,
            Key::PageDown,
            Key::PageUp,
            Key::Home,
            Key::End,
            Key::ArrowRight,
            Key::ArrowLeft,
            Key::Enter,
        ];
        let pressed =
            ui.input_mut(|i| arrows.into_iter().find(|k| i.consume_key(egui::Modifiers::NONE, *k)));
        if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::Escape)) {
            app.outliner.cursor = None;
            crate::palette_view::give_arrows(&ctx, false);
        }
        // Letters go to the row whose name begins with them.
        let contents = app.area_contents.get(&area).map(|c| &c.1);
        let text = |at: TreeAt| contents.map_or_else(String::new, |c| tree::outliner_text(at, c));
        let seek = |from: Option<TreeAt>, typed: &str| tree::seek_in(&listed, from, typed, &text);
        if let Some(at) =
            tree::typed_row(ui, egui::Id::new("outliner-typed"), app.outliner.cursor, &seek)
        {
            app.outliner.cursor = Some(at);
            app.outliner.cursor_moved = true;
            ctx.request_repaint();
        }
        // Delete at the object under the pointer, else at the keys' row:
        // it (and those selected with it) deleted, the keys on to the row
        // after it.
        // (Only a row listed, and so marked where it is the keys': the
        // one under the pointer, else the keys', which may be of a kind
        // since closed, or gone.)
        let listed_row = |at: &TreeAt| listed.iter().any(|r| r.0 == *at);
        let under = keys.hovered.or(app.outliner.cursor.filter(listed_row));
        if let Some(at @ TreeAt::Object(_, kind, index)) = under
            && ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::Delete))
        {
            app.outliner.anchor = None;
            go = Some((area, kind, index, Asked::Delete));
            keys.together = true;
            // (Refused in a read-only area: the keys stay at the row.)
            if crate::claims::read_only(app, area).is_none() {
                let cursor = app.outliner.cursor;
                app.outliner.cursor = cursor.map(|c| tree::past_deleted_in(&listed, c, at));
            }
            ctx.request_repaint();
        }
        match pressed.map_or(Step::None, |key| tree::step_in(&listed, app.outliner.cursor, key)) {
            Step::To(at) => {
                app.outliner.cursor = Some(at);
                app.outliner.cursor_moved = true;
                ctx.request_repaint();
            }
            Step::Fold(at, open) => {
                app.outliner.fold = Some((at, open));
                ctx.request_repaint();
            }
            Step::Go(area, kind, index) => go = Some((area, kind, index, Asked::Go)),
            Step::Open(_) | Step::None => {}
        }
    }
    // An object's row clicked, or asked of by its menu or the keys.
    if let Some((area, kind, index, asked)) = go {
        if asked == Asked::Select {
            app.outliner.anchor = Some(tree::Anchor::new(&app.area_contents, area, (kind, index)));
        }
        if asked == Asked::Delete {
            app.outliner.anchor = None;
        }
        let chosen = keys.together.then(|| app.outliner.chosen.clone());
        let together = chosen.as_ref();
        tree::act_on_object(app, ui, (area, kind, index, asked), together, &menu_commands, false);
        if matches!(asked, Asked::Select | Asked::Go) {
            made_here(app, area);
        }
    }
}

/// The view's selection made by the Outliner itself (a row clicked, Ctrl
/// or Shift with it, Enter) noted as seen: the row is where the pointer or
/// the keys are, so the Outliner does not scroll to the one selected last
/// (a row cut by its foot and clicked would move from under the pointer,
/// and a double click's second click land on another; a Ctrl+click would
/// scroll to the last of those chosen). Only a selection made elsewhere
/// (in the view, by the module tree) is brought into sight.
fn made_here(app: &mut Moonglow, area: ResRef) {
    let selection = app.area_views.get(&area).map(|v| v.selection.clone()).unwrap_or_default();
    app.outliner.chosen = selection.iter().copied().collect();
    app.outliner.seen = Some((area, selection));
    app.outliner.reveal = false;
}

/// The keys given to conversation `key`'s bookmarks (a click on a row,
/// F6, the strip's icon): at the row they were at, else the group's.
pub(crate) fn take_marks(app: &mut Moonglow, ctx: &egui::Context, key: Owner) {
    give_keys(ctx, Keys::Marks);
    let o = &mut app.outliner;
    o.marks_keys = Some(key);
    if o.marks_cursor.is_none_or(|(k, _)| k != key) {
        o.marks_cursor = Some((key, 0));
    }
    o.marks_moved = Some(key);
    ctx.request_repaint();
}

/// Whether the keys are `key`'s list's (a conversation's bookmarks, a
/// table's columns).
fn marks_have_keys(app: &Moonglow, ctx: &egui::Context, key: Owner) -> bool {
    keys_are(ctx, Keys::Marks) && app.outliner.marks_keys == Some(key)
}

/// The Bookmarks group's name for a screen reader, as the Outliner names
/// an area's kinds ("46 creatures"): "5 bookmarks", "1 bookmark".
fn marks_label(count: usize) -> String {
    match count {
        1 => "1 bookmark".to_string(),
        n => format!("{n} bookmarks"),
    }
}

/// The Bookmarks' group's tip, where it has none.
const NO_BOOKMARKS: &str = "No bookmarks: Bookmark on the Node section's bar marks the line chosen";

/// Conversation `key`'s bookmarks as the Outliner lists them (M4's U15,
/// the Conversation board's left dock): the header, "Outliner" and the
/// conversation's name, over the group "Bookmarks" with its icon and count
/// and a row for each bookmark (`dialog_view::bookmark_rows`), which
/// scroll; in the Left Dock, or in its window where it floats (`window`).
/// A click on a row goes to its line and gives the list the keys: Up and
/// Down from row to row, Page Up and Down, Home and End, Right and Left
/// open and close the group (Left from a row goes to the group), Enter
/// goes to the line (or opens and closes the group), Escape hands them
/// back. A group of its own, named [`OUTLINER`].
fn bookmarks(app: &mut Moonglow, ui: &mut Ui, key: ResKey, window: bool) {
    let ctx = ui.ctx().clone();
    mg_uikit::name_group(ui, OUTLINER);
    let owner = Owner::Dialog(key);
    app.outliner.marks_now.push((owner, window));
    let header = mg_uikit::shell::PanelHeader::new(OUTLINER)
        .detail(key.resref.to_string())
        .name(OUTLINER_HEADER);
    header.show(ui, |_| ());
    // (Folded to its header in a dock too short for its rows.)
    if ui.available_height() < mg_uikit::row_height(ui) {
        return;
    }
    let Some(rows) = crate::dialog_view::bookmark_rows(app, &ctx, key) else { return };
    // (The keys' row of another conversation: none here.)
    let mut cursor = match app.outliner.marks_cursor {
        Some((k, at)) if k == owner => Some(at.min(rows.len())),
        _ => None,
    };
    let here = marks_have_keys(app, &ctx, owner) && tree::keys_free(ui);
    // (Taken by its own list, whichever is drawn first.)
    let moved = here && app.outliner.marks_moved == Some(owner);
    if moved {
        app.outliner.marks_moved = None;
    }
    let fold = match app.outliner.marks_fold.take() {
        Some((k, open)) if k == owner => Some(open),
        other => {
            app.outliner.marks_fold = other;
            None
        }
    };
    let mut clicked = None;
    let mut gone_to = None;
    let mut open = false;
    let scroll =
        egui::ScrollArea::vertical().id_salt(("outliner-marks", key, window)).auto_shrink(false);
    let shown = scroll.show(ui, |ui| {
        mg_uikit::list_rows(ui);
        // (Its row's arrow 2 px under the header, as the board has it.)
        ui.add_space(2.0);
        let count = rows.len();
        let marked = |at: usize| here && cursor == Some(at);
        if rows.is_empty() {
            // (None: a quiet row, no arrow, as an area's kind with nothing
            // placed.)
            let row = mg_uikit::TreeRow::new("Bookmarks")
                .icon(mg_uikit::Icon::Bookmark)
                .count(0)
                .label(marks_label(0))
                .dim()
                .show(ui)
                .on_hover_text(NO_BOOKMARKS);
            if moved && cursor == Some(0) {
                row.scroll_to_me(None);
            }
            return;
        }
        let group = mg_uikit::TreeGroup::new(("outliner-bookmarks", key, window), "Bookmarks")
            .icon(mg_uikit::Icon::Bookmark)
            .count(count)
            .label(marks_label(count))
            .default_open(true)
            .open(fold)
            .marked(marked(0));
        let shown = group.show(ui, |ui| {
            for (i, (line, words)) in rows.iter().enumerate() {
                let row = mg_uikit::TreeRow::new(words.as_str()).fit().selected(marked(i + 1));
                let row = row.show(ui);
                if row.clicked() {
                    clicked = Some(i + 1);
                    gone_to = Some(*line);
                }
                if moved && cursor == Some(i + 1) {
                    row.scroll_to_me(None);
                }
            }
        });
        open = shown.body_returned.is_some();
        if shown.header_response.clicked() {
            clicked = Some(0);
        }
        if moved && cursor == Some(0) {
            shown.header_response.scroll_to_me(None);
        }
    });
    if here {
        crate::palette_view::keys_outline(ui, shown.inner_rect, Keys::Marks);
    }
    if let Some(at) = clicked {
        cursor = Some(at);
        app.outliner.marks_cursor = Some((owner, at));
        app.outliner.marks_keys = Some(owner);
        give_keys(&ctx, Keys::Marks);
    }
    // The keys, while they are this list's.
    if here {
        use egui::Key;
        let keys = [
            Key::ArrowDown,
            Key::ArrowUp,
            Key::PageDown,
            Key::PageUp,
            Key::Home,
            Key::End,
            Key::ArrowRight,
            Key::ArrowLeft,
            Key::Enter,
        ];
        let pressed =
            ui.input_mut(|i| keys.into_iter().find(|k| i.consume_key(egui::Modifiers::NONE, *k)));
        if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::Escape)) {
            app.outliner.marks_cursor = None;
            app.outliner.marks_keys = None;
            crate::palette_view::give_arrows(&ctx, false);
            cursor = None;
        }
        // (The rows listed: the group's, then its bookmarks' while open.)
        let last = if open { rows.len() } else { 0 };
        let page = ((shown.inner_rect.height() / mg_uikit::row_pitch(ui)) as usize).max(1);
        let at = cursor.unwrap_or(0).min(last);
        let to = match pressed {
            Some(Key::ArrowDown) => Some((at + 1).min(last)),
            Some(Key::ArrowUp) => Some(at.saturating_sub(1)),
            Some(Key::PageDown) => Some((at + page).min(last)),
            Some(Key::PageUp) => Some(at.saturating_sub(page)),
            Some(Key::Home) => Some(0),
            Some(Key::End) => Some(last),
            Some(Key::ArrowRight) if at == 0 && !open && !rows.is_empty() => {
                app.outliner.marks_fold = Some((owner, true));
                Some(0)
            }
            Some(Key::ArrowRight) if at == 0 && open && !rows.is_empty() => Some(1),
            Some(Key::ArrowLeft) if at == 0 && open => {
                app.outliner.marks_fold = Some((owner, false));
                Some(0)
            }
            Some(Key::ArrowLeft) => Some(0),
            Some(Key::Enter) if at == 0 && !rows.is_empty() => {
                app.outliner.marks_fold = Some((owner, !open));
                Some(0)
            }
            // (A bookmark's row; on the quiet row of none, nothing.)
            Some(Key::Enter) => {
                gone_to = at.checked_sub(1).and_then(|i| rows.get(i)).map(|r| r.0);
                Some(at)
            }
            _ => None,
        };
        // Letters go to the bookmark whose line's text begins with them,
        // as the Outliner's rows of an area go by their names.
        let seek = |from: Option<usize>, typed: &str| {
            let texts: Vec<String> = (rows.iter())
                .map(|(_, words)| words.split_once(": ").map_or(words.as_str(), |w| w.1))
                .map(mg_core::text::fold)
                .collect();
            // (Its rows' places from 1, after the group's own row.)
            // (From the group's own row, or none: from the first, itself
            // first.)
            let last = texts.len().saturating_sub(1);
            let from = from.filter(|at| *at > 0).map_or(last, |at| at - 1);
            let found = crate::dialog_view::type_ahead(&texts, from, &mg_core::text::fold(typed))?;
            Some(found + 1)
        };
        let typed = (open && !rows.is_empty())
            .then(|| {
                crate::tree::typed_row(
                    ui,
                    egui::Id::new(("outliner-marks-typed", key)),
                    cursor,
                    &seek,
                )
            })
            .flatten();
        let to = to.or(typed);
        if let Some(to) = to {
            app.outliner.marks_cursor = Some((owner, to));
            app.outliner.marks_moved = Some(owner);
            ctx.request_repaint();
        }
    }
    if let Some(line) = gone_to {
        crate::dialog_view::go_to_bookmark(app, &ctx, key, line);
    }
}

/// NUI designer `key`'s Layers as the Outliner lists them (M4's U18, the
/// NUI board's Left Dock): `nui_view::panels::layers`, a group of its own,
/// named [`OUTLINER`]. In the Left Dock, or in its window where it floats
/// (`window`).
fn layers(app: &mut Moonglow, ui: &mut Ui, key: ResKey, window: bool) {
    mg_uikit::name_group(ui, OUTLINER);
    crate::nui_view::panels::layers(app, ui, key, window);
}

/// Tileset `id`'s parts as the Outliner lists them (M4's U20, the Tileset
/// board's Left Dock): the header, "Outliner" and the tileset's name and
/// the name it shows, with Expand All and Collapse All; General, then
/// Terrains, Crossers, Primary rules, Secondary rules, Tiles and Groups,
/// each with its icon and count (a group with none quiet, no arrow), each
/// opened to list its parts (a terrain's or crosser's name, a rule, a tile
/// by its number and model, a group by its number, name and size); only
/// the rows in sight laid out. A click on a row goes to its page and its
/// row (the tile or group chosen) and gives the list the keys: Up, Down,
/// Page Up, Page Down, Home and End from row to row, Right and Left open
/// and close a group (Left from a part goes to its group), Enter goes to
/// the row, letters to a row by its words, Escape hands them back. In the
/// Left Dock, or in its window where it floats (`window`). A group of its
/// own, named [`OUTLINER`].
fn tileset(app: &mut Moonglow, ui: &mut Ui, id: u32, window: bool) {
    use crate::tileset_view::{OUTLINE_GROUPS, OutlineRow, outline_counts, part_words};
    let ctx = ui.ctx().clone();
    mg_uikit::name_group(ui, OUTLINER);
    let owner = Owner::Tileset(id);
    app.outliner.marks_now.push((owner, window));
    let Some(at) = app.tilesets.iter().position(|d| d.id == id) else { return };
    crate::side_doc::note_pointer(app, ui, &crate::Tab::Tileset(id));
    let doc = &app.tilesets[at];
    let set = doc.shared();
    let detail = match &set {
        Some(t) => {
            let shown =
                t.general.unlocalized_name.clone().filter(|n| !n.is_empty()).or_else(|| {
                    let game = app.game.as_deref()?;
                    (!t.general.display_name.is_none())
                        .then(|| game.string(t.general.display_name))
                        .flatten()
                });
            // (Named by its file, the board's "tcn01 City Exterior".)
            let stem = doc.path.file_stem().map(|s| s.to_string_lossy().to_lowercase());
            let stem = stem.unwrap_or_else(|| t.general.name.to_lowercase());
            match shown {
                Some(n) if !n.eq_ignore_ascii_case(&stem) => format!("{stem} {n}"),
                _ => stem,
            }
        }
        None => doc.title(),
    };
    let header = mg_uikit::shell::PanelHeader::new(OUTLINER).detail(detail).name(OUTLINER_HEADER);
    let folded_all = header
        .show(ui, |ui| {
            let mut asked = None;
            let expand =
                mg_uikit::IconButton::new(mg_uikit::Icon::ExpandAll, "Expand All").icon_only(true);
            if ui.add(expand).on_hover_text("Open every group").clicked() {
                asked = Some(true);
            }
            let collapse = mg_uikit::IconButton::new(mg_uikit::Icon::CollapseAll, "Collapse All")
                .icon_only(true);
            if ui.add(collapse).on_hover_text("Close every group").clicked() {
                asked = Some(false);
            }
            alone_fold(ui, window);
            asked
        })
        .inner;
    if let Some(open) = folded_all {
        app.tilesets[at].outline_open = [open; 6];
    }
    // (Folded to its header in a dock too short for its rows.)
    if ui.available_height() < mg_uikit::row_height(ui) {
        return;
    }
    let Some(set) = set else { return };
    let doc = &app.tilesets[at];
    let rows = crate::tileset_view::outline_rows(doc);
    let counts = outline_counts(&set);
    let open = doc.outline_open;
    // (The row of what the editor has chosen: its tile or group, where
    // their groups are open.)
    // (Its place found by the groups' counts, not by a walk of the rows.)
    let current = match doc.page {
        crate::tileset_view::Page::Tiles => doc.tile.map(|t| OutlineRow::Part(4, t)),
        crate::tileset_view::Page::Groups => doc.group.map(|g| OutlineRow::Part(5, g)),
        _ => None,
    }
    .and_then(|r| crate::tileset_view::outline_index(r, &open, &counts));
    let mut cursor = match app.outliner.marks_cursor {
        Some((k, at)) if k == owner => Some(at.min(rows.len().saturating_sub(1))),
        _ => None,
    };
    let here = marks_have_keys(app, &ctx, owner) && tree::keys_free(ui);
    if marks_have_keys(app, &ctx, owner) {
        app.undo_keys = Some(crate::Tab::Tileset(id));
    }
    let moved = here && app.outliner.marks_moved == Some(owner);
    if moved {
        app.outliner.marks_moved = None;
    }
    let mut clicked = None;
    let mut toggled = None;
    let scroll =
        egui::ScrollArea::vertical().id_salt(("outliner-tileset", id, window)).auto_shrink(false);
    let shown = scroll.show_viewport(ui, |ui, view| {
        mg_uikit::list_rows(ui);
        let pitch = mg_uikit::row_pitch(ui);
        let count = rows.len();
        ui.set_height(COLUMNS_TOP + count as f32 * pitch);
        let first = (((view.min.y - COLUMNS_TOP) / pitch).floor().max(0.0) as usize).min(count);
        let last = ((((view.max.y - COLUMNS_TOP) / pitch).ceil().max(0.0) as usize) + 1).min(count);
        let top = ui.max_rect().top() + COLUMNS_TOP;
        if moved && let Some(c) = cursor {
            let y = top + c as f32 * pitch;
            let row = egui::Rect::from_x_y_ranges(ui.max_rect().x_range(), y..=y + pitch);
            ui.scroll_to_rect(row, None);
        }
        let indent = ui.spacing().indent;
        for (n, row) in rows.iter().enumerate().take(last).skip(first) {
            let y = top + n as f32 * pitch;
            let mut rect = egui::Rect::from_x_y_ranges(ui.max_rect().x_range(), y..=y + pitch);
            if matches!(row, OutlineRow::Part(..)) {
                rect.min.x += indent;
            }
            let mut ui =
                ui.new_child(egui::UiBuilder::new().max_rect(rect).id_salt(("outline-row", n)));
            let marked = (here && cursor == Some(n)) || (!here && current == Some(n));
            match *row {
                OutlineRow::General => {
                    let r = mg_uikit::TreeRow::new("General")
                        .icon(mg_uikit::Icon::Info)
                        .label("General settings")
                        .selected(marked)
                        .show(&mut ui);
                    if r.clicked() {
                        clicked = Some(n);
                    }
                }
                OutlineRow::Group(g) => {
                    let (name, icon, word, _) = OUTLINE_GROUPS[g];
                    let mut tree_row = mg_uikit::TreeRow::new(name)
                        .icon(icon)
                        .count(counts[g])
                        .label(crate::problems::count(counts[g], word))
                        .selected(marked);
                    if counts[g] == 0 {
                        tree_row = tree_row.dim();
                    } else {
                        tree_row = tree_row.arrow_apart();
                    }
                    let shown = tree_row.show_parts(&mut ui);
                    if counts[g] > 0 {
                        let icon = match open[g] {
                            true => mg_uikit::Icon::ChevronDown,
                            false => mg_uikit::Icon::ChevronRight,
                        };
                        let color = mg_uikit::tokens(&ctx).text_2;
                        let r = shown
                            .arrow
                            .shrink2(egui::vec2(0.0, (shown.arrow.height() - 14.0) / 2.0));
                        let r = egui::Rect::from_center_size(r.center(), egui::vec2(14.0, 14.0));
                        icon.paint(ui.painter(), r, color);
                    }
                    // (Its arrow a button of its own: it opens and closes
                    // the group; the row goes to its page.)
                    let arrow = (counts[g] > 0).then(|| {
                        let id = ui.id().with(("outline-arrow", g));
                        let r = ui.interact(shown.arrow, id, egui::Sense::click());
                        let said = match open[g] {
                            true => format!("Close {name}"),
                            false => format!("Open {name}"),
                        };
                        r.widget_info(|| {
                            egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &said)
                        });
                        r
                    });
                    if arrow.as_ref().is_some_and(egui::Response::clicked) {
                        toggled = Some(g);
                    } else if shown.response.clicked() {
                        clicked = Some(n);
                    }
                }
                OutlineRow::Part(g, p) => {
                    let words = part_words(&set, g, p);
                    // (Named with its kind, "Terrain: Grass", "Tile 16:
                    // tcn01_c02_01": apart from the list's and the
                    // boxes' words of the same.)
                    let said = crate::tileset_view::part_label(&set, g, p);
                    let r = mg_uikit::TreeRow::new(words.as_str())
                        .label(said)
                        .fit()
                        .selected(marked)
                        .show(&mut ui);
                    if r.clicked() {
                        clicked = Some(n);
                    }
                }
            }
        }
    });
    if here {
        crate::palette_view::keys_outline(ui, shown.inner_rect, Keys::Marks);
    }
    let mut go = None;
    if let Some(n) = clicked {
        cursor = Some(n);
        app.outliner.marks_cursor = Some((owner, n));
        app.outliner.marks_keys = Some(owner);
        give_keys(&ctx, Keys::Marks);
        go = Some(n);
    }
    if let Some(g) = toggled {
        let o = &mut app.tilesets[at].outline_open;
        o[g] = !o[g];
        ctx.request_repaint();
    }
    if here {
        use egui::Key;
        let keys = [
            Key::ArrowDown,
            Key::ArrowUp,
            Key::PageDown,
            Key::PageUp,
            Key::Home,
            Key::End,
            Key::ArrowRight,
            Key::ArrowLeft,
            Key::Enter,
        ];
        let pressed =
            ui.input_mut(|i| keys.into_iter().find(|k| i.consume_key(egui::Modifiers::NONE, *k)));
        if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::Escape)) {
            app.outliner.marks_cursor = None;
            app.outliner.marks_keys = None;
            crate::palette_view::give_arrows(&ctx, false);
            cursor = None;
        }
        let last = rows.len().saturating_sub(1);
        let page = ((shown.inner_rect.height() / mg_uikit::row_pitch(ui)) as usize).max(1);
        let at_row = cursor.unwrap_or(0).min(last);
        let row = rows.get(at_row).copied();
        let to = match (pressed, row) {
            (Some(Key::ArrowDown), _) => Some((at_row + 1).min(last)),
            (Some(Key::ArrowUp), _) => Some(at_row.saturating_sub(1)),
            (Some(Key::PageDown), _) => Some((at_row + page).min(last)),
            (Some(Key::PageUp), _) => Some(at_row.saturating_sub(page)),
            (Some(Key::Home), _) => Some(0),
            (Some(Key::End), _) => Some(last),
            (Some(Key::ArrowRight), Some(OutlineRow::Group(g))) if counts[g] > 0 => match open[g] {
                false => toggled_now(app, at, g, true),
                true => return_at(at_row + 1),
            },
            (Some(Key::ArrowLeft), Some(OutlineRow::Group(g))) if open[g] => {
                toggled_now(app, at, g, false)
            }
            (Some(Key::ArrowLeft), Some(OutlineRow::Part(g, _))) => {
                rows.iter().position(|r| *r == OutlineRow::Group(g))
            }
            (Some(Key::Enter), _) => {
                go = Some(at_row);
                Some(at_row)
            }
            _ => None,
        };
        // Letters go to the row whose words begin with them (a part's
        // number left out).
        // (The rows' words made only as letters are typed: a tileset's
        // thousands of tiles each frame the list has the keys.)
        let texts = || -> Vec<String> {
            rows.iter()
                .map(|r| match *r {
                    OutlineRow::General => "general".to_string(),
                    OutlineRow::Group(g) => mg_core::text::fold(OUTLINE_GROUPS[g].0),
                    OutlineRow::Part(g, p) => {
                        let w = part_words(&set, g, p);
                        let w = match g {
                            2..=5 => {
                                w.split_once(' ').map_or(w.clone(), |(_, rest)| rest.to_string())
                            }
                            _ => w,
                        };
                        mg_core::text::fold(&w)
                    }
                })
                .collect()
        };
        let seek = |from: Option<usize>, typed: &str| {
            let texts = texts();
            let from = from.unwrap_or(texts.len().saturating_sub(1));
            crate::dialog_view::type_ahead(&texts, from, &mg_core::text::fold(typed))
        };
        let typed = (!rows.is_empty())
            .then(|| {
                tree::typed_row(ui, egui::Id::new(("outliner-tileset-typed", id)), cursor, &seek)
            })
            .flatten();
        if let Some(to) = to.or(typed).filter(|_| !rows.is_empty()) {
            app.outliner.marks_cursor = Some((owner, to));
            app.outliner.marks_moved = Some(owner);
            ctx.request_repaint();
        }
    }
    if let Some(n) = go
        && let Some(row) = rows.get(n).copied()
    {
        let d = &mut app.tilesets[at];
        use crate::tileset_view::Page;
        match row {
            OutlineRow::General => d.page = Page::General,
            OutlineRow::Group(g) => d.page = OUTLINE_GROUPS[g].3,
            OutlineRow::Part(g, p) => {
                d.page = OUTLINE_GROUPS[g].3;
                match g {
                    4 => {
                        d.tile = Some(p);
                        if !d.filter.is_empty() {
                            d.filter.clear();
                        }
                    }
                    5 => d.group = Some(p),
                    _ => {}
                }
            }
        }
        ctx.request_repaint();
    }
}

/// Hak `id`'s contents by type as the Outliner lists them (M4's U21, the
/// Hak board's Left Dock): the header, "Outliner" and the hak's file's
/// name; a row for each type it holds, the most first, its icon, its
/// extension (".mdl") and how many; only the rows in sight laid out. The
/// row of the type Find holds (".mdl") is chosen. A click on a row finds
/// its type (the board's: Find ".mdl", the table that type's alone), again
/// shows every type, and gives the list the keys: Up, Down, Page Up, Page
/// Down, Home and End from row to row, Enter finding the type, letters to a
/// type by its extension, Escape handing them back. In the Left Dock, or in
/// its window where it floats (`window`). A group of its own, named
/// [`OUTLINER`].
fn hak(app: &mut Moonglow, ui: &mut Ui, id: u32, window: bool) {
    let ctx = ui.ctx().clone();
    mg_uikit::name_group(ui, OUTLINER);
    let owner = Owner::Hak(id);
    app.outliner.marks_now.push((owner, window));
    let Some(facts) = crate::hak_view::outline_types(app, id) else { return };
    let Some(at) = app.haks.iter().position(|d| d.id == id) else { return };
    crate::side_doc::note_pointer(app, ui, &crate::Tab::Hak(id));
    let detail = app.haks[at].title().trim_end_matches(" *").to_string();
    let header = mg_uikit::shell::PanelHeader::new(OUTLINER).detail(detail).name(OUTLINER_HEADER);
    header.show(ui, |ui| alone_fold(ui, window));
    // (Folded to its header in a dock too short for its rows.)
    if ui.available_height() < mg_uikit::row_height(ui) {
        return;
    }
    let types = &facts.types;
    let needle = mg_core::text::Needle::new(&app.haks[at].filter);
    let found = needle.as_str();
    let current = found.strip_prefix('.').and_then(|ext| {
        types.iter().position(|(t, _)| match t.extension() {
            Some(e) => ext == e,
            None => ext.strip_prefix('#').is_some_and(|n| n.parse::<u16>().ok() == Some(t.0)),
        })
    });
    let mut cursor = match app.outliner.marks_cursor {
        Some((k, at)) if k == owner => Some(at.min(types.len().saturating_sub(1))),
        _ => None,
    };
    let here = marks_have_keys(app, &ctx, owner) && tree::keys_free(ui);
    // (Its types with the keys: Undo and Redo are the hak's.)
    if marks_have_keys(app, &ctx, owner) {
        app.undo_keys = Some(crate::Tab::Hak(id));
    }
    let moved = here && app.outliner.marks_moved == Some(owner);
    if moved {
        app.outliner.marks_moved = None;
    }
    let mut clicked = None;
    let scroll =
        egui::ScrollArea::vertical().id_salt(("outliner-hak", id, window)).auto_shrink(false);
    // (Only the rows in sight are laid out, as `ScrollArea::show_rows`
    // does, under the list's 2 points of room.)
    let rows = scroll.show_viewport(ui, |ui, view| {
        mg_uikit::list_rows(ui);
        let pitch = mg_uikit::row_pitch(ui);
        let count = types.len();
        ui.set_height(COLUMNS_TOP + count as f32 * pitch);
        let first = (((view.min.y - COLUMNS_TOP) / pitch).floor().max(0.0) as usize).min(count);
        let last = ((((view.max.y - COLUMNS_TOP) / pitch).ceil().max(0.0) as usize) + 1).min(count);
        let top = ui.max_rect().top() + COLUMNS_TOP;
        if moved && let Some(c) = cursor {
            let y = top + c as f32 * pitch;
            let row = egui::Rect::from_x_y_ranges(ui.max_rect().x_range(), y..=y + pitch);
            ui.scroll_to_rect(row, None);
        }
        let shown_rows = egui::Rect::from_x_y_ranges(
            ui.max_rect().x_range(),
            top + first as f32 * pitch..=top + last as f32 * pitch,
        );
        let ui = &mut ui.new_child(egui::UiBuilder::new().max_rect(shown_rows));
        ui.skip_ahead_auto_ids(first);
        for (n, (t, count)) in types.iter().enumerate().take(last).skip(first) {
            let words = format!(".{t}");
            let said = format!("{words}, {}", crate::problems::count(*count, "resource"));
            let r = mg_uikit::TreeRow::new(words.as_str())
                .kind_icon(crate::browser::kind_icon(*t))
                .count(*count)
                .label(said)
                .fit()
                .selected((here && cursor == Some(n)) || (!here && current == Some(n)))
                .show(ui);
            let r = r.on_hover_text(format!(
                "{}: find them in the table",
                crate::hak_view::kind_words(*t)
            ));
            if r.clicked() {
                clicked = Some(n);
            }
        }
    });
    if here {
        crate::palette_view::keys_outline(ui, rows.inner_rect, Keys::Marks);
    }
    let mut go = None;
    if let Some(n) = clicked {
        cursor = Some(n);
        app.outliner.marks_cursor = Some((owner, n));
        app.outliner.marks_keys = Some(owner);
        give_keys(&ctx, Keys::Marks);
        go = Some(n);
    }
    if here {
        use egui::Key;
        let keys = [
            Key::ArrowDown,
            Key::ArrowUp,
            Key::PageDown,
            Key::PageUp,
            Key::Home,
            Key::End,
            Key::Enter,
        ];
        let pressed =
            ui.input_mut(|i| keys.into_iter().find(|k| i.consume_key(egui::Modifiers::NONE, *k)));
        if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::Escape)) {
            app.outliner.marks_cursor = None;
            app.outliner.marks_keys = None;
            crate::palette_view::give_arrows(&ctx, false);
            cursor = None;
        }
        let last = types.len().saturating_sub(1);
        let page = ((rows.inner_rect.height() / mg_uikit::row_pitch(ui)) as usize).max(1);
        let at_row = cursor.unwrap_or(0).min(last);
        let to = match pressed {
            Some(Key::ArrowDown) => Some((at_row + 1).min(last)),
            Some(Key::ArrowUp) => Some(at_row.saturating_sub(1)),
            Some(Key::PageDown) => Some((at_row + page).min(last)),
            Some(Key::PageUp) => Some(at_row.saturating_sub(page)),
            Some(Key::Home) => Some(0),
            Some(Key::End) => Some(last),
            Some(Key::Enter) => {
                go = Some(at_row);
                Some(at_row)
            }
            _ => None,
        };
        // Letters go to the type whose extension begins with them.
        let seek = |from: Option<usize>, typed: &str| {
            let texts: Vec<String> = types.iter().map(|(t, _)| t.to_string()).collect();
            let from = from.unwrap_or(texts.len().saturating_sub(1));
            crate::dialog_view::type_ahead(&texts, from, &mg_core::text::fold(typed))
        };
        let typed = (!types.is_empty())
            .then(|| tree::typed_row(ui, egui::Id::new(("outliner-hak-typed", id)), cursor, &seek))
            .flatten();
        if let Some(to) = to.or(typed).filter(|_| !types.is_empty()) {
            app.outliner.marks_cursor = Some((owner, to));
            app.outliner.marks_moved = Some(owner);
            ctx.request_repaint();
        }
    }
    // A type gone to: Find holds it (its rows alone in the table), what it
    // held kept; or, where it held that type, what it held before put back
    // (the review's ruling: typed over meanwhile, Find is the builder's own,
    // `hak_view::follow_all`).
    if let Some((t, _)) = go.and_then(|n| types.get(n)) {
        let same = found.strip_prefix('.').is_some_and(|ext| match t.extension() {
            Some(e) => ext == e,
            None => ext.strip_prefix('#').is_some_and(|n| n.parse::<u16>().ok() == Some(t.0)),
        });
        let d = &mut app.haks[at];
        if same {
            d.filter = d.find_before.take().unwrap_or_default();
            d.type_find = None;
        } else {
            let words = format!(".{t}");
            if d.type_find.is_none() {
                d.find_before = Some(std::mem::take(&mut d.filter));
            }
            d.filter = words.clone();
            d.type_find = Some(words);
        }
        ctx.request_repaint();
    }
}

/// The module's factions as the Outliner lists them (M4's U22, the
/// Factions board's Left Dock): the header, "Outliner", "Factions" and how
/// many; a row for each faction, the chosen one chosen; only the rows in
/// sight laid out. A click on a row chooses its faction and gives the list
/// the keys: Up, Down, Page Up, Page Down, Home and End from row to row,
/// each choosing its faction, letters to a faction by its name, Escape
/// handing them back. In the Left Dock, or in its window where it floats
/// (`window`). A group of its own, named [`OUTLINER`].
fn factions(app: &mut Moonglow, ui: &mut Ui, window: bool) {
    let ctx = ui.ctx().clone();
    mg_uikit::name_group(ui, OUTLINER);
    let owner = Owner::Factions;
    app.outliner.marks_now.push((owner, window));
    let Some((names, chosen)) = crate::faction_view::outline(app) else { return };
    let header =
        mg_uikit::shell::PanelHeader::new(OUTLINER).detail("Factions").name(OUTLINER_HEADER);
    header.show(ui, |ui| {
        alone_fold(ui, window);
        ui.add(mg_uikit::Badge::count(names.len()));
    });
    // (Folded to its header in a dock too short for its rows.)
    if ui.available_height() < mg_uikit::row_height(ui) {
        return;
    }
    let here = marks_have_keys(app, &ctx, owner) && tree::keys_free(ui);
    let cursor = Some((chosen as usize).min(names.len().saturating_sub(1)));
    // (The chosen faction brought into sight where it was chosen elsewhere:
    // the matrix, the Reactions panel, an Add.)
    let seen_id = egui::Id::new(("outliner-factions-chosen", window));
    let elsewhere = ctx.data(|d| d.get_temp::<u32>(seen_id)) != Some(chosen);
    if elsewhere {
        ctx.data_mut(|d| d.insert_temp(seen_id, chosen));
    }
    let moved = (here && app.outliner.marks_moved == Some(owner)) || elsewhere;
    if moved {
        app.outliner.marks_moved = None;
    }
    let mut clicked = None;
    let scroll =
        egui::ScrollArea::vertical().id_salt(("outliner-factions", window)).auto_shrink(false);
    // (Only the rows in sight are laid out, as `ScrollArea::show_rows`
    // does, under the list's 2 points of room.)
    let rows = scroll.show_viewport(ui, |ui, view| {
        mg_uikit::list_rows(ui);
        let pitch = mg_uikit::row_pitch(ui);
        let count = names.len();
        ui.set_height(COLUMNS_TOP + count as f32 * pitch);
        let first = (((view.min.y - COLUMNS_TOP) / pitch).floor().max(0.0) as usize).min(count);
        let last = ((((view.max.y - COLUMNS_TOP) / pitch).ceil().max(0.0) as usize) + 1).min(count);
        let top = ui.max_rect().top() + COLUMNS_TOP;
        if moved && let Some(c) = cursor {
            let y = top + c as f32 * pitch;
            let row = egui::Rect::from_x_y_ranges(ui.max_rect().x_range(), y..=y + pitch);
            ui.scroll_to_rect(row, None);
        }
        let shown_rows = egui::Rect::from_x_y_ranges(
            ui.max_rect().x_range(),
            top + first as f32 * pitch..=top + last as f32 * pitch,
        );
        let ui = &mut ui.new_child(egui::UiBuilder::new().max_rect(shown_rows));
        ui.skip_ahead_auto_ids(first);
        for (n, name) in names.iter().enumerate().take(last).skip(first) {
            let r = mg_uikit::TreeRow::new(name.as_str())
                .icon(mg_uikit::Icon::Faction)
                .fit()
                .selected(cursor == Some(n))
                .show(ui);
            if r.clicked() {
                clicked = Some(n);
            }
        }
    });
    if here {
        crate::palette_view::keys_outline(ui, rows.inner_rect, Keys::Marks);
    }
    let mut go = None;
    if let Some(n) = clicked {
        app.outliner.marks_cursor = Some((owner, n));
        app.outliner.marks_keys = Some(owner);
        give_keys(&ctx, Keys::Marks);
        go = Some(n);
    }
    if here {
        use egui::Key;
        let keys = [Key::ArrowDown, Key::ArrowUp, Key::PageDown, Key::PageUp, Key::Home, Key::End];
        let pressed =
            ui.input_mut(|i| keys.into_iter().find(|k| i.consume_key(egui::Modifiers::NONE, *k)));
        if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::Escape)) {
            app.outliner.marks_cursor = None;
            app.outliner.marks_keys = None;
            crate::palette_view::give_arrows(&ctx, false);
        }
        let last = names.len().saturating_sub(1);
        let page = ((rows.inner_rect.height() / mg_uikit::row_pitch(ui)) as usize).max(1);
        let at = (chosen as usize).min(last);
        let to = match pressed {
            Some(Key::ArrowDown) => Some((at + 1).min(last)),
            Some(Key::ArrowUp) => Some(at.saturating_sub(1)),
            Some(Key::PageDown) => Some((at + page).min(last)),
            Some(Key::PageUp) => Some(at.saturating_sub(page)),
            Some(Key::Home) => Some(0),
            Some(Key::End) => Some(last),
            _ => None,
        };
        // Letters go to the faction whose name begins with them (the names
        // folded only as letters are typed, not at each frame).
        let seek = |from: Option<usize>, typed: &str| {
            let texts: Vec<String> = names.iter().map(|n| mg_core::text::fold(n)).collect();
            let from = from.unwrap_or(texts.len().saturating_sub(1));
            crate::dialog_view::type_ahead(&texts, from, &mg_core::text::fold(typed))
        };
        let typed = (!names.is_empty())
            .then(|| tree::typed_row(ui, egui::Id::new("outliner-factions-typed"), cursor, &seek))
            .flatten();
        if let Some(to) = to.or(typed).filter(|_| !names.is_empty()) {
            app.outliner.marks_cursor = Some((owner, to));
            app.outliner.marks_moved = Some(owner);
            go = Some(to);
        }
    }
    if let Some(n) = go {
        crate::faction_view::choose(app, n as u32);
        ctx.request_repaint();
    }
}

/// A tileset's Outliner group `g` opened (or closed) by the keys: the keys'
/// row stays on it.
fn toggled_now(app: &mut Moonglow, at: usize, g: usize, open: bool) -> Option<usize> {
    app.tilesets[at].outline_open[g] = open;
    let d = &app.tilesets[at];
    crate::tileset_view::outline_rows(d)
        .iter()
        .position(|r| *r == crate::tileset_view::OutlineRow::Group(g))
}

/// The keys' row moved to `at`.
fn return_at(at: usize) -> Option<usize> {
    Some(at)
}

/// The Columns' tip on a column's eye, shown or hidden (the Table board's).
const HIDE_TIP: &str =
    "Hide Column: hide it while the table is open. Its cells are kept and saved.";
const SHOW_TIP: &str = "Show the column again";

/// Table `id`'s columns as the Outliner lists them (M4's U16, the Table
/// board's Left Dock): the header, "Outliner" and how many columns, with
/// Show Hidden Columns' eye at its end; a row for each column in its order,
/// its kind's icon, its name and its kind's words weak after it (the
/// table's description's, else what the checks know of the game's table
/// of its name), cut to the panel's width, and an eye that hides it as its
/// menu's Hide Column does (and shows it again); a hidden column's row is
/// quiet. A click on a row goes to the column (the selected row's cell, or
/// the first row's) and gives the list the keys: Up, Down, Page Up, Page
/// Down, Home and End, Enter going to the column, letters to a column by
/// its name, Escape handing them back. In the Left Dock, or in its window
/// where it floats (`window`). A group of its own, named [`OUTLINER`].
fn columns(app: &mut Moonglow, ui: &mut Ui, id: u32, window: bool) {
    let ctx = ui.ctx().clone();
    mg_uikit::name_group(ui, OUTLINER);
    let owner = Owner::Table(id);
    app.outliner.marks_now.push((owner, window));
    let Some(at) = app.tables.iter().position(|d| d.id == id) else { return };
    let seen = columns_seen(app, at);
    let names = &seen.names;
    let doc = &app.tables[at];
    let hidden_count = doc.hidden.len();
    let header = mg_uikit::shell::PanelHeader::new(OUTLINER)
        .detail(match names.len() {
            1 => "1 column".to_string(),
            n => format!("{n} columns"),
        })
        .name(OUTLINER_HEADER);
    let show_all = header
        .show(ui, |ui| {
            let words = crate::table_view::show_hidden_words(hidden_count);
            let tip = match hidden_count {
                0 => "No column is hidden".to_string(),
                _ => format!("{words}: bring back every column hidden from view"),
            };
            let shown = ui.add_enabled_ui(hidden_count > 0, |ui| {
                let b =
                    mg_uikit::IconButton::new(mg_uikit::Icon::Eye, crate::table_view::SHOW_HIDDEN)
                        .icon_only(true);
                ui.add(b).on_hover_text(tip).clicked()
            });
            alone_fold(ui, window);
            shown.inner
        })
        .inner;
    if show_all {
        app.tables[at].hidden.clear();
    }
    // (Folded to its header in a dock too short for its rows.)
    if ui.available_height() < mg_uikit::row_height(ui) {
        return;
    }
    let doc = &app.tables[at];
    let current = doc.selected.map(|s| s.1);
    let shown: Vec<bool> = if doc.hidden.is_empty() {
        vec![true; names.len()]
    } else {
        names.iter().map(|c| !doc.hidden.contains(&c.to_lowercase())).collect()
    };
    let mut cursor = match app.outliner.marks_cursor {
        Some((k, at)) if k == owner => Some(at.min(names.len().saturating_sub(1))),
        _ => None,
    };
    let here = marks_have_keys(app, &ctx, owner) && tree::keys_free(ui);
    // (Its columns with the keys: Undo and Redo are the table's.)
    if marks_have_keys(app, &ctx, owner) {
        app.undo_keys = Some(crate::Tab::Table(id));
    }
    let moved = here && app.outliner.marks_moved == Some(owner);
    if moved {
        app.outliner.marks_moved = None;
    }
    let mut clicked = None;
    let mut eye = None;
    let scroll =
        egui::ScrollArea::vertical().id_salt(("outliner-columns", id, window)).auto_shrink(false);
    // (Only the rows in sight are laid out, as `ScrollArea::show_rows`
    // does, under the list's 2 points of room: the keys' row brought into
    // sight by its place, drawn or not.)
    let rows = scroll.show_viewport(ui, |ui, view| {
        mg_uikit::list_rows(ui);
        let pitch = mg_uikit::row_pitch(ui);
        let count = names.len();
        ui.set_height(COLUMNS_TOP + count as f32 * pitch);
        let first = (((view.min.y - COLUMNS_TOP) / pitch).floor().max(0.0) as usize).min(count);
        let last = ((((view.max.y - COLUMNS_TOP) / pitch).ceil().max(0.0) as usize) + 1).min(count);
        let top = ui.max_rect().top() + COLUMNS_TOP;
        if moved && let Some(c) = cursor {
            let y = top + c as f32 * pitch;
            let row = egui::Rect::from_x_y_ranges(ui.max_rect().x_range(), y..=y + pitch);
            ui.scroll_to_rect(row, None);
        }
        let shown_rows = egui::Rect::from_x_y_ranges(
            ui.max_rect().x_range(),
            top + first as f32 * pitch..=top + last as f32 * pitch,
        );
        let ui = &mut ui.new_child(egui::UiBuilder::new().max_rect(shown_rows));
        ui.skip_ahead_auto_ids(first);
        for c in first..last {
            let column = &names[c];
            let (icon, words) = seen.looks[c].clone();
            let mut row = mg_uikit::TreeRow::new(column.as_str())
                .icon(icon)
                .fit()
                .quiet(!shown[c])
                .selected((here && cursor == Some(c)) || (!here && current == Some(c)));
            // (Its kind's words said with its name to a screen reader.)
            if let Some(words) = words {
                row = row.label(format!("{column}, {words}")).second(words);
            }
            let tip = if shown[c] { HIDE_TIP } else { SHOW_TIP };
            let row = row.eye(shown[c], format!("Show {column}"), tip).show_parts(ui);
            if row.eye.as_ref().is_some_and(egui::Response::clicked) {
                eye = Some(c);
            } else if row.response.clicked() {
                clicked = Some(c);
            }
        }
    });
    if here {
        crate::palette_view::keys_outline(ui, rows.inner_rect, Keys::Marks);
    }
    let mut go = None;
    if let Some(c) = clicked {
        cursor = Some(c);
        app.outliner.marks_cursor = Some((owner, c));
        app.outliner.marks_keys = Some(owner);
        give_keys(&ctx, Keys::Marks);
        go = Some(c);
    }
    if here {
        use egui::Key;
        let keys = [
            Key::ArrowDown,
            Key::ArrowUp,
            Key::PageDown,
            Key::PageUp,
            Key::Home,
            Key::End,
            Key::Enter,
        ];
        let pressed =
            ui.input_mut(|i| keys.into_iter().find(|k| i.consume_key(egui::Modifiers::NONE, *k)));
        if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::Escape)) {
            app.outliner.marks_cursor = None;
            app.outliner.marks_keys = None;
            crate::palette_view::give_arrows(&ctx, false);
            cursor = None;
        }
        let last = names.len().saturating_sub(1);
        let page = ((rows.inner_rect.height() / mg_uikit::row_pitch(ui)) as usize).max(1);
        let at = cursor.unwrap_or(0).min(last);
        let to = match pressed {
            Some(Key::ArrowDown) => Some((at + 1).min(last)),
            Some(Key::ArrowUp) => Some(at.saturating_sub(1)),
            Some(Key::PageDown) => Some((at + page).min(last)),
            Some(Key::PageUp) => Some(at.saturating_sub(page)),
            Some(Key::Home) => Some(0),
            Some(Key::End) => Some(last),
            Some(Key::Enter) => {
                go = Some(at);
                Some(at)
            }
            _ => None,
        };
        // Letters go to the column whose name begins with them.
        let seek = |from: Option<usize>, typed: &str| {
            let texts: Vec<String> = names.iter().map(|n| mg_core::text::fold(n)).collect();
            let from = from.unwrap_or(texts.len().saturating_sub(1));
            crate::dialog_view::type_ahead(&texts, from, &mg_core::text::fold(typed))
        };
        let typed = (!names.is_empty())
            .then(|| {
                tree::typed_row(ui, egui::Id::new(("outliner-columns-typed", id)), cursor, &seek)
            })
            .flatten();
        if let Some(to) = to.or(typed).filter(|_| !names.is_empty()) {
            app.outliner.marks_cursor = Some((owner, to));
            app.outliner.marks_moved = Some(owner);
            ctx.request_repaint();
        }
    }
    let doc = &mut app.tables[at];
    if let Some(c) = eye {
        match shown[c] {
            true => doc.hide_column(c),
            false => _ = doc.hidden.remove(&names[c].to_lowercase()),
        }
    }
    // (A hidden column has no cell in sight to go to.)
    if let Some(c) = go.filter(|c| shown.get(*c) == Some(&true)) {
        let row = doc.selected.map_or(0, |s| s.0);
        if row < doc.edit.table().len() {
            doc.go_to(row, c);
        }
    }
}

/// A floating editor's Outliner folded to a strip at its left (the docks'
/// own fold strip, `mg_uikit::shell::FoldStrip`), under its header: a
/// conversation's bookmarks beside its tree (M4's U15), a table's columns
/// beside its grid (U16). Its icon unfolds the Outliner over the editor,
/// and folds it again; the strip's place noted for it ([`over`]).
pub(crate) fn strip(app: &mut Moonglow, ui: &mut Ui, owner: Owner) {
    use mg_uikit::shell::{FoldStrip, STRIP, Side, StripClick};
    let t = mg_uikit::tokens(ui.ctx());
    let open = app.outliner.strips_open.contains(&owner);
    let tip = match (open, owner) {
        (true, _) => "Fold the Outliner away",
        (false, Owner::Dialog(_)) => "Show the Outliner: the conversation's bookmarks",
        (false, Owner::Table(_)) => "Show the Outliner: the table's columns",
        (false, Owner::Nui(_)) => "Show the Outliner: the window's layers",
        (false, Owner::Tileset(_)) => "Show the Outliner: the tileset's parts",
        (false, Owner::Hak(_)) => "Show the Outliner: the hak's contents by type",
        (false, Owner::Factions) => "Show the Outliner: the factions",
    };
    let shown = egui::Panel::left(egui::Id::new(("outliner-strip", owner)))
        .exact_size(STRIP)
        .resizable(false)
        .frame(egui::Frame::new().fill(t.bar))
        .show(ui, |ui| {
            let name = crate::dialog_view::OUTLINER_STRIP;
            let strip = FoldStrip::new(name, Side::Left).chevron(false);
            strip.panel(mg_uikit::Icon::Outliner, OUTLINER, tip).show(ui)
        });
    if matches!(shown.inner, Some(StripClick::Panel(_) | StripClick::Unfold)) {
        match open {
            true => _ = app.outliner.strips_open.remove(&owner),
            false => _ = app.outliner.strips_open.insert(owner),
        }
        ui.ctx().request_repaint();
    }
    let strip = shown.response.rect;
    app.outliner.strip_at = Some((owner, strip));
}

/// A floating editor's Outliner unfolded from its strip, over the editor
/// from the strip's edge, as wide as the window's pane at first (or what
/// the editor leaves), the whole height under the header; a press there
/// reaches nothing under it. Drawn after the editor, at the window's text
/// size, as the Outliner is.
pub(crate) fn over(app: &mut Moonglow, ui: &mut Ui, owner: Owner) {
    let Some((at, strip)) = app.outliner.strip_at.take() else { return };
    if at != owner || !app.outliner.strips_open.contains(&owner) {
        return;
    }
    let t = mg_uikit::tokens(ui.ctx());
    let room = (ui.max_rect().right() - strip.right()).max(0.0);
    let wide = crate::shell::WINDOW_LEFT_WIDE.min(room * 0.8);
    let over = egui::Rect::from_min_size(
        egui::pos2(strip.right(), strip.top()),
        egui::vec2(wide, strip.height()),
    );
    let mut child =
        ui.new_child(egui::UiBuilder::new().id_salt(("outliner-over", owner)).max_rect(over));
    child.set_style(ui.ctx().global_style());
    child.set_clip_rect(over.intersect(ui.clip_rect()));
    // (Over the editor: its fill and its edge, and the presses there its.)
    child.interact(
        over,
        egui::Id::new(("outliner-over-ground", owner)),
        egui::Sense::click_and_drag(),
    );
    child.painter().rect_filled(over, 0.0, t.panel);
    child.painter().vline(over.right(), over.y_range(), egui::Stroke::new(1.0, t.line));
    app.window_bars_drawn.push(crate::shell::Panel::OUTLINER);
    window_ui(app, &mut child, owner);
}
