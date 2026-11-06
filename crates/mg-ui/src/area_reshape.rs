//! The Resize Area and Rotate Area windows (Aurora's Edit menu): the
//! area shown last, resized at two of its edges or turned a
//! multiple of 90°, as one undoable command (`mg_area::reshape`).

use mg_core::{ResRef, ResType};
use mg_edit::Command;
use mg_resman::ResKey;
use mg_tiles::TileIndex;

use crate::{Action, Moonglow};

/// The Resize Area window's values.
#[derive(Debug, Clone, PartialEq)]
pub struct ResizeDraft {
    pub area: ResRef,
    pub rows: u32,
    pub columns: u32,
    /// The two edges rows and columns come and go at: an index into
    /// [`EDGES`] (0: north and east, as in Aurora).
    pub edges: u8,
}

/// The pairs of edges an area is resized at, in the order
/// `mg_area::reshape::resize_at` counts them.
const EDGES: [&str; 4] = ["North and east", "East and south", "South and west", "West and north"];

/// The Rotate Area window's choice: quarter turns counter-clockwise.
#[derive(Debug, Clone, PartialEq)]
pub struct RotateDraft {
    pub area: ResRef,
    pub turns: u8,
}

/// Aurora's size presets.
const PRESETS: [(&str, u32); 4] = mg_module::new::AREA_SIZES;

/// Opens Resize Area for `area`, with its size.
pub(crate) fn open_resize(app: &mut Moonglow, area: ResRef) {
    let Some(ws) = app.ws.as_mut() else { return };
    let Ok(are) = ws.doc(&ResKey::new(area, ResType::ARE)) else { return };
    let size = |l: &str| are.root.integer(l).unwrap_or(2).clamp(2, 32) as u32;
    app.resize_area =
        Some(ResizeDraft { area, rows: size("Height"), columns: size("Width"), edges: 0 });
}

pub(crate) fn windows(app: &mut Moonglow, ctx: &egui::Context) {
    resize_window(app, ctx);
    rotate_window(app, ctx);
    tileset_window(app, ctx);
}

fn resize_window(app: &mut Moonglow, ctx: &egui::Context) {
    let Some(mut d) = app.resize_area.take() else { return };
    let mut open = true;
    let (mut ok, mut cancel) = (false, false);
    crate::widgets::dialog(ctx, "Resize Area", &mut open).collapsible(false).resizable(false).show(
        ctx,
        |ui| {
            ui.horizontal(|ui| {
                egui::Grid::new("resize").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
                    crate::widgets::field_label(ui, "Rows");
                    crate::widgets::spin_draft(ui, &mut d.rows, 2..=32, 1.0, |d| d)
                        .on_hover_text("Number of rows in the area");
                    ui.end_row();
                    crate::widgets::field_label(ui, "Columns");
                    crate::widgets::spin_draft(ui, &mut d.columns, 2..=32, 1.0, |d| d)
                        .on_hover_text("Number of columns in the area");
                    ui.end_row();
                });
                ui.separator();
                ui.vertical(|ui| {
                    ui.label("Defaults");
                    for (name, size) in PRESETS {
                        if ui.selectable_label(d.rows == size && d.columns == size, name).clicked()
                        {
                            (d.rows, d.columns) = (size, size);
                        }
                    }
                });
            });
            ui.horizontal(|ui| {
                ui.label("Rows and columns come and go at the edges");
                let edges: Vec<u8> = (0..EDGES.len() as u8).collect();
                crate::widgets::dropdown("resize-edges")
                    .selected_text(EDGES[usize::from(d.edges % 4)])
                    .show(ui, &mut d.edges, &edges, |e| EDGES[usize::from(*e % 4)])
                    .on_hover_text(
                        "North and east as in Aurora. The other two edges stay, and what is \
                         in the area keeps its place against them",
                    );
            });
            ui.horizontal(|ui| {
                ok = ui.button("OK").on_hover_text("Accept changes").clicked()
                    || crate::widgets::enter(ui);
                cancel = crate::widgets::cancel_discard(ui);
            });
        },
    );
    if ok {
        resize(app, &d);
    } else if open && !cancel {
        app.resize_area = Some(d);
    }
}

fn rotate_window(app: &mut Moonglow, ctx: &egui::Context) {
    let Some(mut d) = app.rotate_area.take() else { return };
    let mut open = true;
    let (mut ok, mut cancel) = (false, false);
    crate::widgets::dialog(ctx, "Rotate Area", &mut open).collapsible(false).resizable(false).show(
        ctx,
        |ui| {
            egui::Grid::new("rotations").num_columns(2).spacing([24.0, 6.0]).show(ui, |ui| {
                // Counter-clockwise 90, 180, 270 and clockwise 90, 180, 270:
                // as quarter turns counter-clockwise.
                for (ccw, cw) in [(1, 3), (2, 2), (3, 1)] {
                    let degrees = 90 * u32::from(ccw);
                    ui.radio_value(&mut d.turns, ccw, format!("CounterClockwise {degrees}"));
                    ui.radio_value(&mut d.turns, cw + 4, format!("Clockwise {degrees}"));
                    ui.end_row();
                }
            });
            ui.horizontal(|ui| {
                ok = ui.button("OK").on_hover_text("Accept changes").clicked()
                    || crate::widgets::enter(ui);
                cancel = crate::widgets::cancel_discard(ui);
            });
        },
    );
    if ok {
        rotate(app, &d);
    } else if open && !cancel {
        app.rotate_area = Some(d);
    }
}

fn resize(app: &mut Moonglow, d: &ResizeDraft) {
    let (are_key, git_key) = (ResKey::new(d.area, ResType::ARE), ResKey::new(d.area, ResType::GIT));
    let Some(Ok(are)) = app.ws.as_mut().map(|ws| ws.doc(&are_key).map(|g| g.root.clone())) else {
        return;
    };
    let tileset = are.resref("Tileset").unwrap_or(ResRef::EMPTY);
    let set = app.tileset(tileset);
    let (Some(ws), Some(game)) = (app.ws.as_mut(), app.game.as_deref()) else { return };
    let git = ws.doc(&git_key).map(|g| g.root.clone()).unwrap_or_default();
    let Some(set) = set else {
        app.log.error(format!("Resize Area: tileset {tileset} not found"));
        return;
    };
    let index = TileIndex::new(&set);
    let scheme = are
        .integer("LightingScheme")
        .and_then(|row| mg_module::new::Scheme::read(game, row.max(0) as usize).ok());
    let mut lights_rng = fastrand::Rng::new();
    let mut lights = || scheme.as_ref().map_or([0; 3], |s| s.tile_lights(&mut lights_rng));
    let mut rng = fastrand::Rng::new();
    let reshaped = mg_area::reshape::resize_at(
        are_key,
        git_key,
        &are,
        &git,
        &set,
        &index,
        d.columns,
        d.rows,
        d.edges,
        &mut rng,
        &mut lights,
    );
    let Some(r) = reshaped else {
        app.log.error("Resize Area: the area's tiles are not its tileset's");
        return;
    };
    app.actions.push(Action::Apply(Command::new("Resize Area", r.edits)));
    if r.deleted > 0 {
        app.log.warn(format!(
            "Some objects were deleted as a result of this operation ({}). \
             You may recover these objects by undoing the previous action.",
            r.deleted
        ));
    }
}

fn rotate(app: &mut Moonglow, d: &RotateDraft) {
    let Some(ws) = app.ws.as_mut() else { return };
    let (are_key, git_key) = (ResKey::new(d.area, ResType::ARE), ResKey::new(d.area, ResType::GIT));
    let Ok(are) = ws.doc(&are_key).map(|g| g.root.clone()) else { return };
    let git = ws.doc(&git_key).map(|g| g.root.clone()).unwrap_or_default();
    let edits = mg_area::reshape::rotate(are_key, git_key, &are, &git, d.turns % 4);
    app.actions.push(Action::Apply(Command::new("Rotate Area", edits)));
}

/// The Change Tileset window's values: the area, the tileset chosen, and
/// what its terrains and crossers become there.
#[derive(Debug, Clone, PartialEq)]
pub struct TilesetDraft {
    pub area: ResRef,
    /// The area's tileset now.
    pub from: ResRef,
    pub choices: Vec<mg_module::new::TilesetChoice>,
    /// The tileset chosen.
    pub to: ResRef,
    /// The mapping for `to` (made when it is chosen), and the new tileset's
    /// terrains and crossers to choose from.
    pub mapping: Option<mg_area::retile::Mapping>,
    terrains: Vec<String>,
    crossers: Vec<String>,
    mapped_for: Option<ResRef>,
}

/// Opens Change Tileset for `area`.
pub(crate) fn open_change_tileset(app: &mut Moonglow, area: ResRef) {
    let (Some(ws), Some(game)) = (app.ws.as_mut(), app.game.as_deref()) else { return };
    let Ok(are) = ws.doc(&ResKey::new(area, ResType::ARE)) else { return };
    let from = are.root.resref("Tileset").unwrap_or(ResRef::EMPTY);
    let choices = mg_module::new::tilesets(game);
    app.change_tileset = Some(TilesetDraft {
        area,
        from,
        to: from,
        choices,
        mapping: None,
        terrains: Vec::new(),
        crossers: Vec::new(),
        mapped_for: None,
    });
}

/// The area's ARE and GIT roots.
fn area_docs(
    app: &mut Moonglow,
    area: ResRef,
) -> Option<(ResKey, ResKey, mg_gff::Struct, mg_gff::Struct)> {
    let (are_key, git_key) = (ResKey::new(area, ResType::ARE), ResKey::new(area, ResType::GIT));
    let ws = app.ws.as_mut()?;
    let are = ws.doc(&are_key).ok()?.root.clone();
    let git = ws.doc(&git_key).map(|g| g.root.clone()).unwrap_or_default();
    Some((are_key, git_key, are, git))
}

/// The mapping for the tileset chosen, made anew when it changes.
fn map_for(app: &mut Moonglow, d: &mut TilesetDraft) {
    if d.mapped_for == Some(d.to) {
        return;
    }
    d.mapped_for = Some(d.to);
    let (old, new) = (app.tileset(d.from), app.tileset(d.to));
    let docs = area_docs(app, d.area);
    let (Some(old), Some(new), Some((_, _, are, _))) = (old, new, docs) else {
        d.mapping = None;
        return;
    };
    let (oi, ni) = (TileIndex::new(&old), TileIndex::new(&new));
    let set = |resref, set, index| mg_area::retile::Set { resref, set, index };
    d.mapping = mg_area::retile::Mapping::of(&set(d.from, &old, &oi), &set(d.to, &new, &ni), &are);
    let lower = |n: &str| n.to_ascii_lowercase();
    d.terrains = ni.terrains().map(|t| lower(ni.terrain_name(t))).collect();
    d.crossers = ni.crossers().map(|c| lower(ni.crosser_name(c))).collect();
}

fn tileset_window(app: &mut Moonglow, ctx: &egui::Context) {
    let Some(mut d) = app.change_tileset.take() else { return };
    map_for(app, &mut d);
    let mut open = true;
    let (mut ok, mut cancel) = (false, false);
    crate::widgets::dialog(ctx, "Change Tileset", &mut open).collapsible(false).show(ctx, |ui| {
        let name = |r: ResRef| {
            (d.choices.iter().find(|c| c.resref == r))
                .map_or_else(|| r.to_string(), |c| format!("{} ({r})", c.name))
        };
        ui.horizontal(|ui| {
            crate::widgets::field_label(ui, "Tileset");
            let all: Vec<ResRef> = d.choices.iter().map(|c| c.resref).collect();
            crate::widgets::dropdown("change-tileset").selected_text(name(d.to)).show(
                ui,
                &mut d.to,
                &all,
                |r| name(*r),
            );
        });
        ui.weak(
            "The area's tiles are laid anew in this tileset, its layout kept where the tileset \
             has the terrain: each terrain and crosser becomes the one below, the tileset's \
             placement rules fitting the tiles around, and heights where it has them. What \
             stands in the area (objects, sounds), its lighting, variables and scripts stay; \
             doors on the old tiles' hooks go, and the new tiles bring their own. One step to \
             undo. To try a tileset and keep this area, copy it first (the module tree's \
             Copy…).",
        );
        let Some(mapping) = d.mapping.as_mut() else {
            ui.colored_label(
                ui.visuals().warn_fg_color,
                "The area's tiles aren't its tileset's, or a tileset isn't found.",
            );
            return;
        };
        egui::Grid::new("tileset-mapping").num_columns(2).spacing([12.0, 4.0]).show(ui, |ui| {
            for (i, (from, to)) in mapping.terrains.iter_mut().enumerate() {
                crate::widgets::field_label(ui, from.as_str());
                crate::widgets::dropdown(("tileset-terrain", i)).selected_text(to.as_str()).show(
                    ui,
                    to,
                    &d.terrains,
                    |t| t.clone(),
                );
                ui.end_row();
            }
            let crossers: Vec<Option<String>> =
                std::iter::once(None).chain(d.crossers.iter().cloned().map(Some)).collect();
            let shown = |c: &Option<String>| c.clone().unwrap_or_else(|| "(none)".into());
            for (i, (from, to)) in mapping.crossers.iter_mut().enumerate() {
                crate::widgets::field_label(ui, format!("{from} (crosser)"));
                crate::widgets::dropdown(("tileset-crosser", i))
                    .selected_text(shown(to))
                    .show(ui, to, &crossers, shown);
                ui.end_row();
            }
        });
        ui.horizontal(|ui| {
            ok = ui.button("OK").on_hover_text("Lay the area in the tileset").clicked()
                || crate::widgets::enter(ui);
            cancel = crate::widgets::cancel_discard(ui);
        });
    });
    if ok && d.mapping.is_some() {
        change_tileset(app, &d);
    } else if open && !cancel {
        app.change_tileset = Some(d);
    }
}

fn change_tileset(app: &mut Moonglow, d: &TilesetDraft) {
    let (old, new) = (app.tileset(d.from), app.tileset(d.to));
    let Some((are_key, git_key, are, git)) = area_docs(app, d.area) else { return };
    let (Some(old), Some(new), Some(mapping)) = (old, new, d.mapping.as_ref()) else {
        app.log.error(format!("Change Tileset: tileset {} or {} not found", d.from, d.to));
        return;
    };
    let (oi, ni) = (TileIndex::new(&old), TileIndex::new(&new));
    let set = |resref, set, index| mg_area::retile::Set { resref, set, index };
    let (Some(ws), Some(game)) = (app.ws.as_mut(), app.game.as_deref()) else { return };
    let read =
        |r: ResRef| mg_module::gff_root(Some(&ws.module), game, &ResKey::new(r, ResType::UTD));
    let mut rng = fastrand::Rng::new();
    let retiled = mg_area::retile::retile(
        game,
        are_key,
        git_key,
        &are,
        &git,
        &set(d.from, &old, &oi),
        &set(d.to, &new, &ni),
        mapping,
        &read,
        &mut rng,
    );
    let Some(r) = retiled else {
        app.log.error(format!(
            "Change Tileset: the area's tiles aren't {}'s, or {} has no tile of its Default terrain",
            d.from, d.to
        ));
        return;
    };
    app.actions.push(Action::Apply(Command::new("Change Tileset", r.edits)));
    let mut said = format!("Change Tileset: {} laid in {}", d.area, d.to);
    if r.corners_missed > 0 || r.crossers_missed > 0 {
        said += &format!(
            "; {} corners' terrain or height and {} crosser edges have no tiles there",
            r.corners_missed, r.crossers_missed
        );
    }
    if r.groups > 0 {
        said += &format!("; {} of the old tileset's groups laid as terrain", r.groups);
    }
    if r.doors_gone + r.doors_new > 0 {
        said += &format!(
            "; {} doors on the old tiles' hooks taken away, {} brought by the new",
            r.doors_gone, r.doors_new
        );
    }
    app.log.info(said);
}
