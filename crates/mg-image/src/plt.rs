//! PLT: layered textures coloured from palettes (creature bodies, armour,
//! helmets, cloaks). Each pixel is a grey level and a layer (skin, hair,
//! metal 1–2, cloth 1–2, leather 1–2, tattoo 1–2); its colour is the
//! layer's palette at column = grey and row = the chosen colour index,
//! counted from the palette's top (stored row 175 − index), alpha included.

use mg_core::bin::Reader;

use crate::{ImageError, Rgba, check_size};

/// The ten layers, in PLT order.
pub const LAYERS: [&str; 10] = [
    "Skin",
    "Hair",
    "Metal 1",
    "Metal 2",
    "Cloth 1",
    "Cloth 2",
    "Leather 1",
    "Leather 2",
    "Tattoo 1",
    "Tattoo 2",
];

/// Each layer's palette (`pal_*.tga`).
pub const PALETTES: [&str; 10] = [
    "pal_skin01",
    "pal_hair01",
    "pal_armor01",
    "pal_armor02",
    "pal_cloth01",
    "pal_cloth01",
    "pal_leath01",
    "pal_leath01",
    "pal_tattoo01",
    "pal_tattoo01",
];

/// A PLT: per pixel a grey level and a layer, rows bottom first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plt {
    pub width: u32,
    pub height: u32,
    /// (grey, layer) per pixel.
    pub pixels: Vec<[u8; 2]>,
}

impl Plt {
    pub fn read(data: &[u8]) -> Result<Plt, ImageError> {
        if !data.starts_with(b"PLT V1  ") {
            return Err(ImageError::NotPlt);
        }
        let mut r = Reader::at(data, 16)?;
        let width = r.u32()?;
        let height = r.u32()?;
        check_size(width, height)?;
        let bytes = r.bytes(width as usize * height as usize * 2)?;
        let pixels = bytes.as_chunks::<2>().0.to_vec();
        Ok(Plt { width, height, pixels })
    }

    /// The layers the image uses.
    pub fn layers(&self) -> [bool; 10] {
        let mut used = [false; 10];
        for p in &self.pixels {
            if let Some(u) = used.get_mut(p[1] as usize) {
                *u = true;
            }
        }
        used
    }

    /// The image coloured with a colour index per layer, from each layer's
    /// palette (`palettes[layer]`, e.g. from [`PALETTES`]). Missing palettes
    /// and indices past a palette's rows give opaque grey.
    pub fn colorize(&self, palettes: &[Option<&Rgba>; 10], colors: [u8; 10]) -> Rgba {
        let mut out = Rgba::new(self.width, self.height);
        for (p, px) in self.pixels.iter().zip(out.data.as_chunks_mut::<4>().0) {
            let [grey, layer] = *p;
            let layer = (layer as usize).min(9);
            let c = palettes[layer]
                .filter(|pal| u32::from(colors[layer]) < pal.height && pal.width > 0)
                .map(|pal| {
                    let row = pal.height - 1 - u32::from(colors[layer]);
                    let x = (u32::from(grey) * pal.width / 256).min(pal.width - 1);
                    pal.pixel(x, row)
                })
                .unwrap_or([grey, grey, grey, 255]);
            px.copy_from_slice(&c);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colours_come_from_the_palette_row_counted_from_the_top() {
        let mut f = b"PLT V1  ".to_vec();
        for v in [10u32, 0, 2, 1] {
            f.extend_from_slice(&v.to_le_bytes());
        }
        f.extend_from_slice(&[0, 0, 255, 1]); // skin grey 0, hair grey 255
        let plt = Plt::read(&f).unwrap();
        assert_eq!(plt.layers()[..3], [true, true, false]);
        // A 256×2 palette: stored row 0 (bottom) red, row 1 (top) green.
        let mut pal = Rgba::new(256, 2);
        for x in 0..256 {
            pal.data[x * 4..x * 4 + 4].copy_from_slice(&[255, 0, 0, 255]);
            pal.data[(256 + x) * 4..(256 + x) * 4 + 4].copy_from_slice(&[0, x as u8, 0, 255]);
        }
        let mut pals = [None; 10];
        pals[0] = Some(&pal);
        pals[1] = Some(&pal);
        let mut colors = [0u8; 10];
        colors[1] = 1;
        let img = plt.colorize(&pals, colors);
        // Skin, index 0 = the top row: green at column 0.
        assert_eq!(img.pixel(0, 0), [0, 0, 0, 255]);
        // Hair, index 1 = the bottom row: red.
        assert_eq!(img.pixel(1, 0), [255, 0, 0, 255]);
    }
}
