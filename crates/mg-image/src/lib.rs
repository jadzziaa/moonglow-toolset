//! Textures: TGA, DDS (BioWare's header and the standard one) and PLT, and
//! the texture settings (TXI) and EE materials (MTR) that go with them.
//!
//! Every image keeps the game's row order: the first row is the bottom of
//! the picture, which is texture coordinate v = 0. That is how TGA (with its
//! default bottom-left origin) and DDS store their rows, and the game uploads
//! them as stored; only top-left-origin TGAs are flipped. [`Rgba::top_down`]
//! gives the picture the other way up for display.
//!
//! DDS data stays compressed ([`Texture::mips`]) for the GPU;
//! [`Texture::to_rgba`] decodes the largest level on the CPU.

pub mod bc;
pub mod dds;
pub mod mtr;
pub mod plt;
pub mod tga;
pub mod txi;

use mg_core::ResType;
use mg_core::bin::BinError;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ImageError {
    #[error("truncated or corrupt image: {0}")]
    Bin(#[from] BinError),
    #[error("unsupported TGA: image type {0}, {1} bits per pixel")]
    TgaUnsupported(u8, u8),
    #[error("unsupported DDS: {0}")]
    DdsUnsupported(String),
    #[error("not a PLT V1 file")]
    NotPlt,
    #[error("empty or oversized image ({0}×{1})")]
    BadSize(u32, u32),
    #[error("{0} is not an image type")]
    NotImage(ResType),
}

/// How a texture's pixels are stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Format {
    /// 8-bit red, green, blue, alpha.
    Rgba8,
    /// DXT1: colour, 1-bit alpha.
    Bc1,
    /// DXT3: colour, explicit 4-bit alpha.
    Bc2,
    /// DXT5: colour, interpolated alpha.
    Bc3,
    /// One channel (ATI1).
    Bc4,
    /// Two channels (ATI2, normal maps).
    Bc5,
}

impl Format {
    /// Bytes per 4×4 block for block-compressed formats.
    pub fn block_bytes(self) -> Option<usize> {
        match self {
            Format::Rgba8 => None,
            Format::Bc1 | Format::Bc4 => Some(8),
            Format::Bc2 | Format::Bc3 | Format::Bc5 => Some(16),
        }
    }

    /// The size in bytes of one level of `width`×`height`.
    pub fn level_size(self, width: u32, height: u32) -> usize {
        let (w, h) = (width.max(1) as usize, height.max(1) as usize);
        match self.block_bytes() {
            None => w * h * 4,
            Some(b) => w.div_ceil(4) * h.div_ceil(4) * b,
        }
    }
}

/// The largest side accepted (the game's textures are at most 2048; this
/// only guards against corrupt headers).
pub const MAX_SIDE: u32 = 16_384;

fn check_size(width: u32, height: u32) -> Result<(), ImageError> {
    if width == 0 || height == 0 || width > MAX_SIDE || height > MAX_SIDE {
        return Err(ImageError::BadSize(width, height));
    }
    Ok(())
}

/// A decoded or compressed texture: mip levels, largest first, each with
/// rows bottom first.
#[derive(Debug, Clone, PartialEq)]
pub struct Texture {
    pub width: u32,
    pub height: u32,
    pub format: Format,
    pub mips: Vec<Vec<u8>>,
    /// Whether the alpha channel means anything (BioWare's 3-channel DXT1
    /// and 24-bit TGAs have none: treat alpha as 1).
    pub has_alpha: bool,
    /// BioWare DDS's "alpha mean" header value.
    pub alpha_mean: Option<f32>,
}

impl Texture {
    /// The size of a mip level.
    pub fn level_dims(&self, level: usize) -> (u32, u32) {
        ((self.width >> level).max(1), (self.height >> level).max(1))
    }

    /// The largest level as RGBA8 (alpha 255 where the texture has none).
    pub fn to_rgba(&self) -> Rgba {
        let mut data = match self.format {
            Format::Rgba8 => self.mips[0].clone(),
            f => bc::decode(f, self.width, self.height, &self.mips[0]),
        };
        if !self.has_alpha {
            for px in data.as_chunks_mut::<4>().0 {
                px[3] = 255;
            }
        }
        Rgba { width: self.width, height: self.height, data }
    }
}

/// RGBA8 pixels, rows bottom first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rgba {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

impl Rgba {
    pub fn new(width: u32, height: u32) -> Rgba {
        Rgba { width, height, data: vec![0; width as usize * height as usize * 4] }
    }

    /// The pixel at `x` and stored row `y` (0 = bottom).
    pub fn pixel(&self, x: u32, y: u32) -> [u8; 4] {
        let i = (y as usize * self.width as usize + x as usize) * 4;
        [self.data[i], self.data[i + 1], self.data[i + 2], self.data[i + 3]]
    }

    /// The rows in the other order (top first, for display).
    pub fn top_down(&self) -> Rgba {
        let row = self.width as usize * 4;
        let data = self.data.chunks_exact(row).rev().flatten().copied().collect();
        Rgba { width: self.width, height: self.height, data }
    }

    /// As a single-level texture.
    pub fn into_texture(self, has_alpha: bool) -> Texture {
        Texture {
            width: self.width,
            height: self.height,
            format: Format::Rgba8,
            mips: vec![self.data],
            has_alpha,
            alpha_mean: None,
        }
    }
}

/// An uncompressed 24-bit TGA of a picture (rows stored bottom first, as
/// [`Rgba`] keeps them), as the game's minimap pictures are.
pub fn write_tga(image: &Rgba) -> Vec<u8> {
    let (w, h) = (image.width as u16, image.height as u16);
    let mut out = vec![0u8, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    out.extend(w.to_le_bytes());
    out.extend(h.to_le_bytes());
    out.extend([24, 0]);
    for px in image.data.as_chunks::<4>().0 {
        out.extend([px[2], px[1], px[0]]);
    }
    out
}

/// Reads a TGA or DDS.
pub fn read(restype: ResType, data: &[u8]) -> Result<Texture, ImageError> {
    match restype {
        ResType::TGA => tga::read(data),
        ResType::DDS => dds::read(data),
        t => Err(ImageError::NotImage(t)),
    }
}

#[cfg(test)]
mod write_tests {
    #[test]
    fn tga_written_reads_back() {
        let mut img = super::Rgba::new(2, 1);
        img.data.copy_from_slice(&[255, 0, 0, 255, 0, 0, 255, 255]);
        let back = super::read(mg_core::ResType::TGA, &super::write_tga(&img)).unwrap().to_rgba();
        assert_eq!(back, img);
    }
}
