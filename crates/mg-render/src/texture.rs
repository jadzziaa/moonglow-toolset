//! Textures on the GPU: BC-compressed data uploaded as it is (when the
//! device reads BC and the size allows), everything else as RGBA8 with a
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

/// Halves an RGBA8 image (box filter; odd sizes clamp at the edge).
fn half(data: &[u8], w: u32, h: u32) -> (Vec<u8>, u32, u32) {
    let (nw, nh) = ((w / 2).max(1), (h / 2).max(1));
    let mut out = vec![0u8; nw as usize * nh as usize * 4];
    for y in 0..nh {
        for x in 0..nw {
            let mut sum = [0u32; 4];
            for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                let (sx, sy) = ((2 * x + dx).min(w - 1), (2 * y + dy).min(h - 1));
                let i = (sy as usize * w as usize + sx as usize) * 4;
                for c in 0..4 {
                    sum[c] += u32::from(data[i + c]);
                }
            }
            let o = (y as usize * nw as usize + x as usize) * 4;
            for c in 0..4 {
                out[o + c] = ((sum[c] + 2) / 4) as u8;
            }
        }
    }
    (out, nw, nh)
}

impl GpuTexture {
    /// Uploads a texture; `mipmaps` false keeps one level (TXI `mipmap 0`).
    pub fn upload(gpu: &Gpu, label: &str, tex: &Texture, mipmaps: bool) -> GpuTexture {
        let compressed = tex.format != Format::Rgba8;
        let bc_ok = compressed
            && gpu.bc_textures()
            && tex.width.is_multiple_of(4)
            && tex.height.is_multiple_of(4);
        let (format, levels): (Format, Vec<(Vec<u8>, u32, u32)>) = if bc_ok {
            let n = if mipmaps { tex.mips.len() } else { 1 };
            let levels = tex.mips[..n]
                .iter()
                .enumerate()
                .map(|(i, m)| {
                    let (w, h) = tex.level_dims(i);
                    (m.clone(), w, h)
                })
                .collect();
            (tex.format, levels)
        } else {
            let rgba = tex.to_rgba();
            let mut levels = vec![(rgba.data, rgba.width, rgba.height)];
            if mipmaps {
                while let Some((d, w, h)) = levels.last().filter(|(_, w, h)| *w > 1 || *h > 1) {
                    let next = half(d, *w, *h);
                    levels.push(next);
                }
            }
            (Format::Rgba8, levels)
        };
        let size =
            wgpu::Extent3d { width: tex.width, height: tex.height, depth_or_array_layers: 1 };
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
            let (bytes_per_row, rows, extent) = match format.block_bytes() {
                None => (w * 4, *h, (*w, *h)),
                Some(b) => {
                    let (bw, bh) = (w.div_ceil(4), h.div_ceil(4));
                    (bw * b as u32, bh, (bw * 4, bh * 4))
                }
            };
            gpu.queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &texture,
                    mip_level: level as u32,
                    origin: wgpu::Origin3d::ZERO,
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
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        GpuTexture { texture, view, has_alpha: tex.has_alpha }
    }

    /// A 1×1 texture of one colour.
    pub fn solid(gpu: &Gpu, label: &str, rgba: [u8; 4]) -> GpuTexture {
        let tex = mg_image::Rgba { width: 1, height: 1, data: rgba.to_vec() }.into_texture(true);
        GpuTexture::upload(gpu, label, &tex, false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn halving_averages_and_clamps() {
        let data = [0, 0, 0, 0, 255, 255, 255, 255, 100, 100, 100, 100];
        let (out, w, h) = half(&data, 3, 1);
        assert_eq!((w, h), (1, 1));
        assert_eq!(out[0], 128);
    }
}
