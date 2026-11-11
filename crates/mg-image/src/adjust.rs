//! Pictures made ready for encoding: resized, and alpha bled (the color of
//! hidden texels filled in from the visible ones around them).
//!
//! Both work on [`Rgba`] in any row order. Resizing averages color
//! weighted by alpha (and a little more, so texels all hidden keep their
//! own color), so hidden texels don't tint the visible ones beside them.

use crate::Rgba;

/// Per output texel along one side: the source texels it takes and their
/// weights (summing to 1). Shrinking averages the span each output texel
/// covers (parts of texels at its ends in part); enlarging blends the two
/// nearest (bilinear), the edges held.
fn weights(src: usize, dst: usize) -> Vec<Vec<(usize, f32)>> {
    let scale = src as f64 / dst as f64;
    (0..dst)
        .map(|o| {
            let mut w: Vec<(usize, f32)> = Vec::new();
            if scale > 1.0 {
                let (lo, hi) = (o as f64 * scale, (o + 1) as f64 * scale);
                let mut x = lo.floor() as usize;
                while (x as f64) < hi && x < src {
                    let cover = (hi.min(x as f64 + 1.0) - lo.max(x as f64)) / scale;
                    if cover > 0.0 {
                        w.push((x, cover as f32));
                    }
                    x += 1;
                }
            } else {
                let c = ((o as f64 + 0.5) * scale - 0.5).max(0.0);
                let x0 = (c.floor() as usize).min(src - 1);
                let x1 = (x0 + 1).min(src - 1);
                let t = (c - x0 as f64).clamp(0.0, 1.0) as f32;
                w.push((x0, 1.0 - t));
                if x1 != x0 && t > 0.0 {
                    w.push((x1, t));
                }
            }
            let sum: f32 = w.iter().map(|(_, k)| k).sum();
            w.iter_mut().for_each(|(_, k)| *k /= sum);
            w
        })
        .collect()
}

/// The picture at `width`×`height` (each at least 1): shrunk by area
/// averages, enlarged bilinearly; color weighted by alpha.
pub fn resize(image: &Rgba, width: u32, height: u32) -> Rgba {
    let (w, h) = (image.width as usize, image.height as usize);
    let (nw, nh) = (width.max(1) as usize, height.max(1) as usize);
    if (w, h) == (nw, nh) {
        return image.clone();
    }
    // Five channels: color times weight, the weight, alpha.
    let src: Vec<[f32; 5]> = image
        .data
        .as_chunks::<4>()
        .0
        .iter()
        .map(|p| {
            let wt = f32::from(p[3]) / 255.0 + 1.0 / 256.0;
            let c = |k: usize| f32::from(p[k]) * wt;
            [c(0), c(1), c(2), wt, f32::from(p[3])]
        })
        .collect();
    let (wx, wy) = (weights(w, nw), weights(h, nh));
    let mut across = vec![[0f32; 5]; nw * h];
    for y in 0..h {
        for (x, ws) in wx.iter().enumerate() {
            let mut acc = [0f32; 5];
            for &(sx, k) in ws {
                let s = &src[y * w + sx];
                acc.iter_mut().zip(s).for_each(|(a, s)| *a += s * k);
            }
            across[y * nw + x] = acc;
        }
    }
    let mut out = Rgba::new(nw as u32, nh as u32);
    for (y, ws) in wy.iter().enumerate() {
        for x in 0..nw {
            let mut acc = [0f32; 5];
            for &(sy, k) in ws {
                let s = &across[sy * nw + x];
                acc.iter_mut().zip(s).for_each(|(a, s)| *a += s * k);
            }
            let px = &mut out.data[(y * nw + x) * 4..][..4];
            for k in 0..3 {
                px[k] = (acc[k] / acc[3]).round().clamp(0.0, 255.0) as u8;
            }
            px[3] = acc[4].round().clamp(0.0, 255.0) as u8;
        }
    }
    out
}

/// The picture with the color of every texel whose alpha is under
/// `below` replaced by the mean color of the texels at or over it around
/// it, nearer ones counting more (pulled up a pyramid of halvings and
/// pushed back down, so a hidden area of any size is filled). Alpha is
/// kept; a picture with no texel under `below`, or none at or over it,
/// comes back as it was. Filtering and mip levels then mix the visible
/// colors into the edge of a cut-out, not what lay hidden under it.
pub fn bleed(image: &Rgba, below: u8) -> Rgba {
    let (w, h) = (image.width as usize, image.height as usize);
    let known: Vec<bool> = image.data.as_chunks::<4>().0.iter().map(|p| p[3] >= below).collect();
    if known.iter().all(|&k| k) || !known.iter().any(|&k| k) {
        return image.clone();
    }
    // Level 0: the known texels' colors and a weight of 1 each.
    let mut levels: Vec<(usize, usize, Vec<[f32; 4]>)> = Vec::new();
    let first: Vec<[f32; 4]> = (image.data.as_chunks::<4>().0.iter().zip(&known))
        .map(|(p, &k)| match k {
            true => [f32::from(p[0]), f32::from(p[1]), f32::from(p[2]), 1.0],
            false => [0.0; 4],
        })
        .collect();
    levels.push((w, h, first));
    while levels.last().is_some_and(|(lw, lh, _)| *lw > 1 || *lh > 1) {
        let (lw, lh, cells) = levels.last().expect("a level");
        let (nw, nh) = (lw.div_ceil(2), lh.div_ceil(2));
        let mut next = vec![[0f32; 4]; nw * nh];
        for y in 0..*lh {
            for x in 0..*lw {
                let c = &cells[y * lw + x];
                let n = &mut next[(y / 2) * nw + x / 2];
                n.iter_mut().zip(c).for_each(|(n, c)| *n += c);
            }
        }
        levels.push((nw, nh, next));
    }
    // Down again: a cell with no known texel under it takes its parent's.
    let mut fill: Vec<[f32; 3]> = Vec::new();
    let mut fill_w = 0;
    for (lw, lh, cells) in levels.iter().rev() {
        let mut here = vec![[0f32; 3]; lw * lh];
        for y in 0..*lh {
            for x in 0..*lw {
                let c = &cells[y * lw + x];
                here[y * lw + x] = match c[3] > 0.0 {
                    true => [c[0] / c[3], c[1] / c[3], c[2] / c[3]],
                    false => fill[(y / 2) * fill_w + x / 2],
                };
            }
        }
        fill = here;
        fill_w = *lw;
    }
    let mut out = image.clone();
    for ((p, &k), f) in out.data.as_chunks_mut::<4>().0.iter_mut().zip(&known).zip(&fill) {
        if !k {
            for c in 0..3 {
                p[c] = f[c].round().clamp(0.0, 255.0) as u8;
            }
        }
    }
    out
}

/// The nearest power of two to `n` (by ratio; halfway goes up), at least 1.
pub fn nearest_power_of_two(n: u32) -> u32 {
    let n = n.max(1);
    let up = n.next_power_of_two();
    let down = if up == n { n } else { up / 2 };
    // n/down against up/n: n² against down·up.
    if u64::from(n) * u64::from(n) >= u64::from(down) * u64::from(up) { up } else { down }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn picture(w: u32, h: u32, f: impl Fn(u32, u32) -> [u8; 4]) -> Rgba {
        let mut img = Rgba::new(w, h);
        for y in 0..h {
            for x in 0..w {
                img.data[((y * w + x) * 4) as usize..][..4].copy_from_slice(&f(x, y));
            }
        }
        img
    }

    #[test]
    fn shrinking_averages_and_enlarging_blends() {
        let img = picture(4, 2, |x, _| [(x * 60) as u8, 0, 0, 255]);
        let half = resize(&img, 2, 1);
        assert_eq!(half.data, [30, 0, 0, 255, 150, 0, 0, 255]);
        // A third: spans of 4/3 texels.
        let third = resize(&picture(3, 1, |x, _| [(x * 90) as u8, 0, 0, 255]), 1, 1);
        assert_eq!(third.pixel(0, 0), [90, 0, 0, 255]);
        let big = resize(&picture(2, 1, |x, _| [(x * 200) as u8, 0, 0, 255]), 4, 1);
        let reds: Vec<u8> = big.data.as_chunks::<4>().0.iter().map(|p| p[0]).collect();
        assert_eq!(reds, [0, 50, 150, 200]);
        assert_eq!(resize(&img, 4, 2), img);
    }

    #[test]
    fn hidden_texels_do_not_tint_a_resized_picture() {
        let img = picture(2, 1, |x, _| match x {
            0 => [200, 100, 50, 255],
            _ => [255, 0, 255, 0],
        });
        let p = resize(&img, 1, 1).pixel(0, 0);
        assert!(p[0] >= 199 && p[1] >= 99 && p[2] <= 52, "{p:?}");
        assert_eq!(p[3], 128);
    }

    #[test]
    fn bleeding_fills_hidden_colors_from_the_visible_ones() {
        // A green square in the corner of a magenta, hidden field.
        let img = picture(8, 8, |x, y| match x < 2 && y < 2 {
            true => [0, 200, 0, 255],
            false => [255, 0, 255, 0],
        });
        let out = bleed(&img, 1);
        for (p, q) in img.data.as_chunks::<4>().0.iter().zip(out.data.as_chunks::<4>().0) {
            assert_eq!(p[3], q[3], "alpha kept");
            assert_eq!(&q[..3], &[0, 200, 0], "{q:?}");
        }
        // Two colors: each hidden texel takes the nearer one's more.
        let img = picture(8, 1, |x, _| match x {
            0 => [255, 0, 0, 255],
            7 => [0, 0, 255, 255],
            _ => [0, 255, 0, 0],
        });
        let out = bleed(&img, 1);
        assert!(out.pixel(1, 0)[0] > out.pixel(1, 0)[2], "{:?}", out.pixel(1, 0));
        assert!(out.pixel(6, 0)[2] > out.pixel(6, 0)[0], "{:?}", out.pixel(6, 0));
        assert!(out.data.as_chunks::<4>().0.iter().all(|p| p[1] == 0), "no green left");
        // Alpha under the threshold counts as hidden.
        let soft = picture(2, 1, |x, _| [x as u8 * 255, 0, 0, if x == 0 { 255 } else { 100 }]);
        assert_eq!(bleed(&soft, 1), soft);
        assert_eq!(bleed(&soft, 128).pixel(1, 0), [0, 0, 0, 100]);
        // Nothing hidden, or nothing visible: as it was.
        let none = picture(2, 2, |_, _| [1, 2, 3, 0]);
        assert_eq!(bleed(&none, 1), none);
    }

    #[test]
    fn powers_of_two_nearest_by_ratio() {
        let got: Vec<u32> =
            [0, 1, 3, 5, 6, 700, 724, 725, 1000, 1024].map(nearest_power_of_two).into();
        assert_eq!(got, [1, 1, 4, 4, 8, 512, 512, 1024, 1024, 1024]);
    }
}
