//! The Resize Area and Rotate Area windows (Aurora's Edit menu): the
//! area shown last, resized at its north and east edges or turned a
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
}

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
    app.resize_area = Some(ResizeDraft { area, rows: size("Height"), columns: size("Width") });
}

pub(crate) fn windows(app: &mut Moonglow, ctx: &egui::Context) {
    resize_window(app, ctx);
    rotate_window(app, ctx);
}

fn resize_window(app: &mut Moonglow, ctx: &egui::Context) {
    let Some(mut d) = app.resize_area.take() else { return };
    let mut open = true;
    let (mut ok, mut cancel) = (false, false);
    egui::Window::new("Resize Area")
        .pivot(egui::Align2::CENTER_CENTER)
        .default_pos(ctx.content_rect().center())
        .open(crate::widgets::open_unless_escape(ctx, "Resize Area", &mut open))
        .collapsible(false)
        .resizable(false)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                egui::Grid::new("resize").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
                    crate::widgets::field_label(ui, "Rows");
                    ui.add(egui::DragValue::new(&mut d.rows).range(2..=32))
                        .on_hover_text("Number of rows in the area");
                    ui.end_row();
                    crate::widgets::field_label(ui, "Columns");
                    ui.add(egui::DragValue::new(&mut d.columns).range(2..=32))
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
            ui.weak("Rows and columns come and go at the north and east edges.");
            ui.horizontal(|ui| {
                ok = ui.button("OK").on_hover_text("Accept changes").clicked()
                    || crate::widgets::enter(ui);
                cancel = crate::widgets::cancel_discard(ui);
            });
        });
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
    egui::Window::new("Rotate Area")
        .pivot(egui::Align2::CENTER_CENTER)
        .default_pos(ctx.content_rect().center())
        .open(crate::widgets::open_unless_escape(ctx, "Rotate Area", &mut open))
        .collapsible(false)
        .resizable(false)
        .show(ctx, |ui| {
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
        });
    if ok {
        rotate(app, &d);
    } else if open && !cancel {
        app.rotate_area = Some(d);
    }
}

fn resize(app: &mut Moonglow, d: &ResizeDraft) {
    let (Some(ws), Some(game)) = (app.ws.as_mut(), app.game.as_deref()) else { return };
    let (are_key, git_key) = (ResKey::new(d.area, ResType::ARE), ResKey::new(d.area, ResType::GIT));
    let Ok(are) = ws.doc(&are_key).map(|g| g.root.clone()) else { return };
    let git = ws.doc(&git_key).map(|g| g.root.clone()).unwrap_or_default();
    let tileset = are.resref("Tileset").unwrap_or(ResRef::EMPTY);
    let Ok(set) = mg_area::tileset(game, tileset) else {
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
    let reshaped = mg_area::reshape::resize(
        are_key,
        git_key,
        &are,
        &git,
        &set,
        &index,
        d.columns,
        d.rows,
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
