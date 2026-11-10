//! The encoder's quality: four kinds of picture made here (photo-like
//! noise, flat colors, a cut-out over colors it hides, gradients with soft
//! alpha), each encoded, decoded and measured (PSNR against the source,
//! color over the texels that show, alpha on its own). The floors are a
//! little under what it gives; Neverblender's numpy encoder
//! (`nwn_textures.write_dds`), measured on the same pictures, gives less
//! (`docs/research/notes_texture_encoder.md`). The pictures and BioWare DDS files
//! of them (and standard ones, `.std.dds`) are left in
//! `target/tmp/encode-quality/` for that comparison.

use mg_image::encode::compress;
use mg_image::{Format, Rgba, bc, dds};

/// A hash of a lattice point, 0 to 1.
fn hash(x: i32, y: i32, seed: u32) -> f32 {
    let mut h = (x as u32).wrapping_mul(0x8da6_b343) ^ (y as u32).wrapping_mul(0xd816_3841) ^ seed;
    h = (h ^ (h >> 13)).wrapping_mul(0x5bd1_e995);
    h ^= h >> 15;
    (h & 0xFFFF) as f32 / 65535.0
}

/// Smooth value noise of a few octaves, about 0 to 1.
fn noise(x: f32, y: f32, seed: u32, octaves: u32) -> f32 {
    let (mut sum, mut amp, mut freq, mut norm) = (0.0, 1.0, 1.0 / 32.0, 0.0);
    for o in 0..octaves {
        let (fx, fy) = (x * freq, y * freq);
        let (ix, iy) = (fx.floor() as i32, fy.floor() as i32);
        let (tx, ty) = (fx - fx.floor(), fy - fy.floor());
        let s = |t: f32| t * t * (3.0 - 2.0 * t);
        let (sx, sy) = (s(tx), s(ty));
        let seed = seed.wrapping_add(o * 7919);
        let a = hash(ix, iy, seed) + (hash(ix + 1, iy, seed) - hash(ix, iy, seed)) * sx;
        let b = hash(ix, iy + 1, seed) + (hash(ix + 1, iy + 1, seed) - hash(ix, iy + 1, seed)) * sx;
        sum += amp * (a + (b - a) * sy);
        norm += amp;
        amp *= 0.55;
        freq *= 2.0;
    }
    sum / norm
}

fn picture(size: u32, f: impl Fn(f32, f32) -> [f32; 4]) -> Rgba {
    let mut img = Rgba::new(size, size);
    for y in 0..size {
        for x in 0..size {
            let c = f(x as f32, y as f32);
            let i = ((y * size + x) * 4) as usize;
            for (d, c) in img.data[i..i + 4].iter_mut().zip(c) {
                *d = (c.clamp(0.0, 1.0) * 255.0).round() as u8;
            }
        }
    }
    img
}

/// The pictures: name, picture, the format the converter would choose.
fn pictures() -> Vec<(&'static str, Rgba, Format)> {
    let photo = picture(256, |x, y| {
        let l = noise(x, y, 1, 6);
        let warm = noise(x, y, 2, 3);
        let detail = noise(x * 4.0, y * 4.0, 3, 2) - 0.5;
        [
            l * 0.9 + warm * 0.2 + detail * 0.2,
            l * 0.8 + detail * 0.15,
            l * 0.6 - warm * 0.2 + 0.2,
            1.0,
        ]
    });
    let flat = picture(256, |x, y| {
        let (tx, ty) = ((x / 32.0) as i32, (y / 32.0) as i32);
        let (dx, dy) = (x % 64.0 - 32.0, y % 64.0 - 32.0);
        if dx * dx + dy * dy < 150.0 {
            return [0.93, 0.12, 0.27, 1.0];
        }
        [hash(tx, ty, 11), hash(tx, ty, 12), hash(tx, ty, 13), 1.0]
    });
    let cutout = picture(256, |x, y| {
        // Leaves: discs on a grid, a little apart, their edges smoothed
        // over a texel and a half; under them magenta.
        let (cx, cy) = ((x / 40.0).floor() * 40.0 + 20.0, (y / 40.0).floor() * 40.0 + 20.0);
        let r = 12.0 + 6.0 * hash(cx as i32, cy as i32, 5);
        let d = ((x - cx).powi(2) * 0.6 + (y - cy).powi(2)).sqrt();
        let a = ((r - d) / 1.5 + 0.5).clamp(0.0, 1.0);
        match a > 0.0 {
            true => {
                let n = noise(x, y, 7, 5);
                [0.2 + 0.3 * n, 0.35 + 0.5 * n, 0.1 + 0.2 * n, a]
            }
            false => [1.0, 0.0, 1.0, 0.0],
        }
    });
    let gradient = picture(256, |x, y| {
        // (Alpha in waves, a few texels from clear to solid.)
        let a = 0.5 + 0.5 * (x / 9.0).sin() * (y / 13.0).cos();
        [x / 255.0, y / 255.0, 1.0 - (x + y) / 510.0, a]
    });
    vec![
        ("photo", photo, Format::Bc1),
        ("flat", flat, Format::Bc1),
        ("cutout", cutout, Format::Bc3),
        ("gradient", gradient, Format::Bc3),
    ]
}

fn psnr(sum: f64, n: f64) -> f64 {
    if sum == 0.0 { 99.0 } else { 10.0 * (255.0f64 * 255.0 / (sum / n)).log10() }
}

/// Color PSNR over the texels that show (alpha 128 and up) and alpha PSNR
/// over all.
fn measure(src: &Rgba, back: &[u8]) -> (f64, f64) {
    let (mut c, mut cn, mut a) = (0f64, 0f64, 0f64);
    for (p, q) in src.data.as_chunks::<4>().0.iter().zip(back.as_chunks::<4>().0) {
        if p[3] >= 128 {
            for k in 0..3 {
                c += (f64::from(p[k]) - f64::from(q[k])).powi(2);
            }
            cn += 3.0;
        }
        a += (f64::from(p[3]) - f64::from(q[3])).powi(2);
    }
    (psnr(c, cn), psnr(a, (src.data.len() / 4) as f64))
}

#[test]
fn encoded_pictures_keep_their_quality() {
    let dir = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("encode-quality");
    std::fs::create_dir_all(&dir).unwrap();
    // (Floors: color, alpha.)
    let floors = [
        ("photo", 40.5, 0.0),
        ("flat", 49.5, 0.0),
        ("cutout", 43.0, 40.3),
        ("gradient", 45.0, 47.0),
    ];
    let mut said = Vec::new();
    for (name, img, format) in pictures() {
        let t = compress(&img, format, true, false);
        let back = bc::decode(format, img.width, img.height, &t.mips[0]);
        let (color, alpha) = measure(&img, &back);
        said.push(format!("{name} {format:?}: color {color:.2} dB, alpha {alpha:.2} dB"));
        let png = mg_image::png::write_rgba(img.width, img.height, &img.data).unwrap();
        std::fs::write(dir.join(format!("{name}.png")), png).unwrap();
        std::fs::write(dir.join(format!("{name}.dds")), dds::write_bioware(&t).unwrap()).unwrap();
        let standard = dds::write_standard(&t).unwrap();
        std::fs::write(dir.join(format!("{name}.std.dds")), standard).unwrap();
        let (_, c, a) = floors.iter().find(|f| f.0 == name).copied().unwrap();
        assert!(color >= c && (format == Format::Bc1 || alpha >= a), "{}", said.last().unwrap());
    }
    eprintln!("{}", said.join("\n"));
}
