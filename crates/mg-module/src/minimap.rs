//! An area's minimap, as the game's map draws it: each tile's picture (the
//! tileset's `ImageMap2D`, usually `mi_` and the tile's model) turned as
//! the tile is, a quarter turn anticlockwise per orientation step, in the
//! area's grid with its first row at the bottom (south). The game client
//! draws the same (`tests/client_minimap.rs`). A tile without a picture is
//! black, as on the game's map.

use mg_core::{ResRef, ResType};
use mg_gff::Struct;
use mg_image::Rgba;
use mg_resman::ResMan;
use mg_set::Tileset;

/// The pixels one tile takes when not asked: the largest picture's, within
/// these bounds.
const SIZE: (u32, u32) = (16, 256);

/// A tile's picture, as RGBA (rows bottom first).
fn picture(rm: &ResMan, name: &str) -> Option<Rgba> {
    let resref = ResRef::from_str(&name.to_ascii_lowercase()).ok()?;
    let (restype, data) = rm.texture(resref)?;
    Some(mg_image::read(restype, &data).ok()?.to_rgba())
}

/// A picture scaled to `size` pixels square (nearest pixel) and turned
/// `turns` quarter turns anticlockwise.
fn place(out: &mut Rgba, at: (u32, u32), size: u32, image: &Rgba, turns: u32) {
    for y in 0..size {
        for x in 0..size {
            // The output pixel (x, y), turned back to the picture's frame.
            let (mut u, mut v) = (x, y);
            for _ in 0..turns % 4 {
                // Anticlockwise forward is (u, v) -> (size-1-v, u); back is
                // (u, v) -> (v, size-1-u).
                (u, v) = (v, size - 1 - u);
            }
            let sx = (u * image.width / size).min(image.width - 1);
            let sy = (v * image.height / size).min(image.height - 1);
            let px = image.pixel(sx, sy);
            let (ox, oy) = (at.0 + x, at.1 + y);
            let i = (oy as usize * out.width as usize + ox as usize) * 4;
            out.data[i..i + 4].copy_from_slice(&[px[0], px[1], px[2], 255]);
        }
    }
}

/// The minimap of an area (its ARE's root), `tile` pixels a tile (or its
/// pictures' size). Rows bottom first, as [`Rgba`] keeps them.
pub fn minimap(
    rm: &ResMan,
    are: &Struct,
    set: &Tileset,
    tile: Option<u32>,
) -> Result<Rgba, String> {
    let width = are.integer("Width").ok_or("the area has no width")?;
    let height = are.integer("Height").ok_or("the area has no height")?;
    let (width, height) = (
        u32::try_from(width).map_err(|_| "bad width")?,
        u32::try_from(height).map_err(|_| "bad height")?,
    );
    let tiles = match are.get("Tile_List") {
        Some(mg_gff::Value::List(l)) => l.as_slice(),
        _ => &[],
    };
    let mut cache: std::collections::HashMap<String, Option<Rgba>> = Default::default();
    let pictures: Vec<(Option<Rgba>, u32)> = tiles
        .iter()
        .map(|t| {
            let id = t.integer("Tile_ID").unwrap_or(-1);
            let turns = t.integer("Tile_Orientation").unwrap_or(0).rem_euclid(4) as u32;
            let name = usize::try_from(id)
                .ok()
                .and_then(|i| set.tiles.get(i))
                .and_then(|t| t.image_map_2d.clone())
                .filter(|n| !n.is_empty() && !n.eq_ignore_ascii_case("(null)"));
            let image = name
                .and_then(|n| cache.entry(n.clone()).or_insert_with(|| picture(rm, &n)).clone());
            (image, turns)
        })
        .collect();
    let size = tile.unwrap_or_else(|| {
        let largest =
            pictures.iter().filter_map(|(p, _)| p.as_ref()).map(|p| p.width.max(p.height)).max();
        largest.unwrap_or(SIZE.0).clamp(SIZE.0, SIZE.1)
    });
    if size == 0 || size > 1024 {
        return Err("a tile is 1 to 1024 pixels".into());
    }
    let mut out = Rgba::new(width * size, height * size);
    for px in out.data.as_chunks_mut::<4>().0 {
        px[3] = 255;
    }
    for (i, (image, turns)) in pictures.iter().enumerate() {
        let (col, row) = (i as u32 % width, i as u32 / width);
        if row >= height {
            break;
        }
        if let Some(image) = image {
            place(&mut out, (col * size, row * size), size, image, *turns);
        }
    }
    Ok(out)
}

/// The tileset an ARE names, from the game data.
pub fn tileset(rm: &ResMan, are: &Struct) -> Result<Tileset, String> {
    let name = are.resref("Tileset").ok_or("the area names no tileset")?;
    let data = rm.get(&mg_resman::ResKey::new(name, ResType::SET)).map_err(|e| e.to_string())?;
    Tileset::parse(&data, mg_core::Codepage::default()).map_err(|e| format!("{name}.set: {e}"))
}

/// A picture as a PNG (top row first, as PNGs are).
pub fn png(image: &Rgba) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    let mut enc = png::Encoder::new(&mut out, image.width, image.height);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    let mut w = enc.write_header().map_err(|e| e.to_string())?;
    w.write_image_data(&image.top_down().data).map_err(|e| e.to_string())?;
    w.finish().map_err(|e| e.to_string())?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A 2×2 picture: bottom-left red, bottom-right green, top-left blue,
    /// top-right white.
    fn quarters() -> Rgba {
        let mut p = Rgba::new(2, 2);
        let colors = [[255, 0, 0, 255], [0, 255, 0, 255], [0, 0, 255, 255], [255, 255, 255, 255]];
        for (i, c) in colors.iter().enumerate() {
            p.data[i * 4..i * 4 + 4].copy_from_slice(c);
        }
        p
    }

    #[test]
    fn a_quarter_turn_anticlockwise() {
        let mut out = Rgba::new(2, 2);
        place(&mut out, (0, 0), 2, &quarters(), 1);
        // Turned anticlockwise: bottom right to top right, bottom left to
        // bottom right, top left to bottom left, top right to top left.
        assert_eq!(out.pixel(1, 1), [0, 255, 0, 255]);
        assert_eq!(out.pixel(1, 0), [255, 0, 0, 255]);
        assert_eq!(out.pixel(0, 0), [0, 0, 255, 255]);
        assert_eq!(out.pixel(0, 1), [255, 255, 255, 255]);
        let mut half = Rgba::new(2, 2);
        place(&mut half, (0, 0), 2, &quarters(), 2);
        assert_eq!(half.pixel(1, 1), [255, 0, 0, 255]);
        // Scaled: each picture pixel becomes a block.
        let mut big = Rgba::new(4, 4);
        place(&mut big, (0, 0), 4, &quarters(), 0);
        assert_eq!(big.pixel(1, 1), [255, 0, 0, 255]);
        assert_eq!(big.pixel(3, 3), [255, 255, 255, 255]);
        let png = png(&big).unwrap();
        assert_eq!(&png[1..4], b"PNG");
    }
}
