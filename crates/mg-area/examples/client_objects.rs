//! A game client's area: built from values (what a server's messages say,
//! no files), then its objects moved, changed, added and taken out frame
//! by frame without building the scene again.
//!
//! `cargo run -p mg-area --example client_objects` (a GPU, no game: the
//! models are made here). A client builds `mg-area` without its editing
//! half: `mg-area = { ..., default-features = false }`.

use std::num::NonZeroU8;

use glam::Vec3;
use mg_area::values::{Area, Look, Object, Tile};
use mg_area::{AreaModel, AreaObject, AreaScene, Lighting, ObjectKind, Sky, View};
use mg_core::ResType;
use mg_preview::Preview;
use mg_render::Gpu;
use mg_resman::{MemContainer, ResKey, ResMan};
use mg_rules::GameData;

fn main() {
    let game = game();
    let Some(gpu) = Gpu::headless() else {
        eprintln!("no GPU");
        return;
    };

    // The area as values: its size, tiles (by the tileset's number, turn,
    // height, lights, flames and loops), lighting and objects.
    let tiles = (0..4)
        .map(|i| Tile {
            orientation: i as u8,
            main_lights: [3, 0],
            // A flame: `fx_flame01`'s animation "2" at the tile's `sl1` (the
            // file's value; Moonglow Client saw a server send one less, once).
            source_lights: [NonZeroU8::new(2), None],
            ..Tile::new(0)
        })
        .collect();
    let lighting = Lighting {
        sun: Sky { ambient: 0x0040_4040, diffuse: 0x00c0_c0c0, ..Sky::default() },
        fog_clip: 30.0,
        ..Lighting::default()
    };
    let guard = Object::facing(
        ObjectKind::Creature,
        Vec3::new(5.0, 5.0, 0.0),
        90f32.to_radians(),
        // (A creature's look as `mg_preview` assembles it: from what the
        // server says it is and wears, a `DressedLook`, with
        // `Look::creature`, or its fields with `Look::read`. Here, a model
        // made below.)
        Look::Model(Preview::model("ex_pole")),
    );
    let chest = Object::new(ObjectKind::Placeable, Vec3::new(14.0, 6.0, 0.0), 0.0, model("ex_box"));
    let area = Area {
        width: 2,
        height: 2,
        tiles,
        lighting,
        objects: vec![guard, chest],
        ..Area::default()
    };
    // (The tileset, read with `mg_area::tileset`, gives the tiles their
    // models; here they are given by hand.)
    let mut area = AreaModel::from_values(&game, area, None);
    for t in &mut area.tiles {
        t.model = Some("ex_tile".into());
    }
    let mut loaded = AreaScene::new(&gpu, &game, &area);
    let view = View { time: 0.0, ..View::of(&area) };

    for frame in 0..120 {
        // The guard walks: moved in the model, drawn there next frame.
        let walked = Vec3::new(5.0 + frame as f32 * 0.05, 5.0, 0.0);
        area.objects[0].move_to(walked, 0.0);
        // Now and then a server says more.
        match frame {
            // Another object comes: pushed, then shown.
            30 => {
                let crate_ = Object::new(
                    ObjectKind::Placeable,
                    Vec3::new(4.0, 15.0, 0.0),
                    0.0,
                    model("ex_box"),
                );
                area.objects.push(AreaObject::from_values(1, crate_));
                loaded.update_objects(&gpu, &game, &area, &[]);
            }
            // The guard's look changes: shown anew.
            60 => {
                area.objects[0].preview = Some(Preview::model("ex_box"));
                loaded.update_objects(&gpu, &game, &area, &[0]);
            }
            // The chest goes: out of the model and the scene alike.
            90 => {
                area.objects.remove(1);
                loaded.remove_object(&area, 1);
            }
            _ => {}
        }
        let scene = loaded.scene(&area, &View { time: frame as f32 / 60.0, ..view });
        if frame % 30 == 0 || frame == 119 {
            println!(
                "frame {frame}: {} objects, {} instances drawn, {} lights",
                area.objects.len(),
                scene.instances.len(),
                scene.lights.len()
            );
        }
    }
}

fn model(name: &str) -> Look {
    Look::Model(Preview::model(name))
}

/// Game data of three models of one square each (a tile's, and two
/// objects').
fn game() -> GameData {
    let square = |name: &str, h: f32| {
        format!(
            "newmodel {name}\nsetsupermodel {name} NULL\nclassification character\n\
beginmodelgeom {name}\nnode dummy {name}\n  parent NULL\nendnode\n\
node trimesh {name}_mesh\n  parent {name}\n  bitmap white\n  verts 4\n    -{h} -{h} 0\n    \
{h} -{h} 0\n    {h} {h} 0\n    -{h} {h} 1\n  faces 2\n    0 1 2 1 0 1 2 1\n    0 2 3 1 0 2 3 1\n\
endnode\nendmodelgeom {name}\ndonemodel {name}\n"
        )
    };
    let mut c = MemContainer::new();
    for (name, h) in [("ex_tile", 5.0), ("ex_box", 0.5), ("ex_pole", 0.25)] {
        c.insert(ResKey::parse(name, ResType::MDL).unwrap(), square(name, h).into_bytes());
    }
    let mut rm = ResMan::new();
    rm.add(mg_resman::priority::KEY, "game", mg_resman::LayerClass::Key, c);
    GameData::new(rm, mg_tlk::Tlk::new(mg_core::Language::ENGLISH))
}
