//! An area's minimap in the window, as the game's map draws it
//! ([`mg_module::minimap`]): the area view's map in its corner, and the
//! one Setup Area Transition picks its door or waypoint on.

use std::hash::{Hash, Hasher};

use egui::{Pos2, Rect};
use glam::Vec2;
use mg_core::{ResRef, ResType};
use mg_resman::ResKey;

use crate::Moonglow;

/// The pixels a tile takes in the map's picture.
const TILE_PIXELS: u32 = 32;

/// A tile's side, metres.
const TILE: f32 = 10.0;

/// An area's map, kept while its tiles stay as they are.
#[derive(Clone)]
pub(crate) struct Map {
    /// What it was drawn from ([`key`]).
    key: u64,
    /// Its picture (`None`: it couldn't be drawn, the tileset missing).
    pub(crate) texture: Option<egui::TextureHandle>,
    /// The area's size, in tiles.
    pub(crate) tiles: (u32, u32),
}

impl std::fmt::Debug for Map {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Map").field("tiles", &self.tiles).finish_non_exhaustive()
    }
}

/// What an area's map is drawn from: its tileset, its size and each
/// tile's id and turn.
fn key(are: &mg_gff::Struct) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    are.resref("Tileset").hash(&mut h);
    (are.integer("Width"), are.integer("Height")).hash(&mut h);
    for t in are.list("Tile_List").unwrap_or(&[]) {
        (t.integer("Tile_ID"), t.integer("Tile_Orientation")).hash(&mut h);
    }
    h.finish()
}

/// The map of `area` as the module has it now (unsaved tiles included):
/// `kept` while its tiles stay as they are, drawn anew when they change.
/// `None` without the area or game data.
pub(crate) fn of<'m>(
    app: &mut Moonglow,
    ctx: &egui::Context,
    area: ResRef,
    kept: &'m mut Option<Map>,
) -> Option<&'m Map> {
    let (Some(ws), Some(game)) = (app.ws.as_mut(), app.game.as_deref()) else { return None };
    let are = &ws.doc(&ResKey::new(area, ResType::ARE)).ok()?.root;
    let now = key(are);
    if kept.as_ref().is_none_or(|m| m.key != now) {
        let size = |label: &str| are.integer(label).and_then(|v| u32::try_from(v).ok());
        let tiles = (size("Width").unwrap_or(0), size("Height").unwrap_or(0));
        let set = are.resref("Tileset").and_then(|r| mg_area::tileset(game, r).ok());
        let picture = set.and_then(|set| {
            mg_module::minimap::minimap(&game.resman, are, &set, Some(TILE_PIXELS)).ok()
        });
        let texture = picture.map(|p| {
            let p = p.top_down();
            let image = egui::ColorImage::from_rgba_unmultiplied(
                [p.width as usize, p.height as usize],
                &p.data,
            );
            ctx.load_texture(format!("minimap {area}"), image, egui::TextureOptions::LINEAR)
        });
        *kept = Some(Map { key: now, texture, tiles });
    }
    kept.as_ref()
}

impl Map {
    /// The area's size, metres.
    fn metres(&self) -> Vec2 {
        Vec2::new(self.tiles.0 as f32, self.tiles.1 as f32) * TILE
    }

    /// The rectangle the map takes, its tiles square, at most `side`
    /// points on its longer side, its top right corner at `corner`.
    pub(crate) fn placed(&self, corner: Pos2, side: f32) -> Rect {
        let m = self.metres().max(Vec2::splat(TILE));
        let scale = side / m.x.max(m.y);
        let size = egui::vec2(m.x * scale, m.y * scale);
        Rect::from_min_size(corner - egui::vec2(size.x, 0.0), size)
    }

    /// Where a point of the area (metres; north is up) is on the map
    /// drawn in `rect`.
    pub(crate) fn to_map(&self, rect: Rect, p: Vec2) -> Pos2 {
        let m = self.metres().max(Vec2::ONE);
        Pos2::new(rect.left() + p.x / m.x * rect.width(), rect.bottom() - p.y / m.y * rect.height())
    }

    /// The point of the area under `pos` on the map drawn in `rect`
    /// (within the area).
    pub(crate) fn from_map(&self, rect: Rect, pos: Pos2) -> Vec2 {
        let m = self.metres();
        let x = (pos.x - rect.left()) / rect.width().max(1.0) * m.x;
        let y = (rect.bottom() - pos.y) / rect.height().max(1.0) * m.y;
        Vec2::new(x, y).clamp(Vec2::ZERO, m)
    }

    /// Draws the map in `rect`, framed (black where it couldn't be).
    pub(crate) fn draw(&self, painter: &egui::Painter, rect: Rect) {
        painter.rect_filled(rect.expand(2.0), 2.0, egui::Color32::from_black_alpha(200));
        if let Some(t) = &self.texture {
            let uv = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
            painter.image(t.id(), rect, uv, egui::Color32::WHITE);
        }
        painter.rect_stroke(
            rect.expand(1.0),
            1.0,
            egui::Stroke::new(1.0, egui::Color32::from_gray(140)),
            egui::StrokeKind::Outside,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_map_s_points_are_the_area_s_north_up() {
        let map = Map { key: 0, texture: None, tiles: (4, 2) };
        // A map 200 points wide: 40 by 20 m.
        let rect = map.placed(Pos2::new(300.0, 10.0), 200.0);
        assert_eq!(rect, Rect::from_min_size(Pos2::new(100.0, 10.0), egui::vec2(200.0, 100.0)));
        assert_eq!(map.to_map(rect, Vec2::ZERO), rect.left_bottom());
        assert_eq!(map.to_map(rect, Vec2::new(40.0, 20.0)), rect.right_top());
        let p = Vec2::new(12.5, 7.5);
        assert!((map.from_map(rect, map.to_map(rect, p)) - p).length() < 1e-4);
        // (Off the map: the area's edge.)
        assert_eq!(map.from_map(rect, Pos2::new(0.0, 500.0)), Vec2::ZERO);
    }

    #[test]
    fn a_map_is_drawn_again_when_its_tiles_change() {
        let mut are = mg_gff::Struct::new(0);
        are.set("Width", mg_gff::Value::Int(1));
        let tile = |id: i32| {
            let mut t = mg_gff::Struct::new(1);
            t.set("Tile_ID", mg_gff::Value::Int(id));
            t
        };
        are.set("Tile_List", mg_gff::Value::List(vec![tile(3)]));
        let before = key(&are);
        are.set("Name", mg_gff::Value::Int(7));
        assert_eq!(key(&are), before, "not for another field");
        are.set("Tile_List", mg_gff::Value::List(vec![tile(4)]));
        assert_ne!(key(&are), before);
    }
}
