//! Block-compressed (BC1–BC5, a.k.a. DXT1/3/5, ATI1/2) decoding on the CPU,
//! for thumbnails and for GPUs without BC support. Blocks cover 4×4 texels
//! in stored order; colours expand by bit replication and interpolate with
//! integer thirds (and sevenths or fifths for alpha), as common decoders do.

use crate::Format;

fn rgb565(v: u16) -> [u8; 3] {
    let r = ((v >> 11) & 0x1F) as u8;
    let g = ((v >> 5) & 0x3F) as u8;
    let b = (v & 0x1F) as u8;
    [(r << 3) | (r >> 2), (g << 2) | (g >> 4), (b << 3) | (b >> 2)]
}

/// A colour block (BC1, and the colour half of BC2/BC3): 16 RGBA texels.
/// `four_colour` forces the four-colour mode (BC2/BC3 always use it).
fn colour_block(b: &[u8], four_colour: bool, out: &mut [[u8; 4]; 16]) {
    let c0 = u16::from_le_bytes([b[0], b[1]]);
    let c1 = u16::from_le_bytes([b[2], b[3]]);
    let (p0, p1) = (rgb565(c0), rgb565(c1));
    let mix = |a: u8, b: u8, wa: u16, wb: u16, d: u16| ((a as u16 * wa + b as u16 * wb) / d) as u8;
    let mut pal = [[0u8; 4]; 4];
    pal[0] = [p0[0], p0[1], p0[2], 255];
    pal[1] = [p1[0], p1[1], p1[2], 255];
    if c0 > c1 || four_colour {
        for i in 0..3 {
            pal[2][i] = mix(p0[i], p1[i], 2, 1, 3);
            pal[3][i] = mix(p0[i], p1[i], 1, 2, 3);
        }
        pal[2][3] = 255;
        pal[3][3] = 255;
    } else {
        for i in 0..3 {
            pal[2][i] = mix(p0[i], p1[i], 1, 1, 2);
        }
        pal[2][3] = 255;
        pal[3] = [0, 0, 0, 0];
    }
    let bits = u32::from_le_bytes([b[4], b[5], b[6], b[7]]);
    for (i, px) in out.iter_mut().enumerate() {
        *px = pal[((bits >> (2 * i)) & 3) as usize];
    }
}

/// An interpolated 8-bit channel block (BC3 alpha, BC4, BC5 halves).
fn channel_block(b: &[u8], out: &mut [u8; 16]) {
    let (a0, a1) = (b[0] as u16, b[1] as u16);
    let mut pal = [0u8; 8];
    pal[0] = a0 as u8;
    pal[1] = a1 as u8;
    if a0 > a1 {
        for i in 1..7u16 {
            pal[i as usize + 1] = (((7 - i) * a0 + i * a1) / 7) as u8;
        }
    } else {
        for i in 1..5u16 {
            pal[i as usize + 1] = (((5 - i) * a0 + i * a1) / 5) as u8;
        }
        pal[6] = 0;
        pal[7] = 255;
    }
    let mut bits = 0u64;
    for (i, &byte) in b[2..8].iter().enumerate() {
        bits |= (byte as u64) << (8 * i);
    }
    for (i, v) in out.iter_mut().enumerate() {
        *v = pal[((bits >> (3 * i)) & 7) as usize];
    }
}

/// Decodes one level to RGBA8, rows in stored order. BC4 gives grey, BC5
/// red and green (blue 0). Missing data decodes as black.
pub fn decode(format: Format, width: u32, height: u32, data: &[u8]) -> Vec<u8> {
    let (w, h) = (width.max(1) as usize, height.max(1) as usize);
    let mut out = vec![0u8; w * h * 4];
    let Some(block_bytes) = format.block_bytes() else {
        let n = out.len().min(data.len());
        out[..n].copy_from_slice(&data[..n]);
        return out;
    };
    let (bw, bh) = (w.div_ceil(4), h.div_ceil(4));
    for by in 0..bh {
        for bx in 0..bw {
            let at = (by * bw + bx) * block_bytes;
            let Some(b) = data.get(at..at + block_bytes) else { continue };
            let mut px = [[0u8; 4]; 16];
            match format {
                Format::Bc1 => colour_block(b, false, &mut px),
                Format::Bc2 => {
                    colour_block(&b[8..], true, &mut px);
                    for (i, p) in px.iter_mut().enumerate() {
                        let nibble = (b[i / 2] >> (4 * (i % 2))) & 0xF;
                        p[3] = nibble * 17;
                    }
                }
                Format::Bc3 => {
                    colour_block(&b[8..], true, &mut px);
                    let mut a = [0u8; 16];
                    channel_block(&b[..8], &mut a);
                    for (p, a) in px.iter_mut().zip(a) {
                        p[3] = a;
                    }
                }
                Format::Bc4 => {
                    let mut r = [0u8; 16];
                    channel_block(b, &mut r);
                    for (p, r) in px.iter_mut().zip(r) {
                        *p = [r, r, r, 255];
                    }
                }
                Format::Bc5 => {
                    let (mut r, mut g) = ([0u8; 16], [0u8; 16]);
                    channel_block(&b[..8], &mut r);
                    channel_block(&b[8..], &mut g);
                    for i in 0..16 {
                        px[i] = [r[i], g[i], 0, 255];
                    }
                }
                Format::Rgba8 => unreachable!("handled above"),
            }
            for (i, p) in px.iter().enumerate() {
                let (x, y) = (bx * 4 + i % 4, by * 4 + i / 4);
                if x < w && y < h {
                    out[(y * w + x) * 4..][..4].copy_from_slice(p);
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bc1_four_and_three_colour_modes() {
        // Red (0xF800) over blue (0x001F): four colours, indices 0,1,2,3.
        let mut block = vec![0x00, 0xF8, 0x1F, 0x00];
        block.extend_from_slice(&[0b1110_0100, 0, 0, 0]);
        let px = decode(Format::Bc1, 4, 4, &block);
        assert_eq!(&px[0..4], &[255, 0, 0, 255]);
        assert_eq!(&px[4..8], &[0, 0, 255, 255]);
        assert_eq!(&px[8..12], &[170, 0, 85, 255]);
        assert_eq!(&px[12..16], &[85, 0, 170, 255]);
        // Swapped endpoints: three colours and transparent black.
        let mut block = vec![0x1F, 0x00, 0x00, 0xF8];
        block.extend_from_slice(&[0b1110_0100, 0, 0, 0]);
        let px = decode(Format::Bc1, 4, 4, &block);
        assert_eq!(&px[8..12], &[127, 0, 127, 255]);
        assert_eq!(&px[12..16], &[0, 0, 0, 0]);
    }

    #[test]
    fn bc4_interpolates() {
        // 255 to 0, index 1 for texel 0 then 2 for texel 1.
        let block = [255, 0, 0b0001_0001, 0, 0, 0, 0, 0];
        let px = decode(Format::Bc4, 4, 4, &block);
        assert_eq!(px[0], 0);
        assert_eq!(px[4], 218);
    }

    #[test]
    fn partial_blocks_are_clipped() {
        let block = [0xFF, 0xFF, 0xFF, 0xFF, 0, 0, 0, 0];
        assert_eq!(decode(Format::Bc1, 2, 1, &block), vec![255; 8]);
    }
}
