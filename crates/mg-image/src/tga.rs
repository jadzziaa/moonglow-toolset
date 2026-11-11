//! TGA: uncompressed and run-length encoded, true colour (15/16, 24, 32
//! bits), greyscale (8 bits) and colour-mapped. The game's are mostly
//! bottom-left 24- and 32-bit files; 32-bit files that declare no alpha
//! bits still carry alpha in the fourth byte, which is used.

use mg_core::bin::Reader;

use crate::{ImageError, Rgba, Texture, check_size};

/// Reads a TGA as a picture shows it (a NUI window's image, a plugin's):
/// rows from the top and columns from the left, by the origin its header
/// declares. ([`read`] keeps them as stored, as the game takes a model's
/// texture; a plugin's copy turned every file over, a top-left one upside
/// down.) A 15- or 16-bit true-colour picture's five-bit levels are
/// `c × 255 / 31` rounded down, as the client draws a NUI image of one
/// (4 is 32, 30 is 246: `nui_readings_in_the_client` in `mg-corpus-tests`'
/// `client_render.rs`), where its model textures take [`read`]'s.
pub fn read_picture(data: &[u8]) -> Result<Rgba, ImageError> {
    let mut rgba = read(data)?.to_rgba();
    let (image_type, bits) = (data.get(2).copied(), data.get(16).copied());
    if matches!(image_type, Some(2 | 10)) && matches!(bits, Some(15 | 16)) {
        for p in rgba.data.as_chunks_mut::<4>().0 {
            for c in &mut p[..3] {
                // ([`color`]'s level keeps the five bits on top.)
                *c = ((u32::from(*c >> 3) * 255) / 31) as u8;
            }
        }
    }
    let descriptor = data.get(17).copied().unwrap_or(0);
    if descriptor & 0x20 == 0 {
        rgba = rgba.top_down();
    }
    if descriptor & 0x10 != 0 {
        for row in rgba.data.chunks_exact_mut(rgba.width as usize * 4) {
            row.as_chunks_mut::<4>().0.reverse();
        }
    }
    Ok(rgba)
}

/// Whether a TGA is a grayscale one (image type 3, or 11 run-length
/// encoded): the game draws a PLT of the same name over it on a colored
/// part, where it draws a true-color TGA over the PLT (the game's
/// `pmd0_head001` and `pmh0_handl003` are such gray TGAs beside their
/// PLTs). Too short a header: not.
pub fn is_gray(data: &[u8]) -> bool {
    matches!(data.get(2), Some(3 | 11))
}

/// An uncompressed TGA of a picture, rows as [`Rgba`] keeps them (the
/// bottom first, the header's bottom-left origin): 32-bit with its
/// alpha (eight alpha bits declared) or 24-bit without. A side over
/// 65,535 is cut to it, which no texture has.
pub fn write(image: &Rgba, alpha: bool) -> Vec<u8> {
    let (w, h) = (image.width.min(65_535) as u16, image.height.min(65_535) as u16);
    let mut out = vec![0u8, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    out.extend(w.to_le_bytes());
    out.extend(h.to_le_bytes());
    out.extend(if alpha { [32, 8] } else { [24, 0] });
    let row = image.width as usize * 4;
    for line in image.data.chunks_exact(row).take(h as usize) {
        for px in line.as_chunks::<4>().0.iter().take(w as usize) {
            match alpha {
                true => out.extend([px[2], px[1], px[0], px[3]]),
                false => out.extend([px[2], px[1], px[0]]),
            }
        }
    }
    out
}

/// Reads a TGA into a single RGBA level, rows bottom first.
pub fn read(data: &[u8]) -> Result<Texture, ImageError> {
    let mut r = Reader::new(data);
    let id_len = r.u8()?;
    let map_type = r.u8()?;
    let image_type = r.u8()?;
    let map_first = r.u16()? as usize;
    let map_len = r.u16()? as usize;
    let map_bits = r.u8()?;
    let _x = r.u16()?;
    let _y = r.u16()?;
    let width = u32::from(r.u16()?);
    let height = u32::from(r.u16()?);
    let bits = r.u8()?;
    // (The image descriptor: its origin bits are ignored, as the game
    // ignores them, and its alpha bits too: below.)
    let _descriptor = r.u8()?;
    check_size(width, height)?;
    r.skip(id_len as usize)?;
    let map = if map_type == 1 {
        let entry = (map_bits as usize).div_ceil(8);
        let raw = r.bytes(map_len * entry)?;
        // An entry is a true-color pixel. A map of any other depth is
        // skipped: an image that needs it is refused below.
        matches!(map_bits, 15 | 16 | 24 | 32).then(|| {
            let entries = raw.chunks_exact(entry).map(|c| color(c, map_bits));
            (entries.collect::<Vec<_>>(), map_first)
        })
    } else {
        None
    };
    let (rle, kind) = match image_type {
        1 => (false, Kind::Mapped),
        2 => (false, Kind::True),
        3 => (false, Kind::Grey),
        9 => (true, Kind::Mapped),
        10 => (true, Kind::True),
        11 => (true, Kind::Grey),
        t => return Err(ImageError::TgaUnsupported(t, bits)),
    };
    let ok = match kind {
        Kind::True => matches!(bits, 15 | 16 | 24 | 32),
        Kind::Grey => matches!(bits, 8 | 16),
        Kind::Mapped => map.is_some() && matches!(bits, 8 | 16),
    };
    if !ok {
        return Err(ImageError::TgaUnsupported(image_type, bits));
    }
    let bpp = (bits as usize).div_ceil(8);
    let n = width as usize * height as usize;
    // The pixels as stored, before converting.
    let raw: Vec<u8> = if rle {
        let mut out = Vec::with_capacity(rle_capacity(n * bpp, r.remaining()));
        while out.len() < n * bpp {
            let head = r.u8()?;
            let count = (head & 0x7F) as usize + 1;
            if head & 0x80 != 0 {
                let px = r.bytes(bpp)?;
                for _ in 0..count {
                    out.extend_from_slice(px);
                }
            } else {
                out.extend_from_slice(r.bytes(count * bpp)?);
            }
        }
        out.truncate(n * bpp);
        out
    } else {
        r.bytes(n * bpp)?.to_vec()
    };
    let mut img = Rgba::new(width, height);
    for (px, out) in raw.chunks_exact(bpp).zip(img.data.as_chunks_mut::<4>().0) {
        let c = match kind {
            Kind::True => color(px, bits),
            Kind::Grey => [px[0], px[0], px[0], if bpp == 2 { px[1] } else { 255 }],
            Kind::Mapped => {
                let (map, first) = map.as_ref().expect("checked");
                let i = if bpp == 2 {
                    u16::from_le_bytes([px[0], px[1]]) as usize
                } else {
                    px[0] as usize
                };
                map.get(i.wrapping_sub(*first)).copied().unwrap_or([0, 0, 0, 255])
            }
        };
        out.copy_from_slice(&c);
    }
    // (A top-left origin, bit 5, changes nothing: the game takes the rows
    // as stored, the first as the bottom, whatever the file declares. A
    // picture saved top row first, as Krita saves it, shows upside down in
    // the game, and so here: `client_minimap.rs`. Nor does right-to-left,
    // bit 4: the columns are taken as stored too, `placeables_look` in
    // `client_render.rs`.)
    let has_alpha = match kind {
        Kind::True => bits == 32,
        Kind::Grey => bpp == 2,
        Kind::Mapped => map_bits == 32,
    };
    Ok(img.into_texture(has_alpha))
}

/// What to reserve for run-length encoded pixels of `size` bytes from
/// `remaining` bytes of data: a packet of 1 + bpp bytes makes at most 128
/// pixels, so no more than 128 times what is left (a header alone, asking
/// for 16384 × 16384 pixels, reserves nothing like a gigabyte).
fn rle_capacity(size: usize, remaining: usize) -> usize {
    size.min(remaining.saturating_mul(128))
}

#[derive(Clone, Copy)]
enum Kind {
    True,
    Grey,
    Mapped,
}

/// A stored true-colour pixel (BGR order) as RGBA. A 15- or 16-bit
/// pixel's five-bit levels are `(c << 3) | (c >> 2)` (4 is 33, 30 is 247)
/// and it is opaque, its top bit taken for no alpha even where the file
/// declares an alpha bit: as the client draws a model's texture of one
/// (`batch6_tga16_texture` in `mg-corpus-tests`' `client_render.rs`,
/// 2026-11-01), and a NUI image's alpha alike.
fn color(px: &[u8], bits: u8) -> [u8; 4] {
    match bits {
        15 | 16 => {
            let v = u16::from_le_bytes([px[0], px[1]]);
            let c5 = |s: u16| {
                let c = ((v >> s) & 0x1F) as u8;
                (c << 3) | (c >> 2)
            };
            [c5(10), c5(5), c5(0), 255]
        }
        24 => [px[2], px[1], px[0], 255],
        _ => [px[2], px[1], px[0], px[3]],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A TGA of one gray channel (image type 3, or 11 run-length encoded)
    /// is gray; one in true color is not, nor a short file.
    #[test]
    fn a_grayscale_tga_is_gray() {
        assert!(is_gray(&header(3, 1, 1, 8, 8)) && is_gray(&header(11, 1, 1, 8, 0)));
        assert!(!is_gray(&header(2, 1, 1, 24, 0)) && !is_gray(&[0, 0]));
    }

    fn header(image_type: u8, w: u16, h: u16, bits: u8, desc: u8) -> Vec<u8> {
        let mut v = vec![0, 0, image_type, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        v.extend_from_slice(&w.to_le_bytes());
        v.extend_from_slice(&h.to_le_bytes());
        v.push(bits);
        v.push(desc);
        v
    }

    #[test]
    fn bottom_left_rows_stay_as_stored() {
        let mut f = header(2, 1, 2, 24, 0);
        f.extend_from_slice(&[1, 2, 3, 4, 5, 6]);
        let t = read(&f).unwrap();
        let img = t.to_rgba();
        assert_eq!(img.pixel(0, 0), [3, 2, 1, 255]);
        assert_eq!(img.pixel(0, 1), [6, 5, 4, 255]);
        assert!(!t.has_alpha);
    }

    #[test]
    fn a_top_left_origin_is_ignored_as_in_the_game_and_rle_decodes() {
        // Two rows of two pixels: a run of 2 red, then 2 raw pixels.
        let mut f = header(10, 2, 2, 32, 0x28);
        f.extend_from_slice(&[0x81, 0, 0, 255, 255]);
        f.extend_from_slice(&[0x01, 255, 0, 0, 128, 0, 255, 0, 64]);
        let img = read(&f).unwrap().to_rgba();
        // The rows as stored, the first the bottom, though the file says
        // its first row is the top.
        assert_eq!(img.pixel(0, 0), [255, 0, 0, 255]);
        assert_eq!(img.pixel(0, 1), [0, 0, 255, 128]);
        assert_eq!(img.pixel(1, 1), [0, 255, 0, 64]);
    }

    /// A run-length encoded header asking for 16384 × 16384 pixels with a
    /// few bytes after it reserves what those bytes could make (at most
    /// 128 times them), not a gigabyte, and is an error.
    #[test]
    fn rle_reserves_by_the_data_present() {
        assert_eq!(rle_capacity(16_384 * 16_384 * 4, 6), 6 * 128);
        assert_eq!(rle_capacity(16, 6), 16);
        let mut f = header(10, 16_384, 16_384, 32, 0);
        f.extend_from_slice(&[0xFF, 1, 2, 3, 4, 0x81]);
        assert!(read(&f).is_err());
    }

    /// As a picture: top first and left first by the origin declared,
    /// whichever it is.
    #[test]
    fn a_picture_is_read_by_the_origin_its_header_declares() {
        let pixels = [[1u8, 1, 1], [2, 2, 2], [3, 3, 3], [4, 4, 4]].concat();
        // The same picture (1 2 over 3 4) stored from each corner.
        for (descriptor, stored) in
            [(0x20, [1, 2, 3, 4]), (0x00, [3, 4, 1, 2]), (0x10, [4, 3, 2, 1]), (0x30, [2, 1, 4, 3])]
        {
            let mut f = header(2, 2, 2, 24, descriptor);
            for i in stored {
                f.extend_from_slice(&pixels[(i - 1) * 3..i * 3]);
            }
            let img = read_picture(&f).unwrap();
            let grey = |x, y| img.pixel(x, y)[0];
            assert_eq!(
                [grey(0, 0), grey(1, 0), grey(0, 1), grey(1, 1)],
                [1, 2, 3, 4],
                "{descriptor:#x}"
            );
        }
    }

    #[test]
    fn right_to_left_is_ignored_as_in_the_game() {
        let mut f = header(3, 2, 1, 8, 0x10);
        f.extend_from_slice(&[10, 200]);
        let img = read(&f).unwrap().to_rgba();
        // The columns as stored.
        assert_eq!(img.pixel(0, 0), [10, 10, 10, 255]);
        assert_eq!(img.pixel(1, 0), [200, 200, 200, 255]);
    }

    #[test]
    fn greyscale() {
        let mut f = header(3, 2, 1, 8, 0);
        f.extend_from_slice(&[10, 200]);
        let img = read(&f).unwrap().to_rgba();
        assert_eq!(img.pixel(1, 0), [200, 200, 200, 255]);
    }

    #[test]
    fn a_color_map_of_no_pixel_depth_is_an_error() {
        // Mapped, with a map of two entries of 8 bits, then of none.
        for map_bits in [8u8, 0, 12, 20] {
            let mut f = header(1, 1, 1, 8, 0);
            (f[1], f[5], f[7]) = (1, 2, map_bits);
            f.extend_from_slice(&[0; 8]);
            assert!(matches!(read(&f), Err(ImageError::TgaUnsupported(1, 8))), "{map_bits}");
        }
        // A true-color image that carries such a map reads without it.
        let mut f = header(2, 1, 1, 24, 0);
        (f[1], f[5], f[7]) = (1, 2, 8);
        f.extend_from_slice(&[9, 9, 1, 2, 3]);
        assert_eq!(read(&f).unwrap().to_rgba().data, vec![3, 2, 1, 255]);
    }

    /// A 16-bit TGA as the client draws it: a model's texture's levels
    /// `(c << 3) | (c >> 2)`, a picture's (a NUI image's) `c × 255 / 31`
    /// rounded down, and the top bit no alpha in either, though the
    /// descriptor declares one alpha bit.
    #[test]
    fn a_16_bit_tga_is_opaque_and_its_levels_are_the_client_s() {
        // Red 4 with the bit set, green 30 with it clear, blue 8 and
        // white 31 with it clear.
        let mut f = header(2, 4, 1, 16, 0x21);
        for v in [0x8000u16 | (4 << 10), 30 << 5, 8, 0x7fff] {
            f.extend_from_slice(&v.to_le_bytes());
        }
        let t = read(&f).unwrap();
        assert!(!t.has_alpha);
        let img = t.to_rgba();
        assert_eq!(img.pixel(0, 0), [33, 0, 0, 255]);
        assert_eq!(img.pixel(1, 0), [0, 247, 0, 255]);
        assert_eq!(img.pixel(2, 0), [0, 0, 66, 255]);
        assert_eq!(img.pixel(3, 0), [255, 255, 255, 255]);
        let picture = read_picture(&f).unwrap();
        assert_eq!(picture.pixel(0, 0), [32, 0, 0, 255]);
        assert_eq!(picture.pixel(1, 0), [0, 246, 0, 255]);
        assert_eq!(picture.pixel(2, 0), [0, 0, 65, 255]);
        assert_eq!(picture.pixel(3, 0), [255, 255, 255, 255]);
        // Every level of the picture's table.
        for c in 0..32u16 {
            let mut f = header(2, 1, 1, 15, 0);
            f.extend_from_slice(&(c << 10).to_le_bytes());
            let want = (u32::from(c) * 255 / 31) as u8;
            assert_eq!(read_picture(&f).unwrap().pixel(0, 0)[0], want, "{c}");
        }
    }

    #[test]
    fn truncated_data_is_an_error() {
        let mut f = header(2, 4, 4, 32, 0);
        f.extend_from_slice(&[0; 10]);
        assert!(read(&f).is_err());
    }
}
