//! The Outliner (the UI redesign's M3, task T13; ruling R2): under the
//! Left Dock's tabs, split from them by a seam (`shell::left_dock`), what
//! the active editor holds. In M3 the area's: its placed objects by kind
//! (`LightArea_crop_outl.png`), each kind with its count and a show or
//! hide **eye**, which is the area toolbar's Show toggle of the same kind
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

use std::collections::BTreeSet;

use egui::Ui;
use mg_core::ResRef;

use crate::Moonglow;
use crate::palette_view::{Keys, give_keys, keys_are};
use crate::tree::{self, Asked, Look, Placed, Step, TreeAt};

/// The Outliner's name: the group its header and rows are in, apart from
/// the area toolbar's (whose "Show Creatures" its eye shares a name with).
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
    /// The object a Shift+click chooses from.
    anchor: Option<Placed>,
}

/// The area the Outliner lists: the active editor's, where it is an
/// area's tab (`shell::editor_brings_outliner`).
pub(crate) fn area(app: &Moonglow) -> Option<ResRef> {
    match app.active_editor() {
        Some(crate::Tab::Area(area)) if app.ws.is_some() => Some(*area),
        _ => None,
    }
}

/// The keys given to the Outliner (Next Pane, the folded strip's icon): at
/// the row they were at, else its first.
pub(crate) fn take_keys(app: &mut Moonglow, ctx: &egui::Context) {
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
    if area(app).is_none() {
        return;
    }
    ui.scope(|ui| panel(app, ui));
}

fn panel(app: &mut Moonglow, ui: &mut Ui) {
    let Some(area) = area(app) else { return };
    let ctx = ui.ctx().clone();
    mg_uikit::name_group(ui, OUTLINER);
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
    let shown = crate::area_view::kinds_shown(app, area);
    let menu_commands = app.plugin_menu_commands();
    let in_hand = last.map(|(kind, index)| (kind, index, app.outliner.reveal));
    let mut keys = tree::ContentKeys {
        cursor: app.outliner.cursor,
        moved: std::mem::take(&mut app.outliner.cursor_moved),
        fold: app.outliner.fold.take(),
        ..Default::default()
    };
    let mut go = None;
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
        // (A kind hidden in the view: shown, as a plain click shows it.)
        if !shown[kind.index()] {
            crate::area_view::show_kind(app, area, kind, true);
        }
        let objects: Vec<Placed> = (listed.iter())
            .filter_map(|r| match r.0 {
                TreeAt::Object(_, k, i) => Some((k, i)),
                _ => None,
            })
            .collect();
        let mut set = app.outliner.chosen.clone();
        let mut anchor = app.outliner.anchor.or(last);
        crate::widgets::select_in_list(
            &mut set,
            &mut anchor,
            &objects,
            (kind, index),
            command,
            shift,
        );
        app.outliner.anchor = anchor;
        tree::select_in_view(app, area, set.into_iter().collect());
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
        let under = keys.hovered.or(app.outliner.cursor);
        if let Some(at @ TreeAt::Object(_, kind, index)) = under
            && ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::Delete))
        {
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
        // (A kind hidden in the view: shown, for the object selected
        // there; a hidden object cannot stay selected.)
        if matches!(asked, Asked::Select | Asked::Go | Asked::Adjust) && !shown[kind.index()] {
            crate::area_view::show_kind(app, area, kind, true);
        }
        if asked == Asked::Select {
            app.outliner.anchor = Some((kind, index));
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
