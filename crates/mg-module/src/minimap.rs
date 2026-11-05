//! An area's minimap, as the game's map draws it: each tile's picture (the
//! tileset's `ImageMap2D`, usually `mi_` and the tile's model) turned as
//! the tile is, a quarter turn anticlockwise per orientation step, in the
//! area's grid with its first row at the bottom (south). The game client
//! draws the same (`tests/client_minimap.rs`). A tile without a picture is
//! black, as on the game's map.
//!
//! Map notes ([`map_notes`], [`mark`]) are marked as the game's map marks
//! them: an enabled one (`HasMapNote` and `MapNoteEnabled`) by the GUI
//! model `ctl_map_pin`'s picture, centered on the waypoint, a disabled one
//! not at all; the note's text shows only under the pointer there, so it
//! isn't drawn (`client_minimap`'s `reading_map_notes`).

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

/// The pixels a tile takes on the game's map: its minimap pictures' size,
/// by which the map note's marker is sized.
const MAP_TILE: f32 = 16.0;

/// The pixels of a GUI model's unit.
const GUI_UNIT: f32 = 100.0;

/// The marker the game's map draws for a map note: the picture of the GUI
/// model `ctl_map_pin` (its mesh's part of its texture, rows bottom first),
/// its size and where its middle is from the note, in the pixels of the
/// game's map (a tile 16 of them; y up).
#[derive(Debug, Clone)]
pub struct Pin {
    pub image: Rgba,
    pub size: [f32; 2],
    pub offset: [f32; 2],
}

/// The map note's marker, from the game data (a hak's `ctl_map_pin` or
/// its texture over the game's).
pub fn pin(rm: &ResMan) -> Result<Pin, String> {
    let name = ResRef::from_str("ctl_map_pin").map_err(|e| e.to_string())?;
    let data = rm.get(&mg_resman::ResKey::new(name, ResType::MDL)).map_err(|e| e.to_string())?;
    let model = mg_mdl::Model::read(&data).map_err(|e| format!("ctl_map_pin.mdl: {e}"))?;
    let (node, mesh, texture) = (model.nodes.iter())
        .find_map(|n| {
            let m = n.mesh()?;
            let t = m.textures[0].clone().filter(|t| !t.eq_ignore_ascii_case("null"))?;
            (!m.vertices.is_empty() && !m.uvs[0].is_empty()).then_some((n, m, t))
        })
        .ok_or("ctl_map_pin.mdl has no textured mesh")?;
    let bounds = |points: &mut dyn Iterator<Item = [f32; 2]>| {
        points.fold([f32::MAX, f32::MAX, f32::MIN, f32::MIN], |b, [x, y]| {
            [b[0].min(x), b[1].min(y), b[2].max(x), b[3].max(y)]
        })
    };
    let [x0, y0, x1, y1] = bounds(&mut mesh.vertices.iter().map(|v| [v[0], v[1]]));
    let [u0, v0, u1, v1] = bounds(&mut mesh.uvs[0].iter().copied());
    let texture = picture(rm, &texture).ok_or_else(|| format!("{texture}: not found"))?;
    // The texels the mesh shows (v counted from the bottom, as the rows).
    let (w, h) = (texture.width as f32, texture.height as f32);
    let cols = ((u0 * w).round().max(0.0) as u32, ((u1 * w).round() as u32).min(texture.width));
    let rows = ((v0 * h).round().max(0.0) as u32, ((v1 * h).round() as u32).min(texture.height));
    if cols.1 <= cols.0 || rows.1 <= rows.0 {
        return Err("ctl_map_pin.mdl shows none of its texture".into());
    }
    let mut image = Rgba::new(cols.1 - cols.0, rows.1 - rows.0);
    for y in 0..image.height {
        for x in 0..image.width {
            let px = texture.pixel(cols.0 + x, rows.0 + y);
            let i = (y as usize * image.width as usize + x as usize) * 4;
            image.data[i..i + 4].copy_from_slice(&px);
        }
    }
    let size = [(x1 - x0) * GUI_UNIT, (y1 - y0) * GUI_UNIT];
    let offset = [
        (node.position[0] + (x0 + x1) / 2.0) * GUI_UNIT,
        (node.position[1] + (y0 + y1) / 2.0) * GUI_UNIT,
    ];
    Ok(Pin { image, size, offset })
}

/// A map note the game's map marks: where (meters) and what it says.
#[derive(Debug, Clone, PartialEq)]
pub struct MapNote {
    pub tag: String,
    pub text: String,
    pub position: [f32; 2],
}

/// The map notes the game's map shows of an area (its GIT's root): its
/// waypoints with a map note that is enabled, in their order; `language`'s
/// text of the note, or its first, in the module's `codepage`.
pub fn map_notes(
    git: &Struct,
    language: mg_core::Language,
    codepage: mg_core::Codepage,
) -> Vec<MapNote> {
    let Some(mg_gff::Value::List(waypoints)) = git.get("WaypointList") else { return Vec::new() };
    let on = |w: &Struct, label: &str| w.integer(label).is_some_and(|v| v != 0);
    (waypoints.iter())
        .filter(|w| on(w, "HasMapNote") && on(w, "MapNoteEnabled"))
        .filter_map(|w| {
            let x = w.float("XPosition")?;
            let y = w.float("YPosition")?;
            let text = (w.locstring("MapNote"))
                .and_then(|l| {
                    l.get(language, mg_core::Gender::Male)
                        .or_else(|| l.strings.first().map(|(_, t)| t.as_slice()))
                })
                .map(|t| codepage.decode(t).into_owned())
                .unwrap_or_default();
            let tag = w.string("Tag").map(|t| codepage.decode(t).into_owned());
            Some(MapNote { tag: tag.unwrap_or_default(), text, position: [x, y] })
        })
        .collect()
}

/// Marks `notes` on a minimap (as [`minimap`] made it, `tile` pixels a
/// tile) with `pin`, sized as on the game's map (a tile 16 pixels there)
/// and laid over the map by its transparency; one partly off the map is
/// cut at its edge. Where in the picture each note's middle is (pixels
/// from the left and from the top, as a PNG's).
pub fn mark(image: &mut Rgba, tile: u32, notes: &[MapNote], pin: &Pin) -> Vec<[f32; 2]> {
    let scale = tile as f32 / MAP_TILE;
    let (pw, ph) = (pin.size[0] * scale, pin.size[1] * scale);
    let mut middles = Vec::new();
    for note in notes {
        // Meters to pixels: a tile is 10 meters, the first row south.
        let cx = note.position[0] / 10.0 * tile as f32 + pin.offset[0] * scale;
        let cy = note.position[1] / 10.0 * tile as f32 + pin.offset[1] * scale;
        middles.push([cx, image.height as f32 - cy]);
        let (left, bottom) = (cx - pw / 2.0, cy - ph / 2.0);
        let xs =
            left.floor().max(0.0) as u32..((left + pw).ceil().max(0.0) as u32).min(image.width);
        let ys = bottom.floor().max(0.0) as u32
            ..((bottom + ph).ceil().max(0.0) as u32).min(image.height);
        for y in ys {
            for x in xs.clone() {
                // The pin's texel under this pixel's middle (nearest).
                let u = (x as f32 + 0.5 - left) / pw;
                let v = (y as f32 + 0.5 - bottom) / ph;
                if !(0.0..1.0).contains(&u) || !(0.0..1.0).contains(&v) {
                    continue;
                }
                let sx = ((u * pin.image.width as f32) as u32).min(pin.image.width - 1);
                let sy = ((v * pin.image.height as f32) as u32).min(pin.image.height - 1);
                let src = pin.image.pixel(sx, sy);
                let a = u32::from(src[3]);
                let i = (y as usize * image.width as usize + x as usize) * 4;
                for (d, s) in image.data[i..i + 3].iter_mut().zip(src) {
                    *d = ((u32::from(s) * a + u32::from(*d) * (255 - a) + 127) / 255) as u8;
                }
            }
        }
    }
    middles
}

/// An area's minimap as [`draw`] makes it: the picture, and the map notes
/// marked on it, each with where its middle is (pixels from the left and
/// the top).
#[derive(Debug, Clone)]
pub struct Drawn {
    pub image: Rgba,
    pub notes: Vec<(MapNote, [f32; 2])>,
}

/// An area's minimap (its ARE's root), `tile` pixels a tile (or its
/// pictures' size), and, given its GIT's root, its map notes marked as the
/// game's map marks them (their text in the module's `codepage`).
pub fn draw(
    rm: &ResMan,
    are: &Struct,
    git: Option<&Struct>,
    tile: Option<u32>,
    codepage: mg_core::Codepage,
) -> Result<Drawn, String> {
    let set = tileset(rm, are)?;
    let mut image = minimap(rm, are, &set, tile)?;
    let Some(git) = git else { return Ok(Drawn { image, notes: Vec::new() }) };
    let notes = map_notes(git, mg_core::Language::ENGLISH, codepage);
    if notes.is_empty() {
        return Ok(Drawn { image, notes: Vec::new() });
    }
    let width = are.integer("Width").and_then(|w| u32::try_from(w).ok()).unwrap_or(1).max(1);
    let size = image.width / width;
    let middles = mark(&mut image, size, &notes, &pin(rm)?);
    Ok(Drawn { image, notes: notes.into_iter().zip(middles).collect() })
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

    /// A waypoint at `at` (meters) with a map note, enabled or not, or none.
    fn waypoint(tag: &str, at: [f32; 2], note: Option<(bool, &str)>) -> Struct {
        let mut w = crate::instances::walk_waypoint(tag.as_bytes(), [at[0], at[1], 0.0]);
        if let Some((enabled, text)) = note {
            w.set("HasMapNote", mg_gff::Value::Byte(1));
            w.set("MapNoteEnabled", mg_gff::Value::Byte(u8::from(enabled)));
            let mut l = mg_core::LocString::default();
            l.set(mg_core::Language::ENGLISH, mg_core::Gender::Male, text);
            w.set("MapNote", mg_gff::Value::LocString(l));
        }
        w
    }

    /// The map notes the game's map shows: the enabled ones alone (as the
    /// client's map showed four waypoints, `reading_map_notes`), with their
    /// text and where they are.
    #[test]
    fn the_enabled_map_notes_are_the_ones_shown() {
        let mut git = Struct::new(0);
        let list = vec![
            waypoint("ALPHA", [5.0, 25.0], Some((true, "Alpha note"))),
            waypoint("BETA", [25.0, 25.0], Some((false, "Beta note"))),
            waypoint("GAMMA", [35.0, 5.0], None),
            waypoint("DELTA", [32.0, 12.0], Some((true, "Delta note"))),
        ];
        git.set("WaypointList", mg_gff::Value::List(list));
        let notes = map_notes(&git, mg_core::Language::ENGLISH, mg_core::Codepage::default());
        let found: Vec<_> = notes.iter().map(|n| (n.tag.as_str(), n.text.as_str())).collect();
        assert_eq!(found, [("ALPHA", "Alpha note"), ("DELTA", "Delta note")]);
        assert_eq!(notes[1].position, [32.0, 12.0]);
        assert!(
            map_notes(&Struct::new(0), mg_core::Language::ENGLISH, Default::default()).is_empty()
        );
    }

    /// A note is marked by the pin centered on it, sized as on the game's
    /// map (12 of its 16 pixels a tile: 24 at 32), laid over by the pin's
    /// transparency; one at the map's edge is cut there.
    #[test]
    fn a_note_is_marked_by_the_pin_centered_on_it() {
        // A 2×2 pin, opaque red but for a clear top right.
        let mut image = Rgba::new(2, 2);
        for (i, px) in image.data.as_chunks_mut::<4>().0.iter_mut().enumerate() {
            *px = if i == 3 { [0, 0, 0, 0] } else { [255, 0, 0, 255] };
        }
        let pin = Pin { image, size: [12.0, 12.0], offset: [0.0, 0.0] };
        // A 2×1 area at 32 pixels a tile, gray.
        let mut map = Rgba::new(64, 32);
        for px in map.data.as_chunks_mut::<4>().0 {
            *px = [100, 100, 100, 255];
        }
        let note = |x, y| MapNote { tag: String::new(), text: String::new(), position: [x, y] };
        let middles = mark(&mut map, 32, &[note(10.0, 5.0), note(0.0, 0.0)], &pin);
        // 10 m east, 5 m north: (32, 16) from the bottom left, (32, 16)
        // from the top left too (the map is 32 high).
        assert_eq!(middles, [[32.0, 16.0], [0.0, 32.0]]);
        // 24 pixels: from 20 to 44 across and up.
        assert_eq!(map.pixel(20, 4), [255, 0, 0, 255], "bottom left");
        assert_eq!(map.pixel(19, 16), [100, 100, 100, 255], "left of it");
        assert_eq!(map.pixel(43, 10), [255, 0, 0, 255], "the bottom right quarter");
        assert_eq!(map.pixel(43, 27), [100, 100, 100, 255], "the clear quarter");
        assert_eq!(map.pixel(44, 16), [100, 100, 100, 255], "right of it");
        // The one at the corner: its top right quarter alone, on the map.
        assert_eq!(map.pixel(0, 0), [100, 100, 100, 255], "its clear quarter");
        assert_eq!(map.pixel(13, 13), [100, 100, 100, 255], "past it");
    }
}
