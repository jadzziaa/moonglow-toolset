//! A tile of a tileset, to look at: what a double click on its picture in
//! the tiles palette's Gallery opens. Its model in the 3D viewer, beside
//! what the tileset says of it: its corners' terrain and height, its edges'
//! crossers, its lights, loops, doors and sounds, and the group it belongs
//! to. Read only: a tileset of the game's is not the module's to change
//! (the tileset editor edits one on disk).

use std::sync::Arc;

use mg_core::{ResRef, ResType};
use mg_resman::ResKey;
use mg_set::{CORNERS, EDGES, Tileset};

use crate::Moonglow;
use crate::model_view::Source;
use crate::widgets::{field_label, section_heading};

/// Tileset `tileset`, as an open area has it, else read from the game.
fn tileset_of(app: &Moonglow, tileset: ResRef) -> Option<Arc<Tileset>> {
    (app.area_views.values().filter_map(|v| v.terrain.as_ref()))
        .find(|t| t.tileset == tileset)
        .map(|t| t.set.clone())
        .or_else(|| {
            app.game.as_deref().and_then(|g| mg_area::tileset(g, tileset).ok()).map(Arc::new)
        })
}

/// The view of tile `tile` of `tileset`.
pub(crate) fn ui(app: &mut Moonglow, ui: &mut egui::Ui, tileset: ResRef, tile: u32) {
    let Some(set) = tileset_of(app, tileset) else {
        ui.label(format!("The tileset {tileset} could not be read."));
        return;
    };
    let Some(t) = set.tiles.get(tile as usize).cloned() else {
        ui.label(format!("{tileset} has no tile {tile}."));
        return;
    };
    egui::Panel::left(egui::Id::new(("tile-facts", tileset, tile)))
        .resizable(true)
        .default_size(300.0)
        .show(ui, |ui| {
            egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                facts(app, ui, &set, tileset, tile, &t);
            });
        });
    egui::CentralPanel::default().show(ui, |ui| {
        let Some(key) = ResKey::parse(&t.model.to_ascii_lowercase(), ResType::MDL) else {
            ui.label(format!("{} is not a model's name.", t.model));
            return;
        };
        if crate::model_view::embedded(app, ui, Source::Resource(key)) {
            app.actions.push(crate::Action::OpenTab(crate::Tab::Model(key)));
        }
    });
}

/// What the tileset says of the tile, and a button to paint with it.
fn facts(
    app: &mut Moonglow,
    ui: &mut egui::Ui,
    set: &Tileset,
    tileset: ResRef,
    tile: u32,
    t: &mg_set::Tile,
) {
    ui.heading(&t.model);
    let name = set.general.unlocalized_name.as_deref().unwrap_or("");
    ui.weak(format!("Tile {tile} of {tileset} {name}"));
    let group = set.groups.iter().find(|g| g.tiles.contains(&Some(tile)));
    if let Some(g) = group {
        ui.weak(format!("A piece of the group {}: placed with it", g.name));
    } else if ui
        .button("Paint with it")
        .on_hover_text("Choose it in the tiles palette: a click in the area places it")
        .clicked()
    {
        let brush = crate::terrain_mode::TileBrush {
            tileset,
            label: t.model.clone(),
            brush: mg_area::terrain::Brush::Tile(tile),
            target: None,
        };
        app.palette.tile_brush = Some(brush);
        app.palette.tile_choice = Some(tile);
        app.palette.selected = None;
    }
    ui.add_space(6.0);
    section_heading(ui, "Corners");
    egui::Grid::new(("tile-corners", tileset, tile)).num_columns(2).show(ui, |ui| {
        for (name, (terrain, height)) in CORNERS.iter().zip(&t.corners) {
            field_label(ui, *name);
            ui.label(format!("{terrain}, height {height}"));
            ui.end_row();
        }
    });
    section_heading(ui, "Edges");
    egui::Grid::new(("tile-edges", tileset, tile)).num_columns(2).show(ui, |ui| {
        for (name, crosser) in EDGES.iter().zip(&t.edges) {
            field_label(ui, *name);
            ui.label(if crosser.is_empty() { "—" } else { crosser.as_str() });
            ui.end_row();
        }
    });
    section_heading(ui, "The rest");
    let yes = |b: bool| if b { "yes" } else { "no" };
    let named = |n: &Option<String>| n.clone().unwrap_or_else(|| "—".into());
    egui::Grid::new(("tile-rest", tileset, tile)).num_columns(2).show(ui, |ui| {
        let mut row = |label: &str, value: String| {
            field_label(ui, label);
            ui.label(value);
            ui.end_row();
        };
        row("Walkmesh", named(&t.walkmesh));
        row("Main lights", format!("{}, {}", yes(t.main_lights[0]), yes(t.main_lights[1])));
        row("Source lights", format!("{}, {}", yes(t.source_lights[0]), yes(t.source_lights[1])));
        let loops = t.anim_loops.map(yes).join(", ");
        row("Animation loops", loops);
        row("Sounds", t.sounds.to_string());
        row("Path node", named(&t.path_node));
        row("Orientation", format!("{}°", t.orientation));
        row("Visibility node", named(&t.visibility_node));
        row("Map image", named(&t.image_map_2d));
        row("Doors", t.doors.len().to_string());
    });
    for (i, d) in t.doors.iter().enumerate() {
        ui.weak(format!(
            "Door {}: doortypes.2da row {}, at ({:.2}, {:.2}, {:.2}), turned {}°",
            i + 1,
            d.door_type,
            d.position[0],
            d.position[1],
            d.position[2],
            d.orientation
        ));
    }
}
