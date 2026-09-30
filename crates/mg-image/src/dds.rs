//! DDS: BioWare's own header (almost every DDS in the game) and the standard
//! `DDS ` header (BC1–BC5 and uncompressed RGB(A)).
//!
//! BioWare's: width, height, channels (3 = DXT1, 4 = DXT5), the size of the
//! largest level, an "alpha mean" float, then the mip chain down to 1×1.
//! Data stays compressed; rows keep their stored order (the bottom first, as
//! the game uploads them).

use mg_core::bin::Reader;

use crate::{Format, ImageError, Texture, check_size};

/// Reads either kind of DDS.
pub fn read(data: &[u8]) -> Result<Texture, ImageError> {
    if data.starts_with(b"DDS ") { standard(data) } else { bioware(data) }
}

/// Collects up to `count` levels (at least one); stops early when the data
/// runs out, as a couple of shipped files do.
fn levels(
    data: &[u8],
    start: usize,
    format: Format,
    width: u32,
    height: u32,
    count: usize,
) -> Result<Vec<Vec<u8>>, ImageError> {
    let mut mips = Vec::new();
    let mut at = start;
    for level in 0..count.max(1) {
        let size = format.level_size(width >> level, height >> level);
        match data.get(at..at + size) {
            Some(bytes) => mips.push(bytes.to_vec()),
            None if level == 0 => {
                // The largest level must be there.
                let mut r = Reader::at(data, at.min(data.len()))?;
                r.bytes(size)?;
            }
            None => break,
        }
        at += size;
        if (width >> level) <= 1 && (height >> level) <= 1 {
            break;
        }
    }
    Ok(mips)
}

fn full_chain(width: u32, height: u32) -> usize {
    (32 - width.max(height).leading_zeros()) as usize
}

fn bioware(data: &[u8]) -> Result<Texture, ImageError> {
    let mut r = Reader::new(data);
    let width = r.u32()?;
    let height = r.u32()?;
    let channels = r.u32()?;
    let _top_size = r.u32()?;
    let alpha_mean = r.f32()?;
    check_size(width, height)?;
    let format = match channels {
        3 => Format::Bc1,
        4 => Format::Bc3,
        n => return Err(ImageError::DdsUnsupported(format!("BioWare DDS with {n} channels"))),
    };
    let mips = levels(data, 20, format, width, height, full_chain(width, height))?;
    Ok(Texture {
        width,
        height,
        format,
        mips,
        has_alpha: channels == 4,
        alpha_mean: Some(alpha_mean),
    })
}

const DDPF_ALPHAPIXELS: u32 = 0x1;
const DDPF_FOURCC: u32 = 0x4;
const DDPF_RGB: u32 = 0x40;
const DDPF_LUMINANCE: u32 = 0x20000;

fn standard(data: &[u8]) -> Result<Texture, ImageError> {
    let mut r = Reader::at(data, 4)?;
    let _size = r.u32()?;
    let _flags = r.u32()?;
    let height = r.u32()?;
    let width = r.u32()?;
    let _pitch = r.u32()?;
    let depth = r.u32()?;
    let mip_count = r.u32()?;
    r.skip(44)?;
    // Pixel format.
    let _pf_size = r.u32()?;
    let pf_flags = r.u32()?;
    let four_cc = r.array::<4>()?;
    let bit_count = r.u32()?;
    let masks = [r.u32()?, r.u32()?, r.u32()?, r.u32()?];
    check_size(width, height)?;
    if depth > 1 {
        return Err(ImageError::DdsUnsupported("volume texture".into()));
    }
    let count = (mip_count as usize).clamp(1, full_chain(width, height));
    if pf_flags & DDPF_FOURCC != 0 {
        let format = match &four_cc {
            b"DXT1" => Format::Bc1,
            b"DXT2" | b"DXT3" => Format::Bc2,
            b"DXT4" | b"DXT5" => Format::Bc3,
            b"ATI1" | b"BC4U" => Format::Bc4,
            b"ATI2" | b"BC5U" => Format::Bc5,
            other => {
                return Err(ImageError::DdsUnsupported(format!(
                    "FourCC {}",
                    String::from_utf8_lossy(other)
                )));
            }
        };
        let mips = levels(data, 128, format, width, height, count)?;
        return Ok(Texture {
            width,
            height,
            format,
            mips,
            has_alpha: !matches!(format, Format::Bc4 | Format::Bc5),
            alpha_mean: None,
        });
    }
    if pf_flags & (DDPF_RGB | DDPF_LUMINANCE) == 0 || !matches!(bit_count, 8 | 16 | 24 | 32) {
        return Err(ImageError::DdsUnsupported(format!("pixel format flags {pf_flags:#x}")));
    }
    // Uncompressed: convert each level to RGBA8 through the channel masks.
    let bpp = bit_count as usize / 8;
    let has_alpha = pf_flags & DDPF_ALPHAPIXELS != 0 && masks[3] != 0;
    let channel = |v: u32, mask: u32| -> u8 {
        if mask == 0 {
            return 0;
        }
        let shift = mask.trailing_zeros();
        let max = mask >> shift;
        (((v & mask) >> shift) as u64 * 255 / max as u64) as u8
    };
    let mut mips = Vec::new();
    let mut at = 128;
    for level in 0..count {
        let (w, h) = ((width >> level).max(1), (height >> level).max(1));
        let size = w as usize * h as usize * bpp;
        let Some(bytes) = data.get(at..at + size) else {
            if level == 0 {
                Reader::at(data, at.min(data.len()))?.bytes(size)?;
            }
            break;
        };
        let mut out = Vec::with_capacity(w as usize * h as usize * 4);
        for px in bytes.chunks_exact(bpp) {
            let mut v = 0u32;
            for (i, b) in px.iter().enumerate() {
                v |= (*b as u32) << (8 * i);
            }
            if pf_flags & DDPF_LUMINANCE != 0 {
                let l = channel(v, masks[0]);
                out.extend_from_slice(&[
                    l,
                    l,
                    l,
                    if has_alpha { channel(v, masks[3]) } else { 255 },
                ]);
            } else {
                out.extend_from_slice(&[
                    channel(v, masks[0]),
                    channel(v, masks[1]),
                    channel(v, masks[2]),
                    if has_alpha { channel(v, masks[3]) } else { 255 },
                ]);
            }
        }
        mips.push(out);
        at += size;
    }
    Ok(Texture { width, height, format: Format::Rgba8, mips, has_alpha, alpha_mean: None })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bioware_file(w: u32, h: u32, channels: u32, levels: usize) -> Vec<u8> {
        let format = if channels == 3 { Format::Bc1 } else { Format::Bc3 };
        let mut f = Vec::new();
        for v in [w, h, channels, format.level_size(w, h) as u32] {
            f.extend_from_slice(&v.to_le_bytes());
        }
        f.extend_from_slice(&0.5f32.to_le_bytes());
        for l in 0..levels {
            f.extend(std::iter::repeat_n(l as u8, format.level_size(w >> l, h >> l)));
        }
        f
    }

    #[test]
    fn bioware_mip_chain() {
        let t = read(&bioware_file(8, 4, 4, 4)).unwrap();
        assert_eq!(t.format, Format::Bc3);
        assert_eq!(t.mips.len(), 4);
        assert_eq!(t.level_dims(3), (1, 1));
        assert_eq!(t.alpha_mean, Some(0.5));
        assert!(t.has_alpha);
        // A short chain keeps what is there.
        let t = read(&bioware_file(8, 8, 3, 2)).unwrap();
        assert_eq!(t.mips.len(), 2);
        assert!(!t.has_alpha);
    }

    #[test]
    fn missing_top_level_is_an_error() {
        let mut f = bioware_file(8, 8, 3, 1);
        f.truncate(30);
        assert!(read(&f).is_err());
    }

    #[test]
    fn standard_uncompressed_bgra() {
        let mut f = b"DDS ".to_vec();
        let mut h = vec![0u32; 31];
        h[0] = 124;
        h[2] = 1; // height
        h[3] = 2; // width
        h[6] = 1; // mips
        h[18] = 32; // pixel format size
        h[19] = DDPF_RGB | DDPF_ALPHAPIXELS;
        h[21] = 32;
        h[22] = 0x00FF_0000;
        h[23] = 0x0000_FF00;
        h[24] = 0x0000_00FF;
        h[25] = 0xFF00_0000;
        for v in h {
            f.extend_from_slice(&v.to_le_bytes());
        }
        f.extend_from_slice(&[1, 2, 3, 4, 5, 6, 7, 8]);
        let t = read(&f).unwrap();
        assert_eq!(t.to_rgba().data, vec![3, 2, 1, 4, 7, 6, 5, 8]);
    }
}
