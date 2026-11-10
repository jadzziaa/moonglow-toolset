//! PNG pictures (a NUI window's, a plugin's), read the one way: any color
//! type and depth as RGBA8, rows from the top.

use crate::{ImageError, Rgba};

/// Reads a PNG of any color type and depth as RGBA8, rows from the top
/// and columns from the left, as a picture shows it (as
/// [`tga::read_picture`](crate::tga::read_picture) reads a TGA): 16-bit
/// samples rounded to the nearest 8-bit one, palettes and their
/// transparency unpacked. A picture of more than `max_pixels` is refused
/// before it is unpacked (a small file can unpack to a great many), and
/// the decoder is held to what that many take.
pub fn read_picture(data: &[u8], max_pixels: u64) -> Result<Rgba, ImageError> {
    let fail = |e: &dyn std::fmt::Display| ImageError::Png(e.to_string());
    // (Eight bytes a pixel at most: 16-bit RGBA; and the file's own.)
    let bytes = max_pixels.saturating_mul(8).saturating_add(data.len() as u64);
    let limits = ::png::Limits { bytes: usize::try_from(bytes).unwrap_or(usize::MAX) };
    let mut decoder = ::png::Decoder::new_with_limits(std::io::Cursor::new(data), limits);
    decoder.set_transformations(::png::Transformations::EXPAND);
    let mut reader = decoder.read_info().map_err(|e| fail(&e))?;
    let (width, height) = (reader.info().width, reader.info().height);
    if u64::from(width) * u64::from(height) > max_pixels {
        return Err(ImageError::BadSize(width, height));
    }
    let size = reader.output_buffer_size().ok_or(ImageError::BadSize(width, height))?;
    let mut buffer = vec![0; size];
    let info = reader.next_frame(&mut buffer).map_err(|e| fail(&e))?;
    let data = buffer.get(..info.buffer_size()).ok_or_else(|| fail(&"cut short"))?;
    // The samples of a pixel, each as 8 bits.
    let samples: Vec<u8> = match info.bit_depth {
        ::png::BitDepth::Sixteen => data
            .as_chunks::<2>()
            .0
            .iter()
            .map(|s| ((u32::from(u16::from_be_bytes(*s)) + 128) / 257) as u8)
            .collect(),
        _ => data.to_vec(),
    };
    let rgba: Vec<u8> = match info.color_type {
        ::png::ColorType::Grayscale => samples.iter().flat_map(|&g| [g, g, g, 255]).collect(),
        ::png::ColorType::GrayscaleAlpha => {
            samples.as_chunks::<2>().0.iter().flat_map(|p| [p[0], p[0], p[0], p[1]]).collect()
        }
        ::png::ColorType::Rgb => {
            samples.as_chunks::<3>().0.iter().flat_map(|p| [p[0], p[1], p[2], 255]).collect()
        }
        ::png::ColorType::Rgba => samples,
        ::png::ColorType::Indexed => return Err(fail(&"the palette was not unpacked")),
    };
    if rgba.len() != width as usize * height as usize * 4 {
        return Err(fail(&"cut short"));
    }
    Ok(Rgba { width, height, data: rgba })
}

/// A PNG's header: its size, its color type and depth as one word
/// (`rgb8`, `rgba8`, `grey8`, `greya8`, `indexed8`, `rgb16`…), and
/// whether it has alpha (an alpha channel, or a palette's or color's
/// transparency), read without unpacking it.
pub fn info(data: &[u8]) -> Result<(u32, u32, String, bool), ImageError> {
    let fail = |e: &dyn std::fmt::Display| ImageError::Png(e.to_string());
    let decoder = ::png::Decoder::new(std::io::Cursor::new(data));
    let reader = decoder.read_info().map_err(|e| fail(&e))?;
    let info = reader.info();
    let kind = match info.color_type {
        ::png::ColorType::Grayscale => "grey",
        ::png::ColorType::GrayscaleAlpha => "greya",
        ::png::ColorType::Rgb => "rgb",
        ::png::ColorType::Rgba => "rgba",
        ::png::ColorType::Indexed => "indexed",
    };
    let alpha =
        matches!(info.color_type, ::png::ColorType::GrayscaleAlpha | ::png::ColorType::Rgba)
            || info.trns.is_some();
    Ok((info.width, info.height, format!("{kind}{}", info.bit_depth as u8), alpha))
}

/// A PNG of RGBA8 pixels, `width` by `height`, rows from the top as
/// `data` has them. It fails only where `data` is not that many pixels.
pub fn write_rgba(width: u32, height: u32, data: &[u8]) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    let mut enc = ::png::Encoder::new(&mut out, width, height);
    enc.set_color(::png::ColorType::Rgba);
    enc.set_depth(::png::BitDepth::Eight);
    let mut w = enc.write_header().map_err(|e| e.to_string())?;
    w.write_image_data(data).map_err(|e| e.to_string())?;
    w.finish().map_err(|e| e.to_string())?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use ::png::{BitDepth, ColorType};

    use super::*;

    fn png_of(
        color: ColorType,
        depth: BitDepth,
        palette: Option<(&[u8], &[u8])>,
        data: &[u8],
    ) -> Vec<u8> {
        let mut out = Vec::new();
        let mut encoder = ::png::Encoder::new(&mut out, 2, 1);
        encoder.set_color(color);
        encoder.set_depth(depth);
        if let Some((palette, alpha)) = palette {
            encoder.set_palette(palette.to_vec());
            encoder.set_trns(alpha.to_vec());
        }
        let mut writer = encoder.write_header().unwrap();
        writer.write_image_data(data).unwrap();
        drop(writer);
        out
    }

    /// Every color type as RGBA8: 16-bit samples rounded (not cut, as the
    /// plugin API's own reader cut them), palettes and their transparency
    /// unpacked.
    #[test]
    fn a_png_of_any_kind_is_read_as_rgba() {
        let pixels = |bytes: &[u8]| {
            let p = read_picture(bytes, 16).unwrap();
            assert_eq!((p.width, p.height), (2, 1));
            p.data
        };
        let rgb = png_of(ColorType::Rgb, BitDepth::Eight, None, &[1, 2, 3, 4, 5, 6]);
        assert_eq!(pixels(&rgb), [1, 2, 3, 255, 4, 5, 6, 255]);
        let rgba = png_of(ColorType::Rgba, BitDepth::Eight, None, &[1, 2, 3, 4, 5, 6, 7, 8]);
        assert_eq!(pixels(&rgba), [1, 2, 3, 4, 5, 6, 7, 8]);
        let gray = png_of(ColorType::Grayscale, BitDepth::One, None, &[0b0100_0000]);
        assert_eq!(pixels(&gray), [0, 0, 0, 255, 255, 255, 255, 255]);
        let gray_alpha = png_of(ColorType::GrayscaleAlpha, BitDepth::Eight, None, &[9, 8, 7, 6]);
        assert_eq!(pixels(&gray_alpha), [9, 9, 9, 8, 7, 7, 7, 6]);
        // 0x0080 is nearer 0 than 1, 0x00FF nearer 1 (cut, both were 0);
        // 0xFF7F is 254.5 and a bit.
        let deep = png_of(ColorType::Grayscale, BitDepth::Sixteen, None, &[0, 0x80, 0, 0xff]);
        assert_eq!(pixels(&deep), [0, 0, 0, 255, 1, 1, 1, 255]);
        let deep = png_of(
            ColorType::Rgba,
            BitDepth::Sixteen,
            None,
            &[0xff, 0x7f, 0x80, 0, 0x12, 0x34, 0xff, 0xff, 0, 0, 0, 0, 0, 0, 0, 0],
        );
        assert_eq!(pixels(&deep), [255, 128, 18, 255, 0, 0, 0, 0]);
        let indexed = png_of(
            ColorType::Indexed,
            BitDepth::Eight,
            Some((&[10, 20, 30, 40, 50, 60], &[77])),
            &[1, 0],
        );
        assert_eq!(pixels(&indexed), [40, 50, 60, 255, 10, 20, 30, 77]);
    }

    /// Too many pixels is refused before they are unpacked; a file cut
    /// anywhere, or not a PNG, is an error, never a panic.
    #[test]
    fn rgba_written_reads_back_and_a_wrong_size_is_an_error() {
        let data = [1, 2, 3, 4, 5, 6, 7, 8];
        let png = write_rgba(2, 1, &data).unwrap();
        assert_eq!(
            read_picture(&png, 16).unwrap(),
            Rgba { width: 2, height: 1, data: data.to_vec() }
        );
        assert!(write_rgba(2, 2, &data).is_err(), "too few pixels");
        assert!(write_rgba(1, 1, &data).is_err(), "too many");
        assert!(write_rgba(0, 0, &[]).is_err(), "none: no PNG is empty");
    }

    #[test]
    fn a_png_s_header_is_read_without_its_pixels() {
        let rgb = png_of(ColorType::Rgb, BitDepth::Eight, None, &[1, 2, 3, 4, 5, 6]);
        assert_eq!(info(&rgb).unwrap(), (2, 1, "rgb8".to_string(), false));
        let deep = png_of(ColorType::GrayscaleAlpha, BitDepth::Sixteen, None, &[0; 8]);
        assert_eq!(info(&deep).unwrap(), (2, 1, "greya16".to_string(), true));
        let indexed = png_of(ColorType::Indexed, BitDepth::Eight, Some((&[0; 6], &[77])), &[1, 0]);
        assert!(info(&indexed).unwrap().3, "transparency in the palette");
        assert!(info(&rgb[..20]).is_err());
    }

    #[test]
    fn a_png_too_large_or_cut_short_is_an_error() {
        let rgb = png_of(ColorType::Rgb, BitDepth::Eight, None, &[1, 2, 3, 4, 5, 6]);
        assert_eq!(read_picture(&rgb, 1), Err(ImageError::BadSize(2, 1)));
        assert!(read_picture(&rgb, 2).is_ok());
        // (Its last chunk, IEND's 12 bytes, ends nothing it needs.)
        for len in 0..rgb.len() - 12 {
            assert!(read_picture(&rgb[..len], 16).is_err(), "{len} bytes");
        }
        assert!(read_picture(b"not a picture", 16).is_err());
    }
}
