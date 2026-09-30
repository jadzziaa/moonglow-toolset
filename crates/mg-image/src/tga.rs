//! TGA: uncompressed and run-length encoded, true colour (15/16, 24, 32
//! bits), greyscale (8 bits) and colour-mapped. The game's are mostly
//! bottom-left 24- and 32-bit files; 32-bit files that declare no alpha
//! bits still carry alpha in the fourth byte, which is used.

use mg_core::bin::Reader;

use crate::{ImageError, Rgba, Texture, check_size};

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
    let descriptor = r.u8()?;
    check_size(width, height)?;
    r.skip(id_len as usize)?;
    let map = if map_type == 1 {
        let entry = (map_bits as usize).div_ceil(8);
        let raw = r.bytes(map_len * entry)?;
        let entries = raw.chunks_exact(entry).map(|c| color(c, map_bits, true));
        Some((entries.collect::<Vec<_>>(), map_first))
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
        let mut out = Vec::with_capacity(n * bpp);
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
            Kind::True => color(px, bits, descriptor & 0x0F != 0),
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
    let row = width as usize * 4;
    if descriptor & 0x10 != 0 {
        // Right-to-left.
        for line in img.data.chunks_exact_mut(row) {
            line.as_chunks_mut::<4>().0.reverse();
        }
    }
    if descriptor & 0x20 != 0 {
        // Top-left origin: make the bottom row first.
        img = img.top_down();
    }
    let has_alpha = match kind {
        Kind::True => bits == 32 || (bits == 16 && descriptor & 0x0F != 0),
        Kind::Grey => bpp == 2,
        Kind::Mapped => map_bits == 32,
    };
    Ok(img.into_texture(has_alpha))
}

#[derive(Clone, Copy)]
enum Kind {
    True,
    Grey,
    Mapped,
}

/// A stored true-colour pixel (BGR order) as RGBA; a 16-bit pixel's top
/// bit is alpha when the file declares an alpha bit.
fn color(px: &[u8], bits: u8, alpha_bit: bool) -> [u8; 4] {
    match bits {
        15 | 16 => {
            let v = u16::from_le_bytes([px[0], px[1]]);
            let c5 = |s: u16| {
                let c = ((v >> s) & 0x1F) as u8;
                (c << 3) | (c >> 2)
            };
            let a = if bits == 16 && alpha_bit && v & 0x8000 == 0 { 0 } else { 255 };
            [c5(10), c5(5), c5(0), a]
        }
        24 => [px[2], px[1], px[0], 255],
        _ => [px[2], px[1], px[0], px[3]],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn top_left_origin_is_flipped_and_rle_decodes() {
        // Two rows of two pixels: a run of 2 red, then 2 raw pixels.
        let mut f = header(10, 2, 2, 32, 0x28);
        f.extend_from_slice(&[0x81, 0, 0, 255, 255]);
        f.extend_from_slice(&[0x01, 255, 0, 0, 128, 0, 255, 0, 64]);
        let img = read(&f).unwrap().to_rgba();
        // Stored first (the top) is red; bottom-first puts it second.
        assert_eq!(img.pixel(0, 1), [255, 0, 0, 255]);
        assert_eq!(img.pixel(0, 0), [0, 0, 255, 128]);
        assert_eq!(img.pixel(1, 0), [0, 255, 0, 64]);
    }

    #[test]
    fn greyscale() {
        let mut f = header(3, 2, 1, 8, 0);
        f.extend_from_slice(&[10, 200]);
        let img = read(&f).unwrap().to_rgba();
        assert_eq!(img.pixel(1, 0), [200, 200, 200, 255]);
    }

    #[test]
    fn truncated_data_is_an_error() {
        let mut f = header(2, 4, 4, 32, 0);
        f.extend_from_slice(&[0; 10]);
        assert!(read(&f).is_err());
    }
}
