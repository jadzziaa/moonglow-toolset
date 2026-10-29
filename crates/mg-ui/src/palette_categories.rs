//! Palette Categories: the categories a module's blueprints of a type go
//! in. The game's are its palette skeleton (`<type>pal.itp`); a module that
//! adds, renames or removes one gets a skeleton of its own, kept in the
//! module, by which Moonglow sorts its custom palette (and builds the
//! `<type>palcus.itp` the game's DM client reads). Content packs do the
//! same with a skeleton in a hak.
//!
//! A name is written out (in the game's text for its language) or a
//! talk-table string (`STRREF`, as the game's own are), which each
//! player's talk table shows in their language. Removing a category the
//! module's blueprints are in moves them to another, chosen, with the
//! window's OK: one step to undo, with the skeleton.

use std::collections::BTreeMap;

use egui::Ui;
use mg_gff::{Gff, Struct};
use mg_module::palette::{self, BlueprintKind, NodePath};
use mg_resman::ResKey;

use crate::{Action, Moonglow};

/// The Palette Categories window's state.
#[derive(Debug, Clone)]
pub struct PaletteCategories {
    pub kind: BlueprintKind,
    /// The skeleton as edited.
    pub skeleton: Gff,
    /// The module has a skeleton of its own (else this began as the
    /// game's).
    pub own: bool,
    pub selected: Option<NodePath>,
    /// The name typed, for a new group or category, or the selected one's.
    pub name: String,
    /// The talk-table string typed, to name the selected one by.
    pub strref: String,
    pub error: Option<String>,
    changed: bool,
    /// The module's blueprints to move, to the category of each (those of
    /// a category removed), as OK keeps the skeleton.
    moves: BTreeMap<ResKey, u8>,
    /// A category the module's blueprints are in, asked about before it
    /// is removed.
    removing: Option<Removing>,
}

/// A removal waiting for the category its blueprints go to.
#[derive(Debug, Clone)]
struct Removing {
    /// The skeleton without it, and the blueprints in it.
    without: Gff,
    blueprints: Vec<ResKey>,
    /// The category they go to.
    to: Option<u8>,
}

/// Opens the window for `kind`, on the module's skeleton or the game's.
pub(crate) fn open(app: &mut Moonglow, kind: BlueprintKind) {
    let (Some(ws), Some(game)) = (app.ws.as_mut(), app.game.as_deref()) else { return };
    if let Err(e) = ws.flush() {
        app.log.error(e.to_string());
        return;
    }
    let own = ws.module.contains(&kind.skeleton_key());
    match palette::skeleton(&ws.module, game, kind) {
        Ok(mut skeleton) => {
            // The game's categories show by name; the module's own, once
            // it has them, in the order they are given here.
            if !own {
                let name = |s: u32| game.string(mg_core::StrRef(s)).unwrap_or_default();
                palette::sort_by_name(&mut skeleton, &name);
            }
            app.palette_categories = Some(PaletteCategories {
                kind,
                skeleton,
                own,
                selected: None,
                name: String::new(),
                strref: String::new(),
                error: None,
                changed: false,
                moves: BTreeMap::new(),
                removing: None,
            });
        }
        Err(e) => app.log.error(format!("Palette categories: {e}")),
    }
}

/// A node's name as shown.
fn name_of(app: &Moonglow, node: &Struct) -> String {
    // (As the game reads text in its language; UTF-8 as Moonglow wrote
    // names before 1.19.4.)
    let codepage = app.game.as_deref().map_or_else(Default::default, |g| g.codepage());
    let text = |label: &str| node.string(label).map(|t| palette::text_of(t, codepage));
    let named = text("NAME").or_else(|| {
        let strref = u32::try_from(node.integer("STRREF")?).ok()?;
        app.game.as_deref()?.string(mg_core::StrRef(strref))
    });
    let strref = || Some(format!("(StrRef {})", node.integer("STRREF")?));
    named.or_else(|| text("DELETE_ME")).or_else(strref).unwrap_or_default()
}

/// The skeleton's categories (not groups), each by its name and the
/// groups it is in: what blueprints can be put in.
fn categories(app: &Moonglow, skeleton: &Gff) -> Vec<(u8, String)> {
    fn walk(app: &Moonglow, list: &[Struct], within: &str, out: &mut Vec<(u8, String)>) {
        for node in list.iter().filter(|n| !palette::is_placeholder(n)) {
            let name = match within {
                "" => name_of(app, node),
                g => format!("{g} › {}", name_of(app, node)),
            };
            if let Some(id) = node.integer("ID").and_then(|i| u8::try_from(i).ok()) {
                out.push((id, name.clone()));
            }
            if let Some(children) = node.list("LIST") {
                walk(app, children, &name, out);
            }
        }
    }
    let mut out = Vec::new();
    walk(app, skeleton.root.list("MAIN").unwrap_or(&[]), "", &mut out);
    out
}

/// The module's blueprints in the categories `ids` once the window's OK
/// has moved those it moves.
fn blueprints_in(app: &Moonglow, w: &PaletteCategories, ids: &[u8]) -> Vec<ResKey> {
    let Some(ws) = app.ws.as_ref() else { return Vec::new() };
    let now = palette::blueprints_of(&ws.module, w.kind, ids);
    let staying = now.into_iter().map(|(k, _)| k).filter(|k| !w.moves.contains_key(k));
    let coming = w.moves.iter().filter(|(_, to)| ids.contains(to)).map(|(k, _)| *k);
    staying.chain(coming).collect()
}

/// Where a dragged row is let go on another.
#[derive(Debug, Clone, PartialEq)]
struct Drop {
    from: NodePath,
    /// The group it goes in, and its place there.
    parent: NodePath,
    index: usize,
}

/// The skeleton's tree: each node a row, the one chosen highlighted. A row
/// is dragged onto another to move it: on a row's upper half, before it;
/// on its lower half, after it, or into it if it is a group.
fn rows(
    app: &Moonglow,
    ui: &mut Ui,
    list: &[Struct],
    at: &mut NodePath,
    selected: &mut Option<NodePath>,
    name: &mut String,
    drop: &mut Option<Drop>,
) {
    for (i, node) in list.iter().enumerate() {
        if palette::is_placeholder(node) {
            continue;
        }
        at.push(i);
        let shown = name_of(app, node);
        let children = node.list("LIST");
        let group = node.integer("ID").is_none();
        let label = match node.integer("ID") {
            Some(id) => format!("{shown}  (category {id})"),
            None => format!("{shown}  (group)"),
        };
        let chosen = selected.as_ref() == Some(at);
        // (No tooltip on the rows: one under a resting pointer would take
        // the wheel from the list.)
        let r =
            ui.add(egui::Button::selectable(chosen, label).sense(egui::Sense::click_and_drag()));
        if r.clicked() {
            *selected = Some(at.clone());
            *name = shown;
        }
        if r.drag_started() {
            r.dnd_set_drag_payload(at.clone());
        }
        // Where a row dragged over this one would go, and a line there.
        let over = r.dnd_hover_payload::<NodePath>().filter(|from| **from != *at);
        if let Some(from) = over {
            let upper = ui.ctx().pointer_hover_pos().is_none_or(|p| p.y < r.rect.center().y);
            let (parent, index, y) = match (upper, group) {
                (true, _) => (at[..at.len() - 1].to_vec(), i, r.rect.top()),
                (false, true) => (at.clone(), 0, r.rect.bottom()),
                (false, false) => (at[..at.len() - 1].to_vec(), i + 1, r.rect.bottom()),
            };
            let indent = if !upper && group { ui.spacing().indent } else { 0.0 };
            let stroke = egui::Stroke::new(2.0, ui.visuals().selection.bg_fill);
            ui.painter().hline((r.rect.left() + indent)..=r.rect.right().max(260.0), y, stroke);
            if r.dnd_release_payload::<NodePath>().is_some() {
                *drop = Some(Drop { from: (*from).clone(), parent, index });
            }
        }
        if let Some(children) = children {
            ui.indent(("palette-category", at.clone()), |ui| {
                rows(app, ui, children, at, selected, name, drop);
            });
        }
        at.pop();
    }
}

/// A row dragged near the top or the bottom of the list scrolls it, the
/// faster the nearer the edge, so that it can be taken to a row out of
/// sight.
fn scroll_while_dragging(ui: &mut Ui) {
    /// How near an edge the list starts to scroll, and how far a frame at
    /// the edge itself, points.
    const EDGE: f32 = 36.0;
    const STEP: f32 = 14.0;
    if !egui::DragAndDrop::has_payload_of_type::<NodePath>(ui.ctx()) {
        return;
    }
    let seen = ui.clip_rect();
    let Some(pointer) = ui.ctx().pointer_latest_pos() else { return };
    if pointer.x < seen.left() || pointer.x > seen.right() {
        return;
    }
    // The wheel scrolls it too: a scroll area takes no wheel while
    // something is dragged, so it is passed on here.
    let wheel = ui.input(|i| i.smooth_scroll_delta.y);
    if wheel != 0.0 {
        ui.scroll_with_delta(egui::vec2(0.0, wheel));
        ui.input_mut(|i| i.smooth_scroll_delta.y = 0.0);
        ui.ctx().request_repaint();
    }
    // (Above or below the list counts as at its edge.)
    let near = |distance: f32| (1.0 - distance.max(0.0) / EDGE).clamp(0.0, 1.0);
    let (up, down) = (near(pointer.y - seen.top()), near(seen.bottom() - pointer.y));
    let by = (up - down) * STEP;
    if by != 0.0 {
        ui.scroll_with_delta(egui::vec2(0.0, by));
        ui.ctx().request_repaint();
    }
}

/// The node at `path`.
fn node<'a>(skeleton: &'a Gff, path: &[usize]) -> Option<&'a Struct> {
    let (first, rest) = path.split_first()?;
    let mut node = skeleton.root.list("MAIN")?.get(*first)?;
    for i in rest {
        node = node.list("LIST")?.get(*i)?;
    }
    Some(node)
}

pub(crate) fn window(app: &mut Moonglow, ctx: &egui::Context) {
    let Some(mut w) = app.palette_categories.take() else { return };
    let mut open = true;
    let mut done = None;
    let title = format!("Palette Categories: {}", w.kind.label());
    let window =
        crate::widgets::dialog(ctx, &title, &mut open).default_width(420.0).collapsible(false);
    window.show(ctx, |ui| {
        ui.label(if w.own {
            "This module's own categories, kept in the module."
        } else {
            "The game's categories. A change gives the module categories of its own."
        });
        ui.weak("Drag a row to move it: before or after another, or into a group.");
        egui::ScrollArea::vertical().max_height(420.0).auto_shrink([false, true]).show(ui, |ui| {
            let list = w.skeleton.root.list("MAIN").unwrap_or(&[]).to_vec();
            let (mut at, mut drop) = (Vec::new(), None);
            rows(app, ui, &list, &mut at, &mut w.selected, &mut w.name, &mut drop);
            scroll_while_dragging(ui);
            if let Some(d) = drop {
                match palette::move_category(&mut w.skeleton, &d.from, &d.parent, d.index) {
                    Ok(now) => {
                        w.selected = Some(now);
                        w.error = None;
                        w.changed = true;
                        w.removing = None;
                    }
                    Err(e) => w.error = Some(e),
                }
            }
        });
        ui.separator();
        // (A name is written as the game reads text in its language.)
        let codepage = app.game.as_deref().map_or_else(Default::default, |g| g.codepage());
        // Where a new one goes: in the group chosen, beside the
        // category chosen, else at the top.
        let chosen = w.selected.clone();
        let chosen_node = chosen.as_deref().and_then(|p| node(&w.skeleton, p));
        let is_group = chosen_node.is_some_and(|n| n.integer("ID").is_none());
        let parent: NodePath = match &chosen {
            Some(p) if is_group => p.clone(),
            Some(p) => p[..p.len() - 1].to_vec(),
            None => Vec::new(),
        };
        ui.horizontal(|ui| {
            crate::widgets::field_label(ui, "Name");
            ui.add(
                egui::TextEdit::singleline(&mut w.name)
                    .desired_width(220.0)
                    .hint_text("a category's or group's name"),
            );
        });
        let mut result: Option<Result<Option<NodePath>, String>> = None;
        ui.horizontal_wrapped(|ui| {
            let where_ = if parent.is_empty() { "at the top" } else { "in the group chosen" };
            if ui
                .button("Add Category")
                .on_hover_text(format!("A new category of that name, {where_}"))
                .clicked()
            {
                let added = palette::add_category(&mut w.skeleton, &parent, &w.name, codepage);
                result = Some(added.map(|(path, _)| Some(path)));
            }
            if ui
                .button("Add Group")
                .on_hover_text(format!("A new group to hold categories, {where_}"))
                .clicked()
            {
                let added = palette::add_group(&mut w.skeleton, &parent, &w.name, codepage);
                result = Some(added.map(Some));
            }
            if ui
                .add_enabled(chosen.is_some(), egui::Button::new("Rename"))
                .on_hover_text("Give the one chosen that name (its blueprints stay in it)")
                .clicked()
                && let Some(path) = &chosen
            {
                let renamed = palette::rename_category(&mut w.skeleton, path, &w.name, codepage);
                result = Some(renamed.map(|()| Some(path.clone())));
            }
            if ui
                .add_enabled(chosen.is_some(), egui::Button::new("Remove"))
                .on_hover_text("Remove the one chosen, a group with what is in it")
                .clicked()
                && let Some(path) = &chosen
            {
                match remove(app, &mut w, path) {
                    Ok(true) => result = Some(Ok(None)),
                    Ok(false) => {}
                    Err(e) => result = Some(Err(e)),
                }
            }
        });
        // A talk-table string for the name (shown in each player's
        // language): a StrRef typed, or the name typed put into the
        // module's own talk table as a new line.
        ui.horizontal(|ui| {
            crate::widgets::field_label(ui, "StrRef");
            ui.add(
                egui::TextEdit::singleline(&mut w.strref)
                    .desired_width(100.0)
                    .hint_text("a talk-table line"),
            );
            let typed = w.strref.trim().parse::<u32>().ok();
            if let Some(text) = typed.and_then(|n| app.game.as_deref()?.string(mg_core::StrRef(n)))
            {
                ui.weak(text);
            }
            if ui
                .add_enabled(
                    chosen.is_some() && typed.is_some(),
                    egui::Button::new("Name by StrRef"),
                )
                .on_hover_text(
                    "Names the one chosen by that talk-table line, as the game's own categories \
                     are: each player sees it in their language",
                )
                .clicked()
                && let (Some(path), Some(n)) = (&chosen, typed)
            {
                let named = palette::name_by_strref(&mut w.skeleton, path, n);
                result = Some(named.map(|()| Some(path.clone())));
            }
        });
        let movable = crate::talk_view::can_add(app).is_some();
        if ui
            .add_enabled(
                chosen.is_some() && movable && !w.name.trim().is_empty(),
                egui::Button::new("Move Name to Talk Table"),
            )
            .on_hover_text(
                "Adds the name typed to the module's talk table as a new line, and names the one \
                 chosen by it: translate the line in the talk table's other languages",
            )
            .on_disabled_hover_text(
                "Needs the module's own talk table (Module Properties › Custom Content), a name \
                 typed and one chosen",
            )
            .clicked()
            && let Some(path) = &chosen
        {
            let line = mg_module::talk::Line {
                text: w.name.trim().to_string(),
                feminine: None,
                sound: String::new(),
                sound_length: 0.0,
            };
            result = Some(match crate::talk_view::add_line(app, &line) {
                Ok(strref) => {
                    w.strref = strref.0.to_string();
                    app.log.info(format!(
                        "Added talk-table line {}; it's saved with the module",
                        strref.0
                    ));
                    palette::name_by_strref(&mut w.skeleton, path, strref.0)
                        .map(|()| Some(path.clone()))
                }
                Err(e) => Err(format!("Move to Talk Table: {e}")),
            });
        }
        // (Anything else done meanwhile asks again: the list changed.)
        if result.is_some() {
            w.removing = None;
        }
        match result {
            Some(Ok(selected)) => {
                w.selected = selected;
                w.error = None;
                w.changed = true;
            }
            Some(Err(e)) => w.error = Some(e),
            None => {}
        }
        // A category the module's blueprints are in: where they go.
        let mut asked = None;
        if let Some(r) = &mut w.removing {
            let n = r.blueprints.len();
            let s = if n == 1 { " is" } else { "s are" };
            let left = categories(app, &r.without);
            egui::Frame::group(ui.style()).show(ui, |ui| {
                ui.label(format!("{n} of the module's blueprint{s} in it. Move them to:"));
                let shown =
                    r.to.and_then(|id| left.iter().find(|c| c.0 == id))
                        .map_or("choose a category", |c| &c.1);
                crate::widgets::dropdown("palette-category-to")
                    .width(260.0)
                    .selected_text(shown)
                    .show_ui(ui, |ui| {
                        for (id, name) in &left {
                            ui.selectable_value(&mut r.to, Some(*id), name);
                        }
                    });
                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(r.to.is_some(), egui::Button::new("Move and Remove"))
                        .on_hover_text("Moves them there, and removes it, as OK keeps the list")
                        .clicked()
                    {
                        asked = Some(true);
                    }
                    if ui.button("Keep It").clicked() {
                        asked = Some(false);
                    }
                });
            });
        }
        match (asked, w.removing.take()) {
            (Some(true), Some(r)) => {
                let to = r.to.expect("chosen");
                w.skeleton = r.without;
                w.moves.extend(r.blueprints.into_iter().map(|k| (k, to)));
                w.selected = None;
                w.error = None;
                w.changed = true;
            }
            (Some(_), _) => {}
            (None, r) => w.removing = r,
        }
        if let Some(e) = &w.error {
            ui.colored_label(ui.visuals().error_fg_color, e);
        }
        ui.separator();
        ui.horizontal(|ui| {
            if ui.add_enabled(w.changed, egui::Button::new("OK")).clicked() {
                done = Some(Done::Keep);
            }
            if crate::widgets::cancel(ui) {
                done = Some(Done::Cancel);
            }
            if ui
                .add_enabled(w.own, egui::Button::new("Use the Game's Categories"))
                .on_hover_text(
                    "Drop the module's own categories. Blueprints in a category the game \
                         lacks leave the palette until given another",
                )
                .clicked()
            {
                done = Some(Done::Revert);
            }
        });
    });
    let key = w.kind.skeleton_key();
    match done {
        Some(Done::Keep) => match w.skeleton.to_bytes() {
            Ok(data) => {
                // (The blueprints of a category removed into the one
                // chosen: the same step.)
                let mut edits = vec![mg_edit::Edit::SetResource { key, data: Some(data) }];
                for (blueprint, to) in &w.moves {
                    edits.push(mg_edit::Edit::SetField {
                        key: *blueprint,
                        path: mg_edit::GffPath::root(),
                        label: w.kind.palette_field().to_string(),
                        value: Some(mg_gff::Value::Byte(*to)),
                    });
                }
                let command = mg_edit::Command::new("Palette categories", edits);
                app.actions.push(Action::Apply(command));
            }
            Err(e) => app.log.error(format!("Palette categories: {e}")),
        },
        Some(Done::Revert) => {
            let edit = mg_edit::Edit::SetResource { key, data: None };
            let command = mg_edit::Command::new("Use the game's palette categories", vec![edit]);
            app.actions.push(Action::Apply(command));
        }
        Some(Done::Cancel) => {}
        None if open => app.palette_categories = Some(w),
        None => {}
    }
}

enum Done {
    Keep,
    Cancel,
    Revert,
}

/// Removes the group or category at `path`; where blueprints of the
/// module are in it (or in a category of the group), asks first where
/// they go (`Ok(false)`: asked, not removed yet).
fn remove(app: &mut Moonglow, w: &mut PaletteCategories, path: &[usize]) -> Result<bool, String> {
    let mut trial = w.skeleton.clone();
    let ids = palette::remove_category(&mut trial, path)?;
    let blueprints = blueprints_in(app, w, &ids);
    if blueprints.is_empty() {
        w.skeleton = trial;
        return Ok(true);
    }
    if categories(app, &trial).is_empty() {
        return Err("the module's blueprints are in it, and there is no other category".into());
    }
    w.removing = Some(Removing { without: trial, blueprints, to: None });
    Ok(false)
}
