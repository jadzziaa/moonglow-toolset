//! Textures on the GPU: BC-compressed data uploaded as it is (when the
//! device reads BC and the size allows, and it is not a DXT1 whose one-bit
//! texels the game draws opaque), everything else as RGBA8 with a
//! mip chain made on the CPU. Rows go up in stored order (bottom row first),
//! so texture coordinate v = 0 is the image's first stored row, as in the
//! game. Values stay in gamma space (the shader linearises with pow 2.2).

use mg_image::{Format, Texture};

use crate::Gpu;

/// A texture and its view.
#[derive(Debug)]
pub struct GpuTexture {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    /// Whether its alpha means anything.
    pub has_alpha: bool,
}

fn wgpu_format(f: Format) -> wgpu::TextureFormat {
    match f {
        Format::Rgba8 => wgpu::TextureFormat::Rgba8Unorm,
        Format::Bc1 => wgpu::TextureFormat::Bc1RgbaUnorm,
        Format::Bc2 => wgpu::TextureFormat::Bc2RgbaUnorm,
        Format::Bc3 => wgpu::TextureFormat::Bc3RgbaUnorm,
        Format::Bc4 => wgpu::TextureFormat::Bc4RUnorm,
        Format::Bc5 => wgpu::TextureFormat::Bc5RgUnorm,
    }
}

/// Whether the GPU holds a texture of this size.
fn fits(gpu: &Gpu, width: u32, height: u32) -> Result<(), String> {
    let most = gpu.device.limits().max_texture_dimension_2d;
    if width == 0 || height == 0 {
        Err(format!("it is {width} by {height} pixels"))
    } else if width > most || height > most {
        Err(format!("it is {width} by {height} pixels, and the GPU takes {most} at most"))
    } else {
        Ok(())
    }
}

/// Whether a texture without alpha is a DXT1 with one-bit texels (a block
/// in the three-color mode, c0 ≤ c1, using its index 3) in any level. The
/// game draws them opaque black, alpha 1 to every program that reads it
/// (a TXI `alphamean`, a mesh's alpha, an environment map's reflectivity:
/// `batch6_bioware_dxt1_one_bit` in `mg-corpus-tests`' `client_render`),
/// where the GPU's BC1 gives them alpha 0; so such a texture goes up
/// decoded. (Four of the game's 6,737 DDS files: `tdt01_wood04` and `05`
/// with a quarter of their texels so, `tts02_gate03` and `tts02_tree01`
/// with a few in their smaller levels.) Read on lit meshes with the stock
/// lit and environment-mapped programs; decals, additive meshes, an MTR's
/// `transparency`, particles and icons were not.
fn one_bit_texels(tex: &Texture) -> bool {
    tex.format == Format::Bc1
        && !tex.has_alpha
        && tex.mips.iter().any(|level| {
            level.as_chunks::<8>().0.iter().any(|b| {
                let (c0, c1) = (u16::from_le_bytes([b[0], b[1]]), u16::from_le_bytes([b[2], b[3]]));
                let bits = u32::from_le_bytes([b[4], b[5], b[6], b[7]]);
                c0 <= c1 && (0..16).any(|i| (bits >> (2 * i)) & 3 == 3)
            })
        })
}

/// Level `level` of a texture as RGBA8, where it stores one (a DDS's own
/// smaller levels; alpha 255 where the texture has none).
fn stored_level(tex: &Texture, level: usize) -> Option<Vec<u8>> {
    let data = tex.mips.get(level)?;
    let (w, h) = tex.level_dims(level);
    let mut rgba = match tex.format {
        Format::Rgba8 => data.clone(),
        f => mg_image::bc::decode(f, w, h, data),
    };
    if rgba.len() != w as usize * h as usize * 4 {
        return None;
    }
    if !tex.has_alpha {
        for px in rgba.as_chunks_mut::<4>().0 {
            px[3] = 255;
        }
    }
    Some(rgba)
}

/// Halves an RGBA8 image (box filter; odd sizes clamp at the edge). A
/// pixel's color counts by its alpha, so that what a texture keeps under
/// its transparent parts (often a pale background) doesn't show as a rim
/// round what is drawn, wider the smaller the level.
fn half(data: &[u8], w: u32, h: u32) -> (Vec<u8>, u32, u32) {
    let (nw, nh) = ((w / 2).max(1), (h / 2).max(1));
    let mut out = vec![0u8; nw as usize * nh as usize * 4];
    for y in 0..nh {
        for x in 0..nw {
            let (mut plain, mut weighed, mut alpha) = ([0u32; 3], [0u32; 3], 0u32);
            for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                let (sx, sy) = ((2 * x + dx).min(w - 1), (2 * y + dy).min(h - 1));
                let i = (sy as usize * w as usize + sx as usize) * 4;
                let a = u32::from(data[i + 3]);
                for c in 0..3 {
                    plain[c] += u32::from(data[i + c]);
                    weighed[c] += u32::from(data[i + c]) * a;
                }
                alpha += a;
            }
            let o = (y as usize * nw as usize + x as usize) * 4;
            for c in 0..3 {
                // (All of it transparent: the colors as they are.)
                out[o + c] = match alpha {
                    0 => ((plain[c] + 2) / 4) as u8,
                    _ => ((weighed[c] + alpha / 2) / alpha) as u8,
                };
            }
            out[o + 3] = ((alpha + 2) / 4) as u8;
        }
    }
    (out, nw, nh)
}

impl GpuTexture {
    /// [`GpuTexture::upload`], or why the GPU won't take it (a size it
    /// can't hold, data it refuses), instead of the device's error, which
    /// would end the program.
    pub fn try_upload(
        gpu: &Gpu,
        label: &str,
        tex: &Texture,
        mipmaps: bool,
    ) -> Result<GpuTexture, String> {
        fits(gpu, tex.width, tex.height)?;
        crate::game::checked(gpu, || GpuTexture::upload(gpu, label, tex, mipmaps))
    }

    /// [`GpuTexture::upload_cube`], or why the GPU won't take it.
    pub fn try_upload_cube(
        gpu: &Gpu,
        label: &str,
        faces: &[Texture; 6],
    ) -> Result<GpuTexture, String> {
        fits(gpu, faces[0].width, faces[0].height)?;
        crate::game::checked(gpu, || GpuTexture::upload_cube(gpu, label, faces))
    }

    /// Uploads a texture; `mipmaps` false keeps one level (TXI `mipmap 0`).
    /// At most [`Gpu::texture_limit`] pixels a side.
    pub fn upload(gpu: &Gpu, label: &str, tex: &Texture, mipmaps: bool) -> GpuTexture {
        let compressed = tex.format != Format::Rgba8;
        let one_bit = one_bit_texels(tex);
        let bc_ok = compressed
            && !one_bit
            && gpu.bc_textures()
            && tex.width.is_multiple_of(4)
            && tex.height.is_multiple_of(4);
        let limit = gpu.texture_limit();
        let (format, levels): (Format, Vec<(Vec<u8>, u32, u32)>) = if bc_ok {
            // (The first level within the limit, of those stored, whose
            // sides are still whole blocks.)
            let skip = limit.map_or(0, |l| {
                (0..tex.mips.len())
                    .take_while(|&i| {
                        let (w, h) = tex.level_dims(i);
                        w.max(h) > l
                            && tex.level_dims(i + 1).0.is_multiple_of(4)
                            && tex.level_dims(i + 1).1.is_multiple_of(4)
                            && i + 1 < tex.mips.len()
                    })
                    .count()
            });
            let n = if mipmaps { tex.mips.len() } else { skip + 1 };
            let levels = tex.mips[skip..n]
                .iter()
                .enumerate()
                .map(|(i, m)| {
                    let (w, h) = tex.level_dims(skip + i);
                    (m.clone(), w, h)
                })
                .collect();
            (tex.format, levels)
        } else {
            let over = |w: u32, h: u32| limit.is_some_and(|l| w.max(h) > l);
            // (A DXT1 with one-bit texels: its stored levels decoded, as
            // far as they go whole, the texels opaque.)
            let mut levels: Vec<(Vec<u8>, u32, u32)> = match one_bit {
                true => (0..tex.mips.len())
                    .map_while(|i| {
                        let (w, h) = tex.level_dims(i);
                        stored_level(tex, i).map(|d| (d, w, h))
                    })
                    .collect(),
                false => Vec::new(),
            };
            if levels.is_empty() {
                let rgba = tex.to_rgba();
                levels.push((rgba.data, rgba.width, rgba.height));
            }
            while let Some((d, w, h)) =
                levels.last().filter(|(_, w, h)| (mipmaps || over(*w, *h)) && (*w > 1 || *h > 1))
            {
                let next = half(d, *w, *h);
                levels.push(next);
            }
            // (Those over the limit left out: the first within it is the
            // largest.)
            let skip = levels.iter().take_while(|(_, w, h)| over(*w, *h)).count();
            levels.drain(..skip.min(levels.len() - 1));
            if !mipmaps {
                levels.truncate(1);
            }
            (Format::Rgba8, levels)
        };
        let (width, height) = (levels[0].1, levels[0].2);
        let size = wgpu::Extent3d { width, height, depth_or_array_layers: 1 };
        let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size,
            mip_level_count: levels.len() as u32,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu_format(format),
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        for (level, (data, w, h)) in levels.iter().enumerate() {
            write_level(gpu, &texture, (level as u32, 0), format, data, (*w, *h));
        }
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        GpuTexture { texture, view, has_alpha: tex.has_alpha }
    }

    /// A cube map from six faces in the game's order (+X, −X, +Y, −Y, +Z,
    /// −Z, world axes: +Z is up), with its smaller levels: a face's own
    /// where it stores them (a DDS's), else made of the level above, so
    /// that a rough surface's reflection, read at a smaller level (the
    /// game's programs' bias, Moonglow's `cube_level`), is blurred. Faces of
    /// another size than the first are scaled to it.
    pub fn upload_cube(gpu: &Gpu, label: &str, faces: &[Texture; 6]) -> GpuTexture {
        if let Some(cube) = upload_cube_as_stored(gpu, label, faces) {
            return cube;
        }
        let first = faces[0].to_rgba();
        let (w, h) = (first.width.max(1), first.height.max(1));
        let count = 1 + w.max(h).ilog2();
        // (The levels over the limit left out: the first within it is the
        // largest.)
        let over = |level: u32| {
            gpu.texture_limit().is_some_and(|l| (w >> level).max(h >> level).max(1) > l)
        };
        let skip = (0..count - 1).take_while(|&level| over(level)).count() as u32;
        let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d {
                width: (w >> skip).max(1),
                height: (h >> skip).max(1),
                depth_or_array_layers: 6,
            },
            mip_level_count: count - skip,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        for (layer, face) in faces.iter().enumerate() {
            let rgba = if layer == 0 { first.clone() } else { face.to_rgba() };
            let same = (rgba.width, rgba.height) == (w, h);
            let data = if same {
                rgba.data
            } else {
                // Nearest-neighbour to the first face's size.
                (0..h)
                    .flat_map(|y| (0..w).map(move |x| (x, y)))
                    .flat_map(|(x, y)| {
                        rgba.pixel(x * rgba.width.max(1) / w, y * rgba.height.max(1) / h)
                    })
                    .collect()
            };
            let mut level = (data, w, h);
            for mip in 0..count {
                if mip > 0 {
                    let (lw, lh) = ((w >> mip).max(1), (h >> mip).max(1));
                    level = match stored_level(face, mip as usize).filter(|_| same) {
                        Some(data) => (data, lw, lh),
                        None => half(&level.0, level.1, level.2),
                    };
                }
                if mip < skip {
                    continue;
                }
                let (data, lw, lh) = &level;
                let at = (mip - skip, layer as u32);
                write_level(gpu, &texture, at, Format::Rgba8, data, (*lw, *lh));
            }
        }
        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::Cube),
            ..Default::default()
        });
        GpuTexture { texture, view, has_alpha: false }
    }
    /// What it takes on the GPU, bytes: every level of every layer, as its
    /// format stores it (a BC format's blocks, 4 bytes a pixel for RGBA8).
    pub fn bytes(&self) -> u64 {
        texture_bytes(&self.texture)
    }

    /// A 1×1 texture of one colour.
    pub fn solid(gpu: &Gpu, label: &str, rgba: [u8; 4]) -> GpuTexture {
        let tex = mg_image::Rgba { width: 1, height: 1, data: rgba.to_vec() }.into_texture(true);
        GpuTexture::upload(gpu, label, &tex, false)
    }
}

/// A cube map of six BC faces uploaded as they are stored (a DXT cube's
/// faces decoded took four to eight times their stored size): where the
/// device reads BC and the faces agree (one format, one size, whole
/// blocks a side) and each stores its smaller levels down to a pixel (the
/// rough surfaces' blurred reflections are read from them). From the
/// first level within [`Gpu::texture_limit`] whose sides are whole blocks.
/// `None`: uploaded as RGBA8, its levels made.
fn upload_cube_as_stored(gpu: &Gpu, label: &str, faces: &[Texture; 6]) -> Option<GpuTexture> {
    let first = &faces[0];
    let (w, h) = (first.width, first.height);
    let blocks = |level: usize| {
        let (lw, lh) = first.level_dims(level);
        lw.is_multiple_of(4) && lh.is_multiple_of(4)
    };
    if first.format == Format::Rgba8 || !gpu.bc_textures() || w == 0 || h == 0 || !blocks(0) {
        return None;
    }
    let count = 1 + w.max(h).ilog2() as usize;
    let whole = |f: &Texture| {
        (f.format, f.width, f.height) == (first.format, w, h)
            && f.mips.len() >= count
            && (0..count).all(|i| {
                let (lw, lh) = f.level_dims(i);
                f.mips[i].len() == f.format.level_size(lw, lh)
            })
    };
    // (A DXT1 face with one-bit texels: decoded, opaque, as `upload` does.)
    if !faces.iter().all(whole) || faces.iter().any(one_bit_texels) {
        return None;
    }
    let over = |level: usize| {
        let (lw, lh) = first.level_dims(level);
        gpu.texture_limit().is_some_and(|l| lw.max(lh) > l)
    };
    let skip = (0..count - 1).take_while(|&i| over(i) && blocks(i + 1)).count();
    let (width, height) = first.level_dims(skip);
    let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d { width, height, depth_or_array_layers: 6 },
        mip_level_count: (count - skip) as u32,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu_format(first.format),
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    for (layer, face) in faces.iter().enumerate() {
        for level in skip..count {
            let at = ((level - skip) as u32, layer as u32);
            write_level(gpu, &texture, at, face.format, &face.mips[level], face.level_dims(level));
        }
    }
    let view = texture.create_view(&wgpu::TextureViewDescriptor {
        dimension: Some(wgpu::TextureViewDimension::Cube),
        ..Default::default()
    });
    Some(GpuTexture { texture, view, has_alpha: false })
}

/// Writes a level of a layer of `texture` (`at`: the level and the
/// layer): `data` in `format`, `w` by `h` pixels (a BC format's whole
/// blocks).
fn write_level(
    gpu: &Gpu,
    texture: &wgpu::Texture,
    (level, layer): (u32, u32),
    format: Format,
    data: &[u8],
    (w, h): (u32, u32),
) {
    let (bytes_per_row, rows, extent) = match format.block_bytes() {
        None => (w * 4, h, (w, h)),
        Some(b) => {
            let (bw, bh) = (w.div_ceil(4), h.div_ceil(4));
            (bw * b as u32, bh, (bw * 4, bh * 4))
        }
    };
    gpu.queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: level,
            origin: wgpu::Origin3d { x: 0, y: 0, z: layer },
            aspect: wgpu::TextureAspect::All,
        },
        data,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(bytes_per_row),
            rows_per_image: Some(rows),
        },
        wgpu::Extent3d { width: extent.0, height: extent.1, depth_or_array_layers: 1 },
    );
}

/// What a texture takes on the GPU, bytes ([`GpuTexture::bytes`]).
pub fn texture_bytes(t: &wgpu::Texture) -> u64 {
    let format = t.format();
    let (bw, bh) = format.block_dimensions();
    let block = u64::from(format.block_copy_size(None).unwrap_or(4));
    let layers = u64::from(t.depth_or_array_layers());
    let samples = u64::from(t.sample_count());
    (0..t.mip_level_count())
        .map(|level| {
            let (w, h) = ((t.width() >> level).max(1), (t.height() >> level).max(1));
            u64::from(w.div_ceil(bw)) * u64::from(h.div_ceil(bh)) * block
        })
        .sum::<u64>()
        * layers
        * samples
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A face's own smaller level is taken where it stores it (decoded, a
    /// DDS's), and none past what it stores or of another size; a cube is
    /// uploaded with every level down to a pixel.
    #[test]
    fn a_cube_s_levels_are_its_faces_own_or_made() {
        let face = Texture {
            width: 4,
            height: 4,
            format: Format::Rgba8,
            mips: vec![[255, 0, 0, 255].repeat(16), [0, 255, 0, 255].repeat(4)],
            has_alpha: false,
            alpha_mean: None,
        };
        assert_eq!(stored_level(&face, 1), Some([0, 255, 0, 255].repeat(4)));
        assert_eq!(stored_level(&face, 2), None, "past what it stores");
        let short = Texture { mips: vec![face.mips[0].clone(), vec![0; 3]], ..face.clone() };
        assert_eq!(stored_level(&short, 1), None, "cut short");
        let Some(gpu) = mg_testkit::gpu::open(Gpu::headless) else { return };
        let cube = GpuTexture::upload_cube(&gpu, "cube", &std::array::from_fn(|_| face.clone()));
        assert_eq!(cube.texture.mip_level_count(), 3);
    }

    /// A DXT1 whose blocks use the three-color mode's index 3 goes up
    /// decoded, its stored levels whole and those texels opaque black (as
    /// the game draws them, whatever reads their alpha); a DXT1 of four
    /// colors, or of three without index 3, stays compressed.
    #[test]
    fn a_dxt1_s_one_bit_texels_go_up_opaque() {
        // (c0 blue under c1 magenta: three colors; the west half index 3.)
        let one_bit = [0x1F, 0x00, 0x1F, 0xF8, 0x0F, 0x0F, 0x0F, 0x0F];
        let four = [0x1F, 0xF8, 0x1F, 0x00, 0x0F, 0x0F, 0x0F, 0x0F];
        let three = [0x1F, 0x00, 0x1F, 0xF8, 0x05, 0x05, 0x05, 0x05];
        let tex = |block: [u8; 8]| Texture {
            width: 8,
            height: 8,
            format: Format::Bc1,
            mips: vec![block.repeat(4), block.to_vec(), block.to_vec(), block.to_vec()],
            has_alpha: false,
            alpha_mean: Some(1.0),
        };
        let cut = tex(one_bit);
        assert!(one_bit_texels(&cut));
        assert!(!one_bit_texels(&tex(four)) && !one_bit_texels(&tex(three)));
        // (In a smaller level alone.)
        let mut deep = tex(four);
        deep.mips[3] = one_bit.to_vec();
        assert!(one_bit_texels(&deep));
        // (A DXT1 read as having alpha keeps it: not this one's case.)
        assert!(!one_bit_texels(&Texture { has_alpha: true, ..cut.clone() }));
        let level = stored_level(&cut, 0).unwrap();
        assert_eq!(&level[..8], [0, 0, 0, 255, 0, 0, 0, 255], "index 3 opaque black");
        let Some(gpu) = mg_testkit::gpu::open(Gpu::headless) else { return };
        let up = GpuTexture::upload(&gpu, "dxt1", &cut, true);
        assert_eq!(up.texture.format(), wgpu::TextureFormat::Rgba8Unorm);
        assert_eq!(up.texture.mip_level_count(), 4, "the stored levels");
        assert!(!up.has_alpha);
        let once = GpuTexture::upload(&gpu, "dxt1", &cut, false);
        assert_eq!(once.texture.mip_level_count(), 1);
        if gpu.bc_textures() {
            let plain = GpuTexture::upload(&gpu, "dxt1", &tex(four), true);
            assert_eq!(plain.texture.format(), wgpu::TextureFormat::Bc1RgbaUnorm);
        }
    }

    /// A cube map is uploaded within Options › Largest textures, as a
    /// texture is (a hak's 1024-pixel cube took 32 MB where the limit
    /// asked for 2), and its BC faces as they are stored where the device
    /// reads BC and they store every level (decoded, four to eight times
    /// their size).
    #[test]
    fn a_cube_is_uploaded_within_the_limit_and_as_stored() {
        let Some(gpu) = mg_testkit::gpu::open(Gpu::headless) else { return };
        let plain = Texture {
            width: 16,
            height: 16,
            format: Format::Rgba8,
            mips: vec![[255, 0, 0, 255].repeat(256)],
            has_alpha: false,
            alpha_mean: None,
        };
        let cube = |face: &Texture| {
            GpuTexture::upload_cube(&gpu, "cube", &std::array::from_fn(|_| face.clone()))
        };
        let size = |t: &GpuTexture| (t.texture.width(), t.texture.mip_level_count());
        assert_eq!(size(&cube(&plain)), (16, 5));
        let whole = cube(&plain).bytes();
        gpu.set_texture_limit(Some(4));
        let limited = cube(&plain);
        assert_eq!(size(&limited), (4, 3), "the first level within the limit");
        assert_eq!(limited.texture.format(), wgpu::TextureFormat::Rgba8Unorm);
        assert!(limited.bytes() * 10 < whole, "{} of {whole}", limited.bytes());
        gpu.set_texture_limit(None);
        // BC1 faces with every level down to a pixel (one block each from
        // 4 pixels down).
        let block = [0x00, 0xF8, 0x00, 0xF8, 0, 0, 0, 0];
        let stored = Texture {
            format: Format::Bc1,
            mips: [16, 4, 1, 1, 1].iter().map(|blocks| block.repeat(*blocks)).collect(),
            ..plain.clone()
        };
        let compressed = cube(&stored);
        if gpu.bc_textures() {
            assert_eq!(compressed.texture.format(), wgpu::TextureFormat::Bc1RgbaUnorm);
            assert_eq!(size(&compressed), (16, 5));
            assert!(compressed.bytes() * 4 <= whole, "{} of {whole}", compressed.bytes());
            // Within the limit, from a level whose sides are whole blocks.
            gpu.set_texture_limit(Some(8));
            assert_eq!(size(&cube(&stored)), (8, 4));
            gpu.set_texture_limit(Some(2));
            assert_eq!(size(&cube(&stored)), (4, 3), "no smaller than a block");
            gpu.set_texture_limit(None);
        }
        // Faces that store one level alone, or disagree: decoded, their
        // levels made, as before.
        // A face with one-bit texels: decoded, opaque (`one_bit_texels`).
        let mut cut = stored.clone();
        cut.mips[1] = [0x1F, 0x00, 0x1F, 0xF8, 0xFF, 0xFF, 0xFF, 0xFF].repeat(4);
        assert_eq!(cube(&cut).texture.format(), wgpu::TextureFormat::Rgba8Unorm);
        let one_level = Texture { mips: vec![stored.mips[0].clone()], ..stored.clone() };
        assert_eq!(cube(&one_level).texture.format(), wgpu::TextureFormat::Rgba8Unorm);
        let mut mixed: [Texture; 6] = std::array::from_fn(|_| stored.clone());
        mixed[3] = plain.clone();
        let mixed = GpuTexture::upload_cube(&gpu, "cube", &mixed);
        assert_eq!(mixed.texture.format(), wgpu::TextureFormat::Rgba8Unorm);
        assert_eq!(size(&mixed), (16, 5));
    }

    #[test]
    fn halving_averages_and_clamps() {
        let data = [0, 0, 0, 0, 255, 255, 255, 255, 100, 100, 100, 100];
        let (out, w, h) = half(&data, 3, 1);
        assert_eq!((w, h), (1, 1));
        // (Averaged over the pixels drawn; half of them are.)
        assert_eq!((out[0], out[3]), (255, 128));
    }

    #[test]
    fn halving_keeps_transparent_pixels_colors_out() {
        // A green leaf beside a transparent pale background.
        let data = [20, 200, 20, 255, 230, 240, 255, 0];
        let (out, ..) = half(&data, 2, 1);
        assert_eq!(out, [20, 200, 20, 128], "the leaf's color, half there");
        // Nothing drawn at all: the colors as they are.
        let (out, ..) = half(&[10, 20, 30, 0, 30, 40, 50, 0], 2, 1);
        assert_eq!(out, [20, 30, 40, 0]);
        // Opaque pixels average as before.
        let (out, ..) = half(&[0, 0, 0, 255, 100, 200, 50, 255], 2, 1);
        assert_eq!(out, [50, 100, 25, 255]);
    }
}
