//! The area editor's tools (M4's U4; the proposal's sections 6.1 and
//! 9.1, the Main and AreaPaint boards): the tool shelf at the view's
//! left (Select Objects, Select Tiles, Paint, a separator, Terrain), one
//! tool on at a time whichever way it was chosen (the shelf, F10, B,
//! Escape, the tiles palette's brushes and its Select Tiles); and the
//! view's top right corner, the compass with Reorient Camera and Go to
//! Start Location under it.
//!
//! Terrain (Part 2, decision 8): there is no terrain mode, the terrain
//! brushes are the tiles palette's. The button is on while one of the
//! tiles palette's brushes is in hand (choosing one turns it on), and a
//! click on it shows the Palette on its Tiles type (on, it puts the brush
//! down: Select Objects is on again).
//!
//! `Settings::hidden_area_tools` keeps its names: "Select Tiles" and
//! "Paint" leave the shelf, "Reorient Camera" and "Go to Start Location"
//! the corner (their keys and View › Go to Start Location stay).

use egui::Ui;
use mg_uikit::Icon;
use mg_uikit::overlay::{ViewButton, corner_column, paint_compass};
use mg_uikit::shelf::{Tool, ToolShelf};

use super::{AreaView, go_to_start, look_at_start};
use crate::Moonglow;
use crate::keys::Cmd;

/// The shelf's accessible name (the board's "Tools"; named apart from the
/// Top Bar's Tools menu).
pub(crate) const AREA_TOOLS: &str = "Area tools";
/// The compass and its buttons' group.
pub(crate) const COMPASS: &str = "Compass";
pub(crate) const SELECT_OBJECTS: &str = "Select Objects";
pub(crate) const SELECT_TILES: &str = "Select Tiles";
pub(crate) const PAINT: &str = "Paint";
pub(crate) const TERRAIN: &str = "Terrain";
pub(crate) const REORIENT: &str = "Reorient Camera";
pub(crate) const GO_TO_START: &str = "Go to Start Location";

/// The tool in hand in an area's view: one at a time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AreaTool {
    /// Objects are selected, moved and placed (no other tool on).
    Select,
    /// Select Tiles (F10): tiles are selected rather than objects.
    Tiles,
    /// The placeable brush (B).
    Paint,
    /// One of the tiles palette's brushes in hand.
    Terrain,
}

/// The tool in hand in `view` (Paint and Select Tiles are off in a
/// read-only area, and so is a tile brush, `terrain_mode::active`).
pub(crate) fn tool_of(app: &Moonglow, view: &AreaView) -> AreaTool {
    let editable = view.read_only.is_none();
    if app.paint && editable {
        AreaTool::Paint
    } else if view.tile_mode && editable {
        AreaTool::Tiles
    } else if crate::terrain_mode::active(app, view).is_some() {
        AreaTool::Terrain
    } else {
        AreaTool::Select
    }
}

/// Select Tiles on or off in `view`: what was selected lets go when it
/// changes (as F10 has always done).
fn set_tile_mode(view: &mut AreaView, on: bool) {
    if view.tile_mode != on {
        view.tile_mode = on;
        view.selection.clear();
        view.tile_selection.clear();
    }
}

/// `tool` taken in `view`, the others put down: what every way of
/// choosing a tool calls (the shelf, F10, B). Nothing that changes the
/// area is taken in a read-only one. Terrain shows the Palette on its
/// Tiles type, to choose a brush from (none is in hand until one is).
pub(crate) fn choose(app: &mut Moonglow, view: &mut AreaView, tool: AreaTool) {
    if view.read_only.is_some() && tool != AreaTool::Select {
        return;
    }
    match tool {
        AreaTool::Select => {
            app.paint = false;
            app.palette.tile_brush = None;
            set_tile_mode(view, false);
        }
        AreaTool::Tiles => {
            app.paint = false;
            app.palette.tile_brush = None;
            set_tile_mode(view, true);
        }
        AreaTool::Paint => {
            app.paint = true;
            app.palette.tile_brush = None;
            set_tile_mode(view, false);
        }
        AreaTool::Terrain => {
            app.paint = false;
            set_tile_mode(view, false);
            // (A brush already chosen, in the palette: it stays shown.)
            if app.palette.tile_brush.is_none() {
                show_tiles_palette(app);
            }
        }
    }
}

/// [`choose`] in `area`'s view, where one is open (the tiles palette's
/// brushes and its Select Tiles, the Tile window's Paint with It: they
/// run while no view is drawn). Without a view, Paint is put down where a
/// brush or Select Tiles is taken.
pub(crate) fn choose_in(app: &mut Moonglow, area: mg_core::ResRef, tool: AreaTool) {
    match app.area_views.remove(&area) {
        Some(mut view) => {
            choose(app, &mut view, tool);
            app.area_views.insert(area, view);
        }
        None if tool != AreaTool::Paint => app.paint = false,
        None => {}
    }
}

/// The tool's shelf button clicked (or its key pressed): on, or off
/// again where it was on (back to Select Objects).
pub(crate) fn toggle(app: &mut Moonglow, view: &mut AreaView, tool: AreaTool) {
    let now = tool_of(app, view);
    if now == tool && tool != AreaTool::Select {
        choose(app, view, AreaTool::Select);
    } else {
        choose(app, view, tool);
    }
}

/// The Palette in the Right Dock, on its Tiles type (the Terrain tool's
/// click), as the palette's own Tiles type button shows it.
fn show_tiles_palette(app: &mut Moonglow) {
    let p = &mut app.palette;
    (p.tiles, p.prefabs, p.lights) = (true, false, false);
    (p.start, p.placing_start, p.placing_light) = (false, false, None);
    app.settings.palette_type = Some("tiles".to_string());
    app.actions.push(crate::Action::ShowPanel(crate::Panel::Palette));
}

/// Draws the shelf down `ui`'s height at its left and takes the tool
/// clicked.
pub(super) fn shelf(app: &mut Moonglow, ui: &mut Ui, view: &mut AreaView) {
    let hidden = &app.settings.hidden_area_tools;
    let shown = |name: &str| !hidden.iter().any(|h| h == name);
    let editable = view.read_only.is_none();
    let now = tool_of(app, view);
    let ctx = ui.ctx().clone();
    let keyed = |words: &str, cmd: Cmd| app.keymap.titled(words, cmd, &ctx);
    let mut tools = vec![AreaTool::Select];
    let mut shelf = ToolShelf::new().name(AREA_TOOLS).tool(
        Tool::new(Icon::Select, SELECT_OBJECTS).on(now == AreaTool::Select).tip("Select objects"),
    );
    if shown(SELECT_TILES) {
        let tip = keyed("Select tiles rather than objects", Cmd::SelectTiles);
        let tool = Tool::new(crate::icons::SELECT_TILES, SELECT_TILES).on(now == AreaTool::Tiles);
        shelf = shelf.tool(tool.enabled(editable).tip(tip));
        tools.push(AreaTool::Tiles);
    }
    if shown(PAINT) {
        let tip = keyed(
            "Paint the placeables chosen in the palette onto the ground",
            Cmd::PaintPlaceables,
        );
        let tool = Tool::new(crate::icons::PAINT, PAINT).on(now == AreaTool::Paint);
        shelf = shelf.tool(tool.enabled(editable).tip(tip));
        tools.push(AreaTool::Paint);
    }
    let terrain = Tool::new(Icon::Terrain, TERRAIN)
        .on(now == AreaTool::Terrain)
        .enabled(editable)
        .tip("Paint terrain with the tile palette's brushes");
    shelf = shelf.separator().tool(terrain);
    tools.push(AreaTool::Terrain);
    let out = shelf.show(ui);
    if let Some(tool) = out.clicked.and_then(|i| tools.get(i)) {
        toggle(app, view, *tool);
    }
}

/// Turns the view's camera to face north again (the compass's Reorient
/// Camera, `Id::ReorientCamera`).
pub(crate) fn reorient(view: &mut AreaView) {
    if let Some(o) = &mut view.orbit {
        o.yaw = -std::f32::consts::FRAC_PI_2;
    }
}

/// Where north is on the screen in `view`: radians clockwise from the
/// view's up (0 with the camera facing north, as Reorient Camera turns
/// it).
pub(crate) fn north_on_screen(view: &AreaView) -> f32 {
    view.orbit.map_or(0.0, |o| north_of(&o))
}

/// [`north_on_screen`] for the camera `o`.
fn north_of(o: &super::Orbit) -> f32 {
    let camera = o.camera();
    let Some(ahead) = (camera.target - camera.eye).truncate().try_normalize() else {
        return 0.0;
    };
    // (North, +y, against the screen's up, `ahead` on the ground, and its
    // right, `ahead` turned a quarter clockwise seen from above.)
    (-ahead.x).atan2(ahead.y)
}

/// The view's top right corner: the compass, and Reorient Camera and Go
/// to Start Location under it (those not hidden). Where they are is kept
/// in `view.corner_rect`, which the view's input leaves alone.
pub(super) fn corner(app: &mut Moonglow, ui: &mut Ui, view: &mut AreaView) {
    let hidden = &app.settings.hidden_area_tools;
    let shown = |name: &str| !hidden.iter().any(|h| h == name);
    let buttons: Vec<&str> = [REORIENT, GO_TO_START].into_iter().filter(|n| shown(n)).collect();
    let (compass, rects) = corner_column(view.rect, buttons.len());
    let t = mg_uikit::tokens(ui.ctx());
    let painter = ui.painter_at(view.rect);
    paint_compass(&painter, &t, compass.center(), north_on_screen(view));
    let column = rects.iter().fold(compass, |all, r| all.union(*r));
    view.corner_rect = Some(column);
    let ctx = ui.ctx().clone();
    let start_key = app.keymap.label_of(&crate::commands::Id::GoToStart.id(), &ctx);
    let mut clicked = None;
    // (In a scope of the column's own: one at the cursor, under the view,
    // would make the tab's content taller than its room.)
    ui.scope_builder(egui::UiBuilder::new().max_rect(column), |ui| {
        mg_uikit::name_group(ui, COMPASS);
        for (name, rect) in buttons.iter().zip(&rects) {
            let button =
                match *name {
                    REORIENT => ViewButton::new(crate::icons::CAMERA, REORIENT)
                        .tip(app.keymap.titled("Reorient Camera", Cmd::ReorientCamera, &ctx)),
                    _ => {
                        let words = "Go to Start Location";
                        let tip = match start_key.as_str() {
                            "" => words.to_string(),
                            k => format!("{words} ({k})"),
                        };
                        ViewButton::new(crate::icons::START, GO_TO_START).tip(tip)
                    }
                };
            if ui.put(*rect, button).clicked() {
                clicked = Some(*name);
            }
        }
    });
    match clicked {
        Some(REORIENT) => reorient(view),
        Some(GO_TO_START) => {
            // In another area: that area's view is opened at it, as
            // Aurora does.
            if !look_at_start(app, view) {
                go_to_start(app);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The compass's needle points where north is on the screen, the
    /// camera's heading: looking down on the ground (no foreshortening),
    /// a point north of the camera's target is drawn that way from it
    /// (the view's up turned by the angle, clockwise), at every yaw; and
    /// Reorient Camera's yaw has north up.
    #[test]
    fn the_needle_points_where_north_is_drawn() {
        for k in 0..16 {
            let yaw = k as f32 * std::f32::consts::TAU / 16.0 - 2.0;
            let o = super::super::Orbit {
                target: glam::Vec3::new(20.0, 20.0, 0.0),
                distance: 30.0,
                yaw,
                pitch: 1.55,
            };
            let camera = o.camera();
            let m = camera.projection(1.5) * camera.view();
            let ndc = |p: glam::Vec3| {
                let c = m * p.extend(1.0);
                glam::Vec2::new(c.x / c.w, c.y / c.w)
            };
            let d = ndc(o.target + glam::Vec3::Y) - ndc(o.target);
            // (Clockwise from up: NDC's y is up, as the screen's is not.
            // The aspect stretches x: undone.)
            let drawn = (d.x * 1.5).atan2(d.y);
            let off = (north_of(&o) - drawn + std::f32::consts::PI)
                .rem_euclid(std::f32::consts::TAU)
                - std::f32::consts::PI;
            assert!(off.abs() < 0.02, "yaw {yaw}: needle {} drawn {drawn}", north_of(&o));
        }
        let o = super::super::Orbit {
            target: glam::Vec3::ZERO,
            distance: 30.0,
            yaw: -std::f32::consts::FRAC_PI_2,
            pitch: 0.9,
        };
        assert!(north_of(&o).abs() < 1e-4, "reoriented: north up");
    }
}
