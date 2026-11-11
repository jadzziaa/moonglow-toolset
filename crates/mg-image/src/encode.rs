//! BC1 (DXT1) and BC3 (DXT5) encoding on the CPU, and the mip chain a
//! texture is encoded with.
//!
//! A color block's endpoints start at the ends of its colors' principal
//! axis and are refitted by least squares to the indices they give, a few
//! times over, the best kept; a block of one color takes the endpoints
//! whose blend comes nearest it. Every choice is scored with the
//! decoder's own palettes ([`crate::bc`]), integer thirds and all. An
//! alpha block is tried in both of BC3's modes (eight steps between its
//! ends, or six with exact 0 and 255, which a cut-out's edge needs) and
//! the better kept. BC1 with alpha is the punch-through mode: a texel
//! under half alpha transparent, the rest in three colors (the game
//! draws DXT1's transparent texels black and the mesh opaque, so it is
//! for other programs).
//!
//! Mip levels halve each side (down to 1×1, odd sides rounded down, as
//! [`crate::dds`] reads them) by averaging 2×2 texels, color weighted by
//! alpha so hidden texels don't tint the visible ones.

use std::sync::OnceLock;

use crate::bc::{channel_palette, color_palette};
use crate::{Format, Rgba, Texture};

/// Encodes a picture (rows in the game's order, as [`Rgba`] keeps them)
/// as a texture of `format` ([`Format::Bc1`] or [`Format::Bc3`]): its
/// largest level, and with `mipmaps` every smaller one down to 1×1.
/// BC1 keeps alpha only where `punch_through` asks for it. The texture's
/// `alpha_mean` is the largest level's mean alpha (1 for BC1), as
/// BioWare's DDS header carries it.
///
/// # Panics
/// For a format other than BC1 or BC3.
pub fn compress(image: &Rgba, format: Format, mipmaps: bool, punch_through: bool) -> Texture {
    assert!(matches!(format, Format::Bc1 | Format::Bc3), "only BC1 and BC3 are encoded");
    let alpha = format == Format::Bc3 || punch_through;
    let mut levels = vec![encode_level(image, format, punch_through)];
    if mipmaps {
        let mut level = image.clone();
        while level.width > 1 || level.height > 1 {
            level = halve(&level, alpha);
            levels.push(encode_level(&level, format, punch_through));
        }
    }
    let alpha_mean = match format {
        Format::Bc3 => mean_alpha(image),
        _ => 1.0,
    };
    Texture {
        width: image.width,
        height: image.height,
        format,
        mips: levels,
        has_alpha: format == Format::Bc3,
        alpha_mean: Some(alpha_mean),
    }
}

/// The mean of a picture's alpha, 0 to 1.
pub fn mean_alpha(image: &Rgba) -> f32 {
    let sum: u64 = image.data.as_chunks::<4>().0.iter().map(|p| u64::from(p[3])).sum();
    let n = (image.data.len() / 4).max(1) as f64;
    (sum as f64 / n / 255.0) as f32
}

/// The next mip level: each side halved (odd sides rounded down, never
/// below 1), each texel the mean of the 2×2 (or 2×1) it covers. With
/// `alpha`, color is weighted by alpha (and a little more, so a block
/// all hidden keeps its own mean color).
pub fn halve(image: &Rgba, alpha: bool) -> Rgba {
    let (w, h) = (image.width as usize, image.height as usize);
    let (nw, nh) = ((w / 2).max(1), (h / 2).max(1));
    let (fx, fy) = (if w > 1 { 2 } else { 1 }, if h > 1 { 2 } else { 1 });
    let mut out = Rgba::new(nw as u32, nh as u32);
    for y in 0..nh {
        for x in 0..nw {
            let (mut c, mut wsum, mut a) = ([0u32; 3], 0u32, 0u32);
            for dy in 0..fy {
                for dx in 0..fx {
                    let i = ((y * fy + dy) * w + x * fx + dx) * 4;
                    let p = &image.data[i..i + 4];
                    // (Weights in 1/256: alpha + 1, so 0 still counts.)
                    let wt = if alpha { u32::from(p[3]) + 1 } else { 1 };
                    for k in 0..3 {
                        c[k] += u32::from(p[k]) * wt;
                    }
                    wsum += wt;
                    a += u32::from(p[3]);
                }
            }
            let n = (fx * fy) as u32;
            let o = (y * nw + x) * 4;
            for (d, c) in out.data[o..o + 3].iter_mut().zip(c) {
                *d = ((c + wsum / 2) / wsum) as u8;
            }
            out.data[o + 3] = ((a + n / 2) / n) as u8;
        }
    }
    out
}

/// One level's blocks.
fn encode_level(image: &Rgba, format: Format, punch_through: bool) -> Vec<u8> {
    let (w, h) = (image.width.max(1) as usize, image.height.max(1) as usize);
    let (bw, bh) = (w.div_ceil(4), h.div_ceil(4));
    let mut out = Vec::with_capacity(format.level_size(w as u32, h as u32));
    for by in 0..bh {
        for bx in 0..bw {
            // A block past the picture's edge repeats its last texels.
            let mut px = [[0u8; 4]; 16];
            for (i, p) in px.iter_mut().enumerate() {
                let x = (bx * 4 + i % 4).min(w - 1);
                let y = (by * 4 + i / 4).min(h - 1);
                let at = (y * w + x) * 4;
                p.copy_from_slice(&image.data[at..at + 4]);
            }
            match format {
                Format::Bc3 => {
                    let mut a = [0u8; 16];
                    for (a, p) in a.iter_mut().zip(&px) {
                        *a = p[3];
                    }
                    out.extend_from_slice(&alpha_block(&a));
                    out.extend_from_slice(&color_block(&px, Mode::Bc3));
                }
                _ => {
                    let mode = match punch_through && px.iter().any(|p| p[3] < 128) {
                        true => Mode::PunchThrough,
                        false => Mode::Opaque,
                    };
                    out.extend_from_slice(&color_block(&px, mode));
                }
            }
        }
    }
    out
}

/// How a color block may be written.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    /// BC1 without alpha: the four-color mode only (c0 > c1, or both one
    /// color with every index 0), never the transparent index.
    Opaque,
    /// BC1 with alpha: the three-color mode, index 3 for texels under
    /// half alpha.
    PunchThrough,
    /// BC3's color half: always decoded in four colors.
    Bc3,
}

/// The squared error of an RGB texel against a palette color.
fn dist(a: [u8; 4], b: [u8; 4]) -> u32 {
    (0..3).map(|k| (i32::from(a[k]) - i32::from(b[k])).pow(2) as u32).sum()
}

/// The 565 value nearest an RGB color in 0–255 floats.
fn to565(c: [f32; 3]) -> u16 {
    let q = |v: f32, max: f32| (v.clamp(0.0, 255.0) * max / 255.0).round() as u16;
    (q(c[0], 31.0) << 11) | (q(c[1], 63.0) << 5) | q(c[2], 31.0)
}

/// Indices for `px` against the palette of `c0`/`c1` in `mode`, and the
/// error they leave. `hidden` texels take index 3 (punch-through only)
/// and count nothing.
fn fit(px: &[[u8; 4]; 16], hidden: u16, c0: u16, c1: u16, mode: Mode) -> (u32, [u8; 16]) {
    let pal = color_palette(c0, c1, mode == Mode::Bc3);
    // The indices that may be used for a visible texel.
    let usable: &[usize] = match (mode, c0 > c1) {
        (Mode::PunchThrough, _) => &[0, 1, 2],
        (Mode::Opaque, false) => &[0],
        _ => &[0, 1, 2, 3],
    };
    let mut idx = [0u8; 16];
    let mut err = 0;
    for (i, p) in px.iter().enumerate() {
        if hidden & (1 << i) != 0 {
            idx[i] = 3;
            continue;
        }
        let (best, e) = usable
            .iter()
            .map(|&k| (k, dist(*p, pal[k])))
            .min_by_key(|&(_, e)| e)
            .expect("an index is usable");
        idx[i] = best as u8;
        err += e;
    }
    (err, idx)
}

/// The block's bytes: the endpoints, then the indices.
fn block_bytes(c0: u16, c1: u16, idx: &[u8; 16]) -> [u8; 8] {
    let mut bits = 0u32;
    for (i, &k) in idx.iter().enumerate() {
        bits |= u32::from(k) << (2 * i);
    }
    let mut b = [0u8; 8];
    b[0..2].copy_from_slice(&c0.to_le_bytes());
    b[2..4].copy_from_slice(&c1.to_le_bytes());
    b[4..8].copy_from_slice(&bits.to_le_bytes());
    b
}

/// A candidate pair of endpoints put in the order `mode` needs (opaque:
/// c0 > c1; punch-through: c0 <= c1), scored, and kept if better than
/// `best`.
fn consider(
    px: &[[u8; 4]; 16],
    hidden: u16,
    mode: Mode,
    (mut c0, mut c1): (u16, u16),
    best: &mut (u32, u16, u16, [u8; 16]),
) {
    match mode {
        Mode::PunchThrough if c0 > c1 => std::mem::swap(&mut c0, &mut c1),
        Mode::Opaque | Mode::Bc3 if c0 < c1 => std::mem::swap(&mut c0, &mut c1),
        _ => {}
    }
    let (err, idx) = fit(px, hidden, c0, c1, mode);
    if err < best.0 {
        *best = (err, c0, c1, idx);
    }
}

/// Encodes a color block.
fn color_block(px: &[[u8; 4]; 16], mode: Mode) -> [u8; 8] {
    let hidden: u16 = match mode {
        Mode::PunchThrough => (0..16).filter(|&i| px[i][3] < 128).fold(0, |m, i| m | 1 << i),
        _ => 0,
    };
    let visible: Vec<[f32; 3]> = (0..16)
        .filter(|&i| hidden & (1 << i) == 0)
        .map(|i| [px[i][0] as f32, px[i][1] as f32, px[i][2] as f32])
        .collect();
    if visible.is_empty() {
        return block_bytes(0, 0, &[3; 16]);
    }
    let mut best = (u32::MAX, 0u16, 0u16, [0u8; 16]);
    // One color: the endpoints whose blend comes nearest it, and the
    // color itself.
    let first = visible[0];
    if visible.iter().all(|c| *c == first) {
        let c = [first[0] as u8, first[1] as u8, first[2] as u8];
        let q = to565(first);
        consider(px, hidden, mode, (q, q), &mut best);
        let (a, b) = single_color(c, mode == Mode::PunchThrough);
        consider(px, hidden, mode, (a, b), &mut best);
        let (_, c0, c1, idx) = best;
        return block_bytes(c0, c1, &idx);
    }
    // The principal axis of the colors, by power iteration.
    let n = visible.len() as f32;
    let mut mean = [0f32; 3];
    for c in &visible {
        for k in 0..3 {
            mean[k] += c[k] / n;
        }
    }
    let mut cov = [[0f32; 3]; 3];
    for c in &visible {
        let d = [c[0] - mean[0], c[1] - mean[1], c[2] - mean[2]];
        for i in 0..3 {
            for j in 0..3 {
                cov[i][j] += d[i] * d[j];
            }
        }
    }
    let mut axis = [1f32, 1.0, 1.0];
    for _ in 0..8 {
        let next = [0, 1, 2].map(|i| (0..3).map(|j| cov[i][j] * axis[j]).sum::<f32>());
        let len = next.iter().map(|v| v * v).sum::<f32>().sqrt();
        if len < 1e-6 {
            break;
        }
        axis = next.map(|v| v / len);
    }
    let proj = |c: &[f32; 3]| (0..3).map(|k| (c[k] - mean[k]) * axis[k]).sum::<f32>();
    let (lo, hi) =
        visible.iter().map(proj).fold((f32::MAX, f32::MIN), |(lo, hi), t| (lo.min(t), hi.max(t)));
    let at = |t: f32| [0, 1, 2].map(|k| mean[k] + axis[k] * t);
    consider(px, hidden, mode, (to565(at(hi)), to565(at(lo))), &mut best);
    // A little inside the ends too (the extremes are often a texel or two).
    let inset = (hi - lo) / 16.0;
    consider(px, hidden, mode, (to565(at(hi - inset)), to565(at(lo + inset))), &mut best);
    // Least squares: the endpoints that best give the colors with the
    // indices chosen, a few rounds.
    let weights: [f32; 4] = match mode {
        Mode::PunchThrough => [1.0, 0.0, 0.5, 0.0],
        _ => [1.0, 0.0, 2.0 / 3.0, 1.0 / 3.0],
    };
    for _ in 0..3 {
        let (_, c0, c1, idx) = best;
        let (mut aa, mut ab, mut bb) = (0f32, 0f32, 0f32);
        let (mut ax, mut bx) = ([0f32; 3], [0f32; 3]);
        for i in 0..16 {
            if hidden & (1 << i) != 0 {
                continue;
            }
            let wa = weights[idx[i] as usize];
            let wb = 1.0 - wa;
            aa += wa * wa;
            ab += wa * wb;
            bb += wb * wb;
            for k in 0..3 {
                ax[k] += wa * px[i][k] as f32;
                bx[k] += wb * px[i][k] as f32;
            }
        }
        let det = aa * bb - ab * ab;
        if det.abs() < 1e-6 {
            break;
        }
        let e0 = [0, 1, 2].map(|k| (ax[k] * bb - bx[k] * ab) / det);
        let e1 = [0, 1, 2].map(|k| (bx[k] * aa - ax[k] * ab) / det);
        let (q0, q1) = (to565(e0), to565(e1));
        consider(px, hidden, mode, (q0, q1), &mut best);
        if (best.1, best.2) == (c0, c1) || (best.1, best.2) == (c1, c0) {
            // (No better: try the neighbors of the refit's 565 values.)
            for (d0, d1) in [(1i32, 0i32), (-1, 0), (0, 1), (0, -1)] {
                let nudge = |q: u16, d: i32| {
                    let g = (((q >> 5) & 63) as i32 + d).clamp(0, 63) as u16;
                    (q & !(63 << 5)) | (g << 5)
                };
                consider(px, hidden, mode, (nudge(q0, d0), nudge(q1, d1)), &mut best);
            }
            break;
        }
    }
    let (_, c0, c1, idx) = best;
    block_bytes(c0, c1, &idx)
}

/// For one color: the endpoints (each channel's pair) whose two-thirds
/// blend (or midpoint, `three`) decodes nearest it, by tables made once.
fn single_color(c: [u8; 3], three: bool) -> (u16, u16) {
    type Table = [[(u8, u8); 256]; 2];
    // For each target value, the (e0, e1) of 5 and of 6 bits that give
    // it best with the 2/3 blend, and with the midpoint.
    static TABLES: OnceLock<[Table; 2]> = OnceLock::new();
    let tables = TABLES.get_or_init(|| {
        let make = |bits: u32, blend: fn(u16, u16) -> u16| {
            let max = (1u16 << bits) - 1;
            let expand = |v: u16| match bits {
                5 => (v << 3) | (v >> 2),
                _ => (v << 2) | (v >> 4),
            };
            let mut t = [(0u8, 0u8); 256];
            for (target, slot) in t.iter_mut().enumerate() {
                let mut best = (u16::MAX, 0, 0);
                for a in 0..=max {
                    for b in 0..=max {
                        let v = blend(expand(a), expand(b));
                        let e = (v as i32 - target as i32).unsigned_abs() as u16;
                        if e < best.0 {
                            best = (e, a, b);
                        }
                    }
                }
                *slot = (best.1 as u8, best.2 as u8);
            }
            t
        };
        let third: fn(u16, u16) -> u16 = |a, b| (2 * a + b) / 3;
        let half: fn(u16, u16) -> u16 = |a, b| (a + b) / 2;
        [[make(5, third), make(6, third)], [make(5, half), make(6, half)]]
    });
    let t = &tables[usize::from(three)];
    let (r, g, b) = (t[0][c[0] as usize], t[1][c[1] as usize], t[0][c[2] as usize]);
    let pack = |r: u8, g: u8, b: u8| (u16::from(r) << 11) | (u16::from(g) << 5) | u16::from(b);
    (pack(r.0, g.0, b.0), pack(r.1, g.1, b.1))
}

/// [`channel_palette`]'s values unrounded, in 35ths.
fn exact_channel_palette(a0: u8, a1: u8) -> [i32; 8] {
    let (a0, a1) = (i32::from(a0), i32::from(a1));
    let mut pal = [a0 * 35, a1 * 35, 0, 0, 0, 0, 0, 0];
    if a0 > a1 {
        for i in 1..7 {
            pal[i as usize + 1] = ((7 - i) * a0 + i * a1) * 5;
        }
    } else {
        for i in 1..5 {
            pal[i as usize + 1] = ((5 - i) * a0 + i * a1) * 7;
        }
        pal[7] = 255 * 35;
    }
    pal
}

/// Encodes a BC3 alpha block: both modes tried, the better kept.
fn alpha_block(a: &[u8; 16]) -> [u8; 8] {
    let score = |a0: u8, a1: u8| -> (u32, [u8; 16]) {
        // (Scored against the decoder's values, rounded down as
        // bc::decode gives them, and in 35ths against the exact ones,
        // as a GPU interpolates them: good for both.)
        let pal = channel_palette(a0, a1);
        let exact = exact_channel_palette(a0, a1);
        let mut idx = [0u8; 16];
        let mut err = 0;
        for (i, &v) in a.iter().enumerate() {
            let (k, e) = (0..8)
                .map(|k| {
                    let d = (i32::from(pal[k]) - i32::from(v)) * 35;
                    let x = exact[k] - i32::from(v) * 35;
                    (k, (d * d + x * x) as u32)
                })
                .min_by_key(|&(_, e)| e)
                .expect("eight values");
            idx[i] = k as u8;
            err += e;
        }
        (err, idx)
    };
    let (min, max) = (*a.iter().min().expect("16"), *a.iter().max().expect("16"));
    let mut best = (u32::MAX, 0u8, 0u8, [0u8; 16]);
    let mut consider = |a0: u8, a1: u8| {
        let (e, idx) = score(a0, a1);
        if e < best.0 {
            best = (e, a0, a1, idx);
        }
    };
    // Eight steps between the ends (a0 > a1), the ends drawn in by a
    // few levels too (a texel or two at an end costs less than every
    // step between).
    if max > min {
        let reach = ((max - min) / 8).min(3);
        for d0 in 0..=reach {
            for d1 in 0..=reach {
                if max - d0 > min + d1 {
                    consider(max - d0, min + d1);
                }
            }
        }
    } else {
        consider(max, min);
    }
    // Six steps between the values that are not 0 or 255 (a0 <= a1),
    // which have their own indices.
    let inner: Vec<u8> = a.iter().copied().filter(|&v| v != 0 && v != 255).collect();
    match (inner.iter().min(), inner.iter().max()) {
        (Some(&lo), Some(&hi)) => {
            let reach = ((hi - lo) / 6).min(2);
            for d0 in 0..=reach {
                for d1 in 0..=reach {
                    if lo + d0 <= hi - d1 {
                        consider(lo + d0, hi - d1);
                    }
                }
            }
        }
        _ => consider(0, 0),
    }
    let (_, a0, a1, idx) = best;
    let mut bits = 0u64;
    for (i, &k) in idx.iter().enumerate() {
        bits |= u64::from(k) << (3 * i);
    }
    let mut out = [0u8; 8];
    out[0] = a0;
    out[1] = a1;
    out[2..8].copy_from_slice(&bits.to_le_bytes()[..6]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bc::decode;

    fn picture(w: u32, h: u32, f: impl Fn(u32, u32) -> [u8; 4]) -> Rgba {
        let mut img = Rgba::new(w, h);
        for y in 0..h {
            for x in 0..w {
                let i = ((y * w + x) * 4) as usize;
                img.data[i..i + 4].copy_from_slice(&f(x, y));
            }
        }
        img
    }

    #[test]
    fn one_color_blocks_decode_to_their_color_or_next_to_it() {
        for c in [[0, 0, 0], [255, 255, 255], [200, 113, 47], [1, 254, 128], [77, 77, 77]] {
            let img = picture(4, 4, |_, _| [c[0], c[1], c[2], 255]);
            let t = compress(&img, Format::Bc1, false, false);
            let back = decode(Format::Bc1, 4, 4, &t.mips[0]);
            for p in back.as_chunks::<4>().0 {
                for k in 0..3 {
                    assert!((i32::from(p[k]) - i32::from(c[k])).abs() <= 1, "{c:?}: {p:?}");
                }
                assert_eq!(p[3], 255, "opaque: never the transparent index");
            }
        }
    }

    /// Opaque BC1 is always in the four-color mode, so no texel decodes
    /// transparent, whatever the colors.
    #[test]
    fn opaque_bc1_never_uses_the_transparent_index() {
        let img =
            picture(16, 16, |x, y| [(x * 16) as u8, (y * 16) as u8, ((x * y) % 256) as u8, 255]);
        let t = compress(&img, Format::Bc1, true, false);
        for (l, level) in t.mips.iter().enumerate() {
            let (w, h) = t.level_dims(l);
            let back = decode(Format::Bc1, w, h, level);
            assert!(back.as_chunks::<4>().0.iter().all(|p| p[3] == 255), "level {l}");
        }
    }

    #[test]
    fn punch_through_keeps_the_cut() {
        let img = picture(8, 8, |x, y| match (x + y) % 3 {
            0 => [255, 0, 255, 0],
            _ => [(x * 30) as u8, 100, (y * 30) as u8, 255],
        });
        let t = compress(&img, Format::Bc1, false, true);
        let back = decode(Format::Bc1, 8, 8, &t.mips[0]);
        for (p, q) in img.data.as_chunks::<4>().0.iter().zip(back.as_chunks::<4>().0) {
            assert_eq!(p[3] >= 128, q[3] == 255, "{p:?} {q:?}");
        }
    }

    /// Alpha of only 0 and 255 with values between: the six-step mode
    /// keeps all three exact; a smooth ramp takes the eight-step one.
    #[test]
    fn alpha_blocks_take_the_better_mode() {
        let mut a = [0u8; 16];
        for (i, v) in a.iter_mut().enumerate() {
            *v = [0, 255, 100, 120][i % 4];
        }
        let b = alpha_block(&a);
        assert!(b[0] <= b[1], "six-step mode: {b:?}");
        let mut block = b.to_vec();
        block.extend_from_slice(&[0; 8]);
        let back = decode(Format::Bc3, 4, 4, &block);
        let got: Vec<u8> = back.as_chunks::<4>().0.iter().map(|p| p[3]).collect();
        assert_eq!(&got[..4], &[0, 255, 100, 120]);
        let ramp: [u8; 16] = std::array::from_fn(|i| (i * 17) as u8);
        let b = alpha_block(&ramp);
        assert!(b[0] > b[1], "eight-step mode: {b:?}");
        let mut block = b.to_vec();
        block.extend_from_slice(&[0; 8]);
        let back = decode(Format::Bc3, 4, 4, &block);
        for (p, v) in back.as_chunks::<4>().0.iter().zip(ramp) {
            assert!((i32::from(p[3]) - i32::from(v)).abs() <= 18, "{} {v}", p[3]);
        }
    }

    /// Every level down to 1×1, at sizes not a multiple of 4 and under 4;
    /// each level the size the reader expects.
    #[test]
    fn mip_chains_go_down_to_one_texel() {
        for (w, h) in [(1, 1), (2, 3), (5, 3), (16, 4), (6, 10)] {
            let img = picture(w, h, |x, y| [(x * 40) as u8, (y * 40) as u8, 9, 200]);
            for format in [Format::Bc1, Format::Bc3] {
                let t = compress(&img, format, true, false);
                let want = (32 - w.max(h).leading_zeros()) as usize;
                assert_eq!(t.mips.len(), want, "{w}×{h}");
                for (l, level) in t.mips.iter().enumerate() {
                    let (lw, lh) = t.level_dims(l);
                    assert_eq!(level.len(), format.level_size(lw, lh), "{w}×{h} level {l}");
                }
            }
        }
    }

    /// Hidden texels don't tint a mip level's color.
    #[test]
    fn halving_weights_color_by_alpha() {
        let img = picture(2, 2, |x, _| match x {
            0 => [200, 100, 0, 255],
            _ => [0, 0, 255, 0],
        });
        let half = halve(&img, true);
        let p = half.pixel(0, 0);
        assert!(p[0] >= 198 && p[2] <= 2, "{p:?}");
        assert_eq!(p[3], 128);
        let plain = halve(&img, false);
        assert_eq!(plain.pixel(0, 0), [100, 50, 128, 128]);
    }
}
